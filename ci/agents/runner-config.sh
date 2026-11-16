#!/usr/bin/env bash
# usage: ci/agents/runner-config.sh [PATH]     (CI, after import-login.sh. Default PATH: .opencode/opencode.jsonc, which is git-ignored)
# Writes the model settings of the CI runner: for each agent a run can use, one model that the imported login lists under a console- provider, whose id ends with haiku-5-5 (decisions 0388, 0399).
# When two or more such models are listed, the first in byte order is used (decision 0399, amended). Exits 1 when none is listed. No Variable is read: a Variable's value is printed in the public log (decision 0399).
# Prints counts only: no provider name and no model id.
# `opencode models --standalone` runs a private server, so it does not start the background service: that service keeps the config it read at its start (decision 0384).
# The workflow runs this before the first call of the background service (check-agents.sh).
# The worker is the top-level agent of `opencode run` (default_agent, and mode "all" so that it may be primary).
# Permissions are not written here: opencode.jsonc in the repository holds them. Copying them would put an allow rule after the denies (the last matching rule wins).
set -eu
out=${1:-.opencode/opencode.jsonc}
list=$(opencode models --standalone 2> /dev/null) || { echo "::error::モデルの一覧を出せなかった(出力は伏せる)"; exit 1; }
cands=$(printf '%s\n' "$list" | awk 'index($0, "console-") == 1 && $0 ~ /haiku-5-5$/')
n=$(printf '%s\n' "$cands" | awk 'NF' | wc -l)
echo "console- の提供元で作業用の型の候補(型名の末尾が一致する行): $n 件"
if [ "$n" -eq 0 ]; then echo "::error::候補が 0 件"; exit 1; fi
if [ "$n" -gt 1 ]; then echo "2 件以上のため、辞書順で先頭の 1 件を使う"; fi
ref=$(printf '%s\n' "$cands" | awk 'NF' | LC_ALL=C sort | head -n 1)
case "$ref" in *'#'*) echo "::error::候補に #(variant)が付いている"; exit 1 ;; esac
echo "::add-mask::$ref"
mkdir -p "$(dirname "$out")"
cat > "$out" << EOF
{
  "model": "$ref",
  "default_agent": "worker",
  "agents": {
    "build": { "model": "$ref" },
    "worker": { "mode": "all", "model": "$ref" },
    "reviewer": { "model": "$ref" },
    "title": { "model": "$ref" },
    "summary": { "model": "$ref" }
  }
}
EOF
jq -e . "$out" > /dev/null
echo "runner config written to $out (model settings of 5 agents; the permissions come from opencode.jsonc)"
