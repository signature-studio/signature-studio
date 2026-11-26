// Issue #1342 (PROVISIONAL): a real module whose code is a wasm module of its own (fe-wasmgen hybrid --separate; the plugin Fruity Balance first). The engine wasm does not hold that code: its calls into the
// module are imports env.f_<va> (the C part's dispatch table), and this file supplies them. The module imports the engine's memory, its __tls_base and its fle_imp_* functions from the engine's exports.
// It is instantiated when the guest first asks for its real module: the engine calls fle_xmod_load (lift/harness/two/main_two.c, fle_realmod_map_image) with the DLL's path, and the module's bytes are
// the lazy file <root>/<DLL stem>.wasm. The engine Worker runs the wasm synchronously, so the bytes are read in place (lazy-file.ts waits for them), and the module is compiled and instantiated in the same call.
// A DLL with no such file is left alone (fle_xmod_load returns 0). Every failure throws: the engine's call stops by the error, never with a guessed answer.
import type { LazyFileSource } from './lazy-file.ts';

type Fn = (...args: unknown[]) => unknown;

export class XmodLoader {
  private readonly fns = new Map<string, Fn>();
  private readonly loaded = new Set<string>();
  private readonly importNameByFuncIdx = new Map<string, string>();
  private tableSlotsByName: Map<string, number[]> | null = null;
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

  registerImportIndex(funcIdx: number, name: string): void {
    this.importNameByFuncIdx.set(String(funcIdx), name);
  }

  private indexTable(table: WebAssembly.Table): Map<string, number[]> {
    const slotsByName = new Map<string, number[]>();
    for (let slot = 0; slot < table.length; slot++) {
      const entry = table.get(slot);
      if (typeof entry !== 'function') continue;
      const name = this.importNameByFuncIdx.get(entry.name);
      if (!name) continue;
      let list = slotsByName.get(name);
      if (!list) {
        list = [];
        slotsByName.set(name, list);
      }
      list.push(slot);
    }
    return slotsByName;
  }

  /** the supply of the engine's import env.<name>: the function that the separate module exports under that name (an error while that module is not loaded) */
  stub(name: string): Fn {
    return (...args: unknown[]) => {
      const fn = this.fns.get(name);
      if (!fn) throw new Error(`xmod: ${name} is called before its module is loaded`);
      return fn(...args);
    };
  }

  /** fle_xmod_load: the C string at ptr is the DLL's path. 1 when the separate module of that DLL is instantiated (now, or before), 0 when the DLL has none */
  loadAt(ptr: number): number {
    const mem = this.memory();
    const ex = this.engineExports();
    if (!mem || !ex) throw new Error('xmod: the engine has no memory or exports yet');
    const bytes = new Uint8Array(mem.buffer);
    let end = ptr;
    while (bytes[end] !== 0) end++;
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
    const table = ex['__indirect_function_table'];
    if (table instanceof WebAssembly.Table) {
      this.tableSlotsByName ??= this.indexTable(table);
    }
    for (const [k, v] of Object.entries(inst.exports)) {
      if (k.startsWith('f_') && typeof v === 'function') {
        this.fns.set(k, v as Fn);
        if (table instanceof WebAssembly.Table && this.tableSlotsByName) {
          for (const slot of this.tableSlotsByName.get(k) ?? []) {
            table.set(slot, v);
          }
        }
      }
    }
    this.loaded.add(wasmPath);
    return 1;
  }
}

/** the engine's imports for the separate modules: the supply of every env.f_<va> the engine imports (the glue may hold a placeholder for an undefined symbol; this replaces it). The engine's call of fle_xmod_load reaches loader.loadAt through the Module hook fleXmodLoad (fruity.ts) */
export function installXmod(imports: WebAssembly.Imports, module: WebAssembly.Module, loader: XmodLoader): void {
  const env = (imports.env ??= {}) as Record<string, unknown>;
  let funcIdx = 0;
  for (const i of WebAssembly.Module.imports(module)) {
    if (i.kind !== 'function') continue;
    if (i.module === 'env' && i.name.startsWith('f_')) {
      loader.registerImportIndex(funcIdx, i.name);
      env[i.name] = loader.stub(i.name);
    }
    funcIdx++;
  }
}
