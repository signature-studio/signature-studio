// Page side of the engine Worker: spawns it, sends the loaded bundle and hands it the one desktop canvas and the input ring (decision 0223).
import type { Manifest } from '../bundle/manifest.ts';
import type { RawInput } from '../input/events.ts';
import { browserWorkerDiagHost, diagnoseWorkerStart, isPlainWorkerStartEvent } from './worker-diag.ts';
import { parseRequest, type FileDlgAnswer, type FileDlgRequest } from './filedlg.ts';
import type { BundleAssets, DeskHandover, FromWorker, ToWorker } from './protocol.ts';
import { newRingBuffer, RingWriter } from '../input/ring.ts';

export interface ExitInfo { reason: 'finished' | 'stopped' | 'trap'; code: number; message: string; presents: number }

export interface ClientEvents {
  log(level: 'info' | 'warn' | 'error', text: string): void;
  exit(info: ExitInfo): void;
  error(phase: string, message: string): void;
  /** a named instant reported by the Worker; `at` is milliseconds since the page's navigation started (performance.now() of the page) */
  mark?(name: string, at: number, detail?: Record<string, unknown>): void;
  /** lines the engine printed (stdout 'out', stderr 'err') */
  engineLog?(stream: 'out' | 'err', lines: string[]): void;
  /** the engine entered an idle (decision 0136); the whole message of the Worker */
  idle?(m: Extract<FromWorker, { t: 'idle' }>): void;
  resume?(m: Extract<FromWorker, { t: 'resume' }>): void;
  inputDone?(items: Extract<FromWorker, { t: 'input-done' }>['items']): void;
  inputDropped?(seq: number, reason: string): void;
  /** decision 0193: the engine is suspended in a file dialog and asks the person; answer with EngineClient.answerFileDlg */
  fileDlg?(id: number, request: FileDlgRequest): void;
  fileDlgProgress?(id: number, done: number, total: number): void;
  /** decision 0193: a file the engine saved is complete (a copy of its bytes) */
  savedFile?(file: { guestPath: string; name: string; size: number; data: ArrayBuffer }): void;
  /** decision 0223: the page's one canvas for the desktop; it is handed to the Worker (transferControlToOffscreen), after which the page draws nothing on it */
  deskCanvas(): HTMLCanvasElement;
  /** decision 0223: the cursor and the title of the foreground window, when they changed */
  osstate?(m: Extract<FromWorker, { t: 'osstate' }>): void;
  /** decision 0223: the desktop presenter's counters and the instants of its presents (at most every 100 ms) */
  desk?(m: Extract<FromWorker, { t: 'desk' }>): void;
  /** decision 0272: the engine's first minimized window with a taskbar button changed (0: none) */
  taskbar?(m: Extract<FromWorker, { t: 'taskbar' }>): void;
}

export class EngineClient {
  private worker: Worker | null = null;
  private readonly readbacks = new Map<string, (m: FromWorker) => void>();
  private readyResolve: (() => void) | null = null;
  private exitInfo: ExitInfo | null = null;
  private exitWaiters: ((i: ExitInfo) => void)[] = [];
  private isIdle = false;
  private readonly events: ClientEvents;
  private dumpWaiter: ((m: FromWorker) => void) | null = null;
  /** decision 0223 (a test aid): the compositor's whole picture as the window table is now */
  deskDump(peek = false): Promise<{ width: number; height: number; data: ArrayBuffer; windows: string }> {
    return new Promise((res, rej) => {
      this.dumpWaiter = (m) => { if (m.t === 'desk-dump') res({ width: m.width, height: m.height, data: m.data, windows: m.windows }); else if (m.t === 'desk-dump-error') rej(new Error(m.message)); };
      this.send({ cmd: peek ? 'desk-peek' : 'desk-dump' });
    });
  }
  /** decision 0240 (a test aid, when the desktop is shown with WebGL): the scene drawn again and read back from the GL canvas */
  deskGlPixels(): Promise<{ width: number; height: number; data: ArrayBuffer }> {
    return new Promise((res, rej) => {
      this.dumpWaiter = (m) => { if (m.t === 'desk-gl-pixels') res({ width: m.width, height: m.height, data: m.data }); else if (m.t === 'desk-dump-error') rej(new Error(m.message)); };
      this.send({ cmd: 'desk-gl-read' });
    });
  }
  /** decision 0223: the page's end of the input ring */
  private ring: RingWriter | null = null;
  /** events that did not fit in the ring and went as messages, and wake messages sent */
  ringStats = { viaMessage: 0, wakes: 0 };

