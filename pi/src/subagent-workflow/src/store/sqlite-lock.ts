import { chmodSync } from "node:fs";
import { createRequire } from "node:module";
import { performance } from "node:perf_hooks";

export const MAX_SQLITE_BUSY_WAIT_MS = 250;

export interface SqliteLockHandle {
  release(): void;
}

interface LockDatabase {
  exec(sql: string): unknown;
  close(): void;
}

interface LockDatabaseConstructor {
  new(path: string, options?: Record<string, unknown>): LockDatabase;
}

const require = createRequire(import.meta.url);
const LOCK_RETRY_STATE = new Int32Array(new SharedArrayBuffer(4));
let sqliteWarningFilterInstalled = false;

export class SqliteLockBusyError extends Error {
  constructor() {
    super("SQLite lock is held by another owner");
    this.name = "SqliteLockBusyError";
  }
}

/** Acquire SQLite's single writer slot and hold it until the handle is released. */
export function acquireSqliteLock(path: string, waitForBusyMs = 0): SqliteLockHandle {
  waitForBusyMs = Math.min(MAX_SQLITE_BUSY_WAIT_MS, Math.max(0, waitForBusyMs));
  const database = openLockDatabase(path);
  const deadline = performance.now() + waitForBusyMs;
  try {
    try {
      chmodSync(path, 0o600);
    } catch {
      // The lock still works when the platform cannot change its mode.
    }
    database.exec("PRAGMA busy_timeout = 0");
    acquireWriterLock(database, deadline);
  } catch (error) {
    database.close();
    throw error;
  }

  let released = false;
  const release = (): void => {
    if (released) return;
    released = true;
    try {
      database.exec("ROLLBACK");
    } finally {
      database.close();
    }
  };
  return { release };
}

/** Serialize one synchronous read-modify-write operation. */
export function withSqliteMutex<T>(
  path: string,
  action: () => T,
  waitForBusyMs = 0,
): T {
  const lock = acquireSqliteLock(path, waitForBusyMs);
  try {
    return action();
  } finally {
    lock.release();
  }
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

/** Runtime-neutral classifier for SQLite's busy error shapes. */
export function isSqliteBusyError(error: unknown): boolean {
  if (!error || typeof error !== "object") return false;
  const value = error as { code?: unknown; errcode?: unknown; errno?: unknown };
  return value.code === "SQLITE_BUSY"
    || value.errno === 5
    || (typeof value.errno === "number" && (value.errno & 0xff) === 5)
    || (value.code === "ERR_SQLITE_ERROR" && (
      value.errcode === 5
      || (typeof value.errcode === "number" && (value.errcode & 0xff) === 5)
    ));
}

function acquireWriterLock(database: LockDatabase, deadline: number): void {
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

function openLockDatabase(path: string): LockDatabase {
  if (process.versions.bun) {
    const { Database } = require("bun:sqlite") as { Database: LockDatabaseConstructor };
    return new Database(path, { create: true });
  }
  installSqliteWarningFilter();
  const { DatabaseSync } = require("node:sqlite") as { DatabaseSync: LockDatabaseConstructor };
  return new DatabaseSync(path);
}

function installSqliteWarningFilter(): void {
  if (sqliteWarningFilterInstalled) return;
  sqliteWarningFilterInstalled = true;
  const emitWarning = process.emitWarning;
  process.emitWarning = function filteredSqliteWarning(warning: string | Error, ...args: unknown[]): void {
    const message = warning instanceof Error ? warning.message : String(warning);
    const option = args[0];
    const name = warning instanceof Error
      ? warning.name
      : typeof option === "string"
        ? option
        : option && typeof option === "object" && "type" in option
          ? (option as { type?: unknown }).type
          : undefined;
    if (name === "ExperimentalWarning" && message === "SQLite is an experimental feature and might change at any time") return;
    Reflect.apply(emitWarning, process, [warning, ...args]);
  } as typeof process.emitWarning;
}
