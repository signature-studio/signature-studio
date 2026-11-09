import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { AV_CLASS_BYTES, AV_HEADER_BYTES, AV_MAGIC, AV_TITLE_BYTES, AV_VERSION, AV_WINDOW_BYTES, readAppView, W_ACTIVE, W_CAPTURE, W_ENABLED, W_FOCUS, W_TOPMOST, W_VISIBLE } from '../../src/engine/appview.ts';

const REPO = process.env.APP_REPO_ROOT || path.resolve(__dirname, '../../..');
const rt = (f: string): string => fs.readFileSync(path.join(REPO, 'lift/runtime', f), 'utf8');

describe('the layout is the one of lift/runtime/fle_appview.h (read from the file)', () => {
  const h = rt('fle_appview.h'), c = rt('fle_appview.c');
  const def = (n: string): number => Number(new RegExp(`#define ${n}\\s+(0x[0-9a-fA-F]+|\\d+)u?`).exec(h)?.[1]);
  it('magic, version, sizes and flag bits', () => {
    expect(def('FLE_AV_MAGIC')).toBe(AV_MAGIC);
    expect(def('FLE_AV_VERSION')).toBe(AV_VERSION);
    expect(def('FLE_AV_CLASS_BYTES')).toBe(AV_CLASS_BYTES);
    expect(def('FLE_AV_TITLE_BYTES')).toBe(AV_TITLE_BYTES);
    expect(def('FLE_AV_W_VISIBLE')).toBe(W_VISIBLE); expect(def('FLE_AV_W_ENABLED')).toBe(W_ENABLED); expect(def('FLE_AV_W_TOPMOST')).toBe(W_TOPMOST);
    expect(def('FLE_AV_W_ACTIVE')).toBe(W_ACTIVE); expect(def('FLE_AV_W_FOCUS')).toBe(W_FOCUS); expect(def('FLE_AV_W_CAPTURE')).toBe(W_CAPTURE);
  });
  it('the C file asserts the offsets this reader uses', () => {
    expect(c).toContain(`sizeof(fle_av_window) == ${AV_WINDOW_BYTES}`);
    expect(c).toContain(`offsetof(fle_av_state, windows) == ${AV_HEADER_BYTES}`);
    expect(c).toMatch(/offsetof\(fle_av_window, x\) == 16 && offsetof\(fle_av_window, style\) == 32 && offsetof\(fle_av_window, cls\) == 48 && offsetof\(fle_av_window, title\) == 80/);
    expect(c).toMatch(/offsetof\(fle_av_state, seq\) == 16 && offsetof\(fle_av_state, cursor\) == 20 && offsetof\(fle_av_state, pointer_x\) == 32 && offsetof\(fle_av_state, n_windows\) == 44/);
  });
  it('the struct declares the fields in the order the reader assumes', () => {
    const win = /typedef struct \{([^}]*)\} fle_av_window;/.exec(h)?.[1] ?? '';
    expect(win.replace(/\s+/g, ' ')).toContain('uint32_t lid, hwnd, flags, owner_lid; int32_t x, y, width, height; uint32_t style, exstyle, thread, reserved; char cls[FLE_AV_CLASS_BYTES]; char title[FLE_AV_TITLE_BYTES];');
    const st = /typedef struct \{([^}]*)\} fle_av_state;/.exec(h)?.[1] ?? '';
    expect(st.replace(/\/\*.*?\*\//g, '').replace(/\s+/g, ' ')).toContain('uint32_t magic, version, entry_size, max_windows; uint32_t seq; uint32_t cursor; uint32_t cursor_idc; uint32_t cursor_shared; int32_t pointer_x, pointer_y; uint32_t pending_events; uint32_t n_windows; uint32_t screen_w, screen_h; uint32_t reserved[3]; fle_av_window windows[FLE_AV_MAX_WINDOWS];');
  });
});

function build(opts: { windows?: { lid: number; flags: number; owner: number; x: number; y: number; w: number; h: number; cls: string; title: string }[]; cursor?: number; idc?: number; shared?: number; seq?: number; px?: number; py?: number; pending?: number } = {}): { buf: ArrayBuffer; ptr: number } {
  const ptr = 24;   // not at 0: the reader must honour the pointer
  const buf = new ArrayBuffer(ptr + AV_HEADER_BYTES + 32 * AV_WINDOW_BYTES);
  const dv = new DataView(buf, ptr);
  const u8 = new Uint8Array(buf, ptr);
  const ws = opts.windows ?? [];
  dv.setUint32(0, AV_MAGIC, true); dv.setUint32(4, AV_VERSION, true); dv.setUint32(8, AV_WINDOW_BYTES, true); dv.setUint32(12, 32, true);
  dv.setUint32(16, opts.seq ?? 1, true); dv.setUint32(20, opts.cursor ?? 0, true); dv.setUint32(24, opts.idc ?? 0, true); dv.setUint32(28, opts.shared ?? 0, true);
  dv.setInt32(32, opts.px ?? 0, true); dv.setInt32(36, opts.py ?? 0, true); dv.setUint32(40, opts.pending ?? 0, true); dv.setUint32(44, ws.length, true);
  ws.forEach((w, i) => {
    const b = AV_HEADER_BYTES + i * AV_WINDOW_BYTES;
    dv.setUint32(b, w.lid, true); dv.setUint32(b + 4, 0x7f990000 + w.lid * 4, true); dv.setUint32(b + 8, w.flags, true); dv.setUint32(b + 12, w.owner, true);
    dv.setInt32(b + 16, w.x, true); dv.setInt32(b + 20, w.y, true); dv.setInt32(b + 24, w.w, true); dv.setInt32(b + 28, w.h, true);
    dv.setUint32(b + 32, 0x90000000, true); dv.setUint32(b + 36, 8, true); dv.setUint32(b + 40, 3, true);
    u8.set(new TextEncoder().encode(w.cls), b + 48);
    u8.set(new TextEncoder().encode(w.title), b + 48 + AV_CLASS_BYTES);
  });
  return { buf, ptr };
}

