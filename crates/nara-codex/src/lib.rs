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
        for executable in self.command_candidates() {
            let output = Command::new(&executable).arg("--version").output().await;
            if let Ok(output) = output {
                if output.status.success() {
                    return CodexCheck {
                        available: true,
                        path: Some(executable),
                        version: Some(String::from_utf8_lossy(&output.stdout).trim().to_string()),
                    };
                }
            }
        }

        CodexCheck {
            available: false,
            path: None,
            version: None,
        }
    }

    pub async fn run_prompt(
        &self,
        workspace: &Path,
        prompt: &str,
    ) -> anyhow::Result<CodexRunResult> {
        let executable = self.resolve_executable().await;
        let output = Command::new(&executable)
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
            .with_context(|| format!("failed to launch {executable}"))?;

        Ok(CodexRunResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        })
    }

    fn command_candidates(&self) -> Vec<String> {
        let mut candidates = vec![self.executable.clone()];

        if cfg!(windows) && !self.executable.to_ascii_lowercase().ends_with(".cmd") {
            candidates.push(format!("{}.cmd", self.executable));
        }

        if cfg!(windows) && self.executable.eq_ignore_ascii_case("codex") {
            candidates.push("codex.exe".into());
        }

        candidates.sort();
        candidates.dedup();
        candidates
    }

    async fn resolve_executable(&self) -> String {
        for executable in self.command_candidates() {
            let output = Command::new(&executable).arg("--version").output().await;
            if output
                .map(|output| output.status.success())
                .unwrap_or(false)
            {
                return executable;
            }
        }

        self.executable.clone()
    }
}
