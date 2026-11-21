import type { AgentToolResult, ReadToolDetails } from "@earendil-works/pi-coding-agent";

export interface SourceRange {
  firstLine: number;
  lineCount: number;
}

export interface NumberedReadDetails extends ReadToolDetails {
  sourceRange?: SourceRange;
}

export const GUTTER_DIVIDER = " │ ";

export function lineNumberWidth({ firstLine, lineCount }: SourceRange): number {
  return String(firstLine + lineCount - 1).length;
}

export function numberTextRead(
  result: AgentToolResult<ReadToolDetails | undefined>,
  firstLine: number,
  requestedLines: number | undefined,
): AgentToolResult<NumberedReadDetails | undefined> {
  const block = result.content[0];
  if (block?.type !== "text") return result;

  const { text } = block;
  const lines = text.split("\n");
  // Pi appends its continuation notice after the requested source lines.
  const lineCount =
    result.details?.truncation?.outputLines ??
    (requestedLines !== undefined && lines.length > requestedLines
      ? requestedLines
      : text.length === 0
        ? 0
        : lines.length - (text.endsWith("\n") ? 1 : 0));
  if (lineCount === 0) return result;

  const sourceRange = { firstLine, lineCount };
  const width = lineNumberWidth(sourceRange);
  for (let index = 0; index < lineCount; index++) {
    lines[index] = `${String(firstLine + index).padStart(width)}${GUTTER_DIVIDER}${lines[index]}`;
  }
  return {
    ...result,
    content: [{ type: "text", text: lines.join("\n") }],
    details: { ...result.details, sourceRange },
  };
}

export function splitNumberedRead(text: string, range: SourceRange) {
  const lines = text.split("\n");
  const prefixWidth = lineNumberWidth(range) + GUTTER_DIVIDER.length;
  return {
    source: lines
      .slice(0, range.lineCount)
      .map((line) => line.slice(prefixWidth))
      .join("\n"),
    notice: lines.slice(range.lineCount).join("\n"),
  };
}
