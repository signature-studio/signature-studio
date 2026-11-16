// Decision 0314 (PROVISIONAL): the reader of the lazy install tree's index (FLVFSIX1 v1; the grammar is docs/decisions/0314-lazy-install-tree.md 'Format'). The C side (lift/harness/vfs_lazy.h) reads the same bytes
// for the files and directories; the page needs the STREAMS only (names, coding, chunk size, the SHA-256 of every chunk), so `entries: false` (the default) stops after the stream table. A full parse (`entries: true`) is
// for tests and tools.
//
// Limits (the same as the C reader): 16 streams, 65,536 entries, a chunk of 1 B .. 8 MiB, a stream name of [A-Za-z0-9._-], at most 64 characters. Every read is bounded: a malformed index throws VfsIndexError by name.

export const VFS_MAX_STREAMS = 16;
export const VFS_MAX_ENTRIES = 65536;
export const VFS_MAX_CHUNK = 8 * 1024 * 1024;
export const VFS_MAX_INDEX_BYTES = 16 * 1024 * 1024;
const MAGIC = [0x46, 0x4c, 0x56, 0x46, 0x53, 0x49, 0x58, 0x31];   // 'FLVFSIX1'
const VFS_NAME_RE = /^[A-Za-z0-9._-]{1,64}$/;

export class VfsIndexError extends Error {
  constructor(what: string) { super(`The lazy install tree's index is not usable: ${what}.`); this.name = 'VfsIndexError'; }
}

export interface VfsStream {
  name: string;
  /** 0 = identity, 1 = brotli (the object is stored encoded and served with Content-Encoding: br; the fetch API hands the page the decoded bytes) */
  coding: 0 | 1;
  chunkSize: number;
  /** logical bytes of the stream = the sum of the sizes of its files */
  logical: number;
  /** SHA-256 (lower-case hex) of the LOGICAL bytes of every chunk; the object is `vfs/<hex>.chk` */
  chunks: string[];
}

export interface VfsEntry {
  kind: 'file' | 'dir';
  /** UTF-8 decoded, '/' separated, relative to the mount */
  path: string;
  /** FILETIME ticks (100 ns since 1601) */
  mtimeFt: bigint;
  attrs: number;
  /** files only */
  stream?: number;
  size?: number;
  /** files only: where the file starts in its stream's logical bytes (the running sum of the earlier files of that stream) */
  offset?: number;
}

export interface VfsIndex {
  version: number;
  flags: number;
  smallMax: number;
  baseFt: bigint;
  /** the guest directory the entries are relative to, backslashes, e.g. C:\Program Files\Image-Line\FL Studio 21 */
  mount: string;
  streams: VfsStream[];
  nDirs: number;
  nFiles: number;
  entries: VfsEntry[];
}

class Cursor {
  i = 0;
  readonly b: Uint8Array;
  readonly dv: DataView;
  constructor(b: Uint8Array) { this.b = b; this.dv = new DataView(b.buffer, b.byteOffset, b.byteLength); }
  need(n: number, what: string): void { if (n > this.b.length - this.i) throw new VfsIndexError(`cut short in ${what}`); }
  u8(what: string): number { this.need(1, what); return this.b[this.i++] as number; }
  u32(what: string): number { this.need(4, what); const v = this.dv.getUint32(this.i, true); this.i += 4; return v; }
  u64(what: string): bigint { this.need(8, what); const v = this.dv.getBigUint64(this.i, true); this.i += 8; return v; }
  varint(what: string): number {
    let v = 0, mul = 1;
    for (let k = 0; k < 8; k++) {
      const c = this.u8(what);
      v += (c & 0x7f) * mul;
      if (!(c & 0x80)) return v;
      mul *= 128;
    }
    throw new VfsIndexError(`a number in ${what} is longer than 8 bytes of 7 bits`);
  }
  take(n: number, what: string): Uint8Array { this.need(n, what); const r = this.b.subarray(this.i, this.i + n); this.i += n; return r; }
}

const hex = (b: Uint8Array): string => { let s = ''; for (const x of b) s += x.toString(16).padStart(2, '0'); return s; };

