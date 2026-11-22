import type { FastMode } from "@dotfiles/pi-config";
import type { CodeBlockPointer } from "@dotfiles/pi-ui/code-block";
import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import {
  PROMPT_EDITOR_CONFIGURE,
  REPLY_BLOCKS_CONFIGURE,
  type EditorStatus,
  type InputNavigation,
  type PromptEditorApi,
  type RegisterReplyBlock,
  type ReplyBlockRegistration,
} from "@dotfiles/pi-ui/prompt-editor";
import { PromptEditor } from "./component.js";
import { installCopyCommand } from "./copy.js";
import { chainInputNavigation } from "./navigation.js";
import { createReplyMarkdown } from "./reply-markdown.js";
import { createReplyBody, createReplyBodies } from "./reply-body.js";

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
    let readFastMode: (() => FastMode | undefined) | undefined;
    let readWorkflowUsage: (() => string | undefined) | undefined;
    const chainedNavigation = chainInputNavigation(
      () => navigation,
      () => footerNavigation,
    );
    const blockRenderers = new Map<string, ReplyBlockRegistration>();
    const registerReplyBlock: RegisterReplyBlock = (language, registration) => {
      blockRenderers.set(language.toLowerCase(), registration);
    };
    pi.events.emit(REPLY_BLOCKS_CONFIGURE, registerReplyBlock);
    let editor!: PromptEditor;
    const pointer: CodeBlockPointer = {};
    const api: PromptEditorApi = {
      blockPointer: pointer,
      useAgentNavigation: (contribution) => {
        navigation = contribution;
      },
      useFooterNavigation: (contribution) => {
        footerNavigation = contribution;
      },
      useStatus: (contribution) => {
        readStatus = contribution;
      },
      useFastMode: (contribution) => {
        readFastMode = contribution;
      },
      useWorkflowUsage: (contribution) => {
        readWorkflowUsage = contribution;
      },
      createReplyBody: (options) =>
        createReplyBody(options, ctx.ui, replyMarkdown, blockRenderers, pointer, pi.getSettings().outputPad ?? 1),
    };
    pi.events.emit(PROMPT_EDITOR_CONFIGURE, api);
    installedFactory = (tui, theme, keybindings) =>
      (editor = new PromptEditor(
        tui,
        theme,
        keybindings,
        pointer,
        () => chainedNavigation,
        () => readStatus?.(),
        () => currentContext,
        () => readFastMode?.(),
        () => readWorkflowUsage?.(),
      ));
    ctx.ui.setEditorComponent(installedFactory);
    installCopyCommand(editor, ctx);
    ctx.ui.setWidget("reply-bodies", (tui) => createReplyBodies(tui, ctx, replyMarkdown, blockRenderers, pointer));
  });

  pi.on("model_select", (_event, ctx) => {
    currentContext = ctx;
  });
  pi.on("thinking_level_select", (_event, ctx) => {
    currentContext = ctx;
  });
  pi.on("session_shutdown", (_event, ctx) => {
    ctx.ui.setWidget("reply-bodies", undefined);
    if (installedFactory && ctx.ui.getEditorComponent() === installedFactory) ctx.ui.setEditorComponent(undefined);
    installedFactory = undefined;
    currentContext = undefined;
  });
}
