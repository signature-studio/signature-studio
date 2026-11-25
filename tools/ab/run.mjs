// A/B measurement (#1422): the base build and the changed build are measured in turns (base, changed = one pair; n pairs), each run by the tool of its kind: p2 is tools/p2-frames/measure.mjs, p3 is tools/p3-audio-probe/measure-live.mjs. The medians are compared and the gates decide the pass.
// Usage: node tools/ab/run.mjs <p2|p3> <base bundle raw/<name>> <base app dir> <changed bundle raw/<name>> <changed app dir> [n=3] > ab.json
// stdout: ONE JSON line {kind,n,base,changed,runs,medians,gates,pass}. Each run's output is kept in lift/out-app/ab/ (git-ignored). Exit 0 when pass is true, 1 when it is not, 2 when the arguments are wrong.
// p2 gates: every run has error null and inputRateOk true; the changed median minWindowFps is at least 0.90 x the base median; the changed median inputToPresentMsP95 is at most 1.20 x the base median.
// p3 gates (every changed run): error null, playing true, underrunDelta 0, last60sUnderruns 0, inputRateOk true, nonSilentDelta above 0, pcmRepeat8192 null or below 0.01.
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

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

// the median of the values; null when one of them is not a finite number (a run without the value cannot be compared)
function median(values) {
  if (values.some((x) => typeof x !== 'number' || !Number.isFinite(x))) return null;
  const v = [...values].sort((a, b) => a - b);
  const m = v.length >> 1;
  return v.length % 2 === 1 ? v[m] : (v[m - 1] + v[m]) / 2;
}
const MEDIAN_KEYS = { p2: ['minWindowFps', 'inputToPresentMsP95', 'inputDropped'], p3: ['pcmRepeat8192', 'underrunDelta', 'nonSilentDelta'] };
const mediansOf = (runs) => Object.fromEntries(MEDIAN_KEYS[kind].map((k) => [k, median(runs.map((r) => r[k]))]));

const runs = { base: [], changed: [] };
for (let i = 1; i <= n; i++) {
  runs.base.push(runOnce('base', i));
  runs.changed.push(runOnce('changed', i));
}
const medians = { base: mediansOf(runs.base), changed: mediansOf(runs.changed) };

const gates = [];
const gate = (name, pass, values) => gates.push({ name, pass: pass === true, values });
if (kind === 'p2') {
  const all = [...runs.base, ...runs.changed];
  gate('every run: error is null', all.every((r) => r.error === null), all.map((r) => r.error));
  gate('every run: inputRateOk is true', all.every((r) => r.inputRateOk === true), all.map((r) => r.inputRateOk));
  const mw = medians.changed.minWindowFps, bw = medians.base.minWindowFps;
  gate('minWindowFps: changed median >= 0.90 x base median', mw !== null && bw !== null && mw >= bw * 0.9, { base: bw, changed: mw });
  const mp = medians.changed.inputToPresentMsP95, bp = medians.base.inputToPresentMsP95;
  gate('inputToPresentMsP95: changed median <= 1.20 x base median', mp !== null && bp !== null && mp <= bp * 1.2, { base: bp, changed: mp });
} else {
  const ch = runs.changed;
  // each gate: its test on every changed run, and the value of each run that it shows
  const each = (name, key, ok) => gate(`every changed run: ${name}`, ch.length > 0 && ch.every(ok), ch.map((r) => r[key]));
  each('error is null', 'error', (r) => r.error === null);
  each('playing is true', 'playing', (r) => r.playing === true);
  each('underrunDelta is 0', 'underrunDelta', (r) => r.underrunDelta === 0);
  each('last60sUnderruns is 0', 'last60sUnderruns', (r) => r.last60sUnderruns === 0);
  each('inputRateOk is true', 'inputRateOk', (r) => r.inputRateOk === true);
  each('nonSilentDelta is above 0', 'nonSilentDelta', (r) => typeof r.nonSilentDelta === 'number' && r.nonSilentDelta > 0);
  each('pcmRepeat8192 is null or below 0.01', 'pcmRepeat8192', (r) => r.pcmRepeat8192 === null || (typeof r.pcmRepeat8192 === 'number' && r.pcmRepeat8192 < 0.01));
}
const pass = gates.every((g) => g.pass);
const result = { kind, n, base: sides.base, changed: sides.changed, runs, medians, gates, pass };
process.stdout.write(JSON.stringify(result) + '\n');
process.exitCode = pass ? 0 : 1;
