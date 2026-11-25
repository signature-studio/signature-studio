// Issue #1389: the engine's Copy and Paste against the browser's clipboard (navigator.clipboard, navigator.permissions). One start of the bundle in a Chrome of its own,
// at the room of 1836 x 1056 CSS px (as tools/smoke/welcome-flow.mjs). The clipboard permissions are granted (Browser.grantPermissions) for the page's origin.
//   Copy: the default project's "Problems loading the project" box: its 'FLEX (FL)' row is clicked, the 'Copy' popup it opens is clicked; the text the engine
//     gave to SetClipboardData is compared with what navigator.clipboard.readText() returns after the Copy (copied, match).
//   Paste: navigator.clipboard.writeText('<random>'); the channel's name is selected, F2 opens the name box (a text edit of the engine), Ctrl+V pastes into it;
//     the engine's GetClipboardData must return that text (the window-API trace line carries it). pasteDelayMs is the time from the page's Ctrl+V dispatch to
//     the page's answer to the last clipboard read of the paste (the answer is handed to the engine right after; the guest's return follows it at once).
//     pasteRequestMs is the time from the Ctrl+V dispatch to the first read's request (the engine's IsClipboardFormatAvailable asks first).
//   Denied: the permissions are set to 'denied' (Browser.setPermission); the paste is asked again: the engine must stay idle (engine-idle) and the page must show
//     the reason in its status line (role="status", #clipboard-status). deniedHandled.
// Usage: node tools/clipboard/measure.mjs <absolute path to the bundle directory raw/<name>> [--shots <directory>] > clip.json   (needs app/node_modules and app/dist's
//   vite, as tools/smoke/welcome-flow.mjs does). stdout: one JSON line; exit 0 when the JSON was written (its error field says what failed), 2 when no bundle directory is given.
// --shots DIR (a test aid): PNG pictures of the page at the points of the flow are written to DIR.
// The JSON: error (null, or the reason), copied, match, text (the engine's text, 200 characters at most), textLength, pasted, pasteDelayMs, pasteRequestMs, deniedHandled,
//   deniedMessage, state (the engine's state at the end), permissions (the browser's states as the page saw them), copyCalls and pasteCalls (the window-API lines of the clipboard, in order).
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
const PERMISSIONS = ['clipboardReadWrite', 'clipboardSanitizedWrite'];
const PROBLEMS = /Problems loading/;
const POPUP = 'TQuickPopupMenuWindow';
const NAME_EDIT = 'TNameEditForm';
// guest pixels, measured on the default project at the room of 1836 x 1056: the 'FLEX (FL)' row of the Plugins list from the box's top-left corner; the box's Ok from its
// left edge and bottom edge; the first channel's name in the Channel rack
const FLEX_ROW = { dx: 105, dy: 168 };
const OK = { dx: 368, dyFromBottom: 29 };
const CHANNEL_NAME = { x: 520, y: 144 };
const READY_LIMIT_MS = 180000, POLL_MS = 300, CDP_TIMEOUT_MS = 20000;
const CLIP_LINE = /window-api: (OpenClipboard|EmptyClipboard|SetClipboardData|CloseClipboard|IsClipboardFormatAvailable|GetClipboardData)\b/;
const OS_TEXT_GET = /window-api: GetClipboardData fmt=13 -> 0x[0-9a-f]+ \(os text\) bytes=\d+ text="(.*)"$/;

const ISOLATION = [
  '--password-store=basic',
  '--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE 127.0.0.1',
  '--no-proxy-server', '--disable-ipv6', '--disable-quic',
  '--disable-background-networking', '--disable-component-update', '--disable-sync', '--disable-default-apps', '--disable-domain-reliability',
  '--no-pings', '--metrics-recording-only', '--disable-client-side-phishing-detection', '--disable-breakpad', '--disable-extensions', '--safebrowsing-disable-auto-update', '--disable-search-engine-choice-screen', '--no-service-autorun',
  '--disable-features=OptimizationHints,MediaRouter,Translate,CertificateTransparencyComponentUpdater,NetworkTimeServiceQuerying,AutofillServerCommunication,OptimizationGuideModelDownloading,SafeBrowsingEnhancedProtection',
  '--variations-server-url=http://127.0.0.1:9/', '--disable-field-trial-config',
  '--force-color-profile=srgb', '--disable-lcd-text',
];

