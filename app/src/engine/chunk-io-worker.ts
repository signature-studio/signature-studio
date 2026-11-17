// Decision 0314 (PROVISIONAL): the I/O Worker of the lazy install tree. The engine Worker starts it (`new Worker(new URL('./chunk-io-worker.ts', import.meta.url), { type: 'module' })` as ONE expression, decision 0189) and
// posts one message: the shared buffers of chunk-io.ts, the streams of the index and where the chunks are. The loop then serves the engine thread's requests until the engine closes the channel.

import { FetchChunkStore } from './chunk-store.ts';
import { serveChunkIo, type ChunkIoShared } from './chunk-io.ts';
import type { VfsStream } from './vfs-index.ts';

export interface ChunkIoStart { type: 'start'; shared: ChunkIoShared; streams: VfsStream[]; base: string }
export type ChunkIoReply = { type: 'ready' } | { type: 'done'; served: number } | { type: 'error'; message: string };

const scope = self as unknown as { onmessage: ((e: MessageEvent<ChunkIoStart>) => void) | null; postMessage(m: ChunkIoReply): void };
scope.onmessage = (e) => {
  const m = e.data;
  if (m.type !== 'start') return;
  scope.postMessage({ type: 'ready' });
  serveChunkIo(m.shared, m.streams, new FetchChunkStore(m.base)).then((served) => scope.postMessage({ type: 'done', served }), (err: unknown) => scope.postMessage({ type: 'error', message: String((err as Error)?.stack ?? err) }));
};
