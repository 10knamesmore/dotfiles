import { constants } from "node:fs";
import { access, open, readFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import {
  createReadToolDefinition,
  detectSupportedImageMimeTypeFromFile,
  type ExtensionAPI,
} from "@earendil-works/pi-coding-agent";
import { getCapabilities, Text } from "@earendil-works/pi-tui";
import { numberTextRead, splitNumberedRead, type NumberedReadDetails } from "./numbered-text.js";
import { ReadCallHeader, renderNumberedSource } from "./presentation.js";

async function* readLocalChunks(path: string) {
  const handle = await open(path, "r");
  try {
    const stats = await handle.stat();
    const end = stats.isFile() && stats.size > 0 ? stats.size - 1 : undefined;
    yield* handle.createReadStream({ end, autoClose: false });
  } finally {
    await handle.close();
  }
}

export default function registerNumberedRead(pi: ExtensionAPI): void {
  const builtin = createReadToolDefinition(process.cwd());

  pi.registerTool<typeof builtin.parameters, NumberedReadDetails | undefined, { header?: ReadCallHeader }>({
    ...builtin,
    description: `${builtin.description} line numbers are added for text file`,
    async execute(id, input, signal, onUpdate, ctx) {
      let textPath: string | undefined;
      let fileLines = 0;
      const start = Math.max(0, (input.offset ?? 1) - 1);
      const end = input.limit === undefined ? undefined : Math.trunc(start + input.limit);
      const read = createReadToolDefinition(ctx.cwd, {
        autoResizeImages: pi.getSettings().images?.autoResize,
        operations: {
          access: (path) => access(path, constants.R_OK),
          createReadStream: readLocalChunks,
          async detectImageMimeType(path) {
            const mimeType = await detectSupportedImageMimeTypeFromFile(path);
            if (mimeType === null) textPath = path;
            return mimeType;
          },
          async readFile(path) {
            const bytes = await readFile(path);
            if (textPath && end !== undefined && end < 0) {
              fileLines = bytes.toString("utf8").split("\n").length;
            }
            return bytes;
          },
        },
      });
      const result = await read.execute(id, input, signal, onUpdate, ctx);
      if (!textPath) return result;

      const firstIndex = Math.trunc(start);
      const requestedLines =
        end === undefined ? undefined : Math.max(0, (end < 0 ? fileLines + end : end) - firstIndex);
      const numbered = numberTextRead(result, firstIndex + 1, requestedLines);
      return { ...numbered, details: { ...numbered.details, sourcePath: textPath } };
    },
    renderCall(args, theme, context) {
      const header = (context.state.header ??= new ReadCallHeader("", 0, 0));
      return builtin.renderCall!(args, theme, { ...context, lastComponent: header });
    },
    renderResult(result, options, theme, context) {
      const range = result.details?.sourceRange;
      const path = result.details?.sourcePath;
      if (!context.isError && getCapabilities().hyperlinks && (path || range)) {
        context.state.header?.linkToLine(range?.firstLine ?? 1, path && pathToFileURL(path).href);
      }
      const block = result.content[0];
      const builtinContext = {
        ...context,
        lastComponent: context.lastComponent instanceof Text ? context.lastComponent : undefined,
      };
      if (!options.expanded || context.isError || !range || block?.type !== "text") {
        return builtin.renderResult!(result, options, theme, builtinContext);
      }

      const { source, notice } = splitNumberedRead(block.text, range);
      const noticeView =
        notice || result.details?.truncation
          ? builtin.renderResult!(
              { ...result, content: [{ type: "text", text: notice }] },
              options,
              theme,
              builtinContext,
            )
          : undefined;
      return renderNumberedSource(source, context.args.path, range, theme, noticeView);
    },
  });
}
