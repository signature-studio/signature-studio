// Measurement aid (investigation only): ask V8 (node) whether it accepts a wasm object file (relocatable .o or linked .wasm).
// usage: node v8_validate.cjs FILE [FILE ...]      prints: file, byte size, accepted or the CompileError message (cut to 200 chars).
// A relocatable .o has the linking/reloc custom sections (ignored by V8), imports memory/table from env, and padded LEBs, so a
// compile of the bytes exercises V8's per-function checks (locals count, body size) without linking.
const fs = require('fs');
for (const f of process.argv.slice(2)) {
  const b = fs.readFileSync(f);
  let r;
  const t0 = process.hrtime.bigint();
  try { new WebAssembly.Module(b); r = 'accepted'; } catch (e) { r = String(e.message).slice(0, 200); }
  const ms = Number(process.hrtime.bigint() - t0) / 1e6;
  console.log(`${f}\t${b.length} B\t${ms.toFixed(0)} ms\t${r}`);
}
