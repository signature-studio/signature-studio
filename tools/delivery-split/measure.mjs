// Delivery split of the start-up (Issue #1314): what the page downloads before the engine starts (the manifest's pre-start assets) and what it downloads after it has started (manifest.lazy),
// and when the engine is idle. One run = one headless Chrome with a new profile, the same launch as tools/boot-check/boot-bundle.mjs, until the engine is idle (or an error / exit).
// Usage: node tools/delivery-split/measure.mjs <absolute path to the bundle directory raw/<name>> [runs, 1..5, default 3] > delivery.jsonl
// stdout: one JSON object per run, on its own line: {"state","standIn","error","elapsedMs","preStart":[{"name","bytes"}],"lazy":[{"name","bytes"}],"preStartDoneMs","lazyStartMs","lazyDoneMs","lazyWaitFirstMs","lazyWaitLast","fetches":[{"name","startMs","endMs"}],"reads":[{"name","atMs","requested"}]}
//   reads (Issue #1341): the engine's reads of a real module's image, from its line `image-read: <file name> requested=<0|1> <ms>` (the page takes the time of the line: atMs, ms since the navigation, the clock of elapsedMs; requested true when the guest asked for the module). Always an array; [] when the engine read none.
//   lazyWaitFirstMs: the time of the first read of a lazy file that had to wait for its bytes (the page mark lazy_wait, ms since the navigation); lazyWaitLast: the last such read (its path and ms). null: no read waited.
//   state: the page's state at the end ('engine-idle' is the good one); standIn: whether the text "STAND-IN" is in the page body (a reading, as in boot-bundle.mjs); error: null, or what failed.
//   preStart / lazy: the manifest's assets, `bytes` = the decoded size (what the page counts). preStartDoneMs: the page's timeline mark download_done (ms since the navigation started).
//   elapsedMs: the page's timeline mark idle (ms since the navigation started: the engine is idle). fetches: the browser's resource timing of the bundle's assets (startTime and responseEnd, ms since the navigation started).
//   lazy fetches that start before elapsedMs are the ones that could delay the idle; the page's marks lazy_start / lazy_done say when the lazy downloads began and ended (null: none).
// Nothing is written outside the repository: the staged assets and the Chrome profile are in a .tmp-delivery-* directory, removed at the end. Diagnostics go to stderr.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';

const bundleArg = process.argv[2];
if (!bundleArg) {
  process.stderr.write('usage: node tools/delivery-split/measure.mjs <absolute path to the bundle directory raw/<name>> [runs]\n');
  process.exit(2);
}
const RUNS = process.argv[3] === undefined ? 3 : Number(process.argv[3]);
if (!Number.isInteger(RUNS) || RUNS < 1 || RUNS > 5) {
  process.stderr.write('runs must be an integer from 1 to 5\n');
  process.exit(2);
}
const bundleDir = path.resolve(bundleArg);
const BUNDLE = path.basename(bundleDir);
const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const WAIT_LIMIT_MS = 180000;
const POLL_MS = 1000;
const CDP_TIMEOUT_MS = 20000;

// the isolation flags of tools/boot-check/boot-bundle.mjs
const ISOLATION = [
  '--password-store=basic',
  '--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE 127.0.0.1',
  '--no-proxy-server', '--disable-ipv6', '--disable-quic',
  '--disable-background-networking', '--disable-component-update', '--disable-sync', '--disable-default-apps', '--disable-domain-reliability',
  '--no-pings', '--metrics-recording-only', '--disable-client-side-phishing-detection', '--disable-breakpad', '--disable-extensions', '--safebrowsing-disable-auto-update', '--disable-search-engine-choice-screen', '--no-service-autorun',
  '--disable-features=OptimizationHints,MediaRouter,Translate,CertificateTransparencyComponentUpdater,NetworkTimeServiceQuerying,AutofillServerCommunication,OptimizationGuideModelDownloading,SafeBrowsingEnhancedProtection,PrivacySandboxSettings4,Variations,SafeBrowsing,SafeBrowsingHashPrefixRealTimeLookups,HashPrefixRealTimeLookups,SafeBrowsingOnUIThread,SafeBrowsingNewGmsApiForBrowseUrlDatabaseCheck',
  '--variations-server-url=http://127.0.0.1:9/', '--disable-field-trial-config',
  '--force-color-profile=srgb', '--disable-lcd-text',
];

// read in the page: the state, the manifest, the marks of the timeline, and the resource timing of every request made so far
const SNAPSHOT = `(() => { const a = window.__app ?? null; return {
  state: a ? a.state : null,
  details: a ? a.details : {},
  manifest: a ? a.manifest : null,
  timeline: a ? a.timeline : {},
  error: a && a.error ? a.error : null,
  standIn: document.body.innerText.includes('STAND-IN'),
  resources: performance.getEntriesByType('resource').map((e) => ({ name: e.name, startTime: e.startTime, responseEnd: e.responseEnd })),
  imageReads: a ? a.imageReads : [],
}; })()`;

const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};

const num = (v) => (typeof v === 'number' && Number.isFinite(v) ? Math.round(v * 10) / 10 : null);

