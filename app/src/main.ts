// Page entry: loads the bundle (/assets/manifest.json), shows the start-up stages, progress and errors, starts the engine Worker and shows the engine's desktop on its one canvas. The page has no URL parameters (decision 0243).
import './style.css';
import { attachDesktopInput } from './input/pointer.ts';
import { describeRaw, IN, type RawInput } from './input/events.ts';
import { TaskbarRestore } from './ui/taskbar.ts';
import { fetchAsset, fetchManifest, hashModeOfThisBrowser, loadPack, loadWasm, LoadError, type LoadedPack, type Progress } from './bundle/loader.ts';
import { sha256Hex } from './bundle/sha256.ts';
import type { FruityOptions, Manifest } from './bundle/manifest.ts';
import { EngineClient, type ExitInfo } from './engine/client.ts';
import { stateAfterFirstPixel } from './engine/start-state.ts';
import { applyHz, HZ_DEFAULT, measureFrameRate, type FrameHost, type HzMeasurement } from './engine/hz.ts';
import { blockingMissing, describeMissing, detectFeatures, type FeatureReport } from './engine/features.ts';
import { matchStopNotes, summarizeEngineLog, type EngineLogSummary } from './engine/fruity-log.ts';
import type { AppView } from './engine/appview.ts';
import type { HostFileStats } from './engine/hostfiles.ts';
import { cursorCss } from './ui/cursor.ts';
import { DesktopView } from './ui/desktop.ts';
import { engineState, meanCycle, type EngineClockState } from './ui/engine-status.ts';
import { LatencyTracker, toAnyFrame, type InputMeta } from './ui/latency.ts';
import { stopLines } from './ui/stop-message.ts';
import { EngineLogView, renderThreads, renderWindows } from './ui/panels.ts';
import { stageRows, Timeline, type StageId } from './ui/phases.ts';
import { ProgressView } from './ui/progress.ts';
import { chooseGuestScreen, drawScale, RoomDebouncer, screenEnv, SCREEN_DEFAULT, type GuestScreenInfo, type Room } from './ui/screen-fit.ts';
import { collectBrowserInfo, collectClientInfo, webglRenderer, type BrowserInfo } from './perf/browser-info.ts';
import { FailureReporter, detectCapabilities, installFailureWatch, postFailure, type FailureKind } from './perf/failure-beacon.ts';
import { BeaconScheduler, beaconText, buildBeacon, deskBeacon, engineRunBeacon, postBeacon, type BeaconPayload, type BeaconReason, type SendResult } from './perf/beacon.ts';
import { DisplayMonitor, hzFit, type DisplaySnapshot, type HzInfo } from './perf/display.ts';
import { browserDisplayHost } from './perf/display-host.ts';
import { engineExitMessage } from './perf/exit-message.ts';
import { perfNotice, perfRows, renderPerfPanel, type PanelView } from './perf/panel.ts';
import { inputLatencyStats, PerfRecorder } from './perf/stats.ts';
import { audioOffReason, installAudio } from './audio/install.ts';
import { bindAudioStatus } from './audio/status.ts';
import { FileDialogView, SavedFilesView } from './ui/filedlg.ts';
import type { FileDlgRequest } from './engine/filedlg.ts';
import { mountFiles } from './files/mount.ts';
import { EngineFiles } from './files/engine-dialog.ts';
import { filesUsage } from './files/usage.ts';
import { AssetStore } from './bundle/assetstore.ts';
import { AssetsRecorder, assetLines, resourceLookupFromPerformance, sourceSummary, type AssetsReport } from './bundle/assets-report.ts';
import type { AssetRef } from './bundle/manifest.ts';

const $ = <T extends HTMLElement>(id: string): T => document.getElementById(id) as T;
const manifestUrl = '/assets/manifest.json';
const hzFrames: FrameHost = { frame: (cb) => { requestAnimationFrame(cb); }, setTimer: (fn, ms) => setTimeout(fn, ms), clearTimer: (h) => clearTimeout(h as ReturnType<typeof setTimeout>) };
// decision 0175/0211: the AudioWorklet output is always on; it wraps the Worker constructor, so it must run before the engine client makes the Worker. The numbers of the audio output are read from the URL until decision 0241 (aud1) is merged.
const audio = installAudio();
bindAudioStatus(document.getElementById('audio-status'), audioOffReason(), audio);

