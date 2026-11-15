// Pictures for the Image component. pi-tui's own Image does the sizing and the Kitty encoding (so cell maths, image ids
// and the alt-screen cropping metadata are pi's), this file loads the bytes and keeps the placements.
import { getCapabilities, getImageDimensions, Image } from "@earendil-works/pi-tui";
import { readFileSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { basename, isAbsolute, resolve } from "node:path";
import type { ImageService } from "../blocks/types.ts";

export const MAX_IMAGE_BYTES = 12 * 1024 * 1024;

export interface LoadedImage {
  base64: string;
  mime: string;
  name: string;
  bytes: number;
  dims: { widthPx: number; heightPx: number } | null;
}

function sniff(buf: Buffer): string | null {
  if (buf.length >= 8 && buf[0] === 0x89 && buf[1] === 0x50 && buf[2] === 0x4e && buf[3] === 0x47) return "image/png";
  if (buf.length >= 3 && buf[0] === 0xff && buf[1] === 0xd8 && buf[2] === 0xff) return "image/jpeg";
  if (buf.length >= 6 && buf.subarray(0, 4).toString("ascii") === "GIF8") return "image/gif";
  if (
    buf.length >= 12 &&
    buf.subarray(0, 4).toString("ascii") === "RIFF" &&
    buf.subarray(8, 12).toString("ascii") === "WEBP"
  )
    return "image/webp";
  return null;
}

const cache = new Map<string, { stamp: string; value: LoadedImage | { error: string } }>();
/**
 * Pictures are kept as base64 (a file of 12 MB is 16 MB of string), so the cache is bounded by size, not by count: 32 large
 * screenshots would otherwise pin half a gigabyte for as long as pi runs.
 */
const CACHE_BUDGET = 48 * 1024 * 1024;
let cacheBytes = 0;
const weigh = (v: LoadedImage | { error: string }) => ("base64" in v ? v.base64.length : 0);
export function imageCacheStats() {
  return { entries: cache.size, bytes: cacheBytes, budget: CACHE_BUDGET };
}
function remember(path: string, stamp: string, value: LoadedImage | { error: string }) {
  const old = cache.get(path);
  if (old) cacheBytes -= weigh(old.value);
  cache.delete(path);
  cache.set(path, { stamp, value });
  cacheBytes += weigh(value);
  // oldest first; the newest picture is always kept, whatever its size
  for (const [k, v] of cache) {
    if (cache.size <= 1 || (cache.size <= 32 && cacheBytes <= CACHE_BUDGET)) break;
    cache.delete(k);
    cacheBytes -= weigh(v.value);
  }
}

/** Read a local file (absolute, ~/ or relative to the working directory) or a base64 data: URI. Never throws. */
export function loadImage(src: string): LoadedImage | { error: string } {
  const s = String(src ?? "").trim();
  if (!s) return { error: "src is empty" };
  if (/^https?:\/\//i.test(s)) return { error: "remote URLs are not fetched; use a local file path or a data: URI" };
  let stamp = "data";
  let buf: Buffer;
  let name = "image";
  if (s.startsWith("data:")) {
    const m = /^data:([^;,]*)(;[^,]*)?,(.*)$/s.exec(s);
    if (!m || !/;base64/i.test(m[2] ?? ""))
      return { error: "a data: URI must be base64, like data:image/png;base64,..." };
    try {
      buf = Buffer.from(m[3], "base64");
    } catch {
      return { error: "the data: URI is not valid base64" };
    }
    name = `inline.${(m[1] || "image/png").split("/")[1] ?? "png"}`;
  } else {
    const path = s.startsWith("~/") ? resolve(homedir(), s.slice(2)) : isAbsolute(s) ? s : resolve(process.cwd(), s);
    let st;
    try {
      st = statSync(path);
    } catch {
      return { error: `file not found: ${path}` };
    }
    if (!st.isFile()) return { error: `not a file: ${path}` };
    if (st.size > MAX_IMAGE_BYTES)
      return {
        error: `file is ${Math.round(st.size / 1024 / 1024)} MB; the limit is ${MAX_IMAGE_BYTES / 1024 / 1024} MB`,
      };
    stamp = `${st.mtimeMs}:${st.size}`;
    const hit = cache.get(path);
    if (hit && hit.stamp === stamp) return hit.value;
    try {
      buf = readFileSync(path);
    } catch (e) {
      return { error: `cannot read ${path}: ${(e as Error).message}` };
    }
    name = basename(path);
    const value = decode(buf, name);
    remember(path, stamp, value);
    return value;
  }
  if (buf.length > MAX_IMAGE_BYTES) return { error: "the data: URI is too large" };
  return decode(buf, name);
}

function decode(buf: Buffer, name: string): LoadedImage | { error: string } {
  const mime = sniff(buf);
  if (!mime) {
    const head = buf.subarray(0, 200).toString("utf8").trimStart();
    if (/^<(\?xml|svg)/i.test(head)) return { error: "SVG cannot be drawn in a terminal; use png, jpg, gif or webp" };
    return { error: "not a png, jpg, gif or webp picture" };
  }
  const base64 = buf.toString("base64");
  return { base64, mime, name, bytes: buf.length, dims: getImageDimensions(base64, mime) };
}

const kb = (n: number) =>
  n >= 1024 * 1024 ? `${(n / 1024 / 1024).toFixed(1)} MB` : `${Math.max(1, Math.round(n / 1024))} KB`;

interface Placement {
  sequence: string;
  cols: number;
  rows: number;
}

function hash(s: string): string {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 16777619);
  return (h >>> 0).toString(36) + s.length.toString(36);
}

export interface PiImageService extends ImageService {
  /** The terminal sequence that draws a placed image (the first line pi-tui's Image returns). */
  sequence(key: string): string | null;
  /** Called after a picture that could not be drawn yet becomes drawable (pi loads its JPEG/GIF/WebP converter lazily). */
  onLate: (() => void) | null;
  /** Forget every placement: the terminal reported another cell size, so the same picture now needs other columns and rows. */
  reset(): void;
}

export function createImageService(): PiImageService {
  const placements = new Map<string, Placement>();
  const pending = new Map<string, number>();
  // a placement holds the picture's whole transmit command: bound them by size too
  let placedChars = 0;
  const svc: PiImageService = {
    onLate: null,
    reset() {
      placements.clear();
      pending.clear();
      placedChars = 0;
    },
    sequence: (key) => placements.get(key)?.sequence ?? null,
    describe(src) {
      const img = loadImage(src);
      if ("error" in img) return img.error;
      const fmt = img.mime.split("/")[1];
      return [img.name, img.dims ? `${img.dims.widthPx}×${img.dims.heightPx}` : "", fmt, kb(img.bytes)]
        .filter(Boolean)
        .join(" · ");
    },
    place(src, maxCols, maxRows) {
      if (getCapabilities().images !== "kitty") return null;
      const img = loadImage(src);
      if ("error" in img) return null;
      const key = `${hash(src)}:${maxCols}x${maxRows}`;
      let p = placements.get(key);
      if (!p) {
        const comp = new Image(
          img.base64,
          img.mime,
          { fallbackColor: (s: string) => s },
          { maxWidthCells: maxCols, maxHeightCells: maxRows, filename: img.name },
          img.dims ?? undefined,
        );
        const lines = comp.render(maxCols + 2);
        const seq = lines[0] ?? "";
        if (!seq.includes("\x1b_G")) {
          // pi converts JPEG, GIF and WebP lazily after startup; look again shortly so the picture replaces the frame
          const tries = pending.get(key) ?? 0;
          if (tries < 8 && img.mime !== "image/png") {
            pending.set(key, tries + 1);
            const t = setTimeout(() => svc.onLate?.(), 350);
            if (typeof (t as any).unref === "function") (t as any).unref();
          }
          return null;
        }
        const c = /\x1b_G[^;]*,c=(\d+)/.exec(seq);
        p = { sequence: seq, cols: c ? Number(c[1]) : maxCols, rows: lines.length };
        placements.set(key, p);
        placedChars += p.sequence.length;
        for (const [k, old] of placements) {
          if (placements.size <= 1 || (placements.size <= 40 && placedChars <= CACHE_BUDGET)) break;
          placements.delete(k);
          placedChars -= old.sequence.length;
        }
      }
      return { key, cols: p.cols, rows: p.rows };
    },
  };
  return svc;
}

export const imageService = createImageService();
