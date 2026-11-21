import { spawn } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import type { TUI } from "@earendil-works/pi-tui";
import type { SourceDocument } from "./source-layout.js";

export async function viewSourceInEditor(
  document: SourceDocument,
  tui: Pick<TUI, "stop" | "start" | "requestRender">,
  configuredEditor?: string,
): Promise<void> {
  const command = configuredEditor?.trim() || process.env.VISUAL?.trim() || process.env.EDITOR?.trim() || "nvim";
  const [editor, ...args] = command.split(/\s+/);
  if (basename(editor!) === "nvim") args.push("-M", "-n", "-i", "NONE");

  const directory = await mkdtemp(join(tmpdir(), "pi-context-"));
  try {
    const file = join(directory, document.format === "json" ? "context.json" : "context.md");
    const body = document.format === "json" ? JSON.stringify(JSON.parse(document.body), null, 2) + "\n" : document.body;
    await writeFile(file, body, { encoding: "utf8", mode: 0o600 });

    tui.stop();
    try {
      await new Promise<void>((resolve, reject) => {
        const child = spawn(editor!, [...args, file], { stdio: "inherit" });
        child.once("error", reject);
        child.once("close", (code, signal) => {
          if (code === 0) resolve();
          else reject(new Error(`${editor} exited with ${signal ?? `code ${code}`}`));
        });
      });
    } finally {
      tui.start();
      tui.requestRender(true);
    }
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}