type AppState = 'idle' | 'loading' | 'loaded' | 'running' | 'engine-idle' | 'exited' | 'error';
interface AppApi {
  state: AppState;
  manifest: Manifest | null;
  /** the manifest's engine.kind once it is loaded, else null */
  engineKind: string | null;
  exit: ExitInfo | null;
  error: { what: string; where: string } | null;
  /** instants of the start-up in ms since the navigation started (performance.now() of the page): manifest_loaded, download_start, download_done, compile_done, worker_ready, engine_start, main_start, first_pixel, exit ... */
  timeline: Record<string, number>;
  /** what the Worker attached to its marks (wasm_instantiated: memoryBytes, files_ready: file counts and bytes, worker_done: memoryBytes ...) */
  details: Record<string, Record<string, unknown>>;
  features: FeatureReport | null;
  /** decision 0220: where the start-up assets came from and where the time went, as the beacon carries it; null before the downloads began */
  assetsReport(): AssetsReport | null;
  /** what the engine's printed lines say (stop message, windows, threads, surface reports); filled when the engine has exited */
  summary: EngineLogSummary | null;
  engineLog(): string[];
  /** decision 0199 / 0223: what document.title says and where it comes from (the title of the foreground window as the Worker reports it, else the page's own title) */
  pageTitle(): PageTitle;
  whenExited(): Promise<ExitInfo>;
  /** how many input events the page has sent to the engine Worker (decision 0141) */
  inputEvents: number;
  /** the input events with their latencies (page -> engine injection -> engine idle again -> frame drawn) */
  latency: LatencyTracker;
  /** the CSS cursor the desktop shows and why */
  cursor: { css: string; why: string };
  /** what the status line says now */
  engineStatus(): ReturnType<typeof engineState>;
  /** what the engine did with the input (the Worker's counters of the last idle) */
  driverStats: unknown;
  /** the last time the picture changed (for the stop message) */
  lastChanged: { at: number } | null;
  /** every idle the engine reported: when (ms, performance.timeOrigin based), how long it had computed since it was last resumed, its virtual clock and next deadline in ticks */
  idles: { at: number; busyMs: number | null; n: number; now: number; next: number }[];
  /** decision 0170: the size of the engine's wasm memory (bytes) as of the last idle report; null before the first. It only grows: a steady climb during playback is the engine's own allocation */
  engineMemoryBytes: number | null;
  /** decision 0221: what the engine has asked of its host files (open / stat / read counts and bytes; null: an old-format bundle) */
  hostFiles: HostFileStats | null;
  /** decision 0170: the running hash of every idle's (number, virtual clock, next deadline) at every 500th idle, by idle number */
  idleTrace: Record<number, number>;
  /** decision 0175/0211: the audio output, null when the browser lacks it */
  audio: typeof audio;
  /** decision 0182: the refresh rate the page measured and put into the engine's environment (`hz`; null: nothing, the engine runs at its own 60 Hz) */
  hz: { hz: number | null; measurement: HzMeasurement | null; warning: string | null };
  /** decision 0212: the guest's screen. `asked` is what the page put into the engine's environment (null: nothing, the engine's own 1920x1080); `reported` is what the engine says it uses (fle_av_poll: screen_w, screen_h; null until the first
   *  report, {0,0} from a build that does not report it); `guest` is what the desktop is drawn for now; `room` the room measured at the last layout */
  screen: { panelOpen: boolean; engineFollows: boolean | null; resizes: { room: Room; guest: { width: number; height: number }; sent: boolean; at: number }[]; asked: { width: number; height: number; env: Record<string, string> } | null; reported: { width: number; height: number } | null; guest: { width: number; height: number }; room: Room | null; notes: string[] };
  /** decision 0193: the page's answers to the engine's file dialogs (no file names) and the files the engine wrote (Save dialog targets) */
  fileDlg: { readonly open: boolean; readonly history: FileDialogView['history']; /** for the record of decision 0193: whether the page had a transient user activation (navigator.userActivation.isActive) and how long ago the page's last trusted pointer or key event was, when each engine request arrived */ readonly requests: { id: number; save: boolean; at: number; activationActive: boolean | null; msSinceInput: number | null; /** what the engine asked (its own strings: types, title, folder, suggested name) */ request: FileDlgRequest }[] };
  saved: SavedFilesView['files'];
  /** decision 0223: the desktop presenter's counters (as of its last report), and the SHA-256 of the desktop canvas read back in the Worker */
  deskStats: unknown;
  readbackDesk(): Promise<{ width: number; height: number; sha256: string }>;
  /** a test aid (any display method): the compositor's whole picture as the window table is now, as a SHA-256 / as base64 of its R, G, B, A bytes */
  deskDumpHash(): Promise<{ width: number; height: number; sha256: string }>;
  deskDumpBase64(peek?: boolean): Promise<{ windows: string; width: number; height: number; base64: string }>;
  /** decision 0240 (a test aid, when the desktop is shown with WebGL): the scene drawn again and read back from the WebGL canvas, base64 of R, G, B, A bytes */
  deskGlPixelsBase64(): Promise<{ width: number; height: number; base64: string }>;
  /** sends a raw event as the desktop would (for the tests that cannot use a pointer) */
  sendRaw(ev: RawInput): void;
  /** decision 0272: the restore button of a minimized window (the Worker reports the window; a build without the engine's taskbar entry never does) */
  taskbar: TaskbarRestore;
  stop(): void;
  /** the measurement beacon (decision 0152): what the page measures about its own speed and sends to POST /__perf of this origin */
  perf: {
    recorder: PerfRecorder;
    /** the beacon as it would be made now (does not send it, does not use up a sequence number) */
    snapshot(reason?: BeaconReason): BeaconPayload;
    /** sends a beacon now (the scheduled ones go out by themselves); resolves with the server's answer */
    sendNow(reason: BeaconReason): Promise<SendResult>;
    sent: number; accepted: number; stored: number;
    /** after the first HTTP refusal (4xx / 5xx) no more beacons are sent: the reason (null: sending) */
    disabled: string | null;
    /** decision 0192: the screen meter (requestAnimationFrame, canvas draws, long tasks, event timing) */
    display: DisplayMonitor;
    /** the screen measurement as the beacon would carry it now (does not send anything) */
    displaySnapshot(): DisplaySnapshot;
    /** the engine's rate against the screen's rAF rate, as the beacon would carry it now */
    hzInfo(): HzInfo;
    history: { seq: number; reason: BeaconReason; bytes: number; result: SendResult }[];
  };
}
declare global { interface Window { __app?: AppApi } }

const view = new DesktopView($('desktop'));
// decision 0193: the engine's file dialogs are answered here by the person; the answer (File objects) goes to the Worker, never to a server
const engineFiles = new EngineFiles((kind, text) => addLog(kind, text));
// decision 0216 stage 2 (opfs1): the engine's Open / Save are answered by the file manager window (OPFS)
const fileDlgView = new FileDialogView($('filedlg'), { answer: (id, answer) => client.answerFileDlg(id, answer), closed: () => $('desktop').focus({ preventScroll: true }), select: engineFiles.select });
// decision 0216 (opfs1): the browser-files window (OPFS); a header button and an overlay, the layout is untouched
const filesHandle = mountFiles(document.querySelector('header') as HTMLElement, () => $('desktop').focus({ preventScroll: true }));
(window as unknown as { __files: unknown }).__files = Object.assign(filesHandle, { engine: engineFiles });
const savedView = new SavedFilesView($('saved-files'), $('saved-list'));
const logEl = $('log');
const addLog = (level: string, text: string): void => {
  logEl.textContent += `[${level}] ${text}\n`;
  logEl.scrollTop = logEl.scrollHeight;
};
const timeline = new Timeline();
const progress = new ProgressView($('progress'));
const engineLog = new EngineLogView($('engine-log'), $('log-count'));
let hintFirstPixelS: number | undefined;
let failedStage: StageId | null = null;
let endKind: 'stopped' | 'finished' | 'trap' | 'idle' | null = null;
let phaseTimer: ReturnType<typeof setInterval> | null = null;
let cycleLast: number | null = null;
let cycleLastN = 0;
let endRequested = false;

