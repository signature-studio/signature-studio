#!/usr/bin/env node
// spec/auto-retry.test.mjs : tools/auto-retry.mjs(main との衝突で push できなかった作業の自動の再試行の判定と、再試行の起動)を、合成データと偽の gh で確かめる。
// 表: 結論・衝突の有無・印の数の組み合わせで、retry か fail か。印の数は、コメントの本文に印の行を含む物だけ数える。
// trigger は偽の gh(PATH の先頭。呼び出しを GHLOG に残す)で試す。実際の GitHub には書かない。一時のフォルダは .tmp-spec-ar-* に作り、終わったら消す。終了コード 0 = 全部通った。
import { execFileSync, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const TOOL = path.join(ROOT, 'tools/auto-retry.mjs');
const MARKER = '<!-- auto-retry:push-conflict -->';
const TMP = fs.mkdtempSync(path.join(ROOT, '.tmp-spec-ar-'));
let failed = 0;
const ok = (name, got, want) => {
  if (JSON.stringify(got) === JSON.stringify(want)) console.log(`ok   ${name}`);
  else {
    failed++;
    console.log(`FAIL ${name}: expected ${JSON.stringify(want)}, got ${JSON.stringify(got)}`);
  }
};
const run = (args, input = '', env = {}) => {
  const r = spawnSync(process.execPath, [TOOL, ...args], { input, encoding: 'utf8', env: { ...process.env, ...env } });
  return { code: r.status, out: r.stdout.trim(), err: r.stderr };
};

// (1) decide: 結論・衝突・印の数の表
const table = [
  ['failure', 'true', 0, 'retry'],
  ['failure', 'true', 1, 'retry'],
  ['failure', 'true', 2, 'fail'],
  ['failure', 'true', 3, 'fail'],
  ['failure', 'false', 0, 'fail'],
  ['failure', 'false', 1, 'fail'],
  ['cancelled', 'true', 0, 'fail'],
  ['success', 'true', 0, 'fail'],
];
for (const [c, pc, n, want] of table) {
  ok(`decide: 結論 ${c}・衝突 ${pc}・印 ${n} は ${want}`, run(['decide', c, pc, String(n)]).out, want);
}
ok('decide: 印の数が負なら、使い方の誤りで 2 で終わる', run(['decide', 'failure', 'true', '-1']).code, 2);
ok('decide: 印の数が整数でなければ 2 で終わる', run(['decide', 'failure', 'true', 'x']).code, 2);
ok('decide: 衝突の有無が true・false 以外なら 2 で終わる', run(['decide', 'failure', 'yes', '0']).code, 2);
ok('decide: 引数が足りなければ 2 で終わる', run(['decide', 'failure']).code, 2);

// (2) count: コメントの JSON の配列から、印の行を含む物の数
const comments = [
  { body: `${MARKER}\n主文\n` },
  { body: `本文の途中に ${MARKER} が在る` },
  { body: '<!-- auto-retry:push-conflict-x -->' },
  { body: 'auto-retry:push-conflict' },
  { body: null },
  { body: '通常のコメント' },
];
ok('count: 印を含むコメントだけを数える(印の類似・null・通常のコメントは数えない)', run(['count'], JSON.stringify(comments)).out, '2');
ok('count: コメントが空の配列なら 0', run(['count'], '[]').out, '0');
ok('count: 印が 1 件なら 1', run(['count'], JSON.stringify([{ body: MARKER }])).out, '1');
ok('count: JSON でなければ 1 で終わる', run(['count'], 'not json').code, 1);
ok('count: 配列でなければ 1 で終わる', run(['count'], '{"body":"x"}').code, 1);

// (3) trigger: App の token で running を外す(偽の gh で、呼び出しを確かめる)
const bin = path.join(TMP, 'bin'); fs.mkdirSync(bin);
const ghlog = path.join(TMP, 'gh.log');
fs.writeFileSync(path.join(bin, 'gh'), `#!/usr/bin/env bash\necho "$*" >> "$GHLOG"\nexit 0\n`, { mode: 0o755 });
const env = { PATH: `${bin}:${process.env.PATH}`, GHLOG: ghlog, GH_REPO: 'o/r' };
fs.writeFileSync(ghlog, '');
const t1 = run(['trigger', '18', '21'], '', env);
ok('trigger: 指定の Issue ごとに、running を外す gh の呼び出し(--repo と --remove-label running)を行う', fs.readFileSync(ghlog, 'utf8').trim().split('\n'), [
  'issue edit 18 --repo o/r --remove-label running',
  'issue edit 21 --repo o/r --remove-label running',
]);
ok('trigger: 終了コード 0', t1.code, 0);
fs.writeFileSync(ghlog, '');
ok('trigger: 番号が無ければ、gh を呼ばずに 0 で終わる', run(['trigger'], '', env).code + '/' + fs.readFileSync(ghlog, 'utf8').length, '0/0');
fs.writeFileSync(ghlog, '');
ok('trigger: 番号が数でなければ、gh を呼ばずに 2 で終わる', run(['trigger', 'x'], '', env).code + '/' + fs.readFileSync(ghlog, 'utf8').length, '2/0');
// 失敗する gh: trigger は、その理由を標準エラーに出し、0 以外で終わる
const badBin = path.join(TMP, 'badbin'); fs.mkdirSync(badBin);
fs.writeFileSync(path.join(badBin, 'gh'), `#!/usr/bin/env bash\necho "fake gh: 失敗" >&2\nexit 1\n`, { mode: 0o755 });
ok('trigger: gh が失敗すると 0 以外で終わる', run(['trigger', '18'], '', { PATH: `${badBin}:${process.env.PATH}`, GH_REPO: 'o/r' }).code !== 0 ? 'nonzero' : 'zero', 'nonzero');
ok('count・decide・trigger 以外の指定は、使い方の誤りで 2 で終わる', run(['nope']).code, 2);

fs.rmSync(TMP, { recursive: true, force: true });
process.exit(failed ? 1 : 0);
