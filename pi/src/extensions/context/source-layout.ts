import { Worker } from "node:worker_threads";
import type { Theme, ThemeColor } from "@earendil-works/pi-coding-agent";

export interface SourceDocument {
  body: string;
  format: "text" | "json";
}

type TextStyle = [opening: string, closing: string];
export type SyntaxStyles = Record<string, TextStyle>;

export interface SourceViewportRequest {
  documentId: number;
  width: number;
  scrollTop: number;
  height: number;
}

export interface SourceViewport extends SourceViewportRequest {
  contentHeight: number;
  gutterWidth: number;
  /** Zero-based wrapped row where the returned viewport buffer starts. */
  firstRow: number;
  lines: { text: string; sourceLine: number | undefined }[];
}

export type SourceWorkerMessage =
  | { type: "theme"; styles: SyntaxStyles }
  | { type: "document"; documentId: number; document: SourceDocument }
  | { type: "viewport"; request: SourceViewportRequest };

// Pi's highlighter accepts formatter functions; only their ANSI wrappers cross the worker boundary.
function syntaxStyles(theme: Theme): SyntaxStyles {
  const colors = {
    keyword: "syntaxKeyword",
    built_in: "syntaxType",
    literal: "syntaxNumber",
    number: "syntaxNumber",
    regexp: "syntaxString",
    string: "syntaxString",
    subst: "text",
    comment: "syntaxComment",
    doctag: "syntaxComment",
    meta: "muted",
    function: "syntaxFunction",
    title: "syntaxFunction",
    class: "syntaxType",
    type: "syntaxType",
    tag: "syntaxPunctuation",
    name: "syntaxKeyword",
    attr: "syntaxVariable",
    variable: "syntaxVariable",
    params: "syntaxVariable",
    operator: "syntaxOperator",
    punctuation: "syntaxPunctuation",
    addition: "toolDiffAdded",
    deletion: "toolDiffRemoved",
  } satisfies Record<string, ThemeColor>;
  const styles = Object.fromEntries(
    Object.entries(colors).map(([scope, color]) => [scope, theme.fg(color, "\0").split("\0") as TextStyle]),
  );
  styles.emphasis = theme.italic("\0").split("\0") as TextStyle;
  styles.strong = theme.bold("\0").split("\0") as TextStyle;
  styles.link = theme.underline("\0").split("\0") as TextStyle;
  return styles;
}

interface PendingViewport {
  request: SourceViewportRequest;
  receive: (viewport: SourceViewport) => void;
}

/** One inspector owns one lazy worker. Only a viewport-sized buffer returns to the UI thread. */
export class SourceLayouts {
  private worker: Worker | undefined;
  private styles: SyntaxStyles;
  private readonly documents = new Set<number>();
  private readonly queued = new Map<number, PendingViewport>();
  private active: PendingViewport | undefined;

  public constructor(
    theme: Theme,
    private readonly onError: (error: Error) => void,
  ) {
    this.styles = syntaxStyles(theme);
  }

  public updateTheme(theme: Theme): boolean {
    const styles = syntaxStyles(theme);
    if (JSON.stringify(styles) === JSON.stringify(this.styles)) return false;
    this.styles = styles;
    this.worker?.postMessage({ type: "theme", styles } satisfies SourceWorkerMessage);
    return true;
  }

  public request(
    request: SourceViewportRequest,
    document: SourceDocument,
    receive: (viewport: SourceViewport) => void,
  ): void {
    if (!this.worker) {
      const worker = new Worker(new URL("./source-worker.ts", import.meta.url));
      this.worker = worker;
      worker.on("message", (viewport: SourceViewport) => {
        const active = this.active!;
        this.active = undefined;
        active.receive(viewport);
        this.dispatch();
      });
      worker.on("error", (error) => {
        if (this.worker === worker) this.onError(error);
      });
      worker.on("exit", (code) => {
        if (this.worker === worker && code === 0) this.onError(new Error("Context layout worker exited."));
      });
      worker.postMessage({ type: "theme", styles: this.styles } satisfies SourceWorkerMessage);
    }
    if (!this.documents.has(request.documentId)) {
      this.documents.add(request.documentId);
      this.worker.postMessage({
        type: "document",
        documentId: request.documentId,
        document,
      } satisfies SourceWorkerMessage);
    }
    // Replace pending scroll/resize requests for the same page instead of accumulating frames.
    this.queued.set(request.documentId, { request, receive });
    this.dispatch();
  }

  private dispatch(): void {
    if (this.active || !this.queued.size) return;
    const pending = this.queued.values().next().value!;
    this.queued.delete(pending.request.documentId);
    this.active = pending;
    this.worker!.postMessage({ type: "viewport", request: pending.request } satisfies SourceWorkerMessage);
  }

  public dispose(): void {
    const worker = this.worker;
    this.worker = undefined;
    this.queued.clear();
    if (worker) {
      worker.removeAllListeners("message");
      void worker.terminate();
    }
  }
}
