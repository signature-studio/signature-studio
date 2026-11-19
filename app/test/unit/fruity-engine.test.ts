import { describe, expect, it } from 'vitest';
import type { Manifest } from '../../src/bundle/manifest.ts';
import { buildPack } from '../../src/bundle/pack.ts';
import { classifyEngineError, FruityEngine, type CreateModule, type FruityAssets, type FruityHost, type FruityModuleOpts } from '../../src/engine/fruity.ts';
import { hostPackFromBytes } from '../../src/engine/hostfiles.ts';

// a module with a memory and a function fle_surface_table() -> 128 (hand-assembled)
const MIN_ENGINE = new Uint8Array([
  0, 0x61, 0x73, 0x6d, 1, 0, 0, 0,
  1, 5, 1, 0x60, 0, 1, 0x7f,                 // type: () -> i32
  3, 2, 1, 0,                                // one function
  5, 3, 1, 0, 2,                             // memory: min 2 pages
  7, 0x1e, 2, 6, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 2, 0, 0x11, 0x66, 0x6c, 0x65, 0x5f, 0x73, 0x75, 0x72, 0x66, 0x61, 0x63, 0x65, 0x5f, 0x74, 0x61, 0x62, 0x6c, 0x65, 0, 0, // exports
  10, 7, 1, 5, 0, 0x41, 0x80, 0x01, 0x0b,    // code: i32.const 128
]);

const sha = 'a'.repeat(64);
const manifest = (): Manifest => ({
  schema: 1, bundle: { name: 'fl', version: '1' },
  engine: {
    kind: 'emscripten-fruity-v1', wasm: { url: '/e.wasm', size: 1, sha256: sha, compression: 'identity' }, args: ['/data/FLEngine_x64.dll', '/data/m1.dll:/data/m2.dll'],
    env: { FLE_CALL_FRUITY: '1', FLE_VFS_SEED_ROOT: '/data/seed' },
    options: { glue: { url: '/g.js', size: 1, sha256: sha, compression: 'identity' }, root: '/data', hostFilesAbi: 'emscripten-6.0.12' },
  },
  seedPack: { url: '/s', size: 1, sha256: sha, compression: 'identity', format: 'flseedp1' }, dllPack: { url: '/d', size: 1, sha256: sha, compression: 'identity', format: 'flpack1' },
});

function host() {
  const details: Record<string, Record<string, unknown> | undefined> = {};
  const marks: string[] = [], out: string[] = [], err: string[] = [], logs: string[] = [];
  let idles = 0;
  const h: FruityHost = {
    mark: (n, d) => { marks.push(n); details[n] = d; },
    stdout: (l) => out.push(l), stderr: (l) => err.push(l), log: (_l, t) => logs.push(t),
    idle: () => { idles++; },
  };
  return { h, marks, details, out, err, logs, idles: () => idles };
}

const packs = () => {
  const seed = hostPackFromBytes(buildPack([{ path: 'seed/a.tsv', data: new Uint8Array([1, 2]) }, { path: 'seed/x/y.bin', data: new Uint8Array([3]) }, { path: 'seed/x/z.bin', data: new Uint8Array([4, 5, 6]) }]));
  const dll = hostPackFromBytes(buildPack([{ path: 'FLEngine_x64.dll', data: new Uint8Array([9, 9, 9, 9]) }, { path: 'm1.dll', data: new Uint8Array([8]) }]));
  return { seed, dll };
};
const assets = async (): Promise<FruityAssets> => ({ module: await WebAssembly.compile(MIN_ENGINE), glue: new Uint8Array([1]), ...packs() });

// A glue that behaves like the emscripten one: FS / ENV on the options object, instantiateWasm, preRun, onRuntimeInitialized, the engine's hooks, then onExit
function fakeGlue(script: (o: FruityModuleOpts) => Promise<void> | void, withEnv = true): { create: CreateModule; seen: { env: Record<string, string> } } {
  const seen = { env: {} as Record<string, string> };
  const create: CreateModule = async (opts) => {
    if (withEnv) opts.ENV = seen.env;
    await new Promise<void>((res) => { opts.instantiateWasm({}, () => res()); });
    for (const f of opts.preRun) f();
    opts.onRuntimeInitialized();
    await script(opts);
  };
  return { create, seen };
}

