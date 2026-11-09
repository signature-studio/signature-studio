// Host-side handle of one plugin Worker (decision 0173, S1). Environment-neutral: the caller supplies the Worker (a browser `Worker`, a node `worker_threads` Worker, or anything with the same
// four methods) through a WorkerHandle factory, and the compiled WebAssembly.Module (compile once, share it with every instance: postMessage(Module) is a reference to the same compiled code).
import type { InitMessage, WorkerOut } from './plugin-worker.ts';
import { Slot, STATE, W, beginBlock, readOutput, requestQuit, waitResponse, waitResponseAsync } from './protocol.ts';

export interface WorkerHandle {
  post(message: unknown): void;
  onMessage(cb: (m: WorkerOut) => void): void;
  onError(cb: (e: Error) => void): void;
  terminate(): void;
}

export interface PluginConfig {
  module: WebAssembly.Module;
  /** file name (as the harness opens it) -> bytes: the plugin image and the VFS seed list */
  files: Record<string, Uint8Array>;
  pluginPath: string;
  frames?: number;
  sampleRate?: number;
  maxBlocks?: number;
  /** synthetic blocks run through process() before the instance reports READY (warm-up of lazily compiled code and first-touched memory) */
  preroll?: number;
  /** 0 = keep the state, 1 = cycle setProcessing, 2 = cycle setActive + setProcessing after the pre-roll */
  prerollReset?: 0 | 1 | 2;
  extraArgs?: string[];
  env?: string[];
}

export interface PluginResult {
  rc: number;
  served: number;
  result: Record<string, string>;
  lines: string[];
  missing: string[];
}

let nextId = 1;

export class PluginInstance {
  readonly slot: Slot;
  readonly frames: number;
  /** from the Worker: init time up to READY (DllMain … setActive … pre-roll) in the Worker's clock */
  initMs = 0;
  prerollLine: string | null = null;
  private readonly worker: WorkerHandle;
  private doneP: Promise<PluginResult>;

  private constructor(worker: WorkerHandle, slot: Slot, frames: number, doneP: Promise<PluginResult>) {
    this.worker = worker;
    this.slot = slot;
    this.frames = frames;
    this.doneP = doneP;
  }

  /** Spawn the Worker, hand it the module and files, and resolve when the plugin is initialised, pre-rolled and parked waiting for the first block. */
  static start(makeWorker: () => WorkerHandle, cfg: PluginConfig): Promise<PluginInstance> {
    const frames = cfg.frames ?? 4096;
    const slot = Slot.create(frames);
    const worker = makeWorker();
    const id = nextId++;
    return new Promise<PluginInstance>((resolve, reject) => {
      let inst: PluginInstance | null = null;
      let resolveDone: (r: PluginResult) => void = () => undefined;
      let rejectDone: (e: Error) => void = () => undefined;
      const doneP = new Promise<PluginResult>((res, rej) => {
        resolveDone = res;
        rejectDone = rej;
      });
      doneP.catch(() => undefined);
      worker.onError((e) => {
        reject(e);
        rejectDone(e);
      });
      worker.onMessage((m) => {
        if (m.id !== id) return;
        if (m.type === 'ready') {
          inst = new PluginInstance(worker, slot, frames, doneP);
          inst.initMs = m.initMs;
          inst.prerollLine = m.prerollLine;
          resolve(inst);
        } else if (m.type === 'done') {
          resolveDone({ rc: m.rc, served: m.served, result: m.result, lines: m.lines, missing: m.missing });
        } else {
          const e = new Error('plugin worker: ' + m.error);
          if (!inst) reject(e);
          rejectDone(e);
        }
      });
      const init: InitMessage = {
        type: 'init',
        id,
        module: cfg.module,
        files: cfg.files,
        pluginPath: cfg.pluginPath,
        sab: slot.sab,
        frames,
        sampleRate: cfg.sampleRate ?? 44100,
        maxBlocks: cfg.maxBlocks ?? 65536,
        preroll: cfg.preroll ?? 0,
        prerollReset: cfg.prerollReset ?? 0,
        extraArgs: cfg.extraArgs ?? [],
        env: cfg.env ?? ['FLE_VFS_SEED=/seed.tsv'],
      };
      worker.post(init);
    });
  }

  /** Publish one block; returns the request number for waitSync / waitAsync. Fan-out: call begin on every instance, then wait on each. */
  begin(inL: Float32Array, inR: Float32Array): number {
    return beginBlock(this.slot, inL, inR, this.frames);
  }
  /** Blocking wait (Worker / node). */
  waitSync(seq: number, timeoutMs = Infinity): boolean {
    return waitResponse(this.slot, seq, timeoutMs);
  }
  /** Non-blocking wait (any thread). */
  waitAsync(seq: number, timeoutMs = Infinity): Promise<boolean> {
    return waitResponseAsync(this.slot, seq, timeoutMs);
  }
  read(outL: Float32Array, outR: Float32Array): void {
    readOutput(this.slot, outL, outR, this.frames);
  }
  /** Time the plugin Worker spent between taking the request and answering it, last block, in microseconds. */
  get lastProcessUs(): number {
    return Atomics.load(this.slot.ctrl, W.PROC_US);
  }
  get state(): number {
    return this.slot.state;
  }
  get failed(): boolean {
    return this.slot.state === STATE.FAILED;
  }

  /** process() one block synchronously (Worker / node). Returns the round trip in ms. */
  processSync(inL: Float32Array, inR: Float32Array, outL: Float32Array, outR: Float32Array): number {
    const t0 = performance.now();
    const seq = this.begin(inL, inR);
    if (!this.waitSync(seq)) throw new Error('plugin did not answer');
    this.read(outL, outR);
    return performance.now() - t0;
  }

  /** Leave the process loop, let the plugin tear down, and return the harness' RESULT lines (out_hash …). */
  async stop(): Promise<PluginResult> {
    requestQuit(this.slot);
    const r = await this.doneP;
    this.worker.terminate();
    return r;
  }

  terminate(): void {
    this.worker.terminate();
  }
}
