#!/usr/bin/env bash
# E2E entry (Issue #1407, #1485). Usage: bash tools/e2e/run.sh [--scenarios smoke | --area <領域>] [--only ID[,ID]] [--shard i/n] [--repeat n] [--budget 40m] [--list] [--displays 4k-dpr1,4k-dpr2,4k-random,dpr:1.25] [--shots DIR] > e2e.jsonl
# Needs: tools/e2e/node_modules (npm ci there), Chromium (npx playwright install chromium there), app/node_modules and app/dist (npm run build --prefix app),
# and the bundle lift/out-app/bundle/{raw,br}/fl-Oz (fe-build bundle). The stdout is the JSON lines; the log goes to stderr.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
for d in "$here/node_modules/playwright" "$repo/app/node_modules/wrangler" "$repo/app/dist"; do
  if [ ! -e "$d" ]; then echo "run.sh: missing $d" >&2; exit 2; fi
done
cd "$repo"
exec node "$here/run.mjs" "$@"
