// Cloudflare Worker of the private FL Studio web app.
//   /assets/*   large assets (engine wasm, packs, manifests) from the R2 bucket bound as ASSETS_BUCKET: single byte Range, ETag / If-None-Match, stored Content-Encoding.
//   everything else   the built app (HTML / JS / CSS) from the Static Assets binding ASSETS (run_worker_first, so this code sees every request).
// Every response carries COOP / COEP / CORP and X-Robots-Tag. Cloudflare Access is configured at the edge; checkAccess() is the second layer (see access.ts).
import { checkAccess } from './access.ts';
import { CSP, ISOLATION_HEADERS, PRIVACY_HEADERS, cacheControlFor, mimeFor } from '../shared/headers.ts';
import { ifNoneMatchHit, ifRangeAllows, parseRange } from '../shared/range.ts';
import { PERF_MAX_BODY, PERF_PATH, PERF_STORED_HEADER, PERF_SUMMARY_PATH, checkPerfRequest, parsePerfBody } from '../shared/perf.ts';
import { storeBeacon } from './perf-store.ts';
import { perfSummary } from './perf-summary.ts';
import { MAX_MANIFEST_BYTES, brokenManifestBody, isDefaultManifestKey, judgeManifestBytes, notDeployedBody } from '../shared/manifest-guard.ts';

export interface Env {
  ASSETS: Fetcher;
  ASSETS_BUCKET: R2Bucket;
  /** decision 0224: where the measurement beacons are kept (fl-web-cache, prefix perf/). Optional: without it a beacon is still checked and answered 204, but X-Perf-Stored says "no". */
  PERF_BUCKET?: R2Bucket;
  ACCESS_TEAM_DOMAIN?: string;
  ACCESS_AUD?: string;
  /** tests only: where the Access signing keys come from instead of https://<team>/cdn-cgi/access/certs */
  ACCESS_CERTS_URL?: string;
}

const ASSET_PREFIX = '/assets/';

function baseHeaders(): Headers {
  const h = new Headers();
  for (const [k, v] of Object.entries(ISOLATION_HEADERS)) h.set(k, v);
  for (const [k, v] of Object.entries(PRIVACY_HEADERS)) h.set(k, v);
  return h;
}

function text(status: number, body: string, extra: Record<string, string> = {}): Response {
  const h = baseHeaders();
  h.set('Content-Type', 'text/plain; charset=utf-8');
  h.set('Cache-Control', 'no-store');
  for (const [k, v] of Object.entries(extra)) h.set(k, v);
  return new Response(body, { status, headers: h });
}

function validKey(key: string): boolean {
  return key !== '' && key.length <= 1024 && !key.includes('\\') && !/[\u0000-\u001f]/.test(key) && key.split('/').every((s) => s !== '' && s !== '.' && s !== '..');
}

// The default manifest (decision 0152, shared/manifest-guard.ts): read, decoded as its stored Content-Encoding says, judged. Returns the object that was judged, so that the bytes served afterwards
// are conditional on its ETag (a manifest swapped in between gives 412, not an unchecked body).
async function judgeDefaultManifest(bucket: R2Bucket, key: string, head: R2Object): Promise<{ ok: true; head: R2Object } | { ok: false; response: Response }> {
  const fail = (status: 404 | 409 | 500, body: string) => ({ ok: false as const, response: text(status, body) });
  const broken = (what: string) => fail(500, brokenManifestBody(what));
  if (head.size > MAX_MANIFEST_BYTES) return broken(`${head.size} バイトは大きすぎて検査できません`);
  const obj = await bucket.get(key, { onlyIf: { etagMatches: head.etag } });
  if (obj === null) return fail(404, notDeployedBody());
  if (!('body' in obj) || obj.body === null) return broken('読んでいる間に変わりました');
  let bytes = new Uint8Array(await obj.arrayBuffer());
  const enc = (obj.httpMetadata?.contentEncoding ?? '').trim().toLowerCase();
  if (enc !== '' && enc !== 'identity') {
    if (enc !== 'gzip' && enc !== 'deflate') return broken(`Content-Encoding ${enc} の manifest は検査できません。manifest.json は圧縮せずに置いてください`);
    try { bytes = new Uint8Array(await new Response(new Blob([bytes]).stream().pipeThrough(new DecompressionStream(enc))).arrayBuffer()); } catch (e) { return broken(`${enc} として展開できません: ${String((e as Error)?.message ?? e)}`); }
  }
  const v = judgeManifestBytes(bytes);
  if (!v.ok) return fail(v.status, v.body);
  return { ok: true, head: obj };
}

