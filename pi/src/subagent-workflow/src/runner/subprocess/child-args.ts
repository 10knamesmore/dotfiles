/**
 * Maps a resolved child configuration to the `pi --mode rpc` argv. Runtime
 * configuration crosses the subprocess boundary as CLI flags, while the
 * report_result schema tool travels via the child-shim extension (`-e`).
 */

import type { ThinkingLevel } from "../../types.js";

export interface ChildProcessConfig {
  provider: string;
  modelId: string;
  thinkingLevel: ThinkingLevel;
  /** Requested tool narrowing. undefined leaves the child's full toolset; an
   * explicit empty array is an empty allowlist (zero tools), never "no limit". */
  tools?: readonly string[];
  excludeTools?: readonly string[];
  /** Directory the child writes its real pi session file into. */
  sessionDir: string;
  /** Absolute session file whose history is copied into a new child session. */
  forkSessionFile?: string;
  /** The subagent framing line appended to the child's system prompt. */
  appendSystemPrompt: string;
  /** Absolute path of the child-shim extension, when the spec needs one. */
  shimPath?: string;
}

/** Tools children must never expose: the recursion guard, now per-process. */
const RECURSION_GUARD_TOOLS = ["subagent", "workflow"];

export function buildChildArgs(config: ChildProcessConfig): string[] {
  for (const name of [...(config.tools ?? []), ...(config.excludeTools ?? [])]) {
    // pi's --tools/--exclude-tools split on commas, so a comma inside one
    // name would silently request different tools than the author wrote.
    if (name.includes(",")) throw new Error(`Tool name ${JSON.stringify(name)} contains a comma and cannot cross the pi CLI boundary`);
  }
  const excludeTools = [...new Set([...(config.excludeTools ?? []), ...RECURSION_GUARD_TOOLS])];
  const args = [
    "--mode", "rpc",
    "--provider", config.provider,
    "--model", config.modelId,
    "--thinking", config.thinkingLevel,
    "--session-dir", config.sessionDir,
    "--append-system-prompt", config.appendSystemPrompt,
    "--exclude-tools", excludeTools.join(","),
  ];
  // Emit --tools whenever an allowlist was given, even empty: pi parses
  // --tools "" as a zero-tool allowlist (matching in-process tools: []).
  // Omitting the flag would instead grant the full default toolset.
  if (config.tools !== undefined) args.push("--tools", config.tools.join(","));
  if (config.forkSessionFile) args.push("--fork", config.forkSessionFile);
  if (config.shimPath) args.push("--extension", config.shimPath);
  return args;
}
