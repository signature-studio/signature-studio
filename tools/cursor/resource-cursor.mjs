// The reference of a cursor of FL's own resources (#1392, decision 0260), read from the PE file and never from the engine: the measurement compares the picture the page shows with it.
// readPeResourceCursor(file, {name} | {ordinal}): the RT_GROUP_CURSOR of that name (a string) or integer id, its one entry, the RT_CURSOR the entry names (Learn: Resource File Formats, NEWHEADER and RESDIR; the
// cursor's hot spot is the two WORDs before the DIB; biHeight is twice the height). The pixels follow the decision: AND 0 is the XOR colour, opaque; AND 1 and XOR 0 is transparent (0, 0, 0, 0);
// AND 1 and XOR not 0 (a screen inversion, which CSS cannot show) is opaque black. A 32-bit XOR with a non-zero alpha byte is drawn with its alpha and the AND mask is ignored (alpha: true; fle_cursor.h). Output: top row first, R, G, B, A.
// decodePngRgba(bytes): an 8-bit RGBA PNG (the page's picture, cursor.ts / png.ts) as {width, height, rgba}; filters 0..4 are undone.
import fs from 'node:fs';
import zlib from 'node:zlib';

const RT_CURSOR = 1, RT_GROUP_CURSOR = 12;

/** the PE file's resource tree: a reader of the directory entries (names as strings, ids as numbers) */
function peResources(buf) {
  const u16 = (o) => buf.readUInt16LE(o), u32 = (o) => buf.readUInt32LE(o);
  if (u16(0) !== 0x5a4d) throw new Error('not a PE file (no MZ)');
  const lfanew = u32(0x3c), coff = lfanew + 4;
  if (u32(lfanew) !== 0x4550) throw new Error('not a PE file (no PE signature)');
  const nsec = u16(coff + 2), optSize = u16(coff + 16), opt = coff + 20;
  if (u16(opt) !== 0x20b) throw new Error('not a PE32+ file');
  const rsrcRva = u32(opt + 112 + 2 * 8), rsrcSize = u32(opt + 112 + 2 * 8 + 4);
  const secs = [];
  for (let i = 0; i < nsec; i++) {
    const s = opt + optSize + 40 * i;
    secs.push({ vsize: u32(s + 8), va: u32(s + 12), rawSize: u32(s + 16), raw: u32(s + 20) });
  }
  const fileOff = (rva) => {
    for (const s of secs) if (rva >= s.va && rva < s.va + Math.max(s.vsize, s.rawSize)) return rva - s.va + s.raw;
    throw new Error(`rva ${rva} is in no section`);
  };
  const R = fileOff(rsrcRva);
  const entries = (dir) => {
    const nn = u16(dir + 12), ni = u16(dir + 14), out = [];
    for (let i = 0; i < nn + ni; i++) {
      const e = dir + 16 + 8 * i, name = u32(e), target = u32(e + 4);
      let key = name;
      if (name & 0x80000000) {
        const so = R + (name & 0x7fffffff);
        key = buf.subarray(so + 2, so + 2 + 2 * u16(so)).toString('utf16le');
      }
      out.push({ key, sub: (target & 0x80000000) !== 0, target: target & 0x7fffffff });
    }
    return out;
  };
  /** the data of the one language leaf under a name entry (the group or cursor), as a file offset and size */
  const leafOf = (nameEntry) => {
    const lang = entries(R + nameEntry.target)[0];
    if (!lang || lang.sub) throw new Error('a resource name without one language leaf');
    const de = R + lang.target;
    return { off: fileOff(u32(de)), size: u32(de + 4) };
  };
  const typeDir = (id) => {
    const t = entries(R).find((e) => e.key === id);
    if (!t) throw new Error(`no resource type ${id}`);
    return entries(R + t.target);
  };
  return { buf, u16, u32, entries, leafOf, typeDir, rsrcSize };
}

const foldName = (s) => s.replace(/[a-z]/g, (c) => c.toUpperCase());

/** the RT_GROUP_CURSOR entries of the file, for a report: the group's name (or id), its entries (the RT_CURSOR id, width, height, hot spot of each one, read as readPeResourceCursor reads it) */
export function listPeResourceCursors(file) {
  const pe = peResources(fs.readFileSync(file));
  const cursors = new Map(pe.typeDir(RT_CURSOR).map((e) => [e.key, e]));
  return pe.typeDir(RT_GROUP_CURSOR).map((g) => {
    const gl = pe.leafOf(g);
    const count = pe.u16(gl.off + 4);
    const entries = [];
    for (let i = 0; i < count; i++) {
      const cid = pe.u16(gl.off + 6 + 14 * i + 12);
      const cur = cursors.get(cid);
      if (!cur) { entries.push({ cursorId: cid, missing: true }); continue; }
      const c = pe.leafOf(cur);
      const bi = c.off + 4;
      entries.push({ cursorId: cid, hotX: pe.u16(c.off), hotY: pe.u16(c.off + 2), width: pe.u32(bi + 4), height: pe.u32(bi + 8) / 2, bpp: pe.u16(bi + 14) });
    }
    return { name: typeof g.key === 'string' ? g.key : null, id: typeof g.key === 'number' ? g.key : null, count, entries };
  });
}

