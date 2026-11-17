#!/usr/bin/env bash
# usage: scripts/check-consistency.sh        (no argument, no environment variable; reads the git index only, writes nothing)
# Prints the inconsistencies a machine can see in the current documents: first 5 rows and the count. Last line: "consistency: N finding(s)". Exit 0 when it ran;
# exit 1 when git cannot read the index (no count line: scripts/preflight.sh reports "did not run"). N >= 1 is a WARN in preflight.
# The files and their text are the git index (tracked files with their staged content, i.e. what the next commit holds). A file that is only on disk
# (untracked, or ignored by .gitignore) does not exist here, so a path to it is a finding; an edit that is not staged is not read (git add it first).
# a  a backticked path that does not exist. Its first segment must be a top-level entry of the repository: a tracked one, or a directory that .gitignore names
#    (so `~/...`, `/...`, `docs/...` checks are limited to repository paths). A commit-qualified path (<commit>:<path> or <commit>^:<path>) in any word
#    of a code span must be in that commit (git cat-file)
# g  docs/*.md, docs/core/*.md or docs/ops/*.md that docs/index.md does not name; tests/harness/t*.sh that tests/harness/README.md does not name;
#    a scripts/ file that no current document names
# c  AGENTS.md over 140 lines; prompts/worker-rules.md over 60 lines
# d  a decision from 0386 on that says 入れない・捨てる・書き直し・削除 cites neither an owner message (msg_…) nor an old source (archive/… or docs/archive/…)
# e  docs/requests/NNNN-*.md (TEMPLATE.md is not one): an internal term (INTERNAL_* below) in the text the client reads: every line but the
#    自分で試した記録 section, the code blocks and the `inline code`
# Not checked, on purpose:
#   the 自分で試した記録 section of a request, its code and `inline code` (commands and file names are quoted, not terms); a capitalized Worker (Cloudflare's product name)
#   old material and outputs: docs/archive/, archive/, evidence/, patches/, tools/, lift/ (imported measurement material), artifacts/, binaries/, lift/out*
#   a command: a backticked text with a space is checked only by its first word when that word is a path with "/" (`oc-msg.sh send …` is a command)
#   a bare file name (no "/") that the repository lacks: it may name a file in another place (~/work/handover, ~/.agents, an old tree)
#   a sentence (split at 。 and at a table cell |) that names one of those places (~/work/handover, ~/.agents, 旧ツリー, docs/archive)
#   a sentence that says the path is an output of a build or a step to create (PLANNED words: 作る・作成・出力・ビルド・退避 …). Only the prose counts:
#   not a code span, not the path's own name, and not a word inside a katakana or English word (ダウンロード is not ロード)
#   text with < > * $ { } ( ) [ ] … | \ # ~ = ? % or a leading - or /
#   docs/ROADMAP.md rows whose status is todo (their files are planned outputs; the done rule of ROADMAP checks them)
set -o pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd) || { echo "check-consistency: cannot resolve the repository root" >&2; exit 0; }

node - "$root" << 'JS'
const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

const root = process.argv[2];
process.chdir(root);
const rows = [];

function add(kind, where, tok, why) {
  rows.push([kind, where, tok, why]);
}

function git(args, input) {
  const p = spawnSync("git", args, { input, maxBuffer: 1 << 30 });
  if (p.status !== 0) {
    console.error("check-consistency: git " + args.join(" ") + " failed: " + String(p.stderr).trim());
    process.exit(1);
  }
  return p.stdout;
}

