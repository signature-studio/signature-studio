// The smallest report of a failure (decision 0257, beacon2). The measurement beacon (beacon.ts) is made of twenty collectors and is sent at the first idle, every 30 s after it, and when the engine ends or the page fails. What it cannot say:
//   - the engine that never reaches its first idle and never fails (it stays "starting" for ever): nothing is sent, because the 30 s timer is armed by the first idle;
//   - the tab that is closed before the first idle (no pagehide trigger);
//   - an uncaught error / rejection of the page that does not go through showError();
//   - which capabilities the browser had when it failed (crossOriginIsolated, SharedArrayBuffer, WebGL2, OPFS, JSPI ...): the text of the error names a missing feature, the booleans do not exist.
// A failure report is one small POST /__perf (a few KB, built from plain facts, so that it does not depend on the collectors that the big beacon needs), at most once per kind per page load. It carries: the kind, the stage, the first 200 characters of the message
// (through the same cleaning as the engine's last words: no path, file name, e-mail address, URL or IP address), the time since the navigation started, the start-up timeline, the capability booleans, and the browser facts that every beacon has already (browser-info.ts).
// No cookie, no URL, no IP address, no field input, no file content. The answer is ignored: a failure of the report is silent (the page does not depend on it). There is no option: it is always on.
import type { BrowserInfo, ClientInfo } from './browser-info.ts';
import { postBeacon, type SendResult } from './beacon.ts';
import { sanitizeEngineText } from './exit-message.ts';
import { detectFeatures, type FeatureEnv } from '../engine/features.ts';

export type FailureKind =
  | 'feature-missing'        // the feature check named something the browser lacks (JSPI, exception handling, Worker, OffscreenCanvas ...)
  | 'no-shared-memory'       // no SharedArrayBuffer (the page is not cross-origin isolated)
  | 'load-failed'            // the manifest or an asset could not be fetched / compiled
  | 'boot-exception'         // another exception while the page was starting
  | 'engine-error'           // the engine Worker reported an error, or raised one
  | 'engine-trap'            // the engine trapped
  | 'page-error'             // an uncaught error of the page
  | 'unhandled-rejection'    // an unhandled promise rejection of the page
  | 'no-first-idle'          // FAILURE_STALL_MS after the page started there is no first idle, no error and no end
  | 'left-before-first-idle'; // the page was left (pagehide) before the first idle, with no error and no end

/** how long the start may take before it is reported as stuck (the target of P1 is 10 s; a slow network can take longer, and the report says which step was the last: it is a report, not a verdict) */
export const FAILURE_STALL_MS = 60_000;
export const FAILURE_MESSAGE_MAX = 200;
export const FAILURE_WHERE_MAX = 120;

export interface Capabilities {
  crossOriginIsolated: boolean | null;
  secureContext: boolean | null;
  sharedArrayBuffer: boolean;
  /** a WebGL2 context on a canvas of the page; null: no document, or the probe threw */
  webgl2: boolean | null;
  /** a WebGL2 context on an OffscreenCanvas (what the engine Worker uses); null: no OffscreenCanvas, or the probe threw */
  offscreenWebgl2: boolean | null;
  /** navigator.storage.getDirectory exists (the stored copy of the assets lives in OPFS; Firefox has a different path) */
  opfs: boolean;
  jspi: boolean;
  wasmStreaming: boolean;
  wasmExceptions: boolean;
  worker: boolean;
  offscreenCanvas: boolean;
}

type GlContextLike = { getExtension?(n: string): unknown } | null | undefined;
export interface CapabilityEnv extends FeatureEnv {
  crossOriginIsolated?: boolean;
  isSecureContext?: boolean;
  SharedArrayBuffer?: unknown;
  navigator?: { storage?: { getDirectory?: unknown } };
  document?: { createElement(tag: 'canvas'): { getContext(id: string): unknown } };
  OffscreenCanvas?: unknown;
}

const flag = (v: unknown): boolean | null => (typeof v === 'boolean' ? v : null);

function releaseGl(gl: GlContextLike): void {
  try { (gl?.getExtension?.('WEBGL_lose_context') as { loseContext?: () => void } | null | undefined)?.loseContext?.(); } catch { /* the probe is over */ }
}

/** the booleans of the capabilities the engine's start depends on; never throws */
export function detectCapabilities(env: CapabilityEnv = globalThis as unknown as CapabilityEnv): Capabilities {
  let f: ReturnType<typeof detectFeatures> | null = null;
  try { f = detectFeatures(env); } catch { /* every feature below is then false */ }
  const ok = (prefix: string): boolean => f?.features.find((x) => x.name.startsWith(prefix))?.ok === true;
  let webgl2: boolean | null = null;
  try {
    if (env.document) {
      const gl = env.document.createElement('canvas').getContext('webgl2') as GlContextLike;
      webgl2 = !!gl;
      releaseGl(gl);
    }
  } catch { webgl2 = null; }
  let offscreenWebgl2: boolean | null = null;
  try {
    if (typeof env.OffscreenCanvas === 'function') {
      const gl = new (env.OffscreenCanvas as new (w: number, h: number) => { getContext(id: string): unknown })(1, 1).getContext('webgl2') as GlContextLike;
      offscreenWebgl2 = !!gl;
      releaseGl(gl);
    }
  } catch { offscreenWebgl2 = null; }
  return {
    crossOriginIsolated: flag(env.crossOriginIsolated),
    secureContext: flag(env.isSecureContext),
    sharedArrayBuffer: typeof env.SharedArrayBuffer !== 'undefined',
    webgl2,
    offscreenWebgl2,
    opfs: typeof env.navigator?.storage?.getDirectory === 'function',
    jspi: ok('JSPI'),
    wasmStreaming: ok('WebAssembly.compileStreaming'),
    wasmExceptions: ok('WebAssembly exception handling'),
    worker: ok('module Worker'),
    offscreenCanvas: ok('OffscreenCanvas'),
  };
}

