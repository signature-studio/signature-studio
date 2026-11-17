# Production bundle check, main 5c4b6902 (CI run 41964165870, bundle fl-engine-ci-F2xH7-e61f80af), real Chrome 156 and Firefox 158

Real engine: the lifted FLEngine_x64.dll (wasm) ran in both browsers, served with the page built from the same commit and a local rebuild
that is byte-identical to the CI output, at the default entry `/` with no parameters. No stand-in, no replayed image.
Not measured here: sound, 120 fps, input latency.
The previous production bundle (cbf9d1e5, `3a58cb8c`, CI run 41961969613, live 13:49-14:36 JST) was rebuilt and compared too.

## 1. Bundle: CI output vs local rebuild (ci/stage-*.sh, SHIP_PRESET=F2xH7 VARIANT=Oz, CI_JOBS=3, from `git archive`)

| item | CI 5c4b6902 (e61f80af) | local 5c4b6902 | CI cbf9d1e5 (3a58cb8c) | local cbf9d1e5 |
|---|---|---|---|---|
| wasm size | 133,868,905 B | same | 133,868,888 B | same |
| wasm sha256 | 1f598b9d3a0fa1466cb06d0ce9464a934b1bf83a5af6fef2f819c9ad0c444532 | same | 04c21ecfb9101408eab14cab88cdb7ceb6d15820e1d304eac91814052b26461f | same |
| glue / seed / dll sha256 prefix | cbdeb1ff2624 / f7726113ae34 / 50206405ba5f | same | cbdeb1ff2624 / f7726113ae34 / 50206405ba5f | same |
| br prefix wasm / glue / seed / dll | 3b93bdbd781e4cbf / 1b3a65f078ffbc00 / 6f8cf851197d727b / ffcbd680a95ec4e2 | same | 217494c4c3a1c03b / 1b3a65f078ffbc00 / 6f8cf851197d727b / ffcbd680a95ec4e2 | same |
| br sizes wasm / glue / seed / dll | 12,207,167 / 78,847 / 3,929,989 / 14,407,894 | same | 12,211,588 / 78,847 / 3,929,989 / 14,407,894 | same |

The two wasm files differ by 17 B (fle_raster only). The glue, seed and dll are the same in both.

## 2. Full build job in CI (the lifter's generated C changed in cbf9d1e5)

| item | 5c4b6902 (run 41964165870) | cbf9d1e5 (run 41961969613) |
|---|---|---|
| result | success; build job 45 min 27 s (04:50:11-05:35:38Z); deploy 50 s | success; build job 1 h 18 min |
| felift / lift | 26 s / 35 s | 28 s / 37 s |
| wasm objects / link (emcc + wasm-opt) / bundle | 906 s / 1,217 s / 449 s | 1,060 s / 1,305 s / 463 s |
| runner RAM | 7,931 MiB | 7,932 MiB |
| cgroup system.slice memory.peak: after objects / after link and at the end | 7,281,135,616 B / 7,870,214,144 B | 7,401,549,824 B / 7,877,783,552 B |
| swap peak | 81,920 B (3 GiB swap file) | 49,152 B |
| OOM kill | no line in the log | no line in the log |
| deploy: unauthenticated requests | 16/16 answered 302 by Access | 16/16 302 |

The peak is for the whole runner slice, 94.6% of the RAM; it is reached after the link step in every run (81f6638c: 7,876,288,512 B).

## 3. Default entry `/` (local rebuild of 5c4b6902)

| check | Chrome 156 | Firefox 158 |
|---|---|---|
| first idle | 5.80 s | 4.78 s |
| title (foreground window, from the engine) | Welcome to FL Studio - in ?trial mode? | same |
| console errors | 0 | 0 |
| beacon `POST /__perf` | 204, stored, `credentials: same-origin` | same |
| beacon `desk.mode` | gl | gl |

The local rebuild of cbf9d1e5: first idle Chrome 8.10 s, Firefox 5.95 s (machine load 13+), 0 errors, 204 stored, same-origin, desk gl.

## 4. Owner-report scenes and fixes (5c4b6902)

| item | Chrome | Firefox (one process per scene, profile on disk) |
|---|---|---|
| d1-d11 (11 scenes) | all ok | all ok |
| d8 File menu x100 | 7 runs, 700/700 popups visible, popup surface every time | 4 runs, 400/400 |
| `+` > More plugins...: list window | stays (TPluginListForm 'Select generator plugin') | stays |
| Esc, then `+` | list window gone; `+` opens the popup again (176x132) | same |
| Add menu fade, row 'Simple' (40 shots) | 161,172,182 -> 31,40,45, 0 black | 116,127,134 -> 31,40,45, 0 black |
| F8 picker fade-in (25 shots, 4 points) | 95,101,109 -> 31,39,45, monotonic; closed: 95,101,109 | 86,94,101 -> 31,39,45, monotonic; closed: 95,102,110 |

cbf9d1e5 (Chrome only): 10 of 11 scenes ok; d8 once 97/100 (cycles 13, 15, 50: the popup was not visible within 6 s; machine load 13); 3 further d8 runs 300/300.

## 5. Worker CPU with the F8 picker open, steady state (ms/s of the engine Worker; load average 5-12)

| build | Chrome | Firefox |
|---|---|---|
| 81f6638c (1 run) | 533 (6 presents/s) | 430 (1.3 presents/s) |
| 5c4b6902, 3 runs alone | 450 (3.7/s), 450 (3.3/s), 493 (5.0/s) | 617 (4.7/s), 530 (3.0/s), 757 (5.7/s) |
| 5c4b6902 in the full set (load 25) | 410 | 740 |
| idle before / after (5c4b6902, 3 runs) | 125-150 / 130-165 | 195-225 / 200-245 |

The CPU follows the present rate of the run (3-6 per second). The compose cost per second is 3.4-5 ms (Chrome).

## 6. Firefox: the Files window (OPFS) after reloading the page in the same process (d1 scene, openProject)

| profile of the Firefox | build | reloads | Files window ok | not ok |
|---|---|---|---|---|
| /tmp (tmpfs, 80% full, 760-800 MB free) | 515d97dd | 8 | 2 (reps 1-2) | 6 (reps 3-8) |
| /tmp | 515d97dd | 5 | 4 | 1 (rep 5) |
| /tmp | 515d97dd | 8 | 2 | 6 (reps 3-8) |
| /tmp | 5c4b6902 | 8 | 2 (reps 1 and 8) | 6 (reps 2-7) |
| disk (geckodriver --profile-root) | 515d97dd | 10 | 10 | 0 |
| disk | 5c4b6902 | 8 | 8 | 0 |

In every failing rep: state engine-idle, first idle 4.3-7.2 s as in the ok reps, last timeline mark `first_pixel` (4.4-7.3 s),
Worker CPU in 3 s 0.45-0.73 s (150-240 ms/s, as an idle engine), the Files button reads "Browser files are not available: Could not create actor"
(Firefox's own error), and the four assets log "stored copy: failed (io)".
`navigator.storage.estimate()`: quota 388,981,145 B (= 10.0% of the tmpfs size) and usage 213.5 MB on /tmp; quota 10 GiB on disk.
Not isolated: why Firefox refuses the actor at that point (usage is below the quota). Not tested: a person's Firefox with a nearly full profile disk.
