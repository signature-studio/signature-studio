#!/bin/bash
# ci/stage-deploy.sh config|plan|upload|lifecycle|worker|probe - the stage "deploy" (the pieces around wrangler-action).
#   config:  app/wrangler.deploy.json from app/wrangler.jsonc + the variables (ci/lib/wrangler_config.py: refuses a route without Access settings, workers_dev / preview URLs on)
#            env: R2_ASSETS_BUCKET R2_CACHE_BUCKET (the Worker's PERF_BUCKET binding: the measurement beacons, prefix perf/; decision 0224) ACCESS_TEAM_DOMAIN ACCESS_AUD PUBLIC_HOSTNAME
#   plan:    BUNDLE_DIR -> the upload plan for the delivery bucket (checks SHA256SUMS, sizes, the manifest); prints a table; writes $CI_WORK/r2-upload-plan.md
#   upload [objects|switch]:  BUNDLE_DIR -> uploads that plan with the S3 API (aws CLI): "objects" = everything except the default manifest.json, "switch" = only the default manifest.json (after the Worker), none = all, default manifest last. env: R2_ENDPOINT=https://<account>.r2.cloudflarestorage.com, <CREDENTIAL-NAME> / <CREDENTIAL-NAME> of a token that is limited to the delivery bucket
#   lifecycle: the beacon prefix perf/ of the cache bucket expires after 180 days (R2 object lifecycle, S3 API with the cache credentials; idempotent: a rule that is there is left alone, others are kept; ci/lib/r2_lifecycle.py).
#            env: R2_ENDPOINT <CREDENTIAL-NAME> <CREDENTIAL-NAME> R2_CACHE_BUCKET
#   worker:  the Worker only: wrangler versions upload (a new VERSION) and wrangler versions deploy <id>@100%. Neither touches the triggers (the custom domain stays as it is): `wrangler deploy` also updates them, which needs a zone
#            permission that a Workers Scripts: Edit token does not have (the first push to main failed there, 2026-11-04, after the Worker had been uploaded). env: <CREDENTIAL-NAME> CLOUDFLARE_ACCOUNT_ID
#   probe [MANIFEST]:  12 unauthenticated requests to https://$PUBLIC_HOSTNAME (GET / POST / HEAD / Range; the app, /assets/, /__perf, /__perf/summary), plus one Range GET per asset URL of MANIFEST if given: every one must be answered by
#            Cloudflare Access (302 to *.cloudflareaccess.com) or be refused (401 / 403); anything else fails the job
set -u -o pipefail
. "$(dirname "$0")/lib/common.sh"
case "${1:-}" in
  config)
    ASSETS_BUCKET_NAME=${R2_ASSETS_BUCKET:?} PERF_BUCKET_NAME=${R2_CACHE_BUCKET:?} python3 "$CI_DIR/lib/wrangler_config.py" "$REPO/app/wrangler.jsonc" "$REPO/app/wrangler.deploy.json" ;;
  plan)
    python3 "$CI_DIR/lib/upload_plan.py" "${2:?BUNDLE_DIR}" "${R2_ASSETS_BUCKET:?}" --commands "$CI_WORK/r2-upload-commands.txt" --table "$CI_WORK/r2-upload-plan.md" --json "$CI_WORK/r2-upload-plan.json" ;;
  upload)
    : "${R2_ENDPOINT:?}" "${<CREDENTIAL-NAME>:?}" "${<CREDENTIAL-NAME>:?}"
    python3 "$CI_DIR/lib/upload_plan.py" "${2:?BUNDLE_DIR}" "${R2_ASSETS_BUCKET:?}" --execute --mode s3 --endpoint "$R2_ENDPOINT" --phase "${3:-all}" ;;
  lifecycle)
    : "${R2_ENDPOINT:?}" "${<CREDENTIAL-NAME>:?}" "${<CREDENTIAL-NAME>:?}" "${R2_CACHE_BUCKET:?}"
    cur=$CI_WORK/r2-lifecycle-current.json; new=$CI_WORK/r2-lifecycle-new.json
    out=$(aws s3api get-bucket-lifecycle-configuration --endpoint-url "$R2_ENDPOINT" --bucket "$R2_CACHE_BUCKET" --output json 2>&1); rc=$?
    if [ "$rc" = 0 ]; then printf '%s' "$out" > "$cur"
    elif printf '%s' "$out" | grep -q 'NoSuchLifecycleConfiguration'; then echo '{"Rules": []}' > "$cur"
    elif printf '%s' "$out" | grep -q 'AccessDenied'; then
      # the bucket's lifecycle is a bucket setting: an R2 token for objects only (the cache credential) is refused (run 41987198683 failed here, after the build, before the Worker and the manifest were switched).
      # The expiry of the beacons is housekeeping; the deploy must not depend on it. Said loudly, not hidden.
      echo "::warning title=R2 lifecycle not applied::the cache credential may not read the lifecycle of $R2_CACHE_BUCKET (AccessDenied); the perf/ expiry is left as it is"
      summary "- R2 lifecycle of $R2_CACHE_BUCKET: **not applied** (AccessDenied for the cache credential; the beacons under perf/ are kept until a credential with bucket settings applies the 180-day rule)"
      log "the cache credential may not read the lifecycle of $R2_CACHE_BUCKET: the perf/ expiry is not applied (the deploy goes on)"; exit 0
    else die "cannot read the lifecycle of $R2_CACHE_BUCKET: $(printf '%s' "$out" | tail -3)"; fi
    python3 "$CI_DIR/lib/r2_lifecycle.py" "$cur" "$new" 180 perf/ perf-expire; rc=$?
    case "$rc" in
      0) log "the rule for perf/ (180 days) is already there: nothing to do" ;;
      10) if put=$(aws s3api put-bucket-lifecycle-configuration --endpoint-url "$R2_ENDPOINT" --bucket "$R2_CACHE_BUCKET" --lifecycle-configuration "file://$new" 2>&1); then log "the rule for perf/ (expire after 180 days) was added to $R2_CACHE_BUCKET"
          elif printf '%s' "$put" | grep -q 'AccessDenied'; then
            echo "::warning title=R2 lifecycle not applied::the cache credential may not change the lifecycle of $R2_CACHE_BUCKET (AccessDenied)"
            summary "- R2 lifecycle of $R2_CACHE_BUCKET: **not applied** (AccessDenied when setting the rule)"
            log "the cache credential may not change the lifecycle of $R2_CACHE_BUCKET: the perf/ expiry is not applied (the deploy goes on)"
          else die "cannot set the lifecycle of $R2_CACHE_BUCKET: $(printf '%s' "$put" | tail -3)"; fi ;;
      *) die "r2_lifecycle.py failed (rc=$rc)" ;;
    esac ;;
  worker)
    : "${<CREDENTIAL-NAME>:?}" "${CLOUDFLARE_ACCOUNT_ID:?}"
    cd "$REPO/app" || exit 2
    [ -s wrangler.deploy.json ] || die "no app/wrangler.deploy.json: run 'stage-deploy.sh config' first"
    out="$CI_WORK/wrangler-output.ndjson"; rm -f "$out"
    export WRANGLER_OUTPUT_FILE_PATH="$out" WRANGLER_SEND_METRICS=false
    msg="ci ${GITHUB_SHA:-local}"
    npx wrangler versions upload --config wrangler.deploy.json --message "$msg" || die "wrangler versions upload failed"
    vid=$(python3 -c 'import json,sys
for l in open(sys.argv[1]):
    r = json.loads(l)
    if r.get("type") == "version-upload": print(r["version_id"])' "$out" | tail -1)
    [ -n "$vid" ] || die "no version id in $out"
    npx wrangler versions deploy "$vid@100%" --config wrangler.deploy.json --message "$msg" --yes || die "wrangler versions deploy failed"
    log "Worker version $vid is deployed at 100%" ;;
  probe)
    host=${PUBLIC_HOSTNAME:-}; [ -n "$host" ] || { log "PUBLIC_HOSTNAME is empty: the Worker has no route, nothing to probe"; exit 0; }
    # METHOD|PATH|extra header ("" = none). The paths need not exist: Access answers before the Worker is reached. Every asset URL of the manifest given as $2 (the bundle's default manifest) is added with a Range header.
    reqs=("GET|/|" "GET|/static/probe.js|" "GET|/assets/manifest.json|" "GET|/assets/probe/engine.0000000000.wasm|" "GET|/__perf|" "GET|/__perf/summary|"
          "POST|/|" "POST|/assets/manifest.json|" "POST|/__perf|" "POST|/__perf/summary|" "HEAD|/assets/manifest.json|" "GET|/assets/manifest.json|Range: bytes=0-99")
    if [ -n "${2:-}" ] && [ -f "$2" ]; then
      while IFS= read -r u; do reqs+=("GET|$u|Range: bytes=0-99"); done < <(python3 -c 'import json,sys
def walk(x):
    if isinstance(x, dict):
        for k, v in x.items():
            if k == "url" and isinstance(v, str) and v.startswith("/assets/"): yield v
            else: yield from walk(v)
    elif isinstance(x, list):
        for v in x: yield from walk(v)
for u in sorted(set(walk(json.load(open(sys.argv[1]))))): print(u)' "$2")
    fi
    bad=0
    for r in "${reqs[@]}"; do
      IFS='|' read -r m p h <<<"$r"
      args=(-s -o /dev/null -w '%{http_code} %{redirect_url}' --max-time 30)
      if [ "$m" = HEAD ]; then args+=(-I); else args+=(-X "$m"); fi
      [ -n "$h" ] && args+=(-H "$h")
      out=$(curl "${args[@]}" "https://$host$p" | sed 's/?.*//')
      case "$out" in
        "302 https://"*.cloudflareaccess.com/*|401*|403*) echo "ok   $m $p${h:+ [$h]} -> $out" ;;
        *) echo "OPEN $m $p${h:+ [$h]} -> $out"; bad=1 ;;
      esac
    done
    echo "probe: ${#reqs[@]} unauthenticated requests"
    [ "$bad" = 0 ] || die "an unauthenticated request was NOT stopped by Access: remove the route (wrangler triggers / dashboard) and check the Access application of $host" ;;
  *) die "usage: stage-deploy.sh config|plan|upload [objects|switch]|lifecycle|worker|probe [MANIFEST]" ;;
esac
