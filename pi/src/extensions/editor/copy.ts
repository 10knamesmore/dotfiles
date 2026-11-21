import { copyToClipboard, type CustomEditor, type ExtensionContext } from "@earendil-works/pi-coding-agent";
import { SelectList, stripTerminalSequences, truncateToWidth, visibleWidth } from "@earendil-works/pi-tui";
import { highlightCopySource, type CopyTarget } from "./copy-highlight.js";
import { replyCodeBlocks } from "./reply-markdown.js";

interface CopyChoice {
  label: string;
  hint: string;
  text: string;
  target?: CopyTarget;
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
  const choice = await selectCopyChoice(ctx, lastResponse);
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
  { text: response, timestamp, chunks }: LastResponse,
): Promise<CopyChoice | undefined> {
  return ctx.ui.custom<CopyChoice | undefined>((tui, theme, _keybindings, done) => {
    let selectedIndex = 0;
    const choices: CopyChoice[] = [{ label: "Entire response", hint: lineCount(response), text: response }];
    const highlight = highlightCopySource(tui, theme, timestamp, () => choices[selectedIndex]!.target);
    chunks.forEach((text, chunk) => {
      for (const block of replyCodeBlocks(text.trim())) {
        const target = { chunk, key: block.key, occurrence: block.occurrence };
        const content = highlight.copyRenderedBlock(target) ?? block.text;
        const language = stripTerminalSequences(block.language || "text").replace(/\s+/gu, " ");
        choices.push({
          label: stripTerminalSequences(content).replace(/\s+/gu, " ").trim(),
          hint: `${language} · ${lineCount(content)}`,
          text: content,
          target,
        });
      }
    });
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

function lineCount(text: string): string {
  const count = text.replace(/\n$/u, "").split("\n").length;
  return `${count} ${count === 1 ? "line" : "lines"}`;
}

interface LastResponse {
  text: string;
  timestamp: number;
  chunks: string[];
}

function findLastResponse(ctx: ExtensionContext): LastResponse | undefined {
  for (const entry of ctx.sessionManager.getBranch().reverse()) {
    if (entry.type !== "message" || entry.message.role !== "assistant") continue;
    const chunks = entry.message.content.filter((content) => content.type === "text").map((content) => content.text);
    const text = chunks.join("");
    if (text.trim())
      return { text, timestamp: entry.message.timestamp, chunks: chunks.filter((chunk) => chunk.trim()) };
  }
  return undefined;
}
