// The page's answer to the engine's file dialogs (decision 0193, fpick1; decision 0216 stage 2, Issue #1519). The engine is suspended in IFileOpenDialog::Show / IFileSaveDialog::Show.
// With the host's `select` (the page's file manager window, files/engine-dialog.ts): every Open, Save and folder request is answered in that window, and no browser chooser is opened.
// Without it (the fallback, kept for a host that has no window): Open opens the browser's own file chooser (a hidden file input, #filedlg-input, several files at once), at once when the browser says a press of a key or a click was just made,
// else at the person's next press of a key or click on the page (the browser opens a chooser only from such a gesture). Save: the engine writes the suggested name, and SavedFilesView hands the written file to the browser's download. A folder dialog is refused.
// The Files the person chooses go to the Worker, never to a server.
import { acceptFor, describeFilters, planOpenFile, planOpenFolder, planSave, suggestedSaveName, FOS_PICKFOLDERS, type FileDlgAnswer, type FileDlgItem, type FileDlgRequest } from '../engine/filedlg.ts';

export interface FileDlgHost {
  answer(id: number, answer: FileDlgAnswer): void;
  /** Issue #1519: the file manager window answers the dialog (Open, Save, folder). It is built in `root` and calls `done` once with the answer; it returns what undoes it */
  select?(id: number, request: FileDlgRequest, root: HTMLElement, done: (a: FileDlgAnswer) => void): () => void;
  /** the dialog is over (answered, cancelled or refused): give the keyboard back to the desktop */
  closed?(): void;
  /** a line for the page's log (what the engine offered, why a choice was refused) */
  log?(kind: 'info' | 'warn', text: string): void;
}

/** a chooser that the browser closes this quickly was never shown to the person (see buildOpen) */
const QUICK_CANCEL_MS = 1000;

const el = <K extends keyof HTMLElementTagNameMap>(tag: K, attrs: Record<string, string> = {}): HTMLElementTagNameMap[K] => {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  return e;
};

export class FileDialogView {
  private readonly root: HTMLElement;
  private readonly host: FileDlgHost;
  private current: { id: number; request: FileDlgRequest } | null = null;
  private input: HTMLInputElement | null = null;
  /** removes the listeners that open the chooser at the person's next gesture (null: none is waiting) */
  private disarm: (() => void) | null = null;
  /** what the host's select window undoes when the dialog is over (null: no window is shown) */
  private disposeSelect: (() => void) | null = null;
  /** when the chooser was last opened (performance.now) */
  private openedAt = -Infinity;
  /** what the page has done with dialogs, for the tests and the page's log (no file names) */
  readonly history: { id: number; kind: 'open' | 'save'; outcome: 'answered' | 'cancelled' | 'refused'; files: number; bytes: number }[] = [];

  constructor(root: HTMLElement, host: FileDlgHost) {
    this.root = root;
    this.host = host;
    root.replaceChildren();
  }

  get open(): boolean { return this.current !== null; }
  get currentId(): number | null { return this.current?.id ?? null; }

  show(id: number, request: FileDlgRequest): void {
    if (this.current) this.cancel('replaced by a newer dialog');
    this.host.log?.('info', `the engine's ${request.save ? 'Save' : 'Open'} dialog offers: ${describeFilters(request)}`);
    if (this.host.select) { this.current = { id, request }; this.buildSelect(id, request); return; }
    if (request.save) { this.saveNow(id, request); return; }
    if (request.options & FOS_PICKFOLDERS) { this.settle(id, 'open', 'refused', { ok: false, reason: 'The engine asked for a folder of its own virtual volume. There is none to choose from on this computer.' }); return; }
    this.current = { id, request };
    this.buildOpen(id, request);
  }

  progress(id: number, done: number, total: number): void {
    if (done < total) return;
    this.host.log?.('info', `the engine's file ${id} is in its memory (${(total / 1048576).toFixed(1)} MiB)`);
  }

  /** the engine ended: nobody is left to answer the dialog it asked */
  hide(): void {
    this.disarmChooser();
    const dispose = this.disposeSelect;
    this.disposeSelect = null;
    dispose?.();
    delete this.root.dataset.select;
    this.current = null;
    this.input = null;
    this.root.replaceChildren();
  }

  /** Issue #1519: the file manager window (the host's select) is the dialog: it is built in the root and answers through answerSelect */
  private buildSelect(id: number, request: FileDlgRequest): void {
    this.root.dataset.select = '1';
    this.disposeSelect = this.host.select!(id, request, this.root, (a) => this.answerSelect(id, a));
  }

  private answerSelect(id: number, a: FileDlgAnswer): void {
    const c = this.current;
    if (!c || c.id !== id) return;
    const kind = c.request.save ? 'save' : 'open';
    if (!a.ok) { this.settle(id, kind, 'cancelled', a); return; }
    this.settle(id, kind, 'answered', a, a.items.length, a.items.reduce((n, i) => n + (i.file?.size ?? 0), 0));
  }

