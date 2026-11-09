#!/usr/bin/env bash
# t07: scripts/preflight.sh row by row. gh, wrangler, opencode (goal rows) and `git push` are test doubles (tests/harness/fakebin*): no GitHub, no Cloudflare, no server.
# `opencode debug agents` answers with a file produced once by the real resolver in an isolated sandbox (so item 1 is tested with real output).
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
set -o pipefail

sb=$(new_sandbox t07)
v1="anthr"; v2="opic"
FAKE="$HERE/fakebin"; FAKEOC="$HERE/fakebin-opencode"

# real resolver output for the good file (isolated sandbox; the isolated server is stopped again)
if command -v opencode > /dev/null; then
  mkdir -p "$sb/res/proj" "$sb/res/xdg"; cp "$ROOT/opencode.jsonc" "$sb/res/proj/"
  (
    export XDG_CONFIG_HOME="$sb/res/xdg/config" XDG_DATA_HOME="$sb/res/xdg/data" XDG_STATE_HOME="$sb/res/xdg/state" XDG_CACHE_HOME="$sb/res/xdg/cache"
    mkdir -p "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$XDG_STATE_HOME" "$XDG_CACHE_HOME"
    cd "$sb/res/proj" && opencode --standalone debug agents > "$sb/agents.json" 2> /dev/null
    opencode service stop > /dev/null 2>&1
  )
fi
[ -s "$sb/agents.json" ] || { skip "no real resolver output: preflight tests need opencode"; finish; exit $?; }

repo="$sb/repo"; make_repo_copy "$repo"
# a local bare remote named origin with a github-looking URL is not needed: preflight only reads `git remote get-url origin`
git -C "$repo" remote add origin "https://github.com/fake-owner/fake-repo.git"
echo x > "$repo/a.txt"; git -C "$repo" add . > /dev/null 2>&1; git -C "$repo" commit -q -m init

pf() { # extra env assignments are given as arguments: pf VAR=value ...
  (cd "$repo" && env FAKE_GIT_PUSH=ok FAKE_DF_REPO_GIB=600 "$@" PATH="$FAKEOC:$FAKE:$PATH" FAKE_OC_AGENTS="$sb/agents.json" FAKE_OC_LOG="$sb/oc.log" bash scripts/preflight.sh 2> "$sb/stderr.txt")
}
rowof() { awk -v k="$1" '$1 == k' <<< "$2"; }
status() { rowof "$1" "$2" | awk '{print $2}'; }

# ---------- a normal run (private repo, push permitted by the dry-run fake, no session)
out=$(pf FAKE_GH=private); rc=$?
header=$(head -1 <<< "$out")
expect_contains "the table has a header with ITEM RESULT DETAIL" "$header" "ITEM"
for item in agents git-identity git-hooks gh-identity repo-api push-dry-run visibility wrangler toolchain disk scan goal goal-nudge; do
  if [ -n "$(rowof "$item" "$out")" ]; then pass "row present: $item"; else fail "row missing: $item"; fi
done
fails=$(awk '$2 == "FAIL"' <<< "$out" | wc -l | tr -d ' ')
if [ "$fails" -eq 0 ]; then expect_eq "no FAIL row -> exit 0" 0 "$rc"; else expect_eq "FAIL rows ($(awk '$2=="FAIL"{print $1}' <<< "$out" | paste -sd, -)) -> exit 1" 1 "$rc"; fi
expect_eq "agents OK" OK "$(status agents "$out")"
expect_eq "git-identity OK (set by setup-repo.sh)" OK "$(status git-identity "$out")"
expect_eq "git-hooks OK" OK "$(status git-hooks "$out")"
expect_eq "gh-identity OK" OK "$(status gh-identity "$out")"
expect_contains "gh-identity shows the active account name" "$(rowof gh-identity "$out")" "fake-bot[bot]"
expect_eq "repo-api OK" OK "$(status repo-api "$out")"
expect_contains "repo-api shows private and permissions" "$(rowof repo-api "$out")" 'private=true permissions={"admin":false'
expect_eq "visibility OK for a private repository" OK "$(status visibility "$out")"
expect_eq "wrangler OK" OK "$(status wrangler "$out")"
expect_eq "scan OK on the clean copy" OK "$(status scan "$out")"
expect_eq "goal is WARN without a session id" WARN "$(status goal "$out")"
expect_eq "goal-nudge is WARN without a session id" WARN "$(status goal-nudge "$out")"

# ---------- secrets and identifiers never appear
token="$(printf 'gho')_$(printf 'FAKE%.0s' 1 2 3 4 5 6 7 8)"
for needle in "$token" "fake.person@" "$(printf '0123456789abcdef%.0s' 1 2)" "Fake Person"; do
  if grep -qF -- "$needle" <<< "$out$(cat "$sb/stderr.txt")"; then fail "preflight printed a secret or identifier: ${needle:0:12}..."; else pass "preflight output does not contain: ${needle:0:12}..."; fi
