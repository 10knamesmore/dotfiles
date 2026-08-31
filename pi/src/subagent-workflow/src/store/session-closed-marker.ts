import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { isRecord } from "../util.js";
import { replaceAtomicFile } from "./atomic-file.js";

const SESSION_CLOSED_MARKER_VERSION = 1 as const;

interface SessionClosedMarker {
  v: typeof SESSION_CLOSED_MARKER_VERSION;
  childId: string;
}

/** Hash the persisted id so even malformed legacy ids cannot escape the run directory. */
export function sessionClosedMarkerPath(runDir: string, childId: string): string {
  const digest = createHash("sha256").update(childId).digest("hex");
  return join(runDir, `session-closed-${digest}.json`);
}

/** Publish closure only after the child session process has actually exited. */
export function writeSessionClosedMarker(runDir: string, childId: string): void {
  const marker: SessionClosedMarker = { v: SESSION_CLOSED_MARKER_VERSION, childId };
  replaceAtomicFile(sessionClosedMarkerPath(runDir, childId), `${JSON.stringify(marker)}\n`, {
    mode: 0o600,
    exactMode: true,
    fsync: true,
    syncParentDirectory: true,
  });
}

export function hasSessionClosedMarker(runDir: string, childId: string): boolean {
  try {
    const value: unknown = JSON.parse(readFileSync(sessionClosedMarkerPath(runDir, childId), "utf8"));
    return isRecord(value)
      && value.v === SESSION_CLOSED_MARKER_VERSION
      && value.childId === childId;
  } catch {
    return false;
  }
}
