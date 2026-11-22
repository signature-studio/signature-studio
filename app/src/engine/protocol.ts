// Messages between the page and the engine Worker (module worker). Plain structured-clone data plus the transferables named in the comments.
import type { Manifest } from '../bundle/manifest.ts';
import type { RawInput } from '../input/events.ts';
import type { DriverStats } from './idle-driver.ts';
import type { FileDlgAnswer, FileDlgRequest } from './filedlg.ts';
import type { HostFileStats } from './hostfiles.ts';
import type { LazyFileShared } from './lazy-file.ts';

/** decision 0223: what the page hands the engine Worker for the desktop: the input ring (shared), the handshake block with the render Worker (desk-link.ts) and the port to the render Worker. The engine composes; it does not draw. */
export interface DeskHandover { ring: SharedArrayBuffer; ctl: SharedArrayBuffer; port: MessagePort }
/** C-2: what the page hands the render Worker: the ONE canvas of the desktop (transferred), the port to the engine Worker and the handshake block (desk-link.ts). It draws the picture the engine composed. */
export interface RenderHandover { canvas: OffscreenCanvas; port: MessagePort; ctl: SharedArrayBuffer }

/** What the page loaded and verified for the engine (transferred to the Worker). `lazy`: the manifest's lazy files, in its order (none when absent); their bytes come later, through shared memory (lazy-file.ts), so they are not transferred. */
export interface BundleAssets {
  module: WebAssembly.Module; seed: Uint8Array; glue: Uint8Array; dll: Uint8Array;
  lazy?: LazyFileShared[];
}

export type ToWorker =
  | { cmd: 'init'; manifest: Manifest; assets: BundleAssets; /** the input ring and the link to the render Worker */ desk: DeskHandover }
  | { cmd: 'start' }
  | { cmd: 'end-idle' }
  /** a raw input event (decision 0141). `seq` numbers the page's events; `tPage` is performance.timeOrigin + performance.now() of the page when the DOM event was handled */
  | { cmd: 'input'; seq: number; tPage: number; ev: RawInput }
  /** decision 0193: the person's answer to the engine's file dialog (the File objects are handed to the Worker, which reads them into the engine's memory) */
  | { cmd: 'filedlg-answer'; id: number; answer: FileDlgAnswer }
  /** decision 0223: the page wrote to the input ring while the Worker was parked */
  | { cmd: 'wake' }
  /** decision 0223 (a test aid, any display method): the compositor draws the whole desktop from the window table as it is now and the Worker sends the picture (R, G, B, A bytes) */
  | { cmd: 'desk-dump' }
  /** decision 0223 (a test aid, display method D): the picture as the compositor holds it now, without composing (what the canvas has been given is compared with it) */
  | { cmd: 'desk-peek' }
  /** C-6 (#1310, a test aid): the engine's window-API trace (window-api: lines on its stderr) on or off; off at the start */
  | { cmd: 'window-api-trace'; on: boolean }
  | { cmd: 'ping'; id: number };

/** C-2: the messages the page sends the render Worker (the canvas is drawn only here) */
export type ToRender =
  | { cmd: 'init'; desk: RenderHandover }
  | { cmd: 'readback'; key: string }
  /** decision 0240 (a test aid, when the desktop is shown with WebGL): the scene drawn again and read back from the GL canvas (R, G, B, A bytes) */
  | { cmd: 'desk-gl-read' };

/** C-2: on the MessagePort, the engine Worker to the render Worker. `desk-ready`: the engine has the compositor (scene: the description is drawn with WebGL when the canvas can). `frame`: the picture was composed (changed) or the render Worker asked and nothing changed;
 *  `buffer` is the engine's shared memory (the picture and the window surfaces are read from it), `ptr` the address of the compositor's state (or of the scene); `answers`: the inputs this change answers; `read-pixels`: a test aid asks for the picture drawn again (WebGL) */
export type EngineToRender =
  | { t: 'desk-ready'; scene: boolean }
  | { t: 'frame'; changed: boolean; gen: number; ptr: number; buffer?: SharedArrayBuffer; composeMs: number; ringReads: number; wakes: number; answers?: number[] }
  | { t: 'read-pixels' };

/** C-2: on the same port, the render Worker to the engine Worker. `want`: compose now (the render Worker asks at its frame loop, or once in a while while the picture does not move); `resync`: the next compose reports the whole picture; `scene-mode`: the description is drawn with WebGL from now on; `pixels` / `pixels-error`: the answer to read-pixels */
export type RenderToEngine =
  | { cmd: 'want' }
  | { cmd: 'resync' }
  | { cmd: 'scene-mode' }
  | { cmd: 'pixels'; width: number; height: number; data: ArrayBuffer }
  | { cmd: 'pixels-error'; message: string };

