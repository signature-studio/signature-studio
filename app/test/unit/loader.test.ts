import { createHash, randomBytes } from 'node:crypto';
import http from 'node:http';
import zlib from 'node:zlib';
import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';
import { explainCompileError, fetchAsset, fetchManifest, loadPack, loadWasm, LoadError, type Progress } from '../../src/bundle/loader.ts';
import type { AssetRef } from '../../src/bundle/manifest.ts';
import { buildPack } from '../../src/bundle/pack.ts';

const sha = (b: Uint8Array): string => createHash('sha256').update(b).digest('hex');
// a small valid module: it imports env.memory (one page) and exports one function, `convert`
const wasm = (() => {
  const u = (n: number): number[] => [n];
  const name = (t: string): number[] => [t.length, ...Buffer.from(t)];
  const imp = [1, ...name('env'), ...name('memory'), 2, 0, 1];
  const exp = [1, ...name('convert'), 0, 0];
  const body = [0, 0x0b];
  return Buffer.from([0, 0x61, 0x73, 0x6d, 1, 0, 0, 0, 1, 4, 1, 0x60, 0, 0, 2, ...u(imp.length), ...imp, 3, 2, 1, 0, 7, ...u(exp.length), ...exp, 10, body.length + 2, 1, body.length, ...body]);
})();
const big = randomBytes(3_000_000); // several network chunks
const pack = Buffer.from(buildPack([{ path: 'a.txt', data: new TextEncoder().encode('hello') }]));

interface Route { body: Buffer; headers?: Record<string, string>; status?: number; slow?: boolean }
let routes: Record<string, Route>;
let server: http.Server;
let base = '';

beforeAll(async () => {
  routes = {
    '/w/ok.wasm': { body: wasm, headers: { 'Content-Type': 'application/wasm' } },
    '/w/gz.wasm': { body: zlib.gzipSync(wasm), headers: { 'Content-Type': 'application/wasm', 'Content-Encoding': 'gzip' } },
    '/w/br.wasm': { body: zlib.brotliCompressSync(wasm), headers: { 'Content-Type': 'application/wasm', 'Content-Encoding': 'br' } },
    '/w/stored.wasm.gz': { body: zlib.gzipSync(wasm), headers: { 'Content-Type': 'application/gzip' } },
    '/w/garbage.wasm': { body: Buffer.from('this is not a wasm module at all'), headers: { 'Content-Type': 'application/wasm' } },
    '/d/big.bin': { body: big, headers: { 'Content-Type': 'application/octet-stream' } },
    '/d/big.bin.gz': { body: zlib.gzipSync(big), headers: { 'Content-Type': 'application/gzip' } },
    '/d/seed.flpack': { body: pack },
    '/d/notapack.flpack': { body: Buffer.from('FLPKxxxxxxxxxxxxxxxxxxxxxxxx') },
    '/gone': { body: Buffer.from('nope'), status: 404 },
    '/m/good.json': { body: Buffer.from('{}') },
  };
  server = http.createServer((req, res) => {
    const r = routes[req.url ?? ''];
    if (!r) { res.writeHead(404); res.end(); return; }
    res.writeHead(r.status ?? 200, { ...r.headers, 'Content-Length': r.body.length });
    // dribble the body in 64 KiB pieces so that progress is reported more than once
    let at = 0;
    const next = (): void => { if (at >= r.body.length) { res.end(); return; } res.write(r.body.subarray(at, at + 65536)); at += 65536; setImmediate(next); };
    next();
  });
  await new Promise<void>((res) => server.listen(0, '127.0.0.1', res));
  base = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
});
afterAll(() => { server.close(); });

const ref = (url: string, body: Buffer, compression: AssetRef['compression'] = 'identity', over: Partial<AssetRef> = {}): AssetRef => ({ url, size: body.length, sha256: sha(body), compression, ...over });
const expectLoadError = async (p: Promise<unknown>, stage: string, text: string): Promise<LoadError> => {
  const e = await p.then(() => null, (x: unknown) => x);
  expect(e, 'should have failed').toBeInstanceOf(LoadError);
  const le = e as LoadError;
  expect(le.stage).toBe(stage);
  expect(le.message).toContain(text);
  expect(le.describe().where).toContain(stage);
  return le;
};

