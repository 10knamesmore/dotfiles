// Layered graph placement (Sugiyama style), deterministic.
//   1. break cycles (DFS feedback edges are reversed for layout only)
//   2. longest-path layering, sources pulled next to their consumers
//   3. dummy items for edges that span several layers
//   4. crossing minimisation: barycenter sweeps + adjacent transposition
//   5. cross-axis coordinates: per-layer exact solve (weighted isotonic regression), a few sweeps

export interface EdgeIn {
  a: number;
  b: number;
}

export interface LEdge {
  /** layout direction a -> b, always from a lower layer to a higher one */
  a: number;
  b: number;
  /** index into the input edge list */
  src: number;
  /** true when the input edge ran b -> a and was reversed to break a cycle */
  rev: boolean;
}

export interface Layered {
  n: number;
  edges: LEdge[];
  nLayers: number;
  itemLayer: number[];
  /** real node index, or -1 for a dummy */
  itemNode: number[];
  /** per layout edge: item ids from a to b, adjacent layers */
  chains: number[][];
  segs: Array<{ u: number; v: number; e: number }>;
  up: number[][];
  down: number[][];
  order: number[][];
}

export function breakCycles(n: number, edges: EdgeIn[]): boolean[] {
  const out: number[][] = Array.from({ length: n }, () => []);
  const indeg = new Array<number>(n).fill(0);
  edges.forEach((e, i) => {
    out[e.a].push(i);
    indeg[e.b]++;
  });
  const state = new Uint8Array(n);
  const rev = new Array<boolean>(edges.length).fill(false);
  const starts = Array.from({ length: n }, (_, i) => i).sort(
    (x, y) => (indeg[x] === 0 ? 0 : 1) - (indeg[y] === 0 ? 0 : 1) || x - y,
  );
  for (const s of starts) {
    if (state[s]) continue;
    const stack: Array<{ v: number; i: number }> = [{ v: s, i: 0 }];
    state[s] = 1;
    while (stack.length) {
      const top = stack[stack.length - 1];
      if (top.i < out[top.v].length) {
        const e = out[top.v][top.i++];
        const w = edges[e].b;
        if (state[w] === 0) {
          state[w] = 1;
          stack.push({ v: w, i: 0 });
        } else if (state[w] === 1) rev[e] = true;
      } else {
        state[top.v] = 2;
        stack.pop();
      }
    }
  }
  return rev;
}

export function layerize(n: number, edgesIn: EdgeIn[]): Layered {
  const rev = breakCycles(n, edgesIn);
  const edges: LEdge[] = edgesIn.map((e, i) =>
    rev[i] ? { a: e.b, b: e.a, src: i, rev: true } : { a: e.a, b: e.b, src: i, rev: false },
  );

  // longest-path layering (Kahn, stable)
  const outE: number[][] = Array.from({ length: n }, () => []);
  const indeg = new Array<number>(n).fill(0);
  for (const [i, e] of edges.entries()) {
    outE[e.a].push(i);
    indeg[e.b]++;
  }
  const layer = new Array<number>(n).fill(0);
  const hasPred = indeg.map((d) => d > 0);
  const deg = indeg.slice();
  const queue: number[] = [];
  for (let v = 0; v < n; v++) if (deg[v] === 0) queue.push(v);
  for (let qi = 0; qi < queue.length; qi++) {
    const v = queue[qi];
    for (const ei of outE[v]) {
      const w = edges[ei].b;
      layer[w] = Math.max(layer[w], layer[v] + 1);
      if (--deg[w] === 0) queue.push(w);
    }
  }
  for (let v = 0; v < n; v++) {
    if (!hasPred[v] && outE[v].length) layer[v] = Math.min(...outE[v].map((ei) => layer[edges[ei].b])) - 1;
  }
  // compress unused layer indexes
  const used = Array.from(new Set(layer)).sort((x, y) => x - y);
  const remap = new Map(used.map((l, i) => [l, i]));
  for (let v = 0; v < n; v++) layer[v] = remap.get(layer[v])!;
  const nLayers = used.length;

  const itemLayer = layer.slice();
  const itemNode = Array.from({ length: n }, (_, i) => i);
  const chains: number[][] = [];
  const segs: Layered["segs"] = [];
  for (const [ei, e] of edges.entries()) {
    const chain = [e.a];
    for (let l = layer[e.a] + 1; l < layer[e.b]; l++) {
      itemLayer.push(l);
      itemNode.push(-1);
      chain.push(itemLayer.length - 1);
    }
    chain.push(e.b);
    chains.push(chain);
    for (let k = 0; k + 1 < chain.length; k++) segs.push({ u: chain[k], v: chain[k + 1], e: ei });
  }
  const N = itemLayer.length;
  const up: number[][] = Array.from({ length: N }, () => []);
  const down: number[][] = Array.from({ length: N }, () => []);
  for (const s of segs) {
    down[s.u].push(s.v);
    up[s.v].push(s.u);
  }

  // initial order: DFS pre-order from the first layer on
  const order: number[][] = Array.from({ length: nLayers }, () => []);
  const seen = new Uint8Array(N);
  const byLayer = Array.from({ length: N }, (_, i) => i).sort((x, y) => itemLayer[x] - itemLayer[y] || x - y);
  for (const root of byLayer) {
    if (seen[root]) continue;
    const stack = [root];
    while (stack.length) {
      const v = stack.pop()!;
      if (seen[v]) continue;
      seen[v] = 1;
      order[itemLayer[v]].push(v);
      for (let k = down[v].length - 1; k >= 0; k--) stack.push(down[v][k]);
    }
  }

  const L: Layered = { n, edges, nLayers, itemLayer, itemNode, chains, segs, up, down, order };
  minimizeCrossings(L);
  return L;
}

