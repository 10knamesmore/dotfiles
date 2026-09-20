import { TextDecoder } from "node:util";
import { asWebError, WebError } from "./errors.js";

const MAX_RESPONSE_BYTES = 5 * 1024 * 1024;

/** Read a bounded response body and decode its declared charset; never silently truncate a download. */
export async function readResponseText(response: Response, maxBytes = MAX_RESPONSE_BYTES): Promise<string> {
  if (Number(response.headers.get("content-length")) > maxBytes) {
    await response.body?.cancel();
    throw new WebError("too_large", `Response exceeds ${maxBytes} bytes.`);
  }
  if (!response.body) return "";
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let bytes = 0;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      bytes += value.byteLength;
      if (bytes > maxBytes) {
        await reader.cancel();
        throw new WebError("too_large", `Response exceeds ${maxBytes} bytes.`);
      }
      chunks.push(value);
    }
  } finally {
    reader.releaseLock();
  }
  const charset = response.headers.get("content-type")?.match(/charset\s*=\s*["']?([^\s;"']+)/i)?.[1] ?? "utf-8";
  let decoder: TextDecoder;
  try {
    decoder = new TextDecoder(charset);
  } catch {
    throw new WebError("unsupported", "Response uses an unsupported text encoding.");
  }
  return decoder.decode(Buffer.concat(chunks, bytes));
}

/** Search API requests share a deadline through body consumption and never forward credentials across redirects. */
export async function requestText(
  url: string,
  init: RequestInit,
  signal: AbortSignal | undefined,
  timeoutMs: number,
): Promise<{ text: string; status: number; contentType: string; url: string }> {
  const deadline = AbortSignal.timeout(timeoutMs);
  const requestSignal = signal ? AbortSignal.any([signal, deadline]) : deadline;
  try {
    requestSignal.throwIfAborted();
    const response = await fetch(url, { ...init, redirect: "error", signal: requestSignal });
    if (!response.ok) {
      await response.body?.cancel();
      throw new WebError("http", `Search service returned HTTP ${response.status}.`, response.status);
    }
    const text = await readResponseText(response);
    requestSignal.throwIfAborted();
    return {
      text,
      status: response.status,
      contentType: response.headers.get("content-type") ?? "",
      url: response.url || url,
    };
  } catch (error) {
    if (!signal?.aborted && deadline.aborted) throw new WebError("timeout", "Search request timed out.");
    throw asWebError(error, signal);
  }
}