/** what the page knows when it reports (the caller reads it at the moment of the report) */
export interface FailureFacts {
  seq: number;
  sid: string;
  pageMs: number;
  visibility: string | null;
  browser: BrowserInfo;
  client: ClientInfo | null;
  bundle: { name: string | null; version: string | null; engineKind: string | null };
  timeline: Record<string, number>;
  state: string;
}
export interface FailureDetail { stage?: string | null; where?: string | null; message?: string | null }

export interface FailureBeacon {
  v: 3;
  seq: number;
  sid: string;
  reason: 'failure';
  pageMs: number;
  visibility: string | null;
  failure: { kind: FailureKind; stage: string | null; where: string; message: string; elapsedMs: number; state: string; lastStep: { name: string; ms: number } | null; capabilities: Capabilities };
  browser: BrowserInfo;
  client: ClientInfo | null;
  bundle: { name: string | null; version: string | null; engineKind: string | null };
  timeline: Record<string, number>;
}

/** the last step of the start-up timeline that was reached (the largest stamp), or null */
export function lastStep(timeline: Record<string, number>): { name: string; ms: number } | null {
  let best: { name: string; ms: number } | null = null;
  for (const [name, ms] of Object.entries(timeline)) if (typeof ms === 'number' && Number.isFinite(ms) && (best === null || ms >= best.ms)) best = { name, ms: Math.round(ms) };
  return best;
}

export function buildFailureBeacon(kind: FailureKind, detail: FailureDetail, facts: FailureFacts, capabilities: Capabilities): FailureBeacon {
  const timeline: Record<string, number> = {};
  for (const [k, v] of Object.entries(facts.timeline)) if (typeof v === 'number' && Number.isFinite(v)) timeline[k.slice(0, 40)] = Math.round(v);
  return {
    v: 3, seq: facts.seq, sid: facts.sid, reason: 'failure', pageMs: facts.pageMs, visibility: facts.visibility,
    failure: {
      kind,
      stage: detail.stage ? sanitizeEngineText(detail.stage, 40) : null,
      where: sanitizeEngineText(detail.where ?? '', FAILURE_WHERE_MAX),
      message: sanitizeEngineText(detail.message ?? '', FAILURE_MESSAGE_MAX),
      elapsedMs: Math.round(facts.pageMs),
      state: sanitizeEngineText(facts.state, 40),
      lastStep: lastStep(timeline),
      capabilities,
    },
    browser: facts.browser, client: facts.client, bundle: facts.bundle, timeline,
  };
}

export interface FailureDeps {
  facts(): FailureFacts;
  capabilities(): Capabilities;
  post(text: string): Promise<SendResult>;
}

/** sends the failure reports: at most one per kind per page load; never throws; the answer is ignored */
export class FailureReporter {
  private readonly sent = new Set<FailureKind>();
  private readonly deps: FailureDeps;
  constructor(deps: FailureDeps) { this.deps = deps; }
  /** true: a report was sent (the POST was started and finished without an exception); false: it was not (already sent for this kind, or something failed, silently) */
  async report(kind: FailureKind, detail: FailureDetail = {}): Promise<boolean> {
    if (this.sent.has(kind)) return false;
    this.sent.add(kind);   // before anything can fail: a report that throws is not tried again
    try {
      const text = JSON.stringify(buildFailureBeacon(kind, detail, this.deps.facts(), this.deps.capabilities()));
      await this.deps.post(text);
      return true;
    } catch {
      return false;
    }
  }
  has(kind: FailureKind): boolean { return this.sent.has(kind); }
}

export const postFailure = (text: string): Promise<SendResult> => postBeacon(text);

/** the page's own facts that decide whether a stall / a leaving is a failure */
export interface FailureWatchState {
  reachedIdle(): boolean;
  failed(): boolean;
  ended(): boolean;
}
export interface FailureHost {
  setTimer(fn: () => void, ms: number): unknown;
  clearTimer(handle: unknown): void;
  listen(type: 'pagehide' | 'error' | 'unhandledrejection', fn: (ev: unknown) => void): void;
}

const text = (v: unknown): string => {
  if (typeof v === 'string') return v;
  const m = (v as { message?: unknown } | null | undefined)?.message;
  return typeof m === 'string' ? m : v === undefined || v === null ? '' : String(v);
};

/**
 * Arms the three triggers the big beacon does not have: the stall timer (no first idle, no error, no end after `stallMs`), the pagehide before the first idle, and the page's uncaught errors and rejections.
 * Returns a function that disarms the timer (the listeners stay: they are the page's life long).
 */
export function installFailureWatch(host: FailureHost, reporter: FailureReporter, state: FailureWatchState, stallMs = FAILURE_STALL_MS): () => void {
  const timer = host.setTimer(() => {
    if (state.reachedIdle() || state.failed() || state.ended()) return;
    void reporter.report('no-first-idle', { message: `no first idle ${Math.round(stallMs / 1000)} s after the page started` });
  }, stallMs);
  host.listen('pagehide', () => {
    if (state.reachedIdle() || state.failed() || state.ended()) return;
    void reporter.report('left-before-first-idle', { message: 'the page was left before the first idle' });
  });
  host.listen('error', (ev) => {
    const m = text((ev as { message?: unknown } | null)?.message);
    if (m !== '') void reporter.report('page-error', { message: m });
  });
  host.listen('unhandledrejection', (ev) => {
    void reporter.report('unhandled-rejection', { message: text((ev as { reason?: unknown } | null)?.reason) });
  });
  return () => host.clearTimer(timer);
}
