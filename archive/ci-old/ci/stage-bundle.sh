#!/bin/bash
# ci/stage-bundle.sh seed|bundle - the packing stages.
#   seed:   the guest's file pack (FLSEEDP1) and the eight-file DLL directory ($ENGINE_WORK/in): from the extracted inputs and the lift output.
#   bundle: app/scripts/pack-assets.mjs: brotli, engine.<sha>.wasm(.br), glue, packs, manifest.json, SHA256SUMS, upload-commands.txt -> $ENGINE_WORK/bundle/$PREFIX
#           (BUCKET names the delivery bucket in the printed upload commands; ci/lib/upload_plan.py rewrites it anyway).
set -u -o pipefail
. "$(dirname "$0")/lib/common.sh"
case "${1:-}" in
  seed)   t0=$SECONDS; run_bb seed   || die "seed failed"; log "seed done in $((SECONDS - t0)) s" ;;
  bundle) t0=$SECONDS; BUCKET=${R2_ASSETS_BUCKET:-YOUR-BUCKET} run_bb bundle || die "bundle failed"; log "bundle done in $((SECONDS - t0)) s"
          ( cd "$ENGINE_WORK/bundle/$PREFIX" && ls -l && sha256sum -c SHA256SUMS >/dev/null && echo "SHA256SUMS ok" ) ;;
  *) die "usage: stage-bundle.sh seed|bundle" ;;
esac
