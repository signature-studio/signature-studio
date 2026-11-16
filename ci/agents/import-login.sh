#!/usr/bin/env bash
# usage: ci/agents/import-login.sh   (CI. The workflow sets OPENCODE_AUTH_JSON in the environment; this script never prints it)
# Imports the opencode credentials with the import command, which reads the JSON from stdin. The JSON is the output of the export command for opencode.
# Runs the import as a private server (--standalone), so that it does not start the background service before runner-config.sh writes the config (decision 0384, decision 0388).
# Stops the job when OPENCODE_AUTH_JSON is empty, and when nothing is imported. The import output is not shown on failure, because it may hold a part of the credential.
set -eu
if [ -z "${OPENCODE_AUTH_JSON:-}" ]; then
  echo "::error::OPENCODE_AUTH_JSON が空"
  exit 1
fi
tmp=${RUNNER_TEMP:-${TMPDIR:-/tmp}}
out="$tmp/import-login.log"
rc=0
printf '%s' "$OPENCODE_AUTH_JSON" | opencode auth import --standalone > "$out" 2>&1 || rc=$?
if [ "$rc" -ne 0 ]; then
  rm -f "$out"
  echo "::error::認証情報の取り込みが失敗した(出力は伏せる。値を含みうるため)"
  exit 1
fi
if ! grep -qE '^Imported [1-9][0-9]* credential' "$out"; then
  rm -f "$out"
  echo "::error::取り込まれた認証情報が 0 件"
  exit 1
fi
cat "$out"
rm -f "$out"
