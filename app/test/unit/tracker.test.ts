import { describe, expect, it } from 'vitest';
import { BTN, IN, type RawInput } from '../../src/input/events.ts';
import { clientToScreen, InputTracker, WheelAccumulator } from '../../src/input/tracker.ts';

const rig = () => { const out: RawInput[] = []; return { out, t: new InputTracker((e) => out.push(e)) }; };
const M = (kind: number, x: number, y: number, data = 0): RawInput => ({ k: 'mouse', kind: kind as 1, x, y, data });

describe('clientToScreen: CSS position on the desktop view -> the engine\'s integer screen position', () => {
  const rect = { left: 100, top: 50, width: 960, height: 540 };   // half size
  it('scales by the view\'s size (half-size view: 2 screen px per CSS px) and floors', () => {
    expect(clientToScreen(100, 50, rect)).toEqual({ x: 0, y: 0, inside: true });
    expect(clientToScreen(100 + 479, 50 + 299, rect)).toEqual({ x: 958, y: 598, inside: true });
    expect(clientToScreen(100 + 479.9, 50 + 299.9, rect)).toEqual({ x: 959, y: 599, inside: true });
  });
  it('the y axis is not flipped (the top of the view is screen y 0)', () => {
    expect(clientToScreen(100, 50 + 10, rect).y).toBeLessThan(clientToScreen(100, 50 + 200, rect).y);
    expect(clientToScreen(100, 50, rect).y).toBe(0);
  });
  it('the 958,599 of the G3 measurement is reached from a 1:1 view', () => {
    expect(clientToScreen(10 + 958, 20 + 599, { left: 10, top: 20, width: 1920, height: 1080 })).toEqual({ x: 958, y: 599, inside: true });
  });
  it('clamps outside positions to the virtual screen and says they were outside (pointer capture keeps sending them)', () => {
    expect(clientToScreen(-500, -500, rect)).toEqual({ x: 0, y: 0, inside: false });
    expect(clientToScreen(5000, 5000, rect)).toEqual({ x: 1919, y: 1079, inside: false });
    expect(clientToScreen(100 + 960, 50 + 540, rect)).toMatchObject({ x: 1919, y: 1079, inside: false });
  });
  it('a view without size maps to nothing', () => {
    expect(clientToScreen(1, 1, { left: 0, top: 0, width: 0, height: 0 })).toEqual({ x: 0, y: 0, inside: false });
  });
});

describe('InputTracker: pointer', () => {
  it('a click: move, button down, button up - the down and the up carry the position', () => {
    const { out, t } = rig();
    t.pointer({ x: 958, y: 599, buttons: 0 });
    t.pointer({ x: 958, y: 599, buttons: 1 });
    t.pointer({ x: 958, y: 599, buttons: 0 });
    expect(out).toEqual([M(IN.MOVE, 958, 599), M(IN.BDOWN, 958, 599, BTN.L), M(IN.BUP, 958, 599, BTN.L)]);
  });
  it('a button pressed with no move before it still has its move first (the position changed)', () => {
    const { out, t } = rig();
    t.pointer({ x: 10, y: 20, buttons: 1 });
    expect(out).toEqual([M(IN.MOVE, 10, 20), M(IN.BDOWN, 10, 20, BTN.L)]);
  });
  it('no event when nothing changed', () => {
    const { out, t } = rig();
    t.pointer({ x: 5, y: 5, buttons: 0 });
    t.pointer({ x: 5, y: 5, buttons: 0 });
    expect(out).toHaveLength(1);
  });
  it('the DOM `buttons` mask: 2 is the RIGHT button and 4 the MIDDLE button (the DOM `button` numbers them the other way round)', () => {
    const a = rig(); a.t.pointer({ x: 1, y: 1, buttons: 2 });
    expect(a.out[1]).toEqual(M(IN.BDOWN, 1, 1, BTN.R));
    const b = rig(); b.t.pointer({ x: 1, y: 1, buttons: 4 });
    expect(b.out[1]).toEqual(M(IN.BDOWN, 1, 1, BTN.M));
    const c = rig(); c.t.pointer({ x: 1, y: 1, buttons: 1 });
    expect(c.out[1]).toEqual(M(IN.BDOWN, 1, 1, BTN.L));
  });
  it('a second button pressed while the first is held (arrives as a move with a changed mask) is a down of that button only', () => {
    const { out, t } = rig();
    t.pointer({ x: 3, y: 3, buttons: 1 });
    out.length = 0;
    t.pointer({ x: 3, y: 3, buttons: 3 });
    expect(out).toEqual([M(IN.BDOWN, 3, 3, BTN.R)]);
    out.length = 0;
    t.pointer({ x: 3, y: 3, buttons: 2 });
    expect(out).toEqual([M(IN.BUP, 3, 3, BTN.L)]);
  });
  it('the back / forward buttons (mask 8, 16) are not the engine\'s', () => {
    const { out, t } = rig();
    t.pointer({ x: 0, y: 0, buttons: 8 });
    expect(out.filter((e) => e.k === 'mouse' && e.kind === IN.BDOWN)).toHaveLength(0);
  });
  it('a lost mouse-up is made good by the next event: a move with buttons 0 releases the button where the pointer is', () => {
    const { out, t } = rig();
    t.pointer({ x: 7, y: 7, buttons: 1 });
    out.length = 0;
    t.pointer({ x: 9, y: 9, buttons: 0 });
    expect(out).toEqual([M(IN.MOVE, 9, 9), M(IN.BUP, 9, 9, BTN.L)]);
  });
  it('pointercancel releases every button at the last position', () => {
    const { out, t } = rig();
    t.pointer({ x: 4, y: 6, buttons: 3 });
    out.length = 0;
    t.pointerCancel();
    expect(out).toEqual([M(IN.BUP, 4, 6, BTN.L), M(IN.BUP, 4, 6, BTN.R)]);
    expect(t.buttonsDown).toBe(0);
  });
  it('wheel moves the pointer there first and sends the delta; a zero delta sends nothing', () => {
    const { out, t } = rig();
    t.wheel(100, 200, 0);
    expect(out).toHaveLength(0);
    t.wheel(100, 200, 120);
    expect(out).toEqual([M(IN.MOVE, 100, 200), M(IN.WHEEL, 100, 200, 120)]);
  });
});

