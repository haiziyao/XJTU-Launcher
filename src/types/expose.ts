//! expose domain types.

export interface ExposeStatus {
  running: boolean;
  enabled: boolean;
  /** The port actually serving; may differ from the saved one after a fallback. */
  port: number;
  address: string;
  error: string | null;
}

/** How a tool touches data; the page badges each kind differently. */
export type ExposeAccess = 'read' | 'write' | 'update' | 'delete';

export interface ExposeTool {
  name: string;
  description: string;
  /** Short Chinese name shown instead of the raw tool name. */
  title: string;
  /** One line saying what the tool does. */
  summary: string;
  /** Section in the tool list. */
  category: string;
  access: ExposeAccess[];
}
