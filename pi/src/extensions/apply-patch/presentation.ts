import { relative, resolve } from "node:path";
import { renderDiff, type Theme, type ToolDefinition } from "@earendil-works/pi-coding-agent";
import { Box, Container, MouseRegion, Spacer, Text, type TuiMouseEvent } from "@earendil-works/pi-tui";
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

export type PatchStatus = "applying" | "applied" | "cancelled" | "failed";

export interface PatchRenderState {
  expansion?: {
    allExpanded: boolean;
    files: Map<string, boolean>;
  };
}

type PatchRenderContext = Parameters<NonNullable<ToolDefinition<TSchema, unknown, PatchRenderState>["renderCall"]>>[2];

const operationLabels = { add: "Add", delete: "Delete", update: "Update" };
const operationColors = { add: "toolDiffAdded", delete: "toolDiffRemoved", update: "accent" } as const;

function fileExpansion(context: PatchRenderContext): NonNullable<PatchRenderState["expansion"]> {
  // A native expand/collapse action replaces all per-file choices for this call.
  if (!context.state.expansion || context.state.expansion.allExpanded !== context.expanded) {
    context.state.expansion = { allExpanded: context.expanded, files: new Map() };
  }
  return context.state.expansion;
}

function isFileExpanded(path: string, context: PatchRenderContext): boolean {
  const expansion = fileExpansion(context);
  return expansion.files.get(resolve(context.cwd, path)) ?? expansion.allExpanded;
}

function toggleFileOnClick(event: TuiMouseEvent, path: string, context: PatchRenderContext) {
  if (event.type !== "click" || event.button !== "left") return undefined;
  const expanded = !isFileExpanded(path, context);
  fileExpansion(context).files.set(resolve(context.cwd, path), expanded);
  context.invalidate();
  return { handled: true };
}

function disclosureMarker(expanded: boolean, theme: Theme): string {
  return theme.fg("muted", expanded ? "▼" : "▶");
}

/** Consume clicks outside file blocks so Pi's outer tool region does not toggle the entire call. */
class FileList extends MouseRegion {
  readonly content: Container;

  constructor() {
    const content = new Container();
    super(content, (event) =>
      event.type === "click" && event.button === "left" ? { handled: true } : undefined,
    );
    this.content = content;
  }
}

/** Projects complete input lines into a preview; file changes are validated by the patch engine. */
class PatchPreview extends FileList {
  private source = "";
  private parsedLength = 0;
  private readonly fileBlocks = new Container();
  private readonly status = new Text("", 0, 0);
  private files: PatchFilePreview[] = [];

  constructor() {
    super();
    this.content.addChild(this.fileBlocks);
    this.content.addChild(this.status);
  }

  update(patch: string, theme: Theme, context: PatchRenderContext): void {
    // Pi supplies argument snapshots, including replacements made by tool_call handlers.
    if (!patch.startsWith(this.source.slice(0, this.parsedLength))) {
      this.parsedLength = 0;
      this.files = [];
      this.fileBlocks.clear();
    }
    this.source = patch;
    let lineEnd: number;
    while ((lineEnd = patch.indexOf("\n", this.parsedLength)) !== -1) {
      this.appendLine(patch.slice(this.parsedLength, lineEnd).replace(/\r$/, ""), theme, context);
      this.parsedLength = lineEnd + 1;
    }
    if (context.argsComplete && this.parsedLength < patch.length) {
      this.appendLine(patch.slice(this.parsedLength).replace(/\r$/, ""), theme, context);
      this.parsedLength = patch.length;
    }
    for (const file of this.files) file.configure(theme, context);
    this.status.setPaddingX(context.outputPad);
    this.status.setText(context.argsComplete ? "" : theme.fg("muted", "Generating patch…"));
  }

  private appendLine(line: string, theme: Theme, context: PatchRenderContext): void {
    const header = /^\*\*\* (Add|Delete|Update) File: (.+)$/.exec(line.trim());
    if (header) {
      const file = new PatchFilePreview(
        {
          operation: header[1]!.toLowerCase() as PatchOperation,
          path: header[2]!,
          lines: [],
          additions: 0,
          removals: 0,
        },
        theme,
        context,
      );
      if (this.files.length) this.fileBlocks.addChild(new Spacer(1));
      this.files.push(file);
      this.fileBlocks.addChild(new MouseRegion(file, (event) => file.toggleOnClick(event)));
    } else {
      this.files.at(-1)?.appendLine(line);
    }
  }
}

class PatchFilePreview extends Box {
  private readonly heading = new Text("", 0, 0);
  private readonly diff = new Text("", 0, 0);
  private needsTextUpdate = true;
  private showDiff = false;
  private outputPad: number;

