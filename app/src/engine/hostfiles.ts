// The files the engine reads from "the host" with ordinary POSIX / stdio calls (open, stat, pread, fopen, fgets ...): its DLLs, the seed lists and the seed files of the virtual volume.
// The engine is linked with -sFILESYSTEM=0 (no file system in the Emscripten glue); the page answers the libc system calls itself, from the bytes of the packs. Nothing is copied
// into a file system first: a read copies from the pack straight into the engine's memory (the PE sections of the DLLs are read with pread into their image addresses).
//
// ByteSource: where the bytes of a pack are (MemorySource = a Uint8Array the page holds). HostFiles: the table of paths -> (source, offset, size) and the system calls over it.
//
// The system calls are the ones of Emscripten's wasm32 musl (library_syscall.js / library_wasi.js of emsdk 6.0.12 with -sFILESYSTEM=0 are stubs; this file replaces them in the imports object that
// instantiateWasm is given): __syscall_openat, __syscall_stat64, __syscall_lstat64, __syscall_newfstatat, __syscall_fstat64, fd_read, fd_pread, fd_seek, fd_close. The files are read-only packs:
// a path that is not in the table is ENOENT (an ordinary answer), but a call this table does not do (opening for writing, a descriptor it never handed out, a directory-relative open) stops the engine
// by name (HostFilesUnsupported: the message says what was asked, for the page to show) instead of answering something that looks like success. A descriptor below 3 goes to the glue's own function
// (fd_write on 1 and 2 is how the engine's stdout and stderr reach the page).
// The system-call ABI is that of the Emscripten that linked the glue: SUPPORTED_EMSCRIPTEN lists the versions this file was written and tested for (the manifest names the one that linked the bundle),
// and unsupportedImports() lists what the engine wasm imports from the system-call layer that this file neither handles nor knows to be harmless.

import type { PackFormat } from '../bundle/pack.ts';
import { isRelativePackPath } from '../bundle/schema.ts';

export interface ByteSource {
  readonly size: number;
  /** copies up to dst.length bytes starting at `at` into dst, synchronously; returns the number of bytes copied (0 at the end). dst may be a view of the engine's (shared) memory. */
  readInto(dst: Uint8Array, at: number): number;
  close?(): void;
}

export class MemorySource implements ByteSource {
  bytes: Uint8Array;
  private readonly length: number;
  constructor(bytes: Uint8Array) { this.bytes = bytes; this.length = bytes.length; }
  get size(): number { return this.length; }
  /** lets go of the bytes (the pack can be freed); a later read returns 0 bytes */
  close(): void { this.bytes = new Uint8Array(0); }
  readInto(dst: Uint8Array, at: number): number {
    if (at >= this.bytes.length) return 0;
    const n = Math.min(dst.length, this.bytes.length - at);
    dst.set(this.bytes.subarray(at, at + n));
    return n;
  }
}

/** where a member of a pack is: `offset` counts from the start of the pack's source */
export interface PackMember { offset: number; size: number; sha256: string }
export interface HostPack { source: ByteSource; format: PackFormat; members: ReadonlyMap<string, PackMember> }

const EACCES = 2, EBADF = 8, EINVAL = 28, EISDIR = 31, ENOENT = 44, EMFILE = 33, ESPIPE = 70, ENOTDIR = 54;
const O_ACCMODE = 3, O_CREAT = 0o100, O_TRUNC = 0o1000, O_DIRECTORY = 0o200000;
const AT_FDCWD = -100;
const S_IFDIR = 0o040000, S_IFREG = 0o100000;
const FIRST_FD = 3, MAX_FDS = 1024;

/** the Emscripten versions whose system-call ABI (struct stat layout, errno numbers, import names) this file implements */
export const SUPPORTED_EMSCRIPTEN: readonly string[] = ['6.0.12'];
/** the imports this file replaces */
export const HANDLED_IMPORTS: readonly string[] = ['__syscall_openat', '__syscall_stat64', '__syscall_lstat64', '__syscall_newfstatat', '__syscall_fstat64', 'fd_read', 'fd_pread', 'fd_seek', 'fd_close'];
/** system-call imports that the glue's own function answers and that touch no host file (stdout / stderr, fcntl and ioctl on a descriptor of a stream, the environment, the clock, exit) */
const HARMLESS_IMPORTS: readonly string[] = ['fd_write', '__syscall_fcntl64', '__syscall_ioctl', 'proc_exit', 'clock_time_get', 'environ_sizes_get', 'environ_get'];

