// A stand-in for the engine's mixer thread (decision 0173, S1): it drives K plugin instances block by block, either all at once (fan-out) or as a chain, optionally paced at the real-time block
// period, and records the round trip of every block. It needs a thread that may block (a Worker, or node's main thread). Used by the tests; the engine integration (S3/S4) replaces it.
import { Slot, beginBlock, readOutput, waitResponse } from './protocol.ts';

/** What the driver needs from one plugin: PluginInstance implements it, and so does SlotEndpoint (a Worker that was only handed the slot's SharedArrayBuffer). */
export interface BlockEndpoint {
  readonly frames: number;
  begin(inL: Float32Array, inR: Float32Array): number;
  waitSync(seq: number, timeoutMs?: number): boolean;
  read(outL: Float32Array, outR: Float32Array): void;
}

export class SlotEndpoint implements BlockEndpoint {
  readonly slot: Slot;
  readonly frames: number;
  constructor(sab: SharedArrayBuffer, frames = 4096) {
    this.slot = Slot.attach(sab);
    this.frames = frames;
  }
  begin(inL: Float32Array, inR: Float32Array): number {
    return beginBlock(this.slot, inL, inR, this.frames);
  }
  waitSync(seq: number, timeoutMs = Infinity): boolean {
    return waitResponse(this.slot, seq, timeoutMs);
  }
  read(outL: Float32Array, outR: Float32Array): void {
    readOutput(this.slot, outL, outR, this.frames);
  }
}

export interface DriveOptions {
  blocks: number;
  /** block period in ms (4096 frames at 44.1 kHz = 92.879 ms); 0 = as fast as possible */
  periodMs: number;
  topology: 'fanout' | 'chain';
  /** fill the next input block */
  signal: (instance: number, frames: number, l: Float32Array, r: Float32Array) => void;
  /** called with each instance's output block (for hashing / checks); optional */
  sink?: (instance: number, block: number, l: Float32Array, r: Float32Array) => void;
  sleep?: (ms: number) => void;
}

export interface DriveResult {
  latencyMs: number[];
  /** blocks whose request->all-answers time exceeded the period (only counted when periodMs > 0) */
  late: number[];
  startLateMs: number[];
}

function defaultSleep(ms: number): void {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

export function drive(instances: BlockEndpoint[], o: DriveOptions): DriveResult {
  const frames = instances[0]?.frames ?? 4096;
  const inL = instances.map(() => new Float32Array(frames));
  const inR = instances.map(() => new Float32Array(frames));
  const outL = instances.map(() => new Float32Array(frames));
  const outR = instances.map(() => new Float32Array(frames));
  const sleep = o.sleep ?? defaultSleep;
  const res: DriveResult = { latencyMs: [], late: [], startLateMs: [] };
  const t0 = performance.now() + 5;
  for (let b = 0; b < o.blocks; b++) {
    const sched = t0 + b * o.periodMs;
    if (o.periodMs > 0) {
      const d = sched - performance.now();
      if (d > 0) sleep(d);
    }
    const ts = performance.now();
    if (o.topology === 'fanout') {
      const seqs: number[] = [];
      instances.forEach((p, k) => {
        o.signal(k, frames, inL[k]!, inR[k]!);
        seqs.push(p.begin(inL[k]!, inR[k]!));
      });
      instances.forEach((p, k) => {
        if (!p.waitSync(seqs[k]!)) throw new Error('no answer');
        p.read(outL[k]!, outR[k]!);
        o.sink?.(k, b, outL[k]!, outR[k]!);
      });
    } else {
      instances.forEach((p, k) => {
        if (k === 0) o.signal(0, frames, inL[0]!, inR[0]!);
        else {
          inL[k]!.set(outL[k - 1]!);
          inR[k]!.set(outR[k - 1]!);
        }
        const seq = p.begin(inL[k]!, inR[k]!);
        if (!p.waitSync(seq)) throw new Error('no answer');
        p.read(outL[k]!, outR[k]!);
        o.sink?.(k, b, outL[k]!, outR[k]!);
      });
    }
    const te = performance.now();
    res.latencyMs.push(te - ts);
    res.startLateMs.push(ts - sched);
    if (o.periodMs > 0 && te - ts > o.periodMs) res.late.push(b);
  }
  return res;
}
