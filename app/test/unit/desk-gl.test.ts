// Decision 0240 (gpu1): the WebGL2 presenter (src/engine/desk-gl.ts). The layout it reads is pinned to lift/runtime/fle_desk.h; the presenter is run against a RECORDING fake of WebGL2 and a fake engine memory, so that what it
// asks of GL is checked: where each pixel comes from (the engine's memory, with the surface's stride), which texture, which window is drawn in which order with which state, and what it does when nothing changed.
import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { GL_MIN_TEXTURE_SIZE, GlDeskPresenter, webgl2Usable, SCENE_FLAG_CHANGED, SCENE_FLAG_WHOLE, SCENE_GLASS, SCENE_HEADER_BYTES, SCENE_KEY, SCENE_LAYERED, SCENE_MAGIC, SCENE_MAX_UPDATES, SCENE_MAX_WINDOWS, SCENE_PREMUL, SCENE_UPD_BYTES, SCENE_VERSION, SCENE_WIN_BYTES, readScene, type SceneApi } from '../../src/engine/desk-gl.ts';

const REPO = process.env.APP_REPO_ROOT || path.resolve(__dirname, '../../..');
const h = fs.readFileSync(path.join(REPO, 'lift/runtime/fle_desk.h'), 'utf8');
const def = (n: string): number => Number(new RegExp(`#define ${n}\\s+(0x[0-9a-fA-F]+|\\d+)u?`).exec(h)?.[1]);

