// The page's side of the engine's clipboard (#1389, #1406, #1482). The Clipboard API is the only way the page touches the clipboard: navigator.clipboard.writeText (the engine's Copy)
// and navigator.clipboard.readText (the engine's Paste). There is no paste event, no hidden input, no execCommand and no notice: a refused read or write is silent (the reason is kept in state.refusal).
// The engine asks three kinds of question, and the page answers each one by what it is:
//  - a paste: Ctrl+V reaches the engine as a key (keymap.ts cancels the browser's own paste), and a paste from the menu is a click; both are a GetClipboardData, which reads
//    navigator.clipboard.readText() and answers with its text (or nothing when the browser refuses).
//  - a question about the formats (IsClipboardFormatAvailable, CountClipboardFormats, EnumClipboardFormats: probe()) reads nothing (#1406): the text the page knows decides.
//  - a GetClipboardData right after a right button release is FL's right-click question (the knob's menu checks its value): it reads nothing either (#1406). It gets the text
//    the page knows; with none it gets PLACEHOLDER (a 'denied' page gets no text).
// The permission (#1482): at the page's load the browser's state of 'clipboard-read' is read; under 'prompt' (and under 'granted', which keeps the text the page knows up to date)
// readText is called once, which asks the person under 'prompt'. A request the browser refuses (the document has no focus, ...) is made again at the first press or key.
// Under 'granted' the text is read again when the window gets the focus. A right click never reads or asks.
// The browser decides what the page may do: navigator.permissions gives the state of 'clipboard-read' and 'clipboard-write'. The engine is not stopped by any of this.

export type ClipboardPermission = 'clipboard-read' | 'clipboard-write';

/** what the page knows of the clipboard: the browser's state of each permission ('unknown' when the browser does not tell), the calls made, the text last written and last read
 *  (and when the last read was asked, performance.now() of the page), and the reason of the last refusal (kept, never shown; null: none) */
export interface ClipboardState {
  readPermission: string;
  writePermission: string;
  writes: number;
  reads: number;
  probes: number;
  lastWritten: string | null;
  lastRead: string | null;
  lastReadAt: number | null;
  refusal: string | null;
}

const NO_API = 'This browser has no Clipboard API (navigator.clipboard).';
const WRITE_DENIED = 'clipboard-write: denied';
const READ_DENIED = 'clipboard-read: denied';
/** the text a question's answer gets when the page knows no text and reads none: a number, which FL's value check takes */
const PLACEHOLDER = '0';
/** a GetClipboardData this soon after a right button release is FL's right-click question, not a paste */
const QUESTION_MS = 1500;

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
  readonly state: ClipboardState = { readPermission: 'unknown', writePermission: 'unknown', writes: 0, reads: 0, probes: 0, lastWritten: null, lastRead: null, lastReadAt: null, refusal: null };
  /** the text the page knows: a read's text, a Copy's text (null: none yet) */
  private known: string | null = null;
  private lastButton: { button: number; at: number } | null = null;
  /** the request made at the load was refused by the browser: the first press or key makes it again (once) */
  private askAgain = false;

  /** the page's listeners: the right button's release (the question's clock), a press or key that ends a question, and the window's focus */
  attach(win: Window): void {
    win.addEventListener('pointerdown', () => { this.lastButton = null; void this.firstAction(); }, true);
    win.addEventListener('pointerup', (e) => { this.lastButton = { button: e.button, at: performance.now() }; }, true);
    win.addEventListener('keydown', () => { this.lastButton = null; void this.firstAction(); }, true);
    win.addEventListener('focus', () => { void this.onFocus(); });
  }

  /** at the page's load: the state of 'clipboard-read'; under 'prompt' or 'granted' readText is called (under 'prompt' the browser asks the person) */
  async start(): Promise<void> {
    this.state.readPermission = await permissionState('clipboard-read');
    if (this.state.readPermission !== 'prompt' && this.state.readPermission !== 'granted') return;
    const ok = await this.readKnown();
    this.askAgain = !ok && this.state.readPermission === 'prompt';
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
    if (!api || typeof api.writeText !== 'function') { this.state.refusal = NO_API; return; }
    this.state.writePermission = await permissionState('clipboard-write');
    if (this.state.writePermission === 'denied') { this.state.refusal = WRITE_DENIED; return; }
    try {
      await api.writeText(text);
      this.known = text;
      this.state.refusal = null;
    } catch (e) {
      this.state.refusal = `the browser did not take the text (${describe(e)})`;
    }
  }

  /** a question about the formats: whether the clipboard holds text. It reads no content: the text the page knows decides, else the permission */
  async probe(): Promise<boolean> {
    this.state.probes++;
    const api = navigator.clipboard;
    if (!api || typeof api.readText !== 'function') return false;
    if (this.known !== null) return this.known.length > 0;
    this.state.readPermission = await permissionState('clipboard-read');
    return this.state.readPermission !== 'denied';
  }

  /** the engine's GetClipboardData of the text: FL's right-click question (the known text, or PLACEHOLDER: nothing is read), else a paste: readText. null when the browser
   *  gives no text (the reason is kept in state.refusal) */
  async read(): Promise<string | null> {
    this.state.reads++;
    this.state.lastReadAt = performance.now();
    if (this.isQuestion()) {
      if (this.known !== null) return this.known.length > 0 ? this.known : null;
      return (await permissionState('clipboard-read')) === 'denied' ? null : PLACEHOLDER;
    }
    const api = navigator.clipboard;
    if (!api || typeof api.readText !== 'function') { this.state.refusal = NO_API; return null; }
    this.state.readPermission = await permissionState('clipboard-read');
    if (this.state.readPermission === 'denied') { this.state.lastRead = null; this.state.refusal = READ_DENIED; return null; }
    try {
      const text = await api.readText();
      this.state.lastRead = text;
      this.known = text;
      this.state.refusal = null;
      return text;
    } catch (e) {
      this.state.lastRead = null;
      this.state.refusal = `the browser did not give the clipboard text (${describe(e)})`;
      return null;
    }
  }

  /** the last button released was the right one, a moment ago, and no press or key came since: the engine asks for the text as FL's right-click menu does */
  private isQuestion(): boolean {
    return this.lastButton !== null && this.lastButton.button === 2 && performance.now() - this.lastButton.at < QUESTION_MS;
  }

  /** readText: the answer is the text the page knows. false: the browser refused it (the reason is kept) */
  private async readKnown(): Promise<boolean> {
    const api = navigator.clipboard;
    if (!api || typeof api.readText !== 'function') return false;
    try {
      this.known = await api.readText();
      return true;
    } catch (e) {
      this.state.refusal = `the browser did not give the clipboard text (${describe(e)})`;
      return false;
    }
  }

  /** the first press or key of the page: a request the browser refused at the load is made again, once, when the permission is still 'prompt' */
  private async firstAction(): Promise<void> {
    if (!this.askAgain) return;
    this.askAgain = false;
    if ((await permissionState('clipboard-read')) === 'prompt') await this.readKnown();
  }

  /** the window got the focus: under 'granted' the text the page knows is read again */
  private async onFocus(): Promise<void> {
    this.state.readPermission = await permissionState('clipboard-read');
    if (this.state.readPermission === 'granted') await this.readKnown();
  }
}
