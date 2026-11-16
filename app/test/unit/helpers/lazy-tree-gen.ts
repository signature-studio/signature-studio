// Test helper for decision 0314: makes a small lazy install tree (the FLVFSIX1 index, the chunk objects `vfs/<sha256>.chk`) from a list of files, with the grammar written independently of src/engine/vfs-index.ts.
import { brotliCompressSync, brotliDecompressSync } from 'node:zlib';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import type { ChunkStore } from '../../../src/engine/chunk-store.ts';

export const BASE_FT = 134344736000000000n;
export const MOUNT = 'C:\\Program Files\\Image-Line\\FL Studio 21';

export interface GenFile { kind: 'file'; path: string; data: Uint8Array; dsec: number; attrs: number; stream: number }
export interface GenDir { kind: 'dir'; path: string; dsec: number; attrs: number }
export type GenEntry = GenFile | GenDir;
export interface GenStream { name: string; coding: 0 | 1; chunkSize: number }

export const pattern = (n: number, seed: number): Uint8Array => Uint8Array.from({ length: n }, (_, i) => (i * 13 + seed * 31 + 7) & 0xff);

const varint = (v: number): number[] => { const o: number[] = []; while (v >= 0x80) { o.push((v % 128) | 0x80); v = Math.floor(v / 128); } o.push(v); return o; };
const le = (v: bigint | number, n: number): number[] => { let x = BigInt(v); const o: number[] = []; for (let i = 0; i < n; i++) { o.push(Number(x & 0xffn)); x >>= 8n; } return o; };
const sha = (b: Uint8Array): Buffer => createHash('sha256').update(b).digest();

export interface GenResult { index: Uint8Array; streams: Uint8Array[]; chunks: Map<string, Uint8Array> }

/** the index bytes, the logical bytes of each stream, and the chunk objects as stored (hex name -> stored bytes) */
export function generate(entries: GenEntry[], streams: GenStream[], mount = MOUNT, flags = 0): GenResult {
  const logical = streams.map(() => [] as Uint8Array[]);
  for (const e of entries) if (e.kind === 'file') logical[e.stream]!.push(e.data);
  const joined = logical.map((parts) => { const n = parts.reduce((a, p) => a + p.length, 0); const o = new Uint8Array(n); let at = 0; for (const p of parts) { o.set(p, at); at += p.length; } return o; });
  const chunks = new Map<string, Uint8Array>();
  const out: number[] = [...Buffer.from('FLVFSIX1'), ...le(1, 4), ...le(flags, 4), ...le(262144, 4), ...le(BASE_FT, 8)];
  const nd = entries.filter((e) => e.kind === 'dir').length, nf = entries.length - nd;
  const mountB = Buffer.from(mount, 'utf8');
  out.push(...varint(streams.length), ...varint(nd), ...varint(nf), ...varint(mountB.length), ...mountB);
  streams.forEach((s, si) => {
    const data = joined[si]!;
    const nameB = Buffer.from(s.name);
    const n = Math.ceil(data.length / s.chunkSize);
    out.push(...varint(nameB.length), ...nameB, s.coding, ...varint(s.chunkSize), ...varint(n), ...varint(data.length));
    for (let c = 0; c < n; c++) {
      const piece = data.subarray(c * s.chunkSize, Math.min(data.length, (c + 1) * s.chunkSize));
      const h = sha(piece);
      out.push(...h);
      chunks.set(h.toString('hex'), s.coding === 1 ? new Uint8Array(brotliCompressSync(piece)) : piece);
    }
  });
  let prev = Buffer.alloc(0);
  for (const e of entries) {
    const p = Buffer.from(e.path, 'utf8');
    let common = 0;
    while (common < prev.length && common < p.length && prev[common] === p[common]) common++;
    out.push(e.kind === 'file' ? 0 : 1, ...varint(common), ...varint(p.length - common), ...p.subarray(common), ...varint(e.dsec >= 0 ? e.dsec * 2 : -e.dsec * 2 - 1), e.attrs);
    if (e.kind === 'file') {
      out.push(e.stream, ...varint(e.data.length));
      if (flags & 1) out.push(...sha(e.data));
    }
    prev = p;
  }
  return { index: Uint8Array.from(out), streams: joined, chunks };
}

export function writeTree(dir: string, g: GenResult): void {
  mkdirSync(join(dir, 'vfs'), { recursive: true });
  for (const [h, b] of g.chunks) writeFileSync(join(dir, 'vfs', `${h}.chk`), b);
}

/** a ChunkStore over a directory of stored objects (brotli ones are decoded here, as the browser's fetch does) */
export class DirChunkStore implements ChunkStore {
  gets = 0;
  readonly dir: string;
  constructor(dir: string) { this.dir = dir; }
  async get(shaHex: string, coding: 0 | 1, length: number): Promise<Uint8Array> {
    this.gets++;
    const stored = readFileSync(join(this.dir, 'vfs', `${shaHex}.chk`));
    const b = coding === 1 ? new Uint8Array(brotliDecompressSync(stored)) : new Uint8Array(stored);
    if (b.length !== length) throw new Error(`chunk ${shaHex}: ${b.length} bytes, expected ${length}`);
    if (sha(b).toString('hex') !== shaHex) throw new Error(`chunk ${shaHex}: hash mismatch`);
    return b;
  }
}

/** the files of the standard test tree: streams small-bin (identity, 1 KiB chunks) and large-pcm (brotli, 4 KiB chunks) */
export function standardTree(): { entries: GenEntry[]; streams: GenStream[] } {
  return {
    streams: [{ name: 'small-bin', coding: 0, chunkSize: 1024 }, { name: 'large-pcm', coding: 1, chunkSize: 4096 }],
    entries: [
      { kind: 'dir', path: 'Data', dsec: 5, attrs: 0 },
      { kind: 'dir', path: 'Data/Empty folder', dsec: 6, attrs: 0 },
      { kind: 'dir', path: 'Data/Patches', dsec: 7, attrs: 0 },
      { kind: 'file', path: 'Data/Patches/a.txt', data: pattern(10, 1), dsec: 100, attrs: 0x20, stream: 0 },
      { kind: 'file', path: 'Data/Patches/zero.bin', data: new Uint8Array(0), dsec: 101, attrs: 0x20, stream: 0 },
      { kind: 'file', path: 'Data/Patches/日本語.txt', data: pattern(5, 2), dsec: -50, attrs: 0x21, stream: 0 },
      { kind: 'file', path: 'Data/b.bin', data: pattern(3000, 3), dsec: 3, attrs: 0x20, stream: 0 },
      { kind: 'file', path: 'Data/big.dat', data: pattern(10000, 4), dsec: 4000, attrs: 0x20, stream: 1 },
      { kind: 'file', path: 'Data/exact.dat', data: pattern(8192, 5), dsec: 4001, attrs: 0x20, stream: 1 },
    ],
  };
}
