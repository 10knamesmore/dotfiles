/**
 * Below-editor status widget: a glanceable, keyboard-selectable list of running subagent work.
 *
 * Visible only while at least one child is running or queued; cleared when idle.
 * Register the factory once and re-render by calling tui.requestRender(), rather
 * than re-registering per update.
 *
 * Re-renders are event-driven and content-gated, never periodic. Pi draws inline
 * rather than on an alternate screen, so every repaint snaps the terminal
 * viewport back to the bottom. A background run can last minutes - exactly when
 * the user wants to scroll back and read - so a timer-driven repaint makes the
 * scrollback unusable for as long as the run lives. Vanilla pi only repaints
 * while it is itself working, and this widget must not be noisier than that
 * while pi sits idle.
 *
 * That constraint rules out animation here: a spinner or a live elapsed clock
 * would change the painted text on its own schedule and force exactly the
 * repaints this avoids. The row therefore carries only event-driven fields, and
 * elapsed time lives in the tool row and the /agents navigator instead.
 *
 * Repaint frequency is only half of it. Removing lines from pi's inline buffer
 * while an overlay (the /agents navigator, an approval prompt) is composited
 * shifts every overlay line up, which lands above pi-tui's previous viewport top
 * and forces the same scrollback-erasing full redraw. See `paint`.
 */

import { truncateToWidth, type TUI } from "@earendil-works/pi-tui";
import type { ExtensionContext } from "@earendil-works/pi-coding-agent";
import type { SpawnedRun, SubagentRunner } from "../runner/runner.js";
import type { StartedWorkflow } from "../workflow/launch.js";
import type { SubagentEvent, SubagentHandle, UsageSummary, WorkflowPhase } from "../types.js";
import { sumUsage } from "../store/run-store.js";
import { reportDiagnostic } from "../diagnostics.js";
import { errorMessage } from "../util.js";
import {
  childLabel,
  countStatuses,
  formatDuration,
  formatTokenUsage,
  spinnerFrame,
  statusGlyph,
  type StatusCounts,
  type ThemeLike,
} from "./format.js";
import { sanitizeTerminalText } from "./sanitize.js";

const WIDGET_KEY = "subagent-workflow";
const STATUS_KEY = "subagent-workflow";
const FALLBACK_ROW_CAP = 6;
const LABEL_WIDTH = 26;
const REFRESH_MS = 100;

type WidgetCtx = Pick<ExtensionContext, "ui" | "hasUI">;

interface TrackedRun {
  kind: "subagent" | "workflow";
  /** Sanitized display label: workflow meta name, or the child's label for a direct spawn. */
  label: string;
  /** Declared phase skeleton (workflow runs only). */
  phases: WorkflowPhase[];
  /** Phase of the most recently admitted child (workflow runs only). */
  currentPhase?: string;
  handles: SubagentHandle[];
  seenHandles: Set<string>;
  startedAt: number;
  usage: Map<string, UsageSummary>;
  unsubscribers: Array<() => void>;
}

/** Per-run view used by the pure line renderer. */
export interface WidgetRunView {
  kind?: "subagent" | "workflow";
  label: string;
  /** Formatted current-phase segment for workflow rows, e.g. "Research (2/3)". */
  phase?: string;
  counts: StatusCounts;
  startedAt: number;
  usage: UsageSummary;
  /** Whether this row owns keyboard selection from the editor. */
  selected?: boolean;
}

/** Run target selected from the below-editor widget. */
export interface ActiveRunTarget {
  runId: string;
  childId?: string;
}

function phaseView(run: Pick<TrackedRun, "kind" | "phases" | "currentPhase">): string | undefined {
  if (run.kind !== "workflow" || !run.currentPhase) return undefined;
  const index = run.phases.findIndex((phase) => phase.title === run.currentPhase);
  const position = index >= 0 && run.phases.length > 1 ? ` (${index + 1}/${run.phases.length})` : "";
  return `${sanitizeTerminalText(run.currentPhase)}${position}`;
}

