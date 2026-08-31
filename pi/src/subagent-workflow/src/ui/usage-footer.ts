/** Session-scoped child usage shown through pi's native footer status line. */

import { resolve, sep } from "node:path";
import type { ExtensionContext } from "@earendil-works/pi-coding-agent";
import type { ChildRunEvent, SpawnedRun, SubagentRunner } from "../runner/runner.js";
import { sumUsage } from "../store/run-store.js";
import { readRunSnapshot } from "../store/run-snapshot.js";
import type { ResolvedSpec, SubagentStatus, UsageSummary } from "../types.js";
import { reportDiagnostic } from "../diagnostics.js";
import { errorMessage, isRecord } from "../util.js";
import { formatTokens } from "./format.js";
import { defaultRunsRoot, runsDirFor, type RunRecordFile, type RunStatusFile } from "./navigator/store-read.js";

export const USAGE_STATUS_KEY = "subagent-workflow:usage";

type FooterContext = Pick<ExtensionContext, "cwd" | "modelRegistry" | "sessionManager" | "ui">;
type FooterRunner = Pick<SubagentRunner, "subscribeChildEvents" | "subscribeSpawns">;
type UsageAuth = "metered" | "subscription" | "mixed";

interface UsageSnapshot {
  usage: Map<string, UsageSummary>;
  models: Map<string, Pick<ResolvedSpec, "modelId" | "provider">>;
}

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
  private readonly runsRoot: string;
  private unsubscribeSpawns: (() => void) | undefined;
  private unsubscribeChildEvents: (() => void) | undefined;
  private lastStatus: string | undefined;
  private uiFailed = false;
  private disposed = false;

  constructor(runner: FooterRunner, runsRoot: string = defaultRunsRoot()) {
    this.runsRoot = runsRoot;
    this.unsubscribeSpawns = runner.subscribeSpawns((run) => this.observeSpawn(run));
    this.unsubscribeChildEvents = runner.subscribeChildEvents((event) => this.observeChildEvent(event));
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

    const runs = new Set<string>();
    for (const entry of ctx.sessionManager.getEntries()) {
      if (entry.type !== "custom") continue;
      if (entry.customType !== "subagent-workflow:run-started" && entry.customType !== "subagent-workflow:run-completed") continue;
      const data = entry.data;
      if (!isRecord(data) || typeof data.runDir !== "string" || typeof data.runId !== "string") continue;
      if (!this.isAllowedRunDir(data.runDir)) continue;
      runs.add(resolve(data.runDir));
    }
    for (const runDir of runs) this.hydrateRun(runDir);
    this.render();
  }

  /** Attach a just-launched workflow even when it replays without spawning a new child. */
  trackRun(runDir: string, ctx: FooterContext): void {
    if (this.disposed) return;
    if (this.ctx?.sessionManager.getSessionId() !== ctx.sessionManager.getSessionId()) this.attach(ctx);
    else this.ctx = ctx;
    if (!this.isAllowedRunDir(runDir)) return;
    this.hydrateRun(resolve(runDir));
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
    if (this.disposed || run.parentSessionId !== this.ctx?.sessionManager.getSessionId() || !this.isAllowedRunDir(run.runDir)) return;
    this.hydrateRun(resolve(run.runDir));
    this.render();
  }

  private observeChildEvent(observed: ChildRunEvent): void {
    if (this.disposed || observed.parentSessionId !== this.ctx?.sessionManager.getSessionId() || !this.isAllowedRunDir(observed.runDir)) return;
    const runDir = resolve(observed.runDir);
    this.hydrateRun(runDir);
    const key = childKey(runDir, observed.event.id);
    if (observed.resolved) this.models.set(key, { provider: observed.resolved.provider, modelId: observed.resolved.modelId });
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

  private hydrateRun(runDir: string): void {
    if (this.hydratedRuns.has(runDir)) return;
    this.hydratedRuns.add(runDir);
    const snapshot = readUsageSnapshot(runDir);
    for (const [childId, usage] of snapshot.usage) this.mergeUsage(childKey(runDir, childId), usage);
    for (const [childId, model] of snapshot.models) this.models.set(childKey(runDir, childId), model);
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

  private isAllowedRunDir(runDir: string): boolean {
    if (!this.ctx) return false;
    const base = resolve(runsDirFor(this.ctx.cwd, this.runsRoot));
    const candidate = resolve(runDir);
    return candidate.startsWith(`${base}${sep}`);
  }
}

/** Read durable status plus event-log fallback. Malformed telemetry is ignored. */
export function readUsageSnapshot(runDir: string): UsageSnapshot {
  const usage = new Map<string, UsageSummary>();
  const models = new Map<string, Pick<ResolvedSpec, "modelId" | "provider">>();
  const snapshot = readRunSnapshot(runDir);
  const status = snapshot.status as unknown as RunStatusFile | undefined;
  for (const [childId, child] of Object.entries(status?.children ?? {})) {
    if (validUsage(child.usage)) usage.set(childId, copyUsage(child.usage));
  }
  const run = snapshot.record as unknown as RunRecordFile | undefined;
  const runChildren = Array.isArray(run?.children) && run.children.every(validRunChild)
    ? run.children
    : undefined;
  for (const child of runChildren ?? []) {
    if (validResolved(child.resolved)) {
      models.set(child.id, { provider: child.resolved.provider, modelId: child.resolved.modelId });
    }
  }

  // status.json is atomically replaced after each event, but the event itself
  // is appended first. Only a terminal run with terminal, valid snapshots and
  // resolved model metadata for every registered child proves the event log
  // cannot add usage or OAuth classification data left out by a degraded
  // write. Keep the fold for every live/incomplete state and for corruption
  // recovery; avoid it for fully persisted long-running workflows.
  const statusIsComplete = runChildren !== undefined
    && isTerminalStatus(status?.status)
    && runChildren.every((child) => {
      const childStatus = status?.children?.[child.id];
      return validResolved(child.resolved)
        && isTerminalStatus(childStatus?.status)
        && validUsage(childStatus.usage);
    });
  if (statusIsComplete) return { usage, models };

  for (const event of snapshot.events) {
    if (!isRecord(event) || typeof event.id !== "string") continue;
    const eventUsage = event.type === "usage"
      ? event.usage
      : event.type === "result" && isRecord(event.result)
        ? event.result.usage
        : undefined;
    if (validUsage(eventUsage)) {
      const current = usage.get(event.id);
      usage.set(event.id, current ? cumulativeMax(current, eventUsage) : copyUsage(eventUsage));
    }
    if (event.type === "result" && isRecord(event.result) && validResolved(event.result.resolved)) {
      models.set(event.id, { provider: event.result.resolved.provider, modelId: event.result.resolved.modelId });
    }
  }
  return { usage, models };
}

function childKey(runDir: string, childId: string): string {
  return `${runDir}\0${childId}`;
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
  return usage.input > 0 || usage.output > 0 || usage.cacheRead > 0 || usage.cacheWrite > 0 || usage.cost > 0 || usage.turns > 0;
}

function validUsage(value: unknown): value is UsageSummary {
  if (!isRecord(value)) return false;
  return [value.input, value.output, value.cacheRead, value.cacheWrite, value.cost, value.turns]
    .every((item) => typeof item === "number" && Number.isFinite(item) && item >= 0);
}

function validResolved(value: unknown): value is Pick<ResolvedSpec, "modelId" | "provider"> {
  return isRecord(value) && typeof value.provider === "string" && typeof value.modelId === "string";
}

function validRunChild(value: unknown): value is { id: string; resolved?: ResolvedSpec } {
  return isRecord(value) && typeof value.id === "string";
}

function isTerminalStatus(value: unknown): value is Extract<SubagentStatus, "completed" | "failed" | "aborted"> {
  return value === "completed" || value === "failed" || value === "aborted";
}
