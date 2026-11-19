// A minimal zip writer for the tests (decision 0216, opfs1): the raw name bytes, the flags and the extra fields are chosen by the test, which no zip tool lets one do.
import zlib from 'node:zlib';

export interface ZipItem { name: Buffer; data: Buffer; method?: number; flags?: number; extra?: Buffer; crc?: number }

const crcTable = (() => { const t = new Uint32Array(256); for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; } return t; })();
export function crc32(buf: Uint8Array): number { let c = 0xffffffff; for (const b of buf) c = crcTable[(c ^ b) & 0xff]! ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; }

/** entries: { name: Buffer (raw bytes), data: Buffer, method?: 0|8, flags?: number, extra?: Buffer, crc?: number (override: for a damaged zip) } */
export function makeZip(entries: ZipItem[]): Buffer {
  const parts: Buffer[] = [], cd: Buffer[] = [];
  let off = 0;
  for (const e of entries) {
    const method = e.method ?? 8;
    const comp = method === 8 ? zlib.deflateRawSync(e.data) : e.data;
    const crc = e.crc ?? crc32(e.data);
    const flags = e.flags ?? 0;
    const extra = e.extra ?? Buffer.alloc(0);
    const lh = Buffer.alloc(30);
    lh.writeUInt32LE(0x04034b50, 0); lh.writeUInt16LE(20, 4); lh.writeUInt16LE(flags, 6); lh.writeUInt16LE(method, 8); lh.writeUInt16LE(0x6000, 10); lh.writeUInt16LE(0x5821, 12);
    lh.writeUInt32LE(crc, 14); lh.writeUInt32LE(comp.length, 18); lh.writeUInt32LE(e.data.length, 22); lh.writeUInt16LE(e.name.length, 26); lh.writeUInt16LE(extra.length, 28);
    parts.push(lh, e.name, extra, comp);
    const ch = Buffer.alloc(46);
    ch.writeUInt32LE(0x02014b50, 0); ch.writeUInt16LE(20, 4); ch.writeUInt16LE(20, 6); ch.writeUInt16LE(flags, 8); ch.writeUInt16LE(method, 10); ch.writeUInt16LE(0x6000, 12); ch.writeUInt16LE(0x5821, 14);
    ch.writeUInt32LE(crc, 16); ch.writeUInt32LE(comp.length, 20); ch.writeUInt32LE(e.data.length, 24); ch.writeUInt16LE(e.name.length, 28); ch.writeUInt16LE(extra.length, 30);
    ch.writeUInt32LE(off, 42);
    cd.push(ch, e.name, extra);
    off += 30 + e.name.length + extra.length + comp.length;
  }
  const cdBuf = Buffer.concat(cd);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0); end.writeUInt16LE(entries.length, 8); end.writeUInt16LE(entries.length, 10); end.writeUInt32LE(cdBuf.length, 12); end.writeUInt32LE(off, 16);
  return Buffer.concat([...parts, cdBuf, end]);
}

/** the Info-ZIP Unicode Path extra field (0x7075) for a header name */
export function unicodePathExtra(rawName: Buffer, utf8Name: string): Buffer {
  const u = Buffer.from(utf8Name, 'utf8');
  const b = Buffer.alloc(4 + 5 + u.length);
  b.writeUInt16LE(0x7075, 0); b.writeUInt16LE(5 + u.length, 2); b[4] = 1; b.writeUInt32LE(crc32(rawName), 5); u.copy(b, 9);
  return b;
}
