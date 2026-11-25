// Input cost, stage 1 (Issue #1397): the scene iii of measure.mjs (the pointer on the Playlist at 1200 Hz, the welcome window closed) traced for a window with the browser's trace of the categories below. The product is not changed.
// Usage: node tools/engine-cost/trace.mjs --bundle <absolute path to raw/<name>> --out <absolute path to the result JSON> [--seconds 20] [--dpr 1] [--pointer off] [--app <absolute path to an app directory with a built dist>]
// --app: the page served is the app in that directory (default: app/ of this repository); the trace of another build (an unminified copy, a build before a change) is taken with it.
// --pointer off sends nothing in the window (the page and the engine in the background of scene iii). The network requests of the window are counted (requestsInWindow).
// The result (one JSON object, also on stdout): per thread, the self time of each trace event name (the devtools.timeline events, nested by their intervals); the CPU profile of each thread (the trace's ProfileChunk samples, self time per function, the callers of the native and browser frames);
// and the self time of the CPU samples that are (program), by the innermost trace event of the same thread at the sample's time. The input counts: the pointer events sent, and the engine's driver counters (injected, idles, deadline and timer wakes) over the window.
// The trace is read in memory; the browser profile and the asset links are kept in .tmp-engine-cost-trace-<pid>/ only while it runs, and removed at the end. Exit 0 when error is null, 1 when the run failed (the JSON is still written), 2 when the arguments are wrong.
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';

const USAGE = 'usage: node tools/engine-cost/trace.mjs --bundle <absolute path to raw/<name>> --out <absolute path to the result JSON> [--seconds 20] [--dpr 1]\n';
const argv = process.argv.slice(2);
const valueOf = (flag) => { const i = argv.indexOf(flag); return i < 0 ? undefined : argv[i + 1]; };
const bundleArg = valueOf('--bundle');
const OUT = valueOf('--out');
const SECONDS = valueOf('--seconds') === undefined ? 20 : Number(valueOf('--seconds'));
const DPR = valueOf('--dpr') === undefined ? 1 : Number(valueOf('--dpr'));
const POINTER = valueOf('--pointer') !== 'off';   // --pointer off: nothing is sent in the window (the background of the page and the engine)
const APP_ARG = valueOf('--app');
const knownFlags = ['--bundle', '--out', '--seconds', '--dpr', '--pointer', '--app'];
const shapeOk = argv.length % 2 === 0 && argv.every((a, i) => (i % 2 === 0 ? knownFlags.includes(a) : !a.startsWith('--')));
if (!shapeOk || !bundleArg || !path.isAbsolute(bundleArg) || !OUT || !path.isAbsolute(OUT) || !(SECONDS > 0) || !(DPR > 0) || (APP_ARG !== undefined && !path.isAbsolute(APP_ARG))) {
  process.stderr.write(USAGE);
  process.exit(2);
}
const bundleDir = path.resolve(bundleArg);
const BUNDLE = path.basename(bundleDir);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = APP_ARG ?? path.join(repo, 'app');
const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);

// the same settings as measure.mjs (scene iii)
const VIEWPORT = { width: 1836 + 340, height: 1056 + 36 };
const WINDOW_START_MS = 30000;
const SEND_HZ = 1200;
const REV_PER_S = 2;
const PLAYLIST = { cx: 0.78, cy: 0.55, rx: 0.05, ry: 0.05 };
const WIZARD_CLASS = 'TWelcomeWizard';
const WAIT_LIMIT_MS = 180000;
const POLL_MS = 50;
const CDP_TIMEOUT_MS = 20000;
const IDLE_LIMIT_MS = 30000;
const WIZARD_LIMIT_MS = 15000;
const TRACE_CATEGORIES = 'devtools.timeline,v8,v8.wasm,disabled-by-default-v8.cpu_profiler,disabled-by-default-v8.cpu_profiler.hires,toplevel';
const TOP = 40;

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

