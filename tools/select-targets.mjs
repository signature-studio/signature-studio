#!/usr/bin/env node
// tools/select-targets.mjs select | claim TARGETS RETRY : 作業フロー(.github/workflows/agent.yml)が、起動する Issue を決める。設計は pm-work/workflow-design.md の「5」。
// select: GITHUB_EVENT_NAME と GITHUB_EVENT_PATH のイベントから候補を集め、起動してよい物だけを選ぶ。イベントを起こした人(sender)に書き込み権限が無ければ候補を集めない。出力: {"targets":[番号],"retry":真偽,"labels":"対象のラベルの重複なしのカンマ区切り","issue_labels":{"番号":"その Issue のラベルのカンマ区切り"}}
// claim TARGETS RETRY: 候補(JSON の配列)を、もう一度確かめ、起動してよい物に running を付ける。RETRY が true なら failed を外す。出力: {"claimed":[番号]}
// 読みは gh api、書きは gh issue edit だけ。リポジトリは GH_REPO(無ければ GITHUB_REPOSITORY)。認証は gh の既定に従う。
// 起動しない理由は標準エラーに 1 件ずつ出す。標準出力は JSON の 1 行だけ。
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

const repo = process.env.GH_REPO || process.env.GITHUB_REPOSITORY;
const gh = (...args) => execFileSync('gh', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'] });
const api = (path) => JSON.parse(gh('api', path));
const names = (issue) => new Set((issue.labels ?? []).map((l) => l.name));
const log = (message) => process.stderr.write(`select: ${message}\n`);

function reason(issue, { openBlockers, subIssues }, retry) {
  const labels = names(issue);
  if (issue.pull_request) return 'PR である';
  if (issue.state !== 'open') return '閉じている';
  if (issue.type?.name !== 'Task') return `Issue の Type が Task でない(${issue.type?.name ?? '無し'})`;
  for (const l of ['pm-hold', 'running', 'needs-owner']) if (labels.has(l)) return `${l} が付いている`;
  if (labels.has('failed') && !retry) return 'failed が付いている(/retry か、ラベルを外すと再試行する)';
  if (subIssues > 0) return 'sub-issue を持つ(親)';
  if (openBlockers > 0) return `blocked by の open が ${openBlockers} 件ある`;
  return '';
}

function state(n) {
  const issue = api(`repos/${repo}/issues/${n}`);
  if (issue.pull_request) return { issue, openBlockers: 0, subIssues: 0 };
  const blocked = api(`repos/${repo}/issues/${n}/dependencies/blocked_by?per_page=100`);
  const subs = api(`repos/${repo}/issues/${n}/sub_issues?per_page=100`);
  return { issue, openBlockers: blocked.filter((b) => b.state === 'open').length, subIssues: subs.length };
}

const writers = new Set(['admin', 'write']);

function canWrite(actor) {
  if (actor?.type === 'Bot') return true;
  if (!actor?.login) return false;
  try {
    return writers.has(api(`repos/${repo}/collaborators/${actor.login}/permission`).permission);
  } catch {
    log(`${actor.login} の権限を読めない(書き込み無しとして扱う)`);
    return false;
  }
}

function candidates(eventName, event) {
  if (!canWrite(event.sender)) {
    log(`イベントを起こした ${event.sender?.login ?? '(不明)'} に書き込み権限が無い`);
    return { numbers: [], retry: false };
  }
  if (eventName === 'issue_comment') {
    if (event.issue.pull_request || event.comment.user?.type === 'Bot') return { numbers: [], retry: false };
    return { numbers: [event.issue.number], retry: /^\/retry(\s|$)/.test(event.comment.body.trimStart()) };
  }
  if (event.action === 'closed') {
    const blocking = api(`repos/${repo}/issues/${event.issue.number}/dependencies/blocking?per_page=100`);
    return { numbers: blocking.map((b) => b.number), retry: false };
  }
  return { numbers: [event.issue.number], retry: false };
}

function select() {
  const event = JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH, 'utf8'));
  const { numbers, retry } = candidates(process.env.GITHUB_EVENT_NAME, event);
  const targets = [];
  const labels = new Set();
  const issueLabels = {};
  for (const n of [...new Set(numbers)].sort((a, b) => a - b)) {
    const s = state(n);
    const why = reason(s.issue, s, retry);
    if (why) log(`#${n} は起動しない: ${why}`);
    else {
      targets.push(n);
      const mine = [...names(s.issue)].sort();
      mine.forEach((l) => labels.add(l));
      issueLabels[n] = mine.join(',');
    }
  }
  return { targets, retry, labels: [...labels].sort().join(','), issue_labels: issueLabels };
}

function claim(targets, retry) {
  const claimed = [];
  for (const n of targets) {
    const s = state(n);
    const why = reason(s.issue, s, retry);
    if (why) {
      log(`#${n} は起動しない: ${why}`);
      continue;
    }
    gh('issue', 'edit', String(n), '--repo', repo, '--add-label', 'running');
    if (names(s.issue).has('failed')) gh('issue', 'edit', String(n), '--repo', repo, '--remove-label', 'failed');
    claimed.push(n);
    log(`#${n} を起動する`);
  }
  return { claimed };
}

const [mode, targets = '[]', retry = 'false'] = process.argv.slice(2);
if (mode === 'select') process.stdout.write(`${JSON.stringify(select())}\n`);
else if (mode === 'claim') process.stdout.write(`${JSON.stringify(claim(JSON.parse(targets), retry === 'true'))}\n`);
else {
  process.stderr.write('usage: select-targets.mjs select | claim TARGETS RETRY\n');
  process.exit(2);
}
