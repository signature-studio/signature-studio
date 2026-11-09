// perf2 (decision 0138): where do two lid-13 PNGs (same size) differ? usage: node diff-png.mjs A.png B.png  -> changed pixel count, bounding box, and the 64 px cells (x0,y0,count) with the most changed pixels, merged to rectangles by 4-connectivity
import fs from 'node:fs';
import { decodePng, toRgba } from '../present/png.mjs';
const [pa, pb] = process.argv.slice(2).map((f) => decodePng(fs.readFileSync(f)));
const w = pa.width, h = pa.height, A = toRgba(pa), B = toRgba(pb);
const CS = 64, cw = Math.ceil(w / CS), ch = Math.ceil(h / CS), cnt = new Uint32Array(cw * ch);
let n = 0, x0 = w, y0 = h, x1 = 0, y1 = 0;
for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) { const i = (y * w + x) * 4; if (A[i] !== B[i] || A[i + 1] !== B[i + 1] || A[i + 2] !== B[i + 2]) { n++; cnt[(y / CS | 0) * cw + (x / CS | 0)]++; if (x < x0) x0 = x; if (x >= x1) x1 = x + 1; if (y < y0) y0 = y; if (y >= y1) y1 = y + 1; } }
console.log(JSON.stringify({ size: [w, h], changed: n, bbox: [x0, y0, x1, y1] }));
const seen = new Uint8Array(cw * ch), rects = [];
for (let c = 0; c < cnt.length; c++) { if (!cnt[c] || seen[c]) continue; const st = [c]; seen[c] = 1; let cx0 = cw, cy0 = ch, cx1 = 0, cy1 = 0, tot = 0; while (st.length) { const k = st.pop(), kx = k % cw, ky = k / cw | 0; tot += cnt[k]; cx0 = Math.min(cx0, kx); cx1 = Math.max(cx1, kx); cy0 = Math.min(cy0, ky); cy1 = Math.max(cy1, ky); for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) { const nx = kx + dx, ny = ky + dy; if (nx < 0 || ny < 0 || nx >= cw || ny >= ch) continue; const m = ny * cw + nx; if (cnt[m] && !seen[m]) { seen[m] = 1; st.push(m); } } } rects.push({ rect: [cx0 * CS, cy0 * CS, Math.min(w, (cx1 + 1) * CS), Math.min(h, (cy1 + 1) * CS)], changed: tot }); }
rects.sort((p, q) => q.changed - p.changed);
console.log(JSON.stringify(rects.slice(0, 12)));