const STATE = 'window.__app ? window.__app.state : null';
const GEOMETRY = "(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const g = window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: g.width, gh: g.height }; })()";
const WINDOWS = '(async () => (await window.__app.deskDumpBase64(false)).windows)()';
const DRIVER = '(() => { const s = window.__app.driverStats; return s ? { injected: s.injected, idleCount: s.idleCount, deadlineWakes: s.deadlineWakes, timerWakes: s.timerWakes, bounces: s.bounces, chained: s.chained, dropped: s.dropped } : null; })()';

const log = (...a) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ')}\n`);
const r1 = (x) => Math.round(x * 10) / 10;
const r3 = (x) => Math.round(x * 1000) / 1000;
const errText = (e) => String(e?.message ?? e).slice(0, 300);
const sleepUntil = async (at) => { const d = at - Date.now(); if (d > 0) await sleep(d); };
const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};

async function waitIdle(cx, limitMs) {
  const t0 = Date.now();
  for (;;) {
    const st = await cx.ev(STATE).catch(() => null);
    if (st === 'engine-idle') return Date.now();
    if (st === 'error' || st === 'exited') throw new Error(`state ${st} before the engine was idle`);
    if (Date.now() - t0 >= limitMs) throw new Error(`no engine-idle within ${limitMs} ms (state ${st ?? 'window.__app not set'})`);
    await sleep(POLL_MS);
  }
}

async function clickGuest(cx, gx, gy, geo) {
  const c = { x: geo.x + (gx + 0.5) * (geo.w / geo.gw), y: geo.y + (gy + 0.5) * (geo.h / geo.gh) };
  await cx.S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
  await sleep(120);
  await cx.S('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
  await sleep(70);
  await cx.S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
  await sleep(60);
}

function parseWindows(text) {
  return text.split('\n').map((l) => /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean).map((m) => {
    const names = /class "([^"]*)" title "([^"]*)"\s*$/.exec(m[6]);
    return { cls: names ? names[1] : '', x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]) };
  });
}

async function windowsNow(cx) {
  const t0 = Date.now();
  for (;;) {
    try { return parseWindows(await cx.ev(WINDOWS)); } catch (e) {
      if (!/ask again/.test(String(e?.message ?? e)) || Date.now() - t0 >= 15000) throw e;
      await sleep(100);
    }
  }
}

async function closeWelcome(cx) {
  const wiz = (await windowsNow(cx)).find((w) => w.cls === WIZARD_CLASS);
  if (!wiz) return false;
  await clickGuest(cx, wiz.x + wiz.w - 18, wiz.y + 18, await cx.ev(GEOMETRY));
  const t0 = Date.now();
  while ((await windowsNow(cx)).some((w) => w.cls === WIZARD_CLASS)) {
    if (Date.now() - t0 >= WIZARD_LIMIT_MS) throw new Error('the welcome window did not close');
    await sleep(POLL_MS);
  }
  await sleep(1500);
  await waitIdle(cx, IDLE_LIMIT_MS);
  return true;
}

function pointerAt(geo, i) {
  const th = 2 * Math.PI * REV_PER_S * (i / SEND_HZ);
  return { x: geo.x + (PLAYLIST.cx + PLAYLIST.rx * Math.cos(th)) * geo.w, y: geo.y + (PLAYLIST.cy + PLAYLIST.ry * Math.sin(th)) * geo.h };
}

async function pointerLoop(cx, geo, st) {
  const t0 = performance.now();
  while (!st.stop) {
    const due = Math.floor(((performance.now() - t0) * SEND_HZ) / 1000);
    while (st.sent < due) {
      const p = pointerAt(geo, st.sent);
      cx.cdp.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p.x, y: p.y, button: 'none', buttons: 0, pointerType: 'mouse' }, cx.pageSid).catch(() => {});
      st.sent++;
    }
    await sleep(2);
  }
}

