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
  const paths = [
    ...new Set(
      patch.split("\n").flatMap((line) => {
        const header = /^\*\*\* (?:Add File|Delete File|Update File|Move to): (.+)$/.exec(line.trim());
        return header ? [resolve(cwd, header[1]!)] : [];
      }),
    ),
  ];
  // Resolve aliases before nesting locks: two names for one file must not lock it twice.
  const files = [...new Set(await Promise.all(paths.map(mutationPath)))].sort();
  const apply = async (index: number): Promise<PatchResult> => {
    signal?.throwIfAborted();
    const file = files[index];
    if (file !== undefined) return withFileMutationQueue(file, () => apply(index + 1));
    const before = await Promise.all(paths.map(fileContents));
    const result = await pi.exec("codex", ["--codex-run-as-apply-patch", patch], { cwd, signal });
    const after = await Promise.all(paths.map(fileContents));
    const changes = paths.flatMap((path, index) => {
      const { diff } = generateDiffString(before[index]!, after[index]!);
      return diff ? [{ path, diff }] : [];
    });
    return { ...result, changes };
  };
  return apply(0);
}

/** A missing file is the empty side of a creation or deletion diff. */
async function fileContents(path: string): Promise<string> {
  try {
    return (await readFile(path, "utf8")).replace(/\r\n/g, "\n");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return "";
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
