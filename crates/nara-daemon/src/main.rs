use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use nara_codex::CodexBridge;
use nara_policy::{assess_tool_request, PolicyContext, ToolIntent};
use nara_protocol::{
    ApprovalDecision, ApprovalDecisionRequest, ApprovalRequest, CodexStatus, EventEnvelope,
    PromptAcceptedResponse, PromptRequest, RiskLevel, RunCommandRequest, SetWorkspaceRequest,
    StatusResponse, ToolRunStatus, VoiceState, VoiceStatus, WorkspaceInfo,
};
use nara_store::{load_or_create_config, AppConfig, EventLogger};
use nara_tools::ToolRuntime;
use serde::Serialize;
use serde_json::json;
use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    config: AppConfig,
    workspace: Arc<RwLock<Option<PathBuf>>>,
    approvals: Arc<RwLock<HashMap<String, PendingApproval>>>,
    event_tx: broadcast::Sender<EventEnvelope>,
    logger: EventLogger,
    codex: CodexBridge,
}

#[derive(Debug, Clone)]
struct PendingApproval {
    id: String,
    command: String,
    reason: String,
    cwd: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("nara_daemon=debug,tower_http=info")
        .init();

    let config = load_or_create_config().await?;
    let logger = EventLogger::new(config.security.redact_secrets).await?;
    let (event_tx, _) = broadcast::channel(256);
    let codex = CodexBridge::new(config.codex.executable.clone());

    let state = AppState {
        config: config.clone(),
        workspace: Arc::new(RwLock::new(None)),
        approvals: Arc::new(RwLock::new(HashMap::new())),
        event_tx,
        logger,
        codex,
    };

    emit(&state, "daemon.started", json!({ "port": config.daemon.port })).await;

    let app = Router::new()
        .route("/api/status", get(status))
        .route("/api/workspace", post(set_workspace))
        .route("/api/sessions/{id}/prompt", post(prompt))
        .route("/api/tools/command/request", post(request_command))
        .route("/api/approvals/{id}/decision", post(decide_approval))
        .route("/api/git/status", get(git_status))
        .route("/api/git/diff", get(git_diff))
        .route("/api/voice/start", post(voice_start))
        .route("/api/voice/stop", post(voice_stop))
        .route("/events", get(events_ws))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", config.daemon.host, config.daemon.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("NARA daemon listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn status(State(state): State<AppState>) -> Json<StatusResponse> {
    let codex = state.codex.check().await;
    let workspace = state.workspace.read().await.clone();
    let pending_approvals = state.approvals.read().await.len();

    Json(StatusResponse {
        app: state.config.app.name.clone(),
        daemon: "running".into(),
        platform: std::env::consts::OS.into(),
        codex: CodexStatus {
            available: codex.available,
            path: codex.path,
            version: codex.version,
        },
        voice: VoiceStatus {
            mic_available: true,
            tts_available: true,
            wake_word_enabled: state.config.voice.wake_word_enabled,
            state: VoiceState::Idle,
        },
        workspace: WorkspaceInfo {
            name: workspace
                .as_ref()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().to_string()),
            path: workspace.map(|path| path.display().to_string()),
        },
        pending_approvals,
    })
}

async fn set_workspace(
    State(state): State<AppState>,
    Json(request): Json<SetWorkspaceRequest>,
) -> Result<Json<WorkspaceInfo>, ApiError> {
    let path = PathBuf::from(request.path);
    if !path.exists() || !path.is_dir() {
        return Err(ApiError::bad_request("Workspace path tidak valid."));
    }

    *state.workspace.write().await = Some(path.clone());
    let workspace = WorkspaceInfo {
        name: path.file_name().map(|name| name.to_string_lossy().to_string()),
        path: Some(path.display().to_string()),
    };
    emit(&state, "workspace.selected", &workspace).await;
    Ok(Json(workspace))
}

async fn prompt(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(request): Json<PromptRequest>,
) -> Result<Json<PromptAcceptedResponse>, ApiError> {
    let workspace = require_workspace(&state).await?;
    let input_type = request.input_type.clone();
    let message = request.message.clone();

    emit(
        &state,
        "prompt.received",
        json!({ "session_id": &session_id, "input_type": &input_type, "message": &message }),
    )
    .await;

    let state_for_task = state.clone();
    let session_id_for_task = session_id.clone();
    tokio::spawn(async move {
        emit(
            &state_for_task,
            "codex.started",
            json!({ "session_id": session_id_for_task }),
        )
        .await;

        match state_for_task.codex.run_prompt(&workspace, &message).await {
            Ok(result) if result.exit_code == Some(0) => {
                emit(
                    &state_for_task,
                    "codex.completed",
                    json!({
                        "session_id": session_id_for_task,
                        "stdout": result.stdout,
                        "stderr": result.stderr,
                        "exit_code": result.exit_code
                    }),
                )
                .await;
            }
            Ok(result) => {
                emit(
                    &state_for_task,
                    "codex.failed",
                    json!({
                        "session_id": session_id_for_task,
                        "stdout": result.stdout,
                        "stderr": result.stderr,
                        "exit_code": result.exit_code
                    }),
                )
                .await;
            }
            Err(err) => {
                emit(
                    &state_for_task,
                    "codex.error",
                    json!({ "session_id": session_id_for_task, "error": err.to_string() }),
                )
                .await;
            }
        }
    });

    Ok(Json(PromptAcceptedResponse {
        session_id,
        accepted: true,
    }))
}

