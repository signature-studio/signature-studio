// End-to-end tests in headless Chrome (isolated browser context, network blocked except 127.0.0.1) against
//   dev     : `vite` (the dev server), FL_ASSETS_DIR serving /assets/ from files
//   preview : `vite preview` of the production build, FL_ASSETS_DIR serving /assets/ from files
//   worker  : the Cloudflare Worker under `wrangler dev --local` (Static Assets + local R2 simulation)
// Groups that need no engine: compositor-synth (the runtime's desktop compositor and the page's presenter on a window table the test fills), default-manifest (what the server says about the one manifest path), perf-endpoint.
// When FL_ENGINE_BUNDLE=<bundle directory made by scripts/build-engine-bundle.sh> is set, the REAL engine is run as well (decision 0137): its manifest is the default manifest of the server (the page has no URL parameters), the page is opened,
// the engine runs in the browser. The bundle is a product-derived build output: it is never in git.
// usage: node test/e2e/run.mjs [--target dev,preview,worker] [--no-build] [--only name,name]
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import zlib from 'node:zlib';
import { fileURLToPath } from 'node:url';
import { run, start, sleep, freePort, wranglerEnv } from '../helpers/proc.mjs';
import { APP, deleteObject, getPerfObject, listLocalKeys, putObject, startWorker, workDirOf } from '../helpers/wrangler.mjs';
import { openBrowser } from './browser.mjs';
import { MANIFEST_REAL, MANIFEST_STANDIN as MANIFEST_FOREIGN } from '../helpers/fixtures.ts';
import { engineDesktopScenarioD, engineEndSessionScenarioD, engineResizeScenarioD } from './engine-desk.mjs';
import { deskSynthScenario } from './desk-synth.mjs';
import { taskbarScenario } from './taskbar.mjs';

const REPO = process.env.APP_REPO_ROOT || process.env.APP0_REPO_ROOT || path.resolve(APP, '..');
const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const targets = opt('target', 'dev,preview,worker').split(',');
const only = opt('only', '') ? opt('only', '').split(',') : null;
const work = workDirOf('e2e');
const logsDir = process.env.APP_LOGS || process.env.APP0_LOGS || path.join(work, 'logs');
fs.mkdirSync(logsDir, { recursive: true });
const bin = (n) => path.join(APP, 'node_modules/.bin', n);
const sha256 = (b) => crypto.createHash('sha256').update(b).digest('hex');

const results = [];
let currentGroup = '';
function check(name, ok, detail = '') {
  results.push({ group: currentGroup, name, ok: !!ok, detail });
  console.log(`${ok ? 'PASS' : 'FAIL'}  [${currentGroup}] ${name}${detail ? '  -- ' + detail : ''}`);
}

// ---- the real engine bundle (FL_ENGINE_BUNDLE): linked into the assets directory under its own prefix; its manifest is the default manifest (/assets/manifest.json); manifest variants for the error scenarios ----
const ENGINE_BUNDLE = process.env.FL_ENGINE_BUNDLE ? path.resolve(process.env.FL_ENGINE_BUNDLE) : null;
const ENGINE_PREFIX = ENGINE_BUNDLE ? path.basename(ENGINE_BUNDLE) : null;
const engineManifestText = () => fs.readFileSync(path.join(ENGINE_BUNDLE, 'manifest.json'), 'utf8');
let engineVariants = {};
function linkEngineBundle(assetsDir) {
  if (!ENGINE_BUNDLE) return;
  fs.symlinkSync(ENGINE_BUNDLE, path.join(assetsDir, ENGINE_PREFIX), 'dir');
  const m = JSON.parse(engineManifestText());
  const dir = path.join(assetsDir, 'engine-variants');
  fs.mkdirSync(dir, { recursive: true });
  const variant = (name, f) => { const c = structuredClone(m); f(c); engineVariants[name] = JSON.stringify(c); };
  variant('badsha-wasm', (c) => { c.engine.wasm.sha256 = '0'.repeat(64); });
  variant('badsha-seed', (c) => { c.seedPack.sha256 = '1'.repeat(64); });
  variant('badsha-glue', (c) => { c.engine.options.glue.sha256 = '2'.repeat(64); });
  variant('badsha-dll', (c) => { c.dllPack.sha256 = '3'.repeat(64); });
  variant('missing-dll', (c) => { c.dllPack.url = c.dllPack.url.replace('dll.', 'nodll.'); });
  // a glue that fails the way a start-up fails (SIMULATED: the engine is not run; the error is the text V8 throws when it cannot allocate the module's memory) and one that is not a glue at all
  const glues = { oom: 'export default async function createModule() { throw new RangeError("WebAssembly.Instance(): Out of memory: Cannot allocate Wasm memory for new instance"); }\n', nodefault: 'export const notTheGlue = 1;\n' };
  for (const [k, text] of Object.entries(glues)) {
    const bytes = Buffer.from(text);
    fs.writeFileSync(path.join(dir, `glue-${k}.js`), bytes);
    variant(`glue-${k}`, (c) => { c.engine.options.glue = { url: '/assets/engine-variants/glue-' + k + '.js', size: bytes.length, sha256: sha256(bytes), compression: 'identity' }; });
  }
}