const app: AppApi = {
  assetsReport: () => assetsNow(),
  state: 'idle', manifest: null, engineKind: null, exit: null, error: null, timeline: {}, details: {}, features: null, summary: null, inputEvents: 0, latency: new LatencyTracker(400, { framesAreChanges: false }), cursor: { css: 'default', why: 'not asked yet' }, driverStats: null, engineMemoryBytes: null, hostFiles: null, idleTrace: {}, idles: [], lastChanged: null, hz: { hz: null, measurement: null, warning: null },
  screen: { panelOpen: true, engineFollows: null, resizes: [], asked: null, reported: null, guest: { width: SCREEN_DEFAULT.width, height: SCREEN_DEFAULT.height }, room: null, notes: [] },
  engineLog: () => engineLog.lines,
  engineStatus: () => engineState(clock, performance.timeOrigin + performance.now()),
  audio,
  fileDlg: { get open() { return fileDlgView.open; }, history: fileDlgView.history, requests: [] },
  saved: savedView.files,
  sendRaw: (ev) => sendRaw(ev, 'test'),
  taskbar: null as unknown as TaskbarRestore,
  deskStats: null,
  readbackDesk: () => client.readback('desk'),
  deskDumpHash: async () => { const d = await client.deskDump(); return { width: d.width, height: d.height, sha256: await sha256Hex(new Uint8Array(d.data)) }; },
  deskDumpBase64: async (peek = false) => { const d = await client.deskDump(peek); let s = ''; const u = new Uint8Array(d.data); for (let i = 0; i < u.length; i += 0x8000) s += String.fromCharCode(...u.subarray(i, i + 0x8000)); return { windows: d.windows, width: d.width, height: d.height, base64: btoa(s) }; },
  deskGlPixelsBase64: async () => { const d = await client.deskGlPixels(); let s = ''; const u = new Uint8Array(d.data); for (let i = 0; i < u.length; i += 0x8000) s += String.fromCharCode(...u.subarray(i, i + 0x8000)); return { width: d.width, height: d.height, base64: btoa(s) }; },
  perf: null as unknown as AppApi['perf'],
  pageTitle: () => deskTitle(baseTitle),
  whenExited: () => client.whenExited(),
  stop: () => { endRequested = true; client.stop(); },
};
window.__app = app;

// decision 0223: the title of the foreground window as the Worker reported it (the page knows no window); the page's own title until there is one
let osTitle = '';
/** the one picture the Worker shows is the only 'window' the latency tracker is told about */
const DESKTOP = 0;
interface PageTitle { title: string; source: 'engine' | 'base'; why: string; lid: number | null }
function deskTitle(base: string): PageTitle {
  if (osTitle === '') return { title: base, source: 'base', why: 'the engine has reported no foreground window title yet', lid: null };
  return { title: osTitle, source: 'engine', why: 'the title of the foreground window', lid: null };
}
// decision 0199: the page's own title until the real engine has a visible main window; from then on document.title is that window's title (written only when it changed)
let baseTitle = document.title;
function syncPageTitle(): void {
  const t = app.pageTitle().title;
  if (document.title !== t) document.title = t;
}

function setState(s: AppState, text: string): void {
  app.state = s;
  const el = $('status');
  el.dataset.state = s;
  el.textContent = text;
}

// decision 0220: the loader's per-asset facts (recorder), the browser's resource timing for the network-path assets, and the stored copy (OPFS) with its clean-up
let assetsRecorder: AssetsRecorder | null = null;
const resourceLookup = resourceLookupFromPerformance();
function assetsNow(): AssetsReport | null { return assetsRecorder ? assetsRecorder.report(resourceLookup) : null; }

function stamp(name: string, at = performance.now()): void {
  timeline.mark(name, at);
  app.timeline = timeline.toJSON();
}

// decision 0182: the DOM is written only when the text it would show changed (a write of an unchanged text still invalidates style and layout, and the page then produces a frame: 8 of them a second for nothing once the stages are done)
let phasesKey = '';
function renderPhases(): void {
  const ar = assetsNow();
  const rows = stageRows(timeline, { now: performance.now(), failedStage, hintFirstPixelS, endKind, ...(ar && ar.items.length > 0 ? { assetSources: sourceSummary(ar) } : {}) });
  const key = rows.map((r) => `${r.id}\u0001${r.status}\u0001${r.label}\u0001${r.detail ?? ''}`).join('\u0002');
  if (key === phasesKey) return;
  phasesKey = key;
  const ol = $('phases');
  if (ol.children.length !== rows.length) ol.replaceChildren(...rows.map(() => document.createElement('li')));
  rows.forEach((r, i) => {
    const li = ol.children[i] as HTMLElement;
    li.dataset.stage = r.id;
    li.dataset.status = r.status;
    li.replaceChildren(document.createTextNode(r.label));
    if (r.detail) { const small = document.createElement('small'); small.textContent = r.detail; li.append(small); }
  });
}

function showError(what: string, where: string, stage: StageId | null = null, kind: FailureKind = 'boot-exception'): void {
  app.error = { what, where };
  clock.error = true;
  failedStage = stage ?? failedStage ?? 'download';
  stamp('error');
  setState('error', 'failed');
  const el = $('error');
  el.hidden = false;
  el.replaceChildren();
  const w = document.createElement('div');
  w.textContent = what;
  const wh = document.createElement('span');
  wh.className = 'where';
  wh.textContent = `where: ${where}`;
  el.append(w, wh);
  addLog('error', `${what} [${where}]`);
  renderPhases();
  if (phaseTimer) { clearInterval(phaseTimer); phaseTimer = null; }
  beacons.end('error');
  void failures.report(kind, { stage: stage ?? failedStage, where, message: what });   // decision 0257: the small report of the failure, apart from the big beacon
}

