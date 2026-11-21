// feat-c2-render-worker: the desktop's counters for the Worker split (C-2). Boots the bundle the way tools/boot-check/boot-bundle.mjs does (headless Chrome, a new profile, the page at "/"), waits for the first engine-idle,
// then samples window.__app.deskStats and state at t = 0..10 s (one sample a second) and prints ONE JSON line on stdout:
//   {"state","standIn","error","renderWorkerSeen","workers":[names of the page's Worker scripts],"samples":[{"t","presents","ticks","idleTicks","composeMs","putMs"}, ...11]}
// renderWorkerSeen: the page loaded a Worker script whose name has "render-worker" (performance resource entries, initiatorType "other"). Exit 1 when error is set, 2 when no bundle directory is given.
// Usage: node tools/feat-c2-render-worker/measure.mjs <absolute path to the bundle directory raw/<name>> > measure.json   (needs app/node_modules and app/dist)
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';

if (!process.argv[2]) {
  process.stderr.write('usage: node tools/feat-c2-render-worker/measure.mjs <absolute path to the bundle directory>\n');
  process.exit(2);
}
const bundleDir = path.resolve(process.argv[2]);
const BUNDLE = path.basename(bundleDir);
const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const WAIT_LIMIT_MS = 180000;
const POLL_MS = 1000;
const SAMPLES = 10;

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

const POLL = `(() => { const a = window.__app; if (!a) return null; return { state: a.state }; })()`;
const SAMPLE = `(() => { const a = window.__app; const d = (a && a.deskStats) || {}; return { state: a ? a.state : null, presents: d.presents ?? 0, ticks: d.ticks ?? 0, idleTicks: d.idleTicks ?? 0, composeMs: d.composeMs ?? 0, putMs: d.putMs ?? 0 }; })()`;
const PAGE = `(() => {
  const scripts = performance.getEntriesByType('resource').filter((e) => e.initiatorType === 'other').map((e) => e.name.split('/').pop().replace(/[.]js$/, ''));
  return {
    workers: scripts,
    renderWorkerSeen: scripts.some((n) => n.includes('render-worker')),
    standIn: document.body.innerText.includes('STAND-IN'),
    state: window.__app ? window.__app.state : null,
  };
})()`;

async function main() {
  const out = { state: null, standIn: null, error: null, renderWorkerSeen: false, workers: [], samples: [] };
  const root = fs.mkdtempSync(path.join(repo, '.tmp-c2-'));
  const assets = path.join(root, 'assets');
  const profile = path.join(root, 'profile');
  let server = null, chrome = null, cdp = null;
  try {
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    chrome = await launchChrome({ profile, extraArgs: ISOLATION });
    cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
    await cdp.opened;
    const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const S = (method, params) => cdp.send(method, params, sessionId);
    const ev = async (expression) => {
      const r = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.text}`);
      return r.result.value;
    };
    await S('Page.enable');
    const nav = await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
    // the state, as boot-bundle.mjs reads it: engine-idle is the state the check waits for
    const started = Date.now();
    let state = null;
    for (;;) {
      const p = await ev(POLL).catch(() => null);
      state = p ? p.state : null;
      if (state === 'engine-idle' || state === 'error' || state === 'exited') break;
      if (Date.now() - started >= WAIT_LIMIT_MS) break;
      await sleep(POLL_MS);
    }
    if (state !== 'engine-idle') out.error = `state ${state} before the first engine-idle (no engine-idle within ${WAIT_LIMIT_MS} ms)`;
    else {
      for (let t = 0; t <= SAMPLES; t++) {
        if (t > 0) await sleep(1000);
        const s = await ev(SAMPLE);
        out.samples.push({ t, presents: s.presents, ticks: s.ticks, idleTicks: s.idleTicks, composeMs: s.composeMs, putMs: s.putMs });
        out.state = s.state;
      }
    }
    const page = await ev(PAGE);
    out.state = page.state;
    out.standIn = page.standIn;
    out.renderWorkerSeen = page.renderWorkerSeen;
    out.workers = page.workers;
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
  } finally {
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
  out.nproc = os.cpus().length;
  out.loadavg = os.loadavg();
  process.stdout.write(JSON.stringify(out) + '\n');
  process.exitCode = out.error ? 1 : 0;
}

await main();
