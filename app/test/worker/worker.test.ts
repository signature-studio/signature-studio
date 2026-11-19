import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import zlib from 'node:zlib';
import { afterAll, describe, expect, it } from 'vitest';
import { assetSuite, rawGet, rawGetPath, type Raw } from '../helpers/asset-suite.ts';
import { MANIFEST_REAL, MANIFEST_UNKNOWN_KIND, NOT_DEPLOYED_WORDS, REFUSAL_WORDS } from '../helpers/fixtures.ts';
import { deleteObject, getPerfObject, listLocalKeys, putObject } from '../helpers/wrangler.mjs';

const need = (n: string): string => { const v = process.env[n]; if (!v) throw new Error(`${n} is not set: run these tests with "npm run test:worker"`); return v; };
const DEV = need('WORKER_DEV_URL'), ACCESS = need('WORKER_ACCESS_URL'), CLOSED = need('WORKER_CLOSED_URL');
const GUARD = need('WORKER_GUARD_URL'), GUARD_WORK = need('WORKER_GUARD_WORK');
const DEV_WORK = need('WORKER_DEV_WORK'), NOPERF = need('WORKER_NOPERF_URL');
const tokens = JSON.parse(fs.readFileSync(need('WORKER_TOKENS'), 'utf8')) as Record<string, string>;

assetSuite('Worker + local R2 (behind the test Access proxy)', () => DEV, { brVariantIsSibling: false });

describe('Worker (behind the test Access proxy): the app and its headers', () => {
  it('serves index.html from Static Assets with COOP / COEP / CORP, a CSP that names only self, and noindex', async () => {
    const r = await rawGet(`${DEV}/`);
    expect(r.status).toBe(200);
    expect(String(r.headers['content-type'])).toContain('text/html');
    expect(r.headers['cross-origin-opener-policy']).toBe('same-origin');
    expect(r.headers['cross-origin-embedder-policy']).toBe('require-corp');
    expect(r.headers['cross-origin-resource-policy']).toBe('same-origin');
    expect(String(r.headers['x-robots-tag'])).toContain('noindex');
    const csp = String(r.headers['content-security-policy']);
    expect(csp).toContain("default-src 'self'");
    expect(csp).not.toMatch(/https?:|\*/);
    expect(r.body.toString()).toContain('name="robots" content="noindex,nofollow"');
  });
  it('serves the built JS and the engine Worker script from /static/ (not /assets/, which is R2) with the isolation headers', async () => {
    const html = (await rawGet(`${DEV}/`)).body.toString();
    const js = /src="(\/static\/[^"]+\.js)"/.exec(html)?.[1];
    expect(js).toBeTruthy();
    const r = await rawGet(`${DEV}${js}`);
    expect(r.status).toBe(200);
    expect(String(r.headers['content-type'])).toContain('javascript');
    expect(r.headers['cross-origin-embedder-policy']).toBe('require-corp');
    expect(r.headers['cross-origin-resource-policy']).toBe('same-origin');
  });
  it('refuses writes to the app', async () => {
    expect((await rawGet(`${DEV}/`, {}, 'POST')).status).toBe(405);
  });
});

