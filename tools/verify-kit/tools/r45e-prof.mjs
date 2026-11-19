// R4-5e: reads the V8 CPU profiles of profiled P3 runs (out/r45e-mixprof-<label>-chrome-<n>/r45e-p3-w<k>.cpuprofile, one per Worker of the page; wall seconds and the Worker CPU from report.json profs[0]) and prints per run the Worker thread time split into idle and busy, and the functions with the largest self time (all Workers together, ms of thread time per wall second). With several runs a table puts them side by side. Reads only; writes markdown to stdout.
// --funcs <table.tsv>: the table of r45e-wasmfuncs.awk for the module the runs used; the wasm-function[N] rows get their atomic and call counts. Under each run, the (program) time is also split by the function that called it (the parent frame).
// Usage: node tools/verify-kit/tools/r45e-prof.mjs <run dir> [<run dir> ...] [--top N] [--funcs table.tsv]
import fs from 'node:fs';
import path from 'node:path';

const IDLE_NAMES = new Set(['(idle)']);
const args = process.argv.slice(2);
let top = 25;
let funcsPath = null;
const dirs = [];
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--top') top = Number(args[++i]);
  else if (args[i] === '--funcs') funcsPath = args[++i];
  else dirs.push(args[i]);
}
if (!dirs.length || !Number.isFinite(top)) {
  process.stderr.write('usage: node tools/verify-kit/tools/r45e-prof.mjs <run dir> [<run dir> ...] [--top N] [--funcs table.tsv]\n');
  process.exit(2);
}
const funcs = new Map();
if (funcsPath) {
  const lines = fs.readFileSync(funcsPath, 'utf8').trim().split('\n').slice(1);
  for (const l of lines) {
    const [index, name, instr, wait32, notify, cmpxchg, calls, importCalls] = l.split('\t');
    funcs.set(index, { name, instr: Number(instr), wait32: Number(wait32), notify: Number(notify), cmpxchg: Number(cmpxchg), calls: Number(calls), importCalls: Number(importCalls) });
  }
}
const annotate = (key) => {
  const m = /^wasm-function\[(\d+)\]$/.exec(key);
  const f = m && funcs.get(m[1]);
  return f ? `${key} (instr ${f.instr}, wait32 ${f.wait32}, notify ${f.notify}, cmpxchg ${f.cmpxchg}, calls ${f.calls}, import calls ${f.importCalls})` : key;
};

function fnKey(cf) {
  if (cf.functionName) return cf.functionName;
  return `(anonymous ${path.basename(cf.url || '')}:${cf.lineNumber})`;
}

// V8 timeDeltas[i] is the time between sample i-1 and sample i, so sample i lasted about timeDeltas[i+1] microseconds.
function profileWeights(profile) {
  const byNode = new Map();
  const s = profile.samples;
  const d = profile.timeDeltas;
  for (let i = 0; i < s.length; i++) {
    const w = i + 1 < d.length ? d[i + 1] : 0;
    byNode.set(s[i], (byNode.get(s[i]) ?? 0) + w);
  }
  return byNode;
}

