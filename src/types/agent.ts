//! agent domain types.

/** MCP transport as stored in ~/.xjtu-launcher/mcp.json. */
export type McpTransport =
  | { type: 'stdio'; command: string; args?: string[]; env?: Record<string, string> }
  | { type: 'http'; url: string; headers?: Record<string, string> };
export interface McpServer {
  id: string;
  name: string;
  enabled: boolean;
  kind: 'stdio' | 'http';
  target: string;
  /** Names only; values never leave Rust. */
  secrets: string[];
  /** Keys kept, values blanked; blank values keep the stored secret when saving. */
  transport: McpTransport;
}
export interface McpTool {
  name: string;
  description: string;
  inputSchema: unknown;
}
export interface AgentToolCall {
  id: string;
  name: string;
  arguments: string;
}
export interface AgentMessage {
  role: 'user' | 'assistant' | 'tool';
  content: string;
  toolCalls?: AgentToolCall[];
  toolCallId?: string;
  name?: string;
  error?: boolean;
}
export interface AgentConversation {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
  roleId: string | null;
  pinned: boolean;
  workspaceId?: string | null;
  messages: AgentMessage[];
}
export interface AgentSummary {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
  pinned: boolean;
  roleId: string | null;
  workspaceId?: string | null;
  messages: number;
}
/** Named group of conversations, stored beside them. */
export interface AgentWorkspace {
  id: string;
  name: string;
  createdAt: number;
}
export interface AgentRole {
  id: string;
  name: string;
  description: string;
  instructions: string;
}
export type AgentSkill = AgentRole;
export interface AgentTask {
  id: string;
  name: string;
  description: string;
  prompt: string;
  roleId: string | null;
  skillIds: string[];
}
export interface AgentPresets {
  version: number;
  roles: AgentRole[];
  skills: AgentSkill[];
  tasks: AgentTask[];
}
export interface AgentMemory {
  id: string;
  content: string;
  createdAt: number;
}
export type AgentPresetKind = 'role' | 'skill' | 'task';
export type AgentPresetInput =
  | { kind: 'role'; id?: string | null; name: string; description: string; instructions: string }
  | { kind: 'skill'; id?: string | null; name: string; description: string; instructions: string }
  | {
      kind: 'task';
      id?: string | null;
      name: string;
      description: string;
      prompt: string;
      roleId: string | null;
      skillIds: string[];
    };
export interface AgentToolInfo {
  name: string;
  description: string;
  source: 'campus' | 'mcp';
}
export type AgentSearchReason =
  'ready' | 'plugin-disabled' | 'disabled' | 'no-model' | 'unreachable';
export interface AgentSearchStatus {
  available: boolean;
  reason: AgentSearchReason;
}
export interface AgentSearchAnswer {
  answer: string;
}
/** 文件预览（SPEC §4.4）：图片和文本由后端内联，PDF 只带预览 id，交给 pdf.js。 */
export type FilePreview =
  | { kind: 'image'; name: string; mime: string; size: number; dataUrl: string }
  | { kind: 'text'; name: string; mime: string; size: number; text: string; truncated: boolean }
  | { kind: 'pdf'; name: string; size: number; previewId: string }
  | { kind: 'unsupported'; name: string; size: number | null };

/** 写操作预览（SPEC §4.5）：prepare 之后、commit 之前展示给用户的内容。 */
export interface WritePreview {
  id: string;
  kind: SchoolWrite['kind'];
  accountId: string;
  expiresInSeconds: number;
  title: string;
  summary: { label: string; value: string }[];
  files?: { name: string; size: number }[];
  warnings: string[];
  irreversible: boolean;
  captcha?: { image: string; kind: 'slider' | 'text' };
}

export type SchoolWrite =
  | { kind: 'lmsHomework'; activity: number; comment: string; files: string[]; draft: boolean }
  | { kind: 'lmsRetract'; submission: number }
  | { kind: 'libraryBook'; area: string; seat: string }
  | { kind: 'libraryAction'; booking: string; action: 'cancel' | 'checkIn' | 'leave' | 'return' }
  | { kind: 'venueBook'; venue: number; date: string; stocks: number[] }
  | { kind: 'venueCancel'; order: string };

export interface WriteOutcome {
  status: 'done' | 'rejected' | 'unknown';
  message: string;
  detail?: Record<string, unknown>;
}

export type AgentEvent =
  | { type: 'delta'; requestId: string; text: string }
  | {
      type: 'toolStart';
      requestId: string;
      callId: string;
      name: string;
      arguments: string;
    }
  | { type: 'toolEnd'; requestId: string; callId: string; ok: boolean; content: string }
  | { type: 'usage'; requestId: string; inputTokens: number; outputTokens: number }
  | { type: 'done'; requestId: string; conversationId: string }
  | { type: 'error'; requestId: string; message: string }
  | { type: 'cancelled'; requestId: string };
