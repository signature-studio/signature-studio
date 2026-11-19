import { createHash, randomBytes } from 'node:crypto';
import http from 'node:http';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { AssetStore, entryName, storeFailCode } from '../../src/bundle/assetstore.ts';
import { fetchAsset, loadPack, loadWasm, LoadError, type AssetTrace } from '../../src/bundle/loader.ts';
import { buildPack } from '../../src/bundle/pack.ts';
import type { AssetRef } from '../../src/bundle/manifest.ts';
import { FakeDir, fakeEnv } from '../helpers/fake-opfs.ts';

const sha = (b: Uint8Array): string => createHash('sha256').update(b).digest('hex');
const wasm = Buffer.from('0061736d01000000', 'hex');   // an empty but valid module
const big = randomBytes(9_500_000);
const pack = Buffer.from(buildPack([{ path: 'a/one.bin', data: randomBytes(300_000) }, { path: 'b/two.bin', data: randomBytes(70_000) }]));

let hits: Record<string, number> = {};
let server: http.Server;
let base = '';
const routes: Record<string, Buffer> = {};
beforeAll(async () => {
  routes['/w/engine.wasm'] = wasm;
  routes['/d/big.flpack'] = big;
  routes['/d/pack.flpack'] = pack;
  server = http.createServer((req, res) => {
    const b = routes[req.url ?? ''];
    if (!b) { res.writeHead(404); res.end(); return; }
    hits[req.url as string] = (hits[req.url as string] ?? 0) + 1;
    res.writeHead(200, { 'Content-Length': b.length, 'Content-Type': 'application/octet-stream' });
    let at = 0;
    const next = (): void => { if (at >= b.length) { res.end(); return; } res.write(b.subarray(at, at + 65536)); at += 65536; setImmediate(next); };
    next();
  });
  await new Promise<void>((r) => server.listen(0, '127.0.0.1', r));
  base = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
});
afterAll(() => { server.close(); });

const ref = (url: string, body: Buffer, over: Partial<AssetRef> = {}): AssetRef => ({ url, size: body.length, sha256: sha(body), compression: 'identity', ...over });
const PATH = 'fl/cache/assets/';
const traces = (): { list: AssetTrace[]; fn: (t: AssetTrace) => void } => { const list: AssetTrace[] = []; return { list, fn: (t) => { list.push(t); } }; };
const last = (l: AssetTrace[]): AssetTrace => l[l.length - 1] as AssetTrace;
const settle = async (store: AssetStore, r: AssetRef, root: FakeDir): Promise<void> => { for (let i = 0; i < 200; i++) { if (root.at(PATH + entryName(r))?.data.length === r.size) return; await new Promise((x) => setTimeout(x, 10)); } };

