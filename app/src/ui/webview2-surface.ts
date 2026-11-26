// Issue #1362: the Browser panel's WebView2 face. The engine's WebView2 model (lift/runtime/fle_webview2.c) draws nothing, and the browser itself is the web engine, so the page shows the
// controller's area as an iframe over the desktop canvas: at the rectangle that put_Bounds gave (moved by the window's client origin on the guest's screen), hidden while put_IsVisible says 0.
// Its content is the resource that Navigate named (a lazy file of the bundle, with the references to the other resources as blob: URLs).
// The page's CSP (shared/headers.ts: style-src and script-src 'self') forbids the inline <style> and <script> of a document inside the page, so the face takes its styles as constructed
// style sheets (CSSOM, which the CSP does not stop) and its background from the page, and the bridge (window.chrome.webview) is made by the page as the window's own object once the
// document has loaded. The frame runs no script of its own (sandbox: allow-same-origin only), so the pointer and keyboard input stay with the frame and are not sent to the engine.
import type { FromWorker } from '../engine/protocol.ts';
import { suppressContextMenu } from '../input/context-menu.ts';

/** the mark this Issue puts on the face's iframe (tools/browser-panel/measure.mjs counts the iframes that carry it) */
export const FACE_ATTR = 'data-fl-webview2';
/** what the face shows where its document has no background: the colour the error page's own CSS names outside its body (a choice: FL sets no colour on the controller) */
export const FACE_BACKGROUND = '#142028';

export interface FaceBounds { left: number; top: number; right: number; bottom: number }
export interface FaceRect { left: number; top: number; width: number; height: number }

/** the face's rectangle in CSS pixels: the controller's bounds (guest pixels, from its window's client origin on the guest's screen) at `scale` CSS pixels per guest pixel */
export function faceRect(b: FaceBounds, origin: { x: number; y: number } | null, scale: number): FaceRect {
  const ox = origin?.x ?? 0, oy = origin?.y ?? 0;
  return { left: (ox + b.left) * scale, top: (oy + b.top) * scale, width: (b.right - b.left) * scale, height: (b.bottom - b.top) * scale };
}

export interface Box { left: number; top: number; right: number; bottom: number }

/** the parts of `b` that no box of `over` covers: disjoint boxes, none of them empty (Issue #1376: the face minus the windows painted over it) */
export function subtractBoxes(b: Box, over: Box[]): Box[] {
  let parts: Box[] = [b];
  for (const o of over) {
    const next: Box[] = [];
    for (const p of parts) {
      if (o.left >= p.right || o.right <= p.left || o.top >= p.bottom || o.bottom <= p.top) { next.push(p); continue; }
      const top = Math.max(o.top, p.top), bottom = Math.min(o.bottom, p.bottom);
      if (o.top > p.top) next.push({ left: p.left, top: p.top, right: p.right, bottom: o.top });
      if (o.bottom < p.bottom) next.push({ left: p.left, top: o.bottom, right: p.right, bottom: p.bottom });
      if (o.left > p.left) next.push({ left: p.left, top: top, right: o.left, bottom: bottom });
      if (o.right < p.right) next.push({ left: o.right, top: top, right: p.right, bottom: bottom });
    }
    parts = next;
  }
  return parts.filter((p) => p.right > p.left && p.bottom > p.top);
}

/** the clip-path of the face (`b`, in the face's own CSS pixels) for the windows `over` painted over it: the parts of the face that they do not cover, as path(evenodd, ...); '' when no window meets the face */
export function faceClipPath(b: Box, over: Box[]): string {
  const hit = over.filter((o) => o.left < b.right && o.right > b.left && o.top < b.bottom && o.bottom > b.top);
  if (hit.length === 0) return '';
  const r = (v: number): number => Math.round(v * 1000) / 1000;
  const parts = subtractBoxes(b, hit);
  return `path(evenodd, "${parts.map((p) => `M${r(p.left)} ${r(p.top)}H${r(p.right)}V${r(p.bottom)}H${r(p.left)}Z`).join(' ')}")`;
}

/** the document without its <style> blocks, and the blocks' CSS (the page sets them up as style sheets: installing them as elements would be refused by the CSP) */
export function splitStyles(html: string): { markup: string; styles: string[] } {
  const styles: string[] = [];
  const markup = html.replace(/<style[^>]*>([\s\S]*?)<\/style\s*>/gi, (_, css: string) => {
    styles.push(css);
    return '';
  });
  return { markup, styles };
}

/** the document without its scripts and its event handlers (Issue #1419): the frame has no allow-scripts, so it cannot run them, and each one that the page reaches logs
 *  "Blocked script execution" as an error on every load of the document; the handlers are removed from their tags, the rest of the markup is kept */
