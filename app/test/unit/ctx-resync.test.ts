import { describe, expect, it } from 'vitest';
import { watchContextLoss, type ContextLossTarget } from '../../src/engine/ctx-resync.ts';

// decision 0201: a 2D canvas that lost its context (contextlost / contextrestored) is empty afterwards. The Worker asks the desktop compositor for the whole picture again (DeskPresenter.resync, decision 0223).

class FakeTarget implements ContextLossTarget {
  readonly listeners = new Map<string, ((ev: Event) => void)[]>();
  addEventListener(type: 'contextlost' | 'contextrestored', fn: (ev: Event) => void): void { this.listeners.set(type, [...(this.listeners.get(type) ?? []), fn]); }
  fire(type: 'contextlost' | 'contextrestored'): { prevented: boolean } {
    let prevented = false;
    const ev = { type, preventDefault: () => { prevented = true; } } as unknown as Event;
    for (const f of this.listeners.get(type) ?? []) f(ev);
    return { get prevented() { return prevented; } };
  }
}

describe('watchContextLoss', () => {
  it('cancels contextlost (Chrome restores a 2D context only then) and tells the hooks; contextrestored is not cancelled', () => {
    const t = new FakeTarget(), calls: string[] = [];
    watchContextLoss(t, { lost: () => calls.push('lost'), restored: () => calls.push('restored') });
    expect(t.fire('contextlost').prevented).toBe(true);
    expect(calls).toEqual(['lost']);
    t.fire('contextrestored');
    expect(calls).toEqual(['lost', 'restored']);
  });
});