// ---- the trace: self time by event name, per thread (X events nested by their intervals) ----
function selfTimes(events) {
  const byThread = new Map();
  for (const e of events) {
    if (e.ph !== 'X' || typeof e.dur !== 'number') continue;
    const key = `${e.pid}:${e.tid}`;
    if (!byThread.has(key)) byThread.set(key, []);
    byThread.get(key).push(e);
  }
  const out = new Map();
  for (const [key, list] of byThread) {
    list.sort((a, b) => a.ts - b.ts || b.dur - a.dur);
    const stack = [];
    const self = new Array(list.length);
    for (let i = 0; i < list.length; i++) {
      const e = list[i];
      while (stack.length && list[stack[stack.length - 1]].ts + list[stack[stack.length - 1]].dur <= e.ts) stack.pop();
      self[i] = e.dur;
      if (stack.length) { const p = stack[stack.length - 1]; if (e.ts + e.dur <= list[p].ts + list[p].dur) self[p] -= e.dur; }
      stack.push(i);
    }
    const names = new Map();
    for (let i = 0; i < list.length; i++) {
      const n = list[i].name;
      const v = names.get(n) ?? { us: 0, count: 0 };
      v.us += Math.max(0, self[i]);
      v.count++;
      names.set(n, v);
    }
    out.set(key, names);
  }
  return out;
}

// the CPU profile of each thread from the trace's ProfileChunk events: nodes, samples and time deltas, with the time of each sample (µs)
// the Profile event (its pid, tid and id) names the thread that is profiled; the ProfileChunk events of that id come from the profiler's own thread, so they are matched by the pid and the id
function cpuProfiles(events) {
  const profs = new Map();
  for (const e of events) {
    if (e.name !== 'Profile' || !e.args?.data) continue;
    const k = `${e.pid}:${e.id}`;
    profs.set(k, { pid: e.pid, tid: e.tid, nodes: new Map(), samples: [], deltas: [], start: e.args.data.startTime ?? e.ts });
  }
  for (const e of events) {
    if (e.name !== 'ProfileChunk' || !e.args?.data) continue;
    const p = profs.get(`${e.pid}:${e.id}`);
    if (!p) continue;
    const cp = e.args.data.cpuProfile ?? {};
    for (const n of cp.nodes ?? []) p.nodes.set(n.id, n);
    for (const s of cp.samples ?? []) p.samples.push(s);
    for (const d of e.args.data.timeDeltas ?? []) p.deltas.push(d);
  }
  return [...profs.values()].filter((p) => p.samples.length > 0);
}

const SELF_KEYS = new Set(['(program)', 'postMessage', 'decode', 'setTimeout', 'now', 'getElementById', 'wasm-to-js', 'js-to-wasm', '(garbage collector)', 'Atomics.wait', 'fle_js_wait', 'fle_js_input_poll', 'fle_js_input_done']);

// one profile (one id of a thread): the self time per function, the callers of the SELF_KEYS frames, and the time of each sample (the chunk's time deltas from the profile's start)
function analyzeProfile(p) {
  const parent = new Map();
  for (const n of p.nodes.values()) {
    if (n.parent !== undefined) parent.set(n.id, n.parent);
    for (const c of n.children ?? []) parent.set(c, n.id);
  }
  const frame = (id) => p.nodes.get(id)?.callFrame ?? {};
  const nameOf = (id) => frame(id).functionName || '(anonymous)';
  const fnKey = (id) => {
    const f = frame(id);
    const url = (f.url || '').replace(/^https?:\/\/127\.0\.0\.1:\d+/, '');
    return url ? `${nameOf(id)} ${url}:${f.lineNumber + 1}:${f.columnNumber + 1}` : nameOf(id);
  };
  const byFn = new Map(), callers = new Map();
  const times = new Array(p.samples.length);
  let t = p.start;
  for (let i = 0; i < p.samples.length; i++) { t += p.deltas[i] ?? 0; times[i] = t; }
  let total = 0;
  for (let i = 0; i < p.samples.length; i++) {
    const id = p.samples[i];
    const dt = (p.deltas[i + 1] ?? 0) / 1000;
    const name = nameOf(id);
    if (name === '(idle)') continue;
    total += dt;
    byFn.set(name, (byFn.get(name) ?? 0) + dt);
    if (SELF_KEYS.has(name)) {
      const par = parent.get(id);
      const ck = par === undefined ? '(root)' : fnKey(par);
      const m = callers.get(name) ?? new Map();
      m.set(ck, (m.get(ck) ?? 0) + dt);
      callers.set(name, m);
    }
  }
  return { pid: p.pid, tid: p.tid, total, byFn, callers, times, sampleName: (i) => nameOf(p.samples[i]), sampleDt: (i) => (p.deltas[i + 1] ?? 0) / 1000, count: p.samples.length };
}

