// The helpers of the delivery scenarios (Issue #1498). What each one looks at:
//   the bundle's references (the manifest's boot assets and lazy files, each with its URL, size and SHA-256);
//   the Worker in this process (app/worker/index.ts, the same code wrangler dev runs) over the run's bucket, for the stored bytes, the headers and the statuses;
//   the wire (the Worker's own address, lib/delivery.mjs liveDelivery) for the Access checks, which must not pass through the test's Access proxy;
//   what the browser sees (a fetch in the page, through the proxy, with the token the page carries);
//   a page of its own (a page of the run's browser, not attached to the run's detectors: the failures it causes are its own, and its results say them);
//   the beacon's form (app/shared/perf.ts beaconProblems) and the names of its fields.
// The access settings and the addresses are made for this run (lib/access.mjs); nothing here names a product value.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import zlib from 'node:zlib';
import { fileURLToPath } from 'node:url';
import { bundleObjects, liveDelivery } from './delivery.mjs';
import { CSP, ISOLATION_HEADERS, PRIVACY_HEADERS, cacheControlFor } from '../../../app/shared/headers.ts';
import worker from '../../../app/worker/index.ts';
import { validateManifest } from '../../../app/src/bundle/manifest.ts';
import { beaconProblems, PERF_PATH, PERF_MAX_BODY } from '../../../app/shared/perf.ts';

const here = path.dirname(fileURLToPath(import.meta.url));
export const REPO = path.resolve(here, '..', '..', '..');
export const BUNDLE = 'fl-Oz';
const BUNDLE_DIR = path.join(REPO, 'lift', 'out-app', 'bundle', 'raw', BUNDLE);
const BR_DIR = path.join(REPO, 'lift', 'out-app', 'bundle', 'br', BUNDLE);
export const FEATURES_GENERATED = path.resolve(here, '..', 'features', 'generated', 'delivery.tsv');

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
export const sha256Hex = (buf) => crypto.createHash('sha256').update(buf).digest('hex');

// the names of the beacon's top-level fields (app/shared/perf.ts BeaconPayload, beacon.ts): a field outside this list is a field nobody has agreed to send
export const BEACON_KEYS = ['v', 'seq', 'sid', 'reason', 'pageMs', 'visibility', 'browser', 'bundle', 'engine', 'error', 'timeline', 'startup', 'ui', 'input', 'audio', 'memory', 'client', 'display', 'hz', 'rafIntervalMsP95', 'guestScreen', 'files', 'warns', 'assets', 'desk', 'engineRun', 'uiWindows'];
// names that would mean a person or a connection (none of them is a beacon field)
const PERSONAL = ['email', 'ip', 'ipaddress', 'cookie', 'token', 'password', 'username', 'user', 'address', 'location', 'query'];

let cache = null;
/** the bundle of the run: the manifest, the bucket's objects, and the references (boot: the four start-up assets; lazy: the manifest's lazy files) */
export function bundle() {
  if (cache) return cache;
  const manifest = JSON.parse(fs.readFileSync(path.join(BUNDLE_DIR, 'manifest.json'), 'utf8'));
  const boot = [
    { kind: 'wasm', name: 'engine wasm', ref: manifest.engine.wasm },
    { kind: 'seed', name: 'seed pack', ref: manifest.seedPack },
    { kind: 'dll', name: 'DLL pack', ref: manifest.dllPack },
    { kind: 'glue', name: 'engine glue', ref: manifest.engine.options.glue },
  ];
  const lazy = (manifest.lazy ?? []).map((ref) => ({ kind: 'lazy', name: ref.path, ref }));
  cache = { manifest, objects: bundleObjects(BUNDLE_DIR, BR_DIR, BUNDLE), boot, lazy, all: [...boot, ...lazy] };
  return cache;
}

/** the bucket key of a reference (its URL without /assets/): `fl-Oz/<file>` */
export const keyOf = (ref) => decodeURIComponent(ref.url.slice('/assets/'.length));
/** the stored (brotli) file of a reference in the bundle's br directory */
export const brFileOf = (ref) => path.join(BR_DIR, keyOf(ref).slice(BUNDLE.length + 1));
export const rawManifestPath = () => path.join(BUNDLE_DIR, 'manifest.json');

/** the bucket of the Worker's R2 interface over the bundle's objects, read from disk (the same shape as lib/served.mjs) */
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

/** the delivery of this run, or a throw (the scenarios need it: liveDelivery is set by startDelivery) */
export function live() {
  const d = liveDelivery();
  if (!d) throw new Error('no delivery is running (lib/delivery.mjs liveDelivery)');
  return d;
}

/**
 * the Worker in this process, over the run's bucket. `token`: undefined = a valid token of the run, null = none, a string = that token.
 * Returns the Response (its body is the stored bytes as the Worker sends them).
 */
