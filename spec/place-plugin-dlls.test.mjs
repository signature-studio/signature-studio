#!/usr/bin/env node
// spec/place-plugin-dlls.test.mjs : tools/place-plugin-dlls.mjs(作業の木に無い lift/plugins の DLL と Data を、内蔵のプラグインの取り出し(lift/out-app/host-extra/plugins)から複写する)を、一時の木で確かめる。
// 一時の木はリポジトリの中の .tmp-spec-pd-*(git の無視の対象)に作り、終わったら消す。終了コード 0 = 全部通った。
import crypto from 'node:crypto';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const TOOL = path.join(ROOT, 'tools/place-plugin-dlls.mjs');
const mod = await import(pathToFileURL(TOOL).href).catch((e) => {
  console.log(`FAIL import: ${e.message.split('\n')[0]}`);
  return null;
});
let failed = mod ? 0 : 1;
const ok = (name, got, want) => {
  if (JSON.stringify(got) === JSON.stringify(want)) console.log(`ok   ${name}`);
  else {
    failed++;
    console.log(`FAIL ${name}: expected ${JSON.stringify(want)}, got ${JSON.stringify(got)}`);
  }
};

const digest = (b) => crypto.createHash('sha256').update(b).digest('hex');
const same = (p, b) => fs.existsSync(p) && fs.readFileSync(p).equals(b);
const HEADER = ['key', 'guest', 'role', 'sha256', 'size', 'hook', 'args', 'omit'].join('\t');
const DATA_HEADER = ['guest', 'installer', 'sha256', 'size'].join('\t');
const OUT = (placed, present, missing, mismatch) => JSON.stringify({ placed, present, missing, mismatch });
const ALPHA = Buffer.from('alpha box dll bytes\n');
const ALPHA_KEY = 'effects.alpha.a.dll';
const ALPHA_GUEST = 'Plugins\\Fruity\\Effects\\Alpha Box\\Alpha Box_x64.dll';
const ALPHA_SRC = 'Plugins/Fruity/Effects/Alpha Box/Alpha Box_x64.dll';
const ALPHA_SEED = 'lift/out-idle1/inputs/install/Plugins/Fruity/Effects/effects.alpha.a.dll';
const alphaTsv = () => `${HEADER}\n${ALPHA_KEY}\t${ALPHA_GUEST}\t本体\t${digest(ALPHA)}\t${ALPHA.length}\tnone\t\t-\n`;
const BLUR = Buffer.from('blur map bytes\n');
const BLUR_GUEST = 'Plugins\\Fruity\\Effects\\Edison\\Data\\Maps\\Blur - alt.fnv';
const BLUR_SRC = 'Plugins/Fruity/Effects/Edison/Data/Maps/Blur - alt.fnv';
const BLUR_HOST = 'plugins/Plugins/Fruity/Effects/Edison/Data/Maps/Blur_-_alt.fnv';
// the installer's own row of this file (inputs/fl-installer-files.tsv): the name, the SHA-256 and the size it must have
const BLUR_ROW = `${BLUR_GUEST}\t${BLUR_GUEST}\t5840b4347a0fdbc0dfe37d2fc372a2d181ea00b1fed6c9073268b729ca225030\t152\n`;

const tmp = fs.mkdtempSync(path.join(ROOT, '.tmp-spec-pd-'));
let seq = 0;
// 一時の木。plugins: フォルダ名 → plugin.tsv の中身、data: フォルダ名 → data.tsv の中身、extra: host-extra/plugins の下の相対のパス → 中身(null なら host-extra/plugins を作らない)
function repo({ plugins = {}, data = {}, extra = null } = {}) {
  const r = path.join(tmp, `r${seq++}`);
  fs.mkdirSync(path.join(r, 'ci'), { recursive: true });
  fs.copyFileSync(path.join(ROOT, 'ci/plugin-builtin-prefixes.txt'), path.join(r, 'ci/plugin-builtin-prefixes.txt'));
  for (const [dir, text] of Object.entries(plugins)) {
    fs.mkdirSync(path.join(r, 'lift/plugins', dir), { recursive: true });
    fs.writeFileSync(path.join(r, 'lift/plugins', dir, 'plugin.tsv'), text);
  }
  for (const [dir, text] of Object.entries(data)) fs.writeFileSync(path.join(r, 'lift/plugins', dir, 'data.tsv'), text);
  if (extra) {
    fs.mkdirSync(path.join(r, 'lift/out-app/host-extra/plugins'), { recursive: true });
    for (const [rel, body] of Object.entries(extra)) {
      const p = path.join(r, 'lift/out-app/host-extra/plugins', rel);
      fs.mkdirSync(path.dirname(p), { recursive: true });
      fs.writeFileSync(p, body);
    }
  }
  return r;
}
const host = (r, rel) => path.join(r, 'lift/out-app/host', rel);
const run = (r) => {
  const res = spawnSync(process.execPath, [TOOL, '--root', r], { encoding: 'utf8' });
  return { code: res.status, out: res.stdout.trim(), err: res.stderr };
};

