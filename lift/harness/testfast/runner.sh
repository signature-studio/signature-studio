#!/bin/bash
# Cargo test runner (lift/.cargo/config.toml) and the way fletest.py starts every test executable: decision 0149.
#  1. RLIMIT_CORE = 1 byte. A test program that aborts on purpose (about 2,500 times in one full run) otherwise waits for the system core-dump handler: with
#     apport as core_pattern that is a single queue shared by every process on the machine, about one event per second (measured, docs/decisions/0149).
#     The value 1 is the one the kernel treats as "do not start the pipe handler"; 0 does not stop apport.
#     Three tests (cursor, loadimage, wall_fr10) have generated programs that call setrlimit(RLIMIT_CORE, 0) themselves, which would bring the handler back:
#     libnocore.so (LD_PRELOAD, harness/testfast/nocore.c) turns that 0 into 1.
#  2. A caching `clang` first in PATH (harness/testfast/clangcache.c) unless FLE_TEST_NOCACHE=1.
# Usage: runner.sh EXE [ARGS...]
exe=$1
case "$exe" in */deps/*) ;; *) exec "$@" ;; esac   # `cargo run` of a tool goes through the same runner: only test executables (target/*/deps/) are treated
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
if [ -z "${FLE_TEST_CACHE:-}" ]; then
    t=$(dirname "$(dirname "$(dirname "$(readlink -f "$exe")")")")   # target/debug/deps/EXE -> target
    FLE_TEST_CACHE=$t/fle-test-cache
fi
if shimdir=$(bash "$here/shim.sh" "$FLE_TEST_CACHE"); then
    if [ -z "${FLE_TEST_NOCACHE:-}" ] || [ "${FLE_TEST_NOCACHE}" = 0 ]; then
        export PATH=$shimdir:$PATH FLE_CC_CACHE_DIR=$FLE_TEST_CACHE/cc FLE_CC_ROOT=$(cd "$here/../.." && pwd)
    fi
    [ -f "$shimdir/libnocore.so" ] && export LD_PRELOAD="$shimdir/libnocore.so${LD_PRELOAD:+:$LD_PRELOAD}"
fi
if command -v prlimit > /dev/null; then
    exec prlimit --core=1:1 -- "$@"
fi
exec "$@"
