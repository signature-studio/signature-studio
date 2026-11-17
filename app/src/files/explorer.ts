// The file manager: folder tree + list + breadcrumb over the browser's OPFS, in plain DOM (decision 0216, opfs1). Not part of FL Studio (it has no file manager); it is the page's own, so that
// the person can put a project (a folder or a zip) into the browser's storage and the engine's Open / Save can later choose from it. Everything stays in this browser; nothing is uploaded.
// Modal questions are DOM elements inside the window (not window.confirm), so a test can answer them and the keyboard stays in the window.
import { fromEvent } from 'file-selector';
import { baseName, extOf, formatBytes, isInside, joinPath, nameProblem, normalizePath, parentOf, protectedHit, splitPath, uniqueName } from './paths.ts';
import { type FsEntry, OpfsFs, describeError, memoryNotice, roomLeft } from './opfs.ts';
import { filesUsage } from './usage.ts';
import { matchesFilter, saveExtension, type SelectConfig, type SelectResult } from './select.ts';
import { type Conflict, type Incoming, type OpContext, type OpProgress, type OpResult, extractZip, folderToZip, newResult, planZip, putFiles, saveBlob, type ZipPlan } from './ops.ts';

const el = <K extends keyof HTMLElementTagNameMap>(tag: K, attrs: Record<string, string> = {}, ...kids: (Node | string)[]): HTMLElementTagNameMap[K] => {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  for (const k of kids) e.append(k);
  return e;
};

export interface StorageInfo { usage: number | null; quota: number | null; persisted: boolean | null }

export async function storageInfo(): Promise<StorageInfo> {
  const s = navigator.storage;
  let usage: number | null = null, quota: number | null = null, persisted: boolean | null = null;
  try { const e = await s.estimate(); usage = e.usage ?? null; quota = e.quota ?? null; } catch { /* unknown */ }
  try { persisted = await s.persisted(); } catch { /* unknown */ }
  return { usage, quota, persisted };
}

const ICON = { directory: '\u{1F4C1}', file: '\u{1F4C4}' };

export interface ExplorerHost {
  /** the window asked to be closed (Escape / the close button) */
  close(): void;
}

export class Explorer {
  readonly fs: OpfsFs;
  readonly root: HTMLElement;
  cwd = '/';
  entries: FsEntry[] = [];
  readonly selected = new Set<string>();
  clipboard: { mode: 'copy' | 'cut'; paths: string[] } | null = null;
  lastResult: OpResult | null = null;
  /** for a test: answers the conflict question without a click (null = ask the person) */
  conflictPolicy: Conflict | 'cancel' | null = null;
  /** for a test: how a zip dropped with other files is treated without asking (null = ask) */
  zipPolicy: 'extract' | 'keep' | null = null;
  busy = false;
  private anchor = '';
  private readonly host: ExplorerHost;
  private readonly expanded = new Set<string>(['/']);
  private abort: AbortController | null = null;
  private readonly dom: Record<string, HTMLElement> = {};
  private modalOpen = false;
  private navToken = 0;
  /** the navigation whose sizes and dates are all shown (for the tests) */
  statsDone = 0;
  /** the number of navigations so far (the next one gets navSeq + 1) */
  get navSeq(): number { return this.navToken; }
  private readonly rowEls = new Map<string, HTMLElement>();
  /** every status line shown, for the tests and the page's log */
  readonly log: string[] = [];

  /** decision 0216 stage 2: set when the window is the engine's file dialog (selection mode); null = the plain file manager */
  readonly select: { config: SelectConfig; done(r: SelectResult | null): void } | null;
  private filterIndex = 0;
  private finished = false;

  constructor(root: HTMLElement, fs: OpfsFs, host: ExplorerHost, select?: { config: SelectConfig; done(r: SelectResult | null): void }) {
    this.root = root;
    this.fs = fs;
    this.host = host;
    this.select = select ?? null;
    this.filterIndex = select?.config.filterIndex ?? 0;
    this.build();
  }

  // ---------------------------------------------------------------- structure
  private btn(id: string, label: string, title: string, onClick: () => void): HTMLButtonElement {
    const b = el('button', { type: 'button', id: `fx-${id}`, title }, label);
    b.addEventListener('click', () => { void Promise.resolve(onClick()).catch((e) => { filesUsage.fail(e); this.say(describeError(e), true); }); });
    this.dom[id] = b;
    return b;
  }

