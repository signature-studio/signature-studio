#!/usr/bin/env node
// tools/project-status.mjs set STATUS NUMBER... : 作業フロー(.github/workflows/agent.yml)が、起動した Issue の Project の Status を動かす(start が In Progress、report が In Review)。設計は pm-work/workflow-design.md の「17」。
// 読みと書きは gh api graphql と gh api(注記)だけ。認証は gh の既定に従う(agent.yml は App の token を GH_CONFIG_DIR で渡す)。プロジェクトは PROJECT(組織/番号)。リポジトリは GH_REPO(無ければ GITHUB_REPOSITORY)。
// 補助の通知なので、Issue ごとの失敗(権限・閉じた Issue・選択肢の無い Status・gh の失敗)は理由を標準出力に 1 行ずつ出し、終了コードは 0。作業は止めない。使い方の誤りは 2。
// STATUS は Status の選択肢の名前(大文字・小文字の違いは無視する)。閉じた Issue は触らない(Project の組み込みの自動化が Done にする)。権限が無いときは、Issue に注記を 1 回だけ書く(NOTE の印の在るコメントが在れば書かない)。
import { execFileSync } from 'node:child_process';

const PROJECT = 'signature-studio/3';
const NOTE = '<!-- project-status:no-access -->';
const NOTE_BODY = `${NOTE}\nProject の Status を更新できなかった(App に Project の権限が無い、または Project が読めない)。作業は続けている。この注記は、初めて分かったときに 1 回だけ書く。`;
const DENIED = /HTTP 403|Resource not accessible|FORBIDDEN|INSUFFICIENT_SCOPES/;
const Q_PROJECT = 'query($login: String!, $number: Int!) { organization(login: $login) { projectV2(number: $number) { id fields(first: 50) { nodes { ... on ProjectV2SingleSelectField { id name options { id name } } } } } } }';
const Q_ISSUE = 'query($owner: String!, $name: String!, $number: Int!) { repository(owner: $owner, name: $name) { issue(number: $number) { id state projectItems(first: 50) { nodes { id project { id } } } } } }';
const M_ADD = 'mutation($project: ID!, $content: ID!) { addProjectV2ItemById(input: { projectId: $project, contentId: $content }) { item { id } } }';
const M_SET = 'mutation($project: ID!, $item: ID!, $field: ID!, $option: String!) { updateProjectV2ItemFieldValue(input: { projectId: $project, itemId: $item, fieldId: $field, value: { singleSelectOptionId: $option } }) { projectV2Item { id } } }';

class Skip extends Error {}
class Denied extends Error {
  constructor(code) {
    super(code);
    this.code = code;
  }
}

const usage = (message) => {
  process.stderr.write(`project-status: ${message}\nusage: project-status.mjs set STATUS NUMBER...\n`);
  process.exit(2);
};
const codeOf = (text) => text.match(/HTTP \d{3}/)?.[0] ?? 'FORBIDDEN';

function call(args, input) {
  try {
    return execFileSync('gh', args, { input, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] });
  } catch (e) {
    const text = [e.stderr, e.stdout].map((s) => String(s ?? '')).join('\n').trim() || e.message;
    if (DENIED.test(text)) throw new Denied(codeOf(text));
    throw new Error(text.split('\n')[0].slice(0, 200));
  }
}

function gql(query, variables) {
  const r = JSON.parse(call(['api', '-X', 'POST', 'graphql', '--input', '-'], JSON.stringify({ query, variables })));
  if (r.errors?.length) {
    const text = r.errors.map((x) => `${x.type ?? ''} ${x.message ?? ''}`.trim()).join('; ');
    if (r.errors.some((x) => x.type === 'FORBIDDEN' || DENIED.test(x.message ?? ''))) throw new Denied(codeOf(text));
    throw new Error(`GraphQL: ${text}`.slice(0, 200));
  }
  return r.data;
}

function pickOption(options, want) {
  const exact = options.filter((o) => o.name === want);
  const loose = options.filter((o) => o.name.toLowerCase() === want.toLowerCase());
  const hits = exact.length === 1 ? exact : loose;
  return hits.length === 1 ? hits[0] : null;
}

function readTarget(status) {
  const [org, number] = PROJECT.split('/');
  const p = gql(Q_PROJECT, { login: org, number: Number(number) }).organization?.projectV2;
  if (!p) throw new Denied('project not found or not readable');
  const f = p.fields.nodes.find((x) => x?.name === 'Status' && x.options);
  if (!f) throw new Skip('プロジェクトに Status の単一選択が無い');
  const option = pickOption(f.options, status);
  if (!option) throw new Skip(`Status の選択肢 ${status} が無い(在るのは ${f.options.map((o) => o.name).join(', ')})`);
  return { projectId: p.id, fieldId: f.id, option };
}

function setOne(owner, name, n, status, t) {
  const issue = gql(Q_ISSUE, { owner, name, number: Number(n) }).repository?.issue;
  if (!issue) throw new Skip('Issue が読めない');
  if (issue.state !== 'OPEN') throw new Skip('閉じている(Project の自動化に任せる)');
  let item = issue.projectItems.nodes.find((x) => x.project?.id === t.projectId)?.id;
  if (!item) item = gql(M_ADD, { project: t.projectId, content: issue.id }).addProjectV2ItemById.item.id;
  gql(M_SET, { project: t.projectId, item, field: t.fieldId, option: t.option.id });
  console.log(`project-status: #${n} ${status}: ok(${t.option.name})`);
}

function note(repo, n) {
  try {
    const bodies = call(['api', '--paginate', `repos/${repo}/issues/${n}/comments?per_page=100`, '--jq', '.[].body']);
    if (bodies.includes(NOTE)) {
      console.log(`project-status: #${n}: 注記は書いてある(印の行が在る)`);
      return;
    }
    call(['api', '-X', 'POST', `repos/${repo}/issues/${n}/comments`, '--input', '-'], JSON.stringify({ body: NOTE_BODY }));
    console.log(`project-status: #${n}: 注記を 1 回書いた`);
  } catch (e) {
    console.log(`project-status: #${n}: 注記を書けなかった: ${e.message}`);
  }
}

function skip(repo, n, status, e, access) {
  const denied = e instanceof Denied;
  const reason = denied ? `${access} (${e.code})` : e.message;
  console.log(`project-status: #${n} ${status}: skipped: ${reason}`);
  if (denied) note(repo, n);
}

function main(argv) {
  const [mode, status, ...numbers] = argv;
  if (mode !== 'set' || !status || !numbers.every((n) => /^[1-9]\d*$/.test(n))) usage('使い方が違う');
  const repo = process.env.GH_REPO || process.env.GITHUB_REPOSITORY || '';
  const [owner, name] = repo.split('/');
  if (!owner || !name) usage('リポジトリが分からない(GH_REPO)');
  if (numbers.length === 0) return;
  let target;
  try {
    target = readTarget(status);
  } catch (e) {
    for (const n of numbers) skip(repo, n, status, e, 'the App has no access to the project');
    return;
  }
  for (const n of numbers) {
    try {
      setOne(owner, name, n, status, target);
    } catch (e) {
      skip(repo, n, status, e, 'the App has no access');
    }
  }
}

main(process.argv.slice(2));
