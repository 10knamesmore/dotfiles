/**
 * Navigator data model and navigation state machine.
 *
 * Runs owned by this process render from the runner's synchronous in-memory
 * projection. Non-owned runs render from durable snapshots. `isLive` is used
 * only to pin active runs to the top of the list; live handles (steer/stop and
 * transcript following) remain in the overlay shell.
 */

import { join } from "node:path";
import type { FilterMode, Level } from "./controls.js";
import { orderedChildren } from "./controls.js";
import { listRunSummaries, readRunDetail, runsDirFor, type ChildRow, type ReadOptions, type RunDetail, type RunSummary } from "./store-read.js";

export class NavigatorModel {
  constructor(
    private readonly cwd: string,
    private readonly opts: ReadOptions,
    private readonly isLive: (runId: string) => boolean = () => false,
  ) {}

  /** Runs newest-first, with active/live runs pinned to the top. */
  runs(): RunSummary[] {
    const rows = listRunSummaries(this.cwd, this.opts);
    const active: RunSummary[] = [];
    const history: RunSummary[] = [];
    for (const row of rows) {
      const live = !row.corrupt && (row.status === "running" || row.status === "pending" || this.isLive(row.runId));
      (live ? active : history).push(row);
    }
    return [...active, ...history];
  }

  detail(runId: string): RunDetail {
    const runDir = join(runsDirFor(this.cwd, this.opts.root), runId);
    return readRunDetail(runDir, runId, { ...this.opts, bypassCache: true });
  }
}

interface Frame {
  level: Level;
  cursor: number;
}

/** Stack of (level, cursor) frames plus the drilled-in run/child context and detail scroll. */
export class NavigatorState {
  private stack: Frame[] = [{ level: "runs", cursor: 0 }];
  runId: string | undefined;
  private selectedRunId: string | undefined;
  private selectedChildId: string | undefined;
  childId: string | undefined;
  filter: FilterMode = "all";
  /** Explicit workflow layout row used by PageUp/PageDown; undefined follows the selected child. */
  scroll: number | undefined;

  private top(): Frame {
    return this.stack[this.stack.length - 1]!;
  }

  get level(): Level {
    return this.top().level;
  }

  get cursor(): number {
    return this.top().cursor;
  }

  get depth(): number {
    return this.stack.length;
  }

  clampCursor(count: number): void {
    const frame = this.top();
    frame.cursor = count <= 0 ? 0 : Math.max(0, Math.min(frame.cursor, count - 1));
  }

  move(delta: number, count: number): void {
    if (count <= 0) return;
    const frame = this.top();
    frame.cursor = (frame.cursor + delta + count) % count;
  }

  /** Move by one viewport without wrapping at the ends. */
  pageMove(delta: number, count: number, pageSize: number): void {
    if (count <= 0) return;
    const frame = this.top();
    frame.cursor = Math.max(0, Math.min(frame.cursor + delta * Math.max(1, pageSize), count - 1));
  }

  setCursor(cursor: number, count: number): void {
    this.top().cursor = cursor;
    this.clampCursor(count);
  }

  /** Keep the selected run stable when live rows are inserted or reordered. */
  reconcileRuns(runs: readonly RunSummary[]): void {
    if (this.level !== "runs") return;
    const frame = this.top();
    if (this.selectedRunId) {
      const index = runs.findIndex((run) => run.runId === this.selectedRunId);
      if (index >= 0) frame.cursor = index;
    }
    this.clampCursor(runs.length);
    this.selectedRunId = runs[frame.cursor]?.runId;
  }

  /**
   * Keep the selected child stable when live rows reorder or refilter: a
   * pending child gaining a start time changes its rank, so a bare index
   * would silently point at a different agent between refresh renders.
   */
  reconcileChildren(children: readonly ChildRow[]): void {
    if (this.level !== "run") return;
    const frame = this.top();
    if (this.selectedChildId) {
      const index = children.findIndex((child) => child.id === this.selectedChildId);
      if (index >= 0) frame.cursor = index;
    }
    this.clampCursor(children.length);
    this.selectedChildId = children[frame.cursor]?.id;
  }

  moveChild(delta: number, children: readonly ChildRow[]): void {
    this.reconcileChildren(children);
    this.move(delta, children.length);
    this.selectedChildId = children[this.cursor]?.id;
  }

  setChildCursor(cursor: number, children: readonly ChildRow[]): void {
    this.setCursor(cursor, children.length);
    this.selectedChildId = children[this.cursor]?.id;
  }

  moveRun(delta: number, runs: readonly RunSummary[]): void {
    this.reconcileRuns(runs);
    this.move(delta, runs.length);
    this.selectedRunId = runs[this.cursor]?.runId;
  }

  pageMoveRun(delta: number, runs: readonly RunSummary[], pageSize: number): void {
    this.reconcileRuns(runs);
    this.pageMove(delta, runs.length, pageSize);
    this.selectedRunId = runs[this.cursor]?.runId;
  }

  /** The drilled run, or the stable current selection at the run-list level. */
  currentRunId(runs: readonly RunSummary[]): string | undefined {
    if (this.runId) return this.runId;
    this.reconcileRuns(runs);
    return this.selectedRunId;
  }

  /** Start at a run detail while preserving normal back navigation to the run list. */
  seedRun(runId: string): void {
    this.switchRun(runId);
  }

  /** Replace any drilled run or agent context with another run's detail view. */
  switchRun(runId: string): void {
    const runsCursor = this.stack[0]?.cursor ?? 0;
    this.stack = [{ level: "runs", cursor: runsCursor }, { level: "run", cursor: 0 }];
    this.runId = runId;
    this.selectedRunId = runId;
    this.selectedChildId = undefined;
    this.childId = undefined;
    this.scroll = undefined;
  }

  /** Drill into the selected item. Returns the new level, or undefined if nothing to open. */
  drill(model: NavigatorModel): Level | undefined {
    const frame = this.top();
    if (frame.level === "runs") {
      const runs = model.runs();
      this.reconcileRuns(runs);
      const run = runs[frame.cursor];
      if (!run || (run.corrupt && run.label !== "quarantined - crashed mid-resume")) return undefined;
      this.runId = run.runId;
      this.stack.push({ level: "run", cursor: 0 });
      return "run";
    }
    if (frame.level === "run" && this.runId) {
      // Reconcile before reading the index: rows can have reordered since the
      // last render, and Enter must open the agent the user sees selected.
      const children = orderedChildren(model.detail(this.runId), this.filter);
      this.reconcileChildren(children);
      const child = children[frame.cursor];
      if (!child) return undefined;
      this.childId = child.id;
      this.scroll = undefined;
      this.stack.push({ level: "agent", cursor: 0 });
      return "agent";
    }
    return undefined;
  }

  /** Pop one level. Returns false when already at the run list (caller closes the overlay). */
  back(): boolean {
    if (this.stack.length <= 1) return false;
    this.stack.pop();
    this.scroll = undefined;
    if (this.top().level === "runs") {
      this.runId = undefined;
      this.selectedChildId = undefined;
    }
    this.childId = undefined;
    return true;
  }
}
