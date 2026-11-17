// What the beacon says about the browser and the machine it runs on (docs/milestones/performance-targets.md section 6: "this machine" is whatever machine opens the page).
// Nothing personal: no IP address, no cookie, no field input, no URL. The user agent, the screen size, the core count and the GPU's renderer name are what a performance
// number needs to be read against.

export interface BrowserInfo {
  hardwareConcurrency: number | null;
  deviceMemoryGB: number | null;
  userAgent: string;
  screen: { width: number; height: number } | null;
  devicePixelRatio: number | null;
  crossOriginIsolated: boolean | null;
  isSecureContext: boolean | null;
  /** UNMASKED_RENDERER_WEBGL, when the browser gives it */
  webglRenderer: string | null;
  /** performance.memory (Chrome only; the JS heap of the page, not the engine's wasm memory) */
  jsHeap: { usedBytes: number; totalBytes: number; limitBytes: number } | null;
}

/** the parts of the browser's globals this reads (so that a test can hand in a fake) */
export interface BrowserGlobals {
  navigator?: { hardwareConcurrency?: number; deviceMemory?: number; userAgent?: string };
  screen?: { width?: number; height?: number };
  devicePixelRatio?: number;
  crossOriginIsolated?: boolean;
  isSecureContext?: boolean;
  performance?: { memory?: { usedJSHeapSize?: number; totalJSHeapSize?: number; jsHeapSizeLimit?: number } };
  document?: { createElement(tag: 'canvas'): { getContext(id: string): unknown } };
}

const num = (v: unknown): number | null => (typeof v === 'number' && Number.isFinite(v) ? v : null);
const bool = (v: unknown): boolean | null => (typeof v === 'boolean' ? v : null);

interface GlLike { getExtension(n: string): { UNMASKED_RENDERER_WEBGL: number } | null; getParameter(p: number): unknown }

/** the GPU renderer string (WEBGL_debug_renderer_info), or null where WebGL or the extension is missing. A throw-away canvas; its context is released again. */
export function webglRenderer(doc: BrowserGlobals['document']): string | null {
  try {
    if (!doc) return null;
    const canvas = doc.createElement('canvas');
    const gl = (canvas.getContext('webgl') ?? canvas.getContext('experimental-webgl')) as (GlLike & { getExtension(n: string): unknown }) | null;
    if (!gl) return null;
    const ext = gl.getExtension('WEBGL_debug_renderer_info');
    const r = ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : null;
    (gl.getExtension('WEBGL_lose_context') as { loseContext?: () => void } | null)?.loseContext?.();
    return typeof r === 'string' && r !== '' ? r.slice(0, 200) : null;
  } catch { return null; }
}

export function collectBrowserInfo(g: BrowserGlobals): BrowserInfo {
  const mem = g.performance?.memory;
  const used = num(mem?.usedJSHeapSize), total = num(mem?.totalJSHeapSize), limit = num(mem?.jsHeapSizeLimit);
  const w = num(g.screen?.width), h = num(g.screen?.height);
  return {
    hardwareConcurrency: num(g.navigator?.hardwareConcurrency),
    deviceMemoryGB: num(g.navigator?.deviceMemory),
    userAgent: String(g.navigator?.userAgent ?? '').slice(0, 300),
    screen: w !== null && h !== null ? { width: w, height: h } : null,
    devicePixelRatio: num(g.devicePixelRatio),
    crossOriginIsolated: bool(g.crossOriginIsolated),
    isSecureContext: bool(g.isSecureContext),
    webglRenderer: webglRenderer(g.document),
    jsHeap: used !== null && total !== null && limit !== null ? { usedBytes: used, totalBytes: total, limitBytes: limit } : null,
  };
}

// ---- decision 0192: what the screen-side measurement needs to be read against (the fields above are not repeated) ----

