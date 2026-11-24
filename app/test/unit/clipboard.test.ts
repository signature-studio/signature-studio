import { afterEach, describe, expect, it, vi } from 'vitest';
import { PageClipboard } from '../../src/engine/clipboard.ts';

// the page's status line is the one element the page makes; the document is a stand-in with the two members the page uses
class FakeElement {
  id = '';
  hidden = false;
  textContent = '';
  readonly style = { cssText: '' };
  private readonly attrs = new Map<string, string>();
  setAttribute(k: string, v: string): void { this.attrs.set(k, v); }
  getAttribute(k: string): string | null { return this.attrs.get(k) ?? null; }
}

function fakeDocument(): { doc: Document; appended: FakeElement[] } {
  const appended: FakeElement[] = [];
  const doc = { createElement: () => new FakeElement(), body: { append: (el: FakeElement) => { appended.push(el); } } };
  return { doc: doc as unknown as Document, appended };
}

const permissions = (states: Record<string, string>) => ({ query: async ({ name }: { name: string }) => ({ state: states[name] ?? 'prompt' }) });

afterEach(() => { vi.unstubAllGlobals(); });

describe('the page side of the engine clipboard (#1389)', () => {
  it('a read the browser refuses gives no text, makes no read, and shows the reason in the status line', async () => {
    const readText = vi.fn(async () => 'text');
    vi.stubGlobal('navigator', { clipboard: { readText, writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'denied' }) });
    const { doc, appended } = fakeDocument();
    const pc = new PageClipboard(doc);
    expect(await pc.read()).toBeNull();
    expect(readText).not.toHaveBeenCalled();
    expect(pc.state.readPermission).toBe('denied');
    expect(pc.state.message).toMatch(/clipboard-read: denied/);
    expect(appended).toHaveLength(1);
    expect(appended[0]?.getAttribute('role')).toBe('status');
    expect(appended[0]?.hidden).toBe(false);
    expect(appended[0]?.textContent).toBe(pc.state.message);
  });

  it('a granted read gives the text, records it with its time, and hides the reason again', async () => {
    const readText = vi.fn(async () => 'fl-text');
    vi.stubGlobal('navigator', { clipboard: { readText, writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'granted' }) });
    const { doc } = fakeDocument();
    const pc = new PageClipboard(doc);
    expect(await pc.read()).toBe('fl-text');
    expect(readText).toHaveBeenCalledOnce();
    expect(pc.state.lastRead).toBe('fl-text');
    expect(typeof pc.state.lastReadAt).toBe('number');
    expect(pc.state.message).toBeNull();
  });

  it('a browser without a Clipboard API gives no text and says so', async () => {
    vi.stubGlobal('navigator', { permissions: permissions({}) });
    const { doc } = fakeDocument();
    const pc = new PageClipboard(doc);
    expect(await pc.read()).toBeNull();
    await pc.write('x');
    expect(pc.state.message).toMatch(/no Clipboard API/);
    expect(pc.state.writes).toBe(1);
  });

  it('a read that the browser fails gives no text and names the failure', async () => {
    const readText = vi.fn(async () => { throw Object.assign(new Error('Document is not focused.'), { name: 'NotAllowedError' }); });
    vi.stubGlobal('navigator', { clipboard: { readText, writeText: vi.fn() }, permissions: permissions({ 'clipboard-read': 'granted' }) });
    const { doc } = fakeDocument();
    const pc = new PageClipboard(doc);
    expect(await pc.read()).toBeNull();
    expect(pc.state.lastRead).toBeNull();
    expect(pc.state.message).toMatch(/NotAllowedError: Document is not focused/);
  });

  it('a write the browser refuses is not made; a granted write is made and its text recorded', async () => {
    const writeText = vi.fn(async () => undefined);
    vi.stubGlobal('navigator', { clipboard: { readText: vi.fn(), writeText }, permissions: permissions({ 'clipboard-write': 'denied' }) });
    const { doc } = fakeDocument();
    const pc = new PageClipboard(doc);
    await pc.write('FLEX (FL)');
    expect(writeText).not.toHaveBeenCalled();
    expect(pc.state.message).toMatch(/clipboard-write: denied/);
    expect(pc.state.lastWritten).toBe('FLEX (FL)');
    vi.stubGlobal('navigator', { clipboard: { readText: vi.fn(), writeText }, permissions: permissions({ 'clipboard-write': 'granted' }) });
    await pc.write('FLEX (FL)');
    expect(writeText).toHaveBeenCalledWith('FLEX (FL)');
    expect(pc.state.message).toBeNull();
    expect(pc.state.writes).toBe(2);
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
