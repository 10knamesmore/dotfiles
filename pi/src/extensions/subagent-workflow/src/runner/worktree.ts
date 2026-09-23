import { execFile } from "node:child_process";
import { mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { promisify } from "node:util";
import { reportDiagnostic } from "../diagnostics.js";
import { errorMessage } from "../util.js";
import {
  MAX_INLINE_WORKTREE_PATCH_BYTES,
  assertInlineWorktreePatch,
  inlineWorktreePatchLimitMessage,
} from "./inline-patch.js";

const exec = promisify(execFile);
const DIFF_MAX_BUFFER = MAX_INLINE_WORKTREE_PATCH_BYTES + 1;
const GIT_TIMEOUT_MS = 120_000;

interface WorktreeChanges {
  patch: string;
  changed: string[];
}

export interface Worktree {
  /** Root of the temporary checkout. */
  path: string;
  /** Working directory corresponding to the caller's original repo-relative cwd. */
  cwd: string;
  /** Commit the worktree was branched from, so we can diff against it regardless of what the child staged or committed. */
  baseCommit: string;
}

/** Collection failed but the worktree still holds the child's work - the caller must NOT delete it. */
export class WorktreeCollectionError extends Error {
  constructor(message: string, readonly worktreePath: string) {
    super(message);
    this.name = "WorktreeCollectionError";
  }
}

export async function createWorktree(cwd: string, path: string): Promise<Worktree> {
  const worktreePath = resolve(path);
  await mkdir(dirname(worktreePath), { recursive: true });
  let prefix: string;
  try {
    const result = await exec("git", ["-C", cwd, "rev-parse", "--show-prefix"], { signal: AbortSignal.timeout(GIT_TIMEOUT_MS) });
    prefix = result.stdout.replace(/\r?\n$/, "");
    await exec("git", ["-C", cwd, "worktree", "add", worktreePath, "--detach", "HEAD"], { signal: AbortSignal.timeout(GIT_TIMEOUT_MS) });
  } catch (error) {
    throw new Error(`Failed to create isolated git worktree: ${commandError(error)}`);
  }
  try {
    const { stdout } = await exec("git", ["-C", worktreePath, "rev-parse", "HEAD"], { signal: AbortSignal.timeout(GIT_TIMEOUT_MS) });
    const childCwd = resolve(worktreePath, prefix);
    // A valid cwd may be an empty or ignored directory that Git did not copy.
    // Recreate it so the isolated child starts at the same repo-relative path.
    await mkdir(childCwd, { recursive: true });
    return { path: worktreePath, cwd: childCwd, baseCommit: stdout.trim() };
  } catch (error) {
    // The worktree was added but we could not record its base; remove it so we
    // do not leak a registered worktree that nothing tracks.
    await cleanupWorktree(cwd, worktreePath).catch((cleanupError) => {
      reportDiagnostic(`[subagent-workflow] ${errorMessage(cleanupError)}`);
    });
    throw new Error(`Failed to record isolated git worktree base: ${commandError(error)}`);
  }
}

export async function collectWorktree(worktree: Worktree): Promise<WorktreeChanges> {
  const { path, baseCommit } = worktree;
  try {
    // Diff against the base commit, not the index, so the patch captures work
    // the child left unstaged, staged, OR committed (a child told to "commit
    // your work" advances HEAD; diffing index-vs-worktree would miss it, and
    // cleanup would then discard it silently). Intent-to-add surfaces the
    // contents of untracked files in the unified diff.
    await exec("git", ["-C", path, "add", "-A", "-N"], { signal: AbortSignal.timeout(GIT_TIMEOUT_MS) });
    const [{ stdout: patch }, { stdout: names }] = await Promise.all([
      exec("git", ["-C", path, "diff", "--no-ext-diff", "--binary", baseCommit], { maxBuffer: DIFF_MAX_BUFFER, signal: AbortSignal.timeout(GIT_TIMEOUT_MS) }),
      exec("git", ["-C", path, "diff", "--name-only", "-z", baseCommit], { maxBuffer: DIFF_MAX_BUFFER, signal: AbortSignal.timeout(GIT_TIMEOUT_MS) }),
    ]);
    assertInlineWorktreePatch(patch);
    return { patch, changed: names.split("\0").filter(Boolean) };
  } catch (error) {
    // Signal that the work is still on disk so the caller retains the worktree.
    const reason = isMaxBufferError(error) ? inlineWorktreePatchLimitMessage() : commandError(error);
    throw new WorktreeCollectionError(`Failed to collect isolated worktree changes: ${reason}`, path);
  }
}

export async function cleanupWorktree(cwd: string, path: string): Promise<void> {
  try {
    await exec("git", ["-C", cwd, "worktree", "remove", "--force", resolve(path)], { signal: AbortSignal.timeout(GIT_TIMEOUT_MS) });
    await exec("git", ["-C", cwd, "worktree", "prune"], { signal: AbortSignal.timeout(GIT_TIMEOUT_MS) });
  } catch (error) {
    throw new Error(`Failed to clean up isolated git worktree: ${commandError(error)} - worktree retained at ${resolve(path)}`);
  }
}

function isMaxBufferError(error: unknown): boolean {
  if (!(error instanceof Error)) return false;
  const details = error as Error & { code?: string };
  return details.code === "ERR_CHILD_PROCESS_STDIO_MAXBUFFER" || details.message.includes("maxBuffer");
}

function commandError(error: unknown): string {
  if (!(error instanceof Error)) return String(error);
  const details = error as Error & { stderr?: string };
  return details.stderr?.trim() || details.message;
}
