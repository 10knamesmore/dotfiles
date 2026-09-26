import { performance } from "node:perf_hooks";
import type { Usage } from "@earendil-works/pi-ai";

/** Number of completed turns retained by recent-turn footer metrics. */
export const RECENT_TURNS = 5;

/** Cache hit rate over the most recent completed turns. */
export class RecentHitRateTracker {
  private readonly turnTokens: Array<{
    input: number;
    cacheWrite: number;
    cacheRead: number;
  }> = [];
  private readonly totals = { input: 0, cacheWrite: 0, cacheRead: 0 };
  private current = { input: 0, cacheWrite: 0, cacheRead: 0 };

  /** Accumulate one assistant message's usage into the in-flight turn. */
  public record(usage: Usage | undefined): void {
    if (!usage) return;
    this.current.input += usage.input;
    this.current.cacheWrite += usage.cacheWrite ?? 0;
    this.current.cacheRead += usage.cacheRead ?? 0;
  }

  /** Close the in-flight turn, keep only the most recent window, and skip empty turns. */
  public endTurn(): void {
    if (this.current.input === 0 && this.current.cacheWrite === 0 && this.current.cacheRead === 0) return;
    this.turnTokens.push(this.current);
    this.totals.input += this.current.input;
    this.totals.cacheWrite += this.current.cacheWrite;
    this.totals.cacheRead += this.current.cacheRead;
    const oldest = this.turnTokens.length > RECENT_TURNS ? this.turnTokens.shift() : undefined;
    if (oldest) {
      this.totals.input -= oldest.input;
      this.totals.cacheWrite -= oldest.cacheWrite;
      this.totals.cacheRead -= oldest.cacheRead;
    }
    this.current = { input: 0, cacheWrite: 0, cacheRead: 0 };
  }

  /** Clear window state when Pi replaces the active session. */
  public reset(): void {
    this.turnTokens.length = 0;
    this.totals.input = 0;
    this.totals.cacheWrite = 0;
    this.totals.cacheRead = 0;
    this.current = { input: 0, cacheWrite: 0, cacheRead: 0 };
  }

  /** Hit rate percentage over the window, undefined when the window has no input tokens. */
  public hitRatePercent(): number | undefined {
    const promptTokens = this.totals.input + this.totals.cacheWrite + this.totals.cacheRead;
    if (promptTokens <= 0) return undefined;
    return (this.totals.cacheRead / promptTokens) * 100;
  }
}

/** Output throughput over the most recent completed turns. */
export class RecentTokensPerSecondTracker {
  private readonly turns: Array<{
    outputTokens: number;
    modelMilliseconds: number;
  }> = [];
  private currentOutputTokens = 0;

  /** Accumulate generated tokens from one assistant message in the current turn. */
  public record(usage: Usage | undefined): void {
    if (usage) this.currentOutputTokens += usage.output;
  }

  /** Close one turn and retain only the recent window. */
  public endTurn(modelMilliseconds: number): void {
    if (this.currentOutputTokens === 0 && modelMilliseconds === 0) return;
    this.turns.push({
      outputTokens: this.currentOutputTokens,
      modelMilliseconds,
    });
    if (this.turns.length > RECENT_TURNS) this.turns.shift();
    this.currentOutputTokens = 0;
  }

  /** Clear recent throughput when Pi replaces the active session. */
  public reset(): void {
    this.turns.length = 0;
    this.currentOutputTokens = 0;
  }

  /** Return average generated tokens per model second in the recent window. */
  public tokensPerSecond(): number | undefined {
    let outputTokens = 0;
    let modelMilliseconds = 0;
    for (const turn of this.turns) {
      outputTokens += turn.outputTokens;
      modelMilliseconds += turn.modelMilliseconds;
    }
    if (outputTokens <= 0 || modelMilliseconds <= 0) return undefined;
    return outputTokens / (modelMilliseconds / 1_000);
  }
}

/** Measures wall time from provider-request preparation through the matching Pi completion event. */
export class ModelDurationTracker {
  private accumulatedMilliseconds = 0;
  private startedAt: number | undefined;

  /** Start one sequential model call; duplicate starts do not discard already elapsed time. */
  public start(): boolean {
    if (this.startedAt !== undefined) return false;
    this.startedAt = performance.now();
    return true;
  }

  /** Finish an in-flight model call and retain its wall duration. */
  public finish(): boolean {
    if (this.startedAt === undefined) return false;
    this.accumulatedMilliseconds += Math.max(0, performance.now() - this.startedAt);
    this.startedAt = undefined;
    return true;
  }

  /** Reset attribution when Pi replaces the active session. */
  public reset(): void {
    this.accumulatedMilliseconds = 0;
    this.startedAt = undefined;
  }

  /** Include an active call without scheduling an idle repaint timer. */
  public elapsedMilliseconds(): number {
    return (
      this.accumulatedMilliseconds +
      (this.startedAt === undefined ? 0 : Math.max(0, performance.now() - this.startedAt))
    );
  }
}

/** Measures active-session wall time from the latest Pi `session_start` event. */
export class ActiveSessionDurationTracker {
  private startedAt = performance.now();

  /** Reset when Pi starts, resumes, forks, reloads, or replaces the active session. */
  public reset(): void {
    this.startedAt = performance.now();
  }

  /** Return only time observed by the current extension runtime for this active session. */
  public elapsedMilliseconds(): number {
    return Math.max(0, performance.now() - this.startedAt);
  }
}
