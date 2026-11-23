// Welcome flow smoke (Issue #1357): the four ways the Welcome window is closed (the usage-data box's "No thanks", Escape on the Welcome, the Default row, the box's "I'm in") must not stop the engine.
// Each flow is a start of the bundle of its own (a new Chrome profile, a new engine), at DPR 1 in the room of 1836 x 1056 CSS px (the stage, as tools/p2-frames does), and begins at the engine's first idle.
// Usage: node tools/smoke/welcome-flow.mjs <absolute path to the bundle directory raw/<name>> [--shots <directory>] > flow.jsonl   (needs app/node_modules and app/dist, as boot-bundle.mjs does).
//   --shots DIR (a test aid): a PNG of the page at the points of each flow is written to DIR (the directory is made; nothing is removed).
// Exit 0 when the four flows ran (stdout: four lines, one per flow, in the order A, B, C, D; nothing else goes to stdout), 1 when the tool itself failed, 2 when no bundle directory is given.
// The flows (after the first idle; the usage-data box is drawn in the picture over the Welcome window, not in the window list):
//   A: "No thanks" of the box, then the Default row (in the start list, scrolled to its top and 3 notches down) is double-clicked; the project is open when the title has changed and the engine is idle again.
//   B: Escape (the Welcome window is closed; the project opens when the title has changed).
//   C: "No thanks", then the Default row is clicked once.
//   D: "I'm in" of the box. The box closes and the Welcome window stays (the window table still lists it), so the Default row is then double-clicked as in A; the project is open when the Welcome has closed.
// The places of the controls come from the window table: the Welcome window (class TWelcomeWizard) is listed at its position, and the box and the list are at offsets from its top-left corner (measured at the room of 1836 x 1056: the offsets are the picture's, not a rule of the product).
// Each JSON line: flow, error (the page's error text after the flow, or the tool's failure; null when none), state, standIn, trapped, hostErrors (HostFilesUnsupported lines of the flow), welcomeClosed (the window is not in the window table at the end),
//   opened (the title changed from the Welcome's title), elapsedMs (navigation to the first engine-idle: performance.timeOrigin + timeline.idle minus the time before Page.navigate), preBootBytes (the bytes of the start-up assets: assetsReport items),
//   and for reading: hostErrorText (the first such text, cut), imageReadsAfterIdle (the names of the real modules read after the first idle), steps (what the tool did, in order).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');

const ROOM_W = 1836, ROOM_H = 1056;
const VIEW_START = { width: 2176, height: 1092 };
const WIZARD = 'TWelcomeWizard';
// guest pixels from the Welcome window's top-left corner (its window-table position). The box's "No thanks" is at (993, 548) and "I'm in" at (841, 548) for the window at (481, 241)
const NO_THANKS = { dx: 512, dy: 307 };
const IM_IN = { dx: 360, dy: 307 };
// the start list is scrolled with the wheel over LIST: it is taken to its top (12 notches up) and then 3 notches down, where the Default row is at DEFAULT_ROW
const LIST = { dx: 235, dy: 180 };
const DEFAULT_ROW = { dx: 235, dy: 117 };
const NOTCH_UP = 12, NOTCH_DOWN = 3;
const NOTCH_DELTA = 120;
const READY_LIMIT_MS = 180000, OPEN_LIMIT_MS = 120000, POLL_MS = 500, CDP_TIMEOUT_MS = 20000;
const LOG_CHECK = /aborted|\btrap\b/i;

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

const STATE = `(() => { const a = window.__app; if (!a) return null; return { state: a.state, title: document.title, standIn: document.body.innerText.includes('STAND-IN'), error: a.error ? String(a.error.what ?? a.error.message ?? a.error).slice(0, 600) : null, logLength: a.engineLog().length }; })()`;
const READ_TIMES = `(() => { const a = window.__app; return { timeOrigin: performance.timeOrigin, idle: a.timeline.idle ?? null, bytes: (a.assetsReport()?.items ?? []).reduce((n, i) => n + i.bytes, 0), imageReads: a.imageReads.map((r) => ({ name: r.name, atMs: r.atMs })) }; })()`;
const GEOMETRY = `(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const g = window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: g.width, gh: g.height }; })()`;
const ROOM = `(() => { const s = document.getElementById('stage').getBoundingClientRect(); return { sw: s.width, sh: s.height, iw: window.innerWidth, ih: window.innerHeight }; })()`;
const WINDOWS = `(async () => (await window.__app.deskDumpBase64(false)).windows)()`;
const LOG_FROM = (n) => `window.__app.engineLog().slice(${n})`;

const log = (...a) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ')}\n`);
const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};
const round = (v) => Math.round(v);

