// Global grid router: one A* search per edge over a character grid.
//
// Rules that make the picture unambiguous:
//  * every edge owns its own cells, wires never share a cell
//  * two wires may cross only at a right angle, going straight through (drawn as a plus)
//  * corners and end cells are exclusive
//  * boxes are obstacles; zone borders may be crossed at a right angle
// Directions: 0 = north, 1 = east, 2 = south, 3 = west.

import type { Rect } from "./canvas.ts";

export const DX = [0, 1, 0, -1];
export const DY = [-1, 0, 1, 0];
export const OPP = [2, 3, 0, 1];
export const ZONE = -2;

/**
 * Search budget per edge is W*H*guardScale expansions; tests set it to 0 to force failures. `budget` is the total for one
 * diagram: every edge, every retry and every candidate layout (dag, erd and infra try up to ten) draw on it. Without it the cost
 * grows with edges x retries x candidates x grid size, and a 72-node map froze pi for seconds. When it is spent, the remaining
 * edges are not routed and are listed as omitted. Counted in expansions, not time, so the result does not depend on the machine:
 * about 4,000 expansions a millisecond, so the default is roughly half a second at its worst.
 */
export const tuning = { guardScale: 8, budget: 2_000_000 };

/** The work left for the diagram being laid out. Infinity outside `budgeted`, so a Router used on its own is not limited. */
export const work = { left: Infinity, spent: 0 };

/** Run one diagram layout (all its candidates) against the shared budget. */
export function budgeted<R>(layout: () => R): R {
  work.left = tuning.budget;
  work.spent = 0;
  try {
    return layout();
  } finally {
    work.left = Infinity;
  }
}

const STEP = 10;
const TURN = 12;
const CROSS = 30;
const CROSS_ZONE = 4;
const ADJ = 3;
const OVER = 400;

export interface Waypoint {
  x: number;
  y: number;
  dir: number;
}

export interface RouteReq {
  id: number;
  from: Rect;
  to: Rect;
  /** node ids (as given to addSolid) of the two boxes, so their port cells stay usable */
  fromId?: number;
  toId?: number;
  /** Penalty per side (N, E, S, W) for leaving `from` / entering `to`. Negative forbids the side. */
  fromPen: number[];
  toPen: number[];
  waypoints: Waypoint[];
}

/** `dir` is the direction of travel used to enter this cell. For the first cell it points away from the box. */
export interface Step {
  x: number;
  y: number;
  dir: number;
}

export interface Routed {
  id: number;
  steps: Step[];
  ok: boolean;
  forced: boolean;
}

class Heap {
  private f: number[] = [];
  private v: number[] = [];
  get size(): number {
    return this.f.length;
  }
  push(f: number, v: number): void {
    const F = this.f;
    const V = this.v;
    let i = F.length;
    F.push(f);
    V.push(v);
    while (i > 0) {
      const p = (i - 1) >> 1;
      if (F[p] <= f) break;
      F[i] = F[p];
      V[i] = V[p];
      i = p;
    }
    F[i] = f;
    V[i] = v;
  }
  pop(): number {
    const F = this.f;
    const V = this.v;
    const top = V[0];
    const lf = F.pop()!;
    const lv = V.pop()!;
    const n = F.length;
    if (n > 0) {
      let i = 0;
      for (;;) {
        let c = 2 * i + 1;
        if (c >= n) break;
        if (c + 1 < n && F[c + 1] < F[c]) c++;
        if (F[c] >= lf) break;
        F[i] = F[c];
        V[i] = V[c];
        i = c;
      }
      F[i] = lf;
      V[i] = lv;
    }
    return top;
  }
}

interface Source {
  x: number;
  y: number;
  dir: number;
  cost: number;
  /** Port cells must be free. Waypoint cells may already be a crossing. */
  strict?: boolean;
}

export class Router {
  readonly W: number;
  readonly H: number;
  readonly solid: Uint8Array;
  readonly vOwn: Int32Array;
  readonly hOwn: Int32Array;
  readonly full: Int32Array;
  /** cells next to a box side belong to that box: only its own edges may use them */
  readonly portNode: Int32Array;
  private curFrom = -1;
  private curTo = -1;
  private gen = 0;
  /** the net being routed: its own reserved cells count as free */
  private cur = -2;
  private readonly stamp: Int32Array;
  private readonly best: Int32Array;
  private readonly prev: Int32Array;

  constructor(W: number, H: number) {
    this.W = W;
    this.H = H;
    this.solid = new Uint8Array(W * H);
    this.vOwn = new Int32Array(W * H).fill(-1);
    this.hOwn = new Int32Array(W * H).fill(-1);
    this.full = new Int32Array(W * H).fill(-1);
    this.portNode = new Int32Array(W * H).fill(-1);
    this.stamp = new Int32Array(W * H * 4);
    this.best = new Int32Array(W * H * 4);
    this.prev = new Int32Array(W * H * 4);
  }