export interface ClientInfo {
  /** the browser as the user agent says it: name, "major.minor" (not the 4-part number of a reduced Chrome UA: it looks like an address and says nothing more), major as a number, and the OS */
  browser: { name: string; version: string | null; major: number | null; os: string | null; headless: boolean };
  colorDepth: number | null;
  /** the page's viewport (window.innerWidth/innerHeight), CSS px */
  viewport: { width: number; height: number } | null;
  /** how the page was reached: the scheme and a class of the host name (never the name itself): "localhost" 127.x / ::1 / localhost, "ts.net" a Tailscale name, "ip" another address, "other" a name */
  page: { secure: boolean | null; hostKind: 'localhost' | 'ts.net' | 'ip' | 'other' | null };
  /** hardwareConcurrency counts LOGICAL processors (threads, SMT included); no web API gives the number of physical cores: null, and why */
  physicalCores: { value: null; reason: string };
}

export interface ClientGlobals {
  navigator?: { userAgent?: string; userAgentData?: { platform?: string } };
  screen?: { colorDepth?: number };
  innerWidth?: number;
  innerHeight?: number;
  isSecureContext?: boolean;
  location?: { hostname?: string };
}

export const PHYSICAL_CORES_REASON = 'navigator.hardwareConcurrency counts logical processors (hyper-threads included); the web platform exposes no physical core count';

/** name and version out of a user agent string (S-cbeacon1-ua: the order matters: Edge, Opera and Headless Chrome also say "Chrome", Chrome also says "Safari") */
export function parseBrowser(ua: string, platformHint?: string): ClientInfo['browser'] {
  const pick = (re: RegExp, name: string): ClientInfo['browser'] | null => {
    const m = re.exec(ua);
    if (!m) return null;
    const parts = (m[1] ?? '').split('.');
    const major = Number(parts[0]);
    return { name, version: parts.length >= 2 ? `${parts[0]}.${parts[1]}` : (parts[0] ?? null), major: Number.isFinite(major) ? major : null, os: null, headless: /HeadlessChrome/.test(ua) };
  };
  const b = pick(/Edg(?:e|A|iOS)?\/([\d.]+)/, 'Edge') ?? pick(/OPR\/([\d.]+)/, 'Opera') ?? pick(/Firefox\/([\d.]+)/, 'Firefox') ?? pick(/(?:HeadlessChrome|Chrome|CriOS)\/([\d.]+)/, 'Chrome')
    ?? pick(/Version\/([\d.]+).*Safari\//, 'Safari') ?? { name: 'unknown', version: null, major: null, os: null, headless: false };
  const os = platformHint ? platformHint.slice(0, 30)
    : /Windows NT/.test(ua) ? 'Windows' : /Android/.test(ua) ? 'Android' : /iPhone|iPad|iPod/.test(ua) ? 'iOS' : /Mac OS X|Macintosh/.test(ua) ? 'macOS' : /CrOS/.test(ua) ? 'ChromeOS' : /Linux|X11/.test(ua) ? 'Linux' : null;
  return { ...b, os };
}

export function hostKind(hostname: string | undefined): ClientInfo['page']['hostKind'] {
  if (!hostname) return null;
  const h = hostname.toLowerCase();
  if (h === 'localhost' || h === '[::1]' || h === '::1' || /^127\.\d+\.\d+\.\d+$/.test(h)) return 'localhost';
  if (h.endsWith('.ts.net')) return 'ts.net';
  if (/^\d{1,3}(\.\d{1,3}){3}$/.test(h) || h.startsWith('[')) return 'ip';
  return 'other';
}

export function collectClientInfo(g: ClientGlobals): ClientInfo {
  const ua = String(g.navigator?.userAgent ?? '');
  const w = num(g.innerWidth), h = num(g.innerHeight);
  return {
    browser: parseBrowser(ua, typeof g.navigator?.userAgentData?.platform === 'string' && g.navigator.userAgentData.platform !== '' ? g.navigator.userAgentData.platform : undefined),
    colorDepth: num(g.screen?.colorDepth),
    viewport: w !== null && h !== null ? { width: w, height: h } : null,
    page: { secure: bool(g.isSecureContext), hostKind: hostKind(g.location?.hostname) },
    physicalCores: { value: null, reason: PHYSICAL_CORES_REASON },
  };
}
