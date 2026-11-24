// Issue #1383: does a setting that FL writes into the virtual registry (its language) survive a reload of the page (the engine is built again)?
// Two starts of the bundle, each in a Chrome of its own (a new profile); the engine's first idle is awaited as tools/smoke/welcome-flow.mjs does (a start that has not reached it
// within START_LIMIT_MS is loaded again, #1376).
// Language run: the Welcome window is closed with Escape (while listed); F10 opens the Settings window and the General page is pressed (#1358, measure-size.mjs settingsPhase).
// The Language control is opened, and a language other than English is chosen at random from de, es, fr, ja, ko, vi, zh (by its name when the list shows it; else Down k times and Enter).
// restartPrompted: the window "Language change" (FL's restart prompt, a top-level window of its own) is new after the change. It is judged by its title, not by the text of the prompt.
// language: the value FL wrote, read from the engine's window-API trace (RegSetValueExW "..." "Program language" ... value="<code>?"; the NUL shown as ? is left out).
// Then the page is reloaded (Page.reload). registryKept: the trace of that start has RegQueryValueExW "..." "Program Language" -> found with the same value.
// If the engine stops on that start, it is recorded and not an error: stopReason is "missing-ilsvg" (missing-path: LoadLibraryW ... "ILSvg_x64.dll"), "compare-string" (fle: CompareStringW: ...) or "other"; the page is reloaded once more.
// error is set only when the engine neither reaches idle nor stops.
// Default run (another profile, English): the engine's first idle, then Page.reload; defaultReloadState is the engine's state after that reload.
// bytes: the size of /fl/state in the browser's OPFS (the saved registry) after the change, before the reload; bytesAfterReload: after the reload.
// elapsedMs: navigation to the first engine-idle; reloadElapsedMs: Page.reload to the idle after it; preBootBytes: the start-up assets (assetsReport, as welcome-flow.mjs reads them) at the first idle.
// Usage: node tools/persist/measure.mjs <absolute path to the bundle directory raw/<name>> [--shots <directory>] [--language CODE] > persist.json   (needs app/node_modules and app/dist, as boot-bundle.mjs does).
//   --language CODE (a test aid; the default is the random choice): the language to choose (de, es, fr, ja, ko, vi or zh).
//   --shots DIR (a test aid): the page's PNGs (general, language-open, language-after, reload) are written to DIR (the directory is made; nothing is removed).
// stdout: one JSON line (the result). Everything else goes to stderr. Exit 0 when the tool ran (the result's error says whether the run was good), 1 when the tool itself failed, 2 without a bundle directory.
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
const READY_LIMIT_MS = 180000, START_LIMIT_MS = 40000, START_TRIES = 4, POLL_MS = 500, CDP_TIMEOUT_MS = 20000, SETTINGS_SLOTS = 10, RELOAD_SETTLE_MS = 1500, STOP_SETTLE_MS = 2500;
const LANGUAGES = ['de', 'es', 'fr', 'ja', 'ko', 'vi', 'zh'];
const NAME_OF = {
  de: /Deutsch|German/i, es: /Español|Espanol|Spanish/i, fr: /Français|Francais|French/i, ja: /日本語|Japanese/i,
  ko: /한국어|Korean/i, vi: /Tiếng Việt|Tieng Viet|Vietnamese/i, zh: /中文|简体|Chinese/i,
};
const RESTART_TITLE = 'Language change';
const WELCOME = 'TWelcomeWizard';
const SET_LINE = /window-api: RegSetValueExW "[^"]*" "Program language" type=1 size=\d+ value="([^"]*)"/i;
const FOUND_LINE = /window-api: RegQueryValueExW "[^"]*" "Program language" -> found type=1 size=\d+ value="([^"]*)"/i;
const STOP_LINE = /missing-path: LoadLibraryW \d+ "ILSvg_x64\.dll"/;
const COMPARE_STOP = /fle: CompareStringW:/;
const CHILD_RE = /^child lid (\d+) parent lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+) vis (\d) class "([^"]*)" text "([^"]*)"\s*$/;
const TOP_RE = /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+) .*class "([^"]*)" title "([^"]*)"\s*$/;

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
// the window-API trace on from the engine's first instruction (as measure-size.mjs does): the registry calls are in the engine's log
const TRACE_ON = `(function () { const t = setInterval(() => { if (window.__app) { clearInterval(t); window.__app.windowApiTrace(true); } }, 5); })();`;

const STATE = `(() => { const a = window.__app; if (!a) return null; return { state: a.state, error: a.error ? String(a.error.what ?? a.error.message ?? a.error).slice(0, 300) : null }; })()`;
const TIMES = `(() => { const a = window.__app; return { timeOrigin: performance.timeOrigin, idle: a.timeline.idle ?? null, bytes: (a.assetsReport()?.items ?? []).reduce((n, i) => n + i.bytes, 0) }; })()`;
const GEOMETRY = `(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const g = window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: g.width, gh: g.height }; })()`;
const ROOM = `(() => { const s = document.getElementById('stage').getBoundingClientRect(); return { sw: s.width, sh: s.height, iw: window.innerWidth, ih: window.innerHeight }; })()`;
const WINDOWS = `(async () => (await window.__app.deskDumpBase64(true)).windows)()`;
const LOG = `window.__app.engineLog().map((l) => String(l))`;
const SIZE_OF_STATE = `(async () => {
  let d = await navigator.storage.getDirectory();
  try { for (const s of ['fl', 'state']) d = await d.getDirectoryHandle(s); } catch { return { bytes: 0 }; }
  let bytes = 0;
  const walk = async (dir) => { for await (const [, h] of dir.entries()) { if (h.kind === 'file') bytes += (await h.getFile()).size; else await walk(h); } };
  await walk(d);
  return { bytes };
})()`;

const log = (...a) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ')}\n`);
const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};

/** the windows of a window table: the top-level windows and the children, each with its absolute place (a child's place is its parent's plus its own) */
export function parseTable(text) {
  const byLid = new Map();
  for (const l of text.split('\n')) {
    let m = CHILD_RE.exec(l);
    if (m) { byLid.set(Number(m[1]), { lid: Number(m[1]), parent: Number(m[2]), x: Number(m[3]), y: Number(m[4]), w: Number(m[5]), h: Number(m[6]), vis: m[7] === '1', cls: m[8], text: m[9], top: false }); continue; }
    m = TOP_RE.exec(l);
    if (m) byLid.set(Number(m[1]), { lid: Number(m[1]), parent: null, x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), vis: true, cls: m[6], text: m[7], top: true });
  }
  const abs = (w) => {
    let x = w.x, y = w.y, p = w.parent, guard = 0;
    while (p !== null && byLid.has(p) && guard++ < 64) { const q = byLid.get(p); x += q.x; y += q.y; p = q.parent; }
    return { x, y };
  };
  const all = [...byLid.values()].map((w) => ({ ...w, ...abs(w) }));
  const under = (lid) => all.filter((w) => !w.top && descends(w, lid, byLid));
  return { all, tops: all.filter((w) => w.top), under };
}
function descends(w, lid, byLid) {
  let p = w.parent, guard = 0;
  while (p !== null && guard++ < 64) { if (p === lid) return true; p = byLid.get(p)?.parent ?? null; }
  return false;
}
const brief = (w) => ({ lid: w.lid, cls: w.cls, text: w.text, vis: w.vis, x: w.x, y: w.y, w: w.w, h: w.h });

