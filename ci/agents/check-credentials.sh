#!/usr/bin/env bash
# usage: ci/agents/check-credentials.sh   (CI. The workflow sets HAS_* to "true" or "false" from `secrets.X != ''`; this script never sees a secret value)
# Way A: the Bedrock API key (AWS_BEARER_TOKEN_BEDROCK). Way B: an access key pair (AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY). Both need AWS_REGION.
# Stops the job before any step that uses a secret. Issue: docs/requests/0004-actions-model-credentials.md.
set -eu
if [ "${HAS_REGION:-false}" != true ]; then
  echo "::error::Secret AWS_REGION が登録されていない(docs/requests/0004-actions-model-credentials.md を参照)"
  exit 1
fi
if [ "${HAS_BEARER:-false}" = true ]; then echo "モデルの認証: 方式 A(API キー)"; exit 0; fi
if [ "${HAS_KEY_ID:-false}" = true ] && [ "${HAS_SECRET:-false}" = true ]; then echo "モデルの認証: 方式 B(アクセスキー)"; exit 0; fi
echo "::error::モデルの認証情報が登録されていない。方式 A は AWS_BEARER_TOKEN_BEDROCK、方式 B は AWS_ACCESS_KEY_ID と AWS_SECRET_ACCESS_KEY を登録する(docs/requests/0004-actions-model-credentials.md を参照)"
exit 1
