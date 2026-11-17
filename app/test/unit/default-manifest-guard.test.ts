// Decision 0152: the default manifest path /assets/manifest.json is for the real engine's bundle only (docs/policies/progress-integrity.md).
// Part 1 is the decision itself (shared/manifest-guard.ts); part 2 runs it through tools/assets-middleware.ts (vite dev / vite preview) against a real directory, including the ways around it.
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import zlib from 'node:zlib';
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { REAL_ENGINE_KINDS, isDefaultManifestKey, judgeManifestBytes, judgeManifestText, logicalKey } from '../../shared/manifest-guard.ts';
import { assetsMiddleware } from '../../tools/assets-middleware.ts';
import { rawGet, rawGetPath, type Raw } from '../helpers/asset-suite.ts';
import { MANIFEST_REAL, MANIFEST_UNKNOWN_KIND, NOT_DEPLOYED_WORDS, REFUSAL_WORDS } from '../helpers/fixtures.ts';

const text = (r: Raw): string => r.body.toString('utf8');
const expectWords = (r: Raw, words: string[]): void => { for (const w of words) expect(text(r), w).toContain(w); };

describe('manifest-guard: the engine kinds', () => {
  it('the allow-list is the real engine only; the stand-in is named as such', () => {
    expect(REAL_ENGINE_KINDS).toEqual(['emscripten-fruity-v1']);
  });
  it('a real-engine manifest passes and says its kind', () => {
    expect(judgeManifestText(MANIFEST_REAL)).toEqual({ ok: true, kind: 'emscripten-fruity-v1' });
  });
  it('a stand-in manifest is 409 with the PM\'s words', () => {
    const v = judgeManifestText(MANIFEST_UNKNOWN_KIND);
    expect(v.ok).toBe(false);
    if (v.ok) return;
    expect(v.status).toBe(409);
    for (const w of REFUSAL_WORDS) expect(v.body).toContain(w);
  });
  it.each([
    ['no engine', '{}'],
    ['engine is a string', '{"engine":"emscripten-fruity-v1"}'],
    ['engine is an array', '{"engine":["emscripten-fruity-v1"]}'],
    ['kind is a number', '{"engine":{"kind":1}}'],
    ['kind missing', '{"engine":{}}'],
    ['top level is an array', '[{"engine":{"kind":"emscripten-fruity-v1"}}]'],
    ['top level is null', 'null'],
  ])('%s: 409, because the kind cannot be read', (_n, body) => {
    const v = judgeManifestText(body);
    expect(v).toMatchObject({ ok: false, status: 409 });
    if (!v.ok) { expect(v.body).toContain('このパスは実エンジンのバンドル専用です'); expect(v.body).toContain('engine.kind が読めません'); }
  });
  it.each(['surface-table-v2', 'Emscripten-Fruity-V1', 'emscripten-fruity-v1 ', 'emscripten-fruity-v2', ''])('an unknown kind %j is refused too (an allow-list: a new stand-in must not pass by being new)', (kind) => {
    const v = judgeManifestText(JSON.stringify({ engine: { kind } }));
    expect(v).toMatchObject({ ok: false, status: 409 });
    if (!v.ok) expect(v.body).toContain('実エンジンの種類(emscripten-fruity-v1)ではありません');
  });
  it('a kind with a huge name is cut in the message, not echoed whole', () => {
    const v = judgeManifestText(JSON.stringify({ engine: { kind: 'x'.repeat(10000) } }));
    expect(v.ok).toBe(false);
    if (!v.ok) expect(v.body.length).toBeLessThan(800);
  });
  it.each(['', '{', '{"engine":{"kind":"emscripten-fruity-v1"}', 'not json', '<html>', "{'engine':{}}"])('JSON that does not parse (%j) is 500 and says so', (body) => {
    const v = judgeManifestText(body);
    expect(v).toMatchObject({ ok: false, status: 500 });
    if (!v.ok) { expect(v.body).toContain('が壊れています'); expect(v.body).toContain('JSON として読めません'); }
  });
  it('bytes: invalid UTF-8 is 500; a UTF-8 BOM is accepted', () => {
    expect(judgeManifestBytes(Buffer.from([0xff, 0xfe, 0x00, 0x7b]))).toMatchObject({ ok: false, status: 500 });
    expect(judgeManifestBytes(Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), Buffer.from(MANIFEST_REAL)]))).toMatchObject({ ok: true });
    expect(judgeManifestBytes(Buffer.from(MANIFEST_UNKNOWN_KIND))).toMatchObject({ ok: false, status: 409 });
  });
});

