import { asWebError, WebError } from "../errors.js";
import { requestText } from "../http.js";
import { boundedText, cleanUrl, displayDate, isRecord, parseJson } from "./response.js";
import type { SearchRequest, SearchSource } from "./types.js";

const SEARCH_URL = "https://api.deepseek.com/v1/tools/search";
const REQUEST_TIMEOUT_MS = 20_000;
const MAX_QUERY_CHARS = 128;
const MAX_TITLE_CHARS = 500;
const MAX_SITE_NAME_CHARS = 200;
const MAX_SUMMARY_CHARS = 2_000;
const MAX_ATTEMPTS = 3;
const RETRY_BASE_DELAY_MS = 500;

/** Run DeepSeek's hosted web search: structured ranked sources, no model answer. */
export async function searchDeepseek(request: SearchRequest, signal?: AbortSignal): Promise<SearchSource[]> {
  const apiKey = process.env.DEEPSEEK_API_KEY?.trim();
  if (!apiKey) {
    throw new WebError("authentication", "DeepSeek search needs DEEPSEEK_API_KEY in the environment.");
  }

  const query = request.query.trim();
  if ([...query].length > MAX_QUERY_CHARS) {
    throw new WebError("invalid_input", `DeepSeek search queries are limited to ${MAX_QUERY_CHARS} characters.`);
  }
  const body = await requestSearch(
    apiKey,
    { query, count: request.limit, ori_question: request.oriQuestion },
    signal,
  );
  return parseResults(body, request.limit);
}

/** The API guide marks 408, 429 and 5xx as retryable; parallel tool calls routinely reach the account QPS limit. */
async function requestSearch(apiKey: string, payload: Record<string, unknown>, signal?: AbortSignal): Promise<string> {
  for (let attempt = 1; ; attempt += 1) {
    signal?.throwIfAborted();
    try {
      return (
        await requestText(
          SEARCH_URL,
          {
            method: "POST",
            headers: { authorization: `Bearer ${apiKey}`, "content-type": "application/json" },
            body: JSON.stringify(payload),
          },
          signal,
          REQUEST_TIMEOUT_MS,
        )
      ).text;
    } catch (error) {
      const failure = searchError(error);
      if (attempt >= MAX_ATTEMPTS || !isRetryable(failure)) throw failure;
      await retryDelay(attempt, signal);
    }
  }
}

function isRetryable(error: WebError): boolean {
  return error.status === 408 || error.status === 429 || (error.status ?? 0) >= 500;
}

/** Jitter keeps parallel tool calls from retrying in lockstep against the same QPS limit. */
function retryDelay(attempt: number, signal?: AbortSignal): Promise<void> {
  const milliseconds = (RETRY_BASE_DELAY_MS + (Math.random() * RETRY_BASE_DELAY_MS) / 2) * 2 ** (attempt - 1);
  return new Promise((resolve, reject) => {
    const onAbort = () => {
      clearTimeout(timer);
      reject(new WebError("cancelled", "Request cancelled."));
    };
    const timer = setTimeout(() => {
      signal?.removeEventListener("abort", onAbort);
      resolve();
    }, milliseconds);
    signal?.addEventListener("abort", onAbort, { once: true });
  });
}

function parseResults(text: string, limit: number): SearchSource[] {
  const payload = parseJson(text);
  if (!isRecord(payload) || !Array.isArray(payload.results)) {
    throw new WebError("invalid_response", "DeepSeek search returned a malformed response.");
  }

  const sources: SearchSource[] = [];
  const seen = new Set<string>();
  for (const item of payload.results) {
    if (sources.length >= limit) break;
    if (!isRecord(item)) continue;
    const url = cleanUrl(item.url);
    if (url === undefined || seen.has(url)) continue;
    seen.add(url);
    const source: SearchSource = {
      providers: ["deepseek"],
      title: boundedText(item.title, MAX_TITLE_CHARS) || url,
      url,
      summary:
        boundedText(item.summary, MAX_SUMMARY_CHARS) || boundedText(item.snippet, MAX_SUMMARY_CHARS),
    };
    const date = displayDate(item.date);
    if (date) source.date = date;
    const siteName = boundedText(item.site_name, MAX_SITE_NAME_CHARS);
    if (siteName) source.siteName = siteName;
    const authority = authorityValue(item.authority);
    if (authority !== undefined) source.authority = authority;
    const rerankScore = rerankValue(item.rerank_score);
    if (rerankScore !== undefined) source.rerankScore = rerankScore;
    sources.push(source);
  }
  return sources;
}

/** Clear messages for statuses the agent can act on; everything else keeps its transport classification. */
function searchError(error: unknown): WebError {
  if (error instanceof WebError) {
    if (error.status === 401) return new WebError("authentication", "DeepSeek rejected the API key.", 401);
    if (error.status === 402) {
      return new WebError("http", "The DeepSeek account balance is insufficient.", 402);
    }
    if (error.status === 429) {
      return new WebError("http", "DeepSeek search rate limit reached; retry shortly.", 429);
    }
  }
  return asWebError(error);
}

/** Authority spans 0–4; the API's -1 means unknown and stays absent. */
function authorityValue(value: unknown): number | undefined {
  return typeof value === "number" && Number.isInteger(value) && value >= 0 && value <= 4 ? value : undefined;
}

/** Rerank scores span 0–3; two decimals keep the ranking readable. */
function rerankValue(value: unknown): number | undefined {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0 || value > 3) return undefined;
  return Math.round(value * 100) / 100;
}
