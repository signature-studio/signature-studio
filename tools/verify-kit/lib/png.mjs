// Copied from lift/harness/web/present/png.mjs (decision 0126 surface PNGs -> BGRX / RGBA); only this header line is new. The harness is not imported.
// Minimal PNG decoder (8-bit, colour type 2 = RGB or 6 = RGBA, non-interlaced) on node's zlib. Used to turn the engine surface PNGs of decision 0126 into BGRX
// (the byte order of the drawing surface) and into RGBA (the byte order of ImageData / canvas), and to decode Page.captureScreenshot output.
import zlib from 'node:zlib';
import crypto from 'node:crypto';

export function decodePng(buf) {
  const sig = [137, 80, 78, 71, 13, 10, 26, 10];
  for (let i = 0; i < 8; i++) if (buf[i] !== sig[i]) throw new Error('not a PNG');
  let pos = 8, w = 0, h = 0, depth = 0, ctype = 0, interlace = 0;
  const idat = [];
  while (pos < buf.length) {
    const len = buf.readUInt32BE(pos), type = buf.toString('latin1', pos + 4, pos + 8);
    const data = buf.subarray(pos + 8, pos + 8 + len);
    if (type === 'IHDR') { w = data.readUInt32BE(0); h = data.readUInt32BE(4); depth = data[8]; ctype = data[9]; interlace = data[12]; }
    else if (type === 'IDAT') idat.push(data);
    else if (type === 'IEND') break;
    pos += 12 + len;
  }
  if (depth !== 8 || (ctype !== 2 && ctype !== 6) || interlace !== 0) throw new Error(`unsupported PNG: depth ${depth} colour type ${ctype} interlace ${interlace}`);
  const ch = ctype === 2 ? 3 : 4;
  const raw = zlib.inflateSync(Buffer.concat(idat));
  const stride = w * ch;
  if (raw.length !== h * (stride + 1)) throw new Error(`unexpected inflated size ${raw.length} for ${w}x${h}x${ch}`);
  const out = Buffer.alloc(h * stride);
  for (let y = 0; y < h; y++) {
    const ft = raw[y * (stride + 1)];
    const src = y * (stride + 1) + 1, dst = y * stride, up = dst - stride;
    for (let x = 0; x < stride; x++) {
      const a = x >= ch ? out[dst + x - ch] : 0, b = y > 0 ? out[up + x] : 0, c = (x >= ch && y > 0) ? out[up + x - ch] : 0;
      let v = raw[src + x];
      switch (ft) {
        case 0: break;
        case 1: v += a; break;
        case 2: v += b; break;
        case 3: v += (a + b) >> 1; break;
        case 4: { const p = a + b - c, pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c); v += (pa <= pb && pa <= pc) ? a : (pb <= pc ? b : c); break; }
        default: throw new Error('bad filter ' + ft);
      }
      out[dst + x] = v & 255;
    }
  }
  return { width: w, height: h, channels: ch, data: out };
}

// RGBA with A = 255 when the PNG has no alpha (what a canvas read-back of an opaque image gives)
export function toRgba(png) {
  const n = png.width * png.height, o = Buffer.alloc(n * 4);
  for (let i = 0; i < n; i++) {
    o[i * 4] = png.data[i * png.channels]; o[i * 4 + 1] = png.data[i * png.channels + 1]; o[i * 4 + 2] = png.data[i * png.channels + 2];
    o[i * 4 + 3] = png.channels === 4 ? png.data[i * 4 + 3] : 255;
  }
  return o;
}

// BGRX as the drawing surface of fle_raster.c holds it: B, G, R, X (X = 0)
export function toBgrx(png) {
  const n = png.width * png.height, o = Buffer.alloc(n * 4);
  for (let i = 0; i < n; i++) {
    o[i * 4] = png.data[i * png.channels + 2]; o[i * 4 + 1] = png.data[i * png.channels + 1]; o[i * 4 + 2] = png.data[i * png.channels]; o[i * 4 + 3] = 0;
  }
  return o;
}

export const sha256 = (b) => crypto.createHash('sha256').update(b).digest('hex');