describe('Worker with Access validation on', () => {
  const get = (path: string, token?: string) => rawGet(`${ACCESS}${path}`, token ? { 'cf-access-jwt-assertion': token } : {});
  it('a valid token reaches the app and the R2 objects', async () => {
    expect((await get('/', tokens.valid)).status).toBe(200);
    const r = await get('/assets/blob.bin', tokens.valid);
    expect(r.status).toBe(200);
  });
  it('no token: 403 on the app and on /assets', async () => {
    expect((await get('/')).status).toBe(403);
    expect((await get('/assets/blob.bin')).status).toBe(403);
  });
  it.each(['wrongAud', 'wrongIss', 'expired', 'forged', 'noneAlg'])('a %s token: 403, and the body does not leak object data', async (name) => {
    const r = await get('/assets/blob.bin', tokens[name]);
    expect(r.status).toBe(403);
    expect(r.body.length).toBeLessThan(400);
    expect(r.headers['cross-origin-opener-policy']).toBe('same-origin');
  });
  it('403 says why (aud) so the operator can fix the configuration', async () => {
    expect((await get('/', tokens.wrongAud)).body.toString()).toContain('aud');
  });
  it('a malformed token is 403, not 500', async () => {
    expect((await get('/', 'not-a-jwt')).status).toBe(403);
    expect((await get('/', 'a.b.c')).status).toBe(403);
  });
  it('the R2 suite also passes behind Access with a valid token (Range and ETag are unaffected)', async () => {
    const r = await rawGet(`${ACCESS}/assets/blob.bin`, { 'cf-access-jwt-assertion': tokens.valid ?? '', range: 'bytes=10-19' });
    expect(r.status).toBe(206);
    expect(r.headers['content-range']).toMatch(/^bytes 10-19\//);
  });
});

describe('Worker as shipped (no team domain / AUD): fails closed', () => {
  it('403 for everything, even with a token', async () => {
    for (const p of ['/', '/assets/blob.bin', '/assets/manifest.json', '/static/x.js']) {
      expect((await rawGet(`${CLOSED}${p}`)).status, p).toBe(403);
      expect((await rawGet(`${CLOSED}${p}`, { 'cf-access-jwt-assertion': tokens.valid ?? '' })).status, p).toBe(403);
    }
    expect((await rawGet(`${CLOSED}/`)).body.toString()).toContain('ACCESS_TEAM_DOMAIN');
  });
});


// ---- decision 0152: the default manifest (R2 key manifest.json) is for the real engine's bundle only ----
describe('Worker + local R2: the default manifest is for the real engine only (decision 0152)', () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'web1-wguard-'));
  afterAll(() => { fs.rmSync(tmp, { recursive: true, force: true }); });
  const text = (r: Raw): string => r.body.toString('utf8');
  const words = (r: Raw, ws: string[]): void => { for (const w of ws) expect(text(r), w).toContain(w); };
  let n = 0;
  const put = async (key: string, body: string | Buffer, opts: { contentEncoding?: string } = {}): Promise<void> => {
    const f = path.join(tmp, `o${n++}`);
    fs.writeFileSync(f, body);
    await putObject(GUARD_WORK, key, f, { contentType: 'application/json', ...opts });
  };
  const clear = async (...keys: string[]): Promise<void> => { for (const k of keys) await deleteObject(GUARD_WORK, k).catch(() => undefined); };
  const get = (p: string, h: Record<string, string> = {}, m = 'GET'): Promise<Raw> => rawGetPath(GUARD, p, h, m);

  it('(b) the real engine\'s manifest: 200 with the stored bytes (the dev instance, whose R2 holds one), also with a query string; HEAD, Range and If-None-Match work', async () => {
    const r = await rawGet(`${DEV}/assets/manifest.json`);
    expect(r.status).toBe(200);
    expect(text(r)).toBe(MANIFEST_REAL);
    expect((await rawGet(`${DEV}/assets/manifest.json?x=1&bundle=/assets/manifest.json`)).status).toBe(200);
    expect((await rawGet(`${DEV}/assets/manifest.json`, {}, 'HEAD')).status).toBe(200);
    const rg = await rawGet(`${DEV}/assets/manifest.json`, { range: 'bytes=2-9' });
    expect(rg.status).toBe(206);
    expect(text(rg)).toBe(MANIFEST_REAL.slice(2, 10));
    expect((await rawGet(`${DEV}/assets/manifest.json`, { 'if-none-match': String(r.headers.etag) })).status).toBe(304);
    expect((await rawGet(`${DEV}/assets/manifest.json`, { range: 'bytes=99999-' })).status).toBe(416);
  });

  it('(c) nothing stored: 404 whose body says that the real engine\'s bundle is not in place (GET, HEAD, with a query)', async () => {
    for (const p of ['/assets/manifest.json', '/assets/manifest.json?x=1']) {
      const r = await get(p);
      expect(r.status, p).toBe(404);
      expect(String(r.headers['content-type'])).toContain('text/plain');
      words(r, NOT_DEPLOYED_WORDS);
    }
    expect((await get('/assets/manifest.json', {}, 'HEAD')).status).toBe(404);
    expect(text(await get('/assets/nothing.bin'))).toBe('not found: /assets/nothing.bin');
    expect(text(await get('/assets/x/manifest.json'))).toBe('not found: /assets/x/manifest.json');
  });

  it('(a) a stand-in at the default key: 409 with the PM\'s words, for GET, HEAD, Range, If-None-Match and a query; none of its content', async () => {
    await put('manifest.json', MANIFEST_UNKNOWN_KIND);
    const r = await get('/assets/manifest.json');
    expect(r.status).toBe(409);
    expect(String(r.headers['content-type'])).toContain('text/plain');
    expect(r.headers['cache-control']).toContain('no-store');
    expect(r.headers['cross-origin-embedder-policy']).toBe('require-corp');
    words(r, REFUSAL_WORDS);
    expect(text(r)).not.toContain('"schema"');
    expect(r.headers.etag).toBeUndefined();
    expect((await get('/assets/manifest.json', {}, 'HEAD')).status).toBe(409);
    for (const h of [{ range: 'bytes=0-9' }, { 'if-none-match': '*' }, { 'if-range': '"x"', range: 'bytes=0-1' }, { 'accept-encoding': 'br, gzip' }] as Record<string, string>[]) {
      const x = await get('/assets/manifest.json', h);
      expect(x.status, JSON.stringify(h)).toBe(409);
      expect(text(x)).not.toContain('"schema"');
    }
    for (const q of ['?x=1', '?bundle=/assets/manifest.json', '?']) expect((await get(`/assets/manifest.json${q}`)).status, q).toBe(409);
  });

  it('(f) the ways around it do not work: //, ./, %2e, %2e%2e, percent-encoded letters, a path from another prefix: never 200 and never the stand-in', async () => {
    for (const p of ['/assets//manifest.json', '/assets///manifest.json', '/assets/./manifest.json', '/assets/%2e/manifest.json', '/assets/%2E/manifest.json', '/assets/x/../manifest.json', '/assets/x/%2e%2e/manifest.json',
      '/assets/%6danifest.json', '/assets/manifest%2ejson', '/assets/%6d%61%6e%69%66%65%73%74%2e%6a%73%6f%6e', '/assets/../assets/manifest.json', '/assets/a/b/../../manifest.json']) {
      const r = await get(p);
      expect(r.status, p).not.toBe(200);
      expect([409, 400, 404], p).toContain(r.status);
      expect(text(r), p).not.toContain('"schema"');
      if (r.status === 409) words(r, REFUSAL_WORDS);
    }
  });

  it('(f) case: the R2 key is case-sensitive, but a stand-in stored as "Manifest.json" is still refused at that spelling; an absent spelling is the default manifest\'s 404, not a bare "not found"', async () => {
    await put('Manifest.json', MANIFEST_UNKNOWN_KIND);
    await put('MANIFEST.JSON', MANIFEST_UNKNOWN_KIND);
    for (const k of ['Manifest.json', 'MANIFEST.JSON']) {
      const r = await get(`/assets/${k}`);
      expect(r.status, k).toBe(409);
      words(r, REFUSAL_WORDS);
    }
    const absent = await get('/assets/mAnIfEsT.jSoN');
    expect(absent.status).toBe(404);
    words(absent, NOT_DEPLOYED_WORDS);
    await clear('Manifest.json', 'MANIFEST.JSON');
  });

  it('(d) a manifest that is not JSON: 500 that says it is broken; the content is not served', async () => {
    for (const body of ['', '{', '<html>ZZQ', 'ZZQ not json', MANIFEST_REAL.slice(0, 60)]) {
      await put('manifest.json', body);
      const r = await get('/assets/manifest.json');
      expect(r.status, JSON.stringify(body)).toBe(500);
      words(r, ['/assets/manifest.json が壊れています', 'JSON として読めません', 'docs/policies/progress-integrity.md']);
      expect(text(r), 'the content of the file is not echoed').not.toContain('ZZQ');
    }
  });

  it('valid JSON without a readable kind, or with a kind that is not the real engine: 409', async () => {
    await put('manifest.json', '{}');
    const a = await get('/assets/manifest.json');
    expect(a.status).toBe(409);
    words(a, ['engine.kind が読めません']);
    await put('manifest.json', JSON.stringify({ engine: { kind: 'surface-table-v2' } }));
    const b = await get('/assets/manifest.json');
    expect(b.status).toBe(409);
    words(b, ['"surface-table-v2"', '実エンジンの種類(emscripten-fruity-v1)ではありません']);
  });

  it('a manifest stored gzip-encoded is judged after decoding: stand-in 409, real 200 with its Content-Encoding, broken gzip 500; brotli cannot be inspected: 500 that says so', async () => {
    await put('manifest.json', zlib.gzipSync(Buffer.from(MANIFEST_UNKNOWN_KIND)), { contentEncoding: 'gzip' });
    const s = await get('/assets/manifest.json');
    expect(s.status).toBe(409);
    words(s, REFUSAL_WORDS);
    await put('manifest.json', zlib.gzipSync(Buffer.from(MANIFEST_REAL)), { contentEncoding: 'gzip' });
    const ok = await get('/assets/manifest.json');
    expect(ok.status).toBe(200);
    // the local workerd may hand the client the decoded body when it did not ask for gzip; either way the bytes are the real manifest
    expect((ok.headers['content-encoding'] === 'gzip' ? zlib.gunzipSync(ok.body) : ok.body).toString()).toBe(MANIFEST_REAL);
    await put('manifest.json', Buffer.from('not gzip at all'), { contentEncoding: 'gzip' });
    const bad = await get('/assets/manifest.json');
    expect(bad.status).toBe(500);
    words(bad, ['gzip として展開できません']);
    await put('manifest.json', zlib.brotliCompressSync(Buffer.from(MANIFEST_REAL)), { contentEncoding: 'br' });
    const br = await get('/assets/manifest.json');
    expect(br.status).toBe(500);
    words(br, ['Content-Encoding br の manifest は検査できません']);
  });

  it('the stand-in under an explicit name is served as before, and the answer for the default key follows the object: stand-in, real, removed', async () => {
    await put('standin-stop/manifest.json', MANIFEST_UNKNOWN_KIND);
    const e = await get('/assets/standin-stop/manifest.json');
    expect(e.status).toBe(200);
    expect(text(e)).toBe(MANIFEST_UNKNOWN_KIND);
    expect((await get('/assets/standin-stop/manifest.json?bundle=1')).status).toBe(200);
    await put('manifest.json', MANIFEST_UNKNOWN_KIND);
    expect((await get('/assets/manifest.json')).status).toBe(409);
    await put('manifest.json', MANIFEST_REAL);
    const ok = await get('/assets/manifest.json');
    expect(ok.status).toBe(200);
    expect(text(ok)).toBe(MANIFEST_REAL);
    await put('manifest.json', MANIFEST_UNKNOWN_KIND);
    expect((await get('/assets/manifest.json')).status).toBe(409);
    await clear('manifest.json');
    expect((await get('/assets/manifest.json')).status).toBe(404);
    await clear('standin-stop/manifest.json');
  });

  it('POST to the default key is still 405', async () => {
    expect((await get('/assets/manifest.json', {}, 'POST')).status).toBe(405);
  });
});


