#!/usr/bin/env node
// tools/plugin-classes/plugin-classes.mjs [OUT.tsv] : 内蔵のプラグインの DLL(Plugins\Fruity\Effects\ と Plugins\Fruity\Generators\ の下の kind が dll の行)を、木の中の実物で読み、分類の表を書く(#1343)。
// 読む物: inputs/fl-installer-files.tsv(対象の一覧と import)、lift/out-app/host-extra/plugins/(実物と MANIFEST.tsv)、lift/harness/hooks/fruity-delphi-effect-hook-points.txt(hook 点の RVA)。inputs/vfx-deferred.tsv は照合だけに使う。
// 本体か補助か: 入口は export の CreatePlugInstance。入口を持たない DLL は補助。入口を持つ DLL は、同じ組(Plugins\Fruity\<種別>\<名前>\ の下)の本体(入口と __dbk_fcall_wrapper を持つ)が名前で参照すれば補助(その本体のエンジン)、そうでなければ本体。
// hook の要否: Delphi 系の印(コンパイラの版の文字列)を持つ DLL は、hook 点の RVA の先頭 5 バイトが Fruity Balance と一致すれば既存、一致しなければ要。印が無ければ不要。
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { peImports } from '../fl-installer-files.mjs';
import { bytesAtRva, exportNames } from './pe.mjs';

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const INVENTORY = path.join(REPO, 'inputs/fl-installer-files.tsv');
const DEFERRED = path.join(REPO, 'inputs/vfx-deferred.tsv');
const TREE = path.join(REPO, 'lift/out-app/host-extra/plugins');
const HOOKS = path.join(REPO, 'lift/harness/hooks/fruity-delphi-effect-hook-points.txt');
const OUT = path.join(REPO, 'inputs/plugin-classes.tsv');
const KNOWN_HOOKED = ['Fruity Balance', 'Fruity Filter', 'Fruity Delay 2'].map((n) => `Plugins\\Fruity\\Effects\\${n}\\${n}_x64.dll`);
const REFERENCE = KNOWN_HOOKED[0];
const ENTRY = 'CreatePlugInstance';
const BODY_EXPORT = '__dbk_fcall_wrapper';
const DELPHI_MARK = /Delphi for Win(?:32|64) compiler version/;
const OPENGL = 'opengl32.dll';
const KINDS = ['Effects', 'Generators'];
const HEADER = ['パス', '大きさ', 'SHA-256', '種別', '本体か補助か', 'export 名', 'import した DLL', 'hook の要否', 'VFX', '備考'];
const sha256 = (data) => crypto.createHash('sha256').update(data).digest('hex');
const hex = (b) => [...b].map((x) => x.toString(16).padStart(2, '0')).join(' ');
const baseOf = (guest) => guest.slice(guest.lastIndexOf('\\') + 1);
const groupOf = (guest) => guest.split('\\').slice(0, 4).join('\\');

function kindOf(guest) {
  return KINDS.find((k) => guest.startsWith(`Plugins\\Fruity\\${k}\\`)) ?? null;
}

// The dll rows of the two plugin folders, from inputs/fl-installer-files.tsv (its # line is the note, the next line is the header).
function targets(text) {
  const [head, ...rows] = text.split('\n').filter((l) => l && !l.startsWith('#')).map((l) => l.split('\t'));
  if (head.join('\t') !== 'guest\tbytes\tsha256\tkind\timports') throw new Error(`fl-installer-files.tsv: 見出しが違う: ${head.join('\t')}`);
  return rows
    .map(([guest, bytes, sha, kind, imports = '']) => ({ guest, bytes, sha256: sha, kind, imports }))
    .filter((r) => r.kind === 'dll' && kindOf(r.guest));
}

function manifest(text) {
  const [head, ...rows] = text.split('\n').filter(Boolean).map((l) => l.split('\t'));
  if (head.join('\t') !== 'guest\tbytes\tsha256') throw new Error(`MANIFEST.tsv: 見出しが違う: ${head.join('\t')}`);
  return new Map(rows.map(([guest, bytes, sha]) => [guest, { bytes, sha256: sha }]));
}

// The RVAs of the hook points: the "rva:0x..." lines of the file.
function hookRvas(text) {
  return [...text.matchAll(/^rva:(0x[0-9a-f]+)/gim)].map((m) => Number(m[1]));
}

function readTree(guest) {
  return fs.readFileSync(path.join(TREE, guest.replaceAll('\\', '/')));
}

function clean(cell) {
  if (/[\t\r\n]/.test(cell)) throw new Error(`表: 改行かタブを含む値: ${JSON.stringify(cell)}`);
  return cell;
}

function deferredGuests(text) {
  return new Set(text.split('\n').slice(1).filter(Boolean).map((l) => l.split('\t')[0]));
}

