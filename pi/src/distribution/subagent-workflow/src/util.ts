/** Small helpers shared across modules; keep this dependency-free. */

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

/** Display label for a spec: explicit label, else a trimmed slice of the prompt. */
export function childLabel(spec: { label?: string; prompt: string }): string {
  return spec.label?.trim() || spec.prompt.trim().replace(/\s+/g, " ").slice(0, 60) || "Subagent";
}

/** Bind a cancellation callback without missing a signal that was already aborted. */
export function bindAbort(signal: AbortSignal | undefined, onAbort: () => void): () => void {
  if (!signal) return () => {};
  if (signal.aborted) {
    onAbort();
    return () => {};
  }
  signal.addEventListener("abort", onAbort, { once: true });
  return () => signal.removeEventListener("abort", onAbort);
}
