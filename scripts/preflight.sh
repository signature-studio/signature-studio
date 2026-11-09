#!/usr/bin/env bash
# usage: scripts/preflight.sh        (no argument)
# Prints a table: item / result / OK|WARN|FAIL. Exit 1 when any item is FAIL; WARN does not fail.
# Read-only. It does not read ~/.config/opencode/ (except goal-nudge.sessions) and never prints a token, an account id or an e-mail address.
# Item 1 is the one that matters most: a typo in opencode.jsonc silently removes every deny (fail-open). It runs `opencode debug agents` in the
# repository and compares the answer with opencode.jsonc. Items 5 and 6 are WARN only: they are blockers for a human, never a reason to stop.
set -o pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd) || { echo "preflight: cannot resolve the repository root" >&2; exit 2; }
cd "$root" || { echo "preflight: cannot enter $root" >&2; exit 2; }

fails=0
row() { # item status text...
  local item=$1 status=$2
  shift 2
  printf '%-14s %-5s %s\n' "$item" "$status" "$*"
  [ "$status" = FAIL ] && fails=$((fails + 1))
  return 0
}

printf '%-14s %-5s %s\n' ITEM RESULT DETAIL

# 1. agents ----------------------------------------------------------------------------------------------------------
agents_json=""
# shellcheck source=/dev/null
. "$root/scripts/env.sh" > /dev/null 2>&1
if ! command -v opencode > /dev/null; then
  row agents FAIL "opencode is not on PATH"
elif ! agents_json=$(opencode debug agents 2> /dev/null) || ! jq -e 'type=="array"' > /dev/null 2>&1 <<< "$agents_json"; then
  row agents FAIL "opencode debug agents failed or returned no list (run it by hand and read the error)"
else
  python3 - "$root/opencode.jsonc" << 'PY' > "${TMPDIR:-/tmp}/preflight-expect.$$" 2> /dev/null || echo "{}" > "${TMPDIR:-/tmp}/preflight-expect.$$"
import json, re, sys
t = open(sys.argv[1], encoding="utf-8").read()
out, i, n, ins = [], 0, len(t), False
while i < n:
    c = t[i]
    if ins:
        out.append(c)
        if c == "\\":
            out.append(t[i + 1]); i += 2; continue
        if c == '"': ins = False
        i += 1; continue
    if c == '"': ins = True; out.append(c); i += 1; continue
    if t.startswith("//", i):
        while i < n and t[i] != "\n": i += 1
        continue
    if t.startswith("/*", i):
        i = t.index("*/", i + 2) + 2; continue
    out.append(c); i += 1
