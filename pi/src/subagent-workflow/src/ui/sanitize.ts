/** Maximum printable characters retained from any untrusted terminal field. */
export const UNTRUSTED_FIELD_MAX = 64 * 1024;

type TerminalParserState = "ground" | "escape" | "csi" | "osc" | "oscEscape" | "string" | "stringEscape";

interface TerminalParser {
  state: TerminalParserState;
  previousGroundCr: boolean;
}

export interface SanitizedTerminalTail {
  text: string;
  elided: boolean;
}

/** Compact JSON-like preview with bounded traversal, output, and cycle handling. */
export function boundedJsonPreview(value: unknown, maxChars = 120, maxNodes = 64, maxDepth = 6): string {
  const limit = Math.max(0, maxChars);
  if (limit === 0) return "";
  let output = "";
  let nodes = Math.max(0, maxNodes);
  let truncated = false;
  const ancestors = new Set<object>();
  const append = (text: string): void => {
    const remaining = limit - output.length;
    if (text.length > remaining) truncated = true;
    if (remaining > 0) output += text.slice(0, remaining);
  };
  const quote = (text: string): string => JSON.stringify(text.slice(0, limit - output.length)) ?? "\"\"";
  const write = (current: unknown, depth: number): void => {
    if (output.length >= limit) { truncated = true; return; }
    if (nodes <= 0 || depth > maxDepth) { truncated = true; append("…"); return; }
    nodes -= 1;
    if (current === null || typeof current === "boolean") { append(String(current)); return; }
    if (typeof current === "number") { append(Number.isFinite(current) ? String(current) : "\"<unsupported>\""); return; }
    if (typeof current === "string") {
      const clipped = current.slice(0, Math.max(0, limit - output.length));
      append(quote(clipped));
      truncated ||= clipped.length < current.length;
      return;
    }
    if (typeof current !== "object") { append("\"<unsupported>\""); return; }
    if (ancestors.has(current)) { append("\"<cycle>\""); return; }
    try {
      const array = Array.isArray(current);
      if (!array && ![Object.prototype, null].includes(Object.getPrototypeOf(current))) {
        append("\"<unsupported>\"");
        return;
      }
      ancestors.add(current);
      append(array ? "[" : "{");
      let count = 0;
      for (const key in current) {
        if (!Object.hasOwn(current, key)) continue;
        if (nodes <= 0 || output.length >= limit) { truncated = true; append(count ? ",…" : "…"); break; }
        if (count > 0) append(",");
        if (!array) { append(quote(key)); append(":"); }
        try { write((current as Record<string, unknown>)[key], depth + 1); }
        catch { append("\"<unavailable>\""); }
        count += 1;
      }
      append(array ? "]" : "}");
    } catch { append("\"<unavailable>\""); }
    finally { ancestors.delete(current); }
  };
  write(value, 0);
  if (truncated && !output.endsWith("…")) output = `${output.slice(0, Math.max(0, limit - 1))}…`;
  return output;
}

function isBidiFormattingControl(code: number): boolean {
  return code === 0x061c
    || code === 0x200e
    || code === 0x200f
    || (code >= 0x202a && code <= 0x202e)
    || (code >= 0x2066 && code <= 0x2069);
}

function scanTerminalText(
  parser: TerminalParser,
  value: string,
  preserveNewlines: boolean,
  maxInput: number,
  emit: (value: string) => boolean,
): number {
  let inspected = 0;
  for (let index = 0; index < value.length && inspected < maxInput; index += 1) {
    inspected += 1;
    const code = value.charCodeAt(index);

    if (parser.state === "escape") {
      parser.previousGroundCr = false;
      if (code === 0x5b) parser.state = "csi";
      else if (code === 0x5d) parser.state = "osc";
      else if (code === 0x50 || code === 0x58 || code === 0x5e || code === 0x5f) parser.state = "string";
      else parser.state = "ground";
      continue;
    }

    if (parser.state === "csi") {
      parser.previousGroundCr = false;
      if (code >= 0x40 && code <= 0x7e) parser.state = "ground";
      continue;
    }

    if (parser.state === "oscEscape") {
      parser.previousGroundCr = false;
      if (code === 0x5c || code === 0x07 || code === 0x9c) parser.state = "ground";
      else if (code !== 0x1b) parser.state = "osc";
      continue;
    }

    if (parser.state === "osc") {
      parser.previousGroundCr = false;
      if (code === 0x07 || code === 0x9c) parser.state = "ground";
      else if (code === 0x1b) parser.state = "oscEscape";
      continue;
    }

    if (parser.state === "stringEscape") {
      parser.previousGroundCr = false;
      if (code === 0x5c || code === 0x9c) parser.state = "ground";
      else if (code !== 0x1b) parser.state = "string";
      continue;
    }

    if (parser.state === "string") {
      parser.previousGroundCr = false;
      if (code === 0x9c) parser.state = "ground";
      else if (code === 0x1b) parser.state = "stringEscape";
      continue;
    }

    if (code === 0x1b) {
      parser.state = "escape";
      parser.previousGroundCr = false;
    } else if (code === 0x9b) {
      parser.state = "csi";
      parser.previousGroundCr = false;
    } else if (code === 0x9d) {
      parser.state = "osc";
      parser.previousGroundCr = false;
    } else if (code === 0x90 || code === 0x98 || code === 0x9e || code === 0x9f) {
      parser.state = "string";
      parser.previousGroundCr = false;
    } else if (code === 0x0d || code === 0x0a) {
      const output = preserveNewlines && !(code === 0x0a && parser.previousGroundCr) ? "\n" : undefined;
      parser.previousGroundCr = code === 0x0d;
      if (output && !emit(output)) break;
    } else {
      parser.previousGroundCr = false;
      const output = code === 0x09
        ? "\t"
        : code >= 0x20 && code !== 0x7f && (code < 0x80 || code > 0x9f) && !isBidiFormattingControl(code)
          ? value[index]
          : undefined;
      if (output && !emit(output)) break;
    }
  }
  return inspected;
}

