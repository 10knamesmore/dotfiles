import { readFileSync } from "node:fs";
import { Type } from "@earendil-works/pi-ai";
import { defineTool, type ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { readPersonalConfig } from "@dotfiles/pi-config";
import { renderPatchCall, renderPatchResult, type PatchRenderState, type PatchStatus } from "./presentation.ts";
import { runPatch, type PatchFileChange } from "./run.ts";

const grammar = readFileSync(new URL("./apply_patch.lark", import.meta.url), "utf8");
const parameters = Type.Object({
  input: Type.String({ description: "The complete patch, including *** Begin Patch and *** End Patch." }),
});
type PatchDetails = { status: PatchStatus; changes: PatchFileChange[] };

export default function applyPatchExtension(pi: ExtensionAPI): void {
  const { replaceEditAndWrite } = readPersonalConfig()["apply-patch"];
  if (replaceEditAndWrite) {
    pi.on("session_start", () => {
      const activeTools = pi.getActiveTools();
      if (activeTools.includes("apply_patch")) {
        pi.setActiveTools(activeTools.filter((name) => name !== "edit" && name !== "write"));
      }
    });
  }

  pi.registerTool(
    defineTool<typeof parameters, PatchDetails, PatchRenderState>({
      name: "apply_patch",
      label: "Apply patch",
      description:
        "used to edit files. When exposed as a FREEFORM tool, send the patch directly without JSON; otherwise pass it in `input`.",
      promptSnippet: "Add, delete, update, or move files with a Codex patch",
      promptGuidelines: [
        "Use apply_patch to add, delete, update, or move files.",
        "apply_patch uses *** Begin Patch / *** End Patch around one or more file operations. Use *** Add File: <path> with + lines, *** Delete File: <path>, or *** Update File: <path> with optional *** Move to: <path>. Update chunks use @@ or @@ <context>, then space-prefixed context, - removals, and + additions; *** End of File anchors the final chunk. Paths are relative to the working directory or absolute. Do not use unified-diff line numbers.",
      ],
      parameters,
      constrainedSampling: { type: "grammar", variants: { openai_lark: grammar } },
      executionMode: "sequential",
      renderShell: "self",
      async execute(_id, { input }, signal, onUpdate, ctx) {
        onUpdate?.({
          content: [{ type: "text", text: "Applying patch…" }],
          details: { status: "applying", changes: [] },
        });
        const result = await runPatch(pi, input, ctx.cwd, signal);
        const status = result.killed ? "cancelled" : result.code === 0 ? "applied" : "failed";
        return {
          content: [
            {
              type: "text",
              text:
                [result.stdout, result.stderr].filter(Boolean).join("\n") ||
                `codex apply_patch ${status} (exit ${result.code}).`,
            },
          ],
          details: { status, changes: result.changes },
          isError: status !== "applied",
        };
      },
      renderCall(args, theme, context) {
        return renderPatchCall(args.input ?? "", theme, context);
      },
      renderResult(result, options, theme, context) {
        return renderPatchResult(
          result.details?.status,
          result.details?.changes ?? [],
          result.content.filter((block) => block.type === "text").map((block) => block.text).join("\n"),
          options.isPartial,
          theme,
          context,
        );
      },
    }),
  );
}
