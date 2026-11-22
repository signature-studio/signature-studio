// Issue #1314 (PROVISIONAL): a file of the bundle that the page downloads AFTER the engine has started (manifest.lazy: the plugins' data and the assets that the start-up does not need).
// The engine's host files have the file from the start, with its name and size: an open and a stat never wait. A read of bytes that have not arrived yet waits for them: the engine thread
// is blocked in Atomics.wait on one state word (the engine Worker runs the wasm synchronously, so a wait is the only way to answer the read in the same call, as chunk-io.ts does).
// The page (the main thread, not the engine Worker and not the audio Worker) makes the download, verifies it like the start-up assets (size and SHA-256, loader.ts), copies the bytes in
// and only then sets the state word and notifies. A download that fails sets FAILED: the read then throws, and hostfiles.ts answers the call with EIO, which the engine reports by name.
// A read waits at most `timeoutMs`; after that it fails the same way. Nothing here starts a download: the downloads start when the engine starts (main.ts), not when a file is read.

import type { ByteSource } from './hostfiles.ts';

export const LAZY_WAITING = 0, LAZY_READY = 1, LAZY_FAILED = 2;

/** what the page makes for one lazy file and hands to the engine Worker (the buffers are shared, not transferred) */
export interface LazyFileShared { name: string; path: string; size: number; state: SharedArrayBuffer; data: SharedArrayBuffer }

export function makeLazyFileShared(name: string, path: string, size: number): LazyFileShared {
  return { name, path, size, state: new SharedArrayBuffer(4), data: new SharedArrayBuffer(size) };
}

export class LazyFileError extends Error {
  constructor(message: string) { super(message); this.name = 'LazyFileError'; }
}

/** the page's end: the bytes of one lazy file, once they are verified */
export class LazyFileFeed {
  private readonly state: Int32Array;
  private readonly data: Uint8Array;
  constructor(shared: LazyFileShared) {
    this.state = new Int32Array(shared.state);
    this.data = new Uint8Array(shared.data);
  }
  /** the bytes first, then the state: a reader that sees READY sees the bytes */
  deliver(bytes: Uint8Array): void {
    if (bytes.length !== this.data.length) throw new Error(`${bytes.length} bytes for a lazy file of ${this.data.length}`);
    this.data.set(bytes);
    Atomics.store(this.state, 0, LAZY_READY);
    Atomics.notify(this.state, 0);
  }
  fail(): void {
    Atomics.store(this.state, 0, LAZY_FAILED);
    Atomics.notify(this.state, 0);
  }
}

export interface LazyFileSourceOptions {
  /** how long one read may wait for the bytes before it fails (the download is slow, or it has stopped) */
  timeoutMs?: number;
  /** called once per read that had to wait: the path and how long it waited (ms) */
  onWait?: (path: string, ms: number) => void;
}

/** the engine's end: a file whose size is known and whose bytes may arrive later (hostfiles.ts addPersistent keeps it open for the whole run) */
export class LazyFileSource implements ByteSource {
  readonly name: string;
  readonly path: string;
  private readonly length: number;
  private readonly state: Int32Array;
  private readonly data: Uint8Array;
  private readonly timeoutMs: number;
  private readonly onWait: LazyFileSourceOptions['onWait'];

  constructor(shared: LazyFileShared, opts: LazyFileSourceOptions = {}) {
    this.name = shared.name;
    this.path = shared.path;
    this.length = shared.size;
    this.state = new Int32Array(shared.state);
    this.data = new Uint8Array(shared.data);
    this.timeoutMs = opts.timeoutMs ?? 120000;
    this.onWait = opts.onWait;
  }

  get size(): number { return this.length; }

  readInto(dst: Uint8Array, at: number): number {
    const t0 = performance.now();
    let waited = false;
    while (Atomics.load(this.state, 0) === LAZY_WAITING) {
      waited = true;
      const left = this.timeoutMs - (performance.now() - t0);
      if (left <= 0) throw new LazyFileError(`${this.path}: its bytes did not arrive within ${this.timeoutMs} ms`);
      Atomics.wait(this.state, 0, LAZY_WAITING, Math.min(left, 1000));
    }
    if (waited) this.onWait?.(this.path, performance.now() - t0);
    if (Atomics.load(this.state, 0) !== LAZY_READY) throw new LazyFileError(`${this.path}: the download failed`);
    if (at >= this.length || dst.length === 0) return 0;
    const n = Math.min(dst.length, this.length - at);
    dst.set(this.data.subarray(at, at + n));
    return n;
  }
}
