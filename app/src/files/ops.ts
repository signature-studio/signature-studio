// The file manager's operations (decision 0216, opfs1): put things from outside into the OPFS (files, folders, a zip extracted), take things out (a file, a folder as a zip), with progress,
// cancel, and one question per operation when a name is already taken. The browser APIs are only used through OpfsFs and the Blob / stream types, so a test in Chrome drives these functions directly.
import { Zip, ZipDeflate, ZipPassThrough } from 'fflate';
import { baseName, extOf, joinPath, normalizePath, safeRelative, uniqueName } from './paths.ts';
import { type FsEntry, OpfsFs } from './opfs.ts';
import { crc32Update, entryStream, readZipDirectory, type ZipEntry, ZipError } from './zip-read.ts';

export type Conflict = 'replace' | 'keep-both' | 'skip';

export interface OpProgress {
  label: string;
  /** items finished / total items (0 total = unknown) */
  done: number;
  total: number;
  bytes: number;
  totalBytes: number;
}

export interface OpContext {
  fs: OpfsFs;
  signal: AbortSignal;
  progress(p: OpProgress): void;
  /** called once per operation, at the first name that is already taken; the answer is applied to the rest of the operation. 'cancel' stops the operation. */
  askConflict(name: string, existing: 'file' | 'directory'): Promise<Conflict | 'cancel'>;
}

export interface OpResult {
  written: number;
  skipped: number;
  failed: { path: string; reason: string }[];
  /** names that were changed to be acceptable (Windows rules) or numbered: [from, to] */
  renamed: [string, string][];
  bytes: number;
  cancelled: boolean;
}

export const newResult = (): OpResult => ({ written: 0, skipped: 0, failed: [], renamed: [], bytes: 0, cancelled: false });

class Cancelled extends Error {}

/** one question per operation: remembers the answer */
class Resolver {
  private answer: Conflict | null = null;
  private readonly cache = new Map<string, Set<string>>();
  private readonly ctx: OpContext;
  constructor(ctx: OpContext) { this.ctx = ctx; }

  private async names(dir: string): Promise<Set<string>> {
    let s = this.cache.get(dir);
    if (!s) { s = await this.ctx.fs.names(dir).catch(() => new Set<string>()); this.cache.set(dir, s); }
    return s;
  }

  /** the name to write `name` as in `dir` ('' = skip). Creates nothing. */
  async fileName(dir: string, name: string, result: OpResult): Promise<string> {
    const taken = await this.names(dir);
    if (!taken.has(name.toLowerCase())) { taken.add(name.toLowerCase()); return name; }
    const actual = (await this.ctx.fs.findCI(dir, name)) ?? name;
    const st = await this.ctx.fs.stat(joinPath(dir, actual));
    if (this.answer === null) {
      const a = await this.ctx.askConflict(actual, st?.kind ?? 'file');
      if (a === 'cancel') throw new Cancelled();
      this.answer = a;
    }
    if (this.answer === 'skip') { result.skipped++; return ''; }
    if (this.answer === 'keep-both') { const n = uniqueName(name, taken); taken.add(n.toLowerCase()); result.renamed.push([name, n]); return n; }
    await this.ctx.fs.remove(joinPath(dir, actual));
    return name;
  }

  /** the existing folder name that matches `name` ignoring case, else `name`; a file in the way is a conflict like any other */
  async dirName(dir: string, name: string, result: OpResult): Promise<string> {
    const actual = await this.ctx.fs.findCI(dir, name);
    if (actual === null) { (await this.names(dir)).add(name.toLowerCase()); return name; }
    const st = await this.ctx.fs.stat(joinPath(dir, actual));
    if (st?.kind === 'directory') return actual;
    return this.fileName(dir, name, result);
  }
}

const check = (ctx: OpContext): void => { if (ctx.signal.aborted) throw new Cancelled(); };

export interface Incoming {
  /** relative path inside the destination ('a/b/c.wav' or 'c.wav'); '/' and '\\' both separate */
  path: string;
  blob: Blob;
}

