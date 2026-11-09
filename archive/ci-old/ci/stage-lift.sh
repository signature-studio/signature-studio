#!/bin/bash
# ci/stage-lift.sh felift|lift - the stage "lift" (decompile).
#   felift: builds the lifter (fe-pe, fe-lift) and the direct wasm generator of the hybrid (fe-wasmgen, decision 0230) from lift/crates with the locked dependencies. Output: $ENGINE_WORK/cargo-target/release/{fe-pe,fe-lift,fe-wasmgen}
#   lift:   app/scripts/build-engine-bundle.sh `gen resplit` on the extracted inputs ($HOST_ROOT): the generated C of FLEngine and of the seven bundled modules, cut into 10 MB translation units.
#           Output (what the later stages read, and what the lift cache holds): $ENGINE_WORK/wsrc, $ENGINE_WORK/onat/v-*/{dll,modules.txt,native/realmod_table_gen.c}, $ENGINE_WORK/onat/log, $ENGINE_WORK/logs/generated-c-sha256.txt
#           The generated C itself (onat/gen, onat/v-*/gen: the same text as wsrc, before the cut) is not kept: the SHA-256 list of it is (logs/generated-c-sha256.txt).
set -u -o pipefail
. "$(dirname "$0")/lib/common.sh"
case "${1:-}" in
  felift)
    command -v cargo >/dev/null || die "cargo not found (rustup toolchain $RUST_TOOLCHAIN)"
    # the CI runner installs the toolchain by version (ci/setup-toolchain.sh rust); a workstation may have the same version under another name (stable): then the active one is used, with a warning if it differs
    if command -v rustup >/dev/null && rustup toolchain list 2>/dev/null | grep -q "^$RUST_TOOLCHAIN-"; then export RUSTUP_TOOLCHAIN=$RUST_TOOLCHAIN
    else rustc --version | grep -q " $RUST_TOOLCHAIN " || log "WARNING: rustc is not $RUST_TOOLCHAIN ($(rustc --version)); the lifter is built with it anyway"; fi
    mkdir -p "$ENGINE_WORK/logs"
    t0=$SECONDS
    ( cd "$REPO/lift" && CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_TARGET_DIR="$ENGINE_WORK/cargo-target" cargo build --release --locked -p fe-pe -p fe-lift -p fe-wasmgen ) > "$ENGINE_WORK/logs/cargo-build.txt" 2>&1 || { tail -n 30 "$ENGINE_WORK/logs/cargo-build.txt"; die "cargo build failed"; }
    ls -l "$ENGINE_WORK/cargo-target/release/fe-pe" "$ENGINE_WORK/cargo-target/release/fe-lift" "$ENGINE_WORK/cargo-target/release/fe-wasmgen"
    log "felift done in $((SECONDS - t0)) s"
    ;;
  lift)
    [ -r "$HOST_ROOT/binaries/win64/FLEngine_x64.dll" ] || die "no extracted inputs under HOST_ROOT=$HOST_ROOT (run ci/stage-extract.sh fl first)"
    [ -x "$ENGINE_WORK/cargo-target/release/fe-lift" ] || die "no fe-lift under $ENGINE_WORK/cargo-target (run ci/stage-lift.sh felift first)"
    mkdir -p "$B2D_DIR"
    [ -n "${B2D_UPSTREAM_LIB:-}" ] || { [ -r "$B2D_DIR/build-wasm/libblend2d.a" ] && [ -d "$B2D_DIR/src/src" ]; } || \
      { log "no upstream Blend2D yet: placeholders for the gen stage (it only checks that they exist; the wasm stages need the real ones)"; mkdir -p "$B2D_DIR/build-wasm" "$B2D_DIR/src/src"; : > "$B2D_DIR/build-wasm/libblend2d.a"; touch "$B2D_DIR/.placeholder"; }
    t0=$SECONDS
    run_bb gen resplit || die "gen/resplit failed (see $ENGINE_WORK/logs/gen.err)"
    log "lift done in $((SECONDS - t0)) s"
    cat "$ENGINE_WORK/logs/generated-c-sha256.txt" 2>/dev/null | sed 's|  .*/|  |' >&2
    # what the later stages read; the rest of onat (the unsplit generated C, ~half of the disk) goes
    rm -rf "$ENGINE_WORK/onat/gen" "$ENGINE_WORK"/onat/v-*/gen 2>/dev/null || true
    ;;
  *) die "usage: stage-lift.sh felift|lift" ;;
esac
