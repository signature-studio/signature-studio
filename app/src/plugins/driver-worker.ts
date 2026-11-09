// A Worker that plays the engine's mixer thread against plugin slots it was handed (decision 0173, S1; browser tests). The page starts the plugin instances (async, main thread) and passes each
// slot's SharedArrayBuffer here; this Worker may block, so it can use the synchronous protocol exactly as the engine's Worker will.
import { drive, SlotEndpoint, type DriveResult } from './driver.ts';
import { OttTestSignal } from './test-signal.ts';

export interface DriveRequest {
  type: 'drive';
  sabs: SharedArrayBuffer[];
  frames: number;
  blocks: number;
  periodMs: number;
  topology: 'fanout' | 'chain';
  scale?: number;
  silence?: boolean;
}

const g = globalThis as unknown as { postMessage(m: unknown): void; onmessage: ((e: { data: DriveRequest }) => void) | null };
g.onmessage = (e) => {
  const m = e.data;
  if (m.type !== 'drive') return;
  try {
    const eps = m.sabs.map((s) => new SlotEndpoint(s, m.frames));
    const sigs = eps.map(() => new OttTestSignal({ scale: m.scale, silence: m.silence }));
    const res: DriveResult = drive(eps, { blocks: m.blocks, periodMs: m.periodMs, topology: m.topology, signal: (k, n, l, r) => sigs[k]!.next(n, l, r) });
    g.postMessage({ type: 'driven', res });
  } catch (err) {
    g.postMessage({ type: 'error', error: String(err instanceof Error ? (err.stack ?? err.message) : err) });
  }
};
