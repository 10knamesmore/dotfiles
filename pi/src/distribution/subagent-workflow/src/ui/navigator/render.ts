/**
 * Pure line builders for the navigator's run list (level 1) and run detail
 * (level 2). Each returns a fixed-height block: a header, a scroll-windowed body
 * that keeps the cursor visible, and overflow indicators. The agent-detail view
 * (level 3) is a live Component in agent-view.ts.
 *
 * Rendering is theme-role only and ANSI-safe (truncateToWidth); it depends only
 * on data + cursor + width, so it is deterministic under the PLAIN theme.
 */

import { truncateToWidth } from "@earendil-works/pi-tui";
import { clamp, countStatuses, formatDuration, formatTokenUsage, hasTokenUsage, modelEffort, padStart, statusGlyph, type ThemeLike } from "../format.js";
import { sumUsage } from "../../store/run-store.js";
import { sanitizeTerminalText } from "../sanitize.js";
import type { FilterMode } from "./controls.js";
import { orderedChildren } from "./controls.js";
import type { ChildRow, RunDetail, RunSummary } from "./store-read.js";

const LABEL_MAX = 30;
const MODEL_MAX = 22;
const ELAPSED_WIDTH = 6;
const USAGE_WIDTH = 20;

/** Compact relative age: "12s", "5m", "3h", "2d". */
function formatAge(ms: number): string {
  const seconds = Math.max(0, ms) / 1000;
  if (seconds < 60) return `${Math.round(seconds)}s`;
  if (seconds < 3600) return `${Math.round(seconds / 60)}m`;
  if (seconds < 86_400) return `${Math.round(seconds / 3600)}h`;
  return `${Math.round(seconds / 86_400)}d`;
}

interface ScrollWindow {
  start: number;
  count: number;
  moreAbove: boolean;
  moreBelow: boolean;
}

/** A window of up to `cap` rows that keeps `active` visible. */
export function scrollWindow(total: number, active: number, cap: number): ScrollWindow {
  if (total <= cap) return { start: 0, count: total, moreAbove: false, moreBelow: false };
  let start = clamp(active - Math.floor(cap / 2), 0, total - cap);
  if (active < start) start = active;
  if (active >= start + cap) start = active - cap + 1;
  return { start, count: cap, moreAbove: start > 0, moreBelow: start + cap < total };
}

function selector(selected: boolean): string {
  return selected ? "❯ " : "  ";
}

/** Level 1: the run list. `cursor` indexes `rows`; output is at most `maxLines` tall. */
export function renderRunList(rows: RunSummary[], cursor: number, theme: ThemeLike, width: number, now: number, maxLines: number): string[] {
  const cap = Math.max(20, width);
  const active = rows.filter((row) => row.status === "running" || row.status === "pending").length;
  const header = `${theme.bold("Agents")} ${theme.fg("dim", `· ${rows.length} run${rows.length === 1 ? "" : "s"}${active > 0 ? ` · ${active} active` : ""}`)}`;
  const lines: string[] = [truncateToWidth(header, cap)];
  if (rows.length === 0) {
    lines.push(theme.fg("dim", "  No runs yet. Spawn a subagent or start a workflow."));
    return lines;
  }
  const rowCap = Math.max(1, maxLines - lines.length - 2);
  const window = scrollWindow(rows.length, cursor, rowCap);
  if (window.moreAbove) lines.push(theme.fg("dim", "  ↑ more"));
  for (let i = 0; i < window.count; i += 1) {
    const index = window.start + i;
    const row = rows[index]!;
    lines.push(truncateToWidth(renderRunRow(row, index === cursor, theme, now), cap));
  }
  if (window.moreBelow) lines.push(theme.fg("dim", "  ↓ more"));
  return lines;
}

function renderRunRow(row: RunSummary, selected: boolean, theme: ThemeLike, now: number): string {
  const mark = selector(selected);
  if (row.corrupt) return theme.fg("dim", `${mark}· ${sanitizeTerminalText(row.label)} ${sanitizeTerminalText(row.runId)}`);
  const unhealthyCompletion = row.kind === "workflow" && row.status === "completed" && (row.failed > 0 || row.aborted > 0);
  const glyph = unhealthyCompletion ? theme.fg("warning", "⚠") : statusGlyph(row.status, theme, now, false);
  const safeLabel = sanitizeTerminalText(row.label);
  const label = selected ? theme.fg("accent", theme.bold(safeLabel)) : theme.bold(safeLabel);
  const outcome = unhealthyCompletion
    ? ["completed", `${row.completed} ok`, row.failed > 0 ? `${row.failed} failed` : "", row.aborted > 0 ? `${row.aborted} aborted` : ""]
    : [`${row.done}/${row.total}`];
  const meta = [...outcome, hasTokenUsage(row.usage) ? formatTokenUsage(row.usage) : "", formatAge(now - row.createdAt)]
    .filter(Boolean)
    .join(" · ");
  return `${mark}${glyph} ${label}  ${theme.fg("dim", `${row.kind} · ${meta}`)}`;
}

