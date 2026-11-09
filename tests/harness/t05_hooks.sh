#!/usr/bin/env bash
# t05: .githooks/pre-commit and pre-push, and scripts/setup-repo.sh. Only in throwaway repositories under the sandbox (never the real repository).
# gh is a fake (tests/harness/fakebin/gh); pushes go to a local bare repository. No network, no GitHub.
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
set -o pipefail

sb=$(new_sandbox t05)
v1="anthr"; v2="opic"
FAKE="$HERE/fakebin"

# ---------------- setup-repo.sh
r="$sb/setup"; mkdir -p "$r/scripts" "$r/.githooks"; git -C "$r" init -q -b main
cp "$ROOT/scripts/setup-repo.sh" "$r/scripts/"; cp "$ROOT"/.githooks/pre-* "$r/.githooks/"
out=$(cd "$r" && GIT_CONFIG_GLOBAL=/dev/null bash scripts/setup-repo.sh 2>&1); rc=$?
expect_eq "setup-repo.sh exits 0" 0 "$rc"
expect_eq "core.hooksPath is set (local)" ".githooks" "$(git -C "$r" config --local core.hooksPath)"
expect_eq "user.name is set (local)" "signature-studio agent" "$(git -C "$r" config --local user.name)"
expect_eq "user.email is set (local, noreply)" "signature-studio-agent@users.noreply.github.com" "$(git -C "$r" config --local user.email)"
before=$(git -C "$r" config --local -l | sort | md5sum)
(cd "$r" && bash scripts/setup-repo.sh > /dev/null 2>&1)
expect_eq "setup-repo.sh is idempotent (the local config is the same after a second run)" "$before" "$(git -C "$r" config --local -l | sort | md5sum)"
expect_eq "setup-repo.sh wrote no global setting" "" "$(GIT_CONFIG_GLOBAL=/dev/null git config --global -l 2> /dev/null)"
if [ -x "$r/.githooks/pre-commit" ] && [ -x "$r/.githooks/pre-push" ]; then pass "hooks are executable after setup"; else fail "hooks are not executable"; fi
expect_contains "the setup output shows what was set" "$out" "core.hooksPath=.githooks"

# ---------------- pre-commit
repo="$sb/repo"; make_repo_copy "$repo"
echo "hello" > "$repo/ok.txt"
git -C "$repo" add ok.txt scripts .githooks .opencode prompts opencode.jsonc .gitignore tests 2> /dev/null
out=$(git -C "$repo" commit -q -m "clean" 2>&1); rc=$?
expect_eq "pre-commit lets a clean commit through" 0 "$rc"

echo "uses the $v1$v2 api" > "$repo/bad.txt"
git -C "$repo" add bad.txt
out=$(git -C "$repo" commit -q -m "bad" 2>&1); rc=$?
expect_eq "pre-commit rejects a staged vendor word" 1 "$rc"
expect_contains "the rejection names rule and file" "$out" "vendor	bad.txt:1"
expect_not_contains "the rejection does not print the matched line" "$out" "api"
expect_eq "no commit was made" 1 "$(git -C "$repo" rev-list --count HEAD)"

# partially staged: the staged content is judged, not the working tree
git -C "$repo" reset -q bad.txt; rm -f "$repo/bad.txt"
echo "clean staged content" > "$repo/part.txt"; git -C "$repo" add part.txt
echo "later edit with $v1$v2 (not staged)" > "$repo/part.txt"
out=$(git -C "$repo" commit -q -m "staged part is clean" 2>&1); rc=$?
expect_eq "only the STAGED content is scanned: a dirty unstaged edit does not block" 0 "$rc"
git -C "$repo" checkout -q -- part.txt
echo "dirty staged content $v1$v2" > "$repo/part2.txt"; git -C "$repo" add part2.txt
echo "clean now" > "$repo/part2.txt"
out=$(git -C "$repo" commit -q -m "dirty staged" 2>&1); rc=$?
expect_eq "a dirty STAGED content is rejected even when the working tree was cleaned" 1 "$rc"
git -C "$repo" reset -q part2.txt; rm -f "$repo/part2.txt"

