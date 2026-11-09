// node adapter of the WorkerHandle interface (plugin-client.ts) for the tests and for node-side tools: plugin-worker.ts is run as a worker_threads Worker (node 24 strips the types itself).
import { Worker } from 'node:worker_threads';
import type { WorkerOut } from './plugin-worker.ts';
import type { WorkerHandle } from './plugin-client.ts';

export function nodePluginWorker(): WorkerHandle {
  const w = new Worker(new URL('./plugin-worker.ts', import.meta.url));
  return {
    post: (m) => w.postMessage(m),
    onMessage: (cb) => w.on('message', (m: WorkerOut) => cb(m)),
    onError: (cb) => w.on('error', cb),
    terminate: () => void w.terminate(),
  };
}
