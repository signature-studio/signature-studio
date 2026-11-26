// Shared-DLL measurement (Issue #1511): the operations of the list run in the real engine (the bundle, in Chrome), and the engine's log lines of a missing file or module
// ("missing-path: <API> <code> "<name>"", lift/runtime/fle_vfs.c fle_vfs_note_missing, written by fle_load.c not_found for LoadLibrary) are kept with the operation that made them.
// Each group (a list of operations) is a start of the bundle of its own (a new Chrome profile, a new engine), so a stop of the engine in one group does not hide the operations of another.
// Usage: node tools/shared-dll/measure.mjs <absolute path to the bundle directory raw/<name>> [--only <group>[,<group>]] > steps.jsonl
//   (needs app/node_modules and app/dist, as tools/smoke/welcome-flow.mjs does; the bundle at lift/out-app/bundle/raw/fl-Oz)
// stdout: one JSON line per operation: {"index","session","group","step","ok","error","ms","state","title","missing":[{api,code,path}],"stopLines","newWindows","detail"}, then one line per start of the bundle: {"group","session","sessionEnd":true,"stoppedAt","startFailed"} (stoppedAt: the index of the operation after which the engine was not engine-idle, or null).
// stderr: progress. Exit 0 when the tool ran (an operation's error is in its line), 1 when the tool itself failed, 2 when no bundle directory is given.
// The groups (--only takes their ids): start (the Welcome and the Default project, then Space), demo (Ctrl+O on a demo project), file-7, file-8, file-9 (the File menu's submenus, 14 items each: the render forms are started), options (F10 and Options 1-24), tools (F8 and Tools 1-34), mixer (the Plugin database's Sytrus and Fruity Balance), add (Add 1-14).
// The operations use the e2e harness's parts (tools/e2e/lib): the Guest (real mouse and key events, the window table, the engine's state), the menus (menuPath), the file box (openProjectFile), the plugin steps (pluginSteps). The start is the one of tools/smoke/welcome-flow.mjs flow A (startDefault).
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';
import { Guest } from '../e2e/lib/guest.mjs';
import { menuPath, openLids, closeNew, newWindows, waitWindow, windowTable } from '../e2e/lib/common.mjs';
import { openProjectFile, dismissProblems, waitStable, waitProjectOpen } from '../e2e/lib/flow.mjs';
import { childrenOfClass } from '../e2e/lib/dump.mjs';
import { pluginSteps } from '../e2e/lib/plugins.mjs';
import { missingOf } from '../missing-paths/measure.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const DEMO = path.join(repo, 'lift', 'out-app', 'host-extra', 'resources', 'Data', 'Demo projects', 'Demo songs', 'Asher Postman - Future House.flp');
// the room of tools/smoke/welcome-flow.mjs (its Welcome offsets are measured there; the flows A to D pass at it)
const VIEW = { width: 1836, height: 1056 };
const WELCOME = 'TWelcomeWizard';
const GDPR = 'TGDPRModule';
// guest px from the Welcome window's top-left corner (tools/smoke/welcome-flow.mjs): "No thanks", and the start list's Default row after the list is scrolled to its top
const NO_THANKS = { dx: 512, dy: 307 };
const LIST = { dx: 235, dy: 180 }, DEFAULT_ROW = { dx: 235, dy: 117 };
const BOOT_LIMIT_MS = 180000;
const FILE_ITEMS = 14, OPTIONS_ITEMS = 24, TOOLS_ITEMS = 34, ADD_ITEMS = 14;
const RENDER_FORM = /render|export|wave|mp3|ogg|flac|midi|video|\.wav|\.mp3/i;
const MAX_SESSIONS = 40;
const STOP_LINE = /^(\[err\] )?fle: (?!warn: |act1-net\t)/;

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

const log = (...a) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ')}\n`);

