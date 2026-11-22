#!/usr/bin/env node
// tools/plugin-hooks/detect.mjs [OUT.tsv] | --check : 内蔵のプラグインの DLL から、Delphi の実行時が自分の関数に書く hook 点(3 つの RVA と先頭 5 バイト)を検出する(#1344)。
// 規則は、Fruity Balance・Fruity Filter・Fruity Delay 2 の実物で見つけた形。注釈は lift/harness/hooks/fruity-delphi-effect-hook-points.txt、導入は patches/mt3w/0313/impl/0313-g9a-balance-module.diff。
// 組: lea r8,[rip+d](4C 8D 05 d32)は対象の VA、lea r9,[rip+d](4C 8D 0D d32)は置き換えの VA を置く。lea r9 は lea r8 の終わりから 16 バイト以内。
//     lea r9 の次に来る最初の call rel32(E8 d32)が構築関数を呼ぶ。call は lea r8 の先頭から 48 バイト以内。間の命令は問わない(Balance は mov byte [rsp+0x20],0 が入る)。
//     VA は PE ヘッダの ImageBase と RVA から作り、対象の RVA は VA から ImageBase を引いて出す。0x400000 は決め打ちしない。
// 候補: 組を呼び出し先(構築関数)ごとに数え、ちょうど 3 組を持つ呼び出し先。対象の RVA を昇順に並べたものが hook 1〜3。
// 確かめ: (i) 対象が .pdata の関数の先頭(RUNTIME_FUNCTION の BeginAddress)。(ii) 対象の先頭 5 バイトが 55 56 53 c6 44 / 53 48 83 ec 20 / 55 48 83 ec 50 のいずれか。
//         (iii) 構築関数から 2 段以内に kernel32!VirtualProtect の呼び出しが在る。構築関数が直接、VirtualProtect の thunk(IAT を jmp する ff 25 の stub)を呼ぶ(1 段)か、
//         構築関数が呼ぶ関数の中で呼ぶ(2 段)。Balance: 構築関数 0x5c2d70 → 0x5c2ef0 → thunk 0x41d5b0。
//         同じ規則で 3 組を持つ呼び出し先が複数あるときは、(i)(ii)(iii) を全部満たす物を採る(残りは対象が .pdata の先頭でなく、データの番地を指す)。
// 確信: 高 = 候補が 1 件あり、(i)(ii)(iii) を満たす。中 = 候補はあるが (i)(ii)(iii) のどれかが外れる、または満たす候補が複数。低 = 3 組の呼び出し先が無い(RVA と先頭 5 バイトは空)。
// 読む物: 木の中の DLL(lift/out-app/host-extra/plugins。MANIFEST.tsv の大きさと SHA-256 で照合する)と inputs/plugin-classes.tsv(本体 で hook の要否 が 要 か 既存 の行)。
// 出力: inputs/plugin-hooks.tsv(TAB 区切り。見出しは HEADER、行は plugin-classes.tsv の順)。--check は、表の形と行(a)、注釈の 3 件との一致(b)、検出し直した値との一致(c)を確かめる。
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { peLayout } from '../plugin-classes/pe.mjs';

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const CLASSES = path.join(REPO, 'inputs/plugin-classes.tsv');
const TABLE = path.join(REPO, 'inputs/plugin-hooks.tsv');
const TREE = path.join(REPO, 'lift/out-app/host-extra/plugins');
const ANNOTATION = path.join(REPO, 'lift/harness/hooks/fruity-delphi-effect-hook-points.txt');
const HEADER = ['パス', 'hook 1 RVA', 'hook 2 RVA', 'hook 3 RVA', 'hook 1 先頭 5 バイト', 'hook 2 先頭 5 バイト', 'hook 3 先頭 5 バイト', '検出の根拠', '確信', '備考'];
const REFERENCE = ['Fruity Balance', 'Fruity Filter', 'Fruity Delay 2'];
// The other Delphi plugins of the annotation (other three points): reported, not checked.
const OTHERS = [
  ['Frequency Shifter', [0x17e90, 0x320b0, 0x320d0]],
  ['Fruity Parametric EQ 2', [0x17ee0, 0x32100, 0x32120]],
  ['Soundgoodizer', [0x1b7b0, 0x389e0, 0x38a00]],
];
const PATTERNS = ['55 56 53 c6 44', '53 48 83 ec 20', '55 48 83 ec 50'];
const LEA_R8 = [0x4c, 0x8d, 0x05];
const LEA_R9 = [0x4c, 0x8d, 0x0d];
const R9_WINDOW = 16;
const CALL_WINDOW = 48;
const EXECUTE = 0x2000_0000;
const CHECKS = [['i', '(i) 対象が .pdata の先頭でない'], ['ii', '(ii) 先頭 5 バイトが 3 種類の外'], ['iii', '(iii) VirtualProtect への経路が無い']];