try {
  // (a) 無い置き先(取り出しの先と seed の先の 2 つ)へ、取り出しの木から複写する。JSON は 1 行
  const a = repo({ plugins: { 'alpha-box': alphaTsv() }, extra: { [ALPHA_SRC]: ALPHA } });
  const ra = run(a);
  ok('DLL: 無い置き先(取り出しの先と seed の先)へ、取り出しの木から複写する。JSON は 1 行で placed 1',
    [ra.code, ra.out, same(host(a, `lift/out-plugins/${ALPHA_KEY}`), ALPHA), same(host(a, ALPHA_SEED), ALPHA)],
    [0, OUT(1, 0, 0, 0), true, true]);

  // (b) 在る置き先は触らない。中身が違えば警告だけ。両方在れば present 1 で通る
  const b = repo({ plugins: { 'alpha-box': alphaTsv() }, extra: { [ALPHA_SRC]: ALPHA } });
  fs.mkdirSync(path.dirname(host(b, `lift/out-plugins/${ALPHA_KEY}`)), { recursive: true });
  fs.writeFileSync(host(b, `lift/out-plugins/${ALPHA_KEY}`), 'keep');
  fs.mkdirSync(path.dirname(host(b, ALPHA_SEED)), { recursive: true });
  fs.writeFileSync(host(b, ALPHA_SEED), ALPHA);
  const rb = run(b);
  ok('在る置き先: 両方在れば触らず present 1。中身が違えば警告だけで、exit 0',
    [rb.code, rb.out, fs.readFileSync(host(b, `lift/out-plugins/${ALPHA_KEY}`), 'utf8'), /warning/.test(rb.err)],
    [0, OUT(0, 1, 0, 0), 'keep', true]);

  // (b2) 一方だけ在るときは、無い方だけを置き、在る方は中身のまま
  const b2 = repo({ plugins: { 'alpha-box': alphaTsv() }, extra: { [ALPHA_SRC]: ALPHA } });
  fs.mkdirSync(path.dirname(host(b2, `lift/out-plugins/${ALPHA_KEY}`)), { recursive: true });
  fs.writeFileSync(host(b2, `lift/out-plugins/${ALPHA_KEY}`), 'keep');
  const rb2 = run(b2);
  ok('一方だけ在る置き先: 無い方(seed の先)だけを置き、在る方は中身のまま',
    [rb2.code, rb2.out, fs.readFileSync(host(b2, `lift/out-plugins/${ALPHA_KEY}`), 'utf8'), same(host(b2, ALPHA_SEED), ALPHA)],
    [0, OUT(1, 0, 0, 0), 'keep', true]);

  // (c) 源の sha256 が行と違えば、複写せず mismatch に数えて exit 1
  const c = repo({ plugins: { 'alpha-box': alphaTsv() }, extra: { [ALPHA_SRC]: Buffer.from('tampered\n') } });
  const rc = run(c);
  ok('sha256 が違う源: 複写せず、mismatch に数えて exit 1',
    [rc.code, rc.out, fs.existsSync(host(c, ALPHA_SEED)), /mismatch/.test(rc.err)],
    [1, OUT(0, 0, 0, 1), false, true]);

  // (d) 源が無い: missing に数え、exit 1。1 件ずつ標準エラーに出す
  const d = repo({ plugins: { 'alpha-box': alphaTsv() }, extra: { 'Plugins/Fruity/Effects/Other/Other_x64.dll': Buffer.from('other\n') } });
  const rd = run(d);
  ok('源が無い: missing に数え、exit 1。標準エラーに 1 行',
    [rd.code, rd.out, rd.err.split('\n').filter((l) => /missing/.test(l)).length],
    [1, OUT(0, 0, 1, 0), 1]);

  // (e) 取り出しの木(host-extra/plugins)が無い作業(内蔵のプラグインの作業ではない): 置かず、missing に数えて exit 0
  const e = repo({ plugins: { 'alpha-box': alphaTsv() } });
  const re = run(e);
  ok('host-extra/plugins が無い作業: 置かず、missing に数えて exit 0', [re.code, re.out], [0, OUT(0, 0, 1, 0)]);

  // (g) 接頭辞の外(Shared\)の DLL は、取り出しの木に無くても missing でなく skipped。exit 0
  const g = repo({
    plugins: { reverb: `${HEADER}\nreverb_x64.dll\tShared\\Reverb_x64.dll\t補助\t${digest(ALPHA)}\t${ALPHA.length}\tnone\t\t-\n` },
    extra: { [ALPHA_SRC]: ALPHA },
  });
  const rg = run(g);
  ok('接頭辞の外の DLL(Shared\\): missing に数えず、skipped と標準エラーに出し、exit 0',
    [rg.code, rg.out, /skipped 1/.test(rg.err)], [0, OUT(0, 0, 0, 0), true]);

  // (f1) Data: 行の綴り。取り出しの木は guest のまま(空白あり)、置き先は plugins/ の下で空白を _ にする。installer の実物の行を使う
  const f = repo({
    plugins: { edison: `${HEADER}\neffects.edison.edison_x64.dll\tPlugins\\Fruity\\Effects\\Edison\\Edison_x64.dll\t本体\t${digest(ALPHA)}\t${ALPHA.length}\tnone\t\t-\n` },
    data: { edison: `${DATA_HEADER}\n${BLUR_ROW}` },
  });
  const rf = mod ? mod.rowsOf(f).filter((r) => r.guest === BLUR_GUEST) : [];
  ok('Data: 行の guest(空白のまま)と、置き先(plugins/ の下で空白を _)', rf.map((r) => [r.guest, r.targets]), [[BLUR_GUEST, [BLUR_HOST]]]);

  // (f2) Data の源は取り出しの木の空白の綴りで探す。中身が違えば mismatch(源は在る。missing ではない)
  const f2 = repo({
    plugins: { edison: `${HEADER}\neffects.edison.edison_x64.dll\tPlugins\\Fruity\\Effects\\Edison\\Edison_x64.dll\t本体\t${digest(ALPHA)}\t${ALPHA.length}\tnone\t\t-\n` },
    data: { edison: `${DATA_HEADER}\n${BLUR_ROW}` },
    extra: { 'Plugins/Fruity/Effects/Edison/Edison_x64.dll': ALPHA, [BLUR_SRC]: Buffer.from('tampered\n') },
  });
  const rf2 = run(f2);
  ok('Data の源: 空白の綴りで見つかる(中身が違えば mismatch。missing ではない)',
    [rf2.code, rf2.out, /mismatch.*Blur - alt/.test(rf2.err), /missing/.test(rf2.err)],
    [1, OUT(1, 0, 0, 1), true, false]);

  // (f3) Data の複写: 源と行の sha256 が合えば、置き先(_ の綴り)へ複写する
  const f3 = repo({ extra: { [BLUR_SRC]: BLUR } });
  const counts = mod ? mod.placeRows(f3, [{ guest: BLUR_GUEST, sha: digest(BLUR), targets: [BLUR_HOST] }], () => {}) : {};
  ok('Data の複写: 置き先(_ の綴り)に、源と同じ中身を置く', [counts.placed, same(host(f3, BLUR_HOST), BLUR)], [1, true]);

  // (h) 検査に落ちる plugin.tsv: exit 1 で、理由を標準エラーに出す(判定は、受け入れの結果で続く)
  const h = repo({ plugins: { bad: 'key\tguest\trole\n' } });
  const rh = run(h);
  ok('plugin.tsv が検査で落ちる: exit 1(標準エラーに理由)', [rh.code, rh.out, /place-plugin-dlls: .*header/.test(rh.err)], [1, '', true]);

  // (i) lift/plugins が無い: 行が無いので、0 件で exit 0
  const i = repo({});
  const ri = run(i);
  ok('lift/plugins が無ければ、0 件で exit 0', [ri.code, ri.out], [0, OUT(0, 0, 0, 0)]);

  // (k) guest に . や .. の段があれば、源を探さず exit 1(木の外へ出ない)
  const k = repo({ plugins: { 'alpha-box': `${HEADER}\n${ALPHA_KEY}\tPlugins\\Fruity\\Effects\\..\\Alpha.dll\t本体\t${digest(ALPHA)}\t${ALPHA.length}\tnone\t\t-\n` }, extra: { [ALPHA_SRC]: ALPHA } });
  const rk = run(k);
  ok('guest に .. の段があれば、exit 1(木の外へ出ない)', [rk.code, /unsafe/.test(rk.err)], [1, true]);
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}
console.log(failed === 0 ? 'ok   place-plugin-dlls: 全部通った' : `FAIL place-plugin-dlls: ${failed} 件`);
process.exit(failed === 0 ? 0 : 1);
