// Fetches the manifest and the assets it names: progress by bytes, decoding (Content-Encoding by the browser, or DecompressionStream), SHA-256 and size checks,
// WebAssembly.compileStreaming for the engine wasm. Every failure is a LoadError that says WHAT failed (message) and WHERE (stage + url).

import { type AssetRef, type Manifest, type PackRef, formatErrors, validateManifest } from './manifest.ts';
import { parsePack, type PackEntry } from './pack.ts';
import { Sha256, toHex } from './sha256.ts';

export type LoadStage = 'manifest' | 'fetch' | 'decode' | 'verify' | 'compile' | 'pack';

export class LoadError extends Error {
  readonly stage: LoadStage;
  readonly url: string;
  constructor(stage: LoadStage, url: string, message: string, options?: { cause?: unknown }) {
    super(message, options);
    this.name = 'LoadError';
    this.stage = stage;
    this.url = url;
  }
  /** "what" and "where" for the UI */
  describe(): { what: string; where: string } {
    return { what: this.message, where: `${this.stage} ${this.url}` };
  }
}

/** `at` is performance.now() (ms since the navigation started) when the progress was reported */
export interface Progress { label: string; url: string; loaded: number; total: number; done: boolean; at: number }
/** decision 0220: how the SHA-256 of an asset is computed. 'js-stream': incrementally in JS while the bytes arrive (a browser without WebCrypto). 'subtle': once, over all the bytes, with WebCrypto (native), before the asset is used. */
export type HashMode = 'js-stream' | 'subtle';
export type AssetOrigin = 'network' | 'storage-cache';
/** off: no store; hit: a stored copy was used; writing: the verified bytes are being written (a later onTrace for the same asset says stored / failed); stored; skipped: nothing written (another page is writing it, or a stored copy failed verification); failed */
export type StoreState = 'off' | 'hit' | 'writing' | 'stored' | 'skipped' | 'failed';

/** a persistent copy of decoded assets (src/bundle/assetstore.ts); the loader only needs this */
export interface AssetWriter {
  write(data: Uint8Array): Promise<void>;
  /** makes the written bytes the stored copy (called only after the bytes were verified) */
  commit(): Promise<void>;
  abort(): Promise<void>;
}
export interface AssetCache {
  /** the stored copy of the asset (its size already matches), or null */
  get(ref: AssetRef): Promise<{ size: number; arrayBuffer(): Promise<ArrayBuffer> } | null>;
  /** null: nothing to write (another page is writing it) */
  begin(ref: AssetRef): Promise<AssetWriter | null>;
  /** removes a stored copy that failed verification */
  discard(ref: AssetRef): Promise<void>;
}

/** what the loader learned about one asset (decision 0220): where the bytes came from and where the time went. All times in ms. */
export interface AssetTrace {
  label: string;
  url: string;
  /** decoded bytes (the manifest's size) */
  bytes: number;
  origin: AssetOrigin;
  hash: HashMode;
  store: StoreState;
  /** a fixed code (assetstore.ts StoreFail) when store is 'failed' or 'skipped' */
  fail: string | null;
  /** request -> response headers (network) or -> the stored file opened (storage) */
  headersMs: number;
  /** headers -> every byte in memory, without the hashing and copying done inside the loop. For the network this includes the browser's own decoding of Content-Encoding; for the storage it is the file read. */
  readMs: number;
  /** time spent hashing: the sum over the chunks in 'js-stream', the one digest() call in 'subtle' */
  hashMs: number;
  /** time spent copying the chunks into the one buffer */
  copyMs: number;
  /** begin() -> commit() finished (0 when nothing was written) */
  storeMs: number;
  /** from the start of the asset's load to the verified bytes (for the wasm: to the compiled module as well) */
  totalMs: number;
  /** wasm only: from the start of the asset's load to the compiled module (null for the other assets) */
  compileMs: number | null;
  /** packs only: reading the pack's index (and, on the original path, hashing every member again in JS); 0 for the other assets */
  parseMs: number;
}

