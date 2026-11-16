#!/usr/bin/env bash
# usage: ci/agents/plan.sh <before-sha> <after-sha>
# Lists the task files ci/tasks/<name>.md that the push added or changed (all of them when before is all zeros: a new branch).
# Writes tasks=<JSON array of names> and count=<n> to $GITHUB_OUTPUT (stdout when it is unset). At most 60 names; the rest is not run, and a warning says so.
set -euo pipefail
before=${1:?usage: ci/agents/plan.sh <before-sha> <after-sha>}
after=${2:?usage: ci/agents/plan.sh <before-sha> <after-sha>}
max=60
if [[ "$before" =~ ^0+$ ]]; then
  changed=$(git ls-tree -r --name-only "$after" -- ci/tasks)
else
  changed=$(git diff --name-only --diff-filter=AM "$before" "$after" -- ci/tasks)
fi
names=$(printf '%s\n' "$changed" | sed -n 's#^ci/tasks/\([^/]*\)\.md$#\1#p' | sort -u)
bad=$(printf '%s\n' "$names" | grep -v -E '^[A-Za-z0-9][A-Za-z0-9._-]{0,62}$' | grep . || true)
if [ -n "$bad" ]; then
  echo "::error::タスクのファイル名が使えない形式(英数字・ . _ - のみ。先頭は英数字。63 文字まで): $(printf '%s' "$bad" | paste -sd' ' -)"
  exit 1
fi
total=$(printf '%s\n' "$names" | grep -c . || true)
selected=$(printf '%s\n' "$names" | grep . | head -n "$max" || true)
count=$(printf '%s\n' "$selected" | grep -c . || true)
if [ "$total" -gt "$max" ]; then
  echo "::warning::タスクが $total 件。先頭 $max 件だけを実行し、残り $((total - max)) 件は実行しない(実行するには、そのファイルを変更して push し直す)"
fi
tasks=$(printf '%s\n' "$selected" | jq -R -s -c 'split("\n") | map(select(length > 0))')
result=$(printf 'tasks=%s\ncount=%s\n' "$tasks" "$count")
if [ -n "${GITHUB_OUTPUT:-}" ]; then printf '%s\n' "$result" >> "$GITHUB_OUTPUT"; else printf '%s\n' "$result"; fi
echo "plan: $count task(s) to run, from $total changed" >&2
