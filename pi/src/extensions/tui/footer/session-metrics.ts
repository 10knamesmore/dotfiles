import { performance } from "node:perf_hooks";
import type { Usage } from "@earendil-works/pi-ai";
import type { ExtensionAPI, ExtensionContext, SessionEntry } from "@earendil-works/pi-coding-agent";

const RUN_STARTED = "footer:run-started";
const RUN_FINISHED = "footer:run-finished";

interface SessionUsageTotals {
  input: number;
  output: number;
  cacheRead: number;
  /** Parent-model cost; tool-reported child usage has its own cost display. */
  parentCostUsd: number;
}

export interface ToolUsageSnapshot {
  total: number;
  errors: number;
  top: ReadonlyArray<{ name: string; count: number }>;
}

export interface PromptRunSnapshot {
  state: "running" | "finished";
  elapsedMilliseconds: number;
  turns: number;
  /** Parent-model usage, including retries and summaries within this run. */
  usage: { input: number; output: number; costUsd: number } | undefined;
}

export interface SessionMetricsSnapshot {
  /** Finished runs with recorded boundaries; older transcripts have no run markers. */
  runs: number;
  turns: number;
  usage: SessionUsageTotals;
  tools: ToolUsageSnapshot;
  promptRun: PromptRunSnapshot | undefined;
}

interface FinishedRun {
  startEntryId: string;
  elapsedMilliseconds: number;
}

function parentUsage(entry: SessionEntry): Usage | undefined {
  if (entry.type === "message" && entry.message.role === "assistant") return entry.message.usage;
  if (entry.type === "compaction" || entry.type === "branch_summary" || entry.type === "usage") return entry.usage;
  return undefined;
}

function addSessionUsage(totals: SessionUsageTotals, usage: Usage, parent: boolean): void {
  totals.input += usage.input;
  totals.output += usage.output;
  totals.cacheRead += usage.cacheRead;
  if (parent) totals.parentCostUsd += usage.cost.total;
}

function collectSessionMetrics(entries: readonly SessionEntry[]): SessionMetricsSnapshot {
  const snapshot: SessionMetricsSnapshot = {
    runs: 0,
    turns: 0,
    usage: { input: 0, output: 0, cacheRead: 0, parentCostUsd: 0 },
    tools: { total: 0, errors: 0, top: [] },
    promptRun: undefined,
  };
  const toolCounts = new Map<string, number>();
  for (const entry of entries) {
    if (entry.type === "custom" && entry.customType === RUN_FINISHED) snapshot.runs += 1;
    const usage = parentUsage(entry);
    if (usage) addSessionUsage(snapshot.usage, usage, true);
    if (entry.type !== "message") continue;
    const message = entry.message;
    if (message.role === "assistant") snapshot.turns += 1;
    if (message.role !== "toolResult") continue;
    snapshot.tools.total += 1;
    if (message.isError) snapshot.tools.errors += 1;
    toolCounts.set(message.toolName, (toolCounts.get(message.toolName) ?? 0) + 1);
    if (message.usage) addSessionUsage(snapshot.usage, message.usage, false);
  }
  snapshot.tools.top = [...toolCounts.entries()]
    .sort(
      ([leftName, leftCount], [rightName, rightCount]) => rightCount - leftCount || leftName.localeCompare(rightName),
    )
    .slice(0, 4)
    .map(([name, count]) => ({ name, count }));
  return snapshot;
}

function collectRunMetrics(
  entries: readonly SessionEntry[],
  startEntryId: string,
  endIndex: number,
  state: PromptRunSnapshot["state"],
  elapsedMilliseconds: number,
): PromptRunSnapshot | undefined {
  const startIndex = entries.findIndex((entry) => entry.id === startEntryId);
  if (startIndex < 0) return undefined;
  const run: PromptRunSnapshot = { state, elapsedMilliseconds, turns: 0, usage: undefined };
  for (let index = startIndex + 1; index < endIndex; index += 1) {
    const entry = entries[index]!;
    if (entry.type === "message" && entry.message.role === "assistant") run.turns += 1;
    const usage = parentUsage(entry);
    if (!usage) continue;
    const totals = (run.usage ??= { input: 0, output: 0, costUsd: 0 });
    totals.input += usage.input + usage.cacheRead + usage.cacheWrite;
    totals.output += usage.output;
    totals.costUsd += usage.cost.total;
  }
  return run;
}

/** Persist run boundaries; derive counts and usage from Pi's loaded transcript. */
export class SessionMetrics {
  private activeRun: { startEntryId: string; startedAt: number } | undefined;
  private cachedLeafId: string | null | undefined;
  private cachedSnapshot: SessionMetricsSnapshot | undefined;

  public constructor(private readonly appendEntry: ExtensionAPI["appendEntry"]) {}

  /** Session replacement drops runtime clocks; persisted runs remain in the transcript. */
  public reset(): void {
    this.activeRun = undefined;
    this.cachedLeafId = undefined;
    this.cachedSnapshot = undefined;
  }

  public isRunning(): boolean {
    return this.activeRun !== undefined;
  }

  /** A retry or automatic continuation belongs to the already active run. */
  public startRun(ctx: ExtensionContext): void {
    if (this.activeRun) return;
    const startedAt = performance.now();
    this.appendEntry(RUN_STARTED);
    this.activeRun = { startEntryId: ctx.sessionManager.getLeafId()!, startedAt };
    this.cachedSnapshot = undefined;
  }

  /** Settlement, cancellation, or shutdown ends the current run exactly once. */
  public finishRun(): void {
    if (!this.activeRun) return;
    const data: FinishedRun = {
      startEntryId: this.activeRun.startEntryId,
      elapsedMilliseconds: Math.max(0, performance.now() - this.activeRun.startedAt),
    };
    this.appendEntry(RUN_FINISHED, data);
    this.activeRun = undefined;
    this.cachedSnapshot = undefined;
  }

  /** Re-scan only when the transcript leaf changes, not on spinner or idle repaints. */
  public snapshot(ctx: ExtensionContext): SessionMetricsSnapshot {
    const leafId = ctx.sessionManager.getLeafId();
    if (!this.cachedSnapshot || leafId !== this.cachedLeafId) {
      const entries = ctx.sessionManager.getEntries();
      const snapshot = collectSessionMetrics(entries);
      if (this.activeRun) {
        snapshot.promptRun = collectRunMetrics(entries, this.activeRun.startEntryId, entries.length, "running", 0);
      } else {
        const lastFinished = ctx.sessionManager
          .getBranch()
          .reverse()
          .find((entry) => entry.type === "custom" && entry.customType === RUN_FINISHED);
        if (lastFinished?.type === "custom") {
          const data = lastFinished.data as FinishedRun;
          snapshot.promptRun = collectRunMetrics(
            entries,
            data.startEntryId,
            entries.findIndex((entry) => entry.id === lastFinished.id),
            "finished",
            data.elapsedMilliseconds,
          );
        }
      }
      this.cachedLeafId = leafId;
      this.cachedSnapshot = snapshot;
    }
    if (this.activeRun && this.cachedSnapshot.promptRun) {
      return {
        ...this.cachedSnapshot,
        promptRun: {
          ...this.cachedSnapshot.promptRun,
          elapsedMilliseconds: Math.max(0, performance.now() - this.activeRun.startedAt),
        },
      };
    }
    return this.cachedSnapshot;
  }
}
