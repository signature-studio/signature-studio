// Issue #1314 (PROVISIONAL): the downloads that start when the engine has started (manifest.lazy), one after another in the manifest's order. Each file is fetched and verified the way the
// start-up assets are (loader.ts fetchAsset: the size and the SHA-256 from the manifest, the stored copy in OPFS), then handed to the engine's file (engine/lazy-file.ts). A file that fails
// does not stop the others: its feed is told (a read of it then fails by name) and the failure is returned for the page to show.

import { fetchAsset, LoadError, type LoadOptions } from './loader.ts';
import type { LazyRef } from './manifest.ts';
import type { LazyFileFeed } from '../engine/lazy-file.ts';

export interface LazyDownload { ref: LazyRef; feed: LazyFileFeed }
export interface LazyResult { name: string; ok: boolean; /** the LoadError's text, or the reason; null when ok */ message: string | null }

export async function downloadLazy(items: readonly LazyDownload[], opts: LoadOptions, onResult?: (r: LazyResult) => void): Promise<LazyResult[]> {
  const out: LazyResult[] = [];
  for (const { ref, feed } of items) {
    let r: LazyResult;
    try {
      feed.deliver(await fetchAsset(ref, ref.name, opts));
      r = { name: ref.name, ok: true, message: null };
    } catch (e) {
      feed.fail();
      r = { name: ref.name, ok: false, message: e instanceof LoadError ? e.describe().what : String((e as Error)?.message ?? e) };
    }
    out.push(r);
    onResult?.(r);
  }
  return out;
}