  /** a Save: the suggested name is the target (no box asks for another) */
  private saveNow(id: number, request: FileDlgRequest): void {
    const plan = planSave(request, suggestedSaveName(request));
    if (!plan.ok) { this.host.log?.('warn', plan.reason); this.settle(id, 'save', 'refused', { ok: false, reason: plan.reason }); return; }
    this.host.log?.('info', `the engine saves ${plan.value.name}; the browser's download gets it when the engine has written it`);
    this.settle(id, 'save', 'answered', { ok: true, items: [{ role: 'S', path: plan.value.path }] }, 1, 0);
  }

  private buildOpen(id: number, request: FileDlgRequest): void {
    // no box is shown, and the input has no size: it is laid out (a chooser of an input that is not rendered is cancelled by the browser) but takes no room and is not seen
    const input = el('input', { type: 'file', id: 'filedlg-input', accept: acceptFor(request), multiple: '' });
    input.style.cssText = 'position:fixed;left:0;top:0;width:0;height:0;margin:0;padding:0;border:0;opacity:0;overflow:hidden;pointer-events:none;';
    this.root.replaceChildren(input);
    this.input = input;
    input.addEventListener('change', () => this.chosen(id, request, Array.from(input.files ?? [])));
    input.addEventListener('cancel', () => {
      if (this.input !== input) return;
      // a chooser the browser closes within a second was not shown (no dialog came up): the request stays, and the page waits for the person's next gesture
      if (performance.now() - this.openedAt < QUICK_CANCEL_MS) { this.openChooser(input, false); return; }
      this.cancel('cancelled by the person');
    });
    this.openChooser(input);
  }

  /** the browser opens its chooser only from a gesture of the person: now when the browser says one was just made (and `now`), else at the next key press or click on the page */
  private openChooser(input: HTMLInputElement, now = true): void {
    const go = (): void => { this.disarmChooser(); if (this.input === input) { this.openedAt = performance.now(); input.click(); } };
    if (now && navigator.userActivation?.isActive) { go(); return; }
    document.addEventListener('pointerdown', go, true);
    document.addEventListener('keydown', go, true);
    this.disarm = () => { document.removeEventListener('pointerdown', go, true); document.removeEventListener('keydown', go, true); };
  }

  private disarmChooser(): void {
    const d = this.disarm;
    this.disarm = null;
    d?.();
  }

  /** the files the person chose: one file is put as it is named, several are put together in the Open folder (a project and its samples); the Worker reads them */
  private chosen(id: number, request: FileDlgRequest, list: File[]): void {
    if (this.current?.id !== id || list.length === 0) return;
    const picked = list.map((f) => ({ name: f.name, size: f.size }));
    const only = picked[0];
    const bytes = list.reduce((n, f) => n + f.size, 0);
    if (picked.length === 1 && only !== undefined) {
      const plan = planOpenFile(request, only);
      if (!plan.ok) { this.host.log?.('warn', plan.reason); this.settle(id, 'open', 'refused', { ok: false, reason: plan.reason }); return; }
      const items: FileDlgItem[] = plan.value.items.map((it) => ({ role: it.role, path: it.path, file: list[0] as File }));
      this.settle(id, 'open', 'answered', { ok: true, items }, items.length, bytes);
      return;
    }
    const plan = planOpenFolder(request, picked);
    if (!plan.ok) { this.host.log?.('warn', plan.reason); this.settle(id, 'open', 'refused', { ok: false, reason: plan.reason }); return; }
    const items: FileDlgItem[] = plan.value.items.map((it) => ({ role: it.role, path: it.path, file: list[it.index] as File }));
    this.settle(id, 'open', 'answered', { ok: true, items }, items.length, bytes);
  }

  private cancel(why: string): void {
    const c = this.current;
    if (!c) return;
    this.settle(c.id, c.request.save ? 'save' : 'open', 'cancelled', { ok: false, reason: why });
  }

  /** the dialog is over: the engine gets its answer, and the page forgets the request */
  private settle(id: number, kind: 'open' | 'save', outcome: 'answered' | 'cancelled' | 'refused', answer: FileDlgAnswer, files = 0, bytes = 0): void {
    this.history.push({ id, kind, outcome, files, bytes });
    if (this.history.length > 50) this.history.shift();
    this.hide();
    this.host.answer(id, answer);
    this.host.closed?.();
  }
}

/** the files the engine wrote (Save dialog targets): kept in this page's memory (the tests read them), and handed to the browser's download as soon as the engine has written them (Issue #1481: no list is shown); not when the person chose the place in the file window (Issue #1519: the file is in the window's storage then) */
export class SavedFilesView {
  readonly files: { name: string; guestPath: string; size: number; data: ArrayBuffer; at: number }[] = [];

  add(f: { name: string; guestPath: string; size: number; data: ArrayBuffer }, download = true): void {
    const i = this.files.findIndex((x) => x.guestPath.toLowerCase() === f.guestPath.toLowerCase());
    const rec = { ...f, at: Date.now() };
    if (i >= 0) this.files[i] = rec; else this.files.push(rec);
    if (!download) return;
    const url = URL.createObjectURL(new Blob([f.data], { type: 'application/octet-stream' }));
    const a = el('a', { href: url, download: f.name });
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 60000);
  }
}
