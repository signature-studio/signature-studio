// Engine cost, stage 0 aggregation: the runs of tools/engine-cost/measure.mjs (the baseline Oz, and the variants O2 and O3 in runs/O2 and runs/O3) and the contrast of tools/engine-cost/contrast/contrast.mjs, written to pm-work/findings/engine-cost-attribution.json. A run or file that is missing or unreadable leaves its values null, with the reason in notes.
// Usage: node tools/engine-cost/aggregate.mjs [--dry-run]. No argument: writes the JSON. --dry-run writes nothing and prints the JSON to stderr. stdout is the markdown table of the 18 baseline runs (one row per run).
// Exit 0 when it ran; 2 when the arguments are wrong (usage on stderr; nothing is written).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const USAGE = 'usage: node tools/engine-cost/aggregate.mjs [--dry-run]\n';
const argv = process.argv.slice(2);
if (argv.some((a) => a !== '--dry-run')) {
  process.stderr.write(USAGE);
  process.exit(2);
}
const DRY_RUN = argv.includes('--dry-run');

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const RUNS = path.join(repo, 'lift', 'out-app', 'engine-cost', 'runs');
const CONTRAST = path.join(repo, 'lift', 'out-app', 'engine-cost', 'contrast.json');
const VARIANTS = path.join(repo, 'lift', 'out-app', 'variants');
const BASE_WASM = path.join(repo, 'lift', 'out-app', 'web', 'engine.wasm');
const OUT = path.join(repo, 'pm-work', 'findings', 'engine-cost-attribution.json');
const SCENES = ['i', 'ii', 'iii'];
const DPRS = ['1.00', '1.325'];
const NS = [1, 2, 3];
const VARIANT_NAMES = ['Oz', 'O2', 'O3'];
const CLASSES = ['A', 'B', 'C', 'D', 'E', 'F', 'unclassified'];
const CALLS = 1000000;
const UNNAMED = /including the wasm functions with no name in the table \((\d+(?:\.\d+)?) ms\)/;

const nproc = os.availableParallelism?.() ?? os.cpus().length;
const loadavg = os.loadavg().map((x) => Math.round(x * 100) / 100);

const rel = (file) => path.relative(repo, file);
const num = (x) => (typeof x === 'number' && Number.isFinite(x) ? x : null);
const r1 = (x) => (x === null ? null : Math.round(x * 10) / 10);
const r3 = (x) => (x === null ? null : Math.round(x * 1000) / 1000);
const roundInt = (x) => (x === null ? null : Math.round(x));
const mean = (xs) => {
  const v = xs.filter((x) => x !== null);
  return v.length === 0 ? null : v.reduce((a, b) => a + b, 0) / v.length;
};
const count = (xs) => xs.filter((x) => x !== null).length;
const fmt = (x) => (x === null ? 'null' : String(r1(x)));
const failedOf = (json) => json.error !== null && json.error !== undefined;
const usable = (json) => (json !== null && !failedOf(json) ? json : null);

function readJson(file) {
  try {
    const json = JSON.parse(fs.readFileSync(file, 'utf8'));
    if (typeof json !== 'object' || json === null) throw new Error('not an object');
    return { json, problem: null };
  } catch (e) {
    const why = e.code === 'ENOENT' ? 'ファイルが無い' : `読めない (${String(e.message).slice(0, 200)})`;
    return { json: null, problem: `${rel(file)}: ${why}` };
  }
}

function sizeOf(file) {
  try {
    return fs.statSync(file).size;
  } catch {
    return null;
  }
}

const classesOf = (c) => (c !== null && typeof c === 'object' ? Object.fromEntries(CLASSES.map((k) => [k, r1(num(c[k]))])) : null);

const unnamedOf = (notes) => {
  const m = UNNAMED.exec(typeof notes === 'string' ? notes : '');
  return m ? r1(Number(m[1])) : null;
};

