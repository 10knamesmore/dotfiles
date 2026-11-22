import { fileURLToPath } from "node:url";
import { CodeBlockFrame, codeBlockBodyWidth } from "@dotfiles/pi-ui/code-block";
import {
  REPLY_BLOCKS_CONFIGURE,
  type RegisterReplyBlock,
  type ReplyBlockHost,
  type StreamingBlockInput,
  type StreamingReplyBlock,
} from "@dotfiles/pi-ui/prompt-editor";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { renderFrame, renderText, type Frame } from "./frame.ts";
import { startMermaidStream, type MermaidStream } from "./stream.ts";

const MERMAID_SKILL = fileURLToPath(new URL("./skills/mermaid-diagrams/SKILL.md", import.meta.url));

function createMermaidBlock(
  { requestRender, theme, pointer }: ReplyBlockHost,
  running: Set<() => void>,
): StreamingReplyBlock {
  const color = !process.env.NO_COLOR;
  let previous: StreamingBlockInput | undefined;
  let stream: MermaidStream | undefined;
  let written = "";
  let diagram: Frame | undefined;
  let diagnostic: string | undefined;
  let disposed = false;

  const source = () => previous?.source ?? "";
  const renderedText = () => (diagnostic === undefined ? diagram?.lines.join("\n") : undefined);
  const frame = new CodeBlockFrame({
    title: "Mermaid",
    theme,
    pointer,
    requestRender,
    source,
    renderBody(width) {
      const lines =
        diagram && diagnostic === undefined ? renderFrame(diagram, width, color) : renderText(source(), width);
      if (diagnostic !== undefined) {
        const errors = renderText(diagnostic, width);
        lines.push(...(color ? errors.map((line) => theme.fg("error", line)) : errors));
      }
      return { lines, copyText: () => renderedText() ?? source() };
    },
  });
  const redraw = () => {
    frame.invalidate();
    requestRender();
  };
  const stop = () => {
    const cancelled = stream;
    stream = undefined;
    running.delete(stop);
    cancelled?.dispose();
  };

  return {
    update(update) {
      const { source, width, phase } = update;
      if (disposed || (source === previous?.source && width === previous.width && phase === previous.phase)) return;

      // Marked's closed source omits the newline before the closing delimiter.
      const continuesInput = source.startsWith(written) || (phase === "closed" && written === `${source}\n`);
      const restart = width !== previous?.width || !continuesInput || previous.phase === "closed";
      previous = { source, width, phase };
      frame.invalidate();

      if (restart) {
        stop();
        written = "";
        if (!continuesInput) diagram = undefined;
        diagnostic = undefined;
        const next = startMermaidStream(
          codeBlockBodyWidth(width),
          (latest) => {
            if (stream !== next) return;
            diagram = latest;
            redraw();
          },
          (error) => {
            if (stream !== next) return;
            stream = undefined;
            running.delete(stop);
            if (error !== undefined) {
              diagnostic = `mermaid-text: ${error}`;
              redraw();
            }
          },
        );
        stream = next;
        running.add(stop);
      }

      if (!stream) return;
      const end = phase === "closed" ? source.length : source.lastIndexOf("\n") + 1;
      const suffix = source.slice(written.length, end);
      written += suffix;
      if (phase === "closed") stream.finish(suffix);
      else if (suffix) stream.append(suffix);
    },
    render: (width) => frame.render(width),
    handleMouse: (event) => frame.handleMouse(event),
    copyRenderedText: renderedText,
    invalidate: () => frame.invalidate(),
    dispose() {
      disposed = true;
      stop();
      diagram = undefined;
      frame.dispose();
    },
  };
}

export default function (pi: ExtensionAPI) {
  const running = new Set<() => void>();
  let rendersReplyBlocks = false;
  pi.events.on(REPLY_BLOCKS_CONFIGURE, (data) => {
    const register = data as RegisterReplyBlock;
    register("mermaid", {
      createStream: (host) => createMermaidBlock(host, running),
    });
    rendersReplyBlocks = true;
  });
  pi.on("resources_discover", () => (rendersReplyBlocks ? { skillPaths: [MERMAID_SKILL] } : undefined));
  pi.on("before_agent_start", ({ systemPromptOptions }, ctx) => {
    if (ctx.mode !== "tui" || !rendersReplyBlocks) return;
    let guidance =
      "To show a diagram in an assistant reply, write Mermaid source in a top-level fenced `mermaid` code block. Pi renders it inline as a terminal diagram. In YAML frontmatter, use `config.theme` to select a Mermaid theme and `config.themeVariables` to customize colors.";
    if (systemPromptOptions.skills.some(({ name }) => name === "mermaid-diagrams")) {
      guidance += " Use the mermaid-diagrams skill to choose a supported chart and look up its syntax and examples.";
    }
    systemPromptOptions.sections.mermaid = guidance;
  });
  pi.on("session_shutdown", () => {
    for (const stop of running) stop();
  });
}
