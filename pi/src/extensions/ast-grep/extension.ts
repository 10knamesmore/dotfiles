import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { defineTool, truncateHead, withFileMutationQueue, type ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { parameters, outputSchema } from "./schema.ts";
import { runAstGrep } from "./run.ts";

export default function astGrepExtension(pi: ExtensionAPI): void {
  pi.registerTool(
    defineTool({
      name: "ast_grep",
      label: "AST grep",
      description:
        "Search or rewrite code by syntax using ast-grep. Returns matches, captures, or rewrite diffs. " +
        "Syntax matching does not resolve symbols or types. " +
        "Respects ast-grep's ignore rules. Output is limited to 2000 lines or 50KB; full results are saved when truncated.",
      promptSnippet: "Search code structure or preview/apply pattern-based rewrites with ast-grep",
      promptGuidelines: [
        "Inspect the ast_grep rewrite preview before applying changes.",
      ],
      parameters,
      outputSchema,
      executionMode: "sequential",
      async execute(_id, params, signal, _onUpdate, ctx) {
        const result = await runAstGrep(pi, params, ctx.cwd, signal);
        const summary = `ast_grep ${result.mode}: ${result.status}; ${result.matchCount} matches in ${result.files.length} files.`;
        const text = [summary, result.output, result.command.stderr].filter(Boolean).join("\n\n");
        const truncated = truncateHead(text);
        let content = truncated.content;
        if (truncated.truncated) {
          const directory = await mkdtemp(join(tmpdir(), "pi-ast-grep-"));
          result.fullOutputPath = join(directory, "result.json");
          await withFileMutationQueue(result.fullOutputPath, () =>
            writeFile(result.fullOutputPath!, JSON.stringify(result, null, 2), "utf8"),
          );
          content += `\n\nOutput truncated. Full results: ${result.fullOutputPath}`;
        }
        return {
          content: [{ type: "text", text: content }],
          details: result,
          structuredContent: result,
          isError: result.status !== "completed",
        };
      },
    }),
  );
}