// The files are the index (git ls-files): the tracked files with their staged content, i.e. what the next commit holds.
// A file that is only on disk (untracked, or ignored by .gitignore) is not in the index, so a path to it is a finding.
const files = new Set(
  git(["ls-files", "-z"]).toString("utf8").split("\0").filter((p) => p).map((p) => path.posix.normalize(p))
);
const dirname = (p) => {
  const i = p.lastIndexOf("/");
  return i < 0 ? "" : p.slice(0, i);
};
const dirs = new Set();
for (const p of files) {
  let d = dirname(p);
  while (d && !dirs.has(d)) {
    dirs.add(d);
    d = dirname(d);
  }
}
const top = new Set([...files].map((p) => p.split("/")[0]));
const exists = (p) => files.has(p) || dirs.has(p);
const base = (p) => p.split("/").pop();

function readTexts(paths) {
  // one git process reads every text from the index (not from the working tree)
  const data = git(["cat-file", "--batch"], Buffer.from(paths.map((p) => ":" + p + "\n").join(""), "utf8"));
  const texts = {};
  let pos = 0;
  for (const p of paths) {
    const end = data.indexOf(10, pos);
    const head = data.slice(pos, end).toString("utf8").split(" ");
    const last = head[head.length - 1];
    pos = end + 1;
    if (last === "missing") {
      texts[p] = "";
      continue;
    }
    const size = parseInt(last, 10);
    texts[p] = data.slice(pos, pos + size).toString("utf8");
    pos += size + 1;
  }
  return texts;
}

// a top-level directory that .gitignore names (artifacts/, binaries/) is a repository top-level entry too: a path into it is checked
const GITIGNORE = files.has(".gitignore") ? readTexts([".gitignore"])[".gitignore"] : "";
for (const line of GITIGNORE.split("\n")) {
  const name = line.trim().replace(/^\/+/, "");
  if (name.endsWith("/") && !name.startsWith("#") && !name.startsWith("!") && name.length > 1 && !name.slice(0, -1).includes("/")) {
    top.add(name.slice(0, -1));
  }
}

const EXCLUDED = ["docs/archive/", "archive/", "evidence/", "patches/", "tools/", "lift/", "artifacts/", "binaries/"];
const OUTSIDE = ["~/work/handover", "~/.agents", "旧ツリー", "docs/archive"];
// words that mark a path as a build output or a step to create, not a file that must exist now
const PLANNED = ["ビルド", "作る", "作成", "出力", "ロールバック", "退避", "置く", "前に", "ls", "ロード", "build", "出来"];
const isAscii = (w) => /^[\x00-\x7f]*$/.test(w);
const escapeRe = (w) => w.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
// a word of its own: not the end of a katakana word (ダウンロード contains ロード) nor a part of an English word (rebuild, builder)
const PLANNED_RE = PLANNED.map(
  (w) => new RegExp("(?<![A-Za-z\\u30a0-\\u30ff])" + escapeRe(w) + (isAscii(w) ? "(?![A-Za-z])" : ""))
);
// code spans and link targets: a planned word counts only in the prose around them, not in the path's own name
const CODE_G = /`[^`\n]*`|\]\([^)\s]*\)/g;
const BAD = /[<>*$…{}()\[\]|\\#~=?%]/;
const TICK_G = /`([^`\n]+)`/g;
const LINK_G = /\]\(([^)\s]+)\)/g;
const SUFFIX = /:\d[\d,\-]*$/;

function isDoc(p) {
  if (EXCLUDED.some((e) => p.startsWith(e))) return false;
  if (["AGENTS.md", "README.md", "tests/harness/README.md"].includes(p)) return true;
  return p.endsWith(".md") && ["docs/", "app/", "prompts/"].some((s) => p.startsWith(s));
}

