#!/bin/bash
# The test suite entry point (decision 0149; docs/decisions/0149-fast-tests.md). Start it as `harness/test.sh` (needs chmod +x after a patch) or `bash harness/test.sh`.
#
#   harness/test.sh affected   only the test executables that can see a change (git diff against HEAD + untracked files), and the fast tier if code changed
#   harness/test.sh fast       the unit-level tier: executables that spawn nothing and read no product file (seconds)
#   harness/test.sh full       every executable (cached results are reused; FLE_TEST_NOCACHE=1 runs everything for real) and the doc-tests
#   harness/test.sh record     refresh the recorded input sets (strace) and harness/testfast/deps.tsv
#   harness/test.sh names      the sorted names of all tests and their SHA-256
#   harness/test.sh gc         trim the clang object store (also done at the end of every run; FLE_TEST_CACHE_MAX_MB, default 1500)
#
# Options (after the mode): --jobs N (executables at once), --threads N (test threads per executable), --only REGEX, --base REV, --base-tree DIR, --files LIST,
# --fresh (ignore cached results), --record / --no-record (full records input sets with strace by default, once per executable), -- ARGS (passed to the test executables, e.g. a test name filter).
# Environment: FLE_TEST_NOCACHE=1 (no clang cache, no result cache), FLE_TEST_CACHE=DIR (default $CARGO_TARGET_DIR/fle-test-cache), FLE_INPUT_ROOT (see test-env.sh).
set -u
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
mode=${1:-affected}
[ $# -gt 0 ] && shift
. "$here/test-env.sh"
case "$mode" in
    affected | fast | full) exec python3 "$here/testfast/fletest.py" run --mode "$mode" "$@" ;;
    record) exec python3 "$here/testfast/fletest.py" record "$@" ;;
    names) exec python3 "$here/testfast/fletest.py" names "$@" ;;
    gc) exec python3 "$here/testfast/fletest.py" gc "$@" ;;
    env) exit 0 ;;
    *) sed -n '2,13p' "$0"; exit 2 ;;
esac
