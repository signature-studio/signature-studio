// A small in-memory stand-in for the part of FileSystemDirectoryHandle that src/bundle/assetstore.ts uses, with the semantics that matter: a file written through createWritable() only shows its new bytes at close(),
// abort() leaves the old bytes, removeEntry() of a file with an open writable fails (NoModificationAllowedError), Web Locks with shared / exclusive / ifAvailable.
import type { StoreDirectory, StoreEnv, StoreFile, StoreLocks } from '../../src/bundle/assetstore.ts';

class DomErr extends Error { constructor(name: string, message = name) { super(message); this.name = name; } }

class FakeFile implements StoreFile {
  data = new Uint8Array(0);
  writers = 0;
  /** test hooks */
  failWriteAfter = Infinity;
  writeCalls = 0;
  /** each write waits this long BEFORE it reads the caller's bytes (a real writable reads them when its queue gets to the chunk, not when write() is called) */
  writeDelayMs = 0;
  async getFile() { const d = this.data; return { size: d.length, arrayBuffer: async () => d.slice().buffer as ArrayBuffer }; }
  async createWritable() {
    const parts: Uint8Array[] = [];
    this.writers++;
    let open = true;
    const self = this;
    return {
      async write(chunk: Uint8Array) {
        if (!open) throw new DomErr('InvalidStateError');
        if (self.writeDelayMs > 0) await new Promise((r) => setTimeout(r, self.writeDelayMs));
        if (self.writeCalls++ >= self.failWriteAfter) throw new DomErr('QuotaExceededError');
        parts.push(chunk.slice());
      },
      async close() { if (!open) throw new DomErr('InvalidStateError'); open = false; self.writers--; const n = parts.reduce((a, p) => a + p.length, 0); const o = new Uint8Array(n); let at = 0; for (const p of parts) { o.set(p, at); at += p.length; } self.data = o; },
      async abort() { if (open) { open = false; self.writers--; } },
    };
  }
}

export class FakeDir implements StoreDirectory {
  readonly dirs = new Map<string, FakeDir>();
  readonly files = new Map<string, FakeFile>();
  async getDirectoryHandle(name: string, o?: { create?: boolean }) {
    let d = this.dirs.get(name);
    if (!d) { if (!o?.create) throw new DomErr('NotFoundError'); d = new FakeDir(); this.dirs.set(name, d); }
    return d;
  }
  async getFileHandle(name: string, o?: { create?: boolean }) {
    let f = this.files.get(name);
    if (!f) { if (!o?.create) throw new DomErr('NotFoundError'); f = new FakeFile(); this.files.set(name, f); }
    return f;
  }
  async removeEntry(name: string) {
    const f = this.files.get(name);
    if (f) { if (f.writers > 0) throw new DomErr('NoModificationAllowedError'); this.files.delete(name); return; }
    if (this.dirs.delete(name)) return;
    throw new DomErr('NotFoundError');
  }
  async *entries(): AsyncIterable<[string, { kind: 'file' | 'directory' }]> {
    for (const n of this.dirs.keys()) yield [n, { kind: 'directory' }];
    for (const n of this.files.keys()) yield [n, { kind: 'file' }];
  }
  /** the file at a path like 'fl/cache/assets/NAME', or undefined */
  at(path: string): FakeFile | undefined {
    const parts = path.split('/');
    let d: FakeDir = this;
    for (const p of parts.slice(0, -1)) { const n = d.dirs.get(p); if (!n) return undefined; d = n; }
    return d.files.get(parts[parts.length - 1] as string);
  }
}

export class FakeLocks implements StoreLocks {
  private readonly shared = new Map<string, number>();
  private readonly exclusive = new Set<string>();
  /** the names that currently have a holder */
  held(): string[] { return [...new Set([...this.shared.keys(), ...this.exclusive])]; }
  async request<T>(name: string, o: { mode?: 'shared' | 'exclusive'; ifAvailable?: boolean }, cb: (lock: unknown) => Promise<T> | T): Promise<T> {
    const mode = o.mode ?? 'exclusive';
    const busy = mode === 'shared' ? this.exclusive.has(name) : this.exclusive.has(name) || (this.shared.get(name) ?? 0) > 0;
    if (busy) { if (o.ifAvailable) return cb(null); throw new Error('FakeLocks: a waiting request is not modelled'); }
    if (mode === 'shared') this.shared.set(name, (this.shared.get(name) ?? 0) + 1); else this.exclusive.add(name);
    try { return await cb({ name, mode }); } finally { if (mode === 'shared') this.shared.set(name, (this.shared.get(name) ?? 1) - 1); else this.exclusive.delete(name); }
  }
}

export function fakeEnv(root: FakeDir, extra: Partial<StoreEnv> = {}): StoreEnv & { locks: FakeLocks } {
  const locks = new FakeLocks();
  return { root: async () => root, locks, estimate: async () => ({ usage: 1e9, quota: 1e12 }), persisted: async () => false, ...extra } as StoreEnv & { locks: FakeLocks };
}
