#!/usr/bin/env node
// tools/plugin-run/run.mjs <absolute path to the bundle directory raw/<name>> <plugin name> [<plugin name> ...] [--scene c|d|auto] > plugin-run.jsonl   (Issue #1371; scenes Issue #1524)
// The effect plug-ins named. A name is a folder of lift/plugins/<name>/ (case, spaces and hyphens do not count: "Fruity Balance" is "fruity-balance").
// Every main DLL of the folder (role 本体, key effects.*) is checked, one headless Chrome launch per DLL and scene (a new profile, as tools/delivery-split/measure.mjs):
//   1. the engine is idle (the page's state engine-idle); elapsedMs is the page's idle mark (ms since the navigation), the same clock as tools/delivery-split/measure.mjs;
//   2. the scene asks the guest for the plug-in:
//      scene (c): a project that names it. The candidates are the .flp files whose text holds the plug-in's name at a name boundary, as UTF-16LE or as ASCII (the internal name, the event 201 of a project):
//      the characters before and after the name are not ASCII letters or digits, and a space after the name is not a boundary when a letter or digit follows it: "Fruity Delay Bank" and "Fruity Delay 2" are not "Fruity Delay".
//      They are ordered by root (host-extra/resources first, then host/resources) and by size (the smaller first), and at most 5 are tried, one launch each; the first that asks for the plug-in is kept.
//      scene (d): a project made from a template (Data/Templates under the roots): the template's first mixer insert plug-in is renamed to the plug-in and its state (212) is zeroed (tools/plugin-run/flp.mjs),
//      and the project is opened as scene (c) opens one. The templates are taken in root order and path order; one with exactly one mixer insert plug-in is preferred. The project is written under the run's temporary directory.
//      --scene c: scene (c) only; --scene d: scene (d) only; --scene auto (the default): scene (c) over the candidates, then scene (d) when no candidate asks for the plug-in (or there is none).
//   3. instantiated: the page read the plug-in's separate wasm module with requested=1 (the line image-read: <stem>.c<base>.wasm requested=1 of the engine's stderr, xmod.ts);
//   4. processed: Space starts the play and PLAY_MS pass. True when all of these hold: quantaWhilePlaying rises in that time (audio.detail()); no engine line written in that time is a failure
//      (fle thread <n> failed, SuspendError, stopped here: they are listed in playFailures); the page's state is engine-idle after it.
//      outputRms and nonSilentQuanta are recorded only: the samples of a project may be missing (the demo projects often play silence), so they are not part of processed.
// stdout: one JSON line per main DLL: {"plugin":"Fruity Balance","instantiated":true,"processed":false,"error":null,"elapsedMs":1234.5,"playFailures":["..."],"outputRms":0,"nonSilentQuanta":0,"quantaWhilePlaying":400,"engineStop":null,"sceneKind":"c","sceneFile":"...","candidatesTried":1}
//   engineStop (Issue #1537): null when the engine did not stop; else the stop report's facts (tools/e2e/lib/stop.mjs stopOf: exit {reason, code}, message, stop, evidence, returnAddress, registers, report)
//   and lastFle, the last 5 "fle: " lines of the whole engine log, the ones printed before the play too (a stop during the scene is not in playFailures, which read the play's window only).
//   quantaWhilePlaying and nonSilentQuanta are the rises during the play; outputRms is audio.detail().rms after it. error is set only when the scene failed or the guest did not ask for the plug-in (a failure of the play is in playFailures).
//   sceneKind is "c" or "d" for the scene that asked for the plug-in (null when none did); sceneFile is the project of scene (c) relative to the repository (null for scene (d) and when none asked); candidatesTried is the number of scene (c)'s projects tried (0 to 5).
// stderr: the scene's steps, the play's numbers, and on a scene error the last engine lines. A failed scene leaves its screen in .tmp-plugin-run-fail-<name>.png (the name in lower case, spaces as hyphens).
// Exit: 0 when every error is null, 1 otherwise (a name that is not an effect's folder stops the run before anything starts), 64 for a usage error (nothing on stdout; also a --scene value that is not c, d or auto).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';
import { encodeFlp, insertPluginsOf, parseFlp, renamePlugin } from './flp.mjs';
import { stopOf } from '../e2e/lib/stop.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const PLUGINS_DIR = path.join(repo, 'lift', 'plugins');
const PLAY_MS = 8000;
const WAIT_LIMIT_MS = 180000;
const POLL_MS = 1000;
const CDP_TIMEOUT_MS = 20000;
const SCENE_STEP_LIMIT_MS = 120000;
const MAX_CANDIDATES = 5;
const SCENES = ['c', 'd', 'auto'];

