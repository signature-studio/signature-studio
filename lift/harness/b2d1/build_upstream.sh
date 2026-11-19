#!/bin/bash
# Decision 0160 (provisional), opt-in, needs the network: fetches upstream Blend2D at a fixed commit and builds its static library WITHOUT the JIT (-DBLEND2D_NO_JIT=ON).
# The upstream source is never put in this repository. Pins: COMMIT a7f947677e85441abe1769586d3bb935b4086a07 (Blend2D 0.11.1; swap1 found that the DLL matches it, not
# 7e32ebfbf243d6fe04faa0bf69eff71e042c08ad), the SHA-256 of the GitHub archive tarball below (measured 2026-11-03; GitHub generates the archive, so a changed hash with the
# same commit is possible: the script stops and says so, it does not continue).
# NOTE: the first builds of decision 0160 (up/, up2/, up3/ of the work tree) were -O3 (CMake Release) whatever OPT said, and Blend2D's own CMakeLists adds -O2 for Release: -Oz needs MinSizeRel (below).
# Decision 0195 (opt-in): PATCH=<file>[:<file>...] (e.g. lift/harness/b2d1/b2d3-compop.patch: PLUS and OVERLAY templates for the fixed pipeline; then b2d3-fetch-fma.patch, which only does something with OPT="-O2 -DB2D3_FETCH_FMA=1") is applied in order with `patch -p1` to the freshly extracted source before the build; the SHA-256 of each patch is printed. Unset: unchanged.
# usage: OUTDIR=<abs dir> [TARGET=native|wasm] [OPT="-O2"] [JOBS=2] [PATCH=<patch file>] build_upstream.sh   -> $OUTDIR/src/ (extracted), $OUTDIR/build-$TARGET/libblend2d.a
set -eu -o pipefail
OUTDIR=${OUTDIR:?}; TARGET=${TARGET:-native}; OPT=${OPT:--O2}; JOBS=${JOBS:-2}
COMMIT=${COMMIT:-a7f947677e85441abe1769586d3bb935b4086a07}
SHA=${TARBALL_SHA256:-391a57ad2508ab2291ad04b39b0a82d78473662b6d3029dbaf18de06165ea1f0}
mkdir -p "$OUTDIR"; cd "$OUTDIR"
T=blend2d-$COMMIT.tar.gz
[ -e "$T" ] || curl -sSL -o "$T" "https://github.com/blend2d/blend2d/archive/$COMMIT.tar.gz"
GOT=$(sha256sum "$T" | cut -d' ' -f1)
[ "$GOT" = "$SHA" ] || { echo "build_upstream: SHA-256 of $T is $GOT, expected $SHA: stop"; exit 2; }
[ -d src ] || { mkdir src; tar -xzf "$T" -C src --strip-components=1; for PF in $(echo "${PATCH:-}" | tr ':' ' '); do echo "build_upstream: applying $PF (sha256 $(sha256sum "$PF" | cut -d' ' -f1))"; patch -p1 -d src < "$PF" || { echo "build_upstream: the patch does not apply: stop"; exit 2; }; done; }
B=build-$TARGET; rm -rf "$B"; mkdir "$B"
# Blend2D'"'"'s CMakeLists appends -O2 for the Release type (BLEND2D_PRIVATE_CFLAGS_REL), after the user flags: OPT=-O2 is Release as it is; -Oz/-Os need MinSizeRel (the Release-only flags then do not apply)
BT=Release; case "$OPT" in -Oz|-Os) BT=MinSizeRel ;; esac
if [ "$TARGET" = wasm ]; then
    nice -n 15 emcmake cmake -S src -B "$B" -G Ninja -DCMAKE_BUILD_TYPE=$BT -DBLEND2D_STATIC=ON -DBLEND2D_NO_JIT=ON -DCMAKE_CXX_FLAGS="$OPT -fno-exceptions -fno-rtti -DNDEBUG" -DCMAKE_C_FLAGS="$OPT -DNDEBUG" -DCMAKE_CXX_FLAGS_RELEASE=-DNDEBUG -DCMAKE_C_FLAGS_RELEASE=-DNDEBUG -DCMAKE_CXX_FLAGS_MINSIZEREL=-DNDEBUG -DCMAKE_C_FLAGS_MINSIZEREL=-DNDEBUG > "$B/cmake.log" 2>&1
else
    nice -n 15 cmake -S src -B "$B" -G Ninja -DCMAKE_BUILD_TYPE=$BT -DBLEND2D_STATIC=ON -DBLEND2D_NO_JIT=ON -DCMAKE_C_COMPILER=clang -DCMAKE_CXX_COMPILER=clang++ \
        -DCMAKE_CXX_FLAGS="$OPT -fno-exceptions -fno-rtti -fPIE" -DCMAKE_C_FLAGS="$OPT -fPIE" -DCMAKE_CXX_FLAGS_RELEASE=-DNDEBUG -DCMAKE_C_FLAGS_RELEASE=-DNDEBUG -DCMAKE_CXX_FLAGS_MINSIZEREL=-DNDEBUG -DCMAKE_C_FLAGS_MINSIZEREL=-DNDEBUG -DCMAKE_POSITION_INDEPENDENT_CODE=ON > "$B/cmake.log" 2>&1
fi
nice -n 15 cmake --build "$B" -j"$JOBS" > "$B/build.log" 2>&1
ls -l "$B/libblend2d.a"
