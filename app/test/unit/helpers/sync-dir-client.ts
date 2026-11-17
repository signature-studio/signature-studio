// Test helper for decision 0314: a synchronous ChunkClient over a directory of chunk objects (node fs + zlib), the same bytes the I/O Worker would give.
import { brotliDecompressSync } from 'node:zlib';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import type { ChunkClient } from '../../../src/engine/chunk-io.ts';
import type { VfsStream } from '../../../src/engine/vfs-index.ts';

export class SyncDirClient implements ChunkClient {
  fetches = 0;
  private readonly dir: string;
  private readonly streams: readonly VfsStream[];
  constructor(dir: string, streams: readonly VfsStream[]) { this.dir = dir; this.streams = streams; }
  fetchChunk(stream: number, chunk: number): Uint8Array {
    this.fetches++;
    const s = this.streams[stream] as VfsStream;
    const sha = s.chunks[chunk] as string;
    const stored = readFileSync(join(this.dir, 'vfs', `${sha}.chk`));
    const b = new Uint8Array(s.coding === 1 ? brotliDecompressSync(stored) : stored);
    if (createHash('sha256').update(b).digest('hex') !== sha) throw new Error(`chunk ${sha}: the decoded bytes do not hash to the name`);
    return b;
  }
}
