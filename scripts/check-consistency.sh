#!/usr/bin/env bash
# usage: scripts/check-consistency.sh        (no argument, no environment variable; reads files only, writes nothing)
# Prints the inconsistencies a machine can see in the current documents: first 5 rows and the count. Last line: "consistency: N finding(s)". Exit 0 always
# (scripts/preflight.sh turns N >= 1 into a WARN).
# a  a backticked path that does not exist. Its first segment must be a top-level entry of the repository (so `~/...`, `/...`, `docs/...` checks are limited to repository paths)
# g  docs/*.md, docs/core/*.md or docs/ops/*.md that docs/index.md does not name; tests/harness/t*.sh that tests/harness/README.md does not name;
#    a scripts/ file that no current document names
# c  AGENTS.md over 140 lines; prompts/worker-rules.md over 60 lines
# Not checked, on purpose:
#   old material and outputs: docs/archive/, archive/, evidence/, patches/, tools/, lift/ (imported measurement material), artifacts/, binaries/, lift/out*
#   a command: a backticked text with a space is checked only by its first word when that word is a path with "/" (`oc-msg.sh send …` is a command)
#   a bare file name (no "/") that the repository lacks: it may name a file in another place (~/work/handover, ~/.agents, an old tree)
#   a line that names one of those places (~/work/handover, ~/.agents, 旧ツリー, docs/archive)
#   a line that says the path is an output of a build or a step to create (PLANNED words: 作る・作成・出力・ビルド・退避 …)
#   text with < > * $ { } ( ) [ ] … | \ # ~ = ? % or a leading - or /
#   docs/ROADMAP.md rows whose status is todo (their files are planned outputs; the done rule of ROADMAP checks them)
set -o pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd) || { echo "check-consistency: cannot resolve the repository root" >&2; exit 0; }

python3 - "$root" << 'PY'
import os, re, sys

root = sys.argv[1]
os.chdir(root)
rows = []


def add(kind, where, tok, why):
    rows.append((kind, where, tok, why))


files = set()
for dp, dns, fns in os.walk("."):
    rel = os.path.normpath(dp)
    dns[:] = [d for d in dns if d not in (".git", "node_modules")]
    for fn in fns:
        files.add(os.path.normpath(os.path.join(rel, fn)))

top = {p.split("/")[0] for p in files} | {d for d in os.listdir(".") if not d.startswith(".")}
EXCLUDED = ("docs/archive/", "archive/", "evidence/", "patches/", "tools/", "lift/", "artifacts/", "binaries/")
OUTSIDE = ("~/work/handover", "~/.agents", "旧ツリー", "docs/archive")
# words that mark a path as a build output or a step to create, not a file that must exist now
PLANNED = ("ビルド", "作る", "作成", "出力", "ロールバック", "退避", "置く", "前に", "ls ", "ロード", "build", "出来")
REPO_EXT = re.compile(r"\.(md|sh|ts|mjs|cjs|py|jsonc|json|tsv|ya?ml|diff|src)$")
BAD = re.compile(r"[<>*$…{}()\[\]|\\#~=?%]")
TICK = re.compile(r"`([^`\n]+)`")
LINK = re.compile(r"\]\(([^)\s]+)\)")
SUFFIX = re.compile(r":\d[\d,\-]*$")


def is_doc(p):
    if p.startswith(EXCLUDED):
        return False
    if p in ("AGENTS.md", "README.md", "tests/harness/README.md"):
        return True
    return p.endswith(".md") and p.startswith(("docs/", "app/", "prompts/"))


def path_token(doc, line, raw, in_command):
    tok = SUFFIX.sub("", raw.strip())
    if in_command:
        tok = tok.split()[0]
    if not tok or tok.startswith(("-", "/", "http", "git@", "#")) or BAD.search(tok):
        return None
    if "/" not in tok:
        return None  # a bare name: may be another place's file
    if any(o in line for o in OUTSIDE):
        return None
    if any(w in line for w in PLANNED):
        return None
    if tok.split("/")[0] not in top:
        return None
    here = os.path.normpath(os.path.join(os.path.dirname(doc), tok.rstrip("/")))
    if os.path.exists(os.path.normpath(tok.rstrip("/"))) or os.path.exists(here):
        return None
    return tok


docs = sorted(p for p in files if is_doc(p))
for doc in docs:
    lines = open(doc, encoding="utf-8", errors="replace").read().split("\n")
    for i, line in enumerate(lines, 1):
        if doc == "docs/ROADMAP.md" and re.search(r"\| R\d+-\d+ \| todo \|", line):
            continue
        for m in TICK.finditer(line):
            raw = m.group(1)
            t = path_token(doc, line, raw, " " in raw.strip())
            if t:
                add("a", f"{doc}:{i}", t, "実在しない")
        for m in LINK.finditer(line):
            t = m.group(1)
            if t.startswith(("http", "#")):
                continue
            t2 = path_token(doc, line, t, False)
            if t2:
                add("a", f"{doc}:{i}", t2, "リンク先が無い")

# g: the index names the operational and core documents; the harness README names every test
index_text = open("docs/index.md", encoding="utf-8").read() if os.path.exists("docs/index.md") else ""
if not index_text:
    add("g", "docs/index.md", "-", "docs/index.md が無い")
for p in sorted(files):
    if re.fullmatch(r"docs/(core|ops)/[^/]+\.md", p) and os.path.basename(p) not in index_text:
        add("g", p, os.path.basename(p), "docs/index.md に載っていない")

readme_text = open("tests/harness/README.md", encoding="utf-8").read() if os.path.exists("tests/harness/README.md") else ""
for p in sorted(f for f in files if re.fullmatch(r"tests/harness/t\d+[^/]*\.sh", f)):
    if os.path.basename(p) not in readme_text:
        add("g", p, os.path.basename(p), "tests/harness/README.md の表に載っていない")

corpus = "".join(open(p, encoding="utf-8", errors="replace").read() for p in docs)
for p in sorted(f for f in files if re.fullmatch(r"scripts/[^/]+", f)):
    if os.path.basename(p) not in corpus:
        add("g", p, os.path.basename(p), "どの文書にも載っていない")

# c: line limits
for p, limit in (("AGENTS.md", 140), ("prompts/worker-rules.md", 60)):
    if not os.path.exists(p):
        add("c", p, "-", "無い")
        continue
    n = len(open(p, encoding="utf-8").read().splitlines())
    if n > limit:
        add("c", p, f"{n} 行", f"{limit} 行を超える")

print("KIND  WHERE                                 WHAT                                  WHY")
for kind, where, tok, why in rows[:5]:
    print(f"{kind:<5} {where:<37} {tok:<37} {why}")
if len(rows) > 5:
    print(f"(他 {len(rows) - 5} 件)")
print(f"consistency: {len(rows)} finding(s)")
PY
exit 0
