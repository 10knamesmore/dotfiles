import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import { PROMPT_EDITOR_CONFIGURE, type EditorStatus, type PromptEditorApi } from "./api.js";
import { PromptEditor } from "./component.js";
import { installCopyCommand } from "./copy.js";
import { chainInputNavigation, type InputNavigation } from "./navigation.js";
import { createReplyMarkdown } from "./reply-markdown.js";
import { createReplyCodeButtons } from "./reply-code-buttons.js";

type EditorFactory = NonNullable<ReturnType<ExtensionContext["ui"]["getEditorComponent"]>>;

/** Sole owner of the prompt Editor: other extensions contribute behavior, not replacements. */
export default function registerPromptEditor(pi: ExtensionAPI): void {
  let currentContext: ExtensionContext | undefined;
  let installedFactory: EditorFactory | undefined;
  const replyMarkdown = createReplyMarkdown();
  pi.registerMarkdownTransformer(replyMarkdown.transform);

  pi.on("session_start", (_event, ctx) => {
    if (ctx.mode !== "tui") return;
    currentContext = ctx;
    let navigation: InputNavigation | undefined;
    let footerNavigation: InputNavigation | undefined;
    let readStatus: (() => EditorStatus) | undefined;
    let readFastRequested: (() => boolean) | undefined;
    let readWorkflowUsage: (() => string | undefined) | undefined;
    const chainedNavigation = chainInputNavigation(
      () => navigation,
      () => footerNavigation,
    );
    const api: PromptEditorApi = {
      useAgentNavigation: (contribution) => {
        navigation = contribution;
      },
      useFooterNavigation: (contribution) => {
        footerNavigation = contribution;
      },
      useStatus: (contribution) => {
        readStatus = contribution;
      },
      useFastRequested: (contribution) => {
        readFastRequested = contribution;
      },
      useWorkflowUsage: (contribution) => {
        readWorkflowUsage = contribution;
      },
    };
    pi.events.emit(PROMPT_EDITOR_CONFIGURE, api);
    let editor!: PromptEditor;
    installedFactory = (tui, theme, keybindings) =>
      (editor = new PromptEditor(
        tui,
        theme,
        keybindings,
        () => chainedNavigation,
        () => readStatus?.(),
        () => currentContext,
        () => readFastRequested?.() ?? false,
        () => readWorkflowUsage?.(),
      ));
    ctx.ui.setEditorComponent(installedFactory);
    installCopyCommand(editor, ctx, replyMarkdown);
    ctx.ui.setWidget("reply-code-buttons", (tui) => createReplyCodeButtons(tui, ctx));
  });

  pi.on("model_select", (_event, ctx) => {
    currentContext = ctx;
  });
  pi.on("thinking_level_select", (_event, ctx) => {
    currentContext = ctx;
  });
  pi.on("session_shutdown", (_event, ctx) => {
    ctx.ui.setWidget("reply-code-buttons", undefined);
    if (installedFactory && ctx.ui.getEditorComponent() === installedFactory) ctx.ui.setEditorComponent(undefined);
    installedFactory = undefined;
    currentContext = undefined;
  });
}