// ---- display: CSS size of the desktop (the room); the engine's clock state; the input ----
/** decision 0212: the room for the screen, in CSS pixels: the stage (everything below the header and left of the monitor panel) */
function measureRoom(): Room {
  const r = $('stage').getBoundingClientRect();
  return { width: r.width, height: r.height, dpr: window.devicePixelRatio || 1 };
}
function applyDisplay(): void {
  const dpr = window.devicePixelRatio || 1;
  // the desktop fills the stage: the scale is the room over the guest's screen (1 when the guest has exactly the pixels of the room)
  const room = measureRoom(), g = app.screen.guest;
  app.screen.room = room;
  // when the engine uses the size the page asked for, a guest pixel is exactly one CSS pixel; else the screen is fitted into the room
  const asked = app.screen.asked, honoured = asked !== null && asked.width === g.width && asked.height === g.height;
  const d = drawScale(g, room, honoured);
  const scale = Math.max(0.05, d.scale);
  view.setScreen(g.width, g.height);
  view.setScale(scale, d.snap);
  $('canvas-info').textContent = `guest screen ${g.width}x${g.height} in a room of ${room.width.toFixed(0)}x${room.height.toFixed(0)} CSS px (scale ${scale.toFixed(4)}, device pixel ratio ${dpr}, ${(g.width * g.height / 1e6).toFixed(2)} Mpx)`;
}
// decision 0212: the toolbar and the engine's status lines are in the monitor panel so that the stage is the desktop and nothing else; the panel button; the room follower
const panelBtn = $<HTMLButtonElement>('panel-toggle');
function setPanel(open: boolean): void {
  document.body.classList.toggle('panel-off', !open);
  app.screen.panelOpen = open;
  panelBtn.setAttribute('aria-pressed', open ? 'true' : 'false');
  panelBtn.textContent = open ? 'Monitor \u25B8' : '\u25C2 Monitor';
  applyDisplay();
}
$('resize-reload').addEventListener('click', () => location.reload());
document.body.classList.add('screen-fit');
{
  const sec = document.createElement('section');
  sec.id = 'display-section';
  const h = document.createElement('h2');
  h.textContent = 'Display and engine';
  sec.append(h, $('toolbar'), $('engine-line'), $('pointer-info').parentElement as HTMLElement);
  (document.querySelector('aside') as HTMLElement).prepend(sec);
}
panelBtn.addEventListener('click', () => setPanel(!app.screen.panelOpen));
setPanel(true);
view.onLayoutChange(() => { applyDisplay(); renderDesktopInfo(); });
window.addEventListener('resize', applyDisplay);
// decision 0212: the room changes with the browser window and with the panel; the picture is rescaled at once, and what the engine is told waits until the room has been still for a moment (RoomDebouncer)
const roomTimers = { set: (fn: () => void, ms: number): unknown => setTimeout(fn, ms), clear: (h: unknown): void => clearTimeout(h as ReturnType<typeof setTimeout>) };
const roomFollower = new RoomDebouncer((room) => followRoom(room), roomTimers);
if (typeof ResizeObserver !== 'undefined') new ResizeObserver(() => { applyDisplay(); roomFollower.push(measureRoom()); }).observe($('stage'));
window.matchMedia?.(`(resolution: ${window.devicePixelRatio}dppx)`).addEventListener?.('change', applyDisplay);

$('stop-btn').addEventListener('click', () => { endRequested = true; client.stop(); });

/** decision 0212: what the engine says about its screen (fle_av_poll); the desktop is drawn for what the engine uses, whatever the page asked for */
function syncGuestScreen(v: Pick<AppView, 'screenW' | 'screenH'>): void {
  const rep = { width: v.screenW ?? 0, height: v.screenH ?? 0 };
  const prev = app.screen.reported;
  if (prev && prev.width === rep.width && prev.height === rep.height) return;
  app.screen.reported = rep;
  const known = rep.width > 0 && rep.height > 0;
  const g = known ? rep : { width: SCREEN_DEFAULT.width, height: SCREEN_DEFAULT.height };
  if (!known && app.screen.asked) { const n = `this engine build does not report its screen size, so it did not take the ${app.screen.asked.width}x${app.screen.asked.height} that was asked: the page takes ${g.width}x${g.height}`; app.screen.notes.push(n); addLog('warn', n); }
  else if (known && app.screen.asked && (rep.width !== app.screen.asked.width || rep.height !== app.screen.asked.height)) { const n = `the engine reports a screen of ${rep.width}x${rep.height}; the page asked for ${app.screen.asked.width}x${app.screen.asked.height}`; app.screen.notes.push(n); addLog('warn', n); }
  else if (known) addLog('info', `the engine reports its screen as ${rep.width}x${rep.height}`);
  if (g.width !== app.screen.guest.width || g.height !== app.screen.guest.height) { app.screen.guest = { width: g.width, height: g.height }; applyDisplay(); }
}

/** decision 0212: the room settled at a size for which the guest's screen would be another: the engine is told (a display change: WM_DISPLAYCHANGE and WM_SETTINGCHANGE to the guest's windows; FLE_SCREEN_REMAXIMIZE=1 in the
 *  environment makes the runtime resize the maximized windows by the change). */
function followRoom(room: Room): void {
  const g = chooseGuestScreen(room);
  if (!app.screen.asked) return;
  if (g.width === app.screen.asked.width && g.height === app.screen.asked.height) { $('resize-note').hidden = true; return; }
  const rec = { room, guest: { width: g.width, height: g.height }, sent: false, at: nowAbs() };
  app.screen.resizes.push(rec);
  if (app.screen.resizes.length > 50) app.screen.resizes.shift();
  if (app.screen.engineFollows === false) { const n = 'this engine build has no display-change entry (fle_g3_display); a reload takes the new size'; if (app.screen.notes[app.screen.notes.length - 1] !== n) { app.screen.notes.push(n); addLog('warn', n); } return; }
  if (!inputEnabled()) return;   // before the first idle: followRoom runs again at the first idle
  sendRaw({ k: 'display', kind: IN.DISPLAY, width: g.width, height: g.height }, 'room', undefined, false);
  rec.sent = true;
  app.screen.asked = { width: g.width, height: g.height, env: screenEnv(g.width, g.height) };
  addLog('info', `the room is now ${room.width.toFixed(0)}x${room.height.toFixed(0)} CSS px: the engine is told its display is now ${g.width}x${g.height}`);
}

/** decision 0212: measures the room just before the engine is given its environment and asks for a screen of that size (FLE_MONITOR_W / FLE_MONITOR_H) */
function applyScreenParam(manifest: Manifest): void {
  const room = measureRoom();
  const g = chooseGuestScreen(room);
  const env = { ...screenEnv(g.width, g.height), FLE_SCREEN_REMAXIMIZE: '1' };
  Object.assign(manifest.engine.env, env);
  app.screen.asked = { width: g.width, height: g.height, env };
  app.screen.guest = { width: g.width, height: g.height };
  app.screen.room = room;
  applyDisplay();
  addLog('info', `guest screen ${g.width}x${g.height} for a room of ${room.width.toFixed(1)}x${room.height.toFixed(1)} CSS px at device pixel ratio ${room.dpr} (${Object.entries(env).map(([k, v]) => `${k}=${v}`).join(', ')})`);
  if (g.clamped) addLog('warn', `the room is outside what the engine accepts (${640}x${480}..${7680}x${4320}): the size was pulled inside it`);
}


// the engine's state as the page sees it (src/ui/engine-status.ts)
const clock: EngineClockState = { startedAt: null, firstIdleAt: null, lastIdleAt: null, lastResumeAt: null, ended: null, error: false, cycles: [] };
let engineHasInput = false;
const nowAbs = (): number => performance.timeOrigin + performance.now();
const inputEnabled = (): boolean => engineHasInput && clock.firstIdleAt !== null && clock.ended === null && app.error === null;