describe('FruityEngine: the input exports and the window list (decision 0141)', () => {
  const withExports = async (ex: Record<string, unknown>, mem?: WebAssembly.Memory) => {
    const g = fakeGlue(() => undefined);
    const eng = await FruityEngine.create(manifest(), await assets(), host().h, async () => g.create);
    (eng as unknown as { instance: unknown }).instance = { exports: ex };
    if (mem) (eng as unknown as { memory: unknown }).memory = mem;
    return eng;
  };
  it('inputApi passes the arguments of fle_g3_mouse / fle_g3_key / fle_g3_clock_to in the order of fruity_idle.c', async () => {
    const calls: string[] = [];
    const eng = await withExports({
      fle_g3_mouse: (k: number, x: number, y: number, d: number) => { calls.push(`mouse ${k} ${x} ${y} ${d}`); return 1; },
      fle_g3_key: (k: number, vk: number, sc: number, e: number) => { calls.push(`key ${k} ${vk} ${sc} ${e}`); return 1; },
      fle_g3_clock_to: (t: number) => { calls.push(`clock ${t}`); },
    });
    const api = eng.inputApi!;
    expect(api.mouse(2, 958, 599, 0)).toBe(1);
    expect(api.key(5, 0x20, 0x39, 0)).toBe(1);
    api.clockTo(36_000_160_000);
    expect(calls).toEqual(['mouse 2 958 599 0', 'key 5 32 57 0', 'clock 36000160000']);
  });
  it('decision 0272: taskbarIconic needs both taskbar exports of fruity_idle.c; syscommand is in the inputApi only with both; a bundle built before decision 0267 has neither (null, nothing is shown)', async () => {
    const base = { fle_g3_mouse: () => 1, fle_g3_key: () => 1, fle_g3_clock_to: () => undefined };
    const none = await withExports(base);
    expect(none.taskbarIconic()).toBeNull();
    expect(none.inputApi!.syscommand).toBeUndefined();
    const half = await withExports({ ...base, fle_g3_taskbar_iconic: () => 7 });
    expect(half.taskbarIconic()).toBeNull();
    expect(half.inputApi!.syscommand).toBeUndefined();
    const calls: string[] = [];
    let iconic = 0;
    const full = await withExports({ ...base, fle_g3_taskbar_iconic: () => iconic, fle_g3_taskbar_syscommand: (lid: number, cmd: number) => { calls.push(`syscmd ${lid} ${cmd.toString(16)}`); return 1; } });
    expect(full.taskbarIconic()).toBe(0);
    iconic = 7;
    expect(full.taskbarIconic()).toBe(7);
    expect(full.inputApi!.syscommand!(7, 0xf120)).toBe(1);
    expect(calls).toEqual(['syscmd 7 f120']);
  });
  it('a wasm without the input exports (built before decision 0136) has no inputApi and no window list', async () => {
    const eng = await withExports({ fle_surface_table: () => 128 });
    expect(eng.inputApi).toBeNull();
    expect(eng.hasAppView).toBe(false);
    expect(eng.readAppView()).toBeNull();
  });
  it('readAppView reads the structure fle_av_poll() points at, in the engine\'s memory', async () => {
    const mem = new WebAssembly.Memory({ initial: 1 });
    const dv = new DataView(mem.buffer, 256);
    dv.setUint32(0, 0x50414c46, true); dv.setUint32(4, 1, true); dv.setUint32(8, 144, true); dv.setUint32(12, 32, true); dv.setUint32(16, 7, true); dv.setUint32(20, 0x7fca0004, true); dv.setUint32(24, 32512, true);
    const eng = await withExports({ fle_av_poll: () => 256 }, mem);
    expect(eng.hasAppView).toBe(true);
    expect(eng.readAppView()).toMatchObject({ seq: 7, cursor: 0x7fca0004, cursorIdc: 32512, windows: [] });
  });
});

