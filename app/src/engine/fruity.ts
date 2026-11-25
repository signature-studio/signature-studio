// Adapter for the engine: the lifted FL Studio engine built with emscripten (lift/harness/two/build_nmod_wasm.sh FRUITY=1 THREADS=1 JSPI=1 SJLJ=wasm, linked for the web by
// lift/harness/web/fruity/link-web.sh). Ported (copied and typed) from lift/harness/web/final0129/fle-final-worker.js (decision 0134) and lift/harness/web/fruity/fle-fruity-worker.js (decision 0128);
// the harness is not imported. What it does, in the order the engine needs it:
//   1. imports the emscripten glue (an ES module: -sMODULARIZE=1 -sEXPORT_ES6=1) from the bytes the loader verified;
//   2. builds the Module object: locateFile, argv, print/printErr, onExit/onAbort, the engine's hooks (fleIdle, fleThreadFailed, fleProgress) and a preRun that makes the files of the
//      seed pack and the DLL pack readable under `root` and sets the environment (the engine reads its files with fopen / open / pread at the same paths as on the native side). The glue was linked with
//      -sFILESYSTEM=0: the packs are answered by hostfiles.ts at the libc system calls, from the pack bytes;
//   3. instantiateWasm: instantiates the module the page compiled (WebAssembly.compileStreaming) and finds the memory;
//   4. run(): Module(opts). It settles when the engine exits (onExit), when Module() settles, or when it throws; an engine that is idle stays suspended in fleIdle and run() stays pending.
// Display: the runtime composes the windows into one picture (fle_desk_compose) and the Worker shows it (worker.ts, desk.ts).

import type { FruityOptions, Manifest } from '../bundle/manifest.ts';
import { type AppView, readAppView } from './appview.ts';
import type { DeskApi } from './desk.ts';
import { HostFiles, type HostFileStats, type HostPack, installHostFiles, SUPPORTED_EMSCRIPTEN, unsupportedImports } from './hostfiles.ts';
import type { EngineInput } from './idle-driver.ts';
import type { LazyFileSource } from './lazy-file.ts';
import { installXmod, XmodLoader } from './xmod.ts';

/** the engine-side Module object (what the glue is given); the glue adds FS, ENV and the idle functions to it */
/** decision 0234: TextDecoder, ImageData and Blob refuse a view of a SharedArrayBuffer (Chrome 156, Firefox 158); a view of a shared memory is copied first (nothing happens for an ordinary memory) */
export function unshared<T extends Uint8Array>(v: T): T { return typeof SharedArrayBuffer !== 'undefined' && v.buffer instanceof SharedArrayBuffer ? (v.slice() as T) : v; }

export interface FruityModuleOpts {
  locateFile: (p: string) => string;
  arguments: string[];
  print: (line: string) => void;
  printErr: (line: string) => void;
  onExit: (code: number) => void;
  onAbort: (what: unknown) => void;
  onRuntimeInitialized: () => void;
  fleProgress: (entries: number) => void;
  /** the engine is waiting for input (lift/harness/web/fruity/fle_idle_lib.js): its idle number, the virtual clock and its next deadline in ticks (10,000 ticks = 1 ms; next 0 = no deadline) */
  fleIdle: (n: number, nowTicks: number, nextTicks: number) => void;
  fleThreadFailed: (idx: number, e: unknown, msg: string) => void;
  /** decision 0280 (lift/harness/web/fruity/fle_idle_lib.js): the scheduler reads the page's input ring itself; the Worker stages the records into `ptr` (8 words each, at most `max`) and returns the count; fleInputDone(ptr, n) is told what the runtime did with them */
  fleInputPoll?: (ptr: number, max: number) => number;
  fleInputDone?: (ptr: number, n: number) => void;
  /** Issue #1383 (lift/harness/web/fruity/fle_idle_lib.js): the saved records of the registry's HKCU tree, replayed before the guest runs; and the change of one record, which the page keeps */
  fleRegSaved?: () => Uint8Array | null;
  fleRegPut?: (record: Uint8Array) => void;
  /** Issue #1342 (lift/harness/two/xmod_glue.c, lift/harness/web/fruity/fle_idle_lib.js): the engine asks for a real module's separate wasm module (xmod.ts); 1 when it is instantiated, 0 when the DLL has none */
  fleXmodLoad?: (ptr: number) => number;
  preRun: Array<() => void>;
  instantiateWasm: (imports: WebAssembly.Imports, done: (instance: WebAssembly.Instance, module: WebAssembly.Module) => void) => object;
  ENV?: Record<string, string>;
  fleIdleDeliver?: (code: number) => boolean;
  fleIdleWaiting?: () => boolean;
  /** decision 0193 (lift/harness/web/filedlg/fle_filedlg_lib.js): the engine is suspended in a file dialog and asks the host; the host answers through fleFileDlgAnswer (set by the library) */
  fleFileDlg?: (id: number, request: unknown) => void;
  fleFileDlgAnswer?: (id: number, answer: unknown) => boolean;
  fleFileDlgProgress?: (id: number, done: number, total: number) => void;
  /** #1389 (lift/harness/web/clip/fle_clip_lib.js): the engine's Copy text goes to the host; the engine asks the host for the operating system's clipboard text (fleClipRequest), the host answers through fleClipAnswer (set by the library) */
  fleClipWrite?: (text: string) => void;
  fleClipRequest?: (id: number) => void;
  fleClipAnswer?: (id: number, text: string | null) => void;
}
export type CreateModule = (opts: FruityModuleOpts) => Promise<unknown>;
export type GlueImporter = (glue: Uint8Array) => Promise<CreateModule>;

