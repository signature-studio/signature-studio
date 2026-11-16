#!/usr/bin/env bash
# usage: OPENCODE_MODEL=<提供元>/<モデル> ci/agents/check-agents.sh   (CI, after runner-config.sh. Reads the agents that opencode actually uses; prints counts and OK lines only)
# Exits 1 when the Variable OPENCODE_MODEL is empty, is not a Haiku 5.5 model (its model id must end with claude-haiku-5-5, with no #variant: decision 0384), when an agent does not use it,
# when the worker is not primary-capable, or when a deny rule for the worker is missing.
# A deny rule that is missing would be a silent fail-open (see the header of opencode.jsonc). The agent list is not printed: it holds the model ids.
set -eu
want=${OPENCODE_MODEL:-}
if [ -z "$want" ]; then echo "::error::モデルの指定 Variable OPENCODE_MODEL が空。Issue 0006 を参照"; exit 1; fi
case "$want" in *'#'*) echo "::error::Variable OPENCODE_MODEL に #(variant)を付けない"; exit 1 ;; esac
case "$want" in *claude-haiku-5-5) ;; *) echo "::error::Variable OPENCODE_MODEL の末尾が claude-haiku-5-5 でない(Haiku 5.5 だけを許す)"; exit 1 ;; esac
json=$(opencode debug agents) || { echo "::error::opencode debug agents が失敗した"; exit 1; }
bad=$(jq -r --arg want "$want" '
  .[] | select(.id == "build" or .id == "worker" or .id == "reviewer" or .id == "title" or .id == "summary")
  | select(((.model.providerID // "") + "/" + (.model.id // "")) != $want or (.model.variant // "") != "")
  | .id' <<< "$json" | paste -sd' ' -)
if [ -n "$bad" ]; then echo "::error::モデルが Variable OPENCODE_MODEL の値になっていない: $bad"; exit 1; fi
echo "モデル: build・worker・reviewer・title・summary の 5 件が Variable OPENCODE_MODEL の値(Haiku 5.5・variant なし)"
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
