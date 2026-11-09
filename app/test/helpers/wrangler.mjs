// `wrangler dev --local` (miniflare: local R2 and Static Assets simulation) for the tests. Local mode only: nothing here logs in, talks to Cloudflare, or deploys.
// wrangler's own config / log directories are redirected into the work directory (wranglerEnv), so the operator's wrangler login is neither read nor written.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { DatabaseSync } from 'node:sqlite';
import http from 'node:http';
import { freePort, run, start, wranglerEnv } from './proc.mjs';
import { claimsFor, makeKey, signJwt } from './jwt.ts';

export const APP = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
export const BUCKET = 'fl-studio-web-assets-changeme'; // the placeholder name of wrangler.jsonc
export const PERF_BUCKET = 'fl-studio-web-perf-changeme'; // decision 0224: the placeholder name of the beacon bucket (binding PERF_BUCKET)
const bin = (n) => path.join(APP, 'node_modules/.bin', n);

export function workDirOf(name) {
  const d = process.env.APP_WORK || process.env.APP0_WORK || '/tmp/opencode/app1';
  const w = path.join(d, name);
  fs.mkdirSync(w, { recursive: true });
  return w;
}

// puts one object into the local R2 bucket (persisted under workDir/state, which `wrangler dev --persist-to` then reads)
export async function putObject(workDir, key, file, { contentType, contentEncoding, cacheControl } = {}) {
  const args = ['r2', 'object', 'put', `${BUCKET}/${key}`, '--local', '--persist-to', path.join(workDir, 'state'), '--file', file];
  if (contentType) args.push('--content-type', contentType);
  if (contentEncoding) args.push('--content-encoding', contentEncoding);
  if (cacheControl) args.push('--cache-control', cacheControl);
  await run(bin('wrangler'), args, { cwd: APP, env: wranglerEnv(workDir), label: `r2 object put ${key}` });
}

// removes one object from the local R2 bucket (the dev server reads the same persisted state, so it sees the change on its next request)
export async function deleteObject(workDir, key) {
  const args = ['r2', 'object', 'delete', `${BUCKET}/${key}`, '--local', '--persist-to', path.join(workDir, 'state'), '--force'];
  await run(bin('wrangler'), args, { cwd: APP, env: wranglerEnv(workDir), label: `r2 object delete ${key}` });
}

// decision 0224: the keys (and sizes) the Worker has put under a prefix of the local PERF_BUCKET: read from the sqlite file miniflare keeps for the bucket (one *.sqlite per bucket next to metadata.sqlite)
export function listLocalKeys(workDir, prefix = 'perf/') {
  const dir = path.join(workDir, 'state', 'v3', 'r2', 'miniflare-R2BucketObject');
  if (!fs.existsSync(dir)) return [];
  const out = [];
  for (const f of fs.readdirSync(dir)) {
    if (!f.endsWith('.sqlite') || f === 'metadata.sqlite') continue;
    const db = new DatabaseSync(path.join(dir, f), { readOnly: true });
    try { for (const r of db.prepare('SELECT key, size, http_metadata FROM _mf_objects WHERE key LIKE ? ORDER BY key').all(`${prefix}%`)) out.push({ key: r.key, size: r.size, contentType: JSON.parse(r.http_metadata ?? '{}').contentType ?? null }); } finally { db.close(); }
  }
  return out;
}

// the text of one object of the local PERF_BUCKET (null: not there): its blob file, found through the key's row in the bucket's sqlite file (no wrangler process: the e2e run reads many)
export async function getPerfObject(workDir, key) {
  const dir = path.join(workDir, 'state', 'v3', 'r2', 'miniflare-R2BucketObject');
  if (!fs.existsSync(dir)) return null;
  for (const f of fs.readdirSync(dir)) {
    if (!f.endsWith('.sqlite') || f === 'metadata.sqlite') continue;
    const db = new DatabaseSync(path.join(dir, f), { readOnly: true });
    try {
      const row = db.prepare('SELECT blob_id FROM _mf_objects WHERE key = ?').get(key);
      if (row?.blob_id) { const blob = path.join(workDir, 'state', 'v3', 'r2', PERF_BUCKET, 'blobs', row.blob_id); return fs.existsSync(blob) ? fs.readFileSync(blob, 'utf8') : null; }
    } finally { db.close(); }
  }
  return null;
}

