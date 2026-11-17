import { readFileSync } from 'node:fs';
import { WASI } from 'node:wasi';
const file = process.argv[2];
const bytes = readFileSync(file);
const mod = new WebAssembly.Module(bytes);
const imports = WebAssembly.Module.imports(mod).map((i) => `${i.module}.${i.name}`);
console.log('imports:', imports.join(' '));
const wasi = new WASI({ version: 'preview1', args: [file], env: {}, returnOnExit: true });
try {
  const inst = new WebAssembly.Instance(mod, { wasi_snapshot_preview1: wasi.wasiImport });
  const rc = wasi.start(inst);
  console.log('exit:', rc);
} catch (e) {
  console.log('error:', String(e).slice(0, 300));
}
