import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { chooseGuestScreen, drawScale, RoomDebouncer, screenEnv, SCREEN_DEFAULT, SCREEN_MAX, SCREEN_MIN, type GuestScreenInfo, type Room } from '../../src/ui/screen-fit.ts';
import { beaconProblems } from '../../shared/perf.ts';
import { AV_HEADER_BYTES, AV_MAGIC, AV_VERSION, AV_WINDOW_BYTES, readAppView } from '../../src/engine/appview.ts';
import { clientToScreen } from '../../src/input/tracker.ts';

const REPO = process.env.APP_REPO_ROOT || path.resolve(import.meta.dirname, '../../..');
const rt = (f: string): string => fs.readFileSync(path.join(REPO, 'lift/runtime', f), 'utf8');

describe('the guest screen (decision 0212; one mode since decision 0243)', () => {
  it('the limits are the ones of lift/runtime/fle_screen.h (read from the file)', () => {
    const h = rt('fle_screen.h');
    const num = (n: string): number => Number(new RegExp(`#define ${n} (\\d+)u`).exec(h)?.[1]);
    expect({ width: num('FLE_SCREEN_MIN_W'), height: num('FLE_SCREEN_MIN_H') }).toEqual(SCREEN_MIN);
    expect({ width: num('FLE_SCREEN_MAX_W'), height: num('FLE_SCREEN_MAX_H') }).toEqual(SCREEN_MAX);
    expect({ width: num('FLE_SCREEN_DEFAULT_W'), height: num('FLE_SCREEN_DEFAULT_H') }).toEqual(SCREEN_DEFAULT);
    expect(h).toContain('"FLE_MONITOR_W"');
    expect(h).toContain('"FLE_MONITOR_H"');
    expect(h).toContain('"FLE_MONITOR_DPR"');
    expect(Object.keys(screenEnv(1, 2, 1))).toEqual(['FLE_MONITOR_W', 'FLE_MONITOR_H', 'FLE_MONITOR_DPR']);
  });
  it('the device pixel ratio goes to the engine as the ratio x 10000, a whole number (Math.round(dpr * 10000)); the page passes no scale of the UI', () => {
    for (const dpr of [1, 1.25, 1.7647, 1.23456, 2.25]) {
      const v = screenEnv(3240, 1863, dpr).FLE_MONITOR_DPR;
      expect(v).toBe(String(Math.round(dpr * 10000)));
      expect(Number.isInteger(Number(v))).toBe(true);
    }
    expect(Object.keys(screenEnv(3240, 1863, 2))).not.toContain('FLE_SCALE');
  });
});