function runUsage(run: TrackedRun): UsageSummary {
  return sumUsage(run.usage.values());
}

/** Keep the widget near one quarter of the terminal, or retain the old cap when height is unavailable. */
export function statusWidgetRowCap(terminalRows: number | undefined): number {
  if (terminalRows === undefined || !Number.isFinite(terminalRows) || terminalRows < 1) return FALLBACK_ROW_CAP;
  return Math.max(1, Math.floor(terminalRows / 4));
}

function summaryParts(runs: WidgetRunView[]): string[] {
  const running = runs.reduce((sum, run) => sum + run.counts.running, 0);
  const queued = runs.reduce((sum, run) => sum + run.counts.pending, 0);
  const workflows = runs.filter((run) => run.kind === "workflow").length;
  const parts: string[] = [];
  if (workflows > 0) parts.push(`${workflows} workflow${workflows === 1 ? "" : "s"}`);
  if (running > 0) parts.push(`${running} running`);
  if (queued > 0) parts.push(`${queued} queued`);
  return parts;
}

/** Build the widget lines from active run views. Pure and unit tested. */
export function renderWidgetLines(runs: WidgetRunView[], theme: ThemeLike, width: number, maxRows = FALLBACK_ROW_CAP): string[] {
  if (runs.length === 0) return [];
  // Never exceed the host-supplied width: pi-tui kills the process on any
  // over-wide line, so there is no minimum layout width worth crashing for.
  const cap = Math.max(1, width);
  const selectionHint = runs.some((run) => run.selected) ? " · ↑↓ select · enter view" : " · ↓ select";
  const header = `${theme.fg("accent", "▸")} ${theme.bold("agents")} ${theme.fg("dim", `${summaryParts(runs).join(" · ")}${selectionHint}`)}`;
  const lines: string[] = [truncateToWidth(header, cap)];

  for (const run of runs.slice(0, maxRows)) {
    const cursor = run.selected ? theme.fg("accent", "❯") : " ";
    const glyph = statusGlyph("running", theme, 0, false);
    const rawLabel = truncateToWidth(run.label, LABEL_WIDTH, "…", true);
    const label = run.selected ? theme.fg("accent", theme.bold(rawLabel)) : rawLabel;
    const phase = run.phase ? `${theme.fg("dim", run.phase)}  ` : "";
    const progress = theme.fg("dim", run.counts.total > 0 ? `${run.counts.done}/${run.counts.total}` : "starting");
    const usage = theme.fg("dim", formatTokenUsage(run.usage));
    lines.push(truncateToWidth(`${cursor} ${glyph} ${label}  ${phase}${progress}  ${usage}`, cap));
  }
  if (runs.length > maxRows) lines.push(truncateToWidth(theme.fg("dim", `  +${runs.length - maxRows} more runs`), cap));
  return lines;
}

/**
 * Identity of the painted content, excluding anything clock-derived. Two views
 * with the same signature render identical text, so a repaint would be a no-op.
 *
 * JSON rather than joined delimiters: a run label is arbitrary text, so any
 * separator has to be one a label cannot contain. Encoding sidesteps the question
 * instead of answering it, and keeps the source free of the raw control
 * characters that previously made this file register as binary to grep.
 */
function widgetSignature(runs: WidgetRunView[]): string {
  return JSON.stringify(runs.map((run) =>
    [run.kind, run.label, run.phase ?? "", run.counts.done, run.counts.total, run.counts.running, run.counts.failed, formatTokenUsage(run.usage), run.selected === true]));
}

export class SubagentStatusWidget {
  private ctx: WidgetCtx | undefined;
  private readonly runs = new Map<string, TrackedRun>();
  private tui: TUI | undefined;
  private registered = false;
  private lastStatus: string | undefined;
  /** Painted-content identity, so an unchanged update skips the repaint. */
  private lastSignature: string | undefined;
  private selectedRunId: string | undefined;
  /** Line count last painted, so a shrink can be held back under an overlay. */
  private painted = 0;
  private enabled = true;
  private unsubscribeSpawns: (() => void) | undefined;