const sha256 = (b) => crypto.createHash('sha256').update(b).digest('hex');
const hex = (v) => `0x${v.toString(16)}`;
const bytesHex = (b) => [...b].map((x) => x.toString(16).padStart(2, '0')).join(' ');
const guestFile = (guest) => path.join(TREE, ...guest.split('\\'));
const baseName = (guest) => guest.slice(guest.lastIndexOf('\\') + 1);
const matchAt = (d, at, pat) => pat.every((b, n) => d[at + n] === b);
const firstAt = (d, pat, from, to) => {
  for (let k = from; k <= to; k++) if (matchAt(d, k, pat)) return k;
  return -1;
};

// The PE image that the scan reads: the bytes, the ImageBase of its header, the sections, the data directories, and the RVA-to-offset map of pe.mjs.
export function image(d) {
  const L = peLayout(d);
  if (!L) throw new Error('PE ではない');
  const nt = d.readUInt32LE(0x3c), opt = nt + 24;
  const is64 = d.readUInt16LE(opt) === 0x20b;
  const imageBase = is64 ? Number(d.readBigUInt64LE(opt + 24)) : d.readUInt32LE(opt + 28);
  const nSec = d.readUInt16LE(nt + 6), optSize = d.readUInt16LE(nt + 20);
  const sections = [];
  for (let i = 0; i < nSec; i++) {
    const s = opt + optSize + 40 * i;
    sections.push({ va: d.readUInt32LE(s + 12), raw: d.readUInt32LE(s + 16), ptr: d.readUInt32LE(s + 20), chars: d.readUInt32LE(s + 36) });
  }
  const dirs = opt + (is64 ? 112 : 96);
  const dir = (k) => ({ rva: d.readUInt32LE(dirs + 8 * k), size: d.readUInt32LE(dirs + 8 * k + 4) });
  return { d, off: L.off, imageBase, sections, dir };
}

export function codeSections(img) {
  return img.sections.filter((s) => (s.chars & EXECUTE) !== 0 && s.ptr && s.raw);
}

// The triplets (組) of the code sections: { at, r8, r9, callee }, all RVAs. at is the RVA of the lea r8.
export function triplets(img, secs = codeSections(img)) {
  const { d, imageBase } = img;
  const out = [];
  for (const s of secs) {
    const end = s.ptr + s.raw;
    const va = (at) => imageBase + s.va + (at - s.ptr);
    const target = (at, len) => va(at) + len + d.readInt32LE(at + len - 4) - imageBase;
    for (let i = s.ptr; i + 7 <= end; i++) {
      if (!matchAt(d, i, LEA_R8)) continue;
      const j = firstAt(d, LEA_R9, i + 7, Math.min(i + 7 + R9_WINDOW, end - 7));
      if (j < 0) continue;
      const k = firstAt(d, [0xe8], j + 7, Math.min(i + CALL_WINDOW, end - 5));
      if (k < 0) continue;
      out.push({ at: va(i) - imageBase, r8: target(i, 7), r9: target(j, 7), callee: target(k, 5) });
    }
  }
  return out;
}

// The triplets of each callee, the three of one callee sorted by their r8.
export function groupByCallee(list) {
  const g = new Map();
  for (const t of list) g.set(t.callee, [...(g.get(t.callee) ?? []), t]);
  for (const v of g.values()) v.sort((a, b) => a.r8 - b.r8);
  return g;
}

