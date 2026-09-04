/** Keyboard bridge from the main editor to the active subagent widget. */

import { CustomEditor, type KeybindingsManager } from "@earendil-works/pi-coding-agent";
import type { EditorTheme, TUI } from "@earendil-works/pi-tui";
import type { ActiveRunTarget } from "./status-widget.js";

export interface AgentInputNavigation {
  selectNext(): boolean;
  selectPrevious(): boolean;
  hasSelection(): boolean;
  takeSelection(): ActiveRunTarget | undefined;
  clearSelection(): void;
  openSelection(target: ActiveRunTarget): void;
}

/** Editor behavior for selecting an active subagent without changing input text. */
export class AgentInputEditor extends CustomEditor {
  private readonly inputKeybindings: KeybindingsManager;

  constructor(
    tui: TUI,
    theme: EditorTheme,
    keybindings: KeybindingsManager,
    private readonly navigation: AgentInputNavigation,
  ) {
    super(tui, theme, keybindings);
    this.inputKeybindings = keybindings;
  }

  override handleInput(data: string): void {
    if (this.isShowingAutocomplete()) {
      super.handleInput(data);
      return;
    }

    const selected = this.navigation.hasSelection();
    if (this.inputKeybindings.matches(data, "tui.editor.cursorDown")
      && (selected || this.isOnLastInputLine())
      && this.navigation.selectNext()) {
      return;
    }
    if (selected && this.inputKeybindings.matches(data, "tui.editor.cursorUp") && this.navigation.selectPrevious()) {
      return;
    }
    if (selected) {
      if (this.inputKeybindings.matches(data, "tui.input.submit")) {
        const target = this.navigation.takeSelection();
        if (target) {
          this.navigation.openSelection(target);
          return;
        }
      }
      if (this.inputKeybindings.matches(data, "app.interrupt")) {
        this.navigation.clearSelection();
        return;
      }
      this.navigation.clearSelection();
    }

    super.handleInput(data);
  }

  private isOnLastInputLine(): boolean {
    const cursor = this.getCursor();
    return cursor.line >= this.getLines().length - 1;
  }
}

/** Create the editor factory used by the session's main prompt editor. */
export function createAgentInputEditorFactory(navigation: AgentInputNavigation): AgentInputEditorFactory {
  return (tui, theme, keybindings) => new AgentInputEditor(tui, theme, keybindings, navigation);
}

export type AgentInputEditorFactory = (tui: TUI, theme: EditorTheme, keybindings: KeybindingsManager) => AgentInputEditor;

