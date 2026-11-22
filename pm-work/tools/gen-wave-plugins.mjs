#!/usr/bin/env node
// pm-work/tools/gen-wave-plugins.mjs [--dry-run] [--installer FILE] [--deferred FILE] [--template FILE] [--out DIR] [--start NNN] : 第 2 波の下書き(内蔵プラグインの DLL を 3 件ずつ束ねた Issue)を作る。GitHub には書かない(書くのは create-issues.mjs --existing)。
// 入力: 一覧(既定 inputs/fl-installer-files.tsv)の Plugins\Fruity\Effects\ と Plugins\Fruity\Generators\ の DLL(種類が dll)。後回しの一覧(既定 inputs/vfx-deferred.tsv。OpenGL を使う VFX 系)と、第 1 波で扱う 3 件を除く。
// 束: Effects と Generators を別々に、guest の名前順に 3 件ずつ(各種類の最後の束だけ端数)。下書き番号は --start(既定 041)から、Effects の束、Generators の束の順。親は Effects が 021、Generators が 022。
// 本文: テンプレート(既定 pm-work/issue-drafts/wave2-template.md)の @@KIND@@・@@NAMES@@・@@PARENT_REF@@・@@PARENT_LABEL@@・@@PLUGINS@@・@@PATTERNS@@ を、束の内容で置き換える。
// 拒否(終了コード 1。何も書かない): テンプレートに TODO(#1315 の結果で確定)が残る、本文に #数字 や L-0 が在る、置き換えの後に @@ が残る、後回しや第 1 波の DLL が一覧に無い。
// --dry-run: 件数と、束の先頭 5 件を出す。検査は同じに行い、書かない。
// 出力: <出力先>/<NNN>-<effects|generators>-<NN>.md。出力先の同じ形式の古い下書きは消してから書く(他の名前のファイルは残す)。
// 終了コード: 0 = 生成(--dry-run は検査が通った)、1 = 検査の失敗(理由を標準エラーに出す)、2 = 使い方の誤り。
import { existsSync, mkdirSync, readFileSync, readdirSync, unlinkSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const defaults = {
  installer: join(root, 'inputs', 'fl-installer-files.tsv'),
  deferred: join(root, 'inputs', 'vfx-deferred.tsv'),
  template: join(root, 'pm-work', 'issue-drafts', 'wave2-template.md'),
  out: join(root, 'pm-work', 'issue-drafts', 'wave2'),
  start: '041',
};
const FLAGS = new Map([['--installer', 'installer'], ['--deferred', 'deferred'], ['--template', 'template'], ['--out', 'out'], ['--start', 'start']]);
const TODO = 'TODO(#1315 の結果で確定)';
const MILESTONE = 'M5 内蔵のエフェクトとインストゥルメント';
const BLOCKED_BY = ['015', '019'];
const PER_ISSUE = 3;
const PREFIX = 'Plugins\\Fruity\\';
// 第 1 波で扱う 3 件(#1315 と #1319 の対象)
const FIRST_WAVE = [
  `${PREFIX}Effects\\Fruity Balance\\Fruity Balance_x64.dll`,
  `${PREFIX}Effects\\Fruity Filter\\Fruity Filter_x64.dll`,
  `${PREFIX}Effects\\Fruity Delay 2\\Fruity Delay 2_x64.dll`,
];
const GROUPS = [
  { dir: 'Effects', slug: 'effects', parent: '021', word: '内蔵エフェクト', kind: '内蔵のエフェクト', label: '内蔵 Effects の残り' },
  { dir: 'Generators', slug: 'generators', parent: '022', word: '内蔵インストゥルメント', kind: '内蔵のインストゥルメント', label: '内蔵インストゥルメント' },
];
const OUT_NAME = /^\d{3}-(effects|generators)-\d{2}\.md$/;
const HEAD = 'guest\tbytes\tsha256\tkind\timports';

const die = (lines, code = 1) => {
  for (const l of lines) process.stderr.write(`gen-wave-plugins: ${l}\n`);
  process.exit(code);
};
const usage = (msg) => die([msg, 'usage: gen-wave-plugins.mjs [--dry-run] [--installer FILE] [--deferred FILE] [--template FILE] [--out DIR] [--start NNN]'], 2);

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
const inScope = (guest) => guest.startsWith(`${PREFIX}Effects\\`) || guest.startsWith(`${PREFIX}Generators\\`);
const under = (guest, dir) => guest.startsWith(`${PREFIX}${dir}\\`);
const cmp = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

// 一覧: 見出しの確認。Effects・Generators の .dll は、種類が dll でなければ一覧が壊れている
const [head, ...body] = rows(opt.installer);
if (!head || head.join('\t') !== HEAD) die([`一覧の見出しが違う(${opt.installer})`]);
const all = body.map((r) => ({ guest: r[0], bytes: r[1], sha256: r[2], kind: r[3], imports: r[4] ?? '' }));
const stray = all.filter((x) => inScope(x.guest) && /\.dll$/i.test(x.guest) && x.kind !== 'dll');
if (stray.length) die([`一覧の .dll の種類が dll でない: ${stray.map((x) => x.guest).join(', ')}`]);
const dlls = all.filter((x) => x.kind === 'dll' && inScope(x.guest) && /\.dll$/i.test(x.guest));
if (new Set(dlls.map((d) => d.guest)).size !== dlls.length) die(['一覧の DLL に、同じ guest が 2 つ在る']);

const deferredRows = rows(opt.deferred);
if (!deferredRows[0] || deferredRows[0][0] !== 'guest') die([`後回しの一覧の見出しが違う(${opt.deferred})`]);
const deferred = new Set(deferredRows.slice(1).map((r) => r[0]));
const guests = new Set(dlls.map((d) => d.guest));
const missing = [...deferred, ...FIRST_WAVE].filter((g) => !guests.has(g));
if (missing.length) die([`後回し・第 1 波の DLL が一覧の DLL に無い: ${missing.join(', ')}`]);

const isDeferred = (d) => deferred.has(d.guest);
const left = dlls.filter((d) => !isDeferred(d) && !FIRST_WAVE.includes(d.guest));

// 表示の名前: 一覧の guest から、dir の下のパスを取り、拡張子と _x64 を外す(同じ名前の重なりは 1 つにする)
function displayName(guest, dir) {
  const rel = guest.slice(`${PREFIX}${dir}\\`.length).replace(/\.dll$/i, '').replace(/_x64$/i, '');
  const parts = [];
  for (const p of rel.split('\\')) if (!parts.length || parts[parts.length - 1].toLowerCase() !== p.toLowerCase()) parts.push(p);
  return parts.join('/');
}
const escapeRe = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
// accept の条件: 名前の空白・区切りを任意の 1 字に読む(大文字と小文字は無視)
const patternOf = (name) => `/${name.split(/[ /]+/).map(escapeRe).join('.?').toLowerCase()}/i`;

const pluginList = (members, dir) => members.map((d) => [
  `- \`${d.guest}\``,
  `  - 名前: ${displayName(d.guest, dir)}`,
  `  - 大きさ: ${d.bytes} B`,
  `  - SHA-256: ${d.sha256}`,
  `  - import: ${d.imports.split(';').filter(Boolean).join('; ')}`,
].join('\n')).join('\n');

const template = (() => {
  if (!existsSync(opt.template)) die([`テンプレートが無い: ${opt.template}`]);
  return readFileSync(opt.template, 'utf8').replace(/\r\n/g, '\n');
})();
const todo = template.includes(TODO);
const errors = [];
if (todo) errors.push(`テンプレートに ${TODO} が残っている。#1315 の結果で手順を確定してから生成する`);
if (template.includes('L-0')) errors.push('テンプレートに内部の ID(L-0)が在る');

const drafts = [];
let next = Number(opt.start);
for (const g of GROUPS) {
  const items = left.filter((d) => under(d.guest, g.dir)).sort((a, b) => cmp(a.guest, b.guest));
  let index = 0;
  for (let i = 0; i < items.length; i += PER_ISSUE) {
    index++;
    const n = String(next++).padStart(3, '0');
    const members = items.slice(i, i + PER_ISSUE);
    const names = members.map((d) => displayName(d.guest, g.dir));
    const title = `${g.word} ${index}: ${names.join('・')} を、1 プラグイン 1 wasm で入れる`;
    const subst = [
      ['@@KIND@@', g.kind],
      ['@@NAMES@@', names.join('・')],
      ['@@PARENT_REF@@', `#{{${g.parent}}}`],
      ['@@PARENT_LABEL@@', g.label],
      ['@@PLUGINS@@', pluginList(members, g.dir)],
      ['@@PATTERNS@@', names.map(patternOf).join(', ')],
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

const count = (list, dir) => list.filter((d) => under(d.guest, dir)).length;
const bundles = (dir) => drafts.filter((d) => d.dir === dir).length;
const plan = [
  ...(dryRun ? ['dry-run: 書いていない。'] : []),
  `DLL: Effects ${count(dlls, 'Effects')} 件、Generators ${count(dlls, 'Generators')} 件(計 ${dlls.length} 件)`,
  `除く: 後回し ${dlls.filter(isDeferred).length} 件(Effects ${count(dlls.filter(isDeferred), 'Effects')}・Generators ${count(dlls.filter(isDeferred), 'Generators')})、第 1 波 ${FIRST_WAVE.length} 件`,
  `残り: Effects ${count(left, 'Effects')} 件、Generators ${count(left, 'Generators')} 件`,
  `束(${PER_ISSUE} 件ずつ): Effects ${bundles('Effects')} 束、Generators ${bundles('Generators')} 束。計 ${drafts.length} 束。下書き番号 ${drafts[0]?.n ?? '-'}〜${drafts.at(-1)?.n ?? '-'}`,
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
