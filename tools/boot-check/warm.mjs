// Warm-visit boot check: 5 iterations in one run. Each iteration starts Chrome with a new profile, opens "/" (no parameters) in the default
// browsing context, closes that tab, opens "/" again in the same profile, and reads window.__app on both visits. Prints one JSON array on stdout.
// Exit 1 when any visit failed ("error" set) or an iteration is missing.
// Usage: node tools/boot-check/warm.mjs > warm.json   (no arguments; needs app/node_modules and app/dist). The page is opened at "/" with no parameters.
// standIn is only whether the text "STAND-IN" is in the page body; it is a reading, not a proof either way.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from './cdp.mjs';

const BUNDLE = 'fl-engine-ci-0363b050';
const ITERATIONS = 5;
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const bundleDir = path.join(repo, 'artifacts', 'bundles', BUNDLE);
const WAIT_LIMIT_MS = 180000;
const POLL_MS = 1000;
const CDP_TIMEOUT_MS = 20000;

// the isolation flags of boot.mjs (from archive/e2e-old/e2e/browser.mjs)
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

async function visit(cdp, url) {
  const out = { state: null, engineKind: null, timelineIdle: null, engineLog30: null, standIn: null, documentTitle: null, elapsedMs: null, error: null };
  let targetId = null, navAt = null;
  try {
    ({ targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' }));
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const S = (method, params) => timed(cdp.send(method, params, sessionId), CDP_TIMEOUT_MS, method);
    const ev = async (expression) => {
      const r = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.text}`);
      return r.result.value;
    };
    await S('Page.enable');
    navAt = Date.now();
    const nav = await S('Page.navigate', { url });
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
    if (targetId !== null) await cdp.send('Target.closeTarget', { targetId }).catch(() => {});
  }
  return out;
}

async function iteration(i, url, root) {
  const out = { iteration: i, chromeVersion: null, visit1: null, visit2: null, loadavg: null, error: null };
  const profile = path.join(root, `profile-${i}`);
  let chrome = null, cdp = null;
  try {
    chrome = await launchChrome({ profile, extraArgs: ISOLATION });
    out.chromeVersion = chrome.version.Browser;
    cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
    await cdp.opened;
    out.visit1 = await visit(cdp, url);
    out.visit2 = await visit(cdp, url);
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
  } finally {
    out.loadavg = os.loadavg();
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
    fs.rmSync(profile, { recursive: true, force: true });
  }
  return out;
}

async function main() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'warm-'));
  const assets = path.join(root, 'assets');
  const results = [];
  let server = null;
  try {
    fs.mkdirSync(assets);
    fs.symlinkSync(bundleDir, path.join(assets, BUNDLE));
    fs.copyFileSync(path.join(bundleDir, 'manifest.json'), path.join(assets, 'manifest.json'));
    process.env.FL_ASSETS_DIR = assets;

    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    const url = `http://127.0.0.1:${port}/`;
    for (let i = 1; i <= ITERATIONS; i++) results.push(await iteration(i, url, root));
  } catch (e) {
    results.push({ iteration: null, error: String(e?.message ?? e).slice(0, 500) });
  } finally {
    if (server) await server.close().catch(() => {});
    try { fs.unlinkSync(path.join(assets, BUNDLE)); } catch { /* not made */ }
    fs.rmSync(root, { recursive: true, force: true });
  }
  process.stdout.write(JSON.stringify(results, null, 2) + '\n');
  const ok = results.length === ITERATIONS && results.every((r) => !r.error && !r.visit1?.error && !r.visit2?.error);
  process.exitCode = ok ? 0 : 1;
}

await main();
