import type { ExtensionContext } from "@earendil-works/pi-coding-agent";

import { WebError } from "../errors.js";
import { requestText } from "../http.js";
import type { SearchRequest, SearchResult, SearchSource } from "./types.js";

const CODEX_BASE_URL = "https://chatgpt.com/backend-api";
const CODEX_RESPONSES_URL = `${CODEX_BASE_URL}/codex/responses`;
const SEARCH_TIMEOUT_MS = 60_000;
const MAX_ANSWER_CHARS = 12_000;
const MAX_SNIPPET_CHARS = 300;
const EXCLUDED_MODEL_SEGMENTS = new Set(["pro", "ultra"]);

type CodexModel = ReturnType<ExtensionContext["modelRegistry"]["getAll"]>[number];
type JsonObject = Record<string, unknown>;

function isObject(value: unknown): value is JsonObject {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function isOfficialCodexModel(model: CodexModel | undefined): model is CodexModel {
  if (!model || model.provider !== "openai-codex" || model.api !== "openai-codex-responses") return false;
  try {
    const url = new URL(model.baseUrl);
    return url.protocol === "https:"
      && url.hostname.toLowerCase() === "chatgpt.com"
      && url.port === ""
      && url.pathname.replace(/\/+$/u, "") === "/backend-api"
      && url.username === ""
      && url.password === ""
      && url.search === ""
      && url.hash === "";
  } catch {
    return false;
  }
}

function pickPreferredModel(models: readonly CodexModel[]): CodexModel | undefined {
  const candidates = models
    .filter((model) => model.provider === "openai-codex" && model.api === "openai-codex-responses")
    .filter((model) => !model.id.split("-").some((segment) => EXCLUDED_MODEL_SEGMENTS.has(segment)))
    .sort((left, right) => right.id.localeCompare(left.id, undefined, { numeric: true }));
  return candidates.find((model) => model.id.includes("terra"))
    ?? candidates.find((model) => /^gpt-\d+(?:\.\d+)?$/u.test(model.id))
    ?? candidates[0];
}

function selectModel(ctx: ExtensionContext, modelId: string | undefined): CodexModel {
  let models: ReturnType<ExtensionContext["modelRegistry"]["getAll"]>;
  try {
    models = ctx.modelRegistry.getAll();
  } catch {
    throw new WebError("authentication", "Codex model registry is unavailable.");
  }

  if (modelId !== undefined) {
    const configuredModelId = modelId.trim();
    if (!configuredModelId) throw new WebError("invalid_input", "Codex model ID must not be empty.");
    const configured = models.find((model) => model.provider === "openai-codex" && model.id === configuredModelId);
    if (!configured) throw new WebError("unsupported", `Codex model "${configuredModelId}" is not registered.`);
    if (!isOfficialCodexModel(configured)) {
      throw new WebError("unsupported", "Codex search requires the official openai-codex endpoint.");
    }
    return configured;
  }

  const current = ctx.model as CodexModel | undefined;
  if (isOfficialCodexModel(current)) return current;
  const preferred = pickPreferredModel(models);
  if (!preferred) throw new WebError("unsupported", "No openai-codex search model is registered.");
  if (!isOfficialCodexModel(preferred)) {
    throw new WebError("unsupported", "Codex search requires the official openai-codex endpoint.");
  }
  return preferred;
}

function decodeJwtAccountId(token: string): string | undefined {
  const payload = token.split(".")[1];
  if (!payload) return undefined;
  try {
    const decoded: unknown = JSON.parse(Buffer.from(payload, "base64url").toString("utf8"));
    if (!isObject(decoded)) return undefined;
    const auth = decoded["https://api.openai.com/auth"];
    if (!isObject(auth)) return undefined;
    const accountId = auth.chatgpt_account_id;
    return typeof accountId === "string" && accountId.trim() ? accountId.trim() : undefined;
  } catch {
    return undefined;
  }
}

function normalizeDomain(raw: string): { domain: string; blocked: boolean } | undefined {
  const trimmed = raw.trim();
  const blocked = trimmed.startsWith("-");
  const value = (blocked ? trimmed.slice(1) : trimmed).trim();
  if (!value) return undefined;
  try {
    const url = new URL(value.includes("://") ? value : `https://${value}`);
    if (url.protocol !== "http:" && url.protocol !== "https:") return undefined;
    const domain = url.hostname.toLowerCase().replace(/\.$/u, "");
    return domain ? { domain, blocked } : undefined;
  } catch {
    return undefined;
  }
}

function domainFilters(domains: readonly string[] | undefined): JsonObject | undefined {
  const allowed = new Set<string>();
  const blocked = new Set<string>();
  for (const raw of domains ?? []) {
    const normalized = normalizeDomain(raw);
    if (normalized) (normalized.blocked ? blocked : allowed).add(normalized.domain);
  }
  if (allowed.size === 0 && blocked.size === 0) return undefined;
  return {
    ...(allowed.size > 0 ? { allowed_domains: [...allowed].slice(0, 100) } : {}),
    ...(blocked.size > 0 ? { blocked_domains: [...blocked].slice(0, 100) } : {}),
  };
}

function buildBody(request: SearchRequest, model: string): JsonObject {
  const recencyLabels = { day: "past 24 hours", week: "past week", month: "past month", year: "past year" } as const;
  const instructions = [
    "Search the web and return a concise answer grounded only in the web results.",
    "Include clickable source citations in the response text when possible.",
    `Prefer around ${request.limit} distinct sources.`,
  ];
  // Responses supports domain filters directly, but recency only as search guidance.
  if (request.recency) instructions.push(`Prefer sources from the ${recencyLabels[request.recency]}.`);
  const filters = domainFilters(request.domains);
  return {
    model,
    instructions: instructions.join(" "),
    input: [{ role: "user", content: [{ type: "input_text", text: request.query }] }],
    tools: [{ type: "web_search", ...(filters ? { filters } : {}) }],
    include: ["web_search_call.action.sources"],
    store: false,
    stream: true,
    tool_choice: "required",
    parallel_tool_calls: true,
  };
}

function parseSseEvents(text: string): JsonObject[] {
  const events: JsonObject[] = [];
  for (const block of text.replace(/\r\n/g, "\n").split(/\n\n+/u)) {
    const data = block.split("\n")
      .filter((line) => line.startsWith("data:"))
      .map((line) => line.slice(5).trimStart())
      .join("\n")
      .trim();
    if (!data || data === "[DONE]") continue;
    try {
      const parsed: unknown = JSON.parse(data);
      if (isObject(parsed)) events.push(parsed);
    } catch {
      // A malformed non-terminal event is ignored; a valid completion is still required below.
    }
  }
  return events;
}

function isWebSearchCall(value: unknown): value is JsonObject {
  return isObject(value) && value.type === "web_search_call";
}

function parseResponse(text: string): { output: unknown[]; webSearchCallSeen: boolean } {
  const trimmed = text.trim();
  if (trimmed.startsWith("{")) {
    let payload: unknown;
    try {
      payload = JSON.parse(trimmed);
    } catch {
      throw new WebError("invalid_response", "Codex search returned invalid JSON.");
    }
    if (!isObject(payload)) throw new WebError("invalid_response", "Codex search returned an invalid response.");
    if (payload.status === "failed") throw new WebError("invalid_response", "Codex search failed.");
    if (payload.status === "incomplete") throw new WebError("invalid_response", "Codex search returned an incomplete response.");
    const output = Array.isArray(payload.output) ? payload.output : [];
    return { output, webSearchCallSeen: output.some(isWebSearchCall) };
  }

  const completedItems: unknown[] = [];
  let completedResponse: JsonObject | undefined;
  let webSearchCallSeen = false;
  let failed = false;
  let incomplete = false;
  for (const event of parseSseEvents(text)) {
    const type = typeof event.type === "string" ? event.type : "";
    if (type === "error" || type === "response.failed") failed = true;
    if (type === "response.incomplete") incomplete = true;
    if (type.startsWith("response.web_search_call")) webSearchCallSeen = true;
    if (type === "response.output_item.done" && event.item !== undefined) {
      completedItems.push(event.item);
      webSearchCallSeen ||= isWebSearchCall(event.item);
    }
    if ((type === "response.completed" || type === "response.done") && isObject(event.response)) {
      completedResponse = event.response;
      failed ||= event.response.status === "failed";
      incomplete ||= event.response.status === "incomplete";
    }
  }
  if (failed) throw new WebError("invalid_response", "Codex search failed.");
  if (incomplete) throw new WebError("invalid_response", "Codex search returned an incomplete response.");
  if (!completedResponse) throw new WebError("invalid_response", "Codex search returned no completed response.");
  const responseOutput = Array.isArray(completedResponse.output) ? completedResponse.output : [];
  const output = responseOutput.length > 0 ? responseOutput : completedItems;
  return { output, webSearchCallSeen: webSearchCallSeen || output.some(isWebSearchCall) };
}

function cleanUrl(raw: unknown): string | undefined {
  if (typeof raw !== "string") return undefined;
  try {
    const url = new URL(raw);
    if (url.protocol !== "http:" && url.protocol !== "https:") return undefined;
    if (url.searchParams.get("utm_source") === "openai") url.searchParams.delete("utm_source");
    return url.toString();
  } catch {
    return undefined;
  }
}

function capText(value: unknown, limit: number): string {
  return typeof value === "string" ? value.trim().slice(0, limit) : "";
}

function citationSnippet(text: string, start: unknown, end: unknown): string {
  if (typeof start !== "number" || typeof end !== "number") return "";
  return capText(text.slice(Math.max(0, start - 100), Math.min(text.length, end + 100))
    .replace(/\[([^\]]*)\]\([^)]*\)/gu, "$1"), MAX_SNIPPET_CHARS);
}