  constructor(events: ClientEvents) { this.events = events; }

  private send(m: ToWorker, transfer: Transferable[] = []): void { (this.worker as Worker).postMessage(m, transfer); }

  /** spawns the Worker, sends the bundle and resolves when the engine is loaded */
  async init(manifest: Manifest, assets: BundleAssets): Promise<void> {
    const workerCreatedAt = performance.now();
    // NOTE (decision 0189): keep `new Worker(new URL('./worker.ts', import.meta.url), …)` as ONE expression: Vite recognises the Worker only in this form (a `new URL(…)` kept in a variable is emitted as a plain asset `worker-*.ts`, which the server sends with the wrong MIME type and the Worker does not start)
    const w = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module', name: 'fl-engine' });
    this.worker = w;
    w.onmessage = (ev: MessageEvent<FromWorker>) => this.onMessage(ev.data);
    w.onerror = (ev) => {
      this.events.error('worker', `${ev.message || 'the engine Worker raised an error'} (${ev.filename}:${ev.lineno})`);
      // decision 0189: a plain event (no message) = the main script was not loaded; record what the page can still learn (no retry, nothing hidden)
      if (isPlainWorkerStartEvent(ev)) void diagnoseWorkerStart(ev, browserWorkerDiagHost(), workerCreatedAt).then((d) => { (globalThis as { __flWorkerStartFailure?: unknown }).__flWorkerStartFailure = d; this.events.log('warn', `worker start failure diagnostics: ${JSON.stringify(d)}`); });
    };
    w.onmessageerror = () => this.events.error('worker', 'a message from the engine Worker could not be deserialised');
    const ready = new Promise<void>((res) => { this.readyResolve = res; });
    const transfer: Transferable[] = [assets.seed, assets.glue, assets.dll].filter((b): b is Uint8Array => b !== null).map((b) => b.buffer as ArrayBuffer);
    const off = this.events.deskCanvas().transferControlToOffscreen();
    const sab = newRingBuffer();
    this.ring = new RingWriter(sab);
    const desk: DeskHandover = { canvas: off, ring: sab };
    transfer.push(off);
    const msg: ToWorker = { cmd: 'init', manifest, assets, desk };
    // the packs are handed over, not copied (the DLL pack alone is about 70 MB)
    this.send(msg, transfer);
    await ready;
  }

  start(): void { this.send({ cmd: 'start' }); }
  /** decision 0193: the person's answer to the engine's file dialog `id` (the File objects go to the Worker, which reads them into the engine's memory) */
  answerFileDlg(id: number, answer: FileDlgAnswer): void { this.worker?.postMessage({ cmd: 'filedlg-answer', id, answer } satisfies ToWorker); }
  /** a raw input event for the engine (decision 0141); `tPage` is the instant the DOM event was handled */
  sendInput(seq: number, tPage: number, ev: RawInput): void {
    // decision 0223: with display method D the event goes into the shared ring; a message wakes the Worker only if it is parked (idle, nothing but a message or a timer would run it)
    if (this.ring) {
      if (this.ring.push(seq, tPage, ev)) {
        if (this.ring.parked) { this.ringStats.wakes++; this.worker?.postMessage({ cmd: 'wake' } satisfies ToWorker); }
        return;
      }
      this.ringStats.viaMessage++;
    }
    this.worker?.postMessage({ cmd: 'input', seq, tPage, ev } satisfies ToWorker);
  }
  /**
   * The engine runs synchronously inside its wasm stack, so a running engine cannot be asked to stop: if it is idle (waiting for input) the session is ended,
   * otherwise the Worker is terminated (what the engine had drawn stays on the canvas).
   */
  stop(): void {
    if (this.isIdle) { this.worker?.postMessage({ cmd: 'end-idle' } satisfies ToWorker); return; }
    this.terminate();
    this.finishExit({ reason: 'stopped', code: -1, message: 'the engine Worker was terminated by the page while the engine was running', presents: 0 });
  }
  terminate(): void { this.worker?.terminate(); this.worker = null; }