function positions(L: Layered): number[] {
  const pos = new Array<number>(L.itemLayer.length).fill(0);
  for (const ord of L.order) ord.forEach((it, i) => (pos[it] = i));
  return pos;
}

function crossingsBetween(L: Layered, pos: number[], l: number, segsAt: Array<{ u: number; v: number }>): number {
  const m = segsAt.length;
  if (m < 2) return 0;
  const arr = segsAt.map((s) => [pos[s.u], pos[s.v]] as const).sort((p, q) => p[0] - q[0] || p[1] - q[1]);
  const size = L.order[l + 1].length + 2;
  const bit = new Int32Array(size + 1);
  let inv = 0;
  let seen = 0;
  for (const [, pv] of arr) {
    // strictly greater earlier values
    let le = 0;
    for (let i = pv + 1; i > 0; i -= i & -i) le += bit[i];
    inv += seen - le;
    for (let i = pv + 1; i <= size; i += i & -i) bit[i]++;
    seen++;
  }
  return inv;
}

function totalCrossings(L: Layered, pos: number[], segsByLayer: Array<Array<{ u: number; v: number }>>): number {
  let t = 0;
  for (let l = 0; l + 1 < L.nLayers; l++) t += crossingsBetween(L, pos, l, segsByLayer[l]);
  return t;
}

function minimizeCrossings(L: Layered): void {
  if (L.nLayers < 2) return;
  const segsByLayer: Array<Array<{ u: number; v: number }>> = Array.from({ length: L.nLayers - 1 }, () => []);
  for (const s of L.segs) segsByLayer[L.itemLayer[s.u]].push({ u: s.u, v: s.v });
  let pos = positions(L);
  let bestC = totalCrossings(L, pos, segsByLayer);
  let best = L.order.map((o) => o.slice());
  if (bestC === 0) return;
  const reorder = (l: number, nb: number[][]) => {
    const ord = L.order[l];
    const key = new Map<number, number>();
    ord.forEach((it, i) => {
      const ns = nb[it];
      key.set(it, ns.length ? ns.reduce((a, j) => a + pos[j], 0) / ns.length : i);
    });
    ord.sort((x, y) => key.get(x)! - key.get(y)! || pos[x] - pos[y]);
    ord.forEach((it, i) => (pos[it] = i));
  };
  const pairCross = (a: number, b: number): number => {
    let c = 0;
    for (const ua of L.up[a]) for (const ub of L.up[b]) if (pos[ua] > pos[ub]) c++;
    for (const da of L.down[a]) for (const db of L.down[b]) if (pos[da] > pos[db]) c++;
    return c;
  };
  const transpose = () => {
    for (let pass = 0; pass < 8; pass++) {
      let improved = false;
      for (let l = 0; l < L.nLayers; l++) {
        const ord = L.order[l];
        for (let i = 0; i + 1 < ord.length; i++) {
          const a = ord[i];
          const b = ord[i + 1];
          if (pairCross(b, a) < pairCross(a, b)) {
            ord[i] = b;
            ord[i + 1] = a;
            pos[a] = i + 1;
            pos[b] = i;
            improved = true;
          }
        }
      }
      if (!improved) break;
    }
  };
  let stagnant = 0;
  for (let iter = 0; iter < 30 && stagnant < 6 && bestC > 0; iter++) {
    if (iter % 2 === 0) for (let l = 1; l < L.nLayers; l++) reorder(l, L.up);
    else for (let l = L.nLayers - 2; l >= 0; l--) reorder(l, L.down);
    transpose();
    pos = positions(L);
    const c = totalCrossings(L, pos, segsByLayer);
    if (c < bestC) {
      bestC = c;
      best = L.order.map((o) => o.slice());
      stagnant = 0;
    } else stagnant++;
  }
  for (let l = 0; l < L.nLayers; l++) L.order[l] = best[l];
}

