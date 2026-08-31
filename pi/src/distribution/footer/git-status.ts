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

/** Counts represented by the porcelain-v2 status snapshot. */
export interface GitFileStatus {
  /** Paths whose index state differs from HEAD. */
  staged: number;

  /** Tracked paths modified in the worktree. */
  modified: number;

  /** Tracked paths deleted in the worktree. */
  deleted: number;

  /** Rename records reported by porcelain v2. */
  renamed: number;

  /** Untracked paths. */
  untracked: number;

  /** Unmerged paths. */
  conflicted: number;

  /** Stash entries reported by `--show-stash`. */
  stashed: number;

  /** Commits ahead of the configured upstream. */
  ahead: number;

  /** Commits behind the configured upstream. */
  behind: number;
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

      /** Current branch name, or `detached` when HEAD is detached outside a named operation. */
      branch: string;

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

const EMPTY_FILES: GitFileStatus = {
  staged: 0,
  modified: 0,
  deleted: 0,
  renamed: 0,
  untracked: 0,
  conflicted: 0,
  stashed: 0,
  ahead: 0,
  behind: 0,
};

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

function parseStatus(
  output: string,
  operationState: OperationState,
): GitStatusSnapshot {
  const files: GitFileStatus = { ...EMPTY_FILES };
  let branch = "detached";
  for (const line of output.split("\n")) {
    if (line.startsWith("# branch.head ")) {
      const head = sanitizeFooterText(line.slice("# branch.head ".length));
      if (head && head !== "(detached)") branch = head;
      continue;
    }
    if (line.startsWith("# branch.ab ")) {
      const [ahead, behind] = line.slice("# branch.ab ".length).split(" ");
      files.ahead = parseNonNegativeInteger(ahead?.replace(/^\+/, ""));
      files.behind = parseNonNegativeInteger(behind?.replace(/^-/, ""));
      continue;
    }
    if (line.startsWith("# stash ")) {
      files.stashed = parseNonNegativeInteger(line.slice("# stash ".length));
      continue;
    }
    if (line.startsWith("1 ") || line.startsWith("2 ")) {
      const indexState = line[2] ?? ".";
      const worktreeState = line[3] ?? ".";
      if (indexState !== ".") files.staged += 1;
      if (worktreeState === "M" || worktreeState === "T") files.modified += 1;
      if (worktreeState === "D") files.deleted += 1;
      if (line.startsWith("2 ")) files.renamed += 1;
      continue;
    }
    if (line.startsWith("u ")) files.conflicted += 1;
    else if (line.startsWith("? ")) files.untracked += 1;
  }
  if (branch === "detached" && operationState.originalBranch)
    branch = sanitizeFooterText(operationState.originalBranch);
  return {
    kind: "repository",
    branch,
    operation: operationState.operation,
    files,
  };
}

function snapshotSignature(snapshot: GitStatusSnapshot): string {
  if (snapshot.kind === "unavailable") return snapshot.kind;
  return [
    snapshot.branch,
    snapshot.operation?.label ?? "",
    snapshot.operation?.step ?? "",
    snapshot.operation?.total ?? "",
    ...Object.values(snapshot.files),
  ].join("\0");
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
    if (snapshotSignature(next) === snapshotSignature(this.snapshotValue))
      return;
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
      const status = await this.runGit(
        [
          "-C",
          cwd,
          "--no-optional-locks",
          "status",
          "--porcelain=v2",
          "--branch",
          "--show-stash",
        ],
        cwd,
      );
      if (this.disposed || generation !== this.generation) return;
      if (status === undefined) {
        this.installSnapshot(UNAVAILABLE);
        return;
      }
      const operation = await readOperationState(paths.gitDirectory);
      if (this.disposed || generation !== this.generation) return;
      this.installSnapshot(parseStatus(status, operation));
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
