// Stack measurements of the JSPI thread switch (decision 0124, PROVISIONAL), under node. Each probe is one process of run_node_scen.cjs running `deep <depth> <nthreads> <mask> <frame>` of
// thread_scenarios.c: thread i recurses `depth` C frames when bit i of <mask> is set (one frame otherwise), SwitchToThread at the bottom (so every deep thread is at full depth on its own
// JSPI stack at the same time), then returns. A probe is "ok" when the exit status is 0 and all `nthreads` "deep ... ok" lines are printed; else the failing thread index is read from the
// runner's message ("[js] fle thread N failed: ..." or the main thread's exception).
// usage: node measure_stack.mjs <wasm dir> <out.tsv> [jspi stack kB list, e.g. 984,150,145]
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const wasmDir = path.resolve(process.argv[2]);
const outTsv = process.argv[3];
const sizes = (process.argv[4] || '984').split(',').map(Number);
const rows = [];
const log = (s) => { console.log(s); };

function probe(kb, depth, n, mask, frame) {
  const args = ['--experimental-wasm-jspi', `--wasm-stack-switching-stack-size=${kb}`, path.join(HERE, 'run_node_scen.cjs'), wasmDir, 'deep', String(depth), String(n), '0x' + mask.toString(16), String(frame)];
  const r = spawnSync('node', args, { encoding: 'utf8', timeout: 120000 });
  const out = (r.stdout || '') + (r.stderr || '');
  const oks = (out.match(/^thread \d+ deep \d+ ok$/gm) || []).length;
  const failed = [...out.matchAll(/\[js\] fle thread (\d+) failed: ([^\n|]*)/g)].map((m) => `${m[1]}:${m[2].trim().slice(0, 60)}`);
  const mainFail = /wasm trap\/exception: ([^\n]*)/.exec(out);
  const ok = r.status === 0 && oks === n;
  return { ok, status: r.status, oks, failed, mainFail: mainFail ? mainFail[1].slice(0, 60) : null, out };
}
function maxDepth(kb, n, mask, frame, hi0 = 400000) {
  let lo = 1, hi = hi0;           // lo: ok, hi: not ok (assumed)
  if (!probe(kb, lo, n, mask, frame).ok) return 0;
  while (probe(kb, hi, n, mask, frame).ok) { lo = hi; hi *= 2; if (hi > 6.4e6) return -1; }
  while (hi - lo > 1) {
    const mid = Math.floor((lo + hi) / 2);
    if (probe(kb, mid, n, mask, frame).ok) lo = mid; else hi = mid;
  }
  return lo;
}
function rec(kb, what, n, mask, frame, depthMax, extra = '') {
  rows.push([kb, what, n, '0x' + mask.toString(16), frame, depthMax, extra].join('\t'));
  log(`jspi=${kb}kB ${what} threads=${n} mask=0x${mask.toString(16)} frame=${frame}: max depth ${depthMax} ${extra}`);
}

rows.push(['jspi_stack_kB', 'case', 'threads', 'deep_mask', 'frame', 'max_depth_ok', 'note'].join('\t'));
for (const kb of sizes) {
  // 1. one deep thread, the others shallow: which thread index is deep does not matter if the limit is per stack (thread 0 = main, also a promising stack)
  for (const idx of [0, 1, 2]) rec(kb, `single deep thread ${idx}`, 3, 1 << idx, 16, maxDepth(kb, 3, 1 << idx, 16));
  // 2. N threads ALL deep at the same moment: per-thread limit => the same max depth for any N; a shared limit => it would fall with N
  for (const n of [1, 2, 4, 8, 16]) rec(kb, 'all deep', n, (1 << n) - 1, 16, maxDepth(kb, n, (1 << n) - 1, 16));
  // 3. one thread one frame beyond ITS OWN limit while the others stay shallow: only that thread must fail, and the report must name it (thread 0 has its own limit)
  for (const idx of [0, 1, 2]) {
    const d = maxDepth(kb, 3, 1 << idx, 16);
    const p = probe(kb, d + 1, 3, 1 << idx, 16);
    rec(kb, `thread ${idx} one frame beyond its own limit`, 3, 1 << idx, 16, d + 1, `ok=${p.ok} oks=${p.oks} failed=[${p.failed.join(';')}] main=${p.mainFail}`);
  }
}
fs.writeFileSync(outTsv, rows.join('\n') + '\n');