export interface FruityHost {
  /** a named instant of the start-up (glue_loaded, wasm_instantiated, files_ready, main_start, exit ...) */
  mark(name: string, detail?: Record<string, unknown>): void;
  stdout(line: string): void;
  stderr(line: string): void;
  log(level: 'info' | 'warn' | 'error', text: string): void;
  idle(n: number, nowTicks: number, nextTicks: number): void;
  progress?(entries: number): void;
  /** decision 0193: absent = the engine's file dialogs are cancelled at once (nobody to ask) */
  fileDlg?(id: number, request: unknown): void;
  fileDlgProgress?(id: number, done: number, total: number): void;
  /** #1389: the text of the engine's Copy; the operating system's clipboard text is asked by clipRead(id) and answered with answerClipRead */
  clipWrite?(text: string): void;
  clipRead?(id: number): void;
  /** decision 0280: see FruityModuleOpts.fleInputPoll / fleInputDone; absent = the input is taken at the idle only */
  inputPoll?(ptr: number, max: number): number;
  inputDone?(ptr: number, n: number): void;
  /** Issue #1383: the page's keeping of the registry's HKCU tree (regstore.ts): the saved records (null: none), and the change of one record; absent = nothing is kept */
  regSaved?(): Uint8Array | null;
  regPut?(record: Uint8Array): void;
}

export type FruityExit =
  | { kind: 'exit'; code: number }
  | { kind: 'returned'; code: number | null }
  | { kind: 'trap'; message: string };

export interface FruityAssets {
  module: WebAssembly.Module;
  glue: Uint8Array;
  seed: HostPack;
  dll: HostPack;
  /** Issue #1314: the lazy files of the manifest (their bytes may still be on their way; lazy-file.ts). Each is a host file under the root for the whole run. */
  lazy?: LazyFileSource[];
}

/** Imports the glue from a Blob URL made of exactly the bytes whose SHA-256 the manifest named (the CSP allows blob: for scripts for this). */
export const importGlueFromBlob: GlueImporter = async (glue) => {
  const url = URL.createObjectURL(new Blob([glue as BlobPart], { type: 'text/javascript' }));
  try {
    const mod = (await import(/* @vite-ignore */ url)) as { default?: unknown };
    if (typeof mod.default !== 'function') throw new Error('the emscripten glue has no default export function (was it linked with -sMODULARIZE=1 -sEXPORT_ES6=1?)');
    return mod.default as CreateModule;
  } finally {
    URL.revokeObjectURL(url);
  }
};

export class FruityEngine {
  readonly options: FruityOptions;
  memory: WebAssembly.Memory | null = null;
  instance: WebAssembly.Instance | null = null;
  readonly opts: FruityModuleOpts;
  readonly hostFiles: HostFiles;
  private readonly createModule: CreateModule;
  private readonly host: FruityHost;
  private exitCode: number | null = null;
  private idleSince: number | null = null;

