use anyhow::Context;
use chrono::Utc;
use nara_protocol::EventEnvelope;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app: AppSection,
    pub daemon: DaemonSection,
    pub codex: CodexSection,
    pub voice: VoiceSection,
    pub security: SecuritySection,
    pub logs: LogsSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSection {
    pub name: String,
    pub theme: String,
    pub hud_opacity: f32,
    pub always_on_top: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonSection {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexSection {
    pub enabled: bool,
    pub executable: String,
    pub default_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSection {
    pub enabled: bool,
    pub input_mode: String,
    pub wake_word_enabled: bool,
    pub wake_word: String,
    pub stt_provider: String,
    pub tts_provider: String,
    pub speak_responses: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySection {
    pub approval_mode: String,
    pub allow_outside_workspace: bool,
    pub redact_secrets: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsSection {
    pub enabled: bool,
    pub retention_days: u16,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app: AppSection {
                name: "NARA".into(),
                theme: "dark".into(),
                hud_opacity: 0.94,
                always_on_top: false,
            },
            daemon: DaemonSection {
                host: "127.0.0.1".into(),
                port: 44731,
            },
            codex: CodexSection {
                enabled: true,
                executable: default_codex_executable().into(),
                default_mode: "cli".into(),
            },
            voice: VoiceSection {
                enabled: true,
                input_mode: "push_to_talk".into(),
                wake_word_enabled: false,
                wake_word: "nara".into(),
                stt_provider: "openai".into(),
                tts_provider: "windows".into(),
                speak_responses: true,
            },
            security: SecuritySection {
                approval_mode: "strict".into(),
                allow_outside_workspace: false,
                redact_secrets: true,
            },
            logs: LogsSection {
                enabled: true,
                retention_days: 14,
            },
        }
    }
}

fn default_codex_executable() -> &'static str {
    if cfg!(windows) {
        "codex.cmd"
    } else {
        "codex"
    }
}

pub fn app_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("NARA")
}

pub async fn load_or_create_config() -> anyhow::Result<AppConfig> {
    let dir = app_data_dir();
    tokio::fs::create_dir_all(&dir).await?;
    let path = dir.join("config.toml");

    if !path.exists() {
        let config = AppConfig::default();
        let body = toml::to_string_pretty(&config)?;
        tokio::fs::write(&path, body).await?;
        return Ok(config);
    }

    let body = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| format!("failed to read {}", path.display()))?;
    let config = toml::from_str(&body).context("failed to parse NARA config")?;
    Ok(config)
}

#[derive(Debug, Clone)]
pub struct EventLogger {
    log_dir: PathBuf,
    redact_secrets: bool,
}

impl EventLogger {
    pub async fn new(redact_secrets: bool) -> anyhow::Result<Self> {
        let log_dir = app_data_dir().join("logs");
        tokio::fs::create_dir_all(&log_dir).await?;
        Ok(Self {
            log_dir,
            redact_secrets,
        })
    }

    pub async fn append(&self, event: &EventEnvelope) -> anyhow::Result<()> {
        let file_name = format!("events-{}.jsonl", Utc::now().format("%Y-%m-%d"));
        let path = self.log_dir.join(file_name);
        let mut line = serde_json::to_string(event)?;
        if self.redact_secrets {
            line = redact_secrets(&line);
        }
        line.push('\n');

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        file.write_all(line.as_bytes()).await?;
        Ok(())
    }
}

pub fn redact_secrets(input: &str) -> String {
    let patterns = [
        r"sk-[A-Za-z0-9_\-]{20,}",
        r#"(?i)(api[_-]?key|token|password|secret)(\\?":\\?"|=)\s*[^,"\\\s]+"#,
    ];

    patterns.iter().fold(input.to_string(), |current, pattern| {
        Regex::new(pattern)
            .map(|regex| regex.replace_all(&current, "$1$2[REDACTED]").to_string())
            .unwrap_or(current)
    })
}
