// Boot hang measurement (Issue #1393): how often the engine's start does not reach engine-idle (the state stays running), at the device pixel ratio (DPR) that the page has before the start.
// Usage: node tools/boot-hang/measure.mjs <absolute path to the bundle directory raw/<name>> [--dprs 1,2.5,3] [--random LO,HI,COUNT]... [--n N] > hang.json
//   --dprs: fixed DPRs. --random LO,HI,COUNT (repeatable): COUNT random points in [LO, HI] (uniform, 2 decimals); every point is started N times, and each start draws its own DPR.
//   --n N: the starts of each point (default 1). Each point's starts alternate "viewStart" (the viewport with the DPR is set before Page.navigate) and no viewport change (the Chrome default), by the index of the start.
// Each start: a Chrome of its own (a new profile), navigation, the poll of window.__app.state until engine-idle, at most 60000 ms (WAIT_LIMIT_MS; 180 s is not waited for).
// A start that does not reach engine-idle is a hang. Then, at the time of the stall: the page's state (window.__app.state, error, the engine's log lines and the last 20 of them; null when the page's JS does not return), and for every Worker of the page
//   (detail.workers): its kind (url, Module, fleAudioWorkerState: the engine's Worker has Module), its state (fleAudioWorkerState() and Module.fleThreadStats() when it answers), and a CDP CPU profile over PROFILE_MS (3 s): busy and idle time, the top functions, and the most frequent call stacks (60 frames, leaf first).
//   The names come from tools/p3-audio-probe/profile-names.mjs (wasm-function[N] through lift/out-app/gen/wasm-func-names.tsv). The run's busy and topFunctions are those of the engine's Worker when it answered, else of the first Worker with a profile. A Worker that does not answer Runtime.evaluate is busy in a loop (its kind is "unreachable").
//   busy: the Worker's profile has more busy time than idle time (true: it runs; false: it waits; null: no profile). stalled: the reason (null when the start reached engine-idle).
// The tool changes nothing in the product: the viewport, the Chrome flags and the reads are measurements only.
// stdout: one JSON line {"runs":[{dpr, randomDprs?, viewStart, ok, firstIdleMs, stalled, busy, topFunctions, detail?}], "hangRate":{total, hung, rate}}. stderr: the log. Exit 1 when the tool itself failed, 2 when the arguments are wrong.
// The temporary directory is inside the repository (.tmp-boot-hang-*) and is removed.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';

const USAGE = 'usage: node tools/boot-hang/measure.mjs <absolute path to the bundle directory raw/<name>> [--dprs LIST] [--random LO,HI,COUNT]... [--n N] > hang.json\n';
const argv = process.argv.slice(2);
const valuesOf = (name) => argv.flatMap((a, i) => (a === name ? [argv[i + 1]] : []));
const positional = argv.filter((a, i) => !a.startsWith('--') && !['--dprs', '--random', '--n'].includes(argv[i - 1]));
const dprsRaw = valuesOf('--dprs');
const randomRaw = valuesOf('--random');
const nRaw = valuesOf('--n');
const fixed = dprsRaw.length ? dprsRaw.flatMap((s) => String(s).split(',')).map(Number) : [];
const randoms = randomRaw.map((s) => String(s).split(',').map(Number));
const N = nRaw.length ? Number(nRaw[nRaw.length - 1]) : 1;
const okDpr = (v) => Number.isFinite(v) && v > 0 && v <= 4;
const argsOk = positional.length === 1 && fixed.every(okDpr) && randoms.every((r) => r.length === 3 && okDpr(r[0]) && okDpr(r[1]) && r[0] <= r[1] && Number.isInteger(r[2]) && r[2] >= 1)
  && Number.isInteger(N) && N >= 1 && (fixed.length + randoms.length) > 0;
if (!argsOk) {
  process.stderr.write(USAGE);
  process.exit(2);
}

const bundleDir = path.resolve(positional[0]);
const BUNDLE = path.basename(bundleDir);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
const WAIT_LIMIT_MS = 60000;
const POLL_MS = 100;
const CDP_TIMEOUT_MS = 5000;
const PROFILE_MS = 3000;
const PROFILE_INTERVAL_US = 250;
const NAMES = path.join(repo, 'lift', 'out-app', 'gen', 'wasm-func-names.tsv');
// the room of the boot measurement (tools/p3-audio-probe/measure.mjs): the stage is 1836 x 1056 CSS px and the viewport adds the monitor panel and the header
const VIEW = { width: 1836 + 340, height: 1056 + 36 };
// the isolation flags of tools/boot-check/boot-bundle.mjs (copied: that file does not export them)
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

