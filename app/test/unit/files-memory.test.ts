// Decision 0248 (ci1): the file manager when the browser gives the page no usable OPFS. The page then keeps the files in the tab's memory (src/files/memory.ts) behind the same OpfsFs, so that the Files
// window, the engine's Open / Save dialogs and the save-back work as they do with the OPFS. What is tested: the memory tree answers like the OPFS where the page can notice it (names of the failures, a
// file appears when its writable is closed, one writable at a time), OpfsFs works over it (list, stat, write, read, copy, move, remove), and the one place that decides (detectRoot) falls back whenever the
// OPFS is missing, blocked or does not store what is written.
import { describe, expect, it, vi } from 'vitest';
import { MemoryDirectoryHandle } from '../../src/files/memory.ts';
import { FsError, OpfsFs, detectRoot, memoryNotice, roomLeft, type Dir } from '../../src/files/opfs.ts';

const asDir = (d: MemoryDirectoryHandle): Dir => d as unknown as Dir;
const errName = async (p: Promise<unknown>): Promise<string> => { try { await p; return 'no error'; } catch (e) { return (e as { name: string }).name; } };
const put = async (d: MemoryDirectoryHandle, name: string, data: string | Uint8Array | Blob): Promise<void> => { const w = await (await d.getFileHandle(name, { create: true })).createWritable(); await w.write(data); await w.close(); };
const text = async (d: MemoryDirectoryHandle, name: string): Promise<string> => (await (await d.getFileHandle(name)).getFile()).text();

