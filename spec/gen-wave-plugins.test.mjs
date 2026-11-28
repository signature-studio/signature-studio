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
  // 実物: テンプレートに TODO は無く、生成は 44 束を一時の場所に書く(repo の下書きには書かない)
  ok('実物のテンプレートに TODO が無い(手順は #1342 の結果で確定した)', fs.readFileSync(REAL.template, 'utf8').includes(TODO), false);
  const outReal = path.join(tmp, 'real-out');
  const real = gen(['--classes', REAL.classes, '--deferred', REAL.deferred, '--template', REAL.template, '--out', outReal]);
  ok('実物の生成は 0 で終わり、44 束を書く', [real.status, countDrafts(outReal)], [0, 44]);
  const outRealDry = path.join(tmp, 'real-dry');
  const realDry = gen(['--dry-run', '--classes', REAL.classes, '--deferred', REAL.deferred, '--template', REAL.template, '--out', outRealDry]);
  const rd = realDry.stdout;
  ok('実物の --dry-run は 0 で、何も書かない', [realDry.status, fs.existsSync(outRealDry)], [0, false]);
  ok('実物の --dry-run: DLL と フォルダの数(Effects 112・Generators 70。フォルダ 90・51)', [/DLL: Effects 112 件、Generators 70 件\(計 182 件\)/.test(rd), /フォルダ: Effects 90、Generators 51\(計 141\)/.test(rd)], [true, true]);
  ok('実物の --dry-run: 除く(VFX 系 7 フォルダ・DLL 19 件。lift/plugins/ に在る 4 フォルダ・DLL 4 件)', [/除く: VFX 系 7 フォルダ\(DLL 19 件\)、lift\/plugins\/ に在る 4 フォルダ\(DLL 4 件\)/.test(rd)], [true]);
  ok('実物の --dry-run: 残りと束(Effects 85 フォルダ・Generators 45 フォルダ。29 束・15 束。041〜084)', [/残り: Effects 85 フォルダ\(本体 86 件\)、Generators 45 フォルダ\(本体 45 件\)/.test(rd), /束\(3 フォルダずつ\): Effects 29 束、Generators 15 束。計 44 束。下書き番号 041〜084/.test(rd)], [true, true]);
  // 木に無い束の DLL は、受け入れの前に置く(tools/place-plugin-dlls.mjs)。accept では fe-build engine の直前に 1 回在り、本文の「1 プラグインの入れ方」に、その手順の 1 行が在る
  const PLACE = 'node tools/place-plugin-dlls.mjs && lift/target/release/fe-build engine';
  const PLACE_NOTE = '- 受け入れの前に `node tools/place-plugin-dlls.mjs` が、この束の DLL(と Data)を、';
  const realBodies = fs.readdirSync(outReal).filter((f) => f.endsWith('.md')).map((f) => fs.readFileSync(path.join(outReal, f), 'utf8'));
  ok('実物の 44 束の accept: node tools/place-plugin-dlls.mjs が fe-build engine の直前に 1 回在り、fe-build engine は 1 回だけ', realBodies.map((b) => spawnSync(process.execPath, [TASK_ACCEPT], { input: b, encoding: 'utf8' }).stdout).filter((a) => a.split(PLACE).length === 2 && a.split('fe-build engine').length === 2).length, 44);
  ok('実物の 44 束の本文: 手順の 1 行が「1 プラグインの入れ方」の節に 1 つ在り、停止の型より前にある', realBodies.filter((b) => b.split(PLACE_NOTE).length === 2 && b.indexOf('## 1 プラグインの入れ方') < b.indexOf(PLACE_NOTE) && b.indexOf(PLACE_NOTE) < b.indexOf('## 停止が出たときの型')).length, 44);

  // 偽の分類の表: フォルダ単位。Effects は 9 フォルダ(lift/plugins/ に在る 3・VFX 1 を除き 5)、Generators は 4 フォルダ(lift/plugins/ に在る Sytrus 1・VFX 1 を除き 2)
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
    row(`${P}Generators\\Sytrus\\Sytrus_x64.dll`, 9000, '本体', '既存', 'no', B, K, 'lift/plugins/ に在る(#1400)。除く'),
  ];
  const fixCls = path.join(tmp, 'classes.tsv');
  fs.writeFileSync(fixCls, [HEAD.join('\t'), ...classes, ''].join('\n'));
  const fixDef = path.join(tmp, 'deferred.tsv');
  fs.writeFileSync(fixDef, ['guest\treason\timports', `${P}Effects\\Fake V\\engine_x64.dll\topengl32.dll を import\tkernel32.dll;opengl32.dll`, `${P}Generators\\Fake Gl\\engine_x64.dll\topengl32.dll を import\tkernel32.dll;opengl32.dll`, ''].join('\n'));
  const fixHk = path.join(tmp, 'hooks.tsv');
  fs.writeFileSync(fixHk, ['パス\thook 1 RVA\thook 2 RVA\thook 3 RVA', ...classes.map((r) => r.split('\t')).filter((c) => c[4] === '本体' && (c[7] === '要' || c[7] === '既存')).map((c) => `${c[0]}\t0x1000\t0x2000\t0x3000`), ''].join('\n'));
  const fixTpl = path.join(tmp, 'template.md');
  fs.writeFileSync(fixTpl, fs.readFileSync(REAL.template, 'utf8').split('\n').filter((l) => !l.includes(TODO)).join('\n'));
  const fixArgs = ['--classes', fixCls, '--deferred', fixDef, '--template', fixTpl, '--hooks', fixHk];

  const outDry = path.join(tmp, 'fix-dry');
  const dry = gen(['--dry-run', ...fixArgs, '--out', outDry]);
  ok('偽の表の --dry-run は 0 で、ファイルを書かない', [dry.status, fs.existsSync(outDry)], [0, false]);
  ok('偽の表の --dry-run: DLL・フォルダ・除く・残り・束', [
    /DLL: Effects 12 件、Generators 5 件\(計 17 件\)/.test(dry.stdout),
    /フォルダ: Effects 9、Generators 4\(計 13\)/.test(dry.stdout),
    /除く: VFX 系 2 フォルダ\(DLL 4 件\)、lift\/plugins\/ に在る 4 フォルダ\(DLL 4 件\)/.test(dry.stdout),
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
  ok('front matter: type は Task、blocked_by は 126・127、labels は plugin-builtin、size(DLL 4 件・3 件・2 件で M・S・S)、priority High、project', drafts.map((b) => ['type', 'blocked_by', 'labels', 'size', 'priority', 'project'].map((k) => field(b, k)).join(' | ')), [
    '"Task" | ["126","127"] | ["plugin-builtin"] | "M" | "High" | "signature-studio/3"',
    '"Task" | ["126","127"] | ["plugin-builtin"] | "S" | "High" | "signature-studio/3"',
    '"Task" | ["126","127"] | ["plugin-builtin"] | "S" | "High" | "signature-studio/3"',
  ]);
  const bodies = drafts.map((b) => b.slice(b.indexOf('\n---\n') + 5));
  ok('各 Issue の本文は互いに異なる(3 件とも違う)', new Set(bodies).size, 3);
  ok('フォルダの本体の DLL(名前・大きさ・SHA-256・export・import・hook)を本文に差し込む(041 の Fake A)', (() => {
    const guest = `${P}Effects\\Fake A\\Fake A_x64.dll`;
    return b041.includes('### Fake A(`lift/plugins/fake-a/plugin.tsv`)')
      && b041.includes('- 種別: エフェクト。本体 1・補助 1。Data: なし')
      && b041.includes(`- 本体: \`${guest}\``)
      && b041.includes(`  - 大きさ 1000 B・SHA-256 ${sha(guest)}`)
      && b041.includes('  - export: CreatePlugInstance; __dbk_fcall_wrapper')
      && b041.includes('  - import: kernel32.dll; user32.dll')
      && b041.includes('  - hook: rva:0x1000,0x2000,0x3000(hook 点の表の 3 つの RVA)');
  })(), true);
  ok('補助の DLL(大きさ・備考)を、同じフォルダの下に差し込む(041 の Fake Aux)', b041.includes(`- 補助: \`${P}Effects\\Fake A\\Fake Aux_x64.dll\`\n  - 大きさ 500 B・備考: 入口を持たない。同じ組の本体が名前で参照する`), true);
  ok('同じフォルダに本体が 2 つ在るとき、両方を書く(042 の Fake Two と Fake Sub)', [b042.includes('- 種別: エフェクト。本体 2・補助 0。Data: なし'), b042.includes('- 本体: `' + `${P}Effects\\Fake Two\\Fake Two_x64.dll` + '`'), b042.includes('- 本体: `' + `${P}Effects\\Fake Two\\Sub\\Fake Sub_x64.dll` + '`')], [true, true, true]);
  ok('本文に、第 1 波と VFX のフォルダのパスが入らない', drafts.map((b) => /Fruity (Balance|Filter|Delay 2)\\|Fake (V|Gl)\\/.test(b)), [false, false, false]);
  ok('本文の参照は #{{NNN}} の形だけ(#数字・{{ の残り・@@・L-0 が無い)', drafts.map((b) => [/#\d/.test(b), b.replace(/#\{\{\d{3}\}\}/g, '').includes('{{'), b.includes('@@'), b.includes('L-0')]), [[false, false, false, false], [false, false, false, false], [false, false, false, false]]);
  ok('本文の参照: 依存と親を、#{{NNN}} で書く(#{{126}}・#{{127}}・#{{021}}・#{{022}})', [b041.includes('#{{126}}') && b041.includes('#{{127}}') && b041.includes('#{{021}}'), b043.includes('#{{022}}')], [true, true]);
  ok('accept: 各 Issue に 1 つ・1 行。task-accept が通る', drafts.map((b) => {
    const a = spawnSync(process.execPath, [TASK_ACCEPT], { input: b, encoding: 'utf8' });
    return [a.status, a.stdout.startsWith('accept: ')];
  }), [[0, true], [0, true], [0, true]]);
  ok('accept: 各フォルダの dir(名前の slug)を引用符で囲んだ引数を差し込む(041 は 3 フォルダ、042 は Fake Two を含む 2 フォルダ、043 は 2 フォルダ)', [b041.includes("'fake-a' 'fake-b' 'fake-c'"), b042.includes("'fake-d' 'fake-two'"), b043.includes("'fake-g1' 'fake-g2'")], [true, true, true]);
  ok('偽の表の束も、place-plugin-dlls の手順の 1 行と、accept の直前の 1 回を持つ', drafts.map((b) => [b.includes(PLACE_NOTE), b.split(PLACE).length === 2]), [[true, true], [true, true], [true, true]]);

  // 既存の表(検査の中の固定の表。親の 021・022 と、テンプレートが参照する番号は試験用)と、試験用のマイルストーン表を読む create-issues の dry-run で、parent・blocked_by・本文の参照が解決され、検査が通る。実物の表・実物の下書きには依存しない
  const table = path.join(tmp, 'table.tsv');
  const exRows = ['021\t#1322\t試験用の親(Effects)\tFeature', '022\t#1323\t試験用の親(Generators)\tFeature', ...[125, 126, 127, 154, 157, 169, 170, 172].map((n) => `${n}\t#90${n}\t試験\tTask`)];
  fs.writeFileSync(table, ['下書き番号\tIssue 番号\t題名\tType', ...exRows, ''].join('\n'));
  const msFix = path.join(tmp, 'milestones.md');
  fs.writeFileSync(msFix, ['# マイルストーン(試験)', '', '| 番号 | タイトル | 対応 |', '| --- | --- | --- |', `| M5 | ${MILESTONE} | - |`, '', `### ${MILESTONE}`, '', '- 説明: 試験用。', '- 完了の条件: 無し', ''].join('\n'));
  const cdry = spawnSync(process.execPath, [CI, '--drafts', out, '--existing', table, '--milestones', msFix, '--state', path.join(tmp, 'st.json')], { encoding: 'utf8' });
  ok('生成した下書きは、create-issues の検査を通る(既存の表と、125〜127 の参照を解決する)', [cdry.status, /021 -> #1322/.test(cdry.stdout), /022 -> #1323/.test(cdry.stdout), /125 -> #90125/.test(cdry.stdout), /126 -> #90126/.test(cdry.stdout), /127 -> #90127/.test(cdry.stdout), fs.existsSync(path.join(tmp, 'st.json'))], [0, true, true, true, true, true, false]);

  // TODO が残る(テンプレートを差し替えた)ときは、何も書かない
  const todoTpl = path.join(tmp, 'template-todo.md');
  fs.writeFileSync(todoTpl, `${TODO}: 仮\n${fs.readFileSync(fixTpl, 'utf8')}`);
  const outTodo = path.join(tmp, 'fix-todo');
  const t = gen(['--classes', fixCls, '--hooks', fixHk, '--deferred', fixDef, '--template', todoTpl, '--out', outTodo]);
  ok('TODO の行(#1315 を含む)が残るテンプレートは、本文の #数字 として 1 で止まり、何も書かない', [t.status, t.stderr.includes('#数字'), countDrafts(outTodo)], [1, true, 0]);
  const badTpl = path.join(tmp, 'template-bad.md');
  fs.writeFileSync(badTpl, `${fs.readFileSync(fixTpl, 'utf8')}\n本文に #1315 の数字だけの参照。\n`);
  const outBad = path.join(tmp, 'fix-bad');
  const bad = gen(['--classes', fixCls, '--hooks', fixHk, '--deferred', fixDef, '--template', badTpl, '--out', outBad]);
  ok('#数字だけの参照が本文に在れば、1 で止まり、何も書かない', [bad.status, countDrafts(outBad)], [1, 0]);

  // 後回しと VFX の DLL の集合が違えば、1 で止まる(何も書かない)
  const defMismatch = path.join(tmp, 'deferred-mismatch.tsv');
  fs.writeFileSync(defMismatch, ['guest\treason\timports', `${P}Effects\\Fake V\\engine_x64.dll\tx\t`, ''].join('\n'));
  const outMis = path.join(tmp, 'fix-mis');
  const mis = gen(['--classes', fixCls, '--hooks', fixHk, '--deferred', defMismatch, '--template', fixTpl, '--out', outMis]);
  ok('後回しの一覧と VFX が yes の DLL が違えば、1 で止まり、何も書かない', [mis.status, mis.stderr.includes('違う'), countDrafts(outMis)], [1, true, 0]);
  const defUnknown = path.join(tmp, 'deferred-bad.tsv');
  fs.writeFileSync(defUnknown, ['guest\treason\timports', `${P}Generators\\Nothing\\Nothing_x64.dll\tx\t`, ''].join('\n'));
  const outDef = path.join(tmp, 'fix-def');
  const d = gen(['--classes', fixCls, '--hooks', fixHk, '--deferred', defUnknown, '--template', fixTpl, '--out', outDef]);
  ok('後回しの一覧に、分類の表に無い DLL が在れば、1 で止まり、何も書かない', [d.status, countDrafts(outDef)], [1, 0]);

  // 本体の無いフォルダ(補助だけ)が在れば、1 で止まる
  const noBody = path.join(tmp, 'classes-nobody.tsv');
  fs.writeFileSync(noBody, [HEAD.join('\t'), ...classes, row(`${P}Effects\\Fake Orphan\\engine_x64.dll`, 10, '補助', '不要', 'no', '', 'kernel32.dll', '本体が無い'), ''].join('\n'));
  const outNo = path.join(tmp, 'fix-nobody');
  const nb = gen(['--classes', noBody, '--hooks', fixHk, '--deferred', fixDef, '--template', fixTpl, '--out', outNo]);
  ok('本体の無いフォルダが在れば、1 で止まり、何も書かない', [nb.status, nb.stderr.includes('本体の無いフォルダ'), countDrafts(outNo)], [1, true, 0]);

  // accept の引数に引用できない字(')が在るフォルダの名前は、1 で止まり、何も書かない
  const quoteCls = path.join(tmp, 'classes-quote.tsv');
  fs.writeFileSync(quoteCls, [HEAD.join('\t'), ...classes, row(`${P}Effects\\Fake's\\Fake's_x64.dll`, 10, '本体', '不要', 'no', B, K, '引用符を含む名前'), ''].join('\n'));
  const outQuote = path.join(tmp, 'fix-quote');
  const qt = gen(['--classes', quoteCls, '--hooks', fixHk, '--deferred', fixDef, '--template', fixTpl, '--out', outQuote]);
  const quoteDraft = fs.existsSync(outQuote) ? fs.readdirSync(outQuote).map((f) => fs.readFileSync(path.join(outQuote, f), 'utf8')).find((b) => b.includes('lift/plugins/fakes/plugin.tsv')) : undefined;
  ok('フォルダの名前に引用符が在っても、accept の引数は引用符の無い dir(fakes)だけ(名前は入らない)', [qt.status, quoteDraft !== undefined, quoteDraft === undefined ? true : quoteDraft.split('\n').filter((l) => l.startsWith('test -f') || l.includes('plugin-run/run.mjs')).every((l) => !l.includes("Fake's"))], [0, true, true]);

  const u = gen(['--start', 'x', ...fixArgs]);
  ok('使い方の誤り(--start が 3 桁でない)は 2', u.status, 2);
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}

process.exitCode = failed ? 1 : 0;