class Sess {
  constructor(cdp, sessionId, shots) { this.cdp = cdp; this.sessionId = sessionId; this.shots = shots; }
  send(method, params = {}) { return timed(this.cdp.send(method, params, this.sessionId), CDP_TIMEOUT_MS, method); }
  async ev(expression, ms = CDP_TIMEOUT_MS) {
    const r = await timed(this.cdp.send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }, this.sessionId), ms, 'Runtime.evaluate');
    if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.exception?.description ?? r.exceptionDetails.text}`.slice(0, 300));
    return r.result.value;
  }
  async table() { return parseTable(String(await this.ev(WINDOWS, 60000))); }
  /** a test aid (--shots DIR): the page as it is, as a PNG */
  async shot(name) {
    if (!this.shots) return;
    const s = await this.send('Page.captureScreenshot', { format: 'png' });
    fs.mkdirSync(this.shots, { recursive: true });
    fs.writeFileSync(path.join(this.shots, name), Buffer.from(s.data, 'base64'));
  }
  async state() { return this.ev(STATE); }
  /** a press at guest pixel (gx, gy): the page's pointer path (move, press, release) */
  async click(gx, gy, clicks = 1) {
    const g = await this.ev(GEOMETRY);
    const c = { x: g.x + (gx + 0.5) * (g.w / g.gw), y: g.y + (gy + 0.5) * (g.h / g.gh) };
    await this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(60);
    for (let k = 1; k <= clicks; k++) {
      await this.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'left', buttons: 1, clickCount: k, pointerType: 'mouse' });
      await sleep(50);
      await this.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: 'left', buttons: 0, clickCount: k, pointerType: 'mouse' });
      await sleep(400);
    }
  }
  async key(code, vk) {
    await this.ev("document.getElementById('desktop')?.focus(); true");
    await this.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', code, key: code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
    await sleep(40);
    await this.send('Input.dispatchKeyEvent', { type: 'keyUp', code, key: code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
    await sleep(800);
  }
  async log() { return this.ev(LOG, 60000); }
}

/** the engine's state, until it is idle or has stopped, or the limit: the last state read */
async function waitIdle(sess, limitMs) {
  const t0 = Date.now();
  for (;;) {
    const s = await sess.state().catch(() => null);
    if (s && (s.state === 'engine-idle' || s.state === 'error' || s.state === 'exited')) return s;
    if (Date.now() - t0 >= limitMs) return s ?? { state: 'none', error: null };
    await sleep(POLL_MS);
  }
}

/** the room is 1836 x 1056 CSS px (the stage the General page's positions were measured in) */
async function settleRoom(sess) {
  for (let i = 0; i < 4; i++) {
    const r = await sess.ev(ROOM);
    if (Math.abs(r.sw - ROOM_W) <= 0.5 && Math.abs(r.sh - ROOM_H) <= 0.5) break;
    await sess.send('Emulation.setDeviceMetricsOverride', { width: Math.round(r.iw + (ROOM_W - r.sw)), height: Math.round(r.ih + (ROOM_H - r.sh)), deviceScaleFactor: 1, mobile: false });
    await sleep(1500);
  }
  await sleep(1500);
}

/** one start of the page (load: the navigation or the reload), until the engine's first idle. A start that has not reached idle within START_LIMIT_MS is loaded again
 * (START_TRIES; stalls counted in out[stallKey]). A stopped engine is not loaded again here. Returns the last state, the elapsed time and the pre-boot bytes */
async function start(sess, load, out, stallKey) {
  for (let attempt = 1; ; attempt++) {
    const sentAt = Date.now();
    await load();
    await sleep(RELOAD_SETTLE_MS);
    const last = attempt === START_TRIES;
    const s = await waitIdle(sess, last ? READY_LIMIT_MS : START_LIMIT_MS);
    if (s.state !== 'engine-idle' && s.state !== 'error' && s.state !== 'exited' && !last) {
      out[stallKey]++;
      out.steps.push(`${stallKey} ${attempt}: no engine-idle within ${START_LIMIT_MS} ms (state ${s.state}); loaded again`);
      continue;
    }
    if (s.state !== 'engine-idle') return { state: s, elapsedMs: null, preBootBytes: null };
    await settleRoom(sess);
    const t = await sess.ev(TIMES);
    return { state: s, elapsedMs: t.idle === null ? null : Math.round(t.timeOrigin + t.idle - sentAt), preBootBytes: t.bytes };
  }
}

const navigateTo = (sess, port) => async () => {
  const nav = await sess.send('Page.navigate', { url: `http://127.0.0.1:${port}/` });
  if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
};
const reloadPage = (sess) => () => sess.send('Page.reload', { ignoreCache: false });