describe('the memory tree answers like the OPFS', () => {
  it('a written file has its bytes, name and modification time after close (and a new time after each write)', async () => {
    vi.useFakeTimers({ toFake: ['Date'] });
    try {
      const d = new MemoryDirectoryHandle();
      vi.setSystemTime(1_000_000);
      await put(d, 'a.txt', 'hello');
      const f = await (await d.getFileHandle('a.txt')).getFile();
      expect(f.name).toBe('a.txt');
      expect(f.size).toBe(5);
      expect(await f.text()).toBe('hello');
      expect(f.lastModified).toBe(1_000_000);
      vi.setSystemTime(5_000_000);
      await put(d, 'a.txt', 'hello again');
      expect((await (await d.getFileHandle('a.txt')).getFile()).lastModified).toBe(5_000_000);
    } finally { vi.useRealTimers(); }
  });

  it('a file is replaced only when its writable is closed; abort leaves it as it was', async () => {
    const d = new MemoryDirectoryHandle();
    await put(d, 'a.txt', 'old');
    const w = await (await d.getFileHandle('a.txt')).createWritable();
    await w.write('new content');
    expect(await text(d, 'a.txt')).toBe('old');
    await w.abort();
    expect(await text(d, 'a.txt')).toBe('old');
    const w2 = await (await d.getFileHandle('a.txt')).createWritable();
    await w2.write('second');
    await w2.close();
    expect(await text(d, 'a.txt')).toBe('second');
  });

  it('keepExistingData starts from the old bytes; a new file from getFileHandle(create) is empty', async () => {
    const d = new MemoryDirectoryHandle();
    expect((await (await d.getFileHandle('e', { create: true })).getFile()).size).toBe(0);
    await put(d, 'a', 'abc');
    const w = await (await d.getFileHandle('a')).createWritable({ keepExistingData: true });
    await w.write('def');
    await w.close();
    expect(await text(d, 'a')).toBe('abcdef');
  });

  it('one writable at a time per file (Firefox\'s rule); after close or abort another may open', async () => {
    const d = new MemoryDirectoryHandle();
    const fh = await d.getFileHandle('a', { create: true });
    const w = await fh.createWritable();
    expect(await errName(fh.createWritable())).toBe('NoModificationAllowedError');
    expect(await errName(d.removeEntry('a'))).toBe('NoModificationAllowedError');
    await w.abort();
    const w2 = await fh.createWritable();
    await w2.close();
    expect(await errName(w2.write('x'))).toBe('TypeError');
    expect(await errName(w2.close())).toBe('TypeError');
  });

  it('the failures carry the DOMException names the page looks at', async () => {
    const d = new MemoryDirectoryHandle();
    await put(d, 'f', 'x');
    await d.getDirectoryHandle('dir', { create: true });
    expect(await errName(d.getFileHandle('nope'))).toBe('NotFoundError');
    expect(await errName(d.getDirectoryHandle('nope'))).toBe('NotFoundError');
    expect(await errName(d.getDirectoryHandle('f'))).toBe('TypeMismatchError');
    expect(await errName(d.getFileHandle('dir'))).toBe('TypeMismatchError');
    expect(await errName(d.removeEntry('nope'))).toBe('NotFoundError');
    const sub = await d.getDirectoryHandle('dir');
    await put(sub, 'inner', 'y');
    expect(await errName(d.removeEntry('dir'))).toBe('InvalidModificationError');
    await d.removeEntry('dir', { recursive: true });
    expect(await errName(d.getDirectoryHandle('dir'))).toBe('NotFoundError');
    for (const bad of ['', '.', '..', 'a/b', 'a\\b']) expect(await errName(d.getFileHandle(bad, { create: true }))).toBe('TypeError');
  });

  it('a removed file\'s handle is dead, a folder with an open writable cannot be removed', async () => {
    const d = new MemoryDirectoryHandle();
    await put(d, 'a', 'x');
    const fh = await d.getFileHandle('a');
    await d.removeEntry('a');
    expect(await errName(fh.getFile())).toBe('NotFoundError');
    expect(await errName(fh.createWritable())).toBe('NotFoundError');
    const sub = await d.getDirectoryHandle('s', { create: true });
    const w = await (await sub.getFileHandle('b', { create: true })).createWritable();
    expect(await errName(d.removeEntry('s', { recursive: true }))).toBe('NoModificationAllowedError');
    await w.close();
    await d.removeEntry('s', { recursive: true });
  });

  it('entries lists names and kinds, and their handles work', async () => {
    const d = new MemoryDirectoryHandle();
    await put(d, 'a.txt', 'A');
    await d.getDirectoryHandle('sub', { create: true });
    const seen: string[] = [];
    for await (const [n, h] of d.entries()) { seen.push(`${n}:${h.kind}`); if (h.kind === 'file') expect(await (await (h as unknown as { getFile(): Promise<File> }).getFile()).text()).toBe('A'); }
    expect(seen.sort()).toEqual(['a.txt:file', 'sub:directory']);
  });

  it('what is written is copied: changing the source afterwards changes nothing; views and ArrayBuffers and Blobs are written whole', async () => {
    const d = new MemoryDirectoryHandle();
    const src = new Uint8Array([1, 2, 3, 4, 5, 6]);
    const w = await (await d.getFileHandle('b', { create: true })).createWritable();
    await w.write(src.subarray(1, 4));
    src.fill(9);
    await w.write(new Uint8Array([7, 8]).buffer);
    await w.write(new Blob([new Uint8Array([10])]));
    await w.close();
    expect([...new Uint8Array(await (await (await d.getFileHandle('b')).getFile()).arrayBuffer())]).toEqual([2, 3, 4, 7, 8, 10]);
  });
});

