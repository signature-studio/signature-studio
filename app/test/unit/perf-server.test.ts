// Decision 0152: POST /__perf on the server side: what it accepts, what it refuses (413 / 415 / 403 / 405 / 400), what it writes (one JSONL line with the time and the CLASS of the peer, never its address).
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { PERF_MAX_BODY, PERF_PATH, checkPerfRequest, classifyPeer, classifyRemote, parsePerfBody, perfLine } from '../../shared/perf.ts';
import { perfLogPath, perfMiddleware } from '../../tools/perf-middleware.ts';

describe('classifyRemote: only the class of the peer is kept', () => {
  it.each([
    ['127.0.0.1', 'loopback'], ['127.1.2.3', 'loopback'], ['::1', 'loopback'], ['::ffff:127.0.0.1', 'loopback'],
    ['100.64.0.1', 'tailnet'], ['100.100.50.7', 'tailnet'], ['100.127.255.255', 'tailnet'], ['fd7a:115c:a1e0::1', 'tailnet'], ['fd7a:115c:a1e0:ab12:4843:cd96:6200:1234', 'tailnet'], ['::ffff:100.101.102.103', 'tailnet'],
    ['10.0.0.1', 'lan'], ['10.255.255.254', 'lan'], ['172.16.0.1', 'lan'], ['172.31.255.255', 'lan'], ['192.168.1.20', 'lan'], ['169.254.1.1', 'lan'], ['fd00::1', 'lan'], ['fc12::5', 'lan'], ['fe80::1', 'lan'], ['febf::1', 'lan'], ['::ffff:192.168.0.5', 'lan'],
    ['100.63.255.255', 'other'], ['100.128.0.1', 'other'], ['172.15.0.1', 'other'], ['172.32.0.1', 'other'], ['192.169.0.1', 'other'], ['8.8.8.8', 'other'], ['1.2.3.4', 'other'], ['2001:4860:4860::8888', 'other'], ['fec0::1', 'other'],
    ['', 'other'], ['not an address', 'other'], ['999.1.1.1', 'other'], ['127.0.0', 'other'], ['::ffff:8.8.8.8', 'other'],
  ] as const)('%s is %s', (addr, cls) => { expect(classifyRemote(addr)).toBe(cls); });
  it('undefined / null are "other"', () => { expect(classifyRemote(undefined)).toBe('other'); expect(classifyRemote(null)).toBe('other'); });
});

describe('classifyPeer (decision 0192): behind `tailscale serve` the socket peer is 127.0.0.1 and X-Forwarded-For carries the visitor', () => {
  it.each([
    ['127.0.0.1', '100.101.102.103', 'tailnet'], ['::1', '100.64.0.9', 'tailnet'], ['::ffff:127.0.0.1', '100.100.1.1', 'tailnet'], ['127.0.0.1', 'fd7a:115c:a1e0::1234', 'tailnet'],
    ['127.0.0.1', '192.168.1.5', 'lan'], ['127.0.0.1', '8.8.8.8', 'other'], ['127.0.0.1', '127.0.0.1', 'loopback'],
    ['127.0.0.1', '100.101.102.103, 10.0.0.1', 'tailnet'], ['127.0.0.1', ' 100.101.102.103 ', 'tailnet'], ['127.0.0.1', '::ffff:100.101.102.103', 'tailnet'],
  ] as const)('socket %s + X-Forwarded-For %s is %s', (sock, xff, cls) => { expect(classifyPeer(sock, xff)).toBe(cls); });
  it('without the header, or with one that is not an address, a loopback peer stays loopback', () => {
    expect(classifyPeer('127.0.0.1', undefined)).toBe('loopback');
    expect(classifyPeer('127.0.0.1', '')).toBe('loopback');
    expect(classifyPeer('127.0.0.1', 'unknown')).toBe('loopback');
    expect(classifyPeer('127.0.0.1', 'garbage, 100.101.102.103')).toBe('loopback');
  });
  it('the header counts only when the SOCKET peer is loopback: a peer that is not this machine cannot choose its class', () => {
    expect(classifyPeer('8.8.8.8', '100.101.102.103')).toBe('other');
    expect(classifyPeer('192.168.1.20', '100.101.102.103')).toBe('lan');
    expect(classifyPeer('100.101.102.103', '127.0.0.1')).toBe('tailnet');
    expect(classifyPeer(undefined, '100.101.102.103')).toBe('other');
  });
});

