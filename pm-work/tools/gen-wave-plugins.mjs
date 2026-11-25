#!/usr/bin/env node
// pm-work/tools/gen-wave-plugins.mjs [--dry-run] [--classes FILE] [--deferred FILE] [--hooks FILE] [--installer FILE] [--template FILE] [--out DIR] [--start NNN] : 第 2 波の下書き(内蔵プラグインのフォルダを 3 つずつ束ねた Issue)を作る。GitHub には書かない(書くのは create-issues.mjs --existing)。
// 入力: 分類の表(既定 inputs/plugin-classes.tsv)・hook 点の表(inputs/plugin-hooks.tsv)・インストーラのファイルの一覧(inputs/fl-installer-files.tsv。Data の件数だけ)・後回しの一覧(inputs/vfx-deferred.tsv)。
// 単位はプラグインのフォルダ(Plugins\Fruity\<種別>\<フォルダ>\)。本体と補助の DLL は同じフォルダに在る。
// 除く: VFX 系のフォルダ(フォルダの DLL に VFX が yes の物が 1 つでも在る)と、lift/plugins/ に既に在る 4 フォルダ(Fruity Balance・Fruity Filter・Fruity Delay 2 = #1369・#1370。Sytrus = #1400)。
// 束: Effects の束の後に Generators の束。名前順に 3 フォルダずつ(各種類の最後の束だけ端数)。下書き番号は --start(既定 041)から。
// 段: 束の先頭から 15 束ずつ。段 1 は blocked_by が #1370・#1371 だけ。段 2 は段 1 の先頭 3 束も、段 3 は段 2 の先頭 3 束も blocked_by に足す(同時に走る束を段で抑える)。
// フォルダの dir: lift/plugins/<dir>/。dir はフォルダの名前を小文字にし、空白をハイフンにしたもの。Effects と Generators で同じ名前の 2 フォルダは、dir を <種別>-<名前> にする。
// key: <種別の小文字>.<dir>.<DLL の名(小文字。空白は _)>。hook: 本体の 要・既存 は hook 点の表の 3 つの RVA(rva:)、他は none。omit は -(setjmp の入口は #1370 の自動の検出が決める)。args は --pdata。
// 本文: テンプレート(既定 pm-work/issue-drafts/wave2-template.md)の @@...@@ を束の内容で置き換える。plugin.tsv の行は、本文の tsv の段に全文で書く。
// 拒否(終了コード 1。何も書かない): テンプレートに @@ 以外の未確定の語が在る、本文に #数字 や L-0 が在る、置き換えの後に @@ が残る、後回しと VFX の DLL の集合が違う、第 1 波の 4 フォルダが表に無い、本体の無いフォルダが在る、dir・key・guest が一意でない、禁止の数値が散文に在る(SHA-256 の列を除く)。
// --dry-run: 件数と段と先頭 5 件を出す。検査は同じに行い、書かない。
// 出力: <出力先>/<NNN>-<effects|generators>-<NN>.md。出力先の同じ形式の古い下書きは消してから書く(他の名前のファイルは残す)。
// 終了コード: 0 = 生成(--dry-run は検査が通った)、1 = 検査の失敗(理由を標準エラーに出す)、2 = 使い方の誤り。
import { existsSync, mkdirSync, readFileSync, readdirSync, unlinkSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const defaults = {
  classes: join(root, 'inputs', 'plugin-classes.tsv'),
  deferred: join(root, 'inputs', 'vfx-deferred.tsv'),
  hooks: join(root, 'inputs', 'plugin-hooks.tsv'),
  installer: join(root, 'inputs', 'fl-installer-files.tsv'),
  template: join(root, 'pm-work', 'issue-drafts', 'wave2-template.md'),
  out: join(root, 'pm-work', 'issue-drafts', 'wave2'),
  start: '041',
};
const FLAGS = new Map([['--classes', 'classes'], ['--deferred', 'deferred'], ['--hooks', 'hooks'], ['--installer', 'installer'], ['--template', 'template'], ['--out', 'out'], ['--start', 'start']]);
const MILESTONE = 'M5 内蔵のエフェクトとインストゥルメント';
const BASE_BLOCKED = ['126', '127'];
const STAGE_SIZE = 15;
const HEAD_BLOCK = 3;
const PREFIX = 'Plugins\\Fruity\\';
// lift/plugins/ に既に在る 4 フォルダ: 第 1 波の Balance・Filter・Delay 2(#1369・#1370)と、Sytrus(#1400)
const DONE = ['Effects\\Fruity Balance', 'Effects\\Fruity Filter', 'Effects\\Fruity Delay 2', 'Generators\\Sytrus'];
const KINDS = [
  { dir: 'Effects', lc: 'effects', parent: '021', word: '内蔵エフェクト', kind: '内蔵のエフェクト', kindJa: 'エフェクト', label: '内蔵 Effects の残り' },
  { dir: 'Generators', lc: 'generators', parent: '022', word: '内蔵インストゥルメント', kind: '内蔵のインストゥルメント', kindJa: 'インストゥルメント', label: '内蔵インストゥルメント' },
];
const HEAD = ['パス', '大きさ', 'SHA-256', '種別', '本体か補助か', 'export 名', 'import した DLL', 'hook の要否', 'VFX', '備考'];
const HOOK_NEED = ['要', '既存', '不要'];
const TSV_HEAD = 'key\tguest\trole\tsha256\tsize\thook\targs\tomit';
const FORBIDDEN_NUMBERS = /175|168|169|1\.7647/;
const OUT_NAME = /^\d{3}-(effects|generators)-\d{2}\.md$/;

const die = (lines, code = 1) => {
  for (const l of lines) process.stderr.write(`gen-wave-plugins: ${l}\n`);
  process.exit(code);
};
const usage = (msg) => die([msg, 'usage: gen-wave-plugins.mjs [--dry-run] [--classes FILE] [--deferred FILE] [--hooks FILE] [--installer FILE] [--template FILE] [--out DIR] [--start NNN]'], 2);

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
const baseName = (guest) => guest.slice(guest.lastIndexOf('\\') + 1);
const slugOf = (name) => name.toLowerCase().replace(/ /g, '-').replace(/[^a-z0-9-]/g, '').replace(/-+/g, '-').replace(/^-|-$/g, '');
const dllKey = (guest) => baseName(guest).toLowerCase().replace(/ /g, '_').replace(/[^a-z0-9._-]/g, '_');
const quoted = (s) => `'${s}'`;

const [head, ...body] = rows(opt.classes);
if (!head || head.join('\t') !== HEAD.join('\t')) die([`分類の表の見出しが違う(${opt.classes})`]);
const all = body.map((r) => Object.fromEntries(HEAD.map((h, i) => [h, r[i] ?? ''])));
const kinds = KINDS.map((k) => k.dir);
const shape = [];
const seen = new Set();
for (const r of all) {
  const p = r['パス'];
  if (seen.has(p)) shape.push(`分類の表に同じパスが 2 つ在る: ${p}`);
  seen.add(p);
  if (!kinds.includes(r['種別']) || !p.startsWith(`${PREFIX}${r['種別']}\\`) || p.split('\\').length < 5 || !/\.dll$/i.test(p)) shape.push(`種別・パスの形が違う: ${p}`);
  if (!['本体', '補助'].includes(r['本体か補助か'])) shape.push(`本体か補助かが違う: ${p}`);
  if (!HOOK_NEED.includes(r['hook の要否'])) shape.push(`hook の要否が違う: ${p}`);
  if (!['yes', 'no'].includes(r['VFX'])) shape.push(`VFX が違う: ${p}`);
  if (!/^[0-9a-f]{64}$/.test(r['SHA-256'])) shape.push(`SHA-256 の形が違う: ${p}`);
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
const doneMissing = DONE.map((n) => `${PREFIX}${n}`).filter((f) => !folders.has(f));
if (doneMissing.length) folderShape.push(`lift/plugins/ に在るフォルダが表に無い: ${doneMissing.join(', ')}`);
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

const hookRows = new Map(rows(opt.hooks).slice(1).map((r) => [r[0], r.slice(1, 4)]));
const installer = new Map();
for (const r of rows(opt.installer).slice(1)) installer.set(r[0], Number(r[1]));

const isVfx = (list) => list.some((r) => r['VFX'] === 'yes');
const left = [...folders].filter(([f, list]) => !DONE.map((n) => `${PREFIX}${n}`).includes(f) && !isVfx(list)).sort(([a], [b]) => cmp(a, b));
const slugs = left.map(([f]) => slugOf(folderName(f)));
const slugCount = new Map();
for (const s of slugs) slugCount.set(s, (slugCount.get(s) ?? 0) + 1);
const dirs = left.map(([f, list], i) => {
  const kind = list[0]['種別'].toLowerCase();
  return slugCount.get(slugs[i]) > 1 ? `${kind}-${slugs[i]}` : slugs[i];
});
const dirErrors = [];
const doneDirs = DONE.map((n) => slugOf(n.split('\\')[1]));
const dirSeen = new Set();
dirs.forEach((d, i) => {
  if (!/^[a-z0-9-]+$/.test(d) || d === '') dirErrors.push(`dir が空か不正: ${folderName(left[i][0])} -> ${d}`);
  if (dirSeen.has(d)) dirErrors.push(`dir が一意でない: ${d}`);
  if (doneDirs.includes(d)) dirErrors.push(`dir が lift/plugins/ の既存と同じ: ${d}`);
  dirSeen.add(d);
});
if (dirErrors.length) die(dirErrors);

const keySeen = new Set();
const keyErrors = [];
const dllRows = left.map(([f, list], i) => {
  const kind = KINDS.find((k) => k.dir === list[0]['種別']);
  return list.map((r) => {
    const key = `${kind.lc}.${dirs[i]}.${dllKey(r['パス'])}`;
    if (keySeen.has(key)) keyErrors.push(`key が一意でない: ${key}`);
    keySeen.add(key);
    if (!/^[a-z0-9._-]+\.dll$/.test(key)) keyErrors.push(`key の形が違う: ${key}`);
    let hook = 'none';
    if (r['本体か補助か'] === '本体' && r['hook の要否'] !== '不要') {
      const h = hookRows.get(r['パス']);
      if (!h) keyErrors.push(`本体 ${r['パス']} の hook 点の行が無い(hook の要否 ${r['hook の要否']})`);
      else if (h.some((v) => !/^0x[0-9a-f]+$/.test(v))) keyErrors.push(`hook 点の RVA の形が違う: ${r['パス']}`);
      else hook = `rva:${h.join(',')}`;
    }
    return { row: r, key, role: r['本体か補助か'], hook, dir: dirs[i], kind };
  });
});
if (keyErrors.length) die(keyErrors);

const data = (f) => {
  let n = 0, bytes = 0;
  for (const [g, b] of installer) {
    if (g.startsWith(`${f}\\Data\\`) && !guests.has(g)) { n++; bytes += b; }
  }
  return { n, bytes };
};

const pluginTsv = (members) => [TSV_HEAD, ...members.map((d) => [d.key, d.row['パス'], d.role, d.row['SHA-256'], d.row['大きさ'], d.hook, '--pdata', '-'].join('\t'))].join('\n');

const bundles = [];
for (const k of KINDS) {
  const items = left.map((x, i) => ({ f: x[0], list: x[1], i })).filter((x) => x.list[0]['種別'] === k.dir);
  for (let i = 0; i < items.length; i += HEAD_BLOCK) {
    bundles.push({ kind: k, members: items.slice(i, i + HEAD_BLOCK), index: Math.floor(i / HEAD_BLOCK) + 1 });
  }
}
const stageOf = (g) => Math.floor(g / STAGE_SIZE) + 1;
const nums = bundles.map((_, g) => String(Number(opt.start) + g).padStart(3, '0'));
const stageFirst = (s) => bundles.map((b, g) => ({ g, n: nums[g] })).filter((x) => stageOf(x.g) === s).slice(0, 3).map((x) => x.n);

const dllsIn = (b) => b.members.reduce((s, m) => s + dllRows[m.i].length, 0);
const bodiesIn = (b) => b.members.reduce((s, m) => s + dllRows[m.i].filter((d) => d.role === '本体').length, 0);
const sizeFor = (n) => (n <= 3 ? 'S' : n <= 9 ? 'M' : 'L');

const planned = bundles.map((b, g) => {
  const n = nums[g];
  const stage = stageOf(g);
  const blocked = [...BASE_BLOCKED];
  if (stage >= 2) blocked.push(...stageFirst(stage - 1));
  const names = b.members.map((m) => folderName(m.f));
  const title = `${b.kind.word} ${b.index}: ${names.join('・')} を、1 プラグイン 1 wasm で入れる`;
  const dllsCount = dllsIn(b);
  const bodies = bodiesIn(b);
  return { g, n, stage, blocked, names, title, dllsCount, bodies, kind: b.kind, index: b.index, members: b.members, size: sizeFor(dllsCount) };
});

const template = (() => {
  if (!existsSync(opt.template)) die([`テンプレートが無い: ${opt.template}`]);
  return readFileSync(opt.template, 'utf8').replace(/\r\n/g, '\n');
})();
const errors = [];
if (template.includes('L-0')) errors.push('テンプレートに内部の ID(L-0)が在る');

const acceptOf = (p) => {
  const dirsOf = p.members.map((m) => dirs[m.i]);
  const tests = dirsOf.map((d) => `test -f lift/plugins/${d}/plugin.tsv`).join(' && ');
  const slugArgs = dirsOf.map(quoted).join(' ');
  const processed = p.kind.dir === 'Effects' ? '&&r.processed===true' : '';
  return [
    tests,
    'npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle',
    'node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json"',
    `node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json"`,
    `node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" ${slugArgs} > "$RUNNER_TEMP/plugin-run.jsonl"`,
    `node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\\n").map((l)=>JSON.parse(l));process.exit(L.length===${p.bodies}&&L.every((r)=>r.instantiated===true&&r.error===null&&r.elapsedMs<=12500${processed})?0:1)' "$RUNNER_TEMP/plugin-run.jsonl"`,
    'node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl"',
    `node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711&&(!/^[BD]$/.test(r.flow)||r.welcomeClosed===true);process.exit(L.length===4&&L.map((r)=>r.flow).join("")==="ABCD"&&L.every(ok)?0:1)' "$RUNNER_TEMP/flow.jsonl"`,
    `test -z "$(git diff --name-only origin/main...HEAD -- . ':!lift/plugins')"`,
  ].join(' && ');
};

const dependsText = (p) => {
  const base = "#{{126}}(第 2 波の 2 本目の型)・#{{127}}(1 本ずつの道具)";
  if (p.stage === 1) return `${base}。段 1 の束は、この 2 件が閉じたら起動する`;
  const first = p.blocked.slice(2).map((n) => `#{{${n}}}`).join('・');
  return `${base}・段 ${p.stage - 1} の先頭 3 束(${first})。段 ${p.stage - 1} の先頭 3 束が閉じるまで、この束は起動しない`;
};

const sectionOf = (p) => {
  const out = [];
  p.members.forEach((m, i) => {
    const f = m.f;
    const list = dllRows[m.i];
    const d = data(f);
    const bodies = list.filter((x) => x.role === '本体');
    const aux = list.filter((x) => x.role === '補助');
    out.push(`### ${folderName(f)}(\`lift/plugins/${dirs[m.i]}/plugin.tsv\`)`);
    out.push('');
    out.push(`- 種別: ${p.kind.kindJa}。本体 ${bodies.length}・補助 ${aux.length}。Data: ${d.n === 0 ? 'なし' : `${d.n} ファイル(${d.bytes.toLocaleString('en-US')} B。この束は配信しない)`}`);
    for (const x of bodies) {
      out.push(`- 本体: \`${x.row['パス']}\``);
      out.push(`  - 大きさ ${x.row['大きさ']} B・SHA-256 ${x.row['SHA-256']}`);
      out.push(`  - export: ${splitList(x.row['export 名']).join('; ') || 'なし'}`);
      out.push(`  - import: ${splitList(x.row['import した DLL']).join('; ') || 'なし'}`);
      out.push(`  - hook: ${x.hook === 'none' ? 'none(hook 点の表に行が無い、または不要)' : `${x.hook}(hook 点の表の 3 つの RVA)`}`);
    }
    for (const x of aux) {
      out.push(`- 補助: \`${x.row['パス']}\``);
      out.push(`  - 大きさ ${x.row['大きさ']} B・備考: ${x.row['備考'] || 'なし'}`);
    }
    out.push('');
    out.push('```tsv');
    out.push(pluginTsv(list.slice().sort((a, b) => (a.role === b.role ? cmp(a.key, b.key) : a.role === '本体' ? -1 : 1))));
    out.push('```');
    if (i < p.members.length - 1) out.push('');
  });
  return out.join('\n');
};

const drafts = [];
for (const p of planned) {
  const fill = {
    '@@KIND@@': p.kind.kind,
    '@@FOLDERS@@': String(p.members.length),
    '@@NAMES@@': p.names.join('・'),
    '@@BODIES@@': String(p.bodies),
    '@@AUX@@': String(p.dllsCount - p.bodies),
    '@@DLLS@@': String(p.dllsCount),
    '@@PARENT_REF@@': `#{{${p.kind.parent}}}`,
    '@@PARENT_LABEL@@': p.kind.label,
    '@@DEPENDS@@': dependsText(p),
    '@@PLUGINS@@': sectionOf(p),
    '@@ACCEPT@@': acceptOf(p),
    '@@EFFECTS_PROC@@': p.kind.dir === 'Effects' ? '、かつ `processed` が true' : '(インストゥルメントは、1 ブロックの処理を見ない。#{{127}} の範囲外)',
    '@@STAGE@@': `段 ${p.stage}`,
  };
  let text = template;
  for (const [k, v] of Object.entries(fill)) text = text.split(k).join(v);
  const fm = [
    '---',
    `draft: ${JSON.stringify(p.n)}`,
    `title: ${JSON.stringify(p.title)}`,
    'type: "Task"',
    `milestone: ${JSON.stringify(MILESTONE)}`,
    `parent: ${JSON.stringify(p.kind.parent)}`,
    `blocked_by: ${JSON.stringify(p.blocked)}`,
    'labels: ["plugin-builtin"]',
    `size: ${JSON.stringify(p.size)}`,
    'priority: "High"',
    'project: "signature-studio/3"',
    '---',
  ].join('\n');
  const file = `${p.n}-${p.kind.lc}-${String(p.index).padStart(2, '0')}.md`;
  drafts.push({ n: p.n, title: p.title, file, text: `${fm}\n\n${text.trimEnd()}\n`, body: text.trimEnd(), kind: p.kind, stage: p.stage, members: p.members });
}

if (FORBIDDEN_NUMBERS.test(template.replace(/@@[A-Z_]+@@|#\{\{\d{3}\}\}/g, ''))) errors.push('テンプレートの散文に禁止の数値が在る');
for (const d of drafts) {
  if (/#\d/.test(d.body)) errors.push(`${d.file}: 本文に #数字 の参照が在る(#{{NNN}} の形で書く)`);
  if (d.body.replace(/#\{\{\d{3}\}\}/g, '').includes('{{')) errors.push(`${d.file}: 本文に #{{NNN}} の形以外の {{ が在る`);
  if (d.body.includes('@@')) errors.push(`${d.file}: 置き換えの後に @@ が残る`);
  if (d.body.includes('L-0')) errors.push(`${d.file}: 本文に内部の ID(L-0)が在る`);
  // 禁止の数値は、散文だけを見る。表(tsv)・SHA-256・大きさ・受け入れのコマンドの中の数は、データなので除く
  const prose = d.body
    .replace(/```(tsv|accept)\n[\s\S]*?\n```/g, '')
    .replace(/#\{\{\d{3}\}\}/g, '')
    .split('\n')
    .filter((l) => !/大きさ|SHA-256|hook:/.test(l))
    .join('\n');
  if (FORBIDDEN_NUMBERS.test(prose)) errors.push(`${d.file}: 散文に禁止の数値が在る`);
}
if (new Set(drafts.map((d) => d.body)).size !== drafts.length) errors.push('本文が互いに同じ下書きが在る');

const counts = (kind) => drafts.filter((d) => d.kind.dir === kind).length;
const bodiesOf = (kind) => left.filter(([, list]) => list[0]['種別'] === kind).reduce((s, [, list]) => s + list.filter((r) => r['本体か補助か'] === '本体').length, 0);
const foldersOf = (kind) => left.filter(([, list]) => list[0]['種別'] === kind).length;
const excludedVfx = [...folders].filter(([f, list]) => isVfx(list) && !DONE.map((n) => `${PREFIX}${n}`).includes(f));
const excludedDone = [...folders].filter(([f]) => DONE.map((n) => `${PREFIX}${n}`).includes(f));
const dllsOf = (list) => list.reduce((s, [, l]) => s + l.length, 0);
const stageRange = (s) => {
  const ds = drafts.filter((d) => d.stage === s);
  return ds.length ? `${ds[0].n}〜${ds.at(-1).n}` : '-';
};
const stageCount = (s) => drafts.filter((d) => d.stage === s).length;
const plan = [
  ...(dryRun ? ['dry-run: 書いていない。'] : []),
  `DLL: Effects ${all.filter((r) => r['種別'] === 'Effects').length} 件、Generators ${all.filter((r) => r['種別'] === 'Generators').length} 件(計 ${all.length} 件)`,
  `フォルダ: Effects ${[...folders].filter(([, l]) => l[0]['種別'] === 'Effects').length}、Generators ${[...folders].filter(([, l]) => l[0]['種別'] === 'Generators').length}(計 ${folders.size})`,
  `除く: VFX 系 ${excludedVfx.length} フォルダ(DLL ${dllsOf(excludedVfx)} 件)、lift/plugins/ に在る ${excludedDone.length} フォルダ(DLL ${dllsOf(excludedDone)} 件)`,
  `残り: Effects ${foldersOf('Effects')} フォルダ(本体 ${bodiesOf('Effects')} 件)、Generators ${foldersOf('Generators')} フォルダ(本体 ${bodiesOf('Generators')} 件)。DLL ${dllsOf(left)} 件`,
  `束(${HEAD_BLOCK} フォルダずつ): Effects ${counts('Effects')} 束、Generators ${counts('Generators')} 束。計 ${drafts.length} 束。下書き番号 ${drafts[0]?.n ?? '-'}〜${drafts.at(-1)?.n ?? '-'}`,
  `段(${STAGE_SIZE} 束ずつ): 段 1 ${stageRange(1)}(${stageCount(1)} 束)、段 2 ${stageRange(2)}(${stageCount(2)} 束)、段 3 ${stageRange(3)}(${stageCount(3)} 束)`,
  `dir: ${dirs.filter((d) => d.startsWith('effects-') || d.startsWith('generators-')).join(', ') || '(種別を付けたものは無い)'}`,
  `Data(配信しない): ${left.reduce((s, [f]) => s + data(f).n, 0)} ファイル(${left.filter(([f]) => data(f).n > 0).length} フォルダ)`,
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
