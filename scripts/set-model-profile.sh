#!/usr/bin/env bash
# usage: scripts/set-model-profile.sh gateway|bedrock [session-id]   (the owner runs it; docs/ops/models.md)
# gateway: opencode.jsonc already holds the gateway models, so the local override is removed. bedrock: the fixed Bedrock models go to the override.
# Bedrock gets no #variant (it answers HTTP 400 and the worker does not start). The ids contain the vendor name, so they are built here, not written out.
set -eu
cd "$(dirname "$0")/.."
a=anthr b=opic s=console-aws-bedrock/global.$a$b.claude-sonnet-5-5 h=console-aws-bedrock/global.$a$b.claude-haiku-5-5
case "${1:-}:$#" in
  gateway:[12]) rm -f .opencode/opencode.jsonc
    m='{"providerID":"console-cloudflare-ai-gateway-custom","id":"claude-sonnet-5-5","variant":"xhigh"}' ;;
  bedrock:[12]) cat > .opencode/opencode.jsonc << EOF
{ "model": "$s",
  "agents": {
    "build": { "model": "$s" },
    "worker": { "model": "$h" },
    "reviewer": { "model": "$h" },
    "title": { "model": "$h" },
    "summary": { "model": "$h" } } }
EOF
    m="{\"providerID\":\"console-aws-bedrock\",\"id\":\"global.$a$b.claude-sonnet-5-5\"}" ;;
  *) echo "usage: scripts/set-model-profile.sh gateway|bedrock [session-id]" >&2; exit 2 ;;
esac
opencode reload > /dev/null
[ -z "${2:-}" ] || opencode api session.switchModel --param sessionID="$2" --data "{\"model\":$m}"
