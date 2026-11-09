// The plugin Worker protocol (src/plugins/protocol.ts), decision 0173 S1: layout, the Atomics handshake in both waiting styles, timeouts, quit. No wasm, no assets.
import { Worker } from 'node:worker_threads';
import { describe, expect, it } from 'vitest';
import { CMD, HEADER_BYTES, Slot, W, beginBlock, requestQuit, slotBytes, waitResponse, waitResponseAsync, readOutput } from '../../src/plugins/protocol.ts';
import { OttTestSignal } from '../../src/plugins/test-signal.ts';

function spawnEcho(slot: Slot): Worker {
  return new Worker(new URL('../helpers/echo-plugin-worker.ts', import.meta.url), { workerData: { sab: slot.sab } });
}

describe('slot layout', () => {
  it('has a 128-byte header and four planar float32 blocks', () => {
    expect(HEADER_BYTES).toBe(128);
    expect(slotBytes(4096)).toBe(128 + 4 * 4096 * 4);
    const s = Slot.create(4096);
    expect(s.sab.byteLength).toBe(slotBytes(4096));
    expect(s.inL.length).toBe(4096);
    expect(s.outR.byteOffset).toBe(HEADER_BYTES + 12 * 4096);
  });

  it('attach recovers maxFrames and rejects foreign buffers', () => {
    const s = Slot.create(512);
    expect(Slot.attach(s.sab).maxFrames).toBe(512);
    expect(() => Slot.attach(new SharedArrayBuffer(slotBytes(512)))).toThrow(/magic/);
    const bad = Slot.create(512);
    Atomics.store(bad.ctrl, W.MAX_FRAMES, 1024);
    expect(() => Slot.attach(bad.sab)).toThrow(/size/);
  });

  it('refuses a block larger than the slot', () => {
    const s = Slot.create(64);
    expect(() => beginBlock(s, new Float32Array(128), new Float32Array(128), 128)).toThrow(RangeError);
  });
});

describe('request / response', () => {
  it('round-trips 2000 blocks synchronously with the right data', async () => {
    const slot = Slot.create(4096);
    const w = spawnEcho(slot);
    const sig = new OttTestSignal();
    const l = new Float32Array(4096);
    const r = new Float32Array(4096);
    const ol = new Float32Array(4096);
    const or = new Float32Array(4096);
    for (let b = 0; b < 2000; b++) {
      sig.next(4096, l, r);
      const seq = beginBlock(slot, l, r, 4096);
      expect(waitResponse(slot, seq, 5000)).toBe(true);
      readOutput(slot, ol, or, 4096);
      if (b % 500 === 0) {
        expect(ol[10]).toBe(l[10]! * 0.5);
        expect(or[4095]).toBe(r[4095]! * 0.5);
      }
    }
    expect(Atomics.load(slot.ctrl, W.PROC_US)).toBe(7);
    requestQuit(slot);
    expect(waitResponse(slot, Atomics.load(slot.ctrl, W.REQ), 5000)).toBe(true);
    expect(Atomics.load(slot.ctrl, W.CMD)).toBe(CMD.QUIT);
    await w.terminate();
  });

  it('round-trips with the non-blocking wait (the style the browser main thread and a JSPI import need)', async () => {
    const slot = Slot.create(256);
    const w = spawnEcho(slot);
    const l = new Float32Array(256).fill(0.5);
    const r = new Float32Array(256).fill(-0.25);
    const o = new Float32Array(256);
    for (let b = 0; b < 300; b++) {
      const seq = beginBlock(slot, l, r, 256);
      expect(await waitResponseAsync(slot, seq, 5000)).toBe(true);
    }
    readOutput(slot, o, o, 256);
    expect(o[0]).toBe(-0.125);
    requestQuit(slot);
    await w.terminate();
  });

  it('times out when nobody answers (both waits)', async () => {
    const slot = Slot.create(64);
    const seq = beginBlock(slot, new Float32Array(64), new Float32Array(64), 64);
    expect(waitResponse(slot, seq, 30)).toBe(false);
    expect(await waitResponseAsync(slot, seq, 30)).toBe(false);
  });

  it('fans out to several slots and collects them in any order', async () => {
    const slots = [0, 1, 2, 3].map(() => Slot.create(128));
    const ws = slots.map(spawnEcho);
    const l = new Float32Array(128).fill(1);
    for (let b = 0; b < 200; b++) {
      const seqs = slots.map((s, k) => beginBlock(s, l.map((v) => v * (k + 1)), l, 128));
      for (let k = 3; k >= 0; k--) expect(waitResponse(slots[k]!, seqs[k]!, 5000)).toBe(true);
      expect(slots[2]!.outL[0]).toBe(1.5);
    }
    for (const s of slots) requestQuit(s);
    await Promise.all(ws.map((w) => w.terminate()));
  });
});

describe('the OTT harness input stream (test-signal.ts)', () => {
  it('is deterministic and advances across blocks', () => {
    const a = new OttTestSignal();
    const b = new OttTestSignal();
    const x1 = new Float32Array(64);
    const y1 = new Float32Array(64);
    const x2 = new Float32Array(64);
    const y2 = new Float32Array(64);
    a.next(64, x1, y1);
    b.next(64, x2, y2);
    expect(Array.from(x1)).toEqual(Array.from(x2));
    a.next(64, x1, y1);
    expect(Array.from(x1)).not.toEqual(Array.from(x2));
    expect(new OttTestSignal({ silence: true }).next(64, x1, y1)).toBeUndefined();
    expect(x1.every((v) => v === 0)).toBe(true);
  });
});
