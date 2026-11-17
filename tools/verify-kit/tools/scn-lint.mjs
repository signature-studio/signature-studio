// scn-lint.mjs (ci1, 0321): a parse-only check of scenario files against the ops a driver script supports. Nothing is run: no browser, no server.
// usage: node scn-lint.mjs DRIVER.mjs FILE.json [FILE.json ...]   exit 1 if a scenario names an op that the driver has no `case` for, or a file does not parse
import fs from 'node:fs';
const [driver, ...files] = process.argv.slice(2);
const src = fs.readFileSync(driver, 'utf8');
const known = new Set([...src.matchAll(/case '([A-Za-z0-9]+)'/g)].map((m) => m[1]));
let bad = 0, nOps = 0;
const walk = (ops, where) => {
  for (const [i, op] of ops.entries()) {
    nOps++;
    if (!op || typeof op.op !== 'string') { console.log(`${where}[${i}]: no "op" string`); bad++; continue; }
    if (!known.has(op.op)) { console.log(`${where}[${i}]: unknown op "${op.op}"`); bad++; }
    if (Array.isArray(op.ops)) walk(op.ops, `${where}[${i}].ops`);
  }
};
for (const f of files) {
  let d;
  try { d = JSON.parse(fs.readFileSync(f, 'utf8')); } catch (e) { console.log(`${f}: does not parse: ${e.message}`); bad++; continue; }
  if (!Array.isArray(d)) { console.log(`${f}: not an array of scenarios`); bad++; continue; }
  for (const sc of d) { if (typeof sc.name !== 'string' || !Array.isArray(sc.ops)) { console.log(`${f}: a scenario without name/ops`); bad++; continue; } walk(sc.ops, `${f}:${sc.name}`); }
  console.log(`${f}: ${d.length} scenario(s) checked`);
}
console.log(`${known.size} ops known to ${driver}; ${nOps} ops checked; ${bad} problem(s)`);
process.exit(bad ? 1 : 0);
