// draw1: print the call tree (inclusive/self ms, wasm function indices) of a V8 .cpuprofile, pruned at MINMS inclusive.
import fs from 'node:fs';
const [f, minms = '100', maxdepth = '30'] = process.argv.slice(2);
const p = JSON.parse(fs.readFileSync(f, 'utf8'));
const byId = new Map(p.nodes.map((n) => [n.id, n]));
const self = new Map();
for (let i = 0; i < p.samples.length; i++) self.set(p.samples[i], (self.get(p.samples[i]) ?? 0) + (p.timeDeltas[i] ?? 0));
const incl = new Map();
function calc(n) { let t = self.get(n.id) ?? 0; for (const c of n.children ?? []) t += calc(byId.get(c)); incl.set(n.id, t); return t; }
calc(p.nodes[0]);
function pr(n, d) {
  const i = incl.get(n.id) / 1000; if (i < +minms || d > +maxdepth) return;
  const cf = n.callFrame;
  console.log(`${' '.repeat(d)}${i.toFixed(0)} ms (self ${((self.get(n.id) ?? 0) / 1000).toFixed(0)}) ${cf.functionName || '(anon)'} ${cf.url.split('/').pop()}:${cf.lineNumber}`);
  for (const c of (n.children ?? []).map((x) => byId.get(x)).sort((a, b) => incl.get(b.id) - incl.get(a.id))) pr(c, d + 1);
}
pr(p.nodes[0], 0);
