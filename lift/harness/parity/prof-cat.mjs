// parity1: self time of a V8 .cpuprofile of the engine Worker by owner: guest (f_<VA>, fb_<VA>: the lifted code), runtime (fle_*, libc, the harness: C compiled into the wasm), Blend2D (bl::), unnamed wasm, JavaScript, (program) = V8/browser internals, (garbage collector), (idle).
// usage: node prof-cat.mjs FILE...      (one line per file; the names need a build with --profiling-funcs)
import fs from 'node:fs';
for (const f of process.argv.slice(2)) {
  const p = JSON.parse(fs.readFileSync(f, 'utf8')); const byId = new Map(p.nodes.map((x) => [x.id, x])); const self = new Map();
  for (let i = 0; i < p.samples.length; i++) self.set(p.samples[i], (self.get(p.samples[i]) ?? 0) + (p.timeDeltas[i] ?? 0));
  const cat = {}; let total = 0;
  for (const [id, us] of self) {
    const cf = byId.get(id).callFrame; const n = cf.functionName || '(anonymous)';
    if (n === '(idle)') continue;
    let c;
    if (n.startsWith('(')) c = n;
    else if (/^fb?_[0-9a-f]+$/.test(n)) c = 'guest (lifted)';
    else if (/^wasm-function\[/.test(n)) c = 'wasm (unnamed)';
    else if (/^bl::|^_ZN2bl|^BL/.test(n)) c = 'Blend2D (bl::)';
    else if (cf.url && !/wasm/.test(cf.url)) c = 'JavaScript';
    else c = 'runtime (fle_*, libc, harness)';
    cat[c] = (cat[c] ?? 0) + us; total += us;
  }
  console.log(f.split('/').slice(-2).join('/') + '  non-idle ' + (total / 1e6).toFixed(2) + ' s: ' + Object.entries(cat).sort((a, b) => b[1] - a[1]).map(([k, v]) => `${k} ${(100 * v / total).toFixed(0)}%`).join(', '));
}