  addSolid(r: Rect, id = -1): void {
    for (let y = r.y; y < r.y + r.h; y++)
      for (let x = r.x; x < r.x + r.w; x++)
        if (x >= 0 && y >= 0 && x < this.W && y < this.H) this.solid[y * this.W + x] = 1;
    if (id < 0) return;
    for (let side = 0; side < 4; side++)
      for (const c of this.sideCells(r, side)) {
        if (c.x < 0 || c.y < 0 || c.x >= this.W || c.y >= this.H) continue;
        const k = c.y * this.W + c.x;
        if (this.portNode[k] === -1) this.portNode[k] = id;
      }
  }

  /** A zone outline: wires may cross it at a right angle but never run along it. */
  addZoneBorder(r: Rect): void {
    const W = this.W;
    for (let x = r.x; x < r.x + r.w; x++) {
      for (const y of [r.y, r.y + r.h - 1]) {
        if (x < 0 || y < 0 || x >= W || y >= this.H) continue;
        const corner = x === r.x || x === r.x + r.w - 1;
        if (corner) this.full[y * W + x] = ZONE;
        else this.hOwn[y * W + x] = ZONE;
      }
    }
    for (let y = r.y + 1; y < r.y + r.h - 1; y++) {
      for (const x of [r.x, r.x + r.w - 1]) {
        if (x < 0 || y < 0 || x >= W || y >= this.H) continue;
        this.vOwn[y * W + x] = ZONE;
      }
    }
  }

  private cellFree(x: number, y: number): boolean {
    if (x < 0 || y < 0 || x >= this.W || y >= this.H) return false;
    const c = y * this.W + x;
    const k = this.cur;
    return (
      !this.solid[c] &&
      (this.vOwn[c] === -1 || this.vOwn[c] === k) &&
      (this.hOwn[c] === -1 || this.hOwn[c] === k) &&
      (this.full[c] === -1 || this.full[c] === k)
    );
  }

  private adjacency(x: number, y: number, fromX: number, fromY: number): number {
    let n = 0;
    for (let d = 0; d < 4; d++) {
      const ax = x + DX[d];
      const ay = y + DY[d];
      if (ax === fromX && ay === fromY) continue;
      if (ax < 0 || ay < 0 || ax >= this.W || ay >= this.H) continue;
      const c = ay * this.W + ax;
      const k = this.cur;
      const o =
        (this.vOwn[c] > -1 && this.vOwn[c] !== k) ||
        (this.hOwn[c] > -1 && this.hOwn[c] !== k) ||
        (this.full[c] > -1 && this.full[c] !== k);
      if (o) n++;
    }
    return n;
  }

  private sideCells(r: Rect, side: number): Array<{ x: number; y: number }> {
    const out: Array<{ x: number; y: number }> = [];
    if (side === 0 || side === 2) {
      const y = side === 0 ? r.y - 1 : r.y + r.h;
      for (let x = r.x + 1; x <= r.x + r.w - 2; x++) out.push({ x, y });
    } else {
      const x = side === 1 ? r.x + r.w : r.x - 1;
      for (let y = r.y + 1; y <= r.y + r.h - 2; y++) out.push({ x, y });
    }
    return out;
  }

  private search(
    sources: Source[],
    isTarget: (x: number, y: number, d: number) => number,
    bx0: number,
    by0: number,
    bx1: number,
    by1: number,
    relax: boolean,
    straightStart = false,
  ): Step[] | null {
    const W = this.W;
    const H = this.H;
    const gen = ++this.gen;
    const heap = new Heap();
    const hOf = (x: number, y: number): number =>
      (Math.max(0, bx0 - x, x - bx1) + Math.max(0, by0 - y, y - by1)) * STEP;
    for (const s of sources) {
      if (s.x < 0 || s.y < 0 || s.x >= W || s.y >= H || this.solid[s.y * W + s.x]) continue;
      if (s.strict !== false && !relax && !this.cellFree(s.x, s.y)) continue;
      const st = (s.y * W + s.x) * 4 + s.dir;
      if (this.stamp[st] === gen && this.best[st] <= s.cost) continue;
      this.stamp[st] = gen;
      this.best[st] = s.cost;
      this.prev[st] = -1;
      heap.push(s.cost + hOf(s.x, s.y), st);
    }
    let guard = Math.min(W * H * tuning.guardScale, work.left);
    const start = guard;
    while (heap.size > 0 && guard-- > 0) {
      const st = heap.pop();
      const d = st & 3;
      const c = st >> 2;
      const x = c % W;
      const y = (c - x) / W;
      const g = this.best[st];
      if (isTarget(x, y, d) >= 0) {
        work.left -= start - guard;
        work.spent += start - guard;
        const rev: Step[] = [];
        let cur = st;
        while (cur !== -1) {
          const cd = cur & 3;
          const cc = cur >> 2;
          rev.push({ x: cc % W, y: Math.floor(cc / W), dir: cd });
          cur = this.prev[cur];
        }
        return rev.reverse();
      }
      const cur = this.cur;
      const O = (v: number) => (v === cur ? -1 : v);
      const owned = O(this.vOwn[c]) !== -1 || O(this.hOwn[c]) !== -1;
      const firstCell = straightStart && this.prev[st] === -1;
      for (let k = 0; k < 3; k++) {
        const nd = k === 0 ? d : k === 1 ? (d + 1) & 3 : (d + 3) & 3;
        if (nd !== d && (owned || firstCell)) continue;
        const nx = x + DX[nd];
        const ny = y + DY[nd];
        if (nx < 0 || ny < 0 || nx >= W || ny >= H) continue;
        const q = ny * W + nx;
        if (this.solid[q]) continue;
        const pn = this.portNode[q];
        if (pn !== -1 && pn !== this.curFrom && pn !== this.curTo) continue;
        let cost = STEP;
        if (O(this.full[q]) !== -1) {
          if (!relax || this.full[q] === ZONE) continue;
          cost += OVER;
        }
        const vertical = nd === 0 || nd === 2;
        const same = O(vertical ? this.vOwn[q] : this.hOwn[q]);
        const other = O(vertical ? this.hOwn[q] : this.vOwn[q]);
        if (same !== -1) {
          if (!relax || same === ZONE) continue;
          cost += OVER;
        }
        if (other !== -1) cost += other === ZONE ? CROSS_ZONE : CROSS;
        if (nd !== d) cost += TURN;
        cost += ADJ * this.adjacency(nx, ny, x, y);
        const tx = isTarget(nx, ny, nd);
        if (tx > 0) cost += tx;
        const ns = q * 4 + nd;
        const ng = g + cost;
        if (this.stamp[ns] === gen && this.best[ns] <= ng) continue;
        this.stamp[ns] = gen;
        this.best[ns] = ng;
        this.prev[ns] = st;
        heap.push(ng + hOf(nx, ny), ns);
      }
    }
    const used = start - Math.max(guard, 0);
    work.left -= used;
    work.spent += used;
    return null;
  }

