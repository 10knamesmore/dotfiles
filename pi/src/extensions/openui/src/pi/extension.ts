// pi extension: render_widget (OpenUI Lang -> interactive terminal widget) and the component gallery.
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { Text, truncateToWidth } from "@earendil-works/pi-tui";
import { Type, type Static } from "typebox";
import { compile, LIMITS, reference, type Compiled, type El } from "../lang/library.ts";
import { markdownImages } from "../lang/md.ts";
import { LANG_SAMPLES } from "../lang/samples.ts";
import { getCapabilities } from "@earendil-works/pi-tui";
import { loadImage } from "./images.ts";
import { syncWidths } from "./widths.ts";
import { active, confirmDelivery, entryOf, handleKey, hooks, OpenUIWidget, resetAll } from "./widget.ts";

const compiled = new Map<string, Compiled>();
const widgetParameters = Type.Object({
  source: Type.Optional(
    Type.String({ description: "OpenUI Lang source: root = Stack([...]) first, one statement per line" }),
  ),
  reference: Type.Optional(Type.Boolean({ description: "Return the component reference instead of drawing" })),
});
type WidgetDetails = { kind: "reference" } | { kind: "widget"; source: string };
let compiledChars = 0;
/** A source is the key, so the cache is bounded by size as well as count: 64 sources at the 100 KB limit would be 6 MB. */
const CACHE_CHARS = 2_000_000;
export function compileCached(source: string): Compiled {
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
export interface Picture {
  src: string;
  line: number;
  via: "Image" | "Markdown picture";
}

/** Every picture in a compiled tree, wherever it was written. */
export function picturesIn(el: El | undefined, out: Picture[] = []): Picture[] {
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

/** Problems the model can fix: the picture must exist and be a format a terminal can show. */
export function checkImages(root: El | undefined): string[] {
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

  pi.registerTool<typeof widgetParameters, WidgetDetails>({
    name: "render_widget",
    label: "render_widget",
    description:
      "Draw an interactive widget in the terminal: dashboards, tables, charts (bar, line, area, pie, radar), diagrams (DAG, ER, infrastructure map, sequence), maps, tabs, buttons and choices. Pass OpenUI Lang as `source`. Call with reference=true once to get the component signatures. A Button click sends its text back to you as the user's next message. Choice answers are staged and arrive together as one message when the user presses Send answers.",
    promptSnippet: "Draw interactive terminal tables, charts and diagrams from OpenUI Lang",
    promptGuidelines: [
      "Before the first render_widget call in a session, call render_widget with reference=true to read the component signatures. Never guess a signature.",
      "If render_widget returns errors, fix exactly the errors listed and call it again with the full corrected source.",
    ],
    parameters: widgetParameters,
    renderShell: "self",
    async execute(_id: string, params: Static<typeof widgetParameters>) {
      if (params.reference || params.source === undefined)
        return { content: [{ type: "text" as const, text: reference() }], details: { kind: "reference" } };
      const c = compileCached(params.source);
      if (!c.ok)
        throw new Error(
          `The widget was not drawn: ${c.errors.length} error(s). Fix these and call render_widget again with the full corrected source.\n${c.errors.map((e) => `- ${e}`).join("\n")}`,
        );
      const imgErrs = checkImages(c.root);
      if (imgErrs.length)
        throw new Error(
          `The widget was not drawn: ${imgErrs.length} error(s). Fix these and call render_widget again with the full corrected source.\n${imgErrs.map((e) => `- ${e}`).join("\n")}`,
        );
      const hasImage = picturesIn(c.root).length > 0;
      const noPics = hasImage && getCapabilities().images !== "kitty";
      const note = [
        ...c.warnings,
        ...(noPics
          ? ["this terminal cannot show pictures, so each Image is drawn as a labelled frame with its file details"]
          : []),
      ];
      const noteText = note.length ? ` Notes: ${note.join("; ")}.` : "";
      return {
        content: [{ type: "text" as const, text: `Widget drawn in the terminal.${noteText}` }],
        details: { kind: "widget", source: params.source },
      };
    },
    renderCall(args: any, theme: any) {
      const n = typeof args?.source === "string" ? args.source.split("\n").length : 0;
      const label = args?.reference ? "reference" : n ? `${n} lines of OpenUI` : "…";
      return new Text(theme.fg("toolTitle", theme.bold("render_widget")) + theme.fg("dim", ` ${label}`), 0, 0);
    },
    renderResult(result: any, _opts: any, theme: any, ctx: any) {
      if (ctx.isError) {
        // the model gets the whole list; show a person the same thing
        const lines = String(result?.content?.[0]?.text ?? "failed")
          .split("\n")
          .slice(0, 9);
        return new Text(lines.map((l) => theme.fg("error", l)).join("\n"), 0, 0);
      }
      if (result?.details?.kind !== "widget") return new Text(theme.fg("dim", "component reference loaded"), 0, 0);
      const c = compileCached(String(result.details.source ?? ""));
      if (!c.ok || !c.root) return new Text(theme.fg("error", "widget could not be restored"), 0, 0);
      return new OpenUIWidget(ctx.toolCallId, c.root, theme, "live", result.details.source);
    },
  });

  pi.registerMessageRenderer("openui-widget", (message: any, _o: any, theme: any) => {
    const c = compileCached(String(message.details?.source ?? ""));
    if (!c.ok || !c.root) return undefined;
    return new OpenUIWidget(
      String(message.details.id),
      c.root,
      theme,
      message.details.mode === "live" ? "live" : "echo",
      message.details.source,
    );
  });

  const draw = (source: string, title: string, ctx: any) => {
    const c = compileCached(source);
    if (!c.ok) return ctx.ui.notify(`${title}: ${c.errors.slice(0, 3).join(" | ")}`, "error");
    pi.sendMessage({
      customType: "openui-widget",
      content: `[widget: ${title}]`,
      display: true,
      details: { id: `w-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`, source, mode: "echo" },
    });
  };

  pi.registerCommand("openui-gallery", {
    description:
      "Draw one example per OpenUI component (or a key prefix: dashboard, table, tabs, choice, line, pie, radar, image, dag, erd, infra, sequence, geo ...). Clicks echo locally.",
    handler: async (args: string, ctx: any) => {
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

  let unsub: (() => void) | undefined;
  pi.on("session_start", async (_e: any, ctx: any) => {
    if (!ctx.hasUI) return;
    unsub?.();
    hooks.queued = () => {
      try {
        return !!ctx.hasPendingMessages();
      } catch {
        return false; // a context from a replaced session
      }
    };
    unsub = ctx.ui.onTerminalInput((data: string) => (handleKey(data) ? { consume: true } : undefined));
    // the note editor is pi's own multi-line editor: Enter saves, Shift+Enter adds a line, Esc cancels
    hooks.prompt = (title: string, initial: string) => ctx.ui.editor(title, initial);
    hooks.toEditor = (text: string) => ctx.ui.pasteToEditor(text);
    ctx.ui.setWidget("openui-hint", (tui: any, theme: any) => {
      hooks.requestRender = () => tui.requestRender();
      return {
        render: (w: number) => {
          if (!active.id) return [];
          const e = entryOf(active.id);
          const keys = e?.state.noteMode
            ? "notes: click or Enter writes a note  arrows/Tab move  Shift+Enter picks the parent  Esc leaves"
            : e?.state.focus
              ? "arrows move  Tab next  Enter activate  Esc close/leave"
              : "Tab or arrows select  Enter activate  Esc leave";
          return [truncateToWidth(theme.fg("accent", " openui ") + theme.fg("dim", keys), w)];
        },
        invalidate() {},
      };
    });
  });
  pi.on("session_shutdown", async () => {
    unsub?.();
    unsub = undefined;
    resetAll(); // a pi process lives for days across /new and resume: each session's widgets must not pile up
  });
}
