#!/usr/bin/env bash
# usage: bash tests/harness/run.sh        (no arguments, no options)
# Runs every tests/harness/t*.sh, prints each one's FAIL and SKIP lines and a summary table, and exits 1 when anything FAILED.
# Needs: bash, jq, sqlite3, python3, shellcheck. node and opencode are used when present (their tests print SKIP, visibly, when not).
# Safe to run anywhere: it works in $TMPDIR/opencode/s2-sandbox/tests, runs no git command in the real repository, calls no real gh/wrangler/`opencode api`,
# starts only an isolated opencode (XDG_* in the sandbox) that it stops again, and creates no session and no model call.
set -o pipefail
export PYTHONDONTWRITEBYTECODE=1

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd) || { echo "run.sh: cannot resolve the test directory" >&2; exit 2; }
root=$(cd "$here/../.." && pwd) || exit 2
missing=""
for tool in bash jq sqlite3 python3 shellcheck; do command -v "$tool" > /dev/null || missing="$missing $tool"; done
if [ -n "$missing" ]; then echo "run.sh: missing tools:$missing" >&2; exit 2; fi

total_pass=0; total_fail=0; total_skip=0
rows=""
status=0

# 0. static checks: shellcheck on every script, bash -n on the hooks, node --check on the plugins
echo "== static checks"
sc_out=$(cd "$root" && shellcheck -x scripts/*.sh .githooks/pre-commit .githooks/pre-push tests/harness/*.sh tests/harness/fakebin/* tests/harness/fakebin-opencode/opencode 2>&1); sc=$?
if [ "$sc" -eq 0 ]; then echo "PASS  shellcheck: 0 findings in scripts/, .githooks/, tests/harness/"; total_pass=$((total_pass + 1)); else echo "FAIL  shellcheck:"; printf '%s\n' "$sc_out" | head -20; total_fail=$((total_fail + 1)); status=1; fi
if command -v node > /dev/null; then
  nc=0; for f in "$root"/.opencode/plugins/*.ts; do node --check "$f" 2> /dev/null || { echo "FAIL  node --check $f"; nc=1; }; done
  if [ "$nc" -eq 0 ]; then echo "PASS  node --check: every plugin parses"; total_pass=$((total_pass + 1)); else total_fail=$((total_fail + 1)); status=1; fi
else echo "SKIP  node missing: plugin syntax check"; total_skip=$((total_skip + 1)); fi
perm_bad=""; for f in "$root"/scripts/*.sh "$root"/.githooks/pre-commit "$root"/.githooks/pre-push "$root"/tests/harness/run.sh; do [ -x "$f" ] || perm_bad="$perm_bad ${f#"$root"/}"; done
if [ -z "$perm_bad" ]; then echo "PASS  every script and hook is executable"; total_pass=$((total_pass + 1)); else echo "FAIL  not executable:$perm_bad"; total_fail=$((total_fail + 1)); status=1; fi
echo

for t in "$here"/t[0-9]*.sh; do
  name=$(basename "$t" .sh)
  echo "== $name"
  out=$(bash "$t" 2>&1); rc=$?
  printf '%s\n' "$out" | grep -E '^(FAIL|SKIP)' || true
  summary=$(printf '%s\n' "$out" | tail -1)
  p=$(printf '%s\n' "$out" | grep -c '^PASS'); f=$(printf '%s\n' "$out" | grep -c '^FAIL'); s=$(printf '%s\n' "$out" | grep -c '^SKIP')
  total_pass=$((total_pass + p)); total_fail=$((total_fail + f)); total_skip=$((total_skip + s))
  if [ "$rc" -ne 0 ]; then status=1; [ "$f" -eq 0 ] && { echo "FAIL  $name exited $rc with no FAIL line: $(printf '%s\n' "$out" | tail -3 | tr '\n' '|')"; total_fail=$((total_fail + 1)); f=1; }; fi
  rows="$rows$(printf '%-18s pass %-4s fail %-3s skip %-3s\n' "$name" "$p" "$f" "$s")
"
  echo "   $summary"
  echo
done

# the tests create 94-96 MB sample files (size rule); /tmp is a 3.6 GiB tmpfs, so do not leave them behind
find "${TMPDIR:-/tmp}/opencode/s2-sandbox/tests" -type f -size +10M -delete 2> /dev/null
echo "== summary"
printf '%s' "$rows"
printf '%-18s pass %-4s fail %-3s skip %-3s\n' TOTAL "$total_pass" "$total_fail" "$total_skip"
if [ "$total_skip" -gt 0 ]; then echo "note: $total_skip test(s) were SKIPPED (a skipped test is not a passed test)."; fi
echo "note: not covered here (needs a real session): see tests/harness/README.md, section 'Canary'."
if [ "$status" -eq 0 ] && [ "$total_fail" -eq 0 ]; then echo "RESULT: PASS"; exit 0; fi
echo "RESULT: FAIL"
exit 1