export interface LoadOptions {
  baseUrl?: string;
  fetchImpl?: typeof fetch;
  onProgress?: (p: Progress) => void;
  signal?: AbortSignal;
  /** passed to fetch(); the app default is the HTTP cache's own rules */
  cache?: RequestCache;
  /** decision 0220: a persistent copy of the decoded bytes. Null: this browser has no OPFS (nothing is stored or read). */
  store?: AssetCache | null;
  /** decision 0220: called per asset with what was measured (also when a stored copy failed verification and the network was used instead); called AGAIN for the same label when the background write of a stored copy ends (store 'stored' / 'failed', storeMs) */
  onTrace?: (t: AssetTrace) => void;
}

const hasEncoding = (res: Response): boolean => {
  const e = res.headers.get('content-encoding');
  return e !== null && e.trim() !== '' && e.trim().toLowerCase() !== 'identity';
};

function resolveSameOrigin(url: string, baseUrl: string | undefined, stage: LoadStage): string {
  const base = baseUrl ?? (typeof location !== 'undefined' ? location.href : 'http://localhost/');
  const u = new URL(url, base);
  if (u.origin !== new URL(base).origin) throw new LoadError(stage, url, `refusing to fetch from another origin (${u.origin})`);
  return u.href;
}

/** the plain-text explanation a server sent with an error status (the refusal of a manifest of another kind at the default manifest path, decision 0152), or '' */
async function explanationOf(res: Response): Promise<string> {
  if (!/^text\/plain/i.test(res.headers.get('content-type') ?? '')) return '';
  try { return (await res.text()).trim().slice(0, 2000); } catch { return ''; }
}

async function fetchChecked(url: string, stage: LoadStage, opts: LoadOptions, explain = false): Promise<Response> {
  const f = opts.fetchImpl ?? fetch;
  let res: Response;
  try { res = await f(url, { signal: opts.signal ?? null, ...(opts.cache ? { cache: opts.cache } : {}) }); } catch (e) { throw new LoadError(stage, url, `request failed: ${String(e)}`, { cause: e }); }
  if (!res.ok) {
    const why = explain ? await explanationOf(res) : '';
    throw new LoadError(stage, url, `HTTP ${res.status} ${res.statusText}`.trim() + (why ? `\n${why}` : ''));
  }
  if (!res.body) throw new LoadError(stage, url, 'the response has no body');
  return res;
}

export async function fetchManifest(url: string, opts: LoadOptions = {}): Promise<Manifest> {
  const abs = resolveSameOrigin(url, opts.baseUrl, 'manifest');
  const res = await fetchChecked(abs, 'manifest', opts, true);
  let json: unknown;
  try { json = JSON.parse(await res.text()); } catch (e) { throw new LoadError('manifest', abs, `not valid JSON: ${String(e)}`, { cause: e }); }
  const r = validateManifest(json);
  if (!r.ok) throw new LoadError('manifest', abs, `manifest does not match schema 1:\n${formatErrors(r.errors)}`);
  return r.manifest;
}

export interface VerifiedStream {
  stream: ReadableStream<Uint8Array>;
  /** resolves when the stream has been read to its end and the size and SHA-256 matched; rejects with the LoadError otherwise */
  verified: Promise<void>;
}

