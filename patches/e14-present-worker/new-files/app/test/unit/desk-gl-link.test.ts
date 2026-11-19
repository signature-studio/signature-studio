// Decision 0311: the presenter in the present Worker. The description is read from the link's slot (a SharedArrayBuffer of its own) and the pixels from the engine's memory (another buffer); a resync is only a request there
// ('deferred'), so a present that found the screen size changed must not look again at once.
import { describe, expect, it } from 'vitest';
import { GlDeskPresenter, SCENE_FLAG_CHANGED, SCENE_FLAG_WHOLE, SCENE_HEADER_BYTES, SCENE_MAGIC, SCENE_MAX_UPDATES, SCENE_MAX_WINDOWS, SCENE_UPD_BYTES, SCENE_VERSION, SCENE_WIN_BYTES } from '../../src/engine/desk-gl.ts';
import { HEADER_BYTES, newDeskLink } from '../../src/engine/desk-link.ts';

type Call = [string, ...unknown[]];
function fakeGl(): { gl: unknown; calls: Call[] } {
  const calls: Call[] = [];
  let tex = 0;
  const consts: Record<string, number> = { VERTEX_SHADER: 1, FRAGMENT_SHADER: 2, COMPILE_STATUS: 3, LINK_STATUS: 4, TEXTURE_2D: 10, RGBA: 11, RGBA8: 12, UNSIGNED_BYTE: 13, TEXTURE_MIN_FILTER: 14, TEXTURE_MAG_FILTER: 15, TEXTURE_WRAP_S: 16, TEXTURE_WRAP_T: 17, NEAREST: 18, CLAMP_TO_EDGE: 19, UNPACK_ROW_LENGTH: 20, DEPTH_TEST: 21, DITHER: 22, BLEND: 23, ONE: 24, ONE_MINUS_SRC_ALPHA: 25, TRIANGLE_STRIP: 26, COLOR_BUFFER_BIT: 27 };
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

function writeScene(buf: ArrayBufferLike, ptr: number, o: { w: number; h: number; flags: number; wins: { lid: number; x: number; y: number; w: number; h: number; bits: number; stride: number; fresh?: boolean }[]; upds: { lid: number; x0: number; y0: number; x1: number; y1: number }[] }): void {
  const dv = new DataView(buf, ptr);
  dv.setUint32(0, SCENE_MAGIC, true); dv.setUint32(4, SCENE_VERSION, true); dv.setUint32(8, o.w, true); dv.setUint32(12, o.h, true); dv.setUint32(16, 1, true);
  dv.setUint32(20, o.wins.length, true); dv.setUint32(24, o.upds.length, true); dv.setUint32(28, o.flags, true); dv.setUint32(32, 0, true);
  o.wins.forEach((w, i) => {
    const b = SCENE_HEADER_BYTES + i * SCENE_WIN_BYTES;
    [w.lid, w.x, w.y, w.w, w.h, w.bits, w.stride, 0, 255, 0, w.fresh ? 1 : 0, 0, 0, 0, 0, 0, 0, 0, w.w, w.h].forEach((v, k) => dv.setInt32(b + k * 4, v, true));
  });
  const ub = SCENE_HEADER_BYTES + SCENE_MAX_WINDOWS * SCENE_WIN_BYTES;
  o.upds.forEach((u, i) => [u.lid, u.x0, u.y0, u.x1, u.y1].forEach((v, k) => dv.setInt32(ub + i * SCENE_UPD_BYTES + k * 4, v, true)));
  expect(ub + SCENE_MAX_UPDATES * SCENE_UPD_BYTES).toBe(6464);
}

function setup(): { p: GlDeskPresenter; link: SharedArrayBuffer; engineMem: SharedArrayBuffer; calls: Call[]; surface: { width: number; height: number }; api: { modes: number[]; resyncs: number } } {
  const link = newDeskLink(), engineMem = new SharedArrayBuffer(1 << 20);
  const { gl, calls } = fakeGl();
  const surface = { width: 1920, height: 1080, getContext: () => gl as never };
  const api = { modes: [] as number[], resyncs: 0, deferred: true, scene: () => HEADER_BYTES, sceneMode(on: number) { this.modes.push(on); }, resync() { this.resyncs++; }, describe: () => '' };
  const p = new GlDeskPresenter(surface as never, api, () => engineMem, () => 0, () => link);
  return { p, link, engineMem, calls, surface, api };
}

describe('GlDeskPresenter with the description in the link', () => {
  it('reads the description from the link and uploads the pixels from the engine memory', () => {
    const t = setup();
    writeScene(t.link, HEADER_BYTES, { w: 1920, h: 1080, flags: SCENE_FLAG_CHANGED | SCENE_FLAG_WHOLE, wins: [{ lid: 3, x: 10, y: 20, w: 100, h: 50, bits: 0x8000, stride: 448, fresh: true }], upds: [{ lid: 3, x0: 0, y0: 0, x1: 100, y1: 50 }] });
    expect(t.p.present()).toBe(1);
    const up = t.calls.find((c) => c[0] === 'texSubImage2D') as Call;
    expect(up).toBeDefined();
    expect(up[5]).toBe(100);                    // width of the rectangle
    expect(up[6]).toBe(50);                     // height
  });
  it('the pixel source is the engine memory (not the link) at the surface address', () => {
    const t = setup();
    writeScene(t.link, HEADER_BYTES, { w: 1920, h: 1080, flags: SCENE_FLAG_CHANGED | SCENE_FLAG_WHOLE, wins: [{ lid: 3, x: 0, y: 0, w: 64, h: 32, bits: 0x4000, stride: 256, fresh: true }], upds: [{ lid: 3, x0: 0, y0: 0, x1: 64, y1: 32 }] });
    t.p.present();
    const up = t.calls.find((c) => c[0] === 'texSubImage2D') as Call;
    const src = up.find((x) => x instanceof Uint8Array) as Uint8Array;
    expect(src.buffer).toBe(t.engineMem);
    expect(up[up.length - 1]).toBe(0x4000);     // the offset into the engine memory
  });
  it('a screen size that changed asks for a resync and does not look again at once (the new picture comes with a later description)', () => {
    const t = setup();
    writeScene(t.link, HEADER_BYTES, { w: 800, h: 600, flags: SCENE_FLAG_CHANGED, wins: [], upds: [] });
    expect(t.p.present()).toBe(0);
    expect(t.api.resyncs).toBe(1);
    expect(t.surface.width).toBe(800);
  });
});
