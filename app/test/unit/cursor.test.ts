import { describe, expect, it } from 'vitest';
import { cursorCss, IDC_CSS, IDC_UNMAPPED } from '../../src/ui/cursor.ts';

/** a device pixel ratio from 1.01 to 3.00 (2 decimals) drawn from a fixed sequence: the tests do not write a ratio of their own (#1480) */
let seed = 0x1480;
const ratio = (): number => {
  seed = (seed * 1103515245 + 12345) & 0x7fffffff;
  return Math.round((1.01 + (seed / 0x7fffffff) * 1.99) * 100) / 100;
};

const c = (cursor: number, idc = 0, shared = false, made = false) => cursorCss({ cursor, cursorIdc: idc, cursorShared: shared, cursorMade: made }, ratio());

/** the table of the Issue (#1390): Windows standard cursor, its value, the CSS cursor (null: no equivalent, the arrow is shown and the cursor is reported) */
const TABLE: [string, number, string | null][] = [
  ['IDC_ARROW', 32512, 'default'],
  ['IDC_IBEAM', 32513, 'text'],
  ['IDC_WAIT', 32514, 'wait'],
  ['IDC_CROSS', 32515, 'crosshair'],
  ['IDC_UPARROW', 32516, null],
  ['IDC_SIZENWSE', 32642, 'nwse-resize'],
  ['IDC_SIZENESW', 32643, 'nesw-resize'],
  ['IDC_SIZEWE', 32644, 'ew-resize'],
  ['IDC_SIZENS', 32645, 'ns-resize'],
  ['IDC_SIZEALL', 32646, 'move'],
  ['IDC_NO', 32648, 'not-allowed'],
  ['IDC_HAND', 32649, 'pointer'],
  ['IDC_APPSTARTING', 32650, 'progress'],
  ['IDC_HELP', 32651, 'help'],
];

