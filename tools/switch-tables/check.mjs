#!/usr/bin/env node
// tools/switch-tables/check.mjs: the `jmp reg` survey of the plug-in DLLs (issue #1413). For each judged DLL it runs
// `fe-decode jmp-reg` (lift/target/release) and judges, without any baseline: no misrecognised table target (misrecognized 0),
// no switch table whose shape is not recognised (unrecognized_table_shape 0), and enough resolved tables (FLEngine 3, Sytrus 3,
// Balance 2). A missing DLL fails. The other unresolved sites (not the shape of a switch table: the delay-load returns
// `call <stub>; ...; jmp rax`) are counted and printed, not judged. OTT is not judged: it is measured only when its VST3 binary is
// in lift/out-app/external (the plugin-external input is fetched only for that label), otherwise it is reported as not measured.
// Exit: 0 when every judged row holds, 1 otherwise, 64 when the decoder is missing.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const DECODE = path.join(ROOT, 'lift', 'target', 'release', 'fe-decode');
const HOST = path.join(ROOT, 'lift', 'out-app', 'host');
const EXTERNAL = path.join(ROOT, 'lift', 'out-app', 'external');

const JUDGED = [
  { name: 'FLEngine', file: path.join(HOST, 'binaries', 'win64', 'FLEngine_x64.dll'), minResolved: 3 },
  { name: 'Sytrus', file: path.join(HOST, 'lift', 'out-plugins', 'generators.sytrus.sytrus_x64.dll'), minResolved: 3 },
  { name: 'Balance', file: path.join(HOST, 'lift', 'out-plugins', 'effects.fruity-balance.fruity_balance_x64.dll'), minResolved: 2 },
];

/** The VST3 binary of OTT, if the external input was extracted (`tools/external-inputs/extract.mjs` writes OUT/ott/<name>.vst3). */
function ottBinary() {
  const dir = path.join(EXTERNAL, 'ott');
  if (!fs.existsSync(dir)) return null;
  const hit = fs.readdirSync(dir).find((f) => /\.vst3$/i.test(f));
  return hit ? path.join(dir, hit) : null;
}

/** One DLL: the summary of `fe-decode jmp-reg`, and the lines of the sites that have the shape of a switch table but no table. */
function survey(file) {
  const out = execFileSync(DECODE, ['jmp-reg', file], { encoding: 'utf8', maxBuffer: 1 << 28 });
  const s = { open: [] };
  for (const line of out.split('\n')) {
    const m = /^summary (\w+) (\d+)$/.exec(line);
    if (m) s[m[1]] = Number(m[2]);
    if (/ unresolved shape table /.test(line)) s.open.push(line);
  }
  return s;
}

function summaryLine(name, s) {
  return (
    `switch-tables ${name}: sites ${s.sites} resolved_tables ${s.resolved_tables} ` +
    `unrecognized_table_shape ${s.unrecognized_table_shape} unrecognized_other ${s.unrecognized_other} misrecognized ${s.misrecognized}`
  );
}

function main() {
  if (!fs.existsSync(DECODE)) {
    console.error(`switch-tables: ${DECODE} is missing (cargo build --release --locked --manifest-path lift/Cargo.toml -p fe-decode)`);
    return 64;
  }
  let ok = true;
  for (const d of JUDGED) {
    if (!fs.existsSync(d.file)) {
      console.log(`switch-tables ${d.name}: FAIL (the binary is not in this tree: ${path.relative(ROOT, d.file)})`);
      ok = false;
      continue;
    }
    const s = survey(d.file);
    const judged = [
      ['misrecognized', s.misrecognized, 0, s.misrecognized === 0],
      ['unrecognized_table_shape', s.unrecognized_table_shape, 0, s.unrecognized_table_shape === 0],
      ['resolved_tables', s.resolved_tables, `>= ${d.minResolved}`, s.resolved_tables >= d.minResolved],
    ];
    const bad = judged.filter((j) => !j[3]);
    ok = ok && bad.length === 0;
    console.log(`${summaryLine(d.name, s)} ${bad.length === 0 ? 'ok' : 'FAIL'}`);
    for (const [k, got, want] of bad) {
      console.log(`  FAIL ${k}: ${got} (wanted ${want})`);
    }
    for (const line of s.open) {
      console.log(`  open ${line}`);
    }
  }
  const ott = ottBinary();
  if (!ott) {
    console.log('switch-tables OTT: not measured (no VST3 binary in lift/out-app/external; not judged)');
  } else {
    console.log(`${summaryLine('OTT', survey(ott))} (not judged)`);
  }
  return ok ? 0 : 1;
}

process.exitCode = main();
