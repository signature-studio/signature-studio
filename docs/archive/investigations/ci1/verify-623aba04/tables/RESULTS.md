# Production bundle fl-engine-ci-623aba04 (8 modules: oggio + Elastique), main 26a7cf4e (CI run 41991899343; built in run 41987198683 on a6331e92) — checked against a local rebuild of c5ed023b (ci1)

Real engine: the lifted FLEngine_x64.dll (wasm) ran in headless Chrome 156 and Firefox 158 (profiles on the disk, TMPDIR on the disk), page built from c5ed023b, files of the local bundle served by `vite preview` (same page JS as production: index-D2g1_aiT.js). The Worker/Access/R2 delivery path is not in this test. No stand-in, no replay. Not measured: sound, 120 fps, input latency, the owner's machine, a real GPU. Machine load 5-13 from other workers.

## 1. Local rebuild (ci/stage-*.sh of c5ed023b, `git archive`, CI_JOBS=3, nice 15) vs the CI output

| item | CI (build run 41987198683, deploy run 41991899343) | local c5ed023b | same |
|---|---|---|---|
| wasm | 141,795,291 B, sha256 3d0279d45493e42eb5d86e189700e5d701cef808010685e63eb3119bb5b5cac3 | same | yes (also the value the PM expected) |
| glue | 732,085 B, name `engine-glue.6ac38751eb38.js` (first 12 hex of its sha256) | 732,085 B, sha256 6ac38751eb383a518fe3a336c5fe719afdf18831e16acb2ced6b94ee058a44a8 | yes |
| seed pack | 7,248,243 B, sha256 f7726113ae34d52e0712fff5e893ae6a61f1eaecc72545e82eb27b4ebd6d8984 | same | yes |
| dll pack | 77,444,112 B (8 files), sha256 2f494419c1adec95aad70e0beed730a6ed5e9cba695a7b72efca3cdb18fa7d39 | same | yes |
| br wasm | 15,533,723 B, sha256 prefix 2a8ad1ee5c44f3a3 | same | yes |
| br glue | 65,975 B, bc752815cb0901f0 | same | yes |
| br seed | 3,929,989 B, 6f8cf851197d727b | same | yes |
| br dll | 15,953,166 B, 3a9645ab01725069 | same | yes |
| manifest | 3,097 B (prefix fl-engine-ci-623aba04) | 3,086 B (prefix fl-engine-ci-local) | differs by the prefix and the creation text only |
- The raw `flengine6.js` line in the CI link step (239f0202…) is the file before the bundle step's rewrite; the bundle's glue is the one named by its hash.
- Local stage seconds (nice 15, load 10-34): extract 14, felift 40, b2d 89, lift 118, objects 922, link 837, bundle 358 (total 40 min). The local libblend2d.a has +atomics in 159 of 159 objects.

## 2. CI: seconds and memory (lift/out-pmprompts/ci-report.sh; logs/ci-report-*.txt)

| run (commit) | what | seconds / value |
|---|---|---|
| 41982180403 (e840e4d4), cold | felift / lift / wasm objects (8 modules, cold) / link (failed, old libblend2d.a restored) | 27 / 47 / 1,410 / 296 |
| 41986214303 (c5ed023b) | b2d (rebuilt: cache key changed) / objects (restored, 35 s) / link (failed: lifter binaries not restored) | 86 / 35 / - |
| 41987198683 (a6331e92) | link / bundle (brotli, manifest) / build job | 1,369 / 521 / 07:26:36-07:59:35Z (33 min) |
| 41987198683 | deploy: objects uploaded, then step "R2: the beacons (perf/) expire after 180 days" failed (AccessDenied for the cache credential); Worker and manifest not switched | production unchanged |
| 41991899343 (26a7cf4e, the lifecycle fix) | build skipped (bundle cache hit); deploy 08:06:52-08:07:42Z (50 s): lifecycle step = warning, "the perf/ expiry is not applied (the deploy goes on)"; Worker version 568b4580-b36b-4778-8a67-568bbe759829 at 100%; default manifest switched; 16 unauthenticated requests answered 302 by Access | success |
| runner memory (cgroup system.slice memory.peak) | after objects 4,686,192,640 B (objects restored) ; after link 7,894,290,432 B ; swap peak 49,152 B ; RAM 7,931 MiB (94.9% at the peak) ; cold objects in run 41982180403: 7,764,004,864 B | no OOM line |

