#!/usr/bin/env node
// spec/gen-wave-plugins.test.mjs : pm-work/tools/gen-wave-plugins.mjs(第 2 波の下書きを作る道具)を確かめる。偽の小さな一覧を使う。GitHub には書かない。
// 実物の一覧(inputs/fl-installer-files.tsv)は、--dry-run の件数だけを見る。一時ファイルは .tmp-spec-gw-* に作り、終わったら消す。終了コード 0 = 全部通った。
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const TOOL = path.join(ROOT, 'pm-work/tools/gen-wave-plugins.mjs');
const CI = path.join(ROOT, 'pm-work/tools/create-issues.mjs');
const TASK_ACCEPT = path.join(ROOT, 'tools/task-accept.mjs');
const REAL = {
  installer: path.join(ROOT, 'inputs/fl-installer-files.tsv'),
  deferred: path.join(ROOT, 'inputs/vfx-deferred.tsv'),
  template: path.join(ROOT, 'pm-work/issue-drafts/wave2-template.md'),
  table: path.join(ROOT, 'pm-work/created-issues.tsv'),
  milestones: path.join(ROOT, 'pm-work/milestones.md'),
};
const TODO = 'TODO(#1315 の結果で確定)';
const MILESTONE = 'M5 内蔵のエフェクトとインストゥルメント';
let failed = 0;
const ok = (name, got, want) => {
  if (JSON.stringify(got) === JSON.stringify(want)) console.log(`ok   ${name}`);
  else {
    failed++;
    console.log(`FAIL ${name}: expected ${JSON.stringify(want)}, got ${JSON.stringify(got)}`);
  }
};
const sha = (s) => crypto.createHash('sha256').update(s).digest('hex');
const tmp = fs.mkdtempSync(path.join(ROOT, '.tmp-spec-gw-'));
const gen = (args) => spawnSync(process.execPath, [TOOL, ...args], { encoding: 'utf8' });
const field = (text, key) => (text.match(new RegExp(`^${key}: (.*)$`, 'm')) ?? [])[1];
const countDrafts = (dir) => (fs.existsSync(dir) ? fs.readdirSync(dir).filter((f) => f.endsWith('.md')).length : 0);

