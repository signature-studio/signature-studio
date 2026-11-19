import { describe, expect, it } from 'vitest';
import type { Manifest } from '../../src/bundle/manifest.ts';
import { buildPack } from '../../src/bundle/pack.ts';
import { FruityEngine, type CreateModule, type FruityAssets, type FruityHost } from '../../src/engine/fruity.ts';
import { HostFiles, HostFilesUnsupported, hostPackFromBytes, hostPackFromSource, installHostFiles, MemorySource, SUPPORTED_EMSCRIPTEN, unsupportedImports, type ByteSource } from '../../src/engine/hostfiles.ts';

// The libc side is played by hand: the arguments are what Emscripten's musl passes to the imports (pointers into the engine's memory, offsets as BigInt).
const ENOENT = 44, EISDIR = 31, EINVAL = 28;

function rig(extra: Array<[string, Uint8Array]> = []) {
  const mem = new WebAssembly.Memory({ initial: 4 });
  const files = new HostFiles(() => mem);
  const dll = hostPackFromBytes(buildPack([{ path: 'FLEngine_x64.dll', data: Uint8Array.from({ length: 5000 }, (_, i) => i % 251) }, { path: 'm1.dll', data: new Uint8Array([8, 7, 6]) }]));
  const seed = hostPackFromBytes(buildPack([{ path: 'seed/a.tsv', data: new TextEncoder().encode('one\ntwo\nthree') }, { path: 'seed/x/y.bin', data: new Uint8Array([3]) }, { path: 'seed/empty', data: new Uint8Array(0) }, ...extra.map(([path, data]) => ({ path, data }))]));
  files.mountPack('/data', dll);
  files.mountPack('/data', seed);
  const orig: Record<string, unknown> = { fd_write: () => 0, fd_read: () => 52, fd_pread: () => 52, fd_seek: () => 70, fd_close: () => 52, __syscall_fstat64: () => 0 };
  const imports = { env: orig, wasi_snapshot_preview1: orig } as unknown as WebAssembly.Imports;
  installHostFiles(imports, files);
  const sc = orig as Record<string, (...a: unknown[]) => number>;
  const u8 = new Uint8Array(mem.buffer), dv = new DataView(mem.buffer);
  const str = (at: number, s: string) => { u8.set(new TextEncoder().encode(s + '\0'), at); return at; };
  const iov = (at: number, list: Array<[number, number]>) => { list.forEach(([p, l], i) => { dv.setUint32(at + 8 * i, p, true); dv.setUint32(at + 8 * i + 4, l, true); }); return at; };
  return { mem, files, sc, u8, dv, str, iov, orig };
}

