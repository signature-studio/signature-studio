// The production delivery for the test (method a): the Worker of app/worker/index.ts under `wrangler dev --local`, with its ASSETS (app/dist) and its R2
// bucket (the bucket of app/wrangler.jsonc, binding ASSETS_BUCKET) filled with the bundle under the keys deploy.yml uses:
//   br/<bundle>/<file>  -> key <bundle>/<file>, Content-Encoding br, Cache-Control "max-age=31536000, immutable"   (aws s3 sync br ... --content-encoding br --cache-control ...)
//   raw/<bundle>/manifest.json -> keys <bundle>/manifest.json and manifest.json, Cache-Control "no-cache"          (aws s3 sync raw ... / aws s3 cp ...)
// Access: the Worker's own check runs. ACCESS_TEAM_DOMAIN, ACCESS_AUD and ACCESS_CERTS_URL are given on the wrangler command line only (lib/access.mjs makes them for this run).
// The bucket name and the account of the product are read from app/wrangler.jsonc at run time (the bucket's binding name, ASSETS_BUCKET, is the only name written here).
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import net from 'node:net';
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import { makeAccess, startJwks } from './access.mjs';

export const BR_CACHE = 'max-age=31536000, immutable';
export const RAW_CACHE = 'no-cache';

const freePort = () => new Promise((resolve, reject) => {
  const s = net.createServer();
  s.listen(0, '127.0.0.1', () => { const p = s.address().port; s.close(() => resolve(p)); });
  s.on('error', reject);
});

// the environment of the wrangler child: the job's variables, but no credentials (names with TOKEN, SECRET, KEY, PEM, PASSWORD, or the job's service prefixes)
const cleanEnv = () => {
  const out = {};
  for (const [k, v] of Object.entries(process.env)) if (!/^(AWS|R2|GH|GITHUB|OPENCODE)|TOKEN|SECRET|KEY|PEM|PASSWORD/i.test(k)) out[k] = v;
  return { ...out, WRANGLER_SEND_METRICS: 'false', CI: '1' };
};

/** the bucket's keys of one bundle: br files and the manifests (the same keys and metadata as deploy.yml's aws s3 commands) */
export function bundleObjects(bundleDir, brDir, bundle) {
  const files = fs.readdirSync(brDir, { withFileTypes: true }).filter((e) => e.isFile()).map((e) => e.name).sort();
  const objects = files.map((f) => ({ key: `${bundle}/${f}`, file: path.join(brDir, f), httpMetadata: { contentEncoding: 'br', cacheControl: BR_CACHE } }));
  const manifest = path.join(bundleDir, 'manifest.json');
  objects.push({ key: `${bundle}/manifest.json`, file: manifest, httpMetadata: { contentType: 'application/json', cacheControl: RAW_CACHE } });
  objects.push({ key: 'manifest.json', file: manifest, httpMetadata: { contentType: 'application/json', cacheControl: RAW_CACHE } });
  return objects;
}

/** writes the bundle into the local R2 (the state wrangler dev reads: persist path <stateDir>/v3) */
export async function seedBucket({ appDir, stateDir, objects, log }) {
  const wrangler = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('wrangler')).href);
  const proxy = await wrangler.getPlatformProxy({ configPath: path.join(appDir, 'wrangler.jsonc'), persist: { path: path.join(stateDir, 'v3') }, remoteBindings: false });
  let bytes = 0;
  try {
    const bucket = proxy.env.ASSETS_BUCKET;
    if (!bucket || typeof bucket.put !== 'function') throw new Error('the Worker has no ASSETS_BUCKET binding');
    for (const o of objects) {
      const body = fs.readFileSync(o.file);
      await bucket.put(o.key, body, { httpMetadata: o.httpMetadata });
      bytes += body.length;
    }
  } finally {
    await proxy.dispose();
  }
  log(`seeded ${objects.length} objects, ${bytes} bytes, into the local bucket`);
  return { objects: objects.length, bytes };
}

async function pollUntil(fn, ms, what) {
  const t0 = Date.now();
  for (;;) {
    const v = await fn().catch(() => null);
    if (v) return v;
    if (Date.now() - t0 >= ms) throw new Error(`timeout ${ms} ms: ${what}`);
    await new Promise((r) => setTimeout(r, 300));
  }
}

/**
 * The Access in front of the Worker, as the test makes it: a proxy that adds the token to each request that comes without one. The production's Access gate does this for
 * every request it forwards, the worklet's module request included; Chromium sends a page's extra headers with the page's requests only, so the worklet's request
 * (no token) would be refused by the Worker's own check. The Worker's check is the same (it reads Cf-Access-Jwt-Assertion); the requests that carry a token pass unchanged.
 */
