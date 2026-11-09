// The plugin Worker (decision 0173, S1): one lifted VST3 instance per Worker. Browser Worker (type: 'module') and node worker_threads run the same file.
//
//   host                                         this Worker
//   ----                                         -----------
//   post {type:'init', module, files, sab,…} ->  instantiate the shared WebAssembly.Module, run the harness main(): DllMain … setActive (init), then --preroll N synthetic blocks
//                                                (+ optional state reset), then park in mt1_ext_wait
//                                       <- {type:'ready', initMs, …}                (STATE = READY)
//   beginBlock()  (slot: input, REQ++)        ->  mt1_ext_wait wakes, copies the input into the plugin's buffers, process(), mt1_ext_done copies the output to the slot, RESP = REQ
//   requestQuit() (CMD = QUIT, REQ++)         ->  mt1_ext_wait returns 0, the harness prints its RESULT lines, tears the plugin down
//                                       <- {type:'done', result, lines}
//
// The block size is the harness' --frames (fixed at init). Nothing here knows which plugin it is: the wasm is whatever lift/harness/web/mt1/plugin-worker/build.sh (or the same recipe for another VST3) produced.
import { createHost } from './ott-host.ts';
import { CMD, STATE, Slot, W, respond, waitRequest } from './protocol.ts';

export interface InitMessage {
  type: 'init';
  id: number;
  module: WebAssembly.Module;
  files: Record<string, Uint8Array>;
  pluginPath: string;
  sab: SharedArrayBuffer;
  frames: number;
  sampleRate: number;
  maxBlocks: number;
  preroll: number;
  prerollReset: number;
  extraArgs: string[];
  env: string[];
}

export type WorkerOut =
  | { type: 'ready'; id: number; initMs: number; prerollLine: string | null }
  | { type: 'done'; id: number; rc: number; served: number; result: Record<string, string>; lines: string[]; missing: string[] }
  | { type: 'error'; id: number; error: string };

interface Port {
  post(m: WorkerOut): void;
  onMessage(cb: (m: unknown) => void): void;
}

async function openPort(): Promise<Port> {
  if (typeof process !== 'undefined' && process.versions?.node) {
    const wt = await import('node:worker_threads');
    const p = wt.parentPort;
    if (!p) throw new Error('plugin-worker must run as a worker_threads Worker');
    return { post: (m) => p.postMessage(m), onMessage: (cb) => p.on('message', cb) };
  }
  const g = globalThis as unknown as { postMessage(m: unknown): void; onmessage: ((e: { data: unknown }) => void) | null };
  return {
    post: (m) => g.postMessage(m),
    onMessage: (cb) => {
      g.onmessage = (e) => cb(e.data);
    },
  };
}

function parseResult(lines: string[]): Record<string, string> {
  const out: Record<string, string> = {};
  for (const l of lines) {
    if (!l.startsWith('RESULT ')) continue;
    for (const kv of l.slice(7).split(/\s+/)) {
      const i = kv.indexOf('=');
      if (i > 0) out[kv.slice(0, i)] = kv.slice(i + 1);
    }
  }
  return out;
}

async function run(port: Port, m: InitMessage): Promise<void> {
  const slot = Slot.attach(m.sab);
  slot.setState(STATE.LOADING);
  const t0 = performance.now();
  const lines: string[] = [];
  let last = Atomics.load(slot.ctrl, W.REQ);
  let readySent = false;
  let served = 0;
  let tBlock = 0;
  const host = createHost({
    files: m.files,
    args: [m.pluginPath, '--ext', '--blocks', String(m.maxBlocks), '--frames', String(m.frames), '--rate', String(m.sampleRate), '--preroll', String(m.preroll), '--preroll-reset', String(m.prerollReset), ...m.extraArgs],
    env: m.env,
    print: (l) => {
      if (l.startsWith('RESULT ') || l.startsWith('vst: ')) lines.push(l.length > 400 ? l.slice(0, 400) : l);
    },
    ext: {
      wait(mem, inL, inR) {
        if (!readySent) {
          readySent = true;
          slot.setState(STATE.READY);
          const pr = lines.find((l) => l.startsWith('RESULT preroll')) ?? null;
          port.post({ type: 'ready', id: m.id, initMs: performance.now() - t0, prerollLine: pr });
        }
        const seq = waitRequest(slot, last);
        last = seq;
        tBlock = performance.now();
        if (Atomics.load(slot.ctrl, W.CMD) === CMD.QUIT) {
          slot.setState(STATE.STOPPED);
          respond(slot, seq, 0);
          return 0;
        }
        const n = Atomics.load(slot.ctrl, W.FRAMES);
        if (n !== m.frames) {
          // the harness processes a fixed block size: refuse anything else loudly instead of computing on a partial block
          Atomics.store(slot.ctrl, W.ERR, 1);
          slot.setState(STATE.FAILED);
          respond(slot, seq, 0);
          return 0;
        }
        slot.setState(STATE.RUNNING);
        new Float32Array(mem.buffer, inL, n).set(slot.inL.subarray(0, n));
        new Float32Array(mem.buffer, inR, n).set(slot.inR.subarray(0, n));
        return 1;
      },
      done(mem, outL, outR, frames) {
        slot.outL.set(new Float32Array(mem.buffer, outL, frames));
        slot.outR.set(new Float32Array(mem.buffer, outR, frames));
        served++;
        respond(slot, last, (performance.now() - tBlock) * 1000);
      },
    },
  });
  const inst = await WebAssembly.instantiate(m.module, host.imports);
  host.attach(inst);
  const rc = host.run();
  port.post({ type: 'done', id: m.id, rc, served, result: parseResult(lines), lines, missing: host.missingFiles() });
}

const port = await openPort();
port.onMessage((raw) => {
  const m = raw as InitMessage;
  if (m.type !== 'init') return;
  run(port, m).catch((e: unknown) => {
    try {
      Slot.attach(m.sab).setState(STATE.FAILED);
    } catch {
      /* the slot itself was the problem */
    }
    port.post({ type: 'error', id: m.id, error: e instanceof Error ? (e.stack ?? e.message) : String(e) });
  });
});
