# Why run 41982180403 (main e840e4d4) failed at the link, and why c5ed023b does not (ci1, reproduced with ci/plan.sh and ci/needs.sh)

Method: a detached `git worktree` of each commit inside the repository (so `hashpaths` uses `git ls-files -s`, as in CI), `CI_RUNNER_OS=ubuntu24-x64` (the value of the CI job), `bash ci/plan.sh OUT`, then `ci/needs.sh` with the cache lookups that the plan job of the real run printed. The keys below are the same as the keys in the CI logs.

## 1. The Blend2D cache key (k_b2d)

| commit | k_b2d (this machine, ci/plan.sh) | k_b2d in the CI plan job | CI cache lookup of that key |
|---|---|---|---|
| 5c4b6902 (last deployed bundle; library built without atomics) | fl-b2d-cb54413de1e68dce6457f851637737e5a63edfac | fl-b2d-cb54413de1e68dce6457f851637737e5a63edfac (run 41964165870) | "Cache found and can be restored" (this is the library that was saved) |
| e840e4d4 (run 41982180403, failed at weblink) | fl-b2d-cb54413de1e68dce6457f851637737e5a63edfac | fl-b2d-cb54413de1e68dce6457f851637737e5a63edfac | "Cache found and can be restored" |
| c5ed023b (run 41986214303) | fl-b2d-118b3ea31dc4c9910133b6c419398bb1784012cc | fl-b2d-118b3ea31dc4c9910133b6c419398bb1784012cc | "Cache not found for input keys" |

- e840e4d4 has the same k_b2d as 5c4b6902: the new compiler flags (-matomics -mbulk-memory, added by layer c) were in the build (ci/stage-wasm.sh) but not in the key (ci/plan.sh), so the run restored the library that 5c4b6902 had saved. That is the failure of run 41982180403, reproduced.
- c5ed023b puts `cflags=$B2D_CFLAGS` into KB2D: the key changes, the lookup misses.
- Other keys of the same runs also match the logs: k_link e840e4d4 fl-link-8bde3951…, c5ed023b fl-link-ec23c2c3…; k_lift (both) fl-lift-5926a45d…; prefix e840e4d4 fl-engine-ci-48da0f2d, c5ed023b fl-engine-ci-623aba04 (the last 8 digits of the bundle keys 687d6f56…da0f2d and 3643779d…623aba04 in the logs).

## 2. What ci/needs.sh does with the lookups of each run

| output | e840e4d4 (H_B2D=true; extract, felift, lift, link, seed, bundle all not found) | c5ed023b (H_B2D=false; extract, felift, lift found; link, seed, bundle not found) |
|---|---|---|
| need_b2d | true | true |
| restore_b2d | **true** (the cached library, built without atomics, is restored) | **false** |
| run_b2d | **false** | **true** (Blend2D is built again, with the flags) |
| run_lift / restore_lift | true / false | false / true |
| run_extract / run_felift | true / true | false / false |
| need_link, need_seed, need_build | true, true, true | true, true, true |

## 3. The library itself (target_features of the wasm objects inside libblend2d.a; tools/wasm-features.py)

| library | wasm objects | +atomics | +bulk-memory |
|---|---|---|---|
| the one restored at e840e4d4 (a local build of 5c4b6902, lift/out-ci1/w, same ci/stage-wasm.sh before the flags) | 159 | **0** | 159 |
| built by ci/stage-wasm.sh b2d of c5ed023b (lift/out-ci1/c5, 89 s) | 159 | **159** | 159 |

## 4. The guard added in c5ed023b, run against both trees (ci/test-b2d-key.sh)

| tree | result |
|---|---|
| c5ed023b | all 6 checks hold |
| e840e4d4 (the same script, copied) | FAILED: the b2d build line does not use $B2D_CFLAGS; KB2D does not contain cflags=$B2D_CFLAGS; B2D_CFLAGS is not defined |
