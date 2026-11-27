import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { Progress } from '../../src/bundle/loader.ts';
import { fractionOf, ProgressView } from '../../src/ui/progress.ts';
import { LoadingModal } from '../../src/ui/loading-modal.ts';

// The modal's DOM as a small tree of fake elements: what ProgressView and LoadingModal use (createElement, append, dataset, setAttribute, remove)
class FakeEl {
  readonly tag: string;
  readonly children: FakeEl[] = [];
  parent: FakeEl | null = null;
  readonly dataset: Record<string, string> = {};
  readonly attrs = new Map<string, string>();
  textContent = '';
  value = 0;
  max = 0;
  constructor(tag: string) { this.tag = tag; }
  append(...kids: FakeEl[]): void { for (const k of kids) { k.remove(); k.parent = this; this.children.push(k); } }
  setAttribute(k: string, v: string): void { this.attrs.set(k, v); }
  remove(): void {
    if (this.parent) this.parent.children.splice(this.parent.children.indexOf(this), 1);
    this.parent = null;
  }
  /** true when the element is in the tree under the body */
  inBody(): boolean {
    for (let e: FakeEl | null = this; e; e = e.parent) if (e.tag === 'body') return true;
    return false;
  }
}

let body: FakeEl;
const saved = (globalThis as Record<string, unknown>).document;
beforeEach(() => {
  body = new FakeEl('body');
  (globalThis as Record<string, unknown>).document = { createElement: (tag: string) => new FakeEl(tag) };
});
afterEach(() => { (globalThis as Record<string, unknown>).document = saved; });

const ASSETS = ['engine wasm', 'seed pack', 'DLL pack', 'engine glue'];
const p = (label: string, loaded: number, total: number, done = false, at = 0): Progress => ({ label, url: `/x/${label}`, loaded, total, done, at });

/** a row as index.html has it: a list item with its label, a span and a progressbar */
function staticRow(label: string): FakeEl {
  const li = new FakeEl('li');
  li.dataset.label = label;
  const bar = new FakeEl('progress');
  bar.setAttribute('role', 'progressbar');
  li.append(new FakeEl('span'), bar);
  return li;
}

/** the modal as the page has it: a root in the body, with its list of progress rows (the four of index.html, all at 0 from the start) */
function modalInPage(): { root: FakeEl; list: FakeEl; modal: LoadingModal } {
  const root = new FakeEl('div');
  const list = new FakeEl('ul');
  for (const label of ASSETS) list.append(staticRow(label));
  root.append(list);
  body.append(root);
  return { root, list, modal: new LoadingModal(root as unknown as HTMLElement, new ProgressView(list as unknown as HTMLElement, ASSETS)) };
}

describe('the start-up modal (Issue #1520)', () => {
  it('gives the share of an asset that has arrived, from 0 to 1', () => {
    expect(fractionOf(p('engine wasm', 0, 200))).toBe(0);
    expect(fractionOf(p('engine wasm', 50, 200))).toBe(0.25);
    expect(fractionOf(p('engine wasm', 200, 200))).toBe(1);
    expect(fractionOf(p('engine wasm', 300, 200))).toBe(1);
    expect(fractionOf(p('engine glue', 10, 0))).toBe(0);
    expect(fractionOf(p('engine glue', 0, 0, true))).toBe(1);
  });

  it('shows the four downloads from the start, at 0, each a progressbar (the rows of index.html are used)', () => {
    const { list, modal } = modalInPage();
    expect(modal.values()).toEqual(ASSETS.map((label) => ({ label, value: 0 })));
    expect(list.children.map((li) => li.dataset.label)).toEqual(ASSETS);
    for (const li of list.children) {
      const bar = li.children[1];
      expect(bar.attrs.get('role')).toBe('progressbar');
      expect(bar.max).toBe(1);
    }
  });

  it('makes the rows that the page does not have, in the order given', () => {
    const list = new FakeEl('ul');
    const view = new ProgressView(list as unknown as HTMLElement, ASSETS);
    expect(list.children.map((li) => li.dataset.label)).toEqual(ASSETS);
    expect(view.values()).toEqual(ASSETS.map((label) => ({ label, value: 0 })));
  });

  it('moves a bar with each progress report, and a finished asset is full', () => {
    const { modal } = modalInPage();
    modal.update(p('engine wasm', 50, 200, false, 1));
    modal.update(p('seed pack', 30, 60, false, 1));
    modal.update(p('DLL pack', 9, 9, true, 2));
    expect(modal.values()).toEqual([
      { label: 'engine wasm', value: 0.25 },
      { label: 'seed pack', value: 0.5 },
      { label: 'DLL pack', value: 1 },
      { label: 'engine glue', value: 0 },
    ]);
  });

  it('is removed from the DOM by close(), leaves nothing behind, and ignores later reports', () => {
    const { root, list, modal } = modalInPage();
    expect(root.inBody()).toBe(true);
    expect(modal.open).toBe(true);
    modal.close();
    expect(modal.open).toBe(false);
    expect(root.inBody()).toBe(false);
    expect(body.children).toEqual([]);
    modal.update(p('engine glue', 5, 5, true));
    expect(list.children[3].children[1].value).toBe(0);
    modal.close();
    expect(body.children).toEqual([]);
  });
});
