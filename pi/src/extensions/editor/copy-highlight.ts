import { AssistantMessageComponent, type Theme } from "@earendil-works/pi-coding-agent";
import { Container, parseColor, type Component, type Markdown, type Token, type TUI } from "@earendil-works/pi-tui";

const COPY_BACKGROUNDS = {
  dark: parseColor("#403454"),
  light: parseColor("#e8def8"),
};

// Pi exposes the component tree but not message/block decoration. Keep its private
// rendering fields here; every overridden method belongs to this picker invocation.
interface AssistantRendering {
  lastMessage?: { timestamp: number };
  markdownSlots: { kind: string; markdown: Markdown }[];
}

interface MarkdownRendering {
  renderToken(token: Token, ...args: unknown[]): string[];
}

// Zero-width markers survive Markdown wrapping and are removed before terminal output.
const BLOCK_END = "\x1b]777;pi-copy;end\x07";
const BLOCK_MARKER = /\x1b\]777;pi-copy;(\d+|end)\x07/gu;

/** Highlight the original reply while the picker is open; return its disposal callback. */
export function highlightCopySource(
  tui: TUI,
  theme: Theme,
  messageTimestamp: number,
  codeBlocks: string[],
  selectedIndex: () => number,
): () => void {
  const source = findAssistant(tui.children, messageTimestamp);
  if (!source) return () => {};

  // Pi reuses rendered blocks with identical source. Resolve their individual
  // occurrences after layout so selecting one does not highlight its duplicates.
  const groups = new Map<string, number[]>();
  codeBlocks.forEach((text, index) => {
    const key = text.replace(/\t/gu, "   ");
    const group = groups.get(key) ?? [];
    group.push(index + 1);
    groups.set(key, group);
  });
  const choicesByGroup = new Map([...groups.values()].map((choices) => [choices[0]!, choices]));
  const occurrences = new Map<number, number>();
  const restoreMarkdown: (() => void)[] = [];
  const slots = (source as unknown as AssistantRendering).markdownSlots;

  for (const { kind, markdown } of slots) {
    if (kind !== "text") continue;
    const internals = markdown as unknown as MarkdownRendering;
    const renderToken = internals.renderToken;
    const render = markdown.render;
    internals.renderToken = function (token, ...args) {
      const lines = renderToken.call(this, token, ...args);
      if (token.type !== "code") return lines;
      const group = groups.get(token.text)?.[0];
      if (group === undefined) return lines;
      const marked = [...lines];
      marked[0] = `\x1b]777;pi-copy;${group}\x07${marked[0]}`;
      const last = marked.length - (marked.at(-1) === "" ? 2 : 1);
      marked[last] += BLOCK_END;
      return marked;
    };
    markdown.render = function (width) {
      let activeChoice: number | undefined;
      return render.call(this, width).map((line) => {
        let highlight = selectedIndex() === 0 || activeChoice === selectedIndex();
        const clean = line.replace(BLOCK_MARKER, (_marker, group: string) => {
          if (group === "end") {
            activeChoice = undefined;
          } else {
            const id = Number(group);
            const occurrence = occurrences.get(id) ?? 0;
            occurrences.set(id, occurrence + 1);
            activeChoice = choicesByGroup.get(id)![occurrence];
            highlight ||= activeChoice === selectedIndex();
          }
          return "";
        });
        return highlight ? theme.style(clean, { bg: COPY_BACKGROUNDS[theme.appearance] }) : clean;
      });
    };
    markdown.invalidate();
    restoreMarkdown.push(() => {
      internals.renderToken = renderToken;
      markdown.render = render;
      markdown.invalidate();
    });
  }

  const renderSource = source.render;
  source.render = function (width) {
    occurrences.clear();
    return renderSource.call(this, width);
  };
  return () => {
    source.render = renderSource;
    for (const restore of restoreMarkdown) restore();
    tui.requestRender();
  };
}

function findAssistant(children: Component[], timestamp: number): AssistantMessageComponent | undefined {
  for (let index = children.length - 1; index >= 0; index -= 1) {
    const child = children[index]!;
    if (child instanceof AssistantMessageComponent) {
      if ((child as unknown as AssistantRendering).lastMessage?.timestamp === timestamp) return child;
    } else if (child instanceof Container) {
      const found = findAssistant(child.children, timestamp);
      if (found) return found;
    }
  }
  return undefined;
}
