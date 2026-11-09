// Decision 0240 (gpu1): the desktop shown by WebGL2 instead of by putImageData of one composed picture. The worker chooses between the two once, by what the canvas can do (worker.ts makeDeskPresenter); nothing else chooses.
// The runtime (lift/runtime/fle_desk.c, fle_desk_scene) does not compose: it describes the scene - the windows bottom to top with the origin on the picture, the surface (address, stride, size), the layering - and the rectangles of
// the surfaces that were written since the last call. This file keeps one texture per window the runtime names and does exactly what the description says: texSubImage2D of each written rectangle straight from the engine's linear
// memory (UNPACK_ROW_LENGTH = the surface's stride; no JS copy and no conversion: the surface is B, G, R, X and the fragment shader reorders the bytes), then draws every window as a quad in the order given. It knows nothing about
// windows: no class name, no window kind, no position is looked at, nothing is decided here. Moving, restacking or hiding a window uploads no pixel.
import type { DeskPresentStats, DeskState } from './desk.ts';

export const SCENE_MAGIC = 0x4e454353;        // bytes 'S' 'C' 'E' 'N'
export const SCENE_VERSION = 2;
export const SCENE_MAX_WINDOWS = 64;
export const SCENE_MAX_UPDATES = 64;
export const SCENE_HEADER_BYTES = 64;         // 16 words: magic, version, width, height, gen, n_windows, n_updates, flags, background, reserved[7]
export const SCENE_WIN_BYTES = 80;            // 20 words: lid, x, y, w, h, bits, stride, layer, alpha, key, fresh, reserved, gx0, gy0, gx1, gy1, cl, ct, cr, cb
export const SCENE_UPD_BYTES = 20;            // 5 words: lid, x0, y0, x1, y1
export const SCENE_FLAG_WHOLE = 1;
export const SCENE_FLAG_CHANGED = 2;
export const SCENE_LAYERED = 8;
export const SCENE_ALPHA = 1;
export const SCENE_KEY = 2;
export const SCENE_PREMUL = 4;
export const SCENE_GLASS = 16;

export interface SceneWin { lid: number; x: number; y: number; w: number; h: number; bits: number; stride: number; layer: number; alpha: number; key: number; fresh: boolean; gx0: number; gy0: number; gx1: number; gy1: number; cl: number; ct: number; cr: number; cb: number }
export interface SceneUpd { lid: number; x0: number; y0: number; x1: number; y1: number }
export interface SceneState { width: number; height: number; gen: number; flags: number; background: number; windows: SceneWin[]; updates: SceneUpd[] }

export function readScene(buffer: ArrayBufferLike, ptr: number): SceneState | null {
  if (ptr === 0) return null;
  const dv = new DataView(buffer, ptr, SCENE_HEADER_BYTES + SCENE_MAX_WINDOWS * SCENE_WIN_BYTES + SCENE_MAX_UPDATES * SCENE_UPD_BYTES);
  if (dv.getUint32(0, true) !== SCENE_MAGIC || dv.getUint32(4, true) !== SCENE_VERSION) return null;
  const nw = Math.min(dv.getUint32(20, true), SCENE_MAX_WINDOWS), nu = Math.min(dv.getUint32(24, true), SCENE_MAX_UPDATES);
  const windows: SceneWin[] = [];
  for (let i = 0; i < nw; i++) {
    const b = SCENE_HEADER_BYTES + i * SCENE_WIN_BYTES;
    windows.push({ lid: dv.getUint32(b, true), x: dv.getInt32(b + 4, true), y: dv.getInt32(b + 8, true), w: dv.getUint32(b + 12, true), h: dv.getUint32(b + 16, true), bits: dv.getUint32(b + 20, true), stride: dv.getUint32(b + 24, true), layer: dv.getUint32(b + 28, true), alpha: dv.getUint32(b + 32, true), key: dv.getUint32(b + 36, true), fresh: dv.getUint32(b + 40, true) !== 0,
      gx0: dv.getInt32(b + 48, true), gy0: dv.getInt32(b + 52, true), gx1: dv.getInt32(b + 56, true), gy1: dv.getInt32(b + 60, true), cl: dv.getInt32(b + 64, true), ct: dv.getInt32(b + 68, true), cr: dv.getInt32(b + 72, true), cb: dv.getInt32(b + 76, true) });
  }
  const updates: SceneUpd[] = [];
  const ub = SCENE_HEADER_BYTES + SCENE_MAX_WINDOWS * SCENE_WIN_BYTES;
  for (let i = 0; i < nu; i++) {
    const b = ub + i * SCENE_UPD_BYTES;
    updates.push({ lid: dv.getUint32(b, true), x0: dv.getInt32(b + 4, true), y0: dv.getInt32(b + 8, true), x1: dv.getInt32(b + 12, true), y1: dv.getInt32(b + 16, true) });
  }
  return { width: dv.getUint32(8, true), height: dv.getUint32(12, true), gen: dv.getUint32(16, true), flags: dv.getUint32(28, true), background: dv.getUint32(32, true), windows, updates };
}

