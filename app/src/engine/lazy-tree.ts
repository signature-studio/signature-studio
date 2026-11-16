// Decision 0314 (PROVISIONAL): puts the streams of a lazy install tree among the host files of the engine. The engine's seed list has a row `<mount>\<TAB>@lazy:vfs/index.bin<TAB><SHA-256 of the index>` (lift/harness/vfs_lazy.h): at
// start-up the C side reads `<root>/vfs/index.bin` (an ordinary, released-at-first-idle host file here) and registers the streams `<root>/vfs/<stream name>` as the files it reads the lazy bytes from, with pread, at any time.

import { MemorySource, type HostFiles } from './hostfiles.ts';
import { ChunkCache, ChunkedSource } from './chunked-source.ts';
import type { ChunkClient } from './chunk-io.ts';
import { parseIndex, type VfsIndex } from './vfs-index.ts';

export interface MountedLazyTree {
  index: VfsIndex;
  cache: ChunkCache;
  /** the host file names that were added: the index and one per stream */
  paths: string[];
}

/**
 * `root` is the directory the seed list's host names are relative to (the engine's FLE_VFS_SEED_ROOT); the index goes to `<root>/vfs/index.bin` and the streams to `<root>/vfs/<name>`.
 * `cacheBytes` is the budget of decoded chunks the page holds (the chunks of a file being read, and what was read lately).
 */
export function mountLazyTree(files: HostFiles, root: string, indexBytes: Uint8Array, client: ChunkClient, cacheBytes = 64 * 1024 * 1024): MountedLazyTree {
  const index = parseIndex(indexBytes);
  const cache = new ChunkCache(cacheBytes);
  const dir = `${root.replace(/\/+$/, '')}/vfs`;
  const paths: string[] = [`${dir}/index.bin`];
  files.add(paths[0] as string, new MemorySource(indexBytes), 0, indexBytes.length);
  index.streams.forEach((s, i) => {
    const p = `${dir}/${s.name}`;
    files.addPersistent(p, new ChunkedSource(s, i, client, cache));
    paths.push(p);
  });
  return { index, cache, paths };
}
