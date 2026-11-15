// A small generated picture (sky, sun, two hills) as a PNG data URI, so the Image sample needs no file.
import { deflateSync } from "node:zlib";

const crcTable = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();
const crc32 = (buf: Uint8Array): number => {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = crcTable[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
};
const chunk = (type: string, data: Uint8Array): Buffer => {
  const out = Buffer.alloc(12 + data.length);
  out.writeUInt32BE(data.length, 0);
  out.write(type, 4, "ascii");
  Buffer.from(data).copy(out, 8);
  out.writeUInt32BE(crc32(out.subarray(4, 8 + data.length)), 8 + data.length);
  return out;
};

/** An RGB PNG from a per-pixel function. */
export function encodePng(w: number, h: number, pixel: (x: number, y: number) => [number, number, number]): Buffer {
  const raw = Buffer.alloc((w * 3 + 1) * h);
  for (let y = 0; y < h; y++) {
    raw[y * (w * 3 + 1)] = 0;
    for (let x = 0; x < w; x++) {
      const [r, g, b] = pixel(x, y);
      const o = y * (w * 3 + 1) + 1 + x * 3;
      raw[o] = r;
      raw[o + 1] = g;
      raw[o + 2] = b;
    }
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 2; // truecolor
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", new Uint8Array(0)),
  ]);
}

const mix = (a: number[], b: number[], t: number): [number, number, number] => [
  a[0] + (b[0] - a[0]) * t,
  a[1] + (b[1] - a[1]) * t,
  a[2] + (b[2] - a[2]) * t,
];

export function samplePng(w = 192, h = 108): Buffer {
  return encodePng(w, h, (x, y) => {
    const v = y / h;
    let c: [number, number, number] = mix([18, 24, 66], [255, 150, 70], Math.min(1, v * 1.35));
    const sx = w * 0.68;
    const sy = h * 0.5;
    const d = Math.hypot(x - sx, (y - sy) * 1.0);
    if (d < h * 0.17) {
      c = mix([255, 244, 200], [255, 190, 90], d / (h * 0.17));
    } else if (d < h * 0.3) {
      c = mix(c, [255, 200, 120], 0.35 * (1 - (d - h * 0.17) / (h * 0.13)));
    }
    const far = h * (0.66 + 0.07 * Math.sin(x / 17) + 0.04 * Math.sin(x / 6));
    const near = h * (0.8 + 0.06 * Math.sin(x / 11 + 2) + 0.03 * Math.sin(x / 4));
    if (y > far) c = mix([40, 60, 90], [20, 36, 62], (y - far) / (h - far));
    if (y > near) c = mix([14, 30, 38], [6, 14, 22], (y - near) / (h - near));
    return [Math.round(c[0]), Math.round(c[1]), Math.round(c[2])];
  });
}

export const SAMPLE_PNG_DATA_URI = `data:image/png;base64,${samplePng().toString("base64")}`;
