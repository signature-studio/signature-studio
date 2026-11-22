// feat-c1-hidpi: the HiDPI measurement (C-1). Boots the bundle the way tools/boot-check/boot-bundle.mjs does (headless Chrome, a new profile, the page at "/", no page parameters), once for each
// device pixel ratio 1, 1.5 and 2, with the window fixed at 1580 x 957 CSS px. After the first engine-idle it waits a few seconds (the desk counters are sent every 100 ms while the picture moves)
// and prints ONE JSON line per run on stdout:
//   {"dpr","state","standIn","error","elapsedMs","canvasW","canvasH","cssW","cssH","composeMs","putMs","presents"}
// canvasW / canvasH: the width and height of canvas[data-desk="1"] (the drawing buffer); cssW / cssH: its getBoundingClientRect (CSS px). composeMs, putMs, presents: window.__app.deskStats
// (cumulative since the engine started; one present costs composeMs / presents). dpr: window.devicePixelRatio of the page when it was read.
// The ratio is set by --force-device-scale-factor. When the page does not get that ratio (or the window is not 1580 x 957), Emulation.setDeviceMetricsOverride is used instead; stderr says which.
// Exit 1 when a run failed ("error" set), 2 when no bundle directory is given.
// Usage: node tools/feat-c1-hidpi/measure.mjs <absolute path to the bundle directory raw/<name>> > measure.jsonl   (needs app/node_modules and app/dist)
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';

if (!process.argv[2]) {
  process.stderr.write('usage: node tools/feat-c1-hidpi/measure.mjs <absolute path to the bundle directory>\n');
  process.exit(2);
}
const bundleDir = path.resolve(process.argv[2]);
const BUNDLE = path.basename(bundleDir);
const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const DPRS = [1, 1.5, 2];
const WIN = { width: 1580, height: 957 };
const WAIT_LIMIT_MS = 180000;
const POLL_MS = 1000;
const SETTLE_S = 3;

// the isolation flags of tools/boot-check/boot-bundle.mjs (the same launch)
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

const VIEW = `({ dpr: window.devicePixelRatio, w: window.innerWidth, h: window.innerHeight })`;
const POLL = `(() => { const a = window.__app; if (!a) return null; return { state: a.state }; })()`;
const MEASURE = `(() => {
  const a = window.__app ?? null;
  const c = document.querySelector('canvas[data-desk="1"]');
  const r = c ? c.getBoundingClientRect() : null;
  const d = (a && a.deskStats) || {};
  return {
    dpr: window.devicePixelRatio,
    state: a ? a.state : null,
    standIn: document.body.innerText.includes('STAND-IN'),
    canvasW: c ? c.width : null,
    canvasH: c ? c.height : null,
    cssW: r ? r.width : null,
    cssH: r ? r.height : null,
    composeMs: d.composeMs ?? null,
    putMs: d.putMs ?? null,
    presents: d.presents ?? null,
  };
})()`;

const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};

async function runOnce(dpr, port, root, server) {
  const out = { dpr, state: null, standIn: null, error: null, elapsedMs: null, canvasW: null, canvasH: null, cssW: null, cssH: null, composeMs: null, putMs: null, presents: null };
  const profile = path.join(root, `profile-${String(dpr).replace('.', '_')}`);
  let chrome = null, cdp = null, navAt = null;
  try {
    chrome = await launchChrome({ profile, extraArgs: [...ISOLATION, `--window-size=${WIN.width},${WIN.height}`, `--force-device-scale-factor=${dpr}`] });
    cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
    await cdp.opened;
    const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const S = (method, params) => timed(cdp.send(method, params, sessionId), 20000, method);
    const ev = async (expression) => {
      const r = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.text}`);
      return r.result.value;
    };
    await S('Page.enable');
    // the ratio must be in place before the page measures its room (the boot), so it is checked on about:blank first
    const before = await ev(VIEW);
    let how = 'flag';
    if (before.dpr !== dpr || before.w !== WIN.width || before.h !== WIN.height) {
      await S('Emulation.setDeviceMetricsOverride', { width: WIN.width, height: WIN.height, deviceScaleFactor: dpr, mobile: false });
      how = 'Emulation.setDeviceMetricsOverride';
    }
    const view = await ev(VIEW);
    process.stderr.write(`dpr ${dpr}: flag gave ${JSON.stringify(before)}; ${how} used, page now ${JSON.stringify(view)}\n`);
    if (view.dpr !== dpr || view.w !== WIN.width || view.h !== WIN.height) throw new Error(`the page is ${JSON.stringify(view)}, not ${dpr} at ${WIN.width}x${WIN.height}`);

    navAt = Date.now();
    const nav = await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
    // the state the check waits for is engine-idle (boot-bundle.mjs: the state, not timeline.idle)
    let state = null;
    for (;;) {
      const p = await ev(POLL).catch(() => null);
      state = p ? p.state : null;
      if (state === 'engine-idle' || state === 'error' || state === 'exited') break;
      if (Date.now() - navAt >= WAIT_LIMIT_MS) break;
      await sleep(POLL_MS);
    }
    out.elapsedMs = Date.now() - navAt;
    if (state !== 'engine-idle') throw new Error(`state ${state} before the first engine-idle (no engine-idle within ${WAIT_LIMIT_MS} ms)`);
    for (let i = 0; i < SETTLE_S; i++) await sleep(1000);
    Object.assign(out, await ev(MEASURE));
    if (out.canvasW === null || out.cssW === null) throw new Error('no canvas[data-desk="1"] in the page');
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
  } finally {
    if (navAt !== null && out.elapsedMs === null) out.elapsedMs = Date.now() - navAt;
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
  }
  return out;
}

async function main() {
  const root = fs.mkdtempSync(path.join(repo, '.tmp-c1-'));
  const assets = path.join(root, 'assets');
  let server = null, failed = false;
  try {
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    for (const dpr of DPRS) {
      const r = await runOnce(dpr, port, root, server);
      if (r.error) failed = true;
      process.stdout.write(JSON.stringify(r) + '\n');
    }
  } catch (e) {
    failed = true;
    process.stderr.write(`measure: ${String(e?.message ?? e).slice(0, 500)}\n`);
    process.stdout.write(JSON.stringify({ dpr: null, state: null, standIn: null, error: String(e?.message ?? e).slice(0, 500), elapsedMs: null, canvasW: null, canvasH: null, cssW: null, cssH: null, composeMs: null, putMs: null, presents: null }) + '\n');
  } finally {
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
  process.exitCode = failed ? 1 : 0;
}

await main();
