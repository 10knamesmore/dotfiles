import { sanitizeTerminalText } from "./sanitize.js";

export function safeDeliveryValue(value: string): string {
  return sanitizeTerminalText(value);
}

interface CollapsedLine {
  text: string;
  count: number;
}

function collapseConsecutiveLines(value: string): CollapsedLine[] {
  const collapsed: CollapsedLine[] = [];
  for (const text of value.split("\n")) {
    const previous = collapsed.at(-1);
    if (previous?.text === text) previous.count += 1;
    else collapsed.push({ text, count: 1 });
  }
  return collapsed;
}

function displayCollapsedLine(line: CollapsedLine): string {
  return line.count === 1 ? line.text : `${line.text}${line.text.length === 0 ? "" : " "}(repeated ${line.count} times)`;
}

function failureOmissionNote(lines: readonly CollapsedLine[]): string {
  const omittedLines = lines.reduce((sum, line) => sum + line.count, 0);
  const repeatedLines = lines.reduce((sum, line) => sum + Math.max(0, line.count - 1), 0);
  const repeats = repeatedLines > 0 ? `, including ${repeatedLines} repeats` : "";
  return `[earlier output truncated; ${omittedLines} earlier line${omittedLines === 1 ? "" : "s"} omitted${repeats}]`;
}

/** Preserve stderr-shaped causal tails while compacting exact repeated lines. */
export function formatFailureText(value: string, maxLength = 1_500): string {
  const limit = Number.isFinite(maxLength) ? Math.max(1, Math.floor(maxLength)) : 1_500;
  const sanitized = sanitizeTerminalText(value, value.length, true);
  const collapsed = collapseConsecutiveLines(sanitized);
  const full = collapsed.map(displayCollapsedLine).join("\n");
  if (full.length <= limit) return full;

  const selected: string[] = [];
  for (let index = collapsed.length - 1; index >= 0; index -= 1) {
    const line = displayCollapsedLine(collapsed[index]!);
    const note = failureOmissionNote(collapsed.slice(0, index));
    const candidate = [line, ...selected, note].join("\n");
    if (candidate.length > limit) break;
    selected.unshift(line);
  }
  if (selected.length > 0) {
    const omitted = collapsed.slice(0, collapsed.length - selected.length);
    return [...selected, failureOmissionNote(omitted)].join("\n");
  }

  const note = "[earlier output truncated]";
  if (note.length >= limit) return note.slice(0, limit);
  const tailBudget = limit - note.length - 1;
  if (tailBudget <= 0) return note;
  const tail = full.slice(-tailBudget);
  const first = tail.charCodeAt(0);
  const safeTail = first >= 0xdc00 && first <= 0xdfff ? tail.slice(1) : tail;
  return `${safeTail}\n${note}`;
}

/** Split display-only text so generic delivery line bounding never cuts it. */
export function chunkDeliveryText(value: string, maxLineLength = 500): string {
  if (maxLineLength < 1) return "";
  const chunks: string[] = [];
  for (let offset = 0; offset < value.length;) {
    let end = Math.min(value.length, offset + maxLineLength);
    const last = value.charCodeAt(end - 1);
    if (last >= 0xd800 && last <= 0xdbff) end -= 1;
    if (end === offset) end = Math.min(value.length, offset + 2);
    chunks.push(value.slice(offset, end));
    offset = end;
  }
  return chunks.join("\n");
}

/** JSON escapes C0 controls but leaves DEL and C1 controls raw. */
export function stringifyDeliveryJson(value: unknown): string {
  const json = JSON.stringify(value);
  if (json === undefined) return "null";
  return json.replace(new RegExp("[\\u007f-\\u009f]", "g"), (control) =>
    `\\u${control.charCodeAt(0).toString(16).padStart(4, "0")}`);
}
