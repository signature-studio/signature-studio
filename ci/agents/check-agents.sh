#!/usr/bin/env bash
# usage: ci/agents/check-agents.sh   (CI, after runner-config.sh. Reads the agents that opencode actually uses; prints counts and OK lines only)
# Exits 1 when a model is not the runner's Bedrock Haiku 5.5, the worker is not primary-capable, or a deny rule for the worker is missing.
# A deny rule that is missing would be a silent fail-open (see the header of opencode.jsonc). The agent list is not printed: it holds the model ids.
set -eu
a=anthr b=opic
want="amazon-bedrock/global.$a$b.claude-haiku-5-5"
json=$(opencode debug agents) || { echo "::error::opencode debug agents が失敗した"; exit 1; }
bad=$(jq -r --arg want "$want" '
  .[] | select(.id == "build" or .id == "worker" or .id == "reviewer" or .id == "title" or .id == "summary")
  | select(((.model.providerID // "") + "/" + (.model.id // "")) != $want or (.model.variant // "") != "")
  | .id' <<< "$json" | paste -sd' ' -)
if [ -n "$bad" ]; then echo "::error::モデルが Bedrock の Haiku 5.5 になっていない: $bad"; exit 1; fi
echo "モデル: build・worker・reviewer・title・summary の 5 件が Bedrock の Haiku 5.5(variant なし)"
if ! jq -e '.[] | select(.id == "worker") | select(.mode == "all" or .mode == "primary")' > /dev/null <<< "$json"; then
  echo "::error::worker が primary として動けない(mode が all でも primary でもない)"; exit 1
fi
echo "worker: 既定のエージェント(primary)として動ける"
missing=""
for r in "wrangler *" "cf *" "gh secret *" "gh variable *" "gh api *secrets*" "gh api *variables*" "git push --force*" "git clean *"; do
  jq -e --arg r "$r" '.[] | select(.id == "worker") | [.permissions[] | select(.action == "shell" and .effect == "deny" and .resource == $r)] | length > 0' > /dev/null <<< "$json" || missing="$missing [$r]"
done
if [ -n "$missing" ]; then echo "::error::worker に拒否の規則が無い:$missing"; exit 1; fi
echo "worker の拒否の規則: wrangler・cf・gh secret・gh variable・gh api(secrets/variables)・force push・git clean の 8 件を確認"