interface DisplayRow {
  text: string;
  /** Index in the ordered child list when this row is a selectable agent. */
  childIndex?: number;
}

type WorkflowLayoutRow =
  | { kind: "phase"; text: string; hasAgents: boolean }
  | { kind: "log"; text: string; indent: number }
  | { kind: "child"; child: ChildRow; childIndex: number };

/** Move roughly one visible detail viewport while accounting for workflow headings and log rows. */
interface RunDetailPage {
  cursor: number;
  row: number;
}

export function pageRunDetail(
  detail: RunDetail,
  cursor: number,
  filter: FilterMode,
  delta: number,
  pageRows: number,
  currentRow?: number,
): RunDetailPage {
  const order = orderedChildren(detail, filter);
  if (detail.kind !== "workflow") {
    const next = order.length === 0 ? 0 : clamp(cursor + delta * Math.max(1, pageRows), 0, order.length - 1);
    return { cursor: next, row: next };
  }
  const layout = buildWorkflowLayout(detail, order);
  const selectedRow = Math.max(0, layout.findIndex((row) => row.kind === "child" && row.childIndex === cursor));
  const anchor = currentRow === undefined ? selectedRow : clamp(currentRow, 0, Math.max(0, layout.length - 1));
  const targetRow = clamp(anchor + delta * Math.max(1, pageRows), 0, Math.max(0, layout.length - 1));
  let best = cursor;
  let bestDistance = Number.POSITIVE_INFINITY;
  for (let index = 0; index < layout.length; index += 1) {
    const row = layout[index]!;
    if (row.kind !== "child") continue;
    const distance = Math.abs(index - targetRow);
    if (distance < bestDistance || (distance === bestDistance && ((delta >= 0) === (index > targetRow)))) {
      best = row.childIndex;
      bestDistance = distance;
    }
  }
  return { cursor: best, row: targetRow };
}

/** Level 2: run detail. `cursor` indexes the ordered (filtered) child list; output is at most `maxLines` tall. */
export function renderRunDetail(
  detail: RunDetail,
  cursor: number,
  filter: FilterMode,
  theme: ThemeLike,
  width: number,
  now: number,
  maxLines: number,
  focusRow?: number,
): string[] {
  const cap = Math.max(20, width);
  const order = orderedChildren(detail, filter);
  const lines: string[] = renderDetailHeader(detail, order.length, filter, theme, cap);
  if (detail.corrupt) {
    lines.push(theme.fg("dim", "  This run directory could not be read."));
    return lines;
  }
  const body = buildDetailBody(detail, order, cursor, theme, cap, now);
  if (body.length === 0) {
    lines.push(theme.fg("dim", "  No agents match this filter."));
    return lines;
  }
  const rowCap = Math.max(1, maxLines - lines.length - 2);
  const cursorRow = body.findIndex((row) => row.childIndex === cursor);
  const activeRow = focusRow === undefined
    ? (cursorRow < 0 ? 0 : cursorRow)
    : clamp(focusRow, 0, Math.max(0, body.length - 1));
  const window = scrollWindow(body.length, activeRow, rowCap);
  if (window.moreAbove) lines.push(theme.fg("dim", "  ↑ more"));
  for (let i = 0; i < window.count; i += 1) lines.push(body[window.start + i]!.text);
  if (window.moreBelow) lines.push(theme.fg("dim", "  ↓ more"));
  return lines;
}

function renderDetailHeader(detail: RunDetail, shown: number, filter: FilterMode, theme: ThemeLike, cap: number): string[] {
  const total = detail.children.length;
  const usage = sumUsage(detail.children.map((child) => child.usage));
  const counts = countStatuses(detail.children.map((child) => child.status));
  const filterNote = filter === "all" ? "" : ` · filter: ${filter} (${shown})`;
  const title = `${theme.bold(sanitizeTerminalText(detail.label))} ${theme.fg("dim", `· ${detail.kind} · ${sanitizeTerminalText(detail.runId)}`)}`;
  const health = detail.kind === "workflow" && detail.status === "completed" && (counts.failed > 0 || counts.aborted > 0)
    ? `completed · ${counts.completed} ok${counts.failed > 0 ? ` · ${counts.failed} failed` : ""}${counts.aborted > 0 ? ` · ${counts.aborted} aborted` : ""}`
    : `${counts.done}/${total} done · ${detail.status}`;
  const stats = theme.fg("dim", `${health} · ${formatTokenUsage(usage)}${filterNote}`);
  return [truncateToWidth(title, cap), truncateToWidth(stats, cap)];
}

function buildDetailBody(detail: RunDetail, order: ChildRow[], cursor: number, theme: ThemeLike, cap: number, now: number): DisplayRow[] {
  if (detail.kind !== "workflow") {
    return order.map((child, index) => ({ text: truncateToWidth(renderChildRow(child, index === cursor, theme, now), cap), childIndex: index }));
  }
  return buildWorkflowBody(detail, order, cursor, theme, cap, now);
}

