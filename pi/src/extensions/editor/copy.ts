import { copyToClipboard, highlightCode, type CustomEditor, type ExtensionContext } from "@earendil-works/pi-coding-agent";
import { SelectList, stripTerminalSequences, truncateToWidth } from "@earendil-works/pi-tui";
import { marked } from "marked";

interface CopyChoice {
  label: string;
  hint: string;
  language: string;
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
  const response = lastResponseText(ctx);
  if (response === undefined) {
    ctx.ui.notify("No assistant response to copy yet.", "info");
    return;
  }

  const responseLineCount = response.replace(/\n$/u, "").split("\n").length;
  const choices: CopyChoice[] = [{
    label: "Entire response",
    hint: `${responseLineCount} ${responseLineCount === 1 ? "line" : "lines"}`,
    language: "markdown",
    text: response,
  }];
  marked.walkTokens(marked.lexer(response), (token) => {
    if (token.type !== "code") return;
    const lineCount = token.text.replace(/\n$/u, "").split("\n").length;
    const language = stripTerminalSequences(token.lang || "text").replace(/\s+/gu, " ");
    choices.push({
      label: stripTerminalSequences(token.text).replace(/\s+/gu, " ").trim(),
      hint: `${language} · ${lineCount} ${lineCount === 1 ? "line" : "lines"}`,
      language: language.trim().split(/\s+/u)[0]!,
      text: token.text,
    });
  });

  const choice = await selectCopyChoice(ctx, choices);
  if (choice === undefined) return;
  try {
    await copyToClipboard(choice.text);
    ctx.ui.notify("Copied.", "info");
  } catch {
    ctx.ui.notify("Could not write to the clipboard. Check clipboard access and try again.", "error");
  }
}

function selectCopyChoice(ctx: ExtensionContext, choices: CopyChoice[]): Promise<CopyChoice | undefined> {
  return ctx.ui.custom<CopyChoice | undefined>((tui, theme, _keybindings, done) => {
    let selectedIndex = 0;
    const highlightedPreviews = new Map<number, string[]>();
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
    return {
      render(width: number): string[] {
        const contentWidth = Math.max(1, width - 4);
        const selected = choices[selectedIndex]!;
        const hint = selectedIndex === 0 ? `Entire response · ${selected.hint}` : selected.hint;
        let preview = highlightedPreviews.get(selectedIndex);
        if (!preview) {
          preview = highlightCode(stripTerminalSequences(selected.text), selected.language);
          highlightedPreviews.set(selectedIndex, preview);
        }
        return [
          truncateToWidth(theme.bold(theme.fg("accent", `Copy · ${selectedIndex + 1}/${choices.length}`)), width),
          "",
          ...list.render(width),
          "",
          truncateToWidth(theme.fg("accent", `── ${hint} ${"─".repeat(width)}`), width),
          ...preview
            .slice(0, 8)
            .map((line) => truncateToWidth(`  ${truncateToWidth(line, contentWidth)}`, width)),
          ...(preview.length > 8 ? [theme.fg("dim", "  …")] : []),
          "",
          truncateToWidth(theme.fg("accent", "↑↓ select · Enter copy · Esc cancel"), width),
        ];
      },
      invalidate() {
        highlightedPreviews.clear();
        list.invalidate();
      },
      handleInput(data: string) {
        list.handleInput(data);
        tui.requestRender();
      },
    };
  });
}

function lastResponseText(ctx: ExtensionContext): string | undefined {
  for (const entry of ctx.sessionManager.getBranch().reverse()) {
    if (entry.type !== "message" || entry.message.role !== "assistant") continue;
    const text = entry.message.content
      .filter((content) => content.type === "text")
      .map((content) => content.text)
      .join("");
    if (text.trim()) return text;
  }
  return undefined;
}