/** the imports of the engine wasm from the system-call layer (__syscall_*, fd_*, path_*, sock_*) that are neither handled nor known to be harmless: a bundle that has any is refused by name */
export function unsupportedImports(module: WebAssembly.Module): string[] {
  const out: string[] = [];
  for (const i of WebAssembly.Module.imports(module)) {
    if (i.kind !== 'function' || !/^(__syscall_|fd_|path_|sock_|poll_)/.test(i.name)) continue;
    if (!HANDLED_IMPORTS.includes(i.name) && !HARMLESS_IMPORTS.includes(i.name)) out.push(i.name);
  }
  return out;
}

export class HostFilesUnsupported extends Error {
  constructor(what: string) { super(`The engine asked its host files for something they do not do: ${what}. The host files are the read-only packs of the bundle (app/src/engine/hostfiles.ts).`); this.name = 'HostFilesUnsupported'; }
}

interface HostFile { source: ByteSource; offset: number; size: number; ino: number }
interface OpenFile { file: HostFile; pos: number }

export interface HostFileStats {
  opens: number; failedOpens: number; stats: number; reads: number; bytes: number; closes: number; seeks: number;
  /** milliseconds spent inside the replaced system calls (the copy of the bytes included) */
  callMs: number;
  /** performance.now() of the first and the last call (null: none yet) */
  firstAt: number | null; lastAt: number | null;
  /** distinct files opened */
  filesOpened: number;
}

type Fn = (...a: never[]) => unknown;

export class HostFiles {
  private readonly files = new Map<string, HostFile>();
  private readonly dirs = new Set<string>(['/']);
  private readonly fds: (OpenFile | undefined)[] = [];
  private readonly opened = new Set<string>();
  private readonly memory: () => WebAssembly.Memory | null;
  private buf: ArrayBufferLike | null = null;
  private u8: Uint8Array = new Uint8Array(0);
  private dv: DataView = new DataView(new ArrayBuffer(0));
  private nextIno = 1;
  readonly stats: HostFileStats = { opens: 0, failedOpens: 0, stats: 0, reads: 0, bytes: 0, closes: 0, seeks: 0, callMs: 0, firstAt: null, lastAt: null, filesOpened: 0 };

  constructor(memory: () => WebAssembly.Memory | null) { this.memory = memory; }

  get fileCount(): number { return this.files.size; }
  has(path: string): boolean { return this.files.has(normalize(path)); }

  /** makes every member of the pack visible as `root/<member path>` */
  mountPack(root: string, pack: HostPack): { files: number; bytes: number } {
    let n = 0, bytes = 0;
    for (const [p, m] of pack.members) {
      this.add(`${root}/${p}`, pack.source, m.offset, m.size);
      n++;
      bytes += m.size;
    }
    return { files: n, bytes };
  }

  add(path: string, source: ByteSource, offset: number, size: number): void {
    const p = normalize(path);
    if (!p.startsWith('/') || p === '/') throw new Error(`host file path ${JSON.stringify(path)} must be absolute`);
    if (this.files.has(p)) throw new Error(`host file ${p} is added twice`);
    if (offset < 0 || size < 0 || offset + size > source.size) throw new Error(`host file ${p}: ${size} bytes at ${offset} run past the end of the source (${source.size} bytes)`);
    this.files.set(p, { source, offset, size, ino: this.nextIno++ });
    for (let i = p.indexOf('/', 1); i > 0; i = p.indexOf('/', i + 1)) this.dirs.add(p.slice(0, i));
  }

  private released = false;
  private readonly knownPaths = new Set<string>();

  /**
   * The engine has read what it reads at start-up (decision 0221: all of it happens before the first idle; nothing after): closes the sources, so that the bytes of the packs are freed in the Worker
   * (a stored copy is closed, which also gives its access handle back to other tabs). The names stay: an open of one of them from now on stops the engine by name instead of finding nothing.
   */
  release(): void {
    if (this.released) return;
    for (const p of this.files.keys()) this.knownPaths.add(p);
    this.released = true;
    this.close();
  }
  get isReleased(): boolean { return this.released; }

  /** forgets every file and closes the sources (the pack bytes can be freed); later opens fail with ENOENT */
  close(): void {
    const sources = new Set<ByteSource>();
    for (const f of this.files.values()) sources.add(f.source);
    this.files.clear();
    this.dirs.clear();
    this.dirs.add('/');
    this.fds.length = 0;
    for (const s of sources) s.close?.();
  }

