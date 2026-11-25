import { describe, expect, it } from 'vitest';
import { cursorCss, IDC_CSS, IDC_UNMAPPED } from '../../src/ui/cursor.ts';

const c = (cursor: number, idc = 0, shared = false, made = false) => cursorCss({ cursor, cursorIdc: idc, cursorShared: shared, cursorMade: made });

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
  it('a cursor of a module with its picture is the picture (url, then the hot spot), not the arrow (#1392)', () => {
    const url = 'data:image/png;base64,AAAA';
    const pic = { ok: true as const, images: [{ scale: 1, url }], hotX: 1, hotY: 1, source: 'EEPEN', cursorId: 26, width: 32, height: 32, invert: 0, alpha: false };
    const r = cursorCss({ cursor: 0x7fda0008, cursorIdc: 0, cursorShared: true, cursorMade: false }, pic);
    expect(r.css).toBe(`url("${url}") 1 1, default`);
    expect(r.picture).toEqual({ source: 'EEPEN', cursorId: 26, width: 32, height: 32, hotX: 1, hotY: 1, invert: 0, alpha: false, css: `url("${url}") 1 1, default` });
    expect(r.unpictured).toBeUndefined();
  });
  it('above a device pixel ratio of 1 the picture is image-set of its copies (1x, 2x, 3x), the same hot spot; at 1 it is the url (#1392)', () => {
    const images = [{ scale: 1, url: 'A' }, { scale: 2, url: 'B' }, { scale: 3, url: 'C' }];
    const pic = { ok: true as const, images, hotX: 1, hotY: 2, source: 'EEPEN', cursorId: 26, width: 32, height: 32, invert: 0, alpha: false };
    const shared = { cursor: 0x7fda0008, cursorIdc: 0, cursorShared: true, cursorMade: false };
    expect(cursorCss(shared, pic, 2).css).toBe('image-set(url("A") 1x, url("B") 2x, url("C") 3x) 1 2, default');
    expect(cursorCss(shared, pic, 1.5).css).toBe('image-set(url("A") 1x, url("B") 2x, url("C") 3x) 1 2, default');
    expect(cursorCss(shared, pic, 1).css).toBe('url("A") 1 2, default');
    expect(cursorCss(shared, { ...pic, images: [images[0] as { scale: number; url: string }] }, 2).css).toBe('url("A") 1 2, default');
  });
  it('a module cursor whose picture is not known is the arrow, and says so (unpictured)', () => {
    const none = c(0x7fda0008, 0, true);
    expect(none).toMatchObject({ css: 'default', unpictured: true });
    expect(none.why).toContain('has not come');
    const failed = cursorCss({ cursor: 0x7fda0008, cursorIdc: 0, cursorShared: true, cursorMade: false }, { ok: false, why: 'the group has 2 entries' });
    expect(failed).toMatchObject({ css: 'default', unpictured: true });
    expect(failed.why).toContain('the group has 2 entries');
  });
  it('a standard cursor is not given a picture (the module picture is not used for it)', () => {
    const pic = { ok: true as const, images: [{ scale: 1, url: 'data:image/png;base64,AAAA' }], hotX: 1, hotY: 1, source: 'EEPEN', cursorId: 26, width: 32, height: 32, invert: 0, alpha: false };
    expect(cursorCss({ cursor: 1, cursorIdc: 32512, cursorShared: false, cursorMade: false }, pic).css).toBe('default');
    expect(cursorCss({ cursor: 1, cursorIdc: 32513, cursorShared: false, cursorMade: false }, pic).css).toBe('text');
  });
  it('a cursor the guest made with CreateIconIndirect is its picture when the engine has one (#1392), reported as made', () => {
    const pic = { ok: true as const, images: [{ scale: 1, url: 'data:image/png;base64,AAAA' }], hotX: 2, hotY: 3, source: 'made by CreateIconIndirect', cursorId: 0, width: 8, height: 8, invert: 1, alpha: true, raw: 'AAAA' };
    const r = cursorCss({ cursor: 0x7fba0100, cursorIdc: 0, cursorShared: false, cursorMade: true }, pic);
    expect(r.css).toBe('url("data:image/png;base64,AAAA") 2 3, default');
    expect(r.made).toBe(true);
    expect(r.picture).toMatchObject({ source: 'made by CreateIconIndirect', cursorId: 0, hotX: 2, hotY: 3, invert: 1, alpha: true, raw: 'AAAA' });
    expect(cursorCss({ cursor: 0x7fba0100, cursorIdc: 0, cursorShared: false, cursorMade: true }, { ok: false, why: 'an icon' })).toMatchObject({ css: 'default', made: true, unpictured: true });
  });
  it('a handle that is none of these has no identity: unresolved', () => {
    expect(c(0x7fba0100)).toMatchObject({ css: 'default', unresolved: true });
  });
  it('an identifier without a CSS counterpart falls back to the arrow and is reported; no view: the arrow', () => {
    expect(c(1, 12345)).toMatchObject({ css: 'default', unmapped: { name: 'IDC 12345', value: 12345 } });
    expect(cursorCss(null).css).toBe('default');
  });
});
