// Chrome 156 (headless) check of the plugin Worker bridge (decision 0173, S1): the REAL lifted OTT VST3 in plugin Workers behind the SharedArrayBuffer protocol (app/src/plugins/*).
//   node test/e2e/plugin-bridge.mjs [--startup]
// Needs (not in git): MT1_OTT_DIR (default <repo>/lift/out-mt1/data) with vst-ext.wasm, ott.vst3, seed.tsv - see lift/harness/web/mt1/plugin-worker/build.sh. Chrome: CHROME_BIN or /usr/bin/google-chrome.
// Serves app/src/plugins/*.ts with the types stripped (node:module stripTypeScriptTypes) over COOP/COEP; every page is a throw-away Chrome profile on a localhost-only server.
// Checks: out_hash of the four scenarios of decision 0164, four instances at once, 150 blocks paced at 92.88 ms with no late block, and (--startup) the first-block cost cold / after a warm-up instance / with pre-roll.
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { stripTypeScriptTypes } from 'node:module';
import { fileURLToPath } from 'node:url';
import { Cdp, shutdown, sleep } from '../../../lift/harness/web/cdp.mjs';
import { launch } from '../../../lift/harness/web/mt1/lib.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const APP = path.resolve(HERE, '../..');
const REPO = path.resolve(APP, '..');
const DATA = process.env.MT1_OTT_DIR ?? path.join(REPO, 'lift/out-mt1/data');
const startup = process.argv.includes('--startup');
const series = process.argv.includes('--series');
for (const f of ['vst-ext.wasm', 'ott.vst3', 'seed.tsv']) if (!fs.existsSync(path.join(DATA, f))) { console.log(`SKIP: ${f} not found in ${DATA}`); process.exit(0); }

const PAGE = `<!doctype html><meta charset=utf-8><title>plugin bridge</title><script type=module src="/coi/page.js"></script>`;
const PAGE_JS = fs.readFileSync(path.join(HERE, 'plugin-bridge-page.js'), 'utf8');

function startServer() {
  const server = http.createServer((req, res) => {
    const u = new URL(req.url, 'http://x');
    const p = decodeURIComponent(u.pathname);
    const h = { 'Cache-Control': 'no-store', 'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp', 'Cross-Origin-Resource-Policy': 'same-origin' };
    if (!p.startsWith('/coi/') || p.includes('..')) { res.writeHead(404).end('no'); return; }
    const rel = p.slice(5);
    if (rel === 'index.html') { res.writeHead(200, { ...h, 'Content-Type': 'text/html' }).end(PAGE); return; }
    if (rel === 'page.js') { res.writeHead(200, { ...h, 'Content-Type': 'text/javascript' }).end(PAGE_JS); return; }
    if (rel.startsWith('src/plugins/') && rel.endsWith('.ts')) {
      const f = path.join(APP, rel);
      if (!fs.existsSync(f)) { res.writeHead(404).end('no'); return; }
      res.writeHead(200, { ...h, 'Content-Type': 'text/javascript' }).end(stripTypeScriptTypes(fs.readFileSync(f, 'utf8')));
      return;
    }
    if (rel.startsWith('data/')) {
      const f = path.join(DATA, rel.slice(5));
      if (!fs.existsSync(f)) { res.writeHead(404).end('no'); return; }
      const b = fs.readFileSync(f);
      res.writeHead(200, { ...h, 'Content-Type': rel.endsWith('.wasm') ? 'application/wasm' : 'application/octet-stream', 'Content-Length': b.length }).end(b);
      return;
    }
    res.writeHead(404).end('no');
  });
  return new Promise((r) => server.listen(0, '127.0.0.1', () => r({ server, port: server.address().port })));
}