describe('OpfsFs over the memory tree (the code the Files window and the engine dialogs use)', () => {
  const fresh = (): OpfsFs => new OpfsFs(asDir(new MemoryDirectoryHandle()), false, 'blocked');

  it('writes, reads, stats and lists (folders first, then by name ignoring case)', async () => {
    const fs = fresh();
    await fs.mkdir('/proj/samples');
    await fs.write('/proj/b.flp', 'FLhd');
    await fs.write('/proj/A.txt', new Uint8Array([1, 2, 3]));
    await fs.write('/proj/samples/k.wav', new Blob([new Uint8Array(10)]));
    expect((await fs.stat('/proj/b.flp'))?.size).toBe(4);
    expect((await fs.stat('/proj/samples'))?.kind).toBe('directory');
    expect(await fs.stat('/proj/none')).toBeNull();
    expect((await fs.list('/proj')).map((e) => `${e.kind[0]}:${e.name}:${e.size}`)).toEqual(['d:samples:0', 'f:A.txt:3', 'f:b.flp:4']);
    expect(await (await fs.read('/proj/b.flp')).text()).toBe('FLhd');
    expect(await fs.subdirs('/proj')).toEqual(['samples']);
    expect([...await fs.names('/proj')].sort()).toEqual(['a.txt', 'b.flp', 'samples']);
    expect(await fs.findCI('/proj', 'B.FLP')).toBe('b.flp');
    expect(await fs.freeName('/proj', 'a.txt')).toBe('a (2).txt');
  });

  it('writes a ReadableStream and counts the bytes', async () => {
    const fs = fresh();
    const chunks = [new Uint8Array(100), new Uint8Array(50)];
    let i = 0;
    const stream = new ReadableStream<Uint8Array>({ pull(c) { if (i < chunks.length) c.enqueue(chunks[i++]!); else c.close(); } });
    let n = 0;
    await fs.write('/s.bin', stream, (b) => { n += b; });
    expect(n).toBe(150);
    expect((await fs.stat('/s.bin'))?.size).toBe(150);
  });

  it('copies and moves files and folders, removes, measures', async () => {
    const fs = fresh();
    await fs.write('/a/x.txt', 'X');
    await fs.write('/a/sub/y.txt', 'YY');
    expect(await fs.copy('/a', '/', 'a2')).toBe('/a2');
    expect(await (await fs.read('/a2/sub/y.txt')).text()).toBe('YY');
    expect(await fs.move('/a2', '/', 'a3')).toBe('/a3');
    expect(await fs.exists('/a2')).toBe(false);
    expect(await fs.du('/a3')).toEqual([2, 3]);
    await fs.move('/a3/x.txt', '/a', 'moved.txt');
    expect(await fs.exists('/a3/x.txt')).toBe(false);
    expect(await (await fs.read('/a/moved.txt')).text()).toBe('X');
    await fs.remove('/a3');
    expect(await fs.exists('/a3')).toBe(false);
    await expect(fs.remove('/')).rejects.toBeInstanceOf(FsError);
    await expect(fs.copy('/missing', '/')).rejects.toBeInstanceOf(FsError);
  });

  it('fills the sizes of listed files', async () => {
    const fs = fresh();
    await fs.write('/p/one.bin', new Uint8Array(7));
    await fs.write('/p/two.bin', new Uint8Array(9));
    const es = await fs.list('/p', false);
    expect(es.map((e) => e.size)).toEqual([0, 0]);
    await fs.fillStats(es, () => {}, () => true);
    expect(es.map((e) => e.size)).toEqual([7, 9]);
  });
});

