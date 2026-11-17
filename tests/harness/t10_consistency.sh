#!/usr/bin/env bash
# t10: scripts/check-consistency.sh on synthetic repositories (a copy of the script in a sandbox; no real repository is read or changed).
# Detects: a backticked or linked repository path that does not exist; a docs/ops or docs/core file that docs/index.md does not name;
# a tests/harness/t*.sh that the README does not name; a scripts/ file that no document names; AGENTS.md over 140 lines;
# a decision from 0386 that says 入れない・捨てる・書き直し・削除 with neither a msg_ ID nor an archive/ source.
# an internal term (workflow, commit, main, worker, run, probe, binding, standalone, env, rc, dry-run, evidence, 段) in the text a client reads in docs/requests/NNNN-*.md
# Reads the git index: a path to a file on disk only (untracked, or ignored by .gitignore) is found; an unstaged edit is not read; a commit-qualified path (<commit>^:<path>) is not checked.
# Does not detect: a path in old material (docs/archive, archive, lift); a path in a sentence (。 or a table cell |) that names another place or says it is a planned output (the words count in the prose only);
# a command's arguments; a bare file name; a todo row of docs/ROADMAP.md; text with placeholders (<…>, *);
# a request's 自分で試した記録 section, its code and inline code, 普段・段階・段落, a capitalized Worker, コミット, TEMPLATE.md.
# shellcheck source=lib.sh
# shellcheck disable=SC2016  # the synthetic documents contain literal backticks on purpose
. "$(dirname "$0")/lib.sh"
set -o pipefail

sb=$(new_sandbox t10)
mk() { # dir: a minimal repository with the check, an index, the harness README and the two limited files
  local d=$1
  rm -rf "$d"; mkdir -p "$d/scripts" "$d/docs/ops" "$d/docs/core" "$d/docs/archive" "$d/prompts" "$d/tests/harness" "$d/lift/x" "$d/app"
  cp "$ROOT/scripts/check-consistency.sh" "$d/scripts/"
  printf '# docs\n\n| `ops/alpha.md` | a |\n| `core/beta.md` | b |\n' > "$d/docs/index.md"
  printf '# a\nThe helper is `scripts/helper.sh`; the check is `scripts/check-consistency.sh`.\n' > "$d/docs/ops/alpha.md"
  printf '# b\n' > "$d/docs/core/beta.md"
  printf 'ok\n' > "$d/app/real.ts"
  printf 'ok\n' > "$d/scripts/helper.sh"
  printf 'ok\n' > "$d/tests/harness/t01_x.sh"
  printf '| `t01_x.sh` | x |\n' > "$d/tests/harness/README.md"
  printf 'line\n' > "$d/AGENTS.md"
  printf 'rule\n' > "$d/prompts/worker-rules.md"
  printf '# a\n' > "$d/lift/x/readme.md"
  printf 'ok\n' > "$d/docs/archive/old.md"
  git -C "$d" init -q 2> /dev/null
}
run() { (cd "$1" && git add -A && bash scripts/check-consistency.sh 2>&1); } # the check reads the index: stage the whole synthetic tree first
run_unstaged() { (cd "$1" && bash scripts/check-consistency.sh 2>&1); }   # the index as it is (for a file that is on disk only)
count() { printf '%s\n' "$1" | tail -1 | grep -oE '[0-9]+' | head -1; }

# clean: a fully consistent tree reports 0
clean="$sb/clean"; mk "$clean"
out=$(run "$clean"); rc=$?
expect_eq "a consistent tree: 0 findings" 0 "$(count "$out")"
expect_eq "the check always exits 0" 0 "$rc"

# a: a backticked path that does not exist
bad="$sb/bad-path"; mk "$bad"
printf 'See `app/missing.ts` for the entry.\n' > "$bad/docs/ops/gamma.md"
printf '| `ops/gamma.md` | g |\n' >> "$bad/docs/index.md"
out=$(run "$bad")
expect_contains "a backticked path that does not exist is found" "$out" "app/missing.ts"
expect_eq "and counted once" 1 "$(count "$out")"

# a: a markdown link that does not exist
lnk="$sb/bad-link"; mk "$lnk"
printf 'Read [the plan](docs/ops/nowhere.md) first.\n' > "$lnk/docs/ops/delta.md"
printf '| `ops/delta.md` | d |\n' >> "$lnk/docs/index.md"
out=$(run "$lnk")
expect_contains "a link to a path that does not exist is found" "$out" "docs/ops/nowhere.md"