  private constructor(manifest: Manifest, assets: FruityAssets, createModule: CreateModule, host: FruityHost) {
    this.options = manifest.engine.options as unknown as FruityOptions;
    this.createModule = createModule;
    this.host = host;
    const root = this.options.root;
    const hostFiles = new HostFiles(() => this.memory);
    this.hostFiles = hostFiles;
    // Issue #1342: the separate wasm modules of the real modules (xmod.ts); the engine reaches it through fleXmodLoad and the imports of its f_<va> functions
    const xmod = new XmodLoader(root, assets.lazy ?? [], () => (this.instance?.exports ?? null) as Record<string, unknown> | null, () => this.memory);
    const opts: FruityModuleOpts = {
      locateFile: (p) => p,
      arguments: [...manifest.engine.args],
      print: (line) => host.stdout(line),
      printErr: (line) => host.stderr(line),
      // the web glue calls onExit from _proc_exit; run() settles from it as well (an exit on a guest thread other than the first leaves Module() unsettled: decision 0124)
      onExit: (code) => { this.exitCode = code; host.mark('exit', { code }); },
      onAbort: (what) => host.log('error', `the engine aborted: ${String(what)}`),
      onRuntimeInitialized: () => host.mark('main_start'),
      fleProgress: (n) => host.progress?.(n),
      fleIdle: (n, now, next) => { this.idleSince = performance.now(); if (n === 1) { host.mark('idle', { n, now, next, hostFiles: { ...hostFiles.stats } }); hostFiles.release(); host.mark('host_files_released', { ...hostFiles.stats }); } host.idle(n, now, next); },
      fleThreadFailed: (idx, _e, msg) => host.stderr(`[js] ${msg} (guest thread ${idx})`),
      fleXmodLoad: (ptr: number) => xmod.loadAt(ptr),
      ...(host.inputPoll ? { fleInputPoll: (ptr: number, max: number) => host.inputPoll?.(ptr, max) ?? 0, fleInputDone: (ptr: number, n: number) => host.inputDone?.(ptr, n) } : {}),
      ...(host.regSaved ? { fleRegSaved: () => host.regSaved?.() ?? null } : {}),
      ...(host.regPut ? { fleRegPut: (record: Uint8Array) => host.regPut?.(record) } : {}),
      ...(host.fileDlg ? { fleFileDlg: (id: number, request: unknown) => host.fileDlg?.(id, request), fleFileDlgProgress: (id: number, done: number, total: number) => host.fileDlgProgress?.(id, done, total) } : {}),
      ...(host.clipWrite ? { fleClipWrite: (text: string) => host.clipWrite?.(text) } : {}),
      ...(host.clipRead ? { fleClipRequest: (id: number) => host.clipRead?.(id) } : {}),
      preRun: [() => {
        if (!opts.ENV) throw new Error('the emscripten glue exposes no ENV (link with -sEXPORTED_RUNTIME_METHODS=ENV,...)');
        const t0 = performance.now();
        // Issue #1357: the packs are persistent (hostfiles.ts release): the guest reads them after the first idle too (a seed file at its first read, a real module at its LoadLibrary)
        const dll = hostFiles.mountPack(root, assets.dll, true);
        const seed = hostFiles.mountPack(root, assets.seed, true);
        // Issue #1314: a lazy file is persistent (hostfiles.ts addPersistent), so the release at the first idle does not take it away
        for (const f of assets.lazy ?? []) hostFiles.addPersistent(`${root}/${f.path}`, f);
        for (const [k, v] of Object.entries(manifest.engine.env)) opts.ENV[k] = v;
        if (this.traceOn) this.windowApiTrace(true);   // #1347: the trace asked before the start is on from the first instruction
        host.mark('files_ready', { how: 'packs-v1', dllFiles: dll.files, dllBytes: dll.bytes, seedFiles: seed.files, seedBytes: seed.bytes, lazyFiles: assets.lazy?.length ?? 0, ms: +(performance.now() - t0).toFixed(1) });
      }],
      instantiateWasm: (imports, done) => {
        void (async () => {
          installHostFiles(imports, hostFiles);
          // Issue #1342: a real module's code may be a separate wasm module (xmod.ts): the engine's f_<va> imports for it are supplied here
          installXmod(imports, assets.module, xmod);
          const instance = await WebAssembly.instantiate(assets.module, imports);
          const ex = instance.exports as Record<string, unknown>;
          const mem = ex.memory ?? (imports as { env?: { memory?: unknown } }).env?.memory;   // decision 0234 (a MT2_SHARED build imports its shared memory and exports none)
          if (!(mem instanceof WebAssembly.Memory)) throw new Error('the engine wasm exports no memory');
          this.instance = instance;
          this.memory = mem;
          host.mark('wasm_instantiated', { memoryBytes: mem.buffer.byteLength });
          done(instance, assets.module);
        })().catch((e: unknown) => this.fail(e));
        return {};
      },
    };
    this.opts = opts;
  }

