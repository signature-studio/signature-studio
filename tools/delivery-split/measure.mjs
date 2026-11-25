// Delivery split of the start-up (Issue #1314): what the page downloads before the engine starts (the manifest's pre-start assets) and what it downloads after it has started (manifest.lazy),
// and when the engine is idle. One run = one headless Chrome with a new profile, the same launch as tools/boot-check/boot-bundle.mjs, until the engine is idle (or an error / exit).
// Usage: node tools/delivery-split/measure.mjs <absolute path to the bundle directory raw/<name>> [runs, 1..5, default 3] > delivery.jsonl
// stdout: one JSON object per run, on its own line: {"state","standIn","error","elapsedMs","preStart":[{"name","bytes"}],"lazy":[{"name","bytes"}],"preStartDoneMs","lazyStartMs","lazyDoneMs","lazyWaitFirstMs","lazyWaitLast","fetches":[{"name","startMs","endMs"}],"reads":[{"name","atMs","requested"}]}
//   reads (Issue #1341): the engine's reads of a real module's image, from its line `image-read: <file name> requested=<0|1> <ms>` (the page takes the time of the line: atMs, ms since the navigation, the clock of elapsedMs; requested true when the guest asked for the module). Always an array; [] when the engine read none.
//   Issue #1342: the same line also names the instantiation of a real module's separate wasm module (`image-read: fruity_balance_x64.6800000.wasm requested=1 <ms>`, written when the guest asks for the module; xmod.ts).
//   scene (Issue #1342): after the first idle (the numbers above are taken before it), the scene asks the guest for Fruity Balance: the Welcome window is closed as tools/smoke/welcome-flow.mjs does (flow A),
//   Ctrl+O opens the engine's Open dialog (the page's file manager in selection mode), the project Data/Demo projects/NewStuff.flp (host-extra/resources of the bundle's out directory; its project names Fruity Balance, and the guest asks for the plug-in when it opens)
//   is copied in, chosen and opened. The reads above are the reads after the scene. (Fruity Balance.flp in Data/System/Render tests does not name the plug-in in its project: the guest never asks for Balance.)
//   scene: { steps, error, state (the page's state after the scene), welcome ('listed' | 'none'), workers: [{ type, script }] (the Workers of the browser context at the end) }; null when the first idle was not reached.
//   rssMb (Issue #1379): the largest sum of the VmRSS (resident memory, 1 MB = 1,048,576 B) of the browser: the Chrome process and every process under it, sampled every second from the first idle to the end of the scene. null when no sample was taken. This is the memory the judge uses.
//   engineMemoryBytes (Issue #1379, a reference only): window.__app.engineMemoryBytes at the first idle, the wasm memory's byteLength. It is address space, not resident memory: untouched pages are not in rssMb.
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
import { Cdp, descendants, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
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
  engineMemoryBytes: a ? a.engineMemoryBytes : null,
}; })()`;

const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};

// Issue #1342: the scene of the guest's request for Fruity Balance (see the header). The room and the places of the controls are those of tools/smoke/welcome-flow.mjs (its offsets are measured at the room 1836 x 1056).
const SCENE_FILE = ['Data', 'Demo projects', 'NewStuff.flp'];
const SCENE_ROOM_W = 1836, SCENE_ROOM_H = 1056;
// the view before the navigation (as tools/smoke/welcome-flow.mjs): the Welcome window is placed when it is created, so a smaller view puts it at (0, 0) for the whole scene
const SCENE_VIEW_START = { width: 2176, height: 1092 };
const SCENE_WIZARD = 'TWelcomeWizard';
const SCENE_NO_THANKS = { dx: 512, dy: 307 };
const SCENE_LIST = { dx: 235, dy: 180 };
const SCENE_DEFAULT_ROW = { dx: 235, dy: 117 };
const SCENE_STEP_LIMIT_MS = 120000;
const SCENE_GEOMETRY = `(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const g = window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: g.width, gh: g.height }; })()`;
const SCENE_ROOM = `(() => { const s = document.getElementById('stage').getBoundingClientRect(); return { sw: s.width, sh: s.height, iw: window.innerWidth, ih: window.innerHeight }; })()`;
const SCENE_WINDOWS = `(async () => (await window.__app.deskDumpBase64(false)).windows)()`;

/** the windows of a window list: the lid at (x, y) and the class of each line (the page's format: lid N at (x, y) WxH ... class "..." title "...") */
function parseWindows(text) {
  return text.split('\n').map((l) => /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean).map((m) => {
    const names = /class "([^"]*)"/.exec(m[6]);
    const title = /title "([^"]*)"/.exec(m[6]);
    return { x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), cls: names ? names[1] : '', title: title ? title[1] : '' };
  });
}

/** the scene: Welcome closed ("No thanks", then the Default row by a double click), Ctrl+O, the project given to the open box; returns what it did and the reads after it */
async function runScene({ S, ev, cdp, browserContextId, projectPath, shotPath }) {
  const steps = [];
  const out = { steps, error: null, state: null, welcome: null, workers: null, reads: null };
  const windows = async () => {
    const t0 = Date.now();
    for (;;) {
      try { return parseWindows(await ev(SCENE_WINDOWS)); } catch (e) {
        if (!/ask again/.test(String(e?.message ?? e)) || Date.now() - t0 >= 15000) throw e;
        await sleep(100);
      }
    }
  };
  const wizard = async () => (await windows()).find((w) => w.cls === SCENE_WIZARD) ?? null;
  const pointAt = async (gx, gy) => {
    const g = await ev(SCENE_GEOMETRY);
    return { x: g.x + (gx + 0.5) * (g.w / g.gw), y: g.y + (gy + 0.5) * (g.h / g.gh) };
  };
  const click = async (gx, gy, clicks = 1) => {
    const c = await pointAt(gx, gy);
    await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(60);
    for (let k = 1; k <= clicks; k++) {
      await S('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'left', buttons: 1, clickCount: k, pointerType: 'mouse' });
      await sleep(50);
      await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: 'left', buttons: 0, clickCount: k, pointerType: 'mouse' });
      await sleep(100);
    }
  };
  const wheel = async (gx, gy, notches, deltaY) => {
    const c = await pointAt(gx, gy);
    for (let i = 0; i < notches; i++) {
      await S('Input.dispatchMouseEvent', { type: 'mouseWheel', x: c.x, y: c.y, deltaX: 0, deltaY, pointerType: 'mouse' });
      await sleep(300);
    }
  };
  // the engine's "Problems loading the project" box (the Default project's plug-ins and samples) is modal: Enter closes it (its default button)
  const dismissProblems = async () => {
    let n = 0;
    for (let i = 0; i < 4 && (await windows()).some((w) => /Problems loading/.test(w.title)); i++) {
      const key = { code: 'Enter', key: 'Enter', windowsVirtualKeyCode: 0x0d, nativeVirtualKeyCode: 0x0d };
      await S('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...key });
      await sleep(40);
      await S('Input.dispatchKeyEvent', { type: 'keyUp', ...key });
      await sleep(1000);
      n++;
    }
    return n;
  };
  const ctrlO = async () => {
    await ev("document.getElementById('desktop').focus({ preventScroll: true }); true");
    await sleep(100);
    // the engine reads the Ctrl state from its own key messages: Control goes down first (VK_LCONTROL, as keymap.ts has it)
    const ctrl = { code: 'ControlLeft', key: 'Control', windowsVirtualKeyCode: 0xa2, nativeVirtualKeyCode: 0xa2 };
    const key = { code: 'KeyO', key: 'o', windowsVirtualKeyCode: 79, nativeVirtualKeyCode: 79 };
    await S('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...ctrl, modifiers: 2 });
    await sleep(40);
    await S('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...key, modifiers: 2 });
    await sleep(40);
    await S('Input.dispatchKeyEvent', { type: 'keyUp', ...key, modifiers: 2 });
    await sleep(40);
    await S('Input.dispatchKeyEvent', { type: 'keyUp', ...ctrl, modifiers: 0 });
  };
  const untilIdle = async (cond, what) => {
    const t0 = Date.now();
    for (;;) {
      const s = await ev(SNAPSHOT).catch(() => null);
      if (s && (s.state === 'error' || s.state === 'exited')) return s;
      if (s && s.state === 'engine-idle' && (await cond())) return s;
      if (Date.now() - t0 >= SCENE_STEP_LIMIT_MS) throw new Error(`${what}: not within ${SCENE_STEP_LIMIT_MS} ms`);
      await sleep(1000);
    }
  };
  try {
    for (let i = 0; i < 4; i++) {
      const r = await ev(SCENE_ROOM);
      if (Math.abs(r.sw - SCENE_ROOM_W) <= 0.5 && Math.abs(r.sh - SCENE_ROOM_H) <= 0.5) break;
      await S('Emulation.setDeviceMetricsOverride', { width: Math.round(r.iw + (SCENE_ROOM_W - r.sw)), height: Math.round(r.ih + (SCENE_ROOM_H - r.sh)), deviceScaleFactor: 1, mobile: false });
      await sleep(1500);
    }
    await sleep(1500);
    const titleBefore = await ev('document.title');
    const w0 = await wizard();
    out.welcome = w0 ? 'listed' : 'none';
    if (w0) {
      steps.push(`the Welcome window at (${w0.x}, ${w0.y}) ${w0.w}x${w0.h}`);
      await click(w0.x + SCENE_NO_THANKS.dx, w0.y + SCENE_NO_THANKS.dy);
      steps.push(`No thanks at guest (${w0.x + SCENE_NO_THANKS.dx}, ${w0.y + SCENE_NO_THANKS.dy})`);
      await sleep(2500);
      await untilIdle(async () => true, 'after No thanks');
      const w1 = await wizard();
      if (w1) {
        await wheel(w1.x + SCENE_LIST.dx, w1.y + SCENE_LIST.dy, 12, -120);
        await wheel(w1.x + SCENE_LIST.dx, w1.y + SCENE_LIST.dy, 3, 120);
        await sleep(400);
        await click(w1.x + SCENE_DEFAULT_ROW.dx, w1.y + SCENE_DEFAULT_ROW.dy, 2);
        steps.push('the Default row, 2 clicks');
      }
    }
    await untilIdle(async () => (await ev('document.title')) !== titleBefore && !(await wizard()), 'the project open');
    steps.push('the project is open (the title changed, the Welcome closed)');
    const dismissed = await dismissProblems();
    if (dismissed) steps.push(`the Problems box closed ${dismissed} time(s) before Ctrl+O`);
    await ctrlO();
    steps.push('Ctrl+O');
    if (!fs.existsSync(projectPath)) throw new Error(`no project file ${projectPath}`);
    // the open box is the page's file box (#filedlg-input), or the file manager in selection mode (#fx-input-files: the file is copied in, then chosen in the list and Open is pressed)
    const t0 = Date.now();
    let box = null;
    let again = false;
    while (!box && Date.now() - t0 < 40000) {
      box = await ev("document.getElementById('filedlg-input') ? 'filedlg' : (document.getElementById('fx-input-files') ? 'explorer' : null)");
      if (!box && (await dismissProblems())) {
        steps.push('the Problems box closed, Ctrl+O again');
        if (!again) { again = true; await ctrlO(); }
      }
      if (!box) await sleep(500);
    }
    if (!box) throw new Error('no open box (#filedlg-input or #fx-input-files) 40 s after Ctrl+O');
    const name = path.basename(projectPath);
    const setFile = async (selector) => {
      const doc = await S('DOM.getDocument', {});
      const q = await S('DOM.querySelector', { nodeId: doc.root.nodeId, selector });
      if (!q.nodeId) throw new Error(`${selector} is not a node`);
      await S('DOM.setFileInputFiles', { nodeId: q.nodeId, files: [projectPath] });
    };
    const clickSelector = async (selector) => {
      const r = await ev(`(() => { const e = document.querySelector(${JSON.stringify(selector)}); if (!e) return null; const b = e.getBoundingClientRect(); return { x: b.x + b.width / 2, y: b.y + b.height / 2 }; })()`);
      if (!r) throw new Error(`no element ${selector}`);
      await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: r.x, y: r.y, button: 'none', buttons: 0, pointerType: 'mouse' });
      await sleep(60);
      await S('Input.dispatchMouseEvent', { type: 'mousePressed', x: r.x, y: r.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
      await sleep(50);
      await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: r.x, y: r.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
      await sleep(300);
    };
    if (box === 'filedlg') {
      await setFile('#filedlg-input');
      steps.push(`${name} given to #filedlg-input`);
    } else {
      await setFile('#fx-input-files');
      const row = `#fx-list [data-name="${name}"]`;
      const t2 = Date.now();
      while (!(await ev(`!!document.querySelector(${JSON.stringify(row)})`))) {
        if (Date.now() - t2 >= 30000) throw new Error(`${name} is not listed in the file manager 30 s after the upload`);
        await sleep(500);
      }
      await clickSelector(row);
      await clickSelector('#fx-ok');
      steps.push(`${name} uploaded into the file manager, chosen and Open pressed`);
    }
    const t1 = Date.now();
    let snap = null;
    for (;;) {
      await dismissProblems().catch(() => 0);
      snap = await ev(SNAPSHOT).catch(() => null);
      if (snap && snap.imageReads.some((r) => /balance/i.test(r.name))) break;
      if (snap && (snap.state === 'error' || snap.state === 'exited')) break;
      if (Date.now() - t1 >= SCENE_STEP_LIMIT_MS) break;
      await sleep(1000);
    }
    await sleep(3000);
    snap = await ev(SNAPSHOT).catch(() => null);
    out.state = snap?.state ?? null;
    out.reads = (snap?.imageReads ?? []).map((r) => ({ name: r.name, atMs: num(r.atMs), requested: r.requested === true }));
    steps.push(`reads after the scene: ${out.reads.map((r) => `${r.name} requested=${r.requested}`).join(', ') || 'none'}`);
    out.logTail = await ev("window.__app.engineLog().filter((l) => /balance|real module|image-read|LoadLibrary|Problems|\\.flp|filedlg|opened|plug|xmod|trap|abort|exit|stop/i.test(l)).slice(-60)").catch(() => null);
    out.logEnd = await ev("window.__app.engineLog().slice(-25)").catch(() => null);
    out.exit = await ev("JSON.stringify({ state: window.__app.state, exit: window.__app.exit, error: window.__app.error })").catch(() => null);
    const end = await S('Page.captureScreenshot', { format: 'png' }).catch(() => null);
    if (end) fs.writeFileSync(path.join(repo, '.tmp-delivery-scene-end.png'), Buffer.from(end.data, 'base64'));
    if (snap?.error) out.error = `${snap.error.what} [${snap.error.where}]`;
    else if (snap && snap.state !== 'engine-idle') out.error = `state ${snap.state} after the scene`;
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
    const s = await S('Page.captureScreenshot', { format: 'png' }).catch(() => null);
    if (s) fs.writeFileSync(shotPath, Buffer.from(s.data, 'base64'));
  }
  const targets = (await cdp.send('Target.getTargets', {}).catch(() => ({ targetInfos: [] }))).targetInfos;
  out.workers = targets
    .filter((t) => t.browserContextId === browserContextId && ['worker', 'shared_worker', 'service_worker', 'worklet'].includes(t.type))
    .map((t) => ({ type: t.type, title: t.title, script: t.url.startsWith('blob:') ? 'blob' : (new URL(t.url, 'http://x/').pathname.split('/').pop() || t.url) }));
  return out;
}