describe('the stored copy (decision 0220)', () => {
  it('a first load comes from the network, is verified, and is written whole; the second load is served from the store without a request', async () => {
    hits = {};
    const root = new FakeDir();
    const store = AssetStore.open(fakeEnv(root))!;
    const r = ref(`${base}/d/big.flpack`, big);
    const t1 = traces();
    const a = await fetchAsset(r, 'DLL pack', { baseUrl: base, store, onTrace: t1.fn });
    expect(Buffer.from(a).equals(big)).toBe(true);
    expect(hits['/d/big.flpack']).toBe(1);
    await settle(store, r, root);
    const file = root.at(PATH + entryName(r))!;
    expect(file.data.length).toBe(big.length);
    expect(Buffer.from(file.data).equals(big)).toBe(true);
    expect(last(t1.list)).toMatchObject({ origin: 'network', store: 'stored', fail: null });
    expect(last(t1.list).storeMs).toBeGreaterThanOrEqual(0);

    const t2 = traces();
    const b = await fetchAsset(r, 'DLL pack', { baseUrl: base, store, onTrace: t2.fn });
    expect(Buffer.from(b).equals(big)).toBe(true);
    expect(hits['/d/big.flpack']).toBe(1);
    expect(last(t2.list)).toMatchObject({ origin: 'storage-cache', store: 'hit' });
  });

  it('the stored copy is whole even when the caller transfers the returned buffer to a Worker before the write has run (a detached view would write nothing)', async () => {
    const root = new FakeDir();
    const store = AssetStore.open(fakeEnv(root))!;
    const r = ref(`${base}/d/big.flpack`, big);
    const dirs = await (await (await root.getDirectoryHandle('fl', { create: true })).getDirectoryHandle('cache', { create: true })).getDirectoryHandle('assets', { create: true });
    ((await dirs.getFileHandle(entryName(r), { create: true })) as unknown as { writeDelayMs: number }).writeDelayMs = 15; // the writes run after the caller has the bytes
    const t = traces();
    const out = await fetchAsset(r, 'DLL pack', { baseUrl: base, store, onTrace: t.fn });
    structuredClone(out, { transfer: [out.buffer] }); // what client.init does with the DLL pack
    expect(out.byteLength).toBe(0);
    for (let i = 0; i < 300 && last(t.list).store === 'writing'; i++) await new Promise((x) => setTimeout(x, 10));
    expect(last(t.list)).toMatchObject({ store: 'stored', fail: null });
    expect(Buffer.from(root.at(PATH + entryName(r))!.data).equals(big)).toBe(true);
  });

  it('a wasm module is compiled from the stored copy', async () => {
    hits = {};
    const root = new FakeDir();
    const store = AssetStore.open(fakeEnv(root))!;
    const r = ref(`${base}/w/engine.wasm`, wasm);
    const m1 = await loadWasm(r, 'engine wasm', { baseUrl: base, store });
    expect(m1).toBeInstanceOf(WebAssembly.Module);
    await settle(store, r, root);
    const t = traces();
    const m2 = await loadWasm(r, 'engine wasm', { baseUrl: base, store, onTrace: t.fn });
    expect(m2).toBeInstanceOf(WebAssembly.Module);
    expect(hits['/w/engine.wasm']).toBe(1);
    expect(last(t.list)).toMatchObject({ origin: 'storage-cache', store: 'hit' });
    expect(last(t.list).compileMs).not.toBeNull();
  });

  it('a stored copy whose bytes were changed (same size) is a miss: it is discarded and the network is used', async () => {
    hits = {};
    const root = new FakeDir();
    const store = AssetStore.open(fakeEnv(root))!;
    const r = ref(`${base}/d/big.flpack`, big);
    await fetchAsset(r, 'DLL pack', { baseUrl: base, store });
    await settle(store, r, root);
    const f = root.at(PATH + entryName(r))!;
    f.data = f.data.slice(); f.data[1234567] ^= 1;
    const t = traces();
    const out = await fetchAsset(r, 'DLL pack', { baseUrl: base, store, onTrace: t.fn });
    expect(Buffer.from(out).equals(big)).toBe(true);
    expect(hits['/d/big.flpack']).toBe(2);
    expect(t.list.some((x) => x.origin === 'network')).toBe(true);
    expect(t.list.find((x) => x.origin === 'network')?.fail).toBe('verify');
    await settle(store, r, root);
    expect(Buffer.from(root.at(PATH + entryName(r))!.data).equals(big)).toBe(true);
  });

  it('a truncated stored copy is a miss', async () => {
    hits = {};
    const root = new FakeDir();
    const store = AssetStore.open(fakeEnv(root))!;
    const r = ref(`${base}/d/big.flpack`, big);
    await fetchAsset(r, 'DLL pack', { baseUrl: base, store });
    await settle(store, r, root);
    root.at(PATH + entryName(r))!.data = big.subarray(0, 58_720_256 % big.length);
    await fetchAsset(r, 'DLL pack', { baseUrl: base, store });
    expect(hits['/d/big.flpack']).toBe(2);
  });

  it('bytes that do not match the manifest are never stored (the network path verifies before the commit)', async () => {
    const root = new FakeDir();
    const store = AssetStore.open(fakeEnv(root))!;
    const r = ref(`${base}/d/big.flpack`, big, { sha256: '0'.repeat(64) });
    await expect(fetchAsset(r, 'DLL pack', { baseUrl: base, store })).rejects.toThrow(/SHA-256 mismatch/);
    await new Promise((x) => setTimeout(x, 50));
    const f = root.at(PATH + entryName(r));
    expect(f === undefined || f.data.length === 0).toBe(true);
  });

  it('a failing write falls back silently: the load succeeds and the trace says failed with a code', async () => {
    const root = new FakeDir();
    const store = AssetStore.open(fakeEnv(root))!;
    const r = ref(`${base}/d/big.flpack`, big);
    const f = await (await (await root.getDirectoryHandle('fl', { create: true })).getDirectoryHandle('cache', { create: true })).getDirectoryHandle('assets', { create: true });
    const fh = (await f.getFileHandle(entryName(r), { create: true })) as unknown as { failWriteAfter: number };
    fh.failWriteAfter = 1;
    const t = traces();
    const out = await fetchAsset(r, 'DLL pack', { baseUrl: base, store, onTrace: t.fn });
    expect(Buffer.from(out).equals(big)).toBe(true);
    for (let i = 0; i < 100 && last(t.list).store === 'writing'; i++) await new Promise((x) => setTimeout(x, 10));
    expect(last(t.list)).toMatchObject({ store: 'failed', fail: 'quota' });
  });

  it('without a store (a browser without OPFS) the bytes come from the network and the trace says the store is off', async () => {
    const t = traces();
    const out = await fetchAsset(ref(`${base}/d/big.flpack`, big), 'DLL pack', { baseUrl: base, onTrace: t.fn });
    expect(Buffer.from(out).equals(big)).toBe(true);
    expect(last(t.list)).toMatchObject({ origin: 'network', store: 'off', hash: 'subtle' });
  });

  it('a browser without WebCrypto hashes in JS while the bytes arrive: the same bytes, and the trace says so', async () => {
    const root = new FakeDir();
    const store = AssetStore.open(fakeEnv(root))!;
    const t = traces();
    const r = ref(`${base}/d/big.flpack`, big);
    const saved = Object.getOwnPropertyDescriptor(globalThis, 'crypto');
    Object.defineProperty(globalThis, 'crypto', { value: { subtle: undefined }, configurable: true });
    try {
      const out = await fetchAsset(r, 'DLL pack', { baseUrl: base, store, onTrace: t.fn });
      expect(Buffer.from(out).equals(big)).toBe(true);
    } finally { if (saved) Object.defineProperty(globalThis, 'crypto', saved); }
    expect(t.list[0]).toMatchObject({ hash: 'js-stream', origin: 'network' });
    expect(t.list[0]?.hashMs).toBeGreaterThan(0);
  });
});