async function runPage(query) {
  const { server, port } = await startServer();
  const profile = fs.mkdtempSync(path.join(process.env.TMPDIR ?? '/tmp', 'plugin-bridge-'));
  const chrome = await launch({ profile, args: [] });
  const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
  await cdp.opened;
  const logs = [];
  cdp.on((m) => {
    if (m.method === 'Runtime.exceptionThrown') logs.push('exception ' + JSON.stringify(m.params.exceptionDetails).slice(0, 400));
    else if (m.method === 'Target.attachedToTarget' && m.params.targetInfo.type === 'worker') { cdp.send('Runtime.enable', {}, m.params.sessionId).catch(() => {}); cdp.send('Runtime.runIfWaitingForDebugger', {}, m.params.sessionId).catch(() => {}); }
  });
  const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
  const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
  const S = (method, params) => cdp.send(method, params, sessionId);
  await S('Runtime.enable'); await S('Page.enable');
  await S('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true });
  const ev = async (e) => (await S('Runtime.evaluate', { expression: e, returnByValue: true, awaitPromise: true })).result.value;
  await S('Page.navigate', { url: `http://127.0.0.1:${port}/coi/index.html?${query}` });
  let out = null;
  for (let i = 0; i < 1200 && !out; i++) {
    await sleep(250);
    const s = await ev('JSON.stringify({done: window.__done === true, err: window.__error || null, result: window.__result || null})');
    const o = JSON.parse(s);
    if (o.err) { out = { error: o.err }; break; }
    if (o.done) out = o.result;
  }
  out = out ?? { error: 'timeout' };
  out.chrome = chrome.version.Browser; out.loadavg = fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0]; out.logs = logs.slice(0, 5);
  await shutdown(chrome, cdp); server.close(); fs.rmSync(profile, { recursive: true, force: true });
  return out;
}

let failed = false;
const check = (name, ok, extra = '') => { console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}${extra ? '  ' + extra : ''}`); if (!ok) failed = true; };

if (series) {
  for (let rep = 0; rep < 3; rep++) { const r = await runPage('mode=series'); console.log(JSON.stringify({ load: r.loadavg, first10: r.first10, windowMedians20: r.windowMedians20, hash: r.hash, error: r.error })); }
} else if (!startup) {
  const r = await runPage('mode=all');
  if (r.error) { console.log('FAIL page error:', r.error, JSON.stringify(r.logs)); process.exit(1); }
  console.log(`# ${r.chrome}, load ${r.loadavg}, hardwareConcurrency ${r.hardwareConcurrency}, compile ${r.compileMs.toFixed(0)} ms`);
  const want = { default: '8104f50e938ccf36', scale05: '0c435d2acaa363ed', silence: '96deca0f23b22325', paramFirst: '07f081139e0d6448' };
  for (const [k, v] of Object.entries(want)) check(`out_hash ${k} = ${v}`, r.hashes[k] === v, `got ${r.hashes[k]}`);
  check('four instances at once: every out_hash = 8104f50e938ccf36', r.fanout.hashes.every((h) => h === '8104f50e938ccf36'), r.fanout.hashes.join(','));
  check(`paced x4 (${r.paced.blocks} blocks, ${r.paced.periodMs.toFixed(2)} ms): no late block after the first 5`, r.paced.lateAfterWarm === 0, `late ${r.paced.lateAfterWarm} (incl. first blocks ${r.paced.lateAll}), p50 ${r.paced.p50.toFixed(2)} p99 ${r.paced.p99.toFixed(2)} max ${r.paced.max.toFixed(2)} ms`);
  console.log('# first block ms (instance 1 cold ... 4 warm):', JSON.stringify(r.firstBlocks.map((x) => +x.toFixed(1))));
} else {
  const rows = [];
  for (let rep = 0; rep < 3; rep++) for (const mode of ['cold', 'warm-instance', 'preroll8']) {
    const r = await runPage(`mode=${mode}`);
    if (r.error) { console.log('FAIL', mode, r.error); failed = true; continue; }
    rows.push({ mode, load: r.loadavg, firstBlockMs: +r.firstBlockMs.toFixed(1), secondBlockMs: +r.secondBlockMs.toFixed(1), readyMs: +r.readyMs.toFixed(0), hash: r.hash });
    console.log(JSON.stringify(rows.at(-1)));
  }
}
process.exit(failed ? 1 : 0);
