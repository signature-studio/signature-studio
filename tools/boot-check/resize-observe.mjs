// Resize observe (copy of boot-bundle.mjs): runs the real engine in headless Chrome, leaves the Welcome wizard open, shrinks the viewport with CDP Emulation (a display change), waits, reads the engine state, restores the viewport, waits, reads again. Prints one JSON object on stdout. Exit 1 only when the page did not reach the first idle; an engine that stops after the shrink is recorded in "state" and is not an exit error.
// Usage: node tools/boot-check/resize-observe.mjs <absolute path to the bundle directory> > resize.json   (needs app/node_modules and app/dist). The page is opened at "/" with no parameters.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from './cdp.mjs';

if (!process.argv[2]) {
  process.stderr.write('usage: node tools/boot-check/resize-observe.mjs <absolute path to the bundle directory>\n');
  process.exit(2);
}
const bundleDir = path.resolve(process.argv[2]);
const BUNDLE = path.basename(bundleDir);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const WAIT_LIMIT_MS = 180000;
const POLL_MS = 1000;
const CDP_TIMEOUT_MS = 20000;
const START = { width: 2260, height: 1116 };
const SHRUNK = { width: 1500, height: 900 };
const SETTLE_MS = 6000;

// the isolation flags of archive/e2e-old/e2e/browser.mjs, without the net-log and the crash folder
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

const POLL = `(() => { const a = window.__app; if (!a) return null; return { state: a.state, idle: typeof a.timeline.idle === 'number' ? a.timeline.idle : null }; })()`;
const READ = `(() => { const a = window.__app ?? null; const log = a ? a.engineLog() : null; return {
  state: a ? a.state : null,
  engineKind: a ? a.engineKind : null,
  timelineIdle: a && typeof a.timeline.idle === 'number' ? a.timeline.idle : null,
  engineStatus: a ? a.engineStatus().text : null,
  logLength: log ? log.length : null,
  viewport: { w: window.innerWidth, h: window.innerHeight },
}; })()`;
const LOG = `(() => { const a = window.__app ?? null; return a ? a.engineLog() : null; })()`;

const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};

async function main() {
  const t0 = Date.now();
  const out = { startedAt: new Date(t0).toISOString(), chromeVersion: null, engineKind: null, timelineIdle: null, viewport: {}, state: {}, engineStatus: {}, logLengthAtShrink: null, logLengthAfterShrink: null, restored: null, callbackNestingLines: null, logAroundShrink: null, engineLogAll: null, loadavg: null, elapsedMs: null, error: null };
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'resize-observe-'));
  const assets = path.join(root, 'assets');
  const profile = path.join(root, 'profile');
  let server = null, chrome = null, cdp = null, navAt = null;
  try {
    fs.mkdirSync(assets);
    fs.symlinkSync(bundleDir, path.join(assets, BUNDLE));
    fs.copyFileSync(path.join(bundleDir, 'manifest.json'), path.join(assets, 'manifest.json'));
    process.env.FL_ASSETS_DIR = assets;

    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });

    chrome = await launchChrome({ profile, extraArgs: ISOLATION });
    out.chromeVersion = chrome.version.Browser;
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
    await S('Emulation.setDeviceMetricsOverride', { width: START.width, height: START.height, deviceScaleFactor: 1, mobile: false });

    navAt = Date.now();
    const nav = await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
    for (;;) {
      const p = await ev(POLL).catch(() => null);
      if (p && (p.idle !== null || p.state === 'error' || p.state === 'exited')) break;
      if (Date.now() - navAt >= WAIT_LIMIT_MS) break;
      await sleep(POLL_MS);
    }
    await sleep(SETTLE_MS);

    const before = await ev(READ);
    out.engineKind = before.engineKind;
    out.timelineIdle = before.timelineIdle;
    out.viewport.beforeShrink = before.viewport;
    out.state.beforeShrink = before.state;
    out.engineStatus.beforeShrink = before.engineStatus;
    out.logLengthAtShrink = before.logLength;
    if (before.timelineIdle === null) {
      out.error = `state ${before.state} before the first idle`;
    } else {
      await S('Emulation.setDeviceMetricsOverride', { width: SHRUNK.width, height: SHRUNK.height, deviceScaleFactor: 1, mobile: false });
      await sleep(SETTLE_MS);
      const shrunk = await ev(READ);
      out.viewport.afterShrink = shrunk.viewport;
      out.state.afterShrink = shrunk.state;
      out.engineStatus.afterShrink = shrunk.engineStatus;
      out.logLengthAfterShrink = shrunk.logLength;
      if (shrunk.state === 'error' || shrunk.state === 'exited') {
        out.restored = 'skipped: the engine stopped after the shrink';
      } else {
        await S('Emulation.clearDeviceMetricsOverride', {});
        await sleep(SETTLE_MS);
        const restored = await ev(READ);
        out.viewport.afterRestore = restored.viewport;
        out.state.afterRestore = restored.state;
        out.engineStatus.afterRestore = restored.engineStatus;
        out.restored = 'done';
      }
    }

    const log = await ev(LOG);
    if (Array.isArray(log)) {
      const at = out.logLengthAtShrink ?? log.length;
      out.callbackNestingLines = { count: log.filter((l) => l.includes('callback nesting')).length, lines: log.filter((l) => l.includes('callback nesting')) };
      out.logAroundShrink = { before: log.slice(Math.max(0, at - 10), at), after: log.slice(at, at + 10) };
      out.engineLogAll = log;
    }
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
  } finally {
    if (navAt !== null) out.elapsedMs = Date.now() - navAt;
    out.loadavg = os.loadavg();
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
    if (server) await server.close().catch(() => {});
    try { fs.unlinkSync(path.join(assets, BUNDLE)); } catch { /* not made */ }
    fs.rmSync(root, { recursive: true, force: true });
  }
  process.stdout.write(JSON.stringify(out, null, 2) + '\n');
  process.exitCode = out.error ? 1 : 0;
}

await main();
