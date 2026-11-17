// Stage 3 measurement of decision 0216 (opfs1): the browser's synchronous OPFS access from a Worker, and what each way of reaching it from a suspended / blocked engine costs. Chrome (CDP) or Firefox (geckodriver).
// The files are lift/harness/web/opfs1/{bench.html,bench-worker.js} (jspi.wat is the source of the small wasm inlined in bench.html), served by a tiny server with the isolation headers (SharedArrayBuffer needs them).
// usage: APP_WORK=<dir (under $HOME for Firefox)> node test/e2e/opfs-bench.mjs --browser chrome|firefox [--out FILE.json]
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { freePort } from '../helpers/proc.mjs';
import { APP, workDirOf } from '../helpers/wrangler.mjs';
import { openBrowser } from './browser.mjs';

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const browserName = opt('browser', 'chrome');
const dir = path.resolve(APP, '..', 'lift', 'harness', 'web', 'opfs1');
const MIME = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.wasm': 'application/wasm' };
const server = http.createServer((req, res) => {
  const f = path.join(dir, path.basename(new URL(req.url, 'http://x').pathname === '/' ? 'bench.html' : new URL(req.url, 'http://x').pathname));
  if (!fs.existsSync(f)) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { 'content-type': MIME[path.extname(f)] ?? 'application/octet-stream', 'cross-origin-opener-policy': 'same-origin', 'cross-origin-embedder-policy': 'require-corp', 'cross-origin-resource-policy': 'same-origin' });
  res.end(fs.readFileSync(f));
});
const port = await freePort();
await new Promise((r) => server.listen(port, '127.0.0.1', r));
const url = `http://127.0.0.1:${port}/`;
const work = workDirOf('opfs-bench');
let ev, goto, close;
if (browserName === 'chrome') {
  const b = await openBrowser(work, 'bench');
  const p = await b.newPage({ width: 800, height: 600 });
  ev = p.ev; goto = p.goto; close = async () => { await b.close(); };
  var version = 'Chrome ' + b.version;
} else {
  const { openFirefox } = await import('./firefox-driver.mjs');
  const ff = await openFirefox({ work, downloadDir: path.join(work, 'dl'), dataDir: work });
  ev = ff.ev; goto = ff.goto; close = async () => { await ff.close(); };
  version = 'Firefox ' + ff.version;
}
const out = { browser: version, machine: `${os.cpus().length} cores`, load: os.loadavg()[0], date: new Date().toISOString(), results: {} };
const step = async (name, expr) => { const t0 = Date.now(); try { out.results[name] = await ev(expr); } catch (e) { out.results[name] = { error: String(e.message ?? e).slice(0, 300) }; } console.log(name, JSON.stringify(out.results[name]).slice(0, 400), `(${Date.now() - t0} ms)`); };
await goto(url);
await ev('new Promise((r) => { const t = setInterval(() => { if (window.call) { clearInterval(t); r(true); } }, 50); })');
await step('caps', "window.call('caps')");
await step('sync', "window.call('sync', { files: 100, big: 64, block: 1 })");
await step('syncBlock64k', "window.call('sync', { files: 20, big: 32, block: 0.0625 })");
await step('sabview', "window.call('sabview')");
await step('lock', "window.call('lock')");
await step('bridge', 'window.bridge(300)');
await step('jspi', 'window.jspi()');
await step('persistWrite', "window.call('persist', { phase: 'write' })");
await ev('window.kill()');
await goto(url);
await ev('new Promise((r) => { const t = setInterval(() => { if (window.call) { clearInterval(t); r(true); } }, 50); })');
await step('persistRead', "window.call('persist', { phase: 'read' })");
await ev("(async () => { const root = await navigator.storage.getDirectory(); try { await root.removeEntry('bench', { recursive: true }); } catch (e) {} return true; })()").catch(() => {});
await close();
server.close();
const file = opt('out', path.join(work, `opfs-bench-${browserName}.json`));
fs.writeFileSync(file, JSON.stringify(out, null, 1));
console.log('wrote', file);
