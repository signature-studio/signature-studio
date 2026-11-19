// The HTTP contract of /assets/*, written once and run against both implementations: tools/assets-middleware.ts (dev / preview server, files) and worker/index.ts (R2, via wrangler dev --local).
import http from 'node:http';
import { createHash } from 'node:crypto';
import zlib from 'node:zlib';
import { describe, expect, it } from 'vitest';

export interface Raw { status: number; headers: http.IncomingHttpHeaders; body: Buffer }

// node:http, not fetch: fetch would decode Content-Encoding and hide the stored bytes
export function rawGet(url: string, headers: Record<string, string> = {}, method = 'GET'): Promise<Raw> {
  return new Promise((resolve, reject) => {
    const u = new URL(url);
    const req = http.request({ host: u.hostname, port: u.port, path: u.pathname + u.search, method, headers: { 'accept-encoding': 'identity', ...headers } }, (res) => {
      const chunks: Buffer[] = [];
      res.on('data', (c: Buffer) => chunks.push(c));
      res.on('end', () => resolve({ status: res.statusCode ?? 0, headers: res.headers, body: Buffer.concat(chunks) }));
    });
    req.on('error', reject);
    req.end();
  });
}

// the request line is sent exactly as given: `new URL` would resolve `.` / `..` / `%2e` segments before the server ever saw them (decision 0152 tests the server's own normalization)
export function rawGetPath(base: string, rawPath: string, headers: Record<string, string> = {}, method = 'GET'): Promise<Raw> {
  return new Promise((resolve, reject) => {
    const u = new URL(base);
    const req = http.request({ host: u.hostname, port: u.port, path: rawPath, method, headers: { 'accept-encoding': 'identity', ...headers } }, (res) => {
      const chunks: Buffer[] = [];
      res.on('data', (c: Buffer) => chunks.push(c));
      res.on('end', () => resolve({ status: res.statusCode ?? 0, headers: res.headers, body: Buffer.concat(chunks) }));
    });
    req.on('error', reject);
    req.end();
  });
}

import { FIXTURE, PRE_BR, PRE_GZ } from './fixtures.ts';

const etagOf = (r: Raw): string => String(r.headers.etag);

