use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct CodexBridge {
    executable: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexCheck {
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexRunResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

impl CodexBridge {
    pub fn new(executable: impl Into<String>) -> Self {
        Self {
            executable: executable.into(),
        }
    }

    pub async fn check(&self) -> CodexCheck {
        let output = Command::new(&self.executable).arg("--version").output().await;
        match output {
            Ok(output) if output.status.success() => CodexCheck {
                available: true,
                path: Some(self.executable.clone()),
                version: Some(String::from_utf8_lossy(&output.stdout).trim().to_string()),
            },
            _ => CodexCheck {
                available: false,
                path: None,
                version: None,
            },
        }
    }

    pub async fn run_prompt(&self, workspace: &Path, prompt: &str) -> anyhow::Result<CodexRunResult> {
        let output = Command::new(&self.executable)
            .arg("exec")
            .arg("--skip-git-repo-check")
            .arg("--sandbox")
            .arg("read-only")
            .arg("--color")
            .arg("never")
            .arg("--cd")
            .arg(workspace)
            .arg(prompt)
            .output()
            .await
            .with_context(|| format!("failed to launch {}", self.executable))?;

        Ok(CodexRunResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        })
    }
}
