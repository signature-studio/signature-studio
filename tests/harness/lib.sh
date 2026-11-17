# shellcheck shell=bash
# Shared helpers for tests/harness/t_*.sh. Source this file. Each t_*.sh prints "PASS ..." / "FAIL ..." / "SKIP ..." lines and exits non-zero on any FAIL.
# SKIP is visible on purpose: a skipped test is not a passed test, and run.sh prints the number of skips.

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
# shellcheck disable=SC2034  # used by the t_*.sh files that source this one
HERE="$ROOT/tests/harness"
SANDBOX_BASE="${TMPDIR:-/tmp}/opencode/s2-sandbox/tests"
export PYTHONDONTWRITEBYTECODE=1   # rules.py is imported by several tests; do not leave __pycache__ in the repository
unset OPENCODE_SESSION_ID   # the tests run inside sessions too; the scripts must see "no session" unless a test sets one

PASS=0; FAILN=0; SKIPN=0
pass() { PASS=$((PASS + 1)); printf 'PASS  %s\n' "$*"; }
fail() { FAILN=$((FAILN + 1)); printf 'FAIL  %s\n' "$*"; }
skip() { SKIPN=$((SKIPN + 1)); printf 'SKIP  %s\n' "$*"; }
check() { # description, then a command; passes when the command succeeds
  local d=$1; shift
  if "$@" > /dev/null 2>&1; then pass "$d"; else fail "$d"; fi
}
expect_eq() { # description expected actual
  if [ "$2" = "$3" ]; then pass "$1 ($3)"; else fail "$1: expected '$2', got '$3'"; fi
}
expect_contains() { # description text needle
  if [[ "$2" == *"$3"* ]]; then pass "$1"; else fail "$1: '$3' not found in: $(printf '%s' "$2" | head -c 300)"; fi
}
expect_not_contains() {
  if [[ "$2" != *"$3"* ]]; then pass "$1"; else fail "$1: '$3' found"; fi
}
finish() {
  printf '%s: %d passed, %d failed, %d skipped\n' "$(basename "$0")" "$PASS" "$FAILN" "$SKIPN"
  [ "$FAILN" -eq 0 ]
}
new_sandbox() { # name -> path of a fresh empty directory
  local d="$SANDBOX_BASE/$1"
  rm -rf "$d"
  mkdir -p "$d" || { echo "cannot create $d" >&2; exit 2; }
  printf '%s' "$d"
}
# A throwaway git repository holding a copy of the harness files (never the real repository: no git command may touch it).
make_repo_copy() { # dir
  local d=$1
  mkdir -p "$d/scripts" "$d/.githooks" "$d/.opencode" "$d/prompts" "$d/tests/harness"
  cp "$ROOT/opencode.jsonc" "$ROOT/.gitignore" "$d/"
  cp "$ROOT"/scripts/*.sh "$ROOT/scripts/forbidden-allowlist.txt" "$ROOT/scripts/forbidden-terms.tsv" "$d/scripts/"
  cp "$ROOT"/.githooks/pre-commit "$ROOT"/.githooks/pre-push "$d/.githooks/"
  cp -r "$ROOT/.opencode/plugins" "$d/.opencode/"
  printf '<!-- worker-rules v1 -->\nrules for the test\n' > "$d/prompts/worker-rules.md"
  printf 'start\n\n## GOAL\nmake it work\n' > "$d/prompts/START.md"
  git -C "$d" init -q -b main
  (cd "$d" && bash scripts/setup-repo.sh > /dev/null) || { echo "setup-repo.sh failed in $d" >&2; exit 2; }
}
