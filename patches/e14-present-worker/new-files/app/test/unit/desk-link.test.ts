// Decision 0311: the link between the engine Worker and the present Worker: one scene in flight, the description is copied (not shared), the present Worker is woken by the publish, and what it finished comes back as a number.
import { describe, expect, it } from 'vitest';
import { DeskLinkEngine, DeskLinkP, HEADER_BYTES, LINK_BYTES, SCENE_BYTES, newDeskLink } from '../../src/engine/desk-link.ts';

function engineMemory(fill: number, at: number): { mem: SharedArrayBuffer; ptr: number } {
  const mem = new SharedArrayBuffer(1 << 16);
  new Uint8Array(mem, at, SCENE_BYTES).fill(fill);
  return { mem, ptr: at };
}

describe('desk link', () => {
  it('has a header and room for exactly one description', () => {
    expect(SCENE_BYTES).toBe(64 + 64 * 80 + 64 * 20);
    expect(LINK_BYTES).toBe(HEADER_BYTES + SCENE_BYTES);
    expect(newDeskLink().byteLength).toBe(LINK_BYTES);
  });

  it('publishes one scene at a time: the next waits until P has finished the one in flight', () => {
    const sab = newDeskLink(), e = new DeskLinkEngine(sab), p = new DeskLinkP(sab);
    expect(e.canPublish()).toBe(true);
    const { mem, ptr } = engineMemory(7, 4096);
    const seq = e.publish(mem, ptr);
    expect(seq).toBe(1);
    expect(e.canPublish()).toBe(false);
    p.markDone(seq, 3, 1234.5);
    expect(e.canPublish()).toBe(true);
    expect(e.done).toBe(1);
    expect(e.result).toBe(3);
    expect(e.presentAt).toBe(1234.5);
    expect(e.presents).toBe(1);
  });

  it('copies the description into the slot at the time of the publish', () => {
    const sab = newDeskLink(), e = new DeskLinkEngine(sab), p = new DeskLinkP(sab);
    const { mem, ptr } = engineMemory(9, 8192);
    e.publish(mem, ptr);
    new Uint8Array(mem, ptr, SCENE_BYTES).fill(1);   // the runtime writes the next description; the slot keeps the published one
    const slot = new Uint8Array(sab, p.slotOffset, SCENE_BYTES);
    expect(slot[0]).toBe(9);
    expect(slot[SCENE_BYTES - 1]).toBe(9);
  });

  it('wakes P: waitPublished resolves when PUB changes and at once when it already differs', async () => {
    const sab = newDeskLink(), e = new DeskLinkEngine(sab), p = new DeskLinkP(sab);
    const waiting = p.waitPublished(0);
    const { mem, ptr } = engineMemory(5, 0);
    e.publish(mem, ptr);
    await expect(waiting).resolves.toBe(1);
    await expect(p.waitPublished(0)).resolves.toBe(1);
  });

  it('carries the requests: a frame asks, the engine clears it when it publishes; the mode and the resync are asked by P and done by the engine', () => {
    const sab = newDeskLink(), e = new DeskLinkEngine(sab), p = new DeskLinkP(sab);
    expect(e.wantSet).toBe(false);
    p.requestScene();
    expect(e.wantSet).toBe(true);
    e.clearWant();
    expect(e.wantSet).toBe(false);
    expect(e.modeRequested()).toBe(false);
    p.requestMode(1);
    expect(e.modeRequested()).toBe(true);
    e.setModeOn();
    expect(e.modeRequested()).toBe(false);
    expect(e.modeOn).toBe(true);
    expect(e.takeResync()).toBe(false);
    p.requestResync();
    expect(e.takeResync()).toBe(true);
    expect(e.takeResync()).toBe(false);
    p.setAsleep(true);
    expect(e.asleep).toBe(true);
    p.setReady(1);
    expect(e.ready).toBe(1);
    e.setCounters(4, 5, 6);
    expect([p.counter(10), p.counter(11), p.counter(12)]).toEqual([4, 5, 6]);
  });
});
