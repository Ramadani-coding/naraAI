import { open } from "@tauri-apps/plugin-dialog";
import { AnimatePresence, motion } from "framer-motion";
import {
  AlertTriangle,
  Bot,
  CheckCircle2,
  CircleStop,
  Clipboard,
  Code2,
  FileText,
  FolderOpen,
  GitBranch,
  Loader2,
  Mic,
  MicOff,
  Play,
  RefreshCw,
  Send,
  Settings,
  ShieldAlert,
  ShieldCheck,
  Terminal,
  Volume2,
  Wifi,
  WifiOff,
  X
} from "lucide-react";
import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import {
  connectEvents,
  decideApproval,
  getGitDiff,
  getGitStatus,
  getStatus,
  requestCommand,
  sendPrompt,
  setWorkspace,
  startVoice,
  stopVoice
} from "./api";
import { useSpeech } from "./hooks/useSpeech";
import type { ApprovalRequest, ChatMessage, EventEnvelope, TimelineItem, ToolOutput } from "./types";

const SESSION_ID = "default";

function newId() {
  return crypto.randomUUID();
}

function now() {
  return new Date().toISOString();
}

function formatTime(value: string) {
  return new Intl.DateTimeFormat("id-ID", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit"
  }).format(new Date(value));
}

function eventLabel(type: string) {
  const labels: Record<string, string> = {
    "daemon.started": "Daemon started",
    "workspace.selected": "Workspace selected",
    "prompt.received": "Prompt received",
    "codex.started": "Codex thinking",
    "codex.completed": "Codex completed",
    "codex.failed": "Codex failed",
    "codex.error": "Codex error",
    "approval.requested": "Approval requested",
    "approval.approved": "Approval approved",
    "approval.rejected": "Approval rejected",
    "command.running": "Command running",
    "command.completed": "Command completed",
    "command.error": "Command error",
    "git.status": "Git status",
    "git.diff": "Git diff",
    "voice.recording": "Voice recording",
    "voice.stopped": "Voice stopped"
  };
  return labels[type] ?? type;
}

function eventDetail(event: EventEnvelope) {
  const data = event.data as Record<string, unknown>;
  if (typeof data?.message === "string") return data.message;
  if (typeof data?.command === "string") return data.command;
  if (typeof data?.stdout === "string") return data.stdout.slice(0, 120);
  if (typeof data?.error === "string") return data.error;
  if (typeof data?.path === "string") return data.path;
  return "";
}

function outputText(output: ToolOutput | null) {
  if (!output) return "";
  return [output.stdout, output.stderr].filter(Boolean).join("\n").trim();
}

