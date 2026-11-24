// The page's side of the engine's clipboard (#1389). The engine's Copy (SetClipboardData, CF_UNICODETEXT) comes here as text and is written to the operating system's clipboard
// with navigator.clipboard.writeText. The engine's Paste (GetClipboardData) asks for the text, which is read with navigator.clipboard.readText. The browser decides what the
// page may do: navigator.permissions gives the state of 'clipboard-read' and 'clipboard-write'. When the browser says no ('denied'), there is no Clipboard API, or a call fails,
// the page shows the reason in a status line of its own (role="status", outside the engine's picture) and goes on: the engine gets no text (its clipboard reads as empty)
// and is not stopped.

export type ClipboardPermission = 'clipboard-read' | 'clipboard-write';

/** what the page knows of the clipboard: the browser's state of each permission ('unknown' when the browser does not tell), the calls made, the text last written and last read (and when the last read was asked, performance.now() of the page), and the reason the page last showed (null: the last call worked) */
export interface ClipboardState {
  readPermission: string;
  writePermission: string;
  writes: number;
  reads: number;
  lastWritten: string | null;
  lastRead: string | null;
  lastReadAt: number | null;
  message: string | null;
}

const NO_API = 'This browser has no Clipboard API (navigator.clipboard). Copy and Paste do not reach the system clipboard.';
const WRITE_DENIED = 'The browser does not allow this page to write the clipboard (clipboard-write: denied). Copy did not reach the system clipboard. Allow it in the browser\'s site settings.';
const READ_DENIED = 'The browser does not allow this page to read the clipboard (clipboard-read: denied). Paste gets no text. Allow it in the browser\'s site settings.';

const describe = (e: unknown): string => (e instanceof Error ? `${e.name}: ${e.message}` : String(e));

/** the browser's state of a permission, or 'unknown' when the browser has no Permissions API or does not know the name */
async function permissionState(name: ClipboardPermission): Promise<string> {
  try {
    const p = await navigator.permissions.query({ name } as unknown as PermissionDescriptor);
    return p.state;
  } catch {
    return 'unknown';
  }
}

export class PageClipboard {
  readonly state: ClipboardState = { readPermission: 'unknown', writePermission: 'unknown', writes: 0, reads: 0, lastWritten: null, lastRead: null, lastReadAt: null, message: null };
  private status: HTMLElement | null = null;
  private readonly doc: Document;

  constructor(doc: Document) { this.doc = doc; }

  /** the browser's state of both permissions, for the record (each call asks again when it needs the answer) */
  async refresh(): Promise<void> {
    this.state.readPermission = await permissionState('clipboard-read');
    this.state.writePermission = await permissionState('clipboard-write');
  }

  /** the engine's Copy: the text goes to the operating system's clipboard */
  async write(text: string): Promise<void> {
    this.state.writes++;
    this.state.lastWritten = text;
    const api = navigator.clipboard;
    if (!api || typeof api.writeText !== 'function') { this.say(NO_API); return; }
    this.state.writePermission = await permissionState('clipboard-write');
    if (this.state.writePermission === 'denied') { this.say(WRITE_DENIED); return; }
    try {
      await api.writeText(text);
      this.say(null);
    } catch (e) {
      this.say(`The browser did not take the text for the clipboard (${describe(e)}). Copy did not reach the system clipboard.`);
    }
  }

  /** the engine's Paste: the operating system's clipboard text, or null when the browser does not give it (the reason is shown) */
  async read(): Promise<string | null> {
    this.state.reads++;
    this.state.lastReadAt = performance.now();
    const api = navigator.clipboard;
    if (!api || typeof api.readText !== 'function') { this.say(NO_API); return null; }
    this.state.readPermission = await permissionState('clipboard-read');
    if (this.state.readPermission === 'denied') { this.say(READ_DENIED); this.state.lastRead = null; return null; }
    try {
      const text = await api.readText();
      this.state.lastRead = text;
      this.say(null);
      return text;
    } catch (e) {
      this.state.lastRead = null;
      this.say(`The browser did not give the clipboard text (${describe(e)}). Paste gets no text.`);
      return null;
    }
  }

  /** the reason in the page's status line, or the line hidden (null) */
  private say(message: string | null): void {
    this.state.message = message;
    if (message === null) {
      if (this.status) this.status.hidden = true;
      return;
    }
    if (!this.status) {
      const el = this.doc.createElement('div');
      el.id = 'clipboard-status';
      el.setAttribute('role', 'status');
      el.style.cssText = 'position:fixed;left:8px;bottom:8px;max-width:560px;padding:6px 10px;background:#fff3cd;color:#222;border:1px solid #d6b656;font:12px system-ui,sans-serif;z-index:50';
      this.doc.body.append(el);
      this.status = el;
    }
    this.status.textContent = message;
    this.status.hidden = false;
  }
}
