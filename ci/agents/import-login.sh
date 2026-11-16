#!/usr/bin/env bash
# usage: ci/agents/import-login.sh   (CI. The workflow sets OPENCODE_AUTH_JSON from the Secret of the same name; this script never prints it)
# Imports the owner's opencode credentials with the import command, which reads the JSON from stdin. The JSON is the output of the export command for opencode, made by the owner (Issue 0005).
# Stops the job when the Secret is empty, and when nothing is imported. The import output is not shown on failure, because it may hold a part of the credential.
# After a successful import, prints the number of models for each provider, and the number of the Haiku 5.5 model of console-aws-bedrock (counts only; the model ids and the output of models are not printed).
set -eu
if [ -z "${OPENCODE_AUTH_JSON:-}" ]; then
  echo "::error::Secret OPENCODE_AUTH_JSON が登録されていない。Issue 0005 を参照(docs/requests/0005-actions-console-login.md)"
  exit 1
fi
tmp=${RUNNER_TEMP:-${TMPDIR:-/tmp}}
out="$tmp/import-login.log"
rc=0
printf '%s' "$OPENCODE_AUTH_JSON" | opencode auth import > "$out" 2>&1 || rc=$?
if [ "$rc" -ne 0 ]; then
  rm -f "$out"
  echo "::error::認証情報の取り込みが失敗した(出力は伏せる。値を含みうるため)"
  exit 1
fi
if ! grep -qE '^Imported [1-9][0-9]* credential' "$out"; then
  rm -f "$out"
  echo "::error::取り込まれた認証情報が 0 件。Secret の中身が、依頼者の export の出力か確かめる(Issue 0005)"
  exit 1
fi
cat "$out"
rm -f "$out"
models=$(opencode models 2> /dev/null) || { echo "::warning::モデルの一覧を出せなかった(出力は伏せる)"; models=""; }
printf '%s\n' "$models" | awk -F/ 'NF { t++; p = NF >= 2 ? $1 : "(不明)"; n[p]++ } END { if (t == 0) print "提供元の件数: 0"; for (p in n) print "提供元の件数: " p " " n[p] }' | sort
a=anthr b=opic
echo "console-aws-bedrock の Haiku 5.5 の掲載件数(ID は出さない): $(printf '%s\n' "$models" | grep -cxF "console-aws-bedrock/global.$a$b.claude-haiku-5-5" || true)"
unset models
