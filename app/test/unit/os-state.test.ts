import { describe, expect, it } from 'vitest';
import type { AppView, AvWindow } from '../../src/engine/appview.ts';
import { osStateKey, osStateOf } from '../../src/engine/os-state.ts';

const win = (o: Partial<AvWindow>): AvWindow => ({ lid: 1, hwnd: 1, flags: 0, ownerLid: 0, x: 0, y: 0, width: 10, height: 10, style: 0, exstyle: 0, thread: 0, className: 'AnyClass', title: '', visible: true, enabled: true, topmost: false, active: false, focus: false, capture: false, ...o });
const view = (windows: AvWindow[], o: Partial<AppView> = {}): AppView => ({ seq: 2, cursor: 0, cursorIdc: 0, cursorShared: false, pointerX: 0, pointerY: 0, pendingEvents: 0, windows, ...o });

describe('the operating-system state the Worker reports (decision 0223)', () => {
  it('the title is the title of the active, visible window; the class of the window plays no part', () => {
    expect(osStateOf(view([win({ title: 'Other', visible: true }), win({ title: ' FL Studio - song ', active: true, className: 'SomeOtherClass' })]), '').title).toBe('FL Studio - song');
    expect(osStateOf(view([win({ title: 'TFruityLoopsMainForm', className: 'TFruityLoopsMainForm' })]), 'kept').title).toBe('kept');
  });

  it('an inactive window never gives the title, however important its class or position', () => {
    expect(osStateOf(view([win({ title: 'Front', active: false }), win({ title: 'Back', active: false })]), 'old').title).toBe('old');
  });

  it('a hidden active window gives no title', () => {
    expect(osStateOf(view([win({ title: 'Hidden', active: true, visible: false })]), 'old').title).toBe('old');
  });

  it('an active window with an empty (or blank) title, as a menu or a tooltip has, does not blank the title', () => {
    expect(osStateOf(view([win({ title: '', active: true }), win({ title: 'Window', active: false })]), 'Window').title).toBe('Window');
    expect(osStateOf(view([win({ title: '   ', active: true })]), 'Window').title).toBe('Window');
  });

  it('the first active window with a title wins: the top of the z-order is listed first', () => {
    expect(osStateOf(view([win({ title: 'Top', active: true }), win({ title: 'Below', active: true })]), '').title).toBe('Top');
  });

  it('the cursor and the screen size are passed through; a build that does not report the screen size gives 0', () => {
    const s = osStateOf(view([], { cursor: 0x10003, cursorIdc: 32649, cursorShared: true, screenW: 2060, screenH: 1264 }), '');
    expect(s).toEqual({ cursor: 0x10003, cursorIdc: 32649, cursorShared: true, title: '', screenW: 2060, screenH: 1264 });
    expect(osStateOf(view([]), '')).toMatchObject({ screenW: 0, screenH: 0 });
  });

  it('every part of the state is part of the key: a change of any one of them is reported', () => {
    const base = osStateOf(view([win({ title: 'A', active: true })], { cursor: 1, cursorIdc: 2, cursorShared: false, screenW: 1920, screenH: 1080 }), '');
    const keys = new Set([osStateKey(base)]);
    for (const o of [{ cursor: 9 }, { cursorIdc: 9 }, { cursorShared: true }, { title: 'B' }, { screenW: 1921 }, { screenH: 1081 }]) keys.add(osStateKey({ ...base, ...o }));
    expect(keys.size).toBe(7);
    expect(osStateKey({ ...base, screenW: 1, screenH: 23 })).not.toBe(osStateKey({ ...base, screenW: 12, screenH: 3 }));
  });
});
