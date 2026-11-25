// Messages between the page and the engine Worker (module worker). Plain structured-clone data plus the transferables named in the comments.
import type { Manifest } from '../bundle/manifest.ts';
import type { RawInput } from '../input/events.ts';
import type { DriverStats, ResumeWhy } from './idle-driver.ts';
import type { FileDlgAnswer, FileDlgRequest } from './filedlg.ts';
import type { HostFileStats } from './hostfiles.ts';
import type { LazyFileShared } from './lazy-file.ts';
import type { CursorPictureMsg } from './cursor-picture.ts';

/** decision 0223: what the page hands the engine Worker for the desktop: the input ring (shared), the handshake block with the render Worker (desk-link.ts) and the port to the render Worker. The engine composes; it does not draw. */
export interface DeskHandover { ring: SharedArrayBuffer; ctl: SharedArrayBuffer; port: MessagePort }
/** C-2: what the page hands the render Worker: the ONE canvas of the desktop (transferred), the port to the engine Worker and the handshake block (desk-link.ts). It draws the picture the engine composed. */
export interface RenderHandover { canvas: OffscreenCanvas; port: MessagePort; ctl: SharedArrayBuffer }

/** #1358 (a test aid): a text the engine's memory holds, the one with the most lines: the marker it was found by, its encoding (ASCII, or UTF-16LE), the place (byte offset in the memory), and its non-empty lines */
export interface GuestTextBest { marker: string; enc: 'ascii' | 'utf16le'; offset: number; lineCount: number; lines: string[] }
/** #1358 (a test aid): the printable runs (of at least two characters) within the radius of a place a marker was found at */
export interface GuestTextNear { marker: string; enc: 'ascii' | 'utf16le'; offset: number; runs: string[] }

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
  /** decision 0223 (a test aid, any display method): the compositor draws the whole desktop from the window table as it is now and the Worker sends the picture (R, G, B, A bytes) */
  | { cmd: 'desk-dump' }
  /** decision 0223 (a test aid, display method D): the picture as the compositor holds it now, without composing (what the canvas has been given is compared with it) */
  | { cmd: 'desk-peek' }
  /** #1358 (a test aid, any display method): the text the engine's memory holds for the markers (read only; radius > 0 also lists the printable runs near each place) */
  | { cmd: 'guest-text'; markers: string[]; radius: number }
  /** #1384 (a test aid): the window with logical id `lid` and its visible descendants are drawn again (fle_g3_redraw_tree); the engine is woken to paint them */
  | { cmd: 'settings-redraw'; lid: number }
  /** C-6 (#1310, a test aid): the engine's window-API trace (window-api: lines on its stderr) on or off; off at the start */
  | { cmd: 'window-api-trace'; on: boolean }
  /** Issue #1402: the rate the page measured for requestAnimationFrame (24..1000); the engine is paced at it from its next idle (IdleDriver.setHz) */
  | { cmd: 'hz'; hz: number }
  /** Issue #1362: the page's face of the Browser's WebView2 has loaded the document of the navigation that the engine asked for: the engine fires the navigation's events at its next idle */
  | { cmd: 'webview2-loaded' }
  /** Issue #1362: a message the page posted to the engine's WebView2 (window.chrome.webview.postMessage in the face's document) */
  | { cmd: 'webview2-message'; json: string }
  /** #1389: the page's answer to a clipboard-read request (the operating system's text, or null when the page could not read it) */
  | { cmd: 'clipboard-text'; id: number; text: string | null }
  /** Issue #1385: the page is left (pagehide): the engine asks the FL main window to close, so FL closes the files it keeps open (the Worker's onExitRequest) */
  | { cmd: 'exit-request' }
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
  | { t: 'idle'; n: number; now: number; next: number; busyMs: number | null; abs: number; stats: DriverStats; queued: number; caps: { input: boolean; appview: boolean; display?: boolean }; trace?: { n: number; hash: number }[]; memBytes?: number; /** Issue #1402: the number of DwmFlush calls so far (null: a build without the count) */ dwm?: number | null; /** decision 0221: what the engine has asked of its host files so far (a bundle of the current format; the reads all happen before the first idle) */ hostFiles?: HostFileStats; /** decision 0187 (only with ?dedupe=1): rectangles omitted as unchanged / sent, pixels compared */ dedupe?: { omitted: number; sent: number; comparedPixels: number } }
  /** the engine was resumed (it computes until its next idle) */
  | { t: 'resume'; abs: number; why: ResumeWhy }
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
  | { t: 'guest-text'; memBytes: number; best: GuestTextBest | null; near: GuestTextNear[] }
  | { t: 'desk-gl-pixels'; width: number; height: number; data: ArrayBuffer }
  /** decision 0223: what the operating system says that is not a picture: the cursor SetCursor chose (low 32 bits of the handle, standard cursor identifier, module cursor) and the title of the foreground (active) window; sent when it changed */
  | { t: 'osstate'; cursor: number; cursorIdc: number; cursorShared: boolean; cursorMade: boolean; title: string; /** the size of the virtual screen the guest is told (0 from a build that does not report it) */ screenW: number; screenH: number; abs: number }
  /** #1392: the picture of a cursor the guest loaded from one of its modules (the resource's own pixels) or made with CreateIconIndirect (its bitmaps), decision 0260; sent before each osstate that names the handle; `picture` is null when the engine has none (`why`) */
  | { t: 'cursorpic'; handle: number; why: string; picture: CursorPictureMsg | null }
  /** decision 0223: the desktop presenter's counters and the instants (performance.timeOrigin + performance.now() of the Worker) of the presents since the last report, at most every 100 ms; `windows` is the compositor's text (?debug=1 only) */
  /** decision 0272: the first minimized window that has a taskbar button, as the engine says it (fle_g3_taskbar_iconic; 0: none); sent when it changed, never by a build without the export */
  | { t: 'taskbar'; lid: number }
  /** Issue #1362: what the engine's WebView2 model was asked (its log lines, read by the Worker): the controller's rectangle (bounds: left, top, right, bottom, in the guest's pixels relative to the controller's window) and the window's client origin on the guest's screen (null when the window is not known); its visibility; a navigation (its URI, and the resource of the bundle it names: resource is the lazy file's path, or null); a message the engine posts to the page; a script the engine runs (not run by the page) */
  | { t: 'webview2'; op: 'bounds'; left: number; top: number; right: number; bottom: number; originX: number | null; originY: number | null }
  | { t: 'webview2'; op: 'visible'; visible: boolean }
  | { t: 'webview2'; op: 'navigate'; uri: string; resource: string | null }
  | { t: 'webview2'; op: 'post'; json: string }
  | { t: 'webview2'; op: 'exec'; script: string }
  /** Issue #1376: the windows painted over the face (the Browser's window, its bounds): rects is l, t, r, b per window on the guest's screen (the bounds' space). Sent when the picture or the face has changed, and only when the list has */
  | { t: 'webview2'; op: 'front'; rects: number[] }
  /** #1389: the engine's Copy (SetClipboardData, CF_UNICODETEXT): the text the page writes to the operating system's clipboard */
  | { t: 'clipboard-write'; text: string }
  /** #1389: the engine's Paste asks for the operating system's text (GetClipboardData); the page answers with {cmd: 'clipboard-text'} (the same id) */
  | { t: 'clipboard-read'; id: number }
  | { t: 'desk'; stats: { presents: number; rects: number; pixels: number; composeMs: number; putMs: number; whole: number; ringReads: number; ringFull: number; wakes: number; /** C-2: compose requests made by the render Worker's frame loop (its animation frames) / compose requests made while the picture did not move (its wake-up poll, render-worker.ts DESK_WAKE_MS) */ ticks: number; idleTicks: number; /** times the Worker stopped asking for animation frames because the picture did not change (desk2) */ sleeps: number }; at: number[]; /** the inputs the present in this message answers (decision 0223: the Worker made it in the turn that processed them) */ answers?: number[]; nWindows: number };