describe('HostFiles: the libc system calls over the pack bytes', () => {
  it('openat + fd_read: reads a file in pieces, an iov list at a time, then 0 at the end', () => {
    const r = rig();
    const fd = r.sc.__syscall_openat!(-100, r.str(100, '/data/seed/seed/a.tsv'.replace('/seed/seed', '/seed')), 0, 0)!;
    expect(fd).toBeGreaterThanOrEqual(3);
    expect(r.sc.fd_read!(fd, r.iov(200, [[300, 4], [400, 100]]), 2, 600)).toBe(0);
    expect(r.dv.getUint32(600, true)).toBe(13);
    expect(new TextDecoder().decode(r.u8.subarray(300, 304))).toBe('one\n');
    expect(new TextDecoder().decode(r.u8.subarray(400, 409))).toBe('two\nthree');
    expect(r.sc.fd_read!(fd, r.iov(200, [[300, 4]]), 1, 600)).toBe(0);
    expect(r.dv.getUint32(600, true)).toBe(0);
    expect(r.sc.fd_close!(fd)).toBe(0);
    expect(() => r.sc.fd_read!(fd, r.iov(200, [[300, 4]]), 1, 600)).toThrow(HostFilesUnsupported);   // closed: a bug of the caller, stopped by name
  });

  it('fd_pread reads at an offset without moving the position; fd_seek whence 0/1/2', () => {
    const r = rig();
    const fd = r.sc.__syscall_openat!(-100, r.str(100, '/data/FLEngine_x64.dll'), 0, 0)!;
    expect(r.sc.fd_pread!(fd, r.iov(200, [[1000, 6]]), 1, 4994n, 600)).toBe(0);
    expect(r.dv.getUint32(600, true)).toBe(6);                 // only 6 bytes are left of the 5000
    expect([...r.u8.subarray(1000, 1006)]).toEqual([4994, 4995, 4996, 4997, 4998, 4999].map((i) => i % 251));
    expect(r.sc.fd_read!(fd, r.iov(200, [[1000, 3]]), 1, 600)).toBe(0);
    expect([...r.u8.subarray(1000, 1003)]).toEqual([0, 1, 2]);   // the position was still 0
    expect(r.sc.fd_seek!(fd, -2n, 2, 700)).toBe(0);
    expect(r.dv.getBigInt64(700, true)).toBe(4998n);
    expect(r.sc.fd_seek!(fd, 1n, 1, 700)).toBe(0);
    expect(r.dv.getBigInt64(700, true)).toBe(4999n);
    expect(r.sc.fd_seek!(fd, -1n, 0, 700)).toBe(EINVAL);
    expect(r.sc.fd_seek!(fd, 0n, 7, 700)).toBe(EINVAL);
  });

  it('stat64 / lstat64 / newfstatat / fstat64: a regular file read-only, the directories above it, ENOENT for the rest', () => {
    const r = rig();
    expect(r.sc.__syscall_stat64!(r.str(100, '/data/m1.dll'), 800)).toBe(0);
    expect(r.dv.getUint32(804, true)).toBe(0o100444);
    expect(r.dv.getBigInt64(824, true)).toBe(3n);
    expect(r.dv.getInt32(832, true)).toBe(4096);
    expect(r.sc.__syscall_lstat64!(r.str(100, '/data/seed'), 800)).toBe(0);
    expect(r.dv.getUint32(804, true)).toBe(0o040555);
    expect(r.sc.__syscall_stat64!(r.str(100, '/data//seed/'), 800)).toBe(0);   // doubled and trailing slashes are the same directory
    expect(r.sc.__syscall_newfstatat!(-100, r.str(100, '/data/seed/seed/empty'.replace('/seed/seed', '/seed')), 800, 0)).toBe(0);
    expect(r.dv.getBigInt64(824, true)).toBe(0n);
    expect(r.sc.__syscall_stat64!(r.str(100, '/data/nothing'), 800)).toBe(-ENOENT);
    expect(r.sc.__syscall_stat64!(r.str(100, '/etc/passwd'), 800)).toBe(-ENOENT);
    const fd = r.sc.__syscall_openat!(-100, r.str(100, '/data/FLEngine_x64.dll'), 0, 0)!;
    expect(r.sc.__syscall_fstat64!(fd, 800)).toBe(0);
    expect(r.dv.getBigInt64(824, true)).toBe(5000n);
    expect(r.sc.__syscall_fstat64!(1, 800)).toBe(0);   // the glue's function (stdout)
  });

  it('stops by name when the engine writes: opening for writing, creating or truncating throws HostFilesUnsupported with a readable first line', () => {
    const r = rig();
    for (const flags of [1, 2, 0o101, 0o1000]) {
      let err: unknown = null;
      try { r.sc.__syscall_openat!(-100, r.str(100, '/data/m1.dll'), flags, 0); } catch (e) { err = e; }
      expect(err, `flags ${flags}`).toBeInstanceOf(HostFilesUnsupported);
      expect((err as Error).message).toMatch(/^The engine asked its host files for something they do not do: opening \/data\/m1\.dll for writing/);
    }
    expect(r.files.stats.opens).toBe(0);
  });

  it('answers ENOENT for a missing path and EISDIR for a directory (ordinary answers, not stops)', () => {
    const r = rig();
    expect(r.sc.__syscall_openat!(-100, r.str(100, '/data/seed'), 0, 0)).toBe(-EISDIR);
    expect(r.sc.__syscall_openat!(-100, r.str(100, '/data/gone'), 0, 0)).toBe(-ENOENT);
    expect(r.files.stats.opens).toBe(0);
    expect(r.files.stats.failedOpens).toBe(2);
  });

  it('stops by name on a descriptor it never handed out or has closed, and on an open relative to a directory descriptor', () => {
    const r = rig();
    expect(() => r.sc.fd_read!(7, r.iov(200, [[300, 4]]), 1, 600)).toThrow(/read on descriptor 7, which was never opened or has been closed/);
    expect(() => r.sc.fd_seek!(7, 0n, 0, 700)).toThrow(HostFilesUnsupported);
    expect(() => r.sc.fd_close!(7)).toThrow(HostFilesUnsupported);
    expect(() => r.sc.__syscall_openat!(5, r.str(100, 'rel/file'), 0, 0)).toThrow(/relative to the directory descriptor 5/);
    const fd = r.sc.__syscall_openat!(-100, r.str(100, '/data/m1.dll'), 0, 0)!;
    r.sc.fd_close!(fd);
    expect(() => r.sc.fd_close!(fd)).toThrow(HostFilesUnsupported);
  });

  it('descriptors are small numbers from 3, the lowest free one is reused, and unknown ones fall through to the glue', () => {
    const r = rig();
    const a = r.sc.__syscall_openat!(-100, r.str(100, '/data/m1.dll'), 0, 0)!;
    const b = r.sc.__syscall_openat!(-100, r.str(100, '/data/m1.dll'), 0, 0)!;
    expect([a, b]).toEqual([3, 4]);
    r.sc.fd_close!(a);
    expect(r.sc.__syscall_openat!(-100, r.str(100, '/data/m1.dll'), 0, 0)).toBe(3);
    expect(r.sc.fd_read!(1, r.iov(200, [[300, 1]]), 1, 600)).toBe(52);   // stdin / stdout / stderr: the glue's own function
    expect(r.sc.fd_close!(2)).toBe(52);
  });

  it('counts what the engine asked for', () => {
    const r = rig();
    const fd = r.sc.__syscall_openat!(-100, r.str(100, '/data/FLEngine_x64.dll'), 0, 0)!;
    r.sc.fd_pread!(fd, r.iov(200, [[1000, 4096]]), 1, 0n, 600);
    r.sc.fd_close!(fd);
    r.sc.__syscall_stat64!(r.str(100, '/data/m1.dll'), 800);
    expect(r.files.stats).toMatchObject({ opens: 1, closes: 1, stats: 1, reads: 1, bytes: 4096, filesOpened: 1 });
    expect(r.files.stats.firstAt).not.toBeNull();
  });

  it('close() forgets the files and closes the sources', () => {
    const r = rig();
    let closed = 0;
    const src: ByteSource = { size: 3, readInto: () => 0, close: () => { closed++; } };
    r.files.add('/extra/one', src, 0, 3);
    r.files.close();
    expect(closed).toBe(1);
    expect(r.sc.__syscall_stat64!(r.str(100, '/data/m1.dll'), 800)).toBe(-ENOENT);
  });

  it('close() makes later opens ENOENT', () => {
    const r = rig();
    r.files.close();
    expect(r.sc.__syscall_openat!(-100, r.str(100, '/data/m1.dll'), 0, 0)).toBe(-ENOENT);
  });

  it('release() lets go of the pack bytes; opening a file that was there then stops by name, a path that never was is still ENOENT', () => {
    const r = rig();
    const src = (r.files as unknown as { files: Map<string, { source: MemorySource }> }).files.get('/data/FLEngine_x64.dll')!.source;
    expect(src.bytes.length).toBeGreaterThan(0);
    r.files.release();
    expect(r.files.isReleased).toBe(true);
    expect(src.bytes.length).toBe(0);
    expect(() => r.sc.__syscall_openat!(-100, r.str(100, '/data/FLEngine_x64.dll'), 0, 0)).toThrow(/after the host files were released/);
    expect(r.sc.__syscall_openat!(-100, r.str(100, '/data/never'), 0, 0)).toBe(-ENOENT);
    r.files.release();   // twice is the same
  });

  it('refuses a member that runs past its source, a relative path, and a path added twice', () => {
    const files = new HostFiles(() => null);
    const src = new MemorySource(new Uint8Array(10));
    expect(() => files.add('/a/b', src, 5, 6)).toThrow(/past the end/);
    expect(() => files.add('a/b', src, 0, 1)).toThrow(/absolute/);
    files.add('/a/b', src, 0, 1);
    expect(() => files.add('/a/b', src, 0, 1)).toThrow(/twice/);
  });

  it('a path with non-ASCII characters is decoded as UTF-8', () => {
    const r = rig([['seed/\u00e9.txt', new Uint8Array([1])]]);
    expect(r.sc.__syscall_stat64!(r.str(100, '/data/seed/\u00e9.txt'), 800)).toBe(0);
  });

  it('reading through a source that returns short counts stops at the short count', () => {
    const mem = new WebAssembly.Memory({ initial: 1 });
    const files = new HostFiles(() => mem);
    const src: ByteSource = { size: 10, readInto: (dst, at) => { const n = Math.min(dst.length, 3, 10 - at); dst.fill(0x41 + at, 0, n); return n; } };
    files.add('/f', src, 0, 10);
    const orig: Record<string, unknown> = {};
    installHostFiles({ env: orig } as unknown as WebAssembly.Imports, files);
    const sc = orig as Record<string, (...a: unknown[]) => number>;
    const dv = new DataView(mem.buffer);
    new Uint8Array(mem.buffer).set(new TextEncoder().encode('/f\0'), 100);
    const fd = sc.__syscall_openat!(-100, 100, 0, 0)!;
    dv.setUint32(200, 300, true); dv.setUint32(204, 8, true);
    sc.fd_read!(fd, 200, 1, 600);
    expect(dv.getUint32(600, true)).toBe(3);
  });
});