# g: a document missing from the index
idx="$sb/no-index"; mk "$idx"
printf '# e\n' > "$idx/docs/ops/epsilon.md"
out=$(run "$idx")
expect_contains "an ops document not named by the index is found" "$out" "epsilon.md"

# g: a harness test missing from the README
tst="$sb/no-test-row"; mk "$tst"
printf 'ok\n' > "$tst/tests/harness/t02_y.sh"
out=$(run "$tst")
expect_contains "a harness test not in the README table is found" "$out" "t02_y.sh"

# g: a script no document names
scr="$sb/unnamed-script"; mk "$scr"
printf 'ok\n' > "$scr/scripts/orphan.sh"
out=$(run "$scr")
expect_contains "a script that no document names is found" "$out" "orphan.sh"

# c: AGENTS.md over 140 lines
long="$sb/long-agents"; mk "$long"
seq 1 150 > "$long/AGENTS.md"
out=$(run "$long")
expect_contains "AGENTS.md over 140 lines is found" "$out" "150 行"

# NOT detected: a path in old material
arc="$sb/old-material"; mk "$arc"
printf 'Old: `docs/archive/gone.md` and `lift/x/gone.c`.\n' > "$arc/docs/ops/zeta.md"
printf '| `ops/zeta.md` | z |\n' >> "$arc/docs/index.md"
out=$(run "$arc")
expect_eq "a path that names old material is not a finding" 0 "$(count "$out")"

# NOT detected: a path on a line that names another place
oth="$sb/other-place"; mk "$oth"
printf 'Handover: `~/work/handover/missing-file.md`.\n' > "$oth/docs/ops/eta.md"
printf '| `ops/eta.md` | e |\n' >> "$oth/docs/index.md"
out=$(run "$oth")
expect_eq "a path that names another place is not a finding" 0 "$(count "$out")"

# NOT detected: a planned output (the line says it is made by a step)
pln="$sb/planned"; mk "$pln"
printf 'The output `evidence/R9-9.md` is made by step R9.\n' > "$pln/docs/ops/theta.md"
printf '| `ops/theta.md` | t |\n' >> "$pln/docs/index.md"
out=$(run "$pln")
expect_eq "a planned output is not a finding" 0 "$(count "$out")"

# NOT detected: a command example (a space inside the backticks; only its first word is looked at, and that is not a path)
cmd="$sb/command-example"; mk "$cmd"
printf 'Run `scripts/missing.sh --check` once.\n' > "$cmd/docs/ops/iota.md"
printf '| `ops/iota.md` | i |\n' >> "$cmd/docs/index.md"
out=$(run "$cmd")
expect_contains "a command whose first word is a missing path is found (the path is the first word)" "$out" "scripts/missing.sh"
printf 'Send with `oc-msg.sh send --to x`.\n' > "$cmd/docs/ops/kappa.md"
printf '| `ops/kappa.md` | k |\n' >> "$cmd/docs/index.md"
out=$(run "$cmd")
expect_not_contains "a command whose first word is a bare name is not a finding" "$out" "oc-msg.sh"

# NOT detected: a bare file name (it may be another place's file), and a placeholder
bare="$sb/bare-name"; mk "$bare"
printf 'Keep `R2-patch-ledger.tsv` and `<NNNN>-slug.md` as they are.\n' > "$bare/docs/ops/lambda.md"
printf '| `ops/lambda.md` | l |\n' >> "$bare/docs/index.md"
out=$(run "$bare")
expect_eq "a bare name and a placeholder are not findings" 0 "$(count "$out")"

# NOT detected: a todo row of ROADMAP (its file is a planned output)
rm_="$sb/roadmap-todo"; mk "$rm_"
printf '| R9-1 | todo | — | 整備 | x | `evidence/R9-1.md` を作る | y | z | w |\n' > "$rm_/docs/ROADMAP.md"
printf '| `ROADMAP.md` | r |\n' >> "$rm_/docs/index.md"
out=$(run "$rm_")
expect_eq "a todo row of ROADMAP is not a finding" 0 "$(count "$out")"

