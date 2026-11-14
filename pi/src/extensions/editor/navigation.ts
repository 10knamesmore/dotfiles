/** Keyboard-selectable work below the input editor or in the footer to its right. */
export interface InputNavigation {
  selectNext(): boolean;
  selectPrevious(): void;
  hasSelection(): boolean;
  openSelection(): boolean;
  clearSelection(): void;
}

/**
 * Chain the below-editor agent selection into a final footer selection.
 *
 * Contributors keep their own rendering and selected state. At the last agent,
 * `selectNext()` returns false without clearing it, allowing this coordinator
 * to either move into the footer or retain that final agent at the boundary.
 */
export function chainInputNavigation(
  agents: () => InputNavigation | undefined,
  footer: () => InputNavigation | undefined,
): InputNavigation {
  return {
    selectNext() {
      const agent = agents();
      if (agent?.hasSelection()) {
        if (agent.selectNext()) return true;
        if (footer()?.selectNext()) {
          agent.clearSelection();
          return true;
        }
        return true;
      }
      const status = footer();
      if (status?.hasSelection()) return status.selectNext() || true;
      if (agent?.selectNext()) return true;
      return status?.selectNext() ?? false;
    },
    selectPrevious() {
      const agent = agents();
      if (agent?.hasSelection()) {
        agent.selectPrevious();
        return;
      }
      const status = footer();
      if (!status?.hasSelection()) return;
      status.clearSelection();
      agent?.selectPrevious();
    },
    hasSelection: () => agents()?.hasSelection() === true || footer()?.hasSelection() === true,
    openSelection() {
      const agent = agents();
      if (agent?.hasSelection()) return agent.openSelection();
      const status = footer();
      return status?.hasSelection() === true && status.openSelection();
    },
    clearSelection() {
      agents()?.clearSelection();
      footer()?.clearSelection();
    },
  };
}