// (program) samples, attributed to the innermost X event of the same thread that covers the sample's time
function programByEvent(events, prof) {
  const ev = events.filter((e) => e.ph === 'X' && typeof e.dur === 'number' && e.pid === prof.pid && e.tid === prof.tid).sort((x, y) => x.ts - y.ts);
  const by = new Map();
  let k = 0;
  const stack = [];
  for (let i = 0; i < prof.times.length; i++) {
    if (prof.sampleName(i) !== '(program)') continue;
    const t = prof.times[i];
    while (k < ev.length && ev[k].ts <= t) { stack.push(ev[k]); k++; }
    while (stack.length && stack[stack.length - 1].ts + stack[stack.length - 1].dur <= t) stack.pop();
    const inner = stack.length ? stack[stack.length - 1] : null;
    const d = inner?.args?.data;
    const top = inner === null ? '(no trace event)' : d?.url ? `${inner.name} ${(d.functionName || '(anonymous)')} ${String(d.url).replace(/^https?:\/\/127\.0\.0\.1:\d+/, '')}:${(d.lineNumber ?? 0) + 1}:${(d.columnNumber ?? 0) + 1}` : inner.name;
    by.set(top, (by.get(top) ?? 0) + prof.sampleDt(i));
  }
  return by;
}

const top = (map, n) => [...map.entries()].sort((a, b) => b[1] - a[1]).slice(0, n).map(([k, v]) => ({ name: k, ms: r1(v) }));
const mergeMaps = (maps) => { const out = new Map(); for (const m of maps) for (const [k, v] of m) out.set(k, (out.get(k) ?? 0) + v); return out; };