  private touch(): void { const t = performance.now(); if (this.stats.firstAt === null) this.stats.firstAt = t; this.stats.lastAt = t; }

  private heap(): { u8: Uint8Array; dv: DataView } {
    const mem = this.memory();
    if (!mem) throw new Error('the engine memory is not there yet');
    if (mem.buffer !== this.buf) { this.buf = mem.buffer; this.u8 = new Uint8Array(mem.buffer); this.dv = new DataView(mem.buffer); }
    return { u8: this.u8, dv: this.dv };
  }

  private cstr(ptr: number): string {
    const { u8 } = this.heap();
    let end = ptr;
    while (u8[end] !== 0) end++;
    let ascii = true;
    for (let i = ptr; i < end; i++) if ((u8[i] as number) >= 0x80) { ascii = false; break; }
    if (ascii) { let s = ''; for (let i = ptr; i < end; i++) s += String.fromCharCode(u8[i] as number); return s; }
    return new TextDecoder().decode(u8.slice(ptr, end));   // a copy: TextDecoder refuses a view of shared memory
  }

  private lookup(path: string): { kind: 'file'; file: HostFile } | { kind: 'dir' } | null {
    const p = normalize(path);
    const f = this.files.get(p);
    if (f) return { kind: 'file', file: f };
    if (this.dirs.has(p)) return { kind: 'dir' };
    return null;
  }

  private writeStat(buf: number, mode: number, size: number, ino: number): void {
    const { dv } = this.heap();
    dv.setUint32(buf, 1, true);                       // st_dev
    dv.setUint32(buf + 4, mode, true);
    dv.setUint32(buf + 8, 1, true);                   // nlink
    dv.setUint32(buf + 12, 0, true);                  // uid
    dv.setUint32(buf + 16, 0, true);                  // gid
    dv.setUint32(buf + 20, 0, true);                  // rdev
    dv.setBigInt64(buf + 24, BigInt(size), true);
    dv.setInt32(buf + 32, 4096, true);                // blksize
    dv.setInt32(buf + 36, Math.ceil(size / 512), true);   // blocks
    for (const o of [40, 56, 72]) { dv.setBigInt64(buf + o, 0n, true); dv.setUint32(buf + o + 8, 0, true); }   // atime, mtime, ctime: not known
    dv.setBigInt64(buf + 88, BigInt(ino), true);
  }

  private statPath(path: string, buf: number): number {
    this.touch();
    this.stats.stats++;
    const r = this.lookup(path);
    if (!r) return -ENOENT;
    if (r.kind === 'file') this.writeStat(buf, S_IFREG | 0o444, r.file.size, r.file.ino);
    else this.writeStat(buf, S_IFDIR | 0o555, 0, 0);
    return 0;
  }

  private openat(dirfd: number, pathPtr: number, flags: number): number {
    this.touch();
    const path = this.cstr(pathPtr);
    if (!path.startsWith('/') && dirfd !== AT_FDCWD) { this.stats.failedOpens++; throw new HostFilesUnsupported(`opening ${path} relative to the directory descriptor ${dirfd}`); }
    if (this.released && this.knownPaths.has(normalize(path))) { this.stats.failedOpens++; throw new HostFilesUnsupported(`opening ${path} after the host files were released (at the engine's first idle the packs are let go)`); }
    const r = this.lookup(path);
    if ((flags & O_ACCMODE) !== 0 || (flags & (O_CREAT | O_TRUNC)) !== 0) { this.stats.failedOpens++; throw new HostFilesUnsupported(`opening ${path} for writing, creating or truncating (open flags 0x${flags.toString(16)})`); }
    if (!r) { this.stats.failedOpens++; return -ENOENT; }
    if (r.kind === 'dir') { this.stats.failedOpens++; return (flags & O_DIRECTORY) !== 0 ? -EACCES : -EISDIR; }
    if ((flags & O_DIRECTORY) !== 0) { this.stats.failedOpens++; return -ENOTDIR; }
    let fd = FIRST_FD;
    while (this.fds[fd] !== undefined) fd++;
    if (fd >= MAX_FDS) { this.stats.failedOpens++; return -EMFILE; }
    this.fds[fd] = { file: r.file, pos: 0 };
    this.stats.opens++;
    this.opened.add(normalize(path));
    this.stats.filesOpened = this.opened.size;
    return fd;
  }

