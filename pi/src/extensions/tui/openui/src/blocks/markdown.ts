import { textWidth } from "../diagram/canvas.ts";
import { imageLine } from "../lang/md.ts";
import { codeBlock } from "./code.ts";
import { image } from "./core.ts";
import { tableBlock } from "./table.ts";
import type { Block, Ctx } from "./types.ts";
import { cell, fromRows, vstack, wrapRuns, textCells, type Run, padRow } from "./util.ts";

export function inline(src: string, base = ""): Run[] {
  const runs: Run[] = [];
  const re = /(\*\*|__)(.+?)\1|(\*|_)([^*_\s][^*_]*?)\3|`([^`]+)`|\[([^\]]+)\]\(([^)\s]+)\)|~~(.+?)~~/;
  let rest = src;
  while (rest) {
    const m = re.exec(rest);
    if (!m) {
      runs.push({ t: rest, role: base });
      break;
    }
    if (m.index > 0) runs.push({ t: rest.slice(0, m.index), role: base });
    if (m[2] !== undefined) runs.push({ t: m[2], role: "md.bold" });
    else if (m[4] !== undefined) runs.push({ t: m[4], role: "md.italic" });
    else if (m[5] !== undefined) runs.push({ t: m[5], role: "md.code" });
    else if (m[6] !== undefined) runs.push({ t: m[6], role: "md.link" }, { t: ` (${m[7]})`, role: "md.url" });
    else if (m[8] !== undefined) runs.push({ t: m[8], role: "md.strike" });
    rest = rest.slice(m.index + m[0].length);
  }
  return runs;
}

const isSep = (l: string) => /^\s*\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)*\|?\s*$/.test(l) && l.includes("-");
const cells = (l: string) =>
  l
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((c) => c.trim());

/**
 * Markdown as a block. With a render context, a line that is only `![alt](path "caption")` becomes a picture, drawn by the same
 * code as the Image component (so pi-tui sizes and encodes it, and a terminal without graphics gets the labelled frame). Without
 * one (a quote, a tooltip panel) that syntax stays text. `path` makes the picture's target ids unique within the widget.
 */
export function markdownBlock(src: string, width: number, ctx?: Ctx, path = ""): Block {
  const lines = String(src ?? "")
    .replace(/\r/g, "")
    .split("\n");
  const blocks: Block[] = [];
  let i = 0;
  let pictures = 0;
  const para: string[] = [];
  const flush = () => {
    if (!para.length) return;
    blocks.push(fromRows(width, wrapRuns(inline(para.join(" ")), width)));
    para.length = 0;
  };
  while (i < lines.length) {
    const ln = lines[i];
    const fence = /^\s*```\s*([\w+-]*)\s*$/.exec(ln);
    if (fence) {
      flush();
      const body: string[] = [];
      i++;
      while (i < lines.length && !/^\s*```\s*$/.test(lines[i])) body.push(lines[i++]);
      i++;
      blocks.push(codeBlock(body.join("\n"), fence[1] || undefined, undefined, width, null, false));
      continue;
    }
    const pic = ctx ? imageLine(ln) : null;
    if (pic && ctx) {
      flush();
      const el = { type: "Image", props: { src: pic.src, alt: pic.alt || undefined, caption: pic.caption }, line: 0 };
      blocks.push(image(el, width, `${path}.m${pictures++}`, ctx).block);
      i++;
      continue;
    }
    const h = /^(#{1,6})\s+(.*?)\s*#*\s*$/.exec(ln);
    if (h) {
      flush();
      const lvl = Math.min(3, h[1].length);
      blocks.push(fromRows(width, wrapRuns(inline(h[2], `md.h${lvl}`), width)));
      i++;
      continue;
    }
    if (/^\s*([-*_])(\s*\1){2,}\s*$/.test(ln)) {
      flush();
      blocks.push(fromRows(width, [Array.from({ length: width }, () => cell("─", "md.hr"))]));
      i++;
      continue;
    }
    if (/^\s*>/.test(ln)) {
      flush();
      const q: string[] = [];
      while (i < lines.length && /^\s*>/.test(lines[i])) q.push(lines[i++].replace(/^\s*>\s?/, ""));
      const inner = markdownBlock(q.join("\n"), Math.max(4, width - 2));
      blocks.push(
        fromRows(
          width,
          inner.rows.map((r) => [
            cell("▎", "md.quoteBar"),
            cell(" "),
            ...r.map((c) => (c.role ? c : cell(c.ch, "md.quote"))),
          ]),
        ),
      );
      continue;
    }
    if (ln.includes("|") && i + 1 < lines.length && isSep(lines[i + 1])) {
      flush();
      const head = cells(ln);
      i += 2;
      const body: string[][] = [];
      while (i < lines.length && lines[i].includes("|") && lines[i].trim()) body.push(cells(lines[i++]));
      blocks.push(tableBlock(head, body, width).block);
      continue;
    }
    const li = /^(\s*)([-*+]|\d+[.)])\s+(.*)$/.exec(ln);
    if (li) {
      flush();
      const items: Block[] = [];
      while (i < lines.length) {
        const m = /^(\s*)([-*+]|\d+[.)])\s+(.*)$/.exec(lines[i]);
        if (!m) break;
        const indent = Math.min(6, Math.floor(m[1].replace(/\t/g, "  ").length / 2) * 2);
        const bullet = /\d/.test(m[2]) ? m[2] : "•";
        const bw = textWidth(bullet) + 1;
        const body = wrapRuns(inline(m[3]), Math.max(4, width - indent - bw));
        const rows = body.map((r, k) => [
          ...Array.from({ length: indent }, () => cell(" ")),
          ...(k === 0 ? [...textCells(bullet, "md.bullet"), cell(" ")] : Array.from({ length: bw }, () => cell(" "))),
          ...r,
        ]);
        items.push(fromRows(width, rows));
        i++;
      }
      blocks.push(vstack(items, 0, width));
      continue;
    }
    if (!ln.trim()) {
      flush();
      i++;
      continue;
    }
    para.push(ln.trim());
    i++;
  }
  flush();
  const out = vstack(blocks, 1, width);
  return { ...out, rows: out.rows.map((r) => padRow(r, width)) };
}