describe('manifest-guard: which keys are the default manifest (decoded once, normalized, then judged)', () => {
  it.each([
    'manifest.json', 'Manifest.json', 'MANIFEST.JSON', 'mAnIfEsT.jSoN',
    '/manifest.json', '//manifest.json', '///manifest.json', './manifest.json', '././manifest.json', './/./manifest.json',
    'a/../manifest.json', 'a/b/../../manifest.json', '../manifest.json', '../../manifest.json', 'a/./../manifest.json',
    'manifest.json.', 'manifest.json..', 'manifest.json ', 'manifest.json. .',
    'ｍａｎｉｆｅｓｔ.json', 'x/..\\manifest.json',
    '\\manifest.json', 'a\\..\\manifest.json',
  ])('%j is the default manifest', (rel) => {
    expect(isDefaultManifestKey(rel), rel).toBe(true);
  });
  it.each([
    'standin-x/manifest.json', 'x/manifest.json', 'a/../b/manifest.json', 'manifest.json.br', 'manifest.json.gz', 'manifest.jsonx', 'xmanifest.json', 'manifest-2.json', 'manifest.json/',
    'manifest.json/x', '', '.', '..', 'a', '%2e/manifest.json', 'manifest%2ejson', 'manifest.json/.',
  ])('%j is not', (rel) => {
    expect(isDefaultManifestKey(rel), rel).toBe(false);
  });
  it('logicalKey keeps a trailing slash (a directory is not the manifest) and folds case', () => {
    expect(logicalKey('Manifest.json/')).toBe('manifest.json/');
    expect(logicalKey('A/B')).toBe('a/b');
    expect(logicalKey('a//b/./c/../d')).toBe('a/b/d');
  });
});