describe('the guest screen for a room', () => {
  const room = (width: number, height: number, dpr: number): Room => ({ width, height, dpr });
  it('the guest screen is the room in device pixels (CSS x device pixel ratio, rounded to whole pixels), drawn at 1 / devicePixelRatio CSS pixels per pixel (C-1, #1347)', () => {
    const dpr = 1.5, w = 1580, h = 957;
    expect(chooseGuestScreen(room(w, h, dpr))).toEqual({ width: Math.round(w * dpr), height: Math.round(h * dpr), scale: 1 / dpr, clamped: false });
    expect(chooseGuestScreen(room(1240, 921, 2))).toEqual({ width: 2480, height: 1842, scale: 0.5, clamped: false });
    // a room at a ratio of 1 is the room itself: a guest pixel is a CSS pixel
    expect(chooseGuestScreen(room(1836.4, 1048.7, 1))).toMatchObject({ width: Math.round(1836.4), height: Math.round(1048.7), scale: 1 });
  });
  it('drawScale: the natural scale when the guest has the device pixels the room holds, a fit when it does not (a bundle that did not take the size)', () => {
    expect(drawScale({ width: 2480, height: 1842 }, room(1240.4, 921.9, 2))).toEqual({ scale: 0.5 });
    // a guest that is bigger than the room (the engine kept 1920x1080 in a 1240 room): fitted, not cut
    expect(drawScale({ width: 1920, height: 1080 }, room(1240, 921, 1)).scale).toBeCloseTo(1240 / 1920, 9);
    // a guest that the engine did not size to the room (not honoured) is fitted in both directions
    expect(drawScale({ width: 1920, height: 1080 }, room(960, 540, 1), false)).toEqual({ scale: 0.5 });
    expect(drawScale({ width: 1920, height: 1080 }, room(3840, 2160, 1), false)).toEqual({ scale: 2 });
    // a whole-pixel guest that overhangs the room by a fraction of a CSS pixel keeps the natural scale (a fit would resample it)
    const r = room(957, 957, 1.5), g = chooseGuestScreen(r);
    expect(drawScale({ width: g.width, height: g.height }, r).scale).toBe(1 / 1.5);
    // a room too small for the natural scale: the screen is fitted into it
    expect(drawScale({ width: 640, height: 480 }, room(200, 100, 2)).scale).toBeCloseTo(100 / 480, 9);
  });
  it('a room outside the engine limits is pulled inside them: below the minimum the screen is fitted into the room, above the maximum it stays at its natural scale', () => {
    const small = chooseGuestScreen(room(500, 300, 1));
    expect(small).toMatchObject({ width: 640, height: 480, clamped: true });
    expect(small.scale).toBeCloseTo(Math.min(500 / 640, 300 / 480), 9);
    // the limits are in device pixels: a room of 300 x 200 CSS px at ratio 2 is 600 x 400 device px, below the minimum
    expect(chooseGuestScreen(room(300, 200, 2))).toMatchObject({ width: 640, height: 480, clamped: true });
    const big = chooseGuestScreen(room(9000, 5000, 1));
    expect(big).toMatchObject({ width: 7680, height: 4320, clamped: true });
    expect(big.scale).toBe(1);
  });
  it('a pointer at a canvas edge lands on the first and the last guest pixel', () => {
    const g = chooseGuestScreen(room(1836.4, 1048.7, 1.7647));
    const w = g.width * g.scale, h = g.height * g.scale;
    const view = { left: 0, top: 44, width: w, height: h };
    expect(clientToScreen(0, 44, view, g)).toMatchObject({ x: 0, y: 0, inside: true });
    expect(clientToScreen(w - 0.01, 44 + h - 0.01, view, g)).toMatchObject({ x: g.width - 1, y: g.height - 1, inside: true });
  });
});

describe('RoomDebouncer: the engine hears about the new room once the room has been still', () => {
  const fake = () => {
    let now = 0, id = 0;
    const timers = new Map<number, { at: number; fn: () => void }>();
    return {
      set: (fn: () => void, ms: number): unknown => { timers.set(++id, { at: now + ms, fn }); return id; },
      clear: (h: unknown): void => { timers.delete(h as number); },
      advance: (ms: number): void => { now += ms; for (const [k, t] of [...timers]) if (t.at <= now) { timers.delete(k); t.fn(); } },
      count: (): number => timers.size,
    };
  };
  it('a drag of 20 sizes calls back once, with the last, 300 ms after the last', () => {
    const f = fake();
    const seen: Room[] = [];
    const d = new RoomDebouncer((r) => seen.push(r), f);
    for (let i = 0; i < 20; i++) { d.push({ width: 1000 + i * 10, height: 800, dpr: 1 }); f.advance(50); }
    expect(seen).toEqual([]);
    f.advance(249);
    expect(seen).toEqual([]);
    f.advance(1);
    expect(seen).toEqual([{ width: 1190, height: 800, dpr: 1 }]);
    expect(d.pending).toBe(false);
  });
  it('the same room again after it settled does nothing; a new one starts a new wait; cancel stops it', () => {
    const f = fake();
    const seen: Room[] = [];
    const d = new RoomDebouncer((r) => seen.push(r), f, 100);
    d.push({ width: 1, height: 2, dpr: 1 });
    f.advance(100);
    d.push({ width: 1, height: 2, dpr: 1 });
    expect(f.count()).toBe(0);
    d.push({ width: 3, height: 2, dpr: 1 });
    expect(f.count()).toBe(1);
    d.cancel();
    f.advance(1000);
    expect(seen).toEqual([{ width: 1, height: 2, dpr: 1 }]);
  });
  it('a change of the device pixel ratio alone counts as a new room', () => {
    const f = fake();
    const seen: Room[] = [];
    const d = new RoomDebouncer((r) => seen.push(r), f, 10);
    d.push({ width: 5, height: 5, dpr: 1 });
    f.advance(10);
    d.push({ width: 5, height: 5, dpr: 2 });
    f.advance(10);
    expect(seen.map((r) => r.dpr)).toEqual([1, 2]);
  });
});

