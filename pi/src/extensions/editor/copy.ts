import { copyToClipboard, type CustomEditor, type ExtensionContext } from "@earendil-works/pi-coding-agent";
import { SelectList, stripTerminalSequences, truncateToWidth, visibleWidth } from "@earendil-works/pi-tui";
import { marked } from "marked";
import { highlightCopySource } from "./copy-highlight.js";

interface CopyChoice {
  label: string;
  hint: string;
  text: string;
}

/** Install after Pi wires onSubmit: built-in /copy runs before extension commands. */
export function installCopyCommand(editor: CustomEditor, ctx: ExtensionContext): void {
  const submit = editor.onSubmit;
  editor.onSubmit = async (text) => {
    if (text.trim() !== "/copy") {
      submit?.(text);
      return;
    }
    editor.setText("");
    await copyLastResponse(ctx);
  };
}

async function copyLastResponse(ctx: ExtensionContext): Promise<void> {
  const lastResponse = findLastResponse(ctx);
  if (lastResponse === undefined) {
    ctx.ui.notify("No assistant response to copy yet.", "info");
    return;
  }
  const { text: response, timestamp } = lastResponse;

  const responseLineCount = response.replace(/\n$/u, "").split("\n").length;
  const choices: CopyChoice[] = [
    {
      label: "Entire response",
      hint: `${responseLineCount} ${responseLineCount === 1 ? "line" : "lines"}`,
      text: response,
    },
  ];
  marked.walkTokens(marked.lexer(response), (token) => {
    if (token.type !== "code") return;
    const lineCount = token.text.replace(/\n$/u, "").split("\n").length;
    const language = stripTerminalSequences(token.lang || "text").replace(/\s+/gu, " ");
    choices.push({
      label: stripTerminalSequences(token.text).replace(/\s+/gu, " ").trim(),
      hint: `${language} · ${lineCount} ${lineCount === 1 ? "line" : "lines"}`,
      text: token.text,
    });
  });

  const choice = await selectCopyChoice(ctx, choices, timestamp);
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
    list.onSelectionChange = (item) => {
      selectedIndex = Number(item.value);
    };
    list.onSelect = (item) => done(choices[Number(item.value)]!);
    list.onCancel = () => done(undefined);
    const clearHighlight = highlightCopySource(
      tui,
      theme,
      messageTimestamp,
      choices.slice(1).map((choice) => choice.text),
      () => selectedIndex,
    );
    return {
      dispose: clearHighlight,
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

function findLastResponse(ctx: ExtensionContext): { text: string; timestamp: number } | undefined {
  for (const entry of ctx.sessionManager.getBranch().reverse()) {
    if (entry.type !== "message" || entry.message.role !== "assistant") continue;
    const text = entry.message.content
      .filter((content) => content.type === "text")
      .map((content) => content.text)
      .join("");
    if (text.trim()) return { text, timestamp: entry.message.timestamp };
  }
  return undefined;
}