# a secret and an oversize file
echo "token=ghp_$(printf 'A1b2C3d4E5%.0s' 1 2 3 4)" > "$repo/tok.txt"; git -C "$repo" add tok.txt
out=$(git -C "$repo" commit -q -m tok 2>&1); expect_eq "pre-commit rejects a token-like string" 1 "$?"
git -C "$repo" reset -q tok.txt; rm -f "$repo/tok.txt"
head -c 96000000 /dev/zero > "$repo/big.bin"; git -C "$repo" add big.bin
out=$(git -C "$repo" commit -q -m big 2>&1); expect_eq "pre-commit rejects a file over 95 MB" 1 "$?"
git -C "$repo" reset -q big.bin; rm -f "$repo/big.bin"

# a deleted file and a rename are fine; the hook does not leave temporary files behind
git -C "$repo" rm -q ok.txt; out=$(git -C "$repo" commit -q -m "delete" 2>&1); expect_eq "a commit that only deletes a file passes" 0 "$?"
expect_eq "the hook leaves no scan directory in .git" "0" "$(find "$repo/.git" -maxdepth 1 -name 'scan-staged.*' | wc -l | tr -d ' ')"

# the hook refuses to run without the scanner (fail closed)
mv "$repo/scripts/scan-forbidden.sh" "$repo/scripts/scan-forbidden.sh.off"
echo x > "$repo/y.txt"; git -C "$repo" add y.txt
out=$(git -C "$repo" commit -q -m y 2>&1); rc=$?
expect_eq "pre-commit fails closed when scripts/scan-forbidden.sh is missing" 1 "$rc"
expect_contains "and says why" "$out" "refusing to commit without the scan"
mv "$repo/scripts/scan-forbidden.sh.off" "$repo/scripts/scan-forbidden.sh"; git -C "$repo" reset -q y.txt; rm -f "$repo/y.txt"

# there is no switch: environment variables do not turn the scan off
echo "uses the $v1$v2 api" > "$repo/bad2.txt"; git -C "$repo" add bad2.txt
for var in SKIP_SCAN=1 NO_VERIFY=1 SCAN_FORBIDDEN_SKIP=1 HUSKY=0 GIT_SKIP_HOOKS=1 FORCE=1; do
  env "$var" git -C "$repo" commit -q -m bad2 > /dev/null 2>&1; rc=$?
  if [ "$rc" -ne 0 ]; then pass "pre-commit still rejects with $var"; else fail "pre-commit was skipped by $var"; fi
done
git -C "$repo" reset -q bad2.txt; rm -f "$repo/bad2.txt"
if grep -nE 'SKIP|BYPASS|OVERRIDE|\$\{?[A-Z_]{4,}:?[-=]' "$ROOT/.githooks/pre-commit" "$ROOT/.githooks/pre-push" | grep -v 'TMPDIR' > /dev/null; then
  fail "a hook reads an environment variable that could switch it: $(grep -nE 'SKIP|BYPASS|OVERRIDE' "$ROOT"/.githooks/pre-* | head -2)"
else pass "neither hook reads any environment variable (no options, docs/policies: no-options)"; fi

