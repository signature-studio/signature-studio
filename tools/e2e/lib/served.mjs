// servedHeadersMatch: the delivery's responses carry the values of app/shared/headers.ts (ISOLATION_HEADERS, PRIVACY_HEADERS, CSP) and, for /assets/,
// the Cache-Control the Worker's rule gives (app/worker/index.ts: the stored Cache-Control, else cacheControlFor(path), then ", no-transform") and the
// Content-Encoding of the stored object (br for the bundle's brotli files, none for the manifest).
// Two readings are made, and both must match:
//   wire:   the HTTP responses of `wrangler dev --local` (the delivery the browser gets). Its proxy re-encodes a br body as gzip (observed), so Content-Encoding
//           is only reported there (wireContentEncoding), not judged.
//   worker: app/worker/index.ts called directly in Node with the same bucket objects, so that the Content-Encoding the Worker sets is read as it is.
import fs from 'node:fs';
import crypto from 'node:crypto';
import { CSP, ISOLATION_HEADERS, PRIVACY_HEADERS, cacheControlFor } from '../../../app/shared/headers.ts';
import worker from '../../../app/worker/index.ts';

const get = async (url, token, extra = {}) => {
  const r = await fetch(url, { headers: { ...(token ? { 'Cf-Access-Jwt-Assertion': token } : {}), ...extra }, redirect: 'manual', signal: AbortSignal.timeout(20000) });
  await r.body?.cancel();
  return r;
};

function compare(res, want, skip = []) {
  const diffs = [];
  for (const [k, v] of Object.entries(want)) {
    if (skip.includes(k)) continue;
    const got = res.headers.get(k);
    if (got !== v) diffs.push(`${k}: got ${JSON.stringify(got)} want ${JSON.stringify(v)}`);
  }
  return diffs;
}

/** a bucket of the Worker's R2 interface (head, get) over the objects of the bundle, read from disk */
function diskBucket(objects) {
  const byKey = new Map(objects.map((o) => [o.key, o]));
  const meta = (o) => {
    const bytes = fs.readFileSync(o.file);
    const etag = crypto.createHash('md5').update(bytes).digest('hex');
    return { key: o.key, size: bytes.length, etag, httpEtag: `"${etag}"`, httpMetadata: o.httpMetadata ?? {}, bytes };
  };
  return {
    async head(key) { const o = byKey.get(key); if (!o) return null; const { bytes, ...m } = meta(o); return m; },
    async get(key, opts = {}) {
      const o = byKey.get(key); if (!o) return null;
      const m = meta(o);
      let buf = m.bytes;
      if (opts.range) buf = buf.subarray(opts.range.offset, opts.range.offset + opts.range.length);
      const { bytes, ...rest } = m;
      return { ...rest, body: new Response(buf).body, arrayBuffer: async () => buf };
    },
  };
}

/**
 * probes: the document, the manifest (raw, no Content-Encoding) and one brotli object of the bundle.
 * `objects` are the bucket's objects of the run (lib/delivery.mjs bundleObjects); `accessCerts` is the test JWKS URL the Worker reads.
 * Returns { servedHeadersMatch, accessGate, checks: [{reading, path, status, diffs}], wireContentEncoding }
 */
export async function probeServed({ origin, token, bundle, brFile, objects, accessEnv, manifestCacheStored }) {
  const checks = [];
  const brPath = `/assets/${bundle}/${encodeURIComponent(brFile)}`;
  const iso = { ...ISOLATION_HEADERS, ...PRIVACY_HEADERS };
  const wantMani = { ...iso, 'Cache-Control': `${manifestCacheStored ?? cacheControlFor('/assets/manifest.json')}, no-transform` };
  const wantBr = { ...iso, 'Content-Encoding': 'br', 'Cache-Control': 'max-age=31536000, immutable, no-transform' };
  const wantBrWire = { ...iso, 'Cache-Control': 'max-age=31536000, immutable, no-transform' };
  const wantDoc = { ...iso, 'Content-Security-Policy': CSP };
  const add = (reading, path, res, want, skip = []) => {
    checks.push({ reading, path, status: res.status, diffs: res.status === 200 ? compare(res, want, skip) : [`status ${res.status}`] });
  };

  const doc = await get(`${origin}/`, token);
  add('wire', '/', doc, wantDoc);
  const mani = await get(`${origin}/assets/manifest.json`, token);
  add('wire', '/assets/manifest.json', mani, wantMani);
  const brWire = await get(`${origin}${brPath}`, token);
  add('wire', brPath, brWire, wantBrWire, ['Content-Encoding']);
  const wireContentEncoding = brWire.headers.get('content-encoding');

  const env = { ASSETS: { fetch: async () => new Response('<!doctype html>', { headers: { 'Content-Type': 'text/html' } }) }, ASSETS_BUCKET: diskBucket(objects), ...accessEnv };
  const direct = async (path) => worker.fetch(new Request(`${origin}${path}`, { headers: { 'Cf-Access-Jwt-Assertion': token } }), env);
  const dMani = await direct('/assets/manifest.json');
  await dMani.body?.cancel();
  checks.push({ reading: 'worker', path: '/assets/manifest.json', status: dMani.status, diffs: dMani.status === 200 ? compare(dMani, wantMani) : [`status ${dMani.status}`] });
  const dBr = await direct(brPath);
  await dBr.body?.cancel();
  checks.push({ reading: 'worker', path: brPath, status: dBr.status, diffs: dBr.status === 200 ? compare(dBr, wantBr) : [`status ${dBr.status}`] });
  const gate = await get(`${origin}/`, null);
  const accessGate = gate.status === 403;
  const servedHeadersMatch = checks.length === 5 && checks.every((c) => c.diffs.length === 0);
  return { servedHeadersMatch, accessGate, wireContentEncoding, checks };
}