async function main() {
  const out = { scene: 'iii', dpr: DPR, seconds: SECONDS, categories: TRACE_CATEGORIES, error: null, inputs: null, threads: [], totals: null, notes: '' };
  const root = path.join(repo, `.tmp-engine-cost-trace-${process.pid}`);
  const assets = path.join(root, 'assets');
  const chromeProfile = path.join(root, 'chrome-profile');
  let server = null, chrome = null, cdp = null;
  try {
    fs.mkdirSync(root, { recursive: true });
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    chrome = await launchChrome({ profile: chromeProfile, extraArgs: ISOLATION });
    cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
    await cdp.opened;
    const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
    const { sessionId: pageSid } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const cx = {
      cdp, pageSid,
      T: (method, params, sid, ms = CDP_TIMEOUT_MS) => timed(cdp.send(method, params, sid), ms, method),
    };
    cx.S = (method, params = {}) => cx.T(method, params, pageSid);
    cx.ev = async (expression) => {
      const r = await cx.S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(`evaluate: ${(r.exceptionDetails.exception?.description ?? r.exceptionDetails.text ?? '').slice(0, 200)}`);
      return r.result.value;
    };
    await cx.S('Page.enable');
    await cx.S('Emulation.setDeviceMetricsOverride', { width: VIEWPORT.width, height: VIEWPORT.height, deviceScaleFactor: DPR, mobile: false });
    await cx.S('Network.enable');
    const navAt = Date.now();
    const nav = await cx.S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
    const tIdle = await waitIdle(cx, WAIT_LIMIT_MS);
    log('engine-idle', { ms: tIdle - navAt });
    await closeWelcome(cx);
    await sleepUntil(tIdle + WINDOW_START_MS);
    const geo = await cx.ev(GEOMETRY);
    const d0 = await cx.ev(DRIVER);
    const events = [];
    const requests = [];
    const done = new Promise((res) => { cdp.on((m) => {
      if (m.method === 'Tracing.dataCollected' && !m.sessionId) for (const e of m.params.value ?? []) events.push(e);
      if (m.method === 'Tracing.tracingComplete' && !m.sessionId) res();
      if (m.method === 'Network.requestWillBeSent' && m.sessionId === pageSid) requests.push(String(m.params.request?.url ?? '').replace(/^https?:\/\/127\.0\.0\.1:\d+/, '').slice(0, 120));
    }); });
    await cdp.send('Tracing.start', { categories: TRACE_CATEGORIES, options: 'record-as-much-as-possible', transferMode: 'ReportEvents' });
    const tA = Date.now();
    const pointer = { stop: false, sent: 0 };
    const pointerP = POINTER ? pointerLoop(cx, geo, pointer) : Promise.resolve();
    await sleep(SECONDS * 1000);
    pointer.stop = true;
    await pointerP;
    const windowMs = Date.now() - tA;
    const d1 = await cx.ev(DRIVER);
    await cdp.send('Tracing.end');
    await timed(done, 120000, 'Tracing.tracingComplete');
    log('trace collected', { events: events.length, windowMs });

    const meta = new Map();
    for (const e of events) if (e.ph === 'M' && e.name === 'thread_name') meta.set(`${e.pid}:${e.tid}`, e.args?.name ?? '');
    const sums = selfTimes(events);
    const profs = cpuProfiles(events).map(analyzeProfile);
    const threadKeys = new Set([...sums.keys(), ...profs.map((p) => `${p.pid}:${p.tid}`)]);
    let busyAll = 0;
    for (const key of threadKeys) {
      const [pid, tid] = key.split(':').map(Number);
      const names = sums.get(key) ?? new Map();
      const mine = profs.filter((p) => p.pid === pid && p.tid === tid);
      const traceBusy = [...names.values()].reduce((a, v) => a + v.us, 0) / 1000;
      const cpuBusy = mine.reduce((a, p) => a + p.total, 0);
      busyAll += cpuBusy;
      const callers = new Map();
      for (const p of mine) for (const [fn, m] of p.callers) callers.set(fn, mergeMaps([callers.get(fn) ?? new Map(), m]));
      out.threads.push({
        name: meta.get(key) ?? `tid ${tid}`, pid, tid,
        traceSelfMs: r1(traceBusy), cpuBusyMs: r1(cpuBusy),
        eventSelf: [...names.entries()].sort((a, b) => b[1].us - a[1].us).slice(0, TOP).map(([name, v]) => ({ name, ms: r1(v.us / 1000), count: v.count })),
        cpuTopFunctions: top(mergeMaps(mine.map((p) => p.byFn)), TOP),
        callers: Object.fromEntries([...callers.entries()].map(([fn, m]) => [fn, top(m, 12)])),
        programByEvent: top(mergeMaps(mine.map((p) => programByEvent(events, p))), TOP),
      });
    }
    out.inputs = {
      pointerSent: pointer.sent, windowMs: r1(windowMs),
      driverStart: d0, driverEnd: d1,
      injected: d1 && d0 ? d1.injected - d0.injected : null,
      idles: d1 && d0 ? d1.idleCount - d0.idleCount : null,
      deadlineWakes: d1 && d0 ? d1.deadlineWakes - d0.deadlineWakes : null,
      timerWakes: d1 && d0 ? d1.timerWakes - d0.timerWakes : null,
      dropped: d1 && d0 ? (d1.dropped ?? 0) - (d0.dropped ?? 0) : null,
    };
    out.totals = { cpuBusyMs: r1(busyAll) };
    out.requestsInWindow = { count: requests.length, first: requests.slice(0, 20) };
    out.notes = `Trace categories: ${TRACE_CATEGORIES}. Window: ${SECONDS} s from t0+${WINDOW_START_MS} ms. Self time of an X event: its duration minus its nested X events on the same thread. cpuBusyMs: the samples that are not (idle), summed over the threads that have a profile.`;
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
  } finally {
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
  const line = JSON.stringify(out) + '\n';
  fs.mkdirSync(path.dirname(OUT), { recursive: true });
  fs.writeFileSync(OUT, line);
  process.stdout.write(line);
  process.exitCode = out.error ? 1 : 0;
}

await main();