/** write files (with their folders) under destDir */
export async function putFiles(ctx: OpContext, destDir: string, items: Incoming[]): Promise<OpResult> {
  const result = newResult();
  const res = new Resolver(ctx);
  const total = items.length;
  const totalBytes = items.reduce((n, i) => n + i.blob.size, 0);
  let done = 0, bytes = 0;
  try {
    for (const it of items) {
      check(ctx);
      const rel = safeRelative(it.path);
      if (!rel) { result.failed.push({ path: it.path, reason: 'unsafe path (.. or a drive letter)' }); done++; continue; }
      if (rel.changed) result.renamed.push([it.path, rel.parts.join('/')]);
      try {
        let dir = normalizePath(destDir);
        for (const part of rel.parts.slice(0, -1)) {
          const n = await res.dirName(dir, part, result);
          if (n === '') throw new Error('a file is in the way of a folder');
          dir = joinPath(dir, n);
          await ctx.fs.mkdir(dir);
        }
        const leaf = rel.parts[rel.parts.length - 1]!;
        const name = await res.fileName(dir, leaf, result);
        if (name === '') { done++; continue; }
        ctx.progress({ label: it.path, done, total, bytes, totalBytes });
        await ctx.fs.write(joinPath(dir, name), it.blob, (n) => { bytes += n; if (ctx.signal.aborted) throw new Cancelled(); });
        result.written++;
      } catch (e) {
        if (e instanceof Cancelled) throw e;
        result.failed.push({ path: it.path, reason: describe(e) });
      }
      done++;
      ctx.progress({ label: it.path, done, total, bytes, totalBytes });
    }
  } catch (e) { if (!(e instanceof Cancelled)) throw e; result.cancelled = true; }
  result.bytes = bytes;
  return result;
}

const describe = (e: unknown): string => (e instanceof ZipError ? e.message : (e as { name?: string }).name ? `${(e as Error).name}: ${(e as Error).message}` : String(e));

export interface ZipPlan {
  entries: ZipEntry[];
  files: ZipEntry[];
  totalBytes: number;
  /** the common first folder, if every entry is inside one ('Project/'), else '' */
  topFolder: string;
  nameSources: Record<string, number>;
}

export async function planZip(blob: Blob): Promise<ZipPlan> {
  const entries = await readZipDirectory(blob);
  const files = entries.filter((e) => !e.isDir);
  const nameSources: Record<string, number> = {};
  for (const e of entries) nameSources[e.nameSource] = (nameSources[e.nameSource] ?? 0) + 1;
  const firsts = new Set(entries.map((e) => e.name.replace(/\\/g, '/').split('/')[0]));
  const topFolder = firsts.size === 1 && entries.every((e) => e.name.replace(/\\/g, '/').includes('/')) ? [...firsts][0]! : '';
  return { entries, files, totalBytes: files.reduce((n, e) => n + e.size, 0), topFolder, nameSources };
}

/** extract every entry of the zip under destDir (the caller picks the folder); entry by entry, straight from the zip's byte stream into the OPFS file; each file's CRC-32 is checked and a mismatch removes the file */
export async function extractZip(ctx: OpContext, destDir: string, blob: Blob, plan?: ZipPlan): Promise<OpResult> {
  const result = newResult();
  const p = plan ?? await planZip(blob);
  const res = new Resolver(ctx);
  const total = p.files.length;
  let done = 0, bytes = 0;
  await ctx.fs.mkdir(destDir);
  try {
    for (const e of p.entries) {
      check(ctx);
      const rel = safeRelative(e.name);
      if (!rel) { result.failed.push({ path: e.name, reason: 'unsafe path (.. or a drive letter)' }); if (!e.isDir) done++; continue; }
      if (rel.changed) result.renamed.push([e.name, rel.parts.join('/')]);
      try {
        let dir = normalizePath(destDir);
        const dirParts = e.isDir ? rel.parts : rel.parts.slice(0, -1);
        for (const part of dirParts) {
          const n = await res.dirName(dir, part, result);
          if (n === '') throw new Error('a file is in the way of a folder');
          dir = joinPath(dir, n);
          await ctx.fs.mkdir(dir);
        }
        if (e.isDir) continue;
        const name = await res.fileName(dir, rel.parts[rel.parts.length - 1]!, result);
        if (name === '') { done++; continue; }
        ctx.progress({ label: e.name, done, total, bytes, totalBytes: p.totalBytes });
        const path = joinPath(dir, name);
        let crc = 0, got = 0;
        const src = (await entryStream(blob, e)).pipeThrough(new TransformStream<Uint8Array, Uint8Array>({
          transform(chunk, c) { crc = crc32Update(crc, chunk); got += chunk.length; bytes += chunk.length; if (ctx.signal.aborted) throw new Cancelled(); c.enqueue(chunk); },
        }));
        try {
          await ctx.fs.write(path, src);
          if (got !== e.size) throw new ZipError(`size ${got} instead of ${e.size}`);
          if (crc !== e.crc32) throw new ZipError(`CRC-32 mismatch (the zip is damaged)`);
        } catch (err) { await ctx.fs.remove(path).catch(() => {}); throw err; }
        result.written++;
      } catch (err) {
        if (err instanceof Cancelled) throw err;
        result.failed.push({ path: e.name, reason: describe(err) });
      }
      done++;
      ctx.progress({ label: e.name, done, total, bytes, totalBytes: p.totalBytes });
    }
  } catch (e) { if (!(e instanceof Cancelled)) throw e; result.cancelled = true; }
  result.bytes = bytes;
  return result;
}

