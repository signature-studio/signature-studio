// Boot check: runs the E13 bundle once in headless Chrome (a new profile) and prints one JSON object on stdout. Exit 1 when the run failed ("error" set).
// Usage: node tools/boot-check/boot.mjs > boot.json   (no arguments; needs app/node_modules and app/dist). The page is opened at "/" with no parameters.
// standIn is only whether the text "STAND-IN" is in the page body; it is a reading, not a proof either way.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from './cdp.mjs';

const BUNDLE = 'fl-engine-ci-0363b050';
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const bundleDir = path.join(repo, 'artifacts', 'bundles', BUNDLE);
const WAIT_LIMIT_MS = 180000;
const POLL_MS = 1000;
const CDP_TIMEOUT_MS = 20000;

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
const FINAL = `(() => { const a = window.__app ?? null; return {
  state: a ? a.state : null,
  engineKind: a ? a.engineKind : null,
  timelineIdle: a && typeof a.timeline.idle === 'number' ? a.timeline.idle : null,
  engineLog30: a ? a.engineLog().slice(0, 30) : null,
  documentTitle: document.title,
  standIn: document.body.innerText.includes('STAND-IN'),
}; })()`;

const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};

async function main() {
  const t0 = Date.now();
  const out = { startedAt: new Date(t0).toISOString(), state: null, engineKind: null, timelineIdle: null, engineLog30: null, standIn: null, documentTitle: null, elapsedMs: null, loadavg: null, chromeVersion: null, error: null };
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'boot-check-'));
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

    navAt = Date.now();
    const nav = await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
    let stopped = false;
    for (;;) {
      const p = await ev(POLL).catch(() => null);
      if (p && (p.idle !== null || p.state === 'error' || p.state === 'exited')) { stopped = true; break; }
      if (Date.now() - navAt >= WAIT_LIMIT_MS) break;
      await sleep(POLL_MS);
    }
    Object.assign(out, await ev(FINAL));
    if (out.timelineIdle === null && !stopped) out.error = `no first idle, error or exit within ${WAIT_LIMIT_MS} ms (state ${out.state ?? 'window.__app not set'})`;
    else if (out.timelineIdle === null) out.error = `state ${out.state} before the first idle`;
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