  constructor(private readonly runner?: Pick<SubagentRunner, "subscribeSpawns" | "isRunActive">) {
    this.unsubscribeSpawns = runner?.subscribeSpawns((run) => this.observeSpawn(run));
  }

  /**
   * Show a workflow row from launch, before its first child spawns, and keep
   * it while the run controller is registered - a workflow between agent
   * batches (or replaying its journal) has no live child but is still running.
   */
  observeWorkflowStarted(started: StartedWorkflow, ctx: WidgetCtx): void {
    if (!ctx.hasUI) return;
    this.setCtx(ctx);
    const existing = this.runs.get(started.runId);
    if (existing) {
      existing.label = sanitizeTerminalText(started.name);
      existing.phases = started.phases;
    } else {
      this.runs.set(started.runId, {
        kind: "workflow",
        label: sanitizeTerminalText(started.name),
        phases: started.phases,
        currentPhase: started.phases[0]?.title,
        handles: [],
        seenHandles: new Set(),
        startedAt: Date.now(),
        usage: new Map(),
        unsubscribers: [],
      });
    }
    this.safeUpdate();
  }

  /** Merge workflow child spawns into their run's row as agent() calls admit them. */
  private observeSpawn(spawned: SpawnedRun): void {
    const run = this.runs.get(spawned.runId);
    if (!run || run.kind !== "workflow") return;
    const handle = spawned.handle;
    if (run.seenHandles.has(handle.id)) return;
    run.seenHandles.add(handle.id);
    run.handles.push(handle);
    run.unsubscribers.push(handle.subscribe((event) => this.onEvent(spawned.runId, run.usage, event)));
    if (handle.spec.phase !== undefined) run.currentPhase = handle.spec.phase;
    this.safeUpdate();
  }

  /** Apply display settings immediately, including to an already-visible widget. */
  configure(enabled: boolean): void {
    this.enabled = enabled;
    this.safeUpdate();
  }

  /** Register a spawned run for live display. No-op without dialog-capable UI. */
  track(runId: string, handle: SubagentHandle, ctx: WidgetCtx): void {
    if (!ctx.hasUI) return;
    this.setCtx(ctx);
    const usage = new Map<string, UsageSummary>();
    this.runs.set(runId, {
      kind: "subagent",
      // Computed once: the label is fixed for the run's life, and childLabel
      // rescans the whole prompt - not worth repeating on every repaint.
      label: sanitizeTerminalText(childLabel(handle.spec)),
      phases: [],
      handles: [handle],
      seenHandles: new Set([handle.id]),
      startedAt: handle.startedAt,
      usage,
      unsubscribers: [handle.subscribe((event) => this.onEvent(runId, usage, event))],
    });
    this.safeUpdate();
  }

  /** Drop all runs and clear the widget. Called on session shutdown. */
  dispose(): void {
    this.unsubscribeSpawns?.();
    this.unsubscribeSpawns = undefined;
    for (const run of this.runs.values()) run.unsubscribers.forEach((fn) => fn());
    this.runs.clear();
    this.selectedRunId = undefined;
    this.lastSignature = undefined;
    if (this.ctx) {
      try { this.ctx.ui.setWidget(WIDGET_KEY, undefined); } catch (error) { this.logFailure(error); }
      try { this.ctx.ui.setStatus(STATUS_KEY, undefined); } catch (error) { this.logFailure(error); }
    }
    this.registered = false;
    this.tui = undefined;
    this.painted = 0;
    this.lastStatus = undefined;
  }

  private setCtx(ctx: WidgetCtx): void {
    if (ctx === this.ctx) return;
    this.ctx = ctx;
    this.registered = false;
    this.tui = undefined;
    this.painted = 0;
    this.lastStatus = undefined;
  }

