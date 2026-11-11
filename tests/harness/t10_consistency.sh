#!/usr/bin/env bash
# t10: scripts/check-consistency.sh on synthetic repositories (a copy of the script in a sandbox; no real repository is read or changed).
# Detects: a backticked or linked repository path that does not exist; a docs/ops or docs/core file that docs/index.md does not name;
# a tests/harness/t*.sh that the README does not name; a scripts/ file that no document names; AGENTS.md over 140 lines.
# Does not detect: a path in old material (docs/archive, archive, lift); a path on a line that names another place or says it is a planned output;
# a command's arguments; a bare file name; a todo row of docs/ROADMAP.md; text with placeholders (<…>, *).
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
run() { (cd "$1" && bash scripts/check-consistency.sh 2>&1); }
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

# the real repository: the check runs and prints its count line
real=$(cd "$ROOT" && bash scripts/check-consistency.sh 2>&1)
if printf '%s' "$real" | tail -1 | grep -qE '^consistency: [0-9]+ finding\(s\)$'; then pass "the real repository: the count line is printed"; else fail "the real repository: no count line"; fi

finish
