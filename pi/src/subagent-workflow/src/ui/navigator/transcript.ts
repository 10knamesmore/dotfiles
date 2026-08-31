/**
 * Transcript rendering for the agent-detail view (level 3).
 *
 * Live and completed views both read recent messages from the child-written
 * session file. Live session events only trigger re-renders. The bounded
 * reverse scanner avoids retaining the full transcript in either mode.
 *
 * Messages are pi's own `AgentMessage` union, so the persisted session files open
 * with pi's tooling and render identically here. Conversion is defensive: unknown
 * roles and content parts are skipped rather than throwing.
 */

import { closeSync, fstatSync, openSync, readSync } from "node:fs";
import type { SessionEntry } from "@earendil-works/pi-coding-agent";
import { truncateToWidth, visibleWidth } from "@earendil-works/pi-tui";
import type { ThemeLike } from "../format.js";
import {
  BoundedTail,
  boundedJsonPreview,
  sanitizeTerminalText,
  sanitizeTerminalTextTailChunks,
  TerminalTextSanitizer,
  UNTRUSTED_FIELD_MAX,
  type SanitizedTerminalTail,
} from "../sanitize.js";
import { isRecord } from "../../util.js";

/** Minimal structural view of pi's AgentMessage union (role + content parts). */
export interface TranscriptMessage {
  role: string;
  content?: unknown;
  toolName?: string;
  responseId?: string;
  timestamp?: number;
}

/** Max lines kept from a single tool result before it is elided. */
const RESULT_MAX_LINES = 10;
/** Hard bound on eagerly materialized rows for one transcript render. */
export const TRANSCRIPT_MAX_LINES = 2_000;
const TRANSCRIPT_CONTENT_MAX_LINES = TRANSCRIPT_MAX_LINES - 1;
const SESSION_SCAN_CHUNK_BYTES = 256 * 1024;
export const SESSION_SCAN_MAX_BYTES = 4 * 1024 * 1024;
export const SESSION_ENTRY_MAX_BYTES = 512 * 1024;
export const OVERSIZED_MESSAGE_OMISSION = "(oversized persisted message omitted)";
/** Neutral wording: the record was object-shaped but never proven to be a message. */
export const OVERSIZED_RECORD_OMISSION = "(oversized session record omitted)";
/** Neutral wording: unscanned records may include non-message entries, so this
 * never claims transcript messages were omitted. */
export const UNSCANNED_TRANSCRIPT_OMISSION = "(older session records not scanned: reader limit reached)";
export const LARGE_RECORD_SCAN_OMISSION = "(a large session record and older records not scanned: reader limit reached)";
const SESSION_ENTRY_PREFIX_BYTES = 256;
const SESSION_ENTRY_MAX_COUNT = 512;
/** Complete records inspected per render, including malformed and non-message records. */
export const SESSION_RECORD_MAX_COUNT = 4_096;
const SESSION_MATERIALIZED_MAX_BYTES = 2 * 1024 * 1024;

type StandardTranscriptRole = "user" | "assistant" | "toolResult";
type OversizedMessageEntry = { type: "oversized_message"; role?: StandardTranscriptRole; toolName?: string; unproven?: true };
type OversizedPrefixClassification = OversizedMessageEntry | "non-message" | undefined;
type ScanOmissionEntry = { type: "scan_omission"; largeRecord?: boolean };
type PersistedTranscriptEntry = SessionEntry | { type: string } | OversizedMessageEntry | ScanOmissionEntry;

type PrefixScanStatus = "complete" | "incomplete" | "invalid";
interface PrefixScanResult { status: PrefixScanStatus; next: number }
interface PrefixStringResult extends PrefixScanResult { value?: string }

function skipPrefixWhitespace(input: string, start: number): number {
  let index = start;
  while (index < input.length && /\s/u.test(input[index]!)) index += 1;
  return index;
}

