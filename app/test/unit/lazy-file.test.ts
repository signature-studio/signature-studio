import { createHash, randomBytes } from 'node:crypto';
import http from 'node:http';
import { Worker } from 'node:worker_threads';
import { describe, expect, it } from 'vitest';
import { downloadLazy } from '../../src/bundle/lazy.ts';
import type { LazyRef } from '../../src/bundle/manifest.ts';
import { HostFiles, installHostFiles } from '../../src/engine/hostfiles.ts';
import { LazyFileError, LazyFileFeed, LazyFileSource, makeLazyFileShared, type LazyFileShared } from '../../src/engine/lazy-file.ts';

// Issue #1314: a file the page downloads after the engine has started. The expected bytes are the test's own random bytes, not anything the code under test made.

const sha = (b: Uint8Array): string => createHash('sha256').update(b).digest('hex');

function spawnFeeder(shared: LazyFileShared, bytes: Uint8Array | null, delayMs: number): Promise<Worker> {
  return new Promise((resolve, reject) => {
    const w = new Worker(new URL('./helpers/lazy-feed-node-worker.ts', import.meta.url), { workerData: { shared, bytes, delayMs } });
    w.on('message', (m: { type: string }) => { if (m.type === 'ready') resolve(w); });
    w.on('error', reject);
  });
}

describe('LazyFileSource: a host file whose bytes may arrive after the engine started', () => {
  it('a read of bytes that arrived before it returns them, in pieces, and 0 at the end', () => {
    const data = randomBytes(1000);
    const shared = makeLazyFileShared('fruity balance', 'fruity_balance_x64.dll', data.length);
    new LazyFileFeed(shared).deliver(new Uint8Array(data));
    const src = new LazyFileSource(shared);
    expect(src.size).toBe(1000);
    const out = new Uint8Array(1000);
    expect(src.readInto(out.subarray(0, 300), 0)).toBe(300);
    expect(src.readInto(out.subarray(300), 300)).toBe(700);
    expect(Buffer.from(out).equals(data)).toBe(true);
    expect(src.readInto(new Uint8Array(10), 1000)).toBe(0);
  });

  it('a read that comes before the bytes waits for them (the page delivers from another thread)', async () => {
    const data = randomBytes(4096);
    const shared = makeLazyFileShared('x', 'x.dll', data.length);
    const waits: [string, number][] = [];
    const src = new LazyFileSource(shared, { onWait: (p, ms) => waits.push([p, ms]) });
    const w = await spawnFeeder(shared, new Uint8Array(data), 60);
    const out = new Uint8Array(data.length);
    expect(src.readInto(out, 0)).toBe(data.length);   // blocks here until the feeder has delivered
    expect(Buffer.from(out).equals(data)).toBe(true);
    expect(waits.length).toBe(1);
    expect(waits[0]?.[0]).toBe('x.dll');
    expect(waits[0]?.[1]).toBeGreaterThanOrEqual(30);
    await w.terminate();
  });

  it('a download that failed makes the read throw by name (hostfiles answers it with EIO)', async () => {
    const shared = makeLazyFileShared('x', 'x.dll', 100);
    const src = new LazyFileSource(shared);
    const w = await spawnFeeder(shared, null, 20);
    expect(() => src.readInto(new Uint8Array(100), 0)).toThrow(LazyFileError);
    expect(() => src.readInto(new Uint8Array(100), 0)).toThrow(/x\.dll: the download failed/);
    await w.terminate();
  });

  it('a read gives up after timeoutMs when nothing arrives', () => {
    const shared = makeLazyFileShared('x', 'x.dll', 100);
    const src = new LazyFileSource(shared, { timeoutMs: 40 });
    const t0 = performance.now();
    expect(() => src.readInto(new Uint8Array(100), 0)).toThrow(/did not arrive within 40 ms/);
    expect(performance.now() - t0).toBeGreaterThanOrEqual(35);
  });

  it('an empty file is ready at once and reads nothing', () => {
    const shared = makeLazyFileShared('empty', 'empty.bin', 0);
    new LazyFileFeed(shared).deliver(new Uint8Array(0));
    expect(new LazyFileSource(shared).readInto(new Uint8Array(4), 0)).toBe(0);
  });

  it('the feed refuses bytes of another length (the size is the manifest\'s)', () => {
    const shared = makeLazyFileShared('x', 'x.dll', 10);
    expect(() => new LazyFileFeed(shared).deliver(new Uint8Array(9))).toThrow(/9 bytes for a lazy file of 10/);
  });

  it('as a host file: open and read work from the start, and the file stays after the release at the first idle', () => {
    const data = randomBytes(64);
    const shared = makeLazyFileShared('late', 'late.dll', data.length);
    const mem = new WebAssembly.Memory({ initial: 1 });
    const files = new HostFiles(() => mem);
    const src = new LazyFileSource(shared);
    files.addPersistent('/data/late.dll', src);
    const orig: Record<string, unknown> = { fd_write: () => 0, fd_read: () => 52, fd_pread: () => 52, fd_seek: () => 70, fd_close: () => 52, __syscall_fstat64: () => 0 };
    installHostFiles({ env: orig, wasi_snapshot_preview1: orig } as unknown as WebAssembly.Imports, files);
    const sc = orig as Record<string, (...a: unknown[]) => number>;
    const u8 = new Uint8Array(mem.buffer), dv = new DataView(mem.buffer);
    u8.set(new TextEncoder().encode('/data/late.dll\0'), 100);
    const fd = sc.__syscall_openat!(-100, 100, 0, 0)!;   // an open does not wait for the bytes (the size is known)
    expect(fd).toBeGreaterThanOrEqual(3);
    new LazyFileFeed(shared).deliver(new Uint8Array(data));
    dv.setUint32(200, 300, true); dv.setUint32(204, 64, true);
    expect(sc.fd_read!(fd, 200, 1, 600)).toBe(0);
    expect(dv.getUint32(600, true)).toBe(64);
    expect(Buffer.from(u8.subarray(300, 364)).equals(data)).toBe(true);
    files.release();
    expect(files.has('/data/late.dll')).toBe(true);
    expect(sc.__syscall_openat!(-100, 100, 0, 0)).toBeGreaterThanOrEqual(3);
  });
});