/** the manifest's assets: the pre-start ones under the labels of the page (main.ts), and the lazy ones under their names */
function assetsOf(manifest) {
  if (!manifest) return { preStart: [], lazy: [], byUrl: new Map() };
  const pre = [
    { name: 'engine wasm', ref: manifest.engine.wasm },
    { name: 'seed pack', ref: manifest.seedPack },
    { name: 'DLL pack', ref: manifest.dllPack },
    { name: 'engine glue', ref: manifest.engine.options.glue },
  ];
  const lazy = (manifest.lazy ?? []).map((l) => ({ name: l.name, ref: l }));
  const byUrl = new Map();
  for (const a of [...pre, ...lazy]) byUrl.set(a.ref.url, a.name);
  return {
    preStart: pre.map((a) => ({ name: a.name, bytes: a.ref.size })),
    lazy: lazy.map((a) => ({ name: a.name, bytes: a.ref.size })),
    byUrl,
  };
}

/** the browser's resource timing of the bundle's assets: the URL's path is the manifest's url */
function fetchesOf(resources, byUrl) {
  const out = [];
  for (const r of resources ?? []) {
    let p;
    try { p = new URL(r.name).pathname; } catch { continue; }
    const hit = [...byUrl.entries()].find(([u]) => new URL(u, 'http://x/').pathname === p);
    if (!hit) continue;
    out.push({ name: hit[1], startMs: num(r.startTime), endMs: num(r.responseEnd) });
  }
  return out;
}

async function measureOnce(port) {
  const t0 = Date.now();
  const out = { state: null, standIn: null, error: null, elapsedMs: null, preStart: [], lazy: [], preStartDoneMs: null, lazyStartMs: null, lazyDoneMs: null, lazyWaitFirstMs: null, lazyWaitLast: null, fetches: [], reads: [] };
  const root = fs.mkdtempSync(path.join(repo, '.tmp-delivery-run-'));
  let chrome = null, cdp = null;
  try {
    chrome = await launchChrome({ profile: path.join(root, 'profile'), extraArgs: ISOLATION });
    cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
    await cdp.opened;
    const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const S = (method, params) => timed(cdp.send(method, params, sessionId), CDP_TIMEOUT_MS, method);
    const ev = async (expression) => {
      const r = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.text}`);
      return r.result.value;
    };
    await S('Page.enable');
    const nav = await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
    const navAt = Date.now();
    let snap = null;
    for (;;) {
      snap = await ev(SNAPSHOT).catch(() => null);
      if (snap && (snap.state === 'engine-idle' || snap.state === 'error' || snap.state === 'exited')) break;
      if (Date.now() - navAt >= WAIT_LIMIT_MS) break;
      await sleep(POLL_MS);
    }
    const tl = snap?.timeline ?? {};
    const a = assetsOf(snap?.manifest ?? null);
    out.state = snap?.state ?? null;
    out.standIn = snap ? snap.standIn : null;
    out.preStart = a.preStart;
    out.lazy = a.lazy;
    out.preStartDoneMs = num(tl.download_done);
    out.lazyStartMs = num(tl.lazy_start);
    out.lazyDoneMs = num(tl.lazy_done);
    out.lazyWaitFirstMs = num(tl.lazy_wait);
    out.lazyWaitLast = snap?.details?.lazy_wait ?? null;
    out.elapsedMs = num(tl.idle);
    out.fetches = fetchesOf(snap?.resources ?? [], a.byUrl);
    out.reads = (snap?.imageReads ?? []).map((r) => ({ name: r.name, atMs: num(r.atMs), requested: r.requested === true }));
    if (!snap) out.error = `window.__app not readable within ${WAIT_LIMIT_MS} ms`;
    else if (snap.error) out.error = `${snap.error.what} [${snap.error.where}]`;
    else if (snap.state !== 'engine-idle') out.error = `state ${snap.state} at the end (no engine-idle within ${WAIT_LIMIT_MS} ms)`;
    if (snap?.state === 'engine-idle' && out.elapsedMs === null) out.error = 'the engine is idle but the page has no idle mark';
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
  } finally {
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
    fs.rmSync(root, { recursive: true, force: true });
  }
  process.stderr.write(`run: state ${out.state}, elapsedMs ${out.elapsedMs}, ${Date.now() - t0} ms in all\n`);
  return out;
}

async function main() {
  const top = fs.mkdtempSync(path.join(repo, '.tmp-delivery-'));
  const assets = path.join(top, 'assets');
  let server = null;
  try {
    if (!fs.existsSync(brDir)) throw new Error(`no ${brDir}: the brotli directory of the bundle must be next to raw/`);
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    for (let i = 0; i < RUNS; i++) {
      const r = await measureOnce(port);
      process.stdout.write(JSON.stringify(r) + '\n');
    }
  } finally {
    if (server) await server.close().catch(() => {});
    fs.rmSync(top, { recursive: true, force: true });
    process.stderr.write(`loadavg ${os.loadavg().map((x) => x.toFixed(2)).join(' ')}, nproc ${os.cpus().length}\n`);
  }
}

await main();
