import { relative } from "node:path";
import { renderDiff, type Theme, type ToolDefinition } from "@earendil-works/pi-coding-agent";
import { Box, Container, Spacer, Text } from "@earendil-works/pi-tui";
import type { TSchema } from "typebox";
import type { PatchFileChange } from "./run.ts";

type PatchOperation = "add" | "delete" | "update";

interface PatchFile {
  operation: PatchOperation;
  path: string;
  destination?: string;
  lines: string[];
  additions: number;
  removals: number;
}

export interface PatchRenderState {
  patch?: string;
  files?: PatchFile[];
}

export type PatchStatus = "applying" | "applied" | "cancelled" | "failed";
type PatchRenderContext = Parameters<NonNullable<ToolDefinition<TSchema, unknown, PatchRenderState>["renderCall"]>>[2];

const operationLabels = { add: "Add", delete: "Delete", update: "Update" };
const operationColors = { add: "toolDiffAdded", delete: "toolDiffRemoved", update: "accent" } as const;

/** This is a streaming display projection, not the parser that authorizes file changes. */
function describePatch(patch: string): PatchFile[] {
  const files: PatchFile[] = [];
  let file: PatchFile | undefined;
  for (const line of patch.split("\n")) {
    const header = /^\*\*\* (Add|Delete|Update) File: (.+)$/.exec(line.trim());
    if (header) {
      file = {
        operation: header[1]!.toLowerCase() as PatchOperation,
        path: header[2]!,
        lines: [],
        additions: 0,
        removals: 0,
      };
      files.push(file);
      continue;
    }
    if (!file) continue;
    const move = /^\*\*\* Move to: (.+)$/.exec(line.trim());
    if (move) {
      file.destination = move[1]!;
    } else if (/^[+\- ]/.test(line)) {
      file.lines.push(`${line[0]} ${line.slice(1)}`);
      if (line.startsWith("+")) file.additions++;
      if (line.startsWith("-")) file.removals++;
    } else if (line.startsWith("@@")) {
      file.lines.push(line);
    } else if (line.trim() === "*** End of File") {
      file.lines.push("@@ End of file");
    }
  }
  return files;
}

function displayPath(path: string, cwd: string): string {
  if (!path.startsWith("/")) return path;
  const local = relative(cwd, path);
  return local && !local.startsWith("../") ? local : path;
}

function fileSummary(file: PatchFile, cwd: string, theme: Theme): string {
  const label = file.destination ? "Move" : operationLabels[file.operation];
  const path = displayPath(file.path, cwd);
  const destination = file.destination
    ? ` ${theme.fg("muted", "→")} ${theme.fg("accent", displayPath(file.destination, cwd))}`
    : "";
  const parts = [theme.fg(operationColors[file.operation], label), theme.fg("accent", path) + destination];
  if (file.additions > 0) parts.push(theme.fg("toolDiffAdded", `+${file.additions}`));
  if (file.removals > 0) parts.push(theme.fg("toolDiffRemoved", `-${file.removals}`));
  // if (file.operation === "delete") parts.push(theme.fg("muted", "entire file"));
  return parts.join("  ");
}

export function renderPatchCall(patch: string, theme: Theme, context: PatchRenderContext): Container {
  const container = clearContainer(context.lastComponent);
  // Final results replace the previews instead of repeating the same files above their diffs.
  if (!context.isPartial) return container;
  if (context.state.patch !== patch || !context.state.files) {
    context.state.patch = patch;
    context.state.files = describePatch(patch);
  }
  const title = theme.fg("toolTitle", theme.bold("apply_patch"));
  for (const file of context.state.files) {
    const preview = context.expanded && !context.argsComplete ? renderDiff(file.lines.join("\n")) : "";
    addFileBlock(
      container,
      `${title}  ${fileSummary(file, context.cwd, theme)}`,
      preview,
      "toolPendingBg",
      theme,
      context.outputPad,
    );
  }
  if (!context.argsComplete) {
    container.addChild(new Text(theme.fg("muted", "Generating patch…"), context.outputPad, 0));
  }
  return container;
}

export function renderPatchResult(
  status: PatchStatus | undefined,
  changes: PatchFileChange[],
  isPartial: boolean,
  theme: Theme,
  context: PatchRenderContext,
): Container {
  const container = clearContainer(context.lastComponent);
  if (isPartial || status === "applying") {
    container.addChild(new Text(theme.fg("muted", "Applying patch…"), context.outputPad, 0));
    return container;
  }
  const title = theme.fg("toolTitle", theme.bold("apply_patch"));
  for (const change of changes) {
    const additions = change.diff.split("\n").filter((line) => line.startsWith("+")).length;
    const removals = change.diff.split("\n").filter((line) => line.startsWith("-")).length;
    const heading = [title, theme.fg("accent", displayPath(change.path, context.cwd))];
    if (additions) heading.push(theme.fg("toolDiffAdded", `+${additions}`));
    if (removals) heading.push(theme.fg("toolDiffRemoved", `-${removals}`));
    addFileBlock(
      container,
      heading.join("  "),
      context.expanded ? renderDiff(change.diff) : "",
      status === "applied" ? "toolSuccessBg" : "toolErrorBg",
      theme,
      context.outputPad,
    );
  }
  const label = status === "cancelled" ? "Interrupted" : "Failed";
  const summary =
    status === "applied"
      ? theme.fg("success", "✓ Patch applied")
      : theme.fg(
          status === "cancelled" ? "warning" : "error",
          `${label} · Files may be partially modified; inspect them before retrying.`,
        );
  container.addChild(new Text(`${changes.length ? "" : `${title} · `}${summary}`, context.outputPad, 0));
  return container;
}

function addFileBlock(
  container: Container,
  heading: string,
  diff: string,
  background: "toolPendingBg" | "toolSuccessBg" | "toolErrorBg",
  theme: Theme,
  padding: number,
): void {
  if (container.children.length) container.addChild(new Spacer(1));
  const box = new Box(padding, 1, (line) => theme.bg(background, line));
  box.addChild(new Text(diff ? `${heading}\n${diff}` : heading, 0, 0));
  container.addChild(box);
}

function clearContainer(previous: unknown): Container {
  const container = previous instanceof Container ? previous : new Container();
  container.clear();
  return container;
}
