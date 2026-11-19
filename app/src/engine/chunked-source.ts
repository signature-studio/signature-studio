// Decision 0314 (PROVISIONAL): a stream of the lazy install tree as a ByteSource (hostfiles.ts): the logical bytes of the stream, read synchronously. A read that needs a chunk the page does not hold asks the ChunkClient
// (chunk-io.ts: the engine thread waits for the I/O Worker); the decoded chunks are kept in a small LRU shared by all the streams, so a file read in pieces is one fetch.

import type { ByteSource } from './hostfiles.ts';
import type { ChunkClient } from './chunk-io.ts';
import type { VfsStream } from './vfs-index.ts';

export interface ChunkCacheStats { hits: number; misses: number; evictions: number; heldBytes: number; fetchedBytes: number }

/** decoded chunks by (stream, chunk), least recently used out first when the held bytes pass the budget */
export class ChunkCache {
  private readonly map = new Map<number, Uint8Array>();   // insertion order = recency
  readonly stats: ChunkCacheStats = { hits: 0, misses: 0, evictions: 0, heldBytes: 0, fetchedBytes: 0 };
  readonly budget: number;

  constructor(budgetBytes: number) { this.budget = budgetBytes; }

  static key(stream: number, chunk: number): number { return stream * 0x1000000 + chunk; }   // 16 streams x 16 M chunks (a stream is at most 16 M x 8 MiB)

  get(key: number): Uint8Array | undefined {
    const v = this.map.get(key);
    if (v === undefined) return undefined;
    this.map.delete(key);
    this.map.set(key, v);
    this.stats.hits++;
    return v;
  }

  put(key: number, bytes: Uint8Array): void {
    this.stats.misses++;
    this.stats.fetchedBytes += bytes.length;
    const old = this.map.get(key);
    if (old) { this.stats.heldBytes -= old.length; this.map.delete(key); }
    this.map.set(key, bytes);
    this.stats.heldBytes += bytes.length;
    while (this.stats.heldBytes > this.budget && this.map.size > 1) {   // the chunk just put always stays
      const first = this.map.keys().next().value as number;
      const v = this.map.get(first) as Uint8Array;
      this.map.delete(first);
      this.stats.heldBytes -= v.length;
      this.stats.evictions++;
    }
  }
}

export class ChunkedSource implements ByteSource {
  private readonly stream: VfsStream;
  private readonly index: number;
  private readonly client: ChunkClient;
  private readonly cache: ChunkCache;

  constructor(stream: VfsStream, index: number, client: ChunkClient, cache: ChunkCache) {
    this.stream = stream;
    this.index = index;
    this.client = client;
    this.cache = cache;
  }

  get size(): number { return this.stream.logical; }

  private chunk(c: number): Uint8Array {
    const key = ChunkCache.key(this.index, c);
    const hit = this.cache.get(key);
    if (hit) return hit;
    const b = this.client.fetchChunk(this.index, c);
    this.cache.put(key, b);
    return b;
  }

  readInto(dst: Uint8Array, at: number): number {
    const s = this.stream;
    if (at >= s.logical || dst.length === 0) return 0;
    const n = Math.min(dst.length, s.logical - at);
    let done = 0;
    while (done < n) {
      const pos = at + done;
      const c = Math.floor(pos / s.chunkSize), inChunk = pos - c * s.chunkSize;
      const b = this.chunk(c);
      const take = Math.min(n - done, b.length - inChunk);
      if (take <= 0) throw new Error(`stream ${s.name} chunk ${c} is ${b.length} bytes, the read needs byte ${inChunk}`);
      dst.set(b.subarray(inChunk, inChunk + take), done);
      done += take;
    }
    return n;
  }
}
