import { execFile, type ChildProcess, type ExecFileException } from "node:child_process";
import { access, readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { sanitizeFooterText } from "./format.js";

const REFRESH_INTERVAL_MS = 10_000;
const GIT_TIMEOUT_MS = 5_000;
const MAX_GIT_OUTPUT_BYTES = 2 * 1024 * 1024;
const DETACHED_HEAD_HASH_LENGTH = 7;

/** Added/deleted line totals for one side of the index/worktree comparison. */
export interface GitDiffStat {
  /** Files reported by the diff, including binary files. */
  files: number;

  /** Added text lines; binary files contribute no lines. */
  added: number;

  /** Deleted text lines; binary files contribute no lines. */
  deleted: number;
}

/** Worktree/index counters derived from porcelain v2 and `--numstat` output. */
export interface GitFileStatus {
  /** Index state relative to HEAD. */
  staged: GitDiffStat;

  /** Worktree state relative to the index. */
  unstaged: GitDiffStat;

  /** Untracked files, with directories expanded to individual files. */
  untracked: number;

  /** Unmerged paths, reported separately from the staged/unstaged diffs. */
  conflicted: number;

  /** Stash entries reported by `--show-stash`. */
  stashed: number;
}

/** Git operation shown beside the branch while repository metadata carries one. */
export interface GitOperation {
  /** Human-facing operation name such as `REBASING` or `MERGING`. */
  label: string;

  /** Current step when the operation exposes progress files. */
  step: number | undefined;

  /** Total steps when the operation exposes progress files. */
  total: number | undefined;
}

/** Cached repository state consumed synchronously by footer rendering. */
export type GitStatusSnapshot =
  | {
      /** The cwd is outside a repository or git could not produce a snapshot. */
      kind: "unavailable";
    }
  | {
      /** Git resolved a repository for the current cwd. */
      kind: "repository";

      /**
       * Branch name, or `detached @ <short hash>` when HEAD is detached without a
       * recoverable operation branch.
       */
      branch: string;

      /** True when HEAD is detached, including an in-progress rebase. */
      detached: boolean;

      /** Configured upstream short name; absent when the branch has none. */
      upstream: string | undefined;

      /** Commits ahead of the upstream; unknown when its ref cannot be compared. */
      ahead: number | undefined;

      /** Commits behind the upstream; unknown when its ref cannot be compared. */
      behind: number | undefined;

      /** In-progress operation read from the worktree git directory. */
      operation: GitOperation | undefined;

      /** Compact worktree/index/upstream counters. */
      files: GitFileStatus;
    };

interface OperationState {
  /** Visible operation metadata. */
  operation: GitOperation | undefined;

  /** Original branch recovered from rebase metadata when HEAD is detached. */
  originalBranch: string | undefined;
}

/** Values parsed from one `git status --porcelain=v2 -z --branch` output. */
interface PorcelainStatus {
  /** Current branch name; undefined when HEAD is detached. */
  branch: string | undefined;

  /** HEAD commit hash; undefined on an unborn branch. */
  headOid: string | undefined;

  /** Configured upstream short name. */
  upstream: string | undefined;

  /** Commits ahead of the upstream, if Git supplied branch.ab. */
  ahead: number | undefined;

  /** Commits behind the upstream, if Git supplied branch.ab. */
  behind: number | undefined;

  /** Untracked file count with directories expanded. */
  untracked: number;

  /** Unmerged path count. */
  conflicted: number;

  /** Stash entry count. */
  stashed: number;

  /** Unmerged paths, used to keep conflict entries out of the diff totals. */
  unmergedPaths: Set<string>;

  /** True when the index or worktree holds at least one tracked change. */
  hasTrackedChanges: boolean;
}

const EMPTY_DIFF: GitDiffStat = { files: 0, added: 0, deleted: 0 };

const UNAVAILABLE: GitStatusSnapshot = { kind: "unavailable" };

async function exists(path: string): Promise<boolean> {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

async function readTrimmed(path: string): Promise<string | undefined> {
  try {
    return (await readFile(path, "utf8")).trim();
  } catch {
    return undefined;
  }
}

async function readPositiveInteger(path: string): Promise<number | undefined> {
  const raw = await readTrimmed(path);
  if (raw === undefined) return undefined;
  const parsed = Number.parseInt(raw, 10);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : undefined;
}

async function readOperationState(gitDirectory: string): Promise<OperationState> {
  const rebaseMerge = resolve(gitDirectory, "rebase-merge");
  if (await exists(rebaseMerge)) {
    const headName = await readTrimmed(resolve(rebaseMerge, "head-name"));
    return {
      operation: {
        label: "REBASING",
        step: await readPositiveInteger(resolve(rebaseMerge, "msgnum")),
        total: await readPositiveInteger(resolve(rebaseMerge, "end")),
      },
      originalBranch: headName?.replace(/^refs\/heads\//, ""),
    };
  }

  const rebaseApply = resolve(gitDirectory, "rebase-apply");
  if (await exists(rebaseApply)) {
    return {
      operation: {
        label: (await exists(resolve(rebaseApply, "rebasing"))) ? "REBASING" : "AM",
        step: await readPositiveInteger(resolve(rebaseApply, "next")),
        total: await readPositiveInteger(resolve(rebaseApply, "last")),
      },
      originalBranch: undefined,
    };
  }

  for (const [file, label] of [
    ["MERGE_HEAD", "MERGING"],
    ["CHERRY_PICK_HEAD", "CHERRY-PICKING"],
    ["REVERT_HEAD", "REVERTING"],
    ["BISECT_LOG", "BISECTING"],
  ] as const) {
    if (await exists(resolve(gitDirectory, file))) {
      return {
        operation: { label, step: undefined, total: undefined },
        originalBranch: undefined,
      };
    }
  }
  return { operation: undefined, originalBranch: undefined };
}

function parseNonNegativeInteger(raw: string | undefined): number {
  if (raw === undefined) return 0;
  const parsed = Number.parseInt(raw, 10);
  return Number.isInteger(parsed) && parsed >= 0 ? parsed : 0;
}

/** Path field of a porcelain-v2 unmerged record; nine fixed fields precede it. */
function unmergedPath(record: string): string | undefined {
  return record.split(" ").slice(10).join(" ");
}

function parsePorcelainStatus(output: string): PorcelainStatus {
  const status: PorcelainStatus = {
    branch: undefined,
    headOid: undefined,
    upstream: undefined,
    ahead: undefined,
    behind: undefined,
    untracked: 0,
    conflicted: 0,
    stashed: 0,
    unmergedPaths: new Set(),
    hasTrackedChanges: false,
  };

  const records = output.split("\0");
  for (let index = 0; index < records.length; index += 1) {
    const record = records[index] ?? "";
    if (record.startsWith("# branch.oid ")) {
      const oid = record.slice("# branch.oid ".length);
      if (oid !== "(initial)") status.headOid = oid;
    } else if (record.startsWith("# branch.head ")) {
      const head = record.slice("# branch.head ".length);
      if (head !== "(detached)") status.branch = sanitizeFooterText(head);
    } else if (record.startsWith("# branch.upstream ")) {
      status.upstream = sanitizeFooterText(record.slice("# branch.upstream ".length));
    } else if (record.startsWith("# branch.ab ")) {
      const [ahead, behind] = record.slice("# branch.ab ".length).split(" ");
      status.ahead = parseNonNegativeInteger(ahead?.replace(/^\+/, ""));
      status.behind = parseNonNegativeInteger(behind?.replace(/^-/, ""));
    } else if (record.startsWith("# stash ")) {
      status.stashed = parseNonNegativeInteger(record.slice("# stash ".length));
    } else if (record.startsWith("2 ")) {
      status.hasTrackedChanges = true;
      // With -z, a rename record is followed by its original path as its own
      // NUL record; consume it so a path that begins like an entry stays data.
      index += 1;
    } else if (record.startsWith("1 ")) {
      status.hasTrackedChanges = true;
    } else if (record.startsWith("u ")) {
      status.hasTrackedChanges = true;
      status.conflicted += 1;
      const path = unmergedPath(record);
      if (path !== undefined) status.unmergedPaths.add(path);
    } else if (record.startsWith("? ")) {
      status.untracked += 1;
    }
  }

  return status;
}

/**
 * Sum `git diff --numstat -z` records for one side, skipping unmerged paths
 * because their combined diffs do not represent a normal index comparison.
 */
function parseNumstat(output: string, unmergedPaths: ReadonlySet<string>): GitDiffStat {
  const stat: GitDiffStat = { files: 0, added: 0, deleted: 0 };

  const records = output.split("\0");
  for (let index = 0; index < records.length; index += 1) {
    const record = records[index] ?? "";
    if (record === "") continue;

    const firstTab = record.indexOf("\t");
    const secondTab = record.indexOf("\t", firstTab + 1);
    if (firstTab < 0 || secondTab < 0) continue;

    const path = record.slice(secondTab + 1);
    if (path === "") {
      // Rename/copy: with -z, the old and new paths are the next two records.
      const newPath = records[index + 2] ?? "";
      index += 2;
      if (unmergedPaths.has(newPath)) continue;
    } else if (unmergedPaths.has(path)) {
      continue;
    }

    stat.files += 1;
    const added = record.slice(0, firstTab);
    const deleted = record.slice(firstTab + 1, secondTab);
    if (added !== "-" && deleted !== "-") {
      stat.added += parseNonNegativeInteger(added);
      stat.deleted += parseNonNegativeInteger(deleted);
    }
  }

  return stat;
}

function buildSnapshot(
  status: PorcelainStatus,
  staged: GitDiffStat,
  unstaged: GitDiffStat,
  operationState: OperationState,
): GitStatusSnapshot {
  let branch = status.branch ?? operationState.originalBranch;
  if (branch === undefined && status.headOid !== undefined) {
    branch = `detached @ ${status.headOid.slice(0, DETACHED_HEAD_HASH_LENGTH)}`;
  }

  return {
    kind: "repository",
    branch: branch === undefined ? "@" : sanitizeFooterText(branch),
    detached: status.branch === undefined,
    upstream: status.upstream,
    ahead: status.ahead,
    behind: status.behind,
    operation: operationState.operation,
    files: {
      staged,
      unstaged,
      untracked: status.untracked,
      conflicted: status.conflicted,
      stashed: status.stashed,
    },
  };
}

/**
 * Maintains a git snapshot refreshed after each turn and every 10 seconds.
 *
 * Render never starts a process. One refresh may run at a time; requests arriving
 * in flight become one pending refresh. Dispose stops the timer and child processes.
 */
export class GitStatusCache {
  private cwd = "";
  private snapshotValue: GitStatusSnapshot = UNAVAILABLE;
  private generation = 0;
  private readonly refreshTimer: ReturnType<typeof setInterval>;
  private refreshInFlight = false;
  private refreshPending = false;
  private disposed = false;
  private readonly processes = new Set<ChildProcess>();

  public constructor(
    cwd: string,
    private readonly onChange: () => void,
  ) {
    this.setCwd(cwd);
    this.refreshTimer = setInterval(() => this.requestRefresh(), REFRESH_INTERVAL_MS);
  }

  /** Return the latest completed snapshot without I/O. */
  public snapshot(): GitStatusSnapshot {
    return this.snapshotValue;
  }

  /** Start a background refresh, merging requests while a query is running. */
  public requestRefresh(): void {
    void this.refresh();
  }

  /** Rebind the cache to a replacement Pi session cwd. */
  public setCwd(cwd: string): void {
    const nextCwd = resolve(cwd);
    if (this.disposed || nextCwd === this.cwd) return;
    this.cwd = nextCwd;
    this.generation += 1;
    this.refreshPending = false;
    this.installSnapshot(UNAVAILABLE);
    this.requestRefresh();
  }

  /** Close every resource owned by the cache. Safe to call more than once. */
  public dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.generation += 1;
    clearInterval(this.refreshTimer);
    for (const process of this.processes) process.kill();
    this.processes.clear();
  }

  private installSnapshot(next: GitStatusSnapshot): void {
    if (JSON.stringify(next) === JSON.stringify(this.snapshotValue)) return;
    this.snapshotValue = next;
    this.onChange();
  }

  private async refresh(): Promise<void> {
    if (this.disposed) return;
    if (this.refreshInFlight) {
      this.refreshPending = true;
      return;
    }
    this.refreshInFlight = true;
    const generation = this.generation;
    const cwd = this.cwd;
    try {
      const gitDirectory = await this.runGit(["-C", cwd, "rev-parse", "--absolute-git-dir"], cwd);
      if (this.disposed || generation !== this.generation) return;
      if (gitDirectory === undefined) {
        this.installSnapshot(UNAVAILABLE);
        return;
      }
      const statusOutput = await this.runGit(
        [
          "-C",
          cwd,
          "--no-optional-locks",
          "status",
          "--porcelain=v2",
          "-z",
          "--branch",
          "--show-stash",
          "--untracked-files=all",
        ],
        cwd,
      );
      if (this.disposed || generation !== this.generation) return;
      if (statusOutput === undefined) {
        this.installSnapshot(UNAVAILABLE);
        return;
      }
      const status = parsePorcelainStatus(statusOutput);
      let staged: GitDiffStat = EMPTY_DIFF;
      let unstaged: GitDiffStat = EMPTY_DIFF;
      if (status.hasTrackedChanges) {
        const [stagedOutput, unstagedOutput] = await Promise.all([
          this.runGit(["-C", cwd, "--no-optional-locks", "diff", "--numstat", "-z", "--cached"], cwd),
          this.runGit(["-C", cwd, "--no-optional-locks", "diff", "--numstat", "-z"], cwd),
        ]);
        if (this.disposed || generation !== this.generation) return;
        if (stagedOutput === undefined || unstagedOutput === undefined) {
          this.installSnapshot(UNAVAILABLE);
          return;
        }
        staged = parseNumstat(stagedOutput, status.unmergedPaths);
        unstaged = parseNumstat(unstagedOutput, status.unmergedPaths);
      }
      const operation = await readOperationState(gitDirectory);
      if (this.disposed || generation !== this.generation) return;
      this.installSnapshot(buildSnapshot(status, staged, unstaged, operation));
    } catch (error) {
      if (!this.disposed && generation === this.generation) {
        this.installSnapshot(UNAVAILABLE);
        console.error("[pi-footer] Git refresh failed:", error);
      }
    } finally {
      this.refreshInFlight = false;
      if (this.refreshPending && !this.disposed) {
        this.refreshPending = false;
        this.requestRefresh();
      }
    }
  }

  private runGit(args: readonly string[], cwd: string): Promise<string | undefined> {
    return new Promise((resolvePromise) => {
      if (this.disposed) {
        resolvePromise(undefined);
        return;
      }
      const child = execFile(
        "git",
        [...args],
        {
          cwd,
          encoding: "utf8",
          timeout: GIT_TIMEOUT_MS,
          maxBuffer: MAX_GIT_OUTPUT_BYTES,
        },
        (error: ExecFileException | null, stdout: string) => {
          resolvePromise(error ? undefined : stdout.trimEnd());
        },
      );
      this.processes.add(child);
      child.once("close", () => this.processes.delete(child));
    });
  }
}
