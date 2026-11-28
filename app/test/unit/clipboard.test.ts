import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { PageClipboard } from '../../src/engine/clipboard.ts';

// the window of the page is a stand-in: its listeners can be fired (the page adds no element and no paste listener: Issue #1482)
class FakeWindow {
  private readonly listeners = new Map<string, Array<(ev: unknown) => void>>();
  addEventListener(type: string, fn: (ev: unknown) => void): void { this.listeners.set(type, [...(this.listeners.get(type) ?? []), fn]); }
  fire(type: string, ev: Record<string, unknown> = {}): void { for (const fn of this.listeners.get(type) ?? []) fn(ev); }
  types(): string[] { return [...this.listeners.keys()]; }
}

const permissions = (states: Record<string, string>) => ({ query: async ({ name }: { name: string }) => ({ state: states[name] ?? 'prompt' }) });
const flush = async (): Promise<void> => { for (let i = 0; i < 50; i++) await Promise.resolve(); };

function page(states: Record<string, string>, api: Record<string, unknown>) {
  vi.stubGlobal('navigator', { clipboard: { writeText: vi.fn(async () => undefined), ...api }, permissions: permissions(states) });
  const win = new FakeWindow();
  const pc = new PageClipboard();
  pc.attach(win as unknown as Window);
  return { pc, win };
}

beforeEach(() => { vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] }); });
afterEach(() => { vi.useRealTimers(); vi.unstubAllGlobals(); vi.restoreAllMocks(); });

describe('the page listens to no paste event and shows nothing (#1482)', () => {
  it('attach registers the press, key, release and focus listeners, and no paste listener', () => {
    const { win } = page({}, { readText: vi.fn() });
    expect(win.types().sort()).toEqual(['focus', 'keydown', 'pointerdown', 'pointerup']);
  });

  it('a refused read gives no text and records the reason only', async () => {
    const readText = vi.fn(async () => 'x');
    const { pc, win } = page({ 'clipboard-read': 'denied' }, { readText });
    win.fire('keydown', { code: 'KeyV' });
    expect(await pc.read()).toBeNull();
    expect(readText).not.toHaveBeenCalled();
    expect(pc.state.refusal).toBe('clipboard-read: denied');
  });
});

describe('the questions about the formats (#1406): they read nothing', () => {
  it('under prompt with nothing known, the clipboard is said to hold text, and no read or prompt is made', async () => {
    const readText = vi.fn(async () => 'text');
    const { pc } = page({ 'clipboard-read': 'prompt' }, { readText, read: vi.fn() });
    expect(await pc.probe()).toBe(true);
    expect(readText).not.toHaveBeenCalled();
  });

  it('under granted with nothing known, a question reads nothing either', async () => {
    const readText = vi.fn(async () => 'text');
    const { pc } = page({ 'clipboard-read': 'granted' }, { readText });
    expect(await pc.probe()).toBe(true);
    expect(readText).not.toHaveBeenCalled();
  });

  it('under denied, the clipboard is said to hold no text', async () => {
    const readText = vi.fn(async () => 'text');
    const { pc } = page({ 'clipboard-read': 'denied' }, { readText });
    expect(await pc.probe()).toBe(false);
    expect(readText).not.toHaveBeenCalled();
  });

  it('the text the page knows decides the question, before the permission does', async () => {
    const { pc } = page({ 'clipboard-read': 'denied' }, { readText: vi.fn() });
    await pc.write('12.5');
    expect(await pc.probe()).toBe(true);
    await pc.write('');
    expect(await pc.probe()).toBe(false);
  });
});

