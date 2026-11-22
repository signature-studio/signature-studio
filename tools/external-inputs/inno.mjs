// Reads the files of an Inno Setup 6.7.0 installer in place (no program is run): the offset table in the loader, the setup data (the header and the file entries in the first
// compressed block, the file locations in the second), and the file data in the embedded slice, decoded with LZMA2 (lzma2.mjs) and checked by the SHA-256 of each file.
// The layout is that of Inno Setup's Projects/Src/Shared.Struct.pas, Setup.MainFunc.pas, Compression.Base.pas and Setup.FileExtractor.pas at tag is-6_7_0.
import crypto from 'node:crypto';
import { lzmaDecode } from '../../app/scripts/lib/lzma1.mjs';
import { lzma2Decode } from './lzma2.mjs';

const OFFSET_TABLE_ID = Buffer.from([0x72, 0x44, 0x6c, 0x50, 0x74, 0x53, 0xcd, 0xe6, 0xd7, 0x7b, 0x0b, 0x2a]);
const OFFSET_TABLE_VERSION = 2;
const SETUP_ID = 'Inno Setup Setup Data (6.7.0)';
const ZLIB_ID = Buffer.from('zlb\x1a', 'latin1');
const CHUNK = 4096;
const LOCATION_SIZE = 89;
const FLAG_CALL_INSTRUCTION_OPTIMIZED = 4;
const FLAG_CHUNK_COMPRESSED = 16;
const CMP_LZMA2 = 4;

const CRC_TABLE = (() => {
  const t = new Int32Array(256);
  for (let i = 0; i < 256; i++) {
    let c = i;
    for (let k = 0; k < 8; k++) c = c & 1 ? (c >>> 1) ^ 0xedb88320 : c >>> 1;
    t[i] = c;
  }
  return t;
})();

export function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

/** the data of a compressed block: its chunks are [CRC32 4 bytes][at most 4096 bytes], the CRC of each checked */
function unchunk(buf, start, total) {
  const parts = [];
  let q = start, left = total;
  while (left > 0) {
    if (left < 5) throw new Error('Inno: a compressed block is cut short');
    const crc = buf.readUInt32LE(q);
    q += 4;
    left -= 4;
    const len = Math.min(left, CHUNK);
    const data = buf.subarray(q, q + len);
    if (crc32(data) !== crc) throw new Error('Inno: a compressed block chunk fails its CRC32');
    parts.push(data);
    q += len;
    left -= len;
  }
  return Buffer.concat(parts);
}

/** a compressed block at `at` (CRC32, stored size, compressed flag, then the chunks): the decoded bytes and the offset after it */
function readBlock(buf, at) {
  const head = buf.subarray(at + 4, at + 13);
  if (crc32(head) !== buf.readUInt32LE(at)) throw new Error('Inno: a compressed block header fails its CRC32');
  const stored = Number(buf.readBigUInt64LE(at + 4));
  const compressed = head[8] === 1;
  const body = unchunk(buf, at + 13, stored);
  const data = compressed ? lzmaDecode(body.subarray(0, 5), body.subarray(5)) : body;
  return { data, next: at + 13 + stored };
}

class Cursor {
  constructor(buf) {
    this.buf = buf;
    this.p = 0;
  }

  text(enc) {
    const n = this.buf.readUInt32LE(this.p);
    this.p += 4;
    const s = this.buf.toString(enc, this.p, this.p + n);
    this.p += n;
    return s;
  }

  skip(n) {
    this.p += n;
  }

  int() {
    const v = this.buf.readInt32LE(this.p);
    this.p += 4;
    return v;
  }
}

/**
 * The entries that the file extraction needs. Each record is its strings (UTF-16 as Unicode strings, bytes as AnsiStrings; each with a
 * 4-byte length), then its fixed part, whose size is the 32-bit Delphi size of the packed record minus the pointers of its strings.
 */
function readEntries(h) {
  const c = new Cursor(h);
  for (let i = 0; i < 39; i++) c.text('utf16le');
  for (let i = 0; i < 4; i++) c.text('latin1');
  const counts = [];
  for (let i = 0; i < 17; i++) counts.push(c.int());
  const [numLang, numMsg, numPerm, numType, numComp, numTask, numDir, numISSig, numFile, numLoc] = counts;
  c.skip(69 + 6);
  const compressMethod = h[c.p];
  c.skip(1 + 2 + 8 + 8);
  for (let i = 0; i < numLang; i++) {
    for (let k = 0; k < 4; k++) c.text('utf16le');
    for (let k = 0; k < 4; k++) c.text('latin1');
    c.skip(19);
  }
  for (let i = 0; i < numMsg; i++) {
    c.text('utf16le');
    c.text('utf16le');
    c.skip(4);
  }
  for (let i = 0; i < numPerm; i++) c.text('latin1');
  for (let i = 0; i < numType; i++) {
    for (let k = 0; k < 4; k++) c.text('utf16le');
    c.skip(30);
  }
  for (let i = 0; i < numComp; i++) {
    for (let k = 0; k < 5; k++) c.text('utf16le');
    c.skip(39);
  }
  for (let i = 0; i < numTask; i++) {
    for (let k = 0; k < 6; k++) c.text('utf16le');
    c.skip(23);
  }
  for (let i = 0; i < numDir; i++) {
    for (let k = 0; k < 7; k++) c.text('utf16le');
    c.skip(27);
  }
  for (let i = 0; i < numISSig; i++) for (let k = 0; k < 3; k++) c.text('utf16le');
  const files = [];
  for (let i = 0; i < numFile; i++) {
    const dest = [];
    for (let k = 0; k < 15; k++) dest.push(c.text('utf16le'));
    c.text('latin1');
    c.skip(33 + 20);
    const loc = c.int();
    c.skip(4 + 8 + 2 + 8 + 1);
    files.push({ source: dest[0], dest: dest[1], loc });
  }
  return { compressMethod, numLoc, files };
}

