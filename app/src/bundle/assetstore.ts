// A persistent copy of the big, content-addressed assets (decision 0220): the DECODED bytes of the engine wasm, the two packs and the glue, in the page's origin private file system (OPFS).
// Why: the browser's HTTP cache can serve a stored response but the page still pays for decoding (brotli, ~215 MB) and for hashing it in JS; and the HTTP cache is shared with everything else, evicted
// in LRU order, limited per entry (cache size / 8) and absent in private windows. A file named by the SHA-256 that the manifest names needs none of that: it is found by the hash, read as it is, and
// verified again (WebCrypto) before it is used. A damaged, truncated or tampered file is simply a miss.
//   OPFS  /fl/cache/assets/<sha256 hex>.<wasm|flpack|js>      one file per asset; written through createWritable, which only shows the file once it is closed (after the page verified the bytes)
// Web Locks keep a page's files from being removed by another page's clean-up (shared lock 'fl-asset:<sha>' held for the life of the page, clean-up takes the exclusive lock with ifAvailable).
// Nothing here asks for navigator.storage.persist(): Firefox answers that with a question to the person. The state is reported (navigator.storage.persisted / estimate) so that it can be seen.

import type { AssetCache, AssetWriter } from './loader.ts';
import type { AssetRef } from './manifest.ts';

export const STORE_PATH: readonly string[] = ['fl', 'cache', 'assets'];
/** the failure codes of the report: a fixed list, the text of an error never leaves the page */
export type StoreFail = 'unsupported' | 'quota' | 'security' | 'locked' | 'io' | 'other';

export function storeFailCode(e: unknown): StoreFail {
  const n = (e as { name?: string } | null)?.name ?? '';
  const m = String((e as { message?: string } | null)?.message ?? e);
  if (n === 'QuotaExceededError' || /quota|no space|not enough/i.test(m)) return 'quota';
  if (n === 'SecurityError' || n === 'NotAllowedError') return 'security';
  if (n === 'NoModificationAllowedError' || n === 'InvalidStateError') return 'locked';
  if (n === 'NotSupportedError' || n === 'TypeError' || /not (a function|supported)|undefined/i.test(m)) return 'unsupported';
  if (n === 'NotFoundError' || n === 'NotReadableError' || n === 'AbortError' || n === 'EncodingError' || n === 'UnknownError' || n === 'DataError') return 'io';
  return 'other';
}

/** the part of FileSystemDirectoryHandle that is used (the tests give a small in-memory one) */
export interface StoreDirectory {
  getDirectoryHandle(name: string, options?: { create?: boolean }): Promise<StoreDirectory>;
  getFileHandle(name: string, options?: { create?: boolean }): Promise<StoreFile>;
  removeEntry(name: string, options?: { recursive?: boolean }): Promise<void>;
  entries(): AsyncIterable<[string, { kind: 'file' | 'directory' }]>;
}
export interface StoreFile {
  getFile(): Promise<{ size: number; arrayBuffer(): Promise<ArrayBuffer> }>;
  createWritable(options?: { keepExistingData?: boolean }): Promise<{ write(data: Uint8Array): Promise<void>; close(): Promise<void>; abort(): Promise<void> }>;
}
export interface StoreLocks {
  request<T>(name: string, options: { mode?: 'shared' | 'exclusive'; ifAvailable?: boolean }, cb: (lock: unknown) => Promise<T> | T): Promise<T>;
}

export interface StoreEnv {
  root(): Promise<StoreDirectory>;
  locks?: StoreLocks | undefined;
  estimate?: (() => Promise<{ usage?: number; quota?: number }>) | undefined;
  persisted?: (() => Promise<boolean>) | undefined;
}

export function browserEnv(): StoreEnv | null {
  const nav = typeof navigator !== 'undefined' ? navigator : undefined;
  if (!nav?.storage?.getDirectory) return null;
  const st = nav.storage;
  return {
    root: async () => (await st.getDirectory()) as unknown as StoreDirectory,
    locks: (nav as unknown as { locks?: StoreLocks }).locks,
    estimate: st.estimate ? () => st.estimate() : undefined,
    persisted: st.persisted ? () => st.persisted() : undefined,
  };
}

