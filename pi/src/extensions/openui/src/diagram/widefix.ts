// Keep the committed table of two-cell characters (wide.ts), but let the running pi decide the blocks where Unicode keeps adding
// emoji. Pure: the probe is any function from a code point to its width in columns, so nothing here needs pi.

/**
 * Where new two-cell characters appear with each Unicode release: the arrows, symbols and dingbats that got an emoji form
 * (U+2190-2BFF) and the emoji blocks (U+1F000-1FFFF). Everything else the table covers (CJK, Hangul, fullwidth forms) is stable.
 * About 6,800 code points: cheap to ask pi about when the extension loads.
 */
export const PROBE_SPANS: ReadonlyArray<readonly [number, number]> = [
  [0x2190, 0x2bff],
  [0x1f000, 0x1ffff],
];

/**
 * A table (flat [first, last] pairs, ascending, merged) that is `base` outside `spans` and what `probe` says inside them.
 * `probe` is only ever asked about code points inside the spans.
 */
export function healWide(
  base: readonly number[],
  probe: (cp: number) => number,
  spans: ReadonlyArray<readonly [number, number]> = PROBE_SPANS,
): number[] {
  // the base ranges with the spans cut out of them
  const keep: Array<[number, number]> = [];
  for (let i = 0; i < base.length; i += 2) {
    let pieces: Array<[number, number]> = [[base[i], base[i + 1]]];
    for (const [s, e] of spans) {
      const next: Array<[number, number]> = [];
      for (const [a, b] of pieces) {
        if (b < s || a > e) next.push([a, b]);
        else {
          if (a < s) next.push([a, s - 1]);
          if (b > e) next.push([e + 1, b]);
        }
      }
      pieces = next;
    }
    keep.push(...pieces);
  }
  // what the probe says about the spans
  const asked: Array<[number, number]> = [];
  for (const [s, e] of spans) {
    let start = -1;
    for (let cp = s; cp <= e; cp++) {
      const wide = probe(cp) === 2;
      if (wide && start < 0) start = cp;
      else if (!wide && start >= 0) {
        asked.push([start, cp - 1]);
        start = -1;
      }
    }
    if (start >= 0) asked.push([start, e]);
  }
  const out: number[] = [];
  for (const [a, b] of [...keep, ...asked].sort((x, y) => x[0] - y[0])) {
    if (out.length && a <= out[out.length - 1] + 1) out[out.length - 1] = Math.max(out[out.length - 1], b);
    else out.push(a, b);
  }
  return out;
}
