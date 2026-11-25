// The page's side of the engine's clipboard (#1389, #1406). The engine's Copy (SetClipboardData, CF_UNICODETEXT) comes here as text and is written to the operating system's clipboard
// with navigator.clipboard.writeText. The engine asks three kinds of question, and the page answers each one by what it is:
//  - a question about the formats (IsClipboardFormatAvailable, CountClipboardFormats, EnumClipboardFormats: probe()) is no paste. It reads nothing and shows nothing. The answer:
//    (a) the text the page knows (the last paste's or Copy's text, or a text read) decides; (b) 'denied': no text; (c) 'granted': navigator.clipboard.read() lists the formats
//    (no prompt; kept FORMATS_MS); (d) 'prompt' with nothing known: text (the text is not read here, so no prompt comes at a right click).
//  - a GetClipboardData right after a right button release is FL's right click: the knob's menu asks whether the clipboard holds a value (its check of the text). It gets the
//    text the page knows; with none it gets PLACEHOLDER, and nothing is read (a 'denied' page gets no text). The menu's 'Paste value' is the click after that, and it is a paste.
//  - a paste: Ctrl+V gives the paste event's text (the browser fires paste for the focused desktop because the desktop does not cancel the key: see keymap.ts), which the next
//    GetClipboardData gets, with no read and no prompt. Any other GetClipboardData (a paste from a menu) reads the clipboard: readText, which asks the person when the permission is
//    'prompt'; 'denied' reads nothing and the reason is shown.
// The reason of a refused paste or Copy is shown in the page's status line (#clipboard-status, role="status") for TOAST_MS (5 s), or until its close button. A question never shows it. Issue #1481: style.css hides that element (no notice is shown; the reason stays in state.message).
// The browser decides what the page may do: navigator.permissions gives the state of 'clipboard-read' and 'clipboard-write'. The engine is not stopped by any of this.

export type ClipboardPermission = 'clipboard-read' | 'clipboard-write';

/** what the page knows of the clipboard: the browser's state of each permission ('unknown' when the browser does not tell), the calls made, the text last written and last read (and when the last read was asked, performance.now() of the page), and the reason the page last showed (null: nothing shown) */
export interface ClipboardState {
  readPermission: string;
  writePermission: string;
  writes: number;
  reads: number;
  probes: number;
  pastes: number;
  lastWritten: string | null;
  lastRead: string | null;
  lastReadAt: number | null;
  message: string | null;
}

