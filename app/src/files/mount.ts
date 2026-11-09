// Mounts the file manager into the page (decision 0216, opfs1): a 'Files' button in the header and an overlay (a dialog above the engine's windows; the page's layout is not touched).
// The window is built when it is first opened. window.__files is the handle for the tests and the console; it carries no file names.
import './files.css';
import { Explorer } from './explorer.ts';
import { OpfsFs } from './opfs.ts';
import { filesUsage } from './usage.ts';

export interface FilesHandle {
  readonly open: boolean;
  openWindow(path?: string): Promise<Explorer>;
  closeWindow(): void;
  readonly explorer: Explorer | null;
}

export function mountFiles(header: HTMLElement, onClosed: () => void): FilesHandle {
  const btn = document.createElement('button');
  btn.type = 'button';
  btn.id = 'files-btn';
  btn.textContent = 'Files';
  btn.title = 'Browser files (OPFS): put a project folder or zip into this browser, manage files';
  const status = header.querySelector('#status');
  if (status) header.insertBefore(btn, status); else header.append(btn);
  const overlay = document.createElement('div');
  overlay.id = 'files-overlay';
  overlay.hidden = true;
  overlay.setAttribute('role', 'dialog');
  overlay.setAttribute('aria-modal', 'true');
  overlay.setAttribute('aria-label', 'Browser files');
  document.body.append(overlay);

  let explorer: Explorer | null = null;
  let opening: Promise<Explorer> | null = null;
  const handle: FilesHandle = {
    get open() { return !overlay.hidden; },
    get explorer() { return explorer; },
    async openWindow(path = '/') {
      overlay.hidden = false;
      if (!opening) {
        opening = (async () => {
          const fs = await OpfsFs.open();
          const ex = new Explorer(overlay, fs, { close: () => handle.closeWindow() });
          explorer = ex;
          return ex;
        })();
        opening.catch(() => { opening = null; });
      }
      try {
        const ex = await opening;
        await ex.navigate(path === '/' && ex.cwd !== '/' ? ex.cwd : path);
        overlay.querySelector<HTMLElement>('#fx-list')?.focus({ preventScroll: true });
        return ex;
      } catch (e) {
        overlay.hidden = true;
        overlay.replaceChildren();
        throw e;
      }
    },
    closeWindow() {
      if (overlay.hidden) return;
      overlay.hidden = true;
      onClosed();
    },
  };
  btn.addEventListener('click', () => { filesUsage.c.opens++; void handle.openWindow().catch((e) => { btn.title = `Browser files are not available: ${(e as Error).message}`; btn.disabled = true; }); });
  overlay.addEventListener('keydown', (e) => {
    if (e.key !== 'Tab') return;
    const f = [...overlay.querySelectorAll<HTMLElement>('button:not(:disabled), input:not([hidden]), [tabindex="0"]')].filter((x) => x.offsetParent !== null);
    if (f.length === 0) return;
    const first = f[0]!, last = f[f.length - 1]!;
    if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
    else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
  });
  return handle;
}