  /**
   * Move through visible active runs without wrapping at either end.
   *
   * Moving up from the first row clears widget selection and returns false so
   * the editor can handle the same key, putting focus back in the input.
   */
  selectRun(delta: -1 | 1): boolean {
    if (!this.enabled) return false;
    const runs = this.selectableRuns();
    if (runs.length === 0) {
      this.selectedRunId = undefined;
      return false;
    }
    const current = runs.findIndex(([runId]) => runId === this.selectedRunId);
    if (current < 0 && delta < 0) {
      this.selectedRunId = undefined;
      this.safeUpdate();
      return false;
    }
    const next = current < 0 ? 0 : current + delta;
    if (next < 0) {
      this.selectedRunId = undefined;
      this.safeUpdate();
      return false;
    }
    this.selectedRunId = runs[Math.min(next, runs.length - 1)]![0];
    this.safeUpdate();
    return true;
  }

  /** Whether the editor currently owns a visible run selection. */
  hasSelectedRun(): boolean {
    return this.enabled && this.selectableRuns().some(([runId]) => runId === this.selectedRunId);
  }

  /** Consume the current widget selection and return its navigator target. */
  takeSelectedRun(): ActiveRunTarget | undefined {
    const selectedRunId = this.selectedRunId;
    const selected = this.selectableRuns().find(([runId]) => runId === selectedRunId);
    this.selectedRunId = undefined;
    this.safeUpdate();
    if (!selected) return undefined;
    const [, run] = selected;
    const childId = run.kind === "subagent" ? run.handles[0]?.id : undefined;
    return childId === undefined ? { runId: selected[0] } : { runId: selected[0], childId };
  }

  /** Leave widget selection mode without changing editor text. */
  clearSelectedRun(): void {
    if (this.selectedRunId === undefined) return;
    this.selectedRunId = undefined;
    this.safeUpdate();
  }

  private selectableRuns(): Array<[string, TrackedRun]> {
    return this.activeRuns().slice(0, statusWidgetRowCap(this.tui?.terminal?.rows));
  }

  private activeRuns(): Array<[string, TrackedRun]> {
    const active: Array<[string, TrackedRun]> = [];
    for (const [runId, run] of this.runs) {
      if (!this.runIsActive(runId, run)) {
        this.pruneRun(runId);
        continue;
      }
      active.push([runId, run]);
    }
    return active.sort(([, left], [, right]) => left.startedAt - right.startedAt);
  }

  private onEvent(runId: string, usage: Map<string, UsageSummary>, event: SubagentEvent): void {
    if (event.type === "usage") usage.set(event.id, { ...event.usage });
    if (event.type === "result") usage.set(event.id, { ...event.result.usage });
    if (event.type === "status" || event.type === "result") this.pruneRun(runId);
    this.safeUpdate();
  }

  /**
   * An idle run leaves the widget immediately - except a workflow whose run
   * controller is still registered: it is between agent batches or replaying,
   * and its row must not flicker away.
   */
  private pruneRun(runId: string): void {
    const run = this.runs.get(runId);
    if (!run || this.runIsActive(runId, run)) return;
    run.unsubscribers.forEach((fn) => fn());
    this.runs.delete(runId);
    if (this.selectedRunId === runId) this.selectedRunId = undefined;
  }

  private runIsActive(runId: string, run: TrackedRun): boolean {
    const counts = countStatuses(run.handles.map((handle) => handle.status));
    if (counts.active) return true;
    return run.kind === "workflow" && (this.runner?.isRunActive(runId) ?? false);
  }

  private views(): WidgetRunView[] {
    return this.activeRuns().map(([runId, run]) => ({
      kind: run.kind,
      label: run.label,
      phase: phaseView(run),
      counts: countStatuses(run.handles.map((handle) => handle.status)),
      startedAt: run.startedAt,
      usage: runUsage(run),
      selected: runId === this.selectedRunId,
    }));
  }

