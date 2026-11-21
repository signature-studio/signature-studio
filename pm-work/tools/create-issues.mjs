#!/usr/bin/env node
// pm-work/tools/create-issues.mjs [--apply [--resume]] [--drafts DIR] [--milestones FILE] [--state FILE] : 下書き(pm-work/issue-drafts/[0-9]*.md)と pm-work/milestones.md から、GitHub に Issue・マイルストーン・ラベル・プロジェクトの設定・Issue field の値を作る。PM が依頼者の承認の後に実行する道具。
// 既定は dry-run: 検査だけをして、実行する呼び出しの一覧を出す(gh を呼ばない)。--apply で実行する。
// 順序: 検査(ローカル) → 事前の検査(--apply のみ。gh の読みだけ: プロジェクトの Size、組織の Issue field の Priority、Issue Type) → ラベル → マイルストーン → Issue を依存の順に作る(本文の #{{NNN}} はそのまま) → 本文とマイルストーンの説明の #{{NNN}} を実番号に直す → 親子・blocked by → プロジェクトに追加して Size → Issue field の Priority を Issue ごとに設定 → 全ての Issue から pm-hold を外す(最後)。
// 状態は --state(既定: リポジトリ直下の .tmp-create-state.json)に 1 手ごとに書く。途中で止まったら --apply --resume で続ける。作った Issue は作り直さない。
// 読みと書きは gh api だけを使う(gh issue create は使わない。Issue Type と関係の API を使うため)。リポジトリは GH_REPO(無ければ GITHUB_REPOSITORY)。
// 終了コード: 0 = 完了、1 = 検査か実行が失敗(理由を標準エラーに出す)、2 = 使い方の誤り。
import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const TYPES = ['Task', 'Feature', 'Bug'];
const SIZES = ['XS', 'S', 'M', 'L', 'XL'];
const PRIORITIES = ['Urgent', 'High', 'Medium', 'Low'];
// Priority は組織の Issue field で、プロジェクトの単一選択ではない(プロジェクトの同名のフィールドは選択肢が空)
const FIELDS = { Size: SIZES };
const KEYS = ['draft', 'title', 'type', 'milestone', 'parent', 'blocked_by', 'labels', 'size', 'priority', 'project'];
const RESERVED = ['pm-hold', 'running', 'failed', 'needs-owner', 'task', 'epic', ...TYPES, ...SIZES, ...PRIORITIES].map((s) => s.toLowerCase());
const REF = /#\{\{(\d{3})\}\}/g;
const HAS_REF = /#\{\{\d{3}\}\}/;
const FIELD_IN_BODY = /^\s*[#>*-]*\s*(規模|優先度|優先|Size|Priority|Type)\s*[:：]/m;
const Q_PROJECT = 'query($login: String!, $number: Int!) { organization(login: $login) { projectV2(number: $number) { id fields(first: 50) { nodes { ... on ProjectV2SingleSelectField { id name options { id name } } } } } } }';
const M_ADD = 'mutation($project: ID!, $content: ID!) { addProjectV2ItemById(input: { projectId: $project, contentId: $content }) { item { id } } }';
const M_SET = 'mutation($project: ID!, $item: ID!, $field: ID!, $option: String!) { updateProjectV2ItemFieldValue(input: { projectId: $project, itemId: $item, fieldId: $field, value: { singleSelectOptionId: $option } }) { projectV2Item { id } } }';

const argv = process.argv.slice(2);
const usage = (msg) => {
  process.stderr.write(`create-issues: ${msg}\nusage: create-issues.mjs [--apply [--resume]] [--drafts DIR] [--milestones FILE] [--state FILE]\n`);
  process.exit(2);
};
for (let i = 0; i < argv.length; i++) {
  if (['--apply', '--resume'].includes(argv[i])) continue;
  if (['--drafts', '--milestones', '--state'].includes(argv[i]) && argv[i + 1] !== undefined) { i++; continue; }
  usage(`不明な引数: ${argv[i]}`);
}
const apply = argv.includes('--apply');
const resume = argv.includes('--resume');
if (resume && !apply) usage('--resume は --apply と一緒に使う');
const arg = (name) => { const i = argv.indexOf(name); return i < 0 ? null : argv[i + 1]; };
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const draftsDir = resolve(arg('--drafts') ?? join(root, 'pm-work', 'issue-drafts'));
const milestonesFile = resolve(arg('--milestones') ?? join(root, 'pm-work', 'milestones.md'));
const stateFile = resolve(arg('--state') ?? join(root, '.tmp-create-state.json'));
const repo = process.env.GH_REPO || process.env.GITHUB_REPOSITORY || '';

const die = (lines) => {
  for (const l of lines) process.stderr.write(`create-issues: ${l}\n`);
  process.exit(1);
};
const warn = (l) => process.stderr.write(`create-issues: 警告: ${l}\n`);

function parseValue(v) {
  try { return JSON.parse(v); } catch { /* 引用符の無い値 */ }
  if (v.startsWith('[') && v.endsWith(']')) return v.slice(1, -1).split(',').map((s) => s.trim().replace(/^["']|["']$/g, '')).filter(Boolean);
  return v.replace(/^["']|["']$/g, '');
}

function readDrafts(dir) {
  if (!existsSync(dir)) die([`下書きの場所が無い: ${dir}`]);
  return readdirSync(dir).filter((f) => /^[0-9].*\.md$/.test(f)).sort().map((file) => {
    const text = readFileSync(join(dir, file), 'utf8').replace(/\r\n/g, '\n');
    const m = text.match(/^---\n([\s\S]*?)\n---\n?([\s\S]*)$/);
    if (!m) return { file, error: 'front matter が無い' };
    const fm = {};
    for (const line of m[1].split('\n')) {
      if (!line.trim()) continue;
      const kv = line.match(/^([a-z_]+):\s*(.*)$/);
      if (!kv) return { file, error: `front matter の行を読めない: ${line}` };
      fm[kv[1]] = parseValue(kv[2].trim());
    }
    return { file, fm, body: m[2].replace(/^\n+/, '').trimEnd() };
  });
}

function readMilestones(file) {
  if (!existsSync(file)) die([`マイルストーンの一覧が無い: ${file}`]);
  const lines = readFileSync(file, 'utf8').replace(/\r\n/g, '\n').split('\n');
  const out = new Map();
  for (const l of lines) {
    const m = l.match(/^\|\s*M\d+\s*\|\s*([^|]+?)\s*\|/);
    if (m) out.set(m[1], '');
  }
  let heading = null;
  for (const l of lines) {
    const h = l.match(/^###\s+(.+?)\s*$/);
    if (h) { heading = h[1]; continue; }
    const d = l.match(/^-\s*説明:\s*(.+?)\s*$/);
    if (d && heading && out.has(heading)) out.set(heading, d[1]);
  }
  return out;
}

function readColors(dir) {
  const out = new Map();
  const file = join(dir, 'LABELS.md');
  if (!existsSync(file)) return out;
  for (const l of readFileSync(file, 'utf8').split('\n')) {
    const m = l.match(/^\|\s*`([^`]+)`\s*\|\s*`([0-9a-fA-F]{6})`/);
    if (m) out.set(m[1], m[2]);
  }
  return out;
}

function check(raw, milestones) {
  const errors = [];
  const drafts = [];
  const num = (x) => (x === null || x === undefined || x === '' ? null : String(x).trim().padStart(3, '0'));
  for (const r of raw) {
    if (r.error) { errors.push(`${r.file}: ${r.error}`); continue; }
    const { file, fm, body } = r;
    const err = (s) => errors.push(`${file}: ${s}`);
    const missing = KEYS.filter((k) => !(k in fm));
    if (missing.length) { err(`front matter に無い key: ${missing.join(', ')}`); continue; }
    const d = {
      file, n: num(fm.draft), title: fm.title, type: fm.type, milestone: fm.milestone, parent: num(fm.parent),
      blocked_by: Array.isArray(fm.blocked_by) ? fm.blocked_by.map(num) : [], labels: Array.isArray(fm.labels) ? fm.labels : [],
      size: fm.size, priority: fm.priority, project: fm.project, body,
    };
    if (!/^\d{3}$/.test(d.n ?? '')) err(`draft は 3 桁の番号(実際: ${fm.draft})`);
    else if (!file.startsWith(`${d.n}-`)) err(`draft の ${d.n} が、ファイル名の番号と違う`);
    if (typeof d.title !== 'string' || !d.title.trim()) err('title が無い');
    else if (d.title.includes('{{')) err('title に {{ を書かない');
    if (!TYPES.includes(d.type)) err(`type は ${TYPES.join('・')} のいずれか(実際: ${JSON.stringify(d.type)})`);
    if (!SIZES.includes(d.size)) err(`size は ${SIZES.join('・')} のいずれか(実際: ${JSON.stringify(d.size)})`);
    if (!PRIORITIES.includes(d.priority)) err(`priority は ${PRIORITIES.join('・')} のいずれか(実際: ${JSON.stringify(d.priority)})`);
    if (typeof d.project !== 'string' || !/^[A-Za-z0-9-]+\/\d+$/.test(d.project)) err(`project は 組織/番号 の形(実際: ${JSON.stringify(d.project)})`);
    if (d.milestone !== null && !milestones.has(d.milestone)) err(`milestone が milestones.md に無い: ${JSON.stringify(d.milestone)}`);
    if (!Array.isArray(fm.labels) || fm.labels.some((l) => typeof l !== 'string')) err('labels は文字列の配列');
    else for (const l of d.labels) if (RESERVED.includes(l.toLowerCase()) || /^priority-/i.test(l)) err(`labels に入れない: ${l}(Type・Size・Priority と運用のラベルは入れない)`);
    if (!Array.isArray(fm.blocked_by)) err('blocked_by は配列');
    if (FIELD_IN_BODY.test(body)) err('本文に 規模・優先度・Type を書かない(Size・Priority・Type で指定する)');
    if (body.replace(/#\{\{\d{3}\}\}/g, '').includes('{{')) err('本文に #{{NNN}} の形以外の {{ がある');
    drafts.push(d);
  }
  const seen = new Set();
  for (const d of drafts) {
    if (seen.has(d.n)) errors.push(`${d.file}: draft の番号 ${d.n} が重複`);
    seen.add(d.n);
  }
  for (const d of drafts) {
    const err = (s) => errors.push(`${d.file}: ${s}`);
    if (d.parent !== null && (!seen.has(d.parent) || d.parent === d.n)) err(`parent ${d.parent} の下書きが無い`);
    for (const b of d.blocked_by) if (!seen.has(b) || b === d.n) err(`blocked_by ${b} の下書きが無い(自分自身は不可)`);
    if (new Set(d.blocked_by).size !== d.blocked_by.length) err('blocked_by に重複がある');
    for (const [, ref] of d.body.matchAll(REF)) if (!seen.has(ref)) err(`本文の #{{${ref}}} に当たる下書きが無い`);
  }
  for (const [title, description] of milestones) {
    for (const [, ref] of description.matchAll(REF)) if (!seen.has(ref)) errors.push(`マイルストーン ${title}: 説明の #{{${ref}}} に当たる下書きが無い`);
    if (description.replace(/#\{\{\d{3}\}\}/g, '').includes('{{')) errors.push(`マイルストーン ${title}: 説明の #{{NNN}} の形以外の {{ がある`);
  }
  const projects = new Set(drafts.map((d) => d.project));
  if (projects.size > 1) errors.push(`project が下書きで違う: ${[...projects].join(', ')}`);
  return { errors, drafts };
}

function order(drafts) {
  const byN = new Map(drafts.map((d) => [d.n, d]));
  const deps = (d) => [d.parent, ...d.blocked_by].filter((x) => x !== null);
  const placed = new Set();
  const out = [];
  let left = [...byN.keys()].sort();
  while (left.length) {
    const next = left.find((n) => deps(byN.get(n)).every((x) => placed.has(x)));
    if (next === undefined) return { cycle: left };
    placed.add(next);
    out.push(next);
    left = left.filter((n) => n !== next);
  }
  return { out };
}

function plan({ drafts, sorted, byN, milestones, labels, project }) {
  const L = [];
  const say = (s) => L.push(s);
  say('dry-run: 何も書かない(gh を呼ばない)。実行するには --apply(途中から続けるには --apply --resume)。');
  say(`検査: 下書き ${drafts.length} 件、マイルストーン ${milestones.size} 件。エラーは無い。`);
  say(`1. 事前の検査(読みだけ): プロジェクト ${project} の Size の選択肢、組織の Issue field の Priority(${PRIORITIES.join('・')})、Issue Type(${[...new Set(drafts.map((d) => d.type))].join(', ')})`);
  say(`2. ラベルを作る(無い物だけ): ${labels.join(', ')}`);
  say(`3. マイルストーンを作る(無い物だけ): ${[...milestones.keys()].join(' | ')}`);
  say('4. Issue を作る(依存の順。本文の #{{NNN}} はそのまま。labels に pm-hold を付ける):');
  for (const n of sorted) {
    const d = byN.get(n);
    say(`   作る ${n} ${d.type} [${['pm-hold', ...d.labels].join(',')}] ${d.milestone ?? '(マイルストーン無し)'} ${d.title}`);
  }
  const bodies = sorted.filter((n) => HAS_REF.test(byN.get(n).body));
  const descs = [...milestones].filter(([, desc]) => HAS_REF.test(desc)).map(([t]) => t);
  say(`5. #{{NNN}} を実番号に直す(PATCH): 本文 ${bodies.length} 件(${bodies.join(', ') || 'なし'})、マイルストーンの説明 ${descs.length} 件`);
  say('6. 親子(sub-issue)と blocked by を付ける:');
  for (const n of sorted) {
    const d = byN.get(n);
    if (d.parent !== null) say(`   親子 ${d.parent} -> ${n}`);
    for (const b of d.blocked_by) say(`   blocked by ${n} <- ${b}`);
  }
  say(`7. プロジェクトに追加し、Size を設定: ${sorted.length} 件`);
  for (const n of sorted) say(`   追加 ${n} Size=${byN.get(n).size}`);
  say(`8. Priority を組織の Issue field に設定(Issue ごと): ${sorted.length} 件`);
  for (const n of sorted) say(`   設定 ${n} Priority=${byN.get(n).priority}`);
  say(`9. 全ての Issue から pm-hold を外す(最後): ${sorted.length} 件`);
  return L.join('\n');
}

function gh(args, input) {
  try {
    return execFileSync('gh', args, { encoding: 'utf8', input, stdio: [input === undefined ? 'ignore' : 'pipe', 'pipe', 'pipe'], maxBuffer: 64 * 1024 * 1024 });
  } catch (e) {
    throw new Error(`gh ${args.slice(0, 3).join(' ')} が失敗: ${String(e.stderr || e.message).trim()}`);
  }
}

function api(method, path, body) {
  const args = ['api', '-X', method, path];
  if (body !== undefined) args.push('--input', '-');
  const out = gh(args, body === undefined ? undefined : JSON.stringify(body));
  return out.trim() ? JSON.parse(out) : null;
}

function listAll(path) {
  const all = [];
  for (let page = 1; ; page++) {
    const items = api('GET', `${path}${path.includes('?') ? '&' : '?'}per_page=100&page=${page}`);
    all.push(...items);
    if (items.length < 100) return all;
  }
}

function gql(query, variables) {
  const r = api('POST', 'graphql', { query, variables });
  if (r.errors?.length) throw new Error(`GraphQL: ${r.errors.map((e) => e.message).join('; ')}`);
  return r.data;
}

function precheck(project, drafts) {
  const [login, number] = project.split('/');
  const p = gql(Q_PROJECT, { login, number: Number(number) }).organization?.projectV2;
  if (!p) die([`プロジェクト ${project} が読めない(無い、または App に読む権限が無い)`]);
  const problems = [];
  const fields = {};
  for (const [name, want] of Object.entries(FIELDS)) {
    const f = p.fields.nodes.find((x) => x?.name === name && x.options);
    if (!f) { problems.push(`プロジェクトに単一選択のフィールド ${name} が無い`); continue; }
    const options = new Map(f.options.map((o) => [o.name, o.id]));
    const missing = want.filter((w) => !options.has(w));
    if (missing.length) problems.push(`${name} の選択肢が無い: ${missing.join(', ')}`);
    fields[name] = { id: f.id, options };
  }
  const org = repo.split('/')[0];
  let issueFields = null;
  try {
    issueFields = api('GET', `orgs/${org}/issue-fields`) ?? [];
  } catch (e) {
    problems.push(/\b403\b/.test(e.message)
      ? `組織 ${org} の Issue Fields が読めない(403)。App に、組織の Issue Fields の読み取りの権限が要る`
      : `組織 ${org} の Issue Fields が読めない: ${e.message}`);
  }
  let priorityFieldId = null;
  if (issueFields) {
    const f = issueFields.find((x) => x?.name === 'Priority');
    if (!f) problems.push(`組織 ${org} に Issue field の Priority が無い`);
    else {
      if (f.data_type !== 'single_select') problems.push(`Issue field の Priority は単一選択でない(実際: ${f.data_type})`);
      const have = new Set((f.options ?? []).map((o) => o.name));
      const missing = PRIORITIES.filter((w) => !have.has(w));
      if (missing.length) problems.push(`Priority の選択肢が無い: ${missing.join(', ')}`);
      priorityFieldId = f.id;
    }
  }
  if (problems.length) die(problems);
  let types = null;
  try {
    types = (api('GET', `orgs/${org}/issue-types`) ?? []).map((t) => t.name);
  } catch (e) {
    warn(`Issue Type の一覧を読めない(作成時の応答で確かめる): ${e.message}`);
  }
  if (types) {
    const missing = [...new Set(drafts.map((d) => d.type))].filter((t) => !types.includes(t));
    if (missing.length) die([`Issue Type が無い: ${missing.join(', ')}`]);
  }
  return { projectId: p.id, fields, priorityFieldId };
}

function run({ drafts, byN, sorted, milestones, colors, project }) {
  if (!repo) die(['GH_REPO が無い(例: GH_REPO=<owner>/<repo>)']);
  let st;
  if (existsSync(stateFile)) {
    if (!resume) die([`状態ファイルが在る(前回の途中): ${stateFile}。続けるなら --apply --resume、作り直すなら削除してから`]);
    st = JSON.parse(readFileSync(stateFile, 'utf8'));
    if (st.repo !== repo || st.project !== project) die([`状態ファイルの repo・project が違う(${st.repo}・${st.project})`]);
  } else {
    if (resume) die([`--resume だが状態ファイルが無い: ${stateFile}`]);
    st = { repo, project, drafts: {}, steps: {} };
  }
  const save = () => {
    const tmp = `${stateFile}.next`;
    writeFileSync(tmp, `${JSON.stringify(st, null, 1)}\n`);
    renameSync(tmp, stateFile);
  };
  const done = (k) => st.steps[k] === true;
  const mark = (k) => { st.steps[k] = true; save(); };
  const fill = (text) => text.replace(REF, (_, k) => `#${st.drafts[k].number}`);
  const pre = precheck(project, drafts);
  save();

  const have = new Set(listAll(`repos/${repo}/labels`).map((l) => l.name));
  for (const name of ['pm-hold', 'running', 'failed', ...drafts.flatMap((d) => d.labels)]) {
    if (have.has(name)) continue;
    have.add(name);
    api('POST', `repos/${repo}/labels`, { name, color: colors.get(name) ?? 'ededed' });
  }
  const msNumber = new Map(listAll(`repos/${repo}/milestones?state=all`).map((m) => [m.title, m.number]));
  for (const [title, description] of milestones) {
    if (msNumber.has(title)) continue;
    msNumber.set(title, api('POST', `repos/${repo}/milestones`, { title, description }).number);
  }

  for (const n of sorted) {
    if (st.drafts[n]) continue;
    const d = byN.get(n);
    const body = { title: d.title, body: d.body, type: d.type, labels: ['pm-hold', ...d.labels] };
    if (d.milestone !== null) body.milestone = msNumber.get(d.milestone);
    const r = api('POST', `repos/${repo}/issues`, body);
    st.drafts[n] = { number: r.number, id: r.id, node: r.node_id, type: r.type?.name ?? null };
    save();
    console.log(`作った ${n} -> #${r.number}`);
    if (st.drafts[n].type !== d.type) die([`#${r.number}(下書き ${n})の Issue Type が ${d.type} でない(実際: ${st.drafts[n].type ?? '無し'})。状態ファイルに記録した。直してから --apply --resume`]);
  }

  for (const n of sorted) {
    const d = byN.get(n);
    if (!HAS_REF.test(d.body) || done(`body:${n}`)) continue;
    api('PATCH', `repos/${repo}/issues/${st.drafts[n].number}`, { body: fill(d.body) });
    mark(`body:${n}`);
  }
  for (const [title, description] of milestones) {
    if (!HAS_REF.test(description) || done(`ms:${title}`)) continue;
    api('PATCH', `repos/${repo}/milestones/${msNumber.get(title)}`, { description: fill(description) });
    mark(`ms:${title}`);
  }

  for (const n of sorted) {
    const d = byN.get(n);
    if (d.parent === null || done(`sub:${n}`)) continue;
    api('POST', `repos/${repo}/issues/${st.drafts[d.parent].number}/sub_issues`, { sub_issue_id: st.drafts[n].id });
    mark(`sub:${n}`);
  }
  for (const n of sorted) {
    for (const b of byN.get(n).blocked_by) {
      const k = `blk:${n}:${b}`;
      if (done(k)) continue;
      api('POST', `repos/${repo}/issues/${st.drafts[n].number}/dependencies/blocked_by`, { issue_id: st.drafts[b].id });
      mark(k);
    }
  }

  for (const n of sorted) {
    const d = byN.get(n);
    const rec = st.drafts[n];
    if (!rec.item) {
      rec.item = gql(M_ADD, { project: pre.projectId, content: rec.node }).addProjectV2ItemById.item.id;
      save();
    }
    for (const field of Object.keys(FIELDS)) {
      const k = `${field.toLowerCase()}:${n}`;
      if (done(k)) continue;
      gql(M_SET, { project: pre.projectId, item: rec.item, field: pre.fields[field].id, option: pre.fields[field].options.get(d[field.toLowerCase()]) });
      mark(k);
    }
  }

  for (const n of sorted) {
    const k = `issue-priority:${n}`;
    if (done(k)) continue;
    api('POST', `repos/${repo}/issues/${st.drafts[n].number}/issue-field-values`, { issue_field_values: [{ field_id: pre.priorityFieldId, value: byN.get(n).priority }] });
    mark(k);
  }

  for (const n of sorted) {
    if (done(`hold:${n}`)) continue;
    api('DELETE', `repos/${repo}/issues/${st.drafts[n].number}/labels/pm-hold`);
    mark(`hold:${n}`);
  }

  for (const n of sorted) console.log(`${n} -> #${st.drafts[n].number} ${st.drafts[n].type} ${byN.get(n).title}`);
  console.log(`完了。状態ファイル: ${stateFile}`);
}

const raw = readDrafts(draftsDir);
const milestones = readMilestones(milestonesFile);
const colors = readColors(draftsDir);
const { errors, drafts } = check(raw, milestones);
if (drafts.length === 0 && errors.length === 0) errors.push(`下書きが 1 件も無い: ${draftsDir}`);
if (errors.length) die(errors);
const byN = new Map(drafts.map((d) => [d.n, d]));
const ordered = order(drafts);
if (ordered.cycle) die([`依存(親・blocked by)に循環がある: ${ordered.cycle.join(', ')}`]);
const sorted = ordered.out;
const project = drafts[0].project;
const labels = [...new Set(['pm-hold', 'running', 'failed', ...drafts.flatMap((d) => d.labels)])];

if (!apply) {
  console.log(plan({ drafts, sorted, byN, milestones, labels, project }));
} else {
  try {
    run({ drafts, byN, sorted, milestones, colors, project });
  } catch (e) {
    die([e.message, ...(existsSync(stateFile) ? [`途中で止まった。続けるには --apply --resume(状態: ${stateFile})`] : [])]);
  }
}
