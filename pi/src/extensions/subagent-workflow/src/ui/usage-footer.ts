/** Session-scoped child usage shown through pi's native footer status line. */

import type { ExtensionContext } from "@earendil-works/pi-coding-agent";
import type { ChildRunEvent, SpawnedRun, SubagentRunner } from "../runner/runner.js";
import { sumUsage } from "../store/run-store.js";
import type { ResolvedSpec, UsageSummary } from "../types.js";
import { reportDiagnostic } from "../diagnostics.js";
import { errorMessage, isRecord } from "../util.js";
import { formatTokens } from "./format.js";

export const USAGE_STATUS_KEY = "subagent-workflow:usage";

type FooterContext = Pick<ExtensionContext, "modelRegistry" | "sessionManager" | "ui">;
type FooterRunner = Pick<SubagentRunner, "subscribeChildEvents" | "subscribeSpawns" | "sessionRuns" | "runDetail">;
type UsageAuth = "metered" | "subscription" | "mixed";

/** Format a cumulative child-usage snapshot in the same vocabulary as pi's footer. */
export function formatUsageFooter(usage: UsageSummary, auth: UsageAuth = "metered"): string | undefined {
  if (!hasUsage(usage)) return undefined;
  const parts = ["WF total"];
  if (usage.input > 0) parts.push(`↑${formatTokens(usage.input)}`);
  if (usage.output > 0) parts.push(`↓${formatTokens(usage.output)}`);
  if (usage.cacheRead > 0) parts.push(`R${formatTokens(usage.cacheRead)}`);
  if (usage.cacheWrite > 0) parts.push(`W${formatTokens(usage.cacheWrite)}`);
  const suffix = auth === "subscription" ? " (sub)" : auth === "mixed" ? " (mixed)" : "";
  parts.push(`$${usage.cost.toFixed(3)}${suffix}`);
  return parts.join(" ");
}

/**
 * Aggregate every direct subagent and workflow child attached to one pi
 * session. Child snapshots are cumulative, so later events replace earlier
 * values instead of being added again.
 */
export class SubagentUsageFooter {
  private ctx: FooterContext | undefined;
  private readonly usage = new Map<string, UsageSummary>();
  private readonly models = new Map<string, Pick<ResolvedSpec, "modelId" | "provider">>();
  private readonly hydratedRuns = new Set<string>();
  private unsubscribeSpawns: (() => void) | undefined;
  private unsubscribeChildEvents: (() => void) | undefined;
  private lastStatus: string | undefined;
  private uiFailed = false;
  private disposed = false;

  constructor(private readonly runner: FooterRunner) {
    this.unsubscribeSpawns = runner.subscribeSpawns((run) => this.observeSpawn(run));
    this.unsubscribeChildEvents = runner.subscribeChildEvents((event) => this.observeChildEvent(event));
  }

  /** Current footer text, shared with the prompt editor without recalculating usage. */
  statusText(): string | undefined {
    return this.lastStatus;
  }

  /** Restore durable totals whenever pi starts, resumes, forks, or reloads a session. */
  attach(ctx: FooterContext): void {
    if (this.disposed) return;
    this.clearRenderedStatus();
    this.ctx = ctx;
    this.usage.clear();
    this.models.clear();
    this.hydratedRuns.clear();
    this.lastStatus = undefined;
    this.uiFailed = false;

    for (const run of this.runner.sessionRuns(ctx.sessionManager.getSessionId())) this.hydrateRun(run.runId);
    this.render();
  }

  /** Attach a just-launched workflow even when it replays without spawning a new child. */
  trackRun(runId: string, ctx: FooterContext): void {
    if (this.disposed) return;
    if (this.ctx?.sessionManager.getSessionId() !== ctx.sessionManager.getSessionId()) this.attach(ctx);
    else this.ctx = ctx;
    this.hydrateRun(runId);
    this.render();
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.unsubscribeSpawns?.();
    this.unsubscribeSpawns = undefined;
    this.unsubscribeChildEvents?.();
    this.unsubscribeChildEvents = undefined;
    this.clearRenderedStatus();
    this.usage.clear();
    this.models.clear();
    this.hydratedRuns.clear();
    this.ctx = undefined;
  }

  private observeSpawn(run: SpawnedRun): void {
    if (this.disposed || run.parentSessionId !== this.ctx?.sessionManager.getSessionId()) return;
    this.hydrateRun(run.runId);
    this.render();
  }