function pathToken(doc, line, raw) {
  const tok = raw.trim().replace(SUFFIX, "");
  if (!tok || ["-", "/", "http", "git@", "#"].some((s) => tok.startsWith(s)) || BAD.test(tok)) return null;
  if (!tok.includes("/")) return null; // a bare name: may be another place's file
  // a word counts in the sentence (。 or a table cell |) that holds the path, not in the rest of the line. An outside place counts with its code too
  // (a sentence that names docs/archive is about old material); a planned word counts in the prose only (a path's own name is not a word)
  const sent = line.split(/[。|]/).filter((s) => s.includes(raw)).join(" ") || line;
  const prose = sent.replace(CODE_G, " ");
  if (OUTSIDE.some((o) => sent.includes(o) || tok.includes(o))) return null;
  if (PLANNED_RE.some((r) => r.test(prose))) return null;
  if (!top.has(tok.split("/")[0])) return null;
  const trimmed = tok.replace(/\/+$/, "");
  const here = path.posix.normalize(path.posix.join(dirname(doc), trimmed));
  if (exists(path.posix.normalize(trimmed)) || exists(here)) return null;
  return tok;
}

const docs = [...files].filter(isDoc).sort();
const TEXT = readTexts(docs);
// a commit-qualified path (<commit>:<path> or <commit>^:<path>) names a version in the history: any word of a code span, not only the first one
const COMMIT_FULL = /^(?:[0-9a-f]{7,40}\^?:\S+)$/;
const commitRefs = [];
for (const doc of docs) {
  const lines = TEXT[doc].split("\n");
  lines.forEach((line, idx) => {
    const i = idx + 1;
    if (doc === "docs/ROADMAP.md" && /\| R\d+-\d+ \| todo \|/.test(line)) return;
    for (const m of line.matchAll(TICK_G)) {
      const raw = m[1];
      for (const w of raw.split(/\s+/).filter((x) => x)) {
        const s = w.replace(SUFFIX, "");
        if (COMMIT_FULL.test(s)) commitRefs.push([`${doc}:${i}`, s]);
      }
      // a command (a space inside the code span) is checked by each of its words: a path may be an argument, not only the first word
      const words = raw.trim().includes(" ") ? raw.split(/\s+/).filter((x) => x) : [raw];
      for (const w of words) {
        const t = pathToken(doc, line, w);
        if (t) add("a", `${doc}:${i}`, t, "実在しない");
      }
    }
    for (const m of line.matchAll(LINK_G)) {
      const t = m[1];
      if (t.startsWith("http") || t.startsWith("#")) continue;
      const t2 = pathToken(doc, line, t);
      if (t2) add("a", `${doc}:${i}`, t2, "リンク先が無い");
    }
  });
}

const specs = [...new Set(commitRefs.map(([, s]) => s))].sort();
if (specs.length) {
  const answer = git(["cat-file", "--batch-check"], Buffer.from(specs.map((s) => s + "\n").join(""), "utf8"))
    .toString("utf8")
    .split("\n");
  const gone = new Set(specs.filter((s, k) => /(missing|ambiguous)$/.test(answer[k] || "")));
  for (const [where, s] of commitRefs) {
    if (gone.has(s)) add("a", where, s, "その commit に無い");
  }
}

// g: the index names the operational and core documents; the harness README names every test
const indexText = TEXT["docs/index.md"] || "";
if (!indexText) add("g", "docs/index.md", "-", "docs/index.md が無い");
for (const p of [...files].sort()) {
  if (/^docs\/(core|ops)\/[^/]+\.md$/.test(p) && !indexText.includes(base(p))) {
    add("g", p, base(p), "docs/index.md に載っていない");
  }
}

const readmeText = TEXT["tests/harness/README.md"] || "";
for (const p of [...files].sort().filter((f) => /^tests\/harness\/t\d+[^/]*\.sh$/.test(f))) {
  if (!readmeText.includes(base(p))) add("g", p, base(p), "tests/harness/README.md の表に載っていない");
}

const corpus = docs.map((p) => TEXT[p]).join("");
for (const p of [...files].sort().filter((f) => /^scripts\/[^/]+$/.test(f))) {
  if (!corpus.includes(base(p))) add("g", p, base(p), "どの文書にも載っていない");
}

