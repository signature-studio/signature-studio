// node shcopy-run.mjs chrome|firefox OUT.json   (serves shcopy-worker.js with COOP/COEP; the worker needs no page)
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
const HERE = path.dirname(fileURLToPath(import.meta.url));
const which = process.argv[2] || 'chrome', outFile = process.argv[3];
const APP = '/home/ubuntu/work/signature-studio-develop/lift/out-mt2/src/app';
const srv = http.createServer((req, res) => {
  const h = { 'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp', 'Cross-Origin-Resource-Policy': 'same-origin', 'Cache-Control': 'no-store' };
  if (req.url === '/shcopy-worker.js') { res.writeHead(200, { ...h, 'Content-Type': 'text/javascript' }); res.end(fs.readFileSync(path.join(HERE, 'shcopy-worker.js'))); return; }
  res.writeHead(200, { ...h, 'Content-Type': 'text/html' }); res.end('<!doctype html><title>x</title>');
});
await new Promise((r) => srv.listen(0, '127.0.0.1', r));
const url = `http://127.0.0.1:${srv.address().port}/`;
const cases = [
  { label: 'rect 64x64 in 1920x1080', W: 1920, H: 1080, x0: 100, y0: 100, w: 64, h: 64, iters: 1000 },
  { label: 'rect 512x512 in 1920x1080', W: 1920, H: 1080, x0: 200, y0: 100, w: 512, h: 512, iters: 200 },
  { label: 'full 1920x1080', W: 1920, H: 1080, x0: 0, y0: 0, w: 1920, h: 1080, iters: 100 },
  { label: 'full 3240x1946', W: 3240, H: 1946, x0: 0, y0: 0, w: 3240, h: 1946, iters: 40 },
];
const script = `new Promise((resolve) => { const w = new Worker('/shcopy-worker.js'); w.onmessage = (e) => resolve(e.data); w.onerror = (e) => resolve({ error: String(e.message) }); w.postMessage({ cases: ${JSON.stringify(cases)} }); })`;
let r;
if (which === 'chrome') {
  const { openBrowser } = await import(pathToFileURL(path.join(APP, 'test/e2e/browser.mjs')).href);
  const b = await openBrowser('/home/ubuntu/work/signature-studio-develop/lift/out-mt2/runs/shcopy-chrome', 'shc');
  const p = await b.newPage({ width: 400, height: 300 }); await p.goto(url); r = await p.ev(script, 600000); await b.close?.();
} else {
  const { openFirefox } = await import(pathToFileURL(path.join(APP, 'test/e2e/firefox-driver.mjs')).href);
  const w = '/home/ubuntu/work/signature-studio-develop/lift/out-mt2/runs/shcopy-firefox'; fs.mkdirSync(w, { recursive: true });
  const ff = await openFirefox({ work: w, downloadDir: w + '/dl', dataDir: w, scriptTimeoutMs: 600000 }); await ff.goto(url); r = await ff.ev(script); await ff.close();
}
console.log(JSON.stringify(r, null, 1));
if (outFile) fs.writeFileSync(outFile, JSON.stringify(r, null, 1));
srv.close(); process.exit(0);