  /** copies into the iov list from `at` (or from the descriptor's position when at is null); returns the byte count or a negative errno */
  private readv(of: OpenFile, iov: number, iovcnt: number, at: number | null): number {
    const { u8, dv } = this.heap();
    let pos = at ?? of.pos, total = 0;
    const f = of.file;
    for (let i = 0; i < iovcnt; i++) {
      const ptr = dv.getUint32(iov + 8 * i, true), len = dv.getUint32(iov + 8 * i + 4, true);
      if (len === 0) continue;
      const left = f.size - pos;
      if (left <= 0) break;
      const want = Math.min(len, left);
      const got = f.source.readInto(u8.subarray(ptr, ptr + want), f.offset + pos);
      pos += got;
      total += got;
      if (got < len) break;
    }
    this.stats.reads++;
    this.stats.bytes += total;
    if (at === null) of.pos = pos;
    return total;
  }

  /** the functions to put over the glue's imports: `orig` is the imports object the glue made (its functions are called for descriptors this table does not own) */
  overrides(orig: Record<string, unknown>): Record<string, Fn> {
    const o = (name: string): ((...a: unknown[]) => unknown) | undefined => (typeof orig[name] === 'function' ? (orig[name] as (...a: unknown[]) => unknown) : undefined);
    // a descriptor below 3 is the glue's (stdin, stdout, stderr); a higher one that this table did not hand out (or has closed) is a bug of the caller: stop by name
    const owned = (fd: number, what: string): OpenFile | undefined => {
      if (fd < FIRST_FD) return undefined;
      const of = this.fds[fd];
      if (!of) throw new HostFilesUnsupported(`${what} on descriptor ${fd}, which was never opened or has been closed`);
      return of;
    };
    const origRead = o('fd_read'), origPread = o('fd_pread'), origSeek = o('fd_seek'), origClose = o('fd_close'), origFstat = o('__syscall_fstat64');
    const fns: Record<string, (...a: never[]) => unknown> = {
      __syscall_openat: (dirfd: number, path: number, flags: number) => this.openat(dirfd, path, flags),
      __syscall_stat64: (path: number, buf: number) => this.statPath(this.cstr(path), buf),
      __syscall_lstat64: (path: number, buf: number) => this.statPath(this.cstr(path), buf),
      __syscall_newfstatat: (dirfd: number, path: number, buf: number, flags: number) => {
        const p = this.cstr(path);
        if (p === '' && (flags & 0x1000) !== 0) {   // AT_EMPTY_PATH: stat of the descriptor
          const of = owned(dirfd, 'fstatat(AT_EMPTY_PATH)');
          if (of) { this.writeStat(buf, S_IFREG | 0o444, of.file.size, of.file.ino); return 0; }
          return -EBADF;
        }
        return this.statPath(p, buf);
      },
      __syscall_fstat64: (fd: number, buf: number) => {
        const of = owned(fd, 'fstat');
        if (!of) return origFstat ? origFstat(fd, buf) : -EBADF;
        this.touch();
        this.writeStat(buf, S_IFREG | 0o444, of.file.size, of.file.ino);
        return 0;
      },
      fd_read: (fd: number, iov: number, iovcnt: number, pnum: number) => {
        const of = owned(fd, 'read');
        if (!of) return origRead ? origRead(fd, iov, iovcnt, pnum) : EBADF;
        this.touch();
        const n = this.readv(of, iov, iovcnt, null);
        this.heap().dv.setUint32(pnum, n, true);
        return 0;
      },
      fd_pread: (fd: number, iov: number, iovcnt: number, offset: bigint, pnum: number) => {
        const of = owned(fd, 'pread');
        if (!of) return origPread ? origPread(fd, iov, iovcnt, offset, pnum) : EBADF;
        if (offset < 0n) return EINVAL;
        this.touch();
        const n = this.readv(of, iov, iovcnt, Number(offset));
        this.heap().dv.setUint32(pnum, n, true);
        return 0;
      },
      fd_seek: (fd: number, offset: bigint, whence: number, pnew: number) => {
        const of = owned(fd, 'seek');
        if (!of) return origSeek ? origSeek(fd, offset, whence, pnew) : ESPIPE;
        this.touch();
        this.stats.seeks++;
        const base = whence === 0 ? 0 : whence === 1 ? of.pos : whence === 2 ? of.file.size : -1;
        if (base < 0) return EINVAL;
        const np = base + Number(offset);
        if (np < 0) return EINVAL;
        of.pos = np;
        this.heap().dv.setBigInt64(pnew, BigInt(np), true);
        return 0;
      },
      fd_close: (fd: number) => {
        const of = owned(fd, 'close');
        if (!of) return origClose ? origClose(fd) : EBADF;
        this.touch();
        this.stats.closes++;
        this.fds[fd] = undefined;
        return 0;
      },
    };
    for (const k of Object.keys(fns)) {   // the time inside each call is counted (decision 0221: is the replacement slow in some browser?)
      const fn = fns[k] as (...a: unknown[]) => unknown;
      fns[k] = ((...a: unknown[]) => { const t = performance.now(); try { return fn(...a); } finally { this.stats.callMs += performance.now() - t; } }) as (...a: never[]) => unknown;
    }
    return fns;
  }
}