const log = (...a) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ')}\n`);
const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};
const parseWindows = (text) => text.split('\n').map((l) => /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean).map((m) => {
  const names = /class "([^"]*)" title "([^"]*)"\s*$/.exec(m[6]);
  return { lid: Number(m[1]), x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), cls: names ? names[1] : '', title: names ? names[2] : '' };
});
async function waitFor(fn, ms) {
  const t0 = Date.now();
  for (;;) {
    const v = await fn().catch(() => null);
    if (v) return v;
    if (Date.now() - t0 >= ms) return null;
    await sleep(POLL_MS);
  }
}

/** one page of the bundle in its Chrome: the keys and the mouse go to the engine as a person's would */
class Page {
  constructor(cdp, sessionId, browserContextId, origin, shots) {
    this.cdp = cdp;
    this.sessionId = sessionId;
    this.browserContextId = browserContextId;
    this.origin = origin;
    this.shots = shots;
    this.geo = null;
  }
  send(method, params = {}) { return timed(this.cdp.send(method, params, this.sessionId), CDP_TIMEOUT_MS, method); }
  async ev(expression) {
    const r = await timed(this.send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true, userGesture: true }), 15000, `evaluate ${expression.slice(0, 60)}`);
    if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.exception?.description ?? r.exceptionDetails.text}`.slice(0, 300));
    return r.result.value;
  }
  async windows() {
    for (let i = 0; ; i++) {
      try {
        return parseWindows(await this.ev('(async () => (await window.__app.deskDumpBase64(false)).windows)()'));
      } catch (e) {
        if (!/ask again/.test(String(e?.message ?? e)) || i > 150) throw e;
        await sleep(100);
      }
    }
  }
  async find(pred) { return (await this.windows()).find(pred) ?? null; }
  async geometry() { return this.ev('(() => { const g = window.__app.screen.guest; const r = document.getElementById("desktop").getBoundingClientRect(); return { gw: g.width, gh: g.height, x: r.x, y: r.y, w: r.width, h: r.height }; })()'); }
  async clickGuest(gx, gy, right = false) {
    const g = await this.geometry();
    const c = { x: g.x + (gx + 0.5) * (g.w / g.gw), y: g.y + (gy + 0.5) * (g.h / g.gh) };
    const button = right ? 'right' : 'left';
    await this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(80);
    await this.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button, buttons: right ? 2 : 1, clickCount: 1, pointerType: 'mouse' });
    await sleep(60);
    await this.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button, buttons: 0, clickCount: 1, pointerType: 'mouse' });
    await sleep(400);
  }
  async key(code, key, vk, modifiers = 0) {
    await this.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers });
    await sleep(40);
    await this.send('Input.dispatchKeyEvent', { type: 'keyUp', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers });
    await sleep(60);
  }
  /** Ctrl+V: Control goes down first (the engine reads the Control state from its own key messages), V, Control up (as tools/p3-audio-probe does for Ctrl+O) */
  async ctrlV() {
    await this.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: 'ControlLeft', key: 'Control', windowsVirtualKeyCode: 0xa2, nativeVirtualKeyCode: 0xa2, modifiers: 2 });
    await sleep(40);
    await this.key('KeyV', 'v', 86, 2);
    await this.send('Input.dispatchKeyEvent', { type: 'keyUp', code: 'ControlLeft', key: 'Control', windowsVirtualKeyCode: 0xa2, nativeVirtualKeyCode: 0xa2, modifiers: 0 });
    await sleep(40);
  }
  async focusDesktop() { await this.ev("document.getElementById('desktop').focus({ preventScroll: true }); true"); await sleep(100); }
  async shot(name) {
    if (!this.shots) return;
    const s = await this.send('Page.captureScreenshot', { format: 'png' });
    fs.mkdirSync(this.shots, { recursive: true });
    fs.writeFileSync(path.join(this.shots, name), Buffer.from(s.data, 'base64'));
  }
  /** the engine's log lines that are clipboard calls (window-API trace), in order */
  clipLines() { return this.ev(`window.__app.engineLog().filter((l) => ${CLIP_LINE.toString()}.test(l))`); }
  /** the browser's permission state, set by the test (the page is told by the browser, not by the engine) */
  setPermission(name, setting) { return this.cdp.send('Browser.setPermission', { permission: { name }, setting, origin: this.origin, browserContextId: this.browserContextId }); }
}

/** the room is 1836 x 1056 CSS px: the window's size is set until the stage has it */
async function fitRoom(page) {
  for (let i = 0; i < 4; i++) {
    const r = await page.ev("(() => { const s = document.getElementById('stage').getBoundingClientRect(); return { sw: s.width, sh: s.height, iw: window.innerWidth, ih: window.innerHeight }; })()");
    if (Math.abs(r.sw - ROOM_W) <= 0.5 && Math.abs(r.sh - ROOM_H) <= 0.5) return;
    await page.send('Emulation.setDeviceMetricsOverride', { width: Math.round(r.iw + (ROOM_W - r.sw)), height: Math.round(r.ih + (ROOM_H - r.sh)), deviceScaleFactor: 1, mobile: false });
    await sleep(1500);
  }
}

/** the browser's clipboard as the page reads it after the test's writes and the engine's Copy (readText needs the document focused) */
async function readBrowserClipboard(page) {
  const r = await timed(page.send('Runtime.evaluate', { expression: 'navigator.clipboard.readText().then((t) => t, (e) => "ERROR " + (e && e.name) + ": " + (e && e.message))', awaitPromise: true, returnByValue: true, userGesture: true }), 10000, 'readText').catch((e) => `ERROR ${e.message}`);
  return r.result ? r.result.value : r;
}

async function session(bundleDir, port, shots, out, repoRoot) {
  const root = fs.mkdtempSync(path.join(repoRoot, '.tmp-clip-'));
  const profile = path.join(root, 'profile');
  let chrome = null, cdp = null;
  try {
    chrome = await launchChrome({ profile, extraArgs: ISOLATION });
    cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
    await cdp.opened;
    const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const origin = `http://127.0.0.1:${port}`;
    await cdp.send('Browser.grantPermissions', { browserContextId, origin, permissions: PERMISSIONS });
    const page = new Page(cdp, sessionId, browserContextId, origin, shots);
    await page.send('Page.enable');
    await page.send('Emulation.setDeviceMetricsOverride', { width: VIEW_START.width, height: VIEW_START.height, deviceScaleFactor: 1, mobile: false });
    await page.send('Page.navigate', { url: `${origin}/` });
    const idle = await waitFor(async () => ((await page.ev('window.__app ? window.__app.state : null')) === 'engine-idle' ? true : null), READY_LIMIT_MS);
    if (!idle) throw new Error('the engine did not reach its first idle');
    await fitRoom(page);
    await sleep(1500);
    log('engine idle');

    // the Welcome window closes with Escape: the default project opens, and its "Problems loading the project" box comes
    await page.focusDesktop();
    await page.key('Escape', 'Escape', 0x1b);
    const problems = await waitFor(() => page.find((w) => PROBLEMS.test(w.title)), 150000);
    if (!problems) throw new Error('no "Problems loading the project" box within 150 s');
    await page.shot('clip-1-problems.png');
    await page.ev('window.__app.windowApiTrace(true); true');
    await sleep(500);

    // Copy: the item, then the popup's Copy
    const before = (await page.clipLines()).length;
    await page.clickGuest(problems.x + FLEX_ROW.dx, problems.y + FLEX_ROW.dy);
    const popup = await waitFor(() => page.find((w) => w.cls === POPUP), 5000);
    if (!popup) throw new Error('no Copy popup after the click on the item');
    await page.shot('clip-2-popup.png');
    await page.clickGuest(popup.x + popup.w / 2, popup.y + popup.h / 2);
    await sleep(1000);
    await page.shot('clip-3-copied.png');
    const copyLines = (await page.clipLines()).slice(before);
    out.copyCalls = copyLines.map((l) => l.replace(/^\[\w+\] /, ''));
    const engineText = await page.ev('window.__app.clipboard.lastWritten');
    const back = await readBrowserClipboard(page);
    out.text = typeof engineText === 'string' ? engineText.slice(0, 200) : null;
    out.textLength = typeof engineText === 'string' ? engineText.length : null;
    out.copied = typeof back === 'string' && back.length > 0 && !back.startsWith('ERROR') && back !== 'TIMEOUT';
    out.match = out.copied && typeof engineText === 'string' && back === engineText;
    log('copy', { copied: out.copied, match: out.match, text: out.text, lines: copyLines.length });

    // the box's Ok closes it
    const box = await page.find((w) => PROBLEMS.test(w.title));
    if (box) await page.clickGuest(box.x + OK.dx, box.y + box.h - OK.dyFromBottom);
    await waitFor(async () => ((await page.find((w) => PROBLEMS.test(w.title))) ? null : true), 10000);
    await sleep(500);

    // Paste (granted): the browser has the random text; the name box of the channel takes Ctrl+V
    const random = `fl${Math.random().toString(36).slice(2, 12)}`;
    const wrote = await page.ev(`navigator.clipboard.writeText(${JSON.stringify(random)}).then(() => true, (e) => 'ERROR ' + (e && e.message))`);
    if (wrote !== true) log('the test could not write the browser clipboard', wrote);
    await page.focusDesktop();
    await page.clickGuest(CHANNEL_NAME.x, CHANNEL_NAME.y);
    await page.key('F2', 'F2', 0x71);
    const box2 = await waitFor(() => page.find((w) => w.cls === NAME_EDIT), 5000);
    if (!box2) throw new Error('F2 did not open the name box of the channel');
    await sleep(300);
    const beforePaste = (await page.clipLines()).length;
    await page.ev('window.__app.clipboard.lastReadAt = null; true');
    // the page's readText calls keep the time each one was asked and the time it gave its text (the last answer is the one the engine's GetClipboardData waits for)
    await page.ev(`(() => { const api = navigator.clipboard; const read = api.readText.bind(api); window.__readReq = []; window.__readDone = []; api.readText = () => { window.__readReq.push(performance.now()); return read().then((t) => { window.__readDone.push(performance.now()); return t; }); }; return true; })()`);
    const mark = await page.ev('performance.now()');
    await page.ctrlV();
    await waitFor(async () => (await page.ev('window.__app.clipboard.lastReadAt')) !== null, 8000);
    await sleep(1500);
    await page.shot('clip-4-pasted.png');
    const pasteLines = (await page.clipLines()).slice(beforePaste);
    out.pasteCalls = pasteLines.map((l) => l.replace(/^\[\w+\] /, ''));
    const cl = await page.ev('window.__app.clipboard');
    const asked = await page.ev('window.__readReq');
    const answered = await page.ev('window.__readDone');
    out.pasteRequestMs = Array.isArray(asked) && asked.length > 0 ? Math.round(asked[0] - mark) : null;
    out.pasteDelayMs = Array.isArray(answered) && answered.length > 0 ? Math.round(answered[answered.length - 1] - mark) : null;
    const got = pasteLines.map((l) => OS_TEXT_GET.exec(l.replace(/^\[\w+\] /, ''))).find((m) => m) ?? null;
    out.pasted = cl.lastRead === random && got !== null && got[1] === random;
    log('paste', { pasted: out.pasted, pasteDelayMs: out.pasteDelayMs, lastRead: cl.lastRead, lines: pasteLines.length });
    await page.key('Escape', 'Escape', 0x1b);
    await sleep(800);

    // Denied: the browser refuses both clipboard permissions; the paste is asked again
    await page.setPermission('clipboard-read', 'denied');
    await page.setPermission('clipboard-write', 'denied');
    await page.focusDesktop();
    await page.clickGuest(CHANNEL_NAME.x, CHANNEL_NAME.y);
    await page.key('F2', 'F2', 0x71);
    const box3 = await waitFor(() => page.find((w) => w.cls === NAME_EDIT), 5000);
    if (!box3) throw new Error('F2 did not open the name box of the channel (denied)');
    await sleep(300);
    await page.ctrlV();
    const shown = await waitFor(async () => ((await page.ev('window.__app.clipboard.message')) !== null ? true : null), 8000);
    await sleep(1000);
    const dn = await page.ev(`(() => { const el = document.getElementById('clipboard-status'); return { message: window.__app.clipboard.message, readPermission: window.__app.clipboard.readPermission, state: window.__app.state, shown: !!el && !el.hidden && el.getAttribute('role') === 'status', text: el ? el.textContent : null }; })()`);
    await page.shot('clip-5-denied.png');
    out.deniedMessage = typeof dn.text === 'string' ? dn.text.slice(0, 200) : null;
    out.deniedHandled = !!shown && dn.state === 'engine-idle' && dn.message !== null && dn.shown === true && dn.readPermission === 'denied';
    out.permissions = { read: dn.readPermission, write: await page.ev("navigator.permissions.query({ name: 'clipboard-write' }).then((p) => p.state, () => 'unknown')") };
    log('denied', { deniedHandled: out.deniedHandled, state: dn.state, shown: dn.shown, readPermission: dn.readPermission });
    await page.key('Escape', 'Escape', 0x1b);
    out.state = await page.ev('window.__app.state');
  } finally {
    if (chrome && cdp) await shutdown(chrome, cdp);
    fs.rmSync(root, { recursive: true, force: true });
  }
}