/** one page of one Chrome: the evaluate and the CDP calls that the e2e Guest uses (the page's evaluate takes an expression or a function) */
class Session {
  constructor(cdp, sessionId) { this.cdp = cdp; this.sessionId = sessionId; }
  send(method, params = {}) { return this.cdp.send(method, params, this.sessionId); }
  async ev(expression) {
    const r = await this.send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
    if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.exception?.description ?? r.exceptionDetails.text}`.slice(0, 300));
    return r.result.value;
  }
  guest() {
    const page = { evaluate: (x) => this.ev(typeof x === 'function' ? `(${x.toString()})()` : x) };
    const cdp = { send: (method, params) => this.send(method, params) };
    return new Guest(page, cdp, null);
  }
}

/** the start as tools/smoke/welcome-flow.mjs flow A does it: "No thanks" of the usage box, then the Default row (scrolled to the top, 3 notches down) is double-clicked; the project is open when the title has changed and the Welcome has closed */
async function startDefault(guest) {
  const first = await waitStable(guest, BOOT_LIMIT_MS);
  if (first.state !== 'engine-idle') return { opened: false, state: first.state, title: first.title };
  const titleBefore = first.title;
  const welcome = async () => (await guest.windows()).find((x) => x.cls === WELCOME) ?? null;
  let w = await waitWindow(guest, WELCOME, 30000);
  if (!w) return { opened: false, state: first.state, title: titleBefore, note: 'no Welcome window within 30 s of the first idle' };
  await sleep(1500);
  w = (await welcome()) ?? w;
  if (childrenOfClass(await windowTable(guest), [GDPR]).some((c) => c.vis)) {
    await guest.click(w.x + NO_THANKS.dx, w.y + NO_THANKS.dy);
    await sleep(2500);
  }
  w = (await welcome()) ?? w;
  await guest.wheel(w.x + LIST.dx, w.y + LIST.dy, 12, -120);
  await guest.wheel(w.x + LIST.dx, w.y + LIST.dy, 3, 120);
  await sleep(400);
  await guest.click(w.x + DEFAULT_ROW.dx, w.y + DEFAULT_ROW.dy, { clicks: 2 });
  const end = await waitProjectOpen(guest, titleBefore, 120000);
  await dismissProblems(guest);
  return { opened: end.opened, state: end.state, title: end.title };
}

/** a group's operations: `pre` runs first (the start), then `steps`. Each step is { name, run(ctx) } */
function groups(bundleDir) {
  const boot = { name: 'boot', keep: true, run: null };
  const start = { name: 'welcome-default', run: async ({ guest }) => startDefault(guest) };
  const play = { name: 'play-default', run: async ({ guest }) => {
    await guest.focusDesktop();
    const before = await guest.ev('window.__app.audio ? window.__app.audio.detail() : null');
    await guest.key('Space', ' ', 32);
    await sleep(3000);
    await guest.key('Space', ' ', 32);
    const after = await guest.ev('window.__app.audio ? window.__app.audio.detail() : null');
    return { nonSilentQuanta: after && before ? after.nonSilentQuanta - before.nonSilentQuanta : null };
  } };
  const demo = { name: 'open-demo', run: async ({ guest, cdp }) => {
    if (!fs.existsSync(DEMO)) throw new Error(`no demo file ${DEMO}`);
    await openProjectFile(guest, cdp, DEMO, () => {});
    const r = await waitStable(guest, 120000);
    await dismissProblems(guest);
    return { file: path.basename(DEMO), state: r.state, title: r.title };
  } };
  const fileItem = (s, k) => ({ name: `file-${s}-${k}`, run: async (ctx) => {
    const { guest } = ctx;
    const base = await openLids(guest);
    const id = await menuPath(guest, ['File', `#${s}`, `#${k}`]);
    await sleep(1200);
    const { pops, others } = await newWindows(guest, base);
    const made = [...pops, ...others].filter((w) => w.cls !== 'TQuickPopupMenuWindow');
    const opened = [...pops, ...others].map((w) => `${w.cls} "${w.title}"`);
    const form = made.find((w) => RENDER_FORM.test(w.cls) || RENDER_FORM.test(w.title));
    let rendered = null;
    if (form) {
      const saved0 = await guest.ev('window.__app.saved ? window.__app.saved.length : -1');
      await guest.click(form.x + form.w - 38, form.y + form.h - 24);
      let n = saved0;
      for (let t = 0; t < 40 && n === saved0; t++) { await sleep(500); n = await guest.ev('window.__app.saved ? window.__app.saved.length : -1'); }
      rendered = { form: `${form.cls} "${form.title}"`, startPressed: true, savedBefore: saved0, savedAfter: n };
    }
    await closeNew(guest, base);
    return { item: id, opened, rendered };
  } });
  const optionsItem = (k) => ({ name: `options-${k}`, run: async ({ guest }) => {
    const base = await openLids(guest);
    const id = await menuPath(guest, ['Options', `#${k}`]);
    await sleep(1200);
    const { pops, others } = await newWindows(guest, base);
    await closeNew(guest, base);
    return { item: id, opened: [...pops, ...others].map((w) => w.cls) };
  } });
  const toolsItem = (k) => ({ name: `tools-${k}`, run: async ({ guest }) => {
    const base = await openLids(guest);
    const id = await menuPath(guest, ['Tools', `#${k}`]);
    await sleep(1200);
    const { pops, others } = await newWindows(guest, base);
    await closeNew(guest, base);
    return { item: id, opened: [...pops, ...others].map((w) => w.cls) };
  } });
  const addItem = (k) => ({ name: `add-${k}`, run: async ({ guest }) => {
    const base = await openLids(guest);
    const id = await menuPath(guest, ['Add', `#${k}`]);
    await sleep(1200);
    const { pops, others } = await newWindows(guest, base);
    await closeNew(guest, base);
    return { item: id, opened: [...pops, ...others].map((w) => w.cls) };
  } });
  const f10 = { name: 'options-f10-settings', run: async ({ guest }) => {
    const base = await openLids(guest);
    await guest.focusDesktop();
    await guest.key('F10', 'F10', 121);
    await sleep(1500);
    const w = await waitWindow(guest, 'TMIDIForm', 3000);
    await closeNew(guest, base);
    return { opened: w ? w.cls : null };
  } };
  const f8 = { name: 'tools-f8-plugin-list', run: async ({ guest }) => {
    const base = await openLids(guest);
    await guest.focusDesktop();
    await guest.key('F8', 'F8', 119);
    await sleep(1500);
    const w = await waitWindow(guest, 'TPlugListForm', 3000);
    await closeNew(guest, base);
    return { opened: w ? w.cls : null };
  } };
  const mixer = (name, kind) => ({ name: `mixer-slot-${name.toLowerCase().replace(/[^a-z0-9]+/g, '-')}`, run: async ({ guest }) => {
    const r = await pluginSteps(guest, { name, kind });
    return { ok: r.ok, engineState: r.engineState, notes: r.notes };
  } });
  const list = [
    { id: 'start', pre: [boot, start], steps: [play] },
    { id: 'demo', pre: [boot, start], steps: [demo] },
    { id: 'file-8', pre: [boot, start], steps: Array.from({ length: FILE_ITEMS }, (_, i) => fileItem(8, i + 1)) },
    { id: 'file-9', pre: [boot, start], steps: Array.from({ length: FILE_ITEMS }, (_, i) => fileItem(9, i + 1)) },
    { id: 'file-7', pre: [boot, start], steps: Array.from({ length: FILE_ITEMS }, (_, i) => fileItem(7, i + 1)) },
    { id: 'options', pre: [boot, start], steps: [f10, ...Array.from({ length: OPTIONS_ITEMS }, (_, i) => optionsItem(i + 1))] },
    { id: 'tools', pre: [boot, start], steps: [f8, ...Array.from({ length: TOOLS_ITEMS }, (_, i) => toolsItem(i + 1))] },
    { id: 'mixer', pre: [boot, start], steps: [mixer('Sytrus', 'generators'), mixer('Fruity Balance', 'effects')] },
    { id: 'add', pre: [boot, start], steps: Array.from({ length: ADD_ITEMS }, (_, i) => addItem(i + 1)) },
  ];
  return list.map((g) => ({ ...g, pre: g.pre.map((p) => (p === boot ? { ...boot, run: async ({ sess }) => {
    await sess.send('Page.navigate', { url: `http://127.0.0.1:${sess.port}/` });
    const r = await waitStable(sess.guest(), BOOT_LIMIT_MS);
    if (r.state !== 'engine-idle') throw new Error(`no engine-idle at the start: ${r.state} ${r.error ?? ''}`.trim());
    return { state: r.state, bootMs: r.waitedMs };
  } } : p)) }));
}