  private observeChildEvent(observed: ChildRunEvent): void {
    if (this.disposed || observed.parentSessionId !== this.ctx?.sessionManager.getSessionId()) return;
    this.hydrateRun(observed.runId);
    const key = childKey(observed.runId, observed.event.id);
    if (observed.resolved)
      this.models.set(key, { provider: observed.resolved.provider, modelId: observed.resolved.modelId });
    if (observed.event.type === "usage") this.mergeUsage(key, observed.event.usage);
    if (observed.event.type === "result") {
      this.mergeUsage(key, observed.event.result.usage);
      this.models.set(key, {
        provider: observed.event.result.resolved.provider,
        modelId: observed.event.result.resolved.modelId,
      });
    }
    if (observed.event.type === "usage" || observed.event.type === "result") this.render();
  }

  private hydrateRun(runId: string): void {
    if (!this.ctx || this.hydratedRuns.has(runId)) return;
    this.hydratedRuns.add(runId);
    const detail = this.runner.runDetail(this.ctx.sessionManager.getSessionId(), runId);
    for (const child of detail.children) {
      const key = childKey(runId, child.id);
      this.mergeUsage(key, child.usage);
      if (child.resolved) {
        this.models.set(key, { provider: child.resolved.provider, modelId: child.resolved.modelId });
      }
    }
  }

  private mergeUsage(key: string, incoming: UsageSummary): void {
    if (!validUsage(incoming)) return;
    const current = this.usage.get(key);
    this.usage.set(key, current ? cumulativeMax(current, incoming) : copyUsage(incoming));
  }

  private authKind(): UsageAuth {
    if (!this.ctx) return "metered";
    let subscription = 0;
    let metered = 0;
    for (const [key, usage] of this.usage) {
      if (!hasUsage(usage)) continue;
      const resolved = this.models.get(key);
      // A child whose model metadata is missing (still resolving, or lost to a
      // degraded write) is unknown, not metered: it must not flip an
      // all-subscription session's label to "(mixed)".
      if (!resolved) continue;
      try {
        const model = this.ctx.modelRegistry.find(resolved.provider, resolved.modelId);
        if (model && this.ctx.modelRegistry.isUsingOAuth(model)) subscription += 1;
        else metered += 1;
      } catch {
        metered += 1;
      }
    }
    if (subscription > 0 && metered === 0) return "subscription";
    if (subscription > 0) return "mixed";
    return "metered";
  }

  private render(): void {
    if (!this.ctx || this.uiFailed) return;
    const next = formatUsageFooter(sumUsage(this.usage.values()), this.authKind());
    if (next === this.lastStatus) return;
    try {
      this.ctx.ui.setStatus(USAGE_STATUS_KEY, next);
      this.lastStatus = next;
    } catch (error) {
      this.uiFailed = true;
      reportDiagnostic(`[subagent-workflow] usage footer failed: ${errorMessage(error)}`);
    }
  }

  private clearRenderedStatus(): void {
    if (!this.ctx || this.lastStatus === undefined || this.uiFailed) return;
    try {
      this.ctx.ui.setStatus(USAGE_STATUS_KEY, undefined);
    } catch (error) {
      reportDiagnostic(`[subagent-workflow] usage footer failed: ${errorMessage(error)}`);
    }
  }
}

function childKey(runId: string, childId: string): string {
  return `${runId}\0${childId}`;
}

function copyUsage(usage: UsageSummary): UsageSummary {
  return { ...usage };
}

function cumulativeMax(left: UsageSummary, right: UsageSummary): UsageSummary {
  return {
    input: Math.max(left.input, right.input),
    output: Math.max(left.output, right.output),
    cacheRead: Math.max(left.cacheRead, right.cacheRead),
    cacheWrite: Math.max(left.cacheWrite, right.cacheWrite),
    cost: Math.max(left.cost, right.cost),
    turns: Math.max(left.turns, right.turns),
  };
}

function hasUsage(usage: UsageSummary): boolean {
  return (
    usage.input > 0 ||
    usage.output > 0 ||
    usage.cacheRead > 0 ||
    usage.cacheWrite > 0 ||
    usage.cost > 0 ||
    usage.turns > 0
  );
}

function validUsage(value: unknown): value is UsageSummary {
  if (!isRecord(value)) return false;
  return [value.input, value.output, value.cacheRead, value.cacheWrite, value.cost, value.turns].every(
    (item) => typeof item === "number" && Number.isFinite(item) && item >= 0,
  );
}
