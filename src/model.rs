use crate::config::AppConfig;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskMode {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitPayload {
    pub title: String,
    pub repo: String,
    pub repo_root: String,
    pub mode: TaskMode,
    #[serde(default)]
    pub worktree: Option<String>,
    #[serde(default)]
    pub session: Option<String>,
    pub command: Vec<String>,
    #[serde(default)]
    pub priority: Option<u8>,
    #[serde(default)]
    pub external_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DryRunSubmitResponse {
    pub ok: bool,
    pub dry_run: bool,
    pub task: SubmitPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Queued,
    Dispatching,
    Running,
    WaitingInput,
    Succeeded,
    Failed,
    Canceled,
}

impl TaskState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Canceled)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: String,
    pub title: String,
    pub repo: String,
    pub repo_root: String,
    pub mode: TaskMode,
    pub branch: Option<String>,
    pub worktree: Option<String>,
    pub session: Option<String>,
    pub command: Vec<String>,
    pub priority: u8,
    pub external_ref: Option<String>,
    pub state: TaskState,
    pub reason: String,
    pub last_error: Option<String>,
    pub log_file: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl TaskRecord {
    pub fn from_submit(payload: SubmitPayload, config: &AppConfig) -> Self {
        let id = format!("swx-{}", Uuid::now_v7().simple());
        Self::from_submit_with_id(payload, config, id)
    }

    pub fn from_submit_with_id(payload: SubmitPayload, config: &AppConfig, id: String) -> Self {
        let now = Utc::now();
        let log_file = config.logs_dir().join(format!("{id}.log"));
        let (branch, worktree, session) = match payload.mode {
            TaskMode::Auto => {
                let repo_key = payload
                    .repo_root
                    .rsplit('/')
                    .find(|part| !part.is_empty())
                    .unwrap_or("repo");
                let branch = format!("swarmux/{id}");
                let worktree = config
                    .home
                    .join("worktrees")
                    .join(repo_key)
                    .join(&id)
                    .display()
                    .to_string();
                let session = format!("swarmux-{id}");
                (Some(branch), Some(worktree), Some(session))
            }
            TaskMode::Manual => (None, payload.worktree, payload.session),
        };

        Self {
            id,
            title: payload.title,
            repo: payload.repo,
            repo_root: payload.repo_root,
            mode: payload.mode,
            branch,
            worktree,
            session,
            command: payload.command,
            priority: payload.priority.unwrap_or(2),
            external_ref: payload.external_ref,
            state: TaskState::Queued,
            reason: "submitted".to_string(),
            last_error: None,
            log_file: log_file.display().to_string(),
            created_at: now,
            updated_at: now,
            finished_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub task_id: String,
    pub from_state: Option<TaskState>,
    pub to_state: TaskState,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

impl EventRecord {
    pub fn submitted(task: &TaskRecord) -> Self {
        Self {
            task_id: task.id.clone(),
            from_state: None,
            to_state: TaskState::Queued,
            reason: "submitted".to_string(),
            timestamp: Utc::now(),
        }
    }

    pub fn transition(
        task: &TaskRecord,
        from_state: TaskState,
        to_state: TaskState,
        reason: String,
    ) -> Self {
        Self {
            task_id: task.id.clone(),
            from_state: Some(from_state),
            to_state,
            reason,
            timestamp: Utc::now(),
        }
    }
}
