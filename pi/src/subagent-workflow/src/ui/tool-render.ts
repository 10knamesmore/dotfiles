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

import { truncateToWidth, type Component } from "@earendil-works/pi-tui";
import type { SubagentHandle, SubagentSpec, ThinkingLevel } from "../types.js";
import { guardedLines, linesComponent } from "./component.js";
import { childLabel, clamp, modelEffort, shortModel, type ThemeLike } from "./format.js";
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

/** String, or a fallback - the string fields below are measured and sliced. */
function text(value: unknown, fallback: string): string {
  return typeof value === "string" ? value : fallback;
}

/**
 * Coerce a details payload into rows that are safe to draw, or undefined.
 *
 * `details` is untrusted at render time. Pi hands back whatever the tool result
 * carried, and a call this tool *rejected* arrives as a truthy `{}` with no
 * `children` at all - reading `.children.length` on that threw inside pi's render
 * loop and killed the TUI, permanently, because the bad result is persisted and
 * replayed on every resume. Details also survive upgrades, so a resumed session
 * can replay a snapshot written by a different version of this file.
 *
 * Only the fields the renderer would break on are rewritten: `label` and `modelId`
 * get measured and sliced, and `id` keys the per-column cell map. `thinking` is
 * only ever interpolated, so it passes through.
 */
export function safeDetails(details: unknown): SubagentDetails | undefined {
  if (!isRecord(details) || !Array.isArray(details.children)) return undefined;
  const children = details.children.filter(isRecord).map((child, index): ChildSnapshot => ({
    ...child,
    id: text(child.id, `child-${index}`),
    label: text(child.label, ""),
    modelId: text(child.modelId, ""),
  }));
  return children.length === 0 ? undefined : { children };
}

/**
 * Details attached to the subagent tool result for TUI rendering.
 *
 * A run holds exactly one child, but the payload stays a list: sessions written
 * by earlier versions carry several, and they still have to draw on resume.
 */
export interface SubagentDetails {
  children: ChildSnapshot[];
}

const LABEL_MIN = 6;
const LABEL_MAX = 28;
const MODEL_MAX = 24;
const GAP = "  ";

/**
 * Render the per-child rows as width-clamped lines. Columns align across
 * children, which matters only for the multi-child snapshots older sessions
 * carry; the model cell is clipped to the terminal width by truncateToWidth
 * (ANSI-aware).
 */
export function renderRows(details: SubagentDetails, theme: ThemeLike, width: number): string[] {
  // Never exceed the host-supplied width: pi-tui aborts the process on any
  // over-wide line, so there is no minimum layout width worth crashing for. A
  // cramped row is a cosmetic problem; a wide one ends the session.
  const cap = Math.max(1, width);
  const children = details.children;
  const labelWidth = clamp(Math.max(...children.map((child) => child.label.length), LABEL_MIN), LABEL_MIN, LABEL_MAX);
  // Model and effort share one cell, so alignment is computed on the combined
  // text rather than the model id alone.
  const modelCells = new Map(children.map((child) => [child.id, modelEffort(child.modelId, child.thinking, MODEL_MAX)]));
  const modelWidth = clamp(Math.max(0, ...[...modelCells.values()].map((cell) => cell.length)), 0, MODEL_MAX);

  return children.map((child) => {
    // A state-free launch marker, not a status glyph: the receipt never
    // changes after spawn, so any status here would be frozen at "pending".
    const label = truncateToWidth(child.label, labelWidth, "…", true);
    const cells = [`${theme.fg("dim", "▸")} ${label}`];
    if (modelWidth > 0) cells.push(theme.fg("dim", truncateToWidth(sanitizeTerminalText(modelCells.get(child.id) ?? ""), modelWidth, "…", true)));
    return truncateToWidth(cells.join(GAP), cap);
  });
}

/** Single-line call header line. */
export function callHeaderLine(label: string, theme: ThemeLike): string {
  return theme.fg("toolTitle", theme.bold(`subagent · ${sanitizeTerminalText(label)}`));
}

/** Single-line call header component shown above the rows. */
export function renderCallHeader(label: string, theme: ThemeLike): Component {
  return linesComponent((width) => [truncateToWidth(callHeaderLine(label, theme), Math.max(1, width))], "subagent call header");
}

/** Component rendering rows straight from the result's snapshot. */
class SubagentRowsComponent implements Component {
  private details: SubagentDetails | undefined;
  private fallback: string[] = [];
  private readonly draw = guardedLines("subagent rows", (width) => this.details
    ? renderRows(this.details, this.theme, width)
    : this.fallback.map((line) => truncateToWidth(line, Math.max(1, width))));
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
  const textParts = (result.content ?? [])
    .flatMap((part) => part.type === "text" && typeof part.text === "string" ? [part.text] : []);
  const fallback = textParts.length === 0
    ? []
    : sanitizeTerminalTextChunks(textParts, UNTRUSTED_FIELD_MAX, true).split("\n");
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
    children: [{
      id: handle.id,
      label: sanitizeTerminalText(childLabel(spec)),
      modelId: sanitizeTerminalText(shortModel(display.modelId)),
      thinking: display.thinking,
    }],
  };
}
