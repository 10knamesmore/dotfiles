import { Buffer } from "node:buffer";

/**
 * Inline patches are copied through JSON, the workflow worker, and the journal.
 * Keep the cap small relative to the worker's 128 MB old-generation heap.
 */
export const MAX_INLINE_WORKTREE_PATCH_BYTES = 2 * 1024 * 1024;

function inlineWorktreePatchBytes(patch: string): number {
  return Buffer.byteLength(patch, "utf8");
}

export function isInlineWorktreePatch(value: unknown): value is string {
  return typeof value === "string" && inlineWorktreePatchBytes(value) <= MAX_INLINE_WORKTREE_PATCH_BYTES;
}

export function assertInlineWorktreePatch(value: unknown): asserts value is string {
  if (typeof value !== "string") throw new TypeError("Worktree patch must be a string");
  const bytes = inlineWorktreePatchBytes(value);
  if (bytes > MAX_INLINE_WORKTREE_PATCH_BYTES) {
    throw new Error(inlineWorktreePatchLimitMessage(bytes));
  }
}

export function inlineWorktreePatchLimitMessage(actualBytes?: number): string {
  const actual = actualBytes === undefined ? "" : ` (${actualBytes} bytes)`;
  return `Worktree patch${actual} exceeds the ${MAX_INLINE_WORKTREE_PATCH_BYTES}-byte inline safety limit and cannot be delivered inline`;
}