describe('readPackIndex / hostPackFromBytes', () => {
  const pack = buildPack([{ path: 'a/b.bin', data: new Uint8Array([1, 2, 3]) }, { path: 'c.bin', data: new Uint8Array(20) }]);
  it('gives the members of a flpack1 with offsets from the start of the pack', () => {
    const hp = hostPackFromBytes(pack);
    expect(hp.format).toBe('flpack1');
    expect([...hp.members.keys()]).toEqual(['a/b.bin', 'c.bin']);
    const m = hp.members.get('a/b.bin')!;
    expect([...pack.subarray(m.offset, m.offset + m.size)]).toEqual([1, 2, 3]);
  });
  it('reads only the 12 bytes and the index of the source, not the members', () => {
    const reads: Array<[number, number]> = [];
    const src: ByteSource = { size: pack.length, readInto: (dst, at) => { reads.push([at, dst.length]); return new MemorySource(pack).readInto(dst, at); } };
    hostPackFromSource(src);
    expect(reads.length).toBe(2);
    expect(reads[0]).toEqual([0, 12]);
    expect(reads[1]![0]).toBe(12);
  });
  it('reads a flseedp1 (header version 1, offsets from the data region)', () => {
    const hdr = new TextEncoder().encode(JSON.stringify({ version: 1, root: 'seed', count: 1, data_bytes: 2, files: [{ path: 'seed/q', size: 2, offset: 0, sha256: 'b'.repeat(64) }] }));
    const bytes = new Uint8Array(12 + hdr.length + 2);
    bytes.set(new TextEncoder().encode('FLSEEDP1'), 0);
    new DataView(bytes.buffer).setUint32(8, hdr.length, true);
    bytes.set(hdr, 12);
    bytes.set([7, 9], 12 + hdr.length);
    const hp = hostPackFromBytes(bytes);
    expect(hp.format).toBe('flseedp1');
    const m = hp.members.get('seed/q')!;
    expect([...bytes.subarray(m.offset, m.offset + m.size)]).toEqual([7, 9]);
  });
  it('refuses a bad magic, a short pack, an index past the end and a member past the end', () => {
    expect(() => hostPackFromBytes(new Uint8Array(4))).toThrow(/too short/);
    expect(() => hostPackFromBytes(new Uint8Array(40))).toThrow(/bad magic/);
    const cut = pack.subarray(0, 30);
    expect(() => hostPackFromBytes(cut)).toThrow(/past the end/);
    expect(() => hostPackFromBytes(pack.subarray(0, pack.length - 5))).toThrow(/runs past the end/);
  });
});

