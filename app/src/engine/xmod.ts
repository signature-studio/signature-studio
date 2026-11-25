// Issue #1342 (PROVISIONAL): a real module whose code is a wasm module of its own (fe-wasmgen hybrid --separate; the plugin Fruity Balance first). The engine wasm does not hold that code: its calls into the
// module are imports env.f_<va> (the C part's dispatch table), and this file supplies them. The module imports the engine's memory, its __tls_base and its fle_imp_* functions from the engine's exports.
// It is instantiated when the guest first asks for its real module: the engine calls fle_xmod_load (lift/harness/two/main_two.c, fle_realmod_map_image) with the DLL's path, and the module's bytes are
// the lazy file <root>/<DLL stem>.wasm. The engine Worker runs the wasm synchronously, so the bytes are read in place (lazy-file.ts waits for them), and the module is compiled and instantiated in the same call.
// A DLL with no such file is left alone (fle_xmod_load returns 0). Every failure throws: the engine's call stops by the error, never with a guessed answer.
import type { LazyFileSource } from './lazy-file.ts';

type Fn = () => void;

const u32 = (n: number): number[] => { const out = []; do { const b = n & 127; n >>>= 7; out.push(b | (n ? 128 : 0)); } while (n); return out; };
const str = (s: string): number[] => { const bytes = new TextEncoder().encode(s); return [...u32(bytes.length), ...bytes]; };
const section = (id: number, body: number[]): number[] => [id, ...u32(body.length), ...body];

// The f_<va> imports use the C ABI: no arguments/results, guest registers in
// shared memory. A Wasm relay keeps JS frames out of calls that may suspend.
// Its table is filled when the plugin becomes available, after engine instantiation.
function forwarders(names: string[]): WebAssembly.Module {
  const bodies = names.flatMap((_, i) => {
    const slot = u32(i);
    if (slot[slot.length - 1]! & 64) { slot[slot.length - 1]! |= 128; slot.push(0); } // i32.const uses signed LEB
    const body = [0, 0x41, ...slot, 0x11, 0, 0, 0x0b];
    return [...u32(body.length), ...body];
  });
  return new WebAssembly.Module(new Uint8Array([
    0, 97, 115, 109, 1, 0, 0, 0,
    ...section(1, [1, 0x60, 0, 0]),
    ...section(2, [1, ...str('env'), ...str('table'), 1, 0x70, 0, ...u32(names.length)]),
    ...section(3, [...u32(names.length), ...names.map(() => 0)]),
    ...section(7, [...u32(names.length), ...names.flatMap((name, i) => [...str(name), 0, ...u32(i)])]),
    ...section(10, [...u32(names.length), ...bodies]),
  ]));
}

export class XmodLoader {
  private readonly slots = new Map<string, number>();
  private table: WebAssembly.Table | null = null;
  private readonly loaded = new Set<string>();
  private readonly root: string;
  private readonly lazy: LazyFileSource[];
  private readonly engineExports: () => Record<string, unknown> | null;
  private readonly memory: () => WebAssembly.Memory | null;

  constructor(root: string, lazy: LazyFileSource[], engineExports: () => Record<string, unknown> | null, memory: () => WebAssembly.Memory | null) {
    this.root = root;
    this.lazy = lazy;
    this.engineExports = engineExports;
    this.memory = memory;
  }

  /** Bind the engine's imports before it exists, without a JS forwarding frame. Unloaded slots trap. */
  bind(names: string[]): Record<string, Fn> {
    if (this.table) throw new Error('xmod: engine imports are already bound');
    this.table = new WebAssembly.Table({ element: 'anyfunc', initial: names.length });
    names.forEach((name, i) => this.slots.set(name, i));
    return new WebAssembly.Instance(forwarders(names), { env: { table: this.table } }).exports as Record<string, Fn>;
  }

  /** fle_xmod_load: the C string at ptr is the DLL's path. 1 when the separate module of that DLL is instantiated (now, or before), 0 when the DLL has none */
  loadAt(ptr: number): number {
    const mem = this.memory();
    const ex = this.engineExports();
    if (!mem || !ex) throw new Error('xmod: the engine has no memory or exports yet');
    const bytes = new Uint8Array(mem.buffer);
    ptr >>>= 0;
    const end = bytes.indexOf(0, ptr);
    if (end < 0) throw new Error(`xmod: no terminated module path at ${ptr}`);
    const dll = new TextDecoder().decode(bytes.slice(ptr, end));
    const wasmPath = dll.replace(/\.dll$/i, '.wasm');
    if (wasmPath === dll) return 0;
    const src = this.lazy.find((f) => `${this.root}/${f.path}` === wasmPath);
    if (!src) return 0;
    if (this.loaded.has(wasmPath)) return 1;
    const code = new Uint8Array(src.size);
    if (src.readInto(code, 0) !== src.size) throw new Error(`xmod: ${wasmPath} is shorter than its manifest size`);
    const mod = new WebAssembly.Module(code);
    const c: Record<string, unknown> = {};
    for (const i of WebAssembly.Module.imports(mod)) {
      if (i.module !== 'c') throw new Error(`xmod: ${wasmPath} imports ${i.module}.${i.name}; only the engine's exports (module c) are supplied`);
      // the link wraps some imports (link.rsp --wrap): a reference to the name goes to its __wrap_ function, as in the engine
      const v = ex[i.name] ?? ex[`__wrap_${i.name}`];
      if (v === undefined) throw new Error(`xmod: ${wasmPath} imports c.${i.name}, which the engine does not export`);
      c[i.name] = v;
    }
    const inst = new WebAssembly.Instance(mod, { c } as WebAssembly.Imports);
    for (const [k, v] of Object.entries(inst.exports)) {
      const slot = this.slots.get(k);
      if (slot !== undefined && typeof v === 'function') this.table!.set(slot, v);
    }
    this.loaded.add(wasmPath);
    return 1;
  }
}

/** the engine's imports for the separate modules: the supply of every env.f_<va> the engine imports (the glue may hold a placeholder for an undefined symbol; this replaces it). The engine's call of fle_xmod_load reaches loader.loadAt through the Module hook fleXmodLoad (fruity.ts) */
export function installXmod(imports: WebAssembly.Imports, module: WebAssembly.Module, loader: XmodLoader): void {
  const env = (imports.env ??= {}) as Record<string, unknown>;
  const names = [...new Set(WebAssembly.Module.imports(module).filter((i) => i.module === 'env' && i.kind === 'function' && i.name.startsWith('f_')).map((i) => i.name))];
  if (names.length) Object.assign(env, loader.bind(names));
}
