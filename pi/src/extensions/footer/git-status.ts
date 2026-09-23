import {
  execFile,
  type ChildProcess,
  type ExecFileException,
} from "node:child_process";
import { access, readFile } from "node:fs/promises";
import { watch, type FSWatcher } from "node:fs";
import { isAbsolute, relative, resolve, sep } from "node:path";
import { sanitizeFooterText } from "./format.js";

const REFRESH_DEBOUNCE_MS = 500;
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

interface GitPaths {
  /** Repository worktree root. */
  repositoryRoot: string;

  /** Worktree-specific git directory. */
  gitDirectory: string;

  /** Shared git directory that owns refs for linked worktrees. */
  commonGitDirectory: string;
}

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

function pathIsInside(parent: string, child: string): boolean {
  const path = relative(parent, child);
  return (
    path === "" ||
    (path !== ".." && !path.startsWith(`..${sep}`) && !isAbsolute(path))
  );
}

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

async function readOperationState(
  gitDirectory: string,
): Promise<OperationState> {
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
        label: (await exists(resolve(rebaseApply, "rebasing")))
          ? "REBASING"
          : "AM",
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
      status.upstream = sanitizeFooterText(
        record.slice("# branch.upstream ".length),
      );
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
function parseNumstat(
  output: string,
  unmergedPaths: ReadonlySet<string>,
): GitDiffStat {
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
 * Maintains an event-driven git snapshot for one cwd.
 *
 * Render never starts a process. Filesystem events use a trailing 500 ms
 * debounce, one refresh may run at a time, and events arriving in flight become
 * one pending refresh. Dispose closes watchers, timers, and child processes.
 */
export class GitStatusCache {
  private cwd = "";
  private snapshotValue: GitStatusSnapshot = UNAVAILABLE;
  private generation = 0;
  private refreshTimer: ReturnType<typeof setTimeout> | undefined;
  private refreshInFlight = false;
  private refreshPending = false;
  private disposed = false;
  private watchedPaths: GitPaths | undefined;
  private readonly watchers = new Set<FSWatcher>();
  private readonly processes = new Set<ChildProcess>();

  public constructor(
    cwd: string,
    private readonly onChange: () => void,
  ) {
    this.setCwd(cwd);
  }

  /** Return the latest completed snapshot without I/O. */
  public snapshot(): GitStatusSnapshot {
    return this.snapshotValue;
  }

  /** Coalesce a Pi lifecycle or tool event into the same 500 ms refresh path as filesystem changes. */
  public refreshForEvent(): void {
    this.scheduleRefresh();
  }

  /** Rebind the cache to a replacement Pi session cwd. */
  public setCwd(cwd: string): void {
    const nextCwd = resolve(cwd);
    if (this.disposed || nextCwd === this.cwd) return;
    this.cwd = nextCwd;
    this.generation += 1;
    this.refreshPending = false;
    if (this.refreshTimer !== undefined) {
      clearTimeout(this.refreshTimer);
      this.refreshTimer = undefined;
    }
    this.clearWatchers();
    this.watchedPaths = undefined;
    this.installSnapshot(UNAVAILABLE);
    if (this.refreshInFlight) this.refreshPending = true;
    else void this.refresh();
  }

  /** Close every resource owned by the cache. Safe to call more than once. */
  public dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.generation += 1;
    if (this.refreshTimer !== undefined) clearTimeout(this.refreshTimer);
    this.refreshTimer = undefined;
    this.clearWatchers();
    for (const process of this.processes) process.kill();
    this.processes.clear();
  }

  private installSnapshot(next: GitStatusSnapshot): void {
    if (JSON.stringify(next) === JSON.stringify(this.snapshotValue)) return;
    this.snapshotValue = next;
    this.onChange();
  }

  private scheduleRefresh(): void {
    if (this.disposed) return;
    if (this.refreshInFlight) {
      this.refreshPending = true;
      return;
    }
    if (this.refreshTimer !== undefined) clearTimeout(this.refreshTimer);
    this.refreshTimer = setTimeout(() => {
      this.refreshTimer = undefined;
      void this.refresh();
    }, REFRESH_DEBOUNCE_MS);
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
      const paths = await this.discoverPaths(cwd);
      if (this.disposed || generation !== this.generation) return;
      if (!paths) {
        this.installSnapshot(UNAVAILABLE);
        return;
      }
      this.installWatchers(paths);
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
          this.runGit(
            [
              "-C",
              cwd,
              "--no-optional-locks",
              "diff",
              "--numstat",
              "-z",
              "--cached",
            ],
            cwd,
          ),
          this.runGit(
            ["-C", cwd, "--no-optional-locks", "diff", "--numstat", "-z"],
            cwd,
          ),
        ]);
        if (this.disposed || generation !== this.generation) return;
        if (stagedOutput === undefined || unstagedOutput === undefined) {
          this.installSnapshot(UNAVAILABLE);
          return;
        }
        staged = parseNumstat(stagedOutput, status.unmergedPaths);
        unstaged = parseNumstat(unstagedOutput, status.unmergedPaths);
      }
      const operation = await readOperationState(paths.gitDirectory);
      if (this.disposed || generation !== this.generation) return;
      this.installSnapshot(buildSnapshot(status, staged, unstaged, operation));
    } finally {
      this.refreshInFlight = false;
      if (this.refreshPending && !this.disposed) {
        this.refreshPending = false;
        this.scheduleRefresh();
      }
    }
  }

  private async discoverPaths(cwd: string): Promise<GitPaths | undefined> {
    const output = await this.runGit(
      [
        "-C",
        cwd,
        "rev-parse",
        "--path-format=absolute",
        "--show-toplevel",
        "--absolute-git-dir",
        "--git-common-dir",
      ],
      cwd,
    );
    if (output === undefined) return undefined;
    const [repositoryRoot, gitDirectory, commonGitDirectory] = output
      .split("\n")
      .filter(Boolean);
    if (!repositoryRoot || !gitDirectory || !commonGitDirectory)
      return undefined;
    return {
      repositoryRoot: isAbsolute(repositoryRoot)
        ? repositoryRoot
        : resolve(cwd, repositoryRoot),
      gitDirectory: isAbsolute(gitDirectory)
        ? gitDirectory
        : resolve(cwd, gitDirectory),
      commonGitDirectory: isAbsolute(commonGitDirectory)
        ? commonGitDirectory
        : resolve(cwd, commonGitDirectory),
    };
  }

  private installWatchers(paths: GitPaths): void {
    if (
      this.watchedPaths?.repositoryRoot === paths.repositoryRoot &&
      this.watchedPaths.gitDirectory === paths.gitDirectory &&
      this.watchedPaths.commonGitDirectory === paths.commonGitDirectory
    ) {
      return;
    }
    this.clearWatchers();
    this.watchedPaths = paths;
    this.watchPath(paths.repositoryRoot);
    if (!pathIsInside(paths.repositoryRoot, paths.gitDirectory))
      this.watchPath(paths.gitDirectory);
    if (
      paths.commonGitDirectory !== paths.gitDirectory &&
      !pathIsInside(paths.repositoryRoot, paths.commonGitDirectory)
    ) {
      this.watchPath(paths.commonGitDirectory);
    }
  }

  private watchPath(
    path: string,
    recursive: boolean = true,
    generation: number = this.generation,
  ): void {
    if (!this.isCurrentWatchBinding(path, generation)) return;
    try {
      const watcher = watch(path, { recursive }, () => {
        if (this.isCurrentWatchBinding(path, generation))
          this.scheduleRefresh();
      });
      this.watchers.add(watcher);
      watcher.on("error", () => {
        this.watchers.delete(watcher);
        watcher.close();
        if (!this.isCurrentWatchBinding(path, generation)) return;
        if (recursive) this.watchPath(path, false, generation);
        else this.scheduleRefresh();
      });
    } catch {
      if (recursive && this.isCurrentWatchBinding(path, generation))
        this.watchPath(path, false, generation);
    }
  }

  private isCurrentWatchBinding(path: string, generation: number): boolean {
    if (this.disposed || generation !== this.generation || !this.watchedPaths)
      return false;
    return (
      path === this.watchedPaths.repositoryRoot ||
      path === this.watchedPaths.gitDirectory ||
      path === this.watchedPaths.commonGitDirectory
    );
  }

  private clearWatchers(): void {
    for (const watcher of this.watchers) watcher.close();
    this.watchers.clear();
  }

  private runGit(
    args: readonly string[],
    cwd: string,
  ): Promise<string | undefined> {
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