# ---------------- pre-push
remote="$sb/remote.git"; git init -q --bare "$remote"
push_repo="$sb/push"; make_repo_copy "$push_repo"
echo "content" > "$push_repo/a.txt"
git -C "$push_repo" add . > /dev/null 2>&1; git -C "$push_repo" commit -q -m init || { echo "setup failed" >&2; exit 2; }
# The hook reads the URL git passes as $2 = where the push really goes. To test the GitHub branch without a network, the push goes through a wrapper
# that runs the hook by hand with a github.com URL, exactly as git would call it for a github.com remote (name, url; refs on stdin).
git -C "$push_repo" remote add origin "$remote"
push() { # FAKE_GH value -> runs the pre-push hook as git would for https://github.com/fake-owner/fake-repo.git, then pushes if it accepted
  local sha
  sha=$(git -C "$push_repo" rev-parse HEAD)
  if ! out_hook=$(cd "$push_repo" && printf 'refs/heads/main %s refs/heads/main 0000000000000000000000000000000000000000\n' "$sha" \
      | PATH="$FAKE:$PATH" FAKE_GH="$1" bash .githooks/pre-push origin https://github.com/fake-owner/fake-repo.git 2>&1); then
    printf '%s\n' "$out_hook"; return 1
  fi
  (cd "$push_repo" && git push --no-verify origin main 2>&1)
}

out=$(push private); rc=$?
expect_eq "pre-push lets a push to a PRIVATE remote through" 0 "$rc"
expect_eq "(the push really happened, into the local bare repository)" 1 "$(git -C "$remote" rev-list --count main 2> /dev/null)"

echo "second" > "$push_repo/b.txt"; git -C "$push_repo" add b.txt; git -C "$push_repo" commit -q -m second
out=$(push public); rc=$?
expect_eq "pre-push REJECTS a push to a PUBLIC remote" 1 "$rc"
expect_contains "the reason says the remote is public" "$out" "is PUBLIC"
expect_contains "the reason names the human-only way" "$out" "git push --no-verify"
expect_eq "(nothing was pushed)" 1 "$(git -C "$remote" rev-list --count main)"
out=$(push fail); rc=$?
expect_eq "pre-push REJECTS when the visibility cannot be read (treated as public)" 1 "$rc"
expect_contains "and says so" "$out" "Treated as PUBLIC"
for var in SKIP_SCAN=1 ALLOW_PUBLIC=1 ALLOW_PUSH=1 FORCE=1 PUSH_PUBLIC=1; do
  (export "${var?}"; push public > /dev/null 2>&1); rc=$?
  if [ "$rc" -ne 0 ]; then pass "pre-push still rejects a public remote with $var"; else fail "pre-push was bypassed by $var"; fi
done
(cd "$push_repo" && PATH="$FAKE:$PATH" FAKE_GH=public git push --no-verify origin main > /dev/null 2>&1); rc=$?
expect_eq "git push --no-verify is the human's way: it works against the real hook (the hook is not run)" 0 "$rc"
expect_eq "(and the second commit arrived)" 2 "$(git -C "$remote" rev-list --count main)"

# a dirty tracked file blocks the push even to a private remote
echo "uses the $v1$v2 api" > "$push_repo/dirty.txt"; git -C "$push_repo" add dirty.txt
git -C "$push_repo" -c core.hooksPath=/dev/null commit -q -m "dirty (hooks off for the setup only)"
out=$(push private); rc=$?
expect_eq "pre-push rejects tracked files that contain a forbidden string" 1 "$rc"
expect_contains "and names rule and file" "$out" "tracked files contain forbidden strings"

# a non-GitHub remote cannot be checked: rejected. This one is a REAL `git push` through the installed hook (core.hooksPath), not the wrapper.
other="$sb/push2"; make_repo_copy "$other"; echo c > "$other/c.txt"; git -C "$other" add . > /dev/null 2>&1; git -C "$other" commit -q -m init
git -C "$other" remote add origin "$remote"
out=$(cd "$other" && PATH="$FAKE:$PATH" FAKE_GH=private git push origin main 2>&1); rc=$?
expect_eq "pre-push rejects a remote that is not a GitHub URL (visibility cannot be checked)" 1 "$rc"
expect_contains "and says how a human can proceed" "$out" "git push --no-verify"

# fail closed without the scanner
git -C "$push_repo" reset -q --hard HEAD~1 2> /dev/null   # drop the dirty commit first (a reset would also restore the file moved below)
mv "$push_repo/scripts/scan-forbidden.sh" "$push_repo/scripts/scan-forbidden.sh.off"
out=$(push private); rc=$?
expect_eq "pre-push fails closed when scripts/scan-forbidden.sh is missing" 1 "$rc"
mv "$push_repo/scripts/scan-forbidden.sh.off" "$push_repo/scripts/scan-forbidden.sh"

# the SSH and token-in-URL forms of a GitHub remote are recognised (the URL parser of the hook)
parse() { printf '%s' "$1" | sed -nE 's#^(https://([^/@]+@)?github\.com/|git@github\.com:|ssh://git@github\.com/)([^/]+/[^/]+)$#\3#p' | sed -E 's#\.git$##'; }
expect_eq "URL form https" "o/r" "$(parse https://github.com/o/r.git)"
expect_eq "URL form https without .git" "o/r" "$(parse https://github.com/o/r)"
expect_eq "URL form ssh scp-like" "o/r" "$(parse git@github.com:o/r.git)"
expect_eq "URL form ssh://" "o/r" "$(parse ssh://git@github.com/o/r.git)"
expect_eq "URL form https with user" "o/r" "$(parse https://x-access@github.com/o/r.git)"
expect_eq "a non-GitHub URL gives nothing" "" "$(parse https://gitlab.com/o/r.git)"
if grep -q 'https://(\[^/@\]+@)?github\\.com/|git@github\\.com:|ssh://git@github\\.com/' "$ROOT/.githooks/pre-push"; then pass "the parser used in this test is the one in .githooks/pre-push"; else fail "the URL parser in .githooks/pre-push changed: update this test"; fi

finish
