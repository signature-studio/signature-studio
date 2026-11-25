// pm-work/tools/gen-wave-plugins.test.mjs : the structure of the second-wave drafts that gen-wave-plugins.mjs writes (node --test pm-work/tools/).
// It runs the generator into a directory of its own (under .tmp-*, removed after the test), so pm-work/issue-drafts/wave2/ is not touched.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const tool = join(root, 'pm-work', 'tools', 'gen-wave-plugins.mjs');
const tmp = mkdtempSync(join(root, '.tmp-gen-wave-test-'));
const out = join(tmp, 'wave2');
const gen = (args) => spawnSync('node', [tool, ...args], { encoding: 'utf8' });

const FIRST = ['fruity-balance', 'fruity-filter', 'fruity-delay-2', 'sytrus'];
const ACCEPT_BANNED = ['anthr' + 'opic', 'TODO', '@@', 'L-0'];
const ROW = /^(effects|generators)\.[a-z0-9-]+\.[a-z0-9._-]+\.dll$/;

function parse(file) {
  const text = readFileSync(join(out, file), 'utf8');
  const m = text.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/);
  const fm = Object.fromEntries(m[1].split('\n').map((l) => { const i = l.indexOf(': '); return [l.slice(0, i), l.slice(i + 2)]; }));
  const body = m[2];
  const accepts = [...body.matchAll(/```accept\n([\s\S]*?)\n```/g)].map((x) => x[1]);
  const tsv = [...body.matchAll(/```tsv\n([\s\S]*?)\n```/g)].map((x) => x[1]);
  return { fm, body, accepts, tsv };
}

test('the dry-run gives 130 folders, 159 DLLs and 44 bundles (Effects 29, Generators 15)', () => {
  const r = gen(['--dry-run']);
  assert.equal(r.status, 0, r.stderr);
  assert.match(r.stdout, /DLL: Effects 112 件、Generators 70 件/);
  assert.match(r.stdout, /残り: Effects 85 フォルダ\(本体 \d+ 件\)、Generators 45 フォルダ/);
  assert.match(r.stdout, /Effects 29 束、Generators 15 束。計 44 束。下書き番号 041〜084/);
});

test('the generator writes 44 drafts, numbered 041 to 084, and no others', () => {
  const r = gen(['--out', out]);
  assert.equal(r.status, 0, r.stderr);
  const files = readdirSync(out).sort();
  assert.equal(files.length, 44);
  assert.equal(files[0], '041-effects-01.md');
  assert.equal(files.at(-1), '084-generators-15.md');
});

test('each draft has the front matter of the wave: Task, M5, parent, High, project, one size', () => {
  for (const f of readdirSync(out)) {
    const { fm } = parse(f);
    assert.equal(fm.type, '"Task"', f);
    assert.equal(fm.milestone, '"M5 内蔵のエフェクトとインストゥルメント"', f);
    assert.equal(fm.priority, '"High"', f);
    assert.equal(fm.project, '"signature-studio/3"', f);
    assert.match(fm.size, /^"(S|M|L)"$/, f);
    const parent = f.includes('-effects-') ? '"021"' : '"022"';
    assert.equal(fm.parent, parent, f);
    assert.equal(fm.labels, '["plugin-builtin"]', f);
  }
});

test('the blocked_by follow the stages: stage 1 waits for 126 and 127; stage 2 and 3 also wait for the first three of the stage before; a bundle with Data also waits for 192 (#1478)', () => {
  const names = readdirSync(out).sort();
  const dn = (i) => names[i].slice(0, 3);
  for (let i = 0; i < names.length; i++) {
    const { fm, body } = parse(names[i]);
    const want = ['"126"', '"127"'];
    if (/Data: [1-9][0-9]* ファイル/.test(body)) want.push('"192"');
    if (i >= 15 && i < 30) want.push(`"${dn(0)}"`, `"${dn(1)}"`, `"${dn(2)}"`);
    if (i >= 30) want.push(`"${dn(15)}"`, `"${dn(16)}"`, `"${dn(17)}"`);
    assert.equal(fm.blocked_by, `[${want.join(',')}]`, names[i]);
  }
});

test('a bundle with Data declares each folder in its data.tsv: the acceptance checks the file and the row count, and the Data missing count is 0 (19 bundles)', () => {
  let bundles = 0;
  for (const f of readdirSync(out)) {
    const { body, accepts } = parse(f);
    const folders = [...body.matchAll(/^- [^\n]+?\(`lift\/plugins\/([a-z0-9-]+)\/data\.tsv`\): (\d+) ファイル/gm)];
    if (folders.length === 0) {
      assert.ok(!body.includes('data.tsv'), f);
      continue;
    }
    bundles++;
    const a = accepts[0];
    for (const [, dir, n] of folders) {
      assert.ok(a.includes(`test -f lift/plugins/${dir}/data.tsv`), `${f}: ${dir}`);
      assert.ok(a.includes(`process.exit(L.length===${n}?0:1)`), `${f}: ${dir} rows ${n}`);
    }
    assert.ok(a.includes('r.missingInInventory===0'), f);
  }
  assert.equal(bundles, 19);
});