/** one operation: the engine's lines from the operation's start, the state, the windows it opened, and what it returned */
async function operation(ctx, group, step) {
  const { guest, emit } = ctx;
  const n0 = await guest.ev('window.__app ? window.__app.engineLog().length : 0').catch(() => 0);
  const base = await openLids(guest).catch(() => new Set());
  const t0 = Date.now();
  let error = null, detail = null;
  try { detail = (await step.run(ctx)) ?? null; } catch (e) { error = String(e?.message ?? e).slice(0, 300); }
  const ms = Date.now() - t0;
  const st = await guest.state().catch(() => null);
  const state = st?.state ?? 'unreadable';
  const lines = await guest.ev(`window.__app ? window.__app.engineLog().slice(${Number(n0)}) : []`).catch(() => []);
  const newWindows = await guest.windows().then((ws) => ws.filter((w) => !base.has(w.lid)).map((w) => `${w.cls} "${w.title}"`)).catch(() => []);
  const row = {
    index: ctx.index.value++, session: ctx.session, group: group.id, step: step.name, ok: error === null && state === 'engine-idle', error, ms, state, title: st?.title ?? null,
    missing: missingOf(lines), stopLines: state === 'engine-idle' ? [] : lines.filter((l) => STOP_LINE.test(l)).slice(-4).map((l) => l.slice(0, 300)), newWindows: newWindows.slice(0, 16), detail,
  };
  emit(row);
  if (state === 'engine-idle' && !step.keep) await closeNew(guest, base).catch(() => {});
  return state;
}