/** the windows of a window list: { lid, x, y, w, h, cls, title } (the page's format: lid N at (x, y) WxH ... class "..." title "...") */
function parseWindows(text) {
  return text.split('\n').map((l) => /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean).map((m) => {
    const names = /class "([^"]*)" title "([^"]*)"\s*$/.exec(m[6]);
    return { lid: Number(m[1]), x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), cls: names ? names[1] : '', title: names ? names[2] : '' };
  });
}

/** one start of the bundle in a Chrome of its own: the page, the stage at 1836 x 1056 CSS px, and the calls of the flow */
class Session {
  constructor(cdp, sessionId, shots) {
    this.cdp = cdp;
    this.sessionId = sessionId;
    this.shots = shots;
  }
  send(method, params = {}) { return timed(this.cdp.send(method, params, this.sessionId), CDP_TIMEOUT_MS, method); }
  async ev(expression) {
    const r = await this.send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
    if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.exception?.description ?? r.exceptionDetails.text}`.slice(0, 300));
    return r.result.value;
  }
  async windows() {
    const t0 = Date.now();
    for (;;) {
      try { return parseWindows(await this.ev(WINDOWS)); } catch (e) {
        if (!/ask again/.test(String(e?.message ?? e)) || Date.now() - t0 >= 15000) throw e;
        await sleep(100);
      }
    }
  }
  async welcome() { return (await this.windows()).find((w) => w.cls === WIZARD) ?? null; }
  async geometry() { return this.ev(GEOMETRY); }
  async shot(name) {
    if (!this.shots) return;
    const s = await this.send('Page.captureScreenshot', { format: 'png' });
    fs.mkdirSync(this.shots, { recursive: true });
    fs.writeFileSync(path.join(this.shots, name), Buffer.from(s.data, 'base64'));
  }
  /** a click at guest pixel (gx, gy): the pointer moves there, then the button is pressed and released `clicks` times */
  async click(gx, gy, clicks = 1) {
    const g = await this.geometry();
    const c = { x: g.x + (gx + 0.5) * (g.w / g.gw), y: g.y + (gy + 0.5) * (g.h / g.gh) };
    await this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(60);
    for (let k = 1; k <= clicks; k++) {
      await this.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'left', buttons: 1, clickCount: k, pointerType: 'mouse' });
      await sleep(50);
      await this.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: 'left', buttons: 0, clickCount: k, pointerType: 'mouse' });
      await sleep(100);
    }
  }
  /** wheel notches over guest pixel (gx, gy); `deltaY` is NOTCH_DELTA (down) or -NOTCH_DELTA (up) */
  async wheel(gx, gy, notches, deltaY) {
    const g = await this.geometry();
    const c = { x: g.x + (gx + 0.5) * (g.w / g.gw), y: g.y + (gy + 0.5) * (g.h / g.gh) };
    for (let i = 0; i < notches; i++) {
      await this.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: c.x, y: c.y, deltaX: 0, deltaY, pointerType: 'mouse' });
      await sleep(300);
    }
  }
  async escape() {
    // the keys go to the page's focused element: the desktop takes them (as tools/p2-frames does)
    await this.ev("document.getElementById('desktop').focus({ preventScroll: true }); true");
    await sleep(100);
    await this.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: 'Escape', key: 'Escape', windowsVirtualKeyCode: 27, nativeVirtualKeyCode: 27 });
    await sleep(40);
    await this.send('Input.dispatchKeyEvent', { type: 'keyUp', code: 'Escape', key: 'Escape', windowsVirtualKeyCode: 27, nativeVirtualKeyCode: 27 });
  }
  state() { return this.ev(STATE); }
}

/** waits until the engine is idle with the condition true (or the engine stopped, or the limit); returns the last state */
async function untilIdle(sess, cond, ms) {
  const t0 = Date.now();
  let last = null;
  for (;;) {
    last = await sess.state().catch((e) => ({ state: 'unreadable', error: String(e?.message ?? e).slice(0, 300), title: null, standIn: null, logLength: null }));
    if (last && (last.state === 'error' || last.state === 'exited' || last.state === 'unreadable')) return last;
    if (last && last.state === 'engine-idle' && (await cond(last))) return last;
    if (Date.now() - t0 >= ms) return last;
    await sleep(POLL_MS);
  }
}

/** the first idle of one start: the room is 1836 x 1056 CSS px (calibrated as tools/p2-frames does); returns the measures taken at the first idle */
async function firstIdle(sess, navSentAt) {
  const t0 = Date.now();
  for (;;) {
    const s = await sess.state().catch(() => null);
    if (s && (s.state === 'engine-idle' || s.state === 'error' || s.state === 'exited')) break;
    if (Date.now() - t0 >= READY_LIMIT_MS) throw new Error(`no engine-idle within ${READY_LIMIT_MS} ms (state ${s ? s.state : 'window.__app not set'})`);
    await sleep(POLL_MS);
  }
  for (let i = 0; i < 4; i++) {
    const r = await sess.ev(ROOM);
    if (Math.abs(r.sw - ROOM_W) <= 0.5 && Math.abs(r.sh - ROOM_H) <= 0.5) break;
    await sess.send('Emulation.setDeviceMetricsOverride', { width: Math.round(r.iw + (ROOM_W - r.sw)), height: Math.round(r.ih + (ROOM_H - r.sh)), deviceScaleFactor: 1, mobile: false });
    await sleep(1500);
  }
  await sleep(1500);
  const st = await sess.state();
  if (st.state !== 'engine-idle') throw new Error(`state ${st.state} before the first idle: ${st.error ?? ''}`.trim());
  const t = await sess.ev(READ_TIMES);
  if (t.idle === null) throw new Error('engine-idle without the idle mark');
  return { state: st, elapsedMs: round(t.timeOrigin + t.idle - navSentAt), preBootBytes: t.bytes, idleAt: t.idle };
}

/** one flow: the start, the first idle, the steps, the end. Returns the JSON object of the flow */
async function runFlow(flow, port, shots) {
  const steps = [];
  const out = { flow, error: null, state: null, standIn: null, trapped: true, hostErrors: 0, welcomeClosed: false, opened: false, elapsedMs: null, preBootBytes: null, hostErrorText: null, imageReadsAfterIdle: [], steps };
  const root = fs.mkdtempSync(path.join(repo, '.tmp-flow-'));
  const profile = path.join(root, 'profile');
  let chrome = null, cdp = null, sess = null;
  try {
    chrome = await launchChrome({ profile, extraArgs: ISOLATION });
    cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
    await cdp.opened;
    const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    sess = new Session(cdp, sessionId, shots);
    await sess.send('Page.enable');
    await sess.send('Emulation.setDeviceMetricsOverride', { width: VIEW_START.width, height: VIEW_START.height, deviceScaleFactor: 1, mobile: false });
    const navSentAt = Date.now();
    const nav = await sess.send('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
    const start = await firstIdle(sess, navSentAt);
    out.elapsedMs = start.elapsedMs;
    out.preBootBytes = start.preBootBytes;
    const titleBefore = start.state.title;
    const logFrom = start.state.logLength;
    steps.push(`first idle at ${round(start.idleAt)} ms (title "${titleBefore}")`);
    await sess.shot(`${flow}-0-first-idle.png`);
    const wiz0 = await sess.welcome();
    if (!wiz0) throw new Error('no Welcome window at the first idle');
    steps.push(`Welcome window at (${wiz0.x}, ${wiz0.y}) ${wiz0.w}x${wiz0.h}`);

    const noThanks = async () => {
      const w = await sess.welcome();
      if (!w) throw new Error('the Welcome window is gone before "No thanks"');
      await sess.click(w.x + NO_THANKS.dx, w.y + NO_THANKS.dy);
      steps.push(`No thanks at guest (${w.x + NO_THANKS.dx}, ${w.y + NO_THANKS.dy})`);
      await sleep(2500);
      await sess.shot(`${flow}-1-after-no-thanks.png`);
    };
    const openDefault = async (clicks) => {
      const w = await sess.welcome();
      if (!w) throw new Error('the Welcome window is gone before the Default row');
      await sess.wheel(w.x + LIST.dx, w.y + LIST.dy, NOTCH_UP, -NOTCH_DELTA);
      await sess.wheel(w.x + LIST.dx, w.y + LIST.dy, NOTCH_DOWN, NOTCH_DELTA);
      await sleep(400);
      await sess.shot(`${flow}-2-default-row.png`);
      await sess.click(w.x + DEFAULT_ROW.dx, w.y + DEFAULT_ROW.dy, clicks);
      steps.push(`Default row at guest (${w.x + DEFAULT_ROW.dx}, ${w.y + DEFAULT_ROW.dy}), ${clicks} click(s)`);
    };

    if (flow === 'A') {
      await noThanks();
      await untilIdle(sess, async () => true, OPEN_LIMIT_MS);
      await openDefault(2);
    } else if (flow === 'B') {
      await sess.escape();
      steps.push('Escape');
    } else if (flow === 'C') {
      await noThanks();
      await untilIdle(sess, async () => true, OPEN_LIMIT_MS);
      await openDefault(1);
    } else if (flow === 'D') {
      // "I'm in" closes the box only: the Welcome window is still listed after it (measured: the window table and the picture), and it closes when the Default row opens the project
      const w = wiz0;
      await sess.click(w.x + IM_IN.dx, w.y + IM_IN.dy);
      steps.push(`I'm in at guest (${w.x + IM_IN.dx}, ${w.y + IM_IN.dy})`);
      await sleep(2500);
      await sess.shot(`${flow}-1-after-im-in.png`);
      const still = await sess.welcome();
      steps.push(`after I'm in the Welcome window is ${still ? 'still listed' : 'gone'}`);
      if (still) await openDefault(2);
    } else {
      throw new Error(`no flow ${flow}`);
    }

    // the end: the engine is idle again with the project open, or it stopped (or the limit)
    const welcomeGone = async () => !(await sess.welcome());
    const last = await untilIdle(sess, async (s) => s.title !== titleBefore && (await welcomeGone()), OPEN_LIMIT_MS);
    await sleep(2000);
    const end = await sess.state().catch((e) => ({ state: 'unreadable', error: String(e?.message ?? e).slice(0, 300), title: null, standIn: null, logLength: null }));
    await sess.shot(`${flow}-3-end.png`);
    const lines = await sess.ev(LOG_FROM(logFrom)).catch(() => []);
    const reads = await sess.ev(READ_TIMES).catch(() => null);
    const wins = await sess.windows().catch(() => null);   // null: the window table cannot be read (the engine is stopped): the Welcome window is not known to be closed
    const hostLines = lines.filter((l) => /HostFilesUnsupported/.test(l));
    const errText = end.error ?? last?.error ?? null;
    if (errText && /HostFilesUnsupported/.test(errText)) hostLines.push(errText);
    out.error = end.error ?? (end.state === 'engine-idle' ? null : `state ${end.state}`);
    out.state = end.state;
    out.standIn = end.standIn;
    out.trapped = end.state !== 'engine-idle' || lines.some((l) => LOG_CHECK.test(l));
    out.hostErrors = hostLines.length;
    out.hostErrorText = hostLines.length ? String(hostLines[0]).slice(0, 400) : null;
    out.welcomeClosed = wins !== null && !wins.some((w) => w.cls === WIZARD);
    out.opened = end.title !== null && end.title !== titleBefore;
    out.imageReadsAfterIdle = reads ? reads.imageReads.filter((r) => r.atMs > start.idleAt).map((r) => r.name) : [];
    steps.push(`end: state ${end.state}, title "${end.title}", welcome window ${out.welcomeClosed ? 'closed' : 'open'}`);
  } catch (e) {
    out.error = out.error ?? String(e?.message ?? e).slice(0, 600);
    steps.push(`failed: ${String(e?.message ?? e).slice(0, 300)}`);
  } finally {
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
    fs.rmSync(root, { recursive: true, force: true });
  }
  log(`flow ${flow}:`, { error: out.error, state: out.state, trapped: out.trapped, hostErrors: out.hostErrors, welcomeClosed: out.welcomeClosed, opened: out.opened, elapsedMs: out.elapsedMs, preBootBytes: out.preBootBytes });
  for (const s of steps) log(`  ${flow}: ${s}`);
  return out;
}

