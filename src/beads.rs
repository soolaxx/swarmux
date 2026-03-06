use crate::config::AppConfig;
use crate::model::{SubmitPayload, TaskRecord, TaskState};
use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::process::Command;

const LABEL_SWARMUX: &str = "swarmux";
const STATE_PREFIX: &str = "swarmux:state:";

#[derive(Debug, Deserialize)]
struct BeadsIssueRow {
    id: String,
    title: String,
    status: String,
    priority: Option<u8>,
    labels: Option<Vec<String>>,
    notes: Option<String>,
    external_ref: Option<String>,
}

pub fn doctor() -> Result<()> {
    if Command::new("bd").arg("--help").output().is_ok() {
        Ok(())
    } else {
        Err(anyhow!("missing dependency: bd"))
    }
}

pub fn init(config: &AppConfig) -> Result<()> {
    std::fs::create_dir_all(&config.home)?;
    std::fs::create_dir_all(config.logs_dir())?;
    std::fs::create_dir_all(config.locks_dir())?;
    run_bd(config, &["init", "--prefix", "swx", "--json"]).map(|_| ())
}

pub fn submit(config: &AppConfig, payload: SubmitPayload) -> Result<TaskRecord> {
    init(config)?;
    let created = run_bd_json::<serde_json::Value>(
        config,
        &[
            "create",
            "--title",
            &payload.title,
            "-t",
            "task",
            "-p",
            &payload.priority.unwrap_or(2).to_string(),
            "-l",
            &format!("{LABEL_SWARMUX},{}queued", STATE_PREFIX),
            "--json",
        ],
    )?;

    let id = created["id"]
        .as_str()
        .ok_or_else(|| anyhow!("bd create did not return id"))?
        .to_string();
    let task = TaskRecord::from_submit_with_id(payload, config, id.clone());
    run_bd(
        config,
        &[
            "update",
            &id,
            "--notes",
            &serde_json::to_string(&task)?,
            "--json",
        ],
    )?;
    show(config, &id)
}

pub fn list(config: &AppConfig) -> Result<Vec<TaskRecord>> {
    init(config)?;
    let rows =
        run_bd_json::<Vec<BeadsIssueRow>>(config, &["list", "-a", "-l", LABEL_SWARMUX, "--json"])?;
    rows.into_iter().map(|row| show(config, &row.id)).collect()
}

pub fn show(config: &AppConfig, id: &str) -> Result<TaskRecord> {
    let rows = run_bd_json::<Vec<BeadsIssueRow>>(config, &["show", id, "--json"])?;
    let row = rows
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("task not found: {id}"))?;

    let notes = row
        .notes
        .ok_or_else(|| anyhow!("beads task missing notes"))?;
    let mut task: TaskRecord =
        serde_json::from_str(&notes).context("failed to parse beads task notes")?;
    task.title = row.title;
    task.external_ref = row.external_ref.or(task.external_ref);
    task.priority = row.priority.unwrap_or(task.priority);
    if let Some(state) = row
        .labels
        .unwrap_or_default()
        .into_iter()
        .find_map(|label| label.strip_prefix(STATE_PREFIX).map(parse_state_label))
    {
        task.state = state?;
    } else {
        task.state = status_to_state(&row.status);
    }
    Ok(task)
}

pub fn set_state(
    config: &AppConfig,
    id: &str,
    state: TaskState,
    reason: String,
    last_error: Option<String>,
) -> Result<TaskRecord> {
    let mut task = show(config, id)?;
    task.state = state.clone();
    task.reason = reason;
    task.last_error = last_error;
    task.updated_at = chrono::Utc::now();
    if task.state.is_terminal() {
        task.finished_at = Some(task.updated_at);
    }

    run_bd(
        config,
        &[
            "update",
            id,
            "--status",
            status_for_state(&task.state),
            "--set-labels",
            &format!("{LABEL_SWARMUX},{}{}", STATE_PREFIX, state_label(&state)),
            "--notes",
            &serde_json::to_string(&task)?,
            "--json",
        ],
    )?;
    show(config, id)
}

fn run_bd(config: &AppConfig, args: &[&str]) -> Result<String> {
    let output = Command::new("bd")
        .current_dir(&config.home)
        .args(args)
        .output()
        .with_context(|| format!("failed to run bd {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(anyhow!("bd failed: {stderr}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_bd_json<T: for<'de> serde::Deserialize<'de>>(
    config: &AppConfig,
    args: &[&str],
) -> Result<T> {
    let raw = run_bd(config, args)?;
    serde_json::from_str(&raw).with_context(|| format!("failed to parse bd JSON: {raw}"))
}

fn status_for_state(state: &TaskState) -> &'static str {
    match state {
        TaskState::Dispatching | TaskState::Running => "in_progress",
        TaskState::Succeeded | TaskState::Failed | TaskState::Canceled => "closed",
        TaskState::Queued | TaskState::WaitingInput => "open",
    }
}

fn status_to_state(status: &str) -> TaskState {
    match status {
        "in_progress" => TaskState::Running,
        "closed" => TaskState::Succeeded,
        _ => TaskState::Queued,
    }
}

fn parse_state_label(label: &str) -> Result<TaskState> {
    match label {
        "queued" => Ok(TaskState::Queued),
        "dispatching" => Ok(TaskState::Dispatching),
        "running" => Ok(TaskState::Running),
        "waiting_input" => Ok(TaskState::WaitingInput),
        "succeeded" => Ok(TaskState::Succeeded),
        "failed" => Ok(TaskState::Failed),
        "canceled" => Ok(TaskState::Canceled),
        other => Err(anyhow!("unknown beads state label: {other}")),
    }
}

fn state_label(state: &TaskState) -> &'static str {
    match state {
        TaskState::Queued => "queued",
        TaskState::Dispatching => "dispatching",
        TaskState::Running => "running",
        TaskState::WaitingInput => "waiting_input",
        TaskState::Succeeded => "succeeded",
        TaskState::Failed => "failed",
        TaskState::Canceled => "canceled",
    }
}
