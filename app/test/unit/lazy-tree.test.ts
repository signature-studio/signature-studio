import { describe, expect, it } from 'vitest';
import { Worker } from 'node:worker_threads';
import { existsSync, mkdtempSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { brotliDecompressSync } from 'node:zlib';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { ChunkCache, ChunkedSource } from '../../src/engine/chunked-source.ts';
import { ChunkIoError, SabChunkClient, ERR_LENGTH, ERR_STORE, makeChunkIoShared, type ChunkIoShared } from '../../src/engine/chunk-io.ts';
import { ChunkStoreError, FetchChunkStore, sha256Hex } from '../../src/engine/chunk-store.ts';
import { HostFiles, HostFilesUnsupported, installHostFiles } from '../../src/engine/hostfiles.ts';
import { mountLazyTree } from '../../src/engine/lazy-tree.ts';
import { parseIndex, VfsIndexError, type VfsIndex } from '../../src/engine/vfs-index.ts';
import { BASE_FT, DirChunkStore, MOUNT, generate, pattern, standardTree, writeTree, type GenEntry } from './helpers/lazy-tree-gen.ts';
import { SyncDirClient } from './helpers/sync-dir-client.ts';

// The expected bytes are the original files of the generator (pattern(n, seed)), not anything the code under test produced.

function expectedFiles(entries: GenEntry[]): Map<string, Uint8Array> {
  const m = new Map<string, Uint8Array>();
  for (const e of entries) if (e.kind === 'file') m.set(e.path, e.data);
  return m;
}

function readAll(src: ChunkedSource, off: number, size: number, piece = 777): Uint8Array {
  const out = new Uint8Array(size);
  let at = 0;
  while (at < size) {
    const n = src.readInto(out.subarray(at, Math.min(size, at + piece)), off + at);
    if (n === 0) throw new Error(`short read at ${off + at}`);
    at += n;
  }
  return out;
}

describe('FLVFSIX1: the index reader', () => {
  const t = standardTree();
  const g = generate(t.entries, t.streams);

  it('stream table only, by default; full entries on request (names, times, attributes, offsets)', () => {
    const lite = parseIndex(g.index);
    expect(lite.entries).toEqual([]);
    expect(lite.mount).toBe(MOUNT);
    expect(lite.baseFt).toBe(BASE_FT);
    expect(lite.streams.map((s) => [s.name, s.coding, s.chunkSize, s.logical])).toEqual([['small-bin', 0, 1024, 3015], ['large-pcm', 1, 4096, 18192]]);
    expect(lite.streams[0]?.chunks.length).toBe(3);
    expect(lite.streams[1]?.chunks.length).toBe(5);
    const full = parseIndex(g.index, { entries: true });
    expect(full.nDirs).toBe(3);
    expect(full.nFiles).toBe(6);
    expect(full.entries.map((e) => e.path)).toEqual(t.entries.map((e) => e.path));
    const jp = full.entries.find((e) => e.path.endsWith('日本語.txt'));
    expect(jp).toMatchObject({ kind: 'file', stream: 0, size: 5, offset: 10, attrs: 0x21 });
    expect(jp?.mtimeFt).toBe(BASE_FT - 50n * 10000000n);
    const big = full.entries.find((e) => e.path === 'Data/big.dat');
    expect(big).toMatchObject({ stream: 1, size: 10000, offset: 0 });
    expect(full.entries.find((e) => e.path === 'Data/exact.dat')).toMatchObject({ offset: 10000, size: 8192 });
    expect(full.entries.find((e) => e.path === 'Data/Patches/zero.bin')).toMatchObject({ size: 0, offset: 10 });
  });

  it('a per-file SHA-256 (flags bit 0) is skipped over', () => {
    const f = generate(t.entries, t.streams, MOUNT, 1);
    expect(parseIndex(f.index, { entries: true }).entries.length).toBe(9);
  });

  it('every proper prefix is refused by name, and no single-byte change throws anything but VfsIndexError', () => {
    for (let k = 0; k < g.index.length; k++) expect(() => parseIndex(g.index.subarray(0, k), { entries: true })).toThrow(VfsIndexError);
    for (let k = 0; k < g.index.length; k++) {
      for (const x of [0xff, 0x80]) {
        const c = Uint8Array.from(g.index);
        c[k] = (c[k] as number) ^ x;
        try { parseIndex(c, { entries: true }); } catch (e) { expect(e).toBeInstanceOf(VfsIndexError); }
      }
    }
  });

  it('limits: 17 streams, a chunk over 8 MiB, a name outside [A-Za-z0-9._-], a wrong chunk count, files that do not add up', () => {
    const many = generate(t.entries, [...t.streams, ...Array.from({ length: 15 }, (_, i) => ({ name: `x${i}`, coding: 0 as const, chunkSize: 1024 }))]);
    expect(() => parseIndex(many.index)).toThrow(/17 streams/);
    const huge = generate(t.entries, [{ ...t.streams[0]!, chunkSize: 9 * 1024 * 1024 }, t.streams[1]!]);
    expect(() => parseIndex(huge.index)).toThrow(/chunk size/);
    const slash = generate(t.entries, [{ ...t.streams[0]!, name: 'a/b' }, t.streams[1]!]);
    expect(() => parseIndex(slash.index)).toThrow(/name/);
    const dots = generate(t.entries, [{ ...t.streams[0]!, name: '..' }, t.streams[1]!]);
    expect(() => parseIndex(dots.index)).toThrow(/name/);
    const bad = Uint8Array.from(g.index);
    bad.set([9], 8);   // version 9
    expect(() => parseIndex(bad)).toThrow(/version 9/);
  });
});

describe('ChunkedSource and the chunk cache', () => {
  const t = standardTree();
  const g = generate(t.entries, t.streams);
  const dir = mkdtempSync(join(tmpdir(), 'lazy-tree-'));
  writeTree(dir, g);
  const idx = parseIndex(g.index, { entries: true });
  const files = expectedFiles(t.entries);

  it('reads every file back, in odd pieces, across chunk boundaries (identity and brotli streams)', () => {
    const client = new SyncDirClient(dir, idx.streams);
    const cache = new ChunkCache(1 << 20);
    const srcs = idx.streams.map((s, i) => new ChunkedSource(s, i, client, cache));
    for (const e of idx.entries) {
      if (e.kind !== 'file') continue;
      const got = readAll(srcs[e.stream as number] as ChunkedSource, e.offset as number, e.size as number, 333);
      expect(Buffer.compare(Buffer.from(got), Buffer.from(files.get(e.path) as Uint8Array)), e.path).toBe(0);
    }
  });

  it('reads at the end and past it answer 0; a read that ends in the next chunk is one piece of two chunks', () => {
    const client = new SyncDirClient(dir, idx.streams);
    const src = new ChunkedSource(idx.streams[1] as typeof idx.streams[number], 1, client, new ChunkCache(1 << 20));
    expect(src.size).toBe(18192);
    expect(src.readInto(new Uint8Array(10), 18192)).toBe(0);
    expect(src.readInto(new Uint8Array(10), 99999)).toBe(0);
    const buf = new Uint8Array(100);
    expect(src.readInto(buf, 4090)).toBe(100);   // 6 bytes of chunk 0, 94 of chunk 1
    const all = generate(t.entries, t.streams).streams[1] as Uint8Array;
    expect(Buffer.compare(Buffer.from(buf), Buffer.from(all.subarray(4090, 4190)))).toBe(0);
    expect(client.fetches).toBe(2);
    expect(src.readInto(buf, 4100)).toBe(100);   // chunk 1 again: from the cache
    expect(client.fetches).toBe(2);
    expect(src.readInto(new Uint8Array(5000), 18000)).toBe(192);   // the short last chunk
  });

  it('the cache keeps within its budget (the chunk just read always stays) and counts hits, misses, evictions', () => {
    const client = new SyncDirClient(dir, idx.streams);
    const cache = new ChunkCache(5000);   // room for one 4096-byte chunk of the brotli stream
    const src = new ChunkedSource(idx.streams[1] as typeof idx.streams[number], 1, client, cache);
    for (const at of [0, 4096, 8192, 0]) src.readInto(new Uint8Array(1), at);
    expect(cache.stats.misses).toBe(4);
    expect(cache.stats.hits).toBe(0);
    expect(cache.stats.evictions).toBe(3);
    expect(cache.stats.heldBytes).toBeLessThanOrEqual(5000);
    src.readInto(new Uint8Array(1), 5);
    expect(cache.stats.hits).toBe(1);
  });

});

describe('FetchChunkStore', () => {
  const body = pattern(2048, 9);
  const sha = createHash('sha256').update(body).digest('hex');
  const resp = (b: Uint8Array, status = 200): Response => new Response(status === 200 ? (b as Uint8Array<ArrayBuffer>) : null, { status });

  it('gets vfs/<sha>.chk, checks length and hash, retries after a failure with the HTTP cache bypassed', async () => {
    const seen: Array<[string, RequestInit | undefined]> = [];
    let n = 0;
    const store = new FetchChunkStore('https://example.test/assets/x', {
      fetchFn: (async (u: string | URL | Request, init?: RequestInit) => { seen.push([String(u), init]); n++; return n === 1 ? resp(body, 500) : resp(body); }) as typeof fetch,
    });
    const got = await store.get(sha, 0, 2048);
    expect(Buffer.compare(Buffer.from(got), Buffer.from(body))).toBe(0);
    expect(seen[0]?.[0]).toBe(`https://example.test/assets/x/vfs/${sha}.chk`);
    expect(seen[1]?.[1]).toEqual({ cache: 'reload' });
    expect(store.retries).toBe(1);
    expect(store.fetched).toBe(1);
  });

  it('refuses bytes that do not hash to their name, a wrong length, and a name that is not a SHA-256', async () => {
    const wrong = pattern(2048, 10);
    const s1 = new FetchChunkStore('https://e.test/', { attempts: 2, fetchFn: (async () => resp(wrong)) as typeof fetch });
    await expect(s1.get(sha, 0, 2048)).rejects.toThrow(ChunkStoreError);
    const s2 = new FetchChunkStore('https://e.test/', { attempts: 1, fetchFn: (async () => resp(body)) as typeof fetch });
    await expect(s2.get(sha, 0, 2047)).rejects.toThrow(/the index says 2047/);
    await expect(s2.get('../etc/passwd', 0, 1)).rejects.toThrow(/not a SHA-256/);
    expect(await sha256Hex(body)).toBe(sha);
  });

  it('a network error is retried and then reported with the count of tries', async () => {
    const s = new FetchChunkStore('https://e.test/', { attempts: 3, fetchFn: (async () => { throw new Error('offline'); }) as typeof fetch });
    await expect(s.get(sha, 0, 2048)).rejects.toThrow(/offline.*after 3 tries/);
    expect(s.retries).toBe(2);
  });
});

function spawnIoWorker(shared: ChunkIoShared, streams: VfsIndex['streams'], dir: string, opts: { failSha?: string[]; delayMs?: number; wrongLength?: boolean } = {}): Promise<{ w: Worker; done: Promise<number> }> {
  return new Promise((resolve, reject) => {
    const w = new Worker(new URL('./helpers/chunk-io-node-worker.ts', import.meta.url), { workerData: { shared, streams, dir, ...opts } });
    let done!: (n: number) => void;
    const donep = new Promise<number>((r) => { done = r; });
    w.on('message', (m: { type: string; served?: number; message?: string }) => {
      if (m.type === 'ready') resolve({ w, done: donep });
      else if (m.type === 'done') done(m.served as number);
      else if (m.type === 'error') reject(new Error(m.message));
    });
    w.on('error', reject);
  });
}

describe('SabChunkClient + serveChunkIo in a real worker thread (Atomics.wait on both sides)', () => {
  const t = standardTree();
  const g = generate(t.entries, t.streams);
  const dir = mkdtempSync(join(tmpdir(), 'lazy-tree-io-'));
  writeTree(dir, g);
  const idx = parseIndex(g.index, { entries: true });
  const files = expectedFiles(t.entries);

  it('serves every chunk of both streams, in an order that revisits chunks, and the client copy is the decoded bytes', async () => {
    const shared = makeChunkIoShared(4096);
    const { w, done } = await spawnIoWorker(shared, idx.streams, dir);
    const client = new SabChunkClient(shared);
    for (const [si, s] of idx.streams.entries()) {
      const order = [...s.chunks.keys()];
      for (const c of [...order, ...order.slice().reverse()]) {
        const b = client.fetchChunk(si, c);
        const want = (g.streams[si] as Uint8Array).subarray(c * s.chunkSize, Math.min(s.logical, (c + 1) * s.chunkSize));
        expect(Buffer.compare(Buffer.from(b), Buffer.from(want)), `stream ${si} chunk ${c}`).toBe(0);
      }
    }
    expect(client.fetches).toBe(2 * (3 + 5));
    client.close();
    expect(await done).toBe(2 * (3 + 5));
    await w.terminate();
  });

  it('a ChunkedSource over the SAB client reads files whole and in pieces (the engine thread blocks while the other thread fetches)', async () => {
    const shared = makeChunkIoShared(4096);
    const { w, done } = await spawnIoWorker(shared, idx.streams, dir, { delayMs: 2 });
    const client = new SabChunkClient(shared);
    const cache = new ChunkCache(8192);
    const srcs = idx.streams.map((s, i) => new ChunkedSource(s, i, client, cache));
    for (const e of idx.entries) {
      if (e.kind !== 'file') continue;
      const got = readAll(srcs[e.stream as number] as ChunkedSource, e.offset as number, e.size as number, 1500);
      expect(Buffer.compare(Buffer.from(got), Buffer.from(files.get(e.path) as Uint8Array)), e.path).toBe(0);
    }
    client.close();
    await done;
    await w.terminate();
  });

  it('a chunk the store refuses is an ERR_STORE error, the channel is usable afterwards, and an unknown chunk number is refused', async () => {
    const shared = makeChunkIoShared(4096);
    const refused = (idx.streams[0] as typeof idx.streams[number]).chunks[1] as string;
    const { w, done } = await spawnIoWorker(shared, idx.streams, dir, { failSha: [refused] });
    const client = new SabChunkClient(shared);
    expect(() => client.fetchChunk(0, 1)).toThrow(ChunkIoError);
    try { client.fetchChunk(0, 1); } catch (e) { expect((e as ChunkIoError).code).toBe(ERR_STORE); }
    expect(client.fetchChunk(0, 0).length).toBe(1024);
    expect(() => client.fetchChunk(0, 99)).toThrow(ChunkIoError);
    expect(() => client.fetchChunk(7, 0)).toThrow(ChunkIoError);
    client.close();
    await done;
    await w.terminate();
  });

  it('no I/O Worker: the engine thread gives up after the timeout instead of waiting for ever, and the channel stays closed', () => {
    const shared = makeChunkIoShared(4096);
    const client = new SabChunkClient(shared, { timeoutMs: 150 });
    const t0 = performance.now();
    expect(() => client.fetchChunk(0, 0)).toThrow(/no reply/);
    expect(performance.now() - t0).toBeGreaterThanOrEqual(140);
    expect(() => client.fetchChunk(0, 0)).toThrow(/not idle/);
  });

  it('a store that gives a chunk of the wrong length is an ERR_LENGTH error (not a short read)', async () => {
    const shared = makeChunkIoShared(4096);
    const { w, done } = await spawnIoWorker(shared, idx.streams, dir, { wrongLength: true });
    const client = new SabChunkClient(shared);
    try { client.fetchChunk(0, 0); expect.unreachable(); } catch (e) { expect(e).toBeInstanceOf(ChunkIoError); expect((e as ChunkIoError).code).toBe(ERR_LENGTH); }
    client.close();
    await done;
    await w.terminate();
  });
});

describe('HostFiles: lazy streams are persistent host files read with pread', () => {
  const ENOENT = 44, EIO = 29;
  const t = standardTree();
  const g = generate(t.entries, t.streams);
  const dir = mkdtempSync(join(tmpdir(), 'lazy-tree-hf-'));
  writeTree(dir, g);
  const idx = parseIndex(g.index, { entries: true });

  function rig() {
    const mem = new WebAssembly.Memory({ initial: 4 });
    const files = new HostFiles(() => mem);
    const orig: Record<string, unknown> = { fd_write: () => 0, fd_read: () => 52, fd_pread: () => 52, fd_seek: () => 70, fd_close: () => 52, __syscall_fstat64: () => 0 };
    const imports = { env: orig, wasi_snapshot_preview1: orig } as unknown as WebAssembly.Imports;
    installHostFiles(imports, files);
    const sc = orig as Record<string, (...a: unknown[]) => number>;
    const u8 = new Uint8Array(mem.buffer), dv = new DataView(mem.buffer);
    const str = (at: number, s: string) => { u8.set(new TextEncoder().encode(`${s}\0`), at); return at; };
    const iov = (at: number, list: Array<[number, number]>) => { list.forEach(([p, l], i) => { dv.setUint32(at + 8 * i, p, true); dv.setUint32(at + 8 * i + 4, l, true); }); return at; };
    return { mem, files, sc, u8, dv, str, iov };
  }

  it('openat + fd_pread across a chunk boundary; the index is an ordinary file that release() takes away, the streams stay', () => {
    const r = rig();
    const client = new SyncDirClient(dir, idx.streams);
    const m = mountLazyTree(r.files, '/data/seed', g.index, client);
    expect(m.paths).toEqual(['/data/seed/vfs/index.bin', '/data/seed/vfs/small-bin', '/data/seed/vfs/large-pcm']);
    const fd = r.sc.__syscall_openat!(-100, r.str(100, '/data/seed/vfs/large-pcm'), 0, 0) as number;
    expect(fd).toBeGreaterThanOrEqual(3);
    expect(r.sc.fd_pread!(fd, r.iov(200, [[1000, 100]]), 1, 4090n, 600)).toBe(0);
    expect(r.dv.getUint32(600, true)).toBe(100);
    expect(Buffer.compare(Buffer.from(r.u8.subarray(1000, 1100)), Buffer.from((g.streams[1] as Uint8Array).subarray(4090, 4190)))).toBe(0);
    expect(r.sc.__syscall_stat64!(r.str(100, '/data/seed/vfs/large-pcm'), 800)).toBe(0);
    expect(r.dv.getBigInt64(824, true)).toBe(18192n);
    r.files.release();
    // after the first idle: the stream is still there and readable from a descriptor opened before and by a new open; the index is not
    expect(r.sc.fd_pread!(fd, r.iov(200, [[1200, 8]]), 1, 0n, 600)).toBe(0);
    expect(Buffer.compare(Buffer.from(r.u8.subarray(1200, 1208)), Buffer.from((g.streams[1] as Uint8Array).subarray(0, 8)))).toBe(0);
    const fd2 = r.sc.__syscall_openat!(-100, r.str(100, '/data/seed/vfs/small-bin'), 0, 0) as number;
    expect(fd2).toBeGreaterThanOrEqual(3);
    expect(() => r.sc.__syscall_openat!(-100, r.str(100, '/data/seed/vfs/index.bin'), 0, 0)).toThrow(HostFilesUnsupported);
    expect(r.files.has('/data/seed/vfs/index.bin')).toBe(false);
    expect(r.sc.__syscall_stat64!(r.str(100, '/data/seed/vfs'), 800)).toBe(0);   // the directory is still there
    expect(r.sc.__syscall_stat64!(r.str(100, '/data/seed/nothing'), 800)).toBe(-ENOENT);
  });

  it('a chunk that cannot be fetched is EIO for the call (nothing is read), counted, and the message is kept', () => {
    const r = rig();
    const failing = { fetchChunk: (): Uint8Array => { throw new ChunkIoError(ERR_STORE, 'refused for the test'); } };
    mountLazyTree(r.files, '/data/seed', g.index, failing);
    const fd = r.sc.__syscall_openat!(-100, r.str(100, '/data/seed/vfs/small-bin'), 0, 0) as number;
    expect(r.sc.fd_pread!(fd, r.iov(200, [[1000, 16]]), 1, 0n, 600)).toBe(EIO);
    expect(r.dv.getUint32(600, true)).toBe(0);
    expect(r.files.stats.readErrors).toBe(1);
    expect(r.files.lastReadError).toMatch(/refused for the test/);
    expect(r.sc.fd_read!(fd, r.iov(200, [[1000, 16]]), 1, 600)).toBe(EIO);
    expect(r.files.stats.readErrors).toBe(2);
  });

  it('close() (the end of the session) forgets the persistent streams too', () => {
    const r = rig();
    mountLazyTree(r.files, '/data/seed', g.index, new SyncDirClient(dir, idx.streams));
    r.files.close();
    expect(r.files.has('/data/seed/vfs/small-bin')).toBe(false);
    expect(r.sc.__syscall_stat64!(r.str(100, '/data/seed/vfs/small-bin'), 800)).toBe(-ENOENT);
  });
});

// opt1's generator (tools/vfs-build.py V0.1, the format of section 19): its small test tree, and a sample of the real install tree. Skipped when the files are not on this machine.
const OPT1 = '/home/ubuntu/work/opt1-wine/vfs-out';
function fileBytes(p: string): Buffer { return readFileSync(p); }
function listFiles(root: string, rel = ''): string[] {
  const out: string[] = [];
  for (const d of readdirSync(join(root, rel), { withFileTypes: true })) {
    const r = rel ? `${rel}/${d.name}` : d.name;
    if (d.isDirectory()) out.push(...listFiles(root, r)); else out.push(r);
  }
  return out;
}

describe.skipIf(!existsSync(`${OPT1}/testtree-out/index.bin`))("opt1's small test tree (vfs-make-testtree.py): every file reads back byte for byte", () => {
  it('21 files, 21 directories, the mount, the 6 MiB wav across two 4 MiB chunks, Japanese and accented names, a zero-byte file', () => {
    const out = `${OPT1}/testtree-out`, src = `${OPT1}/testtree-src`;
    const bytes = new Uint8Array(readFileSync(`${out}/index.bin`));
    const idx = parseIndex(bytes, { entries: true });
    expect(idx.mount).toBe(MOUNT);
    expect(idx.nFiles).toBe(21);
    expect(idx.nDirs).toBe(21);
    expect(createHash('sha256').update(bytes).digest('hex').slice(0, 12)).toBe(readdirSync(out).find((n) => /^index\..*\.br$/.test(n))?.split('.')[1]);
    expect(Buffer.compare(Buffer.from(brotliDecompressSync(readFileSync(`${out}/${readdirSync(out).find((n) => /^index\..*\.br$/.test(n))}`))), Buffer.from(bytes))).toBe(0);
    const client = new SyncDirClient(out, idx.streams);
    const cache = new ChunkCache(16 << 20);
    const srcs = idx.streams.map((s, i) => new ChunkedSource(s, i, client, cache));
    let compared = 0;
    for (const e of idx.entries) {
      if (e.kind !== 'file') continue;
      const want = fileBytes(`${src}/${e.path}`);
      expect(e.size, e.path).toBe(want.length);
      const got = readAll(srcs[e.stream as number] as ChunkedSource, e.offset as number, e.size as number, 65536);
      expect(Buffer.compare(Buffer.from(got), want), e.path).toBe(0);
      compared++;
    }
    expect(compared).toBe(21);
    const dirs = idx.entries.filter((e) => e.kind === 'dir').map((e) => e.path);
    expect(dirs).toContain('Data/Empty folder');
    expect(idx.entries.some((e) => e.path.includes('日本語'))).toBe(true);
    expect(idx.entries.some((e) => e.path.includes('Noël'))).toBe(true);
    const names = listFiles(src);
    expect(names.length).toBe(21);
  });
});

const REAL = `${OPT1}/run-a`;
const ORIGINAL = '/home/ubuntu/work/opt1-wine/prefix/drive_c/Program Files/Image-Line/FL Studio 2026';
describe.skipIf(!existsSync(`${REAL}/index.bin`) || !existsSync(ORIGINAL))("opt1's index of the whole install tree (27,822 files): a sample of about 200 files, rebuilt from the chunks, is the original", () => {
  it('index.<sha12>.br decodes to index.bin; 27,822 files + 1,522 directories; 200 sampled files match the original bytes', () => {
    const br = readdirSync(REAL).find((n) => /^index\.[0-9a-f]{12}\.br$/.test(n)) as string;
    const bin = new Uint8Array(brotliDecompressSync(readFileSync(`${REAL}/${br}`)));
    expect(Buffer.compare(Buffer.from(bin), readFileSync(`${REAL}/index.bin`))).toBe(0);
    expect(createHash('sha256').update(bin).digest('hex').slice(0, 12)).toBe(br.split('.')[1]);
    const idx = parseIndex(bin, { entries: true });
    expect(idx.nFiles).toBe(27822);
    expect(idx.nDirs).toBe(1522);
    expect(idx.mount).toBe(MOUNT);
    expect(idx.streams.length).toBeLessThanOrEqual(16);
    const files = idx.entries.filter((e) => e.kind === 'file');
    const sample = new Set<number>();
    for (let k = 0; k < files.length; k += Math.floor(files.length / 181)) sample.add(k);   // about 180 evenly spread
    files.map((f, i) => [f.size as number, i] as const).sort((a, b) => b[0] - a[0]).slice(0, 20).forEach(([, i]) => sample.add(i));   // and the 20 largest
    files.forEach((f, i) => { if (f.size === 0 && sample.size < 215) sample.add(i); });
    const client = new SyncDirClient(REAL, idx.streams);
    const cache = new ChunkCache(256 << 20);
    const srcs = idx.streams.map((s, i) => new ChunkedSource(s, i, client, cache));
    let compared = 0, changed = 0, missing = 0, bytes = 0;
    for (const i of sample) {
      const e = files[i]!;
      const p = `${ORIGINAL}/${e.path}`;
      if (!existsSync(p)) { missing++; continue; }
      const want = readFileSync(p);
      if (want.length !== e.size) { changed++; continue; }   // the prefix was used by Wine after the index was made
      const got = readAll(srcs[e.stream as number] as ChunkedSource, e.offset as number, e.size as number, 1 << 20);
      expect(Buffer.compare(Buffer.from(got), want), e.path).toBe(0);
      compared++;
      bytes += want.length;
    }
    // the evidence goes into the test output
    console.log(`lazy-tree real sample: ${sample.size} chosen, ${compared} compared byte for byte (${bytes} bytes), ${changed} with a different size now, ${missing} not at the original path; chunk fetches ${client.fetches}`);
    expect(compared).toBeGreaterThanOrEqual(190);
    expect(statSync(`${REAL}/vfs`).isDirectory()).toBe(true);
  }, 300000);
});
