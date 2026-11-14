import { copyToClipboard, type CustomEditor, type ExtensionContext } from "@earendil-works/pi-coding-agent";
import { SelectList, stripTerminalSequences, truncateToWidth, visibleWidth } from "@earendil-works/pi-tui";
import { highlightCopySource, type CopyTarget } from "./copy-highlight.js";
import { replyCodeBlocks, type ReplyMarkdown } from "./reply-markdown.js";

interface CopyChoice {
  label: string;
  hint: string;
  text: string;
  target?: CopyTarget;
}

/** Install after Pi wires onSubmit: built-in /copy runs before extension commands. */
export function installCopyCommand(editor: CustomEditor, ctx: ExtensionContext, renderer: ReplyMarkdown): void {
  const submit = editor.onSubmit;
  editor.onSubmit = async (text) => {
    if (text.trim() !== "/copy") {
      submit?.(text);
      return;
    }
    editor.setText("");
    await copyLastResponse(ctx, renderer);
  };
}

async function copyLastResponse(ctx: ExtensionContext, renderer: ReplyMarkdown): Promise<void> {
  const lastResponse = findLastResponse(ctx);
  if (lastResponse === undefined) {
    ctx.ui.notify("No assistant response to copy yet.", "info");
    return;
  }
  const { text: response, timestamp, chunks } = lastResponse;

  const responseLineCount = response.replace(/\n$/u, "").split("\n").length;
  const choices: CopyChoice[] = [
    {
      label: "Entire response",
      hint: `${responseLineCount} ${responseLineCount === 1 ? "line" : "lines"}`,
      text: response,
    },
  ];
  chunks.forEach((text, chunk) => {
    for (const block of replyCodeBlocks(text.trim())) {
      const lineCount = block.text.replace(/\n$/u, "").split("\n").length;
      const language = stripTerminalSequences(block.language || "text").replace(/\s+/gu, " ");
      choices.push({
        label: stripTerminalSequences(block.text).replace(/\s+/gu, " ").trim(),
        hint: `${language} · ${lineCount} ${lineCount === 1 ? "line" : "lines"}`,
        text: block.text,
        target: { chunk, key: block.key, occurrence: block.occurrence },
      });
    }
  });

  const choice = await selectCopyChoice(ctx, choices, timestamp, renderer);
  if (choice === undefined) return;
  try {
    await copyToClipboard(choice.text);
    ctx.ui.notify("Copied.", "info");
  } catch {
    ctx.ui.notify("Could not write to the clipboard. Check clipboard access and try again.", "error");
  }
}

function selectCopyChoice(
  ctx: ExtensionContext,
  choices: CopyChoice[],
  messageTimestamp: number,
  renderer: ReplyMarkdown,
): Promise<CopyChoice | undefined> {
  return ctx.ui.custom<CopyChoice | undefined>((tui, theme, _keybindings, done) => {
    let selectedIndex = 0;
    const list = new SelectList(
      choices.map((choice, index) => ({ value: String(index), label: choice.label, description: choice.hint })),
      5,
      {
        selectedPrefix: (text) => theme.fg("accent", text),
        selectedText: (text) => theme.bold(theme.fg("text", text)),
        description: (text) => theme.fg("accent", text),
        scrollInfo: (text) => theme.fg("dim", text),
        noMatch: (text) => theme.fg("muted", text),
      },
    );
    list.onSelect = (item) => done(choices[Number(item.value)]!);
    list.onCancel = () => done(undefined);
    const highlight = highlightCopySource(tui, theme, messageTimestamp, renderer, () => choices[selectedIndex]!.target);
    list.onSelectionChange = (item) => {
      selectedIndex = Number(item.value);
      highlight.refresh();
    };
    return {
      dispose: highlight.dispose,
      render(width: number): string[] {
        const title = ` Copy · ${selectedIndex + 1}/${choices.length} `;
        const border = (text: string) => theme.fg("borderMuted", text);
        return [
          truncateToWidth(
            border("──") +
              theme.bold(theme.fg("accent", title)) +
              border("─".repeat(Math.max(0, width - 2 - visibleWidth(title)))),
            width,
          ),
          ...list.render(width),
          border("─".repeat(width)),
        ];
      },
      invalidate() {
        list.invalidate();
      },
      handleInput(data: string) {
        list.handleInput(data);
        tui.requestRender();
      },
    };
  });
}

function findLastResponse(ctx: ExtensionContext): { text: string; timestamp: number; chunks: string[] } | undefined {
  for (const entry of ctx.sessionManager.getBranch().reverse()) {
    if (entry.type !== "message" || entry.message.role !== "assistant") continue;
    const chunks = entry.message.content.filter((content) => content.type === "text").map((content) => content.text);
    const text = chunks.join("");
    if (text.trim())
      return { text, timestamp: entry.message.timestamp, chunks: chunks.filter((chunk) => chunk.trim()) };
  }
  return undefined;
}
