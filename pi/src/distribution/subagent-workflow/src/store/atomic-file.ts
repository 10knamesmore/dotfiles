import { randomUUID } from "node:crypto";
import { chmodSync, closeSync, fsyncSync, openSync, renameSync, rmSync, writeSync } from "node:fs";
import { dirname } from "node:path";

/** Make directory-entry changes durable. */
export function syncDirectoryDurably(path: string): void {
  const descriptor = openSync(path, "r");
  let failure: unknown;
  try {
    fsyncSync(descriptor);
  } catch (error) {
    failure = error;
  }
  try {
    closeSync(descriptor);
  } catch (error) {
    failure ??= error;
  }
  if (failure !== undefined) throw failure;
}

interface StageAtomicFileOptions {
  /** Initial mode passed to open(2); the process umask still applies. */
  mode: number;
  fsync?: boolean;
  /** Apply this exact mode while the staging descriptor is still open. */
  chmod?: number;
}

interface ReplaceAtomicFileOptions {
  /** Initial mode passed to open(2); the process umask still applies. */
  mode: number;
  fsync?: boolean;
  /** Apply the selected exact mode to the staging file before rename. */
  exactMode?: boolean;
  /** Fsync the parent directory after rename. */
  syncParentDirectory?: boolean;
}

/** Write a complete same-directory O_EXCL staging file and return its path. */
export function stageAtomicFile(path: string, content: string, options: StageAtomicFileOptions): string {
  const temporary = `${path}.tmp-${process.pid}-${randomUUID()}`;
  let descriptor: number | undefined;
  let failure: { error: unknown } | undefined;

  try {
    descriptor = openSync(temporary, "wx", options.mode);
    const buffer = Buffer.from(content, "utf8");
    let offset = 0;
    while (offset < buffer.length) {
      const remaining = buffer.length - offset;
      const written = writeSync(descriptor, buffer, offset, remaining);
      if (!Number.isSafeInteger(written) || written <= 0 || written > remaining) {
        throw new Error(`Unable to complete staging write for ${path}`);
      }
      offset += written;
    }
    // Final metadata must reach the still-open staging descriptor before its fsync.
    if (options.chmod !== undefined) chmodSync(temporary, options.chmod);
    if (options.fsync) fsyncSync(descriptor);
  } catch (error) {
    failure = { error };
  }

  if (descriptor !== undefined) {
    try {
      closeSync(descriptor);
    } catch (error) {
      failure ??= { error };
    }
  }

  if (failure) {
    if (descriptor !== undefined) discardAtomicFile(temporary);
    throw failure.error;
  }
  return temporary;
}

/** Publish a staging file, removing it best-effort if rename fails. */
export function commitAtomicFile(temporary: string, path: string): void {
  try {
    renameSync(temporary, path);
  } catch (error) {
    discardAtomicFile(temporary);
    throw error;
  }
}

/** Best-effort removal for an uncommitted staging file. */
export function discardAtomicFile(temporary: string): void {
  try {
    rmSync(temporary, { force: true });
  } catch {
    // Cleanup must not hide the write or commit failure.
  }
}

/** Stage and atomically replace one file without creating its parent directory. */
export function replaceAtomicFile(path: string, content: string, options: ReplaceAtomicFileOptions): void {
  const temporary = stageAtomicFile(path, content, {
    mode: options.mode,
    fsync: options.fsync,
    chmod: options.exactMode ? options.mode : undefined,
  });
  commitAtomicFile(temporary, path);
  if (options.syncParentDirectory) syncDirectoryDurably(dirname(path));
}