// the isolation flags of tools/boot-check/boot-bundle.mjs (the same as tools/delivery-split/measure.mjs)
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

// read in the page: the state, the manifest, the marks of the timeline, and the engine's image reads
const SNAPSHOT = `(() => { const a = window.__app ?? null; return {
  state: a ? a.state : null,
  timeline: a ? a.timeline : {},
  error: a && a.error ? a.error : null,
  imageReads: a ? a.imageReads : [],
  exit: a && a.exit ? { reason: a.exit.reason, code: a.exit.code, message: String(a.exit.message ?? '') } : null,
}; })()`;

const AUDIO_DETAIL = `(() => { const a = window.__app.audio; if (!a) return null; const d = a.detail(); return { deviceState: d.deviceState, status: d.status, ctxState: d.ctxState, quantaWhilePlaying: d.quantaWhilePlaying, nonSilentQuanta: d.nonSilentQuanta, rms: d.rms }; })()`;

const SCENE_ROOM_W = 1836, SCENE_ROOM_H = 1056;
// the view before the navigation (as tools/delivery-split/measure.mjs): the Welcome window is placed when it is created
const SCENE_VIEW_START = { width: 2176, height: 1092 };
const SCENE_WIZARD = 'TWelcomeWizard';
const SCENE_NO_THANKS = { dx: 512, dy: 307 };
const SCENE_LIST = { dx: 235, dy: 180 };
const SCENE_DEFAULT_ROW = { dx: 235, dy: 117 };
const SCENE_GEOMETRY = `(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const g = window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: g.width, gh: g.height }; })()`;
const SCENE_ROOM = `(() => { const s = document.getElementById('stage').getBoundingClientRect(); return { sw: s.width, sh: s.height, iw: window.innerWidth, ih: window.innerHeight }; })()`;
const SCENE_WINDOWS = `(async () => (await window.__app.deskDumpBase64(false)).windows)()`;

const num = (v) => (typeof v === 'number' && Number.isFinite(v) ? Math.round(v * 10) / 10 : null);
const escapeRe = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/** the name as the lookup compares it: case, spaces and hyphens do not count */
export const keyOf = (s) => String(s).toLowerCase().replace(/[\s-]+/g, '');

/** the effect rows of lift/plugins: one per main DLL (role 本体) whose key starts with effects.; `name` is the plug-in's folder in the guest ("Fruity Balance"), `stem` the DLL's key without .dll */
export function effectRows(pluginsDir = PLUGINS_DIR) {
  const rows = [];
  for (const folder of fs.readdirSync(pluginsDir, { withFileTypes: true }).filter((e) => e.isDirectory()).map((e) => e.name).sort()) {
    const tsv = path.join(pluginsDir, folder, 'plugin.tsv');
    if (!fs.existsSync(tsv)) continue;
    const lines = fs.readFileSync(tsv, 'utf8').split(/\r?\n/).filter((l) => l.length > 0 && !l.startsWith('#'));
    const head = lines[0].split('\t');
    for (const l of lines.slice(1)) {
      const r = Object.fromEntries(l.split('\t').map((c, i) => [head[i], c]));
      if (r.role !== '本体' || !/^effects\./.test(r.key ?? '')) continue;
      const dirs = String(r.guest).split('\\');
      rows.push({ folder, name: dirs.length >= 2 ? dirs[dirs.length - 2] : folder, key: r.key, stem: r.key.replace(/\.dll$/i, '') });
    }
  }
  return rows;
}

/** the effect rows of the names given (a name is a folder of lift/plugins); throws when a name is not an effect's folder */
export function resolveNames(names, rows = effectRows()) {
  return names.flatMap((n) => {
    const hits = rows.filter((r) => keyOf(r.folder) === keyOf(n));
    if (hits.length === 0) throw new Error(`no effect plug-in named "${n}" (a folder of lift/plugins with an effects.* main DLL)`);
    return hits;
  });
}

/** the .flp files under a folder, in path order */
function flpFiles(dir) {
  const out = [];
  for (const e of fs.readdirSync(dir, { withFileTypes: true }).sort((a, b) => (a.name < b.name ? -1 : 1))) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) out.push(...flpFiles(p));
    else if (/\.flp$/i.test(e.name)) out.push(p);
  }
  return out;
}

/** the code unit (1 byte for ASCII, 2 bytes for UTF-16LE) at byte `at` of `buf`; null outside the buffer */
const unitAt = (buf, at, unit) => (at >= 0 && at + unit <= buf.length ? (unit === 2 ? buf.readUInt16LE(at) : buf[at]) : null);
/** an ASCII letter or digit (英数字). Other characters, a non-ASCII letter included, do not touch a name */
const isAsciiAlnum = (u) => u !== null && /^[0-9A-Za-z]$/.test(String.fromCharCode(u));
const isSpace = (u) => u !== null && /^\s$/.test(String.fromCharCode(u));

