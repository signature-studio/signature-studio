// A small async file system over a FileSystemDirectoryHandle (the browser's OPFS: navigator.storage.getDirectory()), for the file manager and, later, for the engine's open/save dialogs (decision 0216, opfs1).
// Everything goes by absolute path ('/a/b'); the root is the handle given to the constructor, so a test (or the guest's volume, stage 3) can use any subdirectory as its root.
// Only what Chrome 102+ and Firefox 111+ both have is used (MDN browser-compat-data, checked 2026-11-04): getFileHandle/getDirectoryHandle/removeEntry/entries, createWritable, FileSystemHandle.move (Chrome: partial, so it falls back to copy + remove).
import { baseName, joinPath, normalizePath, parentOf, splitPath, uniqueName } from './paths.ts';
import { MemoryDirectoryHandle } from './memory.ts';

export type Dir = FileSystemDirectoryHandle;

export interface FsEntry {
  name: string;
  path: string;
  kind: 'file' | 'directory';
  /** bytes (files only) */
  size: number;
  /** ms since the epoch (files only) */
  modified: number;
  /** the handle the listing already had (files only): reading size and date through it saves resolving the path again for every file */
  handle?: FileSystemFileHandle;
}

export class FsError extends Error {
  readonly code: string;
  constructor(code: string, message: string) { super(message); this.code = code; }
}

type DirIter = FileSystemDirectoryHandle & { entries(): AsyncIterable<[string, FileSystemHandle]> };
const iter = (d: Dir): AsyncIterable<[string, FileSystemHandle]> => (d as DirIter).entries();

export function describeError(e: unknown): string {
  if (e instanceof FsError) return e.message;
  const n = (e as { name?: string })?.name;
  const m = (e as { message?: string })?.message ?? String(e);
  if (n === 'NoModificationAllowedError') return 'The file is in use (open in the engine or another tab), so it cannot be changed.';
  if (n === 'NotFoundError') return 'Not found.';
  if (n === 'QuotaExceededError') return 'The browser has no room left for this (storage quota).';
  if (n === 'TypeMismatchError') return 'A file and a folder have the same name.';
  if (n === 'InvalidModificationError') return 'The folder is not empty or the target is in the way.';
  return `${n ?? 'Error'}: ${m}`;
}

/** where the page's files live: the browser's OPFS, or (decision 0248) the tab's memory when the browser gives the page no usable OPFS; `why` says what was wrong with the OPFS */
export interface StorageRoot { root: Dir; persistent: boolean; why: string | null }

/** the one place that decides: the OPFS root if it can be written to and read back, else a memory tree. The probe file is removed again. */
export async function detectRoot(nav: { storage?: { getDirectory?: () => Promise<FileSystemDirectoryHandle> } } = navigator): Promise<StorageRoot> {
  let why: string;
  if (!nav.storage?.getDirectory) why = 'unsupported';
  else {
    try {
      const d = await nav.storage.getDirectory();
      const probe = `.fl-probe-${Math.random().toString(36).slice(2, 10)}`;   // a name of its own: another tab probing at the same moment must not make this one fail (Firefox allows one writable per file)
      try {
        const w = await (await d.getFileHandle(probe, { create: true })).createWritable();
        await w.write(new Uint8Array([1]));
        await w.close();
        if ((await (await d.getFileHandle(probe)).getFile()).size !== 1) throw new FsError('probe', 'The browser stored a different number of bytes than were written.');
      } finally { await d.removeEntry(probe).catch(() => { /* nothing to remove */ }); }
      return { root: d, persistent: true, why: null };
    } catch (e) { why = (e as { name?: string })?.name === 'SecurityError' || (e as { name?: string })?.name === 'NotAllowedError' ? 'blocked' : `failed: ${describeError(e)}`; }
  }
  return { root: new MemoryDirectoryHandle() as unknown as Dir, persistent: false, why };
}

let rootP: Promise<StorageRoot> | null = null;
/** decided once per page: the Files window and the engine's dialogs must see the same files */
export const storageRoot = (): Promise<StorageRoot> => (rootP ??= detectRoot());

/** one line for the person: why the files are only in the tab's memory */
export function memoryNotice(why: string | null): string {
  const reason = why === 'unsupported' ? 'this browser has no file storage for pages'
    : why === 'blocked' ? 'this browser does not give the page file storage (a private window, or site data blocked)'
      : `the browser's file storage did not work (${(why ?? '').replace(/^failed: /, '')})`;
  return `These files are kept in this tab's memory only: ${reason}. They are gone when the tab is closed or reloaded.`;
}

