import { asWebError, WebError } from "../errors.js";
import { githubReader } from "./github.js";
import { httpReader } from "./http.js";
import type { WebDocument, WebReader } from "./reader.js";

const READ_TIMEOUT_MS = 30_000;
const siteReaders: WebReader[] = [githubReader];

/** HTTP handles unmatched URLs, never failures from a selected site reader. */
export function selectReader(url: URL): WebReader {
  return siteReaders.find((reader) => reader.supports(url)) ?? httpReader;
}

export async function readWebDocument(rawUrl: string, signal?: AbortSignal): Promise<WebDocument> {
  let url: URL;
  try {
    url = new URL(rawUrl);
  } catch {
    throw new WebError("invalid_input", "URL must be a valid absolute HTTP(S) URL.");
  }
  if (url.protocol !== "http:" && url.protocol !== "https:") {
    throw new WebError("invalid_input", "Only HTTP and HTTPS URLs are supported.");
  }
  if (url.username || url.password) {
    throw new WebError("invalid_input", "URLs containing credentials are not supported.");
  }

  const startedAt = Date.now();
  const deadline = AbortSignal.timeout(READ_TIMEOUT_MS);
  const readSignal = signal ? AbortSignal.any([signal, deadline]) : deadline;
  try {
    readSignal.throwIfAborted();
    const document = await selectReader(url).read(url, readSignal);
    readSignal.throwIfAborted();
    if (Date.now() - startedAt >= READ_TIMEOUT_MS) {
      throw new WebError("timeout", "Reading the URL timed out.");
    }
    return document;
  } catch (error) {
    if (signal?.aborted) throw asWebError(error, signal);
    if (deadline.aborted) throw new WebError("timeout", "Reading the URL timed out.");
    throw asWebError(error, signal);
  }
}