  private build(): void {
    const r = this.root;
    r.replaceChildren();
    const win = el('div', { class: 'fx-window' });
    const sc = this.select?.config;
    const title = el('div', { class: 'fx-title' }, el('strong', {}, sc ? `FL Studio asks: ${sc.title}` : 'Browser files'), el('span', { class: 'fx-sub' }, sc
      ? ` This is the page's file manager (${this.fs.persistent ? "the browser's storage, OPFS" : "this tab's memory"}), not an engine window and not your computer's disk. Nothing is uploaded; the file you choose is copied into the engine.`
      : ` ${this.fs.persistent ? 'stored by this browser for this site (OPFS)' : "kept in this tab's memory"}. This is the page's own file manager, not the engine's and not your computer's disk. Nothing is uploaded.`));
    const close = el('button', { type: 'button', id: 'fx-close', title: sc ? 'Cancel (Esc)' : 'Close (Esc)' }, '\u2715');
    close.addEventListener('click', () => this.dismiss());
    title.append(close);

    const bar = el('div', { class: 'fx-toolbar', role: 'toolbar', 'aria-label': 'File actions' });
    this.dom.fileInput = el('input', { type: 'file', id: 'fx-input-files', multiple: '', hidden: '' });
    this.dom.dirInput = el('input', { type: 'file', id: 'fx-input-dir', webkitdirectory: '', multiple: '', hidden: '' });
    this.dom.zipInput = el('input', { type: 'file', id: 'fx-input-zip', accept: '.zip,application/zip', multiple: '', hidden: '' });
    (this.dom.fileInput as HTMLInputElement).addEventListener('change', (e) => { const i = e.target as HTMLInputElement; const l = [...(i.files ?? [])]; i.value = ''; if (l.length) filesUsage.c.pick.files++; void this.run(this.uploadFiles(l.map((f) => ({ path: f.name, blob: f })))); });
    (this.dom.dirInput as HTMLInputElement).addEventListener('change', (e) => { const i = e.target as HTMLInputElement; const l = [...(i.files ?? [])]; i.value = ''; if (l.length) filesUsage.c.pick.folder++; void this.run(this.uploadFiles(l.map((f) => ({ path: f.webkitRelativePath || f.name, blob: f })))); });
    (this.dom.zipInput as HTMLInputElement).addEventListener('change', (e) => { const i = e.target as HTMLInputElement; const l = [...(i.files ?? [])]; i.value = ''; if (l.length) filesUsage.c.pick.zip++; void this.run((async () => { for (const f of l) await this.extractZipFile(f); })()); });
    bar.append(
      this.dom.fileInput, this.dom.dirInput, this.dom.zipInput,
      this.btn('up', '\u2191 Up', 'Parent folder (Backspace)', () => this.up()),
      this.btn('refresh', '\u21BB', 'Refresh', () => this.refresh()),
      this.btn('new-folder', 'New folder', 'Create a folder here', () => this.newFolder()),
      this.btn('upload-files', 'Upload files', 'Copy files from your computer into this folder', () => (this.dom.fileInput as HTMLInputElement).click()),
      this.btn('upload-folder', 'Upload folder', 'Copy a whole folder from your computer into this folder', () => (this.dom.dirInput as HTMLInputElement).click()),
      this.btn('upload-zip', 'Upload zip + extract', 'Choose a .zip from your computer and unpack it into a new folder here', () => (this.dom.zipInput as HTMLInputElement).click()),
      this.btn('extract', 'Extract', 'Unpack the selected zip into a new folder here', () => this.extractSelected()),
      this.btn('rename', 'Rename', 'Rename (F2)', () => this.renameSelected()),
      this.btn('copy', 'Copy', 'Copy (Ctrl+C)', () => this.setClipboard('copy')),
      this.btn('cut', 'Cut', 'Cut (Ctrl+X)', () => this.setClipboard('cut')),
      this.btn('paste', 'Paste', 'Paste here (Ctrl+V)', () => this.paste()),
      this.btn('delete', 'Delete', 'Delete (Del). This cannot be undone.', () => this.deleteSelected()),
      this.btn('download', 'Download', 'Save the selected file (or folder as a zip) to your computer', () => this.downloadSelected()),
    );

    this.dom.crumbs = el('nav', { class: 'fx-crumbs', 'aria-label': 'Path', id: 'fx-crumbs' });
    this.dom.tree = el('div', { class: 'fx-tree', id: 'fx-tree', role: 'tree', 'aria-label': 'Folders' });
    this.dom.list = el('div', { class: 'fx-list', id: 'fx-list', role: 'listbox', 'aria-multiselectable': 'true', tabindex: '0', 'aria-label': 'Files in this folder' });
    const body = el('div', { class: 'fx-body' }, this.dom.tree, this.dom.list);
    this.dom.progress = el('div', { class: 'fx-progress', id: 'fx-progress', hidden: '' });
    this.dom.status = el('div', { class: 'fx-status', id: 'fx-status', role: 'status' });
    this.dom.storage = el('div', { class: 'fx-storage', id: 'fx-storage' });
    this.dom.footer = el('div', { class: 'fx-footer', id: 'fx-footer' });
    this.dom.modal = el('div', { class: 'fx-modal', id: 'fx-modal', hidden: '', role: 'dialog', 'aria-modal': 'true' });
    this.dom.drop = el('div', { class: 'fx-drop', id: 'fx-drop', hidden: '' }, 'Drop files, folders or zips here');
    win.append(title, bar, this.dom.crumbs, body, this.dom.progress, this.dom.status, this.dom.storage, this.dom.footer, this.dom.modal, this.dom.drop);
    r.append(win);
    if (this.select) { win.classList.add('fx-select'); win.dataset.mode = this.select.config.mode; this.buildSelectFooter(); }

    // keyboard in the list
    this.dom.list.addEventListener('keydown', (e) => this.onKey(e));
    r.addEventListener('keydown', (e) => { if (e.key === 'Escape' && !this.modalOpen && !this.busy) { e.preventDefault(); this.dismiss(); } });
    // drag and drop from the computer
    let depth = 0;
    const hasFiles = (e: DragEvent): boolean => !!e.dataTransfer && [...e.dataTransfer.types].includes('Files');
    win.addEventListener('dragenter', (e) => { if (hasFiles(e)) { depth++; this.dom.drop!.hidden = false; e.preventDefault(); } });
    win.addEventListener('dragover', (e) => { if (hasFiles(e)) e.preventDefault(); });
    win.addEventListener('dragleave', () => { depth = Math.max(0, depth - 1); if (depth === 0) this.dom.drop!.hidden = true; });
    win.addEventListener('drop', (e) => { depth = 0; this.dom.drop!.hidden = true; if (!hasFiles(e)) return; e.preventDefault(); void this.run(this.onDrop(e)); });
  }

  /** a promise from a handler: its failure goes to the status line */
  private run(p: Promise<unknown>): Promise<void> { return p.then(() => undefined, (e) => { filesUsage.fail(e); this.say(describeError(e), true); }); }

  // ---------------------------------------------------------------- state and rendering
  async navigate(path: string): Promise<void> {
    const p = normalizePath(path);
    const token = ++this.navToken;
    try { this.entries = (await this.fs.list(p, false)).filter((e) => this.visible(e)); } catch (e) {
      if (p !== '/') { this.say(`${p}: ${describeError(e)}`, true); return this.navigate(parentOf(p)); }
      throw e;
    }
    this.cwd = p;
    for (let a = p; ; a = parentOf(a)) { this.expanded.add(a); if (a === '/') break; }
    this.selected.clear();
    this.anchor = '';
    this.render();
    this.syncName();
    void this.renderTree();
    void this.renderStorage();
    // sizes and dates arrive after the first paint (a folder of thousands of files is usable at once)
    const list = this.entries;
    void this.fs.fillStats(list, (e) => this.paintStat(e), () => token === this.navToken).then(() => { if (token === this.navToken) this.statsDone = token; });
  }