describe('packs and the other checks of the new path', () => {
  it('loadPack: the pack is parsed on both loads, the second from the verified stored copy', async () => {
    const root = new FakeDir();
    const store = AssetStore.open(fakeEnv(root))!;
    const r = { ...ref(`${base}/d/pack.flpack`, pack), format: 'flpack1' as const };
    const t = traces();
    const p1 = await loadPack(r, 'DLL pack', { baseUrl: base, store, onTrace: t.fn });
    expect([...p1.files.keys()]).toEqual(['a/one.bin', 'b/two.bin']);
    await settle(store, r, root);
    expect(last(t.list).parseMs).toBeGreaterThanOrEqual(0);
    const p2 = await loadPack(r, 'DLL pack', { baseUrl: base, store });
    expect(p2.files.size).toBe(2);
  });
  it('the new path rejects bytes that do not match the manifest, with or without a store: SHA-256 (WebCrypto), too many bytes, too few bytes', async () => {
    const good = ref(`${base}/d/big.flpack`, big);
    for (const store of [null, AssetStore.open(fakeEnv(new FakeDir()))]) {
      const o = { baseUrl: base, store };
      let e = await fetchAsset({ ...good, sha256: '1'.repeat(64) }, 'x', o).then(() => null, (x: unknown) => x);
      expect(e).toBeInstanceOf(LoadError); expect((e as LoadError).stage).toBe('verify'); expect((e as LoadError).message).toContain('SHA-256 mismatch');
      e = await fetchAsset({ ...good, size: big.length - 5 }, 'x', o).then(() => null, (x: unknown) => x);
      expect((e as LoadError).stage).toBe('verify'); expect((e as LoadError).message).toContain('size mismatch');
      e = await fetchAsset({ ...good, size: big.length + 5 }, 'x', o).then(() => null, (x: unknown) => x);
      expect((e as LoadError).stage).toBe('verify'); expect((e as LoadError).message).toContain('size mismatch');
    }
  });
  it('a pack with a wrong member hash is taken when the whole-pack SHA-256 of the manifest matches (it covers the index and every member); a pack whose bytes are not the manifest\'s is refused at the verify stage', async () => {
    const bad = Buffer.from(pack);
    bad[bad.length - 5] ^= 1; // a byte of the last member, not of the index
    routes['/d/badmember.flpack'] = bad;
    const r = { ...ref(`${base}/d/badmember.flpack`, bad), format: 'flpack1' as const };
    const ok = await loadPack(r, 'seed pack', { baseUrl: base });
    expect(ok.files.size).toBe(2);
    const e = await loadPack({ ...r, sha256: '3'.repeat(64) }, 'seed pack', { baseUrl: base }).then(() => null, (x: unknown) => x);
    expect(e).toBeInstanceOf(LoadError);
    expect((e as LoadError).stage).toBe('verify');
  });
  it('a wasm whose one-buffer allocation fails is streamed instead (here: the impossible size is then reported as a size mismatch by the streaming path, not as a RangeError)', async () => {
    const bad = { ...ref(`${base}/w/engine.wasm`, wasm), size: 2 ** 45 };
    const e = await loadWasm(bad, 'engine wasm', { baseUrl: base }).then(() => null, (x: unknown) => x);
    expect(e).toBeInstanceOf(LoadError);
    expect((e as LoadError).message).toContain('size mismatch');
  });
  it('a wasm whose bytes are not the manifest\'s is never returned as a module, whatever the compiler says', async () => {
    const root = new FakeDir();
    const store = AssetStore.open(fakeEnv(root))!;
    const bad = { ...ref(`${base}/w/engine.wasm`, wasm), sha256: '2'.repeat(64) };
    const e = await loadWasm(bad, 'engine wasm', { baseUrl: base, store }).then(() => null, (x: unknown) => x);
    expect(e).toBeInstanceOf(LoadError);
    expect((e as LoadError).stage).toBe('verify');
    await new Promise((x) => setTimeout(x, 30));
    const f = root.at(PATH + entryName(bad));
    expect(f === undefined || f.data.length === 0).toBe(true);
  });
});

