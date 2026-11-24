import type { CustomEditor, ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import { matchesKey } from "@earendil-works/pi-tui";

export function registerDraftStash(pi: ExtensionAPI) {
  const drafts: string[] = [];

  function restoreDraft(ui: ExtensionContext["ui"]): void {
    const draft = drafts.at(-1);
    if (draft === undefined) {
      ui.notify("No stashed drafts", "info");
      return;
    }
    if (ui.getEditorText() !== "") {
      ui.notify("Empty the editor or stash the current draft with Ctrl+S first", "warning");
      return;
    }
    ui.setEditorText(draft);
    drafts.pop();
    ui.notify(`Draft restored (${drafts.length} remaining)`, "info");
  }

  pi.registerCommand("stash", {
    description: "Restore the latest draft (Ctrl+S to stash)",
    handler: async (args, ctx) => {
      if (ctx.mode !== "tui") return;
      if (args !== "") {
        ctx.ui.notify("Usage: /stash", "warning");
        return;
      }
      restoreDraft(ctx.ui);
    },
  });

  return function installDraftStashShortcut(editor: CustomEditor, ui: ExtensionContext["ui"]): void {
    const onExtensionShortcut = editor.onExtensionShortcut;
    editor.onExtensionShortcut = (data) => {
      if (!matchesKey(data, "ctrl+s")) return onExtensionShortcut?.(data) ?? false;

      const draft = ui.getEditorText();
      if (draft === "") {
        restoreDraft(ui);
      } else {
        drafts.push(draft);
        ui.setEditorText("");
        ui.notify(`Draft stashed (${drafts.length} saved)`, "info");
      }
      return true;
    };
  };
}