let inputSeq = 0;
let lastInput: { desc: string; at: number; screen: { x: number; y: number } | null } | null = null;
function sendRaw(ev: RawInput, from: string, meta?: InputMeta, user = true): void {
  const tPage = nowAbs();
  const seq = ++inputSeq;
  if (!user) { client.sendInput(seq, tPage, ev); return; }   // decision 0212: a display change is not a person's input: it is not counted and its answer is not a latency sample
  app.inputEvents++;
  app.latency.sent(seq, tPage, ev, from, meta ?? null);
  lastInput = { desc: `${describeRaw(ev)} (${from})`, at: tPage, screen: ev.k === 'mouse' ? { x: ev.x, y: ev.y } : lastInput?.screen ?? null };
  client.sendInput(seq, tPage, ev);
}
const sendTabBox = $<HTMLInputElement>('send-tab');
// decision 0272: the button that restores a minimized window; shown only while the engine reports one (a bundle built before decision 0267 reports none)
app.taskbar = new TaskbarRestore($('taskbar-restore') as HTMLButtonElement, { send: (ev) => sendRaw(ev, 'taskbar'), focusDesktop: () => $('desktop').focus({ preventScroll: true }) });
const desktopInput = attachDesktopInput($('desktop'), { enabled: inputEnabled, sendTab: () => sendTabBox.checked, onRaw: sendRaw, rect: () => view.rect(), screen: () => view.screenSize });
void desktopInput;

function renderDesktopInfo(): void {
  $('pointer-info').textContent = lastInput?.screen ? `pointer: screen (${lastInput.screen.x}, ${lastInput.screen.y})` : 'pointer: -';
  $('cursor-info').textContent = `cursor: ${app.cursor.css}${app.cursor.why ? ` (${app.cursor.why})` : ''}`;
}

function renderStatus(): void {
  syncPageTitle();
  const now = nowAbs();
  const st = engineState(clock, now);
  const el = $('engine-state');
  if (el.dataset.state !== st.state) el.dataset.state = st.state;
  if (el.textContent !== st.text) el.textContent = st.text;
  const lat = app.latency;
  const cyc = meanCycle(clock.cycles);
  const parts: string[] = [];
  parts.push(`input: ${lat.sentTotal} sent, ${lat.processedTotal} processed, ${lat.pending} pending${lat.droppedTotal ? `, ${lat.droppedTotal} dropped/merged` : ''}`);
  if (cyc !== null) parts.push(`engine cycle ${(cyc / 1000).toFixed(2)} s (${(1000 / cyc).toFixed(1)}/s)`);
  const last = lat.last;
  if (last && last.tIdle !== null) parts.push(`last input processed ${((last.tIdle - last.tPage) / 1000).toFixed(2)} s after the event${toAnyFrame(last) !== null ? `, drawn ${((toAnyFrame(last) as number) / 1000).toFixed(2)} s after it` : ''}`);
  if (lastInput) parts.push(`last input: ${lastInput.desc}`);
  const metrics = $('engine-metrics'), metricsText = parts.join(' · ');
  if (metrics.textContent !== metricsText) metrics.textContent = metricsText;
  const inputState = inputEnabled() ? 'on' : 'off', desktop = $('desktop');
  if (desktop.dataset.input !== inputState) desktop.dataset.input = inputState;
}
setInterval(renderStatus, 250);

function refreshPanels(): void {
  const s = summarizeEngineLog(engineLog.lines);
  app.summary = s;
  if (s.windows.length > 0) renderWindows($('windows').querySelector('tbody') as HTMLElement, $('win-count'), s.windows);
  renderThreads($('threads'), $('thr-count'), s.threadSummary, s.threads);
}

function describeExit(info: ExitInfo, manifest: Manifest | null): { text: string; notes: string[]; kind: 'stopped' | 'finished' | 'trap' } {
  refreshPanels();
  const s = app.summary as EngineLogSummary;
  const opts = manifest?.engine.options as unknown as FruityOptions | undefined;
  const notes = matchStopNotes(s.runtimeMessage, opts?.stopNotes);
  if (info.reason === 'trap') return { text: `The engine crashed: ${info.message}`, notes: [], kind: 'trap' };
  const lines = stopLines({
    info, runtimeMessage: s.runtimeMessage, stopLine: s.stopLine, endedFromPage: endRequested, now: nowAbs(), lastInput: lastInput ? { desc: lastInput.desc, at: lastInput.at } : null, pendingInput: app.latency.pending, lastPictureChange: app.lastChanged ? { at: app.lastChanged.at } : null,
  });
  return { text: lines.join('\n'), notes, kind: info.reason === 'finished' ? 'finished' : 'stopped' };
}