/** whether the name at byte `at` (`len` bytes, code units of `unit` bytes) is at a boundary: the unit before it is not a letter or digit, and the unit after it is not one either;
 *  a space after it is a boundary unless a letter or digit follows the space ("Fruity Delay Bank", "Fruity Delay 2" are other names) */
export function atBoundary(buf, at, len, unit = 2) {
  if (isAsciiAlnum(unitAt(buf, at - unit, unit))) return false;
  const after = unitAt(buf, at + len, unit);
  if (isSpace(after)) return !isAsciiAlnum(unitAt(buf, at + len + unit, unit));
  return !isAsciiAlnum(after);
}

/** the candidate projects of scene (c) (Issue #1524): the .flp files whose text holds the name at a boundary, as UTF-16LE or as ASCII (the internal name), in root order, then the smaller first (path order on a tie);
 *  a file that another root gives under the same relative path is listed once (the first root's); at most `max` */
export function candidatesFor(name, roots, max = Infinity) {
  const needles = [[Buffer.from(name, 'utf16le'), 2], [Buffer.from(name, 'latin1'), 1]];
  const out = [];
  const seen = new Set();
  for (const root of roots) {
    if (!fs.existsSync(root)) continue;
    const hits = flpFiles(root).filter((f) => {
      const buf = fs.readFileSync(f);
      return needles.some(([needle, unit]) => {
        for (let at = buf.indexOf(needle); at >= 0; at = buf.indexOf(needle, at + 1)) if (atBoundary(buf, at, needle.length, unit)) return true;
        return false;
      });
    }).filter((f) => !seen.has(path.relative(root, f)));
    for (const f of hits) seen.add(path.relative(root, f));
    out.push(...hits.sort((a, b) => fs.statSync(a).size - fs.statSync(b).size));
  }
  return out.slice(0, max);
}

/** the project that names the plug-in (scene (c)): the first candidate of candidatesFor; null when none does */
export function projectFor(name, roots) {
  return candidatesFor(name, roots, 1)[0] ?? null;
}

/** the template projects (Data/Templates under the roots): the .flp files, in root order, then path order */
function templatesFor(roots) {
  return roots.flatMap((root) => (fs.existsSync(path.join(root, 'Data', 'Templates')) ? flpFiles(path.join(root, 'Data', 'Templates')) : []));
}

/** the project of scene (d) (Issue #1524): the first template with exactly one mixer insert plug-in (else the first with one), its plug-in renamed to `name` and its state zeroed (flp.mjs), written into `dir`;
 *  null when no template has a mixer insert plug-in */
export function sceneProjectFor(name, roots, dir) {
  const withInserts = [];
  for (const f of templatesFor(roots)) {
    let parsed;
    try {
      parsed = parseFlp(fs.readFileSync(f));
    } catch {
      continue;
    }
    const n = insertPluginsOf(parsed.events).length;
    if (n > 0) withInserts.push({ file: f, parsed, n });
  }
  const pick = withInserts.find((t) => t.n === 1) ?? withInserts[0];
  if (!pick) return null;
  const project = path.join(dir, `${name.toLowerCase().replace(/\s+/g, '-')}.flp`);
  fs.writeFileSync(project, encodeFlp(renamePlugin(pick.parsed, name)));
  return { template: pick.file, project };
}

/** the reads of the plug-in's separate wasm module (its image-read line is named <stem>.c<base>.wasm, xmod.ts) */
export function wasmReadRe(stem) {
  return new RegExp(`^${escapeRe(stem)}(\\.[^.\\s]+)?\\.wasm$`);
}

/** an engine line that is a failure of the play: a thread that failed, a suspend error, or the engine's stop (Issue #1371) */
export const isPlayFailure = (line) => /fle thread \d+ failed|SuspendError|stopped here/.test(line);

/** the failures of one play: the engine's lines of the window that are failures, the state after the play unless engine-idle, and no rise of quantaWhilePlaying (note: what the audio was doing) */
export function playFailuresOf({ lines, state, stateNote = '', quanta, note = '' }) {
  const out = lines.filter(isPlayFailure).map((l) => l.slice(0, 300));
  if (state !== 'engine-idle') out.push(`state ${state} after the play${stateNote}`);
  if (!(quanta > 0)) out.push(`quantaWhilePlaying did not rise in ${PLAY_MS / 1000} s${note}`);
  return out;
}