describe('readAppView', () => {
  it('reads the header and the windows in the order the engine gave them (top of the z-order first)', () => {
    const { buf, ptr } = build({
      seq: 9, cursor: 0x7fca0004, idc: 32512, px: 958, py: 599, pending: 3,
      windows: [
        { lid: 48, flags: W_VISIBLE | W_ENABLED | W_ACTIVE | W_FOCUS, owner: 5, x: 523, y: 255, w: 874, h: 570, cls: 'TWelcomeWizard', title: 'Welcome to FL Studio' },
        { lid: 13, flags: W_VISIBLE | W_TOPMOST, owner: 5, x: -5, y: -6, w: 1930, h: 1092, cls: 'TFruityLoopsMainForm', title: 'FL Studio 2026' },
      ],
    });
    const v = readAppView(buf, ptr);
    expect(v).not.toBeNull();
    expect(v).toMatchObject({ seq: 9, cursor: 0x7fca0004, cursorIdc: 32512, cursorShared: false, pointerX: 958, pointerY: 599, pendingEvents: 3 });
    expect(v?.windows.map((w) => w.lid)).toEqual([48, 13]);
    expect(v?.windows[0]).toMatchObject({ lid: 48, ownerLid: 5, x: 523, y: 255, width: 874, height: 570, className: 'TWelcomeWizard', title: 'Welcome to FL Studio', visible: true, enabled: true, active: true, focus: true, topmost: false, capture: false });
    expect(v?.windows[1]).toMatchObject({ lid: 13, x: -5, y: -6, width: 1930, height: 1092, visible: true, enabled: false, topmost: true, active: false, style: 0x90000000, exstyle: 8, thread: 3 });
  });
  it('with the sequence number the caller already holds it answers null without reading the window list (decision 0170); another number reads it', () => {
    const { buf, ptr } = build({ seq: 9, windows: [{ lid: 13, flags: 1, owner: 0, x: 1, y: 2, w: 3, h: 4, cls: 'a', title: '' }] });
    expect(readAppView(buf, ptr, 9)).toBeNull();
    expect(readAppView(buf, ptr, 8)?.seq).toBe(9);
    expect(readAppView(buf, ptr, undefined)?.windows).toHaveLength(1);
  });
  it('negative positions stay negative (the main window is at (-5, -6))', () => {
    const { buf, ptr } = build({ windows: [{ lid: 13, flags: 1, owner: 0, x: -5, y: -6, w: 1, h: 1, cls: 'a', title: '' }] });
    expect(readAppView(buf, ptr)?.windows[0]).toMatchObject({ x: -5, y: -6 });
  });
  it('refuses a structure with another magic, version or entry size, and never reads past max_windows', () => {
    const { buf, ptr } = build();
    new DataView(buf, ptr).setUint32(0, 0, true);
    expect(readAppView(buf, ptr)).toBeNull();
    const b2 = build(); new DataView(b2.buf, b2.ptr).setUint32(4, 2, true);
    expect(readAppView(b2.buf, b2.ptr)).toBeNull();
    const b3 = build(); new DataView(b3.buf, b3.ptr).setUint32(8, 100, true);
    expect(readAppView(b3.buf, b3.ptr)).toBeNull();
    const b4 = build(); new DataView(b4.buf, b4.ptr).setUint32(44, 999, true);
    expect(readAppView(b4.buf, b4.ptr)?.windows.length).toBe(32);
  });
  it('a shared (module) cursor is flagged, and a text longer than its field is cut at the NUL', () => {
    const { buf, ptr } = build({ cursor: 0x7fda5008, shared: 1, windows: [{ lid: 1, flags: 1, owner: 0, x: 0, y: 0, w: 1, h: 1, cls: 'X', title: 'abc' }] });
    const v = readAppView(buf, ptr);
    expect(v?.cursorShared).toBe(true);
    expect(v?.windows[0]?.title).toBe('abc');
  });
});
