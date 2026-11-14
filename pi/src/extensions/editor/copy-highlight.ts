import { AssistantMessageComponent, type Theme } from "@earendil-works/pi-coding-agent";
import { Container, Markdown, parseColor, type Component, type TUI } from "@earendil-works/pi-tui";
import type { ReplyMarkdown } from "./reply-markdown.js";

const COPY_BACKGROUNDS = {
  dark: parseColor("#403454"),
  light: parseColor("#e8def8"),
};

export interface CopyTarget {
  chunk: number;
  key: string;
  occurrence: number;
}

/** undefined selects the entire reply; a target identifies one occurrence in one text chunk. */
export function highlightCopySource(
  tui: TUI,
  theme: Theme,
  messageTimestamp: number,
  renderer: ReplyMarkdown,
  selectedTarget: () => CopyTarget | undefined,
): { refresh: () => void; dispose: () => void } {
  const reply = findAssistant(tui, messageTimestamp);
  const chunks: { parent: Container; markdown: Markdown; wrapper: Component; position: { index: number } }[] = [];
  const paint = (line: string) => theme.style(line, { bg: COPY_BACKGROUNDS[theme.appearance] });
  let restoreReply = () => {};
  let nextTextChunk = 0;

  const wrapText = (parent: Container) => {
    parent.children.forEach((child, index) => {
      const existing = chunks.find((chunk) => chunk.markdown === child || chunk.wrapper === child);
      if (existing) {
        existing.position.index = nextTextChunk++;
        parent.children[index] = existing.wrapper;
        return;
      }
      if (child instanceof Markdown) {
        const position = { index: nextTextChunk++ };
        const wrapper: Component = {
          render(width) {
            const target = selectedTarget();
            return renderer.withCodeHighlight(
              target?.chunk === position.index ? { ...target, paint } : undefined,
              () => {
                const lines = child.render(width);
                return target === undefined ? lines.map(paint) : lines;
              },
            );
          },
          invalidate: () => child.invalidate(),
        };
        chunks.push({ parent, markdown: child, wrapper, position });
        parent.children[index] = wrapper;
        child.invalidate();
      } else if (child instanceof Container) {
        wrapText(child);
      }
    });
  };
  if (reply) {
    const { parent, source } = reply;
    const wrapper: Component = {
      render(width) {
        // Expanding thinking rebuilds the reply's children. Compose the body wrappers before each draw.
        // Thinking itself is inside a MouseRegion, not these body Markdown children.
        nextTextChunk = 0;
        wrapText(source);
        return source.render(width);
      },
      invalidate: () => source.invalidate(),
      handleMouse: (event) => source.handleMouse(event),
    };
    parent.children[parent.children.indexOf(source)] = wrapper;
    restoreReply = () => {
      const index = parent.children.indexOf(wrapper);
      if (index !== -1) parent.children[index] = source;
    };
  }

  return {
    refresh() {
      for (const { markdown } of chunks) markdown.invalidate();
      tui.requestRender();
    },
    dispose() {
      restoreReply();
      for (const { parent, markdown, wrapper } of chunks) {
        const index = parent.children.indexOf(wrapper);
        if (index !== -1) parent.children[index] = markdown;
        markdown.invalidate();
      }
      tui.requestRender();
    },
  };
}

function findAssistant(
  parent: Pick<Container, "children">,
  timestamp: number,
): { parent: Pick<Container, "children">; source: AssistantMessageComponent } | undefined {
  for (let index = parent.children.length - 1; index >= 0; index -= 1) {
    const child = parent.children[index]!;
    if (child instanceof AssistantMessageComponent) {
      // Pi has no public message-to-component lookup. This field is only read for identity.
      const message = (child as unknown as { readonly lastMessage?: { timestamp: number } }).lastMessage;
      if (message?.timestamp === timestamp) return { parent, source: child };
    } else if (child instanceof Container) {
      const found = findAssistant(child, timestamp);
      if (found) return found;
    }
  }
  return undefined;
}