/** a result line passes (Issue #1524): the guest asked for the plug-in in scene (c) or (d), the plug-in was instantiated and processed, and error is null */
export const passes = (r) => r.error === null && r.instantiated === true && r.processed === true && (r.sceneKind === 'c' || r.sceneKind === 'd');

/** the windows of a window list: the lid at (x, y) and the class of each line (the page's format: lid N at (x, y) WxH ... class "..." title "...") */
function parseWindows(text) {
  return text.split('\n').map((l) => /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean).map((m) => {
    const names = /class "([^"]*)"/.exec(m[6]);
    const title = /title "([^"]*)"/.exec(m[6]);
    return { lid: Number(m[1]), x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), cls: names ? names[1] : '', title: title ? title[1] : '' };
  });
}

/** the engine's stop of a run (Issue #1537): null when the engine did not stop (no exit fact and no "== stop:" line in the log); else the stop report's facts (tools/e2e/lib/stop.mjs stopOf: exit, message, stop, registers, report)
 *  and the last 5 "fle: " lines of the WHOLE log, the ones printed before the play too (a stop during the scene is not in the play's window) */
export function engineStopOf(lines, exit) {
  const strip = (l) => l.replace(/^\[err\] ?/, '').replace(/^fruity: /, '');
  if (exit === null && !lines.some((l) => strip(l).startsWith('== stop:'))) return null;
  const lastFle = lines.map(strip).filter((l) => l.startsWith('fle: ')).slice(-5).map((l) => l.slice(0, 400));
  return { ...stopOf(lines, exit), lastFle };
}

/** a result line with nothing measured (the fields of the JSON line, in their order) */
const blankRow = (plugin, error) => ({ plugin, instantiated: false, processed: false, error, elapsedMs: null, playFailures: [], outputRms: null, nonSilentQuanta: null, quantaWhilePlaying: null, engineStop: null });

const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};

/** the scene: Welcome closed ("No thanks", then the Default row by a double click), then scene (c) or (d) (Ctrl+O, the project given to the open box),
 *  the plug-in's wasm read awaited, one block of the output */