export type FromWorker =
  | { t: 'ready'; kind: string }
  | { t: 'mark'; name: string; /** performance.timeOrigin + performance.now() of the Worker */ abs: number; detail?: Record<string, unknown> }
  | { t: 'engine-log'; stream: 'out' | 'err'; lines: string[] }
  /** the engine entered its idle: its number, the virtual clock and its next deadline in ticks, how long it ran since it was last resumed (null: first idle), what the Worker did with the input so far */
  | { t: 'idle'; n: number; now: number; next: number; busyMs: number | null; abs: number; stats: DriverStats; queued: number; caps: { input: boolean; appview: boolean; display?: boolean }; trace?: { n: number; hash: number }[]; memBytes?: number; /** decision 0221: what the engine has asked of its host files so far (a bundle of the current format; the reads all happen before the first idle) */ hostFiles?: HostFileStats; /** decision 0187 (only with ?dedupe=1): rectangles omitted as unchanged / sent, pixels compared */ dedupe?: { omitted: number; sent: number; comparedPixels: number } }
  /** the engine was resumed (it computes until its next idle) */
  | { t: 'resume'; abs: number; why: 'input' | 'deadline' | 'end' }
  /** events the engine has processed (it is idle again): when the page made them, when the Worker gave them to the engine, when the engine was idle again (all performance.timeOrigin-based ms) */
  | { t: 'input-done'; items: { seq: number; tPage: number; tInject: number; tIdle: number }[] }
  | { t: 'input-dropped'; seq: number; reason: string }
  | { t: 'log'; level: 'info' | 'warn' | 'error'; text: string }
  | { t: 'exit'; reason: 'finished' | 'stopped' | 'trap'; code: number; message: string; presents: number }
  | { t: 'error'; phase: string; message: string }
  | { t: 'readback'; key: string; width: number; height: number; sha256: string }
  | { t: 'readback-error'; key: string; message: string }
  /** decision 0193: the engine is suspended in IFileOpenDialog::Show / IFileSaveDialog::Show and asks the person at the page */
  | { t: 'filedlg-request'; id: number; request: FileDlgRequest }
  /** decision 0193: bytes of a large chosen file read into the engine's memory so far */
  | { t: 'filedlg-progress'; id: number; done: number; total: number }
  /** decision 0193: a file the engine saved (a Save dialog target the person named) is complete on its virtual volume; `data` is a copy of its bytes (transferred) */
  | { t: 'saved-file'; guestPath: string; name: string; size: number; data: ArrayBuffer }
  | { t: 'pong'; id: number }
  | { t: 'desk-dump'; width: number; height: number; data: ArrayBuffer; /** the compositor's line per window drawn */ windows: string }
  | { t: 'desk-dump-error'; message: string }
  | { t: 'desk-gl-pixels'; width: number; height: number; data: ArrayBuffer }
  /** decision 0223: what the operating system says that is not a picture: the cursor SetCursor chose (low 32 bits of the handle, standard cursor identifier, module cursor) and the title of the foreground (active) window; sent when it changed */
  | { t: 'osstate'; cursor: number; cursorIdc: number; cursorShared: boolean; title: string; /** the size of the virtual screen the guest is told (0 from a build that does not report it) */ screenW: number; screenH: number; abs: number }
  /** decision 0223: the desktop presenter's counters and the instants (performance.timeOrigin + performance.now() of the Worker) of the presents since the last report, at most every 100 ms; `windows` is the compositor's text (?debug=1 only) */
  /** decision 0272: the first minimized window that has a taskbar button, as the engine says it (fle_g3_taskbar_iconic; 0: none); sent when it changed, never by a build without the export */
  | { t: 'taskbar'; lid: number }
  | { t: 'desk'; stats: { presents: number; rects: number; pixels: number; composeMs: number; putMs: number; whole: number; ringReads: number; ringFull: number; wakes: number; /** C-2: compose requests made by the render Worker's frame loop (its animation frames) / compose requests made while the picture did not move (its wake-up poll, render-worker.ts DESK_WAKE_MS) */ ticks: number; idleTicks: number; /** times the Worker stopped asking for animation frames because the picture did not change (desk2) */ sleeps: number }; at: number[]; /** the inputs the present in this message answers (decision 0223: the Worker made it in the turn that processed them) */ answers?: number[]; nWindows: number };
