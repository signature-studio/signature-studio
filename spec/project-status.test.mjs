#!/usr/bin/env node
// spec/project-status.test.mjs : tools/project-status.mjs(起動した Issue の Project の Status を In Progress・In Review に移す)と、.github/workflows/agent.yml の、その段の置き場所・条件・token を、合成の応答と偽の gh で確かめる。
// 偽の gh(PATH の先頭)は、呼び出しを PSLOG に 1 行ずつ残し、応答は PSFX(合成の fixture の JSON)から返す。実際の GitHub には書かない。一時のフォルダは .tmp-spec-ps-* に作り、終わったら消す。終了コード 0 = 全部通った。
import { execFileSync, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const TOOL = path.join(ROOT, 'tools/project-status.mjs');
const WF = path.join(ROOT, '.github/workflows/agent.yml');
const TMP = fs.mkdtempSync(path.join(ROOT, '.tmp-spec-ps-'));
const NOTE = '<!-- project-status:no-access -->';
let failed = 0;
const ok = (name, got, want) => {
  if (JSON.stringify(got) === JSON.stringify(want)) console.log(`ok   ${name}`);
  else {
    failed++;
    console.log(`FAIL ${name}: expected ${JSON.stringify(want)}, got ${JSON.stringify(got)}`);
  }
};

// 偽の gh: graphql は、query の中身で応答を決める。REST の本文の取得(--paginate)は comments、書き込み(POST の comments)は記録だけ
const FAKE_GH = `#!/usr/bin/env node
const fs = require('node:fs');
const args = process.argv.slice(2);
const fx = JSON.parse(fs.readFileSync(process.env.PSFX, 'utf8'));
const stdin = args.includes('--input') ? fs.readFileSync(0, 'utf8') : '';
fs.appendFileSync(process.env.PSLOG, JSON.stringify({ args, body: stdin ? JSON.parse(stdin) : null }) + '\\n');
const fail = (msg) => { process.stderr.write(msg + '\\n'); process.exit(1); };
if (fx.gh_error) fail(fx.gh_error);
if (args[0] !== 'api') fail('fake gh: unsupported');
if (args.includes('graphql')) {
  const body = JSON.parse(stdin);
  const q = body.query; const v = body.variables;
  if (q.includes('organization(login')) {
    if (fx.project === 'denied') fail('gh: Resource not accessible by integration (HTTP 403)');
    process.stdout.write(JSON.stringify({ data: { organization: { projectV2: fx.project ?? null } } }));
  } else if (q.includes('repository(owner')) {
    const i = fx.issues[String(v.number)];
    const issue = i === undefined ? null : { id: i.id, state: i.state, projectItems: { nodes: (i.items ?? []).map((x) => ({ id: x.id, project: { id: x.projectId } })) } };
    process.stdout.write(JSON.stringify({ data: { repository: { issue } } }));
  } else if (q.includes('addProjectV2ItemById')) {
    process.stdout.write(JSON.stringify({ data: { addProjectV2ItemById: { item: { id: 'PVTI_added_' + v.content } } } }));
  } else if (q.includes('updateProjectV2ItemFieldValue')) {
    process.stdout.write(JSON.stringify({ data: { updateProjectV2ItemFieldValue: { projectV2Item: { id: v.item } } } }));
  } else fail('fake gh: unknown query');
} else if (args.includes('--paginate')) {
  const n = args.find((a) => /issues\\/\\d+\\/comments/.test(a)).match(/issues\\/(\\d+)\\//)[1];
  process.stdout.write((fx.comments?.[n] ?? []).join('\\n'));
} else if (args.includes('POST')) {
  process.stdout.write('{}');
} else fail('fake gh: unsupported api call');
`;
const bin = path.join(TMP, 'bin');
fs.mkdirSync(bin);
fs.writeFileSync(path.join(bin, 'gh'), FAKE_GH, { mode: 0o755 });

const OPTIONS = [
  { id: 'b1', name: 'Backlog' },
  { id: 'r1', name: 'Ready' },
  { id: '47fc9ee4', name: 'In progress' },
  { id: 'df73e18b', name: 'In review' },
  { id: 'd1', name: 'Done' },
];
const PROJECT = {
  id: 'PVT_1',
  fields: {
    nodes: [
      {},
      { id: 'PVTSSF_status', name: 'Status', options: OPTIONS },
      { id: 'PVTSSF_size', name: 'Size', options: [{ id: 's1', name: 'XS' }] },
    ],
  },
};
const ISSUES = {
  12: { id: 'I_12', state: 'OPEN', items: [] },
  13: { id: 'I_13', state: 'OPEN', items: [{ id: 'PVTI_13', projectId: 'PVT_1' }] },
  14: { id: 'I_14', state: 'CLOSED', items: [{ id: 'PVTI_14', projectId: 'PVT_1' }] },
};

// 1 回の呼び出し: fixture を書き、tool を偽の gh で動かし、標準出力・終了コード・呼び出しの記録を返す
function tool(name, fixture, args, extraEnv = {}) {
  const dir = path.join(TMP, name);
  fs.mkdirSync(dir, { recursive: true });
  const psfx = path.join(dir, 'fx.json');
  const pslog = path.join(dir, 'gh.log');
  fs.writeFileSync(psfx, JSON.stringify(fixture));
  fs.writeFileSync(pslog, '');
  const r = spawnSync(process.execPath, [TOOL, ...args], {
    encoding: 'utf8',
    env: { ...process.env, PATH: `${bin}:${process.env.PATH}`, GH_REPO: 'signature-studio/signature-studio', PSFX: psfx, PSLOG: pslog, ...extraEnv },
  });
  const calls = fs.readFileSync(pslog, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l));
  return { code: r.status, out: r.stdout.trim(), err: r.stderr, calls };
}
const mutations = (calls) => calls.filter((c) => c.args.includes('graphql') && /mutation/.test(c.body?.query ?? '')).map((c) => c.body.query.match(/(addProjectV2ItemById|updateProjectV2ItemFieldValue)/)[1]);
const setCall = (calls) => calls.find((c) => /updateProjectV2ItemFieldValue/.test(c.body?.query ?? ''))?.body.variables;
const comments = (calls) => calls.filter((c) => c.args.includes('POST') && c.args.some((a) => /issues\/\d+\/comments$/.test(a)));
const fixture = (extra = {}) => ({ project: PROJECT, issues: ISSUES, comments: {}, ...extra });

