#!/usr/bin/env bash
# usage: ci/agents/run-task.sh <task>     (CI, in the repository root. One task file = one run of the worker)
# Writes ci/agents/out/<task>/{report.md,stderr.log,changes.patch,meta.txt}. The prompt (worker rules, CI notes, AGENTS.md, the task) is written outside the repository and is not kept.
# Exits 0 when the run finished, even if the worker's rc is not 0: meta.txt holds the rc, and the last workflow step fails on it.
set -u
task=${1:?usage: ci/agents/run-task.sh <task>}
case "$task" in ''|*[!A-Za-z0-9._-]*) echo "bad task name" >&2; exit 2 ;; esac
src="ci/tasks/$task.md"
out="ci/agents/out/$task"
[ -f "$src" ] || { echo "no $src" >&2; exit 2; }
tmp=${RUNNER_TEMP:-${TMPDIR:-/tmp}}
prompt="$tmp/prompt-$task.md"
mkdir -p "$tmp" "$out" || exit 2
if ! {
  cat prompts/worker-rules.md
  cat << 'NOTE'

## この実行の場(GitHub Actions のワークツリー)
- push はできない。commit もしない。変更はワークツリーに残す(後で差分を読む)。
- `source scripts/env.sh` は使えない(emsdk が無い)。一時ファイルは `$RUNNER_TEMP` に置く。
- Secrets・Variables・Cloudflare の資格情報は、この実行には無い。Issue は出せない。判断は「決定: X。根拠: Y」と書いて進める。
- 報告の先頭に、この実行の結論を 1 行で書く。

---

NOTE
  cat AGENTS.md
  printf '\n\n---\n\n## タスク(ci/tasks/%s.md)\n\n' "$task"
  cat "$src"
} > "$prompt"; then echo "cannot write the prompt to $prompt" >&2; exit 2; fi
start=$(date -u +%Y-%m-%dT%H:%M:%SZ)
opencode run "$(cat "$prompt")" > "$out/report.md" 2> "$out/stderr.log" < /dev/null
rc=$?
end=$(date -u +%Y-%m-%dT%H:%M:%SZ)
git add -N -- . ':(exclude)ci/agents/out'
git diff --no-color HEAD -- . ':(exclude)ci/agents/out' > "$out/changes.patch"
files=$(git diff --name-only HEAD -- . ':(exclude)ci/agents/out' | grep -c . || true)
{
  echo "task: $task"
  echo "rc: $rc"
  echo "started: $start"
  echo "ended: $end"
  echo "opencode: $(opencode --version 2> /dev/null | head -1)"
  echo "commit: $(git rev-parse --short HEAD)"
  echo "changed files: $files"
  echo "report bytes: $(wc -c < "$out/report.md")"
} > "$out/meta.txt"
rm -f "$prompt"
exit 0
