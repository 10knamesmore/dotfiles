import { parentPort } from "node:worker_threads";
import { stripTerminalSequences, wrapTextWithAnsi } from "@earendil-works/pi-tui";
import type { SourceDocument, SourceViewport, SourceWorkerMessage } from "./source-layout.ts";

// Use Pi's installed highlighter with an explicit theme rather than its process-global theme.
const { highlight, loadAllHighlightLanguages } = await import(
  new URL("utils/syntax-highlight.js", import.meta.resolve("@earendil-works/pi-coding-agent")).href
);
await loadAllHighlightLanguages();

interface PreparedDocument {
  document: SourceDocument;
  highlighted?: string[];
  layout?: {
    width: number;
    gutterWidth: number;
    lines: SourceViewport["lines"];
  };
}

const documents = new Map<number, PreparedDocument>();
let theme: Record<string, (text: string) => string> = {};

parentPort!.on("message", (message: SourceWorkerMessage) => {
  switch (message.type) {
    case "theme":
      theme = Object.fromEntries(
        Object.entries(message.styles).map(([scope, [opening, closing]]) => [
          scope,
          (text: string) => opening + text + closing,
        ]),
      );
      for (const document of documents.values()) {
        document.highlighted = undefined;
        document.layout = undefined;
      }
      break;
    case "document":
      documents.set(message.documentId, {
        document: { ...message.document, body: stripTerminalSequences(message.document.body) },
      });
      break;
    case "viewport": {
      const request = message.request;
      const prepared = documents.get(request.documentId)!;
      prepared.highlighted ??= (
        highlight(prepared.document.body, {
          language: prepared.document.format === "json" ? "json" : "xml",
          ignoreIllegals: true,
          theme,
        }) as string
      ).split("\n");
      if (prepared.layout?.width !== request.width) {
        const gutterWidth = String(prepared.highlighted.length).length;
        prepared.layout = {
          width: request.width,
          gutterWidth,
          lines: prepared.highlighted.flatMap((line, index) =>
            wrapTextWithAnsi(line, Math.max(1, request.width - gutterWidth - 3)).map((text, continuation) => ({
              text,
              sourceLine: continuation === 0 ? index + 1 : undefined,
            })),
          ),
        };
      }
      const { lines, gutterWidth } = prepared.layout;
      const scrollTop = Math.min(request.scrollTop, Math.max(0, lines.length - request.height));
      const firstRow = Math.max(0, scrollTop - request.height);
      parentPort!.postMessage({
        ...request,
        scrollTop,
        contentHeight: lines.length,
        gutterWidth,
        firstRow,
        lines: lines.slice(firstRow, scrollTop + request.height * 2),
      } satisfies SourceViewport);
      break;
    }
  }
});