  /** Route one edge. Does not commit; call `commit` with the result. */
  route(req: RouteReq, relax = false): Routed {
    this.cur = req.id;
    this.curFrom = req.fromId ?? -1;
    this.curTo = req.toId ?? -1;
    const attempt = (useWaypoints: boolean): Step[] | null => {
      let sources: Source[] = [];
      for (let side = 0; side < 4; side++) {
        const pen = req.fromPen[side];
        if (pen < 0) continue;
        for (const c of this.sideCells(req.from, side)) sources.push({ x: c.x, y: c.y, dir: side, cost: pen });
      }
      let steps: Step[] = [];
      let portLeg = true;
      const wps = useWaypoints ? req.waypoints : [];
      for (const wp of wps) {
        const path = this.search(
          sources,
          (x, y, d) => (x === wp.x && y === wp.y && d === wp.dir ? 0 : -1),
          wp.x,
          wp.y,
          wp.x,
          wp.y,
          relax,
          portLeg,
        );
        portLeg = false;
        if (!path) return null;
        steps = steps.length ? steps.concat(path.slice(1)) : path;
        sources = [{ x: wp.x, y: wp.y, dir: wp.dir, cost: 0, strict: false }];
      }
      const tmap = new Map<number, [number, number]>();
      for (let side = 0; side < 4; side++) {
        const pen = req.toPen[side];
        if (pen < 0) continue;
        for (const c of this.sideCells(req.to, side)) {
          if (c.x < 0 || c.y < 0 || c.x >= this.W || c.y >= this.H) continue;
          tmap.set(c.y * this.W + c.x, [OPP[side], pen]);
        }
      }
      const t = req.to;
      const path = this.search(
        sources,
        (x, y, d) => {
          const e = tmap.get(y * this.W + x);
          // an end cell is exclusive: it must not already carry a crossing wire
          return e && e[0] === d && (relax || this.cellFree(x, y)) ? e[1] : -1;
        },
        t.x - 1,
        t.y - 1,
        t.x + t.w,
        t.y + t.h,
        relax,
        portLeg,
      );
      if (!path) return null;
      return steps.length ? steps.concat(path.slice(1)) : path;
    };
    let steps = attempt(true);
    if (!steps && req.waypoints.length) steps = attempt(false);
    if (!steps) return { id: req.id, steps: [], ok: false, forced: relax };
    return { id: req.id, steps, ok: true, forced: relax };
  }

  /** Reserve straight cells for a net before routing. Others may cross them at a right angle only. */
  reserve(id: number, cells: Array<{ x: number; y: number }>, vertical: boolean): void {
    for (const c of cells) {
      if (c.x < 0 || c.y < 0 || c.x >= this.W || c.y >= this.H) continue;
      const k = c.y * this.W + c.x;
      if (vertical) this.vOwn[k] = id;
      else this.hOwn[k] = id;
    }
  }

  commit(r: Routed): void {
    const W = this.W;
    const n = r.steps.length;
    for (let i = 0; i < n; i++) {
      const s = r.steps[i];
      const c = s.y * W + s.x;
      const inD = s.dir;
      const outD = i + 1 < n ? r.steps[i + 1].dir : -1;
      if (i === 0 || i === n - 1 || outD !== inD) this.full[c] = r.id;
      else if (inD === 0 || inD === 2) this.vOwn[c] = r.id;
      else this.hOwn[c] = r.id;
    }
  }
}