function measuresOf(json) {
  const j = usable(json);
  const at = (f) => (j === null ? null : f(j));
  return {
    busyMs: at((r) => r1(num(r.busyMs))),
    idleMs: at((r) => r1(num(r.idleMs))),
    classesMs: at((r) => classesOf(r.classesMs)),
    classifiedShare: at((r) => r3(num(r.classifiedShare))),
    atomicsWaitMs: at((r) => r1(num(r.atomicsWaitMs))),
    underrunQuanta: at((r) => num(r.underrunQuanta)),
    elapsedMs: at((r) => r1(num(r.elapsedMs))),
    unnamedWasmMs: at((r) => unnamedOf(r.notes)),
  };
}

function notesOf(json, problem) {
  if (json === null) return problem;
  const text = typeof json.notes === 'string' ? json.notes : '';
  return failedOf(json) ? [`error: ${json.error}`, text].filter(Boolean).join(' ') : text;
}

function baselineRun(scene, dprTag, n) {
  const { json, problem } = readJson(path.join(RUNS, 'Oz', `${scene}-dpr${dprTag}-n${n}.json`));
  const j = usable(json);
  const m = measuresOf(json);
  return {
    scene,
    dpr: Number(dprTag),
    n,
    busyMs: m.busyMs,
    idleMs: m.idleMs,
    classesMs: m.classesMs,
    classifiedShare: m.classifiedShare,
    boundaryCallsPerSec: j?.boundaryCallsPerSec ?? null,
    jspi: j?.jspi ?? null,
    memoryGrow: j?.memoryGrow ?? null,
    atomicsWaitMs: m.atomicsWaitMs,
    underrunQuanta: m.underrunQuanta,
    elapsedMs: m.elapsedMs,
    unnamedWasmMs: m.unnamedWasmMs,
    notes: notesOf(json, problem),
  };
}

function wasmOf(variant) {
  if (variant === 'Oz') {
    const bytes = sizeOf(BASE_WASM);
    return { bytes, note: bytes === null ? `wasmBytes: ${rel(BASE_WASM)} が無い` : `wasmBytes は ${rel(BASE_WASM)} の大きさ` };
  }
  const file = path.join(VARIANTS, variant, 'variant.json');
  const { json, problem } = readJson(file);
  if (json === null) return { bytes: null, note: `wasmBytes: ${problem}` };
  const bytes = num(json.engineWasmBytes);
  if (bytes === null) return { bytes: null, note: `wasmBytes: ${rel(file)} に engineWasmBytes が無い` };
  const web = sizeOf(path.join(VARIANTS, variant, 'web', 'engine.wasm'));
  const check = web === null ? '(web/engine.wasm は無い)' : web === bytes ? '(web/engine.wasm と一致)' : `(web/engine.wasm は ${web} bytes で不一致)`;
  return { bytes, note: `wasmBytes は ${rel(file)} の engineWasmBytes ${check}` };
}

function contrastOf() {
  if (!fs.existsSync(CONTRAST)) {
    return { calls: CALLS, nsPerCallEmscripten: null, nsPerCallDirect: null, notes: 'contrast.json が無い。tools/engine-cost/contrast/contrast.mjs が実行されていない' };
  }
  const { json, problem } = readJson(CONTRAST);
  if (json === null) return { calls: CALLS, nsPerCallEmscripten: null, nsPerCallDirect: null, notes: problem };
  const failed = failedOf(json);
  return {
    calls: num(json.calls) ?? CALLS,
    nsPerCallEmscripten: failed ? null : num(json.nsPerCallEmscripten),
    nsPerCallDirect: failed ? null : num(json.nsPerCallDirect),
    notes: failed ? [`error: ${json.error}`, json.notes].filter(Boolean).join(' ') : String(json.notes ?? ''),
  };
}