describe('the guest\'s cursor as a CSS cursor', () => {
  it('the table has 14 rows, and each row of the table is the CSS cursor of its identifier (no look-alike)', () => {
    expect(TABLE).toHaveLength(14);
    for (const [name, idc, css] of TABLE) {
      if (css === null) {
        expect(IDC_CSS[idc], name).toBeUndefined();
        expect(IDC_UNMAPPED[idc], name).toBe(name);
        expect(c(1, idc), name).toMatchObject({ css: 'default', unmapped: { name, value: idc } });
      } else {
        expect(IDC_CSS[idc], name).toBe(css);
        expect(c(1, idc), name).toMatchObject({ css });
        expect(c(1, idc).unmapped, name).toBeUndefined();
      }
    }
    expect(Object.keys(IDC_CSS)).toHaveLength(13);
  });
  it('width and height resize cursors are not swapped (SIZEWE is horizontal: ew-resize; SIZENS is vertical: ns-resize)', () => {
    expect(IDC_CSS[32644]).toBe('ew-resize');
    expect(IDC_CSS[32645]).toBe('ns-resize');
  });
  it('SetCursor(NULL) removes the cursor: none', () => {
    expect(c(0).css).toBe('none');
    expect(c(0).why).toContain('NULL');
  });
  it('a cursor of the guest\'s own modules is shown as the arrow and the reason says so', () => {
    expect(c(0x7fda5008, 0, true)).toMatchObject({ css: 'default' });
    expect(c(0x7fda5008, 0, true).why).toContain('module');
    expect(c(0x7fda5008, 0, true).made).toBeUndefined();
  });
  it('a cursor the guest made itself (CreateIconIndirect) is reported as made, not as unresolved', () => {
    expect(c(0x7fba0100, 0, false, true)).toMatchObject({ css: 'default', made: true });
    expect(c(0x7fba0100, 0, false, true).why).toContain('made itself');
    expect(c(0x7fba0100, 0, false, true).unresolved).toBeUndefined();
  });
  it('a cursor of a module with its picture is the picture: image-set at the page\'s ratio, the hot spot in CSS px, not the arrow (#1392, #1480)', () => {
    const url = 'data:image/png;base64,AAAA';
    const pic = { ok: true as const, url, hotX: 31, hotY: 30, source: 'EEPEN', cursorId: 26, width: 32, height: 32, invert: 0, alpha: false };
    const dpr = ratio();
    const r = cursorCss({ cursor: 0x7fda0008, cursorIdc: 0, cursorShared: true, cursorMade: false }, dpr, pic);
    const css = `image-set(url("${url}") ${dpr}x) ${Math.round(31 / dpr)} ${Math.round(30 / dpr)}, default`;
    expect(r.css).toBe(css);
    expect(r.picture).toEqual({ source: 'EEPEN', cursorId: 26, width: 32, height: 32, hotX: 31, hotY: 30, invert: 0, alpha: false, css });
    expect(r.unpictured).toBeUndefined();
  });
  it('the picture is one device pixel per picture pixel: the declared resolution is the ratio, so its CSS size times the ratio is its size, at any ratio (#1480)', () => {
    const pic = { ok: true as const, url: 'A', hotX: 0, hotY: 0, source: 'EEPEN', cursorId: 26, width: 32, height: 24, invert: 0, alpha: false };
    const shared = { cursor: 0x7fda0008, cursorIdc: 0, cursorShared: true, cursorMade: false };
    for (let i = 0; i < 4; i++) {
      const dpr = ratio();
      const m = /^image-set\(url\("A"\) ([\d.]+)x\) 0 0, default$/.exec(cursorCss(shared, dpr, pic).css);
      expect(m, `ratio ${dpr}`).not.toBeNull();
      const n = Number(m?.[1]);
      expect(n).toBe(dpr);
      expect((32 / n) * dpr).toBeCloseTo(32, 9);
      expect((24 / n) * dpr).toBeCloseTo(24, 9);
    }
  });
  it('the hot spot is the picture\'s hot spot ÷ the ratio, as a whole CSS px (Chromium multiplies it back by the ratio and rounds)', () => {
    const shared = { cursor: 0x7fda0008, cursorIdc: 0, cursorShared: true, cursorMade: false };
    const pic = { ok: true as const, url: 'A', hotX: 31, hotY: 0, source: 'EEPEN', cursorId: 26, width: 32, height: 32, invert: 0, alpha: false };
    const dpr = ratio();
    const hotX = Number(/\) (\d+) (\d+), default$/.exec(cursorCss(shared, dpr, pic).css)?.[1]);
    expect(hotX).toBe(Math.round(31 / dpr));
    expect(Number.isInteger(hotX)).toBe(true);
    expect(Math.abs(Math.round(hotX * dpr) - 31)).toBeLessThanOrEqual(1);
  });
  it('a picture over 128 CSS px at the ratio is the arrow, and the reason says so: the browser drops it (#1480)', () => {
    const shared = { cursor: 0x7fda0008, cursorIdc: 0, cursorShared: true, cursorMade: false };
    const dpr = ratio();
    const over = Math.ceil(128 * dpr) + 1;
    const within = Math.floor(128 * dpr);
    const big = cursorCss(shared, dpr, { ok: true as const, url: 'A', hotX: 0, hotY: 0, source: 'EEPEN', cursorId: 26, width: over, height: 32, invert: 0, alpha: false });
    expect(big).toMatchObject({ css: 'default', unpictured: true });
    expect(big.why).toContain('larger than the 128 CSS px');
    expect(big.picture).toBeUndefined();
    const edge = cursorCss(shared, dpr, { ok: true as const, url: 'A', hotX: 0, hotY: 0, source: 'EEPEN', cursorId: 26, width: within, height: within, invert: 0, alpha: false });
    expect(edge.css.startsWith('image-set(url("A")')).toBe(true);
  });
  it('a module cursor whose picture is not known is the arrow, and says so (unpictured)', () => {
    const none = c(0x7fda0008, 0, true);
    expect(none).toMatchObject({ css: 'default', unpictured: true });
    expect(none.why).toContain('has not come');
    const failed = cursorCss({ cursor: 0x7fda0008, cursorIdc: 0, cursorShared: true, cursorMade: false }, ratio(), { ok: false, why: 'the group has 2 entries' });
    expect(failed).toMatchObject({ css: 'default', unpictured: true });
    expect(failed.why).toContain('the group has 2 entries');
  });
  it('a standard cursor is not given a picture (the module picture is not used for it)', () => {
    const pic = { ok: true as const, url: 'data:image/png;base64,AAAA', hotX: 1, hotY: 1, source: 'EEPEN', cursorId: 26, width: 32, height: 32, invert: 0, alpha: false };
    expect(cursorCss({ cursor: 1, cursorIdc: 32512, cursorShared: false, cursorMade: false }, ratio(), pic).css).toBe('default');
    expect(cursorCss({ cursor: 1, cursorIdc: 32513, cursorShared: false, cursorMade: false }, ratio(), pic).css).toBe('text');
  });
  it('a cursor the guest made with CreateIconIndirect is its picture when the engine has one (#1392), reported as made', () => {
    const pic = { ok: true as const, url: 'data:image/png;base64,AAAA', hotX: 2, hotY: 3, source: 'made by CreateIconIndirect', cursorId: 0, width: 8, height: 8, invert: 1, alpha: true, raw: 'AAAA' };
    const dpr = ratio();
    const r = cursorCss({ cursor: 0x7fba0100, cursorIdc: 0, cursorShared: false, cursorMade: true }, dpr, pic);
    expect(r.css).toBe(`image-set(url("data:image/png;base64,AAAA") ${dpr}x) ${Math.round(2 / dpr)} ${Math.round(3 / dpr)}, default`);
    expect(r.made).toBe(true);
    expect(r.picture).toMatchObject({ source: 'made by CreateIconIndirect', cursorId: 0, hotX: 2, hotY: 3, invert: 1, alpha: true, raw: 'AAAA' });
    expect(cursorCss({ cursor: 0x7fba0100, cursorIdc: 0, cursorShared: false, cursorMade: true }, ratio(), { ok: false, why: 'an icon' })).toMatchObject({ css: 'default', made: true, unpictured: true });
  });
  it('a handle that is none of these has no identity: unresolved', () => {
    expect(c(0x7fba0100)).toMatchObject({ css: 'default', unresolved: true });
  });
  it('an identifier without a CSS counterpart falls back to the arrow and is reported; no view: the arrow', () => {
    expect(c(1, 12345)).toMatchObject({ css: 'default', unmapped: { name: 'IDC 12345', value: 12345 } });
    expect(cursorCss(null, ratio()).css).toBe('default');
  });
});