describe('a paste: GetClipboardData reads the clipboard through readText', () => {
  it('Ctrl+V (a key, then the engine asks) reads the clipboard text', async () => {
    const readText = vi.fn(async () => 'fl-text');
    const { pc, win } = page({ 'clipboard-read': 'granted' }, { readText });
    win.fire('keydown', { code: 'KeyV' });
    expect(await pc.read()).toBe('fl-text');
    expect(readText).toHaveBeenCalledOnce();
    expect(pc.state.lastRead).toBe('fl-text');
  });

  it('a paste from the menu (a left click, then the engine asks) reads the clipboard text', async () => {
    const readText = vi.fn(async () => 'menu');
    const { pc, win } = page({ 'clipboard-read': 'granted' }, { readText });
    win.fire('pointerup', { button: 0 });
    expect(await pc.read()).toBe('menu');
    expect(readText).toHaveBeenCalledOnce();
  });

  it('a key after a right release is not the right-click question: Ctrl+V after a right click reads', async () => {
    const readText = vi.fn(async () => 'paste');
    const { pc, win } = page({ 'clipboard-read': 'granted' }, { readText });
    win.fire('pointerup', { button: 2 });
    win.fire('keydown', { code: 'KeyV' });
    expect(await pc.read()).toBe('paste');
    expect(readText).toHaveBeenCalledOnce();
  });

  it('a read the browser fails gives no text and names the failure', async () => {
    const readText = vi.fn(async () => { throw Object.assign(new Error('Document is not focused.'), { name: 'NotAllowedError' }); });
    const { pc, win } = page({ 'clipboard-read': 'granted' }, { readText });
    win.fire('pointerup', { button: 0 });
    expect(await pc.read()).toBeNull();
    expect(pc.state.lastRead).toBeNull();
    expect(pc.state.refusal).toMatch(/NotAllowedError: Document is not focused/);
  });

  it('a browser without a Clipboard API gives no text and records it', async () => {
    vi.stubGlobal('navigator', { permissions: permissions({}) });
    const pc = new PageClipboard();
    pc.attach(new FakeWindow() as unknown as Window);
    expect(await pc.probe()).toBe(false);
    expect(await pc.read()).toBeNull();
    await pc.write('x');
    expect(pc.state.refusal).toMatch(/no Clipboard API/);
    expect(pc.state.writes).toBe(1);
  });
});

describe('FL\'s right-click question (#1406): nothing is read', () => {
  it('after a right button release, GetClipboardData reads nothing: the known text, or a number under prompt', async () => {
    const readText = vi.fn(async () => 'secret');
    const { pc, win } = page({ 'clipboard-read': 'prompt' }, { readText, read: vi.fn() });
    win.fire('pointerup', { button: 2 });
    expect(await pc.read()).toBe('0');
    expect(readText).not.toHaveBeenCalled();
    expect(pc.state.refusal).toBeNull();
  });

  it('the known text answers the question', async () => {
    const readText = vi.fn(async () => 'secret');
    const { pc, win } = page({ 'clipboard-read': 'granted' }, { readText });
    await pc.write('12');
    win.fire('pointerup', { button: 2 });
    expect(await pc.read()).toBe('12');
    expect(readText).not.toHaveBeenCalled();
  });

  it('a right-click question under denied gives no text and reads nothing', async () => {
    const readText = vi.fn(async () => 'x');
    const { pc, win } = page({ 'clipboard-read': 'denied' }, { readText });
    win.fire('pointerup', { button: 2 });
    expect(await pc.read()).toBeNull();
    expect(readText).not.toHaveBeenCalled();
  });

  it('the question is over after a moment: a later read is a paste', async () => {
    let now = 1000;
    vi.spyOn(performance, 'now').mockImplementation(() => now);
    const readText = vi.fn(async () => 'later');
    const { pc, win } = page({ 'clipboard-read': 'granted' }, { readText });
    win.fire('pointerup', { button: 2 });
    now += 1600;
    expect(await pc.read()).toBe('later');
    expect(readText).toHaveBeenCalledOnce();
  });
});