export const entryName = (ref: AssetRef): string => {
  const base = ref.url.split(/[?#]/)[0]?.split('/').pop() ?? '';
  const ext = /\.([a-z0-9]{1,8})$/i.exec(base)?.[1]?.toLowerCase() ?? 'bin';
  return `${ref.sha256}.${ext}`;
};
const ENTRY_RE = /^([0-9a-f]{64})\.[a-z0-9]{1,8}$/;
const lockName = (sha: string): string => `fl-asset:${sha}`;

export interface StoreStats { usedMiB: number | null; quotaMiB: number | null; persisted: boolean | null }
export interface CleanResult { removed: number; kept: number; busy: number; failed: number }

export class AssetStore implements AssetCache {
  private dir: Promise<StoreDirectory> | null = null;
  private readonly env: StoreEnv;
  private readonly held = new Set<string>();
  constructor(env: StoreEnv) { this.env = env; }

  /** null: this browser has no OPFS (the page then loads as it always did) */
  static open(env: StoreEnv | null = browserEnv()): AssetStore | null { return env ? new AssetStore(env) : null; }

  private directory(): Promise<StoreDirectory> {
    this.dir ??= (async () => { let d = await this.env.root(); for (const s of STORE_PATH) d = await d.getDirectoryHandle(s, { create: true }); return d; })();
    // a failed attempt is not kept: the next call tries again
    this.dir.catch(() => { this.dir = null; });
    return this.dir;
  }

  async get(ref: AssetRef): Promise<{ size: number; arrayBuffer(): Promise<ArrayBuffer> } | null> {
    const d = await this.directory();
    let fh: StoreFile;
    try { fh = await d.getFileHandle(entryName(ref)); } catch (e) { if ((e as { name?: string })?.name === 'NotFoundError') return null; throw e; }
    const f = await fh.getFile();
    if (f.size !== ref.size) return null;
    this.hold(ref.sha256);
    return f;
  }

  async begin(ref: AssetRef): Promise<AssetWriter | null> {
    const d = await this.directory();
    const name = entryName(ref);
    const est = await this.estimate();
    if (est.quotaMiB !== null && est.usedMiB !== null && (est.quotaMiB - est.usedMiB) * 1024 * 1024 < ref.size * 1.25) throw Object.assign(new Error('not enough room left in the storage quota'), { name: 'QuotaExceededError' });
    // one writer per hash across tabs (Firefox refuses a second writable on the same file)
    const locks = this.env.locks;
    let release: (() => void) | null = null;
    if (locks) {
      const got = await new Promise<boolean>((resolve) => {
        void locks.request(`fl-asset-write:${ref.sha256}`, { mode: 'exclusive', ifAvailable: true }, (lock) => {
          if (!lock) { resolve(false); return undefined; }
          resolve(true);
          return new Promise<void>((r) => { release = r; });
        });
      });
      if (!got) return null;
    }
    const done = (): void => { const r = release as (() => void) | null; release = null; r?.(); };
    let w: Awaited<ReturnType<StoreFile['createWritable']>>;
    try { w = await (await d.getFileHandle(name, { create: true })).createWritable(); } catch (e) { done(); throw e; }
    let written = 0;
    return {
      write: async (data) => { await w.write(data); written += data.length; },
      commit: async () => {
        try {
          if (written !== ref.size) throw Object.assign(new Error(`wrote ${written} of ${ref.size} bytes`), { name: 'EncodingError' });
          await w.close();
          // the file as the browser shows it now must be the whole asset (a short file would be a miss next time and a rewrite each time)
          const f = await (await d.getFileHandle(name)).getFile();
          if (f.size !== ref.size) { await d.removeEntry(name).catch(() => {}); throw Object.assign(new Error(`stored file has ${f.size} of ${ref.size} bytes`), { name: 'EncodingError' }); }
          this.hold(ref.sha256);
        } finally { done(); }
      },
      abort: async () => { try { await w.abort(); } catch { /* already closed */ } try { const f = await (await d.getFileHandle(name)).getFile(); if (f.size !== ref.size) await d.removeEntry(name); } catch { /* nothing to remove */ } done(); },
    };
  }

  async discard(ref: AssetRef): Promise<void> {
    try { await (await this.directory()).removeEntry(entryName(ref)); } catch { /* gone, or in use: a later verified write replaces it */ }
  }

  private hold(sha: string): void {
    const locks = this.env.locks;
    if (!locks || this.held.has(sha)) return;
    this.held.add(sha);
    void locks.request(lockName(sha), { mode: 'shared' }, () => new Promise<void>(() => { /* held until the page is gone */ })).catch(() => {});
  }

  async estimate(): Promise<StoreStats> {
    let usedMiB: number | null = null, quotaMiB: number | null = null, persisted: boolean | null = null;
    try { const e = await this.env.estimate?.(); if (e) { usedMiB = typeof e.usage === 'number' ? Math.floor(e.usage / 1048576) : null; quotaMiB = typeof e.quota === 'number' ? Math.floor(e.quota / 1048576) : null; } } catch { /* unknown */ }
    try { const p = await this.env.persisted?.(); persisted = typeof p === 'boolean' ? p : null; } catch { /* unknown */ }
    return { usedMiB, quotaMiB, persisted };
  }

  /** removes the files that are not named by `keep` (the older versions), unless another page holds them; entries that are not ours are left alone */
  async clean(keep: readonly AssetRef[]): Promise<CleanResult> {
    const res: CleanResult = { removed: 0, kept: 0, busy: 0, failed: 0 };
    const want = new Set(keep.map(entryName));
    const d = await this.directory();
    const names: string[] = [];
    for await (const [n, h] of d.entries()) if (h.kind === 'file') names.push(n);
    for (const n of names) {
      const m = ENTRY_RE.exec(n);
      if (!m) continue;
      if (want.has(n)) { res.kept++; continue; }
      const rm = async (): Promise<void> => { try { await d.removeEntry(n); res.removed++; } catch (e) { if (storeFailCode(e) === 'locked') res.busy++; else res.failed++; } };
      const locks = this.env.locks;
      if (!locks) { res.busy++; continue; } // without Web Locks it cannot be known whether another page is using the file: it is left
      await locks.request(lockName(m[1] as string), { mode: 'exclusive', ifAvailable: true }, async (lock) => { if (lock) await rm(); else res.busy++; });
    }
    return res;
  }
}