const num = (v) => (typeof v === 'number' && Number.isFinite(v) ? Math.round(v * 10) / 10 : null);

/** Issue #1379: the resident memory (VmRSS, KiB) of the browser: the Chrome process and every process under it (the renderers, the GPU process, ...). */
function browserRssKiB(rootPid) {
  let total = 0;
  for (const pid of descendants(rootPid)) {
    try {
      const m = /^VmRSS:\s+(\d+) kB/m.exec(fs.readFileSync(`/proc/${pid}/status`, 'utf8'));
      if (m) total += Number(m[1]);
    } catch { /* the process has gone */ }
  }
  return total;
}

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
  const out = { state: null, standIn: null, error: null, elapsedMs: null, preStart: [], lazy: [], preStartDoneMs: null, lazyStartMs: null, lazyDoneMs: null, lazyWaitFirstMs: null, lazyWaitLast: null, fetches: [], reads: [], scene: null, rssMb: null, engineMemoryBytes: null };
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
    await S('Emulation.setDeviceMetricsOverride', { width: SCENE_VIEW_START.width, height: SCENE_VIEW_START.height, deviceScaleFactor: 1, mobile: false });
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
    out.engineMemoryBytes = snap?.engineMemoryBytes ?? null;
    out.fetches = fetchesOf(snap?.resources ?? [], a.byUrl);
    out.reads = (snap?.imageReads ?? []).map((r) => ({ name: r.name, atMs: num(r.atMs), requested: r.requested === true }));
    if (!snap) out.error = `window.__app not readable within ${WAIT_LIMIT_MS} ms`;
    else if (snap.error) out.error = `${snap.error.what} [${snap.error.where}]`;
    else if (snap.state !== 'engine-idle') out.error = `state ${snap.state} at the end (no engine-idle within ${WAIT_LIMIT_MS} ms)`;
    if (snap?.state === 'engine-idle' && out.elapsedMs === null) out.error = 'the engine is idle but the page has no idle mark';
    if (snap?.state === 'engine-idle') {
      // host-extra/resources is restored only by the resources task; the same file (the installer's, the same SHA-256) is in the host tree of the seed
      const extraPath = path.join(bundleDir, '..', '..', '..', 'host-extra', 'resources', ...SCENE_FILE);
      const seedPath = path.join(bundleDir, '..', '..', '..', 'host', 'lift', 'out-idle1', 'inputs', 'install', 'Data', 'Demo projects', 'NewStuff.flp');
      const projectPath = fs.existsSync(extraPath) ? extraPath : seedPath;
      // rssMb (Issue #1379): the largest sum of the browser's VmRSS, sampled now and every second until the scene has ended (the memory the engine uses, not the address space)
      let rssKiB = null;
      const sample = () => { const k = browserRssKiB(chrome.proc.pid); if (k > 0 && (rssKiB === null || k > rssKiB)) rssKiB = k; };
      sample();
      const sampler = setInterval(sample, 1000);
      let sc;
      try {
        sc = await runScene({ S, ev, cdp, browserContextId, projectPath, shotPath: path.join(repo, '.tmp-delivery-scene-fail.png') });
      } finally {
        clearInterval(sampler);
        sample();
        out.rssMb = rssKiB === null ? null : Math.round((rssKiB / 1024) * 10) / 10;
      }
      out.scene = { steps: sc.steps, error: sc.error, state: sc.state, welcome: sc.welcome, workers: sc.workers, logTail: sc.logTail, logEnd: sc.logEnd, exit: sc.exit };
      if (sc.reads) out.reads = sc.reads;
      process.stderr.write(`scene: ${sc.steps.join('; ')}${sc.error ? ` ERROR ${sc.error}` : ''}\n`);
    }
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