describe('fetchAsset', () => {
  it('returns the decoded bytes, reports progress up to the total, and ends with done', async () => {
    const seen: Progress[] = [];
    const out = await fetchAsset(ref('/d/big.bin', big), 'big', { baseUrl: base, onProgress: (p) => seen.push(p) });
    expect(Buffer.from(out).equals(big)).toBe(true);
    expect(seen.length).toBeGreaterThan(3);
    for (let i = 1; i < seen.length; i++) expect(seen[i]!.loaded).toBeGreaterThanOrEqual(seen[i - 1]!.loaded);
    expect(seen.at(-1)).toMatchObject({ loaded: big.length, total: big.length, done: true, label: 'big' });
    expect(seen.slice(0, -1).every((p) => !p.done)).toBe(true);
  });

  it('stamps every progress report with the time (performance.now()), never going back: the page derives the transfer speed from it', async () => {
    const seen: Progress[] = [];
    await fetchAsset(ref('/d/big.bin', big), 'big', { baseUrl: base, onProgress: (p) => seen.push(p) });
    expect(seen.every((p) => typeof p.at === 'number' && p.at > 0)).toBe(true);
    for (let i = 1; i < seen.length; i++) expect(seen[i]!.at).toBeGreaterThanOrEqual(seen[i - 1]!.at);
  });

  it('a stored .gz without Content-Encoding is decoded with DecompressionStream, and verified against the DECODED size and hash', async () => {
    const out = await fetchAsset(ref('/d/big.bin.gz', big, 'gzip'), 'big', { baseUrl: base });
    expect(Buffer.from(out).equals(big)).toBe(true);
  });

  it('refuses a SHA-256 that differs from the manifest', async () => {
    const e = await expectLoadError(fetchAsset(ref('/d/big.bin', big, 'identity', { sha256: '0'.repeat(64) }), 'big', { baseUrl: base }), 'verify', 'SHA-256 mismatch');
    expect(e.message).toContain(sha(big));
    expect(e.url).toContain('/d/big.bin');
  });

  it('refuses a size that differs from the manifest - too large and too small', async () => {
    await expectLoadError(fetchAsset(ref('/d/big.bin', big, 'identity', { size: big.length + 10 }), 'big', { baseUrl: base }), 'verify', 'size mismatch');
    await expectLoadError(fetchAsset(ref('/d/big.bin', big, 'identity', { size: 1000 }), 'big', { baseUrl: base }), 'verify', 'size mismatch');
  });

  it('reports HTTP errors with the status and the URL', async () => {
    await expectLoadError(fetchAsset(ref('/gone', Buffer.from('x')), 'gone', { baseUrl: base }), 'fetch', 'HTTP 404');
  });

  it('refuses a URL on another origin before any request is made', async () => {
    let called = false;
    const spy = (async () => { called = true; return new Response('x'); }) as unknown as typeof fetch;
    await expectLoadError(fetchAsset(ref('http://evil.example/x', Buffer.from('x')), 'x', { baseUrl: base, fetchImpl: spy }), 'fetch', 'another origin');
    await expectLoadError(fetchAsset(ref('//evil.example/x', Buffer.from('x')), 'x', { baseUrl: base, fetchImpl: spy }), 'fetch', 'another origin');
    expect(called).toBe(false);
  });

  it('reports a connection that cannot be made', async () => {
    await expectLoadError(fetchAsset(ref('/x', Buffer.from('x')), 'x', { baseUrl: 'http://127.0.0.1:1/' }), 'fetch', 'request failed');
  });
});

