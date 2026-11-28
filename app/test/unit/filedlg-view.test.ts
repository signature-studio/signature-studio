import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { SAVE_FOLDER, type FileDlgRequest } from '../../src/engine/filedlg.ts';
import { FileDialogView, SavedFilesView, type FileDlgHost } from '../../src/ui/filedlg.ts';

// Issue #1519: the engine's Open / Save dialog is answered by the page's file manager window when the host has one (select); no browser chooser is opened then.
// The DOM is a small stand-in here (the unit tests run in node): what the view makes, the clicks on what it made, and the listeners it adds to the document.

interface FakeEl {
  tag: string;
  attrs: Record<string, string>;
  children: unknown[];
  style: { cssText: string };
  dataset: Record<string, string>;
  clicks: number;
  setAttribute(k: string, v: string): void;
  addEventListener(t: string, f: () => void): void;
  click(): void;
  replaceChildren(...n: unknown[]): void;
  append(...n: unknown[]): void;
  remove(): void;
}

let made: FakeEl[] = [];
let docListeners: Map<string, number> = new Map();

const fakeEl = (tag: string): FakeEl => ({
  tag, attrs: {}, children: [], style: { cssText: '' }, dataset: {}, clicks: 0,
  setAttribute(k, v) { this.attrs[k] = v; },
  addEventListener() {},
  click() { this.clicks++; },
  replaceChildren(...n) { this.children = n; },
  append(...n) { this.children.push(...n); },
  remove() {},
});

const fakeDocument = {
  createElement(tag: string): FakeEl { const e = fakeEl(tag); made.push(e); return e; },
  addEventListener(t: string) { docListeners.set(t, (docListeners.get(t) ?? 0) + 1); },
  removeEventListener(t: string) { docListeners.set(t, (docListeners.get(t) ?? 0) - 1); },
};

const FILTERS: [string, string][] = [['FL Studio song file (*.flp)', '*.flp'], ['MIDI file (*.mid;*.midi)', '*.mid;*.midi']];
const openReq = (o: Partial<FileDlgRequest> = {}): FileDlgRequest => ({ save: false, options: 0x1000, filterIndex: 1, title: null, okLabel: null, fileName: null, defaultExt: null, folder: null, filters: FILTERS, ...o });
const saveReq = (o: Partial<FileDlgRequest> = {}): FileDlgRequest => openReq({ save: true, options: 0x2, fileName: 'Song', defaultExt: 'flp', ...o });

const setActivation = (isActive: boolean): void => { vi.stubGlobal('navigator', { userActivation: { isActive } }); };

function setup(extra: Partial<FileDlgHost> = {}) {
  const answers: [number, unknown][] = [];
  const closed = vi.fn();
  const root = fakeEl('div') as unknown as HTMLElement;
  const host: FileDlgHost = { answer: (id, a) => { answers.push([id, a]); }, closed, ...extra };
  const view = new FileDialogView(root, host);
  return { view, root: root as unknown as FakeEl, answers, closed };
}