// (1) 選択肢の名前から id を引く(In Progress は In progress と同じ。大文字・小文字の違いを無視する)
let r = tool('progress', fixture(), ['set', 'In Progress', '12']);
ok('ps(1): In Progress は選択肢 In progress に当たり、その id(47fc9ee4)で設定する', setCall(r.calls)?.option, '47fc9ee4');
ok('ps(1): 標準出力に、その Issue の結果を 1 行で出す', r.out, 'project-status: #12 In Progress: ok(In progress)');
ok('ps(1): 終了コードは 0', r.code, 0);
// (2) In Review は In review の id(df73e18b)
r = tool('review', fixture(), ['set', 'In Review', '13']);
ok('ps(2): In Review は選択肢 In review に当たり、その id(df73e18b)で設定する', setCall(r.calls)?.option, 'df73e18b');
// (3) item が無ければ足してから設定する。在れば足さない
r = tool('add', fixture(), ['set', 'In Progress', '12']);
ok('ps(3): item が無い Issue は、addProjectV2ItemById(Issue の node id)で足してから、設定する', mutations(r.calls), ['addProjectV2ItemById', 'updateProjectV2ItemFieldValue']);
ok('ps(3): 足す時の contentId は Issue の node id(I_12)', r.calls.find((c) => /addProjectV2ItemById/.test(c.body?.query ?? ''))?.body.variables.content, 'I_12');
ok('ps(3): 設定の item は、足した item の id', setCall(r.calls)?.item, 'PVTI_added_I_12');
r = tool('noadd', fixture(), ['set', 'In Review', '13']);
ok('ps(3): item が在る Issue は足さず、その item を設定する', [mutations(r.calls), setCall(r.calls)?.item], [['updateProjectV2ItemFieldValue'], 'PVTI_13']);
// (4) 閉じた Issue は触らない(In Review にしない・足さない)。Project の自動化が Done にする
r = tool('closed', fixture(), ['set', 'In Review', '14']);
ok('ps(4): 閉じた Issue は、変更の呼び出しを 1 件もしない', mutations(r.calls), []);
ok('ps(4): 閉じた Issue は、理由を出して 0 で終わる', [r.code, r.out], [0, 'project-status: #14 In Review: skipped: 閉じている(Project の自動化に任せる)']);
r = tool('mixed', fixture(), ['set', 'In Review', '14', '13']);
ok('ps(4): 閉じた 14 と開いた 13 が並んでも、13 だけを設定する', [setCall(r.calls)?.item, r.out.split('\n').length], ['PVTI_13', 2]);
// (5) 権限が無い(403)のとき: 理由を出し、0 で終わり、注記を Issue に 1 回だけ書く
r = tool('denied', fixture({ project: 'denied' }), ['set', 'In Progress', '12', '13']);
ok('ps(5): 403 は、skipped の理由(HTTP 403)を 1 件ずつ出し、0 で終わる', [r.code, r.out.split('\n')[0]], [0, 'project-status: #12 In Progress: skipped: the App has no access to the project (HTTP 403)']);
ok('ps(5): 403 のとき、変更の呼び出しを 1 件もしない', mutations(r.calls), []);
ok('ps(5): 403 のとき、注記のコメントを Issue ごとに 1 件ずつ書く(12 と 13)', comments(r.calls).map((c) => c.args.find((a) => /issues\/\d+\/comments$/.test(a)).match(/issues\/(\d+)/)[1]).sort(), ['12', '13']);
ok('ps(5): 注記の本文は、印の行で始まる', comments(r.calls)[0]?.body.body.startsWith(NOTE), true);
ok('ps(5): 注記の本文に、内部の ID(L-0 で始まる語)を書かない', /L-0\d/.test(comments(r.calls)[0]?.body.body ?? 'x'), false);
r = tool('denied-noted', fixture({ project: 'denied', comments: { 12: [`前の注記\n${NOTE}\n`] } }), ['set', 'In Progress', '12']);
ok('ps(5): 印の行の在るコメントが既に在れば、注記を書かない(1 回だけ)', [r.code, comments(r.calls).length], [0, 0]);
r = tool('denied-grant', fixture({ project: 'denied' }), ['set', 'In Review', '12']);
ok('ps(5): 注記の本文は読めた後で書く(コメントの一覧を読んでから POST する)', r.calls.findIndex((c) => c.args.includes('--paginate')) < r.calls.findIndex((c) => c.args.includes('POST') && c.args.some((a) => /comments$/.test(a))), true);
// (6) 読めない Project(null)も、権限の理由で出す。注記は書く
r = tool('nullproject', fixture({ project: null }), ['set', 'In Review', '13']);
ok('ps(6): Project が読めない(null)は、理由を出し、0 で終わる', [r.code, r.out.split('\n')[0]], [0, 'project-status: #13 In Review: skipped: the App has no access to the project (project not found or not readable)']);
ok('ps(6): Project が読めない(null)のときも、注記を Issue に書く(権限が無い可能性がある)', comments(r.calls).length, 1);
// (7) 選択肢が無い Status・Status の field が無いとき: 理由を出し、変更はしない
r = tool('nooption', fixture(), ['set', 'Doing', '12']);
ok('ps(7): 選択肢が無ければ、理由(在る選択肢の一覧つき)を出し、0 で終わる', [r.code, r.out.startsWith('project-status: #12 Doing: skipped: Status の選択肢 Doing が無い'), mutations(r.calls)], [0, true, []]);
r = tool('nostatus', fixture({ project: { id: 'PVT_1', fields: { nodes: [{ id: 'x', name: 'Size', options: [] }] } } }), ['set', 'In Review', '13']);
ok('ps(7): Status の field が無ければ、理由を出し、0 で終わる', [r.code, r.out, mutations(r.calls)], [0, 'project-status: #13 In Review: skipped: プロジェクトに Status の単一選択が無い', []]);
// (8) 権限以外の失敗(gh の失敗)は、理由を出すが注記は書かない
r = tool('ghfail', fixture({ gh_error: 'gh: boom (HTTP 500)' }), ['set', 'In Review', '13']);
ok('ps(8): 権限以外の失敗は、理由を出し、0 で終わる', [r.code, r.out], [0, 'project-status: #13 In Review: skipped: gh: boom (HTTP 500)']);
ok('ps(8): 権限以外の失敗では、注記を書かない', comments(r.calls).length, 0);
// (9) 使い方: 状態が無い・番号が数でない・リポジトリが無い は 2。番号が 0 件は、何もせず 0
ok('ps(9): 状態が無ければ 2 で終わる', tool('usage1', fixture(), ['set'], {}).code, 2);
ok('ps(9): 番号が数でなければ 2 で終わり、gh を呼ばない', (() => { const x = tool('usage2', fixture(), ['set', 'In Review', 'x']); return [x.code, x.calls.length]; })(), [2, 0]);
ok('ps(9): リポジトリが分からなければ 2 で終わる', (() => { const x = tool('usage3', fixture(), ['set', 'In Review', '13'], { GH_REPO: '', GITHUB_REPOSITORY: '' }); return x.code; })(), 2);
ok('ps(9): 番号が 0 件なら、gh を呼ばずに 0 で終わる', (() => { const x = tool('usage4', fixture(), ['set', 'In Review']); return [x.code, x.calls.length]; })(), [0, 0]);

