#!/usr/bin/env bash
# usage: OPENCODE_PROVIDER=<提供元> ci/agents/runner-config.sh [PATH]     (CI, after import-login.sh. Default PATH: .opencode/opencode.jsonc, which is git-ignored)
# Writes the model settings of the CI runner: for each agent a run can use, the one model of the provider that the Variable OPENCODE_PROVIDER names and whose id contains haiku-5-5 (decision 0388).
# The owner sets the Variable (Issue 0007). Exits 1 when it is empty, or when the provider does not list exactly one such model. Prints counts only: no provider name and no model id.
# `opencode models --standalone` runs a private server, so it does not start the background service: that service keeps the config it read at its start (decision 0384).
# The workflow runs this before the first call of the background service (check-agents.sh).
# The worker is the top-level agent of `opencode run` (default_agent, and mode "all" so that it may be primary).
# Permissions are not written here: opencode.jsonc in the repository holds them. Copying them would put an allow rule after the denies (the last matching rule wins).
set -eu
out=${1:-.opencode/opencode.jsonc}
prov=${OPENCODE_PROVIDER:-}
if [ -z "$prov" ]; then echo "::error::提供元の指定(Variable OPENCODE_PROVIDER)が空。Issue 0007 を参照"; exit 1; fi
case "$prov" in */*|*'#'*|*[[:space:]]*) echo "::error::Variable OPENCODE_PROVIDER が 1 語でない。Issue 0007 を参照"; exit 1 ;; esac
list=$(opencode models --standalone 2> /dev/null) || { echo "::error::モデルの一覧を出せなかった(出力は伏せる)"; exit 1; }
cands=$(printf '%s\n' "$list" | awk -v p="$prov" 'index($0, p "/") == 1 && index($0, "haiku-5-5") > 0')
n=$(printf '%s\n' "$cands" | awk 'NF' | wc -l)
echo "提供元の指定の掲載で作業用の型に当たる候補: $n 件"
if [ "$n" -ne 1 ]; then echo "::error::候補が $n 件(1 件でない)。提供元の指定(Variable OPENCODE_PROVIDER)を確かめる。Issue 0007 を参照"; exit 1; fi
ref=$(printf '%s\n' "$cands" | awk 'NF')
case "$ref" in *'#'*) echo "::error::候補に #(variant)が付いている。Issue 0007 を参照"; exit 1 ;; esac
case "$ref" in *claude-haiku-5-5) ;; *) echo "::error::候補の末尾が想定と違う。Issue 0007 を参照"; exit 1 ;; esac
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