// ---- the engine adapter with a bundle of the current format ----
const MIN_ENGINE = new Uint8Array([
  0, 0x61, 0x73, 0x6d, 1, 0, 0, 0,
  1, 5, 1, 0x60, 0, 1, 0x7f, 3, 2, 1, 0, 5, 3, 1, 0, 2,
  7, 0x1e, 2, 6, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 2, 0, 0x11, 0x66, 0x6c, 0x65, 0x5f, 0x73, 0x75, 0x72, 0x66, 0x61, 0x63, 0x65, 0x5f, 0x74, 0x61, 0x62, 0x6c, 0x65, 0, 0,
  10, 7, 1, 5, 0, 0x41, 0x80, 0x01, 0x0b,
]);
const sha = 'a'.repeat(64);
const newFormat = (): Manifest => ({
  schema: 1, bundle: { name: 'fl', version: '1' },
  engine: {
    kind: 'emscripten-fruity-v1', wasm: { url: '/e.wasm', size: 1, sha256: sha, compression: 'identity' }, args: ['/data/FLEngine_x64.dll'],
    env: { FLE_VFS_SEED_ROOT: '/data/seed' },
    options: { glue: { url: '/g.js', size: 1, sha256: sha, compression: 'identity' }, root: '/data', hostFilesAbi: 'emscripten-6.0.12' },
  },
  seedPack: { url: '/s', size: 1, sha256: sha, compression: 'identity', format: 'flseedp1' }, dllPack: { url: '/d', size: 1, sha256: sha, compression: 'identity', format: 'flpack1' },
});

