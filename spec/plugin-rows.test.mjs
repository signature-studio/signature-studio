#!/usr/bin/env node
// spec/plugin-rows.test.mjs : app/scripts/lib/plugin-rows.mjs(lift/plugins/<名前>/plugin.tsv の行を、抽出の表と seed の表の行にする)を、一時の plugin.tsv で確かめる。
// Balance の行は、表(ci/extract-dlls.tsv・vfs-seed-idle1.tsv)に在った行と同じ文字列になることを確かめる。一時のフォルダは .tmp-spec-pr-* に作り、終わったら消す。終了コード 0 = 全部通った。
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
let pluginRows;
try {
  ({ pluginRows } = await import(pathToFileURL(path.join(ROOT, 'app/scripts/lib/plugin-rows.mjs')).href));
} catch (e) {
  console.log(`FAIL import: ${e.message}`);
  process.exit(1);
}
let failed = 0;
const ok = (name, got, want) => {
  if (JSON.stringify(got) === JSON.stringify(want)) console.log(`ok   ${name}`);
  else {
    failed++;
    console.log(`FAIL ${name}: expected ${JSON.stringify(want)}, got ${JSON.stringify(got)}`);
  }
};

const HEADER = ['key', 'guest', 'role', 'sha256', 'size', 'hook', 'args', 'omit'].join('\t');
const SHA_BALANCE = 'a399cbe060ec3002a920f62fdcc2a2fb36bb1f3918281e01cdd5fc70724b8064';
const SHA_FILTER = 'f4801444224236132b61983b7b81d0d3f89395085a1dc5430653a5edb7273d6c';
const SHA_DELAY = 'c4fed60da5034d5c04019a4992563334aa6eb89e2adc211214aaca520ea928fc';
const GUEST = 'C:\\Program Files\\Image-Line\\FL Studio 21\\';
const BALANCE_ROW = ['effects.fruity-balance.fruity_balance_x64.dll', 'Plugins\\Fruity\\Effects\\Fruity Balance\\Fruity Balance_x64.dll', '本体', SHA_BALANCE, '3718656', 'existing', '--pdata', '0x2c13a0'].join('\t');

const tmp = fs.mkdtempSync(path.join(ROOT, '.tmp-spec-pr-'));
// 一時のフォルダを作り、その中の plugin.tsv を書く(files: フォルダ名 → 中身)
function plugins(files) {
  const dir = fs.mkdtempSync(path.join(tmp, 'p-'));
  for (const [folder, text] of Object.entries(files)) {
    fs.mkdirSync(path.join(dir, folder), { recursive: true });
    if (text !== null) fs.writeFileSync(path.join(dir, folder, 'plugin.tsv'), text);
  }
  return dir;
}
// 例外の文言(失敗なら文言を返し、通れば 'no error')
function error(dir) {
  try {
    pluginRows(dir);
    return 'no error';
  } catch (e) {
    return e.message;
  }
}