function normalize(p: string): string {
  if (p.indexOf('//') < 0 && !p.endsWith('/')) return p;
  const s = p.replace(/\/{2,}/g, '/');
  return s.length > 1 && s.endsWith('/') ? s.slice(0, -1) : s;
}

/** Puts the overrides over an imports object of the glue (its `env` and its `wasi_snapshot_preview1` are one object in the glue of emsdk 6.0.12; both are handled in any case). */
export function installHostFiles(imports: WebAssembly.Imports, files: HostFiles): void {
  const seen = new Set<object>();
  for (const k of ['env', 'wasi_snapshot_preview1']) {
    const ns = imports[k] as Record<string, unknown> | undefined;
    if (!ns || seen.has(ns)) continue;
    seen.add(ns);
    Object.assign(ns, files.overrides(ns));
  }
}

export class HostPackError extends Error {}

/** Reads the index of a flpack1 / flseedp1 pack through its source (the first 12 bytes and the index text only: the members are not read). Offsets in the result count from the start of the source. */
export function readPackIndex(source: ByteSource): { format: PackFormat; members: Map<string, PackMember> } {
  const head = new Uint8Array(12);
  if (source.readInto(head, 0) < 12) throw new HostPackError(`pack too short (${source.size} bytes)`);
  const dv = new DataView(head.buffer);
  const magic = new TextDecoder().decode(head.subarray(0, 8));
  let format: PackFormat;
  if (magic === 'FLSEEDP1') format = 'flseedp1';
  else if (magic.startsWith('FLPK')) {
    if (dv.getUint32(4, true) !== 1) throw new HostPackError(`unsupported pack version ${dv.getUint32(4, true)}`);
    format = 'flpack1';
  } else throw new HostPackError(`bad magic ${JSON.stringify(magic)}`);
  const hl = dv.getUint32(8, true);
  if (12 + hl > source.size) throw new HostPackError('the index runs past the end of the pack');
  const text = new Uint8Array(hl);
  if (source.readInto(text, 12) < hl) throw new HostPackError('the index could not be read whole');
  let idx: { version?: unknown; files?: unknown };
  try { idx = JSON.parse(new TextDecoder().decode(text)) as typeof idx; } catch (e) { throw new HostPackError(`the index is not JSON: ${String(e)}`); }
  if (format === 'flseedp1' && idx.version !== 1) throw new HostPackError(`unsupported flseedp1 version ${String(idx.version)}`);
  if (!Array.isArray(idx.files)) throw new HostPackError('the index has no "files" array');
  const base = 12 + hl;
  const members = new Map<string, PackMember>();
  for (const e of idx.files as Record<string, unknown>[]) {
    const { path, offset, size, sha256 } = e as { path: unknown; offset: unknown; size: unknown; sha256: unknown };
    if (typeof path !== 'string' || !isRelativePackPath(path)) throw new HostPackError(`bad path ${JSON.stringify(path)}`);
    if (!Number.isInteger(offset) || !Number.isInteger(size) || (offset as number) < 0 || (size as number) < 0) throw new HostPackError(`${path}: bad offset/size`);
    if (base + (offset as number) + (size as number) > source.size) throw new HostPackError(`${path}: runs past the end of the pack`);
    if (typeof sha256 !== 'string' || !/^[0-9a-f]{64}$/.test(sha256)) throw new HostPackError(`${path}: bad sha256`);
    if (members.has(path)) throw new HostPackError(`duplicate path ${path}`);
    members.set(path, { offset: base + (offset as number), size: size as number, sha256 });
  }
  return { format, members };
}

export function hostPackFromSource(source: ByteSource): HostPack {
  const { format, members } = readPackIndex(source);
  return { source, format, members };
}

export function hostPackFromBytes(bytes: Uint8Array): HostPack { return hostPackFromSource(new MemorySource(bytes)); }