describe('FruityEngine with host files', () => {
  it('needs no FS: the packs are mounted under root, the imports are replaced before instantiation, the environment is set, and the stats are reported', async () => {
    const dll = hostPackFromBytes(buildPack([{ path: 'FLEngine_x64.dll', data: new Uint8Array([9, 8, 7, 6]) }]));
    const seed = hostPackFromBytes(buildPack([{ path: 'seed/l.tsv', data: new Uint8Array([1, 2]) }]));
    const assets: FruityAssets = { module: await WebAssembly.compile(MIN_ENGINE), glue: new Uint8Array([1]), seed, dll };
    const marks: Array<[string, Record<string, unknown> | undefined]> = [];
    const h: FruityHost = { mark: (n, d) => { marks.push([n, d]); }, stdout: () => {}, stderr: () => {}, log: () => {}, idle: () => {} };
    const env: Record<string, string> = {};
    let seenFd = -1, seenBytes = -1;
    const create: CreateModule = async (opts) => {
      opts.ENV = env;
      const glueFns = { fd_write: () => 0, fd_read: () => 52 };
      const imports = { env: glueFns, wasi_snapshot_preview1: glueFns } as unknown as WebAssembly.Imports;
      await new Promise<void>((res) => { opts.instantiateWasm(imports, () => res()); });
      for (const f of opts.preRun) f();
      opts.onRuntimeInitialized();
      // the engine's first act: open its DLL and read it
      const sc = glueFns as unknown as Record<string, (...a: unknown[]) => number>;
      const mem = eng.memory!;
      new Uint8Array(mem.buffer).set(new TextEncoder().encode('/data/FLEngine_x64.dll\0'), 64);
      seenFd = sc.__syscall_openat!(-100, 64, 0, 0)!;
      const dv = new DataView(mem.buffer);
      dv.setUint32(256, 512, true); dv.setUint32(260, 16, true);
      sc.fd_read!(seenFd, 256, 1, 300);
      seenBytes = dv.getUint32(300, true);
      expect([...new Uint8Array(mem.buffer, 512, 4)]).toEqual([9, 8, 7, 6]);
      opts.fleIdle(1, 5, 0);
      opts.onExit(0);
    };
    const eng = await FruityEngine.create(newFormat(), assets, h, async () => create);
    expect(await eng.run()).toEqual({ kind: 'exit', code: 0 });
    expect(seenFd).toBe(3);
    expect(seenBytes).toBe(4);
    expect(env).toEqual({ FLE_VFS_SEED_ROOT: '/data/seed' });
    const ready = marks.find(([n]) => n === 'files_ready')![1]!;
    expect(ready).toMatchObject({ how: 'packs-v1', dllFiles: 1, dllBytes: 4, seedFiles: 1, seedBytes: 2 });
    const idle = marks.find(([n]) => n === 'idle')![1]!;
    expect((idle.hostFiles as { opens: number; bytes: number })).toMatchObject({ opens: 1, bytes: 4 });
    expect(eng.hostFileStats).toMatchObject({ opens: 1, bytes: 4, filesOpened: 1 });
    expect(eng.hostFiles!.fileCount).toBe(0);   // the files are forgotten when the engine has ended
    expect(marks.map(([n]) => n)).toContain('host_files_released');   // the first idle lets the packs go
  });
});

