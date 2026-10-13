import { performance } from "node:perf_hooks";
import type { AssistantMessageEvent } from "@earendil-works/pi-ai";

/** Footer publishes the editor's activity when its phase or active tools change. */
export const SESSION_ACTIVITY_CHANGED = "dotfiles:session-activity:changed";

export type ModelPhase = "waiting" | "thinking" | "writing" | "compacting" | "ready";

interface ModelRequest {
  startedAt: number;
  active: boolean;
  phase: "waiting" | "thinking" | "writing";
  contentTiming?: { firstAt: number; lastAt: number };
}

/** Tracks ordinary response streams separately from compaction requests. */
export class ModelResponseTracker {
  private request: ModelRequest | undefined;
  private compacting = false;

  public startRequest(): void {
    if (this.compacting) return;
    this.request = { startedAt: performance.now(), active: true, phase: "waiting" };
  }

  /** Empty block-start events do not count as the first generated content. */
  public record(event: AssistantMessageEvent): void {
    if (this.compacting || !this.request?.active) return;
    let phase: ModelRequest["phase"];
    switch (event.type) {
      case "thinking_delta":
      case "text_delta":
      case "toolcall_delta":
        if (!event.delta) return;
        phase = event.type === "thinking_delta" ? "thinking" : "writing";
        break;
      case "toolcall_start": {
        // Some providers emit the complete tool name before any argument delta.
        const content = event.partial.content[event.contentIndex];
        if (content?.type !== "toolCall" || !content.name) return;
        phase = "writing";
        break;
      }
      default:
        return;
    }
    const now = performance.now();
    this.request.contentTiming ??= { firstAt: now, lastAt: now };
    this.request.contentTiming.lastAt = now;
    this.request.phase = phase;
  }

  /** Keep the final request's TTFT available while the agent is ready. */
  public finishRequest(): void {
    if (this.request) this.request.active = false;
  }

  public startCompaction(): void {
    this.finishRequest();
    this.compacting = true;
  }

  public finishCompaction(): void {
    this.compacting = false;
  }

  public reset(): void {
    this.request = undefined;
    this.compacting = false;
  }

  /** Content events bound generation time; stream completion and usage events do not extend it. */
  public generationMilliseconds(): number | undefined {
    const timing = this.request?.contentTiming;
    return timing === undefined ? undefined : timing.lastAt - timing.firstAt;
  }

  public snapshot(): { phase: ModelPhase; timeToFirstTokenMilliseconds?: number } {
    const request = this.request;
    return {
      phase: this.compacting ? "compacting" : request?.active ? request.phase : "ready",
      timeToFirstTokenMilliseconds:
        request?.contentTiming === undefined ? undefined : request.contentTiming.firstAt - request.startedAt,
    };
  }
}