describe('the beacon block guestScreen (additive in version 3)', () => {
  const ok: GuestScreenInfo = { mode: 'fit', source: 'default', width: 1836, height: 1056, reported: true, dpi: null, roomWidth: 1836, roomHeight: 1056, dpr: 1.7647, follow: 'live' };
  const problems = (g: unknown): string[] => beaconProblems({ v: 3, sid: '0123456789ab', guestScreen: g }).filter((p) => p.startsWith('guestScreen'));
  it('the block the page writes is accepted; absent and null are accepted', () => {
    expect(problems(ok)).toEqual([]);
    expect(problems({ ...ok, roomWidth: null, roomHeight: null })).toEqual([]);
    // the server still accepts the other values of the records stored earlier
    expect(problems({ ...ok, dpi: 169, mode: 'fit-device', source: 'stored', follow: 'reload' })).toEqual([]);
    expect(beaconProblems({ v: 3, sid: '0123456789ab' }).filter((p) => p.startsWith('guestScreen'))).toEqual([]);
    expect(problems(null)).toEqual([]);
  });
  it('a block that is not what the page writes is refused field by field', () => {
    expect(problems({ ...ok, mode: 'zoom' }).join()).toMatch(/guestScreen\.mode/);
    expect(problems({ ...ok, source: 'cookie' }).join()).toMatch(/guestScreen\.source/);
    expect(problems({ ...ok, follow: 'now' }).join()).toMatch(/guestScreen\.follow/);
    expect(problems({ ...ok, width: 0 }).join()).toMatch(/guestScreen\.width/);
    expect(problems({ ...ok, height: 1.5 }).join()).toMatch(/guestScreen\.height/);
    expect(problems({ ...ok, dpi: 95 }).join()).toMatch(/guestScreen\.dpi/);
    expect(problems({ ...ok, dpr: 0 }).join()).toMatch(/guestScreen\.dpr/);
    expect(problems({ ...ok, reported: 'yes' }).join()).toMatch(/guestScreen\.reported/);
    expect(problems('x').join()).toMatch(/guestScreen: not an object/);
  });
});

describe('what the engine reports about its screen (fle_av_poll words 12 and 13, decision 0212)', () => {
  const buf = (w: number, h: number): ArrayBuffer => {
    const b = new ArrayBuffer(AV_HEADER_BYTES + 32 * AV_WINDOW_BYTES);
    const dv = new DataView(b);
    dv.setUint32(0, AV_MAGIC, true); dv.setUint32(4, AV_VERSION, true); dv.setUint32(8, AV_WINDOW_BYTES, true); dv.setUint32(12, 32, true);
    dv.setUint32(48, w, true); dv.setUint32(52, h, true);
    return b;
  };
  it('the page reads the width at byte 48 and the height at byte 52; zero is a build that does not report', () => {
    expect(readAppView(buf(3239, 1863), 0)).toMatchObject({ screenW: 3239, screenH: 1863 });
    expect(readAppView(buf(0, 0), 0)).toMatchObject({ screenW: 0, screenH: 0 });
  });
  it('the C header puts screen_w and screen_h at those offsets', () => {
    expect(rt('fle_appview.c')).toContain('offsetof(fle_av_state, screen_w) == 48 && offsetof(fle_av_state, screen_h) == 52');
  });
  it('the one mode the page writes is accepted by the validator', () => {
    for (const mode of ['fit']) expect(beaconProblems({ v: 3, sid: '0123456789ab', guestScreen: { mode, source: 'default', width: 100, height: 100, reported: true, dpi: null, roomWidth: null, roomHeight: null, dpr: 1, follow: 'reload' } }).filter((p) => p.startsWith('guestScreen'))).toEqual([]);
  });
});