/** the part of the engine's exports the GL presenter uses (on top of the compositor's resync / describe) */
export interface SceneApi {
  /** fle_desk_scene(): the address of the description */
  scene(): number;
  /** fle_desk_scene_mode(on) */
  sceneMode(on: number): void;
  resync(): void;
  describe(): string;
}

/** the presenter the Worker uses for the real engine: `present()` shows what changed and says how many rectangles it uploaded (1 when only the arrangement changed, 0 when nothing did) */
export interface DeskShow {
  readonly stats: DeskPresentStats;
  lastState: DeskState | null;
  present(): number;
  resync(): void;
  readonly generation: number;
}

const VERT = `#version 300 es
uniform vec4 u_rect;
uniform vec2 u_screen;
out vec2 v_uv;
void main() {
  vec2 p = vec2(float(gl_VertexID & 1), float((gl_VertexID >> 1) & 1));
  v_uv = p;
  vec2 px = u_rect.xy + p * u_rect.zw;
  gl_Position = vec4(px.x / u_screen.x * 2.0 - 1.0, 1.0 - px.y / u_screen.y * 2.0, 0.0, 1.0);
}`;
// mode 0: an ordinary window, opaque (the surface is B, G, R, X: the fourth byte is not an alpha); mode 3: a window with an extended frame (decision 0244): opaque inside the inner box, a premultiplied ARGB over what is below outside it;
// mode 1: LWA_ALPHA / UpdateLayeredWindow's constant alpha and LWA_COLORKEY; mode 2: UpdateLayeredWindow with a per-pixel alpha
// (Src = Src * SCA, Dst = Src + (1 - Src.A) * Dst, which is blendFunc(ONE, ONE_MINUS_SRC_ALPHA) on the scaled source)
const FRAG = `#version 300 es
precision highp float;
uniform sampler2D u_tex;
uniform int u_mode;
uniform float u_alpha;
uniform int u_use_key;
uniform vec3 u_key;
uniform vec2 u_size;
uniform vec4 u_inner;
uniform vec4 u_region;
in vec2 v_uv;
out vec4 o;
void main() {
  vec4 t = texture(u_tex, v_uv);
  vec3 c = t.bgr;
  vec2 p = floor(v_uv * u_size);
  if (p.x < u_region.x || p.y < u_region.y || p.x >= u_region.z || p.y >= u_region.w) discard;
  if (u_mode == 0) { o = vec4(c, 1.0); return; }
  if (u_mode == 3) {
    if (p.x >= u_inner.x && p.y >= u_inner.y && p.x < u_inner.z && p.y < u_inner.w) { o = vec4(c, 1.0); return; }
    o = vec4(c, t.a);
    return;
  }
  if (u_mode == 1) {
    if (u_use_key != 0 && all(lessThan(abs(c - u_key), vec3(0.5 / 255.0)))) discard;
    o = vec4(c * u_alpha, u_alpha);
    return;
  }
  o = vec4(c * u_alpha, t.a * u_alpha);
}`;

/**
 * The condition for showing the desktop with WebGL2, decided before the desktop canvas is touched (a canvas keeps the kind of its first context, so a failed attempt on it could not fall back to the 2D one). A scratch canvas gives a
 * WebGL2 context WITHOUT a major performance caveat (`failIfMajorPerformanceCaveat: true`: a platform that counts its WebGL2 as a software renderer answers null; software GL measured slower than the 2D picture at 3240x1946, decision 0240.
 * MEASURED: the headless Chrome 156 (SwiftShader) and Firefox 158 (llvmpipe) of that work still answer with a context, so there they get WebGL2; what the owner's browser answers is not known), and its largest texture is at least GL_MIN_TEXTURE_SIZE (a window's surface is a texture; the guest's screen can be up to 8K: 7680 wide, and the main form is a few pixels larger than the screen).
 * The platform decides; no person, URL or setting does.
 */
export const GL_MIN_TEXTURE_SIZE = 8192;
export function webgl2Usable(make: () => { getContext(kind: 'webgl2', attrs?: WebGLContextAttributes): Gl | null }): boolean {
  try {
    const gl = make().getContext('webgl2', { failIfMajorPerformanceCaveat: true });
    if (!gl) return false;
    const ok = (gl.getParameter(gl.MAX_TEXTURE_SIZE) as number) >= GL_MIN_TEXTURE_SIZE;
    gl.getExtension('WEBGL_lose_context')?.loseContext();
    return ok;
  } catch { return false; }
}

