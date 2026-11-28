#!/usr/bin/env node
// Coverage of the features (Issue #1485): the features are the TSV rows of tools/e2e/features/ (the hand-written <area>.tsv) and of tools/e2e/features/generated/ (the inventory),
// and a feature is covered when a scenario that lists it in `covers` passed (ok true, not skipped, no expectFail mark) in one of the runs.
// Usage: node tools/e2e/coverage.mjs --features tools/e2e/features --run e2e.jsonl [--run more.jsonl ...] --out coverage.json
//   --features: the directory of the features: <area>.tsv, generated/<area>.tsv (or any *.tsv in generated/), and <area>.min (the minimums: `scenarios<TAB>n` and `coverage<TAB>r`).
//   --run: a run's output (JSON lines, one row per display; each row's scenarios carry id, covers, ok, expectFail, skipped). Repeat it for every shard.
// Output (--out, or stdout): { total, covered, unknownCovers, perArea: { <area>: { features, covered, ratio, scenarios, min } } }. scenarios: the distinct passing scenarios of the area.
// A feature ID that appears twice (in any files) is an error: the IDs are the keys of the coverage.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { AREAS, areaOf } from './lib/scenario.mjs';

/** the rows of a TSV text: an array of objects keyed by the header line (empty lines and `#` lines are skipped) */
export function parseTsv(text) {
  const lines = String(text).split(/\r?\n/).filter((l) => l.length > 0 && !l.startsWith('#'));
  if (lines.length === 0) return [];
  const head = lines[0].split('\t');
  return lines.slice(1).map((l) => {
    const cells = l.split('\t');
    return Object.fromEntries(head.map((h, i) => [h, cells[i] ?? '']));
  });
}

/** the minimums of an area: `scenarios<TAB>n` and `coverage<TAB>r` lines */
export function parseMin(text) {
  const out = {};
  for (const l of String(text).split(/\r?\n/)) {
    const [k, v] = l.split('\t');
    if (k && v !== undefined && v.trim() !== '') out[k.trim()] = Number(v);
  }
  if (!Number.isFinite(out.scenarios) || !Number.isFinite(out.coverage)) throw new Error(`a .min needs the lines scenarios and coverage (got ${JSON.stringify(out)})`);
  return { scenarios: out.scenarios, coverage: out.coverage };
}

/** a scenario result passes its own rule: it ran, is not skipped, and is ok (a mark scenario counts as passing only when it is ok too) */
export function passing(s) {
  return s.skipped !== true && s.ok === true;
}

/**
 * the coverage. features: [{ id, area }]; rows: the runs' rows (each { scenarios: [...] }); mins: { <area>: { scenarios, coverage } } (optional).
 * A feature is covered when a passing scenario with no expectFail mark lists it in covers (the mark scenarios are not counted).
 */
export function computeCoverage({ features, rows, mins = {} }) {
  const ids = new Set(features.map((f) => f.id));
  const covered = new Set();
  const passCount = new Map(AREAS.map((a) => [a, new Set()]));
  let unknownCovers = 0;
  for (const row of rows) {
    for (const s of row.scenarios ?? []) {
      if (!passing(s)) continue;
      if (s.area && passCount.has(s.area)) passCount.get(s.area).add(s.id ?? s.name);
      if (s.expectFail !== null && s.expectFail !== undefined) continue;
      for (const c of s.covers ?? []) {
        if (ids.has(c)) covered.add(c);
        else unknownCovers++;
      }
    }
  }
  const perArea = {};
  for (const a of AREAS) {
    const fs_ = features.filter((f) => f.area === a);
    const cov = fs_.filter((f) => covered.has(f.id)).length;
    perArea[a] = { features: fs_.length, covered: cov, ratio: fs_.length ? round(cov / fs_.length) : 0, scenarios: passCount.get(a).size, min: mins[a] ?? null };
  }
  return { total: features.length, covered: covered.size, unknownCovers, perArea };
}

const round = (v) => Math.round(v * 10000) / 10000;

/** reads the features of a directory: <area>.tsv and generated/*.tsv; each row needs an id of its own area. Throws on a duplicate ID */
export function readFeatures(dir) {
  const files = [];
  for (const f of fs.readdirSync(dir).sort()) if (f.endsWith('.tsv')) files.push(path.join(dir, f));
  const gen = path.join(dir, 'generated');
  if (fs.existsSync(gen)) for (const f of fs.readdirSync(gen).sort()) if (f.endsWith('.tsv')) files.push(path.join(gen, f));
  const features = [], seen = new Map();
  for (const file of files) {
    for (const row of parseTsv(fs.readFileSync(file, 'utf8'))) {
      const id = String(row.id ?? '');
      const area = areaOf(id);
      if (!area) throw new Error(`${path.relative(dir, file)}: the id ${JSON.stringify(id)} is not of a known area (${AREAS.join(', ')})`);
      if (seen.has(id)) throw new Error(`duplicate feature id ${id}: in ${seen.get(id)} and ${path.relative(dir, file)}`);
      seen.set(id, path.relative(dir, file));
      features.push({ id, area, kind: row.kind ?? '', name: row.name ?? '', file: path.relative(dir, file) });
    }
  }
  return features;
}

/** the minimums of the areas: <dir>/<area>.min (an area with no file has none) */
export function readMins(dir) {
  const mins = {};
  for (const a of AREAS) {
    const f = path.join(dir, `${a}.min`);
    if (fs.existsSync(f)) mins[a] = parseMin(fs.readFileSync(f, 'utf8'));
  }
  return mins;
}

function parseCli(argv) {
  const a = { run: [] };
  for (let i = 0; i < argv.length; i++) {
    const t = argv[i];
    const v = argv[++i];
    if (!t.startsWith('--') || v === undefined) throw new Error(`usage: coverage.mjs --features DIR --run FILE [--run FILE ...] [--out FILE] (bad argument ${t})`);
    if (t === '--run') a.run.push(v);
    else a[t.slice(2)] = v;
  }
  return a;
}

export function runRows(files) {
  const rows = [];
  for (const f of files) {
    for (const l of fs.readFileSync(f, 'utf8').split('\n')) if (l.trim()) rows.push(JSON.parse(l));
  }
  return rows;
}

async function main() {
  const a = parseCli(process.argv.slice(2));
  if (!a.features) throw new Error('--features DIR is needed');
  const features = readFeatures(a.features);
  const mins = readMins(a.features);
  const rows = runRows(a.run);
  const cov = computeCoverage({ features, rows, mins });
  const text = `${JSON.stringify(cov, null, 2)}\n`;
  if (a.out) fs.writeFileSync(a.out, text);
  else process.stdout.write(text);
  process.stderr.write(`coverage: ${cov.covered} of ${cov.total} features covered (${rows.length} rows)\n`);
  return 0;
}

const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) {
  process.exitCode = await main().catch((e) => {
    process.stderr.write(`${String(e?.stack ?? e)}\n`);
    return 1;
  });
}