// c: line limits
for (const [p, limit] of [["AGENTS.md", 140], ["prompts/worker-rules.md", 60]]) {
  if (!files.has(p)) {
    add("c", p, "-", "無い");
    continue;
  }
  const lines = TEXT[p].split(/\r\n|\r|\n/);
  if (lines.length && lines[lines.length - 1] === "") lines.pop();
  if (lines.length > limit) add("c", p, `${lines.length} 行`, `${limit} 行を超える`);
}

// d: a decision that drops or rewrites something cites its source. 0386 is the first decision written under this rule (0385 and older are old documents, handled by another request).
const DECISION_FROM = 386;
const DROP_WORDS = ["入れない", "捨てる", "書き直し", "削除"];
const SOURCE = /msg_[0-9A-Za-z]+|(?:^|[^0-9A-Za-z_./-])(?:docs\/)?archive\//;
for (const p of [...files].sort()) {
  const m = p.match(/^docs\/decisions\/(\d{4})-[^/]+\.md$/);
  if (!m || parseInt(m[1], 10) < DECISION_FROM) continue;
  const text = TEXT[p];
  const word = DROP_WORDS.find((w) => text.includes(w));
  if (word && !SOURCE.test(text)) {
    add("d", p, word, "を決定しているが依頼者の発言 ID も旧設計の出典(archive/…)も無い");
  }
}

// e: the internal terms of the request rule (decision 0390). Worker (capitalized) is the Cloudflare product name, so only the role name worker is counted.
// 段 excludes 普段・段階・段落 (the step word is 手順). コミット is allowed by docs/ops/glossary.md, so only the English commit is counted; ワークフロー is not in the glossary, so it is counted.
// dry-run and dry run are one term (one alternative, so the run inside it is not counted again). evidence is written 証跡 in the client's text; rc was called unclear by the client (decision 0390).
const INTERNAL_EN_G = /(?<![A-Za-z])(?:workflow|commit|main|run|probe|binding|standalone|env|rc|evidence|dry[- ]run)(?![A-Za-z])/gi;
const INTERNAL_ROLE = /(?<![A-Za-z])worker(?![A-Za-z])/;
const INTERNAL_JA_G = /ワークフロー|段/g;
const REQUEST = /^docs\/requests\/\d{4}-[^/]+\.md$/;

function clientLines(text) {
  const out = [];
  let skip = 0;
  let fence = false;
  text.split("\n").forEach((line, idx) => {
    const i = idx + 1;
    if (line.trimStart().startsWith("```")) {
      fence = !fence;
      return;
    }
    if (fence) return;
    const m = line.match(/^(#+) /);
    if (m && skip && m[1].length <= skip) skip = 0;
    if (m && !skip && line.includes("自分で試した記録")) {
      skip = m[1].length;
      return;
    }
    if (!skip) out.push([i, line.replace(/`[^`\n]*`/g, "")]);
  });
  return out;
}

for (const p of [...files].sort()) {
  if (!REQUEST.test(p)) continue;
  for (const [i, line] of clientLines(TEXT[p])) {
    const terms = new Set([...line.matchAll(INTERNAL_EN_G)].map((m) => m[0].toLowerCase().replace(/ /g, "-")));
    if (INTERNAL_ROLE.test(line)) terms.add("worker");
    for (const m of line.replace(/普段|段階|段落/g, "").matchAll(INTERNAL_JA_G)) terms.add(m[0]);
    for (const term of [...terms].sort()) add("e", `${p}:${i}`, term, "依頼者が読む本文に内部用語");
  }
}

console.log("KIND  WHERE                                 WHAT                                  WHY");
for (const [kind, where, tok, why] of rows.slice(0, 5)) {
  console.log(`${kind.padEnd(5)} ${where.padEnd(37)} ${tok.padEnd(37)} ${why}`);
}
if (rows.length > 5) console.log(`(他 ${rows.length - 5} 件)`);
console.log(`consistency: ${rows.length} finding(s)`);
JS
exit $?