export function withoutScripts(html: string): string {
  return html
    .replace(/<script\b[^>]*>[\s\S]*?<\/script\s*>/gi, '')
    .replace(/<script\b[^>]*>/gi, '')
    .replace(/<[a-z][^>]*>/gi, (tag) => tag.replace(/\s+on[a-z]+\s*=\s*("[^"]*"|'[^']*'|[^\s"'>]+)/gi, ''));
}

/** the document without what the page's CSP (shared/headers.ts: style-src and font-src 'self') stops in the frame: its inline style attributes and its data: fonts (Issue #1419);
 *  each one logs an error on every load. A font that cannot load is the fallback font, which is the face's look without the rule */
export function withoutCspBlocked(html: string): string {
  return html
    .replace(/<[a-z][^>]*>/gi, (tag) => tag.replace(/\sstyle\s*=\s*("[^"]*"|'[^']*'|[^\s"'>]+)/gi, ''))
    .replace(/@font-face\s*\{[^}]*\bdata:[^}]*\}/gi, '');
}

/** a reference that names a file of the bundle (not an absolute URL, a fragment, or a root-relative path) */
export const isRelativeRef = (v: string): boolean => v !== '' && !/^([a-z][a-z0-9+.-]*:|#|\/)/i.test(v);

/** the path of `rel` under `dir` ('System/Internet/' + '../x.png' is 'System/x.png') */
export function joinPath(dir: string, rel: string): string {
  const out: string[] = [];
  for (const part of (dir + rel).split('/')) {
    if (part === '' || part === '.') continue;
    if (part === '..') out.pop();
    else out.push(part);
  }
  return out.join('/');
}

/** the references of the document (src and href attributes, url(...) of its styles) that name a resource of the bundle: each becomes a blob: URL made by `url`; the others are left as they are */
export async function rewriteRefs(html: string, dir: string, resource: (path: string) => Promise<Uint8Array | null>, url: (bytes: Uint8Array, path: string) => string): Promise<string> {
  const attr = /(\s(?:src|href)=)(["'])([^"']*)\2/gi;
  const css = /url\((["']?)([^"')]*)\1\)/gi;
  const rels = new Set<string>();
  for (const m of html.matchAll(attr)) if (isRelativeRef(m[3] ?? '')) rels.add(m[3] as string);
  for (const m of html.matchAll(css)) if (isRelativeRef(m[2] ?? '')) rels.add(m[2] as string);
  const to = new Map<string, string>();
  for (const rel of rels) {
    const path = joinPath(dir, rel);
    const bytes = await resource(`resources/${path}`);
    if (bytes) to.set(rel, url(bytes, path));
  }
  if (to.size === 0) return html;
  return html
    .replace(attr, (whole, pre: string, q: string, v: string) => (to.has(v) ? `${pre}${q}${to.get(v)}${q}` : whole))
    .replace(css, (whole, q: string, v: string) => (to.has(v) ? `url(${q}${to.get(v)}${q})` : whole));
}

/** the directory of a resource path ('resources/System/Internet/a.html' is 'System/Internet/') */
export function dirOf(resourcePath: string): string {
  const p = resourcePath.replace(/^resources\//, '');
  return p.slice(0, p.lastIndexOf('/') + 1);
}

export interface FaceHost {
  /** CSS pixels per guest pixel of the desktop */
  scale(): number;
  /** the bytes of a lazy file of the bundle, once it has arrived; null when the bundle has no such file or its download failed */
  resource(path: string): Promise<Uint8Array | null>;
  /** the document of the navigation has loaded */
  loaded(): void;
  /** a message the document posted (window.chrome.webview.postMessage) */
  posted(json: string): void;
  log(level: 'info' | 'warn' | 'error', text: string): void;
}

type FaceWindow = Window & { chrome?: unknown; CSSStyleSheet: new () => CSSStyleSheet };

/** the face: one wrapper over the desktop (clipping the iframe to it) and one iframe in it */
export class WebView2Face {
  private wrap: HTMLElement | null = null;
  private frame: HTMLIFrameElement | null = null;
  private bounds: FaceBounds | null = null;
  private origin: { x: number; y: number } | null = null;
  /** Issue #1376: the windows painted over the face, l, t, r, b each in the guest's pixels on the guest's screen (the same space as the bounds' origin) */
  private front: number[] = [];
  private clipValue = '';
  private visible = true;
  private navSeq = 0;
  private awaitingLoad = false;
  private styles: string[] = [];
  private listeners: ((e: { data: string }) => void)[] = [];
  private readonly root: HTMLElement;
  private readonly host: FaceHost;

  constructor(root: HTMLElement, host: FaceHost) {
    this.root = root;
    this.host = host;
  }

  handle(m: Extract<FromWorker, { t: 'webview2' }>): void {
    switch (m.op) {
      case 'bounds':
        this.bounds = { left: m.left, top: m.top, right: m.right, bottom: m.bottom };
        this.origin = m.originX !== null && m.originY !== null ? { x: m.originX, y: m.originY } : null;
        this.ensure();
        this.place();
        this.clip();
        break;
      case 'visible':
        this.visible = m.visible;
        this.ensure();
        if (this.wrap) this.wrap.style.display = this.visible ? '' : 'none';
        break;
      case 'front': this.front = m.rects; this.clip(); break;
      case 'navigate': void this.navigate(m.resource); break;
      case 'post': this.send(m.json); break;
      case 'exec': this.host.log('info', `the engine runs a script in WebView2 (${m.script.length} characters): the page does not run it`); break;
    }
  }

  /** the desktop was laid out again: the scale may have changed */
  relayout(): void { this.place(); this.clip(); }

  /** the windows painted over the face (Issue #1376, a test aid): their rectangles, l, t, r, b each, in the guest's pixels */
  frontRects(): number[] { return this.front.slice(); }

  /** Issue #1376: the face shows the part of its rectangle that no window painted over it covers; the rest is the canvas's, which draws those windows. The clip-path is written only when it changes */
  private clip(): void {
    if (!this.frame) return;
    let d = '';
    if (this.bounds) {
      const k = this.host.scale();
      const r = faceRect(this.bounds, this.origin, k);
      const over: Box[] = [];
      for (let i = 0; i + 3 < this.front.length; i += 4) {
        const [l, t, rt, b] = this.front.slice(i, i + 4) as [number, number, number, number];
        over.push({ left: l * k - r.left, top: t * k - r.top, right: rt * k - r.left, bottom: b * k - r.top });
      }
      d = faceClipPath({ left: 0, top: 0, right: r.width, bottom: r.height }, over);
    }
    if (d === this.clipValue) return;
    this.clipValue = d;
    this.frame.style.clipPath = d;
  }

  private ensure(): void {
    if (this.frame) return;
    const wrap = document.createElement('div');
    wrap.style.position = 'absolute';
    wrap.style.left = '0';
    wrap.style.top = '0';
    wrap.style.right = '0';
    wrap.style.bottom = '0';
    wrap.style.overflow = 'hidden';
    wrap.style.pointerEvents = 'none';
    wrap.style.display = this.visible ? '' : 'none';
    const frame = document.createElement('iframe');
    frame.setAttribute(FACE_ATTR, '1');
    frame.title = 'WebView2';
    frame.setAttribute('sandbox', 'allow-same-origin');
    frame.style.position = 'absolute';
    frame.style.border = '0';
    frame.style.margin = '0';
    frame.style.padding = '0';
    frame.style.background = FACE_BACKGROUND;
    frame.style.pointerEvents = 'auto';
    frame.addEventListener('load', () => this.onLoad());
    frame.srcdoc = '';
    wrap.append(frame);
    this.root.append(wrap);
    this.wrap = wrap;
    this.frame = frame;
  }

  private onLoad(): void {
    if (this.frame) this.install(this.frame);
    if (!this.awaitingLoad) return;
    this.awaitingLoad = false;
    this.host.loaded();
  }

  /** the document's background, its styles (as style sheets) and the bridge (window.chrome.webview), set by the page once the document has loaded */
  private install(frame: HTMLIFrameElement): void {
    const win = frame.contentWindow as FaceWindow | null;
    const doc = frame.contentDocument;
    if (!win || !doc) { this.host.log('warn', 'the face has no document to set up'); return; }
    suppressContextMenu(win);   // Issue #1482: a frame's events do not reach the page's document
    doc.documentElement.style.background = FACE_BACKGROUND;
    doc.adoptedStyleSheets = this.styles.map((css) => {
      const sheet = new win.CSSStyleSheet();
      sheet.replaceSync(css);
      return sheet;
    });
    this.listeners = [];
    win.chrome = {
      webview: {
        postMessage: (m: unknown) => this.host.posted(typeof m === 'string' ? m : JSON.stringify(m)),
        addEventListener: (type: string, fn: (e: { data: string }) => void) => { if (type === 'message') this.listeners.push(fn); },
        removeEventListener: (_type: string, fn: (e: { data: string }) => void) => { this.listeners = this.listeners.filter((g) => g !== fn); },
      },
    };
  }

  private place(): void {
    if (!this.frame || !this.bounds) return;
    const r = faceRect(this.bounds, this.origin, this.host.scale());
    const s = this.frame.style;
    s.left = `${r.left}px`;
    s.top = `${r.top}px`;
    s.width = `${r.width}px`;
    s.height = `${r.height}px`;
  }

  private async navigate(resource: string | null): Promise<void> {
    this.ensure();
    const seq = ++this.navSeq;
    const bytes = resource === null ? null : await this.host.resource(resource);
    if (seq !== this.navSeq || !this.frame) return;
    let markup = '', styles: string[] = [];
    if (bytes && resource !== null) {
      const html = new TextDecoder().decode(bytes);
      const refs = await rewriteRefs(withoutCspBlocked(withoutScripts(html)), dirOf(resource), (p) => this.host.resource(p), (b) => URL.createObjectURL(new Blob([b.slice().buffer as ArrayBuffer], { type: 'application/octet-stream' })));
      ({ markup, styles } = splitStyles(refs));
    }
    if (seq !== this.navSeq || !this.frame) return;
    this.styles = styles;
    this.awaitingLoad = true;
    this.frame.srcdoc = markup;
  }

  /** FL → page: the message reaches the listeners the document added (PostWebMessageAsJson) */
  private send(json: string): void {
    for (const fn of this.listeners) fn({ data: json });
  }
}
