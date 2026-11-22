import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import { openBottomPanel } from "@dotfiles/pi-ui/bottom-panel";
import { PROMPT_EDITOR_CONFIGURE, type PromptEditorApi } from "@dotfiles/pi-ui/prompt-editor";
import { registerSideRequests } from "./request.js";
import { createPanel, type SideTurn } from "./panel.js";

interface Thread {
  turns: SideTurn[];
  draft: string;
  request?: AbortController;
  unread: boolean;
}

interface OpenPanel {
  close?: () => void;
  redraw?: () => void;
}

function newThread(): Thread {
  return { turns: [], draft: "", unread: false };
}

export default function registerBtw(pi: ExtensionAPI): void {
  const request = registerSideRequests(pi);
  let thread = newThread();
  let panel: OpenPanel | undefined;
  let createReplyBody: PromptEditorApi["createReplyBody"] | undefined;
  pi.events.on(PROMPT_EDITOR_CONFIGURE, (api) => {
    createReplyBody = (api as PromptEditorApi).createReplyBody;
  });

  function updateFooter(ctx: ExtensionContext): void {
    const status = thread.request ? "btw · Answering…" : thread.unread ? "btw · New reply" : undefined;
    ctx.ui.setStatus("btw", panel ? undefined : status);
  }

  function reset(_event: unknown, ctx: ExtensionContext): void {
    thread.request?.abort();
    thread = newThread();
    panel?.close?.();
    panel = undefined;
    updateFooter(ctx);
  }

  pi.on("session_start", reset);
  pi.on("session_tree", reset);
  pi.on("session_shutdown", reset);

  async function ask(ctx: ExtensionContext, question: string): Promise<void> {
    const owner = thread;
    if (owner.request || !question.trim()) return;
    const controller = new AbortController();
    owner.request = controller;
    const history = owner.turns.filter((turn) => turn.outcome === "answered");
    const turn: SideTurn = { question, askedAt: Date.now(), answer: "", outcome: "pending" };
    owner.turns.push(turn);
    panel?.redraw?.();
    try {
      turn.answer = await request(ctx, question, history, controller.signal, (text) => {
        turn.answer = text;
        if (owner === thread) panel?.redraw?.();
      });
      controller.signal.throwIfAborted();
      turn.outcome = "answered";
    } catch (error) {
      turn.outcome = "incomplete";
      turn.notice = controller.signal.aborted ? "Cancelled" : error instanceof Error ? error.message : String(error);
    } finally {
      owner.request = undefined;
      if (owner === thread) {
        owner.unread = panel === undefined;
        panel?.redraw?.();
        updateFooter(ctx);
      }
    }
  }

  pi.registerCommand("btw", {
    description: "Ask a side question about the current conversation",
    async handler(args, ctx) {
      if (ctx.mode !== "tui") {
        ctx.ui.notify("/btw requires the interactive terminal.", "info");
        return;
      }
      if (panel) return;
      const renderReply = createReplyBody;
      if (!renderReply) {
        ctx.ui.notify("/btw requires the editor extension to render replies.", "error");
        return;
      }
      const owner = thread;
      const current: OpenPanel = {};
      panel = current;
      owner.unread = false;
      updateFooter(ctx);

      const question = args.trim();
      if (question && owner.request) owner.draft = [owner.draft, question].filter(Boolean).join("\n");
      else if (question) void ask(ctx, question);

      try {
        await openBottomPanel(ctx.ui, (tui, theme, keys, done) => {
          current.close = done;
          current.redraw = () => tui.requestRender();
          return createPanel(tui, theme, keys, renderReply, {
            turns: owner.turns,
            outputPad: pi.getSettings().outputPad ?? 1,
            draft: owner.draft,
            answering: () => owner.request !== undefined,
            submit: (text) => { void ask(ctx, text); },
            cancel: () => owner.request?.abort(),
            dismiss(draft) {
              owner.draft = draft;
              done();
            },
          });
        }, { heightPercent: 40 });
      } finally {
        if (panel === current) {
          panel = undefined;
          updateFooter(ctx);
        }
      }
    },
  });
}
