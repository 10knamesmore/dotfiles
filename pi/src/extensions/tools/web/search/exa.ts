// MCP envelope parsing adapted from pi-web-access (MIT); see ../LICENSE.pi-web-access.
import { WebError } from "../errors.js";
import { requestText } from "../http.js";
import { boundedText, cleanUrl, displayDate, isRecord, parseJson } from "./response.js";
import type { SearchRequest, SearchSource } from "./types.js";

const SEARCH_TOOL = "web_search_advanced_exa";
const SEARCH_URL = `https://mcp.exa.ai/mcp?tools=${SEARCH_TOOL}`;

/** Exa's keyless hosted MCP returns structured results through its advanced search tool. */
export async function searchExa(request: SearchRequest, signal?: AbortSignal): Promise<SearchSource[]> {
  const response = await requestText(
    SEARCH_URL,
    {
      method: "POST",
      headers: {
        accept: "application/json, text/event-stream",
        "content-type": "application/json",
      },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "tools/call",
        params: { name: SEARCH_TOOL, arguments: searchArguments(request) },
      }),
    },
    signal,
    60_000,
  );
  const result = parseEnvelope(response.text);
  if (result.isError === true) {
    throw new WebError("invalid_response", "Exa MCP could not complete the search.");
  }
  const sources: SearchSource[] = [];
  const seen = new Set<string>();
  const content = Array.isArray(result.content)
    ? result.content.find((item) => isRecord(item) && item.type === "text")
    : undefined;
  if (!isRecord(content) || typeof content.text !== "string") throw invalidResponse();
  const payload = parseJson(content.text);
  if (!isRecord(payload) || !Array.isArray(payload.results)) throw invalidResponse();
  for (const item of payload.results) {
    if (sources.length >= request.limit) break;
    if (!isRecord(item)) continue;
    const url = cleanUrl(item.url);
    if (url === undefined || seen.has(url)) continue;
    seen.add(url);
    const highlights = Array.isArray(item.highlights) ? item.highlights.join("\n") : "";
    sources.push({
      providers: ["exa"],
      title: boundedText(item.title, 500) || url,
      url,
      summary: boundedText(highlights, 2_000) || boundedText(item.text, 2_000),
      date: displayDate(item.publishedDate),
    });
  }
  return sources;
}

/** Exa requires explicit domain/date fields instead of query operators. Quoted phrases stay in the query. */
function searchArguments(request: SearchRequest): Record<string, unknown> {
  const includeDomains: string[] = [];
  const excludeDomains: string[] = [];
  const terms: string[] = [];
  const args: Record<string, unknown> = {
    numResults: request.limit,
    type: "auto",
    enableHighlights: true,
    highlightsMaxCharacters: 2_000,
    textMaxCharacters: 2_000,
  };
  for (const token of request.query.match(/"[^"]*"|\S+/g) ?? []) {
    const domain = /^(-?)site:(.+)$/.exec(token);
    const date = /^(after|before):(\d{4}-\d{2}-\d{2})$/.exec(token);
    if (domain) {
      (domain[1] ? excludeDomains : includeDomains).push(domain[2]!);
    } else if (date) {
      args[date[1] === "after" ? "startPublishedDate" : "endPublishedDate"] = date[2];
    } else {
      terms.push(token);
    }
  }
  args.query = terms.join(" ");
  if (includeDomains.length) args.includeDomains = includeDomains;
  if (excludeDomains.length) args.excludeDomains = excludeDomains;
  return args;
}

/** MCP HTTP responses can carry the JSON-RPC result directly or in an SSE message. */
function parseEnvelope(body: string): Record<string, unknown> {
  const candidates: unknown[] = [];
  for (const event of body.split(/\r?\n\r?\n/)) {
    const data = event
      .split(/\r?\n/)
      .filter((line) => line.startsWith("data:"))
      .map((line) => line.slice(5).trimStart())
      .join("\n")
      .trim();
    if (data) candidates.push(parseJson(data));
  }
  if (candidates.length === 0) candidates.push(parseJson(body));
  for (const candidate of candidates) {
    if (!isRecord(candidate)) continue;
    if (isRecord(candidate.error)) {
      throw new WebError("invalid_response", "Exa MCP rejected the search request.");
    }
    if (isRecord(candidate.result)) return candidate.result;
  }
  throw invalidResponse();
}

function invalidResponse(): WebError {
  return new WebError("invalid_response", "Exa MCP returned malformed search results.");
}