// ---- the measurement beacon and the Performance panel (decision 0152; targets P1-P3: docs/milestones/performance-targets.md) ----
const perfRecorder = new PerfRecorder();
// decision 0209: the engine's last words (its own message, personal parts removed), made once when the engine ends; carried by the beacon as engine.exit.message and shown in the Performance panel
let exitMessage: string | null = null;
// decision 0175 (S-ur-window): the P3 row counts the underrun quanta since the engine's first idle (the start-up before it is not listened to); the note gives the last minute and the device state
const audioMeasured = (): boolean => (audio?.detail().quantaWhilePlaying ?? 0) > 0;
const audioP3 = (): number => { const d = audio?.detail(); return d ? (d.underrunQuantaSinceFirstIdle ?? d.underrunQuanta) : 0; };
const audioNote = (): string => { const d = audio?.detail(); return d ? `last 60 s: ${d.underrunQuantaLast60s ?? 0}, since the first sound: ${d.underrunQuantaSinceFirstSound ?? 'no sound yet'}, device ${d.deviceState} ${d.playingSeconds.toFixed(0)} s, non-silent quanta ${d.nonSilentQuanta}, silence the engine filled in (last 60 s) ${d.engineGapFramesLast60s ?? 'n/a'} frames, status ${d.status}` : ''; };
let glRenderer: string | null | undefined;
let beaconSeq = 0;
// decision 0192: one random token per page load, so that the log can group the beacons of one load (12 hex digits; not kept anywhere in the browser)
const beaconSid = Array.from(crypto.getRandomValues(new Uint8Array(6)), (b) => b.toString(16).padStart(2, '0')).join('');
function browserNow(): BrowserInfo {
  if (glRenderer === undefined) glRenderer = webglRenderer(document);
  return { ...collectBrowserInfo({ navigator, screen, devicePixelRatio: window.devicePixelRatio, crossOriginIsolated: window.crossOriginIsolated, isSecureContext: window.isSecureContext, performance: performance as unknown as { memory?: { usedJSHeapSize?: number; totalJSHeapSize?: number; jsHeapSizeLimit?: number } } }), webglRenderer: glRenderer };
}
// decision 0192: the screen meter. Started at once. It only reads clocks and counts; it never touches the engine, the Worker or a canvas.
const displayMonitor = new DisplayMonitor(browserDisplayHost());
displayMonitor.start();
const clientNow = (): ReturnType<typeof collectClientInfo> => collectClientInfo({ navigator, screen, innerWidth: window.innerWidth, innerHeight: window.innerHeight, isSecureContext: window.isSecureContext, location });
function displaySnapshot(): DisplaySnapshot {
  return displayMonitor.snapshot(false, app.latency.sentTotal, { sent: null, omitted: null });
}
/** decision 0212: what the beacon says about the display (additive block `guestScreen` of version 3) */
function guestScreenInfo(): GuestScreenInfo {
  const sc = app.screen;
  const r1 = (v: number): number => Math.round(v * 10) / 10;
  return { mode: 'fit', source: 'default', width: sc.guest.width, height: sc.guest.height, reported: sc.reported !== null && sc.reported.width > 0, dpi: null, roomWidth: sc.room ? r1(sc.room.width) : null, roomHeight: sc.room ? r1(sc.room.height) : null, dpr: Math.round((window.devicePixelRatio || 1) * 10000) / 10000, follow: 'live' };
}
function hzInfo(): HzInfo {
  const env = app.hz.hz;
  return hzFit({ engineEnvHz: env, engineHz: env ?? HZ_DEFAULT, autoMeasuredHz: app.hz.measurement?.measuredHz ?? null }, displaySnapshot().raf);
}
/** decision 0240: the presenter's counters as of the previous beacon that was sent (the next beacon reports the difference) */
let prevDeskBeacon: { stats: Record<string, unknown>; atMs: number } | null = null;
function makeBeacon(reason: BeaconReason, seq: number): BeaconPayload {
  const snap = perfRecorder.snapshot(nowAbs());
  const lat = app.latency;
  return buildBeacon({
    seq, sid: beaconSid, reason, pageMs: performance.now(), visibility: document.visibilityState, browser: browserNow(),
    bundle: { name: app.manifest?.bundle.name ?? null, version: app.manifest?.bundle.version ?? null, engineKind: app.engineKind },
    state: app.state, exit: app.exit ? { reason: app.exit.reason, code: app.exit.code, presents: app.exit.presents, message: exitMessage } : null, error: app.error,
    timeline: app.timeline, updates: snap.updates, idles: snap.idles,
    input: inputLatencyStats(lat.records, lat.totals(), undefined, lat.framesAreChanges),
    underruns: audioMeasured() ? audioP3() : null, // null: nothing measured (no AudioWorklet, or the audio output has not run yet: the browser starts it at the first gesture)
    audioDetail: audio ? { ...audio.detail() } : null,
    wasmMemoryBytes: app.engineMemoryBytes,
    client: clientNow(),
    display: displayMonitor.isRunning ? displaySnapshot() : null,
    hz: displayMonitor.isRunning ? hzInfo() : null,
    guestScreen: guestScreenInfo(),
    files: filesUsage.snapshot(),
    assets: assetsNow(),
    desk: app.engineKind === 'emscripten-fruity-v1' ? deskBeacon(app.deskStats as Record<string, unknown> | null, prevDeskBeacon, nowAbs()) : null,
    engineRun: engineRunBeacon(app.idles, nowAbs()),
  });
}
const perfState = { sent: 0, accepted: 0, stored: 0, lastNote: null as string | null, disabled: null as string | null, history: [] as AppApi['perf']['history'] };
async function sendBeacon(reason: BeaconReason): Promise<SendResult> {
  if (perfState.disabled !== null) return { ok: false, status: null, stored: null, note: `not sent: ${perfState.disabled}` };
  const seq = beaconSeq++;
  const text = beaconText(makeBeacon(reason, seq));
  if (app.deskStats) prevDeskBeacon = { stats: { ...(app.deskStats as Record<string, unknown>) }, atMs: nowAbs() };
  perfState.sent++;
  const result = await postBeacon(text);
  if (result.ok) { perfState.accepted++; if (result.stored) perfState.stored++; }
  else if (result.status !== null) { perfState.disabled = `the server answered HTTP ${result.status} to a beacon; no more are sent`; addLog('warn', `measurement beacon refused: HTTP ${result.status}; no more beacons are sent from this page`); }
  perfState.lastNote = `#${seq} ${reason}: ${result.note}`;
  perfState.history.push({ seq, reason, bytes: text.length, result });
  if (perfState.history.length > 40) perfState.history.shift();
  return result;
}
const beacons = new BeaconScheduler({ send: (r) => { void sendBeacon(r); }, setTimer: (fn, ms) => setTimeout(fn, ms), clearTimer: (h) => clearTimeout(h as ReturnType<typeof setTimeout>) });
// decision 0257 (beacon2): the small report of a failure, and the three triggers the big beacon does not have (stuck before the first idle, left before the first idle, the page's uncaught errors); once per kind, silent on failure, no option
const failures = new FailureReporter({
  facts: () => ({ seq: beaconSeq++, sid: beaconSid, pageMs: performance.now(), visibility: document.visibilityState, browser: browserNow(), client: clientNow(), bundle: { name: app.manifest?.bundle.name ?? null, version: app.manifest?.bundle.version ?? null, engineKind: app.engineKind }, timeline: app.timeline, state: app.state }),
  capabilities: () => detectCapabilities(),
  post: postFailure,
});
installFailureWatch(
  { setTimer: (fn, ms) => setTimeout(fn, ms), clearTimer: (h) => clearTimeout(h as ReturnType<typeof setTimeout>), listen: (type, fn) => window.addEventListener(type, fn) },
  failures,
  { reachedIdle: () => app.timeline.idle !== undefined, failed: () => app.error !== null, ended: () => app.exit !== null },
);
app.perf = {
  recorder: perfRecorder,
  snapshot: (reason = 'periodic') => makeBeacon(reason, beaconSeq),
  sendNow: sendBeacon,
  get sent() { return perfState.sent; }, get accepted() { return perfState.accepted; }, get stored() { return perfState.stored; }, get disabled() { return perfState.disabled; },
  display: displayMonitor, displaySnapshot, hzInfo,
  history: perfState.history,
};
function panelView(): PanelView {
  const snap = perfRecorder.snapshot(nowAbs());
  const idleMs = app.timeline.idle;
  const waiting = clock.startedAt !== null && idleMs === undefined && clock.ended === null && app.error === null;
  return {
    waitingSeconds: waiting ? performance.now() / 1000 : null,
    firstIdleSeconds: typeof idleMs === 'number' ? idleMs / 1000 : null,
    updates: snap.updates.recent,
    input: inputLatencyStats(app.latency.records, app.latency.totals(), undefined, app.latency.framesAreChanges),
    underruns: audioMeasured() ? audioP3() : null,
    underrunsNote: audio ? audioNote() : undefined,
    wasmMemoryBytes: app.engineMemoryBytes,
    server: { sent: perfState.sent, accepted: perfState.accepted, stored: perfState.stored, lastNote: perfState.lastNote, off: perfState.disabled },
    exit: app.exit ? { reason: app.exit.reason, code: app.exit.code, message: exitMessage } : null,
    display: displayMonitor.isRunning ? displaySnapshot() : null,
    hz: displayMonitor.isRunning ? hzInfo() : null,
    client: clientNow(),
    browser: browserNow(),
    assets: assetsNow(),
  };
}
const perfPanel = $<HTMLDetailsElement>('perf-panel');
function renderPerf(): void {
  if (!perfPanel.open) return;
  const v = panelView();
  renderPerfPanel($('perf-body'), perfRows(v), perfNotice());
}
perfPanel.addEventListener('toggle', () => { if (perfPanel.open) displayMonitor.start(); renderPerf(); });
$('perf-send-note').textContent = ' · 測定値はこのサーバーに送られます';
setInterval(renderPerf, 1000);