  private failure: ((e: unknown) => void) | null = null;
  /** #1347: the window-API trace as last asked (windowApiTrace); the start applies it */
  private traceOn = false;
  private fail(e: unknown): void { if (this.failure) this.failure(e); else this.host.log('error', String((e as Error)?.stack ?? e)); }

  /** imports the glue and builds the Module object; nothing of the engine runs yet */
  static async create(manifest: Manifest, assets: FruityAssets, host: FruityHost, importer: GlueImporter = importGlueFromBlob): Promise<FruityEngine> {
    const o = manifest.engine.options as unknown as FruityOptions;
    {
      const abi = o.hostFilesAbi;
      if (!SUPPORTED_EMSCRIPTEN.some((v) => abi === `emscripten-${v}`)) throw new Error(`This bundle's engine was linked by ${abi || 'an unknown Emscripten'}; the page's file calls (src/engine/hostfiles.ts) are written for ${SUPPORTED_EMSCRIPTEN.map((v) => `emscripten-${v}`).join(', ')}. Rebuild the bundle with that Emscripten or update hostfiles.ts.`);
      const bad = unsupportedImports(assets.module);
      if (bad.length) throw new Error(`The engine wasm imports ${bad.join(', ')} from the system-call layer; src/engine/hostfiles.ts neither implements nor knows these to be harmless (it implements the read-only file calls only).`);
    }
    const createModule = await importer(assets.glue);
    host.mark('glue_loaded');
    return new FruityEngine(manifest, assets, createModule, host);
  }

  /** what the engine has asked of the host files so far */
  get hostFileStats(): HostFileStats { return { ...this.hostFiles.stats }; }

  get canSuspend(): boolean {
    const w = WebAssembly as unknown as { Suspending?: unknown; promising?: unknown };
    return typeof w.Suspending === 'function' && typeof w.promising === 'function';
  }

  /** the engine's exports for input (lift/harness/fruity_idle.c) as a typed object, or null when the wasm has none (a bundle built before decision 0136) */
  get inputApi(): EngineInput | null {
    const ex = this.instance?.exports as Record<string, unknown> | undefined;
    if (!ex || typeof ex.fle_g3_mouse !== 'function' || typeof ex.fle_g3_key !== 'function' || typeof ex.fle_g3_clock_to !== 'function') return null;
    const mouse = ex.fle_g3_mouse as (a: number, b: number, c: number, d: number) => number;
    const key = ex.fle_g3_key as (a: number, b: number, c: number, d: number) => number;
    const clk = ex.fle_g3_clock_to as (t: number) => void;
    const display = typeof ex.fle_g3_display === 'function' ? (ex.fle_g3_display as (w: number, h: number) => number) : null;
    const syscmd = typeof ex.fle_g3_taskbar_iconic === 'function' && typeof ex.fle_g3_taskbar_syscommand === 'function' ? (ex.fle_g3_taskbar_syscommand as (lid: number, cmd: number) => number) : null;
    return { mouse: (k, x, y, d) => mouse(k, x, y, d), key: (k, vk, sc, e) => key(k, vk, sc, e), clockTo: (t) => clk(t), ...(display ? { display: (w: number, h: number) => display(w, h) } : {}), ...(syscmd ? { syscommand: (lid: number, cmd: number) => syscmd(lid, cmd) } : {}) };
  }

  /**
   * Decision 0272 (lift/harness/fruity_idle.c, decision 0267): the number of the first window that is minimized and has a taskbar button (0: none), or null when the wasm lacks the taskbar entry
   * (a bundle built before decision 0267: the page then shows nothing). Both exports are needed: a button that cannot act is not shown.
   */
  taskbarIconic(): number | null {
    const ex = this.instance?.exports as Record<string, unknown> | undefined;
    if (!ex || typeof ex.fle_g3_taskbar_iconic !== 'function' || typeof ex.fle_g3_taskbar_syscommand !== 'function') return null;
    return (ex.fle_g3_taskbar_iconic as () => number)() >>> 0;
  }

