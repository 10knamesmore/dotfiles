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

/** Format a non-negative token count without implying greater precision than the footer can show. */
export function formatTokens(count: number): string {
  if (count < 1_000) return Math.round(count).toString();
  if (count < 10_000) return `${(count / 1_000).toFixed(1)}k`;
  if (count < 1_000_000) return `${Math.round(count / 1_000)}k`;
  if (count < 10_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  return `${Math.round(count / 1_000_000)}M`;
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

/** Preserve both required fields when the model/context row becomes narrower than their full text. */
export function fitRequiredPair(
  left: string,
  right: string,
  width: number,
): string {
  if (width <= 0) return "";
  const complete = `${left}${separator}${right}`;
  if (visibleWidth(complete) <= width) return complete;
  const separatorWidth = visibleWidth(separator);
  if (width <= separatorWidth + 4)
    return truncateToWidth(complete, width, palette.overlay2("…"));

  const available = width - separatorWidth;
  let leftWidth = Math.min(
    visibleWidth(left),
    Math.max(4, Math.floor(available * 0.58)),
  );
  let rightWidth = Math.max(1, available - leftWidth);
  const unusedRight = Math.max(0, rightWidth - visibleWidth(right));
  leftWidth = Math.min(visibleWidth(left), leftWidth + unusedRight);
  rightWidth = Math.max(1, available - leftWidth);
  return `${truncateToWidth(left, leftWidth, palette.overlay2("…"))}${separator}${truncateToWidth(
    right,
    rightWidth,
    palette.overlay2("…"),
  )}`;
}
