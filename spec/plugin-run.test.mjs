#!/usr/bin/env node
// spec/plugin-run.test.mjs : tools/plugin-run/run.mjs (Issue #1371)。名前の解決(lift/plugins の効果のフォルダ、大文字・空白・ハイフンを区別しない)、
// 存在しない名前と効果でない名前で 1 で止まり、標準出力に何も出さないこと、読みの一致(分離 wasm の名前)、.flp の選び方を確かめる。
// (e) は Issue #1524: 場面 (c) の候補(名前の境界、ASCII の内部名も、根の順、小さい順、最大 5 本)、試行の止め方、1 本の判定(passes)、--scene の不明な値(64)。Chrome は使わない。
// (f) は場面 (d) の flp(tools/plugin-run/flp.mjs): 書き戻しの往復、名前の差し替え(長さの欄と FLdt の長さ、状態の落とし方)。テンプレートの試験は build の出力(lift/out-app)が在るときだけ行う。
// 一時ディレクトリはリポジトリの中の .tmp-*(git の無視の対象)に作り、終わったら消す。終了コード 0 = 全部通った。
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const TOOL = path.join(ROOT, 'tools/plugin-run/run.mjs');
let failed = 0;
const ok = (name, cond, detail = '') => {
  if (cond) console.log(`ok   ${name}`);
  else {
    failed++;
    console.log(`FAIL ${name}${detail ? `: ${detail}` : ''}`);
  }
};

let m;
try {
  m = await import(pathToFileURL(TOOL).href);
} catch (e) {
  console.log(`FAIL 道具が読めない: ${e.message.split('\n')[0]}`);
  process.exit(1);
}
const { keyOf, effectRows, resolveNames, projectFor, wasmReadRe, isPlayFailure, playFailuresOf, candidatesFor, passes, sceneProjectFor } = m;
const flp = await import(pathToFileURL(path.join(ROOT, 'tools/plugin-run/flp.mjs')).href);

// 1) 存在しない名前は 1 で止まる(標準出力は空。バンドルの場所は読まれないので、在るディレクトリを渡す)
const run = (args) => spawnSync(process.execPath, [TOOL, ...args], { encoding: 'utf8', timeout: 60000 });
const miss = run([ROOT, 'No Such Plugin']);
ok('存在しない名前で終了コード 1', miss.status === 1, `status ${miss.status}`);
ok('存在しない名前で標準出力が空', miss.stdout === '', JSON.stringify(miss.stdout));
ok('存在しない名前の理由が標準エラーに出る', /no effect plug-in named "No Such Plugin"/.test(miss.stderr), miss.stderr.slice(0, 200));

// 2) 効果でない名前(インストゥルメント)も 1 で止まる
const gen = run([ROOT, 'Sytrus']);
ok('インストゥルメント(Sytrus)で終了コード 1', gen.status === 1, `status ${gen.status}`);
ok('インストゥルメントで標準出力が空', gen.stdout === '', JSON.stringify(gen.stdout));

// 3) 名前が無いときは使い方の終了コード 64
const usage = run([ROOT]);
ok('名前が無いときは終了コード 64', usage.status === 64, `status ${usage.status}`);

// 4) 名前の比べ方: 大文字・空白・ハイフンを区別しない
ok('keyOf: "Fruity Balance" と "fruity-balance" は同じ', keyOf('Fruity Balance') === keyOf('fruity-balance'));
ok('keyOf: 大文字と小文字を区別しない', keyOf('FRUITY-FILTER') === keyOf('fruity filter'));

// 5) 効果の行: lift/plugins の本体(role 本体、key effects.*)の行。Balance と Filter が在り、Sytrus(インストゥルメント)は無い
const rows = effectRows();
const byName = Object.fromEntries(rows.map((r) => [r.name, r]));
ok('効果の行に Fruity Balance と Fruity Filter が在る', 'Fruity Balance' in byName && 'Fruity Filter' in byName, JSON.stringify(rows));
ok('効果の行の stem は DLL の key から .dll を除いたもの', byName['Fruity Balance']?.stem === 'effects.fruity-balance.fruity_balance_x64', JSON.stringify(byName['Fruity Balance']));
ok('効果の行に、インストゥルメントは無い', rows.every((r) => r.key.startsWith('effects.')));