const log = (...a) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ')}\n`);
const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};
const STATE = '(() => { const a = window.__app; return a ? a.state : null; })()';
const DETAIL = `(() => {
  const a = window.__app;
  if (!a) return { appSet: false };
  const log = a.engineLog() ?? [];
  return { appSet: true, state: a.state, error: a.error ? String(a.error.what ?? a.error.message ?? a.error).slice(0, 300) : null, logLines: log.length, logTail: log.slice(-20).map((l) => String(l).slice(0, 200)) };
})()`;
const WORKER_KIND = "(() => ({ url: location.href, hasModule: typeof Module !== 'undefined', hasAudio: typeof globalThis.fleAudioWorkerState === 'function' }))()";
const WORKER_STATE = `(() => {
  const w = typeof globalThis.fleAudioWorkerState === 'function' ? globalThis.fleAudioWorkerState() : null;
  const t = typeof Module !== 'undefined' && Module.fleThreadStats ? Module.fleThreadStats() : null;
  return { audioWorker: w ? { publishes: w.publishes, maxPubGapMs: w.maxPubGapMs } : null, threads: t };
})()`;

// the points of the run, in order: the fixed DPRs, then each random point; every point gives N starts. The random DPR of a start is drawn when it starts.
function plan() {
  const pts = fixed.map((dpr) => ({ kind: 'fixed', dpr }));
  for (const [lo, hi, count] of randoms) for (let c = 0; c < count; c++) pts.push({ kind: 'random', lo, hi });
  const runs = [];
  for (const p of pts) for (let k = 0; k < N; k++) runs.push(p);
  return runs;
}
const drawDpr = (lo, hi) => Math.round((lo + Math.random() * (hi - lo)) * 100) / 100;

// the call stacks of the profile's samples: the most frequent ones (leaf first, names of tools/p3-audio-probe/profile-names.mjs's table for wasm-function[N]), 60 frames each
function stacksOf(file, top = 3) {
  const prof = JSON.parse(fs.readFileSync(file, 'utf8'));
  const names = new Map();
  if (fs.existsSync(NAMES)) for (const line of fs.readFileSync(NAMES, 'utf8').split('\n').slice(1)) { const c = line.split('\t'); if (c.length >= 4) names.set(Number(c[0]), c[3]); }
  const byId = new Map(prof.nodes.map((n) => [n.id, n]));
  const parent = new Map();
  for (const n of prof.nodes) for (const c of n.children ?? []) parent.set(c, n.id);
  const label = (id) => {
    const fn = byId.get(id).callFrame.functionName || '(anonymous)';
    const m = /^wasm-function\[(\d+)\]$/.exec(fn);
    return m ? (names.get(Number(m[1])) ?? fn) : fn;
  };
  const count = new Map();
  for (const id of prof.samples) {
    const frames = [];
    for (let cur = id; cur !== undefined && frames.length < 60; cur = parent.get(cur)) frames.push(label(cur));
    const key = frames.join(' < ');
    count.set(key, (count.get(key) ?? 0) + 1);
  }
  return [...count.entries()].sort((a, b) => b[1] - a[1]).slice(0, top).map(([stack, n]) => ({ samples: n, of: prof.samples.length, stack }));
}

// the engine's CPU profile over PROFILE_MS, named and split into busy and idle time
async function profileWorker(cdp, sid, root, name) {
  await timed(cdp.send('Profiler.enable', {}, sid), CDP_TIMEOUT_MS, 'Profiler.enable');
  await timed(cdp.send('Profiler.setSamplingInterval', { interval: PROFILE_INTERVAL_US }, sid), CDP_TIMEOUT_MS, 'Profiler.setSamplingInterval');
  await timed(cdp.send('Profiler.start', {}, sid), CDP_TIMEOUT_MS, 'Profiler.start');
  await sleep(PROFILE_MS);
  const r = await timed(cdp.send('Profiler.stop', {}, sid), 30000, 'Profiler.stop');
  const file = path.join(root, name);
  fs.writeFileSync(file, JSON.stringify(r.profile));
  const args = [path.join(repo, 'tools', 'p3-audio-probe', 'profile-names.mjs'), file, NAMES, '--top', '5'];
  const rep = JSON.parse(execFileSync(process.execPath, args, { cwd: repo, encoding: 'utf8', maxBuffer: 1 << 26 }));
  const stacks = stacksOf(file);
  return { busyMs: rep.busyMs, idleMs: rep.idleMs, busy: rep.busyMs > rep.idleMs, topFunctions: rep.top.map((t) => ({ name: t.name, selfMs: t.selfMs, shareOfBusy: t.shareOfBusy })), stacks };
}