export async function workerFetch(p, { method = 'GET', headers = {}, token, body } = {}) {
  const d = live();
  const h = { ...headers };
  if (token !== null) h['Cf-Access-Jwt-Assertion'] = token ?? d.newToken();
  const env = { ASSETS: { fetch: async () => new Response('<!doctype html>', { headers: { 'Content-Type': 'text/html' } }) }, ASSETS_BUCKET: diskBucket(bundle().objects), ...d.accessEnv };
  return worker.fetch(new Request(`http://delivery.invalid${p}`, { method, headers: h, body }), env);
}

/** the wire: the Worker's own address (wrangler dev --local), not the test proxy. `token` as in workerFetch. Returns the status and the headers (the body is dropped) */
export async function wireFetch(p, { method = 'GET', headers = {}, token, body } = {}) {
  const d = live();
  const h = { ...headers };
  if (token !== null) h['Cf-Access-Jwt-Assertion'] = token ?? d.newToken();
  const r = await fetch(`${d.upstream}${p}`, { method, headers: h, body, redirect: 'manual', signal: AbortSignal.timeout(20000) });
  await r.body?.cancel();
  return { status: r.status, headers: Object.fromEntries([...r.headers]) };
}

/** a fetch in the page (the shared page: its requests carry the token): the status and the headers; GET reads the body (its size is not kept) */
export async function pageFetch(page, p, method = 'GET') {
  return page.evaluate(async ({ u, m }) => {
    const r = await fetch(u, { method: m, cache: 'no-store' });
    if (m === 'GET') await r.arrayBuffer();
    return { status: r.status, headers: Object.fromEntries([...r.headers]) };
  }, { u: p, m: method });
}

/** the directives of a Cache-Control value, lower case */
export const directivesOf = (v) => new Set(String(v ?? '').split(',').map((s) => s.trim().toLowerCase()).filter(Boolean));

/**
 * the Cache-Control of a served object against the rule of app/shared/headers.ts (cacheControlFor of its path): every directive of the rule (but `public`)
 * is served, and `no-transform` is added (app/worker/index.ts). Returns the directives that are missing (an empty list: it matches).
 */
export function cacheMissing(served, urlPath) {
  const want = [...directivesOf(cacheControlFor(urlPath))].filter((d) => d !== 'public');
  const got = directivesOf(served);
  const missing = want.filter((d) => !got.has(d));
  if (!got.has('no-transform')) missing.push('no-transform');
  return missing;
}

/** the headers that differ from the wanted values (names as the Response keeps them, lower case); [] when all match */
export function headerDiffs(headers, want) {
  const out = [];
  for (const [k, v] of Object.entries(want)) {
    const got = headers[k.toLowerCase()] ?? null;
    if (got !== v) out.push(`${k}: got ${JSON.stringify(got)} want ${JSON.stringify(v)}`);
  }
  return out;
}

export const ISOLATION = { ...ISOLATION_HEADERS, ...PRIVACY_HEADERS };
export { CSP };

/** the state of the page (window.__app.state), polled until it is one of `states` or the limit; returns the last state read */
export async function waitState(page, states, ms) {
  const t0 = Date.now();
  let s = null;
  for (;;) {
    s = await page.evaluate(() => (window.__app ? window.__app.state : null)).catch(() => null);
    if (s !== null && states.includes(s)) return s;
    if (Date.now() - t0 >= ms) return s;
    await sleep(400);
  }
}

/** polls a predicate in the page until it is true or the limit; returns the last value */
export async function waitPage(page, fn, arg, ms) {
  const t0 = Date.now();
  for (;;) {
    const v = await page.evaluate(fn, arg).catch(() => null);
    if (v) return v;
    if (Date.now() - t0 >= ms) return v;
    await sleep(400);
  }
}

/** a page of its own: a new context of the run's browser (no token of its own: the Access proxy adds one). Its failures are counted here, in `counts` */
export async function ownPage(page, { viewport } = {}) {
  const browser = page.context().browser();
  if (!browser) throw new Error('no browser to open a page of its own');
  const context = await browser.newContext({ viewport: viewport ?? { width: 3840, height: 2160 }, deviceScaleFactor: 1, screen: viewport ?? { width: 3840, height: 2160 } });
  const p = await context.newPage();
  const counts = { pageErrors: 0, consoleErrors: 0, failedRequests: 0, details: [] };
  const note = (s) => { if (counts.details.length < 20) counts.details.push(s.slice(0, 200)); };
  p.on('pageerror', (e) => { counts.pageErrors++; note(`pageerror: ${String(e?.message ?? e)}`); });
  p.on('console', (m) => { if (m.type() === 'error') { counts.consoleErrors++; note(`console.error: ${m.text()}`); } });
  p.on('requestfailed', (r) => { counts.failedRequests++; note(`requestfailed: ${r.url()}`); });
  const cdp = await context.newCDPSession(p);
  return { page: p, cdp, counts, close: () => context.close().catch(() => {}) };
}

/** the sum of the decoded sizes of the references (the bundle's manifest) */
export const sizeOf = (refs) => refs.reduce((a, r) => a + r.ref.size, 0);

