// Reading a zip from a Blob (a File): the central directory is parsed here (the names and the extra fields have to be read as bytes), the data is inflated by the browser
// (DecompressionStream('deflate-raw'): Chrome 80+, Firefox 113+) as a stream, so a zip is never held in memory whole and an entry goes straight to its OPFS file.
// Names (decision 0216, opfs1; the observation is the reference project ~/ダウンロード/serina1-serum2.zip, decisions 0178/0193):
//   1. the UTF-8 flag (general purpose bit 11) -> UTF-8;
//   2. an Info-ZIP Unicode Path extra field (0x7075) whose CRC-32 is that of the header's name bytes -> its UTF-8 name (the reference zip has it on both entries with Japanese names);
//   3. ASCII only -> as is;
//   4. otherwise Windows-31J (CP932, WHATWG 'shift_jis') and the writer's '#$XXXX' notation (a character the code page cannot hold, written as Delphi's #$3099) turned back into that UTF-16 unit.
//      MEASURED on the reference zip (test/unit/files-zip-read.test.ts): this fallback is LOSSY for the writer of that zip (it lost characters in its own conversion: メモリアルロヒ and 質 are missing from the header
//      name), so decisions 0178/0193's plan 'CP932 + restore #$3099' does not give the real name; only (2) does, and the reference zip has (2) on both entries: all 48 names equal the names the FLP holds (UTF-16LE search).
// No normalisation: the Unicode field of the reference zip has U+3099 as a separate combining character and the FLP names it that way, so NFC would change the name (and the engine would not find the file).

export type NameSource = 'utf8-flag' | 'unicode-extra' | 'ascii' | 'cp932';

export interface ZipEntry {
  index: number;
  /** the decoded name as the archive wrote it (may contain '\\' and '..': use safeRelative before it becomes a path) */
  name: string;
  nameSource: NameSource;
  isDir: boolean;
  flags: number;
  method: number;
  crc32: number;
  compressedSize: number;
  size: number;
  localOffset: number;
  /** DOS date/time as a Date (local time as written; not applied: OPFS cannot set a modification time) */
  mtime: Date | null;
  encrypted: boolean;
}

export class ZipError extends Error {}

const u16 = (v: DataView, o: number): number => v.getUint16(o, true);
const u32 = (v: DataView, o: number): number => v.getUint32(o, true);
const u64 = (v: DataView, o: number): number => Number(v.getBigUint64(o, true));

let CRC_TABLE: Uint32Array | null = null;
export function crc32Update(crc: number, data: Uint8Array): number {
  if (!CRC_TABLE) {
    CRC_TABLE = new Uint32Array(256);
    for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; CRC_TABLE[n] = c >>> 0; }
  }
  const t = CRC_TABLE;
  let c = ~crc >>> 0;
  for (let i = 0; i < data.length; i++) c = (t[(c ^ data[i]!) & 0xff]!) ^ (c >>> 8);
  return ~c >>> 0;
}

export function crc32(data: Uint8Array): number { return crc32Update(0, data); }

const hasHigh = (b: Uint8Array): boolean => { for (const x of b) if (x > 127) return true; return false; };

