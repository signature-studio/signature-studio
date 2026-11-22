#!/usr/bin/env node
// tools/external-inputs/check.mjs DIR [--lock FILE] : 外部プラグインのインストーラ(ci/artifacts.lock の installers/ の下の、FL Studio 以外の 3 件)を、DIR の中の同じ名前のファイルと照合する。
// 照合は、大きさを先に、続けて SHA-256 を見る。結果を JSON の 1 行で標準出力に出す: {"ok":真偽,"files":[{"used":説明,"ok":真偽,"reason":"ok|missing|size|sha256"}]}
// ファイル名とパスは出さない(used は lock の説明の列)。終了コード: 全件が一致すれば 0、どれか違えば 1、使い方の誤りは 64。
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const CHUNK = 1 << 24;

export function externalEntries(lockText) {
  return lockText.split(/\r\n|\r|\n/)
    .filter((ln) => ln !== '' && !ln.startsWith('#'))
    .map((ln) => ln.split('\t'))
    .filter((c) => c[0].startsWith('installers/') && !path.posix.basename(c[0]).startsWith('flstudio_'))
    .map((c) => ({ name: path.posix.basename(c[0]), size: Number(c[1]), sha256: c[2], used: c[4] }));
}

function sha256File(file) {
  const h = crypto.createHash('sha256');
  const fd = fs.openSync(file, 'r');
  try {
    const b = Buffer.alloc(CHUNK);
    for (;;) {
      const n = fs.readSync(fd, b, 0, CHUNK, null);
      if (n === 0) break;
      h.update(b.subarray(0, n));
    }
  } finally {
    fs.closeSync(fd);
  }
  return h.digest('hex');
}

export function checkDir(entries, dir) {
  const files = entries.map((e) => {
    const file = path.join(dir, e.name);
    const st = fs.existsSync(file) ? fs.statSync(file) : null;
    let reason = 'ok';
    if (!st || !st.isFile()) reason = 'missing';
    else if (st.size !== e.size) reason = 'size';
    else if (sha256File(file) !== e.sha256) reason = 'sha256';
    return { used: e.used, ok: reason === 'ok', reason };
  });
  return { ok: files.every((f) => f.ok), files };
}

function main(argv) {
  const args = [...argv];
  let lock = path.join(ROOT, 'ci/artifacts.lock');
  const at = args.indexOf('--lock');
  if (at >= 0) {
    lock = args[at + 1];
    args.splice(at, 2);
  }
  const [dir] = args;
  if (!dir || lock === undefined) {
    process.stderr.write('usage: tools/external-inputs/check.mjs DIR [--lock FILE]\n');
    return 64;
  }
  const result = checkDir(externalEntries(fs.readFileSync(lock, 'utf8')), dir);
  process.stdout.write(`${JSON.stringify(result)}\n`);
  return result.ok ? 0 : 1;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = main(process.argv.slice(2));
}
