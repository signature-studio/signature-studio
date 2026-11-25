// A PNG of an RGBA picture, for a CSS cursor (#1392). The pixels are written as they are (8 bits per channel, no filter, no scaling): the stream is zlib with stored (uncompressed) deflate blocks,
// so the bytes do not depend on a compressor and the picture is read back to the same pixels. Used by cursor.ts only; the page's canvas is not used (its colour handling would change pixels).

const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(parts: Uint8Array[]): number {
  let c = 0xffffffff;
  for (const p of parts) for (let i = 0; i < p.length; i++) c = (CRC_TABLE[(c ^ (p[i] ?? 0)) & 0xff] ?? 0) ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function adler32(data: Uint8Array): number {
  let a = 1, b = 0;
  for (let i = 0; i < data.length; i++) {
    a = (a + (data[i] ?? 0)) % 65521;
    b = (b + a) % 65521;
  }
  return ((b << 16) | a) >>> 0;
}

const be32 = (v: number): Uint8Array => Uint8Array.of(v >>> 24, (v >>> 16) & 0xff, (v >>> 8) & 0xff, v & 0xff);

function chunk(type: string, data: Uint8Array): Uint8Array {
  const t = Uint8Array.from(type, (ch) => ch.charCodeAt(0));
  const crc = crc32([t, data]);
  const out = new Uint8Array(12 + data.length);
  out.set(be32(data.length), 0);
  out.set(t, 4);
  out.set(data, 8);
  out.set(be32(crc), 8 + data.length);
  return out;
}

/** the zlib stream of `raw` as stored deflate blocks of at most 65535 bytes */
function zlibStored(raw: Uint8Array): Uint8Array {
  const blocks = Math.max(1, Math.ceil(raw.length / 65535));
  const out = new Uint8Array(2 + raw.length + 5 * blocks + 4);
  let p = 0;
  out[p++] = 0x78;
  out[p++] = 0x01;
  for (let i = 0; i < blocks; i++) {
    const start = i * 65535, len = Math.min(65535, raw.length - start);
    out[p++] = i === blocks - 1 ? 1 : 0;
    out[p++] = len & 0xff;
    out[p++] = len >>> 8;
    out[p++] = ~len & 0xff;
    out[p++] = (~len >>> 8) & 0xff;
    out.set(raw.subarray(start, start + len), p);
    p += len;
  }
  out.set(be32(adler32(raw)), p);
  return out;
}

/** the PNG file (8-bit RGBA, colour type 6) of a width x height picture given as width * height * 4 bytes, the top row first */
export function encodePngRgba(width: number, height: number, rgba: Uint8Array): Uint8Array {
  if (rgba.length !== width * height * 4) throw new Error(`encodePngRgba: ${rgba.length} bytes for ${width} x ${height}`);
  const row = width * 4;
  const raw = new Uint8Array((row + 1) * height);
  for (let y = 0; y < height; y++) raw.set(rgba.subarray(y * row, (y + 1) * row), y * (row + 1) + 1);
  const ihdr = new Uint8Array(13);
  ihdr.set(be32(width), 0);
  ihdr.set(be32(height), 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  const parts = [Uint8Array.of(137, 80, 78, 71, 13, 10, 26, 10), chunk('IHDR', ihdr), chunk('IDAT', zlibStored(raw)), chunk('IEND', new Uint8Array(0))];
  const total = parts.reduce((n, p) => n + p.length, 0);
  const out = new Uint8Array(total);
  let p = 0;
  for (const part of parts) { out.set(part, p); p += part.length; }
  return out;
}

/** the bytes in base64 (the alphabet of btoa) */
export function base64Of(bytes: Uint8Array): string {
  let s = '';
  for (let i = 0; i < bytes.length; i++) s += String.fromCharCode(bytes[i] ?? 0);
  return btoa(s);
}

/** the PNG as a data URL, for a CSS `url(...)` */
export function pngDataUrl(width: number, height: number, rgba: Uint8Array): string {
  return `data:image/png;base64,${base64Of(encodePngRgba(width, height, rgba))}`;
}
