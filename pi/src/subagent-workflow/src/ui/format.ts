/**
 * Pure formatting and aggregation helpers for the subagent TUI surfaces.
 *
 * Everything here is deterministic and theme-agnostic so it can be unit tested
 * with the PLAIN theme. Rendering into pi-tui Components lives in the sibling
 * files; this module only produces strings and counts.
 */

import type { SubagentStatus } from "../types.js";

export { childLabel } from "../util.js";

/**
 * Minimal slice of pi's Theme used by the renderers. The real Theme class
 * satisfies this structurally, so renderers accept ThemeLike and callers pass
 * the live theme. Tests pass PLAIN for exact-string assertions.
 */
export interface ThemeLike {
  fg(color: string, text: string): string;
  bold(text: string): string;
}

/** Identity theme: no ANSI, so rendered widths equal string lengths in tests. */
export const PLAIN: ThemeLike = { fg: (_color, text) => text, bold: (text) => text };

/** Native pi braille spinner frames (packages/tui loader DEFAULT_FRAMES). */
export const SPINNER = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"] as const;

/** Frame index derived from the clock so animation needs no per-frame counter. */
export function spinnerFrame(now: number): string {
  return SPINNER[Math.floor(now / 100) % SPINNER.length]!;
}

/** Theme color role for each terminal/active status. */
export function statusColor(status: SubagentStatus): string {
  switch (status) {
    case "completed":
      return "success";
    case "failed":
      return "error";
    case "aborted":
      return "warning";
    case "running":
      return "accent";
    default:
      return "dim";
  }
}

/**
 * Colored status glyph. Running renders the animated spinner when `animate`
 * (a live, streaming row) and a static marker otherwise (a backgrounded row
 * whose live state has moved to the widget).
 */
export function statusGlyph(status: SubagentStatus, theme: ThemeLike, now: number, animate: boolean): string {
  const color = statusColor(status);
  if (status === "running") return theme.fg(color, animate ? spinnerFrame(now) : "◆");
  const glyph =
    status === "completed" ? "✓" : status === "failed" ? "✗" : status === "aborted" ? "⊘" : "·";
  return theme.fg(color, glyph);
}

/** Compact token count: 1234 -> "1.2k", 2_500_000 -> "2.5M". */
export function formatTokens(count: number): string {
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1_000) return `${(count / 1_000).toFixed(1)}k`;
  return `${Math.max(0, Math.round(count))}`;
}

/** Human-readable elapsed: "0.8s", "12s", "3m20s". */
export function formatDuration(ms: number): string {
  const seconds = Math.max(0, ms) / 1000;
  if (seconds < 10) return `${seconds.toFixed(1)}s`;
  if (seconds < 60) return `${Math.round(seconds)}s`;
  const whole = Math.round(seconds);
  return `${Math.floor(whole / 60)}m${(whole % 60).toString().padStart(2, "0")}s`;
}

/** Drop the "provider/" prefix for a compact model label. */
export function shortModel(modelId: string | undefined): string {
  if (!modelId) return "";
  const slash = modelId.indexOf("/");
  return slash >= 0 ? modelId.slice(slash + 1) : modelId;
}

/**
 * Model and reasoning effort as one cell: "gpt-5.6-sol·high".
 *
 * Binding them into a single cell keeps the effort visually attached to the
 * model it applies to, and costs less width than a separate aligned column.
 * Effort is appended after the model is truncated so a long model id can never
 * push it out of view - the effort is the shorter and rarer piece of
 * information, so it is the one worth protecting.
 *
 * "off" renders as bare model: it is the resting state for every non-reasoning
 * model, so showing it would add a column of noise to the common case without
 * telling the reader anything the model name does not already imply.
 */
export function modelEffort(model: string, thinking: string | undefined, max: number): string {
  if (!model) return "";
  if (!thinking || thinking === "off") return truncate(model, max);
  const suffix = `·${thinking}`;
  return `${truncate(model, Math.max(1, max - suffix.length))}${suffix}`;
}

/** Width-truncate a plain (ANSI-free) string with an ellipsis. */
function truncate(text: string, max: number): string {
  return text.length <= max ? text : `${text.slice(0, Math.max(0, max - 1))}…`;
}

/** First non-empty line of result text, collapsed and trimmed. */
export function firstLine(text: string): string {
  return text.split("\n").map((line) => line.trim()).find((line) => line.length > 0) ?? "";
}

/** Counts of children by lifecycle bucket, for header and widget summaries. */
export interface StatusCounts {
  total: number;
  running: number;
  pending: number;
  completed: number;
  failed: number;
  aborted: number;
  /** Terminal children (completed + failed + aborted). */
  done: number;
  /** Any child still pending or running. */
  active: boolean;
}

export function countStatuses(statuses: SubagentStatus[]): StatusCounts {
  const counts: StatusCounts = {
    total: statuses.length,
    running: 0,
    pending: 0,
    completed: 0,
    failed: 0,
    aborted: 0,
    done: 0,
    active: false,
  };
  for (const status of statuses) {
    if (status === "running") counts.running += 1;
    else if (status === "pending") counts.pending += 1;
    else if (status === "completed") counts.completed += 1;
    else if (status === "failed") counts.failed += 1;
    else if (status === "aborted") counts.aborted += 1;
  }
  counts.done = counts.completed + counts.failed + counts.aborted;
  counts.active = counts.running > 0 || counts.pending > 0;
  return counts;
}

/** Number rendered as a right-aligned cell of exactly `width` columns. */
export function padStart(text: string, width: number): string {
  return text.length >= width ? text : " ".repeat(width - text.length) + text;
}

export function clamp(value: number, low: number, high: number): number {
  return Math.max(low, Math.min(high, value));
}