describe('middleware: /assets/manifest.json', () => {
  let dir: string;
  let server: http.Server;
  let base = '';
  let noDirServer: http.Server;
  let noDirBase = '';
  const listen = (s: http.Server): Promise<string> => new Promise((res) => s.listen(0, '127.0.0.1', () => res(`http://127.0.0.1:${(s.address() as { port: number }).port}`)));
  const rootManifest = (): string => path.join(dir, 'manifest.json');
  const put = (rel: string, body: string | Buffer): void => { const f = path.join(dir, rel); fs.mkdirSync(path.dirname(f), { recursive: true }); fs.writeFileSync(f, body); };

  beforeAll(async () => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'web1-guard-'));
    const mw = assetsMiddleware(dir);
    server = http.createServer((req, res) => mw(req, res, () => { res.writeHead(404); res.end('next'); }));
    base = await listen(server);
    const mw2 = assetsMiddleware('');
    noDirServer = http.createServer((req, res) => mw2(req, res, () => { res.writeHead(404); res.end('next'); }));
    noDirBase = await listen(noDirServer);
  });
  afterAll(() => { server.close(); noDirServer.close(); fs.rmSync(dir, { recursive: true, force: true }); });
  beforeEach(() => { for (const e of fs.readdirSync(dir)) fs.rmSync(path.join(dir, e), { recursive: true, force: true }); });

  describe('a stand-in at the default path', () => {
    beforeEach(() => { put('manifest.json', MANIFEST_UNKNOWN_KIND); });

    it('GET is 409, plain text, no-store, with the PM\'s words, the isolation headers, and none of the stand-in manifest', async () => {
      const r = await rawGet(`${base}/assets/manifest.json`);
      expect(r.status).toBe(409);
      expect(String(r.headers['content-type'])).toContain('text/plain');
      expect(r.headers['cache-control']).toBe('no-store');
      expect(r.headers['cross-origin-embedder-policy']).toBe('require-corp');
      expectWords(r, REFUSAL_WORDS);
      expect(text(r)).not.toContain('"schema"');
      expect(text(r)).not.toContain('refused-kind-test');
      expect(r.headers.etag).toBeUndefined();
    });
    it('HEAD is 409 without a body; a Range, If-None-Match, If-Range or Accept-Encoding does not get around it', async () => {
      expect((await rawGet(`${base}/assets/manifest.json`, {}, 'HEAD')).status).toBe(409);
      expect((await rawGet(`${base}/assets/manifest.json`, {}, 'HEAD')).body.length).toBe(0);
      for (const h of [{ range: 'bytes=0-9' }, { 'if-none-match': '*' }, { 'if-none-match': '"1-1"' }, { 'if-range': '"x"', range: 'bytes=0-1' }, { 'accept-encoding': 'br, gzip' }] as Record<string, string>[]) {
        const r = await rawGet(`${base}/assets/manifest.json`, h);
        expect(r.status, JSON.stringify(h)).toBe(409);
        expect(text(r), JSON.stringify(h)).not.toContain('"schema"');
      }
    });
    it('a query string makes no difference', async () => {
      for (const q of ['?x=1', '?bundle=/assets/manifest.json', '?', '?a=b&c=d', '?%2e%2e', '?/../manifest.json']) {
        const r = await rawGet(`${base}/assets/manifest.json${q}`);
        expect(r.status, q).toBe(409);
        expectWords(r, REFUSAL_WORDS);
      }
    });
    it.each([
      '/assets//manifest.json', '/assets///manifest.json', '/assets/./manifest.json', '/assets/%2e/manifest.json', '/assets/%2E/manifest.json', '/assets/x/../manifest.json', '/assets/x/%2e%2e/manifest.json', '/assets/x/%2E%2E/manifest.json',
      '/assets/../assets/manifest.json', '/assets/%6danifest.json', '/assets/%6Danifest.json', '/assets/manifest%2ejson', '/assets/manifest%2Ejson', '/assets/%6d%61%6e%69%66%65%73%74%2e%6a%73%6f%6e', '/assets/.//manifest.json',
      '/assets/a/b/../../manifest.json', '/assets/a/%2e%2e/%2e%2e/assets/manifest.json?',
    ])('the path %s does not get around it: never 200', async (p) => {
      const r = await rawGetPath(base, p);
      expect(r.status, p).not.toBe(200);
      expect([409, 404, 400], p).toContain(r.status);
      expect(text(r), p).not.toContain('"schema"');
      if (r.status === 409) expectWords(r, REFUSAL_WORDS);
    });
    it.each(['/assets/Manifest.json', '/assets/MANIFEST.JSON', '/assets/mAnIfEsT.jSoN', '/assets/manifest.json.', '/assets/manifest.json%20', `/assets/${encodeURIComponent('ｍａｎｉｆｅｓｔ.json')}`, '/assets/MANIFEST%2EJSON'])('the spelling %s is answered as the default manifest (here: nothing is stored under it), never as a plain "not found"', async (p) => {
      const r = await rawGetPath(base, p);
      expect(r.status, p).toBe(404);
      expectWords(r, NOT_DEPLOYED_WORDS);
    });
    it('on a file system that folds case, a differently-cased request would reach the stand-in file: the guard judges by the logical key, not by the stored spelling', async () => {
      // simulated: the stand-in is stored as "Manifest.json" only. The request "/assets/manifest.json" does not find it here (Linux) -> 404 with the explicit message, never 200.
      fs.rmSync(rootManifest());
      put('Manifest.json', MANIFEST_UNKNOWN_KIND);
      const r = await rawGetPath(base, '/assets/Manifest.json');
      expect(r.status).toBe(409);
      expectWords(r, REFUSAL_WORDS);
      expect((await rawGetPath(base, '/assets/manifest.json')).status).toBe(404);
    });
    it('a symlink named manifest.json that points at the stand-in is judged by what it reaches', async () => {
      fs.rmSync(rootManifest());
      put('standin-x/manifest.json', MANIFEST_UNKNOWN_KIND);
      fs.symlinkSync(path.join(dir, 'standin-x/manifest.json'), rootManifest());
      const r = await rawGet(`${base}/assets/manifest.json`);
      expect(r.status).toBe(409);
      expectWords(r, REFUSAL_WORDS);
    });
    it('the same file under an explicit name is served as before (a stand-in is allowed at /assets/<name>/manifest.json)', async () => {
      put('standin-stop/manifest.json', MANIFEST_UNKNOWN_KIND);
      const r = await rawGet(`${base}/assets/standin-stop/manifest.json`);
      expect(r.status).toBe(200);
      expect(text(r)).toBe(MANIFEST_UNKNOWN_KIND);
      const q = await rawGet(`${base}/assets/standin-stop/manifest.json?bundle=1`);
      expect(q.status).toBe(200);
    });
    it('POST is still 405 (the method check comes first)', async () => {
      expect((await rawGet(`${base}/assets/manifest.json`, {}, 'POST')).status).toBe(405);
    });
  });

  describe('the real engine\'s manifest at the default path', () => {
    beforeEach(() => { put('manifest.json', MANIFEST_REAL); });
    it('GET is 200 with exactly the stored bytes, JSON, ETag, no-cache', async () => {
      const r = await rawGet(`${base}/assets/manifest.json`);
      expect(r.status).toBe(200);
      expect(text(r)).toBe(MANIFEST_REAL);
      expect(r.headers['content-type']).toBe('application/json');
      expect(r.headers.etag).toMatch(/^"[^"]+"$/);
      expect(r.headers['cache-control']).toBe('no-cache');
      expect(r.headers['content-length']).toBe(String(Buffer.byteLength(MANIFEST_REAL)));
    });
    it('a query string, //, ./, %2e, case: 200 where the file is found (the same bytes), and the odd spellings that find no file are 404, not a stand-in', async () => {
      for (const p of ['/assets/manifest.json?x=1', '/assets//manifest.json', '/assets/./manifest.json', '/assets/%2e/manifest.json', '/assets/x/../manifest.json', '/assets/%6danifest.json']) {
        const r = await rawGetPath(base, p);
        expect(r.status, p).toBe(200);
        expect(text(r), p).toBe(MANIFEST_REAL);
      }
    });
    it('HEAD, Range (206 from the judged bytes), If-None-Match (304) and 416 work as for any asset', async () => {
      const g = await rawGet(`${base}/assets/manifest.json`);
      const h = await rawGet(`${base}/assets/manifest.json`, {}, 'HEAD');
      expect(h.status).toBe(200);
      expect(h.body.length).toBe(0);
      expect(h.headers['content-length']).toBe(g.headers['content-length']);
      const r = await rawGet(`${base}/assets/manifest.json`, { range: 'bytes=2-9' });
      expect(r.status).toBe(206);
      expect(r.headers['content-range']).toBe(`bytes 2-9/${Buffer.byteLength(MANIFEST_REAL)}`);
      expect(text(r)).toBe(MANIFEST_REAL.slice(2, 10));
      expect((await rawGet(`${base}/assets/manifest.json`, { 'if-none-match': String(g.headers.etag) })).status).toBe(304);
      expect((await rawGet(`${base}/assets/manifest.json`, { range: 'bytes=99999-' })).status).toBe(416);
    });
    it('a stored brotli / gzip variant of the real manifest is judged after decoding and is served with its Content-Encoding', async () => {
      put('manifest.json.br', zlib.brotliCompressSync(Buffer.from(MANIFEST_REAL)));
      const r = await rawGet(`${base}/assets/manifest.json`, { 'accept-encoding': 'br' });
      expect(r.status).toBe(200);
      expect(r.headers['content-encoding']).toBe('br');
      expect(zlib.brotliDecompressSync(r.body).toString()).toBe(MANIFEST_REAL);
      put('manifest.json.gz', zlib.gzipSync(Buffer.from(MANIFEST_REAL)));
      const g = await rawGet(`${base}/assets/manifest.json`, { 'accept-encoding': 'gzip' });
      expect(g.headers['content-encoding']).toBe('gzip');
      expect(zlib.gunzipSync(g.body).toString()).toBe(MANIFEST_REAL);
    });
    it('a stand-in hidden in the .br or .gz variant (the plain file is real) is refused for the clients that would get it', async () => {
      put('manifest.json.br', zlib.brotliCompressSync(Buffer.from(MANIFEST_UNKNOWN_KIND)));
      put('manifest.json.gz', zlib.gzipSync(Buffer.from(MANIFEST_UNKNOWN_KIND)));
      for (const ae of ['br', 'gzip', 'gzip, br']) {
        const r = await rawGet(`${base}/assets/manifest.json`, { 'accept-encoding': ae });
        expect(r.status, ae).toBe(409);
        expectWords(r, REFUSAL_WORDS);
      }
      const plain = await rawGet(`${base}/assets/manifest.json`);
      expect(plain.status).toBe(200);
    });
    it('a variant that does not decode is 500 for the clients that would get it', async () => {
      put('manifest.json.br', Buffer.from('this is not brotli'));
      const r = await rawGet(`${base}/assets/manifest.json`, { 'accept-encoding': 'br' });
      expect(r.status).toBe(500);
      expect(text(r)).toContain('br として展開できません');
      put('manifest.json.gz', Buffer.from('this is not gzip'));
      expect((await rawGet(`${base}/assets/manifest.json`, { 'accept-encoding': 'gzip' })).status).toBe(500);
    });
    it('the stored bytes of an explicit NAME.br are not a manifest and are served as before', async () => {
      const br = zlib.brotliCompressSync(Buffer.from(MANIFEST_UNKNOWN_KIND));
      put('manifest.json.br', br);
      const r = await rawGet(`${base}/assets/manifest.json.br`);
      expect(r.status).toBe(200);
      expect(r.headers['content-encoding']).toBeUndefined();
      expect(r.body.equals(br)).toBe(true);
    });
  });

  describe('nothing, or something broken, at the default path', () => {
    it('nothing: 404 whose body says that the real engine\'s bundle is not in place', async () => {
      const r = await rawGet(`${base}/assets/manifest.json`);
      expect(r.status).toBe(404);
      expect(String(r.headers['content-type'])).toContain('text/plain');
      expectWords(r, NOT_DEPLOYED_WORDS);
      expect(text(r)).not.toBe('not found: /assets/manifest.json');
      expect((await rawGet(`${base}/assets/manifest.json`, {}, 'HEAD')).status).toBe(404);
      expect((await rawGet(`${base}/assets/manifest.json?x=1`)).status).toBe(404);
    });
    it('other missing files keep the plain "not found" body', async () => {
      const r = await rawGet(`${base}/assets/nothing.bin`);
      expect(r.status).toBe(404);
      expect(text(r)).toBe('not found: /assets/nothing.bin');
      const m = await rawGet(`${base}/assets/x/manifest.json`);
      expect(m.status).toBe(404);
      expect(text(m)).toBe('not found: /assets/x/manifest.json');
    });
    it('with a stand-in under another name only, the default path still says "not in place" (the stand-in is not offered instead)', async () => {
      put('standin-stop/manifest.json', MANIFEST_UNKNOWN_KIND);
      const r = await rawGet(`${base}/assets/manifest.json`);
      expect(r.status).toBe(404);
      expectWords(r, NOT_DEPLOYED_WORDS);
      expect(text(r)).not.toContain('"schema"');
    });
    it('a directory named manifest.json is 404 (not deployed)', async () => {
      fs.mkdirSync(rootManifest());
      const r = await rawGet(`${base}/assets/manifest.json`);
      expect(r.status).toBe(404);
      expectWords(r, NOT_DEPLOYED_WORDS);
    });
    it.each([
      ['empty file', ''],
      ['truncated', MANIFEST_REAL.slice(0, 40)],
      ['not JSON', 'ZZQ engine: emscripten-fruity-v1'],
      ['HTML', '<!doctype html><title>ZZQ</title>'],
      ['NUL bytes', '\u0000\u0000\u0000'],
    ])('%s: 500 that names the file as broken, and the content is not served', async (_n, body) => {
      put('manifest.json', body);
      const r = await rawGet(`${base}/assets/manifest.json`);
      expect(r.status).toBe(500);
      expectWords(r, ['/assets/manifest.json が壊れています', 'JSON として読めません', 'docs/policies/progress-integrity.md']);
      expect(text(r), 'the content of the file is not echoed').not.toContain('ZZQ');
    });
    it('invalid UTF-8: 500', async () => {
      put('manifest.json', Buffer.from([0x7b, 0xff, 0xfe, 0x7d]));
      const r = await rawGet(`${base}/assets/manifest.json`);
      expect(r.status).toBe(500);
      expect(text(r)).toContain('UTF-8 として読めません');
    });
    it('a manifest too big to inspect: 500, not served', async () => {
      put('manifest.json', Buffer.concat([Buffer.from('{"engine":{"kind":"emscripten-fruity-v1"},"pad":"'), Buffer.alloc(3 * 1024 * 1024, 0x61), Buffer.from('"}')]));
      const r = await rawGet(`${base}/assets/manifest.json`);
      expect(r.status).toBe(500);
      expect(text(r)).toContain('大きすぎて検査できません');
    });
    it('valid JSON without a readable engine.kind: 409', async () => {
      put('manifest.json', '{}');
      const r = await rawGet(`${base}/assets/manifest.json`);
      expect(r.status).toBe(409);
      expectWords(r, ['このパスは実エンジンのバンドル専用です', 'engine.kind が読めません']);
    });
    it('an unknown engine.kind: 409 that names it', async () => {
      put('manifest.json', JSON.stringify({ engine: { kind: 'surface-table-v2' } }));
      const r = await rawGet(`${base}/assets/manifest.json`);
      expect(r.status).toBe(409);
      expectWords(r, ['"surface-table-v2"', '実エンジンの種類(emscripten-fruity-v1)ではありません']);
    });
    it('the answer follows the file: stand-in, then real, then removed (no caching of the verdict)', async () => {
      put('manifest.json', MANIFEST_UNKNOWN_KIND);
      expect((await rawGet(`${base}/assets/manifest.json`)).status).toBe(409);
      put('manifest.json', MANIFEST_REAL);
      expect((await rawGet(`${base}/assets/manifest.json`)).status).toBe(200);
      put('manifest.json', MANIFEST_UNKNOWN_KIND);
      expect((await rawGet(`${base}/assets/manifest.json`)).status).toBe(409);
      fs.rmSync(rootManifest());
      expect((await rawGet(`${base}/assets/manifest.json`)).status).toBe(404);
    });
  });

  it('without FL_ASSETS_DIR the default path says both: the real engine\'s bundle is not in place, and FL_ASSETS_DIR is not set', async () => {
    const r = await rawGet(`${noDirBase}/assets/manifest.json`);
    expect(r.status).toBe(404);
    expectWords(r, [...NOT_DEPLOYED_WORDS, 'FL_ASSETS_DIR is not set']);
    const o = await rawGet(`${noDirBase}/assets/x/manifest.json`);
    expect(text(o)).toBe('FL_ASSETS_DIR is not set: no assets directory is served at /assets/');
  });
});
