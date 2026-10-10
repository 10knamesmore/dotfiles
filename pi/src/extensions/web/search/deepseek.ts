import { WebError } from "../errors.js";
import { requestText } from "../http.js";
import type { SearchRequest, SearchResult, SearchSource } from "./types.js";

const DEEPSEEK_MESSAGES_URL = "https://api.deepseek.com/anthropic/v1/messages";
const DEEPSEEK_MODEL = "deepseek-flash";
const REQUEST_TIMEOUT_MS = 180_000;
const MAX_OUTPUT_TOKENS = 4_096;
const MAX_SEARCH_USES = 5;
const MAX_ANSWER_CHARS = 12_000;
const MAX_TITLE_CHARS = 500;

/**
 * Run DeepSeek's hosted web search: one Messages request carrying the native web_search server tool.
 * The backend encrypts the pages it reads, so sources keep titles and URLs but no excerpts.
 */
export async function searchDeepseek(request: SearchRequest, signal?: AbortSignal): Promise<SearchResult> {
  const apiKey = process.env.DEEPSEEK_API_KEY?.trim();
  if (!apiKey) {
    throw new WebError("authentication", "DeepSeek search needs DEEPSEEK_API_KEY in the environment.");
  }

  const response = await requestText(
    DEEPSEEK_MESSAGES_URL,
    {
      method: "POST",
      headers: {
        "x-api-key": apiKey,
        "anthropic-version": "2023-06-01",
        "content-type": "application/json",
      },
      body: JSON.stringify({
        model: DEEPSEEK_MODEL,
        max_tokens: MAX_OUTPUT_TOKENS,
        messages: [{ role: "user", content: request.query }],
        tools: [{ type: "web_search_20250305", name: "web_search", max_uses: MAX_SEARCH_USES }],
        tool_choice: { type: "tool", name: "web_search" },
      }),
    },
    signal,
    REQUEST_TIMEOUT_MS,
  );

  const payload = parseJson(response.text);
  if (!isRecord(payload) || !Array.isArray(payload.content)) {
    throw new WebError("invalid_response", "DeepSeek search returned a malformed response.");
  }

  const answerParts: string[] = [];
  const sources: SearchSource[] = [];
  const seen = new Set<string>();
  let searched = false;
  for (const block of payload.content) {
    if (!isRecord(block)) continue;
    if (block.type === "web_search_tool_result") searched = true;
    if (block.type === "text" && typeof block.text === "string" && block.text.trim()) {
      answerParts.push(block.text.trim());
    }
    if (block.type !== "web_search_tool_result" || !Array.isArray(block.content)) continue;
    for (const item of block.content) {
      if (!isRecord(item) || item.type !== "web_search_result") continue;
      const url = cleanUrl(item.url);
      if (url === undefined || seen.has(url) || sources.length >= request.limit) continue;
      seen.add(url);
      sources.push({ title: boundedText(item.title, MAX_TITLE_CHARS) || url, url, snippet: "" });
    }
  }
  if (!searched) {
    throw new WebError("invalid_response", "DeepSeek search returned no web search results.");
  }
  return {
    provider: "deepseek",
    answer: answerParts.join("\n").trim().slice(0, MAX_ANSWER_CHARS),
    sources,
  };
}

function cleanUrl(raw: unknown): string | undefined {
  if (typeof raw !== "string") return undefined;
  try {
    const url = new URL(raw);
    if (url.protocol !== "http:" && url.protocol !== "https:") return undefined;
    url.hash = "";
    // The search backend sometimes carries over a signed access token from the source page.
    if (url.searchParams.has("accessToken")) url.searchParams.delete("accessToken");
    return url.toString();
  } catch {
    return undefined;
  }
}

function boundedText(value: unknown, maximum: number): string {
  return typeof value === "string" ? value.trim().slice(0, maximum) : "";
}

function parseJson(value: string): unknown {
  try {
    return JSON.parse(value) as unknown;
  } catch {
    return undefined;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