// ---- decision 0152 / 0224: POST /__perf on the Worker: checked like the dev server checks it; a beacon that passes is stored as ONE object of the local R2 (PERF_BUCKET), under perf/<UTC date>/<sid>-<seq>.json ----
const post = (base: string, body: string | Buffer, headers: Record<string, string> = {}, method = 'POST', path = '/__perf'): Promise<Raw> => new Promise((resolve, reject) => {
  const u = new URL(base);
  const req = http.request({ host: u.hostname, port: u.port, path, method, agent: false, headers: { 'content-type': 'application/json', ...headers } }, (res) => {
    const ch: Buffer[] = [];
    res.on('data', (c: Buffer) => ch.push(c));
    res.on('end', () => resolve({ status: res.statusCode ?? 0, headers: res.headers, body: Buffer.concat(ch) }));
  });
  req.on('error', reject);
  req.end(body);
});
const utcDays = (): string[] => { const a = new Date().toISOString().slice(0, 10); return [a, new Date(Date.now() + 5000).toISOString().slice(0, 10)]; };
const perfKeys = (): string[] => listLocalKeys(DEV_WORK).map((k) => k.key);
let sidCounter = 0;
const newSid = (): string => (Date.now().toString(16) + (sidCounter++).toString(16).padStart(4, '0') + 'abcdef').slice(0, 12);

