import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { PaddedCodeBlock, type CodeBlockPointer } from "@dotfiles/pi-ui/code-block";
import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import {
  PROMPT_EDITOR_CONFIGURE,
  REPLY_BLOCKS_CONFIGURE,
  type PromptEditorApi,
  type RegisterReplyBlock,
} from "@dotfiles/pi-ui/prompt-editor";
import { compile, LIMITS, type Compiled, type El } from "../lang/library.ts";
import { markdownImages } from "../lang/md.ts";
import { LANG_SAMPLES } from "../lang/samples.ts";
import { loadImage } from "./images.ts";
import { syncWidths } from "./widths.ts";
import { confirmDelivery, hooks, OpenUIWidget, resetAll } from "./widget.ts";

const SHOW_ME_SKILL = fileURLToPath(new URL("../../skills/show-me/SKILL.md", import.meta.url));
const GALLERY_RENDER_HOST = "openui-gallery-render";

interface GalleryDetails {
  id: string;
  source: string;
  mode: "live" | "echo";
}

const compiled = new Map<string, Compiled>();
let compiledChars = 0;
/** A source is the key, so the cache is bounded by size as well as count: 64 sources at the 100 KB limit would be 6 MB. */
const CACHE_CHARS = 2_000_000;
function compileCached(source: string): Compiled {
  if (source.length > LIMITS.source) return compile(source); // rejected at once; never kept
  let c = compiled.get(source);
  if (!c) {
    c = compile(source);
    if (compiled.size >= 64 || compiledChars + source.length > CACHE_CHARS) {
      compiled.clear();
      compiledChars = 0;
    }
    compiled.set(source, c);
    compiledChars += source.length;
  }
  return c;
}

/** A picture the widget will draw: an Image component, or a `![alt](path)` line in a Markdown or an FAQ answer. */
interface Picture {
  src: string;
  line: number;
  via: "Image" | "Markdown picture";
}

/** Every picture in a compiled tree, wherever it was written. */
function picturesIn(el: El | undefined, out: Picture[] = []): Picture[] {
  if (!el || typeof el !== "object") return out;
  if (el.type === "Image") out.push({ src: String(el.props.src ?? ""), line: el.line, via: "Image" });
  const text = el.type === "Markdown" ? el.props.text : el.type === "FaqItem" ? el.props.answer : undefined;
  if (text !== undefined)
    for (const m of markdownImages(text)) out.push({ src: m.src, line: el.line, via: "Markdown picture" });
  for (const v of Object.values(el.props ?? {}))
    for (const x of Array.isArray(v) ? v : [v])
      if (x && typeof x === "object") picturesIn(Array.isArray(x) ? undefined : (x as El), out);
  return out;
}

function checkImages(root: El | undefined): string[] {
  const errs: string[] = [];
  for (const im of picturesIn(root)) {
    const r = loadImage(im.src);
    if ("error" in r)
      errs.push(
        `line ${im.line}: ${im.via} ${im.src.startsWith("data:") ? "data: URI" : `"${im.src.length > 80 ? im.src.slice(0, 77) + "..." : im.src}"`}: ${r.error}`,
      );
  }
  return errs;
}

