import { describe, expect, it } from 'vitest';
import { suppressContextMenu } from '../../src/input/context-menu.ts';

describe('the browser\'s context menu is cancelled on the page, whatever the page is doing (#1482)', () => {
  it('a contextmenu on the target is cancelled', () => {
    const target = new EventTarget();
    suppressContextMenu(target);
    const e = new Event('contextmenu', { cancelable: true });
    target.dispatchEvent(e);
    expect(e.defaultPrevented).toBe(true);
  });

  it('the listener is a capture listener (it runs before the listeners of the elements)', () => {
    const rec: { type?: string; capture?: unknown } = {};
    suppressContextMenu({ addEventListener: (type: string, _fn: EventListener, opts?: unknown) => { rec.type = type; rec.capture = opts; } });
    expect(rec.type).toBe('contextmenu');
    expect(rec.capture).toBe(true);
  });
});
