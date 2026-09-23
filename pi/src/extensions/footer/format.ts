import { isAbsolute, relative, resolve, sep } from "node:path";
import {
  stripTerminalSequences,
  truncateToWidth,
  visibleWidth,
} from "@earendil-works/pi-tui";
import { palette, separator } from "./palette.js";

/** Remove terminal control sequences and normalize untrusted text to one display line. */
export function sanitizeFooterText(value: string): string {
  return stripTerminalSequences(value)
    .replace(/[\u0000-\u001f\u007f-\u009f\u2028\u2029]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

/** Replace a cwd inside the current home directory with a `~`-relative path. */
export function formatFooterCwd(cwd: string, home: string | undefined): string {
  const cleanCwd = sanitizeFooterText(cwd);
  if (!home) return cleanCwd;

  const resolvedCwd = resolve(cwd);
  const resolvedHome = resolve(home);
  const relativeToHome = relative(resolvedHome, resolvedCwd);
  const insideHome =
    relativeToHome === "" ||
    (relativeToHome !== ".." &&
      !relativeToHome.startsWith(`..${sep}`) &&
      !isAbsolute(relativeToHome));
  if (!insideHome) return cleanCwd;
  return relativeToHome === ""
    ? "~"
    : `~${sep}${sanitizeFooterText(relativeToHome)}`;
}

/**
 * Format a non-negative token count. `fractionDigits` is the decimal count at
 * the largest unit (k/M); the next smaller unit keeps one digit less.
 */
export function formatTokens(count: number, fractionDigits: number = 1): string {
  if (count < 1_000) return Math.round(count).toString();
  if (count < 10_000) return `${(count / 1_000).toFixed(fractionDigits)}k`;
  if (count < 1_000_000) return `${(count / 1_000).toFixed(fractionDigits - 1)}k`;
  if (count < 10_000_000)
    return `${(count / 1_000_000).toFixed(fractionDigits)}M`;
  return `${(count / 1_000_000).toFixed(fractionDigits - 1)}M`;
}

/** Format generated-token throughput for compact footer display. */
export function formatTokensPerSecond(tokensPerSecond: number): string {
  if (tokensPerSecond < 1_000) return tokensPerSecond.toFixed(1);
  return `${(tokensPerSecond / 1_000).toFixed(1)}k`;
}

/** Keep sub-second precision for the time to first generated content. */
export function formatTokenLatency(milliseconds: number): string {
  return `${(milliseconds / 1_000).toFixed(1)}s`;
}

/** Format elapsed wall time for the active session and measured model work. */
export function formatDuration(milliseconds: number): string {
  const totalSeconds = Math.max(0, Math.floor(milliseconds / 1_000));
  const hours = Math.floor(totalSeconds / 3_600);
  const minutes = Math.floor((totalSeconds % 3_600) / 60);
  const seconds = totalSeconds % 60;
  if (hours > 0) return `${hours}h${minutes}m`;
  if (minutes > 0) return `${minutes}m${seconds}s`;
  return `${seconds}s`;
}

/** Join colored segments and deterministically remove lower-priority fields until they fit. */
export function fitByDropping(
  parts: readonly string[],
  dropOrder: readonly number[],
  width: number,
  delimiter: string = " ",
): string {
  if (width <= 0) return "";
  const visible = parts.map(() => true);
  const render = (): string =>
    parts
      .filter((part, index) => visible[index] && part.length > 0)
      .join(delimiter);
  let line = render();
  for (const index of dropOrder) {
    if (visibleWidth(line) <= width) break;
    if (index >= 0 && index < visible.length) visible[index] = false;
    line = render();
  }
  return truncateToWidth(line, width, palette.overlay2("…"));
}