/** the part of the WebGL2 API this file calls: WebGL2RenderingContext satisfies it; a test passes a recorder */
export type Gl = WebGL2RenderingContext;
export interface GlSurface { width: number; height: number; getContext(kind: 'webgl2', attrs?: WebGLContextAttributes): Gl | null }

interface Tex { tex: WebGLTexture; w: number; h: number }

export class GlDeskPresenter implements DeskShow {
  readonly stats: DeskPresentStats & { drawMs: number; uploads: number; draws: number; texAllocs: number; uploadBytes: number } = { presents: 0, rects: 0, pixels: 0, composeMs: 0, putMs: 0, whole: 0, drawMs: 0, uploads: 0, draws: 0, texAllocs: 0, uploadBytes: 0 };
  lastState: DeskState | null = null;
  private gl: Gl;
  private loc: Record<string, WebGLUniformLocation | null> = {};
  private texs = new Map<number, Tex>();
  private lastGen = -1;
  private scene: SceneState | null = null;
  private heap: { buffer: ArrayBufferLike; u8: Uint8Array } | null = null;
  private needDraw = true;
  private lost = false;
  private readonly surface: GlSurface;
  private readonly api: SceneApi;
  private readonly memory: () => ArrayBufferLike | null;
  private readonly now: () => number;

  constructor(surface: GlSurface, api: SceneApi, memory: () => ArrayBufferLike | null, now: () => number = () => performance.now()) {
    this.surface = surface;
    this.api = api;
    this.memory = memory;
    this.now = now;
    const gl = surface.getContext('webgl2', { alpha: false, antialias: false, depth: false, stencil: false, preserveDrawingBuffer: false, powerPreference: 'high-performance' });
    if (!gl) throw new Error('no webgl2 context for the desktop canvas');
    this.gl = gl;
    this.build();
    api.sceneMode(1);
  }

  private build(): void {
    const gl = this.gl;
    const sh = (kind: number, src: string): WebGLShader => {
      const s = gl.createShader(kind) as WebGLShader;
      gl.shaderSource(s, src);
      gl.compileShader(s);
      if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(`desktop shader: ${gl.getShaderInfoLog(s)}`);
      return s;
    };
    const p = gl.createProgram() as WebGLProgram;
    gl.attachShader(p, sh(gl.VERTEX_SHADER, VERT));
    gl.attachShader(p, sh(gl.FRAGMENT_SHADER, FRAG));
    gl.linkProgram(p);
    if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(`desktop program: ${gl.getProgramInfoLog(p)}`);
    for (const n of ['u_rect', 'u_screen', 'u_tex', 'u_mode', 'u_alpha', 'u_use_key', 'u_key', 'u_size', 'u_inner', 'u_region']) this.loc[n] = gl.getUniformLocation(p, n);
    gl.useProgram(p);
    gl.uniform1i(this.loc.u_tex as WebGLUniformLocation, 0);
    gl.disable(gl.DEPTH_TEST);
    gl.disable(gl.DITHER);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
  }

  get generation(): number { return this.lastGen; }

  /** the canvas lost its context or got it back empty: every texture is made again and uploaded in full at the next present */
  resync(): void {
    this.texs.clear();
    this.needDraw = true;
    this.api.resync();
  }
  setLost(lost: boolean): void { this.lost = lost; if (!lost) { this.gl = this.surface.getContext('webgl2') as Gl; this.build(); this.texs.clear(); this.resync(); } }

  private u8(): Uint8Array {
    const buf = this.memory() as ArrayBufferLike;
    if (!this.heap || this.heap.buffer !== buf) this.heap = { buffer: buf, u8: new Uint8Array(buf) };
    return this.heap.u8;
  }

