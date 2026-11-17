import { getMarkdownTheme, type MarkdownTransformer } from "@earendil-works/pi-coding-agent";
import { COPY_LABEL } from "@dotfiles/pi-ui/action-feedback";
import {
  Marked,
  Markdown,
  sliceByColumn,
  stripTerminalSequences,
  truncateToWidth,
  visibleWidth,
  type Token,
  type Tokens,
} from "@earendil-works/pi-tui";

const parser = new Marked();

// The button component reads and removes this zero-width marker after Markdown has laid out the header.
export const CODE_COPY_MARKER = /\x1b\]777;pi-code-copy;(\d+);([A-Za-z0-9+/=]*)\x07/g;

export interface ReplyCodeBlock {
  text: string;
  language: string;
  key: string;
  occurrence: number;
}

export interface CodeHighlight {
  key: string;
  occurrence: number;
  paint: (line: string) => string;
}

function blockKey(block: Tokens.Code): string {
  return JSON.stringify([block.lang ?? "", block.text.replace(/\t/g, "   ")]);
}

export function replyCodeBlocks(markdown: string): ReplyCodeBlock[] {
  const blocks: ReplyCodeBlock[] = [];
  const occurrences = new Map<string, number>();
  parser.walkTokens(parser.lexer(markdown), (token) => {
    if (token.type !== "code") return;
    const block = token as Tokens.Code;
    const key = blockKey(block);
    const occurrence = occurrences.get(key) ?? 0;
    occurrences.set(key, occurrence + 1);
    blocks.push({ text: block.text, language: block.lang ?? "", key, occurrence });
  });
  return blocks;
}

