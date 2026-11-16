#!/usr/bin/env bash
# usage: ci/agents/runner-config.sh [PATH]     (CI, after import-login.sh. Default PATH: .opencode/opencode.jsonc, which is git-ignored)
# Writes the model settings of the CI runner: for each agent a run can use, the one model that the imported login lists under a console- provider, whose id ends with haiku-5-5 (decisions 0388 and 0399).
# No Variable is read: a Variable's value is printed in the public log (decision 0399). Exits 1 unless exactly one such model is listed. Prints counts only: no provider name and no model id.
# `opencode models --standalone` runs a private server, so it does not start the background service: that service keeps the config it read at its start (decision 0384).
# The workflow runs this before the first call of the background service (check-agents.sh).
# The worker is the top-level agent of `opencode run` (default_agent, and mode "all" so that it may be primary).
# Permissions are not written here: opencode.jsonc in the repository holds them. Copying them would put an allow rule after the denies (the last matching rule wins).
set -eu
out=${1:-.opencode/opencode.jsonc}
list=$(opencode models --standalone 2> /dev/null) || { echo "::error::モデルの一覧を出せなかった(出力は伏せる)"; exit 1; }
cands=$(printf '%s\n' "$list" | awk 'index($0, "console-") == 1 && $0 ~ /haiku-5-5$/')
n=$(printf '%s\n' "$cands" | awk 'NF' | wc -l)
echo "console- の提供元で作業用の型(末尾 haiku-5-5)の候補: $n 件"
if [ "$n" -ne 1 ]; then echo "::error::候補が $n 件(1 件でない。決定 0399)"; exit 1; fi
ref=$(printf '%s\n' "$cands" | awk 'NF')
case "$ref" in *'#'*) echo "::error::候補に #(variant)が付いている(決定 0399)"; exit 1 ;; esac
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
