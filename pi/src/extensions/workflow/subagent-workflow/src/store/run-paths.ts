import { readdirSync } from "node:fs";
import { join } from "node:path";
import { getAgentDir } from "@earendil-works/pi-coding-agent";

const RUN_ID = /^(run|workflow)-[0-9a-z]+-[0-9a-z]+$/;

/** A parent session owns its run directories for their entire lifetime. */
export function sessionRunsDirectory(
  sessionId: string,
  root: string = join(getAgentDir(), "subagent-workflow", "runs"),
): string {
  return join(root, sessionId);
}

export function runDirectory(sessionId: string, runId: string, root?: string): string {
  if (!RUN_ID.test(runId)) throw new Error(`Invalid run id: "${runId}"`);
  return join(sessionRunsDirectory(sessionId, root), runId);
}

/** Discover this session's saved runs when its runtime starts. */
export function sessionRunIds(sessionId: string, root?: string): string[] {
  try {
    return readdirSync(sessionRunsDirectory(sessionId, root), { withFileTypes: true })
      .filter((entry) => entry.isDirectory() && RUN_ID.test(entry.name))
      .map((entry) => entry.name);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return [];
    throw error;
  }
}
