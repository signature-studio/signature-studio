// The page's answer to the engine's file dialogs (decision 0193, fpick1, PROVISIONAL): a modal box over the desktop while the engine is suspended in IFileOpenDialog::Show / IFileSaveDialog::Show.
// The box says in words that it is the page (not the engine) and that the person chooses. Open: a native file input (the click on it is the person's own click, so no transient-activation problem
// arises; the browser's picker is not opened from a script) and a folder input for a project folder (FLP + samples). Save: a name box. Nothing here is sent anywhere: the File objects go to the Worker.
import { acceptFor, describeFilters, planOpenFile, planOpenFolder, planSave, suggestedSaveName, FOS_PICKFOLDERS, type FileDlgAnswer, type FileDlgItem, type FileDlgRequest } from '../engine/filedlg.ts';

export interface FileDlgHost {
  answer(id: number, answer: FileDlgAnswer): void;
  /** decision 0216 (opfs1): when set, the box is replaced by the file manager window in selection mode (files/engine-dialog.ts); it calls `done` once with the answer and returns what undoes it */
  select?(id: number, request: FileDlgRequest, root: HTMLElement, done: (a: FileDlgAnswer) => void): () => void;
  /** the box is gone (answered and the engine is idle again, cancelled, or the engine ended): give the keyboard back to the desktop */
  closed?(): void;
}

const el = <K extends keyof HTMLElementTagNameMap>(tag: K, attrs: Record<string, string> = {}, text?: string): HTMLElementTagNameMap[K] => {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  if (text !== undefined) e.textContent = text;
  return e;
};

export class FileDialogView {
  private readonly root: HTMLElement;
  private readonly host: FileDlgHost;
  private current: { id: number; request: FileDlgRequest } | null = null;
  /** the id of a dialog that has been answered and whose engine has not been seen idle again yet (the box stays, showing the reading progress) */
  private finishing: number | null = null;
  private readonly body: HTMLElement;
  private readonly status: HTMLElement;
  private disposeSelect: (() => void) | null = null;
  /** what the page has done with dialogs, for the tests and the page's log (no file names) */
  readonly history: { id: number; kind: 'open' | 'save'; outcome: 'answered' | 'cancelled' | 'refused'; files: number; bytes: number }[] = [];

  constructor(root: HTMLElement, host: FileDlgHost) {
    this.root = root;
    this.host = host;
    root.replaceChildren();
    const box = el('div', { class: 'filedlg-box' });
    box.append(el('div', { class: 'filedlg-tag', id: 'filedlg-tag' }, 'This is the page, not the engine. The engine asked for a file (its own dialog has no window here). You choose; the file is not uploaded: it is read in this tab and handed to the engine.'));
    box.append(el('h2', { id: 'filedlg-title' }, ''));
    this.body = el('div', { id: 'filedlg-body' });
    this.status = el('div', { id: 'filedlg-status', role: 'status' });
    box.append(this.body, this.status);
    root.append(box);
    root.hidden = true;
    root.addEventListener('keydown', (e) => { if (e.key === 'Escape') this.cancel('Escape'); });
  }

  get open(): boolean { return this.current !== null; }
  get currentId(): number | null { return this.current?.id ?? null; }

  show(id: number, request: FileDlgRequest): void {
    if (this.current) this.cancel('replaced by a newer dialog');
    this.finishing = null;
    this.current = { id, request };
    this.root.dataset.kind = request.save ? 'save' : 'open';
    this.root.hidden = false;
    this.status.textContent = '';
    (this.root.querySelector('#filedlg-title') as HTMLElement).textContent = request.title ?? (request.save ? 'Save as' : 'Open');
    this.body.replaceChildren();
    if (this.host.select) this.buildSelect(id, request);
    else if (request.save) this.buildSave(id, request);
    else if (request.options & FOS_PICKFOLDERS) this.buildRefusal(id, 'The engine asked for a folder of its own virtual volume. There is none to choose from on this computer.');
    else this.buildOpen(id, request);
  }

