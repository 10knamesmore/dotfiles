import { chmodSync } from "node:fs";
import { performance } from "node:perf_hooks";
import { DatabaseSync } from "node:sqlite";

const MAX_SQLITE_BUSY_WAIT_MS = 250;
const LOCK_RETRY_STATE = new Int32Array(new SharedArrayBuffer(4));

export interface SqliteLockHandle {
  release(): void;
}

export class SqliteLockBusyError extends Error {
  constructor() {
    super("SQLite lock is held by another owner");
    this.name = "SqliteLockBusyError";
  }
}

/** Acquire SQLite's single writer slot and hold it until the handle is released. */
export function acquireSqliteLock(path: string, waitForBusyMs = 0): SqliteLockHandle {
  waitForBusyMs = Math.min(MAX_SQLITE_BUSY_WAIT_MS, Math.max(0, waitForBusyMs));
  const database = new DatabaseSync(path);
  const deadline = performance.now() + waitForBusyMs;
  try {
    chmodSync(path, 0o600);
    database.exec("PRAGMA busy_timeout = 0");
    acquireWriterLock(database, deadline);
  } catch (error) {
    database.close();
    throw error;
  }

  let released = false;
  return {
    release(): void {
      if (released) return;
      released = true;
      try {
        database.exec("ROLLBACK");
      } finally {
        database.close();
      }
    },
  };
}

/** Observe whether another connection currently holds SQLite's writer slot. */
export function probeSqliteLock(path: string): "held" | "free" {
  try {
    const lock = acquireSqliteLock(path);
    lock.release();
    return "free";
  } catch (error) {
    if (error instanceof SqliteLockBusyError) return "held";
    throw error;
  }
}

function isSqliteBusyError(error: unknown): boolean {
  if (!error || typeof error !== "object") return false;
  const value = error as { code?: unknown; errcode?: unknown };
  return value.code === "ERR_SQLITE_ERROR" && typeof value.errcode === "number" && (value.errcode & 0xff) === 5;
}

function acquireWriterLock(database: DatabaseSync, deadline: number): void {
  for (;;) {
    try {
      database.exec("BEGIN IMMEDIATE");
      return;
    } catch (error) {
      if (!isSqliteBusyError(error)) throw error;
      const remaining = deadline - performance.now();
      if (remaining <= 0) throw new SqliteLockBusyError();
      Atomics.wait(LOCK_RETRY_STATE, 0, 0, Math.min(5, remaining));
    }
  }
}
