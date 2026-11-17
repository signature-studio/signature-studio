// mt2 mp1 in headless Firefox (geckodriver): node mp1-chrome.mjs WASM [--n 100000000] [--lat 3000] [--reps 6] [--out FILE.json]   (FL_CHROME_ARGS="--js-flags=--no-liftoff" etc. pass through openBrowser)
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
const HERE = path.dirname(fileURLToPath(import.meta.url));
const a = process.argv.slice(2);
const opt = (n, d) => { const i = a.indexOf('--' + n); return i < 0 ? d : a[i + 1]; };
const APP = process.env.MP1_APP || '/home/ubuntu/work/signature-studio-develop/lift/out-mt2/src/app';
const { openFirefox } = await import(pathToFileURL(path.join(APP, 'test/e2e/firefox-driver.mjs')).href);
const files = { '/mp1-eng.js': path.join(HERE, 'mp1-eng.js'), '/mp1-help.js': path.join(HERE, 'mp1-help.js'), '/mp1.wasm': path.resolve(a[0]), '/pingasync-eng.js': path.join(HERE, 'pingasync-eng.js'), '/pingasync.js': path.join(HERE, 'pingasync.js') };
const srv = http.createServer((req, res) => {
  const f = files[req.url.split('?')[0]];
  const h = { 'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp', 'Cross-Origin-Resource-Policy': 'same-origin', 'Cache-Control': 'no-store' };
  if (req.url === '/' || req.url === '/index.html') { res.writeHead(200, { ...h, 'Content-Type': 'text/html' }); res.end('<!doctype html><title>mp1</title>'); return; }
  if (!f) { res.writeHead(404, h); res.end(); return; }
  res.writeHead(200, { ...h, 'Content-Type': f.endsWith('.wasm') ? 'application/wasm' : 'text/javascript' });
  res.end(fs.readFileSync(f));
});
await new Promise((r) => srv.listen(0, '127.0.0.1', r));
const port = srv.address().port;
const out = path.resolve(opt('outdir', '/home/ubuntu/work/signature-studio-develop/lift/out-mt2/runs/mp1-firefox'));
fs.mkdirSync(out, { recursive: true });
const ff = await openFirefox({ work: out, downloadDir: path.join(out, 'dl'), dataDir: out, scriptTimeoutMs: 600000 });
await ff.goto(`http://127.0.0.1:${port}/`);
const N = Number(opt('n', '100000000')), LAT = Number(opt('lat', '3000')), reps = Number(opt('reps', '6'));
const PING = process.argv.includes('--ping');
const r = PING ? await ff.ev(`new Promise((resolve) => { const w = new Worker('/pingasync-eng.js'); w.onmessage = (e) => resolve(e.data); w.postMessage({ n: 20000 }); })`) : await ff.ev(`new Promise((resolve) => { const w = new Worker('/mp1-eng.js'); w.onmessage = (e) => resolve(Object.assign(e.data, { waitAsync: typeof Atomics.waitAsync, isolated: self.crossOriginIsolated })); w.onerror = (e) => resolve({ error: String(e.message) }); w.postMessage({ url: '/mp1.wasm', N: ${N}, LAT: ${LAT}, SPINS: [0, 200000], reps: ${reps} }); })`);
console.log(JSON.stringify(r, null, 1));
if (opt('out', '')) fs.writeFileSync(opt('out', ''), JSON.stringify(r, null, 1));
await ff.close();
srv.close();
process.exit(0);