function buildWorkflowBody(detail: RunDetail, order: ChildRow[], cursor: number, theme: ThemeLike, cap: number, now: number): DisplayRow[] {
  return buildWorkflowLayout(detail, order).map((row): DisplayRow => {
    if (row.kind === "phase") {
      const heading = `${row.hasAgents ? "▸" : "▹"} ${sanitizeTerminalText(row.text)}`;
      return { text: truncateToWidth(theme.fg(row.hasAgents ? "accent" : "dim", heading), cap) };
    }
    if (row.kind === "log") {
      return { text: truncateToWidth(theme.fg("dim", `${" ".repeat(row.indent)}▪ ${sanitizeTerminalText(row.text)}`), cap) };
    }
    return {
      text: truncateToWidth(`  ${renderChildRow(row.child, row.childIndex === cursor, theme, now)}`, cap),
      childIndex: row.childIndex,
    };
  });
}

/** Structural rows shared by rendering and viewport-aware keyboard paging. */
function buildWorkflowLayout(detail: RunDetail, order: ChildRow[]): WorkflowLayoutRow[] {
  const rows: WorkflowLayoutRow[] = [];
  const indexOf = new Map(order.map((child, index) => [child, index] as const));
  const phaseTitles = detail.phases.map((phase) => phase.title);
  const groups = new Map<string, ChildRow[]>();
  const noPhase = "(no phase)";
  for (const child of order) {
    const key = child.phase && phaseTitles.includes(child.phase) ? child.phase : noPhase;
    const group = groups.get(key);
    if (group) group.push(child);
    else groups.set(key, [child]);
  }
  // A workflow's declared phases are its execution skeleton. Keep every phase
  // visible even before its first child is admitted, rather than deriving the
  // outline from children that happen to exist at this instant.
  const visibleTitles = [...phaseTitles];
  if (groups.has(noPhase)) visibleTitles.push(noPhase);
  const startedPhases = new Set(
    detail.children
      .map((child) => child.phase)
      .filter((phase): phase is string => phase !== undefined && phaseTitles.includes(phase)),
  );
  for (const line of detail.narrator) {
    if (line.kind === "phase" && phaseTitles.includes(line.text)) startedPhases.add(line.text);
  }

  // Append order, not wall-clock timestamps, defines phase attribution. Several
  // synchronous events commonly share one ISO millisecond.
  const firstPhaseIndex = detail.narrator.findIndex((line) => line.kind === "phase");
  for (const line of detail.narrator.slice(0, firstPhaseIndex < 0 ? detail.narrator.length : firstPhaseIndex)) {
    if (line.kind === "log") rows.push({ kind: "log", text: line.text, indent: 2 });
  }
  for (const title of visibleTitles) {
    rows.push({ kind: "phase", text: title, hasAgents: title === noPhase ? groups.has(title) : startedPhases.has(title) });
    for (const line of narratorFor(detail, title)) rows.push({ kind: "log", text: line, indent: 4 });
    for (const child of groups.get(title) ?? []) {
      const index = indexOf.get(child) ?? -1;
      rows.push({ kind: "child", child, childIndex: index });
    }
  }
  return rows;
}

/** Log lines observed while this phase was current, based on event-file order. */
function narratorFor(detail: RunDetail, title: string): string[] {
  const logs: string[] = [];
  let current: string | undefined;
  for (const line of detail.narrator) {
    if (line.kind === "phase") current = line.text;
    else if (current === title) logs.push(line.text);
  }
  return logs;
}

function renderChildRow(child: ChildRow, selected: boolean, theme: ThemeLike, now: number): string {
  const glyph = statusGlyph(child.status, theme, now, child.status === "running");
  const label = truncateToWidth(sanitizeTerminalText(child.label), LABEL_MAX, "…", true);
  const styledLabel = selected ? theme.fg("accent", theme.bold(label)) : label;
  const cells = [`${selector(selected)}${glyph} ${styledLabel}`];
  // modelEffort already fits MODEL_MAX; truncateToWidth pads it to a fixed column.
  if (child.model) cells.push(theme.fg("dim", truncateToWidth(sanitizeTerminalText(modelEffort(child.model, child.thinking, MODEL_MAX)), MODEL_MAX, "…", true)));
  // Elapsed and token usage are right-aligned in fixed columns so the
  // trailing activity/result text starts at the same column on every row.
  const elapsed = child.startedAt !== undefined ? formatDuration((child.endedAt ?? now) - child.startedAt) : "";
  if (elapsed) cells.push(theme.fg("dim", padStart(elapsed, ELAPSED_WIDTH)));
  cells.push(theme.fg("dim", padStart(formatTokenUsage(child.usage), USAGE_WIDTH)));
  const trailing = child.error
    ? theme.fg("error", sanitizeTerminalText(child.error))
    : theme.fg("dim", sanitizeTerminalText(child.status === "running" ? child.activity ?? "" : child.resultLine ?? ""));
  const head = cells.join("  ");
  return trailing.trim() ? `${head}  ${trailing}` : head;
}
