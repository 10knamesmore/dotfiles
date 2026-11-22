import { copyFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

/** Supply editor types without changing the script submitted to the workflow VM. */
export async function writeWorkflowEditorProject(directory: string): Promise<void> {
  await copyFile(new URL("../workflow-api.d.ts", import.meta.url), join(directory, "workflow-api.d.ts"));
  await writeFile(
    join(directory, "workflow-env.d.ts"),
    `declare const agent: typeof import("./workflow-api.js").agent;
declare const parallel: typeof import("./workflow-api.js").parallel;
declare const pipeline: typeof import("./workflow-api.js").pipeline;
declare const phase: typeof import("./workflow-api.js").phase;
declare const log: typeof import("./workflow-api.js").log;
declare const console: typeof import("./workflow-api.js").console;
/** Deeply frozen inputs from args.json. */
declare const args: import("./workflow-api.js").DeepReadonly<typeof import("./args.json")>;
`,
    "utf8",
  );
  await writeFile(
    join(directory, "tsconfig.json"),
    `${JSON.stringify(
      {
        compilerOptions: {
          target: "ES2022",
          module: "ESNext",
          moduleResolution: "Bundler",
          lib: ["ES2022"],
          types: [],
          allowJs: true,
          // Workflows allow top-level return; the editor supplies completion and hover, not diagnostics.
          checkJs: false,
          strict: true,
          noEmit: true,
          resolveJsonModule: true,
        },
        files: ["workflow.js", "workflow-env.d.ts"],
      },
      null,
      2,
    )}\n`,
    "utf8",
  );
}