const NO_API = 'This browser has no Clipboard API (navigator.clipboard). Copy and Paste do not reach the system clipboard.';
const WRITE_DENIED = 'The browser does not allow this page to write the clipboard (clipboard-write: denied). Copy did not reach the system clipboard. Allow it in the browser\'s site settings.';
const READ_DENIED = 'The browser does not allow this page to read the clipboard (clipboard-read: denied). Paste gets no text. Allow it in the browser\'s site settings.';
/** the text a question's answer gets when the page knows no text and reads none: a number, which FL's value check takes */
const PLACEHOLDER = '0';
/** a GetClipboardData this soon after a right button release is FL's right-click question, not a paste */
const QUESTION_MS = 1500;
/** the formats navigator.clipboard.read() lists are kept this long for the questions */
const FORMATS_MS = 1000;
/** the reason stays this long (5 s), then it is hidden */
const TOAST_MS = 5000;

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
  readonly state: ClipboardState = { readPermission: 'unknown', writePermission: 'unknown', writes: 0, reads: 0, probes: 0, pastes: 0, lastWritten: null, lastRead: null, lastReadAt: null, message: null };
  /** the text the page knows: a paste's text, a Copy's text, a text read (null: none yet) */
  private known: string | null = null;
  /** the text of the last paste, for the one GetClipboardData that the paste makes (null: none) */
  private pending: string | null = null;
  private lastButton: { button: number; at: number } | null = null;
  private formats: { at: number; has: boolean } | null = null;
  private status: HTMLElement | null = null;
  private text: HTMLElement | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private readonly doc: Document;

  constructor(doc: Document) {
    this.doc = doc;
    doc.addEventListener('pointerup', (e) => { this.lastButton = { button: e.button, at: performance.now() }; }, true);
    doc.addEventListener('pointerdown', () => { this.pending = null; }, true);
    doc.addEventListener('paste', (e) => {
      if ((e.target as Element | null)?.id !== 'desktop') return;
      this.pasted(e.clipboardData?.getData('text/plain') ?? '');
    });
  }

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
    if (!api || typeof api.writeText !== 'function') { this.toast(NO_API); return; }
    this.state.writePermission = await permissionState('clipboard-write');
    if (this.state.writePermission === 'denied') { this.toast(WRITE_DENIED); return; }
    try {
      await api.writeText(text);
      this.known = text;
      this.formats = null;
      this.hide();
    } catch (e) {
      this.toast(`The browser did not take the text for the clipboard (${describe(e)}). Copy did not reach the system clipboard.`);
    }
  }

  /** the paste event of the desktop: its text is the paste's, and it is the next GetClipboardData's */
  pasted(text: string): void {
    this.state.pastes++;
    this.pending = text;
    this.known = text;
    this.formats = null;
  }

  /** a question about the formats: whether the clipboard holds text. It reads no content and shows nothing */
  async probe(): Promise<boolean> {
    this.state.probes++;
    const api = navigator.clipboard;
    if (!api || typeof api.readText !== 'function') return false;
    if (this.known !== null) return this.known.length > 0;
    this.state.readPermission = await permissionState('clipboard-read');
    if (this.state.readPermission === 'denied') return false;
    if (this.state.readPermission === 'granted') return this.hasText();
    return true;
  }

  /** the engine's GetClipboardData of the text: the paste's text, else FL's right-click question (the known text, or PLACEHOLDER), else the menu's paste (readText); null when the browser does not give it (the reason is shown) */
  async read(): Promise<string | null> {
    this.state.reads++;
    this.state.lastReadAt = performance.now();
    if (this.pending !== null) {
      const text = this.pending;
      this.pending = null;
      this.state.lastRead = text;
      this.hide();
      return text;
    }
    if (this.isQuestion()) {
      if (this.known !== null) return this.known.length > 0 ? this.known : null;
      return (await permissionState('clipboard-read')) === 'denied' ? null : PLACEHOLDER;
    }
    const api = navigator.clipboard;
    if (!api || typeof api.readText !== 'function') { this.toast(NO_API); return null; }
    this.state.readPermission = await permissionState('clipboard-read');
    if (this.state.readPermission === 'denied') { this.state.lastRead = null; this.toast(READ_DENIED); return null; }
    try {
      const text = await api.readText();
      this.state.lastRead = text;
      this.known = text;
      this.hide();
      return text;
    } catch (e) {
      this.state.lastRead = null;
      this.toast(`The browser did not give the clipboard text (${describe(e)}). Paste gets no text.`);
      return null;
    }
  }

  /** the last button released was the right one, a moment ago: the engine asks for the text as FL's right-click menu does */
  private isQuestion(): boolean {
    return this.lastButton !== null && this.lastButton.button === 2 && performance.now() - this.lastButton.at < QUESTION_MS;
  }

  /** whether the browser's clipboard lists a text format (navigator.clipboard.read: no prompt when the permission is granted), kept for FORMATS_MS */
  private async hasText(): Promise<boolean> {
    const now = performance.now();
    if (this.formats !== null && now - this.formats.at < FORMATS_MS) return this.formats.has;
    let has = false;
    try {
      const items = await navigator.clipboard.read();
      has = items.some((item) => item.types.includes('text/plain'));
    } catch {
      has = false;
    }
    this.formats = { at: performance.now(), has };
    return has;
  }

  /** the reason in the page's status line; one line at a time: a reason shown again while it is up only moves its hiding */
  private toast(message: string): void {
    this.state.message = message;
    if (!this.status || !this.text) this.build();
    if (this.text) this.text.textContent = message;
    if (this.status) this.status.hidden = false;
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = setTimeout(() => this.hide(), TOAST_MS);
  }

  private hide(): void {
    this.state.message = null;
    if (this.timer !== null) { clearTimeout(this.timer); this.timer = null; }
    if (this.status) this.status.hidden = true;
  }

  private build(): void {
    const el = this.doc.createElement('div');
    el.id = 'clipboard-status';
    el.setAttribute('role', 'status');
    el.style.cssText = 'position:fixed;left:8px;bottom:8px;max-width:560px;padding:6px 10px;background:#fff3cd;color:#222;border:1px solid #d6b656;font:12px system-ui,sans-serif;z-index:50';
    const text = this.doc.createElement('span');
    const close = this.doc.createElement('button');
    close.type = 'button';
    close.textContent = '×';
    close.setAttribute('aria-label', 'Close');
    close.style.cssText = 'margin-left:8px;font:12px system-ui,sans-serif';
    close.addEventListener('click', () => this.hide());
    el.append(text, close);
    this.doc.body.append(el);
    this.status = el;
    this.text = text;
  }
}
