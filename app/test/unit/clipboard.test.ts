import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { PageClipboard } from '../../src/engine/clipboard.ts';

// the page's status line is the one element the page makes; the document is a stand-in with the members the page uses (its listeners can be fired)
class FakeElement {
  id = '';
  hidden = false;
  textContent = '';
  readonly style = { cssText: '' };
  readonly children: FakeElement[] = [];
  private readonly attrs = new Map<string, string>();
  private readonly listeners = new Map<string, Array<() => void>>();
  setAttribute(k: string, v: string): void { this.attrs.set(k, v); }
  getAttribute(k: string): string | null { return this.attrs.get(k) ?? null; }
  append(...els: FakeElement[]): void { this.children.push(...els); }
  addEventListener(type: string, fn: () => void): void { this.listeners.set(type, [...(this.listeners.get(type) ?? []), fn]); }
  click(): void { for (const fn of this.listeners.get('click') ?? []) fn(); }
}

interface FakeDoc {
  doc: Document;
  appended: FakeElement[];
  fire(type: string, ev: Record<string, unknown>): void;
}

function fakeDocument(): FakeDoc {
  const appended: FakeElement[] = [];
  const listeners = new Map<string, Array<(ev: unknown) => void>>();
  const doc = {
    createElement: () => new FakeElement(),
    addEventListener: (type: string, fn: (ev: unknown) => void) => { listeners.set(type, [...(listeners.get(type) ?? []), fn]); },
    body: { append: (el: FakeElement) => { appended.push(el); } },
  };
  return {
    doc: doc as unknown as Document,
    appended,
    fire: (type, ev) => { for (const fn of listeners.get(type) ?? []) fn(ev); },
  };
}

const permissions = (states: Record<string, string>) => ({ query: async ({ name }: { name: string }) => ({ state: states[name] ?? 'prompt' }) });
const desktop = (text: string) => ({ target: { id: 'desktop' }, clipboardData: { getData: () => text } });

beforeEach(() => { vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] }); });
afterEach(() => { vi.useRealTimers(); vi.unstubAllGlobals(); });

describe('the questions about the formats (#1406): they read nothing and show nothing', () => {
  it('under prompt with nothing known, the clipboard is said to hold text, and no read or prompt is made', async () => {
    const readText = vi.fn(async () => 'text');
    const read = vi.fn(async () => []);
    vi.stubGlobal('navigator', { clipboard: { readText, read, writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'prompt' }) });
    const { doc, appended } = fakeDocument();
    const pc = new PageClipboard(doc);
    expect(await pc.probe()).toBe(true);
    expect(readText).not.toHaveBeenCalled();
    expect(read).not.toHaveBeenCalled();
    expect(appended).toHaveLength(0);
    expect(pc.state.message).toBeNull();
  });

  it('under denied, the clipboard is said to hold no text', async () => {
    const readText = vi.fn(async () => 'text');
    vi.stubGlobal('navigator', { clipboard: { readText, read: vi.fn(), writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'denied' }) });
    const { doc, appended } = fakeDocument();
    const pc = new PageClipboard(doc);
    expect(await pc.probe()).toBe(false);
    expect(readText).not.toHaveBeenCalled();
    expect(appended).toHaveLength(0);
  });

  it('under granted, the formats of navigator.clipboard.read() say whether there is text (no prompt, kept for a moment)', async () => {
    const read = vi.fn(async () => [{ types: ['image/png'] }]);
    vi.stubGlobal('navigator', { clipboard: { readText: vi.fn(), read, writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'granted' }) });
    const pc = new PageClipboard(fakeDocument().doc);
    expect(await pc.probe()).toBe(false);
    expect(await pc.probe()).toBe(false);
    expect(read).toHaveBeenCalledOnce();
    read.mockResolvedValueOnce([{ types: ['text/plain'] }]);
    expect(await new PageClipboard(fakeDocument().doc).probe()).toBe(true);
  });

  it('the text the page knows decides the question, before the permission does', async () => {
    vi.stubGlobal('navigator', { clipboard: { readText: vi.fn(), read: vi.fn(), writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'denied' }) });
    const pc = new PageClipboard(fakeDocument().doc);
    pc.pasted('12.5');
    expect(await pc.probe()).toBe(true);
    pc.pasted('');
    expect(await pc.probe()).toBe(false);
  });
});

