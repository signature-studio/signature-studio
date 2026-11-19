// R4-5e: the Worker CPU of P3 runs from report.json (profs[].cpuS, written by the cpuStop op of sdriveg2-lat-tp.mjs): per run the wall seconds, the Worker threads' CPU (user + system) and its system part, each Worker thread's CPU (largest first) and the rest of the Chrome process (other threads, compositor, main). Reads only; writes TSV to stdout.
// Usage: node tools/verify-kit/tools/r45e-cpu.mjs <run dir> [<run dir> ...]
import fs from 'node:fs';
import path from 'node:path';

const dirs = process.argv.slice(2);
if (!dirs.length) {
  process.stderr.write('usage: node tools/verify-kit/tools/r45e-cpu.mjs <run dir> [<run dir> ...]\n');
  process.exit(2);
}
const rows = ['run\tprofile\twallS\tworker_cpu_s\tworker_sys_s\tworker_user_s\tworkers_n\tworkers_cpu_s\tworkers_sys_s\tother_s\tcompositor_s\tmain_s'];
for (const d of dirs) {
  const r = JSON.parse(fs.readFileSync(path.join(d, 'report.json'), 'utf8'));
  const rep = Array.isArray(r) ? r[0] : r;
  for (const p of rep.profs ?? []) {
    const s = p.cpuS ?? {};
    const w = s.worker ?? 0;
    const sys = s.workerSys ?? 0;
    rows.push([path.basename(d), p.name, p.wallS, w, sys, (w - sys).toFixed(2), (s.workers ?? []).length, (s.workers ?? []).join(','), (s.workersSys ?? []).join(','), s.other ?? '', s.compositor ?? '', s.main ?? ''].join('\t'));
  }
}
process.stdout.write(rows.join('\n') + '\n');
