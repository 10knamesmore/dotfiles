// GeoMap: braille land texture + coastline, pins with labels, dashed links. Equirectangular, square dots.
import { LAND } from "../data/land.ts";
import { textWidth, truncateText, type Cell } from "../diagram/canvas.ts";
import type { El } from "../lang/library.ts";
import { Dots } from "./braille.ts";
import type { Anchor } from "./core.ts";
import type { Block, Target } from "./types.ts";
import { cell, fromRows, textCells, vstack } from "./util.ts";

let rings: Float64Array[] | null = null;
function loadRings(): Float64Array[] {
  if (rings) return rings;
  rings = LAND.split("|").map((r) => {
    const pts = r.split(";").map((p) => p.split(",").map(Number));
    const out = new Float64Array(pts.length * 2);
    let x = 0;
    let y = 0;
    pts.forEach(([a, b], i) => {
      if (i === 0) {
        x = a;
        y = b;
      } else {
        x += a;
        y += b;
      }
      out[i * 2] = x;
      out[i * 2 + 1] = y;
    });
    return out;
  });
  return rings;
}

/** boolean land mask for a view, W x H dots */
export function landMask(
  lonMin: number,
  lonMax: number,
  latMin: number,
  latMax: number,
  W: number,
  H: number,
): Uint8Array {
  const mask = new Uint8Array(W * H);
  const sx = W / (lonMax - lonMin);
  const sy = H / (latMax - latMin);
  const edges: Array<[number, number, number, number]> = [];
  for (const ring of loadRings()) {
    const n = ring.length / 2;
    for (const off of [-3600, 0, 3600]) {
      let minX = Infinity;
      let maxX = -Infinity;
      let minY = Infinity;
      let maxY = -Infinity;
      const px: number[] = [];
      const py: number[] = [];
      for (let i = 0; i < n; i++) {
        const lon = (ring[i * 2] + off) / 10 - 180;
        const lat = 90 - ring[i * 2 + 1] / 10;
        const X = (lon - lonMin) * sx;
        const Y = (latMax - lat) * sy;
        px.push(X);
        py.push(Y);
        minX = Math.min(minX, X);
        maxX = Math.max(maxX, X);
        minY = Math.min(minY, Y);
        maxY = Math.max(maxY, Y);
      }
      if (maxX < 0 || minX > W || maxY < 0 || minY > H) continue;
      for (let i = 0; i < n; i++) {
        const j = (i + 1) % n;
        if (py[i] !== py[j]) edges.push([px[i], py[i], px[j], py[j]]);
      }
    }
  }
  for (let y = 0; y < H; y++) {
    const yc = y + 0.5;
    const xs: number[] = [];
    for (const [x0, y0, x1, y1] of edges) {
      if ((y0 <= yc && y1 > yc) || (y1 <= yc && y0 > yc)) xs.push(x0 + ((yc - y0) / (y1 - y0)) * (x1 - x0));
    }
    xs.sort((a, b) => a - b);
    for (let k = 0; k + 1 < xs.length; k += 2) {
      const a = Math.max(0, Math.ceil(xs[k] - 0.5));
      const b = Math.min(W - 1, Math.floor(xs[k + 1] - 0.5));
      for (let x = a; x <= b; x++) mask[y * W + x] = 1;
    }
  }
  return mask;
}

const TONE: Record<string, string> = { good: "good", warn: "warn", bad: "bad", info: "info", neutral: "neutral" };