describe('loadWasm (streaming compile)', () => {
  it('compiles an identity module and verifies it', async () => {
    const m = await loadWasm(ref('/w/ok.wasm', wasm), 'engine', { baseUrl: base });
    expect(WebAssembly.Module.exports(m).map((e) => e.name)).toEqual(['convert']);
  });
  it('compiles a module the server sent with Content-Encoding gzip / br (the HTTP client decodes it)', async () => {
    for (const [url, c] of [['/w/gz.wasm', 'gzip'], ['/w/br.wasm', 'br']] as const) {
      const m = await loadWasm(ref(url, wasm, c), 'engine', { baseUrl: base });
      expect(WebAssembly.Module.imports(m)[0]).toMatchObject({ module: 'env', name: 'memory' });
    }
  });
  it('compiles a stored .gz with no Content-Encoding', async () => {
    const m = await loadWasm(ref('/w/stored.wasm.gz', wasm, 'gzip'), 'engine', { baseUrl: base });
    expect(WebAssembly.Module.exports(m)).toHaveLength(1);
  });
  it('a SHA-256 mismatch is reported as such, not as a CompileError, and no module is returned', async () => {
    await expectLoadError(loadWasm(ref('/w/ok.wasm', wasm, 'identity', { sha256: 'f'.repeat(64) }), 'engine', { baseUrl: base }), 'verify', 'SHA-256 mismatch');
  });
  it('a size mismatch is reported', async () => {
    await expectLoadError(loadWasm(ref('/w/ok.wasm', wasm, 'identity', { size: wasm.length + 1 }), 'engine', { baseUrl: base }), 'verify', 'size mismatch');
  });
  it('when the one buffer for the whole module cannot be allocated, the module is compiled from the verifying stream: a SHA-256 that differs from the manifest is refused there too, the right one compiles', async () => {
    // only the first allocation of exactly the manifest's size fails (what a browser that is out of memory does for the 300-670 MB engine module); the stream path does not allocate by size
    let armed = false;
    const Failing = new Proxy(Uint8Array, { construct(target, args, newTarget) {
      if (armed && args.length === 1 && args[0] === wasm.length) { armed = false; throw new RangeError('Array buffer allocation failed'); }
      return Reflect.construct(target, args, newTarget);
    } });
    vi.stubGlobal('Uint8Array', Failing);
    try {
      armed = true;
      const wrong = await loadWasm(ref('/w/ok.wasm', wasm, 'identity', { sha256: 'e'.repeat(64) }), 'engine', { baseUrl: base }).then(() => null, (x: unknown) => x);
      expect(armed, 'the allocation of the one buffer was tried and failed').toBe(false);
      expect(wrong, 'no module is returned for bytes that are not the manifest\'s').toBeInstanceOf(LoadError);
      expect((wrong as LoadError).stage).toBe('verify');
      expect((wrong as LoadError).message).toContain('SHA-256 mismatch');
      armed = true;
      const m = await loadWasm(ref('/w/ok.wasm', wasm), 'engine', { baseUrl: base });
      expect(armed).toBe(false);
      expect(WebAssembly.Module.exports(m).map((e) => e.name)).toEqual(['convert']);
    } finally { vi.unstubAllGlobals(); }
  });
  it('bytes that are not wasm (but match the manifest) are a compile error that names the URL', async () => {
    const g = routes['/w/garbage.wasm']!.body;
    const e = await expectLoadError(loadWasm(ref('/w/garbage.wasm', g), 'engine', { baseUrl: base }), 'compile', 'compileStreaming failed');
    expect(e.url).toContain('/w/garbage.wasm');
  });
});

