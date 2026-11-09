import { describe, expect, it } from 'vitest';

// Decision 0271: the engine Worker's desk-dump (src/engine/worker.ts deskDump, the compositor's picture for the tests) copies pixels out of the wasm memory, which is a SharedArrayBuffer, and transfers the copy.
// SharedArrayBuffer.prototype.slice returns a SharedArrayBuffer, and a shared buffer in the transfer list is a DataCloneError (seen in Firefox when the desktop is drawn with the 2D presenter: no WebGL);
// new Uint8Array(sab, start, length).slice().buffer is a plain ArrayBuffer that can be transferred. This test pins the two facts the one-line change relies on.
describe('copying pixels out of the shared wasm memory for a transfer', () => {
  const sab = new SharedArrayBuffer(64);
  new Uint8Array(sab).forEach((_, i, a) => { a[i] = i; });
  it('SharedArrayBuffer.slice stays shared (so it cannot be transferred)', () => {
    const s = sab.slice(8, 16);
    expect(s instanceof SharedArrayBuffer).toBe(true);
    expect(() => structuredClone(s, { transfer: [s as unknown as ArrayBuffer] })).toThrow();
  });
  it('a Uint8Array slice of the shared memory is a plain ArrayBuffer with the same bytes, and it can be transferred', () => {
    const copy = new Uint8Array(sab, 8, 8).slice().buffer;
    expect(copy instanceof SharedArrayBuffer).toBe(false);
    expect([...new Uint8Array(copy)]).toEqual([8, 9, 10, 11, 12, 13, 14, 15]);
    const moved = structuredClone(copy, { transfer: [copy] });
    expect(moved.byteLength).toBe(8);
    expect(copy.byteLength).toBe(0);
  });
});
