import { readFile, realpath } from "node:fs/promises";
import { resolve } from "node:path";
import {
  generateDiffString,
  withFileMutationQueue,
  type ExtensionAPI,
  type ExecResult,
} from "@earendil-works/pi-coding-agent";

export interface PatchFileChange {
  path: string;
  destination?: string;
  diff: string;
}

interface PatchResult extends ExecResult {
  changes: PatchFileChange[];
}

/** Hold the file locks through both snapshots so the displayed diff belongs to this call. */
export async function runPatch(
  pi: ExtensionAPI,
  patch: string,
  cwd: string,
  signal: AbortSignal | undefined,
): Promise<PatchResult> {
  const patchPaths = new Set<string>();
  const moves = new Map<string, string>();
  let source: string | undefined;
  for (const line of patch.split("\n")) {
    const header = /^\*\*\* (Add File|Delete File|Update File|Move to): (.+)$/.exec(line.trim());
    if (!header) continue;
    const path = resolve(cwd, header[2]!);
    patchPaths.add(path);
    if (header[1] === "Move to") {
      if (source) moves.set(source, path);
    } else {
      source = header[1] === "Update File" ? path : undefined;
    }
  }
  const paths = [...patchPaths];
  // Resolve aliases before nesting locks: two names for one file must not lock it twice.
  const files = [...new Set(await Promise.all(paths.map(mutationPath)))].sort();
  const apply = async (index: number): Promise<PatchResult> => {
    signal?.throwIfAborted();
    const file = files[index];
    if (file !== undefined) return withFileMutationQueue(file, () => apply(index + 1));
    const before = new Map(await Promise.all(paths.map(async (path) => [path, await fileContents(path)] as const)));
    const result = await pi.exec("codex", ["--codex-run-as-apply-patch", patch], { cwd, signal });
    const after = new Map(await Promise.all(paths.map(async (path) => [path, await fileContents(path)] as const)));
    // A later operation may fail after a move has already completed.
    const completedMoves = new Map(
      [...moves].filter(([path, destination]) =>
        before.get(path) !== null && after.get(path) === null && after.get(destination) !== null,
      ),
    );
    const movedDestinations = new Set(completedMoves.values());
    const changes = paths.flatMap((path): PatchFileChange[] => {
      if (movedDestinations.has(path)) return [];
      const destination = completedMoves.get(path);
      const { diff } = generateDiffString(before.get(path) ?? "", after.get(destination ?? path) ?? "");
      if (destination) return [{ path, destination, diff }];
      return diff ? [{ path, diff }] : [];
    });
    return { ...result, changes };
  };
  return apply(0);
}

async function fileContents(path: string): Promise<string | null> {
  try {
    return (await readFile(path, "utf8")).replace(/\r\n/g, "\n");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return null;
    throw error;
  }
}

async function mutationPath(path: string): Promise<string> {
  try {
    return await realpath(path);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return path;
    throw error;
  }
}