export class BoundedTail<T> {
  private readonly buffer: T[];
  private start = 0;
  private length = 0;
  elided = false;

  constructor(private readonly limit: number) {
    this.buffer = new Array<T>(limit);
  }

  push(value: T): void {
    if (this.limit === 0) {
      this.elided = true;
      return;
    }
    if (this.length < this.limit) {
      this.buffer[(this.start + this.length) % this.limit] = value;
      this.length += 1;
      return;
    }
    this.buffer[this.start] = value;
    this.start = (this.start + 1) % this.limit;
    this.elided = true;
  }

  values(): T[] {
    if (this.length === 0) return [];
    if (this.start === 0) return this.buffer.slice(0, this.length);
    return [...this.buffer.slice(this.start, this.length), ...this.buffer.slice(0, this.start)];
  }
}

/** Stateful sanitizer for content whose terminal sequences cross logical chunks. */
export class TerminalTextSanitizer {
  private readonly parser: TerminalParser = { state: "ground", previousGroundCr: false };

  /**
   * Retain a bounded printable suffix while scanning the whole chunk. The
   * discarded prefix is never assembled into an intermediate string.
   */
  sanitizeTail(value: string, maxLength = UNTRUSTED_FIELD_MAX, preserveNewlines = false): SanitizedTerminalTail {
    const tail = new BoundedTail<string>(Math.max(0, maxLength));
    scanTerminalText(this.parser, value, preserveNewlines, Number.POSITIVE_INFINITY, (next) => {
      tail.push(next);
      return true;
    });
    return { text: tail.values().join(""), elided: tail.elided };
  }
}

/**
 * Remove terminal controls from untrusted chunks while preserving parser state
 * across chunk boundaries. Tabs are retained. CR/LF are either dropped or
 * normalized to a single LF, and are emitted only while the parser is in the
 * ground state.
 */
export function sanitizeTerminalTextChunks(
  chunks: Iterable<string>,
  maxLength = UNTRUSTED_FIELD_MAX,
  preserveNewlines = false,
): string {
  const limit = Math.max(0, maxLength);
  if (limit === 0) return "";

  // A control-string payload may produce no output, so the output limit alone
  // cannot bound parser work for a live, ever-growing malformed sequence.
  const scanLimit = Math.max(limit, UNTRUSTED_FIELD_MAX);
  const parser: TerminalParser = { state: "ground", previousGroundCr: false };
  let inspected = 0;
  let output = "";

  for (const chunk of chunks) {
    inspected += scanTerminalText(parser, chunk, preserveNewlines, scanLimit - inspected, (next) => {
      if (output.length >= limit) return false;
      output += next;
      return output.length < limit;
    });
    if (output.length >= limit || inspected >= scanLimit) break;
  }

  return output;
}

/**
 * Retain a bounded sanitized suffix from chunks with one shared parser state.
 * When requested, logical chunk boundaries are emitted directly to the output,
 * never scanned as terminal input or allowed to alter pending parser state.
 */
export function sanitizeTerminalTextTailChunks(
  chunks: Iterable<string>,
  maxLength = UNTRUSTED_FIELD_MAX,
  preserveNewlines = false,
  preserveChunkBoundaries = false,
): SanitizedTerminalTail {
  const parser: TerminalParser = { state: "ground", previousGroundCr: false };
  const tail = new BoundedTail<string>(Math.max(0, maxLength));
  let seenChunk = false;
  for (const chunk of chunks) {
    if (seenChunk && preserveChunkBoundaries) tail.push("\n");
    seenChunk = true;
    scanTerminalText(parser, chunk, preserveNewlines, Number.POSITIVE_INFINITY, (next) => {
      tail.push(next);
      return true;
    });
  }
  return { text: tail.values().join(""), elided: tail.elided };
}

/**
 * Remove terminal controls from untrusted text before applying theme styling.
 * Tabs are preserved. Set `preserveNewlines` for multiline display content;
 * newlines inside terminal control strings remain suppressed.
 */
export function sanitizeTerminalText(
  value: string,
  maxLength = UNTRUSTED_FIELD_MAX,
  preserveNewlines = false,
): string {
  return sanitizeTerminalTextChunks([value], maxLength, preserveNewlines);
}