const client = new EngineClient({
  deskCanvas: () => view.deskCanvas(),
  osstate(m) {
    const c = cursorCss({ cursor: m.cursor, cursorIdc: m.cursorIdc, cursorShared: m.cursorShared });
    app.cursor = c;
    view.setCursor(c.css);
    osTitle = m.title;
    syncPageTitle();
    syncGuestScreen({ screenW: m.screenW, screenH: m.screenH });
    renderDesktopInfo();
  },
  taskbar(m) { app.taskbar.update(m.lid); },
  desk(m) {
    app.deskStats = m.stats;
    if (m.answers && m.at.length > 0) app.latency.answer(m.answers, DESKTOP, m.at[m.at.length - 1] as number);
    for (const t of m.at) { app.latency.frame(DESKTOP, t, t, t); app.lastChanged = { at: t }; perfRecorder.update(t); }
  },
  log: addLog,
  mark(name, at, detail) {
    stamp(name, at);
    if (detail) app.details[name] = detail;
    if (name === 'first_pixel') { if (stateAfterFirstPixel(app.state) === 'running') setState('running', 'running · first pixel'); addLog('info', `first pixel at ${(at / 1000).toFixed(1)} s`); }
  },
  engineLog(stream, lines) { engineLog.add(stream, lines); },
  idle(m) {
    const at = m.abs;
    clock.lastIdleAt = at;
    const sinceN = m.n - cycleLastN;
    perfRecorder.idle(at, Math.max(1, sinceN));
    if (clock.firstIdleAt === null) {
      clock.firstIdleAt = at;
      engineHasInput = m.caps.input;
      app.screen.engineFollows = m.caps.display ?? false;
      stamp('idle');
      refreshPanels();
      endKind = 'idle';
      setState('engine-idle', 'engine idle (waiting for input)');
      $('stop-btn').hidden = false;
      $('stop-btn').textContent = 'End session';
      addLog('info', `engine idle (${m.n}) at virtual clock ${(m.now / 10000).toFixed(0)} ms; input ${m.caps.input ? 'is accepted from now on' : 'is NOT supported by this build'}; window list ${m.caps.appview ? 'is read from the engine' : 'is not available (no fle_av_poll)'}`);
      if (m.caps.input && document.activeElement === document.body) $('desktop').focus({ preventScroll: true });
      renderDesktopInfo();
      displayMonitor.markIdle(); // the "since the first idle" windows of the screen meter start here, before the beacon that reports them
      beacons.firstIdle(); // after the state is "engine-idle": the beacon carries it
      if (app.screen.asked) followRoom(measureRoom());   // decision 0212: the room may have changed while the engine started
    }
    fileDlgView.engineIdle();
    app.idles.push({ at, busyMs: m.busyMs, n: m.n, now: m.now, next: m.next });
    if (app.idles.length > 5000) app.idles.shift();
    const prev = cycleLast;
    if (prev !== null) { (clock.cycles as number[]).push((at - prev) / Math.max(1, sinceN)); if (clock.cycles.length > 100) (clock.cycles as number[]).shift(); }
    cycleLast = at;
    cycleLastN = m.n;
    app.driverStats = m.stats;
    if (m.trace) for (const c of m.trace) app.idleTrace[c.n] = c.hash;
    if (m.memBytes !== undefined) app.engineMemoryBytes = m.memBytes;
    if (m.hostFiles) app.hostFiles = m.hostFiles;
  },
  resume(m) { clock.lastResumeAt = m.abs; },
  fileDlg(id, request) {
    addLog('info', `the engine opened its ${request.save ? 'Save' : 'Open'} dialog: it is waiting for the person at the page (dialog ${id})`);
    app.fileDlg.requests.push({ id, save: request.save, at: nowAbs(), activationActive: navigator.userActivation ? navigator.userActivation.isActive : null, msSinceInput: lastInput ? nowAbs() - lastInput.at : null, request });
    fileDlgView.show(id, request);
  },
  fileDlgProgress: (id, done, total) => fileDlgView.progress(id, done, total),
  savedFile(f) {
    addLog('info', `the engine finished writing a file it was asked to save (${f.size} bytes); it is listed under "Files the engine wrote"`);
    savedView.add(f);
    void engineFiles.onSavedFile(f);
  },
  inputDone: (items) => app.latency.done(items),
  inputDropped: (seq, reason) => app.latency.dropped(seq, reason),
  exit(info) {
    fileDlgView.hide();   // decision 0193: nobody is left to answer a box the ended engine asked
    app.taskbar.clear();   // decision 0272: nor to restore a window
    app.exit = info;
    exitMessage = engineExitMessage(engineLog.lines, info);
    clock.ended = { reason: info.reason, code: info.code };
    stamp('exit');
    const d = describeExit(info, app.manifest);
    endKind = d.kind;
    if (info.reason === 'trap') failedStage = 'end';
    setState('exited', `engine ${info.reason === 'finished' ? 'finished' : info.reason === 'trap' ? 'crashed' : 'stopped'}`);
    const el = $('exit');
    el.hidden = false;
    el.dataset.kind = d.kind;
    el.replaceChildren(document.createTextNode(d.text));
    for (const n of d.notes) { const s = document.createElement('span'); s.className = 'note'; s.textContent = n; el.append(s); }
    $('stop-btn').hidden = true;
    addLog(info.reason === 'trap' ? 'error' : 'info', `exit ${info.reason} code ${info.code}: ${info.message}`);
    renderPhases();
    if (phaseTimer) { clearInterval(phaseTimer); phaseTimer = null; }
    // a crash of the engine (including running out of memory while it starts) is also an error of the page: what and where
    if (info.reason === 'trap') showError(info.message.split('\n')[0] ?? info.message, 'engine worker, running (the engine trapped before it could stop by itself)', 'end', 'engine-trap');
    beacons.end('exit');
  },
  error: (phase, message) => showError(message, `engine worker, ${phase}`, 'end', 'engine-error'),
});