try {
  // 実物: テンプレートには TODO が在り、生成は TODO の理由で止まる(何も書かない)
  ok('実物のテンプレートに TODO が在る(確定の前は生成を拒否する)', fs.readFileSync(REAL.template, 'utf8').includes(TODO), true);
  const outReal = path.join(tmp, 'real-out');
  const real = gen(['--installer', REAL.installer, '--deferred', REAL.deferred, '--template', REAL.template, '--out', outReal]);
  ok('実物の生成は、TODO の理由で 1 で止まり、何も書かない', [real.status, real.stderr.includes(TODO), countDrafts(outReal)], [1, true, 0]);
  const realDry = gen(['--dry-run', '--installer', REAL.installer, '--deferred', REAL.deferred, '--template', REAL.template, '--out', outReal]);
  ok('実物の --dry-run は、件数を出してから TODO で 1 で止まり、何も書かない', [realDry.status, /Effects 35 束/.test(realDry.stdout), /Generators 22 束/.test(realDry.stdout), /計 57 束/.test(realDry.stdout), /残り: Effects 104 件、Generators 64 件/.test(realDry.stdout), countDrafts(outReal)], [1, true, true, true, true, 0]);
  ok('実物の --dry-run は、後回しと第 1 波の除いた件数を出す', [/除く: 後回し 11 件\(Effects 5・Generators 6\)、第 1 波 3 件/.test(realDry.stdout)], [true]);

  // 偽の一覧: Effects 7 件(第 1 波 3 件を含む。残り 4 件)、Generators 4 件(後回し 2 件を含む。残り 2 件)
  const P = 'Plugins\\Fruity\\';
  const H = ['guest', 'bytes', 'sha256', 'kind', 'imports'].join('\t');
  const dll = (guest, bytes = 1000) => [guest, String(bytes), sha(guest), 'dll', 'kernel32.dll;user32.dll'].join('\t');
  const fixIns = path.join(tmp, 'installer.tsv');
  fs.writeFileSync(fixIns, [
    '# 試験の一覧', H,
    `Artwork\\Fonts\\x.ttf\t\t\tother\t`,
    dll(`${P}Effects\\Fake D\\Fake D_x64.dll`, 4000),
    dll(`${P}Effects\\Fruity Balance\\Fruity Balance_x64.dll`, 3718656),
    dll(`${P}Effects\\Fake B\\Fake B_x64.dll`, 2000),
    dll(`${P}Effects\\Fruity Filter\\Fruity Filter_x64.dll`, 3780608),
    `${P}Effects\\Fake A\\Artwork\\a.ilcontrol\t54\tabc\tother\t`,
    dll(`${P}Effects\\Fake A\\Fake A_x64.dll`, 1000),
    dll(`${P}Effects\\Fake C\\Fake C_x64.dll`, 3000),
    dll(`${P}Effects\\Fruity Delay 2\\Fruity Delay 2_x64.dll`, 3731968),
    dll(`${P}Generators\\Fake G2\\Fake G2_x64.dll`, 2222),
    dll(`${P}Generators\\Fake Gl\\engine_x64.dll`, 5555),
    dll(`${P}Generators\\Fake G1\\Fake G1_x64.dll`, 1111),
    dll(`${P}Generators\\Fake Gl2\\Fake Gl2_x64.dll`, 6666),
    '',
  ].join('\n'));
  const fixDef = path.join(tmp, 'deferred.tsv');
  fs.writeFileSync(fixDef, ['guest\treason\timports', `${P}Generators\\Fake Gl\\engine_x64.dll\topengl32.dll を import\tkernel32.dll;opengl32.dll`, `${P}Generators\\Fake Gl2\\Fake Gl2_x64.dll\topengl32.dll を import\tkernel32.dll;opengl32.dll`, ''].join('\n'));
  const fixTpl = path.join(tmp, 'template.md');
  fs.writeFileSync(fixTpl, fs.readFileSync(REAL.template, 'utf8').split('\n').filter((l) => !l.includes(TODO)).join('\n'));
  const fixArgs = ['--installer', fixIns, '--deferred', fixDef, '--template', fixTpl];

  const outDry = path.join(tmp, 'fix-dry');
  const dry = gen(['--dry-run', ...fixArgs, '--out', outDry]);
  ok('偽の一覧の --dry-run は 0 で、ファイルを書かない', [dry.status, fs.existsSync(outDry)], [0, false]);
  ok('偽の一覧の --dry-run は、DLL の件数・除いた件数・束の数を出す', [/DLL: Effects 7 件、Generators 4 件/.test(dry.stdout), /除く: 後回し 2 件\(Effects 0・Generators 2\)、第 1 波 3 件/.test(dry.stdout), /残り: Effects 4 件、Generators 2 件/.test(dry.stdout), /Effects 2 束、Generators 1 束/.test(dry.stdout), /下書き番号 041〜043/.test(dry.stdout)], [true, true, true, true, true]);

  const out = path.join(tmp, 'fix-out');
  fs.mkdirSync(out, { recursive: true });
  fs.writeFileSync(path.join(out, '099-effects-09.md'), '古い生成物\n');
  fs.writeFileSync(path.join(out, 'notes.md'), '手で書いた物\n');
  const w = gen([...fixArgs, '--out', out]);
  ok('偽の一覧の生成は 0 で終わり、3 件を名前順に書く(古い生成物は消し、手で書いた物は残す)', [w.status, fs.readdirSync(out).sort()], [0, ['041-effects-01.md', '042-effects-02.md', '043-generators-01.md', 'notes.md']]);
  const f = (n) => fs.readFileSync(path.join(out, n), 'utf8');
  const b041 = f('041-effects-01.md');
  const b042 = f('042-effects-02.md');
  const b043 = f('043-generators-01.md');
  const drafts = [b041, b042, b043];
  ok('束: Effects は 3 件ずつ(最後だけ端数)、Generators も同じ。第 1 波と後回しは入らない', drafts.map((b) => field(b, 'title')), [
    '"内蔵エフェクト 1: Fake A・Fake B・Fake C を、1 プラグイン 1 wasm で入れる"',
    '"内蔵エフェクト 2: Fake D を、1 プラグイン 1 wasm で入れる"',
    '"内蔵インストゥルメント 1: Fake G1・Fake G2 を、1 プラグイン 1 wasm で入れる"',
  ]);
  ok('front matter: draft・parent(Effects は 021、Generators は 022)・milestone', drafts.map((b) => [field(b, 'draft'), field(b, 'parent'), field(b, 'milestone')]), [
    ['"041"', '"021"', `"${MILESTONE}"`],
    ['"042"', '"021"', `"${MILESTONE}"`],
    ['"043"', '"022"', `"${MILESTONE}"`],
  ]);
  ok('front matter: type は Task、blocked_by は 015 と 019、labels は plugin-builtin、size M、priority Medium、project', drafts.map((b) => ['type', 'blocked_by', 'labels', 'size', 'priority', 'project'].map((k) => field(b, k)).join(' | ')), [
    '"Task" | ["015","019"] | ["plugin-builtin"] | "M" | "Medium" | "signature-studio/3"',
    '"Task" | ["015","019"] | ["plugin-builtin"] | "M" | "Medium" | "signature-studio/3"',
    '"Task" | ["015","019"] | ["plugin-builtin"] | "M" | "Medium" | "signature-studio/3"',
  ]);
  const bodies = drafts.map((b) => b.slice(b.indexOf('\n---\n') + 5));
  ok('各 Issue の本文は互いに異なる(3 件とも違う)', new Set(bodies).size, 3);
  ok('束ごとに、各 DLL の名前・大きさ・SHA-256・import を本文に差し込む(041 の 3 件)', ['Fake A', 'Fake B', 'Fake C'].map((name) => {
    const guest = `${P}Effects\\${name}\\${name}_x64.dll`;
    const bytes = { 'Fake A': 1000, 'Fake B': 2000, 'Fake C': 3000 }[name];
    return b041.includes(guest) && b041.includes(String(bytes)) && b041.includes(sha(guest)) && b041.includes('kernel32.dll; user32.dll');
  }), [true, true, true]);
  ok('本文に後回し・第 1 波の DLL の一覧の行が入らない(パスで見る)', [b041, b042, b043].map((b) => /Fruity (Balance|Filter|Delay 2)\\|Fake Gl/.test(b)), [false, false, false]);
  ok('本文の参照は #{{NNN}} の形だけ(#数字・{{ の残り・@@・L-0 が無い)', [b041, b042, b043].map((b) => [/#\d/.test(b), b.replace(/#\{\{\d{3}\}\}/g, '').includes('{{'), b.includes('@@'), b.includes('L-0')]), [[false, false, false, false], [false, false, false, false], [false, false, false, false]]);
  ok('本文の参照: 親と blocked_by を、#{{NNN}} で書く(#{{015}}・#{{019}}・#{{021}})', [b041.includes('#{{015}}') && b041.includes('#{{019}}') && b041.includes('#{{021}}'), b043.includes('#{{022}}')], [true, true]);
  ok('accept: 各 Issue に 1 つ・1 行。task-accept が通る', [b041, b042, b043].map((b) => {
    const a = spawnSync(process.execPath, [TASK_ACCEPT], { input: b, encoding: 'utf8' });
    return [a.status, a.stdout.startsWith('accept: ')];
  }), [[0, true], [0, true], [0, true]]);
  ok('accept: DLL ごとの条件(名前の正規表現)を差し込む', [b041.includes('P=[/fake.?a/i, /fake.?b/i, /fake.?c/i]'), b043.includes('P=[/fake.?g1/i, /fake.?g2/i]')], [true, true]);

  // 既存の表(実物)を読む create-issues の dry-run で、parent・blocked_by・本文の参照が解決され、検査が通る
  const cdry = spawnSync(process.execPath, [CI, '--drafts', out, '--existing', REAL.table, '--milestones', REAL.milestones, '--state', path.join(tmp, 'st.json')], { encoding: 'utf8' });
  ok('生成した下書きは、create-issues の検査を通る(既存の表の参照を解決する)', [cdry.status, /015 -> #1315/.test(cdry.stdout), /019 -> #1319/.test(cdry.stdout), /021 -> #1322/.test(cdry.stdout), /022 -> #1323/.test(cdry.stdout), fs.existsSync(path.join(tmp, 'st.json'))], [0, true, true, true, true, false]);

  // TODO が残る(テンプレートを差し替えた)ときは、何も書かない
  const todoTpl = path.join(tmp, 'template-todo.md');
  fs.writeFileSync(todoTpl, `${TODO}: 仮\n${fs.readFileSync(fixTpl, 'utf8')}`);
  const outTodo = path.join(tmp, 'fix-todo');
  const t = gen(['--installer', fixIns, '--deferred', fixDef, '--template', todoTpl, '--out', outTodo]);
  ok('TODO が残るテンプレートは、1 で止まり、何も書かない', [t.status, t.stderr.includes(TODO), countDrafts(outTodo)], [1, true, 0]);
  const badTpl = path.join(tmp, 'template-bad.md');
  fs.writeFileSync(badTpl, `${fs.readFileSync(fixTpl, 'utf8')}\n本文に #1315 の数字だけの参照。\n`);
  const outBad = path.join(tmp, 'fix-bad');
  const bad = gen(['--installer', fixIns, '--deferred', fixDef, '--template', badTpl, '--out', outBad]);
  ok('#数字だけの参照が本文に在れば、1 で止まり、何も書かない', [bad.status, countDrafts(outBad)], [1, 0]);
  const badDef = path.join(tmp, 'deferred-bad.tsv');
  fs.writeFileSync(badDef, ['guest\treason\timports', `${P}Generators\\Nothing\\Nothing_x64.dll\tx\t`, ''].join('\n'));
  const outDef = path.join(tmp, 'fix-def');
  const d = gen(['--installer', fixIns, '--deferred', badDef, '--template', fixTpl, '--out', outDef]);
  ok('後回しの一覧に、インストーラに無い DLL が在れば、1 で止まり、何も書かない', [d.status, countDrafts(outDef)], [1, 0]);
  const u = gen(['--start', 'x', ...fixArgs]);
  ok('使い方の誤り(--start が 3 桁でない)は 2', u.status, 2);
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}

process.exitCode = failed ? 1 : 0;