function scanPrefixString(input: string, start: number): PrefixStringResult {
  if (input[start] !== "\"") return { status: "invalid", next: start };
  let escaped = false;
  for (let index = start + 1; index < input.length; index += 1) {
    const character = input[index]!;
    if (escaped) {
      escaped = false;
      continue;
    }
    if (character === "\\") {
      escaped = true;
      continue;
    }
    if (character !== "\"") continue;
    try {
      return { status: "complete", next: index + 1, value: JSON.parse(input.slice(start, index + 1)) as string };
    } catch {
      return { status: "invalid", next: index + 1 };
    }
  }
  return { status: "incomplete", next: input.length };
}

/** Classify only syntax proven by the bounded record prefix. */
function oversizedMessageEntry(prefix: string): OversizedPrefixClassification {
  let topLevelType: string | undefined;
  let messageObjectSeen = false;
  let role: StandardTranscriptRole | undefined;
  let toolName: string | undefined;

  const scanValue = (start: number, depth: number, context: "top" | "message" | "other"): PrefixScanResult => {
    if (depth > 24) return { status: "invalid", next: start };
    let index = skipPrefixWhitespace(prefix, start);
    if (index >= prefix.length) return { status: "incomplete", next: index };
    if (prefix[index] === "\"") {
      const string = scanPrefixString(prefix, index);
      return { status: string.status, next: string.next };
    }
    if (prefix[index] === "{") return scanObject(index, depth + 1, context);
    if (prefix[index] === "[") {
      index = skipPrefixWhitespace(prefix, index + 1);
      if (index >= prefix.length) return { status: "incomplete", next: index };
      if (prefix[index] === "]") return { status: "complete", next: index + 1 };
      while (true) {
        const value = scanValue(index, depth + 1, "other");
        if (value.status !== "complete") return value;
        index = skipPrefixWhitespace(prefix, value.next);
        if (index >= prefix.length) return { status: "incomplete", next: index };
        if (prefix[index] === "]") return { status: "complete", next: index + 1 };
        if (prefix[index] !== ",") return { status: "invalid", next: index };
        index = skipPrefixWhitespace(prefix, index + 1);
      }
    }

    const remainder = prefix.slice(index);
    const token = /^(?:true|false|null|-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?)/u.exec(remainder)?.[0];
    if (!token) return { status: "invalid", next: index };
    const next = index + token.length;
    if (next === prefix.length) return { status: "incomplete", next };
    return /[\s,}\]]/u.test(prefix[next]!)
      ? { status: "complete", next }
      : { status: "invalid", next };
  };

  const scanObject = (start: number, depth: number, context: "top" | "message" | "other"): PrefixScanResult => {
    let index = skipPrefixWhitespace(prefix, start + 1);
    if (index >= prefix.length) return { status: "incomplete", next: index };
    if (prefix[index] === "}") return { status: "complete", next: index + 1 };
    while (true) {
      const key = scanPrefixString(prefix, index);
      if (key.status !== "complete" || key.value === undefined) return { status: key.status, next: key.next };
      index = skipPrefixWhitespace(prefix, key.next);
      if (index >= prefix.length) return { status: "incomplete", next: index };
      if (prefix[index] !== ":") return { status: "invalid", next: index };
      index = skipPrefixWhitespace(prefix, index + 1);

      const captureString = (assign: (value: string) => void): PrefixScanResult => {
        const value = scanPrefixString(prefix, index);
        if (value.status === "complete" && value.value !== undefined) assign(value.value);
        return { status: value.status, next: value.next };
      };

      let value: PrefixScanResult;
      if (context === "top" && key.value === "type" && prefix[index] === "\"") {
        value = captureString((captured) => { topLevelType = captured; });
      } else if (context === "top" && key.value === "message" && prefix[index] === "{") {
        messageObjectSeen = true;
        value = scanObject(index, depth + 1, "message");
      } else if (context === "message" && key.value === "role" && prefix[index] === "\"") {
        value = captureString((captured) => {
          if (captured === "user" || captured === "assistant" || captured === "toolResult") role = captured;
        });
      } else if (context === "message" && key.value === "toolName" && prefix[index] === "\"") {
        value = captureString((captured) => { if (captured.length <= 120) toolName = captured; });
      } else {
        value = scanValue(index, depth + 1, "other");
      }
      if (value.status !== "complete") return value;
      index = skipPrefixWhitespace(prefix, value.next);
      if (index >= prefix.length) return { status: "incomplete", next: index };
      if (prefix[index] === "}") return { status: "complete", next: index + 1 };
      if (prefix[index] !== ",") return { status: "invalid", next: index };
      index = skipPrefixWhitespace(prefix, index + 1);
      if (index >= prefix.length) return { status: "incomplete", next: index };
    }
  };

  const start = skipPrefixWhitespace(prefix, 0);
  if (prefix[start] !== "{") return undefined;
  const result = scanObject(start, 0, "top");
  if (topLevelType !== undefined && topLevelType !== "message") return "non-message";
  if (result.status === "invalid" || topLevelType !== "message" || !messageObjectSeen) return undefined;
  return {
    type: "oversized_message",
    ...(role ? { role } : {}),
    ...(role === "toolResult" && toolName ? { toolName } : {}),
  };
}