/** bytes the browser still allows for the OPFS, or null when no limit applies (the files are in the tab's memory: the OPFS quota is not theirs) or the browser did not say */
export function roomLeft(fs: { persistent: boolean }, s: { usage: number | null; quota: number | null }): number | null {
  if (!fs.persistent || s.quota === null || s.usage === null) return null;
  return Math.max(0, s.quota - s.usage);
}

export class OpfsFs {
  readonly root: Dir;
  /** false: the files are in the tab's memory (see detectRoot) */
  readonly persistent: boolean;
  readonly why: string | null;
  constructor(root: Dir, persistent = true, why: string | null = null) { this.root = root; this.persistent = persistent; this.why = why; }

  static async open(sub?: string): Promise<OpfsFs> {
    const r = await storageRoot();
    let d = r.root;
    for (const s of sub ? splitPath(sub) : []) d = await d.getDirectoryHandle(s, { create: true });
    return new OpfsFs(d, r.persistent, r.why);
  }

  async dir(path: string, create = false): Promise<Dir> {
    let d = this.root;
    for (const s of splitPath(normalizePath(path))) d = await d.getDirectoryHandle(s, { create });
    return d;
  }

  async fileHandle(path: string, create = false): Promise<FileSystemFileHandle> {
    const p = normalizePath(path);
    return (await this.dir(parentOf(p), create)).getFileHandle(baseName(p), { create });
  }

  async stat(path: string): Promise<FsEntry | null> {
    const p = normalizePath(path);
    if (p === '/') return { name: '', path: '/', kind: 'directory', size: 0, modified: 0 };
    let parent: Dir;
    try { parent = await this.dir(parentOf(p)); } catch { return null; }
    const name = baseName(p);
    try { const f = await (await parent.getFileHandle(name)).getFile(); return { name, path: p, kind: 'file', size: f.size, modified: f.lastModified }; } catch { /* not a file */ }
    try { await parent.getDirectoryHandle(name); return { name, path: p, kind: 'directory', size: 0, modified: 0 }; } catch { return null; }
  }

  async exists(path: string): Promise<boolean> { return (await this.stat(path)) !== null; }

  /** the entries of a directory, folders first then by name (case-insensitive). stat=false lists names and kinds only (sizes and dates are 0): a file's size needs getFile() per file, which is the slow part of a big folder; see fillStats. */
  async list(path: string, stat = true): Promise<FsEntry[]> {
    const p = normalizePath(path);
    const d = await this.dir(p);
    const out: FsEntry[] = [];
    const files: [number, FileSystemFileHandle][] = [];
    for await (const [name, h] of iter(d)) {
      const e: FsEntry = { name, path: joinPath(p, name), kind: h.kind === 'file' ? 'file' : 'directory', size: 0, modified: 0 };
      if (h.kind === 'file') { e.handle = h as FileSystemFileHandle; files.push([out.length, e.handle]); }
      out.push(e);
    }
    if (stat) {
      const BATCH = 32;
      for (let i = 0; i < files.length; i += BATCH) {
        await Promise.all(files.slice(i, i + BATCH).map(async ([k, h]) => { try { const f = await h.getFile(); out[k]!.size = f.size; out[k]!.modified = f.lastModified; } catch { /* locked or gone: 0 */ } }));
      }
    }
    return out.sort((a, b) => (a.kind === b.kind ? a.name.toLowerCase() < b.name.toLowerCase() ? -1 : a.name.toLowerCase() > b.name.toLowerCase() ? 1 : 0 : a.kind === 'directory' ? -1 : 1));
  }

  /** size and date of the given file entries, 32 at a time; calls `each` as batches finish; stops when `alive()` says no */
  async fillStats(entries: FsEntry[], each: (e: FsEntry) => void, alive: () => boolean): Promise<void> {
    const todo = entries.filter((e) => e.kind === 'file');
    const BATCH = 32;
    for (let i = 0; i < todo.length && alive(); i += BATCH) {
      await Promise.all(todo.slice(i, i + BATCH).map(async (e) => {
        try { const f = await (e.handle ?? await this.fileHandle(e.path)).getFile(); e.size = f.size; e.modified = f.lastModified; each(e); } catch { /* locked or gone */ }
      }));
    }
  }