export function App() {
  const [status, setStatus] = useState<Awaited<ReturnType<typeof getStatus>> | null>(null);
  const [connected, setConnected] = useState(false);
  const [loadingStatus, setLoadingStatus] = useState(true);
  const [messages, setMessages] = useState<ChatMessage[]>([
    {
      id: newId(),
      role: "assistant",
      content: "NARA siap. Pilih workspace, lalu kirim prompt atau jalankan tool yang perlu approval.",
      createdAt: now()
    }
  ]);
  const [timeline, setTimeline] = useState<TimelineItem[]>([]);
  const [approvals, setApprovals] = useState<ApprovalRequest[]>([]);
  const [prompt, setPrompt] = useState("");
  const [command, setCommand] = useState("npm run dev");
  const [toolOutput, setToolOutput] = useState<ToolOutput | null>(null);
  const [busy, setBusy] = useState(false);
  const [speakResponses, setSpeakResponses] = useState(true);
  const [lastError, setLastError] = useState<string | null>(null);

  const speech = useSpeech();

  const refreshStatus = useCallback(async () => {
    try {
      const next = await getStatus();
      setStatus(next);
      setConnected(true);
      setLastError(null);
    } catch (error) {
      setConnected(false);
      setLastError(error instanceof Error ? error.message : "Daemon tidak terhubung.");
    } finally {
      setLoadingStatus(false);
    }
  }, []);

  const appendMessage = useCallback((message: Omit<ChatMessage, "id" | "createdAt">) => {
    setMessages((current) => [
      ...current,
      {
        ...message,
        id: newId(),
        createdAt: now()
      }
    ]);
  }, []);

  const appendTimeline = useCallback((event: EventEnvelope) => {
    setTimeline((current) => [
      {
        id: event.id,
        type: event.type,
        label: eventLabel(event.type),
        detail: eventDetail(event),
        createdAt: event.created_at
      },
      ...current
    ].slice(0, 24));
  }, []);

  const handleEvent = useCallback(
    (event: EventEnvelope) => {
      appendTimeline(event);

      if (event.type === "approval.requested") {
        const approval = event.data as ApprovalRequest;
        setApprovals((current) =>
          current.some((item) => item.id === approval.id) ? current : [approval, ...current]
        );
      }

      if (event.type === "codex.started") {
        setBusy(true);
      }

      if (event.type === "codex.completed" || event.type === "codex.failed") {
        const data = event.data as { stdout?: string; stderr?: string; exit_code?: number | null };
        const content = [data.stdout, data.stderr].filter(Boolean).join("\n").trim();
        appendMessage({
          role: event.type === "codex.completed" ? "assistant" : "system",
          content: content || `Codex selesai dengan exit code ${data.exit_code ?? "unknown"}.`
        });
        if (event.type === "codex.completed" && speakResponses) {
          speech.speak(content || "Codex selesai.");
        }
        setBusy(false);
      }

      if (event.type === "codex.error" || event.type === "command.error") {
        const data = event.data as { error?: string };
        appendMessage({
          role: "system",
          content: data.error ?? "Terjadi error."
        });
        setBusy(false);
      }

      if (event.type === "command.completed" || event.type === "git.status" || event.type === "git.diff") {
        const output = event.data as ToolOutput;
        setToolOutput(output);
      }

      if (event.type === "workspace.selected") {
        void refreshStatus();
      }
    },
    [appendMessage, appendTimeline, refreshStatus, speakResponses, speech.speak]
  );

  useEffect(() => {
    void refreshStatus();
    const interval = window.setInterval(refreshStatus, 8000);
    return () => window.clearInterval(interval);
  }, [refreshStatus]);

  useEffect(() => {
    let reconnect: number | null = null;
    let cleanup: () => void = () => undefined;
    let disposed = false;

    const connect = () => {
      if (disposed) {
        return;
      }

      cleanup = connectEvents(
        handleEvent,
        () => {
          if (disposed) {
            return;
          }

          if (reconnect) {
            window.clearTimeout(reconnect);
            reconnect = null;
          }
          setConnected(true);
        },
        () => {
          if (disposed) {
            return;
          }

          setConnected(false);
          if (!reconnect) {
            reconnect = window.setTimeout(() => {
              reconnect = null;
              connect();
            }, 2500);
          }
        }
      );
    };

    connect();
    return () => {
      disposed = true;
      cleanup();
      if (reconnect) window.clearTimeout(reconnect);
    };
  }, [handleEvent]);

  const workspaceName = status?.workspace.name ?? "Belum dipilih";
  const workspacePath = status?.workspace.path ?? "Pilih folder project aktif";
  const codexReady = Boolean(status?.codex.available);
  const canSend = prompt.trim().length > 0 && connected && Boolean(status?.workspace.path);

  const sendTextPrompt = useCallback(
    async (input: string, inputType: "text" | "voice" = "text") => {
      const message = input.trim();
      if (!message) return;

      appendMessage({ role: "user", content: message, inputType });
      setPrompt("");
      setBusy(true);

      try {
        await sendPrompt(SESSION_ID, { input_type: inputType, message });
      } catch (error) {
        const text = error instanceof Error ? error.message : "Gagal mengirim prompt.";
        appendMessage({ role: "system", content: text });
        setBusy(false);
      }
    },
    [appendMessage]
  );

  const onSubmitPrompt = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (canSend) {
      await sendTextPrompt(prompt);
    }
  };

  const selectWorkspace = async () => {
    let selectedPath: string | null = null;

    try {
      const selected = await open({ directory: true, multiple: false, title: "Select NARA workspace" });
      if (typeof selected === "string") selectedPath = selected;
    } catch {
      selectedPath = window.prompt("Masukkan path workspace di Windows")?.trim() ?? null;
    }

    if (!selectedPath) return;

    try {
      await setWorkspace(selectedPath);
      await refreshStatus();
    } catch (error) {
      setLastError(error instanceof Error ? error.message : "Workspace gagal disimpan.");
    }
  };

  const submitCommand = async () => {
    if (!command.trim()) return;
    try {
      const approval = await requestCommand(command.trim(), "User meminta NARA menjalankan command.");
      setApprovals((current) =>
        current.some((item) => item.id === approval.id) ? current : [approval, ...current]
      );
    } catch (error) {
      appendMessage({
        role: "system",
        content: error instanceof Error ? error.message : "Command ditolak policy."
      });
    }
  };

  const handleApproval = async (approval: ApprovalRequest, decision: "approved" | "rejected") => {
    setApprovals((current) => current.filter((item) => item.id !== approval.id));
    try {
      await decideApproval(approval.id, decision);
    } catch (error) {
      appendMessage({
        role: "system",
        content: error instanceof Error ? error.message : "Approval gagal diproses."
      });
    }
  };

  const runGitStatus = async () => {
    try {
      setToolOutput(await getGitStatus());
    } catch (error) {
      appendMessage({
        role: "system",
        content: error instanceof Error ? error.message : "Gagal membaca git status."
      });
    }
  };

  const runGitDiff = async () => {
    try {
      setToolOutput(await getGitDiff());
    } catch (error) {
      appendMessage({
        role: "system",
        content: error instanceof Error ? error.message : "Gagal membaca git diff."
      });
    }
  };

  const startListening = async () => {
    try {
      await startVoice();
    } catch {
      // The HUD voice fallback can still work without daemon voice endpoints.
    }
    speech.start((text) => {
      void stopVoice().catch(() => undefined);
      void sendTextPrompt(text, "voice");
    });
  };

  const copyOutput = async () => {
    const text = outputText(toolOutput);
    if (text) await navigator.clipboard.writeText(text);
  };

  const statusTone = useMemo(() => {
    if (!connected) return "danger";
    if (!codexReady) return "warning";
    return "good";
  }, [codexReady, connected]);

  return (
    <main className="app-shell">
      <header className="topbar">
        <div className="brand">
          <div className="brand-mark">
            <Bot size={24} />
          </div>
          <div>
            <h1>NARA</h1>
            <p>Neural Agentic Runtime Assistant</p>
          </div>
        </div>

        <div className="topbar-actions">
          <StatusBadge tone={connected ? "good" : "danger"} icon={connected ? <Wifi size={15} /> : <WifiOff size={15} />}>
            {connected ? "Daemon online" : "Daemon offline"}
          </StatusBadge>
          <StatusBadge tone={codexReady ? "good" : "warning"} icon={<Code2 size={15} />}>
            {codexReady ? status?.codex.version ?? "Codex ready" : "Codex missing"}
          </StatusBadge>
          <button className="icon-button" onClick={refreshStatus} title="Refresh status" type="button">
            {loadingStatus ? <Loader2 className="spin" size={17} /> : <RefreshCw size={17} />}
          </button>
        </div>
      </header>

      <section className="workspace-grid">
        <aside className="sidebar panel">
          <section className="workspace-block">
            <div className="section-title">
              <FolderOpen size={16} />
              <span>Workspace</span>
            </div>
            <h2>{workspaceName}</h2>
            <p title={workspacePath}>{workspacePath}</p>
            <button className="primary-button" onClick={selectWorkspace} type="button">
              <FolderOpen size={16} />
              Select Workspace
            </button>
          </section>

          <section className="system-block">
            <div className="section-title">
              <Settings size={16} />
              <span>System</span>
            </div>
            <dl className="system-list">
              <div>
                <dt>Platform</dt>
                <dd>{status?.platform ?? "unknown"}</dd>
              </div>
              <div>
                <dt>Voice</dt>
                <dd>{speech.supported ? "push-to-talk ready" : "speech API unavailable"}</dd>
              </div>
              <div>
                <dt>Pending</dt>
                <dd>{approvals.length || status?.pending_approvals || 0} approval</dd>
              </div>
            </dl>
          </section>

          <section className="voice-block">
            <div className="section-title">
              {speech.listening ? <Mic size={16} /> : <MicOff size={16} />}
              <span>Voice</span>
            </div>
            <div className={`voice-orb ${speech.listening ? "active" : busy ? "thinking" : statusTone}`}>
              <span />
            </div>
            <p className="muted">
              {speech.listening
                ? "Mendengarkan..."
                : speech.supported
                  ? "Tekan mic untuk bicara"
                  : "Speech recognition belum tersedia di runtime ini"}
            </p>
            {speech.transcript ? <p className="transcript">{speech.transcript}</p> : null}
            <div className="button-row">
              <button
                className="icon-text-button"
                disabled={!speech.supported || !connected}
                onClick={speech.listening ? speech.stop : startListening}
                type="button"
              >
                {speech.listening ? <CircleStop size={16} /> : <Mic size={16} />}
                {speech.listening ? "Stop" : "Mic"}
              </button>
              <button
                className="icon-button"
                disabled={!speech.speechSupported}
                onClick={() => setSpeakResponses((value) => !value)}
                title={speakResponses ? "Voice response on" : "Voice response off"}
                type="button"
              >
                <Volume2 size={16} />
              </button>
              <button className="icon-button" onClick={speech.stopSpeaking} title="Stop speaking" type="button">
                <CircleStop size={16} />
              </button>
            </div>
          </section>
        </aside>

        <section className="conversation panel">
          <div className="conversation-header">
            <div>
              <div className="section-title">
                <Bot size={16} />
                <span>Chat</span>
              </div>
              <h2>Assistant Console</h2>
            </div>
            {busy ? (
              <span className="busy-chip">
                <Loader2 className="spin" size={15} />
                thinking
              </span>
            ) : null}
          </div>

          <div className="message-list">
            <AnimatePresence initial={false}>
              {messages.map((message) => (
                <motion.article
                  animate={{ opacity: 1, y: 0 }}
                  className={`message ${message.role}`}
                  exit={{ opacity: 0, y: 6 }}
                  initial={{ opacity: 0, y: 8 }}
                  key={message.id}
                >
                  <div className="message-meta">
                    <span>{message.role === "user" ? "You" : message.role === "assistant" ? "NARA" : "System"}</span>
                    <time>{formatTime(message.createdAt)}</time>
                  </div>
                  <pre>{message.content}</pre>
                </motion.article>
              ))}
            </AnimatePresence>
          </div>

          <form className="prompt-box" onSubmit={onSubmitPrompt}>
            <textarea
              onChange={(event) => setPrompt(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.shiftKey) {
                  event.preventDefault();
                  if (canSend) void sendTextPrompt(prompt);
                }
              }}
              placeholder="Tulis instruksi untuk Codex..."
              value={prompt}
            />
            <button className="send-button" disabled={!canSend || busy} type="submit" title="Send prompt">
              {busy ? <Loader2 className="spin" size={18} /> : <Send size={18} />}
            </button>
          </form>
          {lastError ? <p className="error-line">{lastError}</p> : null}
        </section>

        <aside className="inspector panel">
          <section className="approval-section">
            <div className="section-title">
              <ShieldAlert size={16} />
              <span>Approvals</span>
            </div>
            <div className="approval-list">
              {approvals.length === 0 ? (
                <div className="empty-state">
                  <ShieldCheck size={20} />
                  <span>Tidak ada approval pending</span>
                </div>
              ) : (
                approvals.map((approval) => (
                  <article className="approval-card" key={approval.id}>
                    <div className="approval-card-head">
                      <span>{approval.tool}</span>
                      <RiskBadge level={approval.risk_level} />
                    </div>
                    <pre>{approval.command}</pre>
                    <p>{approval.reason}</p>
                    <small>{approval.cwd}</small>
                    <div className="button-row">
                      <button className="approve-button" onClick={() => handleApproval(approval, "approved")} type="button">
                        <CheckCircle2 size={15} />
                        Approve
                      </button>
                      <button className="reject-button" onClick={() => handleApproval(approval, "rejected")} type="button">
                        <X size={15} />
                        Reject
                      </button>
                    </div>
                  </article>
                ))
              )}
            </div>
          </section>

          <section className="tool-section">
            <div className="section-title">
              <Terminal size={16} />
              <span>Command</span>
            </div>
            <div className="command-input">
              <input value={command} onChange={(event) => setCommand(event.target.value)} />
              <button className="icon-button" onClick={submitCommand} disabled={!connected || !status?.workspace.path} title="Request command approval" type="button">
                <Play size={16} />
              </button>
            </div>
            <div className="button-row">
              <button className="icon-text-button" onClick={runGitStatus} disabled={!connected || !status?.workspace.path} type="button">
                <GitBranch size={16} />
                Status
              </button>
              <button className="icon-text-button" onClick={runGitDiff} disabled={!connected || !status?.workspace.path} type="button">
                <FileText size={16} />
                Diff
              </button>
              <button className="icon-button" onClick={copyOutput} disabled={!toolOutput} title="Copy output" type="button">
                <Clipboard size={16} />
              </button>
            </div>
            <pre className="output-view">{outputText(toolOutput) || "Output command dan git akan tampil di sini."}</pre>
          </section>

          <section className="timeline-section">
            <div className="section-title">
              <AlertTriangle size={16} />
              <span>Timeline</span>
            </div>
            <div className="timeline">
              {timeline.length === 0 ? (
                <div className="empty-state">
                  <Wifi size={19} />
                  <span>Menunggu event daemon</span>
                </div>
              ) : (
                timeline.map((item) => (
                  <article className="timeline-item" key={item.id}>
                    <time>{formatTime(item.createdAt)}</time>
                    <strong>{item.label}</strong>
                    {item.detail ? <p>{item.detail}</p> : null}
                  </article>
                ))
              )}
            </div>
          </section>
        </aside>
      </section>
    </main>
  );
}

function StatusBadge({
  tone,
  icon,
  children
}: {
  tone: "good" | "warning" | "danger";
  icon: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <span className={`status-badge ${tone}`}>
      {icon}
      {children}
    </span>
  );
}

function RiskBadge({ level }: { level: "safe" | "ask" | "block" }) {
  const icon = level === "safe" ? <ShieldCheck size={14} /> : <ShieldAlert size={14} />;
  return (
    <span className={`risk-badge ${level}`}>
      {icon}
      {level}
    </span>
  );
}