  constructor(
    private readonly file: PatchFile,
    private theme: Theme,
    private context: PatchRenderContext,
  ) {
    super(context.outputPad, 1);
    this.outputPad = context.outputPad;
    this.setBgFn((line) => this.theme.bg("toolPendingBg", line));
    this.addChild(this.heading);
    this.addChild(this.diff);
  }

  toggleOnClick(event: TuiMouseEvent) {
    return toggleFileOnClick(event, this.file.path, this.context);
  }

  configure(theme: Theme, context: PatchRenderContext): void {
    const showDiff = isFileExpanded(this.file.path, context);
    if (this.theme !== theme || this.context.cwd !== context.cwd || this.showDiff !== showDiff) {
      this.theme = theme;
      this.showDiff = showDiff;
      this.needsTextUpdate = true;
    }
    this.context = context;
    if (this.outputPad !== context.outputPad) {
      this.outputPad = context.outputPad;
      this.setPaddingX(context.outputPad);
    }
  }

  appendLine(line: string): void {
    const file = this.file;
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
    } else {
      return;
    }
    this.needsTextUpdate = true;
  }

  override invalidate(): void {
    super.invalidate();
    // Theme changes must regenerate ANSI text as well as invalidate layout.
    this.needsTextUpdate = true;
  }

  override render(width: number): string[] {
    if (this.needsTextUpdate) {
      const title = this.theme.fg("toolTitle", this.theme.bold("apply_patch"));
      const heading = `${title}  ${fileSummary(this.file, this.context.cwd, this.theme)}`;
      const diff = this.showDiff ? renderDiff(this.file.lines.join("\n")) : "";
      this.heading.setText(`${disclosureMarker(this.showDiff, this.theme)} ${heading}`);
      this.diff.setText(diff);
      this.needsTextUpdate = false;
    }
    return super.render(width);
  }
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
  return parts.join("  ");
}

export function renderPatchCall(patch: string, theme: Theme, context: PatchRenderContext): Container | MouseRegion {
  // Final results replace the previews instead of repeating the same files above their diffs.
  if (!context.isPartial) return new Container();
  const preview = context.lastComponent instanceof PatchPreview ? context.lastComponent : new PatchPreview();
  preview.update(patch, theme, context);
  return preview;
}

export function renderPatchResult(
  status: PatchStatus | undefined,
  changes: PatchFileChange[],
  output: string,
  isPartial: boolean,
  theme: Theme,
  context: PatchRenderContext,
): MouseRegion {
  const list = context.lastComponent instanceof FileList ? context.lastComponent : new FileList();
  const container = list.content;
  container.clear();
  if (isPartial || status === "applying") {
    container.addChild(new Text(theme.fg("muted", "Applying patch…"), context.outputPad, 0));
    return list;
  }
  const title = theme.fg("toolTitle", theme.bold("apply_patch"));
  for (const change of changes) {
    const additions = change.diff.split("\n").filter((line) => line.startsWith("+")).length;
    const removals = change.diff.split("\n").filter((line) => line.startsWith("-")).length;
    const heading = [title, theme.fg("accent", displayPath(change.path, context.cwd))];
    if (change.destination) {
      heading.splice(1, 0, theme.fg("accent", "Move"));
      heading.push(theme.fg("muted", "→"), theme.fg("accent", displayPath(change.destination, context.cwd)));
    }
    if (additions) heading.push(theme.fg("toolDiffAdded", `+${additions}`));
    if (removals) heading.push(theme.fg("toolDiffRemoved", `-${removals}`));
    addFileBlock(
      container,
      change.path,
      heading.join("  "),
      change.diff,
      status === "applied" ? "toolSuccessBg" : "toolErrorBg",
      theme,
      context,
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
  if (status !== "applied" && output) {
    container.addChild(new Text(theme.fg("toolOutput", output), context.outputPad, 0));
  }
  return list;
}

function addFileBlock(
  container: Container,
  path: string,
  heading: string,
  diff: string,
  background: "toolSuccessBg" | "toolErrorBg",
  theme: Theme,
  context: PatchRenderContext,
): void {
  if (container.children.length) container.addChild(new Spacer(1));
  const expanded = isFileExpanded(path, context);
  const box = new Box(context.outputPad, 1, (line) => theme.bg(background, line));
  box.addChild(new Text(`${disclosureMarker(expanded, theme)} ${heading}`, 0, 0));
  if (expanded && diff) box.addChild(new Text(renderDiff(diff), 0, 0));
  container.addChild(new MouseRegion(box, (event) => toggleFileOnClick(event, path, context)));
}