// Cloudflare Access in front of the Worker, for the tests (decision 0243: the Worker has no switch that skips the check; a test brings its own team, AUD, signing keys and tokens).
// startAccessKeys: a throw-away key pair, a JWKS server on 127.0.0.1 (the Worker's ACCESS_CERTS_URL), and the tokens a test needs. startAccessProxy: what Access does in production - every request reaches the
// Worker with a valid Cf-Access-Jwt-Assertion - so a test that is not about Access asks the proxy's URL, and a test about Access asks the Worker directly (`directUrl`) with the tokens it wants.
export const ACCESS_TEAM = 'test-team.cloudflareaccess.com';
export const ACCESS_AUD = 'aud-tag-0123456789abcdef';

export async function startAccessKeys() {
  const key = await makeKey('kid-current'), stranger = await makeKey('kid-current');
  const now = Math.floor(Date.now() / 1000);
  const tokens = {
    valid: await signJwt(key, claimsFor(ACCESS_TEAM, ACCESS_AUD, now)),
    wrongAud: await signJwt(key, claimsFor(ACCESS_TEAM, 'another-application', now)),
    wrongIss: await signJwt(key, claimsFor('evil.cloudflareaccess.com', ACCESS_AUD, now)),
    expired: await signJwt(key, claimsFor(ACCESS_TEAM, ACCESS_AUD, now - 7200, { exp: now - 3600 })),
    forged: await signJwt(stranger, claimsFor(ACCESS_TEAM, ACCESS_AUD, now)),
    noneAlg: await signJwt(key, claimsFor(ACCESS_TEAM, ACCESS_AUD, now), { alg: 'none' }),
  };
  const port = await freePort();
  let hits = 0;
  const server = http.createServer((req, res) => { hits++; res.writeHead(200, { 'Content-Type': 'application/json' }); res.end(JSON.stringify({ keys: [key.jwk] })); });
  await new Promise((r) => server.listen(port, '127.0.0.1', r));
  return { team: ACCESS_TEAM, aud: ACCESS_AUD, certsUrl: `http://127.0.0.1:${port}/cdn-cgi/access/certs`, tokens, hits: () => hits, stop: () => new Promise((r) => server.close(r)) };
}

export async function startAccessProxy(targetUrl, token) {
  const target = new URL(targetUrl);
  const port = await freePort();
  const server = http.createServer((req, res) => {
    const headers = { ...req.headers, host: target.host, 'cf-access-jwt-assertion': token };
    const up = http.request({ host: target.hostname, port: target.port, path: req.url, method: req.method, headers }, (r) => { res.writeHead(r.statusCode ?? 502, r.headers); r.pipe(res); });
    up.on('error', (e) => { if (!res.headersSent) res.writeHead(502); res.end(String(e)); });
    req.pipe(up);
  });
  await new Promise((r) => server.listen(port, '127.0.0.1', r));
  return { url: `http://127.0.0.1:${port}`, stop: () => new Promise((r) => { server.closeAllConnections(); server.close(r); }) };
}

// opts.config: another wrangler config (decision 0224: a Worker WITHOUT the PERF_BUCKET binding)
// The Worker is configured for the test Access application. `url` is the address behind the Access proxy (every request carries a valid token); `directUrl` is the Worker itself (no token unless the request has one).
// opts.keys: use these keys (and their tokens) instead of making new ones. opts.unconfigured: the Worker exactly as wrangler.jsonc ships it (no team, no AUD: it answers 403 to everything).
export async function startWorker(workDir, vars = {}, opts = {}) {
  const port = await freePort();
  const inspector = await freePort();
  const keys = opts.unconfigured ? null : (opts.keys ?? await startAccessKeys());
  const all = keys ? { ACCESS_TEAM_DOMAIN: keys.team, ACCESS_AUD: keys.aud, ACCESS_CERTS_URL: keys.certsUrl, ...vars } : vars;
  const args = ['dev', '--local', ...(opts.config ? ['--config', opts.config] : []), '--ip', '127.0.0.1', '--port', String(port), '--inspector-port', String(inspector), '--persist-to', path.join(workDir, 'state')];
  for (const [k, v] of Object.entries(all)) args.push('--var', `${k}:${v}`);
  const w = start(bin('wrangler'), args, { cwd: APP, env: wranglerEnv(workDir), ready: /Ready on http/, label: 'wrangler dev' });
  await w.ready;
  const directUrl = `http://127.0.0.1:${port}`;
  const proxy = keys ? await startAccessProxy(directUrl, keys.tokens.valid) : null;
  return {
    url: proxy ? proxy.url : directUrl, directUrl, keys, output: w.output,
    stop: async () => { await proxy?.stop(); await w.stop(); if (keys && !opts.keys) await keys.stop(); },
  };
}
