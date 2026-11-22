import { AssistantMessageComponent, type Theme } from "@earendil-works/pi-coding-agent";
import { Container, parseColor, type TUI } from "@earendil-works/pi-tui";
import { ReplyBody } from "./reply-body.js";

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
  selectedTarget: () => CopyTarget | undefined,
) {
  const reply = findAssistant(tui, messageTimestamp);
  const paint = (line: string) => theme.style(line, { bg: COPY_BACKGROUNDS[theme.appearance] });
  let highlightedBody: ReplyBody | undefined;
  let restoreReply = () => {};
  if (reply) {
    const { parent, source } = reply;
    const wrapper = new (class extends Container {
      override render(width: number): readonly string[] {
        const target = selectedTarget();
        const bodies = replyBodies(source);
        const body = target && bodies[target.chunk];
        if (highlightedBody !== body) highlightedBody?.setHighlight(undefined);
        highlightedBody = body;
        if (target) body?.setHighlight({ ...target, paint });
        if (!target) for (const chunk of bodies) chunk.setPaint(paint);
        try {
          return super.render(width);
        } finally {
          if (!target) for (const chunk of bodies) chunk.setPaint(undefined);
        }
      }
    })();
    wrapper.addChild(source);
    parent.children[parent.children.indexOf(source)] = wrapper;
    restoreReply = () => {
      const index = parent.children.indexOf(wrapper);
      if (index !== -1) parent.children[index] = source;
    };
  }

  return {
    copyRenderedBlock(target: CopyTarget): string | undefined {
      return reply && replyBodies(reply.source)[target.chunk]?.copyRenderedBlock(target);
    },
    refresh() {
      tui.requestRender();
    },
    dispose() {
      restoreReply();
      highlightedBody?.setHighlight(undefined);
      tui.requestRender();
    },
  };
}

function replyBodies(parent: Container): ReplyBody[] {
  return parent.children.flatMap((child) => {
    if (child instanceof ReplyBody) return [child];
    if (child instanceof Container) return replyBodies(child);
    return [];
  });
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
