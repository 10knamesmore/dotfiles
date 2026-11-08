import type { Edit, Selection, State } from "./types.js";

export type { Edit, State, Project } from "./types.js";

export interface ConnectionSnapshot {
  state: State | null;
  pending: number;
  connected: boolean;
  error: string | null;
  failed: boolean;
}

interface EditRequest {
  edits: Edit[];
  snapshot: boolean;
}

export function applyEdits(state: State, edits: Edit[]): State {
  let next = state;
  for (const edit of edits) {
    switch (edit.type) {
      case "text":
      case "style": {
        const previous = next.design.elements[edit.element] ?? {};
        const element = { ...previous };
        if (edit.type === "text") element.text = edit.text;
        else {
          element.style = { ...previous.style };
          for (const [key, value] of Object.entries(edit.style)) {
            if (value === null) delete element.style[key];
            else element.style[key] = value;
          }
        }
        next = { ...next, design: { ...next.design, elements: { ...next.design.elements, [edit.element]: element } } };
        break;
      }
      case "parameter": {
        const parameter = next.design.parameters[edit.name];
        if (parameter) next = { ...next, design: { ...next.design, parameters: {
          ...next.design.parameters, [edit.name]: { ...parameter, value: edit.value },
        } } };
        break;
      }
      case "artboard":
        next = { ...next, workspace: { ...next.workspace, artboards:
          next.workspace.artboards.some(item => item.id === edit.artboard.id)
            ? next.workspace.artboards.map(item => item.id === edit.artboard.id ? edit.artboard : item)
            : [...next.workspace.artboards, edit.artboard],
        } };
        break;
      case "remove_artboard":
        next = { ...next, workspace: { ...next.workspace, artboards: next.workspace.artboards.filter(item => item.id !== edit.id) } };
        break;
      case "comment":
        next = { ...next, workspace: { ...next.workspace, comments:
          next.workspace.comments.some(item => item.id === edit.comment.id)
            ? next.workspace.comments.map(item => item.id === edit.comment.id ? edit.comment : item)
            : [...next.workspace.comments, edit.comment],
        } };
        break;
      case "remove_comment":
        next = { ...next, workspace: { ...next.workspace, comments: next.workspace.comments.filter(item => item.id !== edit.id) } };
    }
  }
  return next;
}

export class DesignConnection {
  readonly endpoint: string;
  private canonical: State | null = null;
  private queue: EditRequest[] = [];
  private listeners = new Set<() => void>();
  private events: EventSource | null = null;
  private flushing = false;
  private failed = false;
  private error: string | null = null;
  private connected = false;
  private snapshot: ConnectionSnapshot = { state: null, pending: 0, connected: false, error: null, failed: false };

  constructor(endpoint: string) {
    this.endpoint = endpoint.replace(/\/$/, "");
  }

  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => { this.listeners.delete(listener); };
  };

  getSnapshot = () => this.snapshot;

  private publish() {
    const state = this.canonical && this.queue.reduce((state, request) => applyEdits(state, request.edits), this.canonical);
    this.snapshot = { state, pending: this.queue.length, connected: this.connected, error: this.error, failed: this.failed };
    for (const listener of this.listeners) listener();
  }

  async request<T>(path: string, body?: unknown): Promise<T> {
    const response = await fetch(`${this.endpoint}${path}`, body === undefined ? undefined : {
      method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body),
    });
    const content = await response.text();
    let data: unknown;
    try { data = content ? JSON.parse(content) : null; }
    catch { throw new Error(`Service returned invalid JSON (${response.status})`); }
    if (!response.ok) throw new Error(
      data && typeof data === "object" && "error" in data ? String(data.error) : `Request failed (${response.status})`,
    );
    return data as T;
  }

  private receive(state: State) {
    if (this.canonical && this.canonical.project?.id === state.project?.id && state.revision < this.canonical.revision) return;
    this.canonical = state;
    this.publish();
    void this.flush();
  }

  start() {
    this.events = new EventSource(`${this.endpoint}/api/events`);
    this.events.onopen = () => { this.connected = true; this.publish(); };
    this.events.addEventListener("state", event => {
      try { this.receive(JSON.parse((event as MessageEvent<string>).data) as State); }
      catch (error) { this.report(error); }
    });
    this.events.addEventListener("error", event => {
      const data = (event as MessageEvent<string>).data;
      if (data) {
        try { this.report(new Error((JSON.parse(data) as { error: string }).error)); }
        catch (error) { this.report(error); }
      } else {
        this.connected = false;
        this.publish();
      }
    });
    void this.request<State>("/api/state").then(state => this.receive(state)).catch(error => this.report(error));
    return () => { this.events?.close(); this.events = null; };
  }

  report(error: unknown) {
    this.error = error instanceof Error ? error.message : String(error);
    this.publish();
  }

  dismissError = () => { this.error = null; this.publish(); };

  edit = (edits: Edit[], options: { snapshot?: boolean } = {}) => {
    if (!edits.length) return;
    this.queue.push({ edits, snapshot: options.snapshot ?? true });
    this.publish();
    void this.flush();
  };

  /** Finalize a continuous gesture after all of its saved intermediate values. */
  finishEdit = () => {
    this.queue.push({ edits: [], snapshot: true });
    this.publish();
    void this.flush();
  };

  private async flush() {
    if (!this.canonical || this.flushing || this.failed) return;
    this.flushing = true;
    try {
      while (this.queue.length) {
        const state = await this.request<State>("/api/edit", this.queue[0]);
        this.queue.shift();
        this.receive(state);
      }
    } catch (error) {
      this.failed = true;
      this.report(error);
    } finally {
      this.flushing = false;
      this.publish();
    }
  }

  retry = () => {
    this.failed = false;
    this.error = null;
    this.publish();
    void this.flush();
    if (!this.canonical || !this.queue.length) void this.request<State>("/api/state").then(state => this.receive(state)).catch(error => this.report(error));
  };

  discard = () => {
    if (this.flushing) return;
    this.queue = [];
    this.failed = false;
    this.error = null;
    this.publish();
  };

  async select(selection: Selection) {
    if (this.canonical) {
      this.canonical = { ...this.canonical, selection };
      this.publish();
    }
    try { await this.request<Selection>("/api/selection", selection); }
    catch (error) { this.report(error); }
  }

  async history(direction: "undo" | "redo") {
    if (this.queue.length) return;
    try { this.receive(await this.request<State>(`/api/${direction}`, {})); }
    catch (error) { this.report(error); }
  }

  async restoreSnapshot(id: number) {
    if (this.queue.length) return;
    try { this.receive(await this.request<State>("/api/history", { id })); }
    catch (error) { this.report(error); }
  }

}