  /** C-6 (#1310, a test aid): the engine's window-API trace on or off (fle_g3_window_api_trace, lift/harness/fruity_idle.c); false when the wasm has no such export */
  windowApiTrace(on: boolean): boolean {
    this.traceOn = on;   // kept for the start (preRun): a trace asked before the engine runs is on from its first instruction
    const ex = this.instance?.exports as Record<string, unknown> | undefined;
    if (!ex || typeof ex.fle_g3_window_api_trace !== 'function') return false;
    (ex.fle_g3_window_api_trace as (on: number) => void)(on ? 1 : 0);
    return true;
  }

  /** decision 0212: the wasm has the export that tells the guest the display changed (a bundle built before screen1 has none) */
  get hasDisplay(): boolean { return typeof (this.instance?.exports as Record<string, unknown> | undefined)?.fle_g3_display === 'function'; }

  /** #1384 (a test aid): the window `lid` and its visible descendants are drawn again at the engine's next message loop (fle_g3_redraw_tree); false when the wasm has no such export */
  redrawTree(lid: number): boolean {
    const f = (this.instance?.exports as Record<string, unknown> | undefined)?.fle_g3_redraw_tree;
    if (typeof f !== 'function') return false;
    (f as (lid: number) => void)(lid);
    return true;
  }

  /** Issue #1362: the page's face has loaded the document of a WebView2 navigation; the engine fires its events at its next idle (fle_g3_wv2_loaded, lift/harness/fruity_idle.c). False when the wasm has no such export. */
  wv2Loaded(): boolean {
    const f = (this.instance?.exports as Record<string, unknown> | undefined)?.fle_g3_wv2_loaded;
    if (typeof f !== 'function') return false;
    f();
    return true;
  }

  /** decision 0193: hands the person's answer to the suspended engine; false when the engine is not waiting for this dialog */
  answerFileDlg(id: number, answer: unknown): boolean { return this.opts.fleFileDlgAnswer ? this.opts.fleFileDlgAnswer(id, answer) : false; }

  /** #1389: hands the operating system's clipboard text (or null: none) to the suspended engine; the library ignores an id it does not wait for */
  answerClipRead(id: number, text: string | null): void { this.opts.fleClipAnswer?.(id, text); }

  /** decision 0193: the next Save target the engine has finished writing (fle_filedlg_saved_poll of lift/runtime/fle_filedlg.c), or null; the bytes are copied */
  pollSaved(): { guestPath: string; size: number; data: Uint8Array } | null {
    const ex = this.instance?.exports as Record<string, unknown> | undefined;
    if (!ex || typeof ex.fle_filedlg_saved_poll !== 'function' || !this.memory) return null;
    const ptr = (ex.fle_filedlg_saved_poll as () => number)();
    if (ptr === 0) return null;
    const heap = new Uint8Array(this.memory.buffer);
    let end = ptr;
    while (heap[end] !== 0) end++;
    const line = new TextDecoder().decode(unshared(heap.subarray(ptr, end)));
    const m = /^(\d+)\t(\d+)\t(.+)$/.exec(line);
    if (!m) return null;
    const size = Number(m[1]), at = Number(m[2]);
    if (at + size > heap.length) return null;
    return { guestPath: m[3] as string, size, data: heap.slice(at, at + size) };
  }