function unavailable(reason: string): TranscriptMessage[] {
  return [{ role: "assistant", content: `(${reason})` }];
}

/** Convert a persisted pi session file into its message list. Never throws. */
export function readSessionMessages(path: string): TranscriptMessage[] {
  let fd: number | undefined;
  try {
    fd = openSync(path, "r");
    const size = fstatSync(fd).size;
    const newestEntries: PersistedTranscriptEntry[] = [];
    let messageEntries = 0;
    let materializedBytes = 0;
    let inspectedRecords = 0;
    let invalidLines = 0;
    let stopped = false;
    let cursor = size;
    let scannedBytes = 0;
    let readerLimitOmission = false;
    let largeRecordLimitOmission = false;
    let recordParts: Buffer[] = [];
    let recordPrefix = Buffer.alloc(0);
    let recordBytes = 0;
    let recordOversized = false;
    let recordMayBePartial = false;
    let trimTerminatingCr = false;

    const resetRecord = (): void => {
      recordParts = [];
      recordPrefix = Buffer.alloc(0);
      recordBytes = 0;
      recordOversized = false;
      recordMayBePartial = false;
      trimTerminatingCr = false;
    };
    const appendRecordPart = (buffer: Buffer, start: number, end: number): void => {
      if (trimTerminatingCr && end > start) {
        if (buffer[end - 1] === 0x0d) end -= 1;
        trimTerminatingCr = false;
      }
      const length = end - start;
      if (length === 0) return;
      const prefixPart = buffer.subarray(start, Math.min(end, start + SESSION_ENTRY_PREFIX_BYTES));
      recordPrefix = prefixPart.length >= SESSION_ENTRY_PREFIX_BYTES
        ? Buffer.from(prefixPart)
        : Buffer.concat(
          [prefixPart, recordPrefix.subarray(0, SESSION_ENTRY_PREFIX_BYTES - prefixPart.length)],
          Math.min(SESSION_ENTRY_PREFIX_BYTES, prefixPart.length + recordPrefix.length),
        );
      if (recordOversized) return;
      if (recordBytes + length > SESSION_ENTRY_MAX_BYTES) {
        recordParts = [];
        recordBytes = SESSION_ENTRY_MAX_BYTES + 1;
        recordOversized = true;
        return;
      }
      recordParts.push(buffer.subarray(start, end));
      recordBytes += length;
    };
    const finishRecord = (): void => {
      if (recordMayBePartial) return;
      inspectedRecords += 1;
      if (inspectedRecords > SESSION_RECORD_MAX_COUNT) {
        stopped = true;
        readerLimitOmission = true;
        return;
      }
      if (recordOversized) {
        // Prefix classification is opportunistic. Object-shaped oversized
        // records retain a chronological marker when the bounded prefix cannot
        // prove their type or role.
        const prefix = recordPrefix.toString("utf8");
        const classification = oversizedMessageEntry(prefix);
        if (classification === "non-message") return;
        const omission = classification
          ?? (prefix.trimStart().startsWith("{") ? { type: "oversized_message", unproven: true } as const : undefined);
        if (omission) {
          newestEntries.push(omission);
          messageEntries += 1;
          if (messageEntries >= SESSION_ENTRY_MAX_COUNT) stopped = true;
        } else {
          invalidLines += 1;
        }
        return;
      }
      if (recordBytes === 0) return;
      if (materializedBytes + recordBytes > SESSION_MATERIALIZED_MAX_BYTES) {
        stopped = true;
        readerLimitOmission = true;
        return;
      }
      materializedBytes += recordBytes;
      const line = recordParts.length === 1
        ? recordParts[0]!.toString("utf8")
        : Buffer.concat(recordParts.reverse(), recordBytes).toString("utf8");
      if (!line.trim()) return;
      try {
        const entry: unknown = JSON.parse(line);
        if (isRecord(entry) && entry.type === "message") {
          const message = entry.message;
          if (isRecord(message) && typeof message.role === "string") {
            newestEntries.push(entry as PersistedTranscriptEntry);
            messageEntries += 1;
            if (messageEntries >= SESSION_ENTRY_MAX_COUNT) stopped = true;
          } else {
            invalidLines += 1;
          }
        } else if (!isRecord(entry) || typeof entry.type !== "string") {
          invalidLines += 1;
        }
      } catch {
        invalidLines += 1;
      }
    };

    while (cursor > 0 && !stopped) {
      const remainingScanBytes = SESSION_SCAN_MAX_BYTES - scannedBytes;
      if (remainingScanBytes <= 0) {
        readerLimitOmission = true;
        largeRecordLimitOmission = recordOversized;
        break;
      }
      const chunkBytes = Math.min(SESSION_SCAN_CHUNK_BYTES, remainingScanBytes);
      const start = Math.max(0, cursor - chunkBytes);
      const buffer = Buffer.allocUnsafe(cursor - start);
      let bytesRead = 0;
      while (bytesRead < buffer.length) {
        const count = readSync(fd, buffer, bytesRead, buffer.length - bytesRead, start + bytesRead);
        if (count === 0) throw new Error("session file ended during read");
        bytesRead += count;
        scannedBytes += count;
      }
      if (cursor === size) recordMayBePartial = size > 0 && buffer[buffer.length - 1] !== 0x0a;

      let segmentEnd = buffer.length;
      for (let index = buffer.length - 1; index >= 0; index -= 1) {
        if (buffer[index] !== 0x0a) continue;
        appendRecordPart(buffer, index + 1, segmentEnd);
        finishRecord();
        resetRecord();
        trimTerminatingCr = true;
        segmentEnd = index;
        if (stopped) {
          if (start > 0 || index > 0) readerLimitOmission = true;
          break;
        }
      }
      if (!stopped) appendRecordPart(buffer, 0, segmentEnd);
      cursor = start;
    }
    if (!stopped && cursor === 0) finishRecord();
    if (!stopped && cursor > 0 && scannedBytes >= SESSION_SCAN_MAX_BYTES) {
      readerLimitOmission = true;
      largeRecordLimitOmission ||= recordOversized;
    }
    if (readerLimitOmission) {
      newestEntries.push({ type: "scan_omission", ...(largeRecordLimitOmission ? { largeRecord: true } : {}) });
    }

    const messages = sessionEntriesToMessages(newestEntries.reverse());
    if (messages.length === 0 && invalidLines > 0) {
      return unavailable("session transcript unavailable: malformed session file");
    }
    return messages;
  } catch {
    return unavailable("session transcript unavailable: could not read session file");
  } finally {
    if (fd !== undefined) {
      try {
        closeSync(fd);
      } catch {
        // The read result is still usable if closing an already-open descriptor fails.
      }
    }
  }
}