  private paintStat(e: FsEntry): void {
    const row = this.rowEls.get(e.name);
    if (!row) return;
    (row.children[1] as HTMLElement).textContent = formatBytes(e.size);
    (row.children[2] as HTMLElement).textContent = e.modified ? new Date(e.modified).toLocaleString() : '';
  }

  async refresh(): Promise<void> {
    const keep = new Set(this.selected);
    await this.navigate(this.cwd);
    for (const n of keep) if (this.entries.some((e) => e.name === n)) this.selected.add(n);
    this.render();
  }

  private up(): Promise<void> { return this.navigate(parentOf(this.cwd)); }

  private render(): void {
    // breadcrumb
    const crumbs = this.dom.crumbs!;
    crumbs.replaceChildren();
    const parts = splitPath(this.cwd);
    const mk = (label: string, path: string): HTMLElement => {
      const b = el('button', { type: 'button', class: 'fx-crumb', 'data-path': path }, label);
      b.addEventListener('click', () => { void this.navigate(path); });
      return b;
    };
    crumbs.append(mk('Browser files', '/'));
    let acc = '/';
    for (const s of parts) { acc = joinPath(acc, s); crumbs.append(' \u203A ', mk(s, acc)); }
    // list
    const list = this.dom.list!;
    list.replaceChildren();
    const head = el('div', { class: 'fx-row fx-head', role: 'presentation' }, el('span', { class: 'fx-name' }, 'Name'), el('span', { class: 'fx-size' }, 'Size'), el('span', { class: 'fx-mod' }, 'Modified'));
    list.append(head);
    this.rowEls.clear();
    if (this.entries.length === 0) list.append(el('div', { class: 'fx-empty' }, 'This folder is empty. Drop files, a folder or a zip here, or use Upload.'));
    for (const e of this.entries) {
      const row = el('div', { class: 'fx-row' + (this.selected.has(e.name) ? ' sel' : ''), role: 'option', 'aria-selected': String(this.selected.has(e.name)), 'data-name': e.name, 'data-kind': e.kind },
        el('span', { class: 'fx-name' }, `${ICON[e.kind]} ${e.name}`),
        el('span', { class: 'fx-size' }, e.kind === 'file' ? formatBytes(e.size) : ''),
        el('span', { class: 'fx-mod' }, e.kind === 'file' && e.modified ? new Date(e.modified).toLocaleString() : ''));
      row.addEventListener('click', (ev) => this.onRowClick(e.name, ev));
      row.addEventListener('dblclick', () => { void this.activate(e); });
      this.rowEls.set(e.name, row);
      list.append(row);
    }
    this.renderButtons();
  }

  /** the selection changed: only the rows' classes and the buttons are touched (a folder of thousands of files is not rebuilt for a click) */
  private renderSelection(): void {
    for (const [name, row] of this.rowEls) { const on = this.selected.has(name); row.classList.toggle('sel', on); row.setAttribute('aria-selected', String(on)); }
    this.renderButtons();
  }

  private renderButtons(): void {
    const sel = this.selectedEntries();
    const one = sel.length === 1 ? sel[0]! : null;
    const set = (id: string, on: boolean): void => { (this.dom[id] as HTMLButtonElement).disabled = !on || (this.busy && id !== 'cancel'); };
    set('up', this.cwd !== '/'); set('refresh', true); set('new-folder', true); set('upload-files', true); set('upload-folder', true); set('upload-zip', true);
    set('extract', !!one && one.kind === 'file' && extOf(one.name) === 'zip');
    set('rename', !!one); set('copy', sel.length > 0); set('cut', sel.length > 0); set('paste', !!this.clipboard); set('delete', sel.length > 0);
    set('download', !!one);
    (this.dom.download as HTMLElement).textContent = one?.kind === 'directory' ? 'Download as zip' : 'Download';
    this.dom.footer!.dataset.count = String(sel.length);
  }

  private async renderTree(): Promise<void> {
    const tree = this.dom.tree!;
    const build = async (path: string, depth: number, into: DocumentFragment): Promise<void> => {
      const isOpen = this.expanded.has(path);
      const row = el('div', { class: 'fx-node' + (path === this.cwd ? ' cur' : ''), role: 'treeitem', 'data-path': path, 'aria-expanded': String(isOpen), 'aria-selected': String(path === this.cwd) });
      row.dataset.depth = String(depth);
      row.classList.add(`d${Math.min(depth, 8)}`);
      const tg = el('button', { type: 'button', class: 'fx-toggle', tabindex: '-1', 'aria-label': isOpen ? 'Collapse' : 'Expand' }, isOpen ? '\u25BE' : '\u25B8');
      tg.addEventListener('click', (e) => { e.stopPropagation(); if (this.expanded.has(path) && path !== '/') this.expanded.delete(path); else this.expanded.add(path); void this.renderTree(); });
      const lb = el('span', { class: 'fx-label' }, path === '/' ? '\u{1F4BE} Browser files' : `\u{1F4C1} ${baseName(path)}`);
      row.append(tg, lb);
      row.addEventListener('click', () => { void this.navigate(path); });
      into.append(row);
      if (isOpen) {
        let subs: string[] = [];
        try { subs = await this.fs.subdirs(path); } catch { /* the folder is gone */ }
        for (const s of subs) await build(joinPath(path, s), depth + 1, into);
      }
    };
    const frag = document.createDocumentFragment();
    await build('/', 0, frag);
    tree.replaceChildren(frag);
  }

