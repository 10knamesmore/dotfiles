/** Current container positions, not expressions or durable object handles. An entry index follows dictionary order. */
export type ValuePath = Array<{ kind: "index" | "entry"; index: number }>;

/** Read only the selected view; offsets count entries except terminal `since`, which counts raw bytes. */
export type InspectRequest =
  | { view: "variables"; search?: string; definitions?: boolean; offset?: number }
  | { view: "value"; name: string; path: ValuePath; offset?: number }
  | { view: "terminals" }
  | { view: "terminal"; id: string; mode: "screen" | "raw"; since?: number };

/** A bounded preview of a known value, without running user-defined repr or properties. */
export interface ValueSummary {
  type: string;
  summary: string;
  expandable: boolean;
  size?: number;
  terminalId?: string;
}

export interface Variable extends ValueSummary {
  name: string;
  category: "data" | "module" | "function";
}

export interface ValueChild extends ValueSummary {
  label: string;
  selector: ValuePath[number];
}

/** An SDK handle remains listed after its child exits, until the handle is explicitly closed. */
export interface TerminalInfo {
  id: string;
  pid: number;
  rows: number;
  cols: number;
  status: { kind: "running" } | { kind: "exited"; code: number; signal: string | null };
  closed: boolean;
  reader_done: boolean;
  generation: number;
  hasError: boolean;
  variables: string[];
  /** Current process argv, not a saved launch command; unavailable after the OS process disappears. */
  command: string[] | null;

  /** Current OS process directory, independent of the worker's per-call cwd. */
  cwd: string | null;
}

export type CellColor =
  | { kind: "default" }
  | { kind: "indexed"; index: number }
  | { kind: "rgb"; red: number; green: number; blue: number };

export interface TerminalCell {
  text: string;
  wide: boolean;
  wide_continuation: boolean;
  foreground: CellColor;
  background: CellColor;
  bold: boolean;
  dim: boolean;
  italic: boolean;
  underline: boolean;
  inverse: boolean;
}

export type InspectData =
  | { view: "variables"; variables: Variable[]; total: number; offset: number; pageSize: number }
  | { view: "value"; name: string; path: ValuePath; value: ValueSummary; children: ValueChild[]; offset: number; total: number; pageSize: number }
  | { view: "terminals"; terminals: TerminalInfo[] }
  | { view: "terminal"; id: string; mode: "screen"; screen: {
      lines: string[];
      cells: TerminalCell[][];
      full_size: { rows: number; cols: number };
      rect: { effective: { x: number; y: number; width: number; height: number } };
      cursor: { x: number; y: number; visible: boolean };
      alternate_screen: boolean;
      generation: number;
      hasError: boolean;
    } }
  | { view: "terminal"; id: string; mode: "raw"; raw: {
      text: string;
      start: number;
      end: number;
      lostBytes: number;
      droppedBytes: number;
      truncated: boolean;
    } };

/** `busy` means another read is pending, not that Python has stopped. Successful times are Unix milliseconds. */
export type InspectResult =
  | { status: "ok"; workspaceId: string; sampledAt: number; data: InspectData }
  | { status: "unavailable" | "busy" | "not_found" | "invalid_request" | "inspection_failed"; workspaceId: string };

/** Current or last call only. Text output is a bounded tail, not a durable execution history. */
export interface ObserverExecution {
  callId: string;
  number: number;
  code: string;
  cwd: string;
  startedAt: number;
  finishedAt?: number;
  timeoutSeconds: number;
  outcome?: string;
  output: string;
  outputTruncated: boolean;
  outputUnavailable?: boolean;
}

/** Pi-side metadata remains available while Python inspection is delayed by the GIL. */
export interface ObserverStatus {
  sessionId: string;

  /** Changes whenever the namespace is replaced; browsers must discard earlier observations. */
  workspaceId: string;
  cwd: string;
  state: "not_started" | "starting" | "running" | "idle" | "stopping" | "exited";
  pid?: number;
  version?: string;
  /** Owned terminal handles, including exited children until close; not a running-process count. */
  terminalCount: number;

  execution?: ObserverExecution;
}