# d: a decision from 0386 that drops something must cite an owner message or an old source; 0385 and older are not checked
dec="$sb/decisions"; mk "$dec"; mkdir -p "$dec/docs/decisions"
printf '# 0390: x\n決定: 旧の処理を入れない。\n' > "$dec/docs/decisions/0390-drop.md"
printf '# 0391: x\n決定: 旧の処理を削除する。根拠: archive/ci-old/deploy.yml の 12 行。\n' > "$dec/docs/decisions/0391-cited.md"
printf '# 0392: x\n決定: 旧の処理は書き直し。根拠: msg_1c30a3dc9001V6htDJWIQixRsO。\n' > "$dec/docs/decisions/0392-owner.md"
printf '# 0385: x\n決定: 旧の処理を捨てる。\n' > "$dec/docs/decisions/0385-old.md"
printf '# 0393: x\n決定: 新しい処理を足す。\n' > "$dec/docs/decisions/0393-plain.md"
out=$(run "$dec")
expect_contains "a decision from 0386 that drops something with no owner message or old source is found" "$out" "0390-drop.md"
expect_not_contains "a decision that cites an old source (archive/) is not a finding" "$out" "0391-cited.md"
expect_not_contains "a decision that cites an owner message (msg_) is not a finding" "$out" "0392-owner.md"
expect_not_contains "a decision 0385 or older is not checked" "$out" "0385-old.md"
expect_not_contains "a decision from 0386 without a drop word is not a finding" "$out" "0393-plain.md"
expect_eq "and only the one decision is counted" 1 "$(count "$out")"

# e: the text a client reads in a request document. An internal term is a finding, except in the 自分で試した記録 section, a code block, `inline code`,
# 普段・段階・段落, a capitalized Worker (the product name), コミット; and TEMPLATE.md is not a request document
rq() { # name file text(printf %b): a repository whose only request document is the given text; prints its path
  local d="$sb/req-$1"; mk "$d"; mkdir -p "$d/docs/requests"
  printf '%b' "$3" > "$d/docs/requests/$2"
  printf '%s' "$d"
}
for w in workflow commit main worker run probe binding standalone env rc dry-run evidence; do
  d=$(rq "term-$w" 0101-x.md "# Issue 0101: x\n## 依頼者にお願いしたい作業\n依頼者の $w を確かめる。\n")
  out=$(run "$d"); expect_contains "an internal term ($w) in the text the client reads is found (no 記録 heading: the whole text is read)" "$out" "0101-x.md"