describe('the permission is asked at the load (#1482)', () => {
  it('under prompt, the page calls readText once at its load (the browser asks the person)', async () => {
    const readText = vi.fn(async () => 'first');
    const { pc } = page({ 'clipboard-read': 'prompt' }, { readText });
    await pc.start();
    expect(readText).toHaveBeenCalledOnce();
    expect(pc.state.refusal).toBeNull();
  });

  it('under granted, the load reads the text the page knows (and a focus reads it again)', async () => {
    const readText = vi.fn(async () => 'kept');
    const { pc, win } = page({ 'clipboard-read': 'granted' }, { readText });
    await pc.start();
    expect(readText).toHaveBeenCalledOnce();
    win.fire('focus');
    await flush();
    expect(readText).toHaveBeenCalledTimes(2);
  });

  it('under prompt, a focus reads nothing (the request is not repeated)', async () => {
    const readText = vi.fn(async () => 'x');
    const { pc, win } = page({ 'clipboard-read': 'prompt' }, { readText });
    await pc.start();
    win.fire('focus');
    await flush();
    expect(readText).toHaveBeenCalledOnce();
  });

  it('under denied, the load asks for nothing', async () => {
    const readText = vi.fn(async () => 'x');
    const { pc } = page({ 'clipboard-read': 'denied' }, { readText });
    await pc.start();
    expect(readText).not.toHaveBeenCalled();
  });

  it('a request the browser refuses at the load (no focus) is made again at the first press or key, once', async () => {
    const readText = vi.fn(async () => { throw Object.assign(new Error('Document is not focused.'), { name: 'NotAllowedError' }); });
    const { pc, win } = page({ 'clipboard-read': 'prompt' }, { readText });
    await pc.start();
    expect(readText).toHaveBeenCalledOnce();
    expect(pc.state.refusal).toMatch(/NotAllowedError/);
    win.fire('pointerdown', { button: 0 });
    await flush();
    expect(readText).toHaveBeenCalledTimes(2);
    win.fire('keydown', { code: 'KeyA' });
    win.fire('pointerdown', { button: 0 });
    await flush();
    expect(readText).toHaveBeenCalledTimes(2);
  });

  it('a right press makes no request (#1503): the one refused at the load waits for a left press or a key', async () => {
    const readText = vi.fn(async () => { throw Object.assign(new Error('Document is not focused.'), { name: 'NotAllowedError' }); });
    const { pc, win } = page({ 'clipboard-read': 'prompt' }, { readText });
    await pc.start();
    win.fire('pointerdown', { button: 2 });
    win.fire('pointerup', { button: 2 });
    await flush();
    expect(readText).toHaveBeenCalledOnce();
    expect(await pc.read()).toBe('0');
    expect(readText).toHaveBeenCalledOnce();
    win.fire('pointerdown', { button: 0 });
    await flush();
    expect(readText).toHaveBeenCalledTimes(2);
  });

  it('under granted, a focus while the right button is down reads nothing (#1503)', async () => {
    const readText = vi.fn(async () => 'kept');
    const { pc, win } = page({ 'clipboard-read': 'granted' }, { readText });
    await pc.start();
    win.fire('pointerdown', { button: 2 });
    win.fire('focus');
    await flush();
    expect(readText).toHaveBeenCalledOnce();
    win.fire('pointerup', { button: 2 });
    win.fire('focus');
    await flush();
    expect(readText).toHaveBeenCalledTimes(2);
  });

  it('refresh records both permission states, and "unknown" when the browser has no Permissions API', async () => {
    const { pc } = page({ 'clipboard-read': 'prompt', 'clipboard-write': 'granted' }, {});
    await pc.refresh();
    expect(pc.state.readPermission).toBe('prompt');
    expect(pc.state.writePermission).toBe('granted');
    vi.stubGlobal('navigator', { clipboard: { readText: vi.fn(), writeText: vi.fn() } });
    const other = new PageClipboard();
    await other.refresh();
    expect(other.state.readPermission).toBe('unknown');
    expect(other.state.writePermission).toBe('unknown');
  });
});

describe('Copy (#1389) and its refusal, silent', () => {
  it('a write the browser refuses is not made; a granted write is made, its text recorded, and the text is known to the questions', async () => {
    const writeText = vi.fn(async () => undefined);
    const { pc } = page({ 'clipboard-write': 'denied' }, { writeText });
    await pc.write('FLEX (FL)');
    expect(writeText).not.toHaveBeenCalled();
    expect(pc.state.refusal).toBe('clipboard-write: denied');
    expect(pc.state.lastWritten).toBe('FLEX (FL)');
    vi.stubGlobal('navigator', { clipboard: { readText: vi.fn(), writeText }, permissions: permissions({ 'clipboard-write': 'granted', 'clipboard-read': 'denied' }) });
    await pc.write('FLEX (FL)');
    expect(writeText).toHaveBeenCalledWith('FLEX (FL)');
    expect(pc.state.refusal).toBeNull();
    expect(pc.state.writes).toBe(2);
    expect(await pc.probe()).toBe(true);
  });

  it('a write the browser fails names the failure in state.refusal only', async () => {
    const writeText = vi.fn(async () => { throw Object.assign(new Error('not focused'), { name: 'NotAllowedError' }); });
    const { pc } = page({ 'clipboard-write': 'granted' }, { writeText });
    await pc.write('x');
    expect(pc.state.refusal).toMatch(/NotAllowedError: not focused/);
  });
});