try {
  // 1. Balance の 1 行。host の先は鍵(Issue #1379)。fe-build の plugins 段が読む名前と同じ
  const b = plugins({ 'fruity-balance': `${HEADER}\n${BALANCE_ROW}\n` });
  const rb = pluginRows(b);
  ok('Balance: 抽出の行は、鍵の名前の host の先(installer の名前・host の先・SHA-256)',
    rb.extract.map((r) => r.join('\t')), [`fruity balance_x64.dll\tlift/out-plugins/effects.fruity-balance.fruity_balance_x64.dll\t${SHA_BALANCE}`]);
  ok('Balance: seed の行は、guest の絶対パス・host の先(末尾が鍵)・SHA-256',
    rb.seed.map((r) => r.join('\t')), [`${GUEST}Plugins\\Fruity\\Effects\\Fruity Balance\\Fruity Balance_x64.dll\tlift/out-idle1/inputs/install/Plugins/Fruity/Effects/effects.fruity-balance.fruity_balance_x64.dll\t${SHA_BALANCE}`]);

  // 2. 2 つのフォルダ。鍵の順に並ぶ(zeta は alpha の後)。補助の DLL も行になる
  const two = plugins({
    zeta: `${HEADER}\neffects.zeta.z.dll\tPlugins\\Fruity\\Effects\\Zeta Box\\Zeta Box_x64.dll\t本体\t${SHA_FILTER}\t100\tnone\t\t-\n`,
    alpha: `# 注釈\n\n${HEADER}\neffects.alpha.a.dll\tPlugins\\Fruity\\Effects\\Alpha Box\\Alpha Box_x64.dll\t本体\t${SHA_DELAY}\t200\trva:0x10,0x20,0x30\t--pdata --x\t0x100,0x200\neffects.alpha.helper.dll\tPlugins\\Fruity\\Effects\\Alpha Box\\helper.dll\t補助\t${SHA_BALANCE}\t300\tnone\t\t-\n`,
  });
  const r2 = pluginRows(two);
  ok('2 フォルダ: 抽出の行は鍵の順(alpha の 2 行、zeta の 1 行)。host の先は鍵',
    r2.extract.map((r) => r.join('\t')), [
      `alpha box_x64.dll\tlift/out-plugins/effects.alpha.a.dll\t${SHA_DELAY}`,
      `helper.dll\tlift/out-plugins/effects.alpha.helper.dll\t${SHA_BALANCE}`,
      `zeta box_x64.dll\tlift/out-plugins/effects.zeta.z.dll\t${SHA_FILTER}`,
    ]);
  ok('2 フォルダ: seed の行は、guest の絶対パス・host の先(フォルダの階層・末尾が鍵)・SHA-256',
    r2.seed.map((r) => r.join('\t')), [
      `${GUEST}Plugins\\Fruity\\Effects\\Alpha Box\\Alpha Box_x64.dll\tlift/out-idle1/inputs/install/Plugins/Fruity/Effects/effects.alpha.a.dll\t${SHA_DELAY}`,
      `${GUEST}Plugins\\Fruity\\Effects\\Alpha Box\\helper.dll\tlift/out-idle1/inputs/install/Plugins/Fruity/Effects/effects.alpha.helper.dll\t${SHA_BALANCE}`,
      `${GUEST}Plugins\\Fruity\\Effects\\Zeta Box\\Zeta Box_x64.dll\tlift/out-idle1/inputs/install/Plugins/Fruity/Effects/effects.zeta.z.dll\t${SHA_FILTER}`,
    ]);

  // 2b. ファイル名が同じ DLL が 2 本(Issue #1379: 18 本が engine_x64.dll)。鍵が違えば、検査は通り、host の先は鍵で別になる
  const same = plugins({
    'effects-a': `${HEADER}\neffects.a.engine_x64.dll\tPlugins\\Fruity\\Effects\\A\\engine_x64.dll\t本体\t${SHA_BALANCE}\t1\tnone\t\t-\n`,
    'generators-b': `${HEADER}\ngenerators.b.engine_x64.dll\tPlugins\\Fruity\\Generators\\B\\engine_x64.dll\t本体\t${SHA_FILTER}\t1\tnone\t\t-\n`,
  });
  ok('同じ名前の DLL が 2 本: 検査は通り、抽出の行は鍵で別の host の先になる',
    pluginRows(same).extract.map((r) => r.join('\t')), [
      `engine_x64.dll\tlift/out-plugins/effects.a.engine_x64.dll\t${SHA_BALANCE}`,
      `engine_x64.dll\tlift/out-plugins/generators.b.engine_x64.dll\t${SHA_FILTER}`,
    ]);

  // 2c. Rust(plugins.rs)の試験の期待値と、同じ行になる(JS と Rust が同じ名前を作ることの検査)
  const rs = fs.readFileSync(path.join(ROOT, 'lift/crates/fe-build/src/plugins.rs'), 'utf8');
  const rustLit = (s) => s.replaceAll('\\', '\\\\').replaceAll('\t', '\\t');
  ok('Rust の試験の期待値(extract_row)と、同じ抽出の行', rs.includes(rustLit(rb.extract[0].join('\t'))), true);
  ok('Rust の試験の期待値(seed_row)と、同じ seed の行', rs.includes(rustLit(rb.seed[0].join('\t'))), true);

  // 3. 無い フォルダ・フォルダの無い plugins は、行が無い(今の main と同じ動作)
  ok('lift/plugins が無ければ、行は無い', pluginRows(path.join(tmp, 'none')), { extract: [], seed: [] });
  ok('フォルダが空なら、行は無い', pluginRows(plugins({})), { extract: [], seed: [] });

  // 4. 検査: 失敗する行は、例外になる(抽出は止まる)。失敗の文言に合う物を 'yes' とし、違えば文言を返す
  const fails = (dir, re) => (re.test(error(dir)) ? 'yes' : error(dir));
  const row = (sha, role = '本体', guest = 'Plugins\\X\\Y\\One_x64.dll', extra = '1\tnone\t\t-') => `${HEADER}\nk.dll\t${guest}\t${role}\t${sha}\t${extra}\n`;
  ok('検査: plugin.tsv の無いフォルダは、例外', fails(plugins({ 'no-file': null }), /ENOENT/), 'yes');
  ok('検査: 鍵が 2 つの plugin で重複すると、例外', fails(plugins({
    a: row(SHA_BALANCE),
    b: row(SHA_FILTER, '本体', 'Plugins\\X\\Y\\Two_x64.dll'),
  }), /not unique/), 'yes');
  ok('検査: sha256 が 63 桁なら、例外', fails(plugins({ a: row(SHA_BALANCE.slice(1)) }), /sha256/), 'yes');
  ok('検査: sha256 が大文字の 16 進なら、例外', fails(plugins({ a: row(SHA_BALANCE.toUpperCase()) }), /sha256/), 'yes');
  ok('検査: 見出しが違えば、例外', fails(plugins({ a: `key\tguest\trole\tsha256\tsize\thook\n${row(SHA_BALANCE).split('\n')[1]}\n` }), /header/), 'yes');
  ok('検査: 役割が 本体・補助 以外なら、例外', fails(plugins({ a: row(SHA_BALANCE, 'main') }), /role/), 'yes');
  ok('検査: 列が 7 つでなければ、例外', fails(plugins({ a: row(SHA_BALANCE, '本体', 'Plugins\\X\\Y\\One_x64.dll', '1\tnone\t') }), /fields/), 'yes');
  ok('検査: omit が - でも 16 進の RVA の並びでもなければ、例外', fails(plugins({ a: row(SHA_BALANCE, '本体', 'Plugins\\X\\Y\\One_x64.dll', '1\tnone\t\tzz') }), /omit/), 'yes');
  ok('検査: omit の列が空なら、例外', fails(plugins({ a: row(SHA_BALANCE, '本体', 'Plugins\\X\\Y\\One_x64.dll', '1\tnone\t\t') }), /omit/), 'yes');
  ok('検査: フォルダの名前が小文字・数字・- 以外なら、例外', fails(plugins({ Fruity: `${HEADER}\n${BALANCE_ROW}\n` }), /folder name/), 'yes');
  ok('検査: guest の場所が同じ(大文字小文字を除く)なら、例外', fails(plugins({
    a: row(SHA_BALANCE),
    b: row(SHA_FILTER, '本体', 'plugins\\x\\y\\one_X64.dll').replace('k.dll', 'k2.dll'),
  }), /guest path is not unique/), 'yes');
  ok('検査: hook の RVA が 3 つでなければ、例外', fails(plugins({ a: row(SHA_BALANCE, '本体', 'Plugins\\X\\Y\\One_x64.dll', '1\trva:0x10,0x20\t\t-') }), /hook/), 'yes');
  ok('検査: 行が無い plugin.tsv は、例外', fails(plugins({ a: `${HEADER}\n` }), /no DLL row/), 'yes');
  ok('検査: 鍵が .dll で終わらないなら、例外', fails(plugins({ a: `${HEADER}\nk\tPlugins\\X\\Y\\One_x64.dll\t本体\t${SHA_BALANCE}\t1\tnone\t\t-\n` }), /ends with \.dll/), 'yes');

  // 5. 実物の lift/plugins(あれば)が検査を通る
  ok('実物の lift/plugins(無ければ空)は検査を通る', (() => { try { pluginRows(path.join(ROOT, 'lift/plugins')); return 'pass'; } catch (e) { return e.message; } })(), 'pass');
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}

if (failed) {
  console.log(`FAIL ${failed} 件`);
  process.exit(1);
}
console.log('ok   全部通った');
