// BarChart, LineChart, AreaChart, PieChart, RadarChart.
import { textWidth, truncateText, type Cell, type Rect } from "../diagram/canvas.ts";
import type { El } from "../lang/library.ts";
import { Dots, fmtNum, monotone, niceNum } from "./braille.ts";
import type { Anchor } from "./core.ts";
import type { Block } from "./types.ts";
import { cell, fromRows, padRow, textCells, vstack } from "./util.ts";

const SER = (i: number) => `series.${((i % 8) + 8) % 8}`;
const EIGHTHS = ["", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

/** One legend line. `rects[i]` is where entry i sits on that line (for tooltips on a series). */
function legendRow(names: string[], width: number): { block: Block; rects: Rect[] } {
  const cells: Cell[] = [];
  const rects: Rect[] = [];
  names.forEach((n, i) => {
    if (i) cells.push(cell(" "), cell(" "));
    const x0 = cells.length;
    cells.push(cell("■", SER(i)), cell(" "), ...textCells(truncateText(n, 24), "chart.legend"));
    rects.push({ x: x0, y: 0, w: cells.length - x0, h: 1 });
  });
  return { block: fromRows(width, [cells]), rects };
}

function titleRow(t: string | undefined, width: number): Block[] {
  return t ? [fromRows(width, [textCells(truncateText(t, width), "chart.title")])] : [];
}

// ---------------- BarChart ----------------

export function barChart(el: El, width: number): { block: Block; anchors: Map<string, Anchor> } {
  const labels: string[] = el.props.labels ?? [];
  const values: number[] = (el.props.values ?? []).map((v: number) => (Number.isFinite(v) ? v : 0));
  const unit = String(el.props.unit ?? "");
  const anchors = new Map<string, Anchor>();
  const n = labels.length;
  const max = Math.max(0, ...values);
  const hasNeg = values.some((v) => v < 0);
  const lw = Math.max(1, ...labels.map((l) => textWidth(l)));
  const colW = Math.min(14, Math.max(6, lw + 1));
  const head = titleRow(el.props.title, width);
  if (!n) return { block: vstack(head, 0, width), anchors };
  if (!hasNeg && n * colW <= width - 1) {
    const H = 8;
    const bw = colW - 2;
    const rows: Cell[][] = [];
    const heights = values.map((v) => (max > 0 ? Math.round((v / max) * H * 8) : 0));
    for (let r = H - 1; r >= 0; r--) {
      const row: Cell[] = [];
      values.forEach((_, i) => {
        const inner = heights[i] - r * 8;
        const ch = inner >= 8 ? "█" : inner > 0 ? EIGHTHS[inner] : " ";
        row.push(cell(" "));
        for (let k = 0; k < bw; k++) row.push(cell(ch, ch === " " ? "" : SER(0)));
        row.push(cell(" "));
      });
      rows.push(row);
    }
    const vals: Cell[] = [];
    values.forEach((v, i) => {
      const t = truncateText(fmtNum(v) + unit, colW);
      const pad = Math.floor((colW - textWidth(t)) / 2);
      for (let k = 0; k < pad; k++) vals.push(cell(" "));
      vals.push(...textCells(t, "chart.value"));
      for (let k = 0; k < colW - pad - textWidth(t); k++) vals.push(cell(" "));
      void i;
    });
    const axis = Array.from({ length: n * colW }, () => cell("─", "chart.axis"));
    const labs: Cell[] = [];
    labels.forEach((l) => {
      const t = truncateText(l, colW - 1);
      const pad = Math.floor((colW - textWidth(t)) / 2);
      for (let k = 0; k < pad; k++) labs.push(cell(" "));
      labs.push(...textCells(t, "chart.label"));
      for (let k = 0; k < colW - pad - textWidth(t); k++) labs.push(cell(" "));
    });
    const body = fromRows(n * colW, [vals, ...rows, axis, labs]);
    const off = 1 + head.length; // not used for anchors inside body; anchors are relative to the final block
    labels.forEach((l, i) => anchors.set(l, { rect: { x: i * colW, y: head.length, w: colW, h: body.rows.length } }));
    void off;
    return { block: vstack([...head, body], 0, width), anchors };
  }
  // horizontal bars
  const lab = Math.min(lw, Math.max(6, Math.floor(width * 0.3)));
  const vw = Math.max(...values.map((v) => textWidth(fmtNum(v) + unit)));
  const bwid = Math.max(4, width - lab - vw - 4);
  const lo = Math.min(0, ...values);
  const span = max - lo || 1;
  const rows: Cell[][] = [];
  values.forEach((v, i) => {
    const zero = Math.round((-lo / span) * bwid);
    const end = Math.round(((v - lo) / span) * bwid);
    const from = Math.min(zero, end);
    const to = Math.max(zero, end);
    const t = truncateText(labels[i], lab);
    const row: Cell[] = [
      ...textCells(t, "chart.label"),
      ...Array.from({ length: lab - textWidth(t) + 1 }, () => cell(" ")),
    ];
    for (let x = 0; x < bwid; x++)
      row.push(x >= from && x < to ? cell("█", SER(0)) : x === zero ? cell("│", "chart.axis") : cell(" "));
    row.push(cell(" "), ...textCells(fmtNum(v) + unit, "chart.value"));
    rows.push(row);
    anchors.set(labels[i], { rect: { x: 0, y: head.length + i, w: width, h: 1 } });
  });
  return { block: vstack([...head, fromRows(width, rows)], 0, width), anchors };
}

// ---------------- Line / Area ----------------

export function lineArea(el: El, width: number, stacked: boolean): { block: Block; anchors: Map<string, Anchor> } {
  const anchors = new Map<string, Anchor>();
  const labels: string[] = el.props.labels ?? [];
  const series: El[] = el.props.series ?? [];
  const variant = el.props.variant ?? "natural";
  const n = labels.length;
  const names = series.map((s) => String(s.props.category ?? ""));
  const data = series.map((s) => (s.props.values ?? []).map((v: number) => (Number.isFinite(v) ? v : 0)) as number[]);
  if (!n || !data.length) return { block: fromRows(width, [textCells("(no data)", "text.muted")]), anchors };
  const cum = data.map((d) => d.slice());
  if (stacked) for (let s = 1; s < cum.length; s++) for (let i = 0; i < n; i++) cum[s][i] += cum[s - 1][i];
  const all = (stacked ? cum : data).flat();
  let hi = Math.max(...all);
  let lo = stacked ? 0 : Math.min(...all);
  if (!stacked && lo > 0 && lo / (hi || 1) < 0.6) lo = 0;
  if (hi === lo) hi = lo + 1;
  const step = niceNum((hi - lo) / 4, true);
  hi = Math.ceil(hi / step) * step;
  lo = lo < 0 || stacked ? Math.floor(lo / step) * step : lo === 0 ? 0 : Math.floor(lo / step) * step;
  const axisW = Math.max(textWidth(fmtNum(hi)), textWidth(fmtNum(lo)), 2) + 1;
  const plotW = Math.max(10, width - axisW - 1);
  const rowsN = Math.min(16, Math.max(6, Math.round((el.props.height ?? 200) / 20)));
  const d = new Dots(plotW, rowsN);
  const X = (i: number) => (n === 1 ? (d.w - 1) / 2 : (i / (n - 1)) * (d.w - 1));
  const Y = (v: number) => (d.h - 1) * (1 - (v - lo) / (hi - lo));
  const xs = Array.from({ length: n }, (_, i) => X(i));
  // y(x) of a series in dot space
  const fnFor = (vals: number[]): ((x: number) => number) => {
    const ys = vals.map(Y);
    if (variant === "step") {
      return (x: number) => {
        let i = 0;
        while (i < n - 1 && x >= (xs[i] + xs[i + 1]) / 2) i++;
        return ys[i];
      };
    }
    if (variant === "natural" && n > 2) {
      const f = monotone(xs, ys);
      return (x: number) => Math.min(d.h - 1, Math.max(0, f(x)));
    }
    return (x: number) => {
      if (n === 1) return ys[0];
      let i = 0;
      while (i < n - 2 && x > xs[i + 1]) i++;
      const t = (x - xs[i]) / (xs[i + 1] - xs[i] || 1);
      return ys[i] + (ys[i + 1] - ys[i]) * Math.min(1, Math.max(0, t));
    };
  };
  const fns = (stacked ? cum : data).map(fnFor);
  if (stacked) {
    for (let s = 0; s < fns.length; s++) {
      const lower = s === 0 ? () => Y(lo) : fns[s - 1];
      for (let x = 0; x < d.w; x++) {
        const top = fns[s](x);
        const bot = lower(x);
        for (let y = Math.round(top); y <= Math.round(bot); y++) d.set(x, y, s);
      }
    }
  }
  for (let s = 0; s < fns.length; s++) {
    let py = fns[s](0);
    for (let x = 1; x < d.w; x++) {
      const y = fns[s](x);
      d.line(x - 1, py, x, y, s);
      py = y;
    }
    if (variant === "step" || n <= 12) for (let i = 0; i < n; i++) d.set(xs[i], fns[s](xs[i]), s);
  }
  const body = d.rows2cells((o) => (o >= 0 ? SER(o) : ""));
  const out: Cell[][] = [];
  const tickRows = new Map<number, number>([
    [0, hi],
    [Math.floor((rowsN - 1) / 2), (hi + lo) / 2],
    [rowsN - 1, lo],
  ]);
  body.forEach((row, r) => {
    const t = tickRows.has(r) ? fmtNum(tickRows.get(r)!) : "";
    const lab = Array.from({ length: axisW - textWidth(t) - 1 }, () => cell(" ")).concat(
      textCells(t, "chart.axisLabel"),
    );
    out.push([...lab, cell("┤", "chart.axis"), ...row]);
  });
  const base: Cell[] = [
    ...Array.from({ length: axisW }, () => cell(" ")),
    cell("└", "chart.axis"),
    ...Array.from({ length: plotW }, () => cell("─", "chart.axis")),
  ];
  out.push(base);
  // x labels: first, last, and evenly spaced ones that fit
  const xl: Cell[] = Array.from({ length: axisW + 1 + plotW }, () => cell(" "));
  const placed: Array<[number, number]> = [];
  const put = (i: number, align: "l" | "c" | "r") => {
    const t = truncateText(labels[i], 10);
    const tw = textWidth(t);
    const cx = axisW + 1 + Math.round(X(i) / 2);
    let x0 = align === "l" ? cx : align === "r" ? cx - tw + 1 : cx - Math.floor(tw / 2);
    x0 = Math.max(axisW + 1, Math.min(axisW + 1 + plotW - tw, x0));
    if (placed.some(([a, b]) => x0 < b + 1 && x0 + tw > a - 1)) return;
    placed.push([x0, x0 + tw]);
    textCells(t, "chart.label").forEach((c, k) => (xl[x0 + k] = c));
  };
  put(0, "l");
  put(n - 1, "r");
  const stepI = Math.max(1, Math.ceil(n / Math.max(2, Math.floor(plotW / 12))));
  for (let i = stepI; i < n - 1; i += stepI) put(i, "c");
  out.push(xl);
  const meta: Block[] = [];
  const axisNames = [el.props.yLabel ? `↑ ${el.props.yLabel}` : "", el.props.xLabel ? `→ ${el.props.xLabel}` : ""]
    .filter(Boolean)
    .join("   ");
  if (axisNames) meta.push(fromRows(width, [textCells(axisNames, "text.muted")]));
  const legend = legendRow(names, width);
  const top = meta.length;
  // a tooltip on a category: hover anywhere in its column of the plot; the badge sits on the curve at that point
  const perPoint = n > 1 ? plotW / (n - 1) : plotW;
  const colW = Math.max(1, Math.min(3, Math.floor(perPoint)));
  labels.forEach((l, i) => {
    const cx = axisW + 1 + Math.floor(X(i) / 2); // the cell that holds the dot at dot-column X(i)
    const ys = fns.map((f) => f(xs[i]));
    const yTop = Math.min(...ys);
    const by = top + Math.max(0, Math.min(rowsN - 1, Math.floor(yTop / 4)));
    const x0 = Math.max(axisW + 1, Math.min(axisW + plotW + 1 - colW, cx - Math.floor(colW / 2)));
    if (!anchors.has(String(l)))
      anchors.set(String(l), { rect: { x: x0, y: top, w: colW, h: rowsN }, badge: [cx, by], badgeForce: true });
  });
  names.forEach((nm, i) => {
    const r = legend.rects[i];
    anchors.set(`series:${nm}`, { rect: { x: r.x, y: top + rowsN + 2, w: r.w, h: 1 } });
  });
  return {
    block: vstack(
      [
        ...meta,
        fromRows(
          width,
          out.map((r) => padRow(r, width)),
        ),
        legend.block,
      ],
      0,
      width,
    ),
    anchors,
  };
}

// ---------------- Pie ----------------

export function pieChart(el: El, width: number): { block: Block; anchors: Map<string, Anchor> } {
  const anchors = new Map<string, Anchor>();
  const labels: string[] = el.props.labels ?? [];
  const vals: number[] = (el.props.values ?? []).map((v: number) => Math.max(0, Number.isFinite(v) ? v : 0));
  const total = vals.reduce((a, b) => a + b, 0);
  if (!labels.length || total <= 0) return { block: fromRows(width, [textCells("(no data)", "text.muted")]), anchors };
  const order = labels.map((_, i) => i).sort((a, b) => vals[b] - vals[a] || a - b);
  const frac = order.map((i) => vals[i] / total);
  const cumulative: number[] = [];
  frac.reduce((a, f, k) => ((cumulative[k] = a + f), a + f), 0);
  const semi = el.props.appearance === "semiCircular";
  const donut = el.props.variant === "donut";
  const legendW = Math.min(36, Math.max(...labels.map((l) => textWidth(l))) + 14);
  const stackLegend = width < 22 + legendW + 3;
  // a half disc is only half as tall, so it can be drawn wider (and so smoother) than a full one in the same space
  const D = Math.max(10, Math.min(semi ? 44 : 24, (stackLegend ? width : width - legendW - 3) & ~1));
  const R = D / 2;
  const pxH = semi ? Math.ceil(D / 2) : D;
  const rowsN = Math.ceil(pxH / 2);
  const sliceAt = (x: number, y: number): number | "x" => {
    const dx = x + 0.5 - R;
    const dy = y + 0.5 - R;
    const r2 = dx * dx + dy * dy;
    if (r2 > R * R || (donut && r2 < (R * 0.55) ** 2)) return "x";
    let f = semi
      ? (Math.atan2(dx, -dy) + Math.PI / 2) / Math.PI
      : ((Math.atan2(dx, -dy) + 2 * Math.PI) % (2 * Math.PI)) / (2 * Math.PI);
    if (semi && (f < 0 || f > 1)) return "x";
    f = Math.min(0.999999, Math.max(0, f));
    let k = 0;
    while (k < cumulative.length - 1 && f >= cumulative[k]) k++;
    return k;
  };
  const disc: Cell[][] = [];
  const wedge: Array<Array<[number, number]>> = order.map(() => []);
  for (let r = 0; r < rowsN; r++) {
    const row: Cell[] = [];
    for (let x = 0; x < D; x++) {
      const a = sliceAt(x, r * 2);
      const b = r * 2 + 1 < pxH ? sliceAt(x, r * 2 + 1) : "x";
      if (a !== "x") wedge[a].push([x, r]);
      if (b !== "x" && b !== a) wedge[b].push([x, r]);
      if (a === "x" && b === "x") row.push(cell(" "));
      else if (b === "x") row.push(cell("▀", `pie.${a}.x`));
      else if (a === "x") row.push(cell("▄", `pie.x.${b}`));
      else row.push(cell("▀", `pie.${a}.${b}`));
    }
    disc.push(row);
  }
  const leg: Cell[][] = order.map((i, k) => {
    const pct = `${Math.round((vals[i] / total) * 100)}%`;
    const left = [
      cell("■", `pie.${k}.x`.replace(/^pie\.(\d+)\.x$/, "series.$1")),
      cell(" "),
      ...textCells(truncateText(labels[i], legendW - textWidth(pct) - 8), "chart.label"),
    ];
    const right = textCells(`${fmtNum(vals[i])}  ${pct}`, "chart.value");
    const gap = Math.max(1, legendW - left.length - right.length);
    return [...left, ...Array.from({ length: gap }, () => cell(" ")), ...right];
  });
  const legendW2 = leg[0]?.length ?? legendW;
  if (stackLegend) {
    order.forEach((li, k) =>
      anchors.set(String(labels[li]), { rect: { x: 0, y: disc.length + 1 + k, w: legendW2, h: 1 }, cells: wedge[k] }),
    );
    return { block: vstack([fromRows(width, disc), fromRows(width, leg)], 1, width), anchors };
  }
  const h = Math.max(disc.length, leg.length);
  const rows: Cell[][] = [];
  const top = Math.floor((h - leg.length) / 2);
  const legX = D + 3;
  for (let y = 0; y < h; y++) {
    const dr = disc[y] ?? Array.from({ length: D }, () => cell(" "));
    const lr = leg[y - top] ?? [];
    rows.push([...dr, cell(" "), cell(" "), cell(" "), ...lr]);
  }
  order.forEach((li, k) =>
    anchors.set(String(labels[li]), { rect: { x: legX, y: top + k, w: legendW2, h: 1 }, cells: wedge[k] }),
  );
  return { block: fromRows(width, rows), anchors };
}

// ---------------- Radar ----------------

export function radarChart(el: El, width: number): { block: Block; anchors: Map<string, Anchor> } {
  const anchors = new Map<string, Anchor>();
  const labels: string[] = el.props.labels ?? [];
  const series: El[] = el.props.series ?? [];
  const n = labels.length;
  const names = series.map((s) => String(s.props.category ?? ""));
  const data = series.map(
    (s) => (s.props.values ?? []).map((v: number) => (Number.isFinite(v) ? Math.max(0, v) : 0)) as number[],
  );
  if (n < 3 || !data.length)
    return { block: fromRows(width, [textCells("(a radar chart needs at least 3 axes)", "text.muted")]), anchors };
  const labW = Math.min(14, Math.max(...labels.map((l) => textWidth(l))));
  const plotCols = Math.max(16, Math.min(40, width - 2 * (labW + 2)) & ~1);
  const rowsN = Math.floor(plotCols / 2);
  const d = new Dots(plotCols, rowsN);
  const cx = (d.w - 1) / 2;
  const cy = (d.h - 1) / 2;
  const R = Math.min(cx, cy) - 1;
  const max = niceNum(Math.max(1, ...data.flat()), false);
  const ang = (i: number) => -Math.PI / 2 + (i * 2 * Math.PI) / n;
  const pt = (i: number, f: number): [number, number] => [cx + R * f * Math.cos(ang(i)), cy + R * f * Math.sin(ang(i))];
  for (const ring of [1 / 3, 2 / 3, 1]) {
    for (let i = 0; i < n; i++) {
      const [x0, y0] = pt(i, ring);
      const [x1, y1] = pt((i + 1) % n, ring);
      const steps = Math.ceil(Math.hypot(x1 - x0, y1 - y0));
      for (let s = 0; s <= steps; s += ring === 1 ? 1 : 2)
        d.set(x0 + ((x1 - x0) * s) / (steps || 1), y0 + ((y1 - y0) * s) / (steps || 1), 99);
    }
  }
  for (let i = 0; i < n; i++) {
    const [x1, y1] = pt(i, 1);
    const steps = Math.ceil(Math.hypot(x1 - cx, y1 - cy));
    for (let s = 0; s <= steps; s += 3)
      d.set(cx + ((x1 - cx) * s) / (steps || 1), cy + ((y1 - cy) * s) / (steps || 1), 99);
  }
  data.forEach((vals, s) => {
    for (let i = 0; i < n; i++) {
      const [x0, y0] = pt(i, Math.min(1, (vals[i] ?? 0) / max));
      const [x1, y1] = pt((i + 1) % n, Math.min(1, (vals[(i + 1) % n] ?? 0) / max));
      d.line(x0, y0, x1, y1, s);
    }
  });
  const grid = d.rows2cells((o) => (o === 99 ? "chart.grid" : o >= 0 ? SER(o) : ""));
  const totalW = plotCols + 2 * (labW + 2);
  const rows: Cell[][] = grid.map((r) => [
    ...Array.from({ length: labW + 2 }, () => cell(" ")),
    ...r,
    ...Array.from({ length: labW + 2 }, () => cell(" ")),
  ]);
  labels.forEach((l, i) => {
    const t = truncateText(l, labW);
    const tw = textWidth(t);
    const px = cx + (R + 3) * Math.cos(ang(i));
    const py = cy + (R + 3) * Math.sin(ang(i));
    const col = labW + 2 + Math.round(px / 2);
    const row = Math.max(0, Math.min(rowsN - 1, Math.round(py / 4)));
    const c = Math.cos(ang(i));
    const x0 = Math.abs(c) < 0.3 ? col - Math.floor(tw / 2) : c > 0 ? col + 1 : col - tw;
    textCells(t, "chart.label").forEach((ce, k) => {
      const x = x0 + k;
      if (x >= 0 && x < totalW && row >= 0) rows[row][x] = ce;
    });
    if (!anchors.has(String(l)))
      anchors.set(String(l), {
        rect: { x: Math.max(0, x0), y: row, w: Math.max(1, Math.min(tw, totalW - Math.max(0, x0))), h: 1 },
      });
  });
  const meta = fromRows(width, [textCells(`scale 0–${fmtNum(max)}`, "text.muted")]);
  const legend = legendRow(names, width);
  names.forEach((nm, i) => {
    const r = legend.rects[i];
    anchors.set(`series:${nm}`, { rect: { x: r.x, y: rows.length + 1, w: r.w, h: 1 } });
  });
  return {
    block: vstack(
      [
        fromRows(
          width,
          rows.map((r) => padRow(r, Math.max(width, totalW)).slice(0, width)),
        ),
        meta,
        legend.block,
      ],
      0,
      width,
    ),
    anchors,
  };
}
void cell;