// 6) 名前の解決
const got = resolveNames(['Fruity Balance', 'fruity-filter']);
ok('resolveNames: 2 つの名前が 2 行になる', got.length === 2 && got[0].folder === 'fruity-balance' && got[1].folder === 'fruity-filter', JSON.stringify(got));
let threw = false;
try { resolveNames(['No Such Plugin']); } catch { threw = true; }
ok('resolveNames: 存在しない名前は投げる', threw);

// 7) 分離 wasm の読み(image-read の行の名前)は、本体の stem の後に任意の基底と .wasm
const re = wasmReadRe('effects.fruity-balance.fruity_balance_x64');
ok('wasmReadRe: 基底つきの wasm の名前に一致する', re.test('effects.fruity-balance.fruity_balance_x64.c1d0000.wasm'));
ok('wasmReadRe: DLL の名前には一致しない', !re.test('effects.fruity-balance.fruity_balance_x64.c1d0000.dll'));
ok('wasmReadRe: 別の本体には一致しない', !re.test('effects.fruity-filter.fruity_filter_x64.c580000.wasm'));

// 8) .flp の選び方: UTF-16LE の名前を含む物。根の順(host-extra が先)。同じ根では最も小さい物
const tmp = fs.mkdtempSync(path.join(ROOT, '.tmp-spec-plugin-run.'));
try {
  const u16 = (s) => Buffer.from(s, 'utf16le');
  const a = path.join(tmp, 'a'), b = path.join(tmp, 'b'), c = path.join(tmp, 'c');
  fs.mkdirSync(path.join(a, 'sub'), { recursive: true });
  fs.mkdirSync(b, { recursive: true });
  fs.mkdirSync(c, { recursive: true });
  fs.writeFileSync(path.join(a, 'sub', 'big.flp'), Buffer.concat([Buffer.alloc(4000), u16('Fruity Test')]));
  fs.writeFileSync(path.join(a, 'small.flp'), Buffer.concat([Buffer.alloc(100), u16('Fruity Test')]));
  fs.writeFileSync(path.join(a, 'ascii-only.flp'), Buffer.from('Fruity Test in ASCII only'));
  fs.writeFileSync(path.join(b, 'other.flp'), Buffer.concat([Buffer.alloc(10), u16('Fruity Test')]));
  fs.writeFileSync(path.join(c, 'none.flp'), Buffer.alloc(10));
  const pick = projectFor('Fruity Test', [a, b]);
  ok('projectFor: 同じ根では、UTF-16LE の名前を含む最も小さい .flp', pick === path.join(a, 'small.flp'), String(pick));
  ok('projectFor: 先の根に候補が在れば、後の根は見ない', projectFor('Fruity Test', [b, a]) === path.join(b, 'other.flp'), String(projectFor('Fruity Test', [b, a])));
  ok('projectFor: ASCII だけの名前は選ばない', projectFor('Fruity Test', [c]) === null);
  ok('projectFor: 名前の無い根は null', projectFor('Nothing Here', [a, b, c]) === null);
  ok('projectFor: 存在しない根は飛ばす', projectFor('Fruity Test', [path.join(tmp, 'missing'), a]) === path.join(a, 'small.flp'));
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}