async function serveAsset(request: Request, env: Env, pathname: string): Promise<Response> {
  if (request.method !== 'GET' && request.method !== 'HEAD') return text(405, 'method not allowed', { Allow: 'GET, HEAD' });
  let key: string;
  try { key = decodeURIComponent(pathname.slice(ASSET_PREFIX.length)); } catch { return text(400, 'bad path'); }
  if (!validKey(key)) return text(400, 'bad path');
  let head = await env.ASSETS_BUCKET.head(key);
  const guarded = isDefaultManifestKey(key);
  if (!head) return text(404, guarded ? notDeployedBody() : `not found: ${pathname}`);
  if (guarded) {
    const j = await judgeDefaultManifest(env.ASSETS_BUCKET, key, head);
    if (!j.ok) return j.response;
    head = j.head;
  }

  const h = baseHeaders();
  const stored = head.httpMetadata?.contentType;
  h.set('Content-Type', stored && stored !== 'application/octet-stream' ? stored : mimeFor(key));
  const enc = head.httpMetadata?.contentEncoding;
  if (enc) h.set('Content-Encoding', enc);
  h.set('ETag', head.httpEtag);
  h.set('Accept-Ranges', 'bytes');
  h.set('Cache-Control', `${head.httpMetadata?.cacheControl ?? cacheControlFor(pathname)}, no-transform`);
  h.set('Vary', 'Accept-Encoding');

  if (ifNoneMatchHit(request.headers.get('If-None-Match'), head.httpEtag)) return new Response(null, { status: 304, headers: h });

  const size = head.size;
  const range = ifRangeAllows(request.headers.get('If-Range'), head.httpEtag) ? parseRange(request.headers.get('Range'), size) : ({ kind: 'none' } as const);
  if (range.kind === 'unsatisfiable') { h.set('Content-Range', `bytes */${size}`); h.set('Content-Length', '0'); return new Response(null, { status: 416, headers: h }); }

  let status = 200, start = 0, end = size - 1;
  if (range.kind === 'partial') { status = 206; start = range.start; end = range.end; h.set('Content-Range', `bytes ${start}-${end}/${size}`); }
  h.set('Content-Length', String(size === 0 ? 0 : end - start + 1));
  // encodeBody: 'manual' - the R2 body is the STORED bytes, already encoded as Content-Encoding says. With the default ('automatic') the runtime would encode it a second time.
  if (request.method === 'HEAD' || size === 0) return new Response(null, { status, headers: h, encodeBody: 'manual' });

  const obj = await env.ASSETS_BUCKET.get(key, {
    onlyIf: { etagMatches: head.etag },
    ...(range.kind === 'partial' ? { range: { offset: start, length: end - start + 1 } } : {}),
  });
  if (obj === null) return text(404, `not found: ${pathname}`);
  if (!('body' in obj) || obj.body === null) return text(412, 'the object changed while it was being read; retry', { 'Retry-After': '1' });
  return new Response(obj.body, { status, headers: h, encodeBody: 'manual' });
}

// POST /__perf (decision 0152): the page's measurement beacon is checked like the dev server checks it. Decision 0224: a beacon that passes is kept (storeBeacon) and X-Perf-Stored says "yes" or "no"; a failed write is logged
// and answered 204 with "no": the page does not depend on the beacon.
async function handlePerf(request: Request, env: Env): Promise<Response> {
  const noStore = { 'Cache-Control': 'no-store', [PERF_STORED_HEADER]: 'no' };
  const c = checkPerfRequest({
    method: request.method, contentType: request.headers.get('Content-Type') ?? undefined, contentLength: request.headers.get('Content-Length') ?? undefined,
    origin: request.headers.get('Origin') ?? undefined, host: new URL(request.url).host, secFetchSite: request.headers.get('Sec-Fetch-Site') ?? undefined,
  });
  if (!c.ok) return text(c.status, c.body, { ...noStore, ...(c.allow ? { Allow: c.allow } : {}) });
  const chunks: Uint8Array[] = [];
  let size = 0;
  const reader = request.body?.getReader();
  while (reader) {
    const { done, value } = await reader.read();
    if (done) break;
    size += value.length;
    if (size > PERF_MAX_BODY) { await reader.cancel(); return text(413, `the beacon is at most ${PERF_MAX_BODY} bytes\n`, noStore); }
    chunks.push(value);
  }
  const all = new Uint8Array(size);
  let at = 0;
  for (const ch of chunks) { all.set(ch, at); at += ch.length; }
  const body = parsePerfBody(all);
  if (!body.ok) return text(body.status, body.body, noStore);
  const stored = await storeBeacon(env.PERF_BUCKET, body.value);
  const h = baseHeaders();
  for (const [k, v] of Object.entries({ ...noStore, [PERF_STORED_HEADER]: stored ? 'yes' : 'no' })) h.set(k, v);
  return new Response(null, { status: 204, headers: h });
}

// GET /__perf/summary (decision 0224): the stored beacons, summarised as text (worker/perf-summary.ts). Behind the same Access check as every route.
async function handlePerfSummary(request: Request, env: Env, url: URL): Promise<Response> {
  if (request.method !== 'GET' && request.method !== 'HEAD') return text(405, 'method not allowed', { Allow: 'GET, HEAD' });
  if (request.method === 'HEAD') return text(200, '');
  const a = await perfSummary(env, url.searchParams);
  return text(a.status, a.body, a.json ? { 'Content-Type': 'application/json; charset=utf-8' } : {});
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const access = await checkAccess(request, env);
    if (!access.ok) return text(access.status, access.reason);
    const url = new URL(request.url);
    if (url.pathname === PERF_PATH) return handlePerf(request, env);
    if (url.pathname === PERF_SUMMARY_PATH) return handlePerfSummary(request, env, url);
    if (url.pathname.startsWith(ASSET_PREFIX)) return serveAsset(request, env, url.pathname);
    if (request.method !== 'GET' && request.method !== 'HEAD') return text(405, 'method not allowed', { Allow: 'GET, HEAD' });
    const res = await env.ASSETS.fetch(request);
    const out = new Response(res.body, res);
    for (const [k, v] of Object.entries(ISOLATION_HEADERS)) out.headers.set(k, v);
    for (const [k, v] of Object.entries(PRIVACY_HEADERS)) out.headers.set(k, v);
    out.headers.set('Content-Security-Policy', CSP);
    return out;
  },
} satisfies ExportedHandler<Env>;
