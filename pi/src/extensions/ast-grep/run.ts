import { readFile, realpath } from "node:fs/promises";
import { resolve } from "node:path";
import {
  generateUnifiedPatch,
  withFileMutationQueue,
  type ExecResult,
  type ExtensionAPI,
} from "@earendil-works/pi-coding-agent";
import type { AstGrepMatch, AstGrepOutput, AstGrepParameters } from "./schema.ts";

class AstGrepInputError extends Error {
  constructor(readonly code: "query_choice" | "missing_replacement") {
    super(
      code === "query_choice"
        ? "Supply exactly one of pattern or rule."
        : "preview requires replacement; omit both to search.",
    );
    this.name = "AstGrepInputError";
  }
}

export async function runAstGrep(
  pi: ExtensionAPI,
  params: AstGrepParameters,
  cwd: string,
  signal: AbortSignal | undefined,
): Promise<AstGrepOutput> {
  if ((params.pattern === undefined) === (params.rule === undefined)) {
    throw new AstGrepInputError("query_choice");
  }
  if (params.preview !== undefined && params.replacement === undefined) {
    throw new AstGrepInputError("missing_replacement");
  }

  const mode = params.replacement === undefined ? "search" : params.preview === false ? "apply" : "preview";
  const rule = JSON.stringify({
    id: "pi-ast-grep",
    language: params.language,
    severity: "hint",
    rule: params.rule ?? { pattern: params.pattern },
    fix: params.replacement,
  });
  const baseArgs = ["scan", "--inline-rules", rule, "--color", "never"];
  for (const glob of params.globs ?? []) baseArgs.push("--globs", glob);

  const execute = (paths: string[], flags: string[]): Promise<ExecResult> => {
    signal?.throwIfAborted();
    return pi.exec("ast-grep", [...baseArgs, ...flags, "--", ...paths], { cwd, signal });
  };
  const scan = async (paths: string[]) => {
    const command = await execute(paths, ["--json=compact"]);
    const matches: AstGrepMatch[] = succeeded(command)
      ? (JSON.parse(command.stdout) as AstGrepMatch[]).map(({ file, text, range, replacement, metaVariables }) => ({
          file,
          text,
          range,
          ...(replacement === undefined ? {} : { replacement }),
          ...(metaVariables === undefined ? {} : { metaVariables }),
        }))
      : [];
    return { command, matches };
  };

  const paths = params.paths.map((path) => resolve(cwd, path));
  const initial = await scan(paths);
  if (!succeeded(initial.command) || initial.matches.length === 0 || mode === "search") {
    const text = succeeded(initial.command) ? JSON.stringify(initial.matches, null, 2) : initial.command.stdout;
    return output(mode, initial.command, initial.matches, text);
  }

  if (mode === "preview") {
    const command = await execute(paths, []);
    return output(mode, command, initial.matches, command.stdout);
  }

  // Discover candidates before locking, then rescan only those files while holding their locks.
  const candidates = filesOf(initial.matches);
  const locks = [...new Set(await Promise.all(candidates.map((path) => realpath(path))))].sort();
  const apply = async (index: number): Promise<AstGrepOutput> => {
    signal?.throwIfAborted();
    const path = locks[index];
    if (path !== undefined) return withFileMutationQueue(path, () => apply(index + 1));

    const current = await scan(candidates);
    if (!succeeded(current.command) || current.matches.length === 0) {
      return output(mode, current.command, current.matches, current.command.stdout);
    }
    const files = filesOf(current.matches);
    const before = await Promise.all(files.map((file) => readFile(file, "utf8")));
    // ast-grep's JSON printer does not write files, even with --update-all.
    const command = await execute(files, ["--update-all"]);
    const after = await Promise.all(files.map((file) => readFile(file, "utf8")));
    const diffs = files.flatMap((file, index) =>
      before[index] === after[index] ? [] : [generateUnifiedPatch(file, before[index]!, after[index]!)],
    );
    return output(mode, command, current.matches, [command.stdout, ...diffs].filter(Boolean).join("\n"));
  };
  return apply(0);
}

function succeeded(command: ExecResult): boolean {
  return command.code === 0 && !command.killed;
}

function filesOf(matches: AstGrepMatch[]): string[] {
  return [...new Set(matches.map((match) => match.file))].sort();
}

function output(
  mode: AstGrepOutput["mode"],
  command: ExecResult,
  matches: AstGrepMatch[],
  text: string,
): AstGrepOutput {
  return {
    mode,
    status: command.killed ? "cancelled" : command.code === 0 ? "completed" : "failed",
    matchCount: matches.length,
    files: filesOf(matches),
    matches,
    output: text,
    command: { exitCode: command.code, stderr: command.stderr },
  };
}