describe('downloadLazy: the page side (verified like the start-up assets)', () => {
  const body = randomBytes(300_000);
  const other = randomBytes(5000);
  let server: http.Server;
  let base = '';
  const routes: Record<string, Buffer> = { '/a/one.dll': Buffer.from(body), '/a/two.bin': Buffer.from(other) };

  it('starts a local server', async () => {
    server = http.createServer((req, res) => {
      const b = routes[req.url ?? ''];
      if (!b) { res.writeHead(404); res.end(); return; }
      res.writeHead(200, { 'Content-Type': 'application/octet-stream', 'Content-Length': b.length });
      res.end(b);
    });
    await new Promise<void>((r) => server.listen(0, '127.0.0.1', r));
    base = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
  });

  const ref = (name: string, url: string, b: Buffer, over: Partial<LazyRef> = {}): LazyRef => ({ name, path: name, url, size: b.length, sha256: sha(b), compression: 'identity', ...over });

  it('verifies each file, hands it to its engine file, and reports in the manifest order', async () => {
    const r1 = ref('one.dll', '/a/one.dll', body), r2 = ref('two.bin', '/a/two.bin', other);
    const s1 = makeLazyFileShared(r1.name, r1.path, r1.size), s2 = makeLazyFileShared(r2.name, r2.path, r2.size);
    const results = await downloadLazy([{ ref: r1, feed: new LazyFileFeed(s1) }, { ref: r2, feed: new LazyFileFeed(s2) }], { baseUrl: base });
    expect(results).toEqual([{ name: 'one.dll', ok: true, message: null }, { name: 'two.bin', ok: true, message: null }]);
    const got = new Uint8Array(body.length);
    expect(new LazyFileSource(s1).readInto(got, 0)).toBe(body.length);
    expect(Buffer.from(got).equals(body)).toBe(true);
  });

  it('a file whose SHA-256 does not match fails: its feed says so, the others still come', async () => {
    const bad = ref('one.dll', '/a/one.dll', body, { sha256: 'f'.repeat(64) });
    const good = ref('two.bin', '/a/two.bin', other);
    const sb = makeLazyFileShared(bad.name, bad.path, bad.size), sg = makeLazyFileShared(good.name, good.path, good.size);
    const seen: string[] = [];
    const results = await downloadLazy([{ ref: bad, feed: new LazyFileFeed(sb) }, { ref: good, feed: new LazyFileFeed(sg) }], { baseUrl: base }, (r) => seen.push(r.name));
    expect(results[0]).toMatchObject({ name: 'one.dll', ok: false });
    expect(results[0]?.message).toContain('SHA-256 mismatch');
    expect(results[1]).toMatchObject({ name: 'two.bin', ok: true });
    expect(seen).toEqual(['one.dll', 'two.bin']);
    expect(() => new LazyFileSource(sb).readInto(new Uint8Array(4), 0)).toThrow(/the download failed/);
  });

  it('a file that is not on the server fails with the HTTP status', async () => {
    const missing = ref('missing.bin', '/a/missing.bin', other);
    const sm = makeLazyFileShared(missing.name, missing.path, missing.size);
    const [r] = await downloadLazy([{ ref: missing, feed: new LazyFileFeed(sm) }], { baseUrl: base });
    expect(r).toMatchObject({ ok: false });
    expect(r?.message).toContain('HTTP 404');
  });

  it('closes the server', () => { server.close(); });
});