export function geoMap(el: El, width: number): { block: Block; anchors: Map<string, Anchor> } {
  const pts: El[] = el.props.points ?? [];
  const links: string[][] = el.props.links ?? [];
  const anchors = new Map<string, Anchor>();
  const head = el.props.title
    ? [fromRows(width, [textCells(truncateText(String(el.props.title), width), "chart.title")])]
    : [];
  const cols = Math.max(24, Math.min(width, 84));
  const W = cols * 2;
  const lats = pts.map((p) => p.props.lat as number);
  const lons = pts.map((p) => p.props.lon as number);
  let lonMin = Math.min(...lons);
  let lonMax = Math.max(...lons);
  let latMin = Math.min(...lats);
  let latMax = Math.max(...lats);
  const padL = Math.max(6, (lonMax - lonMin) * 0.2);
  const padA = Math.max(4, (latMax - latMin) * 0.25);
  lonMin -= padL;
  lonMax += padL;
  latMin -= padA;
  latMax += padA;
  if (lonMax - lonMin < 24) {
    const c = (lonMin + lonMax) / 2;
    lonMin = c - 12;
    lonMax = c + 12;
  }
  let sx = W / (lonMax - lonMin);
  let rowsN = Math.ceil(((latMax - latMin) * sx) / 4);
  if (rowsN > 22) {
    sx = (22 * 4) / (latMax - latMin);
    const c = (lonMin + lonMax) / 2;
    lonMin = c - W / sx / 2;
    lonMax = c + W / sx / 2;
    rowsN = 22;
  }
  if (rowsN < 9) {
    const need = (9 * 4) / sx;
    const c = (latMin + latMax) / 2;
    latMin = c - need / 2;
    latMax = c + need / 2;
    rowsN = 9;
  }
  if (latMax > 90) {
    latMin -= latMax - 90;
    latMax = 90;
  }
  if (latMin < -90) {
    latMax += -90 - latMin;
    latMin = -90;
  }
  const H = rowsN * 4;
  const mask = landMask(lonMin, lonMax, latMin, latMax, W, H);
  const d = new Dots(cols, rowsN);
  const at = (x: number, y: number) => (x < 0 || y < 0 || x >= W || y >= H ? 0 : mask[y * W + x]);
  for (let y = 0; y < H; y++)
    for (let x = 0; x < W; x++) {
      if (!mask[y * W + x]) continue;
      const edge = !at(x - 1, y) || !at(x + 1, y) || !at(x, y - 1) || !at(x, y + 1);
      if (edge) d.set(x, y, 1);
      else if ((x + y) % 2 === 0) d.set(x, y, 0);
    }
  const proj = (lat: number, lon: number): [number, number] => [
    ((lon - lonMin) / (lonMax - lonMin)) * W,
    ((latMax - lat) / (latMax - latMin)) * H,
  ];
  const byLabel = new Map(pts.map((p) => [String(p.props.label), p]));
  for (const [a, b] of links) {
    const pa = byLabel.get(a);
    const pb = byLabel.get(b);
    if (!pa || !pb) continue;
    const [x0, y0] = proj(pa.props.lat, pa.props.lon);
    const [x1, y1] = proj(pb.props.lat, pb.props.lon);
    const steps = Math.ceil(Math.hypot(x1 - x0, y1 - y0));
    for (let s = 0; s <= steps; s++)
      if (Math.floor(s / 3) % 2 === 0)
        d.set(x0 + ((x1 - x0) * s) / (steps || 1), y0 + ((y1 - y0) * s) / (steps || 1), 2);
  }
  const grid = d.rows2cells((o) => (o === 0 ? "map.land" : o === 1 ? "map.coast" : o === 2 ? "map.link" : ""));
  const rows: Cell[][] = grid.map((r) => r.slice());
  const targets: Target[] = [];
  const taken = new Set<string>();
  const free = (x: number, y: number, w: number) => {
    if (y < 0 || y >= rowsN || x < 0 || x + w > cols) return false;
    for (let k = 0; k < w; k++) if (taken.has(`${x + k},${y}`)) return false;
    return true;
  };
  const unlabeled: string[] = [];
  pts.forEach((p, i) => {
    const [dx, dy] = proj(p.props.lat, p.props.lon);
    const cx = Math.max(0, Math.min(cols - 1, Math.floor(dx / 2)));
    const cy = Math.max(0, Math.min(rowsN - 1, Math.floor(dy / 4)));
    const role = `map.pin.${TONE[p.props.tone ?? "info"] ?? "info"}`;
    rows[cy][cx] = cell("●", role);
    taken.add(`${cx},${cy}`);
    const label = truncateText(String(p.props.label), 18);
    const lw = textWidth(label);
    const tries: Array<[number, number]> = [
      [cx + 2, cy],
      [cx - 1 - lw, cy],
      [cx - Math.floor(lw / 2), cy - 1],
      [cx - Math.floor(lw / 2), cy + 1],
      [cx + 2, cy - 1],
      [cx + 2, cy + 1],
    ];
    let placed: [number, number] | null = null;
    for (const [lx, ly] of tries)
      if (free(lx, ly, lw)) {
        placed = [lx, ly];
        break;
      }
    let rect = { x: cx, y: cy, w: 1, h: 1 };
    if (placed) {
      const [lx, ly] = placed;
      textCells(label, "map.label").forEach((c, k) => {
        rows[ly][lx + k] = c;
        taken.add(`${lx + k},${ly}`);
      });
      const x0 = Math.min(cx, lx);
      rect = { x: x0, y: Math.min(cy, ly), w: Math.max(cx + 1, lx + lw) - x0, h: Math.abs(ly - cy) + 1 };
    } else unlabeled.push(String(p.props.label));
    const id = `geo:${i}`;
    targets.push({
      id,
      kind: "node",
      rect,
      label: `${p.props.label}  ${Number(p.props.lat).toFixed(2)}, ${Number(p.props.lon).toFixed(2)}`,
    });
    anchors.set(String(p.props.label), { rect, targetId: id });
  });
  for (const [a, b] of links) {
    const pa = byLabel.get(a);
    const pb = byLabel.get(b);
    if (!pa || !pb) continue;
    const [x0, y0] = proj(pa.props.lat, pa.props.lon);
    const [x1, y1] = proj(pb.props.lat, pb.props.lon);
    anchors.set(`${a}>${b}`, {
      rect: {
        x: Math.max(0, Math.min(cols - 3, Math.floor((x0 + x1) / 4) - 1)),
        y: Math.max(0, Math.min(rowsN - 1, Math.floor((y0 + y1) / 8))),
        w: 3,
        h: 1,
      },
    });
  }
  const map = fromRows(width, rows, targets);
  const notes: Block[] = [];
  if (unlabeled.length)
    notes.push(fromRows(width, [textCells(truncateText(`also: ${unlabeled.join(", ")}`, width), "text.muted")]));
  const out = vstack([...head, map, ...notes], 0, width);
  for (const [, a] of anchors) a.rect = { ...a.rect, y: a.rect.y + head.length };
  return { block: out, anchors };
}