done
d=$(rq ja-words 0101-x.md "# Issue 0101: x\n## 依頼者にお願いしたい作業\n自動処理の段を確かめる。ワークフローを止める。\n")
out=$(run "$d"); expect_contains "the Japanese internal term ワークフロー is found" "$out" "ワークフロー"
expect_contains "the Japanese internal term 段 is found" "$out" "段"
d=$(rq upper-rc 0101-x.md "# Issue 0101: x\n## 依頼者にお願いしたい作業\n結果の RC を確かめる。\n")
out=$(run "$d"); expect_eq "an upper-case RC is found (the match ignores case): one finding" 1 "$(count "$out")"
d=$(rq dry-run-words 0101-x.md "# Issue 0101: x\n## 依頼者にお願いしたい作業\n依頼者の dry run を確かめる。\n依頼者の dry-run を確かめる。\n")
out=$(run "$d"); expect_eq "dry run and dry-run on two lines: two findings (the run inside dry-run is not counted again)" 2 "$(count "$out")"
expect_contains "a dry run with a space is reported as dry-run" "$out" "dry-run"
d=$(rq evidence-path 0101-x.md "# Issue 0101: x\n## 依頼者にお願いしたい作業\n結果は evidence/R8-2.md に残る。\n")
out=$(run "$d"); expect_contains "a path evidence/… outside inline code is found" "$out" "0101-x.md"
d=$(rq evidence-inline 0101-x.md "# Issue 0101: x\n## 依頼者にお願いしたい作業\n結果は \`evidence/R9-3.md\` に残る。\n")
out=$(run "$d"); expect_eq "evidence only in inline code is not a finding" 0 "$(count "$out")"
d=$(rq record-terms 0101-x.md "# Issue 0101: x\n## 依頼者にお願いしたい作業\n依頼者の操作は 1 つだけ。\n## 自分で試した記録\nrc と dry-run と evidence/R6-8.md を実行した。\n")
out=$(run "$d"); expect_eq "rc, dry-run and evidence only in the 自分で試した記録 section are not findings" 0 "$(count "$out")"
d=$(rq partial-words 0101-x.md "# Issue 0101: x\n## 依頼者にお願いしたい作業\n事情(circumstance)、search、evidential、dry-runner、runtime の話。\n")
out=$(run "$d"); expect_eq "partial matches (circumstance, search, evidential, dry-runner, runtime) are not findings" 0 "$(count "$out")"
d=$(rq code-only 0101-x.md "# Issue 0101: x\n## 依頼者にお願いしたい作業\n\`main\` と \`run\` を確かめる。\n\`\`\`\nmain run workflow commit\n\`\`\`\n")
out=$(run "$d"); expect_eq "a term only in inline code or a code block is not a finding" 0 "$(count "$out")"
d=$(rq words 0101-x.md "# Issue 0101: x\n## 依頼者にお願いしたい作業\n普段の段階と、段落の話。\n")
out=$(run "$d"); expect_eq "普段・段階・段落 are not findings" 0 "$(count "$out")"
d=$(rq record 0101-x.md "# Issue 0101: x\n## 依頼者にお願いしたい作業\n依頼者の操作は 1 つだけ。\n## 自分で試した記録\nmain と run と 段 を実行した。\n")
out=$(run "$d"); expect_eq "the 自分で試した記録 section is not read by the client" 0 "$(count "$out")"
d=$(rq tail 0101-x.md "# Issue 0101: x\n## 依頼者にお願いしたい作業\n依頼者の操作は 1 つだけ。\n## 自分で試した記録\nmain を実行した。\n## 依頼なしで続ける作業\n- run を動かさない。\n")
out=$(run "$d"); expect_contains "a section after the 自分で試した記録 section is read by the client" "$out" "0101-x.md"
d=$(rq allowed 0101-x.md "# Issue 0101: x\n## 依頼者にお願いしたい作業\n「Worker \`fl-studio-web\`」の設定。コミット \`b3a70b5\` の内容。\n")
out=$(run "$d"); expect_eq "a capitalized Worker (the product name) and コミット are not findings" 0 "$(count "$out")"
d="$sb/req-template"; mk "$d"; mkdir -p "$d/docs/requests"
printf '# Issue NNNN: <件名>\n## 依頼者にお願いしたい作業\nworkflow と commit\n' > "$d/docs/requests/TEMPLATE.md"
out=$(run "$d"); expect_eq "TEMPLATE.md is not a request document" 0 "$(count "$out")"

# index: the check reads the git index. A path to a file that is on disk only (untracked) is a finding; the same file staged is not
ut="$sb/untracked"; mk "$ut"
printf 'See `app/extra.ts` for the entry.\n' > "$ut/docs/ops/nu.md"
printf '| `ops/nu.md` | n |\n' >> "$ut/docs/index.md"
git -C "$ut" add -A
printf 'extra\n' > "$ut/app/extra.ts"
out=$(run_unstaged "$ut")
expect_contains "a path to a file on disk but not tracked is found" "$out" "app/extra.ts"
git -C "$ut" add app/extra.ts
out=$(run_unstaged "$ut")
expect_eq "the same file staged (in the index) is not a finding" 0 "$(count "$out")"

# index: a path into a directory that .gitignore names (an ignored output) is not in the repository
ig="$sb/ignored"; mk "$ig"
printf 'app/dist/\n' > "$ig/.gitignore"
mkdir -p "$ig/app/dist"; printf 'page\n' > "$ig/app/dist/index.html"
printf 'The page is `app/dist/index.html`.\n' > "$ig/docs/ops/ig.md"
printf '| `ops/ig.md` | i |\n' >> "$ig/docs/index.md"
out=$(run "$ig")
expect_contains "a path into an ignored directory (app/dist) is found" "$out" "app/dist/index.html"
printf 'ビルドの出力 `app/dist/index.html` を読む。\n' > "$ig/docs/ops/ig.md"
out=$(run "$ig")
expect_eq "a planned word in the same sentence (ビルドの出力) is not a finding" 0 "$(count "$out")"