done

# ---------- 403 on push: WARN, not FAIL, tells where to write it
out=$(pf FAKE_GH=private FAKE_GIT_PUSH=403); rc=$?
row=$(rowof push-dry-run "$out")
expect_eq "push 403 -> WARN (never FAIL: a human-only blocker does not stop the agent)" WARN "$(status push-dry-run "$out")"
expect_contains "the 403 row says docs/BLOCKERS.md" "$row" "docs/BLOCKERS.md"
if [ "$(awk '$2=="FAIL"' <<< "$out" | wc -l | tr -d ' ')" -eq 0 ]; then expect_eq "a 403 alone does not make preflight exit 1" 0 "$rc"; else skip "exit code with a 403 (another row is FAIL on this machine: $(awk '$2=="FAIL"{print $1}' <<< "$out" | paste -sd, -))"; fi

# ---------- disk: FAIL under 50 GiB where the repository is, WARN when /tmp (tmpfs) is nearly full, OK otherwise
out=$(pf FAKE_GH=private FAKE_DF_REPO_GIB=40)
expect_eq "disk: 40 GiB free where the repository is -> FAIL" FAIL "$(status disk "$out")"
expect_contains "disk: the FAIL row says what to clean" "$(rowof disk "$out")" "lift/out*"
out=$(pf FAKE_GH=private FAKE_DF_REPO_GIB=600 FAKE_DF_TMP_GIB=1)
expect_eq "disk: /tmp tmpfs with 1 GiB free -> WARN" WARN "$(status disk "$out")"
expect_contains "disk: and points at TMPDIR from env.sh" "$(rowof disk "$out")" "TMPDIR"
out=$(pf FAKE_GH=private FAKE_DF_REPO_GIB=600 FAKE_DF_TMP_GIB=3)
expect_eq "disk: 600 GiB and /tmp 3 GiB -> OK" OK "$(status disk "$out")"

# ---------- PUBLIC repository: WARN with the do-not-push note
out=$(pf FAKE_GH=public)
expect_eq "public repo -> visibility WARN" WARN "$(status visibility "$out")"
expect_contains "and says do not git push" "$(rowof visibility "$out")" "do not git push"
expect_contains "repo-api shows private=false" "$(rowof repo-api "$out")" "private=false"

# ---------- gh failing: WARN rows, visibility treated as public
out=$(pf FAKE_GH=fail)
expect_eq "gh failing -> gh-identity WARN" WARN "$(status gh-identity "$out")"
expect_eq "gh failing -> repo-api WARN" WARN "$(status repo-api "$out")"
expect_eq "gh failing -> visibility WARN (unknown is treated as public)" WARN "$(status visibility "$out")"
expect_contains "and says so" "$(rowof visibility "$out")" "treat as PUBLIC"

# ---------- wrangler logged out
out=$(pf FAKE_GH=private FAKE_WRANGLER=out)
expect_eq "wrangler logged out -> WARN" WARN "$(status wrangler "$out")"

# ---------- git identity and hooks missing -> FAIL
bare="$sb/bare"; make_repo_copy "$bare"; git -C "$bare" remote add origin "https://github.com/fake-owner/fake-repo.git"
git -C "$bare" config --local --unset user.name; git -C "$bare" config --local --unset user.email; git -C "$bare" config --local --unset core.hooksPath
out=$(cd "$bare" && GIT_CONFIG_GLOBAL=/dev/null env FAKE_GIT_PUSH=ok FAKE_DF_REPO_GIB=600 PATH="$FAKEOC:$FAKE:$PATH" FAKE_OC_AGENTS="$sb/agents.json" FAKE_GH=private bash scripts/preflight.sh 2> /dev/null); rc=$?
expect_eq "no local git identity -> FAIL" FAIL "$(status git-identity "$out")"
expect_contains "and tells to run setup-repo.sh" "$(rowof git-identity "$out")" "scripts/setup-repo.sh"
expect_eq "no hooksPath -> FAIL" FAIL "$(status git-hooks "$out")"
expect_eq "so preflight exits 1" 1 "$rc"

# ---------- forbidden string in the tree -> scan FAIL, exit 1, value not shown
dirty="$sb/dirty"; make_repo_copy "$dirty"; git -C "$dirty" remote add origin "https://github.com/fake-owner/fake-repo.git"
echo "uses the $v1$v2 api" > "$dirty/leak.txt"
out=$(cd "$dirty" && env FAKE_GIT_PUSH=ok FAKE_DF_REPO_GIB=600 PATH="$FAKEOC:$FAKE:$PATH" FAKE_OC_AGENTS="$sb/agents.json" FAKE_GH=private bash scripts/preflight.sh 2> /dev/null); rc=$?
expect_eq "a forbidden string in the tree -> scan FAIL" FAIL "$(status scan "$out")"
expect_eq "so preflight exits 1" 1 "$rc"
expect_not_contains "the value is not shown" "$out" "$v1$v2"