// Wraps the decoded body: counts bytes (progress), hashes them, and ERRORS the stream at its end if the size or SHA-256 differs from the manifest.
export function verifyingStream(body: ReadableStream<Uint8Array>, ref: AssetRef, url: string, label: string, onProgress?: (p: Progress) => void): VerifiedStream {
  const reader = body.getReader();
  const sha = new Sha256();
  let loaded = 0;
  let settle!: { resolve: () => void; reject: (e: unknown) => void };
  const verified = new Promise<void>((resolve, reject) => { settle = { resolve, reject }; });
  verified.catch(() => { /* observed by the caller; keep Node from reporting an unhandled rejection when only the stream error is awaited */ });
  const fail = (controller: ReadableStreamDefaultController<Uint8Array>, stage: LoadStage, message: string, cause?: unknown): void => {
    const err = new LoadError(stage, url, message, cause === undefined ? undefined : { cause });
    settle.reject(err);
    controller.error(err);
    void reader.cancel(err).catch(() => {});
  };
  const stream = new ReadableStream<Uint8Array>({
    async pull(controller) {
      let r: ReadableStreamReadResult<Uint8Array>;
      try { r = await reader.read(); } catch (e) { fail(controller, 'fetch', `the transfer broke after ${loaded} bytes: ${String(e)}`, e); return; }
      if (r.done) {
        if (loaded !== ref.size) { fail(controller, 'verify', `size mismatch: manifest says ${ref.size} bytes, received ${loaded}`); return; }
        const got = toHex(sha.digest());
        if (got !== ref.sha256) { fail(controller, 'verify', `SHA-256 mismatch: manifest ${ref.sha256}, received ${got}`); return; }
        onProgress?.({ label, url, loaded, total: ref.size, done: true, at: performance.now() });
        settle.resolve();
        controller.close();
        return;
      }
      loaded += r.value.length;
      if (loaded > ref.size) { fail(controller, 'verify', `size mismatch: more than the ${ref.size} bytes the manifest says (at least ${loaded})`); return; }
      sha.update(r.value);
      onProgress?.({ label, url, loaded, total: ref.size, done: false, at: performance.now() });
      controller.enqueue(r.value);
    },
    cancel(reason) { settle.reject(new LoadError('fetch', url, `cancelled: ${String(reason)}`)); return reader.cancel(reason); },
  });
  return { stream, verified };
}

// the decoded byte stream of an asset: Content-Encoding is left to the browser; a stored gzip / br object without Content-Encoding goes through DecompressionStream
function decodedBody(res: Response, ref: AssetRef, url: string): ReadableStream<Uint8Array> {
  const body = res.body as ReadableStream<Uint8Array>;
  if (hasEncoding(res) || ref.compression === 'identity') return body;
  const fmt = ref.compression === 'gzip' ? 'gzip' : 'brotli';
  try { return body.pipeThrough(new DecompressionStream(fmt as CompressionFormat) as unknown as ReadableWritablePair<Uint8Array, Uint8Array>); } catch (e) {
    throw new LoadError('decode', url, `this browser cannot decode ${ref.compression} with DecompressionStream (and the server sent no Content-Encoding): ${String(e)}`, { cause: e });
  }
}

export async function fetchAsset(ref: AssetRef, label: string, opts: LoadOptions = {}): Promise<Uint8Array> {
  return (await acquire(ref, label, opts, false)).bytes as Uint8Array;
}

export async function loadWasm(ref: AssetRef, label: string, opts: LoadOptions = {}): Promise<WebAssembly.Module> {
  // all the decoded bytes are kept in one buffer (for the WebCrypto digest and the stored copy): if the browser cannot allocate it, the wasm is streamed and nothing is kept
  try { return (await acquire(ref, label, opts, true)).module as WebAssembly.Module; } catch (e) { if (!(e instanceof BufferUnavailable)) throw e; }
  const url = resolveSameOrigin(ref.url, opts.baseUrl, 'fetch');
  const res = await fetchChecked(url, 'fetch', opts);
  const vs = verifyingStream(decodedBody(res, ref, url), ref, url, label, opts.onProgress);
  const wasmResponse = new Response(vs.stream, { headers: { 'Content-Type': 'application/wasm' } });
  let mod: WebAssembly.Module;
  try { mod = await WebAssembly.compileStreaming(wasmResponse); } catch (e) {
    // the verifying stream errors itself on a size / SHA-256 mismatch: report that, not the CompileError that follows from it
    await vs.verified;
    if (e instanceof LoadError) throw e;
    throw new LoadError('compile', url, explainCompileError(e), { cause: e });
  }
  await vs.verified;
  return mod;
}

/** what a failed WebAssembly.compileStreaming means: running out of memory is told apart (the engine module is 300-670 MB and its compiled code is held in the browser as well) */
export function explainCompileError(e: unknown): string {
  const m = String((e as Error)?.message ?? e);
  if (/out of memory|could not allocate|allocation failed|Cannot allocate/i.test(m)) return `the browser ran out of memory while compiling the engine wasm (close other tabs, or use a smaller build such as the Oz variant): ${m}`;
  return `WebAssembly.compileStreaming failed: ${String(e)}`;
}

