/**
 * Tool-call row rendering for the `subagent` tool.
 *
 * Every run is background: the tool returns as soon as its child starts, so the
 * details it attaches are a launch receipt - who was spawned, on what model -
 * and never change afterwards. Live progress, tokens, and elapsed time live in
 * the /agents navigator, a full-screen overlay whose repaints always land inside
 * the viewport.
 *
 * That split is deliberate. Pi draws inline, and pi-tui escalates to a full
 * redraw - which erases the terminal's scrollback and snaps the view to the
 * bottom - whenever the first changed line sits above the visible viewport. A
 * tool row is exactly that: it keeps its place in the buffer while the
 * conversation grows past it. Anything clock-derived here (a spinner frame, a
 * ticking elapsed) would therefore repaint the whole screen on every render any
 * component asks for, for the rest of the session, and the user could never
 * scroll back.
 *
 * Overlays are exempt for their own repaints only. Shrinking the base line
 * buffer while one is mounted shifts every overlay line up and forces the same
 * full redraw - see `paint` in ./status-widget.ts.
 */

import { wrapTextWithAnsi, type Component } from "@earendil-works/pi-tui";
import type { SubagentHandle, SubagentSpec, ThinkingLevel } from "../types.js";
import { guardedLines, linesComponent } from "./component.js";
import { childLabel, formatFullModel, shortModel, type ThemeLike } from "./format.js";
import { sanitizeTerminalText, sanitizeTerminalTextChunks, UNTRUSTED_FIELD_MAX } from "./sanitize.js";
import { isRecord } from "../util.js";

/**
 * Serializable per-child row snapshot carried in the tool result details.
 *
 * Deliberately state-free: the receipt is written once at spawn and replayed
 * verbatim on every resume, so anything that changes over the child's life
 * (status, tokens, timing) would be frozen at its spawn-time value and read as
 * a lie hours later. Live state belongs to the status widget and /agents.
 */
interface ChildSnapshot {
  id: string;
  label: string;
  modelId: string;
  /** Reasoning effort the child resolved to, shown beside the model. */
  thinking?: string;
}

/** Read a string field from the persisted launch receipt. */
function text(value: unknown, fallback: string): string {
  return typeof value === "string" ? value : fallback;
}

/**
 * Read the launch receipt. Rejected tool calls can carry empty details, so
 * missing children must fall back to the tool's error text instead of throwing
 * in Pi's render loop.
 */
export function safeDetails(details: unknown): SubagentDetails | undefined {
  if (!isRecord(details) || !Array.isArray(details.children)) return undefined;
  const child = details.children[0];
  if (!isRecord(child)) return undefined;
  return {
    children: [
      {
        id: text(child.id, ""),
        label: text(child.label, ""),
        modelId: text(child.modelId, ""),
        thinking: typeof child.thinking === "string" ? child.thinking : undefined,
      },
    ],
  };
}

/** The single child's launch receipt, also persisted with the tool result. */
export interface SubagentDetails {
  children: [ChildSnapshot];
}

/** Render the complete launch receipt, wrapping at the available terminal width. */
export function renderRows(details: SubagentDetails, theme: ThemeLike, width: number): string[] {
  const child = details.children[0];
  const cells = [`${theme.fg("dim", "▸")} ${sanitizeTerminalText(child.label)}`];
  const model = sanitizeTerminalText(formatFullModel(child.modelId, child.thinking));
  if (model) cells.push(theme.fg("dim", model));
  return wrapTextWithAnsi(cells.join("  "), Math.max(1, width));
}

/** Single-line call header line. */
export function callHeaderLine(label: string, theme: ThemeLike): string {
  return theme.fg("toolTitle", theme.bold(`subagent · ${sanitizeTerminalText(label)}`));
}

/** Call header component that wraps the full label to the available width. */
export function renderCallHeader(label: string, theme: ThemeLike): Component {
  return linesComponent(
    (width) => wrapTextWithAnsi(callHeaderLine(label, theme), Math.max(1, width)),
    "subagent call header",
  );
}

/** Component rendering rows straight from the result's snapshot. */
class SubagentRowsComponent implements Component {
  private details: SubagentDetails | undefined;
  private fallback: string[] = [];
  private readonly draw = guardedLines("subagent rows", (width) =>
    this.details
      ? renderRows(this.details, this.theme, width)
      : this.fallback.flatMap((line) => wrapTextWithAnsi(line, Math.max(1, width))),
  );
  constructor(private readonly theme: ThemeLike) {}
  /** Stores the normalized snapshot; `details` is untrusted here. */
  set(details: unknown, fallback: string[]): void {
    this.details = safeDetails(details);
    this.fallback = fallback;
  }
  render(width: number): string[] {
    return this.draw(width);
  }
  invalidate(): void {}
}

/**
 * renderResult hook body. No invalidate timer: the rows never change once the
 * result is attached.
 *
 * Defining renderResult replaces pi's own result rendering outright, so when
 * there are no drawable rows - a call this tool rejected carries `details: {}` -
 * the result text has to be shown here or the user never learns why it failed.
 */
export function renderSubagentResult(
  result: { content?: Array<{ type: string; text?: string }>; details?: unknown },
  theme: ThemeLike,
  lastComponent: Component | undefined,
): Component {
  const textParts = (result.content ?? []).flatMap((part) =>
    part.type === "text" && typeof part.text === "string" ? [part.text] : [],
  );
  const fallback =
    textParts.length === 0 ? [] : sanitizeTerminalTextChunks(textParts, UNTRUSTED_FIELD_MAX, true).split("\n");
  const component = lastComponent instanceof SubagentRowsComponent ? lastComponent : new SubagentRowsComponent(theme);
  component.set(result.details, fallback);
  return component;
}

/**
 * Launch receipt for the spawned child.
 *
 * The model is passed in resolved by the caller rather than read off the
 * handle: `handle.resolved` is populated asynchronously after admission, so at
 * receipt time it is always still undefined.
 */
export function initialDetails(
  spec: SubagentSpec,
  handle: SubagentHandle,
  display: { modelId?: string; thinking?: ThinkingLevel },
): SubagentDetails {
  return {
    children: [
      {
        id: handle.id,
        label: sanitizeTerminalText(childLabel(spec)),
        modelId: sanitizeTerminalText(shortModel(display.modelId)),
        thinking: display.thinking,
      },
    ],
  };
}
