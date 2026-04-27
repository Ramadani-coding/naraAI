export type VoiceState =
  | "idle"
  | "listening_for_wake_word"
  | "wake_word_detected"
  | "recording_command"
  | "transcribing"
  | "thinking"
  | "speaking"
  | "waiting_approval"
  | "error";

export type RiskLevel = "safe" | "ask" | "block";

export interface WorkspaceInfo {
  path: string | null;
  name: string | null;
}

export interface StatusResponse {
  app: string;
  daemon: string;
  platform: string;
  codex: {
    available: boolean;
    path: string | null;
    version: string | null;
  };
  voice: {
    mic_available: boolean;
    tts_available: boolean;
    wake_word_enabled: boolean;
    state: VoiceState;
  };
  workspace: WorkspaceInfo;
  pending_approvals: number;
}

export interface PromptRequest {
  input_type: "text" | "voice";
  message: string;
}

export interface ApprovalRequest {
  id: string;
  tool: string;
  command: string | null;
  cwd: string | null;
  risk_level: RiskLevel;
  reason: string;
  created_at: string;
}

export interface ToolOutput {
  id: string;
  tool_name: string;
  status: "pending_approval" | "running" | "completed" | "rejected" | "blocked" | "failed";
  stdout: string;
  stderr: string;
  exit_code: number | null;
  started_at: string;
  finished_at: string;
}

export interface EventEnvelope<T = unknown> {
  id: string;
  type: string;
  data: T;
  created_at: string;
}

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  createdAt: string;
  inputType?: "text" | "voice";
}

export interface TimelineItem {
  id: string;
  type: string;
  label: string;
  detail: string;
  createdAt: string;
}