/** the Settings window's top-level window and the visible page of its tab bar (the page is the TVectorSheet under the window) */
const settingsOf = (t) => {
  const top = t.tops.find((w) => w.cls === 'TMIDIForm') ?? null;
  if (!top) return null;
  const page = t.all.find((w) => !w.top && w.parent === top.lid && w.cls === 'TVectorSheet' && w.vis) ?? null;
  return { top, page };
};

/** the Welcome window and the dialog after it are closed (Escape, while listed); F10 opens the Settings window, and the tab bar's slots are pressed until the caption says General */
async function openGeneral(sess, steps) {
  for (let i = 0; i < 4; i++) {
    const t = await sess.table();
    if (!t.tops.some((w) => w.cls === WELCOME || w.cls === 'TMissingPluginsForm')) break;
    await sess.key('Escape', 27);
    await sleep(1500);
  }
  await sess.key('F10', 121);
  let st = null, tbl = null;
  for (let i = 0; i < 15 && !st; i++) {
    await sleep(1000);
    tbl = await sess.table();
    st = settingsOf(tbl);
  }
  if (!st) { steps.push('the Settings window (TMIDIForm) did not open within 15 s of F10'); return null; }
  const bar = tbl.all.find((w) => !w.top && w.parent === st.top.lid && w.cls === 'TQuickSheetSelector');
  if (!bar) { steps.push('no tab bar (TQuickSheetSelector)'); return null; }
  const captions = [];
  for (let s = 0; s < SETTINGS_SLOTS; s++) {
    await sess.click(bar.x + ((s + 0.5) * bar.w) / SETTINGS_SLOTS, bar.y + bar.h / 2);
    const cur = settingsOf(await sess.table());
    captions.push(cur?.page ? cur.page.text : null);
    if (cur?.page && cur.page.text === 'General') {
      steps.push(`General page at slot ${s}`);
      return { top: cur.top, bar };
    }
  }
  steps.push(`the General page was not reached (captions ${JSON.stringify(captions)})`);
  return null;
}

