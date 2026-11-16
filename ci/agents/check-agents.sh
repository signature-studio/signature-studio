#!/usr/bin/env bash
# usage: ci/agents/check-agents.sh [PATH]   (CI, after runner-config.sh. PATH: the runner config, default .opencode/opencode.jsonc. Reads the agents that opencode actually uses; prints counts and OK lines only)
# Exits 1 when the runner config has no model, has a #variant, or has a model that does not end with the worker's model type (decision 0388); when an agent does not use the config's model;
# when the worker is not primary-capable; or when a deny rule for the worker is missing.
# A deny rule that is missing would be a silent fail-open (see the header of opencode.jsonc). Neither the model value nor the agent list is printed: the value holds the model id.
# This is the first call of the background service of the job (`opencode debug agents`): the config must be written before it (decision 0384).
set -eu
cfg=${1:-.opencode/opencode.jsonc}
want=$(jq -r '.model // ""' "$cfg" 2> /dev/null || true)
if [ -z "$want" ]; then echo "::error::実行用の設定にモデルが無い(runner-config.sh が書いていない)。Issue 0007 を参照"; exit 1; fi
case "$want" in *'#'*) echo "::error::設定のモデルに #(variant)が付いている"; exit 1 ;; esac
case "$want" in *claude-haiku-5-5) ;; *) echo "::error::設定のモデルの末尾が作業用の型と違う(決定 0388)"; exit 1 ;; esac
json=$(opencode debug agents) || { echo "::error::opencode debug agents が失敗した"; exit 1; }
bad=$(jq -r --arg want "$want" '
  .[] | select(.id == "build" or .id == "worker" or .id == "reviewer" or .id == "title" or .id == "summary")
  | select(((.model.providerID // "") + "/" + (.model.id // "")) != $want or (.model.variant // "") != "")
  | .id' <<< "$json" | paste -sd' ' -)
if [ -n "$bad" ]; then echo "::error::モデルが実行用の設定の値になっていない: $bad"; exit 1; fi
echo "モデル: build・worker・reviewer・title・summary の 5 件が実行用の設定の値と一致(variant なし)"
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