// upload every file of a bundle directory to the local R2 the way the printed upload-commands.txt would (the .br file becomes the object with Content-Encoding br)
async function uploadToLocalR2(wdir, assetsDir) {
  for (const b of fs.readdirSync(assetsDir)) {
    const dir = path.join(assetsDir, b);
    for (const f of fs.readdirSync(dir)) {
      const file = path.join(dir, f);
      const key = `${b}/${f}`;
      if (f === 'SHA256SUMS' || f === 'upload-commands.txt') continue;
      if (f.endsWith('.json')) await putObject(wdir, key, file, { contentType: 'application/json', cacheControl: 'no-cache' });
      else if (f.endsWith('.br')) {
        await putObject(wdir, key.slice(0, -3), file, { contentType: f.includes('.wasm') ? 'application/wasm' : f.includes('.js') ? 'text/javascript' : 'application/octet-stream', contentEncoding: 'br', cacheControl: 'public, max-age=31536000, immutable' });
        if (b !== ENGINE_PREFIX) await putObject(wdir, key, file, { contentType: 'application/octet-stream' }); // the raw stored bytes, for the DecompressionStream test
      } else if (f.endsWith('.gz')) await putObject(wdir, key, file, { contentType: 'application/gzip' });
      else if (!fs.existsSync(file + '.br')) await putObject(wdir, key, file, { contentType: f.endsWith('.wasm') ? 'application/wasm' : f.endsWith('.js') ? 'text/javascript' : 'application/octet-stream' });
    }
  }
}

const appExpr = {
  state: 'window.__app && window.__app.state',
  done: "window.__app && (window.__app.state === 'exited' || window.__app.state === 'error')",
};

async function openApp(browser, base, query, viewport) {
  const page = await browser.newPage(viewport);
  await page.goto(`${base}/${query}`);
  return page;
}

