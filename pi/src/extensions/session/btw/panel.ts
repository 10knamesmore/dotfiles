import { fillPanelRows, type BottomPanel } from "@dotfiles/pi-ui/bottom-panel";
import type { PromptEditorApi, ReplyBodyComponent } from "@dotfiles/pi-ui/prompt-editor";
import { getSelectListTheme, UserMessageComponent, type Theme } from "@earendil-works/pi-coding-agent";
import {
  Editor, type KeybindingsManager, matchesKey, mixColors, replaceControlCharacters, ScrollView, stripTerminalSequences,
  Text, truncateToWidth, type Component, type TUI, type TuiMouseEvent,
} from "@earendil-works/pi-tui";

export interface SideTurn {
  readonly question: string;
  readonly askedAt: number;
  answer: string;
  outcome: "pending" | "answered" | "incomplete";
  notice?: string;
}

interface Conversation {
  readonly turns: readonly SideTurn[];
  readonly outputPad: number;
  readonly draft: string;
  answering(): boolean;
  submit(text: string): void;
  cancel(): void;
  dismiss(draft: string): void;
}

interface TurnView {
  question: UserMessageComponent;
  reply: ReplyBodyComponent;
  notice: Text;
}

function displayText(text: string): string {
  return stripTerminalSequences(text).replace(/\r\n/g, "\n").split("\n").map(replaceControlCharacters).join("\n");
}

const gutterWidth = (width: number) => width > 2 ? 2 : 0;
const promptMarkers = /^(?:\x1b\]133;[ABC](?:\x07|\x1b\\))*/;

export function createPanel(
  tui: TUI,
  theme: Theme,
  keys: KeybindingsManager,
  createReplyBody: PromptEditorApi["createReplyBody"],
  conversation: Conversation,
): BottomPanel {
  const redraw = () => tui.requestRender();
  const views = new Map<SideTurn, TurnView>();
  const replyRegions: { body: ReplyBodyComponent; top: number; height: number }[] = [];
  let selected: ReplyBodyComponent | undefined;
  let editorRow = 0;

  const editor = new Editor(tui, { borderColor: (s) => theme.fg("borderMuted", s), selectList: getSelectListTheme() });
  editor.setText(conversation.draft);
  function send(text: string): boolean {
    if (conversation.answering() || !text.trim()) return false;
    scroll.scrollToEnd();
    conversation.submit(text);
    return true;
  }
  editor.onSubmit = send;

  const transcript: Component = {
    render(width) {
      const lines: string[] = [];
      replyRegions.length = 0;
      for (const turn of conversation.turns) {
        let view = views.get(turn);
        if (!view) {
          view = {
            question: new UserMessageComponent(displayText(turn.question), undefined, conversation.outputPad),
            reply: createReplyBody({ requestRender: redraw, sendFollowUp: send }),
            notice: new Text("", conversation.outputPad, 0),
          };
          views.set(turn, view);
        }
        if (lines.length) lines.push("");
        lines.push(...view.question.render(width));
        if (turn.answer || turn.notice) lines.push("");
        if (turn.answer) {
          view.reply.setText(turn.answer, turn.outcome === "pending");
          const body = view.reply.render(width);
          replyRegions.push({ body: view.reply, top: lines.length, height: body.length });
          lines.push(...body);
        }
        if (turn.notice) {
          view.notice.setText(theme.fg("warning", displayText(turn.notice)));
          lines.push(...view.notice.render(width));
        }
      }
      return lines;
    },
    invalidate() {
      for (const view of views.values()) {
        view.question.invalidate();
        view.reply.invalidate();
        view.notice.invalidate();
      }
    },
  };
  const scroll = new ScrollView(transcript, { follow: "end", overscroll: "contain" });

  return {
    capturesViewportInput: true,
    get focused() { return editor.focused; },
    set focused(value: boolean) { editor.focused = value; },
    handleInput(data) {
      if (matchesKey(data, "escape")) conversation.dismiss(editor.getExpandedText());
      else if (matchesKey(data, "ctrl+c")) conversation.cancel();
      else if (selected?.handleInput(data)) return;
      else if (keys.matches(data, "tui.editor.pageUp")) scroll.scrollBy(-scroll.viewportHeight);
      else if (keys.matches(data, "tui.editor.pageDown")) scroll.scrollBy(scroll.viewportHeight);
      else {
        editor.disableSubmit = conversation.answering();
        editor.handleInput(data);
      }
    },
    handleMouse(event: TuiMouseEvent) {
      const gutter = gutterWidth(event.width);
      const x = event.x - gutter;
      const width = event.width - gutter;
      if (x >= 0 && event.y >= 1 && event.y < editorRow) {
        const y = event.y - 1 + scroll.scrollTop;
        const region = replyRegions.find(({ top, height }) => y >= top && y < top + height);
        if (region) {
          const result = region.body.handleMouse({ ...event, x, width, y: y - region.top, height: region.height });
          if (result?.handled) {
            if (event.type === "click" || event.type === "press") selected = region.body;
            return { handled: true, render: result.render };
          }
        }
      }
      if (event.type === "wheel" && event.wheelDelta) {
        scroll.scrollBy(event.wheelDelta);
        return { handled: true };
      }
      if (x >= 0 && event.type === "click" && event.y >= editorRow) {
        selected = undefined;
        editor.handleMouse({ ...event, x, width, y: event.y - editorRow });
        return { handled: true };
      }
      return undefined;
    },
    render(width, panelRows) {
      const gutter = gutterWidth(width);
      const inner = width - gutter;
      const lines = scroll.render(inner);
      const input = editor.render(inner);
      const title = conversation.answering() ? "btw · Answering…" : "btw";
      const showTitle = panelRows > input.length;
      const titleRows = showTitle ? 1 : 0;
      const height = Math.max(0, panelRows - input.length - titleRows);
      scroll.updateLayout(lines.length, height, redraw);
      editorRow = titleRows + height;
      const rail = gutter
        ? theme.style("▎", { fg: mixColors(theme.colors.borderMuted, theme.colors.accent, 0.2) }) + " "
        : "";
      return [
        ...(showTitle ? [theme.fg("accent", truncateToWidth(title, inner))] : []),
        ...fillPanelRows(lines.slice(scroll.scrollTop, scroll.scrollTop + height), height),
        ...input,
      ].map((line) => line.replace(promptMarkers, (markers) => markers + rail));
    },
    invalidate() {
      scroll.invalidate();
      editor.invalidate();
    },
    dispose() {
      for (const view of views.values()) view.reply.dispose();
      views.clear();
    },
  };
}
