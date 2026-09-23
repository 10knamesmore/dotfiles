/**
 * Run-id path resolution guard for workflow resume.
 *
 * A resume references a run id by name. The id must match the fixed shape and
 * must name a direct child of this cwd's runs directory; anything else is
 * rejected so a hostile or corrupt id can never escape the runs root.
 */

import { dirname, join, resolve } from "node:path";
import { getAgentDir } from "@earendil-works/pi-coding-agent";
import { encodeCwd } from "../store/run-store.js";

const RUN_ID = /^(run|workflow)-[0-9a-z]+-[0-9a-z]+$/;

function workflowRunsRoot(): string {
  return join(getAgentDir(), "subagent-workflow", "runs");
}

/** Resolves a run id only when it names a direct child of this cwd's runs directory. */
export function resolveRunDir(cwd: string, runId: string, runsRoot: string = workflowRunsRoot()): string {
  if (!RUN_ID.test(runId)) throw new Error(`Invalid run id: "${runId}"`);
  const cwdRunsDir = resolve(runsRoot, encodeCwd(cwd));
  const runDir = resolve(cwdRunsDir, runId);
  if (dirname(runDir) !== cwdRunsDir) throw new Error(`Invalid run id path: "${runId}"`);
  return runDir;
}