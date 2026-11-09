import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { BTN, IN, type RawInput } from '../../src/input/events.ts';
import { attachDesktopInput } from '../../src/input/pointer.ts';
import type { InputMeta } from '../../src/ui/latency.ts';

// A desktop element of 960 x 540 CSS px at (100, 50) (half size), made of node's EventTarget: the listeners of attachDesktopInput run on events built here
class FakeEl extends EventTarget {
  focused = 0; captured: number[] = [];
  focus(): void { this.focused++; }
  setPointerCapture(id: number): void { this.captured.push(id); }
}
let winT: EventTarget, docT: EventTarget & { visibilityState: string };
const saved: Record<string, unknown> = {};
beforeEach(() => {
  winT = new EventTarget();
  docT = Object.assign(new EventTarget(), { visibilityState: 'visible' });
  for (const k of ['window', 'document']) saved[k] = (globalThis as Record<string, unknown>)[k];
  (globalThis as Record<string, unknown>).window = winT;
  (globalThis as Record<string, unknown>).document = docT;
});
afterEach(() => { for (const k of ['window', 'document']) (globalThis as Record<string, unknown>)[k] = saved[k]; });

const rect = { left: 100, top: 50, width: 960, height: 540 };
function make(enabled = true, sendTab = false) {
  const el = new FakeEl();
  const raws: [RawInput, string][] = [];
  const on = { enabled, sendTab };
  const h = attachDesktopInput(el as unknown as HTMLElement, { enabled: () => on.enabled, sendTab: () => on.sendTab, onRaw: (r, from) => raws.push([r, from]), rect: () => rect });
  return { el, raws, h, on };
}
function ptr(type: string, o: Record<string, unknown> = {}): Event & Record<string, unknown> {
  const e = new Event(type, { cancelable: true }) as Event & Record<string, unknown>;
  Object.assign(e, { pointerId: 1, pointerType: 'mouse', isPrimary: true, clientX: 0, clientY: 0, buttons: 0, ...o });
  return e;
}
function key(type: string, code: string, o: Record<string, unknown> = {}): Event & Record<string, unknown> {
  const e = new Event(type, { cancelable: true }) as Event & Record<string, unknown>;
  Object.assign(e, { code, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, isComposing: false, keyCode: 0, ...o });
  return e;
}