# sentences: a planned word counts only in the sentence (。 or a table cell |) that holds the path, not in the path's own name, a code span, or a katakana word
printf '`app/missing.ts` を読む。ビルドの話。\n' > "$ig/docs/ops/ig.md"
out=$(run "$ig")
expect_contains "a planned word in another sentence of the same line does not hide the path" "$out" "app/missing.ts"
printf '| `app/missing.ts` | 読む | ビルド |\n' > "$ig/docs/ops/ig.md"
out=$(run "$ig")
expect_contains "a planned word in another table cell does not hide the path" "$out" "app/missing.ts"
printf '`app/missing.ts` をダウンロードする。\n' > "$ig/docs/ops/ig.md"
out=$(run "$ig")
expect_contains "a katakana word that ends in ロード (ダウンロード) is not the planned word" "$out" "app/missing.ts"
printf '`scripts/build-missing.sh` を読む。\n' > "$ig/docs/ops/ig.md"
out=$(run "$ig")
expect_contains "a planned word in the path's own name (build) does not hide it" "$out" "scripts/build-missing.sh"

# a commit-qualified path (<commit>^:<path>) names an old version: its first segment is not a repository entry, so it is not a finding
printf '`HEAD^:app/gone.sh` を読む。\n' > "$ig/docs/ops/ig.md"
out=$(run "$ig")
expect_eq "a commit-qualified path (an old version) is not a finding" 0 "$(count "$out")"

# index: the staged text is read, not the working tree. A fix that is not staged does not change the result
ed="$sb/staged-edit"; mk "$ed"
printf 'See `app/missing.ts`.\n' > "$ed/docs/ops/ed.md"
printf '| `ops/ed.md` | e |\n' >> "$ed/docs/index.md"
out=$(run "$ed")
expect_contains "a missing path in a staged document is found" "$out" "app/missing.ts"
printf 'See `app/real.ts`.\n' > "$ed/docs/ops/ed.md"
out=$(run_unstaged "$ed")
expect_contains "an unstaged fix is not read: the staged text is still checked" "$out" "app/missing.ts"
git -C "$ed" add -A
out=$(run_unstaged "$ed")
expect_eq "after git add, the fixed text is read" 0 "$(count "$out")"

# a command: every word is checked, so a path that is an argument (not the first word) is found too
printf 'Run `cat app/missing.ts` once.\n' > "$ig/docs/ops/ig.md"
out=$(run "$ig")
expect_contains "a path that is an argument of a command (not its first word) is found" "$out" "app/missing.ts"

# commit-qualified paths: the path must be in that commit (git cat-file), in the first word or any other word of a code span
cc="$sb/commit-ref"; mk "$cc"
printf 'base\n' > "$cc/app/old.ts"
git -C "$cc" add -A && git -C "$cc" -c user.name=t -c user.email=t@t commit -qm base
c=$(git -C "$cc" rev-parse --short HEAD)
printf 'The old file is `%s:app/old.ts`.\n' "$c" > "$cc/docs/ops/cc.md"
printf '| `ops/cc.md` | c |\n' >> "$cc/docs/index.md"
out=$(run "$cc")
expect_eq "a commit-qualified path that the commit has is not a finding" 0 "$(count "$out")"
printf 'The old file is `%s:app/gone.ts`.\n' "$c" > "$cc/docs/ops/cc.md"
out=$(run "$cc")
expect_contains "a commit-qualified path that the commit lacks is found" "$out" "app/gone.ts"
printf 'Run `bash %s:app/gone.ts` once.\n' "$c" > "$cc/docs/ops/cc.md"
out=$(run "$cc")
expect_contains "a commit-qualified path in a command (not its first word) is found" "$out" "app/gone.ts"
printf 'The root commit has no parent: `%s^:app/old.ts`.\n' "$c" > "$cc/docs/ops/cc.md"
out=$(run "$cc")
expect_contains "a parent of the root commit (<commit>^) is found" "$out" "app/old.ts"

# the real repository: the check runs and prints its count line
real=$(cd "$ROOT" && bash scripts/check-consistency.sh 2>&1)
if printf '%s' "$real" | tail -1 | grep -qE '^consistency: [0-9]+ finding\(s\)$'; then pass "the real repository: the count line is printed"; else fail "the real repository: no count line"; fi

finish
