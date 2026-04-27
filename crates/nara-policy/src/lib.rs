use nara_protocol::RiskLevel;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyContext {
    pub workspace: Option<PathBuf>,
    pub allow_outside_workspace: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolIntent {
    pub name: String,
    pub command: Option<String>,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub risk_level: RiskLevel,
    pub reason: String,
}

impl PolicyDecision {
    pub fn safe(reason: impl Into<String>) -> Self {
        Self {
            risk_level: RiskLevel::Safe,
            reason: reason.into(),
        }
    }

    pub fn ask(reason: impl Into<String>) -> Self {
        Self {
            risk_level: RiskLevel::Ask,
            reason: reason.into(),
        }
    }

    pub fn block(reason: impl Into<String>) -> Self {
        Self {
            risk_level: RiskLevel::Block,
            reason: reason.into(),
        }
    }
}

pub fn assess_tool_request(intent: &ToolIntent, context: &PolicyContext) -> PolicyDecision {
    let name = intent.name.as_str();

    if let Some(command) = &intent.command {
        if let Some(reason) = blocked_command_reason(command) {
            return PolicyDecision::block(reason);
        }
    }

    if let Some(path) = &intent.path {
        if !context.allow_outside_workspace
            && is_outside_workspace(path, context.workspace.as_deref())
        {
            return PolicyDecision::ask("Akses path di luar workspace butuh approval eksplisit.");
        }
    }

    match name {
        "list_dir" | "read_file" | "git_status" | "git_diff" => {
            PolicyDecision::safe("Operasi read-only di workspace.")
        }
        "run_command" => PolicyDecision::ask("Command dapat mengubah state sistem atau project."),
        "write_file" | "delete_file" => {
            PolicyDecision::ask("Perubahan file harus disetujui pengguna.")
        }
        "open_vscode" | "open_file_explorer" | "open_terminal" => {
            PolicyDecision::ask("Membuka aplikasi eksternal membutuhkan approval.")
        }
        _ => PolicyDecision::ask("Tool belum masuk daftar safe, jadi butuh approval."),
    }
}

fn blocked_command_reason(command: &str) -> Option<String> {
    let checks = [
        (r"(?i)\bformat\b", "Command format disk diblokir."),
        (
            r"(?i)\breg\s+(add|delete|import|save|restore)\b",
            "Edit registry diblokir.",
        ),
        (
            r"(?i)\b(system32|c:\\windows|windows\\system32)\b",
            "Akses destructive ke folder Windows diblokir.",
        ),
        (
            r"(?i)\bcredential\s+manager\b",
            "Akses credential Windows diblokir.",
        ),
        (
            r"(?i)\b(disable.*defender|DisableRealtimeMonitoring|Set-MpPreference)\b",
            "Menonaktifkan security tools diblokir.",
        ),
        (
            r"(?i)\b(cipher\s+/w|bcdedit|diskpart|takeown|icacls)\b",
            "Command administratif berisiko tinggi diblokir.",
        ),
        (
            r"(?i)(rm\s+-rf\s+/|del\s+/s\s+/q\s+c:\\)",
            "Command delete massal diblokir.",
        ),
    ];

    checks.iter().find_map(|(pattern, reason)| {
        Regex::new(pattern)
            .ok()
            .filter(|regex| regex.is_match(command))
            .map(|_| (*reason).to_string())
    })
}

fn is_outside_workspace(path: &Path, workspace: Option<&Path>) -> bool {
    let Some(workspace) = workspace else {
        return true;
    };

    let normalized_path = path.components().collect::<PathBuf>();
    let normalized_workspace = workspace.components().collect::<PathBuf>();
    !normalized_path.starts_with(normalized_workspace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_registry_commands() {
        let intent = ToolIntent {
            name: "run_command".into(),
            command: Some("reg delete HKCU\\Software\\Test".into()),
            path: None,
        };
        let decision = assess_tool_request(
            &intent,
            &PolicyContext {
                workspace: None,
                allow_outside_workspace: false,
            },
        );
        assert_eq!(decision.risk_level, RiskLevel::Block);
    }
}
