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
  it('a handle that is none of these has no identity: unresolved', () => {
    expect(c(0x7fba0100)).toMatchObject({ css: 'default', unresolved: true });
  });
  it('an identifier without a CSS counterpart falls back to the arrow and is reported; no view: the arrow', () => {
    expect(c(1, 12345)).toMatchObject({ css: 'default', unmapped: { name: 'IDC 12345', value: 12345 } });
    expect(cursorCss(null).css).toBe('default');
  });
});