describe('fetchManifest: the server\'s explanation of an error status is shown as it is (decision 0152)', () => {
  const REFUSAL = 'このパスは実エンジンのバンドル専用です。\n/assets/manifest.json の engine.kind は "unknown-engine-kind-v0" です(代役)。\n理由: docs/policies/progress-integrity.md\n';
  const NOT_THERE = '実エンジンのバンドルが未配置です。\n/assets/manifest.json が見つかりません。\n';
  it('409 with a plain-text body: the message is "HTTP 409 Conflict", a newline, and the body, unchanged', async () => {
    routes['/m/refused'] = { body: Buffer.from(REFUSAL), status: 409, headers: { 'Content-Type': 'text/plain; charset=utf-8' } };
    const e = await expectLoadError(fetchManifest('/m/refused', { baseUrl: base }), 'manifest', 'HTTP 409');
    expect(e.message).toBe('HTTP 409 Conflict\n' + REFUSAL.trim());
    expect(e.describe().what).toContain('このパスは実エンジンのバンドル専用です');
    expect(e.describe().where).toContain('/m/refused');
  });
  it('404 with a plain-text body: the body says the real engine\'s bundle is not in place', async () => {
    routes['/m/absent'] = { body: Buffer.from(NOT_THERE), status: 404, headers: { 'Content-Type': 'text/plain; charset=utf-8' } };
    const e = await expectLoadError(fetchManifest('/m/absent', { baseUrl: base }), 'manifest', '実エンジンのバンドルが未配置です');
    expect(e.message.startsWith('HTTP 404')).toBe(true);
  });
  it('an error page that is not plain text (HTML) is not copied into the message; neither is an empty body', async () => {
    routes['/m/html'] = { body: Buffer.from('<html><body>Bad gateway</body></html>'), status: 502, headers: { 'Content-Type': 'text/html' } };
    const e = await expectLoadError(fetchManifest('/m/html', { baseUrl: base }), 'manifest', 'HTTP 502');
    expect(e.message).toBe('HTTP 502 Bad Gateway');
    routes['/m/empty'] = { body: Buffer.alloc(0), status: 404, headers: { 'Content-Type': 'text/plain' } };
    expect((await expectLoadError(fetchManifest('/m/empty', { baseUrl: base }), 'manifest', 'HTTP 404')).message).toBe('HTTP 404 Not Found');
  });
  it('a very long explanation is cut at 2000 characters', async () => {
    routes['/m/long'] = { body: Buffer.from('x'.repeat(9000)), status: 500, headers: { 'Content-Type': 'text/plain' } };
    const e = await expectLoadError(fetchManifest('/m/long', { baseUrl: base }), 'manifest', 'HTTP 500');
    expect(e.message).toBe('HTTP 500 Internal Server Error\n' + 'x'.repeat(2000));
  });
  it('the explanation is only for the manifest: a failing big asset keeps its short message', async () => {
    routes['/d/refused.bin'] = { body: Buffer.from(REFUSAL), status: 409, headers: { 'Content-Type': 'text/plain' } };
    const e = await expectLoadError(fetchAsset(ref('/d/refused.bin', Buffer.from('x')), 'x', { baseUrl: base }), 'fetch', 'HTTP 409');
    expect(e.message).toBe('HTTP 409 Conflict');
  });
});

describe('loadPack and fetchManifest', () => {
  it('loads a pack and verifies every file inside it', async () => {
    const p = await loadPack({ ...ref('/d/seed.flpack', pack), format: 'flpack1' }, 'seed', { baseUrl: base });
    expect(new TextDecoder().decode(p.files.get('a.txt')?.data)).toBe('hello');
  });
  it('a pack whose bytes match the manifest but are not a pack is a pack error', async () => {
    await expectLoadError(loadPack({ ...ref('/d/notapack.flpack', routes['/d/notapack.flpack']!.body), format: 'flpack1' }, 'seed', { baseUrl: base }), 'pack', 'not a valid flpack1 pack');
  });
  it('fetchManifest lists the schema errors', async () => {
    routes['/m/bad.json'] = { body: Buffer.from(JSON.stringify({ schema: 1, bundle: { name: 'x' }, engine: {}, seedPack: null, dllPack: null })) };
    const e = await expectLoadError(fetchManifest('/m/bad.json', { baseUrl: base }), 'manifest', 'does not match schema 1');
    expect(e.message).toContain('/bundle/version');
    expect(e.message).toContain('/engine/kind');
  });
  it('fetchManifest says when the body is not JSON, and refuses another origin', async () => {
    routes['/m/notjson'] = { body: Buffer.from('<html>') };
    await expectLoadError(fetchManifest('/m/notjson', { baseUrl: base }), 'manifest', 'not valid JSON');
    await expectLoadError(fetchManifest('https://example.com/m.json', { baseUrl: base }), 'manifest', 'another origin');
  });
});


describe('explainCompileError', () => {
  it('says "out of memory" for the errors V8 throws when it cannot allocate', () => {
    expect(explainCompileError(new RangeError('WebAssembly.compileStreaming(): Out of memory: Cannot allocate Wasm module'))).toMatch(/ran out of memory while compiling the engine wasm/);
    expect(explainCompileError(new RangeError('Array buffer allocation failed'))).toMatch(/ran out of memory/);
  });
  it('keeps every other message as it is', () => {
    expect(explainCompileError(new Error('CompileError: expected magic word'))).toBe('WebAssembly.compileStreaming failed: Error: CompileError: expected magic word');
  });
});