## 3. Why e840e4d4 failed at the link and c5ed023b does not: tables/cache-b2d.md (k_b2d reproduced with ci/plan.sh, identical to the CI logs)

## 4. Default entry `/` (local bundle = production bytes), first idle in ms

| browser | alone | two browsers at the same time | console errors | beacon | desk.mode |
|---|---|---|---|---|---|
| Chrome 156 | 6,700 | 6,265 | 0 | 204 stored, `credentials: same-origin` | gl |
| Firefox 158 | 5,770 | 5,662 | 0 | 204 stored, same-origin | gl |
(The 5c4b6902 bundle: 5,797 / 4,778 alone. Load was 10 here against 5 then; the P1 target is 10 s.)

## 5. Scenes (the 11 of the owner's reports), fixes and fades

| item | Chrome | Firefox (one process per scene, profile on disk) |
|---|---|---|
| d1-d11 | all ok | all ok |
| d8 File menu x100 | 100/100, popup surface 100 | 100/100, popup surface 100 |
| + > More plugins...: list window stays; Esc removes it; + opens the popup again | yes / yes / yes | yes / yes / yes |
| Add-menu fade, 40 shots (row 'Simple') | 134,145,153 -> 31,40,45, 0 black | 168,180,189 -> 31,40,45, 0 black |
| F8 picker fade-in, 25 shots x 4 points | 95,101,109 -> 31,39,45; closed again 95,101,109 | 78,86,93 -> 31,39,45; closed 95,102,110 |
| Firefox, same process, 8 page loads, Files window (d1 openProject) | - | 8/8 ok (first idle 8.3, 10.1, 6.2, 7.0, 5.8, 5.8, 5.0, 5.2 s; quota 10 GiB) |

## 6. Worker CPU with the F8 picker open (ms/s), 3 runs each

| | Chrome | Firefox |
|---|---|---|
| steady state | 437 / 473 / 477 (presents 4.3, 4.3, 5.0 per s) | 320 (1.7/s) / 660 / 690 (6.3/s) |
| 5c4b6902 for comparison | 450 / 450 / 493 | 530-757 |
| idle before / after | 135-140 / 130-145 | 165-225 / 185-225 |
| fade-in 0.8 s / fade-out 1.2 s | 580-620 / 820-840 | 770-860 / 1,210-1,260 |

## 7. 8-module specifics: the reference project (serina1-serum2.zip, 53.5 MB) through the Files window and File > Open (normal window, real OPFS)

| item | Chrome 156 | Firefox 158 |
|---|---|---|
| "Problems loading the project" window | 691x343 at 572,422 | 691x343 at 572,379 |
| 6-module bundles (5c4b6902) | - | 691x423 at 572,339 |
| what it says | "8 plugin(s) could not be opened ... and the plugin(s) will be ignored." and the 8 plugin rows (Fruity Wrapper, FLEX, Frequency Shifter, Fruity Balance, Soundgoodizer, Fruity Parametric EQ 2, Fruity Delay 2, Fruity Filter); the two missing-sample lines (anvil_land.ogg, dltzk glass break 1.wav), their header and the "not a serious error" line of the 6-module window are gone | same |
| double-click on the zip until the title `serina1-serum2.flp - FL Studio 2026` | 15.1 s | 20.6 s |
| Space (play) for 30 s, samples at 2, 5, 10, 20, 30 s | state engine-idle every time; 5 different screens while playing; no stop | same: engine-idle every time, 5 different screens, no stop |
| stop (Space) | ok, engine-idle | ok |
(One earlier Firefox run, with the first version of the play probe, ended with the page in state `error` after the probe had waited 10 minutes for `deskDumpHash()`, which the engine answers only when it yields during playback. Two reruns with a probe that reads only the page's state and screenshots show 30 s of playback without a stop in both browsers. The cause of that single hang is not established; it is more likely the probe than the engine, which these runs do not prove.)
