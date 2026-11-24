#!/usr/bin/env node
// tools/auto-retry.mjs count | decide 結論 衝突 印の数 | trigger 番号...
// 作業フロー(.github/workflows/agent.yml)の報告が、main との衝突で push できなかった作業を自動で再試行するかを決め、再試行の起動をする。設計は pm-work/workflow-design.md の「7」。
// count: 標準入力の、Issue のコメントの JSON の配列から、印の行(MARKER)を含む物の数を出す。印の行は、自動の再試行のたびに報告が書く。
// decide: 作業ジョブの結論・衝突の有無(true か false)・印の数から、retry か fail を出す。retry は、結論が failure で、衝突があり、印の数が MAX 未満のときだけ。
// trigger 番号...: 各 Issue の running を外す(gh issue edit --remove-label running)。running の変更は App の token で行う(呼び出し側が GH_CONFIG_DIR を渡す)。GITHUB_TOKEN の変更は次の run を起こさないため。
// 使い方の誤りは 2 で終わる。失敗は理由を標準エラーに出して 1 で終わる。リポジトリは GH_REPO(無ければ GITHUB_REPOSITORY)。
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

const MARKER = '<!-- auto-retry:push-conflict -->';
const MAX = 2;

function die(message, code = 1) {
  process.stderr.write(`auto-retry: ${message}\n`);
  process.exit(code);
}

function usage() {
  die('使い方: auto-retry.mjs count | decide 結論 衝突 印の数 | trigger 番号...', 2);
}

function count(text) {
  let comments;
  try {
    comments = JSON.parse(text);
  } catch {
    die('コメントの JSON が読めない');
  }
  if (!Array.isArray(comments)) die('コメントが配列ではない');
  return comments.filter((c) => typeof c?.body === 'string' && c.body.includes(MARKER)).length;
}

function decide(conclusion, pushConflict, n) {
  if (pushConflict !== 'true' && pushConflict !== 'false') usage();
  if (!/^\d+$/.test(n)) usage();
  return conclusion === 'failure' && pushConflict === 'true' && Number(n) < MAX ? 'retry' : 'fail';
}

function trigger(numbers) {
  if (!numbers.every((n) => /^[1-9]\d*$/.test(n))) usage();
  if (numbers.length === 0) return;
  const repo = process.env.GH_REPO || process.env.GITHUB_REPOSITORY;
  if (!repo) die('リポジトリが分からない(GH_REPO)', 2);
  for (const n of numbers) {
    try {
      execFileSync('gh', ['issue', 'edit', n, '--repo', repo, '--remove-label', 'running'], { stdio: ['ignore', 'inherit', 'inherit'] });
    } catch {
      die(`#${n} の running を外せなかった`);
    }
  }
}

const [mode, ...args] = process.argv.slice(2);
if (mode === 'count' && args.length === 0) process.stdout.write(`${count(readFileSync(0, 'utf8'))}\n`);
else if (mode === 'decide' && args.length === 3) process.stdout.write(`${decide(...args)}\n`);
else if (mode === 'trigger') trigger(args);
else usage();