describe('the paste of Ctrl+V: the paste event\'s text, with no read and no prompt', () => {
  it('the paste event of the desktop gives the text to the next GetClipboardData, even when the permission is denied', async () => {
    const readText = vi.fn(async () => 'other');
    vi.stubGlobal('navigator', { clipboard: { readText, writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'denied' }) });
    const { doc, appended, fire } = fakeDocument();
    const pc = new PageClipboard(doc);
    fire('paste', desktop('fl-text'));
    expect(await pc.read()).toBe('fl-text');
    expect(readText).not.toHaveBeenCalled();
    expect(pc.state.lastRead).toBe('fl-text');
    expect(appended).toHaveLength(0);
  });

  it('a paste event outside the desktop is not the engine\'s', async () => {
    vi.stubGlobal('navigator', { clipboard: { readText: vi.fn(async () => 'x'), writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'granted' }) });
    const { doc, fire } = fakeDocument();
    const pc = new PageClipboard(doc);
    fire('paste', { target: { id: 'name' }, clipboardData: { getData: () => 'not for the engine' } });
    expect(pc.state.pastes).toBe(0);
  });

  it('the paste text is for one GetClipboardData: the next one (a paste from the menu) reads the clipboard', async () => {
    const readText = vi.fn(async () => 'menu');
    vi.stubGlobal('navigator', { clipboard: { readText, writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'granted' }) });
    const { doc, fire } = fakeDocument();
    const pc = new PageClipboard(doc);
    fire('paste', desktop('ctrl-v'));
    expect(await pc.read()).toBe('ctrl-v');
    expect(readText).not.toHaveBeenCalled();
    expect(await pc.read()).toBe('menu');
    expect(readText).toHaveBeenCalledOnce();
  });

  it('a press of a button drops a paste text that no GetClipboardData took', async () => {
    const readText = vi.fn(async () => 'menu');
    vi.stubGlobal('navigator', { clipboard: { readText, writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'granted' }) });
    const { doc, fire } = fakeDocument();
    const pc = new PageClipboard(doc);
    fire('paste', desktop('stale'));
    fire('pointerdown', { button: 0 });
    expect(await pc.read()).toBe('menu');
  });
});

describe('the knob\'s right click asks for the value (FL\'s check), and the menu\'s paste reads', () => {
  it('after a right button release, GetClipboardData reads nothing: the known text, or a number under prompt', async () => {
    const readText = vi.fn(async () => 'secret');
    vi.stubGlobal('navigator', { clipboard: { readText, read: vi.fn(), writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'prompt' }) });
    const { doc, fire } = fakeDocument();
    const pc = new PageClipboard(doc);
    fire('pointerup', { button: 2 });
    expect(await pc.read()).toBe('0');
    expect(readText).not.toHaveBeenCalled();
    expect(pc.state.message).toBeNull();
  });

  it('a right-click question under denied gives no text', async () => {
    vi.stubGlobal('navigator', { clipboard: { readText: vi.fn(), writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'denied' }) });
    const { doc, fire } = fakeDocument();
    const pc = new PageClipboard(doc);
    fire('pointerup', { button: 2 });
    expect(await pc.read()).toBeNull();
    expect(pc.state.message).toBeNull();
  });

  it('after a left release the menu\'s paste reads: under denied it reads nothing, gives no text, and shows the reason once for 5 s', async () => {
    const readText = vi.fn(async () => 'text');
    vi.stubGlobal('navigator', { clipboard: { readText, writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'denied' }) });
    const { doc, appended, fire } = fakeDocument();
    const pc = new PageClipboard(doc);
    fire('pointerup', { button: 2 });
    fire('pointerup', { button: 0 });
    expect(await pc.read()).toBeNull();
    expect(readText).not.toHaveBeenCalled();
    expect(pc.state.message).toMatch(/clipboard-read: denied/);
    expect(appended).toHaveLength(1);
    expect(appended[0]?.getAttribute('role')).toBe('status');
    expect(appended[0]?.hidden).toBe(false);
    expect(appended[0]?.children[0]?.textContent).toBe(pc.state.message);
    vi.advanceTimersByTime(4900);
    expect(appended[0]?.hidden).toBe(false);
    vi.advanceTimersByTime(200);
    expect(appended[0]?.hidden).toBe(true);
    expect(pc.state.message).toBeNull();
  });

  it('a reason shown again while it is up is still one status line, and its hiding moves on', async () => {
    vi.stubGlobal('navigator', { clipboard: { readText: vi.fn(), writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'denied' }) });
    const { doc, appended, fire } = fakeDocument();
    const pc = new PageClipboard(doc);
    fire('pointerup', { button: 0 });
    await pc.read();
    vi.advanceTimersByTime(3000);
    await pc.read();
    expect(appended).toHaveLength(1);
    vi.advanceTimersByTime(3000);
    expect(appended[0]?.hidden).toBe(false);
    vi.advanceTimersByTime(2000);
    expect(appended[0]?.hidden).toBe(true);
  });

  it('the close button hides the reason at once', async () => {
    vi.stubGlobal('navigator', { clipboard: { readText: vi.fn(), writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'denied' }) });
    const { doc, appended, fire } = fakeDocument();
    const pc = new PageClipboard(doc);
    fire('pointerup', { button: 0 });
    await pc.read();
    const close = appended[0]?.children[1];
    expect(close?.getAttribute('aria-label')).toBe('Close');
    close?.click();
    expect(appended[0]?.hidden).toBe(true);
    expect(pc.state.message).toBeNull();
  });

  it('a menu paste under granted reads the clipboard and gives its text', async () => {
    const readText = vi.fn(async () => 'granted-text');
    vi.stubGlobal('navigator', { clipboard: { readText, writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'granted' }) });
    const { doc, appended, fire } = fakeDocument();
    const pc = new PageClipboard(doc);
    fire('pointerup', { button: 0 });
    expect(await pc.read()).toBe('granted-text');
    expect(readText).toHaveBeenCalledOnce();
    expect(appended).toHaveLength(0);
  });

  it('a read that the browser fails gives no text and names the failure', async () => {
    const readText = vi.fn(async () => { throw Object.assign(new Error('Document is not focused.'), { name: 'NotAllowedError' }); });
    vi.stubGlobal('navigator', { clipboard: { readText, writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'granted' }) });
    const { doc, fire } = fakeDocument();
    const pc = new PageClipboard(doc);
    fire('pointerup', { button: 0 });
    expect(await pc.read()).toBeNull();
    expect(pc.state.lastRead).toBeNull();
    expect(pc.state.message).toMatch(/NotAllowedError: Document is not focused/);
  });
});

