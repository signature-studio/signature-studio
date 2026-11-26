import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { CP_BYTES, CP_MAGIC, readCursorPicture } from '../../src/engine/cursor-picture.ts';

const REPO = process.env.APP_REPO_ROOT || path.resolve(__dirname, '../../..');
const rt = (f: string): string => fs.readFileSync(path.join(REPO, 'lift/runtime', f), 'utf8');

/** the engine's memory with one structure of the layout fle_cursor_pic_state at `ptr` (the words; the name and the reason as NUL-terminated text) and the pixels after it: rgba_addr is their address */
function engineMemory(fields: Record<number, number>, texts: Record<number, string>, rgba: number[]): { buffer: ArrayBuffer; ptr: number } {
  const ptr = 64, pixels = ptr + CP_BYTES, buffer = new ArrayBuffer(pixels + rgba.length + 16);
  const dv = new DataView(buffer);
  for (const [off, v] of Object.entries(fields)) dv.setUint32(ptr + Number(off), v, true);
  for (const [off, s] of Object.entries(texts)) for (let i = 0; i < s.length; i++) dv.setUint8(ptr + Number(off) + i, s.charCodeAt(i));
  dv.setUint32(ptr + 56, pixels, true);
  new Uint8Array(buffer).set(rgba, pixels);
  return { buffer, ptr };
}

describe('the layout of the picture is the one of lift/runtime/fle_appview.h (read from the files)', () => {
  const h = rt('fle_appview.h'), c = rt('fle_appview.c');
  it('the magic is FLCP and the structure is 256 bytes', () => {
    expect(Number(/#define FLE_CP_MAGIC\s+(0x[0-9a-fA-F]+)u?/.exec(h)?.[1])).toBe(CP_MAGIC);
    expect(CP_BYTES).toBe(256);
    expect(c).toContain('sizeof(fle_cursor_pic_state) == 256');
  });
  it('the C file asserts the offsets this reader uses', () => {
    expect(c).toMatch(/offsetof\(fle_cursor_pic_state, ordinal\) == 36 && offsetof\(fle_cursor_pic_state, rgba_addr\) == 56 && offsetof\(fle_cursor_pic_state, raw_addr\) == 60 && offsetof\(fle_cursor_pic_state, name\) == 64 && offsetof\(fle_cursor_pic_state, why\) == 128/);
  });
  it('the struct declares the words in the order the reader assumes', () => {
    const st = /typedef struct \{([^}]*)\} fle_cursor_pic_state;/.exec(h)?.[1] ?? '';
    const words = st.replace(/\/\*.*?\*\//g, '').replace(/\s+/g, ' ');
    expect(words).toContain('uint32_t magic, ok, handle, width, height, hot_x, hot_y, cursor_id; uint32_t named, ordinal, invert, alpha, transparent, opaque, rgba_addr, raw_addr; char name[64]; char why[128];');
  });
});

describe('the reader of the engine\'s picture', () => {
  it('reads a picture: the size, the hot spot, the counts, the name, and a copy of the pixels', () => {
    const px = [1, 2, 3, 255, 0, 0, 0, 0, 9, 8, 7, 255, 0, 0, 0, 255];
    const { buffer, ptr } = engineMemory({ 0: CP_MAGIC, 4: 1, 8: 0x7fda5008, 12: 2, 16: 2, 20: 1, 24: 1, 28: 26, 32: 1, 36: 0, 40: 0, 44: 1, 48: 1, 52: 3 }, { 64: 'EEPEN' }, px);
    const r = readCursorPicture(buffer, ptr, 0x7fda5008);
    expect(r?.ok).toBe(true);
    expect(r?.picture).toMatchObject({ width: 2, height: 2, hotX: 1, hotY: 1, cursorId: 26, named: true, name: 'EEPEN', invert: 0, alpha: true, transparent: 1, opaque: 3 });
    expect(Array.from(new Uint8Array(r?.picture?.rgba ?? new ArrayBuffer(0)))).toEqual(px);
  });
  it('an ordinal is not a name; no picture gives the reason', () => {
    const { buffer, ptr } = engineMemory({ 0: CP_MAGIC, 4: 0, 8: 1 }, { 128: 'the group has 2 entries' }, []);
    expect(readCursorPicture(buffer, ptr, 1)).toEqual({ handle: 1, ok: false, why: 'the group has 2 entries', picture: null });
    const r = engineMemory({ 0: CP_MAGIC, 4: 1, 12: 1, 16: 1, 32: 0, 36: 32762 }, {}, [0, 0, 0, 0]);
    expect(readCursorPicture(r.buffer, r.ptr, 2)?.picture?.named).toBe(false);
    expect(readCursorPicture(r.buffer, r.ptr, 2)?.picture?.ordinal).toBe(32762);
  });
  it('a made cursor (named 2) is read with its raw pixels: width * height * 6 bytes, from raw_addr', () => {
    const raw = [1, 0, 9, 9, 9, 0, 0, 1, 0, 0, 0, 255];
    const rgba = [0, 0, 0, 255, 0, 0, 0, 0];
    const { buffer, ptr } = engineMemory({ 0: CP_MAGIC, 4: 1, 12: 2, 16: 1, 32: 2, 36: 0, 44: 0 }, {}, rgba);
    const dv = new DataView(buffer);
    const rawAt = ptr + CP_BYTES + rgba.length;
    new Uint8Array(buffer).set(raw, rawAt);
    dv.setUint32(ptr + 60, rawAt, true);
    const r = readCursorPicture(buffer, ptr, 0x7fba0100);
    expect(r?.picture).toMatchObject({ made: true, named: false, name: '', width: 2, height: 1 });
    expect(Array.from(new Uint8Array(r?.picture?.raw ?? new ArrayBuffer(0)))).toEqual(raw);
  });
  it('a resource is not made: raw is null', () => {
    const { buffer, ptr } = engineMemory({ 0: CP_MAGIC, 4: 1, 12: 1, 16: 1, 32: 1 }, { 64: 'EEPEN' }, [0, 0, 0, 0]);
    expect(readCursorPicture(buffer, ptr, 1)?.picture).toMatchObject({ made: false, raw: null });
  });
  it('a structure without the magic is not read', () => {
    const { buffer, ptr } = engineMemory({ 0: 0x12345678 }, {}, []);
    expect(readCursorPicture(buffer, ptr, 1)).toBeNull();
  });
  it('an absurd size is refused, not read', () => {
    const { buffer, ptr } = engineMemory({ 0: CP_MAGIC, 4: 1, 12: 4096, 16: 4096 }, {}, []);
    expect(readCursorPicture(buffer, ptr, 1)).toMatchObject({ ok: false, picture: null });
  });
});
