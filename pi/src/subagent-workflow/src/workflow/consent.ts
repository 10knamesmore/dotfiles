/**
 * "Always approve" consent memory for workflow launches.
 *
 * Consent is recorded per (workflow name, project cwd, script hash) so that a saved
 * workflow a user has trusted in a project skips the launch dialog on later runs
 * only while its contents are unchanged. The
 * store is a small JSON file; only saved workflows are ever consulted (inline
 * scripts are a new script each time and never skip the dialog).
 *
 * Mutations are serialized across processes through one exclusive lock and
 * committed by atomic rename, so a concurrent pi process never loses a grant and
 * a reader never observes a truncated file.
 */

import { mkdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { getAgentDir } from "@earendil-works/pi-coding-agent";
import { replaceAtomicFile } from "../store/atomic-file.js";
import { SqliteLockBusyError, withSqliteMutex } from "../store/sqlite-lock.js";
import { reportDiagnostic } from "../diagnostics.js";
import { isRecord } from "../util.js";

interface ConsentRecord {
  workflow: string;
  cwd: string;
  scriptHash: string;
  grantedAt: string;
}

interface ConsentFile {
  version: 2;
  approvals: ConsentRecord[];
}

/** Brief grace so ordinary contention resolves instead of dropping the grant. */
const CONSENT_LOCK_WAIT_MS = 250;

/** Reads and writes the "always approve" consent file. Path is injectable for tests. */
export class ConsentStore {
  constructor(
    private readonly path: string = join(getAgentDir(), "subagent-workflow", "consent.json"),
    private readonly now: () => Date = () => new Date(),
  ) {}

  isApproved(workflow: string, cwd: string, scriptHash: string): boolean {
    const target = resolve(cwd);
    return this.read().approvals.some((record) =>
      record.workflow === workflow && resolve(record.cwd) === target && record.scriptHash === scriptHash);
  }

  /**
   * Add a grant. Re-reads the latest file under the lock and appends only when
   * absent, so a concurrent record from another process is never clobbered.
   * Best-effort: an approval the user just gave must not fail the launch, so a
   * lock we cannot take in time is reported and skipped rather than thrown.
   */
  record(workflow: string, cwd: string, scriptHash: string): void {
    const target = resolve(cwd);
    try {
      this.mutate((file) => {
        if (file.approvals.some((record) =>
          record.workflow === workflow && resolve(record.cwd) === target && record.scriptHash === scriptHash)) {
          return undefined;
        }
        file.approvals.push({ workflow, cwd: target, scriptHash, grantedAt: this.now().toISOString() });
        return file;
      });
    } catch (error) {
      if (error instanceof SqliteLockBusyError) {
        reportDiagnostic(`[subagent-workflow] could not record workflow consent at ${this.path}: ${error.message}`);
        return;
      }
      throw error;
    }
  }

  /**
   * Forget every remembered grant. Unlike record this is authoritative: it does
   * not merge with disk state, so a grant racing a clear is ordered by the lock
   * (last writer wins) and the caller is told when the lock cannot be taken.
   */
  clear(): void {
    this.mutate(() => ({ version: 2, approvals: [] }));
  }

  private mutate(transform: (file: ConsentFile) => ConsentFile | undefined): void {
    const directory = dirname(this.path);
    mkdirSync(directory, { recursive: true, mode: 0o700 });
    withSqliteMutex(`${this.path}.lock.sqlite`, () => {
      const next = transform(this.read());
      if (next) {
        replaceAtomicFile(this.path, `${JSON.stringify(next, null, 2)}\n`, {
          mode: 0o600,
          preserveExistingMode: true,
          fsync: true,
          exactMode: true,
          syncParentDirectory: true,
        });
      }
    }, CONSENT_LOCK_WAIT_MS);
  }

  private read(): ConsentFile {
    try {
      const parsed = JSON.parse(readFileSync(this.path, "utf8")) as Partial<ConsentFile>;
      const approvals = Array.isArray(parsed.approvals) ? parsed.approvals.filter(isConsentRecord) : [];
      return { version: 2, approvals };
    } catch {
      return { version: 2, approvals: [] };
    }
  }
}

function isConsentRecord(value: unknown): value is ConsentRecord {
  if (!isRecord(value)) return false;
  const record = value;
  return typeof record.workflow === "string" && typeof record.cwd === "string"
    && typeof record.scriptHash === "string" && typeof record.grantedAt === "string";
}