/** the reference picture of one cursor of the file: see the header of this file */
export function readPeResourceCursor(file, sel) {
  const pe = peResources(fs.readFileSync(file));
  const { buf, u16, u32 } = pe;
  const groups = pe.typeDir(RT_GROUP_CURSOR);
  const group = sel.name !== undefined ? groups.find((e) => typeof e.key === 'string' && foldName(e.key) === foldName(sel.name)) : groups.find((e) => e.key === sel.ordinal);
  if (!group) throw new Error(`no RT_GROUP_CURSOR of ${sel.name !== undefined ? `name ${sel.name}` : `id ${sel.ordinal}`}`);
  const g = pe.leafOf(group);
  if (u16(g.off) !== 0 || u16(g.off + 2) !== 2) throw new Error('RT_GROUP_CURSOR NEWHEADER is not (Reserved 0, ResType 2)');
  const count = u16(g.off + 4);
  if (count !== 1) throw new Error(`RT_GROUP_CURSOR has ${count} entries (one is read here)`);
  const cursorId = u16(g.off + 6 + 12);
  const cur = pe.typeDir(RT_CURSOR).find((e) => e.key === cursorId);
  if (!cur) throw new Error(`no RT_CURSOR ${cursorId}`);
  const c = pe.leafOf(cur);
  const hotX = u16(c.off), hotY = u16(c.off + 2), bi = c.off + 4;
  if (u32(bi) !== 40) throw new Error('RT_CURSOR DIB header is not BITMAPINFOHEADER');
  const width = u32(bi + 4), height2 = u32(bi + 8), bpp = u16(bi + 14), compression = u32(bi + 16), clrUsed = u32(bi + 32);
  if (u16(bi + 12) !== 1 || compression !== 0) throw new Error('RT_CURSOR DIB is compressed or has several planes');
  if (height2 % 2 !== 0) throw new Error('RT_CURSOR biHeight is not twice the height');
  const height = height2 / 2;
  if (![1, 4, 8, 24, 32].includes(bpp)) throw new Error(`RT_CURSOR bit count ${bpp}`);
  const palN = bpp <= 8 ? (clrUsed || 1 << bpp) : 0;
  const pal = [];
  for (let i = 0; i < palN; i++) { const p = bi + 40 + 4 * i; pal.push([buf[p + 2], buf[p + 1], buf[p]]); }
  const xorStart = bi + 40 + 4 * palN;
  const xorStride = Math.floor((width * bpp + 31) / 32) * 4;
  const andStart = xorStart + xorStride * height;
  const andStride = Math.floor((width + 31) / 32) * 4;
  if (andStart + andStride * height > c.off + c.size) throw new Error('RT_CURSOR is shorter than its masks');
  // the alpha rule: a 32-bit colour with one alpha byte that is not 0 is drawn with the alpha bytes
  let alphaMode = false;
  if (bpp === 32) for (let y = 0; y < height && !alphaMode; y++) for (let x = 0; x < width; x++) if (buf[xorStart + xorStride * (height - 1 - y) + 4 * x + 3] !== 0) { alphaMode = true; break; }
  const rgba = new Uint8Array(width * height * 4);
  const counts = { invert: 0, transparent: 0, opaque: 0 };
  for (let y = 0; y < height; y++) {
    const src = height - 1 - y;   // the DIB's rows are bottom first
    for (let x = 0; x < width; x++) {
      const andBit = (buf[andStart + andStride * src + (x >> 3)] >> (7 - (x & 7))) & 1;
      let raw = 0, r = 0, gg = 0, b = 0, alpha = 255;
      if (bpp === 32) {
        const p = xorStart + xorStride * src + 4 * x;
        b = buf[p]; gg = buf[p + 1]; r = buf[p + 2]; alpha = buf[p + 3];
        raw = (r | (gg << 8) | (b << 16)) >>> 0;
      } else if (bpp === 24) {
        const p = xorStart + xorStride * src + 3 * x;
        b = buf[p]; gg = buf[p + 1]; r = buf[p + 2];
        raw = (r | (gg << 8) | (b << 16)) >>> 0;
      } else {
        if (bpp === 1) raw = (buf[xorStart + xorStride * src + (x >> 3)] >> (7 - (x & 7))) & 1;
        else if (bpp === 4) { const byte = buf[xorStart + xorStride * src + (x >> 1)]; raw = x & 1 ? byte & 15 : byte >> 4; }
        else raw = buf[xorStart + xorStride * src + x];
        if (raw >= palN) throw new Error('a pixel names a palette entry that does not exist');
        [r, gg, b] = pal[raw];
      }
      const i = 4 * (y * width + x);
      if (alphaMode) {
        rgba[i] = r; rgba[i + 1] = gg; rgba[i + 2] = b; rgba[i + 3] = alpha;
        if (alpha === 0) counts.transparent++;
        else if (alpha === 255) counts.opaque++;
        continue;
      }
      if (andBit) {
        if (raw !== 0) { rgba[i + 3] = 255; counts.invert++; }
        else counts.transparent++;
      } else {
        rgba[i] = r; rgba[i + 1] = gg; rgba[i + 2] = b; rgba[i + 3] = 255;
        counts.opaque++;
      }
    }
  }
  return { file, select: sel, cursorId, hotX, hotY, width, height, rgba, counts, alpha: alphaMode };
}