/** Extract message entries (dropping the header and non-message entries) as messages. */
export function sessionEntriesToMessages(entries: PersistedTranscriptEntry[]): TranscriptMessage[] {
  const messages: TranscriptMessage[] = [];
  for (const entry of entries) {
    if (entry.type === "scan_omission") {
      const omission = entry as ScanOmissionEntry;
      messages.push({
        role: "omission",
        content: omission.largeRecord ? LARGE_RECORD_SCAN_OMISSION : UNSCANNED_TRANSCRIPT_OMISSION,
      });
      continue;
    }
    if (entry.type === "oversized_message") {
      const omission = entry as OversizedMessageEntry;
      messages.push({
        role: omission.role ?? "omission",
        content: omission.unproven ? OVERSIZED_RECORD_OMISSION : OVERSIZED_MESSAGE_OMISSION,
        ...(omission.role === "toolResult" ? { toolName: omission.toolName ?? "result" } : {}),
      });
      continue;
    }
    if (entry.type !== "message") continue;
    const message = (entry as { message?: TranscriptMessage }).message;
    if (message && typeof message.role === "string") messages.push(message);
  }
  return messages;
}

function* textChunks(content: unknown): Iterable<string> {
  if (typeof content === "string") {
    yield content;
    return;
  }
  if (!Array.isArray(content)) return;
  for (const part of content) {
    if (!isRecord(part) || part.type !== "text") continue;
    const text = part.text;
    if (typeof text === "string") yield text;
  }
}