  /** decision 0223: the desktop compositor's exports (lift/runtime/fle_desk.c), or null when the wasm has none */
  get deskApi(): DeskApi | null {
    const ex = this.instance?.exports as Record<string, unknown> | undefined;
    if (!ex || typeof ex.fle_desk_compose !== 'function' || typeof ex.fle_desk_resync !== 'function' || !this.memory) return null;
    const compose = ex.fle_desk_compose as () => number, resync = ex.fle_desk_resync as () => void;
    const describe = typeof ex.fle_desk_describe === 'function' ? (ex.fle_desk_describe as () => number) : null;
    const scene = typeof ex.fle_desk_scene === 'function' ? (ex.fle_desk_scene as () => number) : null;
    const sceneMode = typeof ex.fle_desk_scene_mode === 'function' ? (ex.fle_desk_scene_mode as (on: number) => void) : null;
    const front = typeof ex.fle_webview2_front === 'function' ? (ex.fle_webview2_front as () => number) : null;
    const mem = this.memory;
    const decoder = new TextDecoder('latin1');
    return {
      compose: () => compose(),
      resync: () => resync(),
      ...(scene && sceneMode ? { scene: () => scene(), sceneMode: (on: number) => sceneMode(on) } : {}),
      ...(front ? { front: () => front() } : {}),
      describe: () => {
        if (!describe) return '';
        const p = describe(), heap = new Uint8Array(mem.buffer);
        let e = p;
        while (heap[e] !== 0) e++;
        return decoder.decode(unshared(heap.subarray(p, e)));
      },
    };
  }

  get hasAppView(): boolean { return typeof (this.instance?.exports as Record<string, unknown> | undefined)?.fle_av_poll === 'function'; }

  /** what fle_av_poll() (lift/runtime/fle_appview.c, decision 0141) says now; null when the wasm has no such export or the structure does not match */
  readAppView(sinceSeq?: number): AppView | null {
    const ex = this.instance?.exports as Record<string, unknown> | undefined;
    if (!ex || typeof ex.fle_av_poll !== 'function' || !this.memory) return null;
    const ptr = (ex.fle_av_poll as () => number)();
    return readAppView(this.memory.buffer, ptr, sinceSeq);
  }

  get idle(): boolean { return this.opts.fleIdleWaiting ? this.opts.fleIdleWaiting() : false; }
  get idleStartedAt(): number | null { return this.idleSince; }

  /** resumes an idle engine: < 0 ends the session, 0 = input was queued (or only the clock moved), > 0 = move the virtual clock to that many ticks (decision 0136) */
  deliver(code: number): boolean { return this.opts.fleIdleDeliver ? this.opts.fleIdleDeliver(code) : false; }

  /** ends an idle engine (fle_idle_wait returns < 0: the session ends; decision 0136 - decision 0128's 0 now means "events queued") */
  endIdle(): boolean { return this.deliver(-1); }

  run(): Promise<FruityExit> {
    return new Promise<FruityExit>((resolve) => {
      let settled = false;
      const finish = (x: FruityExit): void => { if (!settled) { settled = true; this.hostFiles.close(); resolve(x); } };
      this.failure = (e) => finish({ kind: 'trap', message: String((e as Error)?.stack ?? e) });
      const prevExit = this.opts.onExit;
      this.opts.onExit = (code) => { prevExit(code); setTimeout(() => finish({ kind: 'exit', code }), 0); };
      this.createModule(this.opts).then(
        () => finish(this.exitCode === null ? { kind: 'returned', code: null } : { kind: 'exit', code: this.exitCode }),
        (e: unknown) => {
          // exit() inside the first thread unwinds main with ExitStatus (onExit has run); anything else is a trap
          if ((e as { name?: string } | null)?.name === 'ExitStatus') finish({ kind: 'exit', code: this.exitCode ?? Number((e as { status?: number }).status ?? -1) });
          else finish({ kind: 'trap', message: String((e as Error)?.stack ?? e) });
        },
      );
    });
  }
}

/** what a thrown error from the engine's start-up means for the user: out of memory is told apart from the rest */
export function classifyEngineError(message: string): { what: string; kind: 'oom' | 'jspi' | 'compile' | 'other' } {
  if (/out of memory|could not allocate memory|Array buffer allocation failed|WebAssembly\.Memory\(\)|RangeError: Invalid typed array length|memory access out of bounds.*grow|Cannot enlarge memory/i.test(message)) {
    return { kind: 'oom', what: `the browser ran out of memory while starting the engine (the engine needs about 1 GB of WebAssembly memory plus the compiled module): ${message.split('\n')[0]}` };
  }
  if (/Suspending|promising|JSPI|suspendable/i.test(message)) return { kind: 'jspi', what: `WebAssembly JSPI failed: ${message.split('\n')[0]}` };
  if (/CompileError|LinkError/.test(message)) return { kind: 'compile', what: `the engine wasm could not be instantiated: ${message.split('\n')[0]}` };
  return { kind: 'other', what: message.split('\n')[0] ?? message };
}