// 9) 再生の判定(Issue #1371): 失敗の行は、スレッドの失敗・サスペンドの誤り・エンジンの停止。それ以外の行は失敗ではない
ok('isPlayFailure: fle thread の失敗の行', isPlayFailure('[err] [js] fle thread 23 failed: SuspendError: trying to suspend JS frames'));
ok('isPlayFailure: 停止の行', isPlayFailure('fruity: runtime message: fle: RaiseException: exception code 0xeedfade, flags 0x1, 8 argument(s) raised; no structured exception handling is modelled, stopped here'));
ok('isPlayFailure: メモリの行は失敗ではない', !isPlayFailure('[err] memory-usage: K32GetProcessMemoryInfo WorkingSetSize 198091964'));
ok('playFailuresOf: 再生が増え、失敗の行が無く、engine-idle なら空', playFailuresOf({ lines: ['[err] missing-path x'], state: 'engine-idle', quanta: 400 }).length === 0);
const fl = playFailuresOf({ lines: ['[err] [js] fle thread 23 failed: SuspendError: x', 'ok line'], state: 'engine-idle', quanta: 400 });
ok('playFailuresOf: 失敗の行だけを入れる', fl.length === 1 && fl[0].includes('SuspendError'), JSON.stringify(fl));
ok('playFailuresOf: quantaWhilePlaying が増えないと失敗', playFailuresOf({ lines: [], state: 'engine-idle', quanta: 0 }).length === 1);
ok('playFailuresOf: quantaWhilePlaying が測れないと失敗', playFailuresOf({ lines: [], state: 'engine-idle', quanta: null }).length === 1);
const ex = playFailuresOf({ lines: [], state: 'exited', stateNote: ' (x [y])', quanta: 400 });
ok('playFailuresOf: 再生の後に exited なら、状態の行が入る', ex.length === 1 && ex[0] === 'state exited after the play (x [y])', JSON.stringify(ex));

// (e) 場面 (c) の選び方と、1 本の判定(Issue #1524)。一時ディレクトリは .tmp-* の中に作る
const called = (f, ...a) => (typeof f === 'function' ? f(...a) : undefined);
const tmpE = fs.mkdtempSync(path.join(ROOT, '.tmp-spec-plugin-run.'));
try {
  const u16 = (s) => Buffer.from(s, 'utf16le');
  const pad = (n) => Buffer.alloc(n);
  const r1 = path.join(tmpE, 'r1'), r2 = path.join(tmpE, 'r2');
  fs.mkdirSync(r1, { recursive: true });
  fs.mkdirSync(r2, { recursive: true });
  fs.writeFileSync(path.join(r1, 'bank.flp'), u16('Fruity Delay Bank'));
  fs.writeFileSync(path.join(r1, 'two.flp'), u16('Fruity Delay 2'));
  fs.writeFileSync(path.join(r1, 'letter.flp'), u16('XFruity Delay'));
  fs.writeFileSync(path.join(r1, 'plain-big.flp'), Buffer.concat([pad(3000), u16('Fruity Delay')]));
  fs.writeFileSync(path.join(r1, 'plain-small.flp'), Buffer.concat([pad(100), u16('Fruity Delay')]));
  fs.writeFileSync(path.join(r1, 'dash.flp'), Buffer.concat([pad(500), u16('Fruity Delay - x')]));
  fs.writeFileSync(path.join(r1, 'nonascii.flp'), Buffer.concat([pad(20), u16('\u1ec9Fruity Delay')]));
  fs.writeFileSync(path.join(r2, 'plain.flp'), Buffer.concat([pad(10), u16('Fruity Delay')]));
  fs.writeFileSync(path.join(r2, 'plain3.flp'), Buffer.concat([pad(60), u16('Fruity Delay')]));
  fs.writeFileSync(path.join(r2, 'bank2.flp'), u16('Fruity Delay Bank'));
  const names = (list) => (list ?? []).map((f) => path.basename(f));
  const cand = called(candidatesFor, 'Fruity Delay', [r1, r2], 5);
  ok('candidatesFor: 部分一致の Bank・2 を除き、根の順・同じ根では小さい順、最大 5 本',
    JSON.stringify(names(cand)) === JSON.stringify(['nonascii.flp', 'plain-small.flp', 'dash.flp', 'plain-big.flp', 'plain.flp']), JSON.stringify(names(cand)));
  const cand2 = called(candidatesFor, 'Fruity Delay', [r2, r1], 5);
  ok('candidatesFor: 根の順を変えると、先の根の候補が先に来る',
    JSON.stringify(names(cand2)) === JSON.stringify(['plain.flp', 'plain3.flp', 'nonascii.flp', 'plain-small.flp', 'dash.flp']), JSON.stringify(names(cand2)));
  const cand3 = called(candidatesFor, 'Fruity Delay', [r1, r2]);
  ok('candidatesFor: 上限を省くと 6 本(最大 5 本は既定の上限ではない)', (cand3 ?? []).length === 6, String((cand3 ?? []).length));
  ok('candidatesFor: 名前が無い根は空', JSON.stringify(called(candidatesFor, 'Fruity Nothing', [r1, r2], 5)) === '[]');
  const has = (f) => Array.isArray(cand3) && cand3.length > 0 && !names(cand3).includes(f);
  ok('candidatesFor: 名前の直後が空白 1 字で、その次が英数字の物は除く', has('bank.flp') && has('bank2.flp') && has('two.flp'));
  ok('candidatesFor: 名前の直前が英数字の物は除く', has('letter.flp'));
  ok('candidatesFor: 名前の直後が空白 1 字で、その次が記号の物は残す', Array.isArray(cand3) && names(cand3).includes('dash.flp'));
  ok('projectFor: 候補の先頭を返す(既存の試験 8 と同じ規則)', called(projectFor, 'Fruity Delay', [r1, r2]) === path.join(r1, 'nonascii.flp'), String(called(projectFor, 'Fruity Delay', [r1, r2])));
  const r3a = path.join(tmpE, 'r3a', 'sub'), r3b = path.join(tmpE, 'r3b', 'sub');
  fs.mkdirSync(r3a, { recursive: true });
  fs.mkdirSync(r3b, { recursive: true });
  fs.writeFileSync(path.join(r3a, 'same.flp'), Buffer.concat([pad(40), u16('Fruity Delay')]));
  fs.writeFileSync(path.join(r3b, 'same.flp'), Buffer.concat([pad(40), u16('Fruity Delay')]));
  const same = called(candidatesFor, 'Fruity Delay', [path.join(tmpE, 'r3a'), path.join(tmpE, 'r3b')], 5);
  ok('candidatesFor: 同じ相対パスの曲は、先の根の 1 本だけ', JSON.stringify(same) === JSON.stringify([path.join(r3a, 'same.flp')]), JSON.stringify(same));
  const r4 = path.join(tmpE, 'r4');
  fs.mkdirSync(r4, { recursive: true });
  fs.writeFileSync(path.join(r4, 'ascii-ok.flp'), Buffer.concat([pad(30), Buffer.from('Fruity Delay\0', 'latin1')]));
  fs.writeFileSync(path.join(r4, 'ascii-bank.flp'), Buffer.from('Fruity Delay Bank\0', 'latin1'));
  const asc = called(candidatesFor, 'Fruity Delay', [r4], 5);
  ok('candidatesFor: ASCII の内部名も、境界が在れば候補(Bank は除く)', JSON.stringify(names(asc)) === JSON.stringify(['ascii-ok.flp']), JSON.stringify(names(asc)));
} finally {
  fs.rmSync(tmpE, { recursive: true, force: true });
}

