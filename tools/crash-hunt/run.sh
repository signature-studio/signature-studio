#!/usr/bin/env bash
# crash-hunt (Issue #1405): walks FL's screen operations in the lifted engine (the menus, the keys, the right-click menus) and writes the probes and the fingerprints of the stops.
# Usage: bash tools/crash-hunt/run.sh --budget 50m --out pm-work/findings [--bundle <raw/<name>>] [--seed 20251125] [--resume]
# The work is in hunt.mjs (node). Exit 0 when the walk ran (stops are findings, not failures), 1 when the tool itself failed.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
exec node "$here/hunt.mjs" "$@"
