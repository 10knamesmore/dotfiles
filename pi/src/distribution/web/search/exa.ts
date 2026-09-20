import { WebError } from "../errors.js";
import { requestText } from "../http.js";
import type { SearchRequest, SearchResult } from "./types.js";

const EXA_MCP_URL = "https://mcp.exa.ai/mcp";
const BASIC_SEARCH_TOOL = "web_search_exa";
const ADVANCED_SEARCH_TOOL = "web_search_advanced_exa";
const REQUEST_TIMEOUT_MS = 60_000;
const MAX_TITLE_LENGTH = 500;
const MAX_SNIPPET_LENGTH = 2_000;

interface ExaItem {
	title?: unknown;
	url?: unknown;
	text?: unknown;
	highlights?: unknown;
}

interface RpcEnvelope {
	result?: Record<string, unknown>;
	error?: Record<string, unknown>;
}

type ParsedToolContent = { valid: true; items: ExaItem[] } | { valid: false };

/** Search Exa's hosted MCP. Adapted from pi-web-access/exa.ts at 6c5afa1d (MIT); see ../README.md. */
export async function searchExa(request: SearchRequest, signal?: AbortSignal): Promise<SearchResult> {
	const filtered = request.recency !== undefined || (request.domains?.length ?? 0) > 0;
	const toolName = filtered ? ADVANCED_SEARCH_TOOL : BASIC_SEARCH_TOOL;
	const args = filtered ? advancedArguments(request) : basicArguments(request);
	const response = await requestText(
		`${EXA_MCP_URL}?tools=${toolName}`,
		{
			method: "POST",
			headers: {
				accept: "application/json, text/event-stream",
				"content-type": "application/json",
				"x-exa-source": "pi-web-access",
			},
			body: JSON.stringify({
				jsonrpc: "2.0",
				id: 1,
				method: "tools/call",
				params: { name: toolName, arguments: args },
			}),
		},
		signal,
		REQUEST_TIMEOUT_MS,
	);

	const toolText = extractToolText(parseEnvelope(response.text));
	const parsed = parseToolContent(toolText);
	if (!parsed.valid) {
		throw invalidResponse("Exa MCP returned malformed search results");
	}

	const sources = normalizeSources(parsed.items, request.limit);
	if (parsed.items.length > 0 && sources.length === 0) throw invalidResponse("Exa returned no usable source URLs");
	return {
		provider: "exa",
		answer: "",
		sources,
	};
}

function basicArguments(request: SearchRequest): Record<string, unknown> {
	return { query: request.query, numResults: request.limit };
}

function advancedArguments(request: SearchRequest): Record<string, unknown> {
	const domains = request.domains ?? [];
	const includeDomains = domains.filter(domain => !domain.startsWith("-"));
	const excludeDomains = domains.filter(domain => domain.startsWith("-")).map(domain => domain.slice(1));
	return {
		...basicArguments(request),
		type: "auto",
		...(includeDomains.length > 0 ? { includeDomains } : {}),
		...(excludeDomains.length > 0 ? { excludeDomains } : {}),
		...(request.recency ? { startPublishedDate: recencyStart(request.recency) } : {}),
		enableHighlights: true,
		textMaxCharacters: 3_000,
	};
}

function recencyStart(recency: NonNullable<SearchRequest["recency"]>): string {
	const days = { day: 1, week: 7, month: 30, year: 365 }[recency];
	return new Date(Date.now() - days * 86_400_000).toISOString();
}

function parseEnvelope(body: string): RpcEnvelope {
	const candidates: unknown[] = [];
	for (const event of body.split(/\r?\n\r?\n/)) {
		const data = event
			.split(/\r?\n/)
			.filter((line) => line.startsWith("data:"))
			.map((line) => line.slice(5).trimStart())
			.join("\n")
			.trim();
		if (data && data !== "[DONE]") candidates.push(parseJson(data));
	}
	if (candidates.length === 0) candidates.push(parseJson(body));

	for (const candidate of candidates) {
		if (!isRecord(candidate)) continue;
		if (isRecord(candidate.result) || isRecord(candidate.error)) return candidate as RpcEnvelope;
	}
	throw invalidResponse("Exa MCP returned a malformed JSON-RPC envelope");
}

function extractToolText(envelope: RpcEnvelope): string {
	if (envelope.error) {
		const code = typeof envelope.error.code === "number" ? ` ${envelope.error.code}` : "";
		throw invalidResponse(`Exa MCP rejected the search request (RPC error${code}).`);
	}

	const content = envelope.result?.content;
	const textItem = Array.isArray(content)
		? content.find((item) =>
			isRecord(item)
			&& item.type === "text"
			&& typeof item.text === "string"
			&& item.text.trim().length > 0)
		: undefined;
	const text = isRecord(textItem) ? textItem.text : undefined;
	if (envelope.result?.isError === true) {
		throw invalidResponse("Exa MCP tool could not complete the search.");
	}
	if (typeof text !== "string") throw invalidResponse("Exa MCP returned no text content");
	return text;
}

function parseToolContent(text: string): ParsedToolContent {
	const json = parseJson(text);
	if (isRecord(json) && Array.isArray(json.results)) {
		if (json.results.some(item => !isRecord(item))) return { valid: false };
		return { valid: true, items: json.results as ExaItem[] };
	}

	const items: ExaItem[] = [];
	for (const block of text.split(/(?=^Title:\s*)/m)) {
		const titleMatch = block.match(/^Title:\s*(.*)$/m);
		const urlMatch = block.match(/^URL:\s*(.*)$/m);
		const textStart = /^(?:Text|Highlights):[ \t]*/m.exec(block);
		if (!titleMatch || !urlMatch) continue;
		const excerpt = textStart ? block.slice(textStart.index + textStart[0].length).replace(/\n---\s*$/, "").trim() : "";
		items.push({
			title: titleMatch[1]?.trim() ?? "",
			url: urlMatch[1]?.trim() ?? "",
			text: excerpt,
		});
	}
	return items.length > 0 ? { valid: true, items } : { valid: false };
}

function normalizeSources(items: ExaItem[], limit: number): SearchResult["sources"] {
	const sources: SearchResult["sources"] = [];
	const seen = new Set<string>();
	for (const item of items) {
		if (sources.length >= limit) break;
		if (typeof item.url !== "string") continue;
		const url = normalizeHttpUrl(item.url);
		if (!url || seen.has(url)) continue;
		seen.add(url);
		const title = boundedText(item.title, MAX_TITLE_LENGTH) || `Source ${sources.length + 1}`;
		sources.push({ title, url, snippet: itemExcerpt(item) });
	}
	return sources;
}

function itemExcerpt(item: ExaItem): string {
	const highlights = Array.isArray(item.highlights)
		? item.highlights.filter((value): value is string => typeof value === "string" && value.trim() !== "")
		: [];
	return boundedText(highlights.length > 0 ? highlights.join("\n") : item.text, MAX_SNIPPET_LENGTH);
}

function boundedText(value: unknown, maximum: number): string {
	return typeof value === "string" ? value.trim().slice(0, maximum) : "";
}

function normalizeHttpUrl(value: string): string | null {
	try {
		const url = new URL(value.trim());
		return url.protocol === "http:" || url.protocol === "https:" ? url.toString() : null;
	} catch {
		return null;
	}
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

function invalidResponse(message: string): WebError {
	return new WebError("invalid_response", message);
}