  async renderStorage(): Promise<void> {
    if (!this.fs.persistent) { this.dom.storage!.replaceChildren(el('span', { id: 'fx-memory', class: 'warn' }, memoryNotice(this.fs.why))); return; }
    const s = await storageInfo();
    filesUsage.storage(s.usage, s.quota, s.persisted);
    const box = this.dom.storage!;
    box.replaceChildren();
    const used = s.usage !== null ? formatBytes(s.usage) : 'unknown';
    const quota = s.quota !== null ? formatBytes(s.quota) : 'unknown';
    box.append(el('span', { id: 'fx-usage' }, `This site uses ${used} of about ${quota} the browser allows (includes the engine's cache and other data of this site). `));
    if (s.persisted) box.append(el('span', { id: 'fx-persisted', class: 'ok' }, 'Protected: the browser will not delete this data on its own.'));
    else {
      box.append(el('span', { id: 'fx-persisted', class: 'warn' }, 'Not protected: when the disk runs low the browser may delete this data without asking. '));
      const b = el('button', { type: 'button', id: 'fx-persist' }, 'Ask the browser to keep it');
      b.addEventListener('click', () => { void (async () => {
        this.say('Asking the browser (it may show a permission prompt; answer it there)...');
        filesUsage.c.persist.asked++;
        let answered = false;
        const t = setTimeout(() => { if (!answered) filesUsage.c.persist.pending++; }, 3000);
        let granted = false;
        try { granted = await navigator.storage.persist(); } catch { /* denied */ }
        answered = true; clearTimeout(t);
        if (granted) filesUsage.c.persist.granted++; else filesUsage.c.persist.denied++;
        this.say(granted ? 'The browser will keep this data.' : 'The browser did not grant it (it decides by its own rules, for example how often you use the site). The data stays until the browser needs the space or you clear site data.', !granted);
        void this.renderStorage();
      })(); });
      box.append(b);
    }
  }

  say(text: string, error = false): void {
    this.log.push(text);
    if (this.log.length > 200) this.log.shift();
    const s = this.dom.status!;
    s.textContent = text;
    s.dataset.error = error ? '1' : '0';
  }

  private selectedEntries(): FsEntry[] { return this.entries.filter((e) => this.selected.has(e.name)); }

  private onRowClick(name: string, ev: MouseEvent): void {
    if (ev.shiftKey && this.anchor) {
      const a = this.entries.findIndex((e) => e.name === this.anchor), b = this.entries.findIndex((e) => e.name === name);
      const [lo, hi] = a < b ? [a, b] : [b, a];
      if (!(ev.ctrlKey || ev.metaKey)) this.selected.clear();
      for (let i = lo; i <= hi; i++) this.selected.add(this.entries[i]!.name);
    } else if (ev.ctrlKey || ev.metaKey) {
      if (!this.selected.delete(name)) this.selected.add(name);
      this.anchor = name;
    } else { this.selected.clear(); this.selected.add(name); this.anchor = name; }
    this.renderSelection();
    this.syncName();
    this.dom.list!.focus({ preventScroll: true });
  }

  private async activate(e: FsEntry): Promise<void> {
    if (e.kind === 'directory') return this.navigate(e.path);
    if (this.select) { this.selected.clear(); this.selected.add(e.name); this.syncName(); return this.choose(); }
    if (extOf(e.name) === 'zip') return this.extractZipFile(await this.fs.read(e.path), e.path);
    return this.downloadSelected();
  }

