#!/usr/bin/env bash
# usage: OPENCODE_MODEL=<提供元>/<モデル> ci/agents/runner-config.sh [PATH]     (CI. Default PATH: .opencode/opencode.jsonc, which is git-ignored)
# Writes the model settings of the CI runner: the one model that the Variable OPENCODE_MODEL names, for each agent a run can use (decision 0384).
# The owner sets the Variable (Issue 0006). There is no default: an empty Variable writes nothing, and check-agents.sh stops the job after the import (so the import output is still shown). No model id is written in the repository.
# The workflow runs this before the first opencode command of the job: opencode keeps the config it read when its server starts, so a file written later is not seen at once (measured, decision 0384).
# The worker is the top-level agent of `opencode run` (default_agent, and mode "all" so that it may be primary).
# Permissions are not written here: opencode.jsonc in the repository holds them. Copying them would put an allow rule after the denies (the last matching rule wins).
set -eu
out=${1:-.opencode/opencode.jsonc}
ref=${OPENCODE_MODEL:-}
if [ -z "$ref" ]; then
  echo "::notice::モデルの指定 Variable OPENCODE_MODEL が空。設定は書かない(check-agents.sh が、取り込みの後で止める。Issue 0006 を参照)"
  exit 0
fi
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
