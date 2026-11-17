// node self-check of vst-host.js: node node-check.mjs WASM PLUGIN SEED [--blocks N]
import fs from 'node:fs';
import { createHost } from './vst-host.js';
const [wasm, plugin, seed, ...rest] = process.argv.slice(2);
const files = { '/ott.vst3': new Uint8Array(fs.readFileSync(plugin)), '/seed.tsv': new Uint8Array(fs.readFileSync(seed)) };
const mod = await WebAssembly.compile(fs.readFileSync(wasm));
const host = createHost({ files, args: ['/ott.vst3', ...rest], env: ['FLE_VFS_SEED=/seed.tsv', 'VST_BLOCKS=1'], print: (s) => console.log(s) });
const t0 = performance.now();
const inst = await WebAssembly.instantiate(mod, host.imports);
host.attach(inst);
console.log('instantiate ms', (performance.now() - t0).toFixed(1));
const rc = host.run();
console.log('rc', rc, 'missing', JSON.stringify(host.missing()));