const captures = new WeakMap();
/**
 * the first measurement beacon the page sends after this call (POST PERF_PATH): its body, its request headers and the answer's status and X-Perf-Stored.
 * The page sends one at its first idle and every 30 s while it runs; a capture waits for the next one (up to `ms`). The result is kept per page.
 */
export async function beaconOf(page, ms = 45000) {
  if (captures.has(page)) return captures.get(page);
  const got = { text: null, reqHeaders: null, status: null, stored: null };
  const isBeacon = (u, m) => m === 'POST' && new URL(u).pathname === PERF_PATH;
  const onReq = (r) => { if (got.text === null && isBeacon(r.url(), r.method())) { got.text = r.postData() ?? ''; got.reqHeaders = r.headers(); } };
  const onRes = (res) => { if (got.status === null && isBeacon(res.url(), res.request().method())) { got.status = res.status(); got.stored = res.headers()['x-perf-stored'] ?? null; } };
  page.on('request', onReq);
  page.on('response', onRes);
  try {
    const t0 = Date.now();
    while ((got.text === null || got.status === null) && Date.now() - t0 < ms) await sleep(300);
  } finally {
    page.off('request', onReq);
    page.off('response', onRes);
  }
  captures.set(page, got);
  return got;
}

/** the problems of a beacon's form: app/shared/perf.ts beaconProblems, and the top-level names that are not BEACON_KEYS */
export function beaconForm(text) {
  let b;
  try { b = JSON.parse(text); } catch (e) { return { parsed: false, problems: [`not JSON: ${String(e?.message ?? e).slice(0, 80)}`], unknownKeys: [] }; }
  const unknownKeys = Object.keys(b ?? {}).filter((k) => !BEACON_KEYS.includes(k));
  return { parsed: true, problems: beaconProblems(b), unknownKeys };
}

/** the names of any object's keys at any depth that a beacon must not carry (PERSONAL) */
export function personalNames(v, out = []) {
  if (Array.isArray(v)) { for (const x of v) personalNames(x, out); return out; }
  if (v && typeof v === 'object') {
    for (const [k, x] of Object.entries(v)) {
      if (PERSONAL.includes(k.toLowerCase())) out.push(k);
      personalNames(x, out);
    }
  }
  return out;
}

/** a request with a token the run did not make (an RS256 token with a key of its own, under the kid the test uses): the Worker must refuse it */
export function foreignToken(team, aud) {
  const { privateKey } = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
  const b64u = (buf) => Buffer.from(buf).toString('base64url');
  const s = Math.floor(Date.now() / 1000);
  const head = b64u(JSON.stringify({ alg: 'RS256', kid: 'foreign-kid', typ: 'JWT' }));
  const body = b64u(JSON.stringify({ iss: `https://${team}`, aud: [aud], sub: 'e2e', iat: s, nbf: s - 10, exp: s + 3600 }));
  const sig = crypto.sign('RSA-SHA256', Buffer.from(`${head}.${body}`), privateKey);
  return `${head}.${body}.${b64u(sig)}`;
}

/** a valid token of the run with its payload changed (the signature no longer matches) */
export function tamperedToken(token) {
  const [h, p, s] = token.split('.');
  const claims = JSON.parse(Buffer.from(p, 'base64url').toString('utf8'));
  claims.sub = 'someone-else';
  return `${h}.${Buffer.from(JSON.stringify(claims)).toString('base64url')}.${s}`;
}

/** a body with the decoded bytes of a stored (brotli) object: its SHA-256 and size */
export function brotliDecoded(buf) {
  const out = zlib.brotliDecompressSync(buf);
  return { sha256: sha256Hex(out), size: out.length };
}

/** the Worker's response for one reference: the stored bytes (what is on disk), read through the Worker in this process */
export async function storedBytesOf(ref) {
  const res = await workerFetch(ref.url);
  const buf = Buffer.from(await res.arrayBuffer());
  return { status: res.status, headers: Object.fromEntries([...res.headers]), buf };
}

/** the sha256 values of the bundle (boot and lazy), to tell a stored copy of this bundle from one of another version */
export function bundleShas() {
  return new Set(bundle().all.map((r) => r.ref.sha256));
}

/** the feature IDs of the bundle's lazy files: the rows of features/generated/delivery.tsv (one per lazy file of the manifest) */
export function lazyFeatureIds() {
  const lines = fs.readFileSync(FEATURES_GENERATED, 'utf8').split(/\r?\n/).filter((l) => l.length > 0 && !l.startsWith('#'));
  const head = lines[0].split('\t');
  const ci = head.indexOf('id'), ki = head.indexOf('kind');
  return lines.slice(1).map((l) => l.split('\t')).filter((c) => c[ki] === 'lazy').map((c) => c[ci]);
}

/** the boot references' first 8 hex digits of SHA-256 (the assets report names an asset that way) */
export const bootSha8 = () => new Set(bundle().boot.map((b) => b.ref.sha256.slice(0, 8)));

export { PERF_MAX_BODY, PERF_PATH, validateManifest };
