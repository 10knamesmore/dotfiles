/**
 * The file contract between spawnSubprocessChild and the child-shim
 * extension it loads into every child pi process. The parent writes a spec
 * file and points the child at it through an environment variable; the shim
 * answers with a tool report the parent needs before the first prompt.
 */

import { readFileSync, renameSync, writeFileSync } from "node:fs";
import { readFile } from "node:fs/promises";

/** Environment variable naming the shim spec file inside a child process. */
export const SHIM_SPEC_ENV = "PI_SUBAGENT_SHIM_SPEC";

export interface ShimSpec {
  /** JSON Schema for a structured-output child; the shim registers report_result from it. */
  schema?: Record<string, unknown>;
  /** Where the shim writes its ShimToolReport once the child session starts. */
  toolReportPath: string;
}

export interface ShimToolReport {
  /** The child's active tool names, the authority for the missing-tool and
   * recursion-guard contracts. The environment fingerprint deliberately does
   * NOT use this (the workflow journal falls back to the prospective
   * loadChildExtensionEnvironment scan), so no extension classification is
   * reported here. */
  activeTools: string[];
}

export function readShimSpec(path: string): ShimSpec {
  const parsed = JSON.parse(readFileSync(path, "utf8")) as ShimSpec;
  if (typeof parsed.toolReportPath !== "string" || parsed.toolReportPath.length === 0) {
    throw new Error("Shim spec is missing toolReportPath");
  }
  return parsed;
}

/** Publish atomically so a polling parent never observes a torn report. */
export function writeToolReport(path: string, report: ShimToolReport): void {
  const temporary = `${path}.tmp`;
  writeFileSync(temporary, JSON.stringify(report), { mode: 0o600 });
  renameSync(temporary, path);
}

/** A parsed report, or undefined while the shim has not published one yet. */
export async function readToolReport(path: string): Promise<ShimToolReport | undefined> {
  let raw: string;
  try {
    raw = await readFile(path, "utf8");
  } catch {
    return undefined;
  }
  try {
    const parsed = JSON.parse(raw) as ShimToolReport;
    if (!Array.isArray(parsed.activeTools)) return undefined;
    return { activeTools: parsed.activeTools.filter((name): name is string => typeof name === "string") };
  } catch {
    return undefined;
  }
}