function extractResult(output: unknown[], limit: number): SearchResult {
  const answerParts: string[] = [];
  const sources: SearchSource[] = [];
  const seen = new Set<string>();
  const addSource = (urlValue: unknown, titleValue: unknown, snippetValue: unknown): void => {
    const url = cleanUrl(urlValue);
    if (!url || seen.has(url) || sources.length >= limit) return;
    seen.add(url);
    sources.push({
      title: capText(titleValue, 500) || url,
      url,
      snippet: capText(snippetValue, MAX_SNIPPET_CHARS),
    });
  };

  for (const item of output) {
    if (!isObject(item) || item.type !== "message" || !Array.isArray(item.content)) continue;
    for (const part of item.content) {
      if (!isObject(part)) continue;
      const text = typeof part.text === "string" ? part.text : "";
      if (text.trim()) answerParts.push(text.trim());
      if (!Array.isArray(part.annotations)) continue;
      for (const annotation of part.annotations) {
        if (!isObject(annotation) || annotation.type !== "url_citation") continue;
        addSource(annotation.url, annotation.title, citationSnippet(text, annotation.start_index, annotation.end_index));
      }
    }
  }

  for (const item of output) {
    if (!isWebSearchCall(item)) continue;
    const actionSources = isObject(item.action) ? item.action.sources : undefined;
    for (const group of [actionSources, item.sources, item.results]) {
      if (!Array.isArray(group)) continue;
      for (const source of group) {
        if (!isObject(source)) continue;
        addSource(source.url ?? source.source_website_url, source.title ?? source.caption, source.snippet);
      }
    }
  }

  return { provider: "codex", answer: answerParts.join("\n").trim().slice(0, MAX_ANSWER_CHARS), sources };
}