// decision 0182: the refresh rate goes into the engine's environment before the Worker is given the manifest. The page measures its frame rate from the start of boot() (the manifest fetch and the downloads
// wait for the network meanwhile) and applies the result after the downloads: the measurement does not delay the start (decision 0243).
function applyHzParam(manifest: Manifest, measured: HzMeasurement | null): void {
  const r = applyHz(measured, manifest.engine.env);
  app.hz.measurement = measured;
  app.hz.hz = r.hz;
  app.hz.warning = r.warning;
  for (const l of r.lines) addLog(l.level, l.text);
}


// decision 0220: after the downloads: the browser's storage state, and the removal of the stored copies of older versions (the ones the manifest no longer names and no other page holds)
async function finishAssets(store: AssetStore | null, recorder: AssetsRecorder, keep: AssetRef[]): Promise<void> {
  if (!store) return;
  try {
    recorder.setStorage(await store.estimate());
    await recorder.writesSettled();
    if (keep.every((r) => recorder.stored(r))) recorder.setClean(await store.clean(keep));
    recorder.setStorage(await store.estimate());
  } catch (e) { addLog('warn', `stored copies: ${String((e as Error)?.message ?? e).slice(0, 200)}`); }
  const r = recorder.report(resourceLookup);
  for (const line of assetLines(r)) addLog('info', `asset: ${line}`);
}

async function boot(): Promise<void> {
  stamp('page_script');
  const hzMeasuring = measureFrameRate(hzFrames, { giveUpMs: 2000 });
  try {
    setState('loading', 'loading manifest');
    addLog('info', `bundle ${manifestUrl}`);
    phaseTimer = setInterval(renderPhases, 250);
    renderPhases();
    const manifest = await fetchManifest(manifestUrl);
    app.manifest = manifest;
    app.engineKind = manifest.engine.kind;
    stamp('manifest_loaded');
    $('bundle-info').textContent = `${manifest.bundle.name} ${manifest.bundle.version} · engine ${manifest.engine.kind}${manifest.bundle.description ? ` · ${manifest.bundle.description}` : ''}`;
    $('bundle-info').title = $('bundle-info').textContent;   // the header cuts a long text with an ellipsis (style.css): all of it is here
    baseTitle = `${manifest.bundle.name} · FL Studio web (private)`;
    syncPageTitle();
    app.latency.framesAreChanges = true;   // the desktop is presented only when a pixel changed (the compositor reports what differs), so a frame is a change
    addLog('info', 'the runtime composes the windows into one picture of the screen, the engine Worker shows it on one canvas');
    const fopts = manifest.engine.options;
    hintFirstPixelS = fopts.hints?.firstPixelSeconds;

    // what the engine needs from the browser is checked before the big downloads, and a missing feature is named
    const features = detectFeatures();
    app.features = features;
    const blocking = blockingMissing(features);
    if (blocking.length > 0) { const d = describeMissing(blocking); showError(d.what, d.where, 'download', 'feature-missing'); return; }
    if (typeof SharedArrayBuffer === 'undefined') { showError('the engine takes its input through a SharedArrayBuffer (a cross-origin isolated page): this browser or page lacks it', 'feature check (before any download)', 'download', 'no-shared-memory'); return; }

    setState('loading', 'downloading engine and data');
    const expected = ['engine wasm', 'seed pack', 'DLL pack', 'engine glue'];
    const assetsDone = new Set<string>();
    const onProgress = (p: Progress): void => {
      stamp('download_start', p.at);
      progress.update(p, p.label === 'engine wasm' && !p.done ? ' · compiling as it arrives' : '');
      if (p.done) { assetsDone.add(p.label); if (expected.every((l) => assetsDone.has(l))) stamp('download_done', p.at); }
    };
    // decision 0220: the stored copy lives in OPFS; a browser without it loads from the network every time
    const store = AssetStore.open();
    const recorder = new AssetsRecorder(store ? 'on' : 'none', hashModeOfThisBrowser());
    const base = location.href;
    assetsRecorder = recorder;
    const refs: [string, AssetRef][] = [['engine wasm', manifest.engine.wasm], ['seed pack', manifest.seedPack], ['DLL pack', manifest.dllPack], ['engine glue', fopts.glue]];
    for (const [label, r] of refs) recorder.expect(label, r, base);
    const opts = { onProgress, store, onTrace: recorder.trace };
    const [module, seed, dll, glue]: [WebAssembly.Module, LoadedPack, LoadedPack, Uint8Array] = await Promise.all([
      loadWasm(manifest.engine.wasm, 'engine wasm', opts).then((m) => { stamp('compile_done'); return m; }),
      loadPack(manifest.seedPack, 'seed pack', opts),
      loadPack(manifest.dllPack, 'DLL pack', opts),
      fetchAsset(fopts.glue, 'engine glue', opts),
    ]);
    stamp('download_done');
    void finishAssets(store, recorder, refs.map(([, r]) => r));
    applyHzParam(manifest, await hzMeasuring);
    applyScreenParam(manifest);
    setState('loaded', 'starting engine');
    await client.init(manifest, { module, seed: seed.bytes, glue, dll: dll.bytes });
    stamp('worker_ready');
    setState('running', 'engine starting');
    stamp('engine_start');
    clock.startedAt = nowAbs();
    perfRecorder.start(clock.startedAt);
    $('stop-btn').hidden = false;
    $('stop-btn').textContent = 'Stop engine';
    client.start();
  } catch (e) {
    if (e instanceof LoadError) {
      const d = e.describe();
      showError(d.what, d.where, e.stage === 'compile' ? 'compile' : 'download', 'load-failed');
    } else showError(String((e as Error)?.message ?? e), 'app');
  }
}

void boot();