  progress(id: number, done: number, total: number): void {
    if (this.current?.id !== id && this.finishing !== id) return;
    this.status.textContent = `Reading ${(done / 1048576).toFixed(1)} of ${(total / 1048576).toFixed(1)} MiB into the engine's memory (in this tab)...`;
  }

  /** decision 0216: the OPFS file manager window is the dialog (the page's box with its two inputs is not built) */
  private buildSelect(id: number, request: FileDlgRequest): void {
    this.root.dataset.select = '1';
    this.disposeSelect = this.host.select!(id, request, this.body, (a) => this.answerSelect(id, a));
  }

  private answerSelect(id: number, a: FileDlgAnswer): void {
    const c = this.current;
    if (!c || c.id !== id) return;
    const kind = c.request.save ? 'save' : 'open';
    if (!a.ok) { this.finish(id, kind, 'cancelled', 0, 0); this.host.answer(id, a); this.hide(); return; }
    delete this.status.dataset.error;
    this.status.textContent = a.items.some((i) => i.file) ? 'Giving the files to the engine...' : 'Answering the engine...';
    this.finish(id, kind, 'answered', a.items.length, a.items.reduce((n, i) => n + (i.file?.size ?? 0), 0));
    this.host.answer(id, a);
    this.current = null;
    this.finishing = id;
  }

  hide(): void {
    const was = !this.root.hidden;
    if (this.disposeSelect) { this.disposeSelect(); this.disposeSelect = null; }
    delete this.root.dataset.select;
    this.current = null;
    this.finishing = null;
    this.root.hidden = true;
    this.body.replaceChildren();
    if (was) this.host.closed?.();
  }

  private finish(id: number, kind: 'open' | 'save', outcome: 'answered' | 'cancelled' | 'refused', files: number, bytes: number): void {
    this.history.push({ id, kind, outcome, files, bytes });
    if (this.history.length > 50) this.history.shift();
  }

  private cancel(why: string): void {
    const c = this.current;
    if (!c) return;
    this.finish(c.id, c.request.save ? 'save' : 'open', 'cancelled', 0, 0);
    this.host.answer(c.id, { ok: false, reason: why });
    this.hide();
  }

  private cancelButton(): HTMLButtonElement {
    const b = el('button', { type: 'button', id: 'filedlg-cancel' }, 'Cancel');
    b.addEventListener('click', () => this.cancel('cancelled by the person'));
    return b;
  }

  private buildRefusal(id: number, text: string): void {
    this.body.append(el('p', {}, text));
    const b = this.cancelButton();
    b.textContent = 'Close (cancels the engine\'s dialog)';
    this.body.append(b);
    void id;
  }

  private buildOpen(id: number, request: FileDlgRequest): void {
    this.body.append(el('p', { class: 'muted' }, `The engine offers: ${describeFilters(request)}`));
    const row1 = el('label', { class: 'filedlg-row' }, 'A file from your computer: ');
    const input = el('input', { type: 'file', id: 'filedlg-input', accept: acceptFor(request) });
    row1.append(input);
    queueMicrotask(() => input.focus({ preventScroll: true }));
    const row2 = el('label', { class: 'filedlg-row' }, 'Or a project folder (the .flp and its samples): ');
    const dir = el('input', { type: 'file', id: 'filedlg-dir' });
    dir.setAttribute('webkitdirectory', '');
    row2.append(dir);
    this.body.append(row1, row2, el('p', { class: 'muted' }, `Files are placed on the engine's virtual volume (C:\\Users\\user\\Downloads); a single file is read whole into memory (limit 256 MiB, 384 MiB in all).`), this.cancelButton());
    input.addEventListener('change', () => {
      const f = input.files?.[0];
      if (!f) return;
      const plan = planOpenFile(request, { name: f.name, size: f.size });
      if (!plan.ok) { this.refuse(id, plan.reason); input.value = ''; return; }
      const items: FileDlgItem[] = plan.value.items.map((it) => ({ role: it.role, path: it.path, file: f }));
      this.answer(id, items, f.size);
    });
    dir.addEventListener('change', () => {
      const list = Array.from(dir.files ?? []);
      if (list.length === 0) return;
      const picked = list.map((f) => ({ name: f.name, size: f.size, relativePath: f.webkitRelativePath || f.name }));
      const plan = planOpenFolder(request, picked);
      if (!plan.ok) { this.refuse(id, plan.reason); dir.value = ''; return; }
      const items: FileDlgItem[] = plan.value.items.map((it) => ({ role: it.role, path: it.path, file: list[it.index] as File }));
      this.answer(id, items, list.reduce((n, f) => n + f.size, 0));
    });
  }

