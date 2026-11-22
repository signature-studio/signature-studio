#!/usr/bin/env node
// spec/gen-wave-plugins.test.mjs : pm-work/tools/gen-wave-plugins.mjs(第 2 波の下書きを作る道具。フォルダ単位で束ねる)を確かめる。偽の小さな分類の表を使う。GitHub には書かない。
// 実物の表(inputs/plugin-classes.tsv)は、--dry-run の件数だけを見る。一時ファイルは .tmp-spec-gw-* に作り、終わったら消す。終了コード 0 = 全部通った。
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
  classes: path.join(ROOT, 'inputs/plugin-classes.tsv'),
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
  const real = gen(['--classes', REAL.classes, '--deferred', REAL.deferred, '--template', REAL.template, '--out', outReal]);
  ok('実物の生成は、TODO の理由で 1 で止まり、何も書かない', [real.status, real.stderr.includes(TODO), countDrafts(outReal)], [1, true, 0]);
  const realDry = gen(['--dry-run', '--classes', REAL.classes, '--deferred', REAL.deferred, '--template', REAL.template, '--out', outReal]);
  const rd = realDry.stdout;
  ok('実物の --dry-run は、件数を出してから TODO で 1 で止まり、何も書かない', [realDry.status, realDry.stderr.includes(TODO), countDrafts(outReal)], [1, true, 0]);
  ok('実物の --dry-run: DLL と フォルダの数(Effects 112・Generators 70。フォルダ 90・51)', [/DLL: Effects 112 件、Generators 70 件\(計 182 件\)/.test(rd), /フォルダ: Effects 90、Generators 51\(計 141\)/.test(rd)], [true, true]);
  ok('実物の --dry-run: 除く(VFX 系 7 フォルダ・DLL 19 件。第 1 波 3 フォルダ)', [/除く: VFX 系 7 フォルダ\(Effects 2・Generators 5。DLL 19 件\)、第 1 波 3 フォルダ\(DLL 3 件\)/.test(rd)], [true]);
  ok('実物の --dry-run: 残りと束(Effects 85 フォルダ・Generators 46 フォルダ。29 束・16 束。041〜085)', [/残り: Effects 85 フォルダ\(本体 86 件\)、Generators 46 フォルダ\(本体 46 件\)/.test(rd), /束\(3 フォルダずつ\): Effects 29 束、Generators 16 束。計 45 束。下書き番号 041〜085/.test(rd)], [true, true]);

  // 偽の分類の表: フォルダ単位。Effects は 9 フォルダ(第 1 波 3・VFX 1 を除き 5)、Generators は 3 フォルダ(VFX 1 を除き 2)
  const P = 'Plugins\\Fruity\\';
  const HEAD = ['パス', '大きさ', 'SHA-256', '種別', '本体か補助か', 'export 名', 'import した DLL', 'hook の要否', 'VFX', '備考'];
  const row = (p, bytes, role, hook, vfx, exp, imp, note) => [p, String(bytes), sha(p), p.split('\\')[2], role, exp, imp, hook, vfx, note].join('\t');
  const B = 'CreatePlugInstance;__dbk_fcall_wrapper';
  const K = 'kernel32.dll;user32.dll';
  const classes = [
    row(`${P}Effects\\Fake A\\Fake A_x64.dll`, 1000, '本体', '要', 'no', B, K, 'Fake A の本体'),
    row(`${P}Effects\\Fake A\\Fake Aux_x64.dll`, 500, '補助', '不要', 'no', '', 'kernel32.dll', '入口を持たない。同じ組の本体が名前で参照する'),
    row(`${P}Effects\\Fake B\\Fake B_x64.dll`, 2000, '本体', '既存', 'no', B, 'kernel32.dll', 'Fake B の本体'),
    row(`${P}Effects\\Fake C\\Fake C_x64.dll`, 3000, '本体', '要', 'no', B, K, 'Fake C の本体'),
    row(`${P}Effects\\Fake D\\Fake D_x64.dll`, 4000, '本体', '要', 'no', B, K, 'Fake D の本体'),
    row(`${P}Effects\\Fake V\\engine_x64.dll`, 700, '補助', '不要', 'yes', '', 'kernel32.dll;opengl32.dll', '入口を持たない'),
    row(`${P}Effects\\Fake V\\Fake V_x64.dll`, 800, '本体', '不要', 'no', B, 'kernel32.dll', 'VFX の補助を持つ'),
    row(`${P}Effects\\Fruity Balance\\Fruity Balance_x64.dll`, 3718656, '本体', '既存', 'no', B, K, '第 1 波'),
    row(`${P}Effects\\Fruity Filter\\Fruity Filter_x64.dll`, 3780608, '本体', '既存', 'no', B, K, '第 1 波'),
    row(`${P}Effects\\Fruity Delay 2\\Fruity Delay 2_x64.dll`, 3731968, '本体', '既存', 'no', B, K, '第 1 波'),
    row(`${P}Effects\\Fake Two\\Fake Two_x64.dll`, 5000, '本体', '要', 'no', B, K, 'Fake Two の本体'),
    row(`${P}Effects\\Fake Two\\Sub\\Fake Sub_x64.dll`, 5100, '本体', '要', 'no', B, K, 'Fake Two の中の別の本体'),
    row(`${P}Generators\\Fake G2\\Fake G2_x64.dll`, 2222, '本体', '要', 'no', B, K, 'Fake G2 の本体'),
    row(`${P}Generators\\Fake Gl\\Fake Gl_x64.dll`, 1000, '本体', '要', 'no', B, K, 'Fake Gl の本体'),
    row(`${P}Generators\\Fake Gl\\engine_x64.dll`, 5555, '補助', '不要', 'yes', '', 'kernel32.dll;opengl32.dll', '入口を持たない'),
    row(`${P}Generators\\Fake G1\\Fake G1_x64.dll`, 1111, '本体', '要', 'no', B, K, 'Fake G1 の本体'),
  ];
  const fixCls = path.join(tmp, 'classes.tsv');
  fs.writeFileSync(fixCls, [HEAD.join('\t'), ...classes, ''].join('\n'));
  const fixDef = path.join(tmp, 'deferred.tsv');
  fs.writeFileSync(fixDef, ['guest\treason\timports', `${P}Effects\\Fake V\\engine_x64.dll\topengl32.dll を import\tkernel32.dll;opengl32.dll`, `${P}Generators\\Fake Gl\\engine_x64.dll\topengl32.dll を import\tkernel32.dll;opengl32.dll`, ''].join('\n'));
  const fixTpl = path.join(tmp, 'template.md');
  fs.writeFileSync(fixTpl, fs.readFileSync(REAL.template, 'utf8').split('\n').filter((l) => !l.includes(TODO)).join('\n'));
  const fixArgs = ['--classes', fixCls, '--deferred', fixDef, '--template', fixTpl];

  const outDry = path.join(tmp, 'fix-dry');
  const dry = gen(['--dry-run', ...fixArgs, '--out', outDry]);
  ok('偽の表の --dry-run は 0 で、ファイルを書かない', [dry.status, fs.existsSync(outDry)], [0, false]);
  ok('偽の表の --dry-run: DLL・フォルダ・除く・残り・束', [
    /DLL: Effects 12 件、Generators 4 件\(計 16 件\)/.test(dry.stdout),
    /フォルダ: Effects 9、Generators 3\(計 12\)/.test(dry.stdout),
    /除く: VFX 系 2 フォルダ\(Effects 1・Generators 1。DLL 4 件\)、第 1 波 3 フォルダ\(DLL 3 件\)/.test(dry.stdout),
    /残り: Effects 5 フォルダ\(本体 6 件\)、Generators 2 フォルダ\(本体 2 件\)/.test(dry.stdout),
    /束\(3 フォルダずつ\): Effects 2 束、Generators 1 束。計 3 束。下書き番号 041〜043/.test(dry.stdout),
  ], [true, true, true, true, true]);

  const out = path.join(tmp, 'fix-out');
  fs.mkdirSync(out, { recursive: true });
  fs.writeFileSync(path.join(out, '099-effects-09.md'), '古い生成物\n');
  fs.writeFileSync(path.join(out, 'notes.md'), '手で書いた物\n');
  const w = gen([...fixArgs, '--out', out]);
  ok('偽の表の生成は 0 で終わり、フォルダを名前順に 3 つずつ束ねる(古い生成物は消し、手で書いた物は残す)', [w.status, fs.readdirSync(out).sort()], [0, ['041-effects-01.md', '042-effects-02.md', '043-generators-01.md', 'notes.md']]);
  const f = (n) => fs.readFileSync(path.join(out, n), 'utf8');
  const b041 = f('041-effects-01.md');
  const b042 = f('042-effects-02.md');
  const b043 = f('043-generators-01.md');
  const drafts = [b041, b042, b043];
  ok('束の題名: Effects は 3 フォルダずつ(最後は端数)、Generators も同じ。第 1 波と VFX は入らない', drafts.map((b) => field(b, 'title')), [
    '"内蔵エフェクト 1: Fake A・Fake B・Fake C を、1 プラグイン 1 wasm で入れる"',
    '"内蔵エフェクト 2: Fake D・Fake Two を、1 プラグイン 1 wasm で入れる"',
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
  ok('フォルダの本体の DLL(名前・大きさ・SHA-256・export・import・hook の要否)を本文に差し込む(041 の Fake A)', (() => {
    const guest = `${P}Effects\\Fake A\\Fake A_x64.dll`;
    return b041.includes(`- フォルダ \`${P}Effects\\Fake A\\\`(本体 1・補助 1)`)
      && b041.includes(`  - 本体: \`${guest}\``)
      && b041.includes('    - 大きさ: 1000 B')
      && b041.includes(`    - SHA-256: ${sha(guest)}`)
      && b041.includes('    - export: CreatePlugInstance; __dbk_fcall_wrapper')
      && b041.includes('    - import: kernel32.dll; user32.dll')
      && b041.includes('    - hook の要否: 要');
  })(), true);
  ok('補助の DLL(名前・大きさ・備考)を、同じフォルダの下に差し込む(041 の Fake Aux)', b041.includes(`  - 補助: \`${P}Effects\\Fake A\\Fake Aux_x64.dll\`\n    - 名前: Fake Aux\n    - 大きさ: 500 B\n    - 備考: 入口を持たない。同じ組の本体が名前で参照する`), true);
  ok('同じフォルダに本体が 2 つ在るとき、両方を書く(042 の Fake Two と Fake Sub)', [b042.includes(`- フォルダ \`${P}Effects\\Fake Two\\\`(本体 2・補助 0)`), b042.includes('  - 本体: `' + `${P}Effects\\Fake Two\\Fake Two_x64.dll` + '`'), b042.includes('  - 本体: `' + `${P}Effects\\Fake Two\\Sub\\Fake Sub_x64.dll` + '`')], [true, true, true]);
  ok('本文に、第 1 波と VFX のフォルダのパスが入らない', drafts.map((b) => /Fruity (Balance|Filter|Delay 2)\\|Fake (V|Gl)\\/.test(b)), [false, false, false]);
  ok('本文の参照は #{{NNN}} の形だけ(#数字・{{ の残り・@@・L-0 が無い)', drafts.map((b) => [/#\d/.test(b), b.replace(/#\{\{\d{3}\}\}/g, '').includes('{{'), b.includes('@@'), b.includes('L-0')]), [[false, false, false, false], [false, false, false, false], [false, false, false, false]]);
  ok('本文の参照: 親と blocked_by を、#{{NNN}} で書く(#{{015}}・#{{019}}・#{{021}}・#{{022}})', [b041.includes('#{{015}}') && b041.includes('#{{019}}') && b041.includes('#{{021}}'), b043.includes('#{{022}}')], [true, true]);
  ok('accept: 各 Issue に 1 つ・1 行。task-accept が通る', drafts.map((b) => {
    const a = spawnSync(process.execPath, [TASK_ACCEPT], { input: b, encoding: 'utf8' });
    return [a.status, a.stdout.startsWith('accept: ')];
  }), [[0, true], [0, true], [0, true]]);
  ok('accept: 本体の DLL ごとの名前の条件を差し込む(042 は 2 つの本体)', [b041.includes('P=[/fake.?a/i, /fake.?b/i, /fake.?c/i]'), b042.includes('P=[/fake.?d/i, /fake.?two/i, /fake.?sub/i]'), b043.includes('P=[/fake.?g1/i, /fake.?g2/i]')], [true, true, true]);

  // 既存の表(実物)を読む create-issues の dry-run で、parent・blocked_by・本文の参照が解決され、検査が通る
  const cdry = spawnSync(process.execPath, [CI, '--drafts', out, '--existing', REAL.table, '--milestones', REAL.milestones, '--state', path.join(tmp, 'st.json')], { encoding: 'utf8' });
  ok('生成した下書きは、create-issues の検査を通る(既存の表の参照を解決する)', [cdry.status, /015 -> #1315/.test(cdry.stdout), /019 -> #1319/.test(cdry.stdout), /021 -> #1322/.test(cdry.stdout), /022 -> #1323/.test(cdry.stdout), fs.existsSync(path.join(tmp, 'st.json'))], [0, true, true, true, true, false]);

  // TODO が残る(テンプレートを差し替えた)ときは、何も書かない
  const todoTpl = path.join(tmp, 'template-todo.md');
  fs.writeFileSync(todoTpl, `${TODO}: 仮\n${fs.readFileSync(fixTpl, 'utf8')}`);
  const outTodo = path.join(tmp, 'fix-todo');
  const t = gen(['--classes', fixCls, '--deferred', fixDef, '--template', todoTpl, '--out', outTodo]);
  ok('TODO が残るテンプレートは、1 で止まり、何も書かない', [t.status, t.stderr.includes(TODO), countDrafts(outTodo)], [1, true, 0]);
  const badTpl = path.join(tmp, 'template-bad.md');
  fs.writeFileSync(badTpl, `${fs.readFileSync(fixTpl, 'utf8')}\n本文に #1315 の数字だけの参照。\n`);
  const outBad = path.join(tmp, 'fix-bad');
  const bad = gen(['--classes', fixCls, '--deferred', fixDef, '--template', badTpl, '--out', outBad]);
  ok('#数字だけの参照が本文に在れば、1 で止まり、何も書かない', [bad.status, countDrafts(outBad)], [1, 0]);

  // 後回しと VFX の DLL の集合が違えば、1 で止まる(何も書かない)
  const defMismatch = path.join(tmp, 'deferred-mismatch.tsv');
  fs.writeFileSync(defMismatch, ['guest\treason\timports', `${P}Effects\\Fake V\\engine_x64.dll\tx\t`, ''].join('\n'));
  const outMis = path.join(tmp, 'fix-mis');
  const mis = gen(['--classes', fixCls, '--deferred', defMismatch, '--template', fixTpl, '--out', outMis]);
  ok('後回しの一覧と VFX が yes の DLL が違えば、1 で止まり、何も書かない', [mis.status, mis.stderr.includes('違う'), countDrafts(outMis)], [1, true, 0]);
  const defUnknown = path.join(tmp, 'deferred-bad.tsv');
  fs.writeFileSync(defUnknown, ['guest\treason\timports', `${P}Generators\\Nothing\\Nothing_x64.dll\tx\t`, ''].join('\n'));
  const outDef = path.join(tmp, 'fix-def');
  const d = gen(['--classes', fixCls, '--deferred', defUnknown, '--template', fixTpl, '--out', outDef]);
  ok('後回しの一覧に、分類の表に無い DLL が在れば、1 で止まり、何も書かない', [d.status, countDrafts(outDef)], [1, 0]);

  // 本体の無いフォルダ(補助だけ)が在れば、1 で止まる
  const noBody = path.join(tmp, 'classes-nobody.tsv');
  fs.writeFileSync(noBody, [HEAD.join('\t'), ...classes, row(`${P}Effects\\Fake Orphan\\engine_x64.dll`, 10, '補助', '不要', 'no', '', 'kernel32.dll', '本体が無い'), ''].join('\n'));
  const outNo = path.join(tmp, 'fix-nobody');
  const nb = gen(['--classes', noBody, '--deferred', fixDef, '--template', fixTpl, '--out', outNo]);
  ok('本体の無いフォルダが在れば、1 で止まり、何も書かない', [nb.status, nb.stderr.includes('本体の無いフォルダ'), countDrafts(outNo)], [1, true, 0]);

  const u = gen(['--start', 'x', ...fixArgs]);
  ok('使い方の誤り(--start が 3 桁でない)は 2', u.status, 2);
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}

process.exitCode = failed ? 1 : 0;
