use anyhow::{anyhow, Context};
use chrono::Utc;
use nara_protocol::{ToolOutput, ToolRunStatus};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ToolRuntime {
    workspace: PathBuf,
}

impl ToolRuntime {
    pub fn new(workspace: impl Into<PathBuf>) -> Self {
        Self {
            workspace: workspace.into(),
        }
    }

    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    pub async fn list_dir(&self, path: Option<PathBuf>) -> anyhow::Result<Vec<String>> {
        let path = path.unwrap_or_else(|| self.workspace.clone());
        let mut entries = tokio::fs::read_dir(&path)
            .await
            .with_context(|| format!("failed to read {}", path.display()))?;
        let mut names = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            names.push(entry.path().display().to_string());
        }

        names.sort();
        Ok(names)
    }

    pub async fn read_file(&self, path: impl AsRef<Path>) -> anyhow::Result<String> {
        tokio::fs::read_to_string(path.as_ref())
            .await
            .with_context(|| format!("failed to read {}", path.as_ref().display()))
    }

    pub async fn run_command(&self, command: &str) -> anyhow::Result<ToolOutput> {
        run_shell("run_command", command, &self.workspace).await
    }

    pub async fn git_status(&self) -> anyhow::Result<ToolOutput> {
        run_process(
            "git_status",
            "git",
            vec!["status".into(), "--short".into()],
            &self.workspace,
        )
        .await
    }

    pub async fn git_diff(&self) -> anyhow::Result<ToolOutput> {
        run_process(
            "git_diff",
            "git",
            vec!["diff".into(), "--stat".into(), "--".into(), ".".into()],
            &self.workspace,
        )
        .await
    }

    pub async fn git_diff_full(&self) -> anyhow::Result<ToolOutput> {
        run_process(
            "git_diff",
            "git",
            vec!["diff".into(), "--".into(), ".".into()],
            &self.workspace,
        )
        .await
    }

    pub async fn open_vscode(&self) -> anyhow::Result<ToolOutput> {
        run_process(
            "open_vscode",
            "code",
            vec![self.workspace.to_string_lossy().to_string()],
            &self.workspace,
        )
        .await
    }

    pub async fn open_file_explorer(&self) -> anyhow::Result<ToolOutput> {
        if cfg!(windows) {
            run_process(
                "open_file_explorer",
                "explorer",
                vec![self.workspace.to_string_lossy().to_string()],
                &self.workspace,
            )
            .await
        } else {
            Err(anyhow!(
                "open_file_explorer is only supported on Windows in this MVP"
            ))
        }
    }

    pub async fn open_terminal(&self) -> anyhow::Result<ToolOutput> {
        if cfg!(windows) {
            run_process(
                "open_terminal",
                "powershell",
                vec![
                    "-NoExit".into(),
                    "-Command".into(),
                    format!("Set-Location -LiteralPath '{}'", self.workspace.display()),
                ],
                &self.workspace,
            )
            .await
        } else {
            Err(anyhow!(
                "open_terminal is only supported on Windows in this MVP"
            ))
        }
    }
}

async fn run_shell(tool_name: &str, command: &str, cwd: &Path) -> anyhow::Result<ToolOutput> {
    let started_at = Utc::now().to_rfc3339();
    let mut process = if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg(command);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("-lc").arg(command);
        cmd
    };

    let output = process
        .current_dir(cwd)
        .output()
        .await
        .with_context(|| format!("failed to execute command: {command}"))?;

    Ok(ToolOutput {
        id: Uuid::new_v4().to_string(),
        tool_name: tool_name.into(),
        status: if output.status.success() {
            ToolRunStatus::Completed
        } else {
            ToolRunStatus::Failed
        },
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code(),
        started_at,
        finished_at: Utc::now().to_rfc3339(),
    })
}

async fn run_process(
    tool_name: &str,
    program: &str,
    args: Vec<String>,
    cwd: &Path,
) -> anyhow::Result<ToolOutput> {
    let started_at = Utc::now().to_rfc3339();
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .await
        .with_context(|| format!("failed to execute {program}"))?;

    Ok(ToolOutput {
        id: Uuid::new_v4().to_string(),
        tool_name: tool_name.into(),
        status: if output.status.success() {
            ToolRunStatus::Completed
        } else {
            ToolRunStatus::Failed
        },
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code(),
        started_at,
        finished_at: Utc::now().to_rfc3339(),
    })
}
