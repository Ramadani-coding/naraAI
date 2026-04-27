import type {
  ApprovalRequest,
  EventEnvelope,
  PromptRequest,
  StatusResponse,
  ToolOutput,
  WorkspaceInfo
} from "./types";

export const API_BASE = import.meta.env.VITE_NARA_API ?? "http://127.0.0.1:44731";
const WS_BASE = API_BASE.replace(/^http/, "ws");

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {})
    },
    ...init
  });

  if (!response.ok) {
    let message = response.statusText;
    try {
      const body = (await response.json()) as { error?: string };
      message = body.error ?? message;
    } catch {
      message = await response.text();
    }
    throw new Error(message);
  }

  return (await response.json()) as T;
}

export function getStatus() {
  return request<StatusResponse>("/api/status");
}

export function setWorkspace(path: string) {
  return request<WorkspaceInfo>("/api/workspace", {
    method: "POST",
    body: JSON.stringify({ path })
  });
}

export function sendPrompt(sessionId: string, payload: PromptRequest) {
  return request<{ accepted: boolean; session_id: string }>(`/api/sessions/${sessionId}/prompt`, {
    method: "POST",
    body: JSON.stringify(payload)
  });
}

export function requestCommand(command: string, reason?: string) {
  return request<ApprovalRequest>("/api/tools/command/request", {
    method: "POST",
    body: JSON.stringify({ command, reason })
  });
}

export function decideApproval(id: string, decision: "approved" | "rejected") {
  return request<{ status: string }>(`/api/approvals/${id}/decision`, {
    method: "POST",
    body: JSON.stringify({ decision })
  });
}

export function getGitStatus() {
  return request<ToolOutput>("/api/git/status");
}

export function getGitDiff() {
  return request<ToolOutput>("/api/git/diff");
}

export function startVoice() {
  return request<{ status: string }>("/api/voice/start", { method: "POST" });
}

export function stopVoice() {
  return request<{ status: string; transcript: string }>("/api/voice/stop", { method: "POST" });
}

export function openLogsTerminal() {
  return request<{ status: string; paths: string[] }>("/api/diagnostics/open-terminal", {
    method: "POST"
  });
}

export function connectEvents(
  onEvent: (event: EventEnvelope) => void,
  onOpen: () => void,
  onClose: () => void
) {
  const socket = new WebSocket(`${WS_BASE}/events`);
  let closed = false;
  let manuallyClosed = false;

  const handleOpen = () => {
    if (!manuallyClosed) {
      onOpen();
    }
  };

  const handleClose = () => {
    if (closed || manuallyClosed) {
      return;
    }

    closed = true;
    onClose();
  };

  const handleError = () => {
    if (!manuallyClosed && socket.readyState === WebSocket.OPEN) {
      socket.close();
    }
  };

  const handleMessage = (message: MessageEvent) => {
    try {
      onEvent(JSON.parse(message.data as string) as EventEnvelope);
    } catch {
      onEvent({
        id: crypto.randomUUID(),
        type: "event.parse_error",
        data: { raw: message.data },
        created_at: new Date().toISOString()
      });
    }
  };

  socket.addEventListener("open", handleOpen);
  socket.addEventListener("close", handleClose);
  socket.addEventListener("error", handleError);
  socket.addEventListener("message", handleMessage);

  return () => {
    manuallyClosed = true;
    socket.removeEventListener("open", handleOpen);
    socket.removeEventListener("close", handleClose);
    socket.removeEventListener("error", handleError);
    socket.removeEventListener("message", handleMessage);

    if (socket.readyState === WebSocket.OPEN) {
      socket.close(1000, "NARA HUD reconnect cleanup");
    }

    if (socket.readyState === WebSocket.CONNECTING) {
      socket.addEventListener(
        "open",
        () => {
          if (socket.readyState === WebSocket.OPEN) {
            socket.close(1000, "NARA HUD stale connection cleanup");
          }
        },
        { once: true }
      );
    }
  };
}
