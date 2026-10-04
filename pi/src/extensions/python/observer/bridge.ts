import type { Readable, Writable } from "node:stream";
import { StringDecoder } from "node:string_decoder";
import { logPythonEvent } from "../diagnostics.js";
import type { InspectData, InspectRequest, InspectResult } from "./types.js";

const MAX_RESPONSE_BYTES = 16 * 1024 * 1024;

/** One in-flight inspection, independent of the worker's code-execution pipe. */
export class ObserverBridge {
  private pending?: {
    id: number;
    resolve: (result: InspectResult) => void;
  };
  private nextId = 0;
  private closed = false;

  public constructor(
    private readonly sessionId: string,
    private readonly workspaceId: string,
    private readonly requests: Writable,
    private readonly responses: Readable,
  ) {
    const decoder = new StringDecoder("utf8");
    let buffer = "";
    responses.on("data", (chunk: Buffer) => {
      buffer += decoder.write(chunk);
      try {
        if (Buffer.byteLength(buffer) > MAX_RESPONSE_BYTES) throw new Error("Inspection response exceeded 16 MiB.");
        let end: number;
        while ((end = buffer.indexOf("\n")) !== -1) {
          const line = buffer.slice(0, end);
          buffer = buffer.slice(end + 1);
          this.receive(JSON.parse(line) as Record<string, unknown>);
        }
      } catch (error) {
        this.failed(error);
      }
    });
    requests.on("error", (error) => this.failed(error));
    responses.on("error", (error) => this.failed(error));
    responses.on("end", () => this.close());
  }

  public inspect(query: InspectRequest): Promise<InspectResult> {
    if (this.closed) return Promise.resolve({ status: "unavailable", workspaceId: this.workspaceId });
    if (this.pending) return Promise.resolve({ status: "busy", workspaceId: this.workspaceId });
    return new Promise((resolve) => {
      const id = ++this.nextId;
      this.pending = { id, resolve };
      this.requests.write(`${JSON.stringify({ requestId: id, query })}\n`, (error) => {
        if (error) this.failed(error);
      });
    });
  }

  public close(): void {
    if (this.closed) return;
    this.closed = true;
    this.pending?.resolve({ status: "unavailable", workspaceId: this.workspaceId });
    this.pending = undefined;
    this.requests.destroy();
    this.responses.destroy();
  }

  private receive(message: Record<string, unknown>): void {
    const pending = this.pending;
    if (!pending || message.requestId !== pending.id) throw new Error("Unexpected inspection response.");
    if (!["ok", "not_found", "inspection_failed"].includes(String(message.status))) {
      throw new Error("Invalid inspection response status.");
    }
    if (message.status === "ok" && (!message.data || typeof message.sampledAt !== "number")) {
      throw new Error("Invalid inspection payload.");
    }
    this.pending = undefined;
    if (message.status === "inspection_failed") {
      logPythonEvent({ sessionId: this.sessionId, phase: "observer_inspection_failed", reason: String(message.errorType) });
    }
    pending.resolve(message.status === "ok"
      ? { status: "ok", workspaceId: this.workspaceId, sampledAt: message.sampledAt as number, data: message.data as InspectData }
      : { status: message.status as "not_found" | "inspection_failed", workspaceId: this.workspaceId });
  }

  private failed(error: unknown): void {
    if (this.closed) return;
    logPythonEvent({ sessionId: this.sessionId, phase: "observer_transport_failed", reason: error instanceof Error ? error.message : String(error) });
    this.close();
  }
}