describe('attachDesktopInput', () => {
  it('decision 0260: the browser moves the focus from the desktop to another control on its own: the desktop gets it back; not after a pointer press or a Tab, not from a text field', async () => {
    const { el } = make();
    let contains = false;
    const other = Object.assign(new FakeEl(), { tagName: 'BUTTON', contains: (): boolean => contains });
    const field = Object.assign(new FakeEl(), { tagName: 'INPUT', contains: (): boolean => false });
    const doc = docT as unknown as { activeElement: unknown };
    el.focus = (): void => { el.focused++; doc.activeElement = el; };   // as an element does: focus() makes it the active element
    const out = async (to: unknown, before?: () => void): Promise<void> => {
      doc.activeElement = el;
      before?.();
      doc.activeElement = to;
      const ev = new Event('focusout'); Object.assign(ev, { relatedTarget: to });
      el.dispatchEvent(ev);
      await new Promise((r) => setTimeout(r, 30));
    };
    await new Promise((r) => setTimeout(r, 1050));   // the start: no user move in the last second
    await out(other);
    expect(el.focused).toBe(1);                       // on its own: back
    expect(doc.activeElement).toBe(el);
    await out(field);
    expect(el.focused).toBe(1);                       // a text field keeps it
    await out(other, () => docT.dispatchEvent(ptr('pointerdown')));
    expect(el.focused).toBe(2);                       // a press on the desktop itself (the click that opened a menu) does not excuse a move to another control
    await new Promise((r) => setTimeout(r, 1050));
    contains = true;                                  // the press was on that control
    await out(other, () => docT.dispatchEvent(ptr('pointerdown')));
    expect(el.focused).toBe(2);
    expect(doc.activeElement).toBe(other);
    contains = false;
    await new Promise((r) => setTimeout(r, 1050));
    await out(other, () => docT.dispatchEvent(key('keydown', 'Tab')));
    expect(el.focused).toBe(2);                       // Tab
    await new Promise((r) => setTimeout(r, 1050));
    // the focus fell to the page body (no control to go to)
    let inside = false;
    (el as unknown as { contains: () => boolean }).contains = (): boolean => inside;
    const fall = async (before?: () => void): Promise<void> => {
      doc.activeElement = el;
      before?.();
      doc.activeElement = null;
      const gone = new Event('focusout'); Object.assign(gone, { relatedTarget: null });
      el.dispatchEvent(gone);
      await new Promise((r) => setTimeout(r, 30));
    };
    await fall();
    expect(el.focused).toBe(3);                       // on its own: back
    await new Promise((r) => setTimeout(r, 1050));
    await fall(() => docT.dispatchEvent(ptr('pointerdown')));
    expect(el.focused).toBe(3);                       // a press outside the desktop (a click on the page background): the user's
    await new Promise((r) => setTimeout(r, 1050));
    inside = true;
    await fall(() => docT.dispatchEvent(ptr('pointerdown')));
    expect(el.focused).toBe(4);                       // a press on the desktop itself does not excuse the fall
  });
  it('a real click sequence: pointerdown focuses and captures, the events carry screen coordinates, pointerup ends it', () => {
    const { el, raws } = make();
    el.dispatchEvent(ptr('pointermove', { clientX: 100 + 479, clientY: 50 + 299 }));
    const d = ptr('pointerdown', { clientX: 100 + 479, clientY: 50 + 299, buttons: 1 });
    el.dispatchEvent(d);
    el.dispatchEvent(ptr('pointerup', { clientX: 100 + 479, clientY: 50 + 299, buttons: 0 }));
    expect(raws.map(([r]) => r)).toEqual([
      { k: 'mouse', kind: IN.MOVE, x: 958, y: 598, data: 0 },
      { k: 'mouse', kind: IN.BDOWN, x: 958, y: 598, data: BTN.L },
      { k: 'mouse', kind: IN.BUP, x: 958, y: 598, data: BTN.L },
    ]);
    expect(el.focused).toBe(1);
    expect(el.captured).toEqual([1]);
    expect(d.defaultPrevented).toBe(true);
  });
  it('the context menu is suppressed while the desktop takes input, not otherwise', () => {
    const a = make(true); const e1 = new Event('contextmenu', { cancelable: true }); a.el.dispatchEvent(e1);
    expect(e1.defaultPrevented).toBe(true);
    const b = make(false); const e2 = new Event('contextmenu', { cancelable: true }); b.el.dispatchEvent(e2);
    expect(e2.defaultPrevented).toBe(false);
  });
  it('a disabled desktop (engine starting or stopped) sends nothing and keeps every key and click for the browser', () => {
    const { el, raws } = make(false);
    el.dispatchEvent(ptr('pointerdown', { clientX: 200, clientY: 100, buttons: 1 }));
    const k = key('keydown', 'Space');
    el.dispatchEvent(k);
    expect(raws).toHaveLength(0);
    expect(k.defaultPrevented).toBe(false);
  });
  it('Enter, Space and F5 are forwarded and their browser action suppressed; Tab and Ctrl+W are left alone and not forwarded', () => {
    const { el, raws } = make();
    const enter = key('keydown', 'Enter'), space = key('keydown', 'Space'), tab = key('keydown', 'Tab'), f5 = key('keydown', 'F5'), cw = key('keydown', 'KeyW', { ctrlKey: true });
    for (const e of [enter, space, tab, f5, cw]) el.dispatchEvent(e);
    expect(raws.map(([r]) => (r.k === 'key' ? r.vk : -1))).toEqual([0x0d, 0x20, 0x74]);
    expect([enter.defaultPrevented, space.defaultPrevented, tab.defaultPrevented, f5.defaultPrevented, cw.defaultPrevented]).toEqual([true, true, false, true, false]);
  });
  it('a disabled desktop keeps F5 for the browser (the engine cannot take it yet: the reload is the way out of a stopped page)', () => {
    const { el, raws } = make(false);
    const f5 = key('keydown', 'F5');
    el.dispatchEvent(f5);
    expect(raws).toHaveLength(0);
    expect(f5.defaultPrevented).toBe(false);
  });
  it('Tab is forwarded when asked for', () => {
    const { el, raws } = make(true, true);
    const tab = key('keydown', 'Tab');
    el.dispatchEvent(tab);
    expect(raws).toHaveLength(1);
    expect(tab.defaultPrevented).toBe(true);
  });
  it('blur of the desktop, of the window, and a hidden tab release the held keys and buttons', () => {
    for (const trigger of [(m: ReturnType<typeof make>) => m.el.dispatchEvent(new Event('blur')), () => winT.dispatchEvent(new Event('blur')), () => { docT.visibilityState = 'hidden'; docT.dispatchEvent(new Event('visibilitychange')); }]) {
      docT.visibilityState = 'visible';
      const m = make();
      m.el.dispatchEvent(key('keydown', 'ShiftLeft'));
      m.el.dispatchEvent(ptr('pointerdown', { clientX: 300, clientY: 200, buttons: 1 }));
      m.raws.length = 0;
      trigger(m);
      expect(m.raws.map(([r]) => r)).toEqual([{ k: 'key', kind: IN.KUP, vk: 0xa0, scan: 0x2a, ext: 0 }, { k: 'mouse', kind: IN.BUP, x: 400, y: 300, data: BTN.L }]);
      expect(m.raws.every(([, from]) => from === 'blur')).toBe(true);
      m.h.detach();
    }
  });
  it('a visible tab does not release anything', () => {
    const m = make();
    m.el.dispatchEvent(key('keydown', 'KeyA'));
    m.raws.length = 0;
    docT.dispatchEvent(new Event('visibilitychange'));
    expect(m.raws).toHaveLength(0);
  });
  it('pointercancel releases the buttons', () => {
    const m = make();
    m.el.dispatchEvent(ptr('pointerdown', { clientX: 300, clientY: 200, buttons: 1 }));
    m.raws.length = 0;
    m.el.dispatchEvent(ptr('pointercancel'));
    expect(m.raws.map(([r]) => r)).toEqual([{ k: 'mouse', kind: IN.BUP, x: 400, y: 300, data: BTN.L }]);
  });
  it('the wheel is suppressed and sent as -120 per 100 px, at the pointer', () => {
    const { el, raws } = make();
    const w = Object.assign(new Event('wheel', { cancelable: true }), { clientX: 100, clientY: 50, deltaY: 100, deltaMode: 0 });
    el.dispatchEvent(w);
    expect(w.defaultPrevented).toBe(true);
    expect(raws.map(([r]) => r)).toEqual([{ k: 'mouse', kind: IN.MOVE, x: 0, y: 0, data: 0 }, { k: 'mouse', kind: IN.WHEEL, x: 0, y: 0, data: -120 }]);
  });
  it('a second pointer of a touch (not primary) is ignored', () => {
    const { el, raws } = make();
    el.dispatchEvent(ptr('pointerdown', { pointerType: 'touch', isPrimary: false, buttons: 1, clientX: 200, clientY: 100 }));
    expect(raws).toHaveLength(0);
  });
  it('detach removes the listeners', () => {
    const m = make();
    m.h.detach();
    m.el.dispatchEvent(key('keydown', 'KeyA'));
    expect(m.raws).toHaveLength(0);
  });
});

