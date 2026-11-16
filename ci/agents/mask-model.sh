#!/usr/bin/env bash
# usage: GH_TOKEN=<token> ci/agents/mask-model.sh   (CI, the first step of a worker job after the checkout. The workflow passes no env with the value: GitHub prints a step's env in its log and does not mask a Variable)
# Reads the Variable OPENCODE_MODEL through the API, masks the value, and passes it on through GITHUB_ENV. The value is never printed. An unreadable Variable is treated as empty (check-agents.sh stops the job later, after the import).
set -u
v=$(gh api "repos/$GITHUB_REPOSITORY/actions/variables/OPENCODE_MODEL" --jq .value 2> /dev/null) || { echo "::warning::Variable OPENCODE_MODEL を API で読めなかった(値は出さない。空として扱う)"; v=""; }
if [ -n "$v" ]; then echo "::add-mask::$v"; fi
echo "OPENCODE_MODEL=$v" >> "$GITHUB_ENV"
echo "モデルの指定を読んだ(値は出さない): $([ -n "$v" ] && echo あり || echo なし)"