/** the name of an entry, from the raw bytes of the header's name field and its extra field (a pure function: the tests call it with the bytes of the real zip) */
export function decodeEntryName(raw: Uint8Array, flags: number, extra: Uint8Array): { name: string; source: NameSource } {
  if (flags & 0x800) return { name: new TextDecoder('utf-8').decode(raw), source: 'utf8-flag' };
  const ev = new DataView(extra.buffer, extra.byteOffset, extra.byteLength);
  for (let p = 0; p + 4 <= extra.length;) {
    const id = u16(ev, p), sz = u16(ev, p + 2);
    if (p + 4 + sz > extra.length) break;
    if (id === 0x7075 && sz >= 5 && extra[p + 4] === 1 && u32(ev, p + 5) === crc32(raw)) {
      return { name: new TextDecoder('utf-8').decode(extra.subarray(p + 9, p + 4 + sz)), source: 'unicode-extra' };
    }
    p += 4 + sz;
  }
  if (!hasHigh(raw) && !new TextDecoder('latin1').decode(raw).includes('#$')) return { name: new TextDecoder('latin1').decode(raw), source: 'ascii' };
  const s = new TextDecoder('shift_jis').decode(raw);
  return { name: s.replace(/#\$([0-9A-Fa-f]{4})/g, (_m, h: string) => String.fromCharCode(parseInt(h, 16))), source: 'cp932' };
}

function dosDate(d: number, t: number): Date | null {
  const year = 1980 + (d >> 9), mon = ((d >> 5) & 15) - 1, day = d & 31;
  if (day === 0 || mon < 0 || mon > 11) return null;
  return new Date(year, mon, day, t >> 11, (t >> 5) & 63, (t & 31) * 2);
}

async function bytesAt(blob: Blob, start: number, end: number): Promise<Uint8Array> {
  return new Uint8Array(await blob.slice(start, end).arrayBuffer());
}

export const MAX_ZIP_ENTRIES = 200_000;

/** the entries of the central directory (Zip64 included) */
export async function readZipDirectory(blob: Blob): Promise<ZipEntry[]> {
  if (blob.size < 22) throw new ZipError('The file is too small to be a zip.');
  const tailStart = Math.max(0, blob.size - (22 + 65535));
  const tail = await bytesAt(blob, tailStart, blob.size);
  const tv = new DataView(tail.buffer, tail.byteOffset, tail.byteLength);
  let e = -1;
  for (let i = tail.length - 22; i >= 0; i--) if (u32(tv, i) === 0x06054b50 && i + 22 + u16(tv, i + 20) === tail.length) { e = i; break; }
  if (e < 0) throw new ZipError('This is not a zip (no end-of-central-directory record).');
  let count = u16(tv, e + 10), cdSize = u32(tv, e + 12), cdOffset = u32(tv, e + 16);
  if (count === 0xffff || cdSize === 0xffffffff || cdOffset === 0xffffffff) {
    const locAt = tailStart + e - 20;
    const loc = locAt >= 0 ? await bytesAt(blob, locAt, locAt + 20) : new Uint8Array(0);
    const lv = new DataView(loc.buffer, loc.byteOffset, loc.byteLength);
    if (loc.length < 20 || u32(lv, 0) !== 0x07064b50) throw new ZipError('Zip64 record missing.');
    const z = await bytesAt(blob, u64(lv, 8), u64(lv, 8) + 56);
    const zv = new DataView(z.buffer, z.byteOffset, z.byteLength);
    if (z.length < 56 || u32(zv, 0) !== 0x06064b50) throw new ZipError('Zip64 end record unreadable.');
    count = u64(zv, 32); cdSize = u64(zv, 40); cdOffset = u64(zv, 48);
  }
  if (count > MAX_ZIP_ENTRIES) throw new ZipError(`The zip lists ${count} entries (limit ${MAX_ZIP_ENTRIES}).`);
  if (cdOffset + cdSize > blob.size) throw new ZipError('The central directory is outside the file.');
  const cd = await bytesAt(blob, cdOffset, cdOffset + cdSize);
  const v = new DataView(cd.buffer, cd.byteOffset, cd.byteLength);
  const out: ZipEntry[] = [];
  let p = 0;
  for (let i = 0; i < count; i++) {
    if (p + 46 > cd.length || u32(v, p) !== 0x02014b50) throw new ZipError(`Central directory entry ${i} is damaged.`);
    const flags = u16(v, p + 8), method = u16(v, p + 10), time = u16(v, p + 12), date = u16(v, p + 14);
    const crc = u32(v, p + 16);
    let csize = u32(v, p + 20), usize = u32(v, p + 24);
    const fnl = u16(v, p + 28), exl = u16(v, p + 30), cml = u16(v, p + 32);
    let off = u32(v, p + 42);
    if (p + 46 + fnl + exl + cml > cd.length) throw new ZipError(`Central directory entry ${i} is damaged.`);
    const raw = cd.subarray(p + 46, p + 46 + fnl);
    const extra = cd.subarray(p + 46 + fnl, p + 46 + fnl + exl);
    const ev = new DataView(extra.buffer, extra.byteOffset, extra.byteLength);
    for (let q = 0; q + 4 <= extra.length;) {
      const id = u16(ev, q), sz = u16(ev, q + 2);
      if (id === 0x0001) {
        let r = q + 4;
        if (usize === 0xffffffff) { usize = u64(ev, r); r += 8; }
        if (csize === 0xffffffff) { csize = u64(ev, r); r += 8; }
        if (off === 0xffffffff) { off = u64(ev, r); }
      }
      q += 4 + sz;
    }
    const { name, source } = decodeEntryName(raw, flags, extra);
    out.push({ index: i, name, nameSource: source, isDir: name.endsWith('/') || name.endsWith('\\'), flags, method, crc32: crc, compressedSize: csize, size: usize, localOffset: off, mtime: dosDate(date, time), encrypted: (flags & 1) !== 0 });
    p += 46 + fnl + exl + cml;
  }
  return out;
}

/** where the entry's data starts (the local header has its own name and extra lengths) */
export async function dataStart(blob: Blob, e: ZipEntry): Promise<number> {
  const h = await bytesAt(blob, e.localOffset, e.localOffset + 30);
  const hv = new DataView(h.buffer, h.byteOffset, h.byteLength);
  if (h.length < 30 || u32(hv, 0) !== 0x04034b50) throw new ZipError(`Entry ${e.index}: the local header is damaged.`);
  return e.localOffset + 30 + u16(hv, 26) + u16(hv, 28);
}

/** the entry's bytes as a stream (stored or inflated); throws ZipError for a method the browser cannot inflate */
export async function entryStream(blob: Blob, e: ZipEntry): Promise<ReadableStream<Uint8Array>> {
  if (e.encrypted) throw new ZipError('encrypted entry');
  const start = await dataStart(blob, e);
  if (start + e.compressedSize > blob.size) throw new ZipError('the entry runs past the end of the file');
  const raw = blob.slice(start, start + e.compressedSize).stream() as ReadableStream<Uint8Array>;
  if (e.method === 0) return raw;
  if (e.method === 8) {
    if (typeof DecompressionStream === 'undefined') throw new ZipError('this browser has no DecompressionStream');
    return raw.pipeThrough(new DecompressionStream('deflate-raw') as unknown as ReadableWritablePair<Uint8Array, Uint8Array>);
  }
  throw new ZipError(`compression method ${e.method} is not supported (only stored and deflate)`);
}