describe('the earlier behaviour that stays (#1389): Copy, and a browser without a Clipboard API', () => {
  it('a browser without a Clipboard API gives no text and says so', async () => {
    vi.stubGlobal('navigator', { permissions: permissions({}) });
    const { doc } = fakeDocument();
    const pc = new PageClipboard(doc);
    expect(await pc.probe()).toBe(false);
    expect(await pc.read()).toBeNull();
    await pc.write('x');
    expect(pc.state.message).toMatch(/no Clipboard API/);
    expect(pc.state.writes).toBe(1);
  });

  it('a write the browser refuses is not made; a granted write is made, its text recorded, and the text is known to the questions', async () => {
    const writeText = vi.fn(async () => undefined);
    vi.stubGlobal('navigator', { clipboard: { readText: vi.fn(), writeText }, permissions: permissions({ 'clipboard-write': 'denied' }) });
    const { doc } = fakeDocument();
    const pc = new PageClipboard(doc);
    await pc.write('FLEX (FL)');
    expect(writeText).not.toHaveBeenCalled();
    expect(pc.state.message).toMatch(/clipboard-write: denied/);
    expect(pc.state.lastWritten).toBe('FLEX (FL)');
    vi.stubGlobal('navigator', { clipboard: { readText: vi.fn(), read: vi.fn(), writeText }, permissions: permissions({ 'clipboard-write': 'granted', 'clipboard-read': 'denied' }) });
    await pc.write('FLEX (FL)');
    expect(writeText).toHaveBeenCalledWith('FLEX (FL)');
    expect(pc.state.message).toBeNull();
    expect(pc.state.writes).toBe(2);
    expect(await pc.probe()).toBe(true);
  });

  it('refresh records both permission states, and "unknown" when the browser has no Permissions API', async () => {
    vi.stubGlobal('navigator', { clipboard: { readText: vi.fn(), writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'prompt', 'clipboard-write': 'granted' }) });
    const pc = new PageClipboard(fakeDocument().doc);
    await pc.refresh();
    expect(pc.state.readPermission).toBe('prompt');
    expect(pc.state.writePermission).toBe('granted');
    vi.stubGlobal('navigator', { clipboard: { readText: vi.fn(), writeText: vi.fn() } });
    const other = new PageClipboard(fakeDocument().doc);
    await other.refresh();
    expect(other.state.readPermission).toBe('unknown');
    expect(other.state.writePermission).toBe('unknown');
  });
});