function busyLine(label, xs) {
  const v = xs.filter((x) => x !== null);
  const min = v.length === 0 ? null : Math.min(...v);
  const max = v.length === 0 ? null : Math.max(...v);
  const spread = v.length === 0 ? null : max - min;
  const runs = NS.map((n, k) => `n=${n} ${fmt(xs[k])}`).join(', ');
  return `${label} (DPR 1.00) の busyMs: ${runs}; 平均 ${fmt(mean(xs))}, 最小 ${fmt(min)}, 最大 ${fmt(max)}, 最大 - 最小 ${fmt(spread)}。`;
}

function optLevel(variant) {
  const dir = path.join(RUNS, variant);
  const busy = { iii: [], i: [] };
  const pooled = { underrunQuanta: [], elapsedMs: [], preBootBytes: [] };
  const issues = [];
  for (const scene of ['iii', 'i']) {
    for (const n of NS) {
      const { json, problem } = readJson(path.join(dir, `${scene}-dpr1.00-n${n}.json`));
      const j = usable(json);
      if (problem !== null) issues.push(problem);
      else if (failedOf(json)) issues.push(`${scene} n=${n}: error ${json.error}`);
      busy[scene].push(j === null ? null : num(j.busyMs));
      for (const k of Object.keys(pooled)) pooled[k].push(j === null ? null : num(j[k]));
    }
  }
  const wasm = wasmOf(variant);
  const notes = [
    busyLine('場面 iii', busy.iii),
    busyLine('場面 i', busy.i),
    `busyMs は場面 iii の平均。underrunQuanta・elapsedMs・preBootBytes は場面 i と iii の DPR 1.00 の 6 run の平均(値が null の run は外す。有効数は underrunQuanta ${count(pooled.underrunQuanta)}、elapsedMs ${count(pooled.elapsedMs)}、preBootBytes ${count(pooled.preBootBytes)})。`,
    `${wasm.note}。`,
    `nproc ${nproc}、loadavg ${loadavg.join(' ')} は集計時点の値(計測の run の値ではない)。`,
    issues.length > 0 ? `読めなかった run: ${issues.join('; ')}。` : '',
  ];
  return {
    variant,
    busyMs: r1(mean(busy.iii)),
    underrunQuanta: r1(mean(pooled.underrunQuanta)),
    elapsedMs: r1(mean(pooled.elapsedMs)),
    wasmBytes: wasm.bytes,
    preBootBytes: roundInt(mean(pooled.preBootBytes)),
    notes: notes.filter(Boolean).join(' '),
  };
}

function table(rows) {
  const head = ['scene', 'DPR', 'n', ...CLASSES.map((k) => `${k} ms`), 'idle ms', 'classifiedShare'];
  const cell = (x) => (x === null || x === undefined ? '-' : String(x));
  const dpr = (d) => (d === 1 ? '1.00' : String(d));
  const body = rows.map((r) => [r.scene, dpr(r.dpr), cell(r.n), ...CLASSES.map((k) => cell(r.classesMs?.[k])), cell(r.idleMs), cell(r.classifiedShare)]);
  return [head, head.map(() => '---'), ...body].map((cells) => `| ${cells.join(' | ')} |`).join('\n');
}

const runs = [];
for (const scene of SCENES) for (const dprTag of DPRS) for (const n of NS) runs.push(baselineRun(scene, dprTag, n));
const doc = {
  nproc,
  loadavg,
  runs,
  contrast: contrastOf(),
  optLevels: VARIANT_NAMES.map(optLevel),
};

const text = `${JSON.stringify(doc, null, 2)}\n`;
if (DRY_RUN) {
  process.stderr.write(`dry run: not written (${rel(OUT)})\n${text}`);
} else {
  fs.mkdirSync(path.dirname(OUT), { recursive: true });
  fs.writeFileSync(OUT, text);
  process.stderr.write(`wrote ${rel(OUT)}\n`);
}
process.stdout.write(`${table(runs)}\n`);
