// Decision 0314 (PROVISIONAL): how the engine thread gets a chunk of the lazy install tree while a guest ReadFile is waiting for it.
//
// The engine is one Worker that answers libc calls SYNCHRONOUSLY (hostfiles.ts: ByteSource.readInto), and a fetch is asynchronous. So the fetch is done by a second Worker (the I/O Worker) and the engine thread
// waits for it with Atomics.wait: no JSPI, no change of the engine wasm. The two threads share one small control block and one data buffer (SharedArrayBuffers); there is ONE request in flight at a time
// (the engine thread is blocked while it waits, and it is the only one that asks), so a single slot is the whole protocol:
//
//   ctrl[STATE]  IDLE -> REQUEST (engine writes stream and chunk, notifies) -> REPLY or ERROR (I/O Worker writes length or code, notifies) -> IDLE (engine has copied the bytes out)
//   ctrl[STREAM] ctrl[CHUNK]  the request            ctrl[LENGTH]  bytes in `data` (REPLY)            ctrl[CODE]  an error number (ERROR)
//   CLOSE        the engine side sets it when the engine goes away; the I/O Worker's loop returns.
//
// The I/O Worker's loop blocks in Atomics.wait while it is idle (nothing else runs in that Worker) and is free for the promises of the fetch while it works. Atomics.waitAsync is not used: Firefox did not have it.

import type { ChunkStore } from './chunk-store.ts';
import type { VfsStream } from './vfs-index.ts';

export const STATE = 0, STREAM = 1, CHUNK = 2, LENGTH = 3, CODE = 4;
export const IDLE = 0, REQUEST = 1, REPLY = 2, ERROR = 3, CLOSE = 4;
export const CTRL_WORDS = 8;

export const ERR_STORE = 1, ERR_LENGTH = 2, ERR_REQUEST = 3;

export class ChunkIoError extends Error {
  readonly code: number;
  constructor(code: number, what: string) { super(`The lazy install tree could not fetch a chunk: ${what}.`); this.name = 'ChunkIoError'; this.code = code; }
}

export interface ChunkIoShared { ctrl: SharedArrayBuffer; data: SharedArrayBuffer }

export function makeChunkIoShared(maxChunk: number): ChunkIoShared {
  return { ctrl: new SharedArrayBuffer(CTRL_WORDS * 4), data: new SharedArrayBuffer(maxChunk) };
}

/** what the engine thread holds: ask for a chunk and get its decoded bytes (a copy: the caller may keep it) */
export interface ChunkClient {
  fetchChunk(stream: number, chunk: number): Uint8Array;
}

export interface ChunkClientOptions {
  /** how long one chunk may take before the engine gives up (the I/O Worker is gone or the network is) */
  timeoutMs?: number;
  /** counts, for the performance panel */
  onFetch?: (stream: number, chunk: number, bytes: number, ms: number) => void;
}

export class SabChunkClient implements ChunkClient {
  private readonly ctrl: Int32Array;
  private readonly data: Uint8Array;
  private readonly timeoutMs: number;
  private readonly onFetch: ChunkClientOptions['onFetch'];
  fetches = 0;
  waitedMs = 0;

  constructor(shared: ChunkIoShared, opts: ChunkClientOptions = {}) {
    this.ctrl = new Int32Array(shared.ctrl);
    this.data = new Uint8Array(shared.data);
    this.timeoutMs = opts.timeoutMs ?? 60000;
    this.onFetch = opts.onFetch;
  }

  fetchChunk(stream: number, chunk: number): Uint8Array {
    const c = this.ctrl;
    const t0 = performance.now();
    if (Atomics.load(c, STATE) !== IDLE) throw new ChunkIoError(ERR_REQUEST, 'the I/O channel is not idle (a request is already in flight or the channel is closed)');
    Atomics.store(c, STREAM, stream);
    Atomics.store(c, CHUNK, chunk);
    Atomics.store(c, STATE, REQUEST);
    Atomics.notify(c, STATE);
    const deadline = t0 + this.timeoutMs;
    for (;;) {
      const s = Atomics.load(c, STATE);
      if (s !== REQUEST) break;
      const left = deadline - performance.now();
      if (left <= 0) {
        Atomics.compareExchange(c, STATE, REQUEST, CLOSE);   // the channel is dead from now on: a late reply of the I/O Worker is not taken for the next request's
        throw new ChunkIoError(ERR_STORE, `no reply for stream ${stream} chunk ${chunk} in ${this.timeoutMs} ms`);
      }
      Atomics.wait(c, STATE, REQUEST, Math.min(left, 1000));
    }
    const state = Atomics.load(c, STATE);
    if (state === ERROR) {
      const code = Atomics.load(c, CODE);
      Atomics.store(c, STATE, IDLE);
      throw new ChunkIoError(code, `the store answered an error (code ${code}) for stream ${stream} chunk ${chunk}`);
    }
    if (state !== REPLY) throw new ChunkIoError(ERR_REQUEST, `the I/O channel was closed (state ${state})`);
    const n = Atomics.load(c, LENGTH);
    const out = new Uint8Array(n);
    out.set(this.data.subarray(0, n));
    Atomics.store(c, STATE, IDLE);
    const ms = performance.now() - t0;
    this.fetches++;
    this.waitedMs += ms;
    this.onFetch?.(stream, chunk, n, ms);
    return out;
  }

  /** the engine goes away: the I/O Worker's loop returns */
  close(): void {
    Atomics.store(this.ctrl, STATE, CLOSE);
    Atomics.notify(this.ctrl, STATE);
  }
}

/** the length the chunk `chunk` of the stream must have */
export function chunkLength(s: VfsStream, chunk: number): number {
  return Math.min(s.chunkSize, s.logical - chunk * s.chunkSize);
}

/**
 * The I/O Worker's loop. Serves requests until the engine side closes the channel. `store` gives the decoded bytes of a chunk; their length is checked here against the index (a chunk of the wrong size is an error, not a short read).
 * Returns the number of requests it served.
 */
export async function serveChunkIo(shared: ChunkIoShared, streams: readonly VfsStream[], store: ChunkStore): Promise<number> {
  const c = new Int32Array(shared.ctrl);
  const data = new Uint8Array(shared.data);
  let served = 0;
  for (;;) {
    let s = Atomics.load(c, STATE);
    while (s !== REQUEST) {
      if (s === CLOSE) return served;
      Atomics.wait(c, STATE, s);
      s = Atomics.load(c, STATE);
    }
    const si = Atomics.load(c, STREAM), ci = Atomics.load(c, CHUNK);
    let code = 0, len = 0;
    const st = streams[si];
    if (!st || ci < 0 || ci >= st.chunks.length) code = ERR_REQUEST;
    else {
      try {
        const want = chunkLength(st, ci);
        const bytes = await store.get(st.chunks[ci] as string, st.coding, want);
        if (bytes.length !== want || bytes.length > data.length) code = ERR_LENGTH;
        else { data.set(bytes); len = bytes.length; }
      } catch {
        code = ERR_STORE;
      }
    }
    if (Atomics.load(c, STATE) !== REQUEST) continue;   // the engine gave up (timeout) or closed while the fetch ran: nobody waits for this reply
    if (code) { Atomics.store(c, CODE, code); Atomics.store(c, STATE, ERROR); }
    else { Atomics.store(c, LENGTH, len); Atomics.store(c, STATE, REPLY); }
    Atomics.notify(c, STATE);
    served++;
  }
}