export interface LoadedPack {
  files: Map<string, PackEntry>;
  bytes: Uint8Array;
}

export async function loadPack(ref: PackRef, label: string, opts: LoadOptions = {}): Promise<LoadedPack> {
  let bytes: Uint8Array;
  let trace: AssetTrace | undefined;
  const a = await acquire(ref, label, opts, false);
  bytes = a.bytes as Uint8Array;
  trace = a.trace;
  try {
    const t = now();
    // on the new path the SHA-256 of the whole pack (WebCrypto or JS, against the manifest) has been checked above, and it covers the index and every member: hashing each member again in JS (about the same work again) adds nothing
    const files = parsePack(bytes, { verifyFiles: false, format: ref.format });
    if (trace) { trace.parseMs = now() - t; opts.onTrace?.({ ...trace }); }
    return { files, bytes };
  } catch (e) {
    throw new LoadError('pack', ref.url, `${label} is not a valid ${ref.format} pack: ${String((e as Error).message ?? e)}`, { cause: e });
  }
}

// ---- decision 0220: the stored-copy and WebCrypto path --------------------------------------------------------------------------------------------------------------------------------------------
// The only path. A browser without OPFS has no store (LoadOptions.store null); a browser without WebCrypto hashes in JS while the bytes arrive ('js-stream').
//   1. a stored copy (OPFS) named by the manifest's SHA-256 is read whole, hashed with WebCrypto, and used if it matches (the wasm is compiled while it is hashed); a copy that does not match is discarded
//   2. otherwise the network: one read loop hands each decoded chunk to (a) the WebAssembly.compileStreaming stream, (b) the one buffer, (c) the JS hash ('js-stream') and (d) the storage writer, and the asset
//      is released to the caller only after the size and the SHA-256 matched. The stored copy is committed after that (a file that was not committed does not exist).
const now = (): number => performance.now();
const hasSubtle = (): boolean => typeof crypto !== 'undefined' && !!crypto.subtle && typeof crypto.subtle.digest === 'function';
/** how this browser hashes the assets: WebCrypto once over all the bytes, else in JS while they arrive */
export const hashModeOfThisBrowser = (): HashMode => (hasSubtle() ? 'subtle' : 'js-stream');

async function digestHex(bytes: Uint8Array, mode: HashMode): Promise<{ hex: string; ms: number }> {
  const t = now();
  if (mode === 'subtle' && hasSubtle()) {
    const d = await crypto.subtle.digest('SHA-256', bytes as unknown as ArrayBuffer);
    return { hex: toHex(new Uint8Array(d)), ms: now() - t };
  }
  return { hex: toHex(new Sha256().update(bytes).digest()), ms: now() - t };
}

const STORE_BLOCK = 4 * 1024 * 1024;

/** the one-buffer path could not allocate its buffer (wasm only: the original streaming path is used instead) */
class BufferUnavailable extends Error {}

interface Acquired { bytes?: Uint8Array; module?: WebAssembly.Module; trace?: AssetTrace }

async function acquire(ref: AssetRef, label: string, opts: LoadOptions, wasm: boolean): Promise<Acquired> {
  const url = resolveSameOrigin(ref.url, opts.baseUrl, 'fetch');
  const mode: HashMode = hashModeOfThisBrowser();
  const t0 = now();
  const tr: AssetTrace = { label, url, bytes: ref.size, origin: 'network', hash: mode === 'subtle' && hasSubtle() ? 'subtle' : 'js-stream', store: opts.store ? 'skipped' : 'off', fail: null, headersMs: 0, readMs: 0, hashMs: 0, copyMs: 0, storeMs: 0, totalMs: 0, compileMs: null, parseMs: 0 };
  const emit = (): void => { tr.totalMs = now() - t0; opts.onTrace?.({ ...tr }); };
  if (opts.store) {
    let stored: Awaited<ReturnType<AssetCache['get']>> = null;
    try { stored = await opts.store.get(ref); } catch (e) { tr.store = 'failed'; tr.fail = failCode(e); }
    if (stored) {
      const r = await fromStore(stored, ref, label, url, wasm, opts, tr, t0);
      if (r) { tr.origin = 'storage-cache'; tr.store = 'hit'; emit(); return { ...r, trace: tr }; }
      tr.origin = 'network';
      tr.store = 'skipped';
      tr.fail = 'verify';
      tr.headersMs = tr.readMs = tr.hashMs = tr.copyMs = 0;
      await opts.store.discard(ref).catch(() => {});
    }
  }
  try { const r = await fromNetwork(ref, label, url, wasm, opts, tr, t0); emit(); return { ...r, trace: tr }; } catch (e) { emit(); throw e; }
}