  /** the names of the sub-folders only (the tree's lazy expansion: no file is opened) */
  async subdirs(path: string): Promise<string[]> {
    const out: string[] = [];
    for await (const [n, h] of iter(await this.dir(path))) if (h.kind === 'directory') out.push(n);
    return out.sort((a, b) => a.toLowerCase() < b.toLowerCase() ? -1 : 1);
  }

  async names(path: string): Promise<Set<string>> {
    const s = new Set<string>();
    for await (const [n] of iter(await this.dir(path))) s.add(n.toLowerCase());
    return s;
  }

  /** the actual name in `dir` that equals `name` ignoring case (Windows semantics), or null */
  async findCI(dirPath: string, name: string): Promise<string | null> {
    const want = name.toLowerCase();
    for await (const [n] of iter(await this.dir(dirPath))) if (n.toLowerCase() === want) return n;
    return null;
  }

  async mkdir(path: string): Promise<void> { await this.dir(path, true); }

  async write(path: string, data: Blob | Uint8Array | string | ReadableStream<Uint8Array>, onBytes?: (n: number) => void): Promise<void> {
    const fh = await this.fileHandle(path, true);
    const w = await fh.createWritable();
    try {
      if (data instanceof ReadableStream) {
        const r = data.getReader();
        for (;;) { const { done, value } = await r.read(); if (done) break; await w.write(value as Uint8Array<ArrayBuffer>); onBytes?.(value.length); }
      } else if (data instanceof Blob) {
        const r = data.stream().getReader();
        for (;;) { const { done, value } = await r.read(); if (done) break; await w.write(value as Uint8Array<ArrayBuffer>); onBytes?.(value.length); }
      } else { await w.write(data as Uint8Array<ArrayBuffer> | string); onBytes?.(typeof data === 'string' ? data.length : data.length); }
      await w.close();
    } catch (e) { try { await w.abort(); } catch { /* ignore */ } throw e; }
  }

  async read(path: string): Promise<File> { return (await this.fileHandle(path)).getFile(); }

  async remove(path: string): Promise<void> {
    const p = normalizePath(path);
    if (p === '/') throw new FsError('root', 'The root cannot be deleted.');
    await (await this.dir(parentOf(p))).removeEntry(baseName(p), { recursive: true });
  }

  /** disk use of a path: [files, bytes] */
  async du(path: string): Promise<[number, number]> {
    const s = await this.stat(path);
    if (!s) return [0, 0];
    if (s.kind === 'file') return [1, s.size];
    let files = 0, bytes = 0;
    for (const e of await this.list(path)) { const [f, b] = await this.du(e.path); files += f; bytes += b; }
    return [files, bytes];
  }

  async copy(src: string, destDir: string, newName?: string, onBytes?: (n: number) => void): Promise<string> {
    const s = await this.stat(src);
    if (!s) throw new FsError('missing', `${src} does not exist.`);
    const name = newName ?? baseName(src);
    const dest = joinPath(normalizePath(destDir), name);
    if (s.kind === 'file') { await this.write(dest, await this.read(src), onBytes); return dest; }
    await this.mkdir(dest);
    for (const e of await this.list(src)) await this.copy(e.path, dest, undefined, onBytes);
    return dest;
  }

  /** move or rename. Uses FileSystemHandle.move where the browser has it for this case, else copy + remove (a folder is copied whole first, so a failure leaves the source). */
  async move(src: string, destDir: string, newName?: string): Promise<string> {
    const s = normalizePath(src), dd = normalizePath(destDir);
    const name = newName ?? baseName(s);
    const dest = joinPath(dd, name);
    if (dest === s) return s;
    const st = await this.stat(s);
    if (!st) throw new FsError('missing', `${s} does not exist.`);
    const handle = st.kind === 'file' ? await this.fileHandle(s) : await this.dir(s);
    const mv = (handle as unknown as { move?: (d: Dir, n: string) => Promise<void> }).move;
    if (mv) {
      try { await mv.call(handle, await this.dir(dd, true), name); return dest; } catch (e) {
        if ((e as { name?: string }).name === 'NoModificationAllowedError') throw e;
      }
    }
    await this.copy(s, dd, name);
    await this.remove(s);
    return dest;
  }

  /** a name that is free in destDir (case-insensitive): name, 'name (2)', ... */
  async freeName(destDir: string, name: string): Promise<string> { return uniqueName(name, await this.names(destDir)); }
}
