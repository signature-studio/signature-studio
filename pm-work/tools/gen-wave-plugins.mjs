#!/usr/bin/env node
// pm-work/tools/gen-wave-plugins.mjs [--dry-run] [--classes FILE] [--deferred FILE] [--template FILE] [--out DIR] [--start NNN] : 第 2 波の下書き(内蔵プラグインのフォルダを 3 つずつ束ねた Issue)を作る。GitHub には書かない(書くのは create-issues.mjs --existing)。
// 入力: 分類の表(既定 inputs/plugin-classes.tsv。#1343 の成果)の Plugins\Fruity\Effects\ と Plugins\Fruity\Generators\ の DLL。単位はプラグインのフォルダ(Plugins\Fruity\<種別>\<フォルダ>\)で、本体と補助の DLL は同じフォルダに在る。
// 除く: VFX 系のフォルダ(フォルダの DLL に VFX が yes の物が 1 つでも在る。補助の DLL が OpenGL を import する場合を含む)と、第 1 波の 3 フォルダ(Fruity Balance・Fruity Filter・Fruity Delay 2)。後回しの一覧(既定 inputs/vfx-deferred.tsv)は、VFX が yes の DLL と同じ集合でなければ拒否する。
// 束: Effects と Generators を別々に、フォルダの名前順に 3 フォルダずつ(各種類の最後の束だけ端数)。下書き番号は --start(既定 041)から、Effects の束、Generators の束の順。親は Effects が 021、Generators が 022。
// 本文: テンプレート(既定 pm-work/issue-drafts/wave2-template.md)の @@KIND@@・@@NAMES@@・@@PARENT_REF@@・@@PARENT_LABEL@@・@@PLUGINS@@・@@ARGS@@ を、束の内容で置き換える。@@PLUGINS@@ は、フォルダごとの本体の DLL(名前・大きさ・SHA-256・export・import・hook の要否)と補助の DLL(名前・大きさ・備考)。@@ARGS@@ は、フォルダの名前を引用符で囲んだ並び(#{{125}} の道具の引数)。
// 依存: blocked_by は 125・126・127(#{{125}} の仕組み、#{{126}} の 2 本目、#{{127}} の道具)。1 プラグインは lift/plugins/<名前>/plugin.tsv だけで入る(共有のファイルを変えない)。
// 拒否(終了コード 1。何も書かない): テンプレートに TODO(#1315 の結果で確定)が残る、本文に #数字 や L-0 が在る、置き換えの後に @@ が残る、後回しと VFX の DLL の集合が違う、後回し・第 1 波のフォルダが表に無い、本体の無いフォルダが在る、フォルダの名前に引用符で囲めない字が在る。
// --dry-run: 件数と、束の先頭 5 件を出す。検査は同じに行い、書かない(TODO が残るときは、件数を出した後に 1 で止まる)。
// 出力: <出力先>/<NNN>-<effects|generators>-<NN>.md。出力先の同じ形式の古い下書きは消してから書く(他の名前のファイルは残す)。
// 終了コード: 0 = 生成(--dry-run は検査が通った)、1 = 検査の失敗(理由を標準エラーに出す)、2 = 使い方の誤り。
import { existsSync, mkdirSync, readFileSync, readdirSync, unlinkSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const defaults = {
  classes: join(root, 'inputs', 'plugin-classes.tsv'),
  deferred: join(root, 'inputs', 'vfx-deferred.tsv'),
  template: join(root, 'pm-work', 'issue-drafts', 'wave2-template.md'),
  out: join(root, 'pm-work', 'issue-drafts', 'wave2'),
  start: '041',
};
const FLAGS = new Map([['--classes', 'classes'], ['--deferred', 'deferred'], ['--template', 'template'], ['--out', 'out'], ['--start', 'start']]);
const TODO = 'TODO(#1315 の結果で確定)';
const MILESTONE = 'M5 内蔵のエフェクトとインストゥルメント';
const BLOCKED_BY = ['125', '126', '127'];
const PER_BUNDLE = 3;
const PREFIX = 'Plugins\\Fruity\\';
// 第 1 波で扱う 3 フォルダ(#1315 と #1319 の対象)
const FIRST_WAVE = ['Effects\\Fruity Balance', 'Effects\\Fruity Filter', 'Effects\\Fruity Delay 2'];
const GROUPS = [
  { dir: 'Effects', slug: 'effects', parent: '021', word: '内蔵エフェクト', kind: '内蔵のエフェクト', label: '内蔵 Effects の残り' },
  { dir: 'Generators', slug: 'generators', parent: '022', word: '内蔵インストゥルメント', kind: '内蔵のインストゥルメント', label: '内蔵インストゥルメント' },
];
const HEAD = ['パス', '大きさ', 'SHA-256', '種別', '本体か補助か', 'export 名', 'import した DLL', 'hook の要否', 'VFX', '備考'];
const HOOK = ['要', '既存', '不要'];
const OUT_NAME = /^\d{3}-(effects|generators)-\d{2}\.md$/;

const die = (lines, code = 1) => {
  for (const l of lines) process.stderr.write(`gen-wave-plugins: ${l}\n`);
  process.exit(code);
};
const usage = (msg) => die([msg, 'usage: gen-wave-plugins.mjs [--dry-run] [--classes FILE] [--deferred FILE] [--template FILE] [--out DIR] [--start NNN]'], 2);

const argv = process.argv.slice(2);
const opt = { ...defaults };
let dryRun = false;
for (let i = 0; i < argv.length; i++) {
  if (argv[i] === '--dry-run') { dryRun = true; continue; }
  if (FLAGS.has(argv[i]) && argv[i + 1] !== undefined) { opt[FLAGS.get(argv[i])] = argv[++i]; continue; }
  usage(`不明な引数: ${argv[i]}`);
}
if (!/^\d{3}$/.test(opt.start)) usage(`--start は 3 桁の番号(実際: ${opt.start})`);

const rows = (file) => {
  if (!existsSync(file)) die([`入力が無い: ${file}`]);
  return readFileSync(file, 'utf8').replace(/\r\n/g, '\n').split('\n').filter((l) => l !== '' && !l.startsWith('#')).map((l) => l.split('\t'));
};
const cmp = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
const splitList = (s) => s.split(';').filter(Boolean);
const folderOf = (guest) => guest.split('\\').slice(0, 4).join('\\');
const folderName = (folder) => folder.split('\\')[3];
const baseName = (guest) => guest.slice(guest.lastIndexOf('\\') + 1).replace(/\.dll$/i, '').replace(/_x64$/i, '');
// accept の引数: フォルダの名前を、シングルクォートで囲んで 1 語にする(名前に ' と、シェルで展開される字が無いことを検査で確かめる)
const quotedName = (name) => `'${name}'`;

const [head, ...body] = rows(opt.classes);
if (!head || head.join('\t') !== HEAD.join('\t')) die([`分類の表の見出しが違う(${opt.classes})`]);
const all = body.map((r) => Object.fromEntries(HEAD.map((h, i) => [h, r[i] ?? ''])));
const kinds = GROUPS.map((g) => g.dir);
const shape = [];
const seen = new Set();
for (const r of all) {
  const p = r['パス'];
  if (seen.has(p)) shape.push(`分類の表に同じパスが 2 つ在る: ${p}`);
  seen.add(p);
  if (!kinds.includes(r['種別']) || !p.startsWith(`${PREFIX}${r['種別']}\\`) || p.split('\\').length < 5 || !/\.dll$/i.test(p)) shape.push(`種別・パスの形が違う: ${p}`);
  if (!['本体', '補助'].includes(r['本体か補助か'])) shape.push(`本体か補助かが違う: ${p}`);
  if (!HOOK.includes(r['hook の要否'])) shape.push(`hook の要否が違う: ${p}`);
  if (!['yes', 'no'].includes(r['VFX'])) shape.push(`VFX が違う: ${p}`);
}
if (shape.length) die(shape);

const folders = new Map();
for (const r of all) {
  const f = folderOf(r['パス']);
  if (!folders.has(f)) folders.set(f, []);
  folders.get(f).push(r);
}
const folderShape = [];
for (const [f, list] of folders) {
  if (new Set(list.map((r) => r['種別'])).size !== 1) folderShape.push(`フォルダ ${f} の中で種別が混ざっている`);
  if (!list.some((r) => r['本体か補助か'] === '本体')) folderShape.push(`本体の無いフォルダ: ${f}`);
}
const missingFirst = FIRST_WAVE.filter((n) => !folders.has(`${PREFIX}${n}`));
if (missingFirst.length) folderShape.push(`第 1 波のフォルダが表に無い: ${missingFirst.join(', ')}`);
if (folderShape.length) die(folderShape);

const deferredRows = rows(opt.deferred);
if (!deferredRows[0] || deferredRows[0][0] !== 'guest') die([`後回しの一覧の見出しが違う(${opt.deferred})`]);
const deferred = new Set(deferredRows.slice(1).map((r) => r[0]));
const guests = new Set(all.map((r) => r['パス']));
const vfxYes = new Set(all.filter((r) => r['VFX'] === 'yes').map((r) => r['パス']));
const notInTable = [...deferred].filter((g) => !guests.has(g));
if (notInTable.length) die([`後回しの一覧の DLL が分類の表に無い: ${notInTable.join(', ')}`]);
const mismatch = [...new Set([...deferred, ...vfxYes])].filter((g) => deferred.has(g) !== vfxYes.has(g));
if (mismatch.length) die([`後回しの一覧と、VFX が yes の DLL が違う: ${mismatch.join(', ')}`]);

const isFirst = (f) => FIRST_WAVE.some((n) => f === `${PREFIX}${n}`);
const isVfx = (list) => list.some((r) => r['VFX'] === 'yes');
const left = [...folders].filter(([f, list]) => !isFirst(f) && !isVfx(list)).sort(([a], [b]) => cmp(a, b));

const pluginList = (members) => members.map(([f, list]) => {
  const bodies = list.filter((r) => r['本体か補助か'] === '本体');
  const aux = list.filter((r) => r['本体か補助か'] === '補助');
  const lines = [`- フォルダ \`${f}\\\`(本体 ${bodies.length}・補助 ${aux.length})`];
  for (const r of bodies) {
    lines.push(`  - 本体: \`${r['パス']}\``);
    lines.push(`    - 名前: ${baseName(r['パス'])}`);
    lines.push(`    - 大きさ: ${r['大きさ']} B`);
    lines.push(`    - SHA-256: ${r['SHA-256']}`);
    lines.push(`    - export: ${splitList(r['export 名']).join('; ')}`);
    lines.push(`    - import: ${splitList(r['import した DLL']).join('; ')}`);
    lines.push(`    - hook の要否: ${r['hook の要否']}`);
  }
  for (const r of aux) {
    lines.push(`  - 補助: \`${r['パス']}\``);
    lines.push(`    - 名前: ${baseName(r['パス'])}`);
    lines.push(`    - 大きさ: ${r['大きさ']} B`);
    lines.push(`    - 備考: ${r['備考']}`);
  }
  return lines.join('\n');
}).join('\n');

const template = (() => {
  if (!existsSync(opt.template)) die([`テンプレートが無い: ${opt.template}`]);
  return readFileSync(opt.template, 'utf8').replace(/\r\n/g, '\n');
})();
const todo = template.includes(TODO);
const errors = [];
if (todo) errors.push(`テンプレートに ${TODO} が残っている。#1342 の結果で手順を確定してから生成する`);
if (template.includes('L-0')) errors.push('テンプレートに内部の ID(L-0)が在る');

const drafts = [];
let next = Number(opt.start);
for (const g of GROUPS) {
  const items = left.filter(([, list]) => list[0]['種別'] === g.dir);
  let index = 0;
  for (let i = 0; i < items.length; i += PER_BUNDLE) {
    index++;
    const n = String(next++).padStart(3, '0');
    const members = items.slice(i, i + PER_BUNDLE);
    const names = members.map(([f]) => folderName(f));
    for (const n of names) if (!/^[^'"\\$`\r\n]+$/.test(n)) errors.push(`フォルダの名前に accept の引数にできない字が在る: ${n}`);
    const title = `${g.word} ${index}: ${names.join('・')} を、1 プラグイン 1 wasm で入れる`;
    const subst = [
      ['@@KIND@@', g.kind],
      ['@@NAMES@@', names.join('・')],
      ['@@PARENT_REF@@', `#{{${g.parent}}}`],
      ['@@PARENT_LABEL@@', g.label],
      ['@@PLUGINS@@', pluginList(members)],
      ['@@ARGS@@', names.map(quotedName).join(' ')],
    ];
    let text = template;
    for (const [k, v] of subst) text = text.split(k).join(v);
    const fm = [
      '---',
      `draft: ${JSON.stringify(n)}`,
      `title: ${JSON.stringify(title)}`,
      'type: "Task"',
      `milestone: ${JSON.stringify(MILESTONE)}`,
      `parent: ${JSON.stringify(g.parent)}`,
      `blocked_by: ${JSON.stringify(BLOCKED_BY)}`,
      'labels: ["plugin-builtin"]',
      'size: "M"',
      'priority: "Medium"',
      'project: "signature-studio/3"',
      '---',
    ].join('\n');
    drafts.push({ n, title, file: `${n}-${g.slug}-${String(index).padStart(2, '0')}.md`, text: `${fm}\n\n${text.trimEnd()}\n`, body: text.trimEnd(), dir: g.dir });
  }
}

if (!todo) {
  for (const d of drafts) {
    if (/#\d/.test(d.body)) errors.push(`${d.file}: 本文に #数字 の参照が在る(#{{NNN}} の形で書く)`);
    if (d.body.replace(/#\{\{\d{3}\}\}/g, '').includes('{{')) errors.push(`${d.file}: 本文に #{{NNN}} の形以外の {{ が在る`);
    if (d.body.includes('@@')) errors.push(`${d.file}: 置き換えの後に @@ が残る`);
    if (d.body.includes('L-0')) errors.push(`${d.file}: 本文に内部の ID(L-0)が在る`);
  }
  if (new Set(drafts.map((d) => d.body)).size !== drafts.length) errors.push('本文が互いに同じ下書きが在る');
}

const dllsOf = (kind) => all.filter((r) => r['種別'] === kind).length;
const bodiesOf = (items, kind) => items.filter(([, list]) => list[0]['種別'] === kind).reduce((s, [, list]) => s + list.filter((r) => r['本体か補助か'] === '本体').length, 0);
const foldersOf = (items, kind) => items.filter(([, list]) => list[0]['種別'] === kind).length;
const excluded = [...folders].filter(([f, list]) => !left.some(([g]) => g === f));
const vfxFolders = excluded.filter(([f, list]) => !isFirst(f));
const firstFolders = excluded.filter(([f]) => isFirst(f));
const dllsIn = (items) => items.reduce((s, [, list]) => s + list.length, 0);
const bundles = (dir) => drafts.filter((d) => d.dir === dir).length;
const plan = [
  ...(dryRun ? ['dry-run: 書いていない。'] : []),
  `DLL: Effects ${dllsOf('Effects')} 件、Generators ${dllsOf('Generators')} 件(計 ${all.length} 件)`,
  `フォルダ: Effects ${foldersOf([...folders], 'Effects')}、Generators ${foldersOf([...folders], 'Generators')}(計 ${folders.size})`,
  `除く: VFX 系 ${vfxFolders.length} フォルダ(Effects ${foldersOf(vfxFolders, 'Effects')}・Generators ${foldersOf(vfxFolders, 'Generators')}。DLL ${dllsIn(vfxFolders)} 件)、第 1 波 ${firstFolders.length} フォルダ(DLL ${dllsIn(firstFolders)} 件)`,
  `残り: Effects ${foldersOf(left, 'Effects')} フォルダ(本体 ${bodiesOf(left, 'Effects')} 件)、Generators ${foldersOf(left, 'Generators')} フォルダ(本体 ${bodiesOf(left, 'Generators')} 件)`,
  `束(${PER_BUNDLE} フォルダずつ): Effects ${bundles('Effects')} 束、Generators ${bundles('Generators')} 束。計 ${drafts.length} 束。下書き番号 ${drafts[0]?.n ?? '-'}〜${drafts.at(-1)?.n ?? '-'}`,
  '先頭 5 件:',
  ...drafts.slice(0, 5).map((d) => `  ${d.n} ${d.title}`),
];
process.stdout.write(`${plan.join('\n')}\n`);
if (errors.length) die(errors);
if (dryRun) process.exit(0);

const outDir = resolve(opt.out);
mkdirSync(outDir, { recursive: true });
for (const f of readdirSync(outDir)) if (OUT_NAME.test(f)) unlinkSync(join(outDir, f));
for (const d of drafts) writeFileSync(join(outDir, d.file), d.text);
process.stdout.write(`書いた: ${drafts.length} 件 -> ${relative(root, outDir) || outDir}\n`);