  whenExited(): Promise<ExitInfo> {
    if (this.exitInfo) return Promise.resolve(this.exitInfo);
    return new Promise((res) => this.exitWaiters.push(res));
  }

  /** SHA-256 of the RGBA bytes the canvas of `key` holds (getImageData). The Worker owns the OffscreenCanvas ('desk' for the desktop canvas of display method D) and answers. */
  async readback(key: string): Promise<{ width: number; height: number; sha256: string }> {
    return new Promise((res, rej) => {
      this.readbacks.set(key, (m) => {
        if (m.t === 'readback') res({ width: m.width, height: m.height, sha256: m.sha256 });
        else if (m.t === 'readback-error') rej(new Error(m.message));
      });
      this.send({ cmd: 'readback', key });
    });
  }

  private finishExit(info: ExitInfo): void {
    if (this.exitInfo) return;
    this.exitInfo = info;
    this.events.exit(info);
    for (const w of this.exitWaiters.splice(0)) w(info);
  }

  private onMessage(m: FromWorker): void {
    switch (m.t) {
      case 'ready': if (this.readyResolve) { this.readyResolve(); this.readyResolve = null; } break;
      case 'mark': this.events.mark?.(m.name, m.abs - performance.timeOrigin, m.detail); break;
      case 'engine-log': this.events.engineLog?.(m.stream, m.lines); break;
      case 'idle': this.isIdle = true; this.events.idle?.(m); break;
      case 'resume': this.isIdle = false; this.events.resume?.(m); break;
      case 'input-done': this.events.inputDone?.(m.items); break;
      case 'input-dropped': this.events.inputDropped?.(m.seq, m.reason); break;
      case 'log': this.events.log(m.level, m.text); break;
      case 'exit': this.finishExit({ reason: m.reason, code: m.code, message: m.message, presents: m.presents }); break;
      case 'error': this.events.error(m.phase, m.message); break;
      case 'readback': case 'readback-error': { const f = this.readbacks.get(m.key); this.readbacks.delete(m.key); f?.(m); break; }
      case 'filedlg-request': {
        const req = parseRequest(m.request);
        if (req === null) { this.events.log('warn', `file dialog ${m.id}: the request has a shape the page does not know; cancelled`); this.answerFileDlg(m.id, { ok: false, reason: 'unknown request' }); break; }
        if (this.events.fileDlg) this.events.fileDlg(m.id, req); else this.answerFileDlg(m.id, { ok: false, reason: 'no dialog in this page' });
        break;
      }
      case 'filedlg-progress': this.events.fileDlgProgress?.(m.id, m.done, m.total); break;
      case 'saved-file': this.events.savedFile?.({ guestPath: m.guestPath, name: m.name, size: m.size, data: m.data }); break;
      case 'pong': break;
      case 'desk-dump': case 'desk-gl-pixels': case 'desk-dump-error': { const f = this.dumpWaiter; this.dumpWaiter = null; f?.(m); break; }
      case 'osstate': this.events.osstate?.(m); break;
      case 'desk': this.events.desk?.(m); break;
      case 'taskbar': this.events.taskbar?.(m); break;
    }
  }
}