/** undoes the CALL/JMP transform of Compression.Base.pas (TransformCallInstructions), in pieces of 65536 bytes as Setup.FileExtractor.pas does */
function untransformCalls(buf) {
  for (let base = 0; base < buf.length; base += 65536) {
    const end = Math.min(base + 65536, buf.length);
    const size = end - base;
    if (size < 5) continue;
    const lim = size - 4;
    let i = 0;
    while (i < lim) {
      if (buf[base + i] === 0xe8 || buf[base + i] === 0xe9) {
        i++;
        const hi = base + i + 3;
        if (buf[hi] === 0 || buf[hi] === 0xff) {
          const addr = (base + i + 4) & 0xffffff;
          let rel = buf[base + i] | (buf[base + i + 1] << 8) | (buf[base + i + 2] << 16);
          rel = (rel - addr) >>> 0;
          if (rel & 0x800000) buf[hi] = ~buf[hi] & 0xff;
          buf[base + i] = rel & 0xff;
          buf[base + i + 1] = (rel >>> 8) & 0xff;
          buf[base + i + 2] = (rel >>> 16) & 0xff;
        }
        i += 4;
      } else {
        i++;
      }
    }
  }
}

/** the offset table of the loader: the first copy of the ID whose version and CRC32 are right (the ID may also be in the code) */
function findOffsetTable(buf) {
  let from = 0;
  for (;;) {
    const at = buf.indexOf(OFFSET_TABLE_ID, from);
    if (at < 0 || at + 64 > buf.length) throw new Error('Inno: no offset table (not an Inno Setup loader)');
    if (buf.readUInt32LE(at + 12) === OFFSET_TABLE_VERSION && crc32(buf.subarray(at, at + 60)) === buf.readUInt32LE(at + 60)) return at;
    from = at + 1;
  }
}

/**
 * The files of the Inno Setup installer `buf`: [{ dest, data }], dest as the installer names it (with its {constant} prefix), data checked by the SHA-256 of
 * the file entry. Throws when the installer is not Inno Setup 6.7.0 (the layout above), is encrypted, or fails a check.
 */
export function innoFiles(buf) {
  const at = findOffsetTable(buf);
  const offset0 = Number(buf.readBigInt64LE(at + 40));
  const offset1 = Number(buf.readBigInt64LE(at + 48));
  if (offset1 === 0) throw new Error('Inno: the file data is in separate slice files (not supported)');
  const id = buf.toString('latin1', offset0, offset0 + SETUP_ID.length);
  if (id !== SETUP_ID) throw new Error(`Inno: setup data is not ${SETUP_ID}`);
  const encHdr = buf.subarray(offset0 + 68, offset0 + 68 + 49);
  if (crc32(encHdr) !== buf.readUInt32LE(offset0 + 64)) throw new Error('Inno: the encryption header fails its CRC32');
  if (encHdr[0] !== 0) throw new Error('Inno: the installer is encrypted (not supported)');

  const b1 = readBlock(buf, offset0 + 117);
  const { compressMethod, numLoc, files } = readEntries(b1.data);
  if (compressMethod !== CMP_LZMA2) throw new Error(`Inno: compression ${compressMethod} is not LZMA2 (not supported)`);
  const b2 = readBlock(buf, b1.next);
  if (b2.data.length !== numLoc * LOCATION_SIZE) throw new Error('Inno: the file locations are not as long as their count');
  const locs = [];
  for (let i = 0; i < numLoc; i++) {
    const q = i * LOCATION_SIZE, d = b2.data;
    locs.push({
      firstSlice: d.readInt32LE(q), lastSlice: d.readInt32LE(q + 4),
      startOffset: Number(d.readBigInt64LE(q + 8)), chunkSuboffset: Number(d.readBigInt64LE(q + 16)),
      originalSize: Number(d.readBigInt64LE(q + 24)), chunkCompressedSize: Number(d.readBigInt64LE(q + 32)),
      sha256: d.subarray(q + 40, q + 72).toString('hex'), flags: d[q + 88],
    });
  }

  const chunks = new Map();
  const chunkOf = (loc) => {
    const key = `${loc.firstSlice}:${loc.startOffset}`;
    if (chunks.has(key)) return chunks.get(key);
    if (loc.firstSlice !== 0 || loc.lastSlice !== 0) throw new Error('Inno: a file across slices (not supported)');
    const p = offset1 + loc.startOffset;
    if (!buf.subarray(p, p + 4).equals(ZLIB_ID)) throw new Error('Inno: a chunk does not start with its CompID');
    const body = buf.subarray(p + 4, p + 4 + loc.chunkCompressedSize);
    const data = loc.flags & FLAG_CHUNK_COMPRESSED ? lzma2Decode(body) : body;
    chunks.set(key, data);
    return data;
  };

  return files.filter((f) => f.loc >= 0).map((f) => {
    const loc = locs[f.loc];
    if (!loc) throw new Error(`Inno: the file ${f.dest} has no location`);
    const chunk = chunkOf(loc);
    if (loc.chunkSuboffset + loc.originalSize > chunk.length) throw new Error(`Inno: the chunk is shorter than the file ${f.dest}`);
    const data = Buffer.from(chunk.subarray(loc.chunkSuboffset, loc.chunkSuboffset + loc.originalSize));
    if (loc.flags & FLAG_CALL_INSTRUCTION_OPTIMIZED) untransformCalls(data);
    if (crypto.createHash('sha256').update(data).digest('hex') !== loc.sha256) throw new Error(`Inno: ${f.dest} fails its SHA-256`);
    return { dest: f.dest, data };
  });
}