async function runnerScenarios(browser, base, target, assetsDir, ctl) {
  const want = (n) => !only || only.includes(n);

  // 1c'. (decision 0223) the desktop compositor of the runtime and the page's presenter, in Chrome, on a window table the test fills: the layered-window probes of group 1c (same scene, same numbers), a window's update reaching the screen and nothing else, z-order, visibility, position, a display change, a lost canvas
  if (want('compositor-synth') && target === targets[0]) {
    currentGroup = `${target}/compositor-synth`;
    await deskSynthScenario({ browser, check, work });
  }

  // 1d. (decision 0272) the restore button of a minimized window, without the engine: hidden until the engine reports a window, one button, no room taken, keyboard and mouse, gone again
  if (want('taskbar-restore')) {
    currentGroup = `${target}/taskbar-restore`;
    await taskbarScenario({ browser, base, check });
  }

  // 3b. (decision 0152) the default manifest path is for the real engine only: what the server says (404 / 409 / 200, and the ways around), and what the page shows (the page has no URL parameters: it always asks for this path)
  if (want('default-manifest')) {
    const REFUSED = 'このパスは実エンジンのバンドル専用です', NOT_THERE = '実エンジンのバンドルが未配置です';
    const pageCase = async (name, expectErr, extra) => {
      currentGroup = `${target}/default-manifest-${name}`;
      const page = await openApp(browser, base, '');
      try {
        await page.waitFor(appExpr.done, 60000, 'app done');
        const st = await page.ev('window.__app.state');
        const err = await page.ev('window.__app.error');
        const shown = await page.ev("(() => { const e = document.getElementById('error'); return { hidden: e.hidden, text: e.textContent }; })()");
        const b = await page.ev("JSON.stringify({ kind: window.__app.engineKind, manifest: window.__app.manifest, canvases: document.querySelectorAll('canvas').length, engineState: document.getElementById('engine-state').dataset.state })").then(JSON.parse);
        check('state is "error" (the page does not switch to anything else)', st === 'error', `state=${st}`);
        check(`the error names the server's words: ${expectErr}`, err && err.what.includes(expectErr), JSON.stringify(err)?.slice(0, 500));
        check('the error box in the page shows the server\'s explanation as it is (visible, same text)', shown.hidden === false && shown.text.includes(expectErr) && (err ? shown.text.includes(err.what) : false), shown.text.slice(0, 300));
        check('the error says where: the manifest stage and the default URL', err && /^manifest /.test(err.where) && err.where.includes('/assets/manifest.json'), JSON.stringify(err));
        // decision 0211: the engine line is 'not-started' until the page's 250 ms status render runs once after the error, and 'error' ('Engine: failed') from then on; which one this reads is a race between that render and this evaluate (unmodified HEAD fails it at load 43-54 too), and both say that no engine ran
        check('no engine, no manifest, no canvas: nothing was substituted', b.kind === null && b.manifest === null && b.canvases === 0 && ['not-started', 'error'].includes(b.engineState), JSON.stringify(b));
        const assets = [...page.responses.keys()].filter((u) => u.includes('/assets/'));
        check('the only /assets/ request was the default manifest (no wasm, no pack was fetched)', assets.length === 1 && assets[0].endsWith('/assets/manifest.json'), assets.join(' '));
        // decision 0152: the failure itself is reported by the measurement beacon (reason "error"), with the server's words and no bundle
        await page.waitFor('window.__app.perf.history.length >= 1', 20000, 'the error beacon was answered');
        const hist = await page.ev('window.__app.perf.history');
        check('the page sent one beacon, reason "error", and the server answered 2xx', hist.length === 1 && hist[0].reason === 'error' && hist[0].result.ok === true, JSON.stringify(hist).slice(0, 300));
        if (ctl.perfLog) {
          const recs = await waitPerfLog(ctl.perfLog, (r) => r.some((x) => x.perf?.reason === 'error' && x.perf?.error?.what?.includes(expectErr)));
          const rec = recs.find((x) => x.perf?.reason === 'error' && x.perf?.error?.what?.includes(expectErr));
          check('the log holds it: error text of the server, engineKind null, no bundle, firstIdleSeconds null', rec && rec.perf.bundle.engineKind === null && rec.perf.bundle.name === null && rec.perf.startup.firstIdleSeconds === null && rec.perf.engine.state === 'error', JSON.stringify(rec?.perf?.bundle));
        }
        // decision 0257: the small report of the failure is kept too, apart from the big beacon: the kind, the server's words, the capability booleans, the browser's facts, no bundle, and nothing that names the connection
        const wantFailure = (p) => p?.reason === 'failure' && typeof p.failure?.message === 'string' && p.failure.message.includes(expectErr);
        const failureRec = ctl.perfLog ? (await waitPerfLog(ctl.perfLog, (r) => r.some((x) => wantFailure(x.perf)))).find((x) => wantFailure(x.perf))?.perf
          : ctl.workerPerf ? await waitWorkerPerf(ctl.workerPerf, wantFailure) : null;
        check('the log holds the small failure report: reason "failure", kind "load-failed", the server\'s words, stage and place', !!failureRec && failureRec.v === 3 && failureRec.failure.kind === 'load-failed' && /^manifest /.test(failureRec.failure.where) && failureRec.failure.elapsedMs > 0, JSON.stringify(failureRec?.failure)?.slice(0, 400));
        check('it carries the capability booleans (SharedArrayBuffer, JSPI, WebGL2, OPFS ... each true, false or null) and the browser facts, and no bundle', !!failureRec && Object.keys(failureRec.failure.capabilities).length === 11 && Object.values(failureRec.failure.capabilities).every((v) => v === null || typeof v === 'boolean') && failureRec.failure.capabilities.sharedArrayBuffer === true && failureRec.failure.capabilities.jspi === true && typeof failureRec.browser?.userAgent === 'string' && failureRec.bundle.name === null, JSON.stringify(failureRec?.failure?.capabilities));
        check('it names no connection: no address, host name or URL of the test server anywhere in it (the class of the host, "localhost", is the one fact about it that every beacon has)', !!failureRec && !/127\.0\.0\.1|localhost(?!")|https?:\/\//.test(JSON.stringify(failureRec)), JSON.stringify(failureRec)?.slice(0, 200));
        if (extra) await extra(page, err);
      } catch (e) { check('scenario ran to completion', false, String(e.message ?? e)); }
      await page.close();
    };
    try {
      await ctl.setDefaultManifest(null);
      await pageCase('none', NOT_THERE, async (page) => { check('the manifest response was 404', [...page.responses].find(([u]) => u.endsWith('/assets/manifest.json'))?.[1].status === 404); });
      await ctl.setDefaultManifest(MANIFEST_FOREIGN);
      await pageCase('foreign-kind', REFUSED, async (page, err) => {
        check('the manifest response was 409', [...page.responses].find(([u]) => u.endsWith('/assets/manifest.json'))?.[1].status === 409);
        check('the message names the kind and says why', err && err.what.includes('surface-table-v1') && err.what.includes('docs/policies/progress-integrity.md'), err?.what.slice(0, 400));
      });
      // HTTP level, on this target's own server: the refusal and the ways around it
      for (const [label, p] of [['plain', '/assets/manifest.json'], ['query', '/assets/manifest.json?x=1'], ['double slash', '/assets//manifest.json'], ['dot segment', '/assets/./manifest.json'], ['%2e segment', '/assets/%2e/manifest.json'], ['parent segment', '/assets/x/../manifest.json'], ['%2e%2e', '/assets/x/%2e%2e/manifest.json'], ['encoded letter', '/assets/%6danifest.json'], ['encoded dot', '/assets/manifest%2ejson']]) {
        currentGroup = `${target}/default-manifest-http`;
        const r = await rawGetPath(base, p);
        const body = r.body.toString();
        check(`a manifest of a foreign kind at the default path, ${label} (${p}): not 200, and not that manifest`, r.status !== 200 && !body.includes('"schema"') && (r.status !== 409 || body.includes(REFUSED)), `${r.status} ${body.slice(0, 80).replace(/\n/g, ' ')}`);
      }
      const plain = await rawGetPath(base, '/assets/manifest.json');
      currentGroup = `${target}/default-manifest-http`;
      check('a manifest of a foreign kind at the default path: 409 with the PM\'s words, text/plain', plain.status === 409 && String(plain.headers['content-type']).startsWith('text/plain') && plain.body.toString().includes(REFUSED) && plain.body.toString().includes('surface-table-v1'), `${plain.status} ${plain.headers['content-type']}`);
      // the real engine's kind passes the server and reaches the page (which then judges the manifest against its schema: this one is a bare kind, so the page says so)
      await ctl.setDefaultManifest(MANIFEST_REAL);
      const ok = await rawGetPath(base, '/assets/manifest.json');
      currentGroup = `${target}/default-manifest-http`;
      check('real-engine manifest (emscripten-fruity-v1) at the default path: 200 and the stored bytes', ok.status === 200 && ok.body.toString() === MANIFEST_REAL, `${ok.status}`);
      const okq = await rawGetPath(base, '/assets/manifest.json?x=1');
      check('real-engine manifest with a query string: 200', okq.status === 200);
      currentGroup = `${target}/default-manifest-real`;
      const page = await openApp(browser, base, '');
      try {
        await page.waitFor(appExpr.done, 60000, 'app done');
        const err = await page.ev('window.__app.error');
        const resp = [...page.responses].find(([u]) => u.endsWith('/assets/manifest.json'))?.[1];
        check('the real-engine kind is let through by the server (200) and judged by the page\'s schema (not a 409)', resp?.status === 200 && err && err.what.includes('does not match schema 1') && !err.what.includes('このパスは'), JSON.stringify({ status: resp?.status, err }).slice(0, 300));
      } catch (e) { check('scenario ran to completion', false, String(e.message ?? e)); }
      await page.close();
    } catch (e) { currentGroup = `${target}/default-manifest`; check('scenario ran to completion', false, String(e.stack ?? e)); }
    finally { await ctl.setDefaultManifest(null).catch(() => undefined); }
  }

  // the measurement endpoint (decision 0152, decision 0224): what it refuses and what it keeps
  if (want('perf-endpoint')) {
  // the endpoint on this target: what it refuses
  currentGroup = `${target}/perf-endpoint`;
  const logFile = ctl.perfLog;
  const good = JSON.stringify({ v: 1, probe: 'e2e' });
  const g = await postPerf(base, good);
  check('POST application/json: 204', g.status === 204, `${g.status}`);
  check('a text/plain POST is 415', (await postPerf(base, good, { 'content-type': 'text/plain' })).status === 415);
  check('a body over 16 KB is 413', (await postPerf(base, JSON.stringify({ p: 'x'.repeat(17000) }))).status === 413);
  check('a body that is not one JSON object is 400', (await postPerf(base, '[1,2]')).status === 400 && (await postPerf(base, 'nope')).status === 400);
  check('another origin is 403', (await postPerf(base, good, { origin: 'http://evil.example' })).status === 403 && (await postPerf(base, good, { 'sec-fetch-site': 'cross-site' })).status === 403);
  const gt = await rawGetPath(base, '/__perf');
  check('GET is 405', gt.status === 405, `${gt.status}`);
  if (logFile) {
    const recs = readPerfLog(logFile).filter((r) => r.perf?.probe === 'e2e');
    check('of those requests exactly the one accepted POST was stored', recs.length === 1, `${recs.length}`);
    // decision 0192: behind `tailscale serve` (a reverse proxy on this machine) the socket peer is 127.0.0.1 and X-Forwarded-For carries the visitor
    const px = await postPerf(base, JSON.stringify({ v: 2, probe: 'e2e-xff' }), { 'x-forwarded-for': '100.101.102.103', 'x-forwarded-proto': 'https' });
    const pr = readPerfLog(logFile).find((r) => r.perf?.probe === 'e2e-xff');
    check('behind a reverse proxy on this machine: X-Forwarded-For 100.x is recorded as "tailnet" (a visitor over the tailnet), and that address is nowhere in the log', px.status === 204 && pr?.remote === 'tailnet' && !fs.readFileSync(logFile, 'utf8').includes('100.101.102.103'), `${px.status} ${pr?.remote}`);
  }
  if (ctl.workerPerf) {
    const probeKeys = async (name) => { const out = []; for (const o of ctl.workerPerf.keys()) { const t = await ctl.workerPerf.get(o.key); if (t && JSON.parse(t).perf?.probe === name) out.push({ key: o.key, text: t }); } return out; };
    const stored = await probeKeys('e2e');
    check('of those requests exactly the one accepted POST was stored (an object in the Worker\'s R2; the refused ones left none)', stored.length === 1, `${stored.length}`);
    const px = await postPerf(base, JSON.stringify({ v: 2, probe: 'e2e-xff' }), { 'x-forwarded-for': '100.101.102.103', 'cf-connecting-ip': '203.0.113.5', 'x-forwarded-proto': 'https' });
    const pr = (await probeKeys('e2e-xff'))[0];
    check('behind Cloudflare: the visitor\'s address (CF-Connecting-IP / X-Forwarded-For) is stored nowhere; the record only says remote "workers"', px.status === 204 && px.headers['x-perf-stored'] === 'yes' && pr && JSON.parse(pr.text).remote === 'workers' && !pr.text.includes('100.101.102.103') && !pr.text.includes('203.0.113.5') && /nosid-[0-9a-f]{8}-0\.json$/.test(pr.key), `${px.status} ${pr?.key}`);
  }
  }
}


// a GET that does NOT decode Content-Encoding (fetch() in node does), so that the bytes on the wire can be compared with the stored ones
function rawGet(url, headers) {
  return new Promise((resolve, reject) => {
    http.get(url, { headers }, (res) => {
      const chunks = [];
      res.on('data', (d) => chunks.push(d));
      res.on('end', () => resolve({ status: res.statusCode, headers: res.headers, body: Buffer.concat(chunks) }));
    }).on('error', reject);
  });
}

// the request line exactly as given (node's URL class would resolve `.` / `..` / `%2e` before the server saw them)
function rawGetPath(base, rawPath, headers = {}) {
  return new Promise((resolve, reject) => {
    const u = new URL(base);
    http.get({ host: u.hostname, port: u.port, path: rawPath, headers: { 'accept-encoding': 'identity', ...headers } }, (res) => {
      const chunks = [];
      res.on('data', (d) => chunks.push(d));
      res.on('end', () => resolve({ status: res.statusCode, headers: res.headers, body: Buffer.concat(chunks) }));
    }).on('error', reject);
  });
}


// ---- decision 0152: the measurement beacon's log (dev / preview write a JSONL file; the Worker stores nothing) ----
const readPerfLog = (file) => (file && fs.existsSync(file) ? fs.readFileSync(file, 'utf8').split('\n').filter(Boolean).flatMap((l, i, a) => { try { return [JSON.parse(l)]; } catch (e) { if (i === a.length - 1) return []; throw e; } }) : []); // a last line still being appended is not a record yet (decision 0192)
async function waitWorkerPerf(w, pred, ms = 20000) {
  const t0 = Date.now();
  for (;;) {
    for (const o of w.keys()) {
      const t = await w.get(o.key);
      try { const p = JSON.parse(t).perf; if (pred(p)) return p; } catch { /* not a beacon */ }
    }
    if (Date.now() - t0 > ms) return null;
    await sleep(200);
  }
}
async function waitPerfLog(file, pred, ms = 20000) {
  const t0 = Date.now();
  for (;;) {
    const recs = readPerfLog(file);
    if (pred(recs)) return recs;
    if (Date.now() - t0 > ms) return recs;
    await sleep(200);
  }
}
function postPerf(base, body, headers = {}) {
  return new Promise((resolve, reject) => {
    const u = new URL(base);
    const req = http.request({ host: u.hostname, port: u.port, path: '/__perf', method: 'POST', agent: false, headers: { 'content-type': 'application/json', ...headers } }, (res) => {
      const chunks = [];
      res.on('data', (d) => chunks.push(d));
      res.on('end', () => resolve({ status: res.statusCode, headers: res.headers, body: Buffer.concat(chunks).toString() }));
    });
    req.on('error', (e) => resolve({ status: 0, headers: {}, body: String(e.message) }));
    req.end(body);
  });
}

// ---- resident memory of Chrome's process tree (sampled; the renderer holds the engine's ~1 GB of wasm memory and the compiled module) ----
function treeRssMB(rootPid) {
  const kids = new Map();
  const rss = new Map();
  for (const d of fs.readdirSync('/proc')) {
    if (!/^\d+$/.test(d)) continue;
    try {
      const stat = fs.readFileSync(`/proc/${d}/stat`, 'utf8');
      const ppid = Number(stat.slice(stat.lastIndexOf(')') + 2).split(' ')[1]);
      (kids.get(ppid) ?? kids.set(ppid, []).get(ppid)).push(Number(d));
      const m = /VmRSS:\s+(\d+) kB/.exec(fs.readFileSync(`/proc/${d}/status`, 'utf8'));
      if (m) rss.set(Number(d), Number(m[1]) / 1024);
    } catch { /* the process went away */ }
  }
  let total = 0, biggest = 0;
  const stack = [rootPid];
  while (stack.length) { const p = stack.pop(); const r = rss.get(p) ?? 0; total += r; biggest = Math.max(biggest, r); stack.push(...(kids.get(p) ?? [])); }
  return { total, biggest };
}

// ---- the real engine in the browser (F1) ----
async function engineScenarios(browser, base, target, ctl) {
  const want = (n) => !only || only.includes(n);
  const exp0129 = JSON.parse(fs.readFileSync(path.join(REPO, 'docs/investigations/wasm-final0129/tools/expected-0129.json'), 'utf8'));
  const errors = [
    ['err-engine-badsha-wasm', 'badsha-wasm', ['SHA-256 mismatch', '0'.repeat(64)], 'verify'],
    ['err-engine-badsha-seed', 'badsha-seed', ['SHA-256 mismatch', '1'.repeat(64)], 'verify'],
    ['err-engine-badsha-glue', 'badsha-glue', ['SHA-256 mismatch', '2'.repeat(64)], 'verify'],
    ['err-engine-badsha-dll', 'badsha-dll', ['SHA-256 mismatch', '3'.repeat(64)], 'verify'],
    ['err-engine-missing-dll', 'missing-dll', ['HTTP 404'], 'fetch'],
    ['err-engine-oom-simulated', 'glue-oom', ['ran out of memory while starting the engine', 'Out of memory: Cannot allocate Wasm memory'], 'engine worker'],
    ['err-engine-glue-not-a-glue', 'glue-nodefault', ['no default export function', '-sEXPORT_ES6=1'], 'engine worker'],
  ];
  for (const [name, mf, texts, where] of errors) {
    if (!want(name)) continue;
    currentGroup = `${target}/${name}`;
    await ctl.setDefaultManifest(engineVariants[mf]);
    const page = await openApp(browser, base, '');
    try {
      await page.waitFor(appExpr.done, 120000);
      const err = await page.ev('window.__app.error');
      check('state is "error"', (await page.ev('window.__app.state')) === 'error');
      check(`error names what: ${texts.join(' + ')}`, err && texts.every((t) => err.what.includes(t)), JSON.stringify(err)?.slice(0, 300));
      check(`error names where: ${where}`, err && err.where.includes(where), JSON.stringify(err)?.slice(0, 300));
      check('the stage list marks a failed stage and none active', await page.ev("(() => { const r = [...document.querySelectorAll('#phases li')].map((l) => l.dataset.status); return r.includes('failed') && !r.includes('active'); })()"));
      if (name.startsWith('err-engine-oom') || name.startsWith('err-engine-glue')) check('the engine had been started when it failed (the failure is the engine\'s)', !!(await page.ev('window.__app.timeline.engine_start')) || name.startsWith('err-engine-glue'));
      else check('no canvas was created and the engine never started', (await page.ev("document.querySelectorAll('canvas').length")) === 0 && !(await page.ev('window.__app.timeline.engine_start')));
    } catch (e) { check('scenario ran to completion', false, String(e.message ?? e)); }
    await page.close();
  }

  // what the server (file server or Worker + R2) returns for the engine's four assets when the client says Accept-Encoding: br: the stored brotli bytes, with Content-Encoding: br; and for Chrome's own list
  if (want('engine-assets-encoding')) {
    currentGroup = `${target}/engine-assets-encoding`;
    try {
      const man = JSON.parse(fs.readFileSync(path.join(ENGINE_BUNDLE, 'manifest.json'), 'utf8'));
      const refs = [['engine wasm', man.engine.wasm], ['engine glue', man.engine.options.glue], ['seed pack', man.seedPack], ['DLL pack', man.dllPack]];
      for (const [label, ref] of refs) {
        const stored = fs.readFileSync(path.join(ENGINE_BUNDLE, path.basename(ref.url) + '.br'));
        const t1 = Date.now();
        const r = await rawGet(base + ref.url, { 'accept-encoding': 'br' });
        const body = r.body;
        const ms = Date.now() - t1;
        check(`${label}: Accept-Encoding br gets the STORED brotli bytes (${stored.length} B) with Content-Encoding br`, r.status === 200 && r.headers['content-encoding'] === 'br' && body.length === stored.length && sha256(body) === sha256(stored), `status ${r.status} enc ${r.headers['content-encoding']} ${body.length} B in ${ms} ms`);
        check(`${label}: the decoded body is the manifest's size and SHA-256`, zlib.brotliDecompressSync(body).length === ref.size && sha256(zlib.brotliDecompressSync(body)) === ref.sha256);
        const c = await rawGet(base + ref.url, { 'accept-encoding': 'gzip, deflate, br, zstd' });
        results.push({ group: currentGroup, name: `record: ${label} with Chrome's Accept-Encoding (gzip, deflate, br, zstd)`, ok: true, detail: `Content-Encoding ${c.headers['content-encoding']}, ${c.body.length} B on the wire` });
      }
    } catch (e) { check('scenario ran to completion', false, String(e.message ?? e)); }
  }

  await ctl.setDefaultManifest(engineManifestText());

  if (want('engine-nojspi')) {
    currentGroup = `${target}/engine-nojspi`;
    const page = await browser.newPage({ initScript: 'delete WebAssembly.Suspending; delete WebAssembly.promising;' });
    try {
      await page.goto(`${base}/`);
      await page.waitFor(appExpr.done, 60000);
      const err = await page.ev('window.__app.error');
      check('a browser without JSPI is told what is missing, before any download', err && /JSPI/.test(err.what) && /WebAssembly\.Suspending and WebAssembly\.promising are missing/.test(err.what) && /before any download/.test(err.where), JSON.stringify(err)?.slice(0, 400));
      const urls = [...page.responses.keys()];
      check('the engine wasm, packs and glue were NOT requested', !urls.some((u) => /\/assets\/.*\/(engine|seed|dll|engine-glue)\.[0-9a-f]{12}\./.test(u)), urls.join(' '));
    } catch (e) { check('scenario ran to completion', false, String(e.message ?? e)); }
    await page.close();
  }

  // the Stop button while the engine is still starting: it cannot be asked to stop (it never returns to its event loop), so the Worker is terminated and the page says so
  if (want('engine-stop-button')) {
    currentGroup = `${target}/engine-stop-button`;
    const page = await openApp(browser, base, '', { width: 2400, height: 1300 });
    try {
      await page.waitFor('window.__app && window.__app.timeline && window.__app.timeline.main_start', 120000, 'main() started');
      check('the Stop button is shown while the engine runs', (await page.ev("document.getElementById('stop-btn').hidden")) === false);
      await page.ev("document.getElementById('stop-btn').click()");
      await page.waitFor("window.__app.state === 'exited'", 30000, 'exited after Stop');
      const ex = await page.ev('window.__app.exit');
      check('the exit says the Worker was terminated by the page (reason "stopped")', ex && ex.reason === 'stopped' && /terminated by the page/.test(ex.message), JSON.stringify(ex));
      const txt = await page.ev("document.getElementById('exit').textContent");
      check('the page says that the engine no longer runs and that no input is answered', /no longer runs/.test(txt) && !(await page.ev('window.__app.timeline.first_pixel')), txt.slice(0, 300));
      check('the Stop button is gone and the stage list shows the engine row done', (await page.ev("document.getElementById('stop-btn').hidden")) === true);
      check('no console errors or exceptions', page.consoleLog.filter((l) => l.startsWith('error') || l.startsWith('EXCEPTION')).length === 0, page.consoleLog.join(' | ').slice(0, 300));
    } catch (e) { check('scenario ran to completion', false, String(e.message ?? e)); }
    await page.close();
  }

  // the real engine on the desktop with real input (decision 0141): start-up, the overlapping windows, a click, a key, the stop
  if (want('engine-desk')) {
    let peak = { total: 0, biggest: 0 };
    const sampler = setInterval(() => { const r = treeRssMB(browser.pid); peak = { total: Math.max(peak.total, r.total), biggest: Math.max(peak.biggest, r.biggest) }; }, 1000);
    const marks = await engineDesktopScenarioD({ browser, base, target, exp0129, check, record: (name, detail) => results.push({ group: currentGroup, name, ok: true, detail }), setGroup: (g) => { currentGroup = g; }, logsDir, appExpr });
    clearInterval(sampler);
    results.push({ group: currentGroup, name: 'record: Chrome process tree RSS peak (MB): total / biggest process', ok: true, detail: `${peak.total.toFixed(0)} / ${peak.biggest.toFixed(0)}` });
    results.push({ group: currentGroup, name: 'record: scenario marks', ok: true, detail: JSON.stringify(marks).slice(0, 4000) });
  }
  if (want('engine-desk-end-session')) await engineEndSessionScenarioD({ browser, base, target, check, record: (name, detail) => results.push({ group: currentGroup, name, ok: true, detail }), setGroup: (g) => { currentGroup = g; } });
  if (want('engine-desk-resize')) await engineResizeScenarioD({ browser, base, target, check, record: (name, detail) => results.push({ group: currentGroup, name, ok: true, detail }), setGroup: (g) => { currentGroup = g; }, logsDir });
}

// ---- main ----
let failed = 0;
const summary = {};
const assetsDir = path.join(work, 'assets');
fs.rmSync(assetsDir, { recursive: true, force: true });
fs.mkdirSync(assetsDir, { recursive: true });
linkEngineBundle(assetsDir);
if (!argv.includes('--no-build') && targets.some((t) => t !== 'dev')) await run(bin('vite'), ['build'], { cwd: APP, env: process.env, label: 'vite build' });

for (const target of targets) {
  const stops = [];
  let base;
  const perfLog = path.join(work, `perf-${target}.jsonl`);
  fs.rmSync(perfLog, { force: true });
  let ctl = { perfLog: target === 'worker' ? null : perfLog, setDefaultManifest: async (body) => { const f = path.join(assetsDir, 'manifest.json'); if (body === null) fs.rmSync(f, { force: true }); else fs.writeFileSync(f, body); } };
  try {
    if (target === 'dev') {
      const port = await freePort();
      const p = start(bin('vite'), ['--host', '127.0.0.1', '--port', String(port), '--strictPort'], { cwd: APP, env: { ...process.env, FL_ASSETS_DIR: assetsDir, FL_PERF_LOG: perfLog }, ready: /Local:\s+http/, label: 'vite dev' });
      stops.push(p.stop);
      await p.ready;
      base = `http://127.0.0.1:${port}`;
    } else if (target === 'preview') {
      const port = await freePort();
      const p = start(bin('vite'), ['preview', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], { cwd: APP, env: { ...process.env, FL_ASSETS_DIR: assetsDir, FL_PERF_LOG: perfLog }, ready: /Local:\s+http/, label: 'vite preview' });
      stops.push(p.stop);
      await p.ready;
      base = `http://127.0.0.1:${port}`;
    } else if (target === 'worker') {
      const wdir = path.join(work, 'worker');
      fs.rmSync(wdir, { recursive: true, force: true });
      fs.mkdirSync(wdir, { recursive: true });
      await uploadToLocalR2(wdir, assetsDir);
      const w = await startWorker(wdir);
      stops.push(w.stop);
      base = w.url;
      // decision 0152: the default manifest of the local R2 is put and taken away while the Worker runs
      ctl = { perfLog: null, workerPerf: { keys: () => listLocalKeys(wdir), get: (k) => getPerfObject(wdir, k) }, setDefaultManifest: async (body) => { if (body === null) { await deleteObject(wdir, 'manifest.json').catch(() => undefined); return; } const f = path.join(work, 'default-manifest.json'); fs.writeFileSync(f, body); await putObject(wdir, 'manifest.json', f, { contentType: 'application/json', cacheControl: 'no-cache' }); } };
    } else throw new Error('unknown target ' + target);
    const browser = await openBrowser(work, target);
    console.log(`\n== target ${target} at ${base}; ${browser.version} (V8 ${browser.v8})`);
    try { await runnerScenarios(browser, base, target, assetsDir, ctl); if (ENGINE_BUNDLE) await engineScenarios(browser, base, target, ctl); } finally {
      const nl = await browser.close();
      console.log(nl.line);
      summary[target] = { chrome: browser.version, v8: browser.v8, netlog: nl.line, nonLoopbackTcp: nl.nonLoopbackTcp, nonLoopbackUdpConnects: nl.nonLoopbackUdpConnects, udpBytesSentEvents: nl.udpBytesSentEvents };
      check(`[${target}] net-log: TCP connections to non-loopback addresses = 0`, nl.nonLoopbackTcp === 0, String(nl.nonLoopbackTcp));
      check(`[${target}] net-log: UDP packets sent (UDP_BYTES_SENT events) = 0`, nl.udpBytesSentEvents === 0, String(nl.udpBytesSentEvents));
      fs.copyFileSync(nl.netlogFile, path.join(work, `netlog-${target}.keep.json`));
    }
  } catch (e) {
    check(`[${target}] harness`, false, String(e.stack ?? e));
  } finally {
    for (const s of stops) await s();
  }
}
failed = results.filter((r) => !r.ok).length;
const total = results.filter((r) => !r.name.startsWith('record:')).length;
fs.writeFileSync(path.join(logsDir, 'e2e-results.json'), JSON.stringify({ results, summary }, null, 2));
console.log(`\ne2e: ${total - failed}/${total} checks passed${failed ? `, ${failed} FAILED` : ''}`);
process.exit(failed ? 1 : 0);
