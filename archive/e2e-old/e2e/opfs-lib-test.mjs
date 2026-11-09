// Stage 3 of decision 0216 (opfs1): runs the OPFS file library (lift/harness/web/opfs1/fle_opfs_lib.js + opfs_test.c, built with emscripten -sJSPI -sSHARED_MEMORY) in a dedicated Worker of Chrome or Firefox.
// usage: OPFS_BUILD=<dir with opfs_test.js/.wasm> APP_WORK=<dir (under $HOME for Firefox)> node test/e2e/opfs-lib-test.mjs --browser chrome|firefox [--out FILE.txt]
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
const src = path.resolve(APP, '..', 'lift', 'harness', 'web', 'opfs1');
const build = path.resolve(process.env.OPFS_BUILD ?? path.join(APP, '..', '..', 'build'));
const MIME = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.wasm': 'application/wasm' };
const server = http.createServer((req, res) => {
  const name = path.basename(new URL(req.url, 'http://x').pathname) || 'opfs-test.html';
  const f = [path.join(build, name), path.join(src, name)].find((p) => fs.existsSync(p));
  if (!f) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { 'content-type': MIME[path.extname(f)] ?? 'application/octet-stream', 'cross-origin-opener-policy': 'same-origin', 'cross-origin-embedder-policy': 'require-corp', 'cross-origin-resource-policy': 'same-origin' });
  res.end(fs.readFileSync(f));
});
const port = await freePort();
await new Promise((r) => server.listen(port, '127.0.0.1', r));
const url = `http://127.0.0.1:${port}/opfs-test.html`;
const work = workDirOf('opfs-lib-test');
let ev, goto, close, version;
if (browserName === 'chrome') {
  const b = await openBrowser(work, 'lib');
  const p = await b.newPage({ width: 800, height: 600 });
  ev = p.ev; goto = p.goto; version = 'Chrome ' + b.version; close = async () => { await b.close(); };
} else {
  const { openFirefox } = await import('./firefox-driver.mjs');
  const ff = await openFirefox({ work, downloadDir: path.join(work, 'dl'), dataDir: work });
  ev = ff.ev; goto = ff.goto; version = 'Firefox ' + ff.version; close = async () => { await ff.close(); };
}
const out = [`# ${version}; ${os.cpus().length} cores; load ${os.loadavg()[0].toFixed(1)}; ${new Date().toISOString()}`];
let failed = 0;
const show = (lines) => { for (const l of lines) { out.push(l); if (/^T FAIL/.test(l) || /^E /.test(l)) failed++; } };
await goto(url);
await ev('new Promise((r) => { const t = setInterval(() => { if (window.run) { clearInterval(t); r(true); } }, 50); })');
await ev('window.wipe()');
show((await ev("window.run('all', 240000)")).lines);
await ev("window.wipe().then(() => true)");
// persistence: a Worker that wrote and was terminated without closing; then a new page load reads
const w = await ev("window.run('persist-write', 60000)");
show(w.lines);
await ev('window.kill()');
await goto(url);
await ev('new Promise((r) => { const t = setInterval(() => { if (window.run) { clearInterval(t); r(true); } }, 50); })');
show((await ev("window.run('persist-read', 60000)")).lines);
await ev('window.wipe()');
await close();
server.close();
const text = out.join('\n') + '\n';
fs.writeFileSync(opt('out', path.join(work, `opfs-lib-${browserName}.txt`)), text);
console.log(text);
console.log(failed ? `${failed} FAILED` : 'all checks passed');
process.exit(failed ? 1 : 0);