/** Strip terminal controls while retaining only the newest bounded text. */
function sanitizeTranscriptContent(content: unknown): SanitizedTerminalTail {
  // Keep part boundaries visual, but outside terminal parser input. In
  // particular, a boundary must not consume an ESC at the end of one part
  // before the next actual part supplies its OSC/DCS/APC/SOS/PM introducer.
  return sanitizeTerminalTextTailChunks(textChunks(content), UNTRUSTED_FIELD_MAX, true, true);
}

function summarizeArgs(args: unknown): string {
  if (!isRecord(args) && !Array.isArray(args)) return "";
  return boundedJsonPreview(args, 120);
}

interface WrappedText {
  lines: string[];
  elided: boolean;
}

const wordSegmenter = new Intl.Segmenter(undefined, { granularity: "word" });
const graphemeSegmenter = new Intl.Segmenter(undefined, { granularity: "grapheme" });

/** Word-wrap into a bounded row suffix without eagerly splitting logical lines. */
function wrapTail(text: string, width: number, maxLines: number): WrappedText {
  const cap = Math.max(20, width);
  const rows = new BoundedTail<string>(Math.max(0, maxLines));

  const wrapLogicalLine = (line: string): void => {
    if (!line) {
      rows.push("");
      return;
    }
    let current = "";
    let currentWidth = 0;

    for (const tokenData of wordSegmenter.segment(line)) {
      const token = tokenData.segment;
      const tokenWidth = visibleWidth(token);
      const whitespace = token.trim().length === 0;

      if (tokenWidth > cap && !whitespace) {
        if (current) rows.push(current.trimEnd());
        current = "";
        currentWidth = 0;
        for (const graphemeData of graphemeSegmenter.segment(token)) {
          const grapheme = graphemeData.segment;
          const graphemeWidth = visibleWidth(grapheme);
          if (current && currentWidth + graphemeWidth > cap) {
            rows.push(current);
            current = "";
            currentWidth = 0;
          }
          current += grapheme;
          currentWidth += graphemeWidth;
        }
        continue;
      }

      if (tokenWidth > cap && whitespace) {
        if (current) rows.push(current.trimEnd());
        current = "";
        currentWidth = 0;
        continue;
      }

      if (currentWidth > 0 && currentWidth + tokenWidth > cap) {
        rows.push(current.trimEnd());
        current = whitespace ? "" : token;
        currentWidth = whitespace ? 0 : tokenWidth;
      } else {
        current += token;
        currentWidth += tokenWidth;
      }
    }

    rows.push(current.trimEnd());
  };

  let start = 0;
  while (true) {
    const newline = text.indexOf("\n", start);
    const end = newline < 0 ? text.length : newline;
    wrapLogicalLine(text.slice(start, end));
    if (newline < 0) break;
    start = newline + 1;
  }

  return { lines: rows.values(), elided: rows.elided };
}

