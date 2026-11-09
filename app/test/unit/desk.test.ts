import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { DESK_FLAG_WHOLE, DESK_HEADER_BYTES, DESK_MAGIC, DESK_MAX_RECTS, DESK_RECT_BYTES, DESK_VERSION, DeskPresenter, readDeskState, type DeskApi } from '../../src/engine/desk.ts';

const REPO = process.env.APP_REPO_ROOT || path.resolve(__dirname, '../../..');
const rt = (f: string): string => fs.readFileSync(path.join(REPO, 'lift/runtime', f), 'utf8');

describe('the layout is the one of lift/runtime/fle_desk.h (read from the file)', () => {
  const h = rt('fle_desk.h');
  const def = (n: string): number => Number(new RegExp(`#define ${n}\\s+(0x[0-9a-fA-F]+|\\d+)u?`).exec(h)?.[1]);
  it('magic, version and the number of rectangles', () => {
    expect(def('FLE_DESK_MAGIC')).toBe(DESK_MAGIC);
    expect(def('FLE_DESK_VERSION')).toBe(DESK_VERSION);
    expect(def('FLE_DESK_MAX_RECTS')).toBe(DESK_MAX_RECTS);
  });
  it('the struct declares the fields in the order the reader assumes (16 header words, then rectangles of 4 words)', () => {
    const st = /typedef struct \{([^}]*)\} fle_desk_state;/.exec(h)?.[1] ?? '';
    const flat = st.replace(/\/\*.*?\*\//g, '').replace(/\s+/g, ' ');
    expect(flat).toContain('uint32_t magic, version; uint32_t width, height, stride; uint32_t pixels; uint32_t gen; uint32_t n_rects; uint32_t n_windows; uint32_t flags; uint32_t reserved[6]; fle_desk_rect rects[FLE_DESK_MAX_RECTS];');
    expect(DESK_HEADER_BYTES).toBe(16 * 4);
    expect(DESK_RECT_BYTES).toBe(16);
    const rect = /typedef struct \{([^}]*)\} fle_desk_rect;/.exec(h)?.[1] ?? '';
    expect(rect.replace(/\/\*.*?\*\//g, '').replace(/\s+/g, ' ')).toContain('int32_t x0, y0, x1, y1;');
  });
  it('the picture is RGBA bytes: the header says so and the compositor converts BGRX to R, G, B, 255', () => {
    expect(h).toMatch(/R, G, B, A \(A = 255\)/);
    expect(rt('fle_desk.c')).toContain('bgrx_to_rgba');
  });
});

function state(mem: ArrayBuffer, ptr: number, o: { w?: number; h?: number; pixels?: number; gen?: number; flags?: number; rects?: number[][] }): void {
  const dv = new DataView(mem, ptr);
  dv.setUint32(0, DESK_MAGIC, true); dv.setUint32(4, DESK_VERSION, true);
  dv.setUint32(8, o.w ?? 1920, true); dv.setUint32(12, o.h ?? 1080, true); dv.setUint32(16, (o.w ?? 1920) * 4, true); dv.setUint32(20, o.pixels ?? 0x1000, true);
  dv.setUint32(24, o.gen ?? 1, true); dv.setUint32(28, o.rects?.length ?? 0, true); dv.setUint32(32, 3, true); dv.setUint32(36, o.flags ?? 0, true);
  (o.rects ?? []).forEach((r, i) => r.forEach((v, k) => dv.setInt32(DESK_HEADER_BYTES + i * DESK_RECT_BYTES + k * 4, v, true)));
}

describe('readDeskState', () => {
  it('reads the header and the rectangles', () => {
    const mem = new ArrayBuffer(4096);
    state(mem, 256, { gen: 7, flags: DESK_FLAG_WHOLE, rects: [[0, 0, 1920, 1080], [10, 20, 30, 40]] });
    expect(readDeskState(mem, 256)).toEqual({ width: 1920, height: 1080, stride: 7680, pixels: 0x1000, gen: 7, nWindows: 3, whole: true, rects: [{ x0: 0, y0: 0, x1: 1920, y1: 1080 }, { x0: 10, y0: 20, x1: 30, y1: 40 }] });
  });
  it('refuses a pointer that is not a state (wrong magic, null)', () => {
    const mem = new ArrayBuffer(4096);
    expect(readDeskState(mem, 0)).toBeNull();
    expect(readDeskState(mem, 256)).toBeNull();
  });
  it('never reads more rectangles than the structure holds', () => {
    const mem = new ArrayBuffer(4096);
    state(mem, 0x100, { rects: [[0, 0, 1, 1]] });
    new DataView(mem, 0x100).setUint32(28, 999, true);
    expect(readDeskState(mem, 0x100)?.rects.length).toBe(DESK_MAX_RECTS);
  });
});

describe('DeskPresenter', () => {
  function rig(rectsPerCall: number[][][]) {
    const mem = new ArrayBuffer(1 << 16);
    const puts: number[][] = [];
    const surface = { width: 1920, height: 1080, getContext: () => ({ putImageData: (_i: unknown, dx: number, dy: number, sx: number, sy: number, sw: number, sh: number) => { puts.push([dx, dy, sx, sy, sw, sh]); } }) };
    let call = 0, resyncs = 0, buf: ArrayBuffer = mem;
    const api: DeskApi = { compose: () => { state(buf, 0x200, { gen: call + 1, rects: rectsPerCall[call++] ?? [] }); return 0x200; }, resync: () => { resyncs++; }, describe: () => '' };
    const images: ArrayBuffer[] = [];
    const p = new DeskPresenter(surface as never, api, () => buf, (b) => { images.push(b as ArrayBuffer); return {} as ImageData; });
    return { p, puts, images, grow: () => { buf = new ArrayBuffer(1 << 16); }, get resyncs() { return resyncs; } };
  }
  it('puts each changed rectangle with the dirty rectangle of one ImageData over the picture (no pixel is copied by JS)', () => {
    const r = rig([[[0, 0, 1920, 1080]], [[5, 6, 15, 26], [100, 100, 101, 101]], []]);
    expect(r.p.present()).toBe(1);
    expect(r.p.present()).toBe(2);
    expect(r.p.present()).toBe(0);
    expect(r.puts).toEqual([[0, 0, 0, 0, 1920, 1080], [0, 0, 5, 6, 10, 20], [0, 0, 100, 100, 1, 1]]);
    expect(r.p.stats.presents).toBe(2);
    expect(r.p.stats.pixels).toBe(1920 * 1080 + 200 + 1);
  });
  it('follows a change of the screen size: the canvas is resized, the compositor is asked for the whole picture, and the new picture is shown', () => {
    const mem = new ArrayBuffer(1 << 16);
    const puts: number[][] = [];
    const surface = { width: 1920, height: 1080, getContext: () => ({ putImageData: (_i: unknown, dx: number, dy: number, sx: number, sy: number, sw: number, sh: number) => { puts.push([dx, dy, sx, sy, sw, sh]); } }) };
    let resyncs = 0, calls = 0;
    const api: DeskApi = { compose: () => { calls++; state(mem, 0x200, { w: 2060, h: 1264, rects: [[0, 0, 2060, 1264]], flags: DESK_FLAG_WHOLE }); return 0x200; }, resync: () => { resyncs++; }, describe: () => '' };
    const p = new DeskPresenter(surface as never, api, () => mem, () => ({}) as ImageData);
    expect(p.present()).toBe(1);
    expect([surface.width, surface.height]).toEqual([2060, 1264]);
    expect(resyncs).toBe(1);
    expect(calls).toBe(2);
    expect(puts).toEqual([[0, 0, 0, 0, 2060, 1264]]);
  });
  it('an empty rectangle is not put', () => {
    const r = rig([[[5, 5, 5, 9], [7, 7, 9, 7], [1, 1, 3, 3]]]);
    expect(r.p.present()).toBe(3);
    expect(r.puts).toEqual([[0, 0, 1, 1, 2, 2]]);
  });
  it('compose() that grows the engine memory (the old buffer is detached): the report is read from the new buffer, not lost, and nothing throws', () => {
    let buf = new ArrayBuffer(1 << 16);
    const puts: number[][] = [];
    const surface = { width: 1920, height: 1080, getContext: () => ({ putImageData: (_i: unknown, dx: number, dy: number, sx: number, sy: number, sw: number, sh: number) => { puts.push([dx, dy, sx, sy, sw, sh]); } }) };
    let resyncs = 0;
    const api: DeskApi = {
      compose: () => { const old = buf; buf = new ArrayBuffer(1 << 16); structuredClone(old, { transfer: [old] }); state(buf, 0x200, { gen: 1, rects: [[0, 0, 1920, 6], [0, 0, 1, 1080]] }); return 0x200; },
      resync: () => { resyncs++; }, describe: () => '',
    };
    const p = new DeskPresenter(surface as never, api, () => buf, () => ({}) as ImageData);
    expect(() => p.present()).not.toThrow();
    expect(puts).toEqual([[0, 0, 0, 0, 1920, 6], [0, 0, 0, 0, 1, 1080]]);
    expect(resyncs).toBe(0);
  });
  it('a failure to read the report asks for the whole picture next time (the rectangles that were lost are shown again) and still reports the error', () => {
    const buf = new ArrayBuffer(1 << 16);
    const surface = { width: 1920, height: 1080, getContext: () => ({ putImageData: () => {} }) };
    let resyncs = 0;
    const api: DeskApi = { compose: () => 0x7fff0000, resync: () => { resyncs++; }, describe: () => '' };
    const p = new DeskPresenter(surface as never, api, () => buf, () => ({}) as ImageData);
    expect(() => p.present()).toThrow();
    expect(resyncs).toBe(1);
  });
  it('makes the ImageData again only when the engine memory was replaced (memory.grow)', () => {
    const r = rig([[[0, 0, 2, 2]], [[0, 0, 2, 2]], [[0, 0, 2, 2]]]);
    r.p.present(); r.p.present();
    expect(r.images.length).toBe(1);
    r.grow();
    r.p.present();
    expect(r.images.length).toBe(2);
  });
});
