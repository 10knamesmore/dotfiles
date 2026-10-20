import type { MessageRenderer } from "@earendil-works/pi-coding-agent";
import { Container, MouseRegion, type Component } from "@earendil-works/pi-tui";

/** Click anywhere in a message to toggle it; the global expansion action resets that choice. */
export function createExpandableMessageRenderer<T>(
  renderContent: (...args: Parameters<MessageRenderer<T>>) => Component,
): MessageRenderer<T> {
  const expansionByMessage = new WeakMap<object, { allExpanded: boolean; expanded: boolean }>();
  return (message, options, theme) => {
    let expansion = expansionByMessage.get(message);
    if (!expansion || expansion.allExpanded !== options.expanded) {
      expansion = { allExpanded: options.expanded, expanded: options.expanded };
      expansionByMessage.set(message, expansion);
    }
    const state = expansion;
    const content = new Container();
    const rebuild = () => {
      content.clear();
      content.addChild(renderContent(message, { ...options, expanded: state.expanded }, theme));
    };
    rebuild();
    return new MouseRegion(content, (event) => {
      if (event.type !== "click" || event.button !== "left") return undefined;
      state.expanded = !state.expanded;
      rebuild();
      return { handled: true };
    });
  };
}
