// Test helper for Issue #1314: a node worker_thread that does what the page's download does for one lazy file, after a delay: it delivers the bytes, or fails the file (bytes null).
import { parentPort, workerData } from 'node:worker_threads';
import { LazyFileFeed, type LazyFileShared } from '../../../src/engine/lazy-file.ts';

const d = workerData as { shared: LazyFileShared; bytes: Uint8Array | null; delayMs: number };
const feed = new LazyFileFeed(d.shared);
parentPort?.postMessage({ type: 'ready' });
setTimeout(() => {
  if (d.bytes) feed.deliver(d.bytes);
  else feed.fail();
  parentPort?.postMessage({ type: 'done' });
}, d.delayMs);