  present(): number {
    if (!this.memory() || this.lost) return 0;
    const t0 = this.now();
    const ptr = this.api.scene();
    const sc = readScene(this.memory() as ArrayBufferLike, ptr);
    const t1 = this.now();
    this.stats.composeMs += t1 - t0;
    if (!sc) return 0;
    if (this.surface.width !== sc.width || this.surface.height !== sc.height) { this.surface.width = sc.width; this.surface.height = sc.height; this.needDraw = true; this.api.resync(); return this.present(); }
    const changed = (sc.flags & (SCENE_FLAG_CHANGED | SCENE_FLAG_WHOLE)) !== 0;
    if (sc.updates.length === 0 && !changed && !this.needDraw) return 0;
    const gl = this.gl;
    const live = new Set<number>();
    for (const w of sc.windows) live.add(w.lid);
    for (const [lid, t] of this.texs) if (!live.has(lid)) { gl.deleteTexture(t.tex); this.texs.delete(lid); }
    for (const w of sc.windows) {
      const have = this.texs.get(w.lid);
      if (!have || have.w !== w.w || have.h !== w.h || w.fresh) {
        if (have) gl.deleteTexture(have.tex);
        const tex = gl.createTexture() as WebGLTexture;
        gl.bindTexture(gl.TEXTURE_2D, tex);
        gl.texStorage2D(gl.TEXTURE_2D, 1, gl.RGBA8, w.w, w.h);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
        this.texs.set(w.lid, { tex, w: w.w, h: w.h });
        this.stats.texAllocs++;
      }
    }
    const heap = this.u8();
    let px = 0;
    const byLid = new Map<number, SceneWin>();
    for (const w of sc.windows) byLid.set(w.lid, w);
    for (const u of sc.updates) {
      const w = byLid.get(u.lid), t = this.texs.get(u.lid);
      if (!w || !t) continue;
      const cw = u.x1 - u.x0, ch = u.y1 - u.y0;
      if (cw <= 0 || ch <= 0) continue;
      gl.bindTexture(gl.TEXTURE_2D, t.tex);
      gl.pixelStorei(gl.UNPACK_ROW_LENGTH, w.stride >> 2);
      gl.texSubImage2D(gl.TEXTURE_2D, 0, u.x0, u.y0, cw, ch, gl.RGBA, gl.UNSIGNED_BYTE, heap, w.bits + u.y0 * w.stride + u.x0 * 4);
      px += cw * ch;
      this.stats.uploads++;
      this.stats.uploadBytes += cw * ch * 4;
    }
    gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0);
    const t2 = this.now();
    this.draw(sc);
    const t3 = this.now();
    this.scene = sc;
    this.needDraw = false;
    this.stats.presents++;
    this.stats.rects += sc.updates.length;
    this.stats.pixels += px;
    if ((sc.flags & SCENE_FLAG_WHOLE) !== 0) this.stats.whole++;
    this.stats.putMs += t2 - t1;
    this.stats.drawMs += t3 - t2;
    this.lastGen = sc.gen;
    return Math.max(1, sc.updates.length);
  }

  private draw(sc: SceneState): void {
    const gl = this.gl;
    gl.viewport(0, 0, sc.width, sc.height);
    gl.clearColor((sc.background & 255) / 255, ((sc.background >> 8) & 255) / 255, ((sc.background >> 16) & 255) / 255, 1);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.uniform2f(this.loc.u_screen as WebGLUniformLocation, sc.width, sc.height);
    gl.activeTexture(gl.TEXTURE0);
    for (const w of sc.windows) {
      const t = this.texs.get(w.lid);
      if (!t) continue;
      const layered = (w.layer & SCENE_LAYERED) !== 0, glass = !layered && (w.layer & SCENE_GLASS) !== 0;
      const keyed = layered && (w.layer & SCENE_KEY) !== 0, premul = layered && (w.layer & SCENE_PREMUL) !== 0;
      const alpha = (w.layer & SCENE_ALPHA) !== 0 ? w.alpha & 255 : 255;
      const opaque = !layered || (alpha === 255 && !keyed && !premul);
      gl.bindTexture(gl.TEXTURE_2D, t.tex);
      gl.uniform4f(this.loc.u_rect as WebGLUniformLocation, w.x, w.y, w.w, w.h);
      gl.uniform1i(this.loc.u_mode as WebGLUniformLocation, glass ? 3 : opaque ? 0 : premul ? 2 : 1);
      gl.uniform2f(this.loc.u_size as WebGLUniformLocation, w.w, w.h);
      gl.uniform4f(this.loc.u_inner as WebGLUniformLocation, w.gx0, w.gy0, w.gx1, w.gy1);
      gl.uniform4f(this.loc.u_region as WebGLUniformLocation, w.cl, w.ct, w.cr, w.cb);
      gl.uniform1f(this.loc.u_alpha as WebGLUniformLocation, alpha / 255);
      gl.uniform1i(this.loc.u_use_key as WebGLUniformLocation, keyed ? 1 : 0);
      gl.uniform3f(this.loc.u_key as WebGLUniformLocation, (w.key & 255) / 255, ((w.key >> 8) & 255) / 255, ((w.key >> 16) & 255) / 255);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
      this.stats.draws++;
    }
  }

  /** a test aid: the scene drawn again and read back at once (a drawing buffer that was presented is cleared), R, G, B, A bytes, top row first */
  readPixels(): { width: number; height: number; data: Uint8Array } | null {
    const sc = this.scene;
    if (!sc) return null;
    const gl = this.gl;
    this.draw(sc);
    const w = sc.width, h = sc.height;
    const raw = new Uint8Array(w * h * 4);
    gl.readPixels(0, 0, w, h, gl.RGBA, gl.UNSIGNED_BYTE, raw);
    const out = new Uint8Array(w * h * 4);
    for (let y = 0; y < h; y++) out.set(raw.subarray((h - 1 - y) * w * 4, (h - y) * w * 4), y * w * 4);
    this.needDraw = true;
    return { width: w, height: h, data: out };
  }
}
