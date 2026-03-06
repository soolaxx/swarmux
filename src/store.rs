use crate::config::AppConfig;
use crate::model::{EventRecord, SubmitPayload, TaskRecord, TaskState};
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use std::fs;
use std::path::PathBuf;

pub struct Store {
    config: AppConfig,
}

impl Store {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    pub fn paths(&self) -> &AppConfig {
        &self.config
    }

    pub fn ensure_layout(&self) -> Result<()> {
        fs::create_dir_all(&self.config.home)?;
        fs::create_dir_all(self.config.tasks_dir())?;
        fs::create_dir_all(self.config.logs_dir())?;
        fs::create_dir_all(self.config.locks_dir())?;

        if !self.config.events_file().exists() {
            fs::write(self.config.events_file(), "")?;
        }

        Ok(())
    }

    pub fn submit(&self, payload: SubmitPayload) -> Result<TaskRecord> {
        self.ensure_layout()?;
        let task = TaskRecord::from_submit(payload, &self.config);
        self.write_task(&task)?;
        self.append_event(EventRecord::submitted(&task))?;
        Ok(task)
    }

    pub fn list(&self) -> Result<Vec<TaskRecord>> {
        self.ensure_layout()?;
        let mut tasks = Vec::new();
        for entry in fs::read_dir(self.config.tasks_dir())? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }

            tasks.push(self.read_task_path(path)?);
        }

        tasks.sort_by_key(|task| task.created_at);
        Ok(tasks)
    }

    pub fn get(&self, id: &str) -> Result<TaskRecord> {
        self.ensure_layout()?;
        self.read_task_path(self.task_path(id))
    }

    pub fn overwrite(
        &self,
        task: &TaskRecord,
        from_state: TaskState,
        reason: String,
    ) -> Result<()> {
        self.ensure_layout()?;
        self.write_task(task)?;
        self.append_event(EventRecord::transition(
            task,
            from_state,
            task.state.clone(),
            reason,
        ))?;
        Ok(())
    }

    pub fn set_state(
        &self,
        id: &str,
        state: TaskState,
        reason: String,
        last_error: Option<String>,
    ) -> Result<TaskRecord> {
        self.ensure_layout()?;
        let mut task = self.get(id)?;
        let previous = task.state.clone();
        task.state = state;
        task.reason = reason.clone();
        task.last_error = last_error;
        task.updated_at = Utc::now();
        if task.state.is_terminal() {
            task.finished_at = Some(task.updated_at);
        }

        self.write_task(&task)?;
        self.append_event(EventRecord::transition(
            &task,
            previous,
            task.state.clone(),
            reason,
        ))?;
        Ok(task)
    }

    fn task_path(&self, id: &str) -> PathBuf {
        self.config.tasks_dir().join(format!("{id}.json"))
    }

    fn read_task_path(&self, path: PathBuf) -> Result<TaskRecord> {
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read task file: {}", path.display()))?;
        serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse task file: {}", path.display()))
    }

    fn write_task(&self, task: &TaskRecord) -> Result<()> {
        let path = self.task_path(&task.id);
        let raw = serde_json::to_vec_pretty(task)?;
        fs::write(&path, raw)
            .with_context(|| format!("failed to write task file: {}", path.display()))
    }

    fn append_event(&self, event: EventRecord) -> Result<()> {
        let line = serde_json::to_string(&event)?;
        let mut content = fs::read_to_string(self.config.events_file())?;
        content.push_str(&line);
        content.push('\n');
        fs::write(self.config.events_file(), content)?;
        Ok(())
    }
}

pub fn require_task_id(id: &str) -> Result<()> {
    if id.is_empty() {
        return Err(anyhow!("task id must not be empty"));
    }

    Ok(())
}