describe('Worker (dev mode): POST /__perf is checked, and a beacon that passes is stored as one object (decision 0224)', () => {
  it('a beacon: 204, no body, X-Perf-Stored: yes, isolation headers, no-store; one object perf/<UTC date>/<sid>-<seq>.json, one JSONL line {t, remote: "workers", perf}', async () => {
    const sid = newSid();
    const beacon = { v: 3, seq: 5, sid, reason: 'periodic', startup: { firstIdleSeconds: 7.25 } };
    const before = perfKeys().length;
    const r = await post(DEV, JSON.stringify(beacon));
    expect(r.status).toBe(204);
    expect(r.body.length).toBe(0);
    expect(r.headers['x-perf-stored']).toBe('yes');
    expect(r.headers['cache-control']).toContain('no-store');
    expect(r.headers['cross-origin-embedder-policy']).toBe('require-corp');
    const keys = utcDays().map((d) => `perf/${d}/${sid}-5.json`);
    const listed = listLocalKeys(DEV_WORK).filter((k) => keys.includes(k.key));
    expect(listed).toHaveLength(1);
    expect(perfKeys().length).toBe(before + 1);
    expect(listed[0]?.contentType).toBe('application/json');
    const text = await getPerfObject(DEV_WORK, (listed[0] as { key: string }).key);
    expect(text).not.toBeNull();
    expect((text as string).endsWith('\n')).toBe(true);
    expect((text as string).trim().split('\n')).toHaveLength(1);
    const rec = JSON.parse(text as string);
    expect(Object.keys(rec)).toEqual(['t', 'remote', 'perf']);
    expect(rec.remote).toBe('workers');
    expect(Math.abs(Date.parse(rec.t) - Date.now())).toBeLessThan(60_000);
    expect(rec.perf).toEqual(beacon);
  });
  it('keeps neither the IP address, nor the Access identity, nor a cookie, nor the token: only the time, "workers" and the beacon', async () => {
    const sid = newSid();
    const r = await post(DEV, JSON.stringify({ v: 3, seq: 0, sid }), { 'cf-connecting-ip': '203.0.113.77', 'x-forwarded-for': '198.51.100.23', 'cf-access-authenticated-user-email': 'someone.private@example.invalid', 'cf-access-jwt-assertion': 'aaa.bbb.ccc', cookie: 'CF_Authorization=SECRETCOOKIEVALUE', authorization: 'Bearer SECRETBEARER', 'user-agent': 'UA-THAT-IS-NOT-IN-THE-BEACON' });
    expect(r.headers['x-perf-stored']).toBe('yes');
    const key = listLocalKeys(DEV_WORK).map((k) => k.key).find((k) => k.includes(sid)) as string;
    const text = (await getPerfObject(DEV_WORK, key)) as string;
    for (const secret of ['203.0.113.77', '198.51.100.23', 'someone.private', 'example.invalid', 'aaa.bbb.ccc', 'SECRETCOOKIEVALUE', 'SECRETBEARER', 'UA-THAT-IS-NOT-IN-THE-BEACON']) expect(text).not.toContain(secret);
  });
  it('a sid that is not 12 hex digits does not reach the key: nosid-<random>, under perf/, never a path the page chose', async () => {
    const before = new Set(perfKeys());
    for (const [i, sid] of ['../../assets/manifest.json', 'UPPERCASE123', 'x'.repeat(200)].entries()) expect((await post(DEV, JSON.stringify({ v: 3, seq: i, sid }))).headers['x-perf-stored']).toBe('yes');
    expect((await post(DEV, JSON.stringify({ v: 1, probe: 'no sid at all' }))).headers['x-perf-stored']).toBe('yes');
    const added = perfKeys().filter((k) => !before.has(k));
    expect(added).toHaveLength(4);
    for (const k of added) expect(k).toMatch(/^perf\/\d{4}-\d\d-\d\d\/nosid-[0-9a-f]{8}-\d+\.json$/);
    expect(listLocalKeys(DEV_WORK, '').filter((k) => k.key.includes('..'))).toEqual([]);
  });
  it('the same refusals as the dev server: 415 (not JSON), 413 (over 16 KB), 400 (not one JSON object), 403 (another origin), 405 (GET); each says X-Perf-Stored: no and stores nothing', async () => {
    const before = perfKeys().length;
    const refusals: [Raw, number][] = [
      [await post(DEV, '{"a":1}', { 'content-type': 'text/plain' }), 415],
      [await post(DEV, JSON.stringify({ p: 'x'.repeat(17000) })), 413],
      [await post(DEV, '[1]'), 400],
      [await post(DEV, 'nope'), 400],
      [await post(DEV, '{"a":1}', { origin: 'http://evil.example' }), 403],
      [await post(DEV, '{"a":1}', { 'sec-fetch-site': 'cross-site' }), 403],
      [await post(DEV, '', {}, 'GET'), 405],
    ];
    for (const [r, status] of refusals) { expect(r.status).toBe(status); expect(r.headers['x-perf-stored']).toBe('no'); }
    expect(refusals[6]?.[0].headers.allow).toBe('POST');
    expect(perfKeys().length).toBe(before);
  });
  it('a chunked body that goes over 16 KB is answered 413 while the client is still sending (the Worker stops reading at the limit), and nothing is stored', async () => {
    const before = perfKeys().length;
    const u = new URL(DEV);
    const status = await new Promise<number>((resolve, reject) => {
      const timer = setTimeout(() => { req.destroy(); reject(new Error('no answer within 8 s: the Worker kept reading')); }, 8000);
      const req = http.request({ host: u.hostname, port: u.port, path: '/__perf', method: 'POST', agent: false, headers: { 'content-type': 'application/json', 'transfer-encoding': 'chunked' } }, (res) => {
        clearTimeout(timer);
        res.resume();
        resolve(res.statusCode ?? 0);
        req.destroy();
      });
      req.on('error', () => { /* destroyed after the answer */ });
      req.write('{"p":"' + 'x'.repeat(17000)); // never ended
    });
    expect(status).toBe(413);
    expect(perfKeys().length).toBe(before);
  });
  it('a body of exactly 16 KB is stored (the object is a little larger: the time and "workers" are added); the Worker still answers 403 without a token when Access is on', async () => {
    const sid = newSid();
    const overhead = JSON.stringify({ sid, seq: 9, p: '' }).length;
    const exact = JSON.stringify({ sid, seq: 9, p: 'x'.repeat(16384 - overhead) });
    expect(exact.length).toBe(16384);
    const r = await post(DEV, exact);
    expect(r.status).toBe(204);
    expect(r.headers['x-perf-stored']).toBe('yes');
    const o = listLocalKeys(DEV_WORK).find((k) => k.key.includes(sid));
    expect(o?.size).toBeGreaterThan(16384);
    const denied = await rawGetPath(ACCESS, '/__perf', {}, 'POST');
    expect(denied.status).toBe(403);
  });
  it('behind Access, a beacon with a valid token is stored, and neither the token nor the e-mail address of its claims is', async () => {
    const sid = newSid();
    const r = await post(ACCESS, JSON.stringify({ v: 3, seq: 1, sid }), { 'cf-access-jwt-assertion': tokens.valid ?? '' });
    expect(r.status).toBe(204);
    expect(r.headers['x-perf-stored']).toBe('yes');
    const key = listLocalKeys(DEV_WORK).map((k) => k.key).find((k) => k.includes(sid)) as string;
    const text = (await getPerfObject(DEV_WORK, key)) as string;
    expect(text).not.toContain('owner@example.invalid');
    expect(text).not.toContain((tokens.valid ?? 'x').slice(0, 40));
  });
  it('a Worker WITHOUT the PERF_BUCKET binding still checks and answers 204, and says it did not store it; its summary says why (503)', async () => {
    const r = await post(NOPERF, JSON.stringify({ v: 3, seq: 0, sid: newSid() }));
    expect(r.status).toBe(204);
    expect(r.headers['x-perf-stored']).toBe('no');
    expect((await post(NOPERF, '[1]')).status).toBe(400);
    const s = await rawGetPath(NOPERF, '/__perf/summary');
    expect(s.status).toBe(503);
    expect(s.body.toString()).toContain('PERF_BUCKET');
  });
});