// The IAT slots (RVA) of kernel32!VirtualProtect, in the import directory (1) and the delay import directory (13).
function vpSlots(img) {
  const { d, off } = img;
  const slots = new Set();
  const cstr = (rva) => {
    const at = off(rva);
    return at < 0 ? '' : d.toString('latin1', at, d.indexOf(0, at));
  };
  for (const [k, delay] of [[1, false], [13, true]]) {
    const { rva } = img.dir(k);
    if (!rva) continue;
    for (let n = 0; n < 1000; n++) {
      const at = off(rva + (delay ? 32 : 20) * n);
      if (at < 0) break;
      const iltRva = d.readUInt32LE(at + (delay ? 16 : 0));
      const nameRva = d.readUInt32LE(at + (delay ? 4 : 12));
      const iatRva = d.readUInt32LE(at + (delay ? 12 : 16));
      if (!nameRva) break;
      if (cstr(nameRva).toLowerCase() !== 'kernel32.dll') continue;
      const thunks = iltRva || iatRva;
      for (let m = 0; m < 100000; m++) {
        const t = off(thunks + 8 * m);
        if (t < 0) break;
        const e = d.readBigUInt64LE(t);
        if (e === 0n) break;
        if (e >> 63n) continue;
        const hn = off(Number(e & 0xffffffffn));
        if (hn >= 0 && d.toString('latin1', hn + 2, d.indexOf(0, hn + 2)) === 'VirtualProtect') slots.add(iatRva + 8 * m);
      }
    }
  }
  return slots;
}

// The functions of .pdata: { begin, end } (RVAs; end is exclusive).
function bodiesOf(img) {
  const { rva, size } = img.dir(3);
  const out = [];
  for (let n = 0; rva && n < Math.floor(size / 12); n++) {
    const at = img.off(rva + 12 * n);
    if (at < 0) break;
    out.push({ begin: img.d.readUInt32LE(at), end: img.d.readUInt32LE(at + 4) });
  }
  return out;
}

// The RVAs of the call targets (e8 rel32) in a function.
function calleesOf(img, b) {
  const out = [];
  for (let r = b.begin; r + 5 <= b.end; r++) {
    const at = img.off(r);
    if (at < 0) break;
    if (at + 5 <= img.d.length && img.d[at] === 0xe8) out.push(r + 5 + img.d.readInt32LE(at + 1));
  }
  return out;
}

// The facts of the image that the checks read: the VirtualProtect stubs (ff 25) and direct calls (ff 15) of the IAT slots, and the functions of .pdata.
function facts(img) {
  const slots = vpSlots(img);
  const stubs = new Set(), calls = new Set();
  for (const s of codeSections(img)) {
    for (let i = s.ptr; i + 6 <= s.ptr + s.raw; i++) {
      if (img.d[i] !== 0xff || (img.d[i + 1] !== 0x25 && img.d[i + 1] !== 0x15)) continue;
      const rva = s.va + (i - s.ptr);
      if (!slots.has(rva + 6 + img.d.readInt32LE(i + 2))) continue;
      (img.d[i + 1] === 0x25 ? stubs : calls).add(rva);
    }
  }
  return { stubs, calls, bodies: new Map(bodiesOf(img).map((b) => [b.begin, b])) };
}

// The checks (i)(ii)(iii) of one candidate. vp: { depth, via } (depth 1: the constructor calls VirtualProtect; depth 2: through the function via), or null.
function judge(img, f, callee, sites) {
  const hooks = sites.map((t) => t.r8);
  const bytes = hooks.map((r) => {
    const at = img.off(r);
    return at < 0 || at + 5 > img.d.length ? '' : bytesHex(img.d.subarray(at, at + 5));
  });
  const callsVP = (b) => calleesOf(img, b).some((x) => f.stubs.has(x)) || [...f.calls].some((r) => r >= b.begin && r < b.end);
  let vp = null;
  const body = f.bodies.get(callee);
  if (body && callsVP(body)) vp = { depth: 1, via: null };
  else if (body) {
    const via = calleesOf(img, body).find((x) => f.bodies.has(x) && callsVP(f.bodies.get(x)));
    if (via !== undefined) vp = { depth: 2, via };
  }
  return { callee, hooks, bytes, vp, checks: { i: hooks.every((r) => f.bodies.has(r)), ii: bytes.every((b) => PATTERNS.includes(b)), iii: vp !== null } };
}