# ---------- goal rows (fake opencode answers the rpc)
reg_home="$sb/home"; mkdir -p "$reg_home/.config/opencode"
sid=ses_TESTSESSION01
goalrun() { # goal status, registry content
  printf '%s' "$2" > "$reg_home/.config/opencode/goal-nudge.sessions"
  (cd "$repo" && env FAKE_GIT_PUSH=ok FAKE_DF_REPO_GIB=600 HOME="$reg_home" OPENCODE_SESSION_ID="$sid" FAKE_GH=private FAKE_OC_GOAL="$1" PATH="$FAKEOC:$FAKE:$PATH" FAKE_OC_AGENTS="$sb/agents.json" FAKE_OC_LOG="$sb/oc.log" bash scripts/preflight.sh 2> /dev/null)
}
: > "$sb/oc.log"
out=$(goalrun active "$sid  # test
")
expect_eq "goal active -> OK" OK "$(status goal "$out")"
expect_eq "registered in goal-nudge.sessions -> OK" OK "$(status goal-nudge "$out")"
if grep -q 'POST /api/rpc/opencode.goal/get' "$sb/oc.log"; then pass "the goal is read with POST /api/rpc/opencode.goal/get"; else fail "goal/get was not called"; fi
out=$(goalrun paused "ses_OTHER
")
expect_eq "goal paused -> WARN" WARN "$(status goal "$out")"
expect_contains "and points at the resume command" "$(rowof goal "$out")" "AGENTS.md"
expect_eq "not in the registry -> WARN" WARN "$(status goal-nudge "$out")"
expect_contains "and says launch.sh adds it" "$(rowof goal-nudge "$out")" "launch.sh"
out=$(goalrun complete "$sid")
expect_eq "goal complete -> OK" OK "$(status goal "$out")"
expect_eq "registry line without a comment is found too" OK "$(status goal-nudge "$out")"
out=$(goalrun none "")
expect_eq "no goal -> WARN" WARN "$(status goal "$out")"
rm -f "$reg_home/.config/opencode/goal-nudge.sessions"
out=$(cd "$repo" && env FAKE_GIT_PUSH=ok FAKE_DF_REPO_GIB=600 HOME="$reg_home" OPENCODE_SESSION_ID="$sid" FAKE_GH=private PATH="$FAKEOC:$FAKE:$PATH" FAKE_OC_AGENTS="$sb/agents.json" bash scripts/preflight.sh 2> /dev/null)
expect_eq "missing registry file -> WARN, not a crash" WARN "$(status goal-nudge "$out")"
if grep -q 'sessionID.*ses_OTHER' "$sb/oc.log"; then fail "the goal was read for a session other than OPENCODE_SESSION_ID"; else pass "only the own session's goal is read"; fi

# ---------- static: what preflight may read
reads=$(grep -n '\.config/opencode' "$ROOT/scripts/preflight.sh" | grep -v '^[0-9]*:#' | cut -d: -f2- | sed 's/^ *//')
expect_contains "the only ~/.config/opencode path in preflight is the goal-nudge registry" "$reads" 'goal-nudge.sessions'
expect_eq "(and it appears once)" 1 "$(grep -v '^ *#' "$ROOT/scripts/preflight.sh" | grep -c '\.config/opencode')"
for pat in 'auth\.json' '\.local/share/botid' '\.config/botid' 'hosts\.yml' 'default\.toml' 'GH_CONFIG_DIR' 'cat .*\.config' '\.config/opencode/(opencode|auth|env)'; do
  if grep -v '^ *#' "$ROOT/scripts/preflight.sh" | grep -qE "$pat"; then fail "preflight touches: $pat"; else pass "preflight does not touch: $pat"; fi
done
if grep -q 'gh auth status' "$ROOT/scripts/preflight.sh" && ! grep -qE 'gh auth token|--show-token' "$ROOT/scripts/preflight.sh"; then pass "preflight calls gh auth status and never asks for a token"; else fail "preflight asks gh for a token"; fi
# nothing in preflight writes anywhere but its own temporary file (the expected models), which it removes again
writes=$(grep -v '^ *#' "$ROOT/scripts/preflight.sh" | grep -nE '>>|git (add|commit|config --local [a-z.]+ [^ ]+$)|systemctl|\brm |wrangler (deploy|delete|secret|put)|\btee\b|sed -i' | grep -vF "rm -f \"\$expect_file\"" || true)
if [ -z "$writes" ]; then pass "preflight is read-only (apart from its own temporary file)"; else fail "preflight may write: $(head -2 <<< "$writes")"; fi
if [ -z "$(ls "$repo"/.git/*.lock 2> /dev/null)" ]; then pass "preflight left no lock file in the repository"; else fail "a git lock file is left"; fi

finish