function main(argv) {
  const [out = OUT] = argv;
  const files = targets(fs.readFileSync(INVENTORY, 'utf8'));
  const man = manifest(fs.readFileSync(path.join(TREE, 'MANIFEST.tsv'), 'utf8'));
  const rvas = hookRvas(fs.readFileSync(HOOKS, 'utf8'));
  const groups = new Map();
  for (const f of files) {
    const g = groupOf(f.guest);
    if (!groups.has(g)) groups.set(g, []);
    groups.get(g).push(f);
  }

  const refs = new Map();
  const named = new Map();
  const info = files.map((f) => {
    const data = readTree(f.guest);
    const size = String(data.length);
    const sha = sha256(data);
    const imports = peImports(data);
    const bad = [];
    const m = man.get(f.guest);
    if (!m) bad.push('MANIFEST に無い');
    else if (m.sha256 !== sha || m.bytes !== size) bad.push('MANIFEST の大きさ・SHA-256 と違う');
    if (f.bytes !== size || f.sha256 !== sha) bad.push('inputs の表の大きさ・SHA-256 と違う');
    if (imports.join(';') !== f.imports) bad.push('import が inputs の表と違う');
    if (bad.length) throw new Error(`${f.guest}: ${bad.join(', ')}`);

    const exports = exportNames(data);
    const isBody = exports.includes(ENTRY) && exports.includes(BODY_EXPORT);
    if (isBody) {
      const lower = data.toString('latin1').toLowerCase();
      const wide = data.toString('utf16le').toLowerCase();
      const hit = groups.get(groupOf(f.guest)).filter((o) => {
        const name = baseOf(o.guest).toLowerCase();
        return o.guest !== f.guest && (lower.includes(name) || wide.includes(name));
      });
      refs.set(f.guest, hit.map((o) => o.guest));
      for (const o of hit) {
        if (!named.has(o.guest)) named.set(o.guest, []);
        named.get(o.guest).push(f.guest);
      }
    }
    const hookBytes = rvas.map((rva) => {
      const b = bytesAtRva(data, rva, 5);
      return b ? hex(b) : '';
    });
    return { f, size, sha256: sha, imports, exports, isBody, hookBytes, delphi: DELPHI_MARK.test(data.toString('latin1')) };
  });

  const refBytes = info.find((i) => i.f.guest === REFERENCE).hookBytes;
  const items = info.map((i) => {
    const bodies = named.get(i.f.guest) ?? [];
    const role = !i.exports.includes(ENTRY) || bodies.length > 0 ? '補助' : '本体';
    const hook = !i.delphi ? '不要' : i.hookBytes.every((b, k) => b !== '' && b === refBytes[k]) ? '既存' : '要';
    const notes = [];
    if (role === '本体') {
      notes.push(i.isBody ? `${ENTRY} と ${BODY_EXPORT} を export する Delphi 系の本体` : `${ENTRY} を export し、同じ組の本体が名前で参照しない`);
      const used = (refs.get(i.f.guest) ?? []).map(baseOf);
      if (used.length) notes.push(`名前で参照する同じ組の DLL: ${used.join(', ')}`);
    } else if (i.exports.includes(ENTRY)) {
      notes.push(`${ENTRY} を export するが、同じ組の本体 ${bodies.map(baseOf).join(', ')} が名前で参照するので、その本体のエンジン`);
    } else {
      notes.push(`${ENTRY} を export しない(入口が無い)`);
    }
    notes.push({
      既存: 'hook 点 3 点の先頭 5 バイトが Fruity Balance と一致(既存)',
      要: 'Delphi 系の印があるが、hook 点 3 点の先頭 5 バイトが一致しない(要)',
      不要: 'Delphi 系の印が無い(不要)',
    }[hook]);
    return {
      guest: i.f.guest,
      size: i.size,
      sha256: i.sha256,
      kind: kindOf(i.f.guest),
      role,
      hook,
      exports: i.exports,
      imports: i.imports,
      vfx: i.imports.includes(OPENGL) ? 'yes' : 'no',
      note: notes.join('。') + '。',
    };
  });

  for (const guest of KNOWN_HOOKED) {
    const it = items.find((x) => x.guest === guest);
    if (!it || it.hook !== '既存') throw new Error(`既存の hook 点の DLL が既存にならない: ${guest}`);
  }

  const rows = items.map((it) => [
    it.guest,
    it.size,
    it.sha256,
    it.kind,
    it.role,
    it.exports.join(';'),
    it.imports.join(';'),
    it.hook,
    it.vfx,
    it.note,
  ].map(clean));
  fs.mkdirSync(path.dirname(out), { recursive: true });
  fs.writeFileSync(out, [HEADER, ...rows].map((r) => `${r.join('\t')}\n`).join(''));

  console.log(`plugin-classes: ${items.length} 件を書いた(Effects ${items.filter((x) => x.kind === 'Effects').length}・Generators ${items.filter((x) => x.kind === 'Generators').length})`);
  for (const k of KINDS) {
    const of = items.filter((x) => x.kind === k);
    const c = (f) => of.filter(f).length;
    console.log(`${k}: 本体 ${c((x) => x.role === '本体')} 補助 ${c((x) => x.role === '補助')} | hook 既存 ${c((x) => x.hook === '既存')} 要 ${c((x) => x.hook === '要')} 不要 ${c((x) => x.hook === '不要')} | VFX yes ${c((x) => x.vfx === 'yes')}`);
    console.log(`  hook 要: ${of.filter((x) => x.hook === '要').map((x) => baseOf(x.guest)).join(', ') || '-'}`);
    console.log(`  VFX yes: ${of.filter((x) => x.vfx === 'yes').map((x) => baseOf(x.guest)).join(', ') || '-'}`);
  }
  const yes = new Set(items.filter((x) => x.vfx === 'yes').map((x) => x.guest));
  const deferred = deferredGuests(fs.readFileSync(DEFERRED, 'utf8'));
  const onlyDeferred = [...deferred].filter((g) => !yes.has(g));
  const onlyYes = [...yes].filter((g) => !deferred.has(g));
  console.log(`vfx-deferred ${deferred.size} 件 / VFX yes ${yes.size} 件: 一致 ${[...deferred].filter((g) => yes.has(g)).length} 件`);
  console.log(`  deferred にだけ在る: ${onlyDeferred.map(baseOf).join(', ') || '-'}`);
  console.log(`  VFX yes にだけ在る: ${onlyYes.map(baseOf).join(', ') || '-'}`);
  return 0;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    process.exitCode = main(process.argv.slice(2));
  } catch (e) {
    console.error(`plugin-classes: ${e.message}`);
    process.exitCode = 1;
  }
}