const failCode = (e: unknown): string => {
  const n = (e as { name?: string } | null)?.name ?? '';
  if (n === 'QuotaExceededError') return 'quota';
  if (n === 'SecurityError' || n === 'NotAllowedError') return 'security';
  if (n === 'NoModificationAllowedError' || n === 'InvalidStateError') return 'locked';
  if (n === 'TypeError' || n === 'NotSupportedError') return 'unsupported';
  if (n === 'NotFoundError' || n === 'NotReadableError' || n === 'AbortError' || n === 'UnknownError' || n === 'EncodingError') return 'io';
  return 'other';
};

async function fromStore(file: { size: number; arrayBuffer(): Promise<ArrayBuffer> }, ref: AssetRef, label: string, url: string, wasm: boolean, opts: LoadOptions, tr: AssetTrace, t0: number): Promise<Acquired | null> {
  opts.onProgress?.({ label, url, loaded: 0, total: ref.size, done: false, at: now() });
  const t1 = now();
  tr.headersMs = t1 - t0;
  let buf: Uint8Array;
  try { buf = new Uint8Array(await file.arrayBuffer()); } catch { return null; }
  tr.readMs = now() - t1;
  if (buf.length !== ref.size) return null;
  const hashing = digestHex(buf, 'subtle');
  let module: WebAssembly.Module | undefined;
  let compileError: unknown = null;
  const compiling = wasm ? WebAssembly.compile(buf as unknown as BufferSource).then((m) => { module = m; tr.compileMs = now() - t0; }, (e: unknown) => { compileError = e; }) : Promise.resolve();
  const [h] = await Promise.all([hashing, compiling]);
  tr.hash = hasSubtle() ? 'subtle' : 'js-stream';
  tr.hashMs = h.ms;
  if (h.hex !== ref.sha256) return null;
  if (compileError !== null) throw new LoadError('compile', url, explainCompileError(compileError), { cause: compileError });
  opts.onProgress?.({ label, url, loaded: ref.size, total: ref.size, done: true, at: now() });
  return wasm ? { module: module as WebAssembly.Module } : { bytes: buf };
}