/**
 * Render messages into width-clamped, theme-styled lines. Roles use theme roles
 * only (accent user, bold assistant, toolTitle tool calls, dim results/thinking).
 */
export function messagesToLines(messages: TranscriptMessage[], theme: ThemeLike, width: number): string[] {
  const cap = Math.max(20, width);
  const groups: string[][] = [];
  let used = 0;
  let omitted = false;

  // Work backwards so old transcript history is never rendered merely to be
  // discarded. Each message renderer likewise retains its own bounded tail.
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const separatorRows = groups.length === 0 ? 0 : 1;
    const remaining = TRANSCRIPT_CONTENT_MAX_LINES - used - separatorRows;
    if (remaining <= 0) {
      omitted = true;
      break;
    }

    const rendered = renderMessage(messages[index]!, theme, cap, remaining);
    if (rendered.lines.length === 0) {
      if (rendered.elided) {
        omitted = true;
        break;
      }
      continue;
    }
    groups.push(rendered.lines);
    used += separatorRows + rendered.lines.length;
    if (rendered.elided) {
      omitted = true;
      break;
    }
  }

  const lines: string[] = [];
  const push = (text: string) => lines.push(truncateToWidth(text, cap));
  if (omitted) push(theme.fg("dim", "(transcript elided)"));
  for (let index = groups.length - 1; index >= 0; index -= 1) {
    if (index < groups.length - 1) push(theme.fg("dim", "─"));
    for (const line of groups[index]!) push(line);
  }
  if (lines.length === 0) push(theme.fg("dim", "(no transcript yet)"));
  return lines;
}

interface RenderedMessage {
  lines: string[];
  /** Earlier visible content from this message did not fit. */
  elided: boolean;
}

function renderMessage(
  message: TranscriptMessage,
  theme: ThemeLike,
  width: number,
  maxLines: number,
): RenderedMessage {
  if (message.role === "user") return renderUser(message, theme, width, maxLines);
  if (message.role === "assistant") return renderAssistant(message, theme, width, maxLines);
  if (message.role === "toolResult") return renderToolResult(message, theme, width, maxLines);
  if (message.role === "omission") {
    const text = typeof message.content === "string" ? message.content : UNSCANNED_TRANSCRIPT_OMISSION;
    return { lines: [theme.fg("dim", truncateToWidth(text, width))], elided: false };
  }
  return { lines: [], elided: false };
}

/** Model-facing framing prepended to navigator follow-up messages. Persisted
 * with the prompt (honest data), folded out of the rendered transcript (the
 * human already knows they sent it). */
export const FOLLOW_UP_PROMPT_PREFIX =
  "[Direct human follow-up: The reply will be shown to the person who sent this message in the agent transcript, not delivered to an orchestrator.]\n\n";

function renderUser(
  message: TranscriptMessage,
  theme: ThemeLike,
  width: number,
  maxLines: number,
): RenderedMessage {
  const sanitized = sanitizeTranscriptContent(message.content);
  let text = sanitized.text.trim();
  if (text.startsWith(FOLLOW_UP_PROMPT_PREFIX.trim())) {
    text = text.slice(FOLLOW_UP_PROMPT_PREFIX.trim().length).trim();
  }
  if (!text) return { lines: [], elided: sanitized.elided };
  const wrapped = wrapTail(text, width, Math.max(0, maxLines - 1));
  return {
    lines: [theme.fg("accent", theme.bold("▌ user")), ...wrapped.lines],
    elided: sanitized.elided || wrapped.elided,
  };
}

