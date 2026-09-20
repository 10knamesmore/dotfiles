/** Failure categories shared by tool results, human-readable rendering, and diagnostics. */
export type WebErrorCode =
  | "invalid_input"
  | "authentication"
  | "network"
  | "timeout"
  | "cancelled"
  | "http"
  | "invalid_response"
  | "unsupported"
  | "too_large"
  | "cache_miss"
  | "storage";

/** Controlled diagnostic text only: never pass upstream response bodies or credentials. */
export class WebError extends Error {
  constructor(
    public readonly code: WebErrorCode,
    message: string,
    public readonly status?: number,
  ) {
    super(message);
    this.name = "WebError";
  }
}

/** Classify transport failures without copying possibly sensitive fetch error messages. */
export function asWebError(error: unknown, signal?: AbortSignal): WebError {
  if (signal?.aborted) return new WebError("cancelled", "Request cancelled.");
  if (error instanceof WebError) return error;
  if (error instanceof Error && error.name === "TimeoutError") {
    return new WebError("timeout", "Request timed out.");
  }
  if (error instanceof Error && error.name === "AbortError") {
    return new WebError("cancelled", "Request cancelled.");
  }
  return new WebError("network", "The request could not be completed.");
}