describe('detectRoot: the one place that decides between the OPFS and the tab\'s memory', () => {
  const memoryRoot = (): MemoryDirectoryHandle => new MemoryDirectoryHandle();

  it('without navigator.storage.getDirectory the files are in memory', async () => {
    const r = await detectRoot({});
    expect(r.persistent).toBe(false);
    expect(r.why).toBe('unsupported');
    await new OpfsFs(r.root, r.persistent, r.why).write('/x', 'y');
  });

  it('a SecurityError (Firefox private window) is "blocked"', async () => {
    const r = await detectRoot({ storage: { getDirectory: () => Promise.reject(Object.assign(new Error('Security error when calling GetDirectory'), { name: 'SecurityError' })) } });
    expect(r.persistent).toBe(false);
    expect(r.why).toBe('blocked');
  });

  it('an OPFS that hands out a root but cannot write ("Could not create actor") falls back, and the probe is cleaned', async () => {
    const real = memoryRoot();
    const broken = { getFileHandle: real.getFileHandle.bind(real), removeEntry: vi.fn(real.removeEntry.bind(real)), getDirectoryHandle: real.getDirectoryHandle.bind(real), entries: real.entries.bind(real) };
    broken.getFileHandle = async (n: string, o?: { create?: boolean }) => { const h = await real.getFileHandle(n, o); return { ...h, getFile: () => h.getFile(), createWritable: () => Promise.reject(Object.assign(new Error('Could not create actor'), { name: 'UnknownError' })) } as unknown as typeof h; };
    const r = await detectRoot({ storage: { getDirectory: async () => broken as unknown as FileSystemDirectoryHandle } });
    expect(r.persistent).toBe(false);
    expect(r.why).toBe('failed: UnknownError: Could not create actor');
    expect(broken.removeEntry).toHaveBeenCalledWith(expect.stringMatching(/^\.fl-probe-[a-z0-9]+$/));
  });

  it('an OPFS that stores a different number of bytes falls back', async () => {
    const real = memoryRoot();
    const liar = { getFileHandle: async (n: string, o?: { create?: boolean }) => { const h = await real.getFileHandle(n, o); return { createWritable: async () => ({ write: async () => {}, close: async () => {} }), getFile: () => h.getFile() }; }, removeEntry: real.removeEntry.bind(real) };
    const r = await detectRoot({ storage: { getDirectory: async () => liar as unknown as FileSystemDirectoryHandle } });
    expect(r.persistent).toBe(false);
    expect(r.why).toMatch(/^failed: /);
  });

  it('two pages probing at the same moment both get the OPFS (each probes with a name of its own)', async () => {
    const real = memoryRoot();
    const nav = { storage: { getDirectory: async () => real as unknown as FileSystemDirectoryHandle } };
    const [a, b] = await Promise.all([detectRoot(nav), detectRoot(nav)]);
    expect([a.persistent, b.persistent]).toEqual([true, true]);
  });

  it('a working OPFS is used as it is, and the probe file is gone again', async () => {
    const real = memoryRoot();
    const r = await detectRoot({ storage: { getDirectory: async () => real as unknown as FileSystemDirectoryHandle } });
    expect(r.persistent).toBe(true);
    expect(r.why).toBeNull();
    expect(r.root).toBe(real);
    const names: string[] = [];
    for await (const [n] of real.entries()) names.push(n);
    expect(names).toEqual([]);
  });

  it('the Files window and the engine dialog share one tree: what one writes the other lists', async () => {
    vi.resetModules();
    vi.stubGlobal('navigator', {});
    try {
      const mod = await import('../../src/files/opfs.ts');
      const a = await mod.OpfsFs.open();
      const b = await mod.OpfsFs.open();
      expect(a.persistent).toBe(false);
      await a.write('/Downloads/p.flp', 'project');
      expect((await b.list('/Downloads')).map((e) => e.name)).toEqual(['p.flp']);
      expect(await (await b.read('/Downloads/p.flp')).text()).toBe('project');
      const sub = await mod.OpfsFs.open('/Downloads');
      expect((await sub.list('/')).map((e) => e.name)).toEqual(['p.flp']);
    } finally { vi.unstubAllGlobals(); }
  });
});

describe('the line the person reads', () => {
  it('says that the files are in the tab\'s memory and when they are gone, for every reason', () => {
    for (const why of ['unsupported', 'blocked', 'failed: UnknownError: Could not create actor']) {
      const t = memoryNotice(why);
      expect(t).toContain("tab's memory");
      expect(t).toContain('closed or reloaded');
    }
    expect(memoryNotice('blocked')).toContain('private window');
    expect(memoryNotice('failed: UnknownError: Could not create actor')).toContain('Could not create actor');
    expect(memoryNotice('failed: UnknownError: Could not create actor')).not.toContain('failed:');
  });
});

describe('the browser\'s storage quota is the OPFS\'s, not the memory tree\'s', () => {
  it('the room left is quota minus usage for the OPFS, and no limit at all for the tab\'s memory (a small quota must not refuse files that go to memory)', () => {
    const small = { usage: 213_000_000, quota: 389_000_000 };
    expect(roomLeft({ persistent: true }, small)).toBe(176_000_000);
    expect(roomLeft({ persistent: false }, small)).toBeNull();
    expect(roomLeft({ persistent: true }, { usage: 500, quota: 100 })).toBe(0);
    expect(roomLeft({ persistent: true }, { usage: null, quota: 100 })).toBeNull();
    expect(roomLeft({ persistent: true }, { usage: 5, quota: null })).toBeNull();
  });
});
