// Pictures inside markdown text. One definition of "this line is a picture", shared by the renderer (which draws it), the
// validator (which checks the file exists) and the limit on pictures per widget, so they cannot disagree.
//
//   ![alt text](path/to/file.png)
//   ![alt text](path/to/file.png "a caption")
//   ![alt text](<a path with spaces.png>)
//
// A picture is a line of its own. In the middle of a sentence the syntax stays text: a terminal cannot put a bitmap inside a
// line of characters. Inside a fenced code block or a quote it stays text too.

export interface MdImage {
  alt: string;
  src: string;
  caption?: string;
}

// An unquoted path ends at a space or a ")": a path with either goes in <angle brackets>.
const IMAGE_LINE = /^\s*!\[([^\]]*)\]\(\s*(?:<([^>]+)>|([^\s)]+))(?:\s+"([^"]*)")?\s*\)\s*$/;

/** The picture a line describes, or null when it is anything else. */
export function imageLine(line: string): MdImage | null {
  const m = IMAGE_LINE.exec(line);
  if (!m) return null;
  const src = (m[2] ?? m[3] ?? "").trim();
  if (!src) return null;
  return { alt: m[1].trim(), src, ...(m[4] ? { caption: m[4] } : {}) };
}

const FENCE_OPEN = /^\s*```\s*([\w+-]*)\s*$/;
const FENCE_CLOSE = /^\s*```\s*$/;

/** Every picture the renderer will draw for this text, in order. Mirrors the renderer: not in code fences, not in quotes. */
export function markdownImages(text: unknown): MdImage[] {
  const out: MdImage[] = [];
  const lines = String(text ?? "")
    .replace(/\r/g, "")
    .split("\n");
  for (let i = 0; i < lines.length; i++) {
    if (FENCE_OPEN.test(lines[i])) {
      i++;
      while (i < lines.length && !FENCE_CLOSE.test(lines[i])) i++;
      continue;
    }
    if (/^\s*>/.test(lines[i])) continue;
    const im = imageLine(lines[i]);
    if (im) out.push(im);
  }
  return out;
}
