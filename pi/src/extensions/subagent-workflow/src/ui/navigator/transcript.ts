/** Read bounded recent messages from the child-written Pi session, for live and historical views. */

import { closeSync, fstatSync, openSync, readSync } from "node:fs";
import type { SessionEntry } from "@earendil-works/pi-coding-agent";
import { isRecord } from "../../util.js";

/** Minimal structural view of pi's AgentMessage union (role + content parts). */
export interface TranscriptMessage {
  role: string;
  content?: unknown;
  toolName?: string;
  toolCallId?: string;
  isError?: boolean;
  responseId?: string;
  timestamp?: number;
  entryId?: string;
  /** Carries an in-flight message's UI identity across its first persisted entry. */
  viewId?: string;
  omitted?: boolean;
}

/** Model-facing framing persisted with navigator follow-ups, folded out of the human-facing task. */
export const FOLLOW_UP_PROMPT_PREFIX =
  "[Direct human follow-up: The reply will be shown to the person who sent this message in the agent transcript, not delivered to an orchestrator.]\n\n";

const SESSION_SCAN_CHUNK_BYTES = 256 * 1024;
export const SESSION_SCAN_MAX_BYTES = 4 * 1024 * 1024;
export const SESSION_ENTRY_MAX_BYTES = 512 * 1024;
export const OVERSIZED_MESSAGE_OMISSION = "(oversized persisted message omitted)";
/** Neutral wording: the record was object-shaped but never proven to be a message. */
export const OVERSIZED_RECORD_OMISSION = "(oversized session record omitted)";
/** Neutral wording: unscanned records may include non-message entries, so this
 * never claims transcript messages were omitted. */
export const UNSCANNED_TRANSCRIPT_OMISSION = "(older session records not scanned: reader limit reached)";
export const LARGE_RECORD_SCAN_OMISSION =
  "(a large session record and older records not scanned: reader limit reached)";
const SESSION_ENTRY_PREFIX_BYTES = 256;
const SESSION_ENTRY_MAX_COUNT = 512;
/** Complete records inspected per render, including malformed and non-message records. */
export const SESSION_RECORD_MAX_COUNT = 4_096;
const SESSION_MATERIALIZED_MAX_BYTES = 2 * 1024 * 1024;

type StandardTranscriptRole = "user" | "assistant" | "toolResult";
type OversizedMessageEntry = {
  type: "oversized_message";
  role?: StandardTranscriptRole;
  toolName?: string;
  unproven?: true;
};
type OversizedPrefixClassification = OversizedMessageEntry | "non-message" | undefined;
type ScanOmissionEntry = { type: "scan_omission"; largeRecord?: boolean };
type PersistedTranscriptEntry = SessionEntry | { type: string } | OversizedMessageEntry | ScanOmissionEntry;

type PrefixScanStatus = "complete" | "incomplete" | "invalid";
interface PrefixScanResult {
  status: PrefixScanStatus;
  next: number;
}
interface PrefixStringResult extends PrefixScanResult {
  value?: string;
}

function skipPrefixWhitespace(input: string, start: number): number {
  let index = start;
  while (index < input.length && /\s/u.test(input[index]!)) index += 1;
  return index;
}

function scanPrefixString(input: string, start: number): PrefixStringResult {
  if (input[start] !== '"') return { status: "invalid", next: start };
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
    if (character !== '"') continue;
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
    if (prefix[index] === '"') {
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
    return /[\s,}\]]/u.test(prefix[next]!) ? { status: "complete", next } : { status: "invalid", next };
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
      if (context === "top" && key.value === "type" && prefix[index] === '"') {
        value = captureString((captured) => {
          topLevelType = captured;
        });
      } else if (context === "top" && key.value === "message" && prefix[index] === "{") {
        messageObjectSeen = true;
        value = scanObject(index, depth + 1, "message");
      } else if (context === "message" && key.value === "role" && prefix[index] === '"') {
        value = captureString((captured) => {
          if (captured === "user" || captured === "assistant" || captured === "toolResult") role = captured;
        });
      } else if (context === "message" && key.value === "toolName" && prefix[index] === '"') {
        value = captureString((captured) => {
          if (captured.length <= 120) toolName = captured;
        });
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
  return [{ role: "omission", content: `(${reason})` }];
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
      recordPrefix =
        prefixPart.length >= SESSION_ENTRY_PREFIX_BYTES
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
        const omission =
          classification ??
          (prefix.trimStart().startsWith("{") ? ({ type: "oversized_message", unproven: true } as const) : undefined);
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
      const line =
        recordParts.length === 1
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
        omitted: true,
        content: omission.unproven ? OVERSIZED_RECORD_OMISSION : OVERSIZED_MESSAGE_OMISSION,
        ...(omission.role === "toolResult" ? { toolName: omission.toolName ?? "result" } : {}),
      });
      continue;
    }
    if (entry.type !== "message") continue;
    const message = (entry as { message?: TranscriptMessage }).message;
    if (message && typeof message.role === "string") {
      messages.push({ ...message, entryId: (entry as { id?: string }).id });
    }
  }
  return messages;
}