describe('checkPerfRequest', () => {
  const ok = { method: 'POST', contentType: 'application/json', contentLength: '100', host: 'a.example:5173' };
  it('a same-origin JSON POST passes (with or without Origin / Sec-Fetch-Site, charset allowed)', () => {
    expect(checkPerfRequest(ok)).toEqual({ ok: true });
    expect(checkPerfRequest({ ...ok, origin: 'http://a.example:5173', secFetchSite: 'same-origin' })).toEqual({ ok: true });
    expect(checkPerfRequest({ ...ok, contentType: 'Application/JSON; charset=utf-8' })).toEqual({ ok: true });
    expect(checkPerfRequest({ ...ok, contentLength: undefined })).toEqual({ ok: true });
    expect(checkPerfRequest({ ...ok, secFetchSite: 'none' })).toEqual({ ok: true });
  });
  it.each(['GET', 'HEAD', 'PUT', 'DELETE', 'OPTIONS', 'PATCH'])('%s is 405 and says POST', (m) => {
    expect(checkPerfRequest({ ...ok, method: m })).toMatchObject({ ok: false, status: 405, allow: 'POST' });
  });
  it.each(['text/plain', 'text/plain;charset=utf-8', 'application/x-www-form-urlencoded', 'multipart/form-data; boundary=x', 'application/jsonx', 'application/octet-stream', ''])('Content-Type %j is 415', (ct) => {
    expect(checkPerfRequest({ ...ok, contentType: ct })).toMatchObject({ ok: false, status: 415 });
  });
  it('no Content-Type is 415', () => { expect(checkPerfRequest({ ...ok, contentType: undefined })).toMatchObject({ ok: false, status: 415 }); });
  it('a Content-Length over 16 KB is 413; exactly 16 KB passes; a bad one is 413', () => {
    expect(PERF_MAX_BODY).toBe(16384);
    expect(checkPerfRequest({ ...ok, contentLength: String(PERF_MAX_BODY + 1) })).toMatchObject({ ok: false, status: 413 });
    expect(checkPerfRequest({ ...ok, contentLength: String(PERF_MAX_BODY) })).toEqual({ ok: true });
    expect(checkPerfRequest({ ...ok, contentLength: 'abc' })).toMatchObject({ ok: false, status: 413 });
    expect(checkPerfRequest({ ...ok, contentLength: '-5' })).toMatchObject({ ok: false, status: 413 });
  });
  it('another origin is 403: a different host, "null", a malformed Origin; Sec-Fetch-Site cross-site / same-site / unknown', () => {
    for (const origin of ['http://evil.example', 'http://a.example:5174', 'http://a.example', 'null', 'garbage', '']) {
      expect(checkPerfRequest({ ...ok, origin }), origin).toMatchObject({ ok: false, status: 403 });
    }
    for (const s of ['cross-site', 'same-site', 'unknown', '']) expect(checkPerfRequest({ ...ok, secFetchSite: s }), s).toMatchObject({ ok: false, status: 403 });
  });
  it('Sec-Fetch-Site: same-origin (set by the browser, not by scripts) decides: a proxy that rewrote Host does not make the page\'s own beacon foreign; without it the Origin must name this host', () => {
    expect(checkPerfRequest({ ...ok, origin: 'https://machine.example.ts.net', host: '127.0.0.1:5173', secFetchSite: 'same-origin' })).toEqual({ ok: true });
    expect(checkPerfRequest({ ...ok, origin: 'https://machine.example.ts.net', host: '127.0.0.1:5173' })).toMatchObject({ ok: false, status: 403 });
    expect(checkPerfRequest({ ...ok, origin: 'https://evil.example', host: 'a.example:5173', secFetchSite: 'cross-site' })).toMatchObject({ ok: false, status: 403 });
  });
  it('the scheme of Origin is not compared (behind a TLS proxy such as `tailscale serve` the page is https and this server sees http): same host and port passes', () => {
    expect(checkPerfRequest({ ...ok, origin: 'https://a.example:5173' })).toEqual({ ok: true });
  });
  it('the order of the checks: method first, then origin, then type, then size', () => {
    expect(checkPerfRequest({ method: 'GET', origin: 'http://evil', host: 'a', contentType: 'x', contentLength: '99999999' }).ok).toBe(false);
    expect(checkPerfRequest({ method: 'POST', origin: 'http://evil', host: 'a', contentType: 'x', contentLength: '99999999' })).toMatchObject({ status: 403 });
    expect(checkPerfRequest({ method: 'POST', host: 'a', contentType: 'x', contentLength: '99999999' })).toMatchObject({ status: 415 });
  });
});