export function parseIndex(bytes: Uint8Array, opts: { entries?: boolean } = {}): VfsIndex {
  if (bytes.length > VFS_MAX_INDEX_BYTES) throw new VfsIndexError(`${bytes.length} bytes is over the ${VFS_MAX_INDEX_BYTES} limit`);
  const c = new Cursor(bytes);
  const td = new TextDecoder('utf-8', { fatal: true });
  if (bytes.length < 8 || !MAGIC.every((c, i) => bytes[i] === c)) throw new VfsIndexError('it does not start with FLVFSIX1');
  c.i = 8;
  const version = c.u32('the header'), flags = c.u32('the header'), smallMax = c.u32('the header'), baseFt = c.u64('the header');
  const ns = c.varint('the header'), nd = c.varint('the header'), nf = c.varint('the header');
  const mountLen = c.varint('the header');
  let mount: string;
  try { mount = td.decode(c.take(mountLen, 'the mount')); } catch (e) { if (e instanceof VfsIndexError) throw e; throw new VfsIndexError('the mount is not valid UTF-8'); }
  if (version !== 1) throw new VfsIndexError(`version ${version} (this reader knows 1)`);
  if ((flags & ~1) !== 0) throw new VfsIndexError(`unknown flags 0x${flags.toString(16)}`);
  if (ns < 1 || ns > VFS_MAX_STREAMS) throw new VfsIndexError(`${ns} streams (1..${VFS_MAX_STREAMS} are supported)`);
  if (nd > VFS_MAX_ENTRIES || nf > VFS_MAX_ENTRIES || nd + nf > VFS_MAX_ENTRIES) throw new VfsIndexError(`${nd} directories and ${nf} files (at most ${VFS_MAX_ENTRIES} entries)`);
  const streams: VfsStream[] = [];
  const seen = new Set<string>();
  for (let s = 0; s < ns; s++) {
    const nameLen = c.varint(`stream ${s}`);
    const name = nameLen > 64 ? '' : new TextDecoder().decode(c.take(nameLen, `stream ${s}`));
    const coding = c.u8(`stream ${s}`);
    const chunkSize = c.varint(`stream ${s}`), nChunks = c.varint(`stream ${s}`), logical = c.varint(`stream ${s}`);
    if (!VFS_NAME_RE.test(name) || name === '.' || name === '..') throw new VfsIndexError(`stream ${s} has a name outside [A-Za-z0-9._-]{1,64}`);
    if (seen.has(name)) throw new VfsIndexError(`stream name ${name} is used twice`);
    seen.add(name);
    if (coding > 1) throw new VfsIndexError(`stream ${name} has coding ${coding} (0 identity, 1 brotli)`);
    if (chunkSize < 1 || chunkSize > VFS_MAX_CHUNK) throw new VfsIndexError(`stream ${name} has chunk size ${chunkSize} (1..${VFS_MAX_CHUNK})`);
    if (nChunks !== Math.ceil(logical / chunkSize)) throw new VfsIndexError(`stream ${name} has ${nChunks} chunks for ${logical} bytes of ${chunkSize} per chunk`);
    if (nChunks > (c.b.length - c.i) / 32) throw new VfsIndexError(`cut short in the chunk hashes of stream ${name}`);
    const chunks: string[] = [];
    for (let k = 0; k < nChunks; k++) chunks.push(hex(c.take(32, `the chunk hashes of ${name}`)));
    streams.push({ name, coding: coding as 0 | 1, chunkSize, logical, chunks });
  }
  const index: VfsIndex = { version, flags, smallMax, baseFt, mount, streams, nDirs: nd, nFiles: nf, entries: [] };
  if (!opts.entries) return index;

  const cursor = new Array<number>(ns).fill(0);
  let prev = new Uint8Array(0);
  let seenDirs = 0, seenFiles = 0;
  for (let e = 0; e < nd + nf; e++) {
    const kind = c.u8(`entry ${e}`);
    const common = c.varint(`entry ${e}`), sufLen = c.varint(`entry ${e}`);
    if (common > prev.length) throw new VfsIndexError(`entry ${e} shares ${common} bytes with a path of ${prev.length}`);
    if (kind > 1) throw new VfsIndexError(`entry ${e} has kind ${kind}`);
    const suffix = c.take(sufLen, `entry ${e}`);
    const zz = c.varint(`entry ${e}`);
    const attrs = c.u8(`entry ${e}`);
    const pb = new Uint8Array(common + sufLen);
    pb.set(prev.subarray(0, common));
    pb.set(suffix, common);
    prev = pb;
    let path: string;
    try { path = td.decode(pb); } catch { throw new VfsIndexError(`entry ${e} is not valid UTF-8`); }
    const delta = zz % 2 === 0 ? zz / 2 : -(zz + 1) / 2;
    const mtimeFt = baseFt + BigInt(delta) * 10000000n;
    if (kind === 1) {
      index.entries.push({ kind: 'dir', path, mtimeFt, attrs });
      seenDirs++;
      continue;
    }
    const stream = c.u8(`entry ${e}`), size = c.varint(`entry ${e}`);
    if (flags & 1) c.take(32, `entry ${e}`);
    const st = streams[stream];
    if (!st) throw new VfsIndexError(`file ${path} names stream ${stream} of ${ns}`);
    const offset = cursor[stream] as number;
    if (size > st.logical - offset) throw new VfsIndexError(`file ${path} runs past the end of stream ${st.name}`);
    cursor[stream] = offset + size;
    index.entries.push({ kind: 'file', path, mtimeFt, attrs, stream, size, offset });
    seenFiles++;
  }
  if (seenDirs !== nd || seenFiles !== nf) throw new VfsIndexError(`the header says ${nd} directories and ${nf} files, the entries are ${seenDirs} and ${seenFiles}`);
  if (c.i !== c.b.length) throw new VfsIndexError(`${c.b.length - c.i} bytes after the last entry`);
  for (let s = 0; s < ns; s++) if (cursor[s] !== (streams[s] as VfsStream).logical) throw new VfsIndexError(`the files of stream ${(streams[s] as VfsStream).name} add up to ${cursor[s]} bytes, the stream has ${(streams[s] as VfsStream).logical}`);
  return index;
}