describe('InputTracker: keys', () => {
  it('Space down and up: the virtual key 0x20, the scan code 0x39', () => {
    const { out, t } = rig();
    expect(t.keyDown('Space')).toBe(true);
    expect(t.keyUp('Space')).toBe(true);
    expect(out).toEqual([{ k: 'key', kind: IN.KDOWN, vk: 0x20, scan: 0x39, ext: 0 }, { k: 'key', kind: IN.KUP, vk: 0x20, scan: 0x39, ext: 0 }]);
  });
  it('Enter is 0xd / 0x1c and the keypad Enter is extended', () => {
    const { out, t } = rig();
    t.keyDown('Enter'); t.keyDown('NumpadEnter');
    expect(out).toEqual([{ k: 'key', kind: IN.KDOWN, vk: 0xd, scan: 0x1c, ext: 0 }, { k: 'key', kind: IN.KDOWN, vk: 0xd, scan: 0x1c, ext: 1 }]);
  });
  it('an auto-repeat is another key down (the engine derives the previous-state bit from its own state)', () => {
    const { out, t } = rig();
    t.keyDown('KeyA'); t.keyDown('KeyA'); t.keyDown('KeyA');
    expect(out.filter((e) => e.k === 'key' && e.kind === IN.KDOWN)).toHaveLength(3);
    expect(t.keysDown).toEqual(['KeyA']);
  });
  it('modifiers are keys like the others: Shift down, click, Shift up arrive in that order', () => {
    const { out, t } = rig();
    t.keyDown('ShiftLeft');
    t.pointer({ x: 2, y: 2, buttons: 1 });
    t.keyUp('ShiftLeft');
    expect(out.map((e) => (e.k === 'key' ? `key${e.kind}:${e.vk.toString(16)}` : `m${e.kind}`))).toEqual(['key5:a0', 'm1', 'm2', 'key6:a0']);
  });
  it('an unknown code is not sent', () => {
    const { out, t } = rig();
    expect(t.keyDown('LaunchMail')).toBe(false);
    expect(t.keyUp('LaunchMail')).toBe(false);
    expect(out).toHaveLength(0);
  });
  it('losing the focus releases every held key (last pressed first) and every held button', () => {
    const { out, t } = rig();
    t.keyDown('ControlLeft'); t.keyDown('KeyC'); t.pointer({ x: 8, y: 8, buttons: 1 });
    out.length = 0;
    t.releaseAll();
    expect(out).toEqual([{ k: 'key', kind: IN.KUP, vk: 0x43, scan: 0x2e, ext: 0 }, { k: 'key', kind: IN.KUP, vk: 0xa2, scan: 0x1d, ext: 0 }, M(IN.BUP, 8, 8, BTN.L)]);
    expect(t.keysDown).toEqual([]);
    expect(t.buttonsDown).toBe(0);
    out.length = 0;
    t.releaseAll();
    expect(out).toHaveLength(0);   // nothing held: nothing sent
  });
});

describe('WheelAccumulator', () => {
  it('100 px of DOM deltaY (scroll down) is one notch BACKWARD: -120', () => {
    expect(new WheelAccumulator().add(100, 0, 0)).toBe(-120);
    expect(new WheelAccumulator().add(-100, 0, 0)).toBe(120);
  });
  it('lines and pages', () => {
    expect(new WheelAccumulator().add(1, 1, 0)).toBe(-120);
    expect(new WheelAccumulator().add(-2, 1, 0)).toBe(240);
    expect(new WheelAccumulator().add(1, 2, 0)).toBe(-360);
  });
  it('small trackpad steps add up to a notch instead of being lost', () => {
    const w = new WheelAccumulator();
    const got = [30, 30, 30, 30].map((d, i) => w.add(d, 0, i * 10));
    expect(got).toEqual([0, 0, 0, -120]);
  });
  it('the remainder is forgotten after a pause', () => {
    const w = new WheelAccumulator(300);
    expect(w.add(60, 0, 0)).toBe(0);
    expect(w.add(60, 0, 1000)).toBe(0);   // 60 + 60 would be a notch, but the first 60 is a second old
  });
});