// The hook points of one DLL: { conf, hooks, bytes, basis, note }. hooks and bytes are empty for 低.
export function detect(img) {
  const f = facts(img);
  const groups = groupByCallee(triplets(img));
  const cands = [...groups].filter(([, v]) => v.length === 3).sort(([a], [b]) => a - b);
  if (cands.length === 0) {
    const most = Math.max(0, ...[...groups.values()].map((v) => v.length));
    return { conf: '低', hooks: [], bytes: [], basis: `3 組の呼び出し先が無い(同じ呼び出し先の組の最大は ${most})`, note: '対象の 3 つは手で探す。' };
  }
  const judged = cands.map(([callee, sites]) => judge(img, f, callee, sites));
  const score = (j) => Object.values(j.checks).filter(Boolean).length;
  const full = judged.filter((j) => score(j) === 3);
  const pick = full[0] ?? judged.reduce((best, j) => (score(j) > score(best) ? j : best));
  const others = judged.filter((j) => j !== pick);
  const vpText = pick.vp === null ? '無し' : pick.vp.depth === 1 ? '1 段(構築関数が直接呼ぶ)' : `2 段(経由 ${hex(pick.vp.via)})`;
  const basis = `構築関数 ${hex(pick.callee)} を 3 回呼ぶ(組 3 件)。VirtualProtect: ${vpText}`;
  const failed = CHECKS.filter(([k]) => !pick.checks[k]).map(([, t]) => t);
  if (full.length === 1) {
    const extra = others.length ? ` 同じ規則で 3 組を持つ他の呼び出し先 ${others.map((o) => hex(o.callee)).join('・')} は (i)(ii)(iii) を満たさず除いた。` : '';
    return { conf: '高', hooks: pick.hooks, bytes: pick.bytes, basis, note: `(i)(ii)(iii) を満たす。${extra}` };
  }
  const note = full.length > 1
    ? `(i)(ii)(iii) を満たす呼び出し先が ${full.length} 件あり、最も低い番地 ${hex(pick.callee)} を採った。どれが hook の構築関数かを手で見る。`
    : `外れ: ${failed.join('、')}。手で見る: 構築関数 ${hex(pick.callee)} の呼び出し先と、対象の番地の中身。`;
  return { conf: '中', hooks: pick.hooks, bytes: pick.bytes, basis, note };
}

// The lines of the annotation of the three points: { rva, bytes } in the order of the file.
function annotation(text) {
  const out = [...text.matchAll(/^rva:(0x[0-9a-f]+)\b.*?first bytes ((?:[0-9a-f]{2} ){4}[0-9a-f]{2})/gm)].map((m) => ({ rva: Number(m[1]), bytes: m[2] }));
  if (out.length !== 3) throw new Error(`注釈の hook 点が 3 件でない: ${out.length}`);
  return out;
}

// The rows of the 本体 DLLs that need a hook point check (要 or 既存), in the order of inputs/plugin-classes.tsv.
function targets() {
  const [head, ...rows] = fs.readFileSync(CLASSES, 'utf8').split('\n').filter(Boolean).map((l) => l.split('\t'));
  const col = (name) => head.indexOf(name);
  const [guestCol, roleCol, hookCol] = ['パス', '本体か補助か', 'hook の要否'].map(col);
  if ([guestCol, roleCol, hookCol].some((c) => c < 0)) throw new Error('plugin-classes.tsv: 見出しが違う');
  return rows.filter((r) => r[roleCol] === '本体' && (r[hookCol] === '要' || r[hookCol] === '既存')).map((r) => ({ guest: r[guestCol], cls: r[hookCol] }));
}

function manifest() {
  const file = path.join(TREE, 'MANIFEST.tsv');
  if (!fs.existsSync(file)) throw new Error('木が無い: lift/out-app/host-extra/plugins/MANIFEST.tsv(R2 のキャッシュから置く)');
  const [, ...rows] = fs.readFileSync(file, 'utf8').split('\n').filter(Boolean).map((l) => l.split('\t'));
  return new Map(rows.map(([guest, bytes, sha]) => [guest, { bytes, sha256: sha }]));
}

function readDll(guest, man) {
  const m = man.get(guest);
  if (!m) throw new Error(`MANIFEST に無い: ${guest}`);
  const data = fs.readFileSync(guestFile(guest));
  if (String(data.length) !== m.bytes || sha256(data) !== m.sha256) throw new Error(`MANIFEST の大きさ・SHA-256 と違う: ${guest}`);
  return data;
}

// The detections of the targets, in the order of inputs/plugin-classes.tsv.
function run() {
  const man = manifest();
  return targets().map(({ guest, cls }) => ({ guest, cls, det: detect(image(readDll(guest, man))) }));
}

const clean = (cell) => {
  if (/[\t\r\n]/.test(cell)) throw new Error(`表: 改行かタブを含む値: ${JSON.stringify(cell)}`);
  return cell;
};

function rowOf(guest, det) {
  const at = (xs, k, f) => (xs[k] === undefined ? '' : f(xs[k]));
  return [guest, ...[0, 1, 2].map((k) => at(det.hooks, k, hex)), ...[0, 1, 2].map((k) => at(det.bytes, k, (b) => b)), det.basis, det.conf, det.note].map(clean);
}