const STORED_EXT = new Set(['zip', 'mp3', 'ogg', 'oga', 'flac', 'm4a', 'aac', 'png', 'jpg', 'jpeg', 'gif', 'webp', '7z', 'rar', 'gz', 'br', 'mp4']);

/** every file under `path` as a zip Blob (fflate's streaming Zip: names are UTF-8 with the language-encoding flag; files already compressed by their format are stored, the rest deflated at level 3) */
export async function folderToZip(ctx: OpContext, path: string, includeTop = true): Promise<Blob> {
  const base = normalizePath(path);
  const topName = baseName(base);
  const files: FsEntry[] = [];
  const dirs: string[] = [];
  const walk = async (p: string): Promise<void> => {
    for (const e of await ctx.fs.list(p)) {
      if (e.kind === 'directory') { dirs.push(e.path); await walk(e.path); } else files.push(e);
    }
  };
  await walk(base);
  const totalBytes = files.reduce((n, f) => n + f.size, 0);
  const chunks: Uint8Array[] = [];
  let failure: unknown = null;
  let pending = 0;
  let finished: () => void = () => {};
  const finishedP = new Promise<void>((r) => { finished = r; });
  const zip = new Zip((err, data, final) => {
    if (err) { failure = err; finished(); return; }
    chunks.push(data);
    pending += data.length;
    if (final) finished();
  });
  const rel = (p: string): string => (includeTop && topName ? topName + p.slice(base.length) : p.slice(base.length + 1));
  let bytes = 0, done = 0;
  for (const d of dirs) {
    const f = new ZipPassThrough(rel(d).replace(/^\//, '') + '/');
    zip.add(f);
    f.push(new Uint8Array(0), true);
  }
  for (const fe of files) {
    check(ctx);
    ctx.progress({ label: fe.path, done, total: files.length, bytes, totalBytes });
    const stored = STORED_EXT.has(extOf(fe.name));
    const zf = stored ? new ZipPassThrough(rel(fe.path).replace(/^\//, '')) : new ZipDeflate(rel(fe.path).replace(/^\//, ''), { level: 3 });
    zf.mtime = fe.modified || Date.now();
    zip.add(zf);
    const r = (await ctx.fs.read(fe.path)).stream().getReader();
    let any = false;
    for (;;) {
      const { done: end, value } = await r.read();
      if (end) break;
      any = true;
      zf.push(value, false);
      bytes += value.length;
      if (pending > 8 << 20) { pending = 0; await new Promise((res) => setTimeout(res, 0)); check(ctx); }
    }
    zf.push(new Uint8Array(0), true);
    void any;
    done++;
  }
  zip.end();
  await finishedP;
  if (failure) throw failure;
  ctx.progress({ label: '', done, total: files.length, bytes, totalBytes });
  return new Blob(chunks as BlobPart[], { type: 'application/zip' });
}

/** hand a Blob to the browser's download (the person's own click is the gesture; nothing is uploaded) */
export function saveBlob(blob: Blob, name: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  a.rel = 'noopener';
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 60_000);
}

export { Cancelled };