// Fixture keys: blob.bin (identity), engine.wasm (identity, application/wasm), v1-0123456789ab/pinned.bin (versioned name -> immutable),
// pre.wasm (stored brotli-encoded; Content-Encoding br; for the file server it is the sibling pre.wasm.br), manifest.json (mutable).
export function assetSuite(label: string, base: () => string, opts: { brVariantIsSibling: boolean }): void {
  const url = (k: string): string => `${base()}/assets/${k}`;
  describe(`${label}: /assets/*`, () => {
    it('GET returns the whole object with length, type, ETag, Accept-Ranges', async () => {
      const r = await rawGet(url('blob.bin'));
      expect(r.status).toBe(200);
      expect(r.body.equals(FIXTURE.blob)).toBe(true);
      expect(r.headers['content-length']).toBe(String(FIXTURE.blob.length));
      expect(r.headers['accept-ranges']).toBe('bytes');
      expect(etagOf(r)).toMatch(/^"[^"]+"$/);
    });

    it('wasm is served as application/wasm', async () => {
      const r = await rawGet(url('engine.wasm'));
      expect(r.headers['content-type']).toBe('application/wasm');
      expect(r.body.equals(FIXTURE.wasm)).toBe(true);
    });

    it('Range: first, middle and last bytes come back as 206 with the right Content-Range and the right bytes', async () => {
      const n = FIXTURE.blob.length;
      for (const [hdr, s, e] of [['bytes=0-0', 0, 0], ['bytes=0-99', 0, 99], ['bytes=1000-1999', 1000, 1999], [`bytes=${n - 1}-${n - 1}`, n - 1, n - 1], [`bytes=${n - 10}-`, n - 10, n - 1], ['bytes=-10', n - 10, n - 1], [`bytes=99990-999999`, 99990, n - 1]] as [string, number, number][]) {
        const r = await rawGet(url('blob.bin'), { range: hdr });
        expect(r.status, hdr).toBe(206);
        expect(r.headers['content-range'], hdr).toBe(`bytes ${s}-${e}/${n}`);
        expect(r.headers['content-length'], hdr).toBe(String(e - s + 1));
        expect(r.body.equals(FIXTURE.blob.subarray(s, e + 1)), hdr).toBe(true);
      }
    });

    it('Range past the end is 416 with Content-Range: bytes */size and no body', async () => {
      const n = FIXTURE.blob.length;
      for (const hdr of [`bytes=${n}-`, `bytes=${n + 5}-${n + 10}`, 'bytes=-0']) {
        const r = await rawGet(url('blob.bin'), { range: hdr });
        expect(r.status, hdr).toBe(416);
        expect(r.headers['content-range'], hdr).toBe(`bytes */${n}`);
        expect(r.body.length, hdr).toBe(0);
      }
    });

    it('a Range this server does not serve (several ranges, other unit, garbage) is ignored: 200 with the whole object', async () => {
      for (const hdr of ['bytes=0-1,5-6', 'items=0-5', 'bytes=abc']) {
        const r = await rawGet(url('blob.bin'), { range: hdr });
        expect(r.status, hdr).toBe(200);
        expect(r.body.length, hdr).toBe(FIXTURE.blob.length);
      }
    });

    it('If-None-Match with the current ETag is 304 with no body; another ETag is 200', async () => {
      const first = await rawGet(url('blob.bin'));
      const tag = etagOf(first);
      const r304 = await rawGet(url('blob.bin'), { 'if-none-match': tag });
      expect(r304.status).toBe(304);
      expect(r304.body.length).toBe(0);
      expect(etagOf(r304)).toBe(tag);
      expect((await rawGet(url('blob.bin'), { 'if-none-match': '"something-else"' })).status).toBe(200);
      expect((await rawGet(url('blob.bin'), { 'if-none-match': `W/${tag}` })).status).toBe(304);
    });

    it('If-Range: a matching ETag keeps the range, a different one gives the whole object', async () => {
      const tag = etagOf(await rawGet(url('blob.bin')));
      expect((await rawGet(url('blob.bin'), { range: 'bytes=0-9', 'if-range': tag })).status).toBe(206);
      const r = await rawGet(url('blob.bin'), { range: 'bytes=0-9', 'if-range': '"stale"' });
      expect(r.status).toBe(200);
      expect(r.body.length).toBe(FIXTURE.blob.length);
    });

    it('HEAD has the headers of GET and no body', async () => {
      const g = await rawGet(url('blob.bin'));
      const h = await rawGet(url('blob.bin'), {}, 'HEAD');
      expect(h.status).toBe(200);
      expect(h.body.length).toBe(0);
      expect(h.headers['content-length']).toBe(g.headers['content-length']);
      expect(etagOf(h)).toBe(etagOf(g));
    });

    it('a pre-compressed object is sent with Content-Encoding: br and its stored bytes; the browser-side decode gives the original', async () => {
      const r = await rawGet(url('pre.wasm'), { 'accept-encoding': 'br' });
      expect(r.status).toBe(200);
      expect(r.headers['content-encoding']).toBe('br');
      expect(r.headers['content-type']).toBe('application/wasm');
      expect(r.body.equals(PRE_BR)).toBe(true);
      expect(zlib.brotliDecompressSync(r.body).equals(FIXTURE.pre)).toBe(true);
      expect(r.headers['content-length']).toBe(String(PRE_BR.length));
    });

    it('Range applies to the ENCODED bytes of a pre-compressed object', async () => {
      const r = await rawGet(url('pre.wasm'), { 'accept-encoding': 'br', range: 'bytes=2-9' });
      expect(r.status).toBe(206);
      expect(r.headers['content-encoding']).toBe('br');
      expect(r.headers['content-range']).toBe(`bytes 2-9/${PRE_BR.length}`);
      expect(r.body.equals(PRE_BR.subarray(2, 10))).toBe(true);
    });

    if (opts.brVariantIsSibling) {
      it('(file server) a client that accepts gzip but not br gets the .gz sibling; one that accepts neither gets the plain file', async () => {
        const gz = await rawGet(url('pre.wasm'), { 'accept-encoding': 'gzip' });
        expect(gz.headers['content-encoding']).toBe('gzip');
        expect(gz.body.equals(PRE_GZ)).toBe(true);
        const id = await rawGet(url('pre.wasm'), { 'accept-encoding': 'identity' });
        expect(id.headers['content-encoding']).toBeUndefined();
        expect(id.body.equals(FIXTURE.pre)).toBe(true);
      });
      it('(file server) an explicit request for NAME.gz gets the stored bytes without Content-Encoding', async () => {
        const r = await rawGet(url('pre.wasm.gz'), { 'accept-encoding': 'gzip, br' });
        expect(r.headers['content-encoding']).toBeUndefined();
        expect(r.body.equals(PRE_GZ)).toBe(true);
      });
    }

    it('a versioned name is immutable, a manifest is not', async () => {
      expect(String((await rawGet(url('v1-0123456789ab/pinned.bin'))).headers['cache-control'])).toContain('immutable');
      const m = String((await rawGet(url('manifest.json'))).headers['cache-control']);
      expect(m).toContain('no-cache');
      expect(m).not.toContain('immutable');
    });

    it('every response carries COOP / COEP / CORP / X-Robots-Tag / nosniff - 200, 206, 304, 404 and 416 alike', async () => {
      const tag = etagOf(await rawGet(url('blob.bin')));
      const rs = [
        await rawGet(url('blob.bin')),
        await rawGet(url('blob.bin'), { range: 'bytes=0-9' }),
        await rawGet(url('blob.bin'), { 'if-none-match': tag }),
        await rawGet(url('nope.bin')),
        await rawGet(url('blob.bin'), { range: 'bytes=999999-' }),
      ];
      expect(rs.map((r) => r.status)).toEqual([200, 206, 304, 404, 416]);
      for (const r of rs) {
        expect(r.headers['cross-origin-opener-policy'], String(r.status)).toBe('same-origin');
        expect(r.headers['cross-origin-embedder-policy'], String(r.status)).toBe('require-corp');
        expect(r.headers['cross-origin-resource-policy'], String(r.status)).toBe('same-origin');
        expect(String(r.headers['x-robots-tag']), String(r.status)).toContain('noindex');
        expect(r.headers['x-content-type-options'], String(r.status)).toBe('nosniff');
      }
    });

    it('404 for a missing object, 400 for a path that climbs out, 405 for a write', async () => {
      expect((await rawGet(url('does/not/exist.wasm'))).status).toBe(404);
      expect((await rawGet(`${base()}/assets/%2e%2e/%2e%2e/etc/passwd`)).status).toBeGreaterThanOrEqual(400);
      expect((await rawGet(`${base()}/assets/..%2fsecret`)).status).toBeGreaterThanOrEqual(400);
      expect((await rawGet(url('blob.bin'), {}, 'POST')).status).toBe(405);
      expect((await rawGet(url('blob.bin'), {}, 'DELETE')).status).toBe(405);
    });

    it('the whole object read in pieces by Range adds up to the object (what a resumed download does)', async () => {
      const n = FIXTURE.blob.length;
      const parts: Buffer[] = [];
      for (let s = 0; s < n; s += 33_333) parts.push((await rawGet(url('blob.bin'), { range: `bytes=${s}-${Math.min(n, s + 33_333) - 1}` })).body);
      expect(createHash('sha256').update(Buffer.concat(parts)).digest('hex')).toBe(createHash('sha256').update(FIXTURE.blob).digest('hex'));
    });
  });
}