describe('parsePerfBody', () => {
  const enc = (s: string): Uint8Array => new TextEncoder().encode(s);
  it('one JSON object passes', () => { expect(parsePerfBody(enc('{"v":1,"a":[1,2]}'))).toEqual({ ok: true, value: { v: 1, a: [1, 2] } }); });
  it('a UTF-8 BOM is dropped; Japanese text passes', () => {
    expect(parsePerfBody(new Uint8Array([0xef, 0xbb, 0xbf, ...enc('{"a":"日本語"}')]))).toEqual({ ok: true, value: { a: '日本語' } });
  });
  it.each(['[]', '[{"a":1}]', '1', '"x"', 'null', 'true'])('%s is 400 (JSON object only)', (b) => { expect(parsePerfBody(enc(b))).toMatchObject({ ok: false, status: 400 }); });
  it.each(['', '{', '{"a":}', 'not json', "{'a':1}", '{"a":1}{"b":2}'])('%j is 400 (not JSON)', (b) => { expect(parsePerfBody(enc(b))).toMatchObject({ ok: false, status: 400 }); });
  it('invalid UTF-8 is 400', () => { expect(parsePerfBody(new Uint8Array([0x7b, 0xff, 0x7d]))).toMatchObject({ ok: false, status: 400 }); });
  it('over 16 KB is 413 even if it is valid JSON; exactly 16 KB of JSON passes', () => {
    const pad = (n: number): Uint8Array => enc(JSON.stringify({ p: 'x'.repeat(n - 8) }));
    expect(pad(PERF_MAX_BODY).length).toBe(PERF_MAX_BODY);
    expect(parsePerfBody(pad(PERF_MAX_BODY)).ok).toBe(true);
    expect(parsePerfBody(pad(PERF_MAX_BODY + 1))).toMatchObject({ ok: false, status: 413 });
  });
});

describe('perfLine', () => {
  it('is one line: time, class of the peer, the beacon; a newline inside a string stays escaped', () => {
    const line = perfLine({ v: 1, note: 'a\nb', nested: { x: [1, 2] } }, new Date('2026-11-03T01:02:03.456Z'), 'lan');
    expect(line.endsWith('\n')).toBe(true);
    expect(line.slice(0, -1).includes('\n')).toBe(false);
    expect(JSON.parse(line)).toEqual({ t: '2026-11-03T01:02:03.456Z', remote: 'lan', perf: { v: 1, note: 'a\nb', nested: { x: [1, 2] } } });
  });
});

describe('perfLogPath', () => {
  it('FL_PERF_LOG wins; else perf-log.jsonl in the PARENT of FL_ASSETS_DIR; else not configured', () => {
    expect(perfLogPath({ FL_PERF_LOG: '/x/y/log.jsonl', FL_ASSETS_DIR: '/a/b/assets' })).toBe('/x/y/log.jsonl');
    expect(perfLogPath({ FL_ASSETS_DIR: '/a/b/assets' })).toBe('/a/b/perf-log.jsonl');
    expect(perfLogPath({ FL_ASSETS_DIR: '/a/b/assets/' })).toBe('/a/b/perf-log.jsonl');
    expect(perfLogPath({})).toBe('');
    expect(perfLogPath({ FL_PERF_LOG: '', FL_ASSETS_DIR: '' })).toBe('');
  });
});