/** the General page now: its visible sheet's children (absolute places) */
async function generalNow(sess) {
  const t = await sess.table();
  const st = settingsOf(t);
  if (!st?.page) return { page: null, children: [] };
  return { page: st.page, children: t.under(st.page.lid).map(brief) };
}

/** the language control: the Language panel's control row, under its caption (the position is the one the General page has at 1836 x 1056: measured on the picture) */
const controlOf = (panel) => ({ x: panel.x + 12, y: panel.y + 20, w: 174, h: 22 });

/** why the engine stopped: the last of its two known stops in the log (the missing ILSvg file, or CompareStringW's comparison not modelled); other when neither is there */
function stopReasonOf(lines) {
  const stops = lines.map((l, i) => ({ i, reason: STOP_LINE.test(l) ? 'missing-ilsvg' : COMPARE_STOP.test(l) ? 'compare-string' : null })).filter((s) => s.reason !== null);
  return stops.length === 0 ? 'other' : stops[stops.length - 1].reason;
}

/** the value of the trace line that matches, with the NUL the trace shows as ? left out; null when no line matches */
function valueOf(lines, re) {
  const found = lines.map((l) => re.exec(l)).filter(Boolean);
  if (found.length === 0) return { value: null, line: null };
  const m = found[found.length - 1];
  return { value: m[1].replace(/\?$/, ''), line: m[0].slice(0, 240) };
}