async function runScene({ S, ev, scene, name, wasmRe }) {
  const steps = [];
  const out = { steps, error: null, state: null, welcome: null, reads: [], block: null };
  const retried = async (f) => {
    const t0 = Date.now();
    for (;;) {
      try { return await f(); } catch (e) {
        if (!/ask again/.test(String(e?.message ?? e)) || Date.now() - t0 >= 15000) throw e;
        await sleep(100);
      }
    }
  };
  const windows = async () => parseWindows(await retried(() => ev(SCENE_WINDOWS)));
  const wizard = async () => (await windows()).find((w) => w.cls === SCENE_WIZARD) ?? null;
  const pointAt = async (gx, gy) => {
    const g = await ev(SCENE_GEOMETRY);
    return { x: g.x + (gx + 0.5) * (g.w / g.gw), y: g.y + (gy + 0.5) * (g.h / g.gh) };
  };
  const click = async (gx, gy, clicks = 1) => {
    const c = await pointAt(gx, gy);
    await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(60);
    for (let k = 1; k <= clicks; k++) {
      await S('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'left', buttons: 1, clickCount: k, pointerType: 'mouse' });
      await sleep(50);
      await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: 'left', buttons: 0, clickCount: k, pointerType: 'mouse' });
      await sleep(100);
    }
  };
  const wheel = async (gx, gy, notches, deltaY) => {
    const c = await pointAt(gx, gy);
    for (let i = 0; i < notches; i++) {
      await S('Input.dispatchMouseEvent', { type: 'mouseWheel', x: c.x, y: c.y, deltaX: 0, deltaY, pointerType: 'mouse' });
      await sleep(300);
    }
  };
  const keyDown = (code, k, vk, modifiers) => S('Input.dispatchKeyEvent', { type: 'rawKeyDown', code, key: k, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers });
  const keyUp = (code, k, vk, modifiers) => S('Input.dispatchKeyEvent', { type: 'keyUp', code, key: k, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers });
  const key = async (code, k, vk, modifiers = 0) => {
    await keyDown(code, k, vk, modifiers);
    await sleep(40);
    await keyUp(code, k, vk, modifiers);
    await sleep(60);
  };
  // the engine's "Problems loading the project" box is modal: Enter closes it (its default button)
  const dismissProblems = async () => {
    let n = 0;
    for (let i = 0; i < 4 && (await windows()).some((w) => /Problems loading/.test(w.title)); i++) {
      await key('Enter', 'Enter', 0x0d);
      await sleep(1000);
      n++;
    }
    return n;
  };
  const ctrlO = async () => {
    await ev("document.getElementById('desktop').focus({ preventScroll: true }); true");
    await sleep(100);
    // the engine reads the Ctrl state from its own key messages: Control goes down first (VK_LCONTROL, as keymap.ts has it)
    await keyDown('ControlLeft', 'Control', 0xa2, 2);
    await sleep(40);
    await keyDown('KeyO', 'o', 79, 2);
    await sleep(40);
    await keyUp('KeyO', 'o', 79, 2);
    await sleep(40);
    await keyUp('ControlLeft', 'Control', 0xa2, 0);
  };
  const untilIdle = async (cond, what) => {
    const t0 = Date.now();
    for (;;) {
      const s = await ev(SNAPSHOT).catch(() => null);
      if (s && (s.state === 'error' || s.state === 'exited')) return s;
      if (s && s.state === 'engine-idle' && (await cond())) return s;
      if (Date.now() - t0 >= SCENE_STEP_LIMIT_MS) throw new Error(`${what}: not within ${SCENE_STEP_LIMIT_MS} ms`);
      await sleep(1000);
    }
  };
  // the play (Issue #1371): Space starts it, PLAY_MS pass. processed is true when quantaWhilePlaying rises, no engine line of the window is a failure, and the state is engine-idle after it
  const play = async () => {
    const before = await ev(AUDIO_DETAIL);
    if (!before) return { processed: false, playFailures: ['no audio output (window.__app.audio is null)'], numbers: null };
    const from = await ev('window.__app.engineLog().length');
    await ev("document.getElementById('desktop').focus({ preventScroll: true }); true");
    await key('Space', ' ', 32);
    await sleep(PLAY_MS);
    const after = await ev(AUDIO_DETAIL).catch(() => null);
    const snap = await ev(SNAPSHOT).catch(() => null);
    const lines = await ev(`window.__app.engineLog().slice(${from})`).catch(() => []);
    const quanta = after ? after.quantaWhilePlaying - before.quantaWhilePlaying : null;
    const playFailures = playFailuresOf({
      lines,
      state: snap?.state ?? null,
      stateNote: snap?.error ? ` (${snap.error.what} [${snap.error.where}])` : '',
      quanta,
      note: ` (ctx ${before.ctxState} -> ${after?.ctxState ?? 'none'}, device ${before.deviceState} -> ${after?.deviceState ?? 'none'}, status ${after?.status ?? 'none'})`,
    });
    const numbers = {
      quantaWhilePlaying: quanta,
      nonSilentQuanta: after ? after.nonSilentQuanta - before.nonSilentQuanta : null,
      outputRms: after?.rms ?? null,
    };
    process.stderr.write(`play: ${JSON.stringify(before)} -> ${JSON.stringify(after)}, state ${snap?.state ?? null}\n`);
    return { processed: playFailures.length === 0, playFailures, numbers };
  };
  try {
    for (let i = 0; i < 4; i++) {
      const r = await ev(SCENE_ROOM);
      if (Math.abs(r.sw - SCENE_ROOM_W) <= 0.5 && Math.abs(r.sh - SCENE_ROOM_H) <= 0.5) break;
      await S('Emulation.setDeviceMetricsOverride', { width: Math.round(r.iw + (SCENE_ROOM_W - r.sw)), height: Math.round(r.ih + (SCENE_ROOM_H - r.sh)), deviceScaleFactor: 1, mobile: false });
      await sleep(1500);
    }
    await sleep(1500);
    const titleBefore = await ev('document.title');
    const w0 = await wizard();
    out.welcome = w0 ? 'listed' : 'none';
    if (w0) {
      steps.push(`the Welcome window at (${w0.x}, ${w0.y}) ${w0.w}x${w0.h}`);
      await click(w0.x + SCENE_NO_THANKS.dx, w0.y + SCENE_NO_THANKS.dy);
      steps.push(`No thanks at guest (${w0.x + SCENE_NO_THANKS.dx}, ${w0.y + SCENE_NO_THANKS.dy})`);
      await sleep(2500);
      await untilIdle(async () => true, 'after No thanks');
      const w1 = await wizard();
      if (w1) {
        await wheel(w1.x + SCENE_LIST.dx, w1.y + SCENE_LIST.dy, 12, -120);
        await wheel(w1.x + SCENE_LIST.dx, w1.y + SCENE_LIST.dy, 3, 120);
        await sleep(400);
        await click(w1.x + SCENE_DEFAULT_ROW.dx, w1.y + SCENE_DEFAULT_ROW.dy, 2);
        steps.push('the Default row, 2 clicks');
      }
    }
    await untilIdle(async () => (await ev('document.title')) !== titleBefore && !(await wizard()), 'the project open');
    steps.push('the project is open (the title changed, the Welcome closed)');
    const dismissed = await dismissProblems();
    if (dismissed) steps.push(`the Problems box closed ${dismissed} time(s) before the scene`);
    await ctrlO();
    steps.push('Ctrl+O');
    if (!fs.existsSync(scene.projectPath)) throw new Error(`no project file ${scene.projectPath}`);
    // the open box is the page's file box (#filedlg-input), or the file manager in selection mode (#fx-input-files: the file is copied in, then chosen in the list and Open is pressed)
    const t0 = Date.now();
    let box = null;
    let again = false;
    while (!box && Date.now() - t0 < 40000) {
      box = await ev("document.getElementById('filedlg-input') ? 'filedlg' : (document.getElementById('fx-input-files') ? 'explorer' : null)");
      if (!box && (await dismissProblems())) {
        steps.push('the Problems box closed, Ctrl+O again');
        if (!again) { again = true; await ctrlO(); }
      }
      if (!box) await sleep(500);
    }
    if (!box) throw new Error('no open box (#filedlg-input or #fx-input-files) 40 s after Ctrl+O');
    const projectPath = scene.projectPath;
    const fileName = path.basename(projectPath);
    const setFile = async (selector) => {
      const doc = await S('DOM.getDocument', {});
      const q = await S('DOM.querySelector', { nodeId: doc.root.nodeId, selector });
      if (!q.nodeId) throw new Error(`${selector} is not a node`);
      await S('DOM.setFileInputFiles', { nodeId: q.nodeId, files: [projectPath] });
    };
    const clickSelector = async (selector) => {
      const r = await ev(`(() => { const e = document.querySelector(${JSON.stringify(selector)}); if (!e) return null; const b = e.getBoundingClientRect(); return { x: b.x + b.width / 2, y: b.y + b.height / 2 }; })()`);
      if (!r) throw new Error(`no element ${selector}`);
      await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: r.x, y: r.y, button: 'none', buttons: 0, pointerType: 'mouse' });
      await sleep(60);
      await S('Input.dispatchMouseEvent', { type: 'mousePressed', x: r.x, y: r.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
      await sleep(50);
      await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: r.x, y: r.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
      await sleep(300);
    };
    if (box === 'filedlg') {
      await setFile('#filedlg-input');
      steps.push(`${fileName} given to #filedlg-input`);
    } else {
      await setFile('#fx-input-files');
      const row = `#fx-list [data-name="${fileName}"]`;
      const t2 = Date.now();
      while (!(await ev(`!!document.querySelector(${JSON.stringify(row)})`))) {
        if (Date.now() - t2 >= 30000) throw new Error(`${fileName} is not listed in the file manager 30 s after the upload`);
        await sleep(500);
      }
      await clickSelector(row);
      await clickSelector('#fx-ok');
      steps.push(`${fileName} uploaded into the file manager, chosen and Open pressed`);
    }
    const t1 = Date.now();
    let snap = null;
    for (;;) {
      await dismissProblems().catch(() => 0);
      snap = await ev(SNAPSHOT).catch(() => null);
      if (snap && snap.imageReads.some((r) => wasmRe.test(r.name))) break;
      if (snap && (snap.state === 'error' || snap.state === 'exited')) break;
      if (Date.now() - t1 >= SCENE_STEP_LIMIT_MS) break;
      await sleep(1000);
    }
    await sleep(3000);
    snap = await ev(SNAPSHOT).catch(() => null);
    out.state = snap?.state ?? null;
    out.reads = (snap?.imageReads ?? []).map((r) => ({ name: r.name, atMs: num(r.atMs), requested: r.requested === true }));
    steps.push(`reads after the scene: ${out.reads.map((r) => `${r.name} requested=${r.requested}`).join(', ') || 'none'}`);
    // the play is measured once the guest has asked for the plug-in; a state other than engine-idle here is a failure of the play (playFailures), not of the scene
    if (out.reads.some((r) => wasmRe.test(r.name) && r.requested)) {
      out.block = await play();
      steps.push(`the play: ${JSON.stringify(out.block.numbers)}, processed ${out.block.processed}${out.block.playFailures.length ? `, playFailures ${out.block.playFailures.length}` : ''}`);
    }
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
  }
  return out;
}

