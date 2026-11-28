#!/usr/bin/env node
// tools/place-plugin-dlls.mjs [--root DIR] : 作業の抽出の木(lift/out-app/host)に無い、lift/plugins/<名前>/plugin.tsv・data.tsv の行の実物を、内蔵のプラグインの取り出し(lift/out-app/host-extra/plugins)から複写する。
// 置き先と綴りは app/scripts/extract-fl.mjs と同じ(app/scripts/lib/plugin-rows.mjs の pluginRows())。在る置き先は触らず、sha256 が違えば警告だけ。
// .exe の行(Issue #1499: PluginManager.exe)は、取り出しの木(host-extra/plugins)でなく、resources の実物(lift/out-app/host/resources)から複写する(接頭辞の外でも置く)。
// 源が無い行は missing、源の sha256 が行と違う行は mismatch。接頭辞(ci/plugin-builtin-prefixes.txt)の外の DLL の行は skipped(ci/extract-dlls.tsv が扱う)。
// 標準出力に JSON の 1 行(placed・present・missing・mismatch は行の数)。標準エラーに、missing・mismatch・warning の行を 1 件ずつ。
// 終了コード: missing と mismatch が 0 なら 0。取り出しの木(host-extra/plugins)が無い作業(内蔵のプラグインの作業でない)は、置けないが 0。
// guard/judge.sh が受け入れの前に呼ぶ。失敗しても判定は続き、受け入れの結果で通るかが決まる。
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { pluginExes, pluginRows } from '../app/scripts/lib/plugin-rows.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const HOST = 'lift/out-app/host';
const EXTRA = 'lift/out-app/host-extra/plugins';
const INSTALL = 'C:\\Program Files\\Image-Line\\FL Studio 21\\';
const LAZY = '@lazy-file:';
const sha256 = (data) => crypto.createHash('sha256').update(data).digest('hex');

// 行: guest(インストール先の下の綴り)・sha(行の SHA-256)・targets(HOST の下の、置くべき先の全部)。DLL は取り出しの先と seed の先の 2 つ、Data は 1 つ。
// .exe の行は取り出しの先の 1 つ(from: resources。源は HOST の下の resources/ の実物)
export function rowsOf(root) {
  const { extract, seed } = pluginRows(path.join(root, 'lift/plugins'));
  const extractHost = new Map(extract.map(([, h]) => [path.posix.basename(h), h]));
  const dlls = seed.map(([full, h, sha]) => {
    if (!full.startsWith(INSTALL)) throw new Error(`the guest path is not under the install folder: ${full}`);
    const guest = full.slice(INSTALL.length);
    if (guest.split('\\').some((s) => s === '' || s === '.' || s === '..')) throw new Error(`unsafe guest path: ${guest}`);
    if (h.startsWith(LAZY)) return { guest, sha, targets: [h.slice(LAZY.length)] };
    const key = path.posix.basename(h);
    if (!extractHost.has(key)) throw new Error(`no extraction row for ${key}`);
    return { guest, sha, targets: [h, extractHost.get(key)] };
  });
  const exes = pluginExes(path.join(root, 'lift/plugins')).map(({ host, guest, sha256 }) => ({ guest, sha: sha256, targets: [host], from: 'resources' }));
  return [...dlls, ...exes];
}

// 行を置く。戻り値は行の数(placed・present・missing・mismatch・skipped)。log は 1 件ずつの標準エラー
export function placeRows(root, rows, log = (m) => console.error(m)) {
  const host = path.join(root, HOST), extra = path.join(root, EXTRA);
  const list = fs.readFileSync(path.join(root, 'ci/plugin-builtin-prefixes.txt'), 'utf8').split(/\r\n|\r|\n/).filter((l) => l !== '').map((l) => l.toLowerCase());
  const builtin = (guest) => list.some((p) => (p.endsWith('\\') ? guest.toLowerCase().startsWith(p) : guest.toLowerCase() === p));
  const count = { placed: 0, present: 0, missing: 0, mismatch: 0, skipped: 0 };
  for (const { guest, sha, targets, from: where } of rows) {
    for (const t of targets) {
      if (fs.existsSync(path.join(host, t)) && sha256(fs.readFileSync(path.join(host, t))) !== sha) log(`place-plugin-dlls: warning: ${t} は在るが sha256 が違う(触らない)`);
    }
    const absent = targets.filter((t) => !fs.existsSync(path.join(host, t)));
    if (absent.length === 0) {
      count.present++;
      continue;
    }
    if (where !== 'resources' && !builtin(guest)) {
      count.skipped++;
      continue;
    }
    const from = where === 'resources' ? path.join(host, 'resources', ...guest.split('\\')) : path.join(extra, ...guest.split('\\'));
    if (!fs.existsSync(from)) {
      count.missing++;
      log(`place-plugin-dlls: missing ${guest}(取り出しの木に源が無い)`);
      continue;
    }
    const data = fs.readFileSync(from);
    if (sha256(data) !== sha) {
      count.mismatch++;
      log(`place-plugin-dlls: mismatch ${guest}(源の sha256 が行と違う)`);
      continue;
    }
    for (const t of absent) {
      fs.mkdirSync(path.dirname(path.join(host, t)), { recursive: true });
      fs.writeFileSync(path.join(host, t), data);
    }
    count.placed++;
  }
  return count;
}

function main(argv) {
  const i = argv.indexOf('--root');
  try {
    const root = path.resolve(i >= 0 ? argv[i + 1] : ROOT);
    const { skipped, ...count } = placeRows(root, rowsOf(root));
    console.log(JSON.stringify(count));
    if (skipped > 0) console.error(`place-plugin-dlls: skipped ${skipped}(接頭辞の外。ci/extract-dlls.tsv が扱う)`);
    const hasExtra = fs.existsSync(path.join(root, EXTRA));
    return (count.missing === 0 && count.mismatch === 0) || !hasExtra ? 0 : 1;
  } catch (e) {
    console.error(`place-plugin-dlls: ${e.message}`);
    return 1;
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = main(process.argv.slice(2));
}