  private onKey(e: KeyboardEvent): void {
    if (this.busy) return;
    const idx = this.entries.findIndex((x) => x.name === this.anchor);
    const go = (i: number, extend: boolean): void => {
      const n = this.entries[Math.max(0, Math.min(this.entries.length - 1, i))];
      if (!n) return;
      if (!extend) this.selected.clear();
      this.selected.add(n.name);
      if (!extend) this.anchor = n.name;
      this.renderSelection();
      this.syncName();
      this.dom.list!.querySelector<HTMLElement>(`[data-name="${CSS.escape(n.name)}"]`)?.scrollIntoView({ block: 'nearest' });
    };
    const mod = e.ctrlKey || e.metaKey;
    if (e.key === 'ArrowDown') { e.preventDefault(); go(idx + 1, e.shiftKey); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); go(idx < 0 ? 0 : idx - 1, e.shiftKey); }
    else if (e.key === 'Enter') { const s = this.selectedEntries()[0]; if (s) { e.preventDefault(); void this.activate(s); } }
    else if (e.key === 'Backspace') { e.preventDefault(); void this.up(); }
    else if (e.key === 'F2') { e.preventDefault(); void this.run(this.renameSelected()); }
    else if (e.key === 'Delete') { e.preventDefault(); void this.run(this.deleteSelected()); }
    else if (mod && e.key.toLowerCase() === 'a') { e.preventDefault(); this.entries.forEach((x) => this.selected.add(x.name)); this.renderSelection(); }
    else if (mod && e.key.toLowerCase() === 'c') { e.preventDefault(); this.setClipboard('copy'); }
    else if (mod && e.key.toLowerCase() === 'x') { e.preventDefault(); this.setClipboard('cut'); }
    else if (mod && e.key.toLowerCase() === 'v') { e.preventDefault(); void this.run(this.paste()); }
  }

  // ---------------------------------------------------------------- selection mode (the engine's file dialog)
  /** Esc / the close button: cancels the engine's dialog in selection mode, closes the window otherwise */
  private dismiss(): void {
    if (this.select) { if (!this.finished) { this.finished = true; this.select.done(null); } } else this.host.close();
  }

  /** a name the dialog shows: a folder always, a file when it passes the selected filter (Open and Save show only the matching files, as the Windows dialog does) */
  private visible(e: FsEntry): boolean {
    if (!this.select) return true;
    if (e.kind === 'directory') return true;
    if (this.select.config.mode === 'folder') return false;
    return matchesFilter(e.name, this.select.config.filters[this.filterIndex]);
  }

  private nameInput(): HTMLInputElement | null { return (this.dom.fileName as HTMLInputElement | undefined) ?? null; }

  private buildSelectFooter(): void {
    const sc = this.select!.config;
    const ft = this.dom.footer!;
    ft.replaceChildren();
    if (sc.note) ft.append(el('div', { class: 'fx-note', id: 'fx-note' }, sc.note));
    const row = el('div', { class: 'fx-selrow' });
    if (sc.mode !== 'folder') {
      const input = el('input', { type: 'text', id: 'fx-filename', value: sc.fileName, size: '40', spellcheck: 'false', autocomplete: 'off', 'aria-label': 'File name' });
      this.dom.fileName = input;
      input.addEventListener('keydown', (e) => { e.stopPropagation(); if (e.key === 'Enter') { e.preventDefault(); void this.choose(); } else if (e.key === 'Escape') { e.preventDefault(); this.dismiss(); } });
      row.append(el('label', {}, 'File name: ', input));
      const sel = el('select', { id: 'fx-filter', 'aria-label': 'File type' });
      sc.filters.forEach((f, i) => { const o = el('option', { value: String(i) }, f.label); if (i === this.filterIndex) o.selected = true; sel.append(o); });
      sel.addEventListener('change', () => { this.filterIndex = Number(sel.value); void this.refresh(); });
      row.append(sel);
    } else row.append(el('span', { id: 'fx-folder-hint' }, 'Open the folder you want and press the button; the files inside are not shown.'));
    const ok = el('button', { type: 'button', id: 'fx-ok', class: 'primary' }, sc.okLabel);
    const cancel = el('button', { type: 'button', id: 'fx-cancel-dialog' }, 'Cancel');
    ok.addEventListener('click', () => { void this.choose(); });
    cancel.addEventListener('click', () => this.dismiss());
    this.dom.ok = ok;
    row.append(ok, cancel);
    ft.append(row);
  }

  /** the name box follows the selection (Open: the chosen names; Save: the one file) */
  private syncName(): void {
    const inp = this.nameInput();
    if (!inp || !this.select) return;
    const files = this.selectedEntries().filter((e) => e.kind === 'file');
    if (files.length === 1) inp.value = files[0]!.name;
    else if (files.length > 1) inp.value = files.map((f) => `"${f.name}"`).join(' ');
  }

  /** the Open / Save / Choose button: checks what the person chose and hands it to the engine's side. A problem is shown in the window and the dialog stays. */
  async choose(): Promise<void> {
    const sel = this.select;
    if (!sel || this.finished || this.busy) return;
    const sc = sel.config;
    const fail = (m: string): void => { this.say(m, true); };
    try {
      if (sc.mode === 'folder') {
        const one = this.selectedEntries().filter((e) => e.kind === 'directory');
        const dir = one.length === 1 ? one[0]!.path : this.cwd;
        this.finished = true; sel.done({ paths: [dir], filterIndex: this.filterIndex });
        return;
      }
      const typed = (this.nameInput()?.value ?? '').trim();
      if (sc.mode === 'open') {
        let chosen = this.selectedEntries().filter((e) => e.kind === 'file').map((e) => e.path);
        const quoted = [...typed.matchAll(/"([^"]+)"/g)].map((m) => m[1]!);
        if (chosen.length === 0 || (typed && !(chosen.length === 1 && chosen[0]!.endsWith('/' + typed)) && quoted.length === 0)) {
          if (typed === '') return fail('Choose a file.');
          const names = quoted.length > 0 ? quoted : [typed];
          chosen = [];
          for (const n of names) {
            const actual = await this.fs.findCI(this.cwd, n);
            const st = actual ? await this.fs.stat(joinPath(this.cwd, actual)) : null;
            if (!st) return fail(`"${n}" was not found in this folder.`);
            if (st.kind === 'directory') { await this.navigate(st.path); return; }
            chosen.push(st.path);
          }
        }
        if (chosen.length > 1 && !sc.multi) return fail('Choose one file.');
        this.finished = true; sel.done({ paths: chosen, filterIndex: this.filterIndex });
        return;
      }
      // save
      if (typed === '') return fail('Type a file name.');
      let name = typed.replace(/^"+|"+$/g, '');
      const dirTarget = await this.fs.findCI(this.cwd, name);
      if (dirTarget && (await this.fs.stat(joinPath(this.cwd, dirTarget)))?.kind === 'directory') { await this.navigate(joinPath(this.cwd, dirTarget)); return; }
      if (!name.includes('.')) name += saveExtension(sc.defaultExt, sc.filters[this.filterIndex]);
      const prob = nameProblem(name);
      if (prob) return fail(prob);
      const existing = await this.fs.findCI(this.cwd, name);
      if (existing !== null) {
        if (sc.overwritePrompt) {
          this.busy = true;
          const ok = await this.ask('Replace the file?', `"${existing}" already exists in this folder. Do you want to replace it?`, 'Replace', true).finally(() => { this.busy = false; });
          if (!ok) return;
        }
        name = existing;
      }
      this.finished = true; sel.done({ paths: [joinPath(this.cwd, name)], filterIndex: this.filterIndex });
    } catch (e) { fail(describeError(e)); }
  }

  /** for the engine-dialog glue: run something long (a zip to unpack, files to read) with the window's progress bar and Cancel button; the dialog stays open and its buttons are off meanwhile */
  async withProgress<T>(fn: (ctx: OpContext) => Promise<T>, what: string): Promise<T | undefined> {
    let out: T | undefined;
    await this.guard(async (ctx) => { out = await fn(ctx); }, what);
    return out;
  }

  /** the dialog was not finished after all (the glue could not give the engine what was chosen): the window takes input again */
  reopen(message: string): void {
    this.finished = false;
    this.say(message, true);
  }

  // ---------------------------------------------------------------- modal questions
  private modal<T>(title: string, build: (body: HTMLElement, done: (v: T) => void) => HTMLElement | void, cancelValue: T): Promise<T> {
    return new Promise<T>((resolve) => {
      const m = this.dom.modal!;
      this.modalOpen = true;
      m.replaceChildren();
      const box = el('div', { class: 'fx-modal-box' }, el('h3', {}, title));
      const finish = (v: T): void => { this.modalOpen = false; m.hidden = true; m.replaceChildren(); this.dom.list!.focus({ preventScroll: true }); resolve(v); };
      const focus = build(box, finish);
      m.append(box);
      m.hidden = false;
      m.onkeydown = (e) => { e.stopPropagation(); if (e.key === 'Escape') finish(cancelValue); };
      (focus ?? box.querySelector<HTMLElement>('button'))?.focus();
    });
  }

  private ask(title: string, text: string, ok: string, danger = false): Promise<boolean> {
    return this.modal<boolean>(title, (box, done) => {
      box.append(el('p', {}, text));
      const o = el('button', { type: 'button', id: 'fx-modal-ok', class: danger ? 'danger' : '' }, ok);
      const c = el('button', { type: 'button', id: 'fx-modal-cancel' }, 'Cancel');
      o.addEventListener('click', () => done(true)); c.addEventListener('click', () => done(false));
      box.append(el('div', { class: 'fx-buttons' }, o, c));
      return c;
    }, false);
  }

  private askText(title: string, label: string, value: string, ok: string, validate: (v: string) => string | null): Promise<string | null> {
    return this.modal<string | null>(title, (box, done) => {
      const input = el('input', { type: 'text', id: 'fx-modal-text', value, spellcheck: 'false', autocomplete: 'off', size: '48' });
      const err = el('div', { class: 'fx-err', id: 'fx-modal-err' });
      const o = el('button', { type: 'button', id: 'fx-modal-ok' }, ok);
      const c = el('button', { type: 'button', id: 'fx-modal-cancel' }, 'Cancel');
      const go = (): void => { const v = input.value; const p = validate(v); if (p) { err.textContent = p; return; } done(v); };
      o.addEventListener('click', go); c.addEventListener('click', () => done(null));
      input.addEventListener('keydown', (e) => { if (e.key === 'Enter') go(); });
      box.append(el('label', {}, label + ' ', input), err, el('div', { class: 'fx-buttons' }, o, c));
      queueMicrotask(() => { input.focus(); const dot = value.lastIndexOf('.'); input.setSelectionRange(0, dot > 0 ? dot : value.length); });
      return input;
    }, null);
  }

  private askConflict = (name: string, existing: 'file' | 'directory'): Promise<Conflict | 'cancel'> => {
    if (this.conflictPolicy) return Promise.resolve(this.conflictPolicy);
    return this.modal<Conflict | 'cancel'>('Name already in use', (box, done) => {
      box.append(el('p', {}, `${existing === 'directory' ? 'A folder' : 'A file'} named "${name}" is already here. This choice is used for every name that clashes in this operation.`));
      const mk = (id: string, label: string, v: Conflict | 'cancel'): HTMLButtonElement => { const b = el('button', { type: 'button', id: `fx-conflict-${id}` }, label); b.addEventListener('click', () => done(v)); return b; };
      box.append(el('div', { class: 'fx-buttons' }, mk('replace', 'Replace', 'replace'), mk('keep', 'Keep both', 'keep-both'), mk('skip', 'Skip', 'skip'), mk('cancel', 'Cancel', 'cancel')));
      return box.querySelector('#fx-conflict-keep') as HTMLElement;
    }, 'cancel');
  };

  // ---------------------------------------------------------------- operations
  /** one operation at a time; shows progress and a Cancel button; reports the outcome in the status line */
  private async guard(fn: (ctx: OpContext) => Promise<unknown>, what: string): Promise<void> {
    if (this.busy) { this.say('Another operation is still running.', true); return; }
    this.busy = true;
    this.abort = new AbortController();
    this.renderButtons();
    const prog = this.dom.progress!;
    prog.hidden = false;
    prog.replaceChildren();
    const bar = el('progress', { max: '1', value: '0' });
    const lab = el('span', { class: 'fx-prog-label' }, '');
    const cancel = el('button', { type: 'button', id: 'fx-cancel' }, 'Cancel');
    cancel.addEventListener('click', () => this.abort?.abort());
    prog.append(bar, lab, cancel);
    const ctx: OpContext = {
      fs: this.fs, signal: this.abort.signal, askConflict: this.askConflict,
      progress: (p: OpProgress) => {
        bar.max = p.totalBytes > 0 ? p.totalBytes : Math.max(1, p.total);
        bar.value = p.totalBytes > 0 ? p.bytes : p.done;
        lab.textContent = ` ${p.done}/${p.total} ${p.totalBytes ? `(${formatBytes(p.bytes)} of ${formatBytes(p.totalBytes)})` : ''} ${p.label}`;
      },
    };
    try { await fn(ctx); } catch (e) { filesUsage.fail(e); this.say(`${what}: ${describeError(e)}`, true); }
    finally {
      this.busy = false; this.abort = null; prog.hidden = true; prog.replaceChildren();
      try { await this.refresh(); } catch (e) { this.say(describeError(e), true); }
    }
  }

  private summarize(what: string, r: OpResult): void {
    this.lastResult = r;
    filesUsage.result(r);
    const parts = [`${what}: ${r.written} file${r.written === 1 ? '' : 's'} written (${formatBytes(r.bytes)})`];
    if (r.skipped) parts.push(`${r.skipped} skipped`);
    if (r.renamed.length) parts.push(`${r.renamed.length} name${r.renamed.length === 1 ? '' : 's'} changed`);
    if (r.failed.length) parts.push(`${r.failed.length} failed (${r.failed[0]!.path}: ${r.failed[0]!.reason})`);
    if (r.cancelled) parts.push('cancelled');
    this.say(parts.join(', ') + '.', r.failed.length > 0 || r.cancelled);
  }

  private newFolder(): Promise<void> {
    return this.guardAsk(async () => {
      const taken = await this.fs.names(this.cwd);
      const name = await this.askText('New folder', 'Name:', uniqueName('New folder', taken), 'Create', (v) => nameProblem(v) ?? (taken.has(v.toLowerCase()) ? 'That name is already used here (names are not case-sensitive, like on Windows).' : null));
      if (name === null) return;
      await this.fs.mkdir(joinPath(this.cwd, name));
      filesUsage.c.ops.mkdir++;
      this.say(`Created folder "${name}".`);
      this.selected.add(name);
    });
  }

  /** a short operation that needs no progress bar but must not overlap another */
  private async guardAsk(fn: () => Promise<void>): Promise<void> {
    if (this.busy) { this.say('Another operation is still running.', true); return; }
    this.busy = true;
    try { await fn(); } catch (e) { filesUsage.fail(e); this.say(describeError(e), true); }
    finally { this.busy = false; await this.refresh(); }
  }

  private renameSelected(): Promise<void> {
    const one = this.selectedEntries();
    if (one.length !== 1) { this.say('Select one item to rename.', true); return Promise.resolve(); }
    const e = one[0]!;
    return this.guardAsk(async () => {
      const taken = await this.fs.names(this.cwd);
      const name = await this.askText('Rename', 'New name:', e.name, 'Rename', (v) => nameProblem(v) ?? (v !== e.name && taken.has(v.toLowerCase()) && v.toLowerCase() !== e.name.toLowerCase() ? 'That name is already used here.' : null));
      if (name === null || name === e.name) return;
      const hit = protectedHit(e.path);
      if (hit && !(await this.ask('Rename', `"${e.name}" holds the engine's saved registration / licence data (${hit}). Renaming it makes the engine forget what it was activated with. Rename anyway?`, 'Rename', true))) return;
      await this.fs.move(e.path, this.cwd, name);
      filesUsage.c.ops.rename++;
      this.say(`Renamed "${e.name}" to "${name}".`);
      this.selected.clear(); this.selected.add(name);
    });
  }

  private async deleteSelected(): Promise<void> {
    const sel = this.selectedEntries();
    if (sel.length === 0) return;
    await this.guardAsk(async () => {
      let folders = 0, files = 0, bytes = 0;
      for (const e of sel) { const [f, b] = await this.fs.du(e.path); files += f; bytes += b; if (e.kind === 'directory') folders++; }
      const what = sel.length === 1 ? `"${sel[0]!.name}"` : `${sel.length} items`;
      const hit = sel.map((e) => protectedHit(e.path)).find((x) => x !== null);
      const warn = hit ? ` WARNING: this holds the engine's saved registration / licence data (${hit}); without it the engine forgets what it was activated with.` : '';
      const ok = await this.ask('Delete', `Delete ${what}${folders ? ` (${files} file${files === 1 ? '' : 's'}, ${formatBytes(bytes)} inside)` : ` (${formatBytes(bytes)})`}? This cannot be undone.${warn}`, 'Delete', true);
      if (!ok) return;
      const failed: string[] = [];
      for (const e of sel) { try { await this.fs.remove(e.path); filesUsage.c.ops.delete++; } catch (err) { filesUsage.fail(err); failed.push(`${e.name}: ${describeError(err)}`); } }
      this.selected.clear();
      this.say(failed.length ? `Deleted ${sel.length - failed.length} of ${sel.length}; ${failed[0]}` : `Deleted ${what}.`, failed.length > 0);
    });
  }

  private setClipboard(mode: 'copy' | 'cut'): void {
    const sel = this.selectedEntries();
    if (sel.length === 0) return;
    this.clipboard = { mode, paths: sel.map((e) => e.path) };
    this.say(`${mode === 'copy' ? 'Copied' : 'Cut'} ${sel.length} item${sel.length === 1 ? '' : 's'}; open a folder and press Paste.`);
    this.renderButtons();
  }

  private paste(): Promise<void> {
    const cb = this.clipboard;
    if (!cb) return Promise.resolve();
    return this.guard(async (ctx) => {
      const res = newResult();
      const hit = cb.mode === 'cut' ? cb.paths.map((p) => protectedHit(p)).find((x) => x !== null) : null;
      if (hit && !(await this.ask('Move', `You are moving data that holds the engine's saved registration / licence data (${hit}). Moving it makes the engine forget what it was activated with. Move anyway?`, 'Move', true))) { res.cancelled = true; this.summarize('Move', res); return; }
      let answer: Conflict | 'cancel' | null = null;
      let done = 0;
      for (const src of cb.paths) {
        if (ctx.signal.aborted) { res.cancelled = true; break; }
        const name = baseName(src);
        ctx.progress({ label: name, done, total: cb.paths.length, bytes: 0, totalBytes: 0 });
        try {
          if (isInside(this.cwd, src) && (await this.fs.stat(src))?.kind === 'directory') throw new Error('a folder cannot be put inside itself');
          let target = name;
          const taken = await this.fs.findCI(this.cwd, name);
          const samePlace = parentOf(src) === this.cwd;
          if (taken !== null) {
            if (cb.mode === 'cut' && samePlace) { done++; continue; }
            if (cb.mode === 'copy' && samePlace) { target = await this.fs.freeName(this.cwd, name); }
            else {
              if (answer === null) answer = await this.askConflict(taken, (await this.fs.stat(joinPath(this.cwd, taken)))?.kind ?? 'file');
              if (answer === 'cancel') { res.cancelled = true; break; }
              if (answer === 'skip') { res.skipped++; done++; continue; }
              if (answer === 'keep-both') { target = await this.fs.freeName(this.cwd, name); res.renamed.push([name, target]); }
              else await this.fs.remove(joinPath(this.cwd, taken));
            }
          }
          if (cb.mode === 'copy') await this.fs.copy(src, this.cwd, target, (n) => { res.bytes += n; });
          else await this.fs.move(src, this.cwd, target);
          filesUsage.c.ops[cb.mode === 'copy' ? 'copy' : 'move']++;
          res.written++;
        } catch (e) { res.failed.push({ path: name, reason: describeError(e) }); }
        done++;
      }
      if (cb.mode === 'cut') this.clipboard = null;
      this.summarize(cb.mode === 'copy' ? 'Copy' : 'Move', res);
    }, 'paste');
  }

  // ---------------------------------------------------------------- upload
  async uploadFiles(items: Incoming[]): Promise<void> {
    await this.guard(async (ctx) => {
      if (items.length === 0) return;
      const total = items.reduce((n, i) => n + i.blob.size, 0);
      const free = roomLeft(this.fs, await storageInfo());
      if (free !== null && total > free) { this.say(`There is not enough room: ${formatBytes(total)} to add, ${formatBytes(free)} free by the browser's estimate.`, true); return; }
      this.summarize('Upload', await putFiles(ctx, this.cwd, items));
    }, 'upload');
  }

  /** a zip that the person gave (file chooser, double-click, or a drop): unpack it into a new folder here named like the zip */
  async extractZipFile(file: Blob & { name?: string }, fromPath?: string): Promise<void> {
    const zname = fromPath ? baseName(fromPath) : (file.name ?? 'archive.zip');
    let plan: ZipPlan;
    try { plan = await planZip(file); } catch (e) { filesUsage.fail(e); this.say(`${zname}: ${describeError(e)}`, true); return; }
    const taken = await this.fs.names(this.cwd);
    const stem = zname.replace(/\.zip$/i, '');
    const srcs = Object.entries(plan.nameSources).map(([k, v]) => `${v} ${k}`).join(', ');
    const folder = await this.modal<string | null>('Extract zip', (box, done) => {
      box.append(el('p', {}, `${zname}: ${plan.files.length} file${plan.files.length === 1 ? '' : 's'}, ${formatBytes(plan.totalBytes)} when unpacked. Names read from: ${srcs}.`));
      const input = el('input', { type: 'text', id: 'fx-modal-text', value: uniqueName(stem, taken), size: '48', spellcheck: 'false', autocomplete: 'off' });
      const err = el('div', { class: 'fx-err', id: 'fx-modal-err' });
      const o = el('button', { type: 'button', id: 'fx-modal-ok' }, 'Extract');
      const c = el('button', { type: 'button', id: 'fx-modal-cancel' }, 'Cancel');
      const go = (): void => { const v = input.value.trim(); const p = nameProblem(v); if (p) { err.textContent = p; return; } done(v); };
      o.addEventListener('click', go); c.addEventListener('click', () => done(null));
      input.addEventListener('keydown', (e) => { if (e.key === 'Enter') go(); });
      box.append(el('label', {}, 'Into a folder named: ', input), err, el('div', { class: 'fx-buttons' }, o, c));
      return input;
    }, null);
    if (folder === null) return;
    await this.guard(async (ctx) => {
      const free = roomLeft(this.fs, await storageInfo());
      if (free !== null && plan.totalBytes > free) { this.say(`There is not enough room to unpack ${zname}: ${formatBytes(plan.totalBytes)} needed, ${formatBytes(free)} free by the browser's estimate.`, true); return; }
      const xr = await extractZip(ctx, joinPath(this.cwd, folder), file, plan);
      const xc = filesUsage.c.extract;
      xc.zips++; xc.files += xr.written; if (xr.cancelled) xc.cancelled++;
      this.summarize(`Extract ${zname}`, xr);
    }, 'extract zip');
  }

  private async extractSelected(): Promise<void> {
    const e = this.selectedEntries()[0];
    if (!e || extOf(e.name) !== 'zip') return;
    await this.extractZipFile(await this.fs.read(e.path), e.path);
  }

  /** a drop from the computer: files and folders (read with file-selector: webkitGetAsEntry, and getAsFileSystemHandle where there is one); a zip among them is asked about once */
  private async onDrop(e: DragEvent): Promise<void> {
    const files = (await fromEvent(e)) as (File & { path?: string; relativePath?: string })[];
    const rel = (f: File & { path?: string; relativePath?: string }): string => ((f.relativePath && f.relativePath !== '' ? f.relativePath : f.path) ?? f.name).replace(/^(\.\/|\/)+/, '') || f.name;
    const items: Incoming[] = files.filter((f) => f instanceof File).map((f) => ({ path: rel(f), blob: f }));
    const dc = filesUsage.c.drop;
    if (items.some((i) => i.path.includes('/'))) dc.folders++;
    if (items.some((i) => !i.path.includes('/'))) dc.files++;
    if (items.some((i) => !i.path.includes('/') && extOf(i.path) === 'zip')) dc.zips++;
    await this.handleIncoming(items);
  }

  /** files given by a drop or by a test: a loose .zip at the top level is offered for extraction */
  async handleIncoming(items: Incoming[]): Promise<void> {
    const zips = items.filter((i) => !i.path.includes('/') && extOf(i.path) === 'zip');
    let extract = false;
    if (zips.length > 0) {
      extract = this.zipPolicy ? this.zipPolicy === 'extract' : await this.ask('Zip file', `${zips.length === 1 ? `"${zips[0]!.path}" is a zip` : `${zips.length} of the dropped files are zips`}. Unpack ${zips.length === 1 ? 'it' : 'them'} into a new folder${zips.length === 1 ? '' : 's'}? (Cancel keeps the zip as a file.)`, 'Unpack');
    }
    const rest = extract ? items.filter((i) => !zips.includes(i)) : items;
    if (rest.length) await this.uploadFiles(rest);
    if (extract) for (const z of zips) await this.extractZipFile(z.blob as Blob & { name?: string }, undefined).catch((err) => this.say(describeError(err), true));
  }

  // ---------------------------------------------------------------- download
  private async downloadSelected(): Promise<void> {
    const e = this.selectedEntries()[0];
    if (!e) return;
    if (e.kind === 'file') { saveBlob(await this.fs.read(e.path), e.name); filesUsage.c.download.file++; this.say(`Sent "${e.name}" to the browser's download.`); return; }
    await this.guard(async (ctx) => {
      const [files, bytes] = await this.fs.du(e.path);
      if (bytes > 1.5 * 1024 ** 3) { this.say(`"${e.name}" is ${formatBytes(bytes)}: too large to build a zip in memory. Download its files one by one.`, true); return; }
      const zip = await folderToZip(ctx, e.path);
      saveBlob(zip, `${e.name}.zip`);
      filesUsage.c.download.folderZip++;
      this.say(`Sent "${e.name}.zip" (${files} files, ${formatBytes(zip.size)}) to the browser's download.`);
    }, 'download');
  }
}
