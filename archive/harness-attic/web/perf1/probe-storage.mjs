// perf1 (decision 0130, PROVISIONAL): what an isolated (new browser context, as run-chrome-perf.mjs) headless Chrome keeps between two loads of the same page: HTTP cache entry size, Cache Storage put, IndexedDB put, storage quota.
// usage: node probe-storage.mjs OUT.json
// The page is served by a local server (127.0.0.1, a secure context); the probes run with Runtime.evaluate. Network isolation as in run-chrome-perf.mjs (the same isolationArgs, net-log, the same check).
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { launchChrome, Cdp, shutdown, sleep } from '../cdp.mjs';
import { isolationArgs, withTimeout } from '../present/lib.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const outFile = process.argv[2];
const WORK = process.env.PRESENT_WORK || '/tmp/opencode/perf1/chrome';
fs.mkdirSync(WORK, { recursive: true });
const hits = {};   // path -> number of requests that reached the server
const server = http.createServer((req, res) => {
  const u = new URL(req.url, 'http://x');
  if (u.pathname === '/') { res.writeHead(200, { 'Content-Type': 'text/html' }); res.end('<!doctype html><title>probe</title>'); return; }
  const m = /^\/blob-(\d+)$/.exec(u.pathname);
  if (!m) { res.writeHead(404).end(); return; }
  hits[u.pathname] = (hits[u.pathname] || 0) + 1;
  const n = +m[1] * 1048576;
  const buf = Buffer.alloc(n);
  for (let i = 0; i < n; i += 4) buf.writeUInt32LE((i * 2654435761) >>> 0, i);   // not compressible by accident, not a constant
  const etag = '"' + n + '"';
  if (req.headers['if-none-match'] === etag) { res.writeHead(304, { ETag: etag }); res.end(); hits[u.pathname + ' 304'] = (hits[u.pathname + ' 304'] || 0) + 1; return; }
  res.writeHead(200, { 'Content-Type': 'application/octet-stream', 'Content-Length': n, 'Cache-Control': 'max-age=3600', ETag: etag });
  res.end(buf);
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const port = server.address().port;
const profile = path.join(WORK, 'profile-probe');
const netlog = path.join(WORK, 'netlog-probe.json');
fs.rmSync(profile, { recursive: true, force: true });
const chrome = await launchChrome({ profile, extraArgs: isolationArgs(profile, netlog) });
const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
await cdp.opened;
const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
const S = (method, params, ms = 300000) => withTimeout(cdp.send(method, params, sessionId), ms, 'CDP ' + method);
await S('Runtime.enable'); await S('Page.enable'); await S('Network.enable'); await S('Network.setCacheDisabled', { cacheDisabled: false });
await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
await sleep(800);
const ev = async (expression) => { const r = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); if (r.exceptionDetails) return { error: JSON.stringify(r.exceptionDetails).slice(0, 300) }; return r.result.value; };
const out = { chrome: chrome.version.Browser, context: 'new browser context (Target.createBrowserContext)', storageEstimate: await ev('navigator.storage.estimate().then(e => ({quota: e.quota, usage: e.usage}))'),
  persisted: await ev('navigator.storage.persisted ? navigator.storage.persisted() : null'), http: [], cacheStorage: [], indexedDB: [] };
// 1. HTTP cache: fetch /blob-N twice (second time after a pause); did the second request reach the server?
for (const mb of (process.env.HTTP_SIZES ? process.env.HTTP_SIZES.split(',').map(Number) : [1, 4, 8, 16, 20, 24, 32, 64])) {
  const p = '/blob-' + mb;
  const t1 = await ev(`(async () => { const t = performance.now(); const r = await fetch('${p}'); const b = await r.arrayBuffer(); return {ms: performance.now() - t, bytes: b.byteLength}; })()`);
  const before = hits[p] || 0;
  await sleep(500);
  const t2 = await ev(`(async () => { const t = performance.now(); const r = await fetch('${p}'); const b = await r.arrayBuffer(); return {ms: performance.now() - t, bytes: b.byteLength}; })()`);
  out.http.push({ mb, firstMs: t1.ms && +t1.ms.toFixed(1), secondMs: t2.ms && +t2.ms.toFixed(1), serverRequestsAfterFirst: before, serverRequestsAfterSecond: hits[p] || 0, secondFromCache: (hits[p] || 0) === before });
}
// 2. Cache Storage: put a Response of N MB, read it back
for (const mb of (process.env.CS_SIZES ? process.env.CS_SIZES.split(',').map(Number) : [8, 16, 22, 32, 64, 128, 256])) {
  const r = await ev(`(async () => { try { const c = await caches.open('probe'); const b = new Uint8Array(${mb} * 1048576); for (let i = 0; i < b.length; i += 4096) b[i] = i >> 12; const t = performance.now(); await c.put('/c-${mb}', new Response(b)); const t1 = performance.now() - t; const g = await c.match('/c-${mb}'); const ab = await g.arrayBuffer(); return {ok: true, putMs: +t1.toFixed(1), readBytes: ab.byteLength}; } catch (e) { return {ok: false, error: String(e).slice(0, 160)}; } })()`);
  out.cacheStorage.push(Object.assign({ mb }, r));
}
// 3. IndexedDB: put an ArrayBuffer of N MB
for (const mb of (process.env.IDB_SIZES ? process.env.IDB_SIZES.split(',').map(Number) : [8, 22, 64, 128, 256])) {
  const r = await ev(`(async () => { try { const db = await new Promise((res, rej) => { const q = indexedDB.open('probe', 1); q.onupgradeneeded = () => { if (!q.result.objectStoreNames.contains('s')) q.result.createObjectStore('s'); }; q.onsuccess = () => res(q.result); q.onerror = () => rej(q.error); }); const b = new Uint8Array(${mb} * 1048576); for (let i = 0; i < b.length; i += 4096) b[i] = i >> 12; const t = performance.now(); await new Promise((res, rej) => { const tx = db.transaction('s', 'readwrite'); tx.objectStore('s').put(b.buffer, 'k${mb}'); tx.oncomplete = res; tx.onerror = () => rej(tx.error); tx.onabort = () => rej(tx.error); }); const t1 = performance.now() - t; const back = await new Promise((res, rej) => { const q = db.transaction('s').objectStore('s').get('k${mb}'); q.onsuccess = () => res(q.result); q.onerror = () => rej(q.error); }); db.close(); return {ok: true, putMs: +t1.toFixed(1), readBytes: back.byteLength}; } catch (e) { return {ok: false, error: String(e && e.message || e).slice(0, 160)}; } })()`);
  out.indexedDB.push(Object.assign({ mb }, r));
}
out.storageEstimateAfter = await ev('navigator.storage.estimate().then(e => ({quota: e.quota, usage: e.usage}))');
await shutdown(chrome, cdp);
server.closeAllConnections(); server.close();
await sleep(400);
const nl = spawnSync('node', [path.join(HERE, '../threads/check_netlog.mjs'), netlog, '127.0.0.1', 'tolerate-ipv6-probe'], { encoding: 'utf8' });
out.netlog = { summary: (nl.stdout || nl.stderr || '').trim(), status: nl.status };
fs.rmSync(profile, { recursive: true, force: true });
fs.rmSync(netlog, { force: true });
fs.writeFileSync(outFile, JSON.stringify(out, null, 1));
console.log(JSON.stringify(out, null, 1));
