import { expect, it } from 'vitest';
import { installXmod, XmodLoader } from '../../src/engine/xmod.ts';
import { LazyFileFeed, LazyFileSource, makeLazyFileShared } from '../../src/engine/lazy-file.ts';

const u32 = (n: number): number[] => { const out = []; do { const b = n & 127; n >>>= 7; out.push(b | (n ? 128 : 0)); } while (n); return out; };
const str = (s: string): number[] => [...u32(s.length), ...new TextEncoder().encode(s)];
const section = (id: number, body: number[]): number[] => [id, ...u32(body.length), ...body];

it('rejects out-of-bounds and unterminated module paths without scanning forever', () => {
  const memory = new WebAssembly.Memory({ initial: 1 });
  const loader = new XmodLoader('/data', [], () => ({}), () => memory);
  new Uint8Array(memory.buffer)[65535] = 120;
  for (const ptr of [65535, 65536, -1]) expect(() => loader.loadAt(ptr)).toThrow('no terminated module path');
});

// Each defined function calls one import. The engine calls the lazy plugin,
// which calls back into the engine's wait function, which suspends asynchronously.
function fixture(imports: [string, string][], exports: [string, number][]): Uint8Array<ArrayBuffer> {
  const bodies = exports.flatMap(([, target]) => { const body = [0, 0x10, ...u32(target), 0x0b]; return [...u32(body.length), ...body]; });
  return new Uint8Array([
    0, 97, 115, 109, 1, 0, 0, 0,
    ...section(1, [1, 0x60, 0, 0]),
    ...section(2, [...u32(imports.length), ...imports.flatMap(([m, n]) => [...str(m), ...str(n), 0, 0])]),
    ...section(3, [...u32(exports.length), ...exports.map(() => 0)]),
    ...section(7, [...u32(exports.length), ...exports.flatMap(([name], i) => [...str(name), 0, ...u32(imports.length + i)])]),
    ...section(10, [...u32(exports.length), ...bodies]),
  ]);
}

it('resumes a lazy plugin call that suspends through an engine callback', async () => {
  const jspi = WebAssembly as typeof WebAssembly & {
    Suspending: new (fn: () => Promise<void>) => WebAssembly.ImportValue;
    promising: (fn: WebAssembly.ExportValue) => () => Promise<void>;
  };
  const plugin = fixture([['c', 'wait']], [['f_10', 0]]);
  const shared = makeLazyFileShared('demo.wasm', 'demo.wasm', plugin.length);
  new LazyFileFeed(shared).deliver(plugin);
  const memory = new WebAssembly.Memory({ initial: 1 });
  new Uint8Array(memory.buffer).set(new TextEncoder().encode('/data/demo.dll\0'), 16);
  let engine: WebAssembly.Instance | null = null;
  const loader = new XmodLoader('/data', [new LazyFileSource(shared)], () => engine?.exports ?? null, () => memory);
  const events: string[] = [];
  const imports: WebAssembly.Imports = { env: { pause: new jspi.Suspending(async () => {
    events.push('suspend');
    await Promise.resolve();
    events.push('resume');
  }) } };
  const module = new WebAssembly.Module(fixture([['env', 'f_10'], ['env', 'pause']], [['run', 0], ['wait', 1]]));
  installXmod(imports, module, loader);
  engine = new WebAssembly.Instance(module, imports);
  expect(() => (engine!.exports.run as () => void)()).toThrow();
  expect(loader.loadAt(16)).toBe(1);
  expect(loader.loadAt(16)).toBe(1);
  await expect(jspi.promising(engine.exports.run)()).resolves.toBeUndefined();
  expect(events).toEqual(['suspend', 'resume']);
});

it('forwards slots beyond signed LEB boundaries in a large plugin', () => {
  const names = Array.from({ length: 8193 }, (_, i) => `f_${i.toString(16)}`);
  const plugin = fixture([['c', 'mark']], names.map((name) => [name, 0]));
  const shared = makeLazyFileShared('demo.wasm', 'demo.wasm', plugin.length);
  new LazyFileFeed(shared).deliver(plugin);
  const memory = new WebAssembly.Memory({ initial: 1 });
  new Uint8Array(memory.buffer).set(new TextEncoder().encode('/data/demo.dll\0'), 16);
  let calls = 0;
  const loader = new XmodLoader('/data', [new LazyFileSource(shared)], () => ({ mark: () => { calls++; } }), () => memory);
  const probes = [63, 64, 127, 8192];
  const module = new WebAssembly.Module(fixture(names.map((name) => ['env', name]), probes.map((i) => [`probe_${i}`, i])));
  const imports: WebAssembly.Imports = {};
  installXmod(imports, module, loader);
  const engine = new WebAssembly.Instance(module, imports);
  loader.loadAt(16);
  for (const i of probes) (engine.exports[`probe_${i}`] as () => void)();
  expect(calls).toBe(probes.length);
});