async function main() {
  const argv = process.argv.slice(2);
  const shotsAt = argv.indexOf('--shots');
  const shots = shotsAt >= 0 ? path.resolve(argv[shotsAt + 1] ?? '') : null;
  const positional = argv.filter((a, i) => !a.startsWith('--') && (shotsAt < 0 || i !== shotsAt + 1));
  if (positional.length !== 1 || (shotsAt >= 0 && !argv[shotsAt + 1])) {
    process.stderr.write('usage: node tools/smoke/welcome-flow.mjs <absolute path to the bundle directory raw/<name>> [--shots <directory>]\n');
    process.exit(2);
  }
  const bundleDir = path.resolve(positional[0]);
  const BUNDLE = path.basename(bundleDir);
  const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
  const root = fs.mkdtempSync(path.join(repo, '.tmp-flow-assets-'));
  const assets = path.join(root, 'assets');
  let server = null;
  const results = [];
  try {
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    for (const flow of ['A', 'B', 'C', 'D']) results.push(await runFlow(flow, port, shots));
  } finally {
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
  for (const r of results) process.stdout.write(JSON.stringify(r) + '\n');
  log('loadavg', os.loadavg(), 'nproc', os.availableParallelism());
}

// the flows run when this file is the program
const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) {
  await main().catch((e) => {
    process.stderr.write(`${String(e?.stack ?? e)}\n`);
    process.exitCode = 1;
  });
}
