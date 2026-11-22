// P2 measure (#1307): how many frames the page draws per second with the UI moving, and the input -> update p95 (the page's own number, decision 0202), for a bundle directory served as boot-bundle.mjs serves it.
// Usage: node tools/p2-frames/measure.mjs <absolute path to the bundle directory raw/<name>> > p2.json   (needs app/node_modules and app/dist). Exit 0 when error is null, 1 when the run failed, 2 when no bundle directory is given.
// stdout: ONE JSON line: state, standIn, error, frames, rafFps, rafIntervalP95Ms, inputLatencyP95Ms, presentsPerS, loadavg, nproc. Nothing else goes to stdout.
// Order: the bundle is staged and served (stage-assets.mjs, vite preview); headless Chrome starts with the boot-check isolation flags and FRAME_FLAGS (the frame-rate limits off: the page is not held at the display's 60 Hz);
// the engine reaches engine-idle; the welcome wizard is closed with its X (as sdriveg2-lat.mjs does), so the File menu gets the clicks; then, once the engine is idle again, for WINDOW_MS on the page's own clock:
// the page counts its requestAnimationFrame callbacks (frames, rafFps = frames / the span, rafIntervalP95Ms), and the input script (a click on the File menu, then Escape, again and again) runs.
// presentsPerS: the desk presenter's counter (window.__app.deskStats.presents) over the same span. With this input script it follows the input rate (one click and one Escape per cycle), so it is not the engine's capacity.
// inputLatencyP95Ms: window.__app.perf.snapshot().input.eventToRaf.p95Ms (the last 20 inputs; null when no input got an answer).
// Not decided here: whether 120 fps is met (#1308 decides it). The values are the runner's, not this machine's.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';

if (!process.argv[2]) {
  process.stderr.write('usage: node tools/p2-frames/measure.mjs <absolute path to the bundle directory raw/<name>>\n');
  process.exit(2);
}
const bundleDir = path.resolve(process.argv[2]);
const BUNDLE = path.basename(bundleDir);
const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');

const VIEW = { width: 1920, height: 1080 };
const WINDOW_MS = 10000;
const READY_LIMIT_MS = 180000;
const IDLE_LIMIT_MS = 30000;
const CDP_TIMEOUT_MS = 20000;
const POLL_MS = 250;
const SETTLE_MS = 250;
const FILE_MENU = { x: 18, y: 20 };
// the switches that lift the frame-count limit of headless Chrome 157 on the runner. Blank page, rAF over 5 s: none 60 fps; --disable-frame-rate-limit + --disable-gpu-vsync 58; --run-all-compositor-stages-before-draw alone 60;
// --disable-gpu-vsync + --run-all 60; --disable-frame-rate-limit + --run-all about 15000 per second. Only the last pair lifts the cap. Its callbacks are the page's rAF throughput, not frames the display shows, and the loop itself keeps a core busy.
const FRAME_FLAGS = ['--disable-frame-rate-limit', '--run-all-compositor-stages-before-draw'];

// the isolation flags of boot-bundle.mjs (copied: the same Chrome as the boot check, without its net-log)
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

const STATE = `(() => { const a = window.__app; if (!a) return null; return { state: a.state, error: a.error ? String(a.error.message ?? a.error).slice(0, 300) : null }; })()`;
const GEOMETRY = `(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const g = window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: g.width, gh: g.height }; })()`;
// the window list of the compositor (the dump composes from the engine's memory; peek would read the WebGL scene back, which fails in this page)
const WINDOWS = `(async () => (await window.__app.deskDumpBase64(false)).windows)()`;
const PRESENTS = `(() => { const s = window.__app.deskStats; return s && typeof s.presents === 'number' ? s.presents : null; })()`;
// the frames of the page: one callback per requestAnimationFrame, the clock is the page's own; resolves after WINDOW_MS
const RAF_SPAN = `new Promise((resolve) => { const t0 = performance.now(); const ts = []; const frame = () => { ts.push(performance.now()); if (ts[ts.length - 1] - t0 >= ${WINDOW_MS}) resolve({ t0, ts }); else requestAnimationFrame(frame); }; requestAnimationFrame(frame); })`;
const INPUT_P95 = `(() => { const e = window.__app.perf.snapshot().input.eventToRaf; return typeof e.p95Ms === 'number' ? e.p95Ms : null; })()`;

const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};

// nearest rank, as the page's own percentile (app/src/ui/latency.ts percentile)
const percentile = (sorted, p) => (sorted.length === 0 ? null : sorted[Math.min(sorted.length - 1, Math.max(0, Math.ceil((p / 100) * sorted.length) - 1))]);

function parseWindows(text) {
  return text.split('\n').map((l) => /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean).map((m) => {
    const names = /class "([^"]*)" title "([^"]*)"\s*$/.exec(m[6]);
    return { cls: names ? names[1] : '', x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]) };
  });
}

