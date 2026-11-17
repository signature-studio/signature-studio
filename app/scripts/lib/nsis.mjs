// Reads an NSIS 3 installer in place (no temporary file): its header (the script, LZMA-compressed after the first header) and the streams of its files.
// The same reading as tools/extract_nsis2.py (read_header, parse_blocks, extract_files, stream_file); the script decoding is in nsis-script.mjs.
import fs from 'node:fs';
import { lzmaDecode } from './lzma1.mjs';

const MAGIC = Buffer.concat([Buffer.from([0xef, 0xbe, 0xad, 0xde]), Buffer.from('NullsoftInst')]);
const EW_EXTRACTFILE = 20;

export class Installer {
  constructor(file) {
    this.fd = fs.openSync(file, 'r');
    this.size = fs.fstatSync(this.fd).size;
  }

  read(pos, n) {
    const b = Buffer.alloc(n);
    let got = 0;
    while (got < n) {
      const r = fs.readSync(this.fd, b, got, n - got, pos + got);
      if (r === 0) throw new Error(`unexpected end of the installer at ${pos + got}`);
      got += r;
    }
    return b;
  }

  close() {
    fs.closeSync(this.fd);
  }
}

function findFirstHeader(inst) {
  const chunk = 1 << 24;
  let pos = 0, prev = Buffer.alloc(0);
  while (pos < inst.size) {
    const b = inst.read(pos, Math.min(chunk, inst.size - pos));
    const d = Buffer.concat([prev, b]);
    const i = d.indexOf(MAGIC);
    if (i >= 0) return pos - prev.length + i - 4;
    prev = d.subarray(Math.max(0, d.length - (MAGIC.length - 1)));
    pos += b.length;
  }
  throw new Error('NSIS firstheader signature not found');
}

/** the decompressed header (the script) and the offsets of the installer: { hdr, info } */
export function readHeader(inst) {
  const fhOff = findFirstHeader(inst);
  const fh = inst.read(fhOff, 28);
  const infHdrLen = fh.readUInt32LE(20), totalLen = fh.readUInt32LE(24);
  let pos = fhOff + 28, skipped = 0;
  while (inst.read(pos, 4).readUInt32LE(0) === 0) {
    pos += 4;
    skipped += 4;
    if (skipped > 64) throw new Error('too much zero padding after firstheader');
  }
  const word = inst.read(pos, 4).readUInt32LE(0);
  if ((word >>> 31) === 0) throw new Error(`header is not LZMA-compressed (word=${word.toString(16)})`);
  const clen = word & 0x7fffffff;
  const hdr = lzmaDecode(inst.read(pos + 4, 5), inst.read(pos + 9, clen - 5), infHdrLen);
  if (hdr.length !== infHdrLen) throw new Error(`header inflated to ${hdr.length}, expected ${infHdrLen}`);
  const info = { fhOff, skipped, clen, infHdrLen, totalLen, dataStart: pos + 4 + clen };
  return { hdr, info };
}

export function parseBlocks(hdr) {
  const flags = hdr.readUInt32LE(0);
  const blocks = [];
  for (let i = 0; i < 8; i++) blocks.push([hdr.readUInt32LE(4 + 8 * i), hdr.readUInt32LE(8 + 8 * i)]);
  return { flags, blocks };
}

export function entrySize(blocks) {
  const [off, num] = blocks[2];
  const span = blocks[3][0] - off;
  if (num === 0 || span % num) throw new Error(`cannot derive entry size: span=${span} num=${num}`);
  return span / num;
}

/** the string of the Unicode string table at index idx, with the two-unit escapes of the extractor (tools/extract_nsis2.py read_string) */
export function readString(hdr, stringsOff, idx) {
  let pos = stringsOff + idx * 2;
  let out = '';
  for (;;) {
    const u = hdr.readUInt16LE(pos);
    pos += 2;
    if (u === 0) break;
    if (u === 0xe000) {
      out += String.fromCharCode(hdr.readUInt16LE(pos));
      pos += 2;
    } else if (u === 0xe001 || u === 0xe002 || u === 0xe003) {
      pos += 2;
      out += `<${u.toString(16).toUpperCase()}>`;
    } else {
      out += String.fromCharCode(u);
    }
  }
  return out;
}

/** [entry number, file name, stream offset] of every EXTRACTFILE entry of the script */
export function extractFiles(hdr, blocks) {
  const stringsOff = blocks[3][0];
  const [off, num] = blocks[2];
  const es = entrySize(blocks);
  const rows = [];
  for (let i = 0; i < num; i++) {
    const at = off + i * es;
    if (hdr.readInt32LE(at) === EW_EXTRACTFILE) {
      rows.push([i, readString(hdr, stringsOff, hdr.readInt32LE(at + 8)), hdr.readUInt32LE(at + 12)]);
    }
  }
  return rows;
}

/** the bytes of the file whose stream is at `offset` (from dataStart): stored, or LZMA1 */
export function streamFile(inst, dataStart, offset) {
  const word = inst.read(dataStart + offset, 4).readUInt32LE(0);
  const clen = word & 0x7fffffff;
  if ((word >>> 31) === 0) return inst.read(dataStart + offset + 4, clen);
  return lzmaDecode(inst.read(dataStart + offset + 4, 5), inst.read(dataStart + offset + 9, clen - 5));
}
