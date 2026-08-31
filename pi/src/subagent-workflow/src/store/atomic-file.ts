import { randomUUID } from "node:crypto";
import { chmodSync, closeSync, fsyncSync, openSync, renameSync, rmSync, statSync, writeSync } from "node:fs";
import { dirname } from "node:path";

interface AtomicFileOperations {
  open(path: string, flags: string, mode: number): number;
  write(fd: number, buffer: Uint8Array, offset: number, length: number): number;
  fsync(fd: number): void;
  close(fd: number): void;
  rename(from: string, to: string): void;
  chmod(path: string, mode: number): void;
  remove(path: string): void;
  existingMode(path: string): number | undefined;
  syncDirectory(path: string): void;
}

export interface DirectorySyncOperations {
  open(path: string, flags: string): number;
  fsync(fd: number): void;
  close(fd: number): void;
}

const DIRECTORY_SYNC_OPERATIONS: DirectorySyncOperations = {
  open: (path, flags) => openSync(path, flags),
  fsync: (fd) => fsyncSync(fd),
  close: (fd) => closeSync(fd),
};

/** Make directory-entry changes durable, tolerating only unsupported platforms. */
export function syncDirectoryDurably(
  path: string,
  operations: DirectorySyncOperations = DIRECTORY_SYNC_OPERATIONS,
): void {
  let descriptor: number | undefined;
  let failure: unknown;
  try {
    descriptor = operations.open(path, "r");
    operations.fsync(descriptor);
  } catch (error) {
    if (!isUnsupportedDirectorySync(error)) failure = error;
  }
  if (descriptor !== undefined) {
    try {
      operations.close(descriptor);
    } catch (error) {
      failure ??= error;
    }
  }
  if (failure !== undefined) throw failure;
}

const FILE_OPERATIONS: AtomicFileOperations = {
  open: (path, flags, mode) => openSync(path, flags, mode),
  write: (fd, buffer, offset, length) => writeSync(fd, buffer, offset, length),
  fsync: (fd) => fsyncSync(fd),
  close: (fd) => closeSync(fd),
  rename: (from, to) => renameSync(from, to),
  chmod: (path, mode) => chmodSync(path, mode),
  remove: (path) => rmSync(path, { force: true }),
  existingMode: (path) => {
    try {
      return statSync(path).mode & 0o777;
    } catch {
      return undefined;
    }
  },
  syncDirectory: syncDirectoryDurably,
};

interface StageAtomicFileOptions {
  /** Initial mode passed to open(2); the process umask still applies. */
  mode: number;
  fsync?: boolean;
  /** Apply this exact mode while the staging descriptor is still open. */
  chmod?: number;
  operations?: Partial<AtomicFileOperations>;
}

interface CommitAtomicFileOptions {
  operations?: Partial<AtomicFileOperations>;
}

interface ReplaceAtomicFileOptions {
  /** Initial mode passed to open(2); the process umask still applies. */
  mode: number;
  fsync?: boolean;
  operations?: Partial<AtomicFileOperations>;
  /** Use the canonical file's current permission bits when it exists. */
  preserveExistingMode?: boolean;
  /** Apply the selected exact mode to the staging file before rename. */
  exactMode?: boolean;
  /** Fsync the parent directory after rename. */
  syncParentDirectory?: boolean;
}

/** Write a complete same-directory O_EXCL staging file and return its path. */
export function stageAtomicFile(path: string, content: string, options: StageAtomicFileOptions): string {
  const operations = fileOperations(options.operations);
  const temporary = `${path}.tmp-${process.pid}-${randomUUID()}`;
  let descriptor: number | undefined;
  let failure: { error: unknown } | undefined;

  try {
    descriptor = operations.open(temporary, "wx", options.mode);
    const buffer = Buffer.from(content, "utf8");
    let offset = 0;
    while (offset < buffer.length) {
      const remaining = buffer.length - offset;
      const written = operations.write(descriptor, buffer, offset, remaining);
      if (!Number.isSafeInteger(written) || written <= 0 || written > remaining) {
        throw new Error(`Unable to complete staging write for ${path}`);
      }
      offset += written;
    }
    // Final metadata must reach the still-open staging descriptor before its fsync.
    if (options.chmod !== undefined) operations.chmod(temporary, options.chmod);
    if (options.fsync) operations.fsync(descriptor);
  } catch (error) {
    failure = { error };
  }

  if (descriptor !== undefined) {
    try {
      operations.close(descriptor);
    } catch (error) {
      failure ??= { error };
    }
  }

  if (failure) {
    if (descriptor !== undefined) discardAtomicFile(temporary, operations);
    throw failure.error;
  }
  return temporary;
}

/** Publish a staging file, removing it best-effort if rename fails. */
export function commitAtomicFile(temporary: string, path: string, options: CommitAtomicFileOptions = {}): void {
  const operations = fileOperations(options.operations);
  try {
    operations.rename(temporary, path);
  } catch (error) {
    discardAtomicFile(temporary, operations);
    throw error;
  }
}

/** Best-effort removal for an uncommitted staging file. */
export function discardAtomicFile(
  temporary: string,
  operations: Partial<AtomicFileOperations> | AtomicFileOperations = FILE_OPERATIONS,
): void {
  const resolved = fileOperations(operations);
  try {
    resolved.remove(temporary);
  } catch {
    // Cleanup must not hide the write or commit failure.
  }
}

/** Stage and atomically replace one file without creating its parent directory. */
export function replaceAtomicFile(path: string, content: string, options: ReplaceAtomicFileOptions): void {
  const operations = fileOperations(options.operations);
  const mode = options.preserveExistingMode
    ? operations.existingMode(path) ?? options.mode
    : options.mode;
  const temporary = stageAtomicFile(path, content, {
    mode,
    fsync: options.fsync,
    chmod: options.exactMode ? mode : undefined,
    operations,
  });
  commitAtomicFile(temporary, path, { operations });
  if (options.syncParentDirectory) operations.syncDirectory(dirname(path));
}

function fileOperations(overrides: Partial<AtomicFileOperations> | AtomicFileOperations | undefined): AtomicFileOperations {
  return overrides ? { ...FILE_OPERATIONS, ...overrides } : FILE_OPERATIONS;
}

function isUnsupportedDirectorySync(error: unknown): boolean {
  const code = (error as NodeJS.ErrnoException | undefined)?.code;
  if (code === "EINVAL" || code === "ENOTSUP" || code === "EOPNOTSUPP" || code === "ENOSYS") return true;
  return process.platform === "win32" && code === "EISDIR";
}