/** the change of the language (the first start, the General page, the list, the change), then the reload. Fills out; the stopped engine is recorded, not a throw */
async function languageRun(sess, port, out, forced) {
  const first = await start(sess, navigateTo(sess, port), out, 'startStalls');
  out.elapsedMs = first.elapsedMs;
  out.preBootBytes = first.preBootBytes;
  if (first.state.state !== 'engine-idle') throw new Error(`no engine-idle at the first start (state ${first.state.state}: ${first.state.error ?? ''})`);
  out.steps.push(`first idle: ${first.elapsedMs} ms`);

  const g = await openGeneral(sess, out.steps);
  if (!g) throw new Error('the General page is not open');
  const now = await generalNow(sess);
  const panel = now.children.find((c) => c.text && c.text.startsWith('Language'));
  if (!panel) throw new Error('no Language panel on the General page');
  const ctl = controlOf(panel);
  const code = forced ?? LANGUAGES[Math.floor(Math.random() * LANGUAGES.length)];
  out.languageIntended = code;
  await sess.shot('general.png');

  const beforeLids = new Set((await sess.table()).all.map((w) => w.lid));
  // the control opens its list; the list is read from the window table when its items are windows of their own (chosen by name); else by keys (Down k times, Enter)
  await sess.click(ctl.x + ctl.w / 2, ctl.y + ctl.h / 2);
  await sleep(1500);
  await sess.shot('language-open.png');
  const pop = await sess.table();
  const items = pop.all.filter((w) => !beforeLids.has(w.lid) && !w.top && w.vis && w.text);
  const target = items.find((w) => NAME_OF[code].test(w.text)) ?? null;
  if (target) {
    await sess.click(target.x + target.w / 2, target.y + target.h / 2);
    out.languageMatch = true;
    out.languageLabel = target.text;
    out.steps.push(`chose "${target.text}" from the list (by name)`);
  } else {
    const k = 1 + LANGUAGES.indexOf(code);
    for (let i = 0; i < k; i++) await sess.key('ArrowDown', 40);
    await sess.key('Enter', 13);
    out.languageMatch = false;
    out.steps.push(`no list item by name: Down ${k} times and Enter`);
  }
  await sleep(3000);
  await sess.shot('language-after.png');
  const after = await sess.table();
  out.restartWindows = after.tops.filter((w) => !beforeLids.has(w.lid)).map((w) => ({ cls: w.cls, title: w.text }));
  out.restartPrompted = out.restartWindows.some((w) => w.title === RESTART_TITLE);
  out.steps.push(`restart prompted: ${out.restartPrompted} (new windows ${JSON.stringify(out.restartWindows)})`);

  const written = valueOf(await sess.log(), SET_LINE);
  out.registryWrite = written.line;
  out.language = written.value ?? code;
  out.bytes = (await sess.ev(SIZE_OF_STATE)).bytes;
  await sess.shot('language-after-change.png');

  const second = await start(sess, reloadPage(sess), out, 'reloadStalls');
  out.reloadElapsedMs = second.elapsedMs;
  out.steps.push(`reload: state ${second.state.state} (${second.elapsedMs} ms)`);
  let lines = await sess.log();
  let read = valueOf(lines, FOUND_LINE);
  if (second.state.state === 'engine-idle' && read.value === null) {
    // FL may read the setting when the General page is opened (the start's trace is read again after the page is opened)
    await openGeneral(sess, out.steps);
    lines = await sess.log();
    read = valueOf(lines, FOUND_LINE);
  }
  out.registryLine = read.line;
  out.registryKept = read.value !== null && read.value === out.language;
  out.bytesAfterReload = (await sess.ev(SIZE_OF_STATE)).bytes;

  if (second.state.state !== 'engine-idle') {
    out.engineStoppedAfterReload = second.state.state === 'error' || second.state.state === 'exited';
    // the engine's last lines reach the page a moment after its state does (the log is flushed when the run ends)
    await sleep(STOP_SETTLE_MS);
    lines = await sess.log();
    out.engineTail = lines.slice(-40).map((l) => String(l).slice(0, 240));
    out.stopLine = lines.filter((l) => /fle: /.test(l)).slice(-1)[0]?.slice(0, 240) ?? null;
    // a stop after the reload is recorded, not an error (#1395 fixes the stops of the languages other than English)
    if (out.engineStoppedAfterReload) {
      out.stopReason = stopReasonOf(lines);
    } else {
      out.error = `the engine did not reach idle after the reload (state ${second.state.state}: ${second.state.error ?? ''})`.slice(0, 600);
    }
    out.steps.push(`the engine stopped on the reload: stopReason ${out.stopReason}`);
    const again = await start(sess, reloadPage(sess), out, 'recoverStalls');
    out.recoveredIdle = again.state.state === 'engine-idle';
    out.steps.push(`the page was reloaded again: state ${again.state.state} (${again.elapsedMs} ms)`);
  }
}

