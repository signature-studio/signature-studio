// Reader of fle_cursor_picture_poll (lift/runtime/fle_appview.h, decision 0260, #1392): the picture of a cursor the guest loaded from one of its own modules (read from the resource itself) or made with CreateIconIndirect (read from its bitmaps when it was made).
// The layout is the C structure fle_cursor_pic_state (256 bytes, little-endian 32-bit words; app/test/unit/cursor-picture.test.ts pins the offsets to the static asserts of fle_appview.c).
export const CP_MAGIC = 0x50434c46; // bytes 'F' 'L' 'C' 'P'
export const CP_BYTES = 256;
const CP_MAX = 256;

/** what the engine gives for one cursor handle; `picture` is null when the engine has none (`why` says why) */
export interface CursorPictureMsg {
  width: number; height: number;
  /** the hot spot (the RT_CURSOR's first two WORDs, or the ICONINFO's hot spot of a made cursor) */
  hotX: number; hotY: number;
  /** the RT_CURSOR id the group names (0 for a made cursor) */
  cursorId: number;
  /** loaded by name (`name` holds it) or by ordinal (`ordinal` holds the ordinal 32762..32767 when named is false, else the position of the name among the named groups); made: made by CreateIconIndirect (no resource) */
  named: boolean; ordinal: number; name: string; made: boolean;
  /** the picture is drawn with the alpha bytes of its 32-bit colour bitmap (the AND mask is then ignored; fle_cursor.h); else the AND and XOR rule */
  alpha: boolean;
  /** pixel counts: AND 1 and XOR not 0 (drawn opaque black; alpha off only); transparent (AND 1 and XOR 0, or alpha 0); opaque (AND 0, or alpha 255); the rest are partly transparent (alpha on) */
  invert: number; transparent: number; opaque: number;
  /** width * height * 4 bytes, the top row first (R, G, B, A) */
  rgba: ArrayBuffer;
  /** a made cursor: width * height * 6 bytes, per pixel a (the AND bit), nz (the XOR value is not 0), r, g, b (the XOR colour), al (the alpha byte); null for a resource */
  raw: ArrayBuffer | null;
}

export interface CursorPictureRead {
  handle: number;
  ok: boolean;
  why: string;
  picture: CursorPictureMsg | null;
}

const LATIN1_CHAR = (u8: Uint8Array, off: number, len: number): string => {
  let s = '';
  for (let i = off; i < off + len && u8[i] !== 0; i++) s += String.fromCharCode(u8[i] ?? 0);
  return s;
};

/** the structure at `ptr` in the engine's memory; null when the magic does not match (a build without the export, or a structure of another layout) */
export function readCursorPicture(buffer: ArrayBufferLike, ptr: number, handle: number): CursorPictureRead | null {
  const dv = new DataView(buffer, ptr, CP_BYTES);
  if (dv.getUint32(0, true) !== CP_MAGIC) return null;
  const u8 = new Uint8Array(buffer, ptr, CP_BYTES);
  const why = LATIN1_CHAR(u8, 128, 128);
  if (dv.getUint32(4, true) !== 1) return { handle, ok: false, why, picture: null };
  const width = dv.getUint32(12, true), height = dv.getUint32(16, true);
  if (width === 0 || height === 0 || width > CP_MAX || height > CP_MAX) return { handle, ok: false, why: `the engine gave a picture of ${width} x ${height}`, picture: null };
  const rgbaAddr = dv.getUint32(56, true), rawAddr = dv.getUint32(60, true);
  const n = width * height * 4;
  const rgba = new Uint8Array(n);
  rgba.set(new Uint8Array(buffer, rgbaAddr, n));   // a copy: the engine's buffer is reused by the next call
  const named = dv.getUint32(32, true);
  const made = named === 2;
  let raw: ArrayBuffer | null = null;
  if (made) {
    const rawBytes = new Uint8Array(width * height * 6);
    rawBytes.set(new Uint8Array(buffer, rawAddr, rawBytes.length));
    raw = rawBytes.buffer;
  }
  return {
    handle, ok: true, why: '',
    picture: {
      width, height,
      hotX: dv.getUint32(20, true), hotY: dv.getUint32(24, true),
      cursorId: dv.getUint32(28, true),
      named: named === 1, ordinal: dv.getUint32(36, true), name: LATIN1_CHAR(u8, 64, 64), made,
      invert: dv.getUint32(40, true), alpha: dv.getUint32(44, true) !== 0, transparent: dv.getUint32(48, true), opaque: dv.getUint32(52, true),
      rgba: rgba.buffer,
      raw,
    },
  };
}
