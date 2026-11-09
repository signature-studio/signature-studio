#!/bin/bash
# ci/check-config.sh - are the five GitHub Secrets there? Reads only whether each is set (env S_<NAME> = true|false); prints the NAMES of what is missing, never a value.
# There are no GitHub Variables: the names and the build configuration are in the workflow's env.
# Outputs (GITHUB_OUTPUT): r2_ready (the build can run: the artifact and the cache/assets S3 credentials), deploy_ready (the Workers token too), missing (list).
set -u -o pipefail
. "$(dirname "$0")/lib/common.sh"
miss=()
s() { local n=S_$1; [ "${!n:-}" = true ] || miss+=("secret $1"); }
s <CREDENTIAL-NAME>; s <CREDENTIAL-NAME>; s <CREDENTIAL-NAME>; s <CREDENTIAL-NAME>
build_missing=("${miss[@]}")
miss=(); s <CREDENTIAL-NAME>
deploy_missing=("${miss[@]}")
if [ "${#build_missing[@]}" = 0 ]; then set_output r2_ready true; else set_output r2_ready false; fi
if [ "${#build_missing[@]}" = 0 ] && [ "${#deploy_missing[@]}" = 0 ]; then set_output deploy_ready true; else set_output deploy_ready false; fi
{
  echo "### Configuration"
  if [ "${#build_missing[@]}" = 0 ]; then echo "Build (cache + artifacts): **complete**."; else echo "Build (cache + artifacts): **incomplete**, missing:"; printf -- '- %s\n' "${build_missing[@]}"; fi
  if [ "${#deploy_missing[@]}" = 0 ]; then echo "Deploy token: **complete**."; else echo "Deploy token: **missing**:"; printf -- '- %s\n' "${deploy_missing[@]}"; fi
  echo; echo "What to set and how: the issue \"CI の設定\" (Secrets only; there are no Variables)."
} | tee -a "${GITHUB_STEP_SUMMARY:-/dev/null}"
