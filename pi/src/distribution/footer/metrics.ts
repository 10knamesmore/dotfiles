import { performance } from "node:perf_hooks";
import type { Usage } from "@earendil-works/pi-ai";
import type { ExtensionContext } from "@earendil-works/pi-coding-agent";

/** Usage that Pi has durably recorded for the current session. */
export interface SessionUsageTotals {
  /** Input tokens reported by provider and tool usage records. */
  input: number;

  /** Output tokens reported by provider and tool usage records. */
  output: number;

  /** Prompt tokens served from provider caches. */
  cacheRead: number;

  /** Prompt tokens written into provider caches. */
  cacheWrite: number;

  /** Provider-calculated USD cost summed across recorded usage. */
  cost: number;
}

/** A verified Codex subscription window read from one provider response. */
export interface RateLimitWindow {
  /** Footer label derived from the exact backend window length. */
  readonly label: "5h" | "168h";

  /** Backend-reported fraction already consumed, expressed as 0 through 100. */
  readonly usedPercent: number;
}

/** Codex subscription windows present on the latest response. */
export interface RateLimitSnapshot {
  /** Primary or secondary window whose own backend length is exactly 300 minutes. */
  readonly fiveHour: RateLimitWindow | undefined;

  /** Primary or secondary window whose own backend length is exactly 10080 minutes. */
  readonly weekly: RateLimitWindow | undefined;
}

const EMPTY_RATE_LIMITS: RateLimitSnapshot = {
  fiveHour: undefined,
  weekly: undefined,
};

function addUsage(totals: SessionUsageTotals, usage: Usage): void {
  totals.input += usage.input;
  totals.output += usage.output;
  totals.cacheRead += usage.cacheRead;
  totals.cacheWrite += usage.cacheWrite;
  totals.cost += usage.cost.total;
}

/** Sum Pi's recorded assistant, tool, compaction, and branch-summary usage. */
export function collectSessionUsage(ctx: ExtensionContext): SessionUsageTotals {
  const totals: SessionUsageTotals = {
    input: 0,
    output: 0,
    cacheRead: 0,
    cacheWrite: 0,
    cost: 0,
  };
  for (const entry of ctx.sessionManager.getEntries()) {
    if (entry.type === "message" && entry.message.role === "assistant") {
      addUsage(totals, entry.message.usage);
    } else if (
      entry.type === "message" &&
      entry.message.role === "toolResult" &&
      entry.message.usage
    ) {
      addUsage(totals, entry.message.usage);
    } else if (
      (entry.type === "compaction" || entry.type === "branch_summary") &&
      entry.usage
    ) {
      addUsage(totals, entry.usage);
    }
  }
  return totals;
}

function responseHeader(
  headers: Readonly<Record<string, string>>,
  name: string,
): string | undefined {
  const direct = headers[name];
  if (direct !== undefined) return direct;
  const normalizedName = name.toLowerCase();
  for (const [key, value] of Object.entries(headers)) {
    if (key.toLowerCase() === normalizedName) return value;
  }
  return undefined;
}

function finiteNumber(value: string | undefined): number | undefined {
  if (value === undefined || value.trim() === "") return undefined;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : undefined;
}

/** Parse only the source-verified `openai-codex` primary/secondary response-header contract. */
export function parseCodexRateLimits(
  headers: Readonly<Record<string, string>>,
  provider: string | undefined,
): RateLimitSnapshot {
  if (provider !== "openai-codex") return EMPTY_RATE_LIMITS;
  let fiveHour: RateLimitWindow | undefined;
  let weekly: RateLimitWindow | undefined;
  for (const key of ["primary", "secondary"] as const) {
    const usedPercent = finiteNumber(
      responseHeader(headers, `x-codex-${key}-used-percent`),
    );
    const windowMinutes = finiteNumber(
      responseHeader(headers, `x-codex-${key}-window-minutes`),
    );
    if (usedPercent === undefined || usedPercent < 0 || usedPercent > 100)
      continue;
    if (windowMinutes === 300) fiveHour = { label: "5h", usedPercent };
    else if (windowMinutes === 10_080) weekly = { label: "168h", usedPercent };
  }
  return { fiveHour, weekly };
}

/** Retains only the latest provider-backed rate-limit snapshot. */
export class RateLimitTracker {
  private current: RateLimitSnapshot = EMPTY_RATE_LIMITS;

  /** Replace the snapshot and report whether visible footer data changed. */
  public update(
    headers: Readonly<Record<string, string>>,
    provider: string | undefined,
  ): boolean {
    const next = parseCodexRateLimits(headers, provider);
    const changed =
      next.fiveHour?.usedPercent !== this.current.fiveHour?.usedPercent ||
      next.weekly?.usedPercent !== this.current.weekly?.usedPercent;
    this.current = next;
    return changed;
  }

  /** Clear data when a session or provider changes so old account usage is never shown. */
  public clear(): boolean {
    const changed =
      this.current.fiveHour !== undefined || this.current.weekly !== undefined;
    this.current = EMPTY_RATE_LIMITS;
    return changed;
  }

  /** Return the immutable current snapshot. */
  public snapshot(): RateLimitSnapshot {
    return this.current;
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
    this.accumulatedMilliseconds += Math.max(
      0,
      performance.now() - this.startedAt,
    );
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
      (this.startedAt === undefined
        ? 0
        : Math.max(0, performance.now() - this.startedAt))
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
