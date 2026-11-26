// A/B measurement (#1422): the base build and the changed build are measured in turns (base, changed = one pair; n pairs), each run by the tool of its kind: p2 is tools/p2-frames/measure.mjs, p3 is tools/p3-audio-probe/measure-live.mjs. The medians are compared and the gates decide the pass.
// The base tree is prepared by tools/ab/base.mjs (the acceptance does it before the builds; its inputs are checked with --verify after them).
// Usage: node tools/ab/run.mjs <p2|p3> <base bundle raw/<name>> <base app dir> <changed bundle raw/<name>> <changed app dir> [n=3] > ab.json
// stdout: ONE JSON line {kind,n,base,changed,runs,medians,gates,pass}. medians: {base, changed} (each key's median per side), baseMax (each key's largest base run) and, per key, {base, after}. Each run's output is kept in lift/out-app/ab/ (git-ignored). Exit 0 when pass is true, 1 when it is not, 2 when the arguments are wrong.
// p2 gates: every run has error null and inputRateOk true; the changed median minWindowFps is at least 0.90 x the base median; the changed median inputToPresentMsP95 is at most 1.20 x the base median.
// p3 gates (every changed run): error null, playing true, inputRateOk true, nonSilentDelta above 0, pcmRepeat8192 null or below 0.01; and the changed medians underrunDelta and last60sUnderruns at most the largest base run + 4 (not 0, and not the base median: the same build gives 4 to 14 underruns across runs on a loaded runner, #1426).
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { mediansOf, gatesOf } from './gates.mjs';

const USAGE = 'usage: node tools/ab/run.mjs <p2|p3> <base bundle raw/<name>> <base app dir> <changed bundle raw/<name>> <changed app dir> [n=3]\n';
const argv = process.argv.slice(2);
const [kind, baseBundleArg, baseAppArg, changedBundleArg, changedAppArg, nText = '3'] = argv;
const n = Number(nText);
if (argv.length < 5 || argv.length > 6 || !['p2', 'p3'].includes(kind) || !Number.isInteger(n) || n < 1) {
  process.stderr.write(USAGE);
  process.exit(2);
}
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const outDir = path.join(repo, 'lift', 'out-app', 'ab');
const TOOL = { p2: path.join(repo, 'tools', 'p2-frames', 'measure.mjs'), p3: path.join(repo, 'tools', 'p3-audio-probe', 'measure-live.mjs') };
const sides = {
  base: { bundle: path.resolve(baseBundleArg), app: path.resolve(baseAppArg) },
  changed: { bundle: path.resolve(changedBundleArg), app: path.resolve(changedAppArg) },
};
for (const [label, s] of Object.entries(sides)) {
  if (!fs.existsSync(s.bundle) || !fs.existsSync(path.join(s.app, 'package.json'))) {
    process.stderr.write(`ab: the ${label} bundle or app is not there (${s.bundle}, ${s.app})\n`);
    process.exit(2);
  }
}
const runArgs = {
  p2: (s) => [s.bundle, '--dpr', '1.75', '--load', 'a', '--app', s.app],
  p3: (s) => [s.bundle, '--dpr', '1.75', '--play', '1', '--hz', '1000', '--mouse', '20', '--sound', '--app', s.app],
};

// one run: its JSON line is the result (a run that gives no JSON has the error that says so)
function runOnce(label, index) {
  const r = spawnSync(process.execPath, [TOOL[kind], ...runArgs[kind](sides[label])], { cwd: repo, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, stdio: ['ignore', 'pipe', 'inherit'] });
  const lines = (r.stdout ?? '').split('\n').filter((l) => l.trim() !== '');
  let result;
  try { result = JSON.parse(lines[lines.length - 1] ?? ''); } catch { result = { error: `no JSON from the run (exit ${r.status}, signal ${r.signal}): ${(r.stdout ?? '').slice(0, 200)}` }; }
  fs.mkdirSync(outDir, { recursive: true });
  fs.writeFileSync(path.join(outDir, `${kind}-${label}-${index}.json`), JSON.stringify({ kind, label, index, exit: r.status, result }, null, 2) + '\n');
  process.stderr.write(`ab: ${kind} ${label} ${index}: exit ${r.status}, error ${result.error ?? 'null'}\n`);
  return result;
}

const runs = { base: [], changed: [] };
for (let i = 1; i <= n; i++) {
  runs.base.push(runOnce('base', i));
  runs.changed.push(runOnce('changed', i));
}
const medians = mediansOf(kind, runs.base, runs.changed);
const gates = gatesOf(kind, runs, medians);
const pass = gates.every((g) => g.pass);
const result = { kind, n, base: sides.base, changed: sides.changed, runs, medians, gates, pass };
process.stdout.write(JSON.stringify(result) + '\n');
process.exitCode = pass ? 0 : 1;