beforeEach(() => {
  made = [];
  docListeners = new Map();
  vi.stubGlobal('document', fakeDocument);
  setActivation(true);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('the file manager window answers the engine (Issue #1519)', () => {
  it('Open: the window is built, and no browser chooser is opened (no input, no click, no gesture listener), even when a gesture was just made', () => {
    const select = vi.fn((_id: number, _req: FileDlgRequest, _root: HTMLElement, _done: (a: unknown) => void) => () => {});
    const { view, root } = setup({ select: select as never });
    view.show(7, openReq());
    expect(select).toHaveBeenCalledTimes(1);
    expect(select.mock.calls[0]?.[0]).toBe(7);
    expect(select.mock.calls[0]?.[2]).toBe(root);
    expect(made.some((e) => e.tag === 'input')).toBe(false);
    expect(made.some((e) => e.clicks > 0)).toBe(false);
    expect([...docListeners.values()].every((n) => n === 0)).toBe(true);
    expect(root.dataset.select).toBe('1');
    expect(view.currentId).toBe(7);
  });

  it('the answer of the window goes to the engine once; the dialog is over and the keyboard goes back to the desktop', () => {
    const select = vi.fn((_id: number, _req: FileDlgRequest, _root: HTMLElement, _done: (a: unknown) => void) => () => {});
    const { view, root, answers, closed } = setup({ select: select as never });
    view.show(7, openReq());
    const done = select.mock.calls[0]?.[3] as (a: unknown) => void;
    const file = new File(['abc'], 'Song.flp');
    done({ ok: true, items: [{ role: 'R', path: 'C:\\Users\\user\\Downloads\\Song.flp', file }] });
    expect(answers).toEqual([[7, { ok: true, items: [{ role: 'R', path: 'C:\\Users\\user\\Downloads\\Song.flp', file }] }]]);
    expect(view.open).toBe(false);
    expect(root.dataset.select).toBeUndefined();
    expect(root.children).toEqual([]);
    expect(closed).toHaveBeenCalledTimes(1);
    expect(view.history).toEqual([{ id: 7, kind: 'open', outcome: 'answered', files: 1, bytes: 3 }]);
    done({ ok: false, reason: 'late' });
    expect(answers.length).toBe(1);
  });

  it('a Save goes to the window too: the page does not answer the engine with a suggested name', () => {
    const select = vi.fn((_id: number, _req: FileDlgRequest, _root: HTMLElement, _done: (a: unknown) => void) => () => {});
    const { view, answers } = setup({ select: select as never });
    view.show(8, saveReq());
    expect(select).toHaveBeenCalledTimes(1);
    expect(select.mock.calls[0]?.[1]).toMatchObject({ save: true, fileName: 'Song' });
    expect(answers).toEqual([]);
    const done = select.mock.calls[0]?.[3] as (a: unknown) => void;
    done({ ok: true, items: [{ role: 'S', path: `${SAVE_FOLDER}\\Song.flp` }] });
    expect(answers).toEqual([[8, { ok: true, items: [{ role: 'S', path: `${SAVE_FOLDER}\\Song.flp` }] }]]);
    expect(view.history[0]).toMatchObject({ id: 8, kind: 'save', outcome: 'answered' });
  });

  it('a window that is cancelled answers the engine with the reason, as cancelled', () => {
    const select = vi.fn((_id: number, _req: FileDlgRequest, _root: HTMLElement, _done: (a: unknown) => void) => () => {});
    const { view, answers } = setup({ select: select as never });
    view.show(9, openReq());
    const done = select.mock.calls[0]?.[3] as (a: unknown) => void;
    done({ ok: false, reason: 'cancelled by the person' });
    expect(answers).toEqual([[9, { ok: false, reason: 'cancelled by the person' }]]);
    expect(view.history[0]).toMatchObject({ id: 9, outcome: 'cancelled' });
  });

  it('when the engine ends, the window is taken down (its disposer runs, its nodes are removed)', () => {
    const dispose = vi.fn();
    const select = vi.fn((_id: number, _req: FileDlgRequest, _root: HTMLElement, _done: (a: unknown) => void) => dispose);
    const { view, root } = setup({ select: select as never });
    view.show(10, openReq());
    view.hide();
    expect(dispose).toHaveBeenCalledTimes(1);
    expect(root.children).toEqual([]);
    expect(root.dataset.select).toBeUndefined();
    expect(view.open).toBe(false);
  });
});

describe('without a window the view is as before (Issue #1481)', () => {
  it('Open with a gesture just made: the browser chooser is opened at once (a hidden file input, several files)', () => {
    const { view } = setup();
    view.show(11, openReq());
    const input = made.find((e) => e.tag === 'input');
    expect(input).toBeDefined();
    expect(input?.attrs.type).toBe('file');
    expect(input?.attrs.id).toBe('filedlg-input');
    expect(input?.attrs.multiple).toBe('');
    expect(input?.clicks).toBe(1);
  });

  it('Open with no gesture just made: the chooser waits for the next key press or click on the page', () => {
    setActivation(false);
    const { view } = setup();
    view.show(12, openReq());
    const input = made.find((e) => e.tag === 'input');
    expect(input?.clicks).toBe(0);
    expect(docListeners.get('pointerdown')).toBe(1);
    expect(docListeners.get('keydown')).toBe(1);
  });

  it('Save: the engine\'s suggested name is answered at once (no window to ask)', () => {
    const { view, answers } = setup();
    view.show(13, saveReq());
    expect(answers).toEqual([[13, { ok: true, items: [{ role: 'S', path: `${SAVE_FOLDER}\\Song.flp` }] }]]);
    expect(made.some((e) => e.tag === 'input')).toBe(false);
  });

  it('a folder dialog is refused when there is no window (the page has no folder of the engine\'s volume to offer)', () => {
    const { view, answers } = setup();
    view.show(14, openReq({ options: 0x20 }));
    expect(answers[0]?.[0]).toBe(14);
    expect(answers[0]?.[1]).toMatchObject({ ok: false });
  });
});

describe('SavedFilesView', () => {
  it('a written file is kept, and is handed to the download only when the person did not choose its place in the window', () => {
    const saved = new SavedFilesView();
    const data = new Uint8Array([1, 2, 3]).buffer;
    saved.add({ name: 'a.flp', guestPath: 'C:\\Users\\user\\Downloads\\a.flp', size: 3, data }, false);
    expect(saved.files.map((f) => f.name)).toEqual(['a.flp']);
    expect(made.some((e) => e.tag === 'a')).toBe(false);
  });
});
