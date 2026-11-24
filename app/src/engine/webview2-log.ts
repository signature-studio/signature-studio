// Issue #1362: the lines of the engine's WebView2 model (lift/runtime/fle_webview2.c). The engine Worker reads its stderr with this, and turns the calls that the page's face needs into messages (protocol.ts, `webview2`).
// A line is `webview2: <method> (<interface>) <arguments>`. A text argument is UTF-16 written with the backslash doubled and each unit outside 0x20..0x7E (and the quote) as \uXXXX;
// a text of more than 300 units is cut, and its `units=` is then more than 300 (such a text is not used).
import type { FromWorker } from './protocol.ts';

export type WebView2Message = Extract<FromWorker, { t: 'webview2' }>;

const LINE = /^(?:\[err\] )?webview2: (\S+) \(([^)]*)\)(?: (.*))?$/;
const BOUNDS = /^bounds=(-?\d+),(-?\d+),(-?\d+),(-?\d+)(?: origin=(?:(-?\d+),(-?\d+)|none))?$/;
const VISIBLE = /^isVisible=([01])$/;
const TEXT = /^(uri|message|script)="(.*)" units=(\d+)(?: handler=\S+)?$/;
const WHOLE_UNITS = 300;

/** the text as the model wrote it: the backslash doubled, the other units as \uXXXX */
export function decodeText(s: string): string {
  return s.replace(/\\(\\|u[0-9a-fA-F]{4})/g, (_, g: string) => (g === '\\' ? '\\' : String.fromCharCode(parseInt(g.slice(1), 16))));
}

/** the resource of the bundle that a file: URI names, as its lazy file's path (`resources/System/Internet/error-offline.html` for `...\FL Studio 21\System\Internet\error-offline.html`); null for any other URI */
export function resourceOf(uri: string): string | null {
  if (!/^file:\/\//i.test(uri)) return null;
  const k = uri.indexOf('\\System\\');
  if (k < 0) return null;
  return `resources/${uri.slice(k + 1).split('\\').join('/')}`;
}

/** the message of one line of the model's log, or null for a line the page does not need */
export function webView2Message(line: string): WebView2Message | null {
  const m = LINE.exec(line);
  if (!m) return null;
  const method = m[1] as string, args = m[3] ?? '';
  if (method === 'put_Bounds') {
    const b = BOUNDS.exec(args);
    if (!b) return null;
    const hasOrigin = b[5] !== undefined;
    return { t: 'webview2', op: 'bounds', left: +(b[1] as string), top: +(b[2] as string), right: +(b[3] as string), bottom: +(b[4] as string), originX: hasOrigin ? +(b[5] as string) : null, originY: hasOrigin ? +(b[6] as string) : null };
  }
  if (method === 'put_IsVisible') {
    const v = VISIBLE.exec(args);
    return v ? { t: 'webview2', op: 'visible', visible: v[1] === '1' } : null;
  }
  const t = TEXT.exec(args);
  if (!t || +(t[3] as string) > WHOLE_UNITS) return null;
  const text = decodeText(t[2] as string);
  if (method === 'Navigate' && t[1] === 'uri') return { t: 'webview2', op: 'navigate', uri: text, resource: resourceOf(text) };
  if ((method === 'PostWebMessageAsJson' || method === 'PostWebMessageAsString') && t[1] === 'message') return { t: 'webview2', op: 'post', json: text };
  if (method === 'ExecuteScript' && t[1] === 'script') return { t: 'webview2', op: 'exec', script: text };
  return null;
}