// Decision 0202: each RawInput carries the DOM event's own timeStamp (when the browser made the event) and the instant its listener started, both on the clock performance.timeOrigin + performance.now()
describe('attachDesktopInput: the event timeStamp goes with every input it causes (decision 0202)', () => {
  function makeMeta() {
    const el = new FakeEl();
    const got: { r: RawInput; from: string; meta: InputMeta | undefined }[] = [];
    const h = attachDesktopInput(el as unknown as HTMLElement, { enabled: () => true, onRaw: (r, from, meta) => got.push({ r, from, meta }), rect: () => rect });
    return { el, got, h };
  }
  const stamped = <T extends Event>(e: T, ts: number): T => { Object.defineProperty(e, 'timeStamp', { value: ts }); return e; };
  it('a pointer event: tEvent is performance.timeOrigin + event.timeStamp, tListener is later (the listener ran after the event was made)', () => {
    const { el, got } = makeMeta();
    const ts = performance.now() - 7;
    el.dispatchEvent(stamped(ptr('pointermove', { clientX: 300, clientY: 200 }), ts));
    expect(got).toHaveLength(1);
    expect(got[0]!.meta!.tEvent).toBe(performance.timeOrigin + ts);
    expect(got[0]!.meta!.tListener - got[0]!.meta!.tEvent).toBeGreaterThanOrEqual(6.9);
    expect(got[0]!.meta!.tListener).toBeLessThanOrEqual(performance.timeOrigin + performance.now());
  });
  it('every RawInput one DOM event causes has that event\'s stamp (the wheel sends a move and then the wheel)', () => {
    const { el, got } = makeMeta();
    el.dispatchEvent(stamped(Object.assign(new Event('wheel', { cancelable: true }), { clientX: 100, clientY: 50, deltaY: 100, deltaMode: 0 }), 1234.5));
    expect(got.map((g) => g.r.kind)).toEqual([IN.MOVE, IN.WHEEL]);
    expect(got.map((g) => g.meta!.tEvent)).toEqual([performance.timeOrigin + 1234.5, performance.timeOrigin + 1234.5]);
  });
  it('pointerdown, pointerup, keydown and keyup are stamped; pointerdown\'s focus() and capture do not move the stamp', () => {
    const { el, got } = makeMeta();
    el.dispatchEvent(stamped(ptr('pointerdown', { clientX: 300, clientY: 200, buttons: 1 }), 10));
    el.dispatchEvent(stamped(ptr('pointerup', { clientX: 300, clientY: 200, buttons: 0 }), 20));
    el.dispatchEvent(stamped(key('keydown', 'KeyA'), 30));
    el.dispatchEvent(stamped(key('keyup', 'KeyA'), 40));
    expect(got.every((g) => g.meta !== undefined)).toBe(true);
    expect([...new Set(got.map((g) => `${g.from} ${g.meta!.tEvent - performance.timeOrigin}`))]).toEqual(['pointerdown 10', 'pointerup 20', 'keydown 30', 'keyup 40']);
  });
  it('a release caused by blur is not an input event: no stamp', () => {
    const { el, got } = makeMeta();
    el.dispatchEvent(stamped(key('keydown', 'ShiftLeft'), 5));
    got.length = 0;
    el.dispatchEvent(new Event('blur'));
    expect(got.length).toBeGreaterThan(0);
    expect(got.every((g) => g.from === 'blur' && g.meta === undefined)).toBe(true);
  });
  it('the stamp does not outlive its event: what is released later by another cause has none', () => {
    const { el, got } = makeMeta();
    el.dispatchEvent(stamped(key('keydown', 'ShiftLeft'), 5));
    got.length = 0;
    docT.visibilityState = 'hidden';
    docT.dispatchEvent(new Event('visibilitychange'));
    expect(got.length).toBeGreaterThan(0);
    expect(got.every((g) => g.meta === undefined)).toBe(true);
  });
  it('detach removes the stamping listeners too', () => {
    const { el, got, h } = makeMeta();
    h.detach();
    el.dispatchEvent(stamped(key('keydown', 'KeyA'), 5));
    expect(got).toHaveLength(0);
  });
});