describe('the middleware', () => {
  let dir: string, log: string;
  let server: http.Server, base = '';
  let noLog: http.Server, noLogBase = '';
  const T = new Date('2026-11-03T05:06:07.000Z');
  const listen = (s: http.Server): Promise<string> => new Promise((res) => s.listen(0, '127.0.0.1', () => res(`http://127.0.0.1:${(s.address() as { port: number }).port}`)));
  interface R { status: number; headers: http.IncomingHttpHeaders; body: string }
  const call = (b: string, o: { method?: string; path?: string; headers?: Record<string, string>; body?: string | Buffer } = {}): Promise<R> => new Promise((resolve, reject) => {
    const u = new URL(b);
    const req = http.request({ host: u.hostname, port: u.port, path: o.path ?? PERF_PATH, method: o.method ?? 'POST', agent: false, headers: { 'content-type': 'application/json', ...o.headers } }, (res) => {
      const ch: Buffer[] = [];
      res.on('data', (c: Buffer) => ch.push(c));
      res.on('end', () => resolve({ status: res.statusCode ?? 0, headers: res.headers, body: Buffer.concat(ch).toString() }));
    });
    req.on('error', reject);
    req.end(o.body);
  });
  const lines = (): string[] => (fs.existsSync(log) ? fs.readFileSync(log, 'utf8').split('\n').filter(Boolean) : []);
  beforeAll(async () => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'web1-perf-'));
    log = path.join(dir, 'sub', 'perf-log.jsonl');
    const mw = perfMiddleware(log, () => T);
    server = http.createServer((req, res) => mw(req, res, () => { res.writeHead(404); res.end('next'); }));
    base = await listen(server);
    const mw2 = perfMiddleware('', () => T);
    noLog = http.createServer((req, res) => mw2(req, res, () => { res.writeHead(404); res.end('next'); }));
    noLogBase = await listen(noLog);
  });
  afterAll(() => { server.close(); noLog.close(); fs.rmSync(dir, { recursive: true, force: true }); });
  beforeEach(() => { fs.rmSync(path.join(dir, 'sub'), { recursive: true, force: true }); });

  it('a beacon is 204, X-Perf-Stored: yes, no body; the directory is made; one line: time, remote "loopback", the beacon', async () => {
    const r = await call(base, { body: JSON.stringify({ v: 1, seq: 0, reason: 'first-idle' }) });
    expect(r.status).toBe(204);
    expect(r.headers['x-perf-stored']).toBe('yes');
    expect(r.headers['cache-control']).toBe('no-store');
    expect(r.headers['cross-origin-embedder-policy']).toBe('require-corp');
    expect(r.body).toBe('');
    const l = lines();
    expect(l).toHaveLength(1);
    expect(JSON.parse(l[0] as string)).toEqual({ t: '2026-11-03T05:06:07.000Z', remote: 'loopback', perf: { v: 1, seq: 0, reason: 'first-idle' } });
  });
  it('behind a reverse proxy on this machine (tailscale serve): the visitor\'s class from X-Forwarded-For is recorded, its address is not', async () => {
    const r = await call(base, { headers: { 'x-forwarded-for': '100.101.102.103', 'x-forwarded-proto': 'https', 'x-forwarded-host': 'machine.example.ts.net' }, body: JSON.stringify({ v: 2, seq: 0 }) });
    expect(r.status).toBe(204);
    const text = fs.readFileSync(log, 'utf8');
    expect(JSON.parse(text).remote).toBe('tailnet');
    expect(text).not.toContain('100.101.102.103');
    expect(text).not.toContain('ts.net');
    await call(base, { headers: { 'x-forwarded-for': 'rubbish' }, body: '{"v":2}' });
    expect(lines().map((l) => JSON.parse(l).remote)).toEqual(['tailnet', 'loopback']);
  });
  it('the peer\'s address is nowhere in the file', async () => {
    await call(base, { body: '{"v":1}' });
    const text = fs.readFileSync(log, 'utf8');
    expect(text).not.toContain('127.0.0.1');
    expect(text).not.toMatch(/\d+\.\d+\.\d+\.\d+/);
    expect(Object.keys(JSON.parse(text))).toEqual(['t', 'remote', 'perf']);
  });
  it('records are appended, one per line, in order', async () => {
    for (let i = 0; i < 5; i++) expect((await call(base, { body: JSON.stringify({ seq: i }) })).status).toBe(204);
    expect(lines().map((x) => (JSON.parse(x) as { perf: { seq: number } }).perf.seq)).toEqual([0, 1, 2, 3, 4]);
  });
  it('a body of exactly 16 KB is stored; one byte more is 413 and nothing is stored', async () => {
    const pad = (n: number): string => JSON.stringify({ p: 'x'.repeat(n - 8) });
    expect(pad(PERF_MAX_BODY).length).toBe(PERF_MAX_BODY);
    expect((await call(base, { body: pad(PERF_MAX_BODY) })).status).toBe(204);
    expect(lines()).toHaveLength(1);
    const big = await call(base, { body: pad(PERF_MAX_BODY + 1) });
    expect(big.status).toBe(413);
    expect(big.headers['x-perf-stored']).toBe('no');
    expect(lines()).toHaveLength(1);
  });
  it('a body far over the limit is 413 (answered without reading it all)', async () => {
    const r = await call(base, { body: Buffer.alloc(2 * 1024 * 1024, 0x20) }).catch(() => ({ status: 413 }));
    expect(r.status).toBe(413);
    expect(lines()).toHaveLength(0);
  });
  it('not JSON: 415 for the wrong Content-Type, 400 for a body that is not one JSON object; nothing stored', async () => {
    expect((await call(base, { headers: { 'content-type': 'text/plain' }, body: '{"a":1}' })).status).toBe(415);
    expect((await call(base, { headers: { 'content-type': 'application/x-www-form-urlencoded' }, body: 'a=1' })).status).toBe(415);
    for (const body of ['not json', '[1]', '"x"', '']) expect((await call(base, { body })).status, body).toBe(400);
    expect(lines()).toHaveLength(0);
  });
  it('GET / PUT / DELETE are 405 with Allow: POST; nothing stored', async () => {
    for (const method of ['GET', 'PUT', 'DELETE']) {
      const r = await call(base, { method, body: method === 'GET' ? undefined : '{"a":1}' });
      expect(r.status, method).toBe(405);
      expect(r.headers.allow).toBe('POST');
    }
    expect(lines()).toHaveLength(0);
  });
  it('another origin (Origin header or Sec-Fetch-Site) is 403; the same origin passes', async () => {
    const host = new URL(base).host;
    expect((await call(base, { headers: { origin: 'http://evil.example' }, body: '{"a":1}' })).status).toBe(403);
    expect((await call(base, { headers: { 'sec-fetch-site': 'cross-site' }, body: '{"a":1}' })).status).toBe(403);
    expect((await call(base, { headers: { origin: `http://${host}`, 'sec-fetch-site': 'same-origin' }, body: '{"a":1}' })).status).toBe(204);
    expect(lines()).toHaveLength(1);
  });
  it('with a query string it is still the endpoint; other paths go to the next handler', async () => {
    expect((await call(base, { path: '/__perf?x=1', body: '{"a":1}' })).status).toBe(204);
    expect((await call(base, { path: '/__perf/', body: '{"a":1}' })).body).toBe('next');
    expect((await call(base, { path: '/other', method: 'GET' })).body).toBe('next');
  });
  it('a log that is a directory cannot be written: 500 that says so', async () => {
    fs.mkdirSync(log, { recursive: true });
    const r = await call(base, { body: '{"a":1}' });
    expect(r.status).toBe(500);
    expect(r.body).toContain('could not be stored');
    expect(r.headers['x-perf-stored']).toBe('no');
  });
  it('without a log path: 503 and nothing stored (it does not pretend)', async () => {
    const r = await call(noLogBase, { body: '{"a":1}' });
    expect(r.status).toBe(503);
    expect(r.headers['x-perf-stored']).toBe('no');
    expect(r.body).toContain('FL_PERF_LOG');
  });
});