/** the engine's stop as the log holds it now (engineStopOf). A stopped engine's report reaches the log late (tools/e2e/lib/resilience.mjs stopReport): the "exit code" line is awaited (6 s at most), then 1 s for the engine's stderr line */
async function engineStopNow(ev) {
  const snap = await ev(SNAPSHOT).catch(() => null);
  const lines = () => ev('window.__app.engineLog()').catch(() => []);
  if (snap && (snap.state === 'error' || snap.state === 'exited')) {
    const t0 = Date.now();
    while (!(await lines()).some((l) => /exit code \d+/.test(l)) && Date.now() - t0 < 6000) await sleep(250);
    await sleep(1000);
  }
  return engineStopOf(await lines(), snap?.exit ?? null);
}

/** one launch: the plug-in's DLL named by `target`, in the scene given ({kind:'c' or 'd', projectPath}), from the navigation to the end of the scene; returns the JSON line's object */
async function measureOnce(port, target, scene, shotDir) {
  const out = blankRow(target.name, null);
  const wasmRe = wasmReadRe(target.stem);
  const root = fs.mkdtempSync(path.join(repo, '.tmp-plugin-run-'));
  let chrome = null, cdp = null;
  try {
    chrome = await launchChrome({ profile: path.join(root, 'profile'), extraArgs: ISOLATION });
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
    await S('Emulation.setDeviceMetricsOverride', { width: SCENE_VIEW_START.width, height: SCENE_VIEW_START.height, deviceScaleFactor: 1, mobile: false });
    const nav = await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
    const navAt = Date.now();
    let snap = null;
    for (;;) {
      snap = await ev(SNAPSHOT).catch(() => null);
      if (snap && (snap.state === 'engine-idle' || snap.state === 'error' || snap.state === 'exited')) break;
      if (Date.now() - navAt >= WAIT_LIMIT_MS) break;
      await sleep(POLL_MS);
    }
    out.elapsedMs = num(snap?.timeline?.idle);
    if (!snap) out.error = `window.__app not readable within ${WAIT_LIMIT_MS} ms`;
    else if (snap.error) out.error = `${snap.error.what} [${snap.error.where}]`;
    else if (snap.state !== 'engine-idle') out.error = `state ${snap.state} at the end (no engine-idle within ${WAIT_LIMIT_MS} ms)`;
    else if (out.elapsedMs === null) out.error = 'the engine is idle but the page has no idle mark';
    if (out.error === null) {
      const sc = await runScene({ S, ev, scene, name: target.name, wasmRe });
      out.instantiated = sc.reads.some((r) => wasmRe.test(r.name) && r.requested);
      out.processed = sc.block?.processed === true;
      out.playFailures = sc.block?.playFailures ?? [];
      out.outputRms = sc.block?.numbers?.outputRms ?? null;
      out.nonSilentQuanta = sc.block?.numbers?.nonSilentQuanta ?? null;
      out.quantaWhilePlaying = sc.block?.numbers?.quantaWhilePlaying ?? null;
      out.error = sc.error ?? (out.instantiated ? null : `the guest did not ask for ${target.name} in scene (${scene.kind}) (no ${wasmRe} read with requested=1); state ${sc.state}`);
      process.stderr.write(`scene (${scene.kind}): ${sc.steps.join('; ')}${sc.error ? ` ERROR ${sc.error}` : ''}\n`);
    }
    out.engineStop = await engineStopNow(ev);
    if (out.engineStop) process.stderr.write(`engine stop: exit ${JSON.stringify(out.engineStop.exit)}, lastFle ${JSON.stringify(out.engineStop.lastFle)}\n`);
    if (out.error !== null) {
      const s = await S('Page.captureScreenshot', { format: 'png' }).catch(() => null);
      if (s) fs.writeFileSync(path.join(shotDir, `.tmp-plugin-run-fail-${target.folder.toLowerCase().replace(/\s+/g, '-')}.png`), Buffer.from(s.data, 'base64'));
      const lines = await ev('window.__app.engineLog().filter((l) => !/memory-usage/.test(l)).slice(-40)').catch(() => null);
      if (lines) process.stderr.write(`engine log (last 40, the memory-usage lines left out):\n${lines.join('\n')}\n`);
    }
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
  } finally {
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
    fs.rmSync(root, { recursive: true, force: true });
  }
  process.stderr.write(`run: ${target.name} (${target.key}) scene (${scene.kind}) ${out.error === null ? 'scene ok' : 'scene error'}, instantiated ${out.instantiated}, processed ${out.processed}, elapsedMs ${out.elapsedMs}\n`);
  return out;
}