/** the default (English) start: the first idle, then the reload with nothing changed */
async function defaultRun(sess, port, out) {
  const first = await start(sess, navigateTo(sess, port), out, 'defaultStalls');
  if (first.state.state !== 'engine-idle') throw new Error(`no engine-idle at the default start (state ${first.state.state}: ${first.state.error ?? ''})`);
  const second = await start(sess, reloadPage(sess), out, 'defaultReloadStalls');
  out.defaultReloadState = second.state.state;
  out.defaultReloadElapsedMs = second.elapsedMs;
  out.steps.push(`default reload: state ${second.state.state} (${second.elapsedMs} ms)`);
}

/** one page in a Chrome of its own (a new profile); fn gets the session */
async function withPage(port, profile, shots, fn) {
  let chrome = null, cdp = null;
  try {
    chrome = await launchChrome({ profile, extraArgs: ISOLATION });
    cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
    await cdp.opened;
    const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const sess = new Sess(cdp, sessionId, shots);
    await sess.send('Page.enable');
    await sess.send('Emulation.setDeviceMetricsOverride', { width: VIEW_START.width, height: VIEW_START.height, deviceScaleFactor: 1, mobile: false });
    await sess.send('Page.addScriptToEvaluateOnNewDocument', { source: TRACE_ON });
    await fn(sess);
  } finally {
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
  }
}

async function run(port, root, shots, opts) {
  const out = {
    language: null, languageIntended: null, languageLabel: null, languageMatch: null, registryWrite: null, registryLine: null,
    restartPrompted: false, restartWindows: [], registryKept: false, engineStoppedAfterReload: false, stopReason: null, recoveredIdle: null,
    defaultReloadState: null, defaultReloadElapsedMs: null, bytes: null, bytesAfterReload: null, engineTail: null, stopLine: null, error: null,
    elapsedMs: null, reloadElapsedMs: null, preBootBytes: null, startStalls: 0, reloadStalls: 0, recoverStalls: 0, defaultStalls: 0, defaultReloadStalls: 0, steps: [],
  };
  try {
    await withPage(port, path.join(root, 'profile'), shots, (sess) => languageRun(sess, port, out, opts.language));
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 600);
    out.steps.push(`failed: ${out.error}`);
  }
  try {
    await withPage(port, path.join(root, 'profile-default'), null, (sess) => defaultRun(sess, port, out));
  } catch (e) {
    out.error = out.error ?? String(e?.message ?? e).slice(0, 600);
    out.steps.push(`default run failed: ${String(e?.message ?? e).slice(0, 300)}`);
  }
  return out;
}

async function main() {
  const argv = process.argv.slice(2);
  const shotsAt = argv.indexOf('--shots');
  const shots = shotsAt >= 0 ? path.resolve(argv[shotsAt + 1] ?? '') : null;
  const langAt = argv.indexOf('--language');
  const language = langAt >= 0 ? argv[langAt + 1] ?? '' : null;
  const valued = [shotsAt, langAt].filter((i) => i >= 0).map((i) => i + 1);
  const positional = argv.filter((a, i) => !a.startsWith('--') && !valued.includes(i));
  if (positional.length !== 1 || (shotsAt >= 0 && !argv[shotsAt + 1]) || (language !== null && !LANGUAGES.includes(language))) {
    process.stderr.write(`usage: node tools/persist/measure.mjs <absolute path to the bundle directory raw/<name>> [--shots <directory>] [--language ${LANGUAGES.join('|')}]\n`);
    process.exit(2);
  }
  const bundleDir = path.resolve(positional[0]);
  const BUNDLE = path.basename(bundleDir);
  const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
  const root = fs.mkdtempSync(path.join(repo, '.tmp-persist-'));
  const assets = path.join(root, 'assets');
  let server = null;
  let result = null;
  try {
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    result = await run(port, root, shots, { language });
  } finally {
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
  for (const s of result.steps) log(s);
  log('loadavg', os.loadavg(), 'nproc', os.availableParallelism());
  process.stdout.write(JSON.stringify(result) + '\n');
}

const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) {
  await main().catch((e) => {
    process.stderr.write(`${String(e?.stack ?? e)}\n`);
    process.exitCode = 1;
  });
}