// (10) agent.yml: 段の置き場所・条件・token(python3 と PyYAML で読む。検査だけに使う)
const yaml = JSON.parse(execFileSync('python3', ['-c', `
import sys, yaml, json
d = yaml.safe_load(open(sys.argv[1], encoding="utf-8"))
if True in d: d["on"] = d.pop(True)
print(json.dumps(d, ensure_ascii=False))`, WF], { encoding: 'utf8' }));
const steps = (job) => yaml.jobs[job].steps;
const idx = (job, re) => steps(job).findIndex((s) => re.test(s.name ?? ''));
const claim = idx('start', /running を付ける/);
const inProgress = idx('start', /^Project の Status を In Progress/);
ok('wf(10): start に、Project の In Progress の段が在り、running を付ける段の直後に在る', [claim >= 0, inProgress === claim + 1], [true, true]);
ok('wf(10): In Progress の段は、確認の成功の後で、claim の出力が空でない時だけ動く', steps('start')[inProgress]?.if, "success() && steps.claim.outputs.claimed != '[]'");
const reportWrite = idx('report', /^結果を Issue に書き/);
const inReview = idx('report', /^Project の Status を In Review/);
const retry = idx('report', /^自動の再試行を起動する/);
ok('wf(10): report の In Review の段は、結果を書く段の後で、自動の再試行の起動より前に在る', [reportWrite >= 0, inReview > reportWrite, retry > inReview], [true, true, true]);
ok('wf(10): In Review の段は、結果を書く段が落ちたときや取り消されたときも動く(always())', steps('report')[inReview]?.if, 'always()');
ok('wf(10): In Progress の段は、道具を In Progress で呼ぶ', /tools\/project-status[.]mjs set "In Progress"/.test(steps('start')[inProgress]?.run ?? ''), true);
ok('wf(10): In Review の段は、道具を In Review で呼ぶ', /tools\/project-status[.]mjs set "In Review"/.test(steps('report')[inReview]?.run ?? ''), true);
const appEnv = (s) => ['APP_CLIENT_ID', 'APP_PRIVATE_KEY', 'APP_INSTALLATION_ID'].every((k) => k in (s?.env ?? {}));
ok('wf(10): 2 つの段は App の鍵(GH_APP_CLIENT_ID・GH_APP_PEM・GH_APP_INSTALLATION_ID)を、その段だけに渡す', [appEnv(steps('start')[inProgress]), appEnv(steps('report')[inReview])], [true, true]);
ok('wf(10): 2 つの段は、GH_TOKEN(GITHUB_TOKEN)を使わない(Project は App の token で読み書きする)', [JSON.stringify(steps('start')[inProgress]?.env ?? {}).includes('github.token'), JSON.stringify(steps('report')[inReview]?.env ?? {}).includes('github.token')], [false, false]);
ok('wf(10): 2 つの段は、continue-on-error を持たない(失敗は段の中で理由を出して 0 で終わる)', [steps('start')[inProgress], steps('report')[inReview]].map((s) => 'continue-on-error' in (s ?? {})), [false, false]);
ok('wf(10): 2 つの段は、道具が落ちたときも exit 0 で終わる(段の最後が exit 0)', [steps('start')[inProgress], steps('report')[inReview]].map((s) => /(^|\n)\s*exit 0\s*$/.test(s.run ?? '')), [true, true]);
// (11) 作業者の段(opencode)と作業ジョブに、App の鍵・Project の token・道具を渡さない
ok('wf(11): 作業ジョブの段に、Project の道具が無い', steps('work').filter((s) => /project-status/.test(JSON.stringify(s))).map((s) => s.name), []);
ok('wf(11): 作業者の段の env に、secrets・github.token が無い', JSON.stringify(steps('work').find((s) => s.name === '作業者を 1 回動かす')?.env ?? {}).match(/secrets|github[.]token/) === null, true);
ok('wf(11): 作業ジョブの env に、secrets・github.token が無い', JSON.stringify(yaml.jobs.work.env ?? {}).match(/secrets|github[.]token/) === null, true);
// (12) 既存の段の並び(段の名前の順)は変わらず、足した段だけが増える
const names = (job) => steps(job).map((s) => s.name);
ok('wf(12): start の段の名前は、旧版(running を付ける段の後に In Progress の段を 1 つ足しただけ)', names('start').filter((n) => !/^Project の Status/.test(n)).join('|'), [
  'リポジトリを取得する(資格は残さない)',
  '起動してよい物に running を付ける(GITHUB_TOKEN で。失敗・再試行の判定は、もう一度行う)',
].join('|'));
ok('wf(12): report の段の名前は、既存の段に In Review の段を 1 つ足しただけ', names('report').filter((n) => !/^Project の Status/.test(n)).length, names('report').length - 1);
ok('wf(12): start・report に、Project の段は 1 つずつ', [names('start').filter((n) => /^Project の Status/.test(n)).length, names('report').filter((n) => /^Project の Status/.test(n)).length], [1, 1]);

fs.rmSync(TMP, { recursive: true, force: true });
process.exit(failed ? 1 : 0);