async function start(env, point, viewStart) {
  const { root, port, n } = env;
  const requested = point.kind === 'fixed' ? point.dpr : drawDpr(point.lo, point.hi);
  const dpr = viewStart ? requested : 1;
  const run = { dpr, viewStart, ok: false, firstIdleMs: null, stalled: null, busy: null, topFunctions: [] };
  if (point.kind === 'random') run.randomDprs = requested;
  const profile = path.join(root, `profile-${n}`);
  let chrome = null, cdp = null, sid = null;
  const workers = [];
  let answered = false;
  try {
    chrome = await launchChrome({ profile, extraArgs: ISOLATION });
    cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
    await cdp.opened;
    const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
    const attached = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    sid = attached.sessionId;
    cdp.on((m) => {
      if (m.sessionId === sid && m.method === 'Target.attachedToTarget' && m.params.targetInfo.type === 'worker') workers.push(m.params.sessionId);
    });
    const S = (method, params) => timed(cdp.send(method, params, sid), CDP_TIMEOUT_MS, method);
    const ev = async (expression, ms = CDP_TIMEOUT_MS) => {
      const r = await timed(cdp.send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }, sid), ms, 'Runtime.evaluate');
      if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.text}`);
      return r.result.value;
    };
    await S('Page.enable');
    await S('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true });
    if (viewStart) await S('Emulation.setDeviceMetricsOverride', { width: VIEW.width, height: VIEW.height, deviceScaleFactor: dpr, mobile: false });
    const t0 = Date.now();
    const nav = await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
    let last = null, errorState = null;
    for (;;) {
      let st = null;
      try {
        st = await ev(STATE);
        answered = true;
      } catch {
        st = undefined;
      }
      last = st;
      if (st === 'engine-idle') { run.ok = true; run.firstIdleMs = Date.now() - t0; break; }
      if (st === 'error' || st === 'exited') { errorState = st; break; }
      if (Date.now() - t0 >= WAIT_LIMIT_MS) break;
      await sleep(POLL_MS);
    }
    if (!run.ok) {
      run.stalled = errorState !== null ? `state ${errorState}` : !answered ? `page JS did not return (Runtime.evaluate timeout ${CDP_TIMEOUT_MS} ms) within ${WAIT_LIMIT_MS} ms` : last === null ? `window.__app not set within ${WAIT_LIMIT_MS} ms` : `state ${last} after ${WAIT_LIMIT_MS} ms`;
      const detail = {};
      detail.page = await ev(DETAIL, CDP_TIMEOUT_MS).catch((e) => ({ unreachable: String(e?.message ?? e).slice(0, 120) }));
      // every Worker of the page: its kind (the engine's Worker has Module; the one with the audio counters has fleAudioWorkerState) and its profile over PROFILE_MS, one after another
      detail.workers = [];
      let engine = null;
      for (const w of workers) {
        const kind = await timed(cdp.send('Runtime.evaluate', { expression: WORKER_KIND, returnByValue: true }, w), CDP_TIMEOUT_MS, 'the worker').then((r) => r?.result?.value ?? null, (e) => ({ unreachable: String(e?.message ?? e).slice(0, 120) }));
        const row = { session: w, kind };
        if (engine === null && kind && (kind.hasModule || kind.hasAudio)) engine = w;
        row.state = await timed(cdp.send('Runtime.evaluate', { expression: WORKER_STATE, returnByValue: true }, w), CDP_TIMEOUT_MS, 'the worker state').then((r) => r?.result?.value ?? null, () => null);
        const p = await profileWorker(cdp, w, root, `profile-worker-${detail.workers.length}.json`).catch((e) => ({ error: String(e?.message ?? e).slice(0, 200) }));
        row.profile = p.error ? { error: p.error } : { busyMs: p.busyMs, idleMs: p.idleMs, busy: p.busy, top: p.topFunctions, stacks: p.stacks };
        detail.workers.push(row);
      }
      const main = detail.workers.find((r) => r.session === engine) ?? detail.workers.find((r) => r.profile?.top);
      if (main) { run.busy = main.profile.busy ?? null; run.topFunctions = main.profile.top ?? []; }
      detail.engineWorker = engine;
      run.detail = detail;
      log('stall', { viewStart, dpr, stalled: run.stalled, busy: run.busy });
    }
  } catch (e) {
    run.stalled = run.stalled ?? `tool: ${String(e?.message ?? e).slice(0, 300)}`;
    log('start failed', { viewStart, dpr, error: run.stalled });
  } finally {
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
    fs.rmSync(profile, { recursive: true, force: true });
  }
  return run;
}

async function main() {
  const root = fs.mkdtempSync(path.join(repo, '.tmp-boot-hang-'));
  const assets = path.join(root, 'assets');
  let server = null;
  const out = { startedAt: new Date().toISOString(), bundle: BUNDLE, runs: [], hangRate: null, nproc: os.availableParallelism?.() ?? os.cpus().length, loadavg: os.loadavg() };
  let toolError = null;
  try {
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    const runs = plan();
    for (let k = 0; k < runs.length; k++) {
      const viewStart = k % 2 === 0;
      log('start', { k, kind: runs[k].kind, viewStart });
      const r = await start({ root, port, n: k }, runs[k], viewStart);
      out.runs.push(r);
      log('done', { k, ok: r.ok, firstIdleMs: r.firstIdleMs, dpr: r.dpr, viewStart });
    }
  } catch (e) {
    toolError = String(e?.message ?? e).slice(0, 500);
  } finally {
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
    const hung = out.runs.filter((r) => !r.ok).length;
    out.hangRate = { total: out.runs.length, hung, rate: out.runs.length ? Math.round((hung / out.runs.length) * 1000) / 1000 : null };
    out.error = toolError;
    process.stdout.write(JSON.stringify(out) + '\n');
    process.exitCode = toolError ? 1 : 0;
  }
}

main();
