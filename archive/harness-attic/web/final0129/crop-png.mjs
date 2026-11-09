// final0129 (decision 0134, PROVISIONAL): crops a region of a PNG and enlarges it by an integer factor (nearest neighbour), to look at the text of a screenshot. usage: node crop-png.mjs IN.png X Y W H SCALE OUT.png
import fs from 'node:fs';
import zlib from 'node:zlib';
import { decodePng, toRgba } from '../present/png.mjs';
const [inp, X, Y, W, H, SC, out] = process.argv.slice(2);
const x0 = +X, y0 = +Y, w = +W, h = +H, k = +SC;
const img = decodePng(fs.readFileSync(inp));
const rgba = toRgba(img);
const ow = w * k, oh = h * k;
const raw = Buffer.alloc(oh * (ow * 3 + 1));
for (let y = 0; y < oh; y++) {
  raw[y * (ow * 3 + 1)] = 0;
  const sy = y0 + Math.floor(y / k);
  for (let x = 0; x < ow; x++) {
    const sx = x0 + Math.floor(x / k), s = (sy * img.width + sx) * 4, d = y * (ow * 3 + 1) + 1 + x * 3;
    raw[d] = rgba[s]; raw[d + 1] = rgba[s + 1]; raw[d + 2] = rgba[s + 2];
  }
}
const chunk = (t, d) => { const b = Buffer.alloc(12 + d.length); b.writeUInt32BE(d.length, 0); b.write(t, 4, 'latin1'); d.copy(b, 8); b.writeUInt32BE(zlib.crc32(b.subarray(4, 8 + d.length)) >>> 0, 8 + d.length); return b; };
const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(ow, 0); ihdr.writeUInt32BE(oh, 4); ihdr[8] = 8; ihdr[9] = 2;
fs.writeFileSync(out, Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(raw, { level: 6 })), chunk('IEND', Buffer.alloc(0))]));
console.log('wrote', out, ow + 'x' + oh);