  private buildSave(id: number, request: FileDlgRequest): void {
    this.body.append(el('p', { class: 'muted' }, `The engine offers: ${describeFilters(request)}`));
    const row = el('label', { class: 'filedlg-row' }, 'File name: ');
    const name = el('input', { type: 'text', id: 'filedlg-name', value: suggestedSaveName(request), size: '40', autocomplete: 'off', spellcheck: 'false' });
    row.append(name);
    const ok = el('button', { type: 'button', id: 'filedlg-ok' }, request.okLabel ?? 'Save');
    this.body.append(row, el('p', { class: 'muted' }, 'The engine writes the file inside this tab (its virtual volume). When it has finished, the file is listed under "Files the engine wrote" and you download it from there.'), ok, this.cancelButton());
    const go = (): void => {
      const plan = planSave(request, name.value);
      if (!plan.ok) { this.refuse(id, plan.reason); return; }
      this.answer(id, [{ role: 'S', path: plan.value.path }], 0);
    };
    ok.addEventListener('click', go);
    name.addEventListener('keydown', (e) => { if (e.key === 'Enter') go(); });
    queueMicrotask(() => name.focus({ preventScroll: true }));
  }

  private refuse(id: number, reason: string): void {
    this.status.textContent = reason;
    this.status.dataset.error = '1';
    const c = this.current;
    if (c) this.finish(id, c.request.save ? 'save' : 'open', 'refused', 0, 0);
  }

  private answer(id: number, items: FileDlgItem[], bytes: number): void {
    const c = this.current;
    if (!c || c.id !== id) return;
    delete this.status.dataset.error;
    this.status.textContent = items.some((i) => i.file) ? 'Giving the file to the engine...' : 'Answering the engine...';
    this.finish(id, c.request.save ? 'save' : 'open', 'answered', items.length, bytes);
    this.host.answer(id, { ok: true, items });
    this.current = null;
    this.finishing = id;
    this.body.replaceChildren();
  }

  /** the engine is idle again: a dialog that was answered is over */
  engineIdle(): void {
    if (this.finishing !== null && this.current === null) this.hide();
  }
}

/** the list of files the engine wrote (Save dialog targets): each can be downloaded; the bytes stay in this page's memory */
export class SavedFilesView {
  private readonly list: HTMLElement;
  private readonly section: HTMLElement;
  readonly files: { name: string; guestPath: string; size: number; data: ArrayBuffer; at: number }[] = [];
  constructor(section: HTMLElement, list: HTMLElement) { this.section = section; this.list = list; }

  add(f: { name: string; guestPath: string; size: number; data: ArrayBuffer }): void {
    const i = this.files.findIndex((x) => x.guestPath.toLowerCase() === f.guestPath.toLowerCase());
    const rec = { ...f, at: Date.now() };
    if (i >= 0) this.files[i] = rec; else this.files.push(rec);
    this.render();
  }

  private render(): void {
    this.section.hidden = this.files.length === 0;
    this.list.replaceChildren(...this.files.map((f) => {
      const li = el('li', { 'data-name': f.name });
      li.append(el('span', {}, `${f.name} - ${f.size.toLocaleString('en-US')} bytes `));
      const b = el('button', { type: 'button', class: 'saved-download' }, 'Download');
      b.addEventListener('click', () => {
        const url = URL.createObjectURL(new Blob([f.data], { type: 'application/octet-stream' }));
        const a = el('a', { href: url, download: f.name });
        document.body.append(a);
        a.click();
        a.remove();
        setTimeout(() => URL.revokeObjectURL(url), 60000);
      });
      li.append(b);
      return li;
    }));
  }
}
