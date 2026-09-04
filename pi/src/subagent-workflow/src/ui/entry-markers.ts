/**
 * Transcript completion markers.
 *
 * Phase 2a appends context-excluded custom entries for run lifecycle. These
 * renderers turn them into compact, native-looking transcript records so history
 * reads coherently, including on session resume.
 */

import { truncateToWidth } from "@earendil-works/pi-tui";
import type { ExtensionAPI, Theme } from "@earendil-works/pi-coding-agent";
import type { SubagentStatus, UsageSummary } from "../types.js";
import { reportDiagnostic } from "../diagnostics.js";
import { errorMessage } from "../util.js";
import { countStatuses, formatDuration, formatTokenUsage, statusGlyph, type ThemeLike } from "./format.js";
import { linesComponent } from "./component.js";
import { sanitizeTerminalText } from "./sanitize.js";

interface RunStartedData {
  runId: string;
  runDir: string;
  /** Subagent runs carry child ids/labels; workflow runs carry phases instead. */
  childIds?: string[];
  labels?: string[];
  phases?: { title: string }[];
}

interface RunCompletedData {
  runId: string;
  runDir: string;
  perChild?: { id: string; status: SubagentStatus; label: string }[];
  usageTotals?: UsageSummary;
  durationMs?: number;
  phases?: { title: string }[];
}

/** Lifecycle markers are auxiliary and must never strand a run or hide its result. */
export function appendEntrySafely(pi: ExtensionAPI, type: string, data: unknown): void {
  try {
    pi.appendEntry(type, data);
  } catch (error) {
    reportDiagnostic(`[subagent-workflow] parent transcript marker failed: ${sanitizeTerminalText(errorMessage(error))}`);
  }
}

/**
 * Entry data comes straight off the session JSONL, so it is the least trusted
 * input any renderer here sees: a truncated write, or an entry appended by a
 * different version of this extension, arrives with whatever shape it has. A
 * throw would land in pi's render loop and kill the TUI on every resume, so the
 * arrays are checked rather than assumed.
 */
function textList(value: unknown): string[] | undefined {
  return Array.isArray(value) ? value.map((item) => sanitizeTerminalText(typeof item === "string" ? item : String(item))) : undefined;
}

export function renderRunStarted(data: RunStartedData, theme: ThemeLike, width: number): string[] {
  const cap = Math.max(20, width);
  const labelList = textList(data.labels);
  if (!labelList) {
    // Phase titles are workflow-authored; sanitize before they hit the terminal.
    const phases = (Array.isArray(data.phases) ? data.phases : [])
      .map((phase) => sanitizeTerminalText(phase?.title ?? "")).join(" → ");
    const text = `▸ workflow run ${data.runId} started${phases ? `: ${phases}` : ""}`;
    return [truncateToWidth(theme.fg("dim", text), cap)];
  }
  const count = labelList.length;
  const text = `▸ subagent run ${data.runId} started · ${count} agent${count === 1 ? "" : "s"}: ${labelList.join(", ")}`;
  return [truncateToWidth(theme.fg("dim", text), cap)];
}

export function renderRunCompleted(data: RunCompletedData, theme: ThemeLike, width: number): string[] {
  const cap = Math.max(20, width);
  if (!Array.isArray(data.perChild) || !data.usageTotals) {
    const marker = theme.fg("success", "●");
    const text = `${marker} ${theme.bold("workflow")} ${data.runId} ${theme.fg("dim", "· completed")}`;
    return [truncateToWidth(text, cap)];
  }
  const counts = countStatuses(data.perChild.map((child) => child?.status));
  const usage = formatTokenUsage(data.usageTotals);
  const marker = theme.fg(counts.failed > 0 ? "error" : counts.aborted > 0 ? "warning" : "success", "●");
  const summary =
    `${marker} ${theme.bold("subagent")} ${data.runId} ${theme.fg("dim", "·")} ` +
    `${counts.completed}/${counts.total} done` +
    (counts.failed > 0 ? theme.fg("error", ` · ${counts.failed} failed`) : "") +
    (counts.aborted > 0 ? theme.fg("warning", ` · ${counts.aborted} aborted`) : "") +
    ` ${theme.fg("dim", "·")} ${theme.fg("dim", usage)} ${theme.fg("dim", "·")} ${theme.fg("dim", formatDuration(data.durationMs ?? 0))}`;

  const glyphs = data.perChild
    .map((child) => `${statusGlyph(child?.status, theme, 0, false)} ${sanitizeTerminalText(child?.label ?? "")}`)
    .join(theme.fg("dim", "   "));
  return [truncateToWidth(summary, cap), truncateToWidth(`  ${glyphs}`, cap)];
}

/** Register both lifecycle-entry renderers. Safe to call in any mode. */
export function registerEntryMarkers(pi: ExtensionAPI): void {
  pi.registerEntryRenderer<RunStartedData>("subagent-workflow:run-started", (entry, _options, theme: Theme) => {
    if (!entry.data) return undefined;
    return linesComponent((width) => renderRunStarted(entry.data as RunStartedData, theme, width), "run-started marker");
  });
  pi.registerEntryRenderer<RunCompletedData>("subagent-workflow:run-completed", (entry, _options, theme: Theme) => {
    if (!entry.data) return undefined;
    return linesComponent((width) => renderRunCompleted(entry.data as RunCompletedData, theme, width), "run-completed marker");
  });
}