// the loop of scene (c): the attempts stop at the first that asks (no Chrome: the attempt is a stub)
{
  const tried = [];
  const hit = (await called(m.firstAsking, ['a', 'b', 'c'], async (p) => { tried.push(p); return { instantiated: p === 'b', error: p === 'b' ? null : 'x' }; })) ?? {};
  ok('firstAsking: 要求した最初の曲で止まる(後の曲は試さない)', hit.index === 1 && tried.join(',') === 'a,b', JSON.stringify({ index: hit.index, tried }));
  tried.length = 0;
  const none = (await called(m.firstAsking, ['a', 'b'], async (p) => { tried.push(p); return { instantiated: false, error: `no ${p}` }; })) ?? {};
  ok('firstAsking: 誰も要求しなければ、全部試して -1 と最後の行を返す', none.index === -1 && tried.join(',') === 'a,b' && none.last?.error === 'no b', JSON.stringify({ index: none.index, tried }));
  const empty = (await called(m.firstAsking, [], async () => ({ instantiated: true }))) ?? {};
  ok('firstAsking: 候補が 0 本なら試さず、最後の行は null', empty.index === -1 && empty.last === null);
}

const good = { sceneKind: 'c', instantiated: true, processed: true, error: null };
ok('passes: 場面 (c) で instantiated と processed が true、error が null なら通る', called(passes, good) === true);
ok('passes: 場面 (d) で、同じ条件なら通る', called(passes, { ...good, sceneKind: 'd' }) === true);
ok('passes: instantiated が false なら落ちる', called(passes, { ...good, instantiated: false }) === false);
ok('passes: processed が false なら落ちる', called(passes, { ...good, processed: false }) === false);
ok('passes: error が文字列なら落ちる', called(passes, { ...good, error: 'x' }) === false);
ok('passes: sceneKind が null なら落ちる', called(passes, { ...good, sceneKind: null }) === false);