/** Keep ANSI colours and literal code intact when Pi parses the transformed Markdown. */
function codeSpan(line: string): string {
  const longest = Math.max(0, ...Array.from(line.matchAll(/`+/g), (match) => match[0].length));
  const fence = "`".repeat(longest + 1);
  return `${fence} ${line} ${fence}`;
}

function borderedCode(block: Tokens.Code, width: number, paint?: (line: string) => string): string {
  const theme = getMarkdownTheme();
  const border = theme.codeBlockBorder;
  const innerWidth = Math.max(1, width - 4);
  const button = truncateToWidth(COPY_LABEL, innerWidth + 2, "");
  const titleWidth = innerWidth + 2 - visibleWidth(button);
  const heading = truncateToWidth(block.lang ? `─ ${block.lang} ` : "─", titleWidth, "");
  const marker = `\x1b]777;pi-code-copy;${visibleWidth(button)};${Buffer.from(block.text).toString("base64")}\x07`;
  const top =
    border(`╭${heading}${"─".repeat(titleWidth - visibleWidth(heading))}`) + marker + theme.link(button) + border("╮");
  const bottom = border(`╰${"─".repeat(innerWidth + 2)}╯`);
  // Empty fence rows stay one line even when the language name exceeds the available width.
  // This retains Pi's highlighting, wrapping and handling of unfinished streaming fences.
  const code = new Markdown(block.raw, 0, 0, { ...theme, codeBlockIndent: "", codeBlockBorder: () => "" });
  const body = code
    .render(innerWidth)
    .slice(1, -1)
    .map((line) => border("│ ") + line + border(" │"));
  const lines = [top, ...body, bottom];
  return `\n\n${lines.map((line) => codeSpan(paint ? paint(line) : line)).join("  \n")}\n\n`;
}

function trailingNewlines(text: string): string {
  return /\n*$/.exec(text)![0];
}

function centeredTable(table: Tokens.Table, width: number): string {
  // Pi displays raw Markdown when even one character per column cannot fit.
  if (width < table.header.length * 4 + 1) return table.raw;
  const lines = new Markdown(table.raw, 0, 0, getMarkdownTheme()).render(width);
  const columnWidths = lines[0]!
    .trimEnd()
    .slice(1, -1)
    .split("┬")
    .map((cell) => cell.length - 2);
  const centered = lines.map((line, index) => {
    if (index === 0) return line.trimEnd().replace(/^┌/, "╭").replace(/┐$/, "╮");
    if (index === lines.length - 1) return line.trimEnd().replace(/^└/, "╰").replace(/┘$/, "╯");
    if (!line.startsWith("│")) return line.trimEnd();
    let columnStart = 2;
    const cells = columnWidths.map((columnWidth) => {
      // Include the following space so truncation closes ANSI styles and hyperlinks.
      const cell = sliceByColumn(line, columnStart, columnWidth + 1);
      columnStart += columnWidth + 3;
      const textWidth = visibleWidth(stripTerminalSequences(cell).trimEnd());
      const padding = columnWidth - textWidth;
      const left = Math.floor(padding / 2);
      return " ".repeat(left) + truncateToWidth(cell, textWidth, "") + " ".repeat(padding - left);
    });
    return `│ ${cells.join(" │ ")} │`;
  });
  const tableWidth = visibleWidth(centered[0]!);
  const margin = " ".repeat(Math.floor((width - tableWidth) / 2));
  // Inline code preserves the laid-out rows; reset its foreground to normal table text.
  return `\n\n${centered.map((line) => codeSpan(`\x1b[39m${margin}${line}`)).join("  \n")}\n\n`;
}

/** One renderer per extension instance; a copy wrapper scopes highlighting to its own text chunk. */
export function createReplyMarkdown() {
  let highlight: CodeHighlight | undefined;

  const transform: MarkdownTransformer = (markdown, context) => {
    if (context.messageType !== "assistant") return markdown;
    const occurrences = new Map<string, number>();
    let renderedBlocks = 0;

    const transformList = (list: Tokens.List, width: number, depth = 0): string => {
      const before = renderedBlocks;
      const items = list.items.map((item, index) => {
        const prefix = /^( {0,3}(?:[-+*]|\d{1,9}[.)])[ \t]*)/.exec(item.raw)![0];
        const marker = list.ordered ? `${Number(list.start) + index}. ` : "- ";
        const itemWidth = Math.max(1, width - depth * 4 - visibleWidth(marker) - (item.task ? 4 : 0));
        const text = item.tokens
          .map((token) =>
            token.type === "list"
              ? transformList(token as Tokens.List, width, depth + 1)
              : transformToken(token, itemWidth),
          )
          .join("")
          .replace(/^\n+/u, "")
          .replace(/\n+$/u, "");
        const indent = " ".repeat(visibleWidth(prefix));
        return prefix + text.replace(/\n/g, `\n${indent}`) + trailingNewlines(item.raw);
      });
      return renderedBlocks === before ? list.raw : items.join("").replace(/\n*$/u, trailingNewlines(list.raw));
    };

    const transformToken = (token: Token, width: number): string => {
      if (token.type === "table") {
        renderedBlocks++;
        return centeredTable(token as Tokens.Table, width);
      }
      if (token.type === "code") {
        renderedBlocks++;
        const block = token as Tokens.Code;
        const key = blockKey(block);
        const occurrence = occurrences.get(key) ?? 0;
        occurrences.set(key, occurrence + 1);
        const paint = highlight?.key === key && highlight.occurrence === occurrence ? highlight.paint : undefined;
        return borderedCode(block, width, paint);
      }
      if (token.type === "blockquote") {
        const before = renderedBlocks;
        const quote = token as Tokens.Blockquote;
        const body = quote.tokens.map((child) => transformToken(child, Math.max(1, width - 2))).join("");
        if (renderedBlocks === before) return token.raw;
        return (
          body
            .replace(/^\n+/u, "")
            .replace(/\n+$/u, "")
            .split("\n")
            .map((line) => `> ${line}`)
            .join("\n") + trailingNewlines(token.raw)
        );
      }
      if (token.type === "list") return transformList(token as Tokens.List, width);
      return token.raw;
    };

    const rendered = parser
      .lexer(markdown)
      .map((token) => transformToken(token, context.availableWidth))
      .join("");
    return renderedBlocks ? rendered.trim() : markdown;
  };

  return {
    transform,
    withCodeHighlight<T>(selection: CodeHighlight | undefined, render: () => T): T {
      const previous = highlight;
      highlight = selection;
      try {
        return render();
      } finally {
        highlight = previous;
      }
    },
  };
}

export type ReplyMarkdown = ReturnType<typeof createReplyMarkdown>;