describe('FruityEngine', () => {
  it('runs: imports the glue, mounts the DLLs and the seed as host files, sets the environment and argv, reports the exit code', async () => {
    const g = fakeGlue(async (o) => {
      expect(o.arguments).toEqual(['/data/FLEngine_x64.dll', '/data/m1.dll:/data/m2.dll']);
      o.print('hello');
      o.printErr('warn');
      o.onExit(134);
      const e = new Error('exit(134)'); e.name = 'ExitStatus'; throw e;
    });
    const h = host();
    const eng = await FruityEngine.create(manifest(), await assets(), h.h, async () => g.create);
    expect(h.marks).toEqual(['glue_loaded']);
    const r = await eng.run();
    expect(r).toEqual({ kind: 'exit', code: 134 });
    expect(h.marks).toEqual(['glue_loaded', 'wasm_instantiated', 'files_ready', 'main_start', 'exit']);
    expect(h.details.files_ready).toMatchObject({ how: 'packs-v1', dllFiles: 2, dllBytes: 5, seedFiles: 3, seedBytes: 6 });
    expect(g.seen.env).toEqual({ FLE_CALL_FRUITY: '1', FLE_VFS_SEED_ROOT: '/data/seed' });
    expect(h.out).toEqual(['hello']);
    expect(h.err).toEqual(['warn']);
    expect(eng.memory!.buffer.byteLength).toBe(2 * 65536);
  });

  it('a normal return of Module() without onExit is "returned"', async () => {
    const g = fakeGlue(() => undefined);
    const eng = await FruityEngine.create(manifest(), await assets(), host().h, async () => g.create);
    expect(await eng.run()).toEqual({ kind: 'returned', code: null });
  });

  it('settles when onExit is called but Module() never settles (an exit on a guest thread other than the first)', async () => {
    const g = fakeGlue((o) => { o.onExit(3); return new Promise<void>(() => { /* never */ }); });
    const eng = await FruityEngine.create(manifest(), await assets(), host().h, async () => g.create);
    expect(await eng.run()).toEqual({ kind: 'exit', code: 3 });
  });

  it('a throw that is not an ExitStatus is a trap with the stack', async () => {
    const g = fakeGlue(() => { throw new RangeError('Maximum call stack size exceeded'); });
    const eng = await FruityEngine.create(manifest(), await assets(), host().h, async () => g.create);
    const r = await eng.run();
    expect(r.kind).toBe('trap');
    expect((r as { message: string }).message).toContain('Maximum call stack size exceeded');
  });

  it('stays pending while the engine is idle, and endIdle() ends the session', async () => {
    let release: (() => void) | null = null;
    let delivered: number | null = null;
    const h = host();
    const g = fakeGlue(async (o) => {
      o.fleIdleWaiting = () => release !== null;
      o.fleIdleDeliver = (code) => { const r = release; release = null; if (r) { delivered = code; r(); o.onExit(code < 0 ? 0 : code); return true; } return false; };
      o.fleIdle(1, 99, 0);
      await new Promise<void>((res) => { release = res; });
    });
    const eng = await FruityEngine.create(manifest(), await assets(), h.h, async () => g.create);
    let settled = false;
    const p = eng.run().then((r) => { settled = true; return r; });
    await new Promise((r) => setTimeout(r, 30));
    expect(settled).toBe(false);
    expect(h.idles()).toBe(1);
    expect(eng.idle).toBe(true);
    expect(eng.endIdle()).toBe(true);
    expect(delivered).toBe(-1);   // decision 0136: < 0 ends the session; 0 means 'events were queued'
    expect(await p).toEqual({ kind: 'exit', code: 0 });
    expect(eng.endIdle()).toBe(false);
  });

  it('fails with a clear message when the glue gives no ENV', async () => {
    const g = fakeGlue(() => undefined, false);
    const h = host();
    const eng = await FruityEngine.create(manifest(), await assets(), h.h, async () => g.create);
    const r = await eng.run();
    expect(r.kind).toBe('trap');
    expect((r as { message: string }).message).toContain('exposes no ENV');
  });

  it('the glue importer\'s failure reaches the caller of create()', async () => {
    await expect(FruityEngine.create(manifest(), await assets(), host().h, async () => { throw new Error('no default export'); })).rejects.toThrow('no default export');
  });
});

describe('classifyEngineError', () => {
  it('tells out-of-memory apart from the rest', () => {
    expect(classifyEngineError('RangeError: WebAssembly.Memory(): could not allocate memory').kind).toBe('oom');
    expect(classifyEngineError('RangeError: Array buffer allocation failed').kind).toBe('oom');
    expect(classifyEngineError('Aborted(Cannot enlarge memory arrays to size 2147483648)').kind).toBe('oom');
    expect(classifyEngineError('CompileError: WebAssembly.instantiate(): invalid opcode').kind).toBe('compile');
    expect(classifyEngineError('TypeError: WebAssembly.Suspending is not a constructor').kind).toBe('jspi');
    expect(classifyEngineError('Error: unreachable\n  at foo').kind).toBe('other');
  });
  it('the OOM text says what happened', () => {
    expect(classifyEngineError('RangeError: WebAssembly.Memory(): could not allocate memory').what).toContain('ran out of memory');
  });
});
