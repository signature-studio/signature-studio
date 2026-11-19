import { describe, expect, it } from 'vitest';
import { cursorCss, IDC_CSS } from '../../src/ui/cursor.ts';

const c = (cursor: number, idc = 0, shared = false) => cursorCss({ cursor, cursorIdc: idc, cursorShared: shared });

describe('the guest\'s cursor as a CSS cursor', () => {
  it('the standard cursors of Learn "About Cursors" (numbers of lift/runtime/fle_cursor.h)', () => {
    expect(c(0x7fca0004, 32512).css).toBe('default');
    expect(c(1, 32513).css).toBe('text');
    expect(c(1, 32514).css).toBe('wait');
    expect(c(1, 32515).css).toBe('crosshair');
    expect(c(1, 32642).css).toBe('nwse-resize');
    expect(c(1, 32643).css).toBe('nesw-resize');
    expect(c(1, 32644).css).toBe('ew-resize');
    expect(c(1, 32645).css).toBe('ns-resize');
    expect(c(1, 32646).css).toBe('move');
    expect(c(1, 32648).css).toBe('not-allowed');
    expect(c(1, 32649).css).toBe('pointer');
    expect(c(1, 32650).css).toBe('progress');
    expect(c(1, 32651).css).toBe('help');
  });
  it('width and height resize cursors are not swapped (SIZEWE is horizontal: ew-resize; SIZENS is vertical: ns-resize)', () => {
    expect(IDC_CSS[32644]).toBe('ew-resize');
    expect(IDC_CSS[32645]).toBe('ns-resize');
  });
  it('SetCursor(NULL) removes the cursor: none', () => {
    expect(c(0).css).toBe('none');
    expect(c(0).why).toContain('NULL');
  });
  it('a cursor of the guest\'s own modules, or one it made, is shown as the arrow and the reason says which', () => {
    expect(c(0x7fda5008, 0, true)).toMatchObject({ css: 'default' });
    expect(c(0x7fda5008, 0, true).why).toContain('module');
    expect(c(0x7fba0100).why).toContain('made itself');
  });
  it('an identifier without a CSS counterpart falls back to the arrow and says so; no view: the arrow', () => {
    expect(c(1, 12345)).toMatchObject({ css: 'default' });
    expect(cursorCss(null).css).toBe('default');
  });
});
