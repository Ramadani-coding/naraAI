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

export function connectEvents(
  onEvent: (event: EventEnvelope) => void,
  onOpen: () => void,
  onClose: () => void
) {
  const socket = new WebSocket(`${WS_BASE}/events`);

  socket.addEventListener("open", onOpen);
  socket.addEventListener("close", onClose);
  socket.addEventListener("error", onClose);
  socket.addEventListener("message", (message) => {
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
  });

  return () => socket.close();
}
