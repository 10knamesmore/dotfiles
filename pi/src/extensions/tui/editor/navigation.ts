import type { InputNavigation } from "@dotfiles/pi-ui/prompt-editor";

/**
 * Move from the below-editor agents through the available footer statuses.
 *
 * Contributors keep their own rendering and selected state. At the last agent,
 * `selectNext()` returns false without clearing it, allowing this coordinator
 * to either move into the footer or retain that final agent at the boundary.
 */
export function chainInputNavigation(
  agents: () => InputNavigation | undefined,
  footer: InputNavigation[],
): InputNavigation {
  const selectedFooter = () => footer.find((entry) => entry.hasSelection());

  return {
    selectNext() {
      const selected = footer.findIndex((entry) => entry.hasSelection());
      if (selected >= 0) {
        if (footer.slice(selected + 1).some((entry) => entry.selectNext())) footer[selected]!.clearSelection();
        return true;
      }
      const agent = agents();
      if (agent?.selectNext()) return true;
      if (footer.some((entry) => entry.selectNext())) {
        agent?.clearSelection();
        return true;
      }
      return agent?.hasSelection() ?? false;
    },
    selectPrevious() {
      const agent = agents();
      if (agent?.hasSelection()) {
        agent.selectPrevious();
        return;
      }
      const selected = footer.findIndex((entry) => entry.hasSelection());
      if (selected < 0) return;
      footer[selected]!.clearSelection();
      const previous = footer.slice(0, selected).reverse();
      if (!previous.some((entry) => entry.selectNext())) agent?.selectPrevious();
    },
    hasSelection: () => agents()?.hasSelection() === true || selectedFooter() !== undefined,
    openSelection() {
      const agent = agents();
      if (agent?.hasSelection()) return agent.openSelection();
      return selectedFooter()?.openSelection() ?? false;
    },
    setExpanded(expanded) {
      const agent = agents();
      return agent?.hasSelection() === true && (agent.setExpanded?.(expanded) ?? false);
    },
    clearSelection() {
      agents()?.clearSelection();
      for (const entry of footer) entry.clearSelection();
    },
  };
}
