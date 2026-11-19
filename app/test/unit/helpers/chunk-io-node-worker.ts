// Test helper for decision 0314: the I/O Worker of the tests, a node worker_thread over a directory of chunk objects. `failSha`: the hex names whose chunk is refused (the store throws).
import { parentPort, workerData } from 'node:worker_threads';
import { serveChunkIo, type ChunkIoShared } from '../../../src/engine/chunk-io.ts';
import { DirChunkStore } from './lazy-tree-gen.ts';
import type { VfsStream } from '../../../src/engine/vfs-index.ts';
import type { ChunkStore } from '../../../src/engine/chunk-store.ts';

const d = workerData as { shared: ChunkIoShared; streams: VfsStream[]; dir: string; failSha?: string[]; delayMs?: number; wrongLength?: boolean };
const inner = new DirChunkStore(d.dir);
const store: ChunkStore = {
  async get(sha, coding, length) {
    if (d.delayMs) await new Promise((r) => setTimeout(r, d.delayMs));
    if (d.failSha?.includes(sha)) throw new Error('refused by the test');
    if (d.wrongLength) return new Uint8Array(10);
    return inner.get(sha, coding, length);
  },
};
parentPort?.postMessage({ type: 'ready' });
serveChunkIo(d.shared, d.streams, store).then((served) => parentPort?.postMessage({ type: 'done', served }), (e: unknown) => parentPort?.postMessage({ type: 'error', message: String(e) }));