function analyse(dir) {
  const name = path.basename(dir);
  const reportPath = path.join(dir, 'report.json');
  const report = fs.existsSync(reportPath) ? JSON.parse(fs.readFileSync(reportPath, 'utf8')) : null;
  const prof = report ? (report[0] ?? report).profs?.[0] : null;
  const files = fs.readdirSync(dir).filter((f) => /^r45e-p3-w\d+\.cpuprofile$/.test(f)).sort();
  if (!files.length) throw new Error(`${name}: no r45e-p3-w<k>.cpuprofile in the run folder`);
  const fns = new Map();
  const programBy = new Map();
  const perWorker = [];
  let idleUs = 0;
  let busyUs = 0;
  let spanUs = 0;
  for (const f of files) {
    const profile = JSON.parse(fs.readFileSync(path.join(dir, f), 'utf8'));
    const nodes = new Map(profile.nodes.map((n) => [n.id, n]));
    const parent = new Map();
    for (const n of profile.nodes) for (const c of n.children ?? []) parent.set(c, n.id);
    let wIdle = 0;
    let wBusy = 0;
    for (const [id, us] of profileWeights(profile)) {
      const node = nodes.get(id);
      const key = node ? fnKey(node.callFrame) : '(unknown)';
      if (IDLE_NAMES.has(key)) {
        wIdle += us;
        continue;
      }
      wBusy += us;
      fns.set(key, (fns.get(key) ?? 0) + us);
      if (key === '(program)') {
        const p = parent.get(id);
        const pk = p !== undefined && nodes.get(p) ? fnKey(nodes.get(p).callFrame) : '(root)';
        programBy.set(pk, (programBy.get(pk) ?? 0) + us);
      }
    }
    idleUs += wIdle;
    busyUs += wBusy;
    spanUs = Math.max(spanUs, (profile.endTime - profile.startTime));
    perWorker.push({ file: f, busyS: wBusy / 1e6, idleS: wIdle / 1e6, samples: profile.samples.length });
  }
  const wallS = prof?.wallS ?? spanUs / 1e6;
  const perS = (us) => (us / 1e6) / wallS * 1000;
  const ranked = [...fns.entries()].sort((a, b) => b[1] - a[1]);
  return {
    name,
    wallS,
    workerCpuS: prof?.cpuS?.worker ?? null,
    workerCountProfiled: files.length,
    busyS: busyUs / 1e6,
    idleS: idleUs / 1e6,
    busyMsPerS: perS(busyUs),
    perWorker,
    ranked: ranked.slice(0, top).map(([k, us]) => ({ key: k, msPerS: perS(us), shareOfBusy: us / busyUs })),
    programTop: [...programBy.entries()].sort((a, b) => b[1] - a[1]).slice(0, 12).map(([k, us]) => ({ key: k, msPerS: perS(us) })),
    msPerSByKey: new Map(ranked.map(([k, us]) => [k, perS(us)])),
  };
}

const runs = dirs.map(analyse);
const out = [];
out.push('## Per run (thread time of the Worker threads in the profiled window)\n');
out.push('| run | wall s | Worker CPU s (cpuS) | profiled Workers | busy s | idle s | busy ms per wall s |');
out.push('|---|---:|---:|---:|---:|---:|---:|');
for (const r of runs) out.push(`| ${r.name} | ${r.wallS.toFixed(1)} | ${r.workerCpuS ?? '-'} | ${r.workerCountProfiled} | ${r.busyS.toFixed(1)} | ${r.idleS.toFixed(1)} | ${r.busyMsPerS.toFixed(1)} |`);
out.push('');
const keys = new Set();
for (const r of runs) for (const e of r.ranked) keys.add(e.key);
const maxOf = (k) => Math.max(...runs.map((r) => r.msPerSByKey.get(k) ?? 0));
const union = [...keys].sort((a, b) => maxOf(b) - maxOf(a)).slice(0, top);
out.push(`## Self time by function (ms of thread time per wall second, all Workers; top ${top} by the largest run)\n`);
out.push(`| function | ${runs.map((r) => r.name).join(' | ')} |`);
out.push(`|---|${runs.map(() => '---:').join('|')}|`);
for (const k of union) out.push(`| ${annotate(k).replace(/\|/g, '/')} | ${runs.map((r) => (r.msPerSByKey.get(k) ?? 0).toFixed(2)).join(' | ')} |`);
out.push('');
for (const r of runs) {
  out.push(`### ${r.name}: (program) time by the function that called it (ms of thread time per wall second)\n`);
  out.push('| caller | ms per wall s |');
  out.push('|---|---:|');
  for (const p of r.programTop) out.push(`| ${annotate(p.key).replace(/\|/g, '/')} | ${p.msPerS.toFixed(2)} |`);
  out.push('');
}
for (const r of runs) {
  out.push(`### ${r.name}: per Worker file (busy s, idle s)\n`);
  out.push('| file | busy s | idle s | samples |');
  out.push('|---|---:|---:|---:|');
  for (const w of r.perWorker) out.push(`| ${w.file} | ${w.busyS.toFixed(2)} | ${w.idleS.toFixed(2)} | ${w.samples} |`);
  out.push('');
}
process.stdout.write(out.join('\n') + '\n');
