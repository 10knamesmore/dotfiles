// Adapted from pi-web-access extract.ts at 6c5afa1d0d43eef8552284ad73f4bd9f0612a378 (MIT).
import { Readability } from "@mozilla/readability";
import { parseHTML } from "linkedom";
import type TurndownService from "turndown";
import { asWebError, WebError } from "../errors.js";
import { readResponseText } from "../http.js";
import { validatePublicUrl } from "./ssrf.js";

const REQUEST_TIMEOUT_MS = 30_000;
const MAX_RESPONSE_BYTES = 5 * 1024 * 1024;
const MAX_REDIRECTS = 5;
const REDIRECT_STATUSES = new Set([301, 302, 303, 307, 308]);
const HTML_TYPES = new Set(["text/html", "application/xhtml+xml"]);
const PASSTHROUGH_TYPES = new Set([
  "text/plain",
  "text/markdown",
  "text/x-markdown",
  "application/markdown",
  "application/x-markdown",
  "application/json",
  "application/xml",
  "text/xml",
]);

export interface FetchedPage {
  url: string;
  title: string;
  contentType: string;
  content: string;
}

interface FetchResult {
  response: Response;
  finalUrl: URL;
}

/** Fetch and extract a public text webpage without browser rendering or hosted fallbacks. */
export async function fetchPage(url: string, signal?: AbortSignal): Promise<FetchedPage> {
  const startedAt = Date.now();
  const deadline = AbortSignal.timeout(REQUEST_TIMEOUT_MS);
  const requestSignal = signal ? AbortSignal.any([signal, deadline]) : deadline;

  try {
    requestSignal.throwIfAborted();
    const { response, finalUrl } = await fetchFollowingRedirects(url, requestSignal);
    checkDeadline(requestSignal, startedAt);

    if (!response.ok) {
      discardBody(response);
      throw new WebError("http", `Remote server returned HTTP ${response.status}.`, response.status);
    }

    const contentType = normalizedContentType(response);
    if (!isSupportedContentType(contentType)) {
      discardBody(response);
      const label = contentType || "missing Content-Type";
      throw new WebError("unsupported", `Unsupported response content type: ${label}.`);
    }

    const content = await readResponseText(response, MAX_RESPONSE_BYTES);
    checkDeadline(requestSignal, startedAt);

    if (!HTML_TYPES.has(contentType)) {
      return {
        url: finalUrl.toString(),
        title: extractTextTitle(content, finalUrl),
        contentType,
        content,
      };
    }

    const extracted = await extractHtml(content, finalUrl);
    checkDeadline(requestSignal, startedAt);
    return {
      url: finalUrl.toString(),
      title: extracted.title,
      contentType,
      content: extracted.content,
    };
  } catch (error) {
    if (signal?.aborted) throw asWebError(error, signal);
    if (deadline.aborted || Date.now() - startedAt >= REQUEST_TIMEOUT_MS) {
      throw new WebError("timeout", "Webpage request timed out.");
    }
    throw asWebError(error, signal);
  }
}

async function fetchFollowingRedirects(url: string, signal: AbortSignal): Promise<FetchResult> {
  let current = await validatePublicUrl(url, signal);

  for (let redirects = 0; redirects <= MAX_REDIRECTS; redirects += 1) {
    signal.throwIfAborted();
    const response = await fetch(current, {
      redirect: "manual",
      signal,
      headers: {
        accept:
          "text/html,application/xhtml+xml,text/plain,text/markdown,application/json,application/xml,text/xml;q=0.9,*/*;q=0.1",
        "user-agent": "dotfiles-pi-webfetch/1.0",
      },
    });

    if (!REDIRECT_STATUSES.has(response.status)) return { response, finalUrl: current };

    const location = response.headers.get("location");
    if (!location) return { response, finalUrl: current };
    discardBody(response);
    if (redirects === MAX_REDIRECTS) {
      throw new WebError("invalid_response", "Remote server returned too many redirects.");
    }

    let next: URL;
    try {
      next = new URL(location, current);
    } catch {
      throw new WebError("invalid_response", "Remote server returned an invalid redirect URL.");
    }
    current = await validatePublicUrl(next, signal);
  }

  throw new WebError("invalid_response", "Remote server returned too many redirects.");
}