describe('Worker (dev mode): GET /__perf/summary', () => {
  const get = (q = '', headers: Record<string, string> = {}, base = DEV, method = 'GET') => rawGetPath(base, `/__perf/summary${q}`, headers, method);
  it('is text with the isolation headers and no-store; it counts the beacons the Worker stored, as "workers" records', async () => {
    const sid = newSid();
    await post(DEV, JSON.stringify({ v: 3, seq: 0, sid, reason: 'exit', bundle: { name: 'fl', version: '1', engineKind: 'emscripten-fruity-v1', standIn: false }, startup: { firstIdleSeconds: 4.5 } }));
    const r = await get('?rows=all');
    expect(r.status).toBe(200);
    expect(String(r.headers['content-type'])).toContain('text/plain');
    expect(r.headers['cache-control']).toContain('no-store');
    expect(r.headers['cross-origin-embedder-policy']).toBe('require-corp');
    const body = r.body.toString();
    expect(body).toContain('計測の集計');
    expect(body).toMatch(/範囲内のオブジェクト \d+ 件、読んだ \d+ 件/);
    expect(body).toMatch(/P1 first idle \(target <= 10 s\), real engine: .*\b4\.5\b/);
    expect(body).toMatch(/ workers {2}#0 exit /);
    expect(body).toContain('workers (through Cloudflare Workers, behind Access)');
  });
  it('?format=json is the library\'s summary with the window; ?days / ?rows / ?machine are checked (400), HEAD is 200 without a body, POST is 405', async () => {
    const j = await get('?format=json&days=3');
    expect(j.status).toBe(200);
    expect(String(j.headers['content-type'])).toContain('application/json');
    const o = JSON.parse(j.body.toString());
    expect(o.window.dates).toHaveLength(3);
    expect(o.summary.rows.every((x: { remote: string }) => x.remote === 'workers')).toBe(true);
    for (const q of ['?days=0', '?days=181', '?days=x', '?rows=-1', '?machine=nope']) expect((await get(q)).status, q).toBe(400);
    const h = await get('', {}, DEV, 'HEAD');
    expect(h.status).toBe(200);
    expect(h.body.length).toBe(0);
    const p = await post(DEV, '{}', {}, 'POST', '/__perf/summary');
    expect(p.status).toBe(405);
    expect(p.headers.allow).toBe('GET, HEAD');
  });
  it('is behind Access like every route: no token 403, a bad token 403, a valid token 200; the closed Worker is 403', async () => {
    expect((await get('', {}, ACCESS)).status).toBe(403);
    expect((await get('', { 'cf-access-jwt-assertion': tokens.forged ?? '' }, ACCESS)).status).toBe(403);
    expect((await get('', { 'cf-access-jwt-assertion': tokens.valid ?? '' }, ACCESS)).status).toBe(200);
    expect((await get('', {}, CLOSED)).status).toBe(403);
    expect((await post(CLOSED, JSON.stringify({ v: 3 }))).status).toBe(403);
  });
});
