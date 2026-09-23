import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import { PROMPT_EDITOR_CONFIGURE, type EditorStatus, type PromptEditorApi } from "./api.js";
import { PromptEditor } from "./component.js";
import type { AgentInputNavigation } from "./navigation.js";

type EditorFactory = NonNullable<ReturnType<ExtensionContext["ui"]["getEditorComponent"]>>;

/** Sole owner of the prompt Editor: other extensions contribute behavior, not replacements. */
export default function registerPromptEditor(pi: ExtensionAPI): void {
  let currentContext: ExtensionContext | undefined;
  let installedFactory: EditorFactory | undefined;

  pi.on("session_start", (_event, ctx) => {
    if (ctx.mode !== "tui") return;
    currentContext = ctx;
    let navigation: AgentInputNavigation | undefined;
    let readStatus: (() => EditorStatus) | undefined;
    let readTodoStatus: (() => string | undefined) | undefined;
    let readWorkflowUsage: (() => string | undefined) | undefined;
    const api: PromptEditorApi = {
      useAgentNavigation: (contribution) => { navigation = contribution; },
      useStatus: (contribution) => { readStatus = contribution; },
      useTodoStatus: (contribution) => { readTodoStatus = contribution; },
      useWorkflowUsage: (contribution) => { readWorkflowUsage = contribution; },
    };
    pi.events.emit(PROMPT_EDITOR_CONFIGURE, api);
    installedFactory = (tui, theme, keybindings) => new PromptEditor(
      tui, theme, keybindings,
      () => navigation,
      () => readStatus?.(),
      () => currentContext,
      () => readTodoStatus?.(),
      () => readWorkflowUsage?.(),
    );
    ctx.ui.setEditorComponent(installedFactory);
  });

  pi.on("model_select", (_event, ctx) => { currentContext = ctx; });
  pi.on("thinking_level_select", (_event, ctx) => { currentContext = ctx; });
  pi.on("session_shutdown", (_event, ctx) => {
    if (installedFactory && ctx.ui.getEditorComponent() === installedFactory) ctx.ui.setEditorComponent(undefined);
    installedFactory = undefined;
    currentContext = undefined;
  });
}