d = json.loads(re.sub(r",(\s*[}\]])", r"\1", "".join(out)))
print(json.dumps({a: d["agents"][a].get("model", "") for a in ("build", "worker", "reviewer")}))
PY
  expect_file="${TMPDIR:-/tmp}/preflight-expect.$$"
  problems=$(jq -r --slurpfile exp "$expect_file" '
    def m: if .model == null then "" else (.model.providerID + "/" + .model.id + (if .model.variant then "#" + .model.variant else "" end)) end;
    ([.[] | .id] | sort) as $ids
    | ($exp[0]) as $e
    | [ (if $ids != ["build","compaction","reviewer","summary","title","worker"] then "agent ids are [" + ($ids | join(",")) + "], expected [build,compaction,reviewer,summary,title,worker]" else empty end),
        (.[] | select(.id == "build" or .id == "worker" or .id == "reviewer") | . as $a
          | if ($e[$a.id] // "") == "" then "opencode.jsonc has no model for " + $a.id
            elif ($a | m) != $e[$a.id] then $a.id + " model is \"" + ($a | m) + "\", opencode.jsonc says \"" + $e[$a.id] + "\"" else empty end),
        (.[] | select(.id == "build") | if ([.permissions[] | select(.action == "*" and .effect == "deny")] | length) == 0 then "build has no `* deny` rule (the main session is not restricted)" else empty end),
        (.[] | select(.id == "worker") | if ([.permissions[] | select(.action == "shell" and .resource == "sleep *" and .effect == "deny")] | length) == 0 then "worker has no `sleep *` deny" else empty end),
        (.[] | select(.id == "reviewer") | if ([.permissions[] | select(.action == "edit" and .effect == "deny")] | length) == 0 then "reviewer can edit" else empty end)
      ] | .[]' <<< "$agents_json" 2>&1)
  rm -f "$expect_file"
  if [ -n "$problems" ]; then
    row agents FAIL "$(printf '%s' "$problems" | paste -sd';' - | cut -c1-300). The configuration may have been silently dropped: check opencode.jsonc for a missing quote or comma."
  else
    summary=$(jq -r '[.[] | select(.id == "build" or .id == "worker" or .id == "reviewer") | .id + "=" + (.model.id // "?") + "#" + (.model.variant // "")] | join(" ")' <<< "$agents_json")
    row agents OK "ids build,compaction,reviewer,summary,title,worker; $summary; no general/explore/plan"
  fi
fi

# 2. git identity -----------------------------------------------------------------------------------------------------
gname=$(git config --local user.name 2> /dev/null)
gmail=$(git config --local user.email 2> /dev/null)
if [ -n "$gname" ] && [ -n "$gmail" ]; then
  row git-identity OK "local user.name and user.email are set (name: $gname)"
else
  row git-identity FAIL "repository-local user.name/user.email not set: run scripts/setup-repo.sh"
fi
hooks=$(git config --local core.hooksPath 2> /dev/null)
if [ "$hooks" = ".githooks" ] && [ -x .githooks/pre-commit ] && [ -x .githooks/pre-push ]; then
  row git-hooks OK "core.hooksPath=.githooks, pre-commit and pre-push are executable"
else
  row git-hooks FAIL "core.hooksPath is '$hooks' (expected .githooks) or a hook is missing/not executable: run scripts/setup-repo.sh"
fi

# 3-6. GitHub ----------------------------------------------------------------------------------------------------------
remote=$(git remote get-url origin 2> /dev/null)
owner_repo=$(printf '%s' "$remote" | sed -E 's#^(https://github.com/|git@github.com:|ssh://git@github.com/)##; s#\.git$##')
if ! command -v gh > /dev/null; then
  row gh-identity WARN "gh is not installed"
elif gh_status=$(gh auth status 2>&1); then
  active=$(printf '%s\n' "$gh_status" | grep -iE 'Logged in to .* account' | head -1 | sed -E 's/.*account ([^ ]+).*/\1/')
  row gh-identity OK "active account: ${active:-unknown} (token not shown)"
else
  row gh-identity WARN "gh auth status failed: $(printf '%s' "$gh_status" | head -1 | cut -c1-120)"
fi

private=""
if [ -z "$owner_repo" ] || [[ "$owner_repo" != */* ]]; then
  row repo-api WARN "no usable origin remote ($remote)"
elif api=$(gh api "repos/$owner_repo" --jq '{private,permissions}' 2>&1); then
  private=$(jq -r '.private' <<< "$api" 2> /dev/null)
  perms=$(jq -c '.permissions // {}' <<< "$api" 2> /dev/null)
  row repo-api OK "$owner_repo private=$private permissions=$perms"
else
  row repo-api WARN "gh api repos/$owner_repo failed: $(printf '%s' "$api" | head -1 | cut -c1-120)"
fi

# --no-verify only for this dry-run: nothing is sent, and without it the pre-push hook (which refuses PUBLIC remotes) would answer before git asks the server about permission.
if push_out=$(git push --dry-run --no-verify origin HEAD 2>&1); then
  row push-dry-run OK "git push --dry-run origin HEAD succeeded (permission is there; whether to push is decided by the visibility row)"
elif printf '%s' "$push_out" | grep -qE '403|denied|Permission'; then
  row push-dry-run WARN "403: no push permission. Human-only blocker: add one line to docs/BLOCKERS.md (status, this command, date, effect) and continue everything except push"
elif printf '%s' "$push_out" | grep -qiE 'no commits|src refspec|does not match any'; then
  row push-dry-run WARN "nothing to push yet ($(printf '%s' "$push_out" | head -1 | cut -c1-100))"
else
  row push-dry-run WARN "dry-run failed: $(printf '%s' "$push_out" | head -1 | cut -c1-140). If the cause is not a 403, record it in docs/BLOCKERS.md"
fi

case "$private" in
  false) row visibility WARN "repository is PUBLIC: do not git push (lift output, wasm, FL DLLs and investigation excerpts would be published). Commit and continue" ;;
  true) row visibility OK "repository is private" ;;
  *) row visibility WARN "visibility unknown (item repo-api failed); treat as PUBLIC: do not git push" ;;
esac

# 7. wrangler ---------------------------------------------------------------------------------------------------------
if ! command -v wrangler > /dev/null; then
  row wrangler WARN "wrangler is not on PATH"
elif who=$(wrangler whoami 2>&1) && printf '%s' "$who" | grep -qiE 'logged in|associated with'; then
  row wrangler OK "authenticated (account id and e-mail not shown)"
else
  row wrangler WARN "not authenticated or whoami failed ($(printf '%s' "$who" | grep -v '^$' | head -1 | cut -c1-80))"
fi

# 8. toolchain (after sourcing scripts/env.sh above) --------------------------------------------------------------------
missing=""
detail=""
for tool in emcc wasm-opt node cargo clang google-chrome npx; do
  if command -v "$tool" > /dev/null; then
    case "$tool" in
      wasm-opt) v=$(wasm-opt --version 2> /dev/null | grep -oE 'version [0-9]+' | head -1) ;;
      emcc) v=$(emcc --version 2> /dev/null | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1) ;;
      node) v=$(node --version 2> /dev/null) ;;
      cargo) v=$(cargo --version 2> /dev/null | cut -d' ' -f2) ;;
      clang) v=$(clang --version 2> /dev/null | grep -oE 'version [0-9]+' | head -1) ;;
      google-chrome) v=$(google-chrome --version 2> /dev/null | grep -oE '[0-9]+(\.[0-9]+)*' | head -1) ;;
      *) v=ok ;;
    esac
    detail="$detail $tool=${v:-ok}"
  else
    missing="$missing $tool"
  fi
done
if [ -n "$missing" ]; then
  row toolchain FAIL "missing after sourcing scripts/env.sh:$missing (see docs/ops/environment.md)"
elif [ "$(wasm-opt --version 2> /dev/null | grep -oE 'version [0-9]+' | head -1)" != "version 133" ]; then
  row toolchain WARN "wasm-opt is not version 133 (the build expects 133):$detail"
else
  row toolchain OK "${detail# }"
fi

# 9. disk -----------------------------------------------------------------------------------------------------------
# Where the work happens: the repository (and TMPDIR from scripts/env.sh, which lives under the home directory). /tmp is checked separately because it is a small tmpfs.
work_free=$(df -BG --output=avail "$root" 2> /dev/null | tail -1 | tr -dc '0-9')
tmp_free=$(df -BG --output=avail /tmp 2> /dev/null | tail -1 | tr -dc '0-9')
tmp_fs=$(df --output=fstype /tmp 2> /dev/null | tail -1 | tr -d ' ')
if [ -z "$work_free" ]; then
  row disk WARN "df failed"
elif [ "$work_free" -lt 50 ]; then
  row disk FAIL "only ${work_free} GiB free where the repository is (need 50+). Clean generated output you created (lift/out*, target/)"
elif [ "${tmp_fs:-}" = tmpfs ] && [ -n "$tmp_free" ] && [ "$tmp_free" -lt 2 ]; then
  row disk WARN "repository ${work_free} GiB free; /tmp is tmpfs with only ${tmp_free} GiB free: use TMPDIR from scripts/env.sh"
else
  row disk OK "repository ${work_free} GiB free; /tmp ${tmp_free:-?} GiB free (${tmp_fs:-?}); TMPDIR=\$HOME/.cache/signature-studio/tmp via scripts/env.sh"
fi

# 10. forbidden strings -------------------------------------------------------------------------------------------------
if scan=$(bash "$root/scripts/scan-forbidden.sh" 2>&1); then
  row scan OK "$(printf '%s' "$scan" | tail -1)"
else
  row scan FAIL "$(printf '%s\n' "$scan" | grep -c . | tr -d ' ') line(s) of findings. Run scripts/scan-forbidden.sh; fix the files (values are not printed)"
fi

# 11. goal and goal-nudge ---------------------------------------------------------------------------------------------
sid="${OPENCODE_SESSION_ID:-}"
if [ -z "$sid" ]; then
  row goal WARN "OPENCODE_SESSION_ID is not set (run inside a session to see the goal)"
  row goal-nudge WARN "OPENCODE_SESSION_ID is not set"
else
  if g=$(opencode api POST /api/rpc/opencode.goal/get --data "{\"input\":{\"sessionID\":\"$sid\"}}" 2> /dev/null); then
    st=$(jq -r '.output.status // "none"' <<< "$g" 2> /dev/null)
    case "$st" in
      active) row goal OK "goal is active" ;;
      paused) row goal WARN "goal is PAUSED (reason not read on purpose): resume it with the command in AGENTS.md section 2" ;;
      complete) row goal OK "goal is complete" ;;
      none) row goal WARN "no goal is set for this session (launch.sh sets one)" ;;
      *) row goal WARN "unknown goal status: $st" ;;
    esac
  else
    row goal WARN "could not read the goal (opencode api failed)"
  fi
  reg="$HOME/.config/opencode/goal-nudge.sessions"
  if [ ! -r "$reg" ]; then
    row goal-nudge WARN "registry not readable: $reg"
  elif grep -qE "^[[:space:]]*$sid([[:space:]]|$)" "$reg"; then
    row goal-nudge OK "this session is in the goal-nudge registry"
  else
    row goal-nudge WARN "this session is not in the goal-nudge registry (scripts/launch.sh adds it); a paused goal is not resumed by the timer"
  fi
fi

echo
if [ "$fails" -gt 0 ]; then
  echo "preflight: $fails FAIL" >&2
  exit 1
fi
echo "preflight: no FAIL"