async function main() {
  const out = { state: null, standIn: null, error: null, frames: null, rafFps: null, rafIntervalP95Ms: null, inputLatencyP95Ms: null, presentsPerS: null, loadavg: null, nproc: null };
  const root = fs.mkdtempSync(path.join(repo, '.tmp-p2-'));
  const assets = path.join(root, 'assets');
  const profile = path.join(root, 'profile');
  let server = null, chrome = null, cdp = null;
  try {
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });

    chrome = await launchChrome({ profile, extraArgs: [...ISOLATION, ...FRAME_FLAGS] });
    cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
    await cdp.opened;
    const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const S = (method, params, ms = CDP_TIMEOUT_MS) => timed(cdp.send(method, params, sessionId), ms, method);
    const ev = async (expression, ms = CDP_TIMEOUT_MS) => {
      const r = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }, ms);
      if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.exception?.description ?? r.exceptionDetails.text}`.slice(0, 300));
      return r.result.value;
    };
    await S('Page.enable');
    await S('Emulation.setDeviceMetricsOverride', { width: VIEW.width, height: VIEW.height, deviceScaleFactor: 1, mobile: false });
    const nav = await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);

    // the engine's idle: returns the state when it is idle (or after the limit, with the state it has); throws when the engine stopped
    const waitIdle = async (ms) => {
      const t0 = Date.now();
      for (;;) {
        const s = await ev(STATE).catch(() => null);
        if (s && (s.state === 'error' || s.state === 'exited')) throw new Error(`state ${s.state} ${s.error ?? ''}`.trim());
        if (s && s.state === 'engine-idle') return s.state;
        if (Date.now() - t0 >= ms) return s ? s.state : null;
        await sleep(POLL_MS);
      }
    };
    const first = await waitIdle(READY_LIMIT_MS);
    if (first !== 'engine-idle') throw new Error(`no engine-idle within ${READY_LIMIT_MS} ms (state ${first ?? 'window.__app not set'})`);

    const click = async (sx, sy, geo) => {
      const c = { x: geo.x + (sx + 0.5) * (geo.w / geo.gw), y: geo.y + (sy + 0.5) * (geo.h / geo.gh) };
      await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
      await sleep(60);
      await S('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
      await sleep(50);
      await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
    };
    const escape = async () => {
      await S('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: 'Escape', key: 'Escape', windowsVirtualKeyCode: 27, nativeVirtualKeyCode: 27 });
      await sleep(40);
      await S('Input.dispatchKeyEvent', { type: 'keyUp', code: 'Escape', key: 'Escape', windowsVirtualKeyCode: 27, nativeVirtualKeyCode: 27 });
    };

    // the welcome wizard, if the page shows it: its X, then wait until it is gone (not a second try)
    const wiz = parseWindows(await ev(WINDOWS)).find((w) => w.cls === 'TWelcomeWizard');
    if (wiz) {
      await click(wiz.x + wiz.w - 18, wiz.y + 18, await ev(GEOMETRY));
      const t0 = Date.now();
      while (parseWindows(await ev(WINDOWS)).some((w) => w.cls === 'TWelcomeWizard')) {
        if (Date.now() - t0 >= 15000) throw new Error('the welcome wizard did not close');
        await sleep(POLL_MS);
      }
    }
    await sleep(1500);
    if ((await waitIdle(IDLE_LIMIT_MS)) !== 'engine-idle') throw new Error('the engine did not return to engine-idle after the wizard');

    // the measured span starts from the engine's idle
    const geo = await ev(GEOMETRY);
    const t0 = Date.now();
    const p0 = await ev(PRESENTS);
    let rafEnded = false;
    const raf = ev(RAF_SPAN, WINDOW_MS + CDP_TIMEOUT_MS).then((v) => ({ v }), (e) => ({ e })).finally(() => { rafEnded = true; });
    while (!rafEnded) {
      await click(FILE_MENU.x, FILE_MENU.y, geo);
      await sleep(300);
      if (rafEnded) break;
      await escape();
      await sleep(300);
    }
    const rafRes = await raf;
    if (rafRes.e) throw new Error(`frames: ${rafRes.e.message ?? rafRes.e}`.slice(0, 300));
    const { t0: pageT0, ts } = rafRes.v;
    const spanMs = ts[ts.length - 1] - pageT0;
    const gaps = ts.slice(1).map((t, i) => t - ts[i]).sort((a, b) => a - b);
    out.frames = ts.length;
    out.rafFps = +(ts.length / (spanMs / 1000)).toFixed(2);
    if (gaps.length === 0) throw new Error(`frames: ${ts.length} callback(s) in the span`);
    out.rafIntervalP95Ms = +percentile(gaps, 95).toFixed(3);

    await sleep(SETTLE_MS);
    const p1 = await ev(PRESENTS);
    const elapsedS = (Date.now() - t0) / 1000;
    out.presentsPerS = p0 === null || p1 === null ? null : +((p1 - p0) / elapsedS).toFixed(2);

    // the last inputs are answered in the engine's idle; then the page's own input -> update numbers
    const after = await waitIdle(IDLE_LIMIT_MS);
    out.inputLatencyP95Ms = await ev(INPUT_P95);
    out.state = after;
    out.standIn = await ev(`document.body.innerText.includes('STAND-IN')`);
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
  } finally {
    out.loadavg = os.loadavg();
    out.nproc = os.availableParallelism();
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
  process.stdout.write(JSON.stringify(out) + '\n');
  process.exitCode = out.error ? 1 : 0;
}

await main();