const scene = run([ROOT, 'Fruity Balance', '--scene', 'x']);
ok('--scene の不明な値は終了コード 64', scene.status === 64, `status ${scene.status}`);
ok('--scene の不明な値で標準出力が空', scene.stdout === '', JSON.stringify(scene.stdout));

// (f) 場面 (d) の flp (Issue #1524): 書き戻しは同じバイト列、名前の差し替えは長さの欄と FLdt の長さを整え、状態(212)だけを落とす
const listFlp = (dir) => fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? listFlp(path.join(dir, e.name)) : /\.flp$/i.test(e.name) ? [path.join(dir, e.name)] : []));
const roundTrips = (b) => {
  try {
    return flp.encodeFlp(flp.parseFlp(b)).equals(b);
  } catch {
    return false;
  }
};
const tmpF = fs.mkdtempSync(path.join(ROOT, '.tmp-spec-plugin-run.'));
try {
  const u16 = (s) => Buffer.from(`${s}\0`, 'utf16le');
  const varint = (n) => {
    const out = [];
    let v = n;
    do {
      let b = v % 128;
      v = Math.floor(v / 128);
      if (v > 0) b |= 0x80;
      out.push(b);
    } while (v > 0);
    return Buffer.from(out);
  };
  const event = (id, data) => (id >= 192 ? Buffer.concat([Buffer.from([id]), varint(data.length), data]) : Buffer.concat([Buffer.from([id]), data]));
  const hdr = Buffer.concat([Buffer.from('FLhd', 'latin1'), Buffer.from([6, 0, 0, 0, 0, 0, 1, 0, 96, 0])]);
  const synth = (events, tail) => {
    const body = Buffer.concat(events);
    const dt = Buffer.alloc(8);
    dt.write('FLdt', 0, 'latin1');
    dt.writeUInt32LE(body.length, 4);
    return Buffer.concat([hdr, dt, body, tail]);
  };
  const events = [
    event(0, Buffer.from([1])),
    event(64, Buffer.from([2, 0])),
    event(128, Buffer.from([3, 0, 0, 0])),
    event(200, Buffer.alloc(200, 7)),
    event(210, Buffer.alloc(20000, 8)),
    event(204, u16('Delay')),
    event(236, Buffer.alloc(12)),
    event(201, u16('Fruity Delay Bank')),
    event(212, Buffer.alloc(52, 9)),
    event(213, Buffer.alloc(792, 1)),
    event(155, Buffer.alloc(4)),
  ];
  const tail = Buffer.from('TAIL', 'latin1');
  const bytes = synth(events, tail);
  const p0 = flp.parseFlp(bytes);
  ok('flp: 合成の曲の書き戻しは同じバイト列(1・2・4 バイトの ID、2・3 バイトの長さの欄、tail を含む)', p0.events.length === events.length && flp.encodeFlp(p0).equals(bytes));
  ok('flp: 挿入のプラグインは 1 つ(204 → 236 → 201 → 212 の並び)', JSON.stringify(flp.insertPluginsOf(p0.events)) === JSON.stringify([7]), JSON.stringify(flp.insertPluginsOf(p0.events)));
  const longName = `Fruity Balance ${'x'.repeat(130)}`;
  const renamed = flp.renamePlugin(p0, longName);
  const out = flp.encodeFlp(renamed);
  const back = flp.parseFlp(out);
  const named = back.events.filter((e) => e.id === flp.EV_PLUGIN_NAME).map((e) => flp.nameOf(e.data));
  ok('flp: 名前を差し替えると、名前の欄は新しい名前(長さの欄が 2 バイトになる場合も)', named.length === 1 && named[0] === longName, JSON.stringify(named.map((s) => s?.length ?? null)));
  const dtAt = 8 + out.readUInt32LE(4);
  ok('flp: FLdt の長さの欄は events の長さと合い、tail は元のまま', out.readUInt32LE(dtAt + 4) === out.length - (dtAt + 8) - tail.length && out.subarray(out.length - tail.length).equals(tail));
  ok('flp: 差し替えたときは状態(212)だけがゼロになり(長さは同じ)、他の events は残る',
    JSON.stringify(back.events.map((e) => e.id)) === JSON.stringify(events.map((e) => e[0])) && back.events[8].data.length === 52 && back.events[8].data.every((b) => b === 0) && p0.events.every((e, i) => i === 7 || i === 8 || back.events[i].data.equals(e.data)),
    JSON.stringify(back.events.map((e) => e.id)));
  ok('flp: 名前の無い events(ASCII の 7 バイト)は名前として読まない', flp.nameOf(Buffer.from('3x Osc\0', 'latin1')) === null && flp.nameOf(u16('Fruity Delay Bank')) === 'Fruity Delay Bank');

  const outDir = path.join(ROOT, 'lift', 'out-app');
  const troots = [path.join(outDir, 'host-extra', 'resources'), path.join(outDir, 'host', 'resources')];
  const tfiles = troots.flatMap((r) => (fs.existsSync(path.join(r, 'Data', 'Templates')) ? listFlp(path.join(r, 'Data', 'Templates')) : []));
  if (tfiles.length === 0) {
    console.log('ok   テンプレートの試験: build の出力(lift/out-app)が無いので省く(build の後に走らせると、テンプレートで確かめる)');
  } else {
    const same = tfiles.filter((f) => roundTrips(fs.readFileSync(f)));
    ok(`テンプレートの往復: ${tfiles.length} 本の flp が、書き戻して同じバイト列`, same.length === tfiles.length, `${same.length}/${tfiles.length}`);
    const made = sceneProjectFor('Fruity Balance', troots, tmpF);
    ok('sceneProjectFor: テンプレートから場面 (d) の曲を 1 本作る', made !== null && fs.existsSync(made.project), String(made?.project));
    if (made) {
      const tp = flp.parseFlp(fs.readFileSync(made.template));
      const mp = flp.parseFlp(fs.readFileSync(made.project));
      const at = flp.insertPluginsOf(tp.events)[0];
      const names2 = mp.events.filter((e) => e.id === flp.EV_PLUGIN_NAME).map((e) => flp.nameOf(e.data));
      ok('sceneProjectFor: 作った曲の挿入の名前は Fruity Balance', names2.includes('Fruity Balance'), JSON.stringify(names2));
      ok('sceneProjectFor: 差し替えたのは名前と状態(ゼロ)だけ(events の ID と長さは同じ)', JSON.stringify(mp.events.map((e) => e.id)) === JSON.stringify(tp.events.map((e) => e.id)) && mp.events[at + 1].data.every((b) => b === 0) && tp.events.every((e, i) => i === at || i === at + 1 || mp.events[i].data.equals(e.data)));
      const singles = tfiles.filter((f) => flp.insertPluginsOf(flp.parseFlp(fs.readFileSync(f)).events).length === 1);
      ok('sceneProjectFor: 挿入のプラグインが 1 つのテンプレートが在れば、それを選ぶ', singles.length === 0 || singles.includes(made.template), made.template);
      ok('sceneProjectFor: 作った曲は、書き戻して同じバイト列になる(整合している)', roundTrips(fs.readFileSync(made.project)));
    }
  }
} finally {
  fs.rmSync(tmpF, { recursive: true, force: true });
}

console.log(failed === 0 ? 'ok   spec/plugin-run.test.mjs: 全部通った' : `FAIL spec/plugin-run.test.mjs: ${failed} 件`);
process.exit(failed === 0 ? 0 : 1);
