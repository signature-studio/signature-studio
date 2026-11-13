// G observation (ROADMAP R5-1): runs the E13 bundle once in headless Chrome, the way boot.mjs does (a new profile, "/" with no parameters, the same isolation flags),
// and reads what the real engine printed and drew: every line of the engine log (into evidence/R5-1/engine-log-<stamp>.txt), the window table the engine printed,
// document.title and the page's title, the desktop canvas's box, the idle reports, the canvas read back in the Worker (a SHA-256), and a screenshot of the canvas box
// taken as soon as the first idle shows (evidence/R5-1/g4-<stamp>.png). Prints one JSON object on stdout. Exit 1 when the run failed ("error" set).
// Usage: node tools/boot-check/g-observe.mjs > evidence/R5-1/run-1.json   (no arguments; needs app/node_modules and app/dist). It prints what it read and judges nothing.
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
const outDir = path.join(repo, 'evidence', 'R5-1');
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
const FINAL = `(() => { const a = window.__app ?? null; const c = document.querySelector('#desktop canvas'); const r = c ? c.getBoundingClientRect() : null; return {
  state: a ? a.state : null,
  engineKind: a ? a.engineKind : null,
  timelineIdle: a && typeof a.timeline.idle === 'number' ? a.timeline.idle : null,
  timeline: a ? a.timeline : null,
  engineLog: a ? a.engineLog() : null,
  windowTable: a && a.summary ? a.summary.windows : null,
  idles: a ? { count: a.idles.length, first: a.idles[0] ?? null, last: a.idles[a.idles.length - 1] ?? null } : null,
  pageTitle: a ? a.pageTitle() : null,
  documentTitle: document.title,
  standIn: document.body.innerText.includes('STAND-IN'),
  canvas: c && r ? { count: document.querySelectorAll('#desktop canvas').length, width: c.width, height: c.height, rect: { x: r.x, y: r.y, width: r.width, height: r.height } } : null,
  pageMs: performance.now(),
}; })()`;
const READBACK = `window.__app.readbackDesk().then((r) => r, (e) => ({ error: String((e && e.message) || e) }))`;
// the compositor's picture (RGBA, as the engine holds it; peek = no side effect). Its non-background pixel count is read here; the background is the most frequent colour
const DESK = `window.__app.deskDumpBase64(true).then((d) => ({ width: d.width, height: d.height, base64: d.base64, windows: d.windows }), (e) => ({ error: String((e && e.message) || e) }))`;

const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};

async function main() {
  const t0 = Date.now();
  const out = { startedAt: new Date(t0).toISOString(), state: null, engineKind: null, timelineIdle: null, timeline: null, engineLogLines: null, engineLogFile: null, windowTable: null, idles: null, pageTitle: null, documentTitle: null, standIn: null, canvas: null, pageMs: null, screenshot: null, readback: null, chromeVersion: null, elapsedMs: null, loadavg: null, error: null };
  const stamp = out.startedAt.replace(/[:.]/g, '-');
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'g-observe-'));
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
    const fin = await ev(FINAL);
    const { engineLog, ...rest } = fin;
    Object.assign(out, rest);
    out.engineLogLines = engineLog ? engineLog.length : null;
    if (engineLog) {
      fs.mkdirSync(outDir, { recursive: true });
      const file = path.join(outDir, `engine-log-${stamp}.txt`);
      fs.writeFileSync(file, engineLog.join('\n') + '\n');
      out.engineLogFile = path.relative(repo, file);
    }
    if (out.timelineIdle === null && !stopped) out.error = `no first idle, error or exit within ${WAIT_LIMIT_MS} ms (state ${out.state ?? 'window.__app not set'})`;
    else if (out.timelineIdle === null) out.error = `state ${out.state} before the first idle`;

    if (out.timelineIdle !== null) {
      const shot = await S('Page.captureScreenshot', { format: 'png', ...(out.canvas ? { clip: { x: out.canvas.rect.x, y: out.canvas.rect.y, width: out.canvas.rect.width, height: out.canvas.rect.height, scale: 1 } } : {}) });
      fs.mkdirSync(outDir, { recursive: true });
      const png = Buffer.from(shot.data, 'base64');
      const file = path.join(outDir, `g4-${stamp}.png`);
      fs.writeFileSync(file, png);
      out.screenshot = { file: path.relative(repo, file), clipped: out.canvas !== null, bytes: png.length };
      out.readback = await ev(READBACK);
      const desk = await ev(DESK);
      if (desk.error) out.desk = { error: desk.error };
      else {
        const px = Buffer.from(desk.base64, 'base64');
        const counts = new Map();
        for (let i = 0; i < px.length; i += 4) { const k = px.readUInt32BE(i); counts.set(k, (counts.get(k) ?? 0) + 1); }
        let bg = 0, bgN = -1;
        for (const [k, n] of counts) if (n > bgN) { bg = k; bgN = n; }
        let nonBg = 0;
        for (let i = 0; i < px.length; i += 4) if (px.readUInt32BE(i) !== bg) nonBg++;
        out.desk = { width: desk.width, height: desk.height, pixels: px.length / 4, backgroundRgba: [(bg >>> 24) & 255, (bg >>> 16) & 255, (bg >>> 8) & 255, bg & 255], backgroundPixels: bgN, nonBackgroundPixels: nonBg, distinctColours: counts.size, windows: desk.windows };
      }
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
