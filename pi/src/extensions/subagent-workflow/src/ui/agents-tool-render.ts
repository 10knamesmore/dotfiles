import type { AgentToolResult, ToolRenderResultOptions } from "@earendil-works/pi-coding-agent";
import { truncateToWidth, wrapTextWithAnsi, type Component } from "@earendil-works/pi-tui";
import type { SubagentStatus } from "../types.js";
import { linesComponent } from "./component.js";
import type { ThemeLike } from "./format.js";
import { sanitizeTerminalText } from "./sanitize.js";

export interface AgentControlRow {
  name: string;
  status: SubagentStatus;
  metadata?: string;
}

/** Frozen display data for one control operation; renderers never read live runs. */
export interface AgentsToolSnapshot {
  action: "list" | "peek" | "stop" | "send_message";
  target?: string;
  /** Actual child/run status returned by stop. */
  status?: SubagentStatus;
  rows: AgentControlRow[];
  activity?: string;
  quote?: string;
  expandedLines: string[];
}

export interface AgentsToolRenderState {
  snapshot?: AgentsToolSnapshot;
  settled?: boolean;
  isError?: boolean;
}

const COLLAPSED_ROWS = 5;
const COLLAPSED_QUOTE_LINES = 3;

function glyph(status: SubagentStatus, theme: ThemeLike): string {
  switch (status) {
    case "pending": return theme.fg("dim", "◌");
    case "running": return theme.fg("accent", "●");
    case "completed": return theme.fg("success", "✓");
    case "failed": return theme.fg("error", "✕");
    case "aborted": return theme.fg("error", "■");
  }
}

export function renderAgentsCall(
  action: string | undefined,
  theme: ThemeLike,
  state: AgentsToolRenderState,
): Component {
  // Pi invokes renderCall before renderResult, then paints both components.
  // Read the shared snapshot at paint time so the header uses this result's names.
  return linesComponent((width) => {
    const snapshot = state.snapshot;
    const status = state.isError ? "failed"
      : !state.settled ? "pending"
      : snapshot?.action === "stop" ? snapshot.status! : "completed";
    const operation = action ? ` · ${sanitizeTerminalText(action)}` : "";
    const target = snapshot?.target ? ` · ${sanitizeTerminalText(snapshot.target)}` : "";
    return wrapTextWithAnsi(
      `${glyph(status, theme)} ${theme.fg("toolTitle", theme.bold(`agents${operation}${target}`))}`,
      width,
    ).map((line) => truncateToWidth(line, width));
  }, "agents call");
}

export function renderAgentsResult(
  result: AgentToolResult<AgentsToolSnapshot | undefined>,
  options: ToolRenderResultOptions,
  theme: ThemeLike,
  context: { state: AgentsToolRenderState; isError: boolean },
): Component {
  const snapshot = result.details;
  if (snapshot) context.state.snapshot = snapshot;
  context.state.settled = !options.isPartial;
  context.state.isError = context.isError;
  return linesComponent((width) => {
    if (context.isError || !snapshot) {
      return result.content.flatMap((part) => part.type === "text"
        ? wrapTextWithAnsi(
            theme.fg(context.isError ? "error" : "toolOutput", sanitizeTerminalText(part.text, undefined, true)),
            width,
          ).map((line) => truncateToWidth(line, width))
        : []);
    }

    const lines: string[] = [];
    const rows = options.expanded ? snapshot.rows : snapshot.rows.slice(0, COLLAPSED_ROWS);
    for (const row of rows) {
      const metadata = row.metadata ? `  ${theme.fg("dim", sanitizeTerminalText(row.metadata))}` : "";
      lines.push(truncateToWidth(
        `${glyph(row.status, theme)} ${sanitizeTerminalText(row.name)}${metadata}`,
        width,
      ));
    }
    if (rows.length < snapshot.rows.length) {
      lines.push(truncateToWidth(theme.fg("dim", `+${snapshot.rows.length - rows.length}`), width));
    }
    if (snapshot.activity) {
      lines.push(truncateToWidth(theme.fg("muted", sanitizeTerminalText(snapshot.activity)), width));
    }
    if (snapshot.quote) {
      const quote = sanitizeTerminalText(snapshot.quote, undefined, true);
      const wrapped = wrapTextWithAnsi(quote, Math.max(1, width - 2));
      const shown = options.expanded ? wrapped : wrapped.slice(0, COLLAPSED_QUOTE_LINES);
      lines.push(...shown.map((line) => truncateToWidth(`${theme.fg("dim", "│")} ${line}`, width)));
      if (shown.length < wrapped.length) lines.push(truncateToWidth(theme.fg("dim", "│ …"), width));
    }
    if (options.expanded && snapshot.expandedLines.length > 0) {
      if (lines.length > 0) lines.push("");
      for (const line of snapshot.expandedLines) {
        lines.push(...wrapTextWithAnsi(theme.fg("dim", sanitizeTerminalText(line, undefined, true)), width));
      }
    }
    return lines.map((line) => truncateToWidth(line, width));
  }, "agents result");
}