export default function (pi: ExtensionAPI) {
  syncWidths(); // our grid must agree with this pi about which characters take two cells
  hooks.sendLive = (text) => pi.sendUserMessage(text, { deliverAs: "followUp" });
  hooks.sendEcho = (text) =>
    pi.sendMessage({ customType: "openui-click", content: `[widget click → assistant] ${text}`, display: true });

  let rendersReplyBlocks = false;
  pi.events.on(REPLY_BLOCKS_CONFIGURE, (data) => {
    const register = data as RegisterReplyBlock;
    register("openui", (source, id, host, scope) => {
      const compiled = compileCached(source);
      if (!compiled.ok || !compiled.root) return { errors: compiled.errors };
      const errors = checkImages(compiled.root);
      if (errors.length) return { errors };
      const destination = scope ?? { mode: "live" as const, requestRender: host.requestRender };
      return { component: new OpenUIWidget(id, compiled.root, host, destination, source) };
    });
    rendersReplyBlocks = true;
  });

  pi.on("resources_discover", () => (rendersReplyBlocks ? { skillPaths: [SHOW_ME_SKILL] } : undefined));

  pi.on("before_agent_start", ({ systemPromptOptions }, ctx) => {
    if (ctx.mode !== "tui" || !rendersReplyBlocks) return;
    const guidance = [
      "Pi renders top-level fenced `openui` blocks in assistant replies as interactive diagrams, charts, and tables.",
      "Write OpenUI Lang directly in the block, starting with `root = Stack([...])`.",
    ];
    const skill = systemPromptOptions.skills.find(({ name }) => name === "show-me");
    if (skill) {
      const reference = join(dirname(skill.filePath), "references", "openui.md");
      guidance.push(`Read \`${reference}\` only if you need syntax or component signatures.`);
    }
    systemPromptOptions.sections.openui = guidance.join(" ");
  });

  let galleryUI: ExtensionContext["ui"] | undefined;
  let renderGallery: (() => void) | undefined;
  let galleryPointer: CodeBlockPointer = {};
  pi.events.on(PROMPT_EDITOR_CONFIGURE, (data) => {
    galleryPointer = (data as PromptEditorApi).blockPointer;
  });
  const requestGalleryRender = () => renderGallery?.();
  pi.registerMessageRenderer<GalleryDetails>("openui-widget", (message, options, theme) => {
    const details = message.details;
    if (!details) return undefined;
    const c = compileCached(details.source);
    if (!c.ok || !c.root) return undefined;
    // Pi supplies the TUI to widget factories. Gallery message renderers receive only the theme.
    if (!renderGallery) {
      galleryUI?.setWidget(
        GALLERY_RENDER_HOST,
        (tui) => {
          renderGallery = () => tui.requestRender();
          return {
            render: () => [],
            invalidate() {},
            dispose() {
              renderGallery = undefined;
            },
          };
        },
        { placement: "belowEditor" },
      );
    }
    const widget = new OpenUIWidget(
      details.id,
      c.root,
      { theme, pointer: galleryPointer },
      {
        mode: details.mode === "live" ? "live" : "echo",
        requestRender: requestGalleryRender,
      },
      details.source,
    );
    return new PaddedCodeBlock(widget, options.outputPad);
  });

  const draw = (source: string, title: string, ctx: ExtensionContext) => {
    const c = compileCached(source);
    if (!c.ok) return ctx.ui.notify(`${title}: ${c.errors.slice(0, 3).join(" | ")}`, "error");
    pi.sendMessage<GalleryDetails>({
      customType: "openui-widget",
      content: `[widget: ${title}]`,
      display: true,
      details: { id: `w-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`, source, mode: "echo" },
    });
  };

  pi.registerCommand("openui-gallery", {
    description:
      "Draw one example per OpenUI component (or a key prefix: dashboard, table, tabs, choice, line, pie, radar, image, dag, erd, infra, sequence, geo ...). Clicks echo locally.",
    handler: async (args, ctx) => {
      const which = args.trim() || "all";
      const picked = LANG_SAMPLES.filter((s) => which === "all" || s.key.startsWith(which));
      if (!picked.length)
        return ctx.ui.notify(
          `unknown example "${which}". Try: all, ${LANG_SAMPLES.map((s) => s.key).join(", ")}`,
          "warning",
        );
      for (const s of picked) draw(s.source, s.title, ctx);
    },
  });

  // pi reports every user message it takes, whoever sent it: that is how a widget learns its answers were delivered
  pi.on("message_start", async (event: any) => {
    const m = event?.message;
    if (m?.role !== "user") return;
    const words =
      typeof m.content === "string"
        ? m.content
        : Array.isArray(m.content)
          ? m.content
              .filter((p: any) => p?.type === "text")
              .map((p: any) => p.text)
              .join("")
          : "";
    confirmDelivery(words);
  });

  pi.on("session_start", async (_event, ctx) => {
    if (ctx.mode !== "tui") return;
    galleryUI = ctx.ui;
    hooks.queued = () => {
      try {
        return !!ctx.hasPendingMessages();
      } catch {
        return false; // a context from a replaced session
      }
    };
    // the note editor is pi's own multi-line editor: Enter saves, Shift+Enter adds a line, Esc cancels
    hooks.prompt = (title: string, initial: string) => ctx.ui.editor(title, initial);
    hooks.toEditor = (text: string) => ctx.ui.pasteToEditor(text);
  });
  pi.on("session_shutdown", async () => {
    rendersReplyBlocks = false;
    if (renderGallery) galleryUI?.setWidget(GALLERY_RENDER_HOST, undefined);
    galleryUI = undefined;
    galleryPointer = {};
    resetAll(); // a pi process lives for days across /new and resume: each session's widgets must not pile up
  });
}