async function fromNetwork(ref: AssetRef, label: string, url: string, wasm: boolean, opts: LoadOptions, tr: AssetTrace, t0: number): Promise<Acquired> {
  const res = await fetchChecked(url, 'fetch', opts);
  const tHeaders = now();
  tr.headersMs = tHeaders - t0;
  const reader = decodedBody(res, ref, url).getReader();
  let buf: Uint8Array;
  try { buf = new Uint8Array(ref.size); } catch (e) { void res.body?.cancel().catch(() => {}); if (wasm) throw new BufferUnavailable(String(e)); throw new LoadError('fetch', url, `could not allocate ${ref.size} bytes for the asset: ${String(e)}`, { cause: e }); }
  const sha = tr.hash === 'js-stream' ? new Sha256() : null;
  let loaded = 0;
  let compileCtl!: ReadableStreamDefaultController<Uint8Array>;
  const compileStream = wasm ? new ReadableStream<Uint8Array>({ start(c) { compileCtl = c; } }) : null;
  let compiled: Promise<WebAssembly.Module> | null = null;
  if (compileStream) { compiled = WebAssembly.compileStreaming(new Response(compileStream, { headers: { 'Content-Type': 'application/wasm' } })); compiled.catch(() => { /* observed below */ }); }

  // storage writer: blocks of STORE_BLOCK bytes of `buf`, written in order without making the read loop wait
  let writer: AssetWriter | null = null;
  let writeChain: Promise<void> = Promise.resolve();
  let writeFailed: unknown = null;
  let storeStart = 0, flushed = 0;
  if (opts.store) {
    try { writer = await opts.store.begin(ref); if (writer) { tr.store = 'writing'; storeStart = now(); } else { tr.store = 'skipped'; tr.fail = 'busy'; } } catch (e) { tr.store = 'failed'; tr.fail = failCode(e); }
  }
  const flush = (upTo: number): void => {
    if (!writer || writeFailed !== null || upTo <= flushed) return;
    // a COPY, not a view: the caller may transfer the asset's buffer to a Worker (postMessage with a transfer list) before the queued write runs, and a detached view writes nothing
    const part = buf.slice(flushed, upTo);
    flushed = upTo;
    const w = writer;
    writeChain = writeChain.then(() => (writeFailed === null ? w.write(part) : undefined)).catch((e: unknown) => { writeFailed = e; });
  };
  const dropWriter = async (): Promise<void> => {
    const w = writer; writer = null;
    if (!w) return;
    try { await writeChain; } catch { /* reported through writeFailed */ }
    await w.abort().catch(() => {});
  };
  const fail = async (stage: LoadStage, message: string, cause?: unknown): Promise<never> => {
    const err = new LoadError(stage, url, message, cause === undefined ? undefined : { cause });
    compileCtl?.error(err);
    void reader.cancel(err).catch(() => {});
    await dropWriter();
    throw err;
  };

  let at = 0;
  for (;;) {
    let r: ReadableStreamReadResult<Uint8Array>;
    try { r = await reader.read(); } catch (e) { return fail('fetch', `the transfer broke after ${loaded} bytes: ${String(e)}`, e); }
    if (r.done) break;
    const c = r.value;
    loaded += c.length;
    if (loaded > ref.size) return fail('verify', `size mismatch: more than the ${ref.size} bytes the manifest says (at least ${loaded})`);
    const tc = now();
    buf.set(c, at);
    at += c.length;
    tr.copyMs += now() - tc;
    if (sha) { const th = now(); sha.update(c); tr.hashMs += now() - th; }
    compileCtl?.enqueue(c);
    if (writer && at - flushed >= STORE_BLOCK) flush(at);
    opts.onProgress?.({ label, url, loaded, total: ref.size, done: false, at: now() });
  }
  // the read loop's wall time without the JS hashing and the copying done inside it, so that read + hash + copy do not count a millisecond twice
  tr.readMs = Math.max(0, now() - tHeaders - tr.hashMs - tr.copyMs);
  if (loaded !== ref.size) return fail('verify', `size mismatch: manifest says ${ref.size} bytes, received ${loaded}`);
  compileCtl?.close();
  let hex: string;
  if (sha) { const th = now(); hex = toHex(sha.digest()); tr.hashMs += now() - th; } else { const h = await digestHex(buf, 'subtle'); hex = h.hex; tr.hashMs = h.ms; }
  if (hex !== ref.sha256) return fail('verify', `SHA-256 mismatch: manifest ${ref.sha256}, received ${hex}`);
  let module: WebAssembly.Module | undefined;
  if (compiled) {
    try { module = await compiled; tr.compileMs = now() - t0; } catch (e) { await dropWriter(); throw new LoadError('compile', url, explainCompileError(e), { cause: e }); }
  }
  opts.onProgress?.({ label, url, loaded, total: ref.size, done: true, at: now() });
  if (writer) {
    // verified: the rest of the bytes are written and the file is committed in the background (the caller does not wait for the disk)
    flush(at);
    const w = writer;
    const finish = (async (): Promise<void> => {
      try {
        await writeChain;
        if (writeFailed !== null) throw writeFailed;
        await w.commit();
        tr.store = 'stored';
      } catch (e) {
        tr.store = 'failed'; tr.fail = failCode(e);
        await w.abort().catch(() => {});
      }
      tr.storeMs = now() - storeStart;
      opts.onTrace?.({ ...tr });
    })();
    void finish;
  }
  return wasm ? { module: module as WebAssembly.Module } : { bytes: buf };
}
