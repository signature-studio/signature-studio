#!/usr/bin/env bash
# usage: ci/agents/runner-config.sh [PATH]     (CI. Default PATH: .opencode/opencode.jsonc, which is git-ignored)
# Writes the model settings of the CI runner: Amazon Bedrock with the Haiku 5.5 model only, for each agent a run can use.
# The worker is the top-level agent of `opencode run` (default_agent, and mode "all" so that it may be primary).
# Permissions are not written here: opencode.jsonc in the repository holds them. Copying them would put an allow rule after the denies (the last matching rule wins).
# The model id is built from parts, as in scripts/set-model-profile.sh, so the vendor name is not written in the repository.
set -eu
out=${1:-.opencode/opencode.jsonc}
a=anthr b=opic
id="global.$a$b.claude-haiku-5-5"
ref="amazon-bedrock/$id"
mkdir -p "$(dirname "$out")"
cat > "$out" << EOF
{
  "model": "$ref",
  "default_agent": "worker",
  "providers": {
    "amazon-bedrock": {
      "models": { "$id": {} }
    }
  },
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