test('each body has one accept line that runs the plugin tool on its own folders, and keeps the other files as they are', () => {
  for (const f of readdirSync(out)) {
    const { accepts, body } = parse(f);
    assert.equal(accepts.length, 1, f);
    const a = accepts[0];
    assert.ok(!a.includes('\n'), f);
    assert.match(a, /node tools\/plugin-run\/run\.mjs "\$PWD\/lift\/out-app\/bundle\/raw\/fl-Oz" '[a-z0-9-]+'/, f);
    assert.match(a, /node tools\/boot-check\/boot-bundle\.mjs/, f);
    assert.match(a, /r\.elapsedMs<=12500/, f);
    assert.match(a, /node tools\/smoke\/welcome-flow\.mjs/, f);
    assert.match(a, /r\.preBootBytes<=314368711/, f);
    assert.match(a, /git diff --name-only origin\/main\.\.\.HEAD -- \. ':!lift\/plugins'/, f);
    for (const bad of ACCEPT_BANNED) assert.ok(!body.includes(bad), `${f}: ${bad}`);
    assert.ok(!/#\d/.test(body), `${f}: #数字 in the body`);
  }
});

test('the plugin rows are 8 fields, the keys follow <kind>.<dir>.<dll>, and the tables of the body hold all 159 DLLs', () => {
  let rows = 0, bodies = 0;
  const keys = new Set();
  for (const f of readdirSync(out)) {
    const { tsv } = parse(f);
    assert.ok(tsv.length >= 1, f);
    for (const block of tsv) {
      const lines = block.split('\n');
      assert.equal(lines[0], 'key\tguest\trole\tsha256\tsize\thook\targs\tomit', f);
      for (const line of lines.slice(1)) {
        const c = line.split('\t');
        assert.equal(c.length, 8, `${f}: ${line}`);
        assert.match(c[0], ROW, f);
        assert.ok(!keys.has(c[0]), `key twice: ${c[0]}`);
        keys.add(c[0]);
        assert.match(c[1], /^Plugins\\Fruity\\(Effects|Generators)\\/, f);
        assert.ok(['本体', '補助'].includes(c[2]), f);
        assert.match(c[3], /^[0-9a-f]{64}$/, f);
        assert.match(c[4], /^[1-9][0-9]*$/, f);
        assert.match(c[5], /^(none|rva:0x[0-9a-f]+,0x[0-9a-f]+,0x[0-9a-f]+)$/, f);
        assert.equal(c[6], '--pdata', f);
        assert.equal(c[7], '-', f);
        rows++;
        if (c[2] === '本体') bodies++;
      }
    }
  }
  assert.equal(rows, 159);
  assert.equal(bodies, 131);
  assert.ok(![...keys].some((k) => k.startsWith('generators.sytrus.')), 'Sytrus is not in the second wave');
});

test('the directories are unique, and the two folder names that exist in both kinds take the kind in front', () => {
  const all = readdirSync(out).map((f) => parse(f).body).join('\n');
  for (const d of ['effects-fruity-wrapper', 'generators-fruity-wrapper', 'effects-patcher', 'generators-patcher']) {
    assert.ok(all.includes(`lift/plugins/${d}/plugin.tsv`), d);
  }
  const tsvKeys = readdirSync(out).flatMap((f) => parse(f).tsv.flatMap((b) => b.split('\n').slice(1).map((l) => l.split('\t')[0])));
  for (const d of FIRST) assert.ok(!tsvKeys.some((k) => k.split('.')[1] === d), `${d} is already in lift/plugins`);
});

test('the size follows the DLL count: S for 1 to 3, M for 4 to 9, L for 10 or more', () => {
  for (const f of readdirSync(out)) {
    const { fm, tsv } = parse(f);
    const n = tsv.reduce((s, b) => s + b.split('\n').length - 1, 0);
    const want = n <= 3 ? 'S' : n <= 9 ? 'M' : 'L';
    assert.equal(fm.size, `"${want}"`, `${f}: ${n} DLLs`);
  }
});

test('the acceptance of a generators bundle checks instantiation and not the processing (the effects tool covers effects only)', () => {
  for (const f of readdirSync(out).filter((x) => x.includes('-generators-'))) {
    const a = parse(f).accepts[0];
    assert.ok(!a.includes('r.processed===true'), f);
  }
  for (const f of readdirSync(out).filter((x) => x.includes('-effects-'))) {
    const a = parse(f).accepts[0];
    assert.ok(a.includes('r.processed===true'), f);
  }
});

test('no shared file is named as a file to change, and no draft asks for a product flag or an environment variable', () => {
  for (const f of readdirSync(out)) {
    const { body } = parse(f);
    assert.ok(!/(環境変数|#ifdef|URL パラメータ)を足/.test(body) || /足さない/.test(body), f);
    assert.ok(!/inputs\/plugin-hooks\.tsv を編集|ci\/plugin-builtin-prefixes\.txt を編集/.test(body), f);
  }
});

test.after(() => rmSync(tmp, { recursive: true, force: true }));