function startAccessProxy(upstreamPort, newToken) {
  const server = http.createServer((req, res) => {
    const headers = { ...req.headers };
    if (!headers['cf-access-jwt-assertion']) headers['cf-access-jwt-assertion'] = newToken();
    const up = http.request({ host: '127.0.0.1', port: upstreamPort, method: req.method, path: req.url, headers }, (ur) => {
      res.writeHead(ur.statusCode ?? 502, ur.headers);
      ur.pipe(res);
    });
    up.on('error', (e) => {
      if (!res.headersSent) res.writeHead(502, { 'content-type': 'text/plain' });
      res.end(`${e?.message ?? e}\n`);
    });
    req.pipe(up);
  });
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      resolve({ port, close: () => new Promise((r) => { server.closeAllConnections?.(); server.close(() => r()); }) });
    });
  });
}

/**
 * Starts the delivery: seeds the bucket, runs `wrangler dev --local` on the Worker (app/), waits until the Worker answers with a token.
 * Returns { origin, upstream, token, delivery: 'a', access, close(), wranglerLog() }: origin is the page's (the Access proxy in front of the Worker),
 * upstream is the Worker's own address (the served-headers probe asks it directly: its gate check needs a request without a token).
 */
export async function startDelivery({ appDir, workDir, objects, log }) {
  const access = makeAccess();
  const jwks = await startJwks(access.jwk);
  const stateDir = path.join(workDir, 'state');
  fs.mkdirSync(stateDir, { recursive: true });
  await seedBucket({ appDir, stateDir, objects, log });
  const port = await freePort();
  const inspector = await freePort();
  const logFile = path.join(workDir, 'wrangler.log');
  const out = fs.openSync(logFile, 'w');
  const args = [
    path.join(appDir, 'node_modules', 'wrangler', 'bin', 'wrangler.js'), 'dev', '--config', path.join(appDir, 'wrangler.jsonc'),
    '--local', '--ip', '127.0.0.1', '--port', String(port), '--inspector-port', String(inspector), '--persist-to', stateDir,
    '--show-interactive-dev-session=false',
    '--var', `ACCESS_TEAM_DOMAIN:${access.team}`, '--var', `ACCESS_AUD:${access.aud}`, '--var', `ACCESS_CERTS_URL:${jwks.url}`,
  ];
  const child = spawn(process.execPath, args, { cwd: appDir, env: cleanEnv(), stdio: ['ignore', out, out], detached: true });
  let exited = null;
  child.on('exit', (code, sig) => { exited = { code, sig }; });
  const origin = `http://127.0.0.1:${port}`;
  const probe = async () => {
    if (exited) throw new Error(`wrangler dev exited: ${JSON.stringify(exited)}`);
    const r = await fetch(`${origin}/`, { headers: { 'Cf-Access-Jwt-Assertion': access.token() }, signal: AbortSignal.timeout(3000) });
    await r.body?.cancel();
    return r.status === 200 ? r : null;
  };
  try {
    await pollUntil(probe, 180000, 'the Worker answers 200 under wrangler dev --local');
  } catch (e) {
    await stopChild(child, jwks, out);
    throw new Error(`${e.message}\n--- wrangler log (tail) ---\n${tail(logFile)}`);
  }
  const proxy = await startAccessProxy(port, () => access.token());
  const pageOrigin = `http://127.0.0.1:${proxy.port}`;
  log(`delivery a: wrangler dev --local on ${origin}, the page on ${pageOrigin} (Access in front)`);
  return {
    origin: pageOrigin, upstream: origin, delivery: 'a', token: access.token(),
    accessEnv: { ACCESS_TEAM_DOMAIN: access.team, ACCESS_AUD: access.aud, ACCESS_CERTS_URL: jwks.url },
    newToken: () => access.token(),
    close: async () => { await proxy.close().catch(() => {}); await stopChild(child, jwks, out); },
    wranglerLog: () => tail(logFile),
  };
}

function tail(file, n = 4000) {
  try { const s = fs.readFileSync(file, 'utf8'); return s.slice(-n); } catch { return ''; }
}

async function stopChild(child, jwks, fd) {
  if (child.exitCode === null && child.signalCode === null) {
    const exited = new Promise((r) => child.once('exit', r));
    try { process.kill(-child.pid, 'SIGTERM'); } catch { /* gone */ }
    await Promise.race([exited, new Promise((r) => setTimeout(r, 8000))]);
    try { process.kill(-child.pid, 'SIGKILL'); } catch { /* gone */ }
  }
  await jwks.close().catch(() => {});
  try { fs.closeSync(fd); } catch { /* closed */ }
}