const render = (rows) => [HEADER, ...rows].map((r) => `${r.join('\t')}\n`).join('');

function check() {
  let failed = 0;
  const report = (name, pass, msg) => {
    if (!pass) failed++;
    console.log(`${pass ? 'ok  ' : 'FAIL'} ${name}: ${msg}`);
  };
  const want = targets();
  const text = fs.existsSync(TABLE) ? fs.readFileSync(TABLE, 'utf8') : '';
  const lines = text.split('\n');
  if (lines.at(-1) === '') lines.pop();
  const [head, ...body] = lines.map((l) => l.split('\t'));
  const paths = body.map((r) => r[0]);
  const wanted = new Set(want.map((t) => t.guest));
  const shape = head?.join('\t') === HEADER.join('\t') && body.every((r) => r.length === HEADER.length);
  const rows = paths.length === want.length && new Set(paths).size === paths.length && paths.every((p) => wanted.has(p));
  report('(a) 表の形と行', shape && rows, `見出し・列数 ${shape ? '合う' : '合わない'}、行 ${body.length} 件(本体 で 要 か 既存 の ${want.length} 件と ${rows ? '一致' : '一致しない'})`);

  const ann = annotation(fs.readFileSync(ANNOTATION, 'utf8'));
  for (const name of REFERENCE) {
    const row = body.find((r) => r[0] === `Plugins\\Fruity\\Effects\\${name}\\${name}_x64.dll`);
    const pass = !!row && ann.every((a, k) => row[1 + k] === hex(a.rva) && row[4 + k] === a.bytes);
    report(`(b) ${name}`, pass, row ? `RVA ${row.slice(1, 4).join(' ')} 先頭 5 バイト ${row.slice(4, 7).join(' / ')}(注釈と ${pass ? '一致' : '不一致'})` : '行が無い');
  }

  try {
    const fresh = render(run().map((r) => rowOf(r.guest, r.det)));
    const pass = fresh === text;
    const firstDiff = fresh.split('\n').findIndex((l, n) => l !== lines[n]) + 1;
    report('(c) 検出し直した値との一致', pass, pass ? `${body.length} 行が道具の出力と同じ` : `${firstDiff} 行目から違う`);
  } catch (e) {
    report('(c) 検出し直した値との一致', false, e.message);
  }
  return failed ? 1 : 0;
}

function main(argv) {
  if (argv[0] === '--check') return check();
  const [out = TABLE] = argv;
  const res = run();
  fs.mkdirSync(path.dirname(out), { recursive: true });
  fs.writeFileSync(out, render(res.map((r) => rowOf(r.guest, r.det))));
  const n = (c) => res.filter((r) => r.det.conf === c).length;
  console.log(`plugin-hooks: ${res.length} 件を書いた(要 ${res.filter((r) => r.cls === '要').length}・既存 ${res.filter((r) => r.cls === '既存').length} | 高 ${n('高')}・中 ${n('中')}・低 ${n('低')})`);
  const ann = annotation(fs.readFileSync(ANNOTATION, 'utf8'));
  const same = (r) => r.det.hooks.length === 3 && r.det.hooks.every((h, k) => h === ann[k].rva);
  const existing = res.filter((r) => r.cls === '既存');
  console.log(`既存 ${existing.length} 件: 検出の 3 つの RVA が注釈と一致 ${existing.filter(same).length} 件`);
  for (const r of existing.filter((x) => !same(x))) console.log(`  食い違い: ${baseName(r.guest)} ${r.det.hooks.map(hex).join(' ') || '(検出できず)'}`);
  for (const [name, rvas] of OTHERS) {
    const r = res.find((x) => baseName(x.guest) === `${name}_x64.dll`);
    const got = r ? r.det.hooks.map(hex).join(' ') : '(対象外)';
    const pass = !!r && r.det.hooks.length === 3 && r.det.hooks.every((h, k) => h === rvas[k]);
    console.log(`参考 ${name}: 検出 ${got} / 注釈 ${rvas.map(hex).join(' ')}(${pass ? '一致' : '不一致'})`);
  }
  for (const r of res.filter((x) => x.det.conf !== '高')) console.log(`${r.det.conf}: ${baseName(r.guest)} | ${r.det.note}`);
  return 0;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    process.exitCode = main(process.argv.slice(2));
  } catch (e) {
    console.error(`plugin-hooks: ${e.message}`);
    process.exitCode = 1;
  }
}