async function main() {
  const positional = process.argv.slice(2).filter((a, i, all) => !(a === '--shots' || all[i - 1] === '--shots'));
  const shotsAt = process.argv.indexOf('--shots');
  const shots = shotsAt >= 0 ? path.resolve(process.argv[shotsAt + 1]) : null;
  if (positional.length === 0) {
    process.stderr.write('usage: node tools/clipboard/measure.mjs <absolute path to the bundle directory raw/<name>> [--shots <directory>]\n');
    process.exit(2);
  }
  const bundleDir = path.resolve(positional[0]);
  const BUNDLE = path.basename(bundleDir);
  const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
  const out = {
    error: null, copied: false, match: false, text: null, textLength: null, pasted: false, pasteDelayMs: null, pasteRequestMs: null, deniedHandled: false, deniedMessage: null,
    state: null, permissions: null, copyCalls: [], pasteCalls: [],
  };
  const root = fs.mkdtempSync(path.join(repo, '.tmp-clip-assets-'));
  const assets = path.join(root, 'assets');
  let server = null;
  try {
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    await session(bundleDir, port, shots, out, repo);
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
  } finally {
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
  process.stdout.write(`${JSON.stringify(out)}\n`);
  log('loadavg', os.loadavg(), 'nproc', os.availableParallelism());
}

const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) {
  await main().catch((e) => {
    process.stderr.write(`${String(e?.stack ?? e)}\n`);
    process.exitCode = 1;
  });
}
