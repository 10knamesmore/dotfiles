import { truncateHead } from "@earendil-works/pi-coding-agent";
import { WebError } from "../errors.js";

export const DEFAULT_PAGE_CHARS = 30_000;
const MAX_PAGE_BYTES = 40 * 1024;
const MAX_PAGE_LINES = 1_800;

/** Offsets count UTF-16 code units, as String.slice does; nextOffset always resumes the same content without gaps. */
export function pageContent(
  content: string,
  offset = 0,
  limit = DEFAULT_PAGE_CHARS,
): {
  text: string;
  offset: number;
  nextOffset?: number;
  totalChars: number;
} {
  if (!Number.isInteger(offset) || offset < 0 || offset > content.length) {
    throw new WebError("invalid_input", `offset must be between 0 and ${content.length}.`);
  }
  if (!Number.isInteger(limit) || limit < 2 || limit > DEFAULT_PAGE_CHARS) {
    throw new WebError("invalid_input", `limit must be between 2 and ${DEFAULT_PAGE_CHARS}.`);
  }
  if (isHighSurrogate(content.charCodeAt(offset - 1)) && isLowSurrogate(content.charCodeAt(offset))) {
    throw new WebError("invalid_input", "offset splits a Unicode character; use the returned nextOffset.");
  }
  let candidate = content.slice(offset, offset + limit);
  if (isHighSurrogate(candidate.charCodeAt(candidate.length - 1))) candidate = candidate.slice(0, -1);
  const truncated = truncateHead(candidate, { maxBytes: MAX_PAGE_BYTES, maxLines: MAX_PAGE_LINES });
  // An oversized line, possibly after a blank line, can leave Pi's line truncator empty. Keep a complete UTF-8 prefix to advance.
  const text =
    candidate.length > 0 && truncated.content.length === 0
      ? new TextDecoder().decode(Buffer.from(candidate).subarray(0, MAX_PAGE_BYTES), { stream: true })
      : truncated.content;
  const end = offset + text.length;
  return { text, offset, ...(end < content.length ? { nextOffset: end } : {}), totalChars: content.length };
}

function isHighSurrogate(value: number): boolean {
  return value >= 0xd800 && value <= 0xdbff;
}

function isLowSurrogate(value: number): boolean {
  return value >= 0xdc00 && value <= 0xdfff;
}