/** the reference picture of a cursor made by CreateIconIndirect (#1392): `raw` is the engine's copy of its bitmaps, six bytes per pixel (a = the AND bit, nz = the XOR value is not 0, r, g, b = the XOR
 * colour, al = the alpha byte), top row first. The same rule as the decision: with `alpha` (the 32-bit colour has an alpha byte that is not 0) the alpha is drawn and the AND bit is ignored; else AND 0
 * is the XOR colour, opaque; AND 1 and XOR 0 is transparent; AND 1 and XOR not 0 is opaque black. */
export function referenceFromRaw(raw, width, height, hot, alpha = false) {
  if (raw.length !== width * height * 6) throw new Error(`raw has ${raw.length} bytes for ${width} x ${height}`);
  const rgba = new Uint8Array(width * height * 4);
  const counts = { invert: 0, transparent: 0, opaque: 0 };
  for (let i = 0; i < width * height; i++) {
    const [a, nz, r, g, b, al] = raw.subarray(6 * i, 6 * i + 6);
    const o = 4 * i;
    if (alpha) {
      rgba[o] = r; rgba[o + 1] = g; rgba[o + 2] = b; rgba[o + 3] = al;
      if (al === 0) counts.transparent++;
      else if (al === 255) counts.opaque++;
    } else if (!a) { rgba[o] = r; rgba[o + 1] = g; rgba[o + 2] = b; rgba[o + 3] = 255; counts.opaque++; }
    else if (nz) { rgba[o + 3] = 255; counts.invert++; }
    else counts.transparent++;
  }
  return { width, height, hotX: hot.hotX, hotY: hot.hotY, rgba, counts, alpha };
}

/** the picture at the whole scale k: each pixel repeated k x k times (nearest neighbour, no interpolation); the reference of a copy the page shows at a device pixel ratio above 1 */
export function scaleNearestRef(width, height, rgba, k) {
  const w = width * k, out = new Uint8Array(w * height * k * 4);
  for (let y = 0; y < height * k; y++) {
    for (let x = 0; x < w; x++) {
      const s = (Math.floor(y / k) * width + Math.floor(x / k)) * 4, d = (y * w + x) * 4;
      out.set(rgba.subarray(s, s + 4), d);
    }
  }
  return out;
}

/** an 8-bit RGBA PNG (colour type 6, no interlace) as {width, height, rgba}; throws on anything else */
export function decodePngRgba(bytes) {
  const sig = [137, 80, 78, 71, 13, 10, 26, 10];
  if (!sig.every((v, i) => bytes[i] === v)) throw new Error('not a PNG');
  let p = 8, width = 0, height = 0, idat = [];
  let depth = 0, ctype = 0, interlace = 0;
  while (p < bytes.length) {
    const len = Buffer.from(bytes.subarray(p, p + 4)).readUInt32BE(0);
    const type = Buffer.from(bytes.subarray(p + 4, p + 8)).toString('ascii');
    const data = Buffer.from(bytes.subarray(p + 8, p + 8 + len));
    if (type === 'IHDR') { width = data.readUInt32BE(0); height = data.readUInt32BE(4); depth = data[8]; ctype = data[9]; interlace = data[12]; }
    else if (type === 'IDAT') idat.push(data);
    else if (type === 'IEND') break;
    p += 12 + len;
  }
  if (depth !== 8 || ctype !== 6 || interlace !== 0) throw new Error(`PNG depth ${depth} colour type ${ctype} interlace ${interlace}: not read`);
  const raw = zlib.inflateSync(Buffer.concat(idat));
  const stride = width * 4;
  if (raw.length !== (stride + 1) * height) throw new Error(`PNG data has ${raw.length} bytes for ${width} x ${height}`);
  const rgba = new Uint8Array(stride * height);
  const bpp = 4;
  for (let y = 0; y < height; y++) {
    const f = raw[y * (stride + 1)];
    const src = y * (stride + 1) + 1, dst = y * stride;
    for (let x = 0; x < stride; x++) {
      const v = raw[src + x];
      const a = x >= bpp ? rgba[dst + x - bpp] : 0;
      const b = y > 0 ? rgba[dst - stride + x] : 0;
      const c = x >= bpp && y > 0 ? rgba[dst - stride + x - bpp] : 0;
      let pred = 0;
      if (f === 0) pred = 0;
      else if (f === 1) pred = a;
      else if (f === 2) pred = b;
      else if (f === 3) pred = (a + b) >> 1;
      else if (f === 4) {
        const pa = Math.abs(b - c), pb = Math.abs(a - c), pc = Math.abs(a + b - 2 * c);
        pred = pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      } else throw new Error(`PNG filter ${f}`);
      rgba[dst + x] = (v + pred) & 0xff;
    }
  }
  return { width, height, rgba };
}
