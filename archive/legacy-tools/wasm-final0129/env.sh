#!/bin/bash
# wasm-final0129 (decision 0134, PROVISIONAL): environment (sourced). Inputs are read from the repository checkout (read only); writes only under lift/out-wasm0129 and /tmp/opencode/wasm0129.
R=/home/ubuntu/work/signature-studio-develop
L=$R/lift
W=$L/out-wasm0129
export PYTHONDONTWRITEBYTECODE=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_TARGET_DIR=$W/cargo-target
export TMPDIR=$W/tmp
export PATH=/opt/emsdk:/opt/emsdk/upstream/emscripten:/home/ubuntu/.local/bin:$PATH
SRCTREE=${SRCTREE:-$W/src}
S=$SRCTREE/lift
H=$S/harness
T=$SRCTREE/docs/investigations/wasm-final0129/tools