async fn request_command(
    State(state): State<AppState>,
    Json(request): Json<RunCommandRequest>,
) -> Result<Json<ApprovalRequest>, ApiError> {
    let workspace = require_workspace(&state).await?;
    let intent = ToolIntent {
        name: "run_command".into(),
        command: Some(request.command.clone()),
        path: Some(workspace.clone()),
    };
    let decision = assess_tool_request(
        &intent,
        &PolicyContext {
            workspace: Some(workspace.clone()),
            allow_outside_workspace: state.config.security.allow_outside_workspace,
        },
    );

    if decision.risk_level == RiskLevel::Block {
        emit(
            &state,
            "tool.blocked",
            json!({ "tool": "run_command", "command": request.command, "reason": decision.reason }),
        )
        .await;
        return Err(ApiError::forbidden(decision.reason));
    }

    let id = Uuid::new_v4().to_string();
    let approval = ApprovalRequest {
        id: id.clone(),
        tool: "run_command".into(),
        command: Some(request.command.clone()),
        cwd: Some(workspace.display().to_string()),
        risk_level: decision.risk_level,
        reason: request.reason.unwrap_or(decision.reason),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    state.approvals.write().await.insert(
        id.clone(),
        PendingApproval {
            id,
            command: request.command,
            reason: approval.reason.clone(),
            cwd: workspace,
        },
    );

    emit(&state, "approval.requested", &approval).await;
    Ok(Json(approval))
}

async fn decide_approval(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<ApprovalDecisionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let approval = state
        .approvals
        .write()
        .await
        .remove(&id)
        .ok_or_else(|| ApiError::not_found("Approval tidak ditemukan."))?;

    match request.decision {
        ApprovalDecision::Rejected => {
            emit(
                &state,
                "approval.rejected",
                json!({ "id": approval.id, "reason": approval.reason }),
            )
            .await;
            Ok(Json(json!({ "status": ToolRunStatus::Rejected })))
        }
        ApprovalDecision::Approved => {
            emit(&state, "approval.approved", json!({ "id": approval.id })).await;
            let runtime = ToolRuntime::new(approval.cwd);
            let state_for_task = state.clone();
            let command = approval.command.clone();

            tokio::spawn(async move {
                emit(
                    &state_for_task,
                    "command.running",
                    json!({ "command": command }),
                )
                .await;

                match runtime.run_command(&command).await {
                    Ok(output) => emit(&state_for_task, "command.completed", &output).await,
                    Err(err) => {
                        error!("command failed: {err}");
                        emit(
                            &state_for_task,
                            "command.error",
                            json!({ "command": command, "error": err.to_string() }),
                        )
                        .await;
                    }
                }
            });

            Ok(Json(json!({ "status": ToolRunStatus::Running })))
        }
    }
}

async fn git_status(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let workspace = require_workspace(&state).await?;
    let output = ToolRuntime::new(workspace)
        .git_status()
        .await
        .map_err(ApiError::internal)?;
    emit(&state, "git.status", &output).await;
    Ok(Json(json!(output)))
}

async fn git_diff(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let workspace = require_workspace(&state).await?;
    let output = ToolRuntime::new(workspace)
        .git_diff_full()
        .await
        .map_err(ApiError::internal)?;
    emit(&state, "git.diff", &output).await;
    Ok(Json(json!(output)))
}

async fn voice_start(State(state): State<AppState>) -> Json<serde_json::Value> {
    emit(&state, "voice.recording", json!({})).await;
    Json(json!({ "status": "recording" }))
}

async fn voice_stop(State(state): State<AppState>) -> Json<serde_json::Value> {
    emit(&state, "voice.stopped", json!({})).await;
    Json(json!({ "status": "stopped", "transcript": "" }))
}

async fn events_ws(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut events = state.event_tx.subscribe();

    let send_task = tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            let Ok(text) = serde_json::to_string(&event) else {
                continue;
            };
            if sender.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(message)) = receiver.next().await {
        if matches!(message, Message::Close(_)) {
            break;
        }
    }

    send_task.abort();
}

async fn require_workspace(state: &AppState) -> Result<PathBuf, ApiError> {
    state
        .workspace
        .read()
        .await
        .clone()
        .ok_or_else(|| ApiError::bad_request("Pilih workspace dulu."))
}

async fn emit(state: &AppState, event_type: &str, data: impl Serialize) {
    let event = EventEnvelope::new(event_type, data);
    let _ = state.event_tx.send(event.clone());
    if let Err(err) = state.logger.append(&event).await {
        error!("failed to append event log: {err}");
    }
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn internal(error: anyhow::Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({
                "error": self.message
            })),
        )
            .into_response()
    }
}
