/** Terminal layout for transcript blocks; expanded content stays bounded by the history view's row budget. */
import { Markdown, Text, truncateToWidth, type MarkdownTheme } from "@earendil-works/pi-tui";
import type { ThemeLike } from "../format.js";
import type { SanitizedTerminalTail } from "../sanitize.js";
import { blockCanCollapse, blockIsExpanded, type TranscriptBlock, type ToolBlock } from "./transcript-blocks.js";

const TRANSCRIPT_MAX_LINES = 2_000;

export interface TranscriptDisplayState {
  expanded: ReadonlyMap<string, boolean>;
  selectedId?: string;
  outputOnly: boolean;
}

/** Row ranges let selection and scroll anchors follow blocks after folding or streaming changes. */
export interface TranscriptLayout {
  lines: string[];
  blocks: Array<{ id: string; start: number; end: number; collapsible: boolean }>;
}

function toolStatusLabel(block: ToolBlock): string {
  switch (block.status) {
    case "completed":
      return "✓ done";
    case "failed":
      return "✗ failed";
    case "waiting":
      return "◌ awaiting result";
    case "unavailable":
      return "? result not loaded";
  }
}

function toolColor(block: ToolBlock): string {
  return block.status === "failed"
    ? "error"
    : block.status === "completed"
      ? "success"
      : block.status === "waiting"
        ? "accent"
        : "muted";
}

function boundedBody(lines: string[], maxLines: number, elided: boolean): string[] {
  if (lines.length <= maxLines && !elided) return lines;
  return ["… earlier content not shown", ...lines.slice(-Math.max(0, maxLines - 1))].slice(0, maxLines);
}

function plainBody(body: SanitizedTerminalTail, width: number, maxLines: number): string[] {
  const lines = new Text(body.text, 0, 0).render(width);
  return boundedBody(lines, maxLines, body.elided);
}

function blockColor(block: TranscriptBlock): string {
  switch (block.kind) {
    case "thinking":
      return "thinkingMedium";
    case "tool":
      return block.status === "failed" ? "error" : "toolTitle";
    case "task":
      return "accent";
    case "notice":
      return "warning";
    case "output":
      return block.final ? "success" : "text";
  }
}

function renderBlock(
  block: TranscriptBlock,
  state: TranscriptDisplayState,
  width: number,
  maxLines: number,
  theme: ThemeLike,
  markdownTheme: MarkdownTheme,
): string[] {
  const expanded = blockIsExpanded(block, state.expanded) && !(state.outputOnly && block.kind === "tool");
  const color = blockColor(block);
  const inner = Math.max(1, width - 4);
  const fold = blockCanCollapse(block) ? (expanded ? "▾" : "▸") : "●";
  const kind = block.kind === "task" ? "TASK" : block.kind === "notice" ? "NOTE" : block.kind.toUpperCase();
  let heading = `${fold} ${theme.fg(color, theme.bold(kind))}`;
  let body: string[] = [];
  const bodyLimit = Math.max(1, maxLines - 1);
  if (block.kind === "tool") {
    // Keep the status before the path/command, so narrow terminals never hide failure.
    heading += `  ${theme.fg(toolColor(block), toolStatusLabel(block))}${state.outputOnly ? " · Space details" : ""}  ${theme.bold(block.name)}  ${block.target}`;
    if (expanded && !state.outputOnly) {
      if (block.missingCall) body.push(theme.fg("warning", "Invocation not loaded in this history window."));
      if (block.arguments) {
        body.push(theme.fg("muted", "Arguments"), ...plainBody(block.arguments, inner, bodyLimit));
      }
      body.push(theme.fg("toolTitle", "RESULT · tool response"));
      if (block.result) body.push(...plainBody(block.result, inner, bodyLimit));
      else
        body.push(
          theme.fg(
            "muted",
            block.status === "waiting"
              ? "Waiting for the tool to return."
              : "Result unavailable in the loaded history.",
          ),
        );
    }
  } else if (block.kind === "thinking") {
    heading += theme.fg(
      "muted",
      `  ${expanded ? "expanded" : "folded"}${block.body.elided ? " · partial history" : ""}`,
    );
    if (expanded) body = plainBody(block.body, inner, bodyLimit).map((line) => theme.fg("muted", line));
  } else if (block.kind === "task") {
    heading += `  ${expanded ? "Task" : (block.body.text.split("\n")[0] ?? "Task")}`;
    if (expanded) body = plainBody(block.body, inner, bodyLimit);
  } else if (block.kind === "notice") {
    body = plainBody(block.body, inner, bodyLimit).map((line) => theme.fg("warning", line));
  } else {
    heading += theme.fg(
      block.final ? "success" : "muted",
      block.final ? `  Final result${block.structured ? " · JSON" : ""}` : "  Message",
    );
    body = block.structured
      ? plainBody(block.body, inner, bodyLimit)
      : boundedBody(new Markdown(block.body.text, 0, 0, markdownTheme).render(inner), bodyLimit, block.body.elided);
  }
  body = boundedBody(body, bodyLimit, false);
  return [heading, ...body].map((line, index) => {
    const border = theme.fg(color, index === 0 && state.selectedId === block.id ? "❯ │ " : "  │ ");
    return truncateToWidth(`${border}${line}`, width);
  });
}

/** Show failed tools even in output-only mode; successful tool details and thinking start folded. */
export function renderTranscript(
  blocks: readonly TranscriptBlock[],
  state: TranscriptDisplayState,
  width: number,
  theme: ThemeLike,
  markdownTheme: MarkdownTheme,
): TranscriptLayout {
  const visible = blocks.filter(
    (block) =>
      !state.outputOnly ||
      block.kind === "output" ||
      block.kind === "notice" ||
      (block.kind === "tool" && block.status === "failed"),
  );
  const turns = new Map<string, number>();
  for (const block of blocks) if (block.turn && !turns.has(block.turn)) turns.set(block.turn, turns.size + 1);
  const rendered: Array<{ block: TranscriptBlock; lines: string[] }> = [];
  let remaining = TRANSCRIPT_MAX_LINES - 2;
  let omitted = false;
  for (let index = visible.length - 1; index >= 0; index -= 1) {
    if (remaining < 4) {
      omitted = true;
      break;
    }
    const block = visible[index]!;
    const lines = renderBlock(block, state, width, remaining - 2, theme, markdownTheme);
    rendered.push({ block, lines });
    remaining -= lines.length + 2;
  }
  const layout: TranscriptLayout = { lines: [], blocks: [] };
  if (omitted) layout.lines.push(theme.fg("muted", "… older transcript blocks not shown"));
  let lastTurn: string | undefined;
  for (const { block, lines } of rendered.reverse()) {
    if (layout.lines.length > 0) layout.lines.push("");
    if (block.turn && block.turn !== lastTurn) layout.lines.push(theme.fg("dim", `  Turn ${turns.get(block.turn)}`));
    lastTurn = block.turn;
    const start = layout.lines.length;
    layout.lines.push(...lines);
    layout.blocks.push({ id: block.id, start, end: layout.lines.length, collapsible: blockCanCollapse(block) });
  }
  if (layout.lines.length === 0)
    layout.lines.push(theme.fg("muted", state.outputOnly ? "No output yet." : "No transcript yet."));
  layout.lines = layout.lines.map((line) => truncateToWidth(line, width));
  return layout;
}