describe('the layout is the one of lift/runtime/fle_desk.h (read from the file)', () => {
  it('magic, version, flags and the layered words', () => {
    expect(def('FLE_SCENE_MAGIC')).toBe(SCENE_MAGIC);
    expect(def('FLE_SCENE_VERSION')).toBe(SCENE_VERSION);
    expect(def('FLE_SCENE_FLAG_WHOLE')).toBe(SCENE_FLAG_WHOLE);
    expect(def('FLE_SCENE_FLAG_CHANGED')).toBe(SCENE_FLAG_CHANGED);
    expect(def('FLE_SCENE_LAYERED')).toBe(SCENE_LAYERED);
    expect(def('FLE_SCENE_KEY')).toBe(SCENE_KEY);
    expect(def('FLE_SCENE_PREMUL')).toBe(SCENE_PREMUL);
    expect(def('FLE_SCENE_GLASS')).toBe(SCENE_GLASS);
  });
  it('the structs declare the fields in the order the reader assumes', () => {
    const flat = (name: string): string => (new RegExp(`typedef struct \\{([^}]*)\\} ${name};`).exec(h)?.[1] ?? '').replace(/\/\*.*?\*\//g, '').replace(/\s+/g, ' ');
    expect(flat('fle_scene_win')).toContain('uint32_t lid, x, y, w, h; uint32_t bits, stride; uint32_t layer, alpha, key; uint32_t fresh; uint32_t reserved; int32_t gx0, gy0, gx1, gy1; int32_t cl, ct, cr, cb;');
    expect(flat('fle_scene_upd')).toContain('uint32_t lid; int32_t x0, y0, x1, y1;');
    expect(flat('fle_scene')).toContain('uint32_t magic, version, width, height, gen, n_windows, n_updates, flags, background; uint32_t reserved[7]; fle_scene_win win[FLE_SCENE_MAX_WINDOWS]; fle_scene_upd upd[FLE_SCENE_MAX_UPDATES];');
    expect(SCENE_HEADER_BYTES).toBe(16 * 4);
    expect(SCENE_WIN_BYTES).toBe(20 * 4);
    expect(SCENE_UPD_BYTES).toBe(5 * 4);
    expect(SCENE_MAX_WINDOWS).toBe(64);
    expect(SCENE_MAX_UPDATES).toBe(64);
  });
  it('the runtime says: no pixel is converted, the surface is B, G, R, X', () => {
    expect(h).toMatch(/B, G, R, X bytes, top-down/);
  });
});

interface W { lid: number; x: number; y: number; w: number; h: number; bits: number; stride: number; layer?: number; alpha?: number; key?: number; fresh?: boolean; inner?: number[]; region?: number[] }
interface U { lid: number; x0: number; y0: number; x1: number; y1: number }
function writeScene(mem: ArrayBuffer, ptr: number, o: { w?: number; h?: number; flags: number; wins: W[]; upds: U[]; gen?: number; bg?: number }): void {
  const dv = new DataView(mem, ptr);
  dv.setUint32(0, SCENE_MAGIC, true); dv.setUint32(4, SCENE_VERSION, true); dv.setUint32(8, o.w ?? 1920, true); dv.setUint32(12, o.h ?? 1080, true); dv.setUint32(16, o.gen ?? 1, true);
  dv.setUint32(20, o.wins.length, true); dv.setUint32(24, o.upds.length, true); dv.setUint32(28, o.flags, true); dv.setUint32(32, o.bg ?? (5 | (7 << 8) | (10 << 16)), true);
  o.wins.forEach((w, i) => {
    const b = SCENE_HEADER_BYTES + i * SCENE_WIN_BYTES;
    [w.lid, w.x, w.y, w.w, w.h, w.bits, w.stride, w.layer ?? 0, w.alpha ?? 255, w.key ?? 0, w.fresh ? 1 : 0, 0, ...(w.inner ?? [0, 0, 0, 0]), ...(w.region ?? [0, 0, w.w, w.h])].forEach((v, k) => dv.setInt32(b + k * 4, v, true));
  });
  const ub = SCENE_HEADER_BYTES + SCENE_MAX_WINDOWS * SCENE_WIN_BYTES;
  o.upds.forEach((u, i) => [u.lid, u.x0, u.y0, u.x1, u.y1].forEach((v, k) => dv.setInt32(ub + i * SCENE_UPD_BYTES + k * 4, v, true)));
}

describe('readScene', () => {
  it('reads the header, the windows and the updates; refuses another magic or version', () => {
    const mem = new ArrayBuffer(1 << 16);
    writeScene(mem, 256, { flags: SCENE_FLAG_CHANGED, wins: [{ lid: 7, x: -5, y: -6, w: 100, h: 50, bits: 0x9000, stride: 448, layer: SCENE_LAYERED | SCENE_KEY, alpha: 255, key: 0x123456, fresh: true }], upds: [{ lid: 7, x0: 1, y0: 2, x1: 30, y1: 40 }] });
    const s = readScene(mem, 256);
    expect(s).not.toBeNull();
    expect(s?.windows[0]).toEqual({ lid: 7, x: -5, y: -6, w: 100, h: 50, bits: 0x9000, stride: 448, layer: SCENE_LAYERED | SCENE_KEY, alpha: 255, key: 0x123456, fresh: true, gx0: 0, gy0: 0, gx1: 0, gy1: 0, cl: 0, ct: 0, cr: 100, cb: 50 });
    expect(s?.updates).toEqual([{ lid: 7, x0: 1, y0: 2, x1: 30, y1: 40 }]);
    expect(s?.background).toBe(5 | (7 << 8) | (10 << 16));
    new DataView(mem, 256).setUint32(0, 1, true);
    expect(readScene(mem, 256)).toBeNull();
    expect(readScene(mem, 0)).toBeNull();
  });
});

// ---- a recording WebGL2 ----
type Call = [string, ...unknown[]];
function fakeGl(): { gl: unknown; calls: Call[] } {
  const calls: Call[] = [];
  let tex = 0;
  const consts: Record<string, number> = { VERTEX_SHADER: 1, FRAGMENT_SHADER: 2, COMPILE_STATUS: 3, LINK_STATUS: 4, TEXTURE_2D: 10, RGBA: 11, RGBA8: 12, UNSIGNED_BYTE: 13, TEXTURE_MIN_FILTER: 14, TEXTURE_MAG_FILTER: 15, NEAREST: 16, TEXTURE_WRAP_S: 17, TEXTURE_WRAP_T: 18, CLAMP_TO_EDGE: 19, UNPACK_ROW_LENGTH: 20, TRIANGLE_STRIP: 21, COLOR_BUFFER_BIT: 22, BLEND: 23, DEPTH_TEST: 24, DITHER: 25, ONE: 26, ONE_MINUS_SRC_ALPHA: 27, TEXTURE0: 28 };
  const gl = new Proxy({}, {
    get(_t, name: string) {
      if (name in consts) return consts[name];
      return (...a: unknown[]) => {
        calls.push([name, ...a]);
        if (name === 'getShaderParameter' || name === 'getProgramParameter') return true;
        if (name === 'createTexture') return { id: ++tex };
        if (name === 'createShader' || name === 'createProgram') return {};
        if (name === 'getUniformLocation') return { name: a[1] };
        return undefined;
      };
    },
  });
  return { gl, calls };
}

function setup(): { p: GlDeskPresenter; mem: ArrayBuffer; calls: Call[]; surface: { width: number; height: number }; set(o: { flags: number; wins: W[]; upds: U[]; w?: number; h?: number }): void; api: SceneApi & { modes: number[]; resyncs: number } } {
  const mem = new ArrayBuffer(1 << 20);
  const { gl, calls } = fakeGl();
  const surface = { width: 1920, height: 1080, getContext: () => gl as never };
  const api = { modes: [] as number[], resyncs: 0, scene: () => 4096, sceneMode(on: number) { this.modes.push(on); }, resync() { this.resyncs++; }, describe: () => '' };
  const p = new GlDeskPresenter(surface as never, api, () => mem);
  return { p, mem, calls, surface, set: (o) => writeScene(mem, 4096, o), api };
}
const names = (calls: Call[]): string[] => calls.map((c) => c[0]);

describe('GlDeskPresenter', () => {
  it('switches the runtime to the scene only after the GL context and the program exist', () => {
    const t = setup();
    expect(t.api.modes).toEqual([1]);
    expect(t.calls.some((c) => c[0] === 'linkProgram')).toBe(true);
  });
  it('a new window: a texture of its size, its whole surface uploaded straight from the engine memory with the surface stride, one quad drawn', () => {
    const t = setup();
    t.set({ flags: SCENE_FLAG_CHANGED | SCENE_FLAG_WHOLE, wins: [{ lid: 3, x: 10, y: 20, w: 100, h: 50, bits: 0x8000, stride: 448, fresh: true }], upds: [{ lid: 3, x0: 0, y0: 0, x1: 100, y1: 50 }] });
    t.calls.length = 0;
    expect(t.p.present()).toBe(1);
    const n = names(t.calls);
    expect(n).toContain('texStorage2D');
    const st = t.calls.find((c) => c[0] === 'texStorage2D') as Call;
    expect(st.slice(1)).toEqual([10, 1, 12, 100, 50]);
    const up = t.calls.find((c) => c[0] === 'texSubImage2D') as Call;
    expect(up.slice(1, 10)).toEqual([10, 0, 0, 0, 100, 50, 11, 13, expect.anything()]);
    expect(up[10]).toBe(0x8000);                                                  // the byte offset of (0, 0) in the engine's memory
    expect(t.calls.find((c) => c[0] === 'pixelStorei' && c[1] === 20)?.[2]).toBe(112); // UNPACK_ROW_LENGTH = stride / 4
    expect(t.calls.filter((c) => c[0] === 'drawArrays')).toHaveLength(1);
    const rect = t.calls.find((c) => c[0] === 'uniform4f' && (c[1] as { name: string }).name === 'u_rect') as Call;
    expect(rect.slice(2)).toEqual([10, 20, 100, 50]);
    expect(t.p.stats.uploads).toBe(1);
    expect(t.p.stats.pixels).toBe(5000);
  });
  it('a written rectangle: the offset is that of its first pixel; nothing changed: no GL call at all', () => {
    const t = setup();
    t.set({ flags: SCENE_FLAG_CHANGED | SCENE_FLAG_WHOLE, wins: [{ lid: 3, x: 10, y: 20, w: 100, h: 50, bits: 0x8000, stride: 448, fresh: true }], upds: [{ lid: 3, x0: 0, y0: 0, x1: 100, y1: 50 }] });
    t.p.present();
    t.set({ flags: 0, wins: [{ lid: 3, x: 10, y: 20, w: 100, h: 50, bits: 0x8000, stride: 448 }], upds: [{ lid: 3, x0: 7, y0: 9, x1: 17, y1: 12 }] });
    t.calls.length = 0;
    expect(t.p.present()).toBe(1);
    const up = t.calls.find((c) => c[0] === 'texSubImage2D') as Call;
    expect(up.slice(3, 7)).toEqual([7, 9, 10, 3]);
    expect(up[10]).toBe(0x8000 + 9 * 448 + 7 * 4);
    expect(names(t.calls)).not.toContain('texStorage2D');
    t.set({ flags: 0, wins: [{ lid: 3, x: 10, y: 20, w: 100, h: 50, bits: 0x8000, stride: 448 }], upds: [] });
    t.calls.length = 0;
    expect(t.p.present()).toBe(0);
    expect(t.calls).toEqual([]);
  });
  it('a move or a restack: the quads are drawn again in the new order and no pixel is uploaded (returns 1: the picture changed)', () => {
    const t = setup();
    const a: W = { lid: 3, x: 10, y: 20, w: 100, h: 50, bits: 0x8000, stride: 400, fresh: true }, b: W = { lid: 4, x: 50, y: 40, w: 80, h: 60, bits: 0x20000, stride: 320, fresh: true };
    t.set({ flags: SCENE_FLAG_CHANGED | SCENE_FLAG_WHOLE, wins: [a, b], upds: [{ lid: 3, x0: 0, y0: 0, x1: 100, y1: 50 }, { lid: 4, x0: 0, y0: 0, x1: 80, y1: 60 }] });
    t.p.present();
    t.set({ flags: SCENE_FLAG_CHANGED, wins: [{ ...b, fresh: false }, { ...a, x: 300, y: 200, fresh: false }], upds: [] });
    t.calls.length = 0;
    expect(t.p.present()).toBe(1);
    expect(names(t.calls)).not.toContain('texSubImage2D');
    const rects = t.calls.filter((c) => c[0] === 'uniform4f' && (c[1] as { name: string }).name === 'u_rect').map((c) => (c as Call).slice(2));
    expect(rects).toEqual([[50, 40, 80, 60], [300, 200, 100, 50]]);
    expect(t.calls.filter((c) => c[0] === 'drawArrays')).toHaveLength(2);
  });
  it('a window that is gone loses its texture; a window of another size gets a new one', () => {
    const t = setup();
    const a: W = { lid: 3, x: 0, y: 0, w: 100, h: 50, bits: 0x8000, stride: 400, fresh: true };
    t.set({ flags: SCENE_FLAG_CHANGED | SCENE_FLAG_WHOLE, wins: [a, { lid: 4, x: 0, y: 0, w: 10, h: 10, bits: 0x20000, stride: 40, fresh: true }], upds: [{ lid: 3, x0: 0, y0: 0, x1: 100, y1: 50 }, { lid: 4, x0: 0, y0: 0, x1: 10, y1: 10 }] });
    t.p.present();
    t.set({ flags: SCENE_FLAG_CHANGED, wins: [{ ...a, w: 120, h: 60, stride: 480, fresh: true }], upds: [{ lid: 3, x0: 0, y0: 0, x1: 120, y1: 60 }] });
    t.calls.length = 0;
    t.p.present();
    expect(t.calls.filter((c) => c[0] === 'deleteTexture')).toHaveLength(2);   // window 4 (gone) and the old texture of window 3
    expect(t.calls.filter((c) => c[0] === 'texStorage2D')).toHaveLength(1);
  });
  it('layering: an opaque window draws with mode 0, a constant alpha or a colour key with mode 1, a per-pixel alpha with mode 2; nothing looks at a class or a title', () => {
    const t = setup();
    const base = { x: 0, y: 0, w: 4, h: 4, bits: 0x8000, stride: 16, fresh: true };
    t.set({ flags: SCENE_FLAG_CHANGED | SCENE_FLAG_WHOLE, wins: [{ ...base, lid: 1 }, { ...base, lid: 2, layer: SCENE_LAYERED | 1, alpha: 128 }, { ...base, lid: 3, layer: SCENE_LAYERED | SCENE_KEY, key: 0x0000ff }, { ...base, lid: 4, layer: SCENE_LAYERED | SCENE_PREMUL | 1, alpha: 200 }], upds: [] });
    t.calls.length = 0;
    t.p.present();
    const modes = t.calls.filter((c) => c[0] === 'uniform1i' && (c[1] as { name: string }).name === 'u_mode').map((c) => (c as Call)[2]);
    expect(modes).toEqual([0, 1, 1, 2]);
    const alphas = t.calls.filter((c) => c[0] === 'uniform1f' && (c[1] as { name: string }).name === 'u_alpha').map((c) => (c as Call)[2]);
    expect(alphas).toEqual([1, 128 / 255, 1, 200 / 255]);
  });
  it('a window with an extended frame draws with mode 3 and its inner box and region box reach the shader; an ordinary window has a region box of its whole size', () => {
    const t = setup();
    const base = { x: 0, y: 0, w: 40, h: 30, bits: 0x8000, stride: 160, fresh: true };
    t.set({ flags: SCENE_FLAG_CHANGED | SCENE_FLAG_WHOLE, wins: [{ ...base, lid: 1 }, { ...base, lid: 2, layer: SCENE_GLASS, inner: [4, 6, 36, 24], region: [2, 3, 38, 28] }], upds: [] });
    t.calls.length = 0;
    t.p.present();
    const uni = (n: string): unknown[][] => t.calls.filter((c) => c[0].startsWith('uniform') && (c[1] as { name: string }).name === n).map((c) => (c as Call).slice(2));
    expect(uni('u_mode')).toEqual([[0], [3]]);
    expect(uni('u_inner')[1]).toEqual([4, 6, 36, 24]);
    expect(uni('u_region')).toEqual([[0, 0, 40, 30], [2, 3, 38, 28]]);
    expect(uni('u_size')).toEqual([[40, 30], [40, 30]]);
  });
  it('a canvas of another size than the screen is resized and the runtime is asked for everything again', () => {
    const t = setup();
    t.set({ flags: SCENE_FLAG_CHANGED | SCENE_FLAG_WHOLE, w: 1600, h: 900, wins: [], upds: [] });
    t.p.present();
    expect(t.surface.width).toBe(1600);
    expect(t.surface.height).toBe(900);
    expect(t.api.resyncs).toBe(1);
  });
  it('a lost context draws nothing; a restored one builds the program again, forgets every texture and asks the runtime for everything', () => {
    const t = setup();
    const a: W = { lid: 3, x: 0, y: 0, w: 100, h: 50, bits: 0x8000, stride: 400, fresh: true };
    t.set({ flags: SCENE_FLAG_CHANGED | SCENE_FLAG_WHOLE, wins: [a], upds: [{ lid: 3, x0: 0, y0: 0, x1: 100, y1: 50 }] });
    t.p.present();
    t.p.setLost(true);
    t.calls.length = 0;
    t.set({ flags: SCENE_FLAG_CHANGED, wins: [{ ...a, fresh: false }], upds: [{ lid: 3, x0: 1, y0: 1, x1: 5, y1: 5 }] });
    expect(t.p.present()).toBe(0);
    expect(t.calls).toEqual([]);
    const resyncs = t.api.resyncs;
    t.p.setLost(false);
    expect(t.api.resyncs).toBe(resyncs + 1);
    expect(names(t.calls)).toContain('linkProgram');
    t.set({ flags: SCENE_FLAG_CHANGED | SCENE_FLAG_WHOLE, wins: [{ ...a, fresh: true }], upds: [{ lid: 3, x0: 0, y0: 0, x1: 100, y1: 50 }] });
    t.calls.length = 0;
    expect(t.p.present()).toBe(1);
    expect(names(t.calls)).toContain('texStorage2D');
    expect(names(t.calls)).toContain('texSubImage2D');
  });
});

describe('webgl2Usable (the one condition for choosing WebGL2 over the 2D picture)', () => {
  it('asks the platform for a context without a major performance caveat (a platform that answers null for it shows the 2D picture; the headless Chrome and Firefox of decision 0240 do not answer null)', () => {
    const seen: unknown[] = [];
    expect(webgl2Usable(() => ({ getContext: (_k: string, a?: unknown) => { seen.push(a); return null; } }))).toBe(false);
    expect(seen).toEqual([{ failIfMajorPerformanceCaveat: true }]);
  });
  const ctx = (max: number, lose?: () => void): unknown => ({ MAX_TEXTURE_SIZE: 0x0d33, getParameter: (p: number) => (p === 0x0d33 ? max : undefined), getExtension: (n: string) => (n === 'WEBGL_lose_context' ? { loseContext: lose ?? (() => {}) } : null) });
  it('a context whose largest texture holds the biggest screen: usable, and the scratch context is given up', () => {
    let lost = 0;
    expect(webgl2Usable(() => ({ getContext: () => ctx(16384, () => lost++) as never }))).toBe(true);
    expect(lost).toBe(1);
  });
  it('no WebGL2, a texture limit below the floor, or a throwing browser: not usable (the 2D picture is shown)', () => {
    expect(webgl2Usable(() => ({ getContext: () => null }))).toBe(false);
    expect(webgl2Usable(() => ({ getContext: () => ctx(GL_MIN_TEXTURE_SIZE - 1) as never }))).toBe(false);
    expect(webgl2Usable(() => ({ getContext: () => ctx(GL_MIN_TEXTURE_SIZE) as never }))).toBe(true);
    expect(webgl2Usable(() => { throw new Error('no OffscreenCanvas'); })).toBe(false);
  });
});
