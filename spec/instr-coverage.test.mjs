import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const LISTED = fs.readFileSync(path.join(repo, 'lift/harness/hyb/lists/lowered-fle.txt'), 'utf8').split('\n').find((l) => /^0x[0-9a-f]+$/i.test(l.trim())).trim();
const COUNT_TOOL = path.join(repo, 'tools/instr-coverage/profile-counts.mjs');
const COVER_TOOL = path.join(repo, 'tools/instr-coverage/measure.mjs');

test('measure: the share is over the lifted functions (the rows whose va is a VA); the baseline is recorded next to the TSV once', () => {
  const dir = fs.mkdtempSync(path.join(repo, '.tmp-instr-coverage-'));
  try {
    const tsv = path.join(dir, 'p3.tsv');
    fs.writeFileSync(tsv, `# note\nva\tmodule\tinstructions\n${LISTED}\tfle\t30\n0x1\tfle\t10\nfle_desk_scene\truntime\t60\n`);
    const r = JSON.parse(execFileSync('node', [COVER_TOOL, tsv], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }));
    assert.deepEqual(r, { instrTotal: 100, instrVa: 40, instrDirect: 30, coveragePct: 75, baselinePct: 75, coveragePctAll: 30, baselinePctAll: 30 });
    const base = JSON.parse(fs.readFileSync(path.join(dir, 'p3.baseline.json'), 'utf8'));
    assert.equal(base.baselinePct, 75);
    assert.equal(base.baselinePctAll, 30);
    fs.writeFileSync(tsv, `va\tmodule\tinstructions\n${LISTED}\tfle\t90\n0x1\tfle\t10\n`);
    const again = JSON.parse(execFileSync('node', [COVER_TOOL, tsv], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }));
    assert.deepEqual(again, { instrTotal: 100, instrVa: 100, instrDirect: 90, coveragePct: 90, baselinePct: 75, coveragePctAll: 90, baselinePctAll: 30 });
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test('profile-counts: samples of the leaf frames by function, idle left out, the wasm names by the table', () => {
  const dir = fs.mkdtempSync(path.join(repo, '.tmp-instr-coverage-'));
  try {
    const names = path.join(dir, 'names.tsv');
    fs.writeFileSync(names, `index\tkind\tva\tname\n0\timport\t\tenv.x\n1\tf\t${LISTED}\tf_${LISTED.slice(2)}\n2\tc\t\tfle_desk_scene\n3\tc\t\tunnamed_3\n`);
    const hyb = path.join(dir, 'hyb');
    fs.mkdirSync(hyb);
    fs.writeFileSync(path.join(hyb, 'n0-fle.tsv'), `va\tinsns\tnative_bytes\twasm_code_bytes\tsig_in\tsig_out\telided\tstatus\tindirect\n${LISTED}\t1\t1\t1\t0\t0\t0\tok\t\n0x2\t1\t1\t1\t0\t0\t0\tnot selected for the new backend (hybrid)\t\n`);
    const nodes = [
      { id: 1, callFrame: { functionName: '(root)', url: '', scriptId: '0' } },
      { id: 2, callFrame: { functionName: 'wasm-function[1]', url: 'wasm://wasm/1', scriptId: '2' } },
      { id: 3, callFrame: { functionName: 'wasm-function[2]', url: 'wasm://wasm/1', scriptId: '2' } },
      { id: 4, callFrame: { functionName: 'wasm-function[3]', url: 'wasm://wasm/1', scriptId: '2' } },
      { id: 5, callFrame: { functionName: '(idle)', url: '', scriptId: '0' } },
      { id: 6, callFrame: { functionName: 'pump', url: 'blob:http://127.0.0.1:9/abc', scriptId: '3' } },
    ];
    const samples = [2, 2, 3, 4, 5, 5, 6];
    const profile = path.join(dir, 'profile.json');
    fs.writeFileSync(profile, JSON.stringify({ nodes, samples, timeDeltas: samples.map(() => 350), startTime: 0, endTime: 350 * samples.length }));
    const out = execFileSync('node', [COUNT_TOOL, profile, names, hyb, '--note', 'test'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim().split('\n');
    assert.equal(out[0].startsWith('# '), true);
    assert.equal(out[1], 'va\tmodule\tinstructions');
    const rows = out.slice(2).map((l) => l.split('\t'));
    assert.deepEqual(rows.find((r) => r[0] === LISTED), [LISTED, 'fle', '2']);
    assert.deepEqual(rows.find((r) => r[0] === 'fle_desk_scene'), ['fle_desk_scene', 'runtime', '1']);
    assert.deepEqual(rows.find((r) => r[0] === 'unnamed_3'), ['unnamed_3', 'unnamed', '1']);
    assert.deepEqual(rows.find((r) => r[0] === 'js:pump@blob'), ['js:pump@blob', 'js', '1']);
    assert.equal(rows.some((r) => r[0].includes('idle')), false);
    assert.equal(rows.reduce((s, r) => s + Number(r[2]), 0), 5);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});