function renderAssistant(
  message: TranscriptMessage,
  theme: ThemeLike,
  width: number,
  maxLines: number,
): RenderedMessage {
  if (typeof message.content === "string") {
    const sanitized = sanitizeTranscriptContent(message.content);
    const text = sanitized.text.trim();
    if (!text) return { lines: [], elided: sanitized.elided };
    const wrapped = wrapTail(text, width, Math.max(0, maxLines - 1));
    return {
      lines: [theme.bold("● assistant"), ...wrapped.lines],
      elided: sanitized.elided || wrapped.elided,
    };
  }

  const content = Array.isArray(message.content) ? message.content : [];
  const bodyLimit = Math.max(0, maxLines - 1);
  const body = new BoundedTail<string>(bodyLimit);
  const sanitizer = new TerminalTextSanitizer();
  let elided = false;

  for (const part of content) {
    if (!isRecord(part)) continue;
    if (part.type === "text" && typeof part.text === "string") {
      const sanitized = sanitizer.sanitizeTail(part.text, UNTRUSTED_FIELD_MAX, true);
      const text = sanitized.text.trim();
      elided ||= sanitized.elided;
      if (!text) continue;
      const wrapped = wrapTail(text, width, bodyLimit);
      for (const line of wrapped.lines) body.push(line);
      elided ||= wrapped.elided;
    } else if (part.type === "thinking" && typeof part.thinking === "string") {
      const sanitized = sanitizer.sanitizeTail(part.thinking, UNTRUSTED_FIELD_MAX, true);
      const thinking = sanitized.text.trim();
      elided ||= sanitized.elided;
      if (!thinking) continue;
      const wrapped = wrapTail(thinking, width, bodyLimit);
      for (const line of wrapped.lines) body.push(theme.fg("dim", line));
      elided ||= wrapped.elided;
    } else if (part.type === "toolCall") {
      const name = typeof part.name === "string" ? sanitizeTerminalText(part.name) : "tool";
      const args = sanitizeTerminalText(summarizeArgs(part.arguments));
      body.push(theme.fg("toolTitle", `  ⚙ ${name}`) + (args ? theme.fg("dim", ` ${args}`) : ""));
    }
  }

  const lines = body.values();
  elided ||= body.elided;
  return lines.length > 0
    ? { lines: [theme.bold("● assistant"), ...lines], elided }
    : { lines: [], elided };
}

function renderToolResult(
  message: TranscriptMessage,
  theme: ThemeLike,
  width: number,
  maxLines: number,
): RenderedMessage {
  const sanitized = sanitizeTranscriptContent(message.content);
  const text = sanitized.text.trim();
  const name = typeof message.toolName === "string" ? sanitizeTerminalText(message.toolName) : "result";
  const heading = theme.fg("dim", `  ↳ ${name}`);
  const body: string[] = [];
  let contentElided = sanitized.elided;
  if (text) {
    const wrapped = wrapTail(text, width - 4, RESULT_MAX_LINES);
    body.push(...wrapped.lines.map((line) => theme.fg("dim", `    ${line}`)));
    contentElided ||= wrapped.elided;
  }
  if (contentElided) body.unshift(theme.fg("dim", "    … earlier lines elided"));

  const lines = [heading, ...body];
  const limit = Math.max(0, maxLines);
  if (lines.length <= limit) return { lines, elided: false };
  if (limit === 0) return { lines: [], elided: true };
  return {
    lines: limit === 1 ? [heading] : [heading, ...lines.slice(-(limit - 1))],
    elided: true,
  };
}
