/** Project child messages into readable blocks while retaining their turn and tool-call identity. */
import type { SubagentResult } from "../../types.js";
import { isRecord } from "../../util.js";
import { boundedJsonPreview, sanitizeTerminalText, sanitizeTerminalTextTailChunks, TerminalTextSanitizer, UNTRUSTED_FIELD_MAX, type SanitizedTerminalTail } from "../sanitize.js";
import { FOLLOW_UP_PROMPT_PREFIX, type TranscriptMessage } from "./transcript.js";

/** Only a persisted terminal result can identify final output, including validated JSON. */
export type TranscriptResult = Pick<SubagentResult, "status" | "text" | "structured">;

interface BlockPosition {
  id: string;
  /** Identity of the source assistant message; labels count only the loaded turns. */
  turn?: string;
}

type TextBlock = BlockPosition & {
  kind: "task" | "thinking" | "output" | "notice";
  body: SanitizedTerminalTail;
  final?: boolean;
  structured?: boolean;
};

export type ToolBlock = BlockPosition & {
  kind: "tool";
  name: string;
  target: string;
  arguments?: SanitizedTerminalTail;
  result?: SanitizedTerminalTail;
  status: "completed" | "failed" | "waiting" | "unavailable";
  /** A result can be retained after its invocation fell outside the loaded history. */
  missingCall?: boolean;
};

export type TranscriptBlock = TextBlock | ToolBlock;

function* textChunks(content: unknown): Iterable<string> {
  if (typeof content === "string") yield content;
  else if (Array.isArray(content)) {
    for (const part of content) if (isRecord(part) && part.type === "text" && typeof part.text === "string") yield part.text;
  }
}

function textBody(content: unknown): SanitizedTerminalTail {
  return sanitizeTerminalTextTailChunks(textChunks(content), UNTRUSTED_FIELD_MAX, true, true);
}

function jsonBody(value: unknown): SanitizedTerminalTail {
  // Source values are persisted JSON or schema-validated tool arguments.
  return textBody(JSON.stringify(value, null, 2) ?? "");
}

function messageIdentity(message: TranscriptMessage, index: number): string {
  return `${message.role}:${message.viewId ?? message.entryId ?? message.responseId ?? message.timestamp ?? index}`;
}

function toolStatus(result: TranscriptMessage | undefined): ToolBlock["status"] {
  if (!result || result.omitted || typeof result.isError !== "boolean") return "unavailable";
  return result.isError ? "failed" : "completed";
}

/** Match only known call IDs. Missing history never becomes a fabricated success or active tool. */
export function buildTranscriptBlocks(
  messages: readonly TranscriptMessage[],
  outcome: TranscriptResult | undefined,
  live: boolean,
): TranscriptBlock[] {
  const results = new Map<string, TranscriptMessage>();
  const calls = new Set<string>();
  let lastAssistant = -1;
  for (const [index, message] of messages.entries()) {
    if (message.role === "toolResult" && message.toolCallId) results.set(message.toolCallId, message);
    if (message.role === "assistant") {
      lastAssistant = index;
      if (Array.isArray(message.content)) {
        for (const part of message.content) if (isRecord(part) && part.type === "toolCall" && typeof part.id === "string") calls.add(part.id);
      }
    }
  }

  const hasOmissionsAfterAssistant = messages.slice(lastAssistant + 1).some((message) => message.role === "omission" || message.omitted);
  const completed = outcome?.status === "completed";
  const structured = completed && Object.hasOwn(outcome, "structured");
  const lastMessage = messages[lastAssistant];
  const finalMessageMatches = completed && !structured && !!outcome.text && lastMessage
    && [...textChunks(lastMessage.content)].join("\n") === outcome.text;
  const blocks: TranscriptBlock[] = [];
  const identities = new Map<string, number>();

  for (const [index, message] of messages.entries()) {
    const base = messageIdentity(message, index);
    const occurrence = identities.get(base) ?? 0;
    identities.set(base, occurrence + 1);
    const id = `${base}:${occurrence}`;
    if (message.omitted || message.role === "omission") {
      blocks.push({ id, kind: "notice", body: textBody(message.content) });
      continue;
    }
    if (message.role === "user") {
      const body = textBody(message.content);
      if (body.text.startsWith(FOLLOW_UP_PROMPT_PREFIX.trim())) body.text = body.text.slice(FOLLOW_UP_PROMPT_PREFIX.trim().length).trimStart();
      blocks.push({ id, kind: "task", body });
      continue;
    }
    if (message.role === "toolResult") {
      if (message.toolCallId && calls.has(message.toolCallId)) continue;
      blocks.push({
        id, kind: "tool", name: sanitizeTerminalText(message.toolName ?? "tool"), target: "Call not loaded",
        result: textBody(message.content), status: toolStatus(message), missingCall: true,
      });
      continue;
    }
    if (message.role !== "assistant") continue;
    const parts = typeof message.content === "string" ? [{ type: "text", text: message.content }] : message.content;
    if (!Array.isArray(parts)) continue;
    const sanitizer = new TerminalTextSanitizer();
    for (const [partIndex, part] of parts.entries()) {
      if (!isRecord(part)) continue;
      const position = { id: `${id}:${partIndex}`, turn: id };
      if (part.type === "thinking" || part.type === "text") {
        const raw = part.type === "thinking" ? part.thinking : part.text;
        if (typeof raw !== "string") continue;
        const body = sanitizer.sanitizeTail(raw, UNTRUSTED_FIELD_MAX, true);
        if (!body.text.trim() && !body.elided) continue;
        if (part.type === "text" && index === lastAssistant && finalMessageMatches) continue;
        blocks.push({ ...position, kind: part.type === "thinking" ? "thinking" : "output", body });
      } else if (part.type === "toolCall") {
        const callId = typeof part.id === "string" ? part.id : undefined;
        const result = callId ? results.get(callId) : undefined;
        const args = isRecord(part.arguments) ? part.arguments : undefined;
        const target = args?.path ?? args?.command ?? args?.query;
        blocks.push({
          ...position,
          id: callId ? `tool:${callId}` : position.id,
          kind: "tool",
          name: sanitizeTerminalText(typeof part.name === "string" ? part.name : "tool"),
          target: sanitizeTerminalText(typeof target === "string" ? target : boundedJsonPreview(part.arguments)),
          arguments: jsonBody(part.arguments),
          result: result ? textBody(result.content) : undefined,
          status: result ? toolStatus(result) : live && index === lastAssistant && !hasOmissionsAfterAssistant ? "waiting" : "unavailable",
        });
      }
    }
  }
  if (completed && (structured || outcome.text)) {
    blocks.push({
      id: "final-result", kind: "output", final: true, structured,
      body: structured ? jsonBody(outcome.structured) : textBody(outcome.text),
    });
  }
  return blocks;
}

export function blockCanCollapse(block: TranscriptBlock): boolean {
  return block.kind === "thinking" || block.kind === "task" || block.kind === "tool";
}

/** Explicit user choices override the default of opening failed tool calls. */
export function blockIsExpanded(block: TranscriptBlock, choices: ReadonlyMap<string, boolean>): boolean {
  return !blockCanCollapse(block) || (choices.get(block.id) ?? (block.kind === "tool" && block.status === "failed"));
}
