// Screen observation (ROADMAP R5-2): the same run as g-observe.mjs (R5-1) — the E13 bundle once in headless Chrome, a new profile, "/" with no parameters, the same isolation flags —
// but after the first idle it saves the whole viewport (no clip) and the compositor's picture (RGBA, as the engine holds it) as PNG files. It judges nothing.
// Outputs: evidence/R5-2/page-run<N>.png (viewport) and evidence/R5-2/compositor-run<N>.png (compositor). N = 1 + the number of page-run*.png already in evidence/R5-2/.
// Usage: node tools/boot-check/screen-observe.mjs > $TMPDIR/screen-N.json   (no arguments; needs app/node_modules and app/dist). Prints one JSON object on stdout. Exit 1 when the run failed ("error" set).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from './cdp.mjs';

const BUNDLE = 'fl-engine-ci-0363b050';
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const bundleDir = path.join(repo, 'artifacts', 'bundles', BUNDLE);
const outDir = path.join(repo, 'evidence', 'R5-2');
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
// RGBA bytes (rows of width*4, top row first) -> PNG, with Python 3's standard library only (zlib, struct)
const PNG_PY = `import struct, sys, zlib
w, h = int(sys.argv[1]), int(sys.argv[2])
raw = sys.stdin.buffer.read()
if len(raw) != w * h * 4:
    sys.exit(1)
rows = b''.join(b'\\x00' + raw[y * w * 4:(y + 1) * w * 4] for y in range(h))
def chunk(kind, body):
    return struct.pack('>I', len(body)) + kind + body + struct.pack('>I', zlib.crc32(kind + body) & 0xffffffff)
sys.stdout.buffer.write(b'\\x89PNG\\r\\n\\x1a\\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(rows, 9)) + chunk(b'IEND', b''))
`;
const toPng = (w, h, rgba) => execFileSync('python3', ['-c', PNG_PY, String(w), String(h)], { input: rgba, maxBuffer: 64 * 1024 * 1024 });

const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};

async function main() {
  const t0 = Date.now();
  const out = { startedAt: new Date(t0).toISOString(), run: null, state: null, engineKind: null, timelineIdle: null, timeline: null, engineLogLines: null, windowTable: null, idles: null, pageTitle: null, documentTitle: null, standIn: null, canvas: null, pageMs: null, page: null, readback: null, desk: null, compositor: null, chromeVersion: null, elapsedMs: null, loadavg: null, error: null };
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'screen-observe-'));
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
    if (out.timelineIdle === null && !stopped) out.error = `no first idle, error or exit within ${WAIT_LIMIT_MS} ms (state ${out.state ?? 'window.__app not set'})`;
    else if (out.timelineIdle === null) out.error = `state ${out.state} before the first idle`;

    if (out.timelineIdle !== null) {
      fs.mkdirSync(outDir, { recursive: true });
      const n = fs.readdirSync(outDir).filter((f) => /^page-run\d+\.png$/.test(f)).length + 1;
      out.run = n;
      const shot = await S('Page.captureScreenshot', { format: 'png' });
      const png = Buffer.from(shot.data, 'base64');
      const pageFile = path.join(outDir, `page-run${n}.png`);
      fs.writeFileSync(pageFile, png);
      out.page = { file: path.relative(repo, pageFile), bytes: png.length, width: png.readUInt32BE(16), height: png.readUInt32BE(20) };
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
        const cpng = toPng(desk.width, desk.height, px);
        const deskFile = path.join(outDir, `compositor-run${n}.png`);
        fs.writeFileSync(deskFile, cpng);
        out.compositor = { file: path.relative(repo, deskFile), bytes: cpng.length, width: desk.width, height: desk.height };
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