/** the loop of scene (c) (Issue #1524): one attempt per candidate, in order; the first attempt that asks for the plug-in ends it. Returns the index of that candidate (-1 when none asked), its row, and the last row (null with no candidate) */
export async function firstAsking(candidates, attempt) {
  let last = null;
  for (let i = 0; i < candidates.length; i++) {
    const r = await attempt(candidates[i]);
    if (r.instantiated) return { index: i, row: r, last: r };
    last = r;
  }
  return { index: -1, row: null, last };
}

/** scene (d) (Issue #1524): the project made from a template (sceneProjectFor), opened as scene (c) opens its project; one line */
async function measureSceneD(port, t, roots, workDir, tried) {
  const made = sceneProjectFor(t.name, roots, workDir);
  if (!made) return { ...blankRow(t.name, `no template under host-extra/resources or host/resources (Data/Templates) has a mixer insert plug-in to rename to ${t.name}`), sceneKind: null, sceneFile: null, candidatesTried: tried };
  process.stderr.write(`scene (d) for ${t.name}: ${path.relative(repo, made.template)}, its first mixer insert plug-in renamed (the state zeroed)\n`);
  const r = await measureOnce(port, t, { kind: 'd', projectPath: made.project }, repo);
  return { ...r, sceneKind: r.instantiated ? 'd' : null, sceneFile: null, candidatesTried: tried };
}