describe('clean-up and locks', () => {
  const mk = async (names: string[], root: FakeDir): Promise<void> => {
    let d = root;
    for (const p of ['fl', 'cache', 'assets']) d = (await d.getDirectoryHandle(p, { create: true })) as FakeDir;
    for (const n of names) { const fh = await d.getFileHandle(n, { create: true }); const w = await fh.createWritable(); await w.write(new Uint8Array(3)); await w.close(); }
  };
  const A = 'a'.repeat(64), B = 'b'.repeat(64), C = 'c'.repeat(64);
  it('removes the stored copies the manifest does not name and leaves foreign files alone', async () => {
    const root = new FakeDir();
    await mk([`${A}.wasm`, `${B}.flpack`, `${C}.js`, 'notes.txt', `${'d'.repeat(63)}.wasm`], root);
    const store = AssetStore.open(fakeEnv(root))!;
    const keep: AssetRef[] = [{ url: '/assets/x/e.wasm', size: 3, sha256: A, compression: 'br' }, { url: '/assets/x/d.flpack', size: 3, sha256: B, compression: 'br' }];
    const r = await store.clean(keep);
    expect(r).toEqual({ removed: 1, kept: 2, busy: 0, failed: 0 });
    expect(root.at(`${PATH}${A}.wasm`)).toBeDefined();
    expect(root.at(`${PATH}${C}.js`)).toBeUndefined();
    expect(root.at(`${PATH}notes.txt`)).toBeDefined();
    expect(root.at(`${PATH}${'d'.repeat(63)}.wasm`)).toBeDefined();
  });
  it('does not remove a file another page holds (shared Web Lock), and counts it as busy', async () => {
    const root = new FakeDir();
    await mk([`${C}.js`], root);
    const env = fakeEnv(root);
    const reader = AssetStore.open(env)!;
    const other = AssetStore.open(env)!;
    const rc: AssetRef = { url: '/assets/x/g.js', size: 3, sha256: C, compression: 'br' };
    expect(await reader.get(rc)).not.toBeNull();
    expect(env.locks.held()).toContain(`fl-asset:${C}`);
    const r = await other.clean([]);
    expect(r).toMatchObject({ removed: 0, busy: 1 });
    expect(root.at(`${PATH}${C}.js`)).toBeDefined();
  });
  it('without Web Locks nothing is removed (it cannot be known whether the file is in use)', async () => {
    const root = new FakeDir();
    await mk([`${C}.js`], root);
    const store = AssetStore.open({ root: async () => root })!;
    expect(await store.clean([])).toMatchObject({ removed: 0, busy: 1 });
  });
  it('a second writer of the same hash gets null while the first is open', async () => {
    const root = new FakeDir();
    const env = fakeEnv(root);
    const s1 = AssetStore.open(env)!, s2 = AssetStore.open(env)!;
    const r: AssetRef = { url: '/assets/x/e.wasm', size: 3, sha256: A, compression: 'br' };
    const w1 = await s1.begin(r);
    expect(w1).not.toBeNull();
    expect(await s2.begin(r)).toBeNull();
    await w1!.write(new Uint8Array(3));
    await w1!.commit();
    expect(await s2.begin(r)).not.toBeNull();
  });
  it('refuses to start a write that cannot fit the quota', async () => {
    const root = new FakeDir();
    const store = AssetStore.open(fakeEnv(root, { estimate: async () => ({ usage: 99, quota: 100 }) }))!;
    await expect(store.begin({ url: '/assets/x/e.wasm', size: 1_000_000, sha256: A, compression: 'br' })).rejects.toMatchObject({ name: 'QuotaExceededError' });
  });
});

describe('names and codes', () => {
  it('the entry name is the manifest hash plus the extension of the url', () => {
    expect(entryName({ url: '/assets/b/engine.f4a57e84a5e1.wasm', size: 1, sha256: 'ab'.repeat(32), compression: 'br' })).toBe(`${'ab'.repeat(32)}.wasm`);
    expect(entryName({ url: '/assets/b/seed.0f4b547e6656.flpack?x=1', size: 1, sha256: 'cd'.repeat(32), compression: 'br' })).toBe(`${'cd'.repeat(32)}.flpack`);
    expect(entryName({ url: '/assets/b/noext', size: 1, sha256: 'ef'.repeat(32), compression: 'br' })).toBe(`${'ef'.repeat(32)}.bin`);
  });
  it('error names map to the fixed codes', () => {
    expect(storeFailCode({ name: 'QuotaExceededError' })).toBe('quota');
    expect(storeFailCode({ name: 'SecurityError' })).toBe('security');
    expect(storeFailCode({ name: 'NoModificationAllowedError' })).toBe('locked');
    expect(storeFailCode({ name: 'TypeError', message: 'x is not a function' })).toBe('unsupported');
    expect(storeFailCode(new Error('boom'))).toBe('other');
  });
});