describe('the system-call layer of the engine wasm', () => {
  // a module that imports the named functions (all with the type () -> ())
  const moduleImporting = (names: string[], ns = 'env'): WebAssembly.Module => {
    const enc = (s: string) => [s.length, ...new TextEncoder().encode(s)];
    const entries = names.flatMap((n) => [...enc(ns), ...enc(n), 0, 0]);
    const imp = [names.length, ...entries];
    return new WebAssembly.Module(new Uint8Array([0, 0x61, 0x73, 0x6d, 1, 0, 0, 0, 1, 4, 1, 0x60, 0, 0, 2, imp.length, ...imp]));
  };
  it('lists the system-call imports that are neither handled nor known to be harmless', () => {
    expect(unsupportedImports(moduleImporting(['__syscall_openat', 'fd_read', 'fd_write', '__syscall_ioctl', 'proc_exit', 'something_else']))).toEqual([]);
    expect(unsupportedImports(moduleImporting(['__syscall_openat', '__syscall_mkdirat', 'fd_fdstat_get', 'path_open']))).toEqual(['__syscall_mkdirat', 'fd_fdstat_get', 'path_open']);
  });
  it('names the Emscripten versions it is written for', () => {
    expect(SUPPORTED_EMSCRIPTEN).toContain('6.0.12');
  });
});

describe('FruityEngine: the packs are let go at the first idle', () => {
  it('released at the first idle, and a later open stops by name', async () => {
    const dll = hostPackFromBytes(buildPack([{ path: 'FLEngine_x64.dll', data: new Uint8Array([9, 8, 7, 6]) }]));
    const seed = hostPackFromBytes(buildPack([{ path: 'seed/l.tsv', data: new Uint8Array([1, 2]) }]));
    const marks: string[] = [];
    const h: FruityHost = { mark: (n) => { marks.push(n); }, stdout: () => {}, stderr: () => {}, log: () => {}, idle: () => {} };
    let after = '';
    const create: CreateModule = async (opts) => {
      opts.ENV = {};
      const fns = { fd_write: () => 0 };
      await new Promise<void>((res) => { opts.instantiateWasm({ env: fns, wasi_snapshot_preview1: fns } as unknown as WebAssembly.Imports, () => res()); });
      for (const f of opts.preRun) f();
      opts.fleIdle(1, 1, 0);
      const mem = eng.memory!;
      new Uint8Array(mem.buffer).set(new TextEncoder().encode('/data/FLEngine_x64.dll\0'), 64);
      try { after = String((fns as unknown as Record<string, (...a: unknown[]) => unknown>).__syscall_openat!(-100, 64, 0, 0)); } catch (e) { after = (e as Error).name; }
      opts.onExit(0);
    };
    const eng = await FruityEngine.create(newFormat(), { module: await WebAssembly.compile(MIN_ENGINE), glue: new Uint8Array([1]), seed, dll }, h, async () => create);
    await eng.run();
    expect(marks).toContain('host_files_released');
    expect(after).toBe('HostFilesUnsupported');
  });
});

describe('FruityEngine refuses a bundle whose file calls it cannot answer', () => {
  const host = (): FruityHost => ({ mark: () => {}, stdout: () => {}, stderr: () => {}, log: () => {}, idle: () => {} });
  const assetsFor = async (module: WebAssembly.Module): Promise<FruityAssets> => ({ module, glue: new Uint8Array([1]), seed: hostPackFromBytes(buildPack([])), dll: hostPackFromBytes(buildPack([])) });
  it('another Emscripten version: stopped before the glue is loaded, with the versions named', async () => {
    const m = newFormat();
    (m.engine.options as { hostFilesAbi: string }).hostFilesAbi = 'emscripten-7.1.0';
    let imported = false;
    await expect(FruityEngine.create(m, await assetsFor(await WebAssembly.compile(MIN_ENGINE)), host(), async () => { imported = true; return (async () => undefined) as CreateModule; })).rejects.toThrow(/linked by emscripten-7\.1\.0.*written for emscripten-6\.0\.12/);
    expect(imported).toBe(false);
  });
  it('a wasm that imports a system call the file layer does not know: stopped by the name of the import', async () => {
    const enc = (t: string) => [t.length, ...new TextEncoder().encode(t)];
    const imp = [1, ...enc('env'), ...enc('__syscall_mkdirat'), 0, 0];
    const bytes = new Uint8Array([0, 0x61, 0x73, 0x6d, 1, 0, 0, 0, 1, 4, 1, 0x60, 0, 0, 2, imp.length, ...imp]);
    await expect(FruityEngine.create(newFormat(), await assetsFor(await WebAssembly.compile(bytes)), host(), async () => (async () => undefined) as CreateModule)).rejects.toThrow(/imports __syscall_mkdirat/);
  });
});
