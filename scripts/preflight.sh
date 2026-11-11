#!/usr/bin/env bash
# usage: scripts/preflight.sh        (no argument)
# Prints a table: item / result / OK|WARN|FAIL. Exit 1 when any item is FAIL; WARN does not fail.
# Read-only. It does not read ~/.config/opencode/ (except goal-nudge.sessions) and never prints a token, an account id or an e-mail address.
# Item 1 is the one that matters most: a typo in opencode.jsonc silently removes every deny (fail-open). It runs `opencode debug agents` in the
# repository and compares the answer with opencode.jsonc. The WARN rows (GitHub, goal) are requests for a human (docs/ops/requests.md), never a reason to stop.
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
  # The effective model lines: opencode.jsonc with the local override (.opencode/opencode.jsonc, git-ignored) applied on top.
  python3 - "$root/opencode.jsonc" "$root/.opencode/opencode.jsonc" << 'PY' > "${TMPDIR:-/tmp}/preflight-expect.$$" 2> /dev/null || echo "{}" > "${TMPDIR:-/tmp}/preflight-expect.$$"
import json, re, sys
def load(path):
    t = open(path, encoding="utf-8").read()
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
    return json.loads(re.sub(r",(\s*[}\]])", r"\1", "".join(out)))
d = load(sys.argv[1])
want = {"top": d.get("model", ""), **{a: d["agents"][a].get("model", "") for a in ("build", "worker", "reviewer", "title", "summary")}}
try:
    o = load(sys.argv[2])
    if "model" in o: want["top"] = o["model"]
    for a, v in (o.get("agents") or {}).items():
        if isinstance(v, dict) and "model" in v and a in want: want[a] = v["model"]
except FileNotFoundError:
    pass
print(json.dumps(want))
PY
  expect_file="${TMPDIR:-/tmp}/preflight-expect.$$"
  problems=$(jq -r --slurpfile exp "$expect_file" '
    def m: if .model == null then "" else (.model.providerID + "/" + .model.id + (if .model.variant then "#" + .model.variant else "" end)) end;
    ([.[] | .id] | sort) as $ids
    | ($exp[0]) as $e
    | ([$e.top, $e.build, $e.worker, $e.reviewer, $e.title, $e.summary] | map(select(. != null and . != "") | split("/")[0]) | unique) as $provs
    | [ (if $ids != ["build","compaction","reviewer","summary","title","worker"] then "agent ids are [" + ($ids | join(",")) + "], expected [build,compaction,reviewer,summary,title,worker]" else empty end),
        (.[] | select(.id == "build" or .id == "worker" or .id == "reviewer" or .id == "title" or .id == "summary") | . as $a
          | if ($e[$a.id] // "") == "" then "no model set for " + $a.id
            elif ($a | m) != $e[$a.id] then $a.id + " model is \"" + ($a | m) + "\", the configuration says \"" + $e[$a.id] + "\"" else empty end),
        (if ($provs | length) > 1 then "the six model settings use more than one provider (" + ($provs | join(", ")) + ")" else empty end),
        (if ([$e.worker, $e.reviewer, $e.title, $e.summary] | map((. // "") | test("claude-haiku-5-5")) | all) then empty
          else "サブエージェントの作業用・検証用、表題、要約のどれかが Haiku 5.5 ではありません" end),
        (.[] | select(.id == "build") | if ([.permissions[] | select(.action == "*" and .effect == "deny")] | length) == 0 then "build has no `* deny` rule (the main session is not restricted)" else empty end),
        (.[] | select(.id == "worker") | if ([.permissions[] | select(.action == "shell" and .resource == "sleep *" and .effect == "deny")] | length) == 0 then "worker has no `sleep *` deny" else empty end),
        (.[] | select(.id == "reviewer") | if ([.permissions[] | select(.action == "edit" and .effect == "deny")] | length) == 0 then "reviewer can edit" else empty end)
      ] | .[]' <<< "$agents_json" 2>&1)
  rm -f "$expect_file"
  if [ -n "$problems" ]; then
    row agents FAIL "$(printf '%s' "$problems" | paste -sd';' - | cut -c1-300). The configuration may have been silently dropped: check opencode.jsonc for a missing quote or comma."
  else
    summary=$(jq -r '[.[] | select(.id == "build" or .id == "worker" or .id == "reviewer") | .id + "=" + (.model.id // "?") + "#" + (.model.variant // "")] | join(" ")' <<< "$agents_json")
    row agents OK "ids build,compaction,reviewer,summary,title,worker; $summary; six settings on one provider; no general/explore/plan"
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

# 3-5. GitHub ----------------------------------------------------------------------------------------------------------
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

if [ -z "$owner_repo" ] || [[ "$owner_repo" != */* ]]; then
  row repo-api WARN "no usable origin remote ($remote)"
elif api=$(gh api "repos/$owner_repo" --jq .default_branch 2>&1) && [ -n "$api" ]; then
  row repo-api OK "reachable (default branch $api)"
else
  row repo-api WARN "gh api repos/$owner_repo failed: $(printf '%s' "$api" | head -1 | cut -c1-120)"
fi

# --no-verify only for this dry-run: nothing is sent, and the pre-push scan would otherwise answer before git asks the server about permission.
if push_out=$(git push --dry-run --no-verify origin HEAD 2>&1); then
  row push-dry-run OK "\`git push --dry-run origin HEAD\` succeeded: commit, then \`git push origin main\`"
elif printf '%s' "$push_out" | grep -qE '403|denied|Permission'; then
  row push-dry-run WARN "403. Check first: are you in the repository directory (identity depends on it)? \`gh auth status\`. If both are right, file a request (docs/ops/requests.md) and continue everything except push"
else
  row push-dry-run WARN "dry-run failed: $(printf '%s' "$push_out" | head -1 | cut -c1-140)"
fi

# 7. toolchain (after sourcing scripts/env.sh above) --------------------------------------------------------------------
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

# 8. disk -----------------------------------------------------------------------------------------------------------
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

# 9. forbidden strings -------------------------------------------------------------------------------------------------
if scan=$(bash "$root/scripts/scan-forbidden.sh" 2>&1); then
  row scan OK "$(printf '%s' "$scan" | tail -1)"
else
  row scan FAIL "$(printf '%s\n' "$scan" | grep -c . | tr -d ' ') line(s) of findings. Run scripts/scan-forbidden.sh; fix the files (values are not printed)"
fi

# 9b. consistency: documents that disagree with the repository (scripts/check-consistency.sh). A finding is a WARN, never a FAIL.
if cons=$(bash "$root/scripts/check-consistency.sh" 2> /dev/null); then
  cons_n=$(printf '%s\n' "$cons" | tail -1 | grep -oE '[0-9]+' | head -1)
  if [ "${cons_n:-1}" -eq 0 ]; then
    row consistency OK "0 finding(s) in the documents (scripts/check-consistency.sh)"
  else
    row consistency WARN "${cons_n} finding(s): fix them in this round (scripts/check-consistency.sh; first rows: $(printf '%s\n' "$cons" | sed -n '2,3p' | tr -s ' ' | paste -sd';' - | cut -c1-200))"
  fi
else
  row consistency WARN "scripts/check-consistency.sh did not run (see the script)"
fi

# 10. goal and goal-nudge ---------------------------------------------------------------------------------------------
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