/** Run hosted web_search with Pi's Codex credentials. Adapted from pi-web-access/openai-search.ts at 6c5afa1 (MIT). */
export async function searchCodex(
  request: SearchRequest,
  ctx: ExtensionContext,
  modelId?: string,
  signal?: AbortSignal,
): Promise<SearchResult> {
  if (!request.query.trim()) throw new WebError("invalid_input", "Search query must not be empty.");
  if (!Number.isInteger(request.limit) || request.limit < 1 || request.limit > 20) {
    throw new WebError("invalid_input", "Search result limit must be an integer from 1 to 20.");
  }
  if (signal?.aborted) throw new WebError("cancelled", "Request cancelled.");

  const model = selectModel(ctx, modelId);
  let auth: Awaited<ReturnType<ExtensionContext["modelRegistry"]["getApiKeyAndHeaders"]>>;
  try {
    auth = await ctx.modelRegistry.getApiKeyAndHeaders(model);
  } catch {
    throw new WebError("authentication", `Codex authentication failed for model "${model.id}".`);
  }
  if (signal?.aborted) throw new WebError("cancelled", "Request cancelled.");
  if (!auth.ok || !auth.apiKey) {
    throw new WebError("authentication", "Codex authentication is unavailable. Sign in to OpenAI Codex using Pi's /login.");
  }
  if (auth.baseUrl !== undefined && !isOfficialCodexModel({ ...model, baseUrl: auth.baseUrl })) {
    throw new WebError("unsupported", "Codex search refuses credentials resolved for a nonofficial endpoint.");
  }

  const headers = new Headers();
  for (const [name, value] of Object.entries(auth.headers ?? {})) {
    if (value === null) headers.delete(name);
    else headers.set(name, value);
  }
  headers.set("Authorization", `Bearer ${auth.apiKey}`);
  headers.set("Content-Type", "application/json");
  headers.set("Accept", "text/event-stream");
  headers.set("OpenAI-Beta", "responses=experimental");
  headers.set("originator", "pi");
  const accountId = decodeJwtAccountId(auth.apiKey);
  if (accountId) headers.set("chatgpt-account-id", accountId);

  const response = await requestText(CODEX_RESPONSES_URL, {
    method: "POST",
    headers,
    body: JSON.stringify(buildBody(request, model.id)),
  }, signal, SEARCH_TIMEOUT_MS);
  const parsed = parseResponse(response.text);
  if (!parsed.webSearchCallSeen) {
    throw new WebError("invalid_response", "Codex search returned no web_search_call.");
  }
  const result = extractResult(parsed.output, request.limit);
  if (!result.answer && result.sources.length === 0) {
    throw new WebError("invalid_response", "Codex search returned no answer or sources.");
  }
  return result;
}