export function countCrossings(L: Layered): number {
  const segsByLayer: Array<Array<{ u: number; v: number }>> = Array.from(
    { length: Math.max(0, L.nLayers - 1) },
    () => [],
  );
  for (const s of L.segs) segsByLayer[L.itemLayer[s.u]].push({ u: s.u, v: s.v });
  return totalCrossings(L, positions(L), segsByLayer);
}

/** Weighted isotonic regression (pool adjacent violators). Returns the fitted value for each input. */
function pav(t: number[], w: number[]): number[] {
  const sum: number[] = [];
  const wsum: number[] = [];
  const cnt: number[] = [];
  for (let i = 0; i < t.length; i++) {
    sum.push(t[i] * w[i]);
    wsum.push(w[i]);
    cnt.push(1);
    while (sum.length >= 2) {
      const k = sum.length - 1;
      if (sum[k - 1] / wsum[k - 1] <= sum[k] / wsum[k]) break;
      sum[k - 1] += sum[k];
      wsum[k - 1] += wsum[k];
      cnt[k - 1] += cnt[k];
      sum.pop();
      wsum.pop();
      cnt.pop();
    }
  }
  const out: number[] = [];
  for (let b = 0; b < sum.length; b++) for (let j = 0; j < cnt[b]; j++) out.push(sum[b] / wsum[b]);
  return out;
}

/**
 * Cross-axis coordinates (left edge per item), minimum 0.
 * `cs` is the item size along the cross axis, `gap(a, b)` the blank cells required between neighbours.
 */
export function assignCross(
  L: Layered,
  cs: number[],
  gap: (a: number, b: number) => number,
  weight: (u: number, v: number) => number,
): number[] {
  const N = L.itemLayer.length;
  const left = new Array<number>(N).fill(0);
  const layerWidth = L.order.map((ord) => ord.reduce((acc, it, i) => acc + cs[it] + (i ? gap(ord[i - 1], it) : 0), 0));
  const maxW = Math.max(0, ...layerWidth);
  for (let l = 0; l < L.nLayers; l++) {
    let x = Math.floor((maxW - layerWidth[l]) / 2);
    L.order[l].forEach((it, i, ord) => {
      if (i) x += gap(ord[i - 1], it);
      left[it] = x;
      x += cs[it];
    });
  }
  const center = (i: number) => left[i] + (cs[i] - 1) / 2;
  const relayer = (l: number, useUp: boolean, useDown: boolean) => {
    const ord = L.order[l];
    const k = ord.length;
    if (!k) return;
    const off: number[] = [];
    const t: number[] = [];
    const wt: number[] = [];
    for (let i = 0; i < k; i++) {
      const it = ord[i];
      off.push(i ? off[i - 1] + cs[ord[i - 1]] + gap(ord[i - 1], it) : 0);
      let sw = 0;
      let sc = 0;
      if (useUp)
        for (const u of L.up[it]) {
          const w = weight(u, it);
          sw += w;
          sc += w * center(u);
        }
      if (useDown)
        for (const d of L.down[it]) {
          const w = weight(it, d);
          sw += w;
          sc += w * center(d);
        }
      const desired = sw > 0 ? sc / sw - (cs[it] - 1) / 2 : left[it];
      t.push(desired - off[i]);
      wt.push(sw > 0 ? sw : 0.01);
    }
    const fit = pav(t, wt);
    for (let i = 0; i < k; i++) left[ord[i]] = fit[i] + off[i];
  };
  for (let r = 0; r < 5; r++) {
    for (let l = 1; l < L.nLayers; l++) relayer(l, true, false);
    for (let l = L.nLayers - 2; l >= 0; l--) relayer(l, false, true);
  }
  for (let r = 0; r < 2; r++) for (let l = 0; l < L.nLayers; l++) relayer(l, true, true);
  // integer positions with the separation enforced
  for (let l = 0; l < L.nLayers; l++) {
    const ord = L.order[l];
    for (let i = 0; i < ord.length; i++) {
      let v = Math.round(left[ord[i]]);
      if (i) v = Math.max(v, left[ord[i - 1]] + cs[ord[i - 1]] + gap(ord[i - 1], ord[i]));
      left[ord[i]] = v;
    }
  }
  const min = Math.min(...left);
  return left.map((v) => v - min);
}
