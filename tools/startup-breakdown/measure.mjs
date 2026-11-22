// Start-up breakdown (Issue #1329): runs the bundle three times in headless Chrome (a new profile each time, as tools/boot-check/boot-bundle.mjs does) and prints one JSON line per run on stdout.
// P1 is counted from the navigation to the first engine-idle mark: the start is the time just before Page.navigate, the end is the Worker's first 'idle' mark (window.__app.timeline.idle).
// The end is a mark, not the poll that sees the state, so the poll's interval does not count (the poll's reading is only on stderr).
//   navToIdleMs   performance.timeOrigin + timeline.idle − the start (on the clock of Date.now()).
//   timelineIdle  timeline.idle, counted from the page's navigation start (the value boot-bundle.mjs reports as timelineIdle).
// phases (ms, null when not available):
//   fetchWasm     engine wasm: its fetch, responseEnd − startTime (resource timing).
//   compileWasm   engine wasm: compile_done − its responseEnd. The tail after the last byte: the SHA-256 check and the rest of the compile (compileStreaming runs during the download).
//   fetchPacks    seed pack and DLL pack together: the later responseEnd − the earlier startTime.
//   instantiate   wasm_instantiated − engine_start: the Worker's WebAssembly.instantiate (fruity.ts), from the page's start of the engine.
//   idle          idle − wasm_instantiated: the guest's start-up up to its first idle.
// stderr (not the JSON): one line per run with the page's marks, the poll's reading, the wasm's split from the assets report, the available parallelism and the loadavg.
// Usage: node tools/startup-breakdown/measure.mjs <absolute path to the bundle directory raw/<name>> > startup.jsonl   (needs app/node_modules and app/dist).
// The temporary directory is inside the repository (.tmp-startup-*) and is removed. Exit 1 when a run failed ("error" set), 2 when no bundle directory is given.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';

if (!process.argv[2]) {
  process.stderr.write('usage: node tools/startup-breakdown/measure.mjs <absolute path to the bundle directory>\n');
  process.exit(2);
}
const RUNS = 3;
const PHASES = ['fetchWasm', 'compileWasm', 'fetchPacks', 'instantiate', 'idle'];
const bundleDir = path.resolve(process.argv[2]);
const BUNDLE = path.basename(bundleDir);
const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const WAIT_LIMIT_MS = 180000;
const POLL_MS = 1000;
const CDP_TIMEOUT_MS = 20000;

// the isolation flags of tools/boot-check/boot-bundle.mjs (the same browser, so that the numbers compare with its runs)
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

const POLL = `(() => { const a = window.__app; return a ? { state: a.state } : null; })()`;
const READ = `(() => {
  const a = window.__app ?? null;
  const m = a && a.manifest ? a.manifest : null;
  const entry = (ref) => {
    const e = ref && ref.url ? performance.getEntriesByName(new URL(ref.url, location.href).href, 'resource').pop() : undefined;
    return e ? { startTime: e.startTime, responseEnd: e.responseEnd } : null;
  };
  const items = a && a.assetsReport ? (a.assetsReport()?.items ?? []) : [];
  return {
    state: a ? a.state : null,
    standIn: document.body.innerText.includes('STAND-IN'),
    timeOrigin: performance.timeOrigin,
    marks: a ? a.timeline : {},
    wasm: entry(m ? m.engine.wasm : null),
    seed: entry(m ? m.seedPack : null),
    dll: entry(m ? m.dllPack : null),
    wasmSplit: items.filter((i) => i.kind === 'wasm').map((i) => ({ origin: i.origin, transfer: i.transfer, ms: i.ms })),
  };
})()`;

const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`no answer in ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};
const round = (v) => Math.round(v * 10) / 10;
const num = (v) => (typeof v === 'number' && Number.isFinite(v) ? v : null);
const span = (a, b) => (num(a) === null || num(b) === null ? null : round(a - b));
const roundMarks = (marks) => Object.fromEntries(Object.entries(marks).map(([k, v]) => [k, num(v) === null ? v : round(v)]));

function phasesOf(r) {
  const k = r.marks;
  const packs = r.seed && r.dll ? Math.max(r.seed.responseEnd, r.dll.responseEnd) - Math.min(r.seed.startTime, r.dll.startTime) : null;
  return {
    fetchWasm: r.wasm ? round(r.wasm.responseEnd - r.wasm.startTime) : null,
    compileWasm: r.wasm ? span(k.compile_done, r.wasm.responseEnd) : null,
    fetchPacks: packs === null ? null : round(packs),
    instantiate: span(k.wasm_instantiated, k.engine_start),
    idle: span(k.idle, k.wasm_instantiated),
  };
}

async function oneRun(port, profile, index) {
  const out = { state: null, standIn: null, error: null, navToIdleMs: null, timelineIdle: null, phases: Object.fromEntries(PHASES.map((p) => [p, null])) };
  let chrome = null, cdp = null, polledAt = null, r = null;
  try {
    chrome = await launchChrome({ profile, extraArgs: ISOLATION });
    cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
    await cdp.opened;
    const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const S = (method, params) => timed(cdp.send(method, params, sessionId), CDP_TIMEOUT_MS, method);
    const ev = async (expression) => {
      const x = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      if (x.exceptionDetails) throw new Error(`evaluate: ${x.exceptionDetails.text}`);
      return x.result.value;
    };
    await S('Page.enable');
    const navSentAt = Date.now();
    const nav = await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
    for (;;) {
      const p = await ev(POLL).catch(() => null);
      const state = p ? p.state : null;
      if (state === 'engine-idle' && polledAt === null) polledAt = Date.now() - navSentAt;
      if (state === 'engine-idle' || state === 'error' || state === 'exited') break;
      if (Date.now() - navSentAt >= WAIT_LIMIT_MS) break;
      await sleep(POLL_MS);
    }
    r = await ev(READ);
    out.state = r.state;
    out.standIn = r.standIn;
    const idleAt = num(r.marks.idle);
    if (idleAt !== null) {
      out.timelineIdle = round(idleAt);
      out.navToIdleMs = round(r.timeOrigin + idleAt - navSentAt);
    }
    out.phases = phasesOf(r);
    if (out.state === 'error' || out.state === 'exited') out.error = `state ${out.state} before the first idle`;
    else if (out.state !== 'engine-idle') out.error = `no engine-idle within ${WAIT_LIMIT_MS} ms (state ${out.state ?? 'window.__app not set'})`;
    else if (idleAt === null) out.error = 'engine-idle without the idle mark';
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
  } finally {
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
  }
  process.stderr.write(`run ${index}: ${JSON.stringify({
    state: out.state,
    navToIdleMs: out.navToIdleMs,
    pollSeenMs: polledAt === null ? null : round(polledAt),
    marks: r ? roundMarks(r.marks) : null,
    wasmSplit: r ? r.wasmSplit : null,
    parallelism: os.availableParallelism(),
    loadavg: os.loadavg().map((x) => x.toFixed(2)),
  })}\n`);
  return out;
}

async function main() {
  const root = fs.mkdtempSync(path.join(repo, '.tmp-startup-'));
  let server = null;
  let code = 0;
  try {
    const assets = path.join(root, 'assets');
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    for (let i = 1; i <= RUNS; i++) {
      const out = await oneRun(port, path.join(root, `profile-${i}`), i);
      if (out.error) code = 1;
      process.stdout.write(JSON.stringify(out) + '\n');
    }
  } catch (e) {
    process.stderr.write(`startup-breakdown: ${String(e?.message ?? e).slice(0, 500)}\n`);
    code = 1;
  } finally {
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
  process.exitCode = code;
}

await main();