  private update(): void {
    if (!this.ctx) return;
    if (!this.enabled) {
      this.hide();
      return;
    }
    const views = this.views();

    if (views.length === 0) {
      this.hide();
      return;
    }

    const status = summaryParts(views).join(", ");
    if (status !== this.lastStatus) {
      this.ctx.ui.setStatus(STATUS_KEY, status);
      this.lastStatus = status;
    }

    // Repaint only when the painted text would actually differ. Event floods
    // (usage ticks especially) otherwise repaint identical content and snap the
    // viewport for no visible gain.
    const signature = widgetSignature(views);
    const changed = signature !== this.lastSignature;
    this.lastSignature = signature;

    if (!this.registered) {
      this.ctx.ui.setWidget(
        WIDGET_KEY,
        (tui, theme) => {
          this.tui = tui;
          return {
            render: (width: number) => this.paint(renderWidgetLines(
              this.views(),
              theme,
              Math.max(1, width),
              statusWidgetRowCap(tui.terminal?.rows),
            ), tui),
            invalidate: () => {
              this.registered = false;
              this.tui = undefined;
            },
          };
        },
        { placement: "belowEditor" },
      );
      this.registered = true;
    } else if (changed) {
      this.tui?.requestRender();
    }
  }

  /**
   * Hold the painted line count steady while an overlay is mounted.
   *
   * Dropping lines from pi's inline buffer under a composited overlay moves
   * every overlay line up, so the first changed line lands above pi-tui's
   * previous viewport top and it falls back to a full redraw - which erases the
   * terminal's scrollback. Measured against pi-tui: a one-line shrink is always
   * safe (overlays reserve a one-row margin), two lines erase at 31 rows or
   * fewer, three at any height. Growth is always safe, and so is any shrink with
   * no overlay up.
   *
   * Padding back to the previous height costs nothing visible, because the
   * overlay is covering these rows anyway, and both pi-tui overlay-close paths
   * request a render - so the real shrink happens a frame later with no overlay
   * mounted, on the safe path.
   */
  private paint(lines: string[], tui: Pick<TUI, "hasOverlay">): string[] {
    const held = tui.hasOverlay() && lines.length < this.painted
      ? [...lines, ...Array.from({ length: this.painted - lines.length }, () => "")]
      : lines;
    this.painted = held.length;
    return held;
  }

  private hide(): void {
    this.selectedRunId = undefined;
    if (!this.ctx) return;
    if (this.registered) {
      // Unregistering removes the widget's lines outright, which is the same
      // buffer shrink paint() defers - and it bypasses paint() entirely. Stay
      // mounted until the overlay closes; paint() then returns no lines, which
      // looks identical to being unregistered, and the next update() or
      // dispose() reclaims the registration.
      if (this.tui?.hasOverlay()) {
        this.tui.requestRender();
      } else {
        this.ctx.ui.setWidget(WIDGET_KEY, undefined);
        this.registered = false;
        this.tui = undefined;
        this.painted = 0;
      }
    }
    if (this.lastStatus !== undefined) {
      this.ctx.ui.setStatus(STATUS_KEY, undefined);
      this.lastStatus = undefined;
    }
    this.lastSignature = undefined;
  }

  private safeUpdate(): void {
    try {
      this.update();
    } catch (error) {
      for (const run of this.runs.values()) run.unsubscribers.forEach((fn) => fn());
      this.runs.clear();
      this.selectedRunId = undefined;
      this.lastSignature = undefined;
      this.registered = false;
      this.tui = undefined;
      this.painted = 0;
      this.lastStatus = undefined;
      try { this.ctx?.ui.setWidget(WIDGET_KEY, undefined); } catch { /* host UI already failed */ }
      try { this.ctx?.ui.setStatus(STATUS_KEY, undefined); } catch { /* host UI already failed */ }
      this.logFailure(error);
    }
  }

  private logFailure(error: unknown): void {
    reportDiagnostic(`[subagent-workflow] status widget failed: ${errorMessage(error)}`);
  }
}