async function extractHtml(html: string, url: URL): Promise<{ title: string; content: string }> {
  try {
    const { document } = parseHTML(html);
    const documentTitle = document.title?.trim() ?? "";
    const baseUrl = resolveBaseUrl(document.querySelector("base[href]")?.getAttribute("href"), url);

    for (const element of document.querySelectorAll("a[href], img[src], source[src]")) {
      const attribute = element.hasAttribute("href") ? "href" : "src";
      const value = element.getAttribute(attribute);
      if (!value) continue;
      try {
        element.setAttribute(attribute, new URL(value, baseUrl).toString());
      } catch {
        // Leave malformed author-provided links as text instead of failing the page.
      }
    }
    for (const element of document.querySelectorAll("script, style, nav, noscript, template")) {
      element.remove();
    }

    const fallbackHtml = document.body?.innerHTML ?? "";
    type ReadabilityDocument = ConstructorParameters<typeof Readability>[0];
    const article = new Readability(document as unknown as ReadabilityDocument).parse();
    const turndown = await getTurndown();
    const readableMarkdown = typeof article?.content === "string" ? turndown.turndown(article.content).trim() : "";
    const fallbackMarkdown = readableMarkdown ? "" : turndown.turndown(fallbackHtml).trim();
    const content = readableMarkdown || fallbackMarkdown;

    if (!content) {
      throw new WebError("invalid_response", "No readable text was found in the HTML response.");
    }
    return {
      title: article?.title?.trim() || documentTitle || fallbackTitle(url),
      content,
    };
  } catch (error) {
    if (error instanceof WebError) throw error;
    throw new WebError("invalid_response", "The HTML response could not be extracted.");
  }
}

function resolveBaseUrl(baseHref: string | null | undefined, pageUrl: URL): URL {
  if (!baseHref) return pageUrl;
  try {
    const baseUrl = new URL(baseHref, pageUrl);
    return baseUrl.protocol === "http:" || baseUrl.protocol === "https:" ? baseUrl : pageUrl;
  } catch {
    return pageUrl;
  }
}

function normalizedContentType(response: Response): string {
  return response.headers.get("content-type")?.split(";", 1)[0]?.trim().toLowerCase() ?? "";
}

function isSupportedContentType(contentType: string): boolean {
  return (
    HTML_TYPES.has(contentType) ||
    PASSTHROUGH_TYPES.has(contentType) ||
    contentType.endsWith("+json") ||
    contentType.endsWith("+xml")
  );
}

function extractTextTitle(text: string, url: URL): string {
  if (isMarkdownType(text)) {
    const heading = text
      .match(/^#{1,2}\s+(.+)$/m)?.[1]
      ?.replace(/[*_`]+/g, "")
      .trim();
    if (heading) return heading;
  }
  return fallbackTitle(url);
}

function isMarkdownType(text: string): boolean {
  return /^#{1,2}\s+\S/m.test(text);
}

function fallbackTitle(url: URL): string {
  const segment = url.pathname.split("/").filter(Boolean).at(-1);
  if (!segment) return url.hostname;
  try {
    return decodeURIComponent(segment);
  } catch {
    return segment;
  }
}

function checkDeadline(signal: AbortSignal, startedAt: number): void {
  signal.throwIfAborted();
  if (Date.now() - startedAt >= REQUEST_TIMEOUT_MS) {
    throw new WebError("timeout", "Webpage request timed out.");
  }
}

function discardBody(response: Response): void {
  void response.body?.cancel().catch(() => {
    // The response is already being abandoned; cancellation failure is irrelevant.
  });
}

let turndownInstance: Promise<TurndownService> | undefined;

function getTurndown(): Promise<TurndownService> {
  turndownInstance ??= loadTurndown();
  return turndownInstance;
}

async function loadTurndown(): Promise<TurndownService> {
  const { default: Turndown } = await import("turndown");
  const service = new Turndown({
    headingStyle: "atx",
    codeBlockStyle: "fenced",
    bulletListMarker: "-",
  });
  service.remove(["script", "style", "nav", "noscript", "template"]);
  return service;
}