/** one plug-in's line (Issue #1524): scene (c) over the candidates (the first that asks for the plug-in is kept), then scene (d) when the scene is auto and no candidate asks */
async function measureTarget(port, t, roots, scene, workDir) {
  const withScene = (r, sceneKind, sceneFile, candidatesTried) => ({ ...r, sceneKind, sceneFile, candidatesTried });
  if (scene === 'd') return measureSceneD(port, t, roots, workDir, 0);
  const cands = candidatesFor(t.name, roots, MAX_CANDIDATES);
  process.stderr.write(`scene (c) for ${t.name}: ${cands.length ? cands.map((p) => path.relative(repo, p)).join(' | ') : 'no .flp names it (UTF-16LE or ASCII, at a name boundary) under host-extra/resources or host/resources'}\n`);
  const { index, row, last } = await firstAsking(cands, (p) => measureOnce(port, t, { kind: 'c', projectPath: p }, repo));
  if (index >= 0) return withScene(row, 'c', path.relative(repo, cands[index]), index + 1);
  if (scene === 'c') {
    if (last === null) return withScene(blankRow(t.name, `no .flp names ${t.name} (UTF-16LE or ASCII, at a name boundary) under host-extra/resources or host/resources`), null, null, 0);
    return withScene({ ...last, error: `${last.error}; scene (c) tried ${cands.length} candidate(s)` }, null, null, cands.length);
  }
  process.stderr.write(`scene (d) for ${t.name}: ${cands.length ? `no candidate of ${cands.length} asked for it` : 'no candidate names it'}\n`);
  const r = await measureSceneD(port, t, roots, workDir, cands.length);
  if (r.sceneKind === 'd') return r;
  return { ...r, error: `${r.error}; scene (c) tried ${cands.length} candidate(s)` };
}

async function main() {
  const [bundleArg, ...rest] = process.argv.slice(2);
  const names = [];
  let scene = 'auto';
  let badScene = false;
  for (let i = 0; i < rest.length; i++) {
    if (rest[i] !== '--scene') names.push(rest[i]);
    else if (SCENES.includes(rest[i + 1])) scene = rest[++i];
    else badScene = true;
  }
  if (!bundleArg || names.length === 0 || badScene) {
    process.stderr.write('usage: node tools/plugin-run/run.mjs <absolute path to the bundle directory raw/<name>> <plugin name> [<plugin name> ...] [--scene c|d|auto]\n');
    process.exit(64);
  }
  let targets;
  try {
    targets = resolveNames(names);
  } catch (e) {
    process.stderr.write(`${e.message}\n`);
    process.exit(1);
  }
  const bundleDir = path.resolve(bundleArg);
  const BUNDLE = path.basename(bundleDir);
  const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
  const outDir = path.resolve(bundleDir, '..', '..', '..');
  const roots = [path.join(outDir, 'host-extra', 'resources'), path.join(outDir, 'host', 'resources')];
  const top = fs.mkdtempSync(path.join(repo, '.tmp-plugin-run-'));
  const assets = path.join(top, 'assets');
  let server = null;
  let ok = true;
  try {
    if (!fs.existsSync(brDir)) throw new Error(`no ${brDir}: the brotli directory of the bundle must be next to raw/`);
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    for (const t of targets) {
      const r = await measureTarget(port, t, roots, scene, top);
      process.stdout.write(`${JSON.stringify(r)}\n`);
      if (r.error !== null) ok = false;
    }
  } catch (e) {
    process.stderr.write(`${String(e?.message ?? e).split('\n')[0]}\n`);
    ok = false;
  } finally {
    if (server) await server.close().catch(() => {});
    fs.rmSync(top, { recursive: true, force: true });
    process.stderr.write(`loadavg ${os.loadavg().map((x) => x.toFixed(2)).join(' ')}, nproc ${os.cpus().length}\n`);
  }
  process.exitCode = ok ? 0 : 1;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await main();