/** a group's operations in order. When the engine stops, the rest of the group runs in a new start of the bundle (the start's own operations first), so one stop does not hide the others */
async function runGroup(group, index, emit, port, root) {
  const queue = [...group.steps];
  let session = 0;
  do {
    session++;
    const profile = path.join(root, `profile-${group.id}-${session}`);
    let chrome = null, cdp = null, stoppedAt = null, failed = null, error = null;
    try {
      chrome = await launchChrome({ profile, extraArgs: ISOLATION });
      cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
      await cdp.opened;
      const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
      const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
      const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
      const sess = new Session(cdp, sessionId);
      sess.port = port;
      await sess.send('Page.enable');
      await sess.send('Emulation.setDeviceMetricsOverride', { width: VIEW.width, height: VIEW.height, deviceScaleFactor: 1, mobile: false });
      const guest = sess.guest();
      const ctx = { guest, cdp: guest.cdp, sess, emit, index, session };
      log(`group ${group.id}: session ${session} start (${queue.length} operations left)`);
      for (const p of group.pre) {
        const state = await operation(ctx, group, p);
        if (state !== 'engine-idle') { failed = p.name; stoppedAt = index.value - 1; break; }
      }
      while (failed === null && stoppedAt === null && queue.length > 0) {
        const s = queue.shift();
        const state = await operation(ctx, group, s);
        if (state !== 'engine-idle') stoppedAt = index.value - 1;
      }
    } catch (e) {
      failed = failed ?? 'exception';
      error = String(e?.message ?? e).slice(0, 300);
      log(`group ${group.id}: session ${session} failed: ${error}`);
    } finally {
      if (cdp) await shutdown(chrome, cdp).catch(() => {});
      else if (chrome) chrome.proc.kill('SIGKILL');
    }
    emit({ group: group.id, session, sessionEnd: true, stoppedAt, startFailed: failed, error });
    log(`group ${group.id}: session ${session} end (stopped at ${stoppedAt ?? 'none'}, ${queue.length} operations left)`);
    if (failed !== null) break;
  } while (queue.length > 0 && session < MAX_SESSIONS);
}

async function main() {
  const argv = process.argv.slice(2);
  const onlyAt = argv.indexOf('--only');
  const only = onlyAt >= 0 ? argv[onlyAt + 1].split(',') : null;
  const positional = argv.filter((a, i) => !a.startsWith('--') && (onlyAt < 0 || i !== onlyAt + 1));
  if (positional.length !== 1) {
    process.stderr.write('usage: node tools/shared-dll/measure.mjs <absolute path to the bundle directory raw/<name>> [--only <group>[,<group>]]\n');
    process.exit(2);
  }
  const bundleDir = path.resolve(positional[0]);
  const BUNDLE = path.basename(bundleDir);
  const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
  const root = fs.mkdtempSync(path.join(repo, '.tmp-shared-dll-'));
  const assets = path.join(root, 'assets');
  let server = null;
  const emit = (row) => process.stdout.write(JSON.stringify(row) + '\n');
  try {
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    const index = { value: 0 };
    for (const group of groups(bundleDir)) {
      if (only && !only.includes(group.id)) continue;
      await runGroup(group, index, emit, port, root);
    }
  } catch (e) {
    process.stderr.write(`${String(e?.stack ?? e)}\n`);
    process.exitCode = 1;
  } finally {
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
}

// the measurement runs when this file is the program
const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) await main();
