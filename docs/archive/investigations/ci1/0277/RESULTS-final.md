# 0277: verification of `fl-engine-ci-d9c160aa` (Worker 51f102d8, page of git 0b228624) against the previous production bundle `fl-engine-ci-ff5ee429`

Real engine in real browsers (Chrome / Firefox, headless; Firefox on Xvfb :77 with WebGL). No stand-in, no replay. The bundle comes from R2 (read only); every manifest sha256 is checked by `fetch`.

## Pass criteria

| criterion | value | pass |
|---|---|---|
| Firefox has WebGL (preflight) | preflight: webglRenderer llvmpipe, or similar desk.mode gl first idle 4724 | yes |
| No stop (11 scenes + more, both browsers, play, export; d7 is the intended exit) | stops: none; not ok: none | yes |
| Export WAV equals the reference | 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` (expected 22863828 B, `381cdb5acf891a16…`) | yes |
| First idle ≤ previous + 500 ms (chrome, WebGL runs) | all runs: new median 5958 ms, previous 6396 ms, difference -438 ms; quiet runs only (load ≤12, I/O avg10 <1; n new 6 / prev 6): new 5403, previous 5996, difference -593 ms | yes |
| First idle ≤ previous + 500 ms (firefox, WebGL runs) | all runs: new median 4900 ms, previous 5276 ms, difference -376 ms; quiet runs only (load ≤12, I/O avg10 <1; n new 4 / prev 3): new 4945, previous 5297, difference -352 ms | yes |
| File menu ×100 (chrome) | 100/100 | yes |
| File menu ×100 (firefox) | 100/100 | yes |

## Scenes (new bundle)

| browser | scene | result | first idle ms | File ×100 | load, I/O pressure (start→end) |
|---|---|---|---|---|---|
| chrome | d1-sampler-envelope | ok | 5995 | - | load 10.2→10.1, io 0.0→0.0 |
| chrome | d2-browser-folder | ok | 9743 | - | load 16.2→23.8, io 0.0→0.0 |
| chrome | d3-settings-all-tabs | ok | 8883 | - | load 23.8→28.1, io 0.0→0.0 |
| chrome | d4-playlist-middle-drag | ok | 6959 | - | load 27.7→26.8, io 0.0→0.1 |
| chrome | d5-mixer-front | ok | 5543 | - | load 26.8→23.2, io 0.1→0.0 |
| chrome | d6-playlist-close | ok | 5565 | - | load 23.2→21.7, io 0.0→0.0 |
| chrome | d7-main-close | ok (intended exit) | 5587 | - | load 21.7→17.9, io 0.0→0.1 |
| chrome | d8-file-menu-100 | ok | 4884 | 100/100 | load 17.9→13.2, io 0.1→0.0 |
| chrome | d9-keys | ok | 5476 | - | load 13.2→16.9, io 0.0→0.0 |
| chrome | d10-pianoroll-f12 | ok | 5912 | - | load 10.1→9.9, io 0.0→0.0 |
| chrome | d11-menu-hover-escape | ok | 6133 | - | load 9.4→10.2, io 0.0→0.0 |
| chrome | add-menu-simple-fade | ok | 6380 | - | load 14.2→10.8, io 0.3→0.0 |
| chrome | picker-cost | ok | 4220 | - | load 8.7→11.7, io 0.1→0.2 |
| chrome | picker-f8-fade | ok | 4389 | - | load 10.8→8.7, io 0.0→0.1 |
| chrome | plus-more-plugins | ok | 8749 | - | load 16.2→14.2, io 0.0→0.4 |
| firefox | d1-sampler-envelope | ok | 4905 | - | load 9.9→10.6, io 0.0→0.0 |
| firefox | d2-browser-folder | ok | 8928 | - | load 17.9→25.5, io 0.0→0.0 |
| firefox | d3-settings-all-tabs | ok | 8962 | - | load 27.0→27.3, io 0.2→0.0 |
| firefox | d4-playlist-middle-drag | ok | 5805 | - | load 27.3→23.8, io 0.2→0.0 |
| firefox | d5-mixer-front | ok | 5373 | - | load 23.8→20.6, io 0.0→0.0 |
| firefox | d6-playlist-close | ok | 4852 | - | load 19.8→17.2, io 0.0→0.0 |
| firefox | d7-main-close | ok (intended exit) | 4962 | - | load 8.2→8.5, io 0.0→0.0 |
| firefox | d8-file-menu-100 | ok | 6463 | 100/100 | load 15.6→18.7, io 0.0→0.0 |
| firefox | d9-keys | ok | 6524 | - | load 18.7→25.3, io 0.0→0.0 |
| firefox | d10-pianoroll-f12 | ok | 5389 | - | load 10.5→8.9, io 0.0→0.0 |
| firefox | d11-menu-hover-escape | ok | 5237 | - | load 10.0→10.2, io 0.2→0.0 |
| firefox | add-menu-simple-fade | ok | 4836 | - | load 11.7→13.1, io 0.1→2.0 |
| firefox | picker-cost | ok | 6904 | - | load 11.6→12.5, io 0.4→0.0 |
| firefox | picker-f8-fade | ok | 4493 | - | load 13.1→11.6, io 2.0→0.4 |
| firefox | plus-more-plugins | ok | 5277 | - | load 8.3→11.7, io 0.3→0.1 |

The scenes ran as two parallel jobs (Chrome and Firefox), so the first idle values of this table are reference values (the load is next to each).

Firefox d7 (the intended exit): the first run lost the WebDriver session (`POST /session/d7a665e4-87dd-40c3-a2d0-27ec3f115412/execute/sync: 404 {"value":{"error":"invalid session id","message":"T`; at load 17.23 io / io 0.03, two jobs in parallel; the engine's exit was not read). Re-runs: 3, all `ok (the intended exit)`. Not reproduced; the cause of the first loss is unknown. The table shows the latest run; the first run is kept in `out/d7-ff-first-session-lost/`.

Invalid runs, kept aside in `out/invalid-noproj/` and NOT counted above: d1, d10, d11 (Chrome and Firefox) and the two d11 runs on the new and the previous bundle, taken while the kit served no `Sampler.flp` / `SHP_Sawing Synth.wav` (a 404 body was opened as the project; the engine stopped with a named CompareStringW stop on both bundles). They were re-run after the kit served the real files (rows above).

## alpha7 (main window geometry, guest 1920x1080, Chrome)

result ok; load 11.6→13.7, io 0.1→1.6; windows visible at the end: 1; windows larger than 1800 px wide:
- `14 TFruityLoopsMainForm 'FL Studio 2026' @0,0 1920x1080 z0`
- expected after alpha7: the main window @0,0 1920x1080 (previous df0512b0: @-5,-6 1930x1092). Screenshots: `main-1920x1080.png, scenario-alpha7-look.png`

## First idle (single runs, cold, a new profile each; previous and new alternating)

| browser | bundle | run | first idle ms | load, I/O pressure at the start | at the end | quiet (load ≤12, I/O <1) | webglRenderer | desk.mode |
|---|---|---|---|---|---|---|---|---|
| chrome | prev | 1 | 6771 | 6.40 / io 0.18 | 6.99 / io 0.05 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 1 | 6618 | 6.99 / io 0.05 | 7.71 / io 0.08 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 2 | 5647 | 7.71 / io 0.08 | 7.45 / io 0.10 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 2 | 6378 | 7.45 / io 0.10 | 9.16 / io 0.21 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 3 | 9349 | 9.16 / io 0.21 | 12.58 / io 0.05 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 3 | 10723 | 12.58 / io 0.05 | 15.36 / io 0.00 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 4 | 9682 | 15.36 / io 0.00 | 18.30 / io 0.00 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 4 | 9766 | 18.30 / io 0.00 | 19.94 / io 0.00 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 5 | 9122 | 19.94 / io 0.00 | 20.26 / io 0.00 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 5 | 6269 | 20.26 / io 0.00 | 17.83 / io 0.00 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 6 | 6411 | 17.83 / io 0.00 | 16.89 / io 0.00 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 6 | 6414 | 16.89 / io 0.00 | 15.75 / io 0.00 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 1q | 6243 | 4.76 / io 0.00 | 5.47 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 1q | 5457 | 5.47 / io 0.00 | 5.77 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 2q | 5349 | 5.77 / io 0.00 | 5.80 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 2q | 5349 | 5.80 / io 0.00 | 5.85 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 3q | 5529 | 5.85 / io 0.00 | 5.79 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 3q | 4830 | 5.79 / io 0.00 | 5.37 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 4q | 4946 | 5.37 / io 0.00 | 5.60 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 4q | 5749 | 5.60 / io 0.00 | 5.49 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| firefox | prev | 1 | 5786 | 14.89 / io 0.00 | 14.42 / io 0.09 | no (reference) | llvmpipe, or similar | gl |
| firefox | new | 1 | 4890 | 14.42 / io 0.09 | 13.21 / io 0.02 | no (reference) | llvmpipe, or similar | gl |
| firefox | new | 2 | 4778 | 13.21 / io 0.02 | 12.41 / io 0.00 | no (reference) | llvmpipe, or similar | gl |
| firefox | prev | 2 | 5254 | 12.41 / io 0.00 | 12.02 / io 0.08 | no (reference) | llvmpipe, or similar | gl |
| firefox | prev | 3 | 4721 | 12.02 / io 0.08 | 11.57 / io 0.03 | no (reference) | llvmpipe, or similar | gl |
| firefox | new | 3 | 4981 | 11.57 / io 0.03 | 10.80 / io 0.01 | yes | llvmpipe, or similar | gl |
| firefox | new | 4 | 4909 | 10.80 / io 0.01 | 9.35 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 4 | 4454 | 9.35 / io 0.00 | 8.84 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 5 | 5297 | 8.84 / io 0.00 | 8.64 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 5 | 4701 | 8.64 / io 0.00 | 8.53 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 6 | 5718 | 8.53 / io 0.00 | 9.23 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 6 | 5934 | 9.23 / io 0.00 | 9.36 / io 0.08 | yes | llvmpipe, or similar | gl |

| browser | mode | previous median (n) | new median (n) | difference |
|---|---|---|---|---|
| chrome | gl | 6396 (10) | 5958 (10) | -438 |
| firefox | gl | 5276 (6) | 4900 (6) | -376 |

## Export (Chrome, the reference project, Ctrl+R)

result ok; load 8.5→4.3, io 0.0→0.0; seconds 358
- WAV `serina1-serum2.wav`: 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`; expected 22863828 B / `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`: MATCH

## Playback (the reference project zip, ~60 s)

**chrome (new)**: result ok, stopped False; load 4.1→4.8, io 0.0→0.0

| window | wall s | Worker busy ms/s (= CPU ÷ wall ×1000) | main ms/s | load |
|---|---|---|---|---|
| idle-main | 5 | 116 | 0 | 3.57 |
| project-loaded-stopped | 5 | 284 | 0 | 3.27 |
| playing-a | 10 | 802 | 0 | 3.60 |
| playing-b | 10.0 | 862 | 0 | 4.03 |
| playing-c | 10 | 718 | 0 | 3.95 |
| playing-d | 10 | 679 | 0 | 3.88 |
| playing-e | 10 | 715 | 0 | 4.26 |
| playing-f | 10.0 | 815 | 0 | 5.01 |
| stopped-after | 5 | 354 | 0 | 4.85 |

| moment | load | status | playingSeconds | underrunQuanta | episodes | longestRunQuanta (maxQuanta) | fillMinFrames |
|---|---|---|---|---|---|---|---|
| before-play | 3.38 | running | 33.538321995464855 | 1375 | 1 | 1375 | -175786 |
| playing-a | 3.60 | running | 52.63383219954648 | 1396 | 4 | 1375 | -175786 |
| playing-b | 4.03 | running | 62.66485260770975 | 1396 | 4 | 1375 | -175786 |
| playing-c | 3.95 | running | 72.7045804988662 | 1624 | 28 | 1375 | -175786 |
| playing-d | 3.88 | running | 82.73269841269841 | 2013 | 62 | 1375 | -175786 |
| playing-e | 4.26 | running | 92.76081632653062 | 2321 | 89 | 1375 | -175786 |
| playing-f | 5.01 | running | 102.7889342403628 | 2342 | 94 | 1375 | -175786 |
| after-stop | 5.01 | running | 104.33886621315193 | 2342 | 94 | 1375 | -175786 |

- Worker busy while playing: min 679, median 760, max 862 ms/s. Underrun from the end of playing-a to the stop (52 s of playback): 946 quanta in 90 episodes (10.5 quanta per episode; one quantum = 128 frames ≈ 2.9 ms). Before the playback (the project load stalls the engine): 1375 quanta in 1 episode(s); `longestRunQuanta` 1375 is that load stall, not the playback.

**chrome r2 (new)**: result ok, stopped False; load 8.8→8.6, io 0.0→0.0

| window | wall s | Worker busy ms/s (= CPU ÷ wall ×1000) | main ms/s | load |
|---|---|---|---|---|
| idle-main | 5 | 132 | 0 | 9.32 |
| project-loaded-stopped | 5 | 264 | 0 | 8.85 |
| playing-a | 10 | 790 | 0 | 8.74 |
| playing-b | 10.0 | 831 | 0 | 8.24 |
| playing-c | 10.0 | 753 | 0 | 8.04 |
| playing-d | 10 | 824 | 0 | 8.97 |
| playing-e | 10 | 760 | 0 | 9.67 |
| playing-f | 10.0 | 833 | 0 | 9.18 |
| stopped-after | 5 | 362 | 0 | 8.61 |

| moment | load | status | playingSeconds | underrunQuanta | episodes | longestRunQuanta (maxQuanta) | fillMinFrames |
|---|---|---|---|---|---|---|---|
| before-play | 9.18 | running | 33.898231292517 | 1432 | 2 | 1430 | -182826 |
| playing-a | 8.74 | running | 52.99374149659864 | 1464 | 5 | 1430 | -182826 |
| playing-b | 8.24 | running | 63.033469387755105 | 1464 | 5 | 1430 | -182826 |
| playing-c | 8.04 | running | 73.07319727891156 | 1638 | 26 | 1430 | -182826 |
| playing-d | 8.97 | running | 83.10131519274377 | 1718 | 41 | 1430 | -182826 |
| playing-e | 9.67 | running | 93.14104308390023 | 1904 | 64 | 1430 | -182826 |
| playing-f | 9.18 | running | 103.17206349206349 | 1908 | 66 | 1430 | -182826 |
| after-stop | 8.92 | running | 104.71909297052154 | 1908 | 66 | 1430 | -182826 |

- Worker busy while playing: min 753, median 807, max 833 ms/s. Underrun from the end of playing-a to the stop (52 s of playback): 444 quanta in 61 episodes (7.3 quanta per episode; one quantum = 128 frames ≈ 2.9 ms). Before the playback (the project load stalls the engine): 1432 quanta in 2 episode(s); `longestRunQuanta` 1430 is that load stall, not the playback.

**chrome r3 (new)**: result ok, stopped False; load 10.4→8.8, io 0.0→0.0

| window | wall s | Worker busy ms/s (= CPU ÷ wall ×1000) | main ms/s | load |
|---|---|---|---|---|
| idle-main | 5 | 122 | 0 | 9.27 |
| project-loaded-stopped | 5 | 310 | 0 | 8.61 |
| playing-a | 10 | 836 | 0 | 8.70 |
| playing-b | 10.0 | 885 | 0 | 8.74 |
| playing-c | 10 | 787 | 0 | 8.17 |
| playing-d | 10 | 683 | 0 | 7.99 |
| playing-e | 10 | 702 | 0 | 8.88 |
| playing-f | 10.0 | 809 | 0 | 9.16 |
| stopped-after | 5 | 378 | 0 | 8.83 |

| moment | load | status | playingSeconds | underrunQuanta | episodes | longestRunQuanta (maxQuanta) | fillMinFrames |
|---|---|---|---|---|---|---|---|
| before-play | 8.75 | running | 34.02013605442177 | 1522 | 1 | 1522 | -194602 |
| playing-a | 8.70 | running | 53.13886621315193 | 1555 | 4 | 1522 | -194602 |
| playing-b | 8.74 | running | 63.17859410430839 | 1555 | 4 | 1522 | -194602 |
| playing-c | 8.17 | running | 73.20671201814059 | 1658 | 17 | 1522 | -194602 |
| playing-d | 7.99 | running | 83.23482993197278 | 2034 | 54 | 1522 | -194602 |
| playing-e | 8.88 | running | 93.26585034013605 | 2403 | 84 | 1522 | -194602 |
| playing-f | 9.16 | running | 103.29396825396826 | 2407 | 86 | 1522 | -194602 |
| after-stop | 9.16 | running | 104.8409977324263 | 2407 | 86 | 1522 | -194602 |

- Worker busy while playing: min 683, median 798, max 885 ms/s. Underrun from the end of playing-a to the stop (52 s of playback): 852 quanta in 82 episodes (10.4 quanta per episode; one quantum = 128 frames ≈ 2.9 ms). Before the playback (the project load stalls the engine): 1522 quanta in 1 episode(s); `longestRunQuanta` 1522 is that load stall, not the playback.

**firefox (new)**: result ok, stopped False; load 4.8→6.3, io 0.0→0.0

| window | wall s | Worker busy ms/s (= CPU ÷ wall ×1000) | main ms/s | load |
|---|---|---|---|---|
| idle-main | 5 | 136 | 0 | 3.88 |
| project-loaded-stopped | 5 | 318 | 0 | 3.15 |
| playing-a | 10 | 917 | 0 | 3.57 |
| playing-b | 10 | 991 | 0 | 5.60 |
| playing-c | 10.0 | 886 | 0 | 5.05 |
| playing-d | 10 | 714 | 0 | 4.51 |
| playing-e | 10 | 706 | 0 | 5.49 |
| playing-f | 10 | 846 | 0 | 5.52 |
| stopped-after | 5 | 340 | 0 | 6.31 |

| moment | load | status | playingSeconds | underrunQuanta | episodes | longestRunQuanta (maxQuanta) | fillMinFrames |
|---|---|---|---|---|---|---|---|
| before-play | 3.25 | running | 37.477006802721085 | 2538 | 1 | 2538 | -324650 |
| playing-a | 3.57 | running | 56.61315192743764 | 2538 | 1 | 2538 | -324650 |
| playing-b | 5.60 | running | 66.64997732426303 | 2538 | 1 | 2538 | -324650 |
| playing-c | 5.05 | running | 76.6984126984127 | 2618 | 10 | 2538 | -324650 |
| playing-d | 4.51 | running | 86.73523809523809 | 2885 | 39 | 2538 | -324650 |
| playing-e | 5.49 | running | 96.7720634920635 | 3223 | 70 | 2538 | -324650 |
| playing-f | 5.52 | running | 106.80888888888889 | 3223 | 70 | 2538 | -324650 |
| after-stop | 6.52 | running | 108.3762358276644 | 3223 | 70 | 2538 | -324650 |

- Worker busy while playing: min 706, median 866, max 991 ms/s. Underrun from the end of playing-a to the stop (52 s of playback): 685 quanta in 69 episodes (9.9 quanta per episode; one quantum = 128 frames ≈ 2.9 ms). Before the playback (the project load stalls the engine): 2538 quanta in 1 episode(s); `longestRunQuanta` 2538 is that load stall, not the playback.

**chrome (previous fl-engine-ci-ff5ee429)**: result ok, stopped False; load 15.2→9.3, io 0.0→0.0

| window | wall s | Worker busy ms/s (= CPU ÷ wall ×1000) | main ms/s | load |
|---|---|---|---|---|
| idle-main | 5 | 126 | 0 | 13.21 |
| project-loaded-stopped | 5 | 272 | 0 | 11.01 |
| playing-a | 10.0 | 873 | 0 | 9.84 |
| playing-b | 10 | 890 | 0 | 10.69 |
| playing-c | 10 | 884 | 0 | 10.42 |
| playing-d | 10.0 | 819 | 0 | 9.58 |
| playing-e | 10.0 | 827 | 0 | 10.32 |
| playing-f | 10 | 900 | 0 | 9.65 |
| stopped-after | 5 | 414 | 0 | 9.32 |

| moment | load | status | playingSeconds | underrunQuanta | episodes | longestRunQuanta (maxQuanta) | fillMinFrames |
|---|---|---|---|---|---|---|---|
| before-play | 11.62 | running | 34.426485260770974 | 1684 | 1 | 1684 | -215338 |
| playing-a | 9.84 | running | 53.53360544217687 | 1693 | 2 | 1684 | -215338 |
| playing-b | 10.69 | running | 63.56172335600907 | 1693 | 2 | 1684 | -215338 |
| playing-c | 10.42 | running | 73.60145124716553 | 1707 | 6 | 1684 | -215338 |
| playing-d | 9.58 | running | 83.62956916099773 | 1742 | 16 | 1684 | -215338 |
| playing-e | 10.32 | running | 93.66929705215419 | 1809 | 32 | 1684 | -215338 |
| playing-f | 9.65 | running | 103.6887074829932 | 1809 | 32 | 1684 | -215338 |
| after-stop | 9.44 | running | 105.23573696145125 | 1809 | 32 | 1684 | -215338 |

- Worker busy while playing: min 819, median 878, max 900 ms/s. Underrun from the end of playing-a to the stop (52 s of playback): 116 quanta in 30 episodes (3.9 quanta per episode; one quantum = 128 frames ≈ 2.9 ms). Before the playback (the project load stalls the engine): 1684 quanta in 1 episode(s); `longestRunQuanta` 1684 is that load stall, not the playback.

**chrome r2 (previous fl-engine-ci-ff5ee429)**: result ok, stopped False; load 8.3→11.4, io 0.0→0.0

| window | wall s | Worker busy ms/s (= CPU ÷ wall ×1000) | main ms/s | load |
|---|---|---|---|---|
| idle-main | 5 | 126 | 0 | 8.81 |
| project-loaded-stopped | 5 | 310 | 0 | 10.23 |
| playing-a | 10 | 898 | 0 | 9.95 |
| playing-b | 10 | 907 | 0 | 11.33 |
| playing-c | 10.0 | 898 | 0 | 10.97 |
| playing-d | 10.0 | 891 | 0 | 12.16 |
| playing-e | 10 | 908 | 0 | 12.13 |
| playing-f | 10 | 885 | 0 | 11.65 |
| stopped-after | 5 | 398 | 0 | 11.36 |

| moment | load | status | playingSeconds | underrunQuanta | episodes | longestRunQuanta (maxQuanta) | fillMinFrames |
|---|---|---|---|---|---|---|---|
| before-play | 10.16 | running | 35.96480725623583 | 2118 | 1 | 2118 | -270890 |
| playing-a | 9.95 | running | 55.071927437641726 | 2126 | 3 | 2118 | -270890 |
| playing-b | 11.33 | running | 65.1116553287982 | 2126 | 3 | 2118 | -270890 |
| playing-c | 10.97 | running | 75.15138321995465 | 2128 | 4 | 2118 | -270890 |
| playing-d | 12.16 | running | 85.17950113378684 | 2132 | 6 | 2118 | -270890 |
| playing-e | 12.13 | running | 95.21922902494332 | 2140 | 8 | 2118 | -270890 |
| playing-f | 11.65 | running | 105.24734693877551 | 2140 | 8 | 2118 | -270890 |
| after-stop | 11.65 | running | 106.79437641723356 | 2140 | 8 | 2118 | -270890 |

- Worker busy while playing: min 885, median 898, max 908 ms/s. Underrun from the end of playing-a to the stop (52 s of playback): 14 quanta in 5 episodes (2.8 quanta per episode; one quantum = 128 frames ≈ 2.9 ms). Before the playback (the project load stalls the engine): 2118 quanta in 1 episode(s); `longestRunQuanta` 2118 is that load stall, not the playback.

**chrome r3 (previous fl-engine-ci-ff5ee429)**: result ok, stopped False; load 8.8→5.7, io 0.0→0.0

| window | wall s | Worker busy ms/s (= CPU ÷ wall ×1000) | main ms/s | load |
|---|---|---|---|---|
| idle-main | 5 | 122 | 0 | 8.19 |
| project-loaded-stopped | 5 | 262 | 0 | 8.57 |
| playing-a | 10 | 870 | 0 | 7.42 |
| playing-b | 10.0 | 894 | 0 | 8.00 |
| playing-c | 10 | 784 | 0 | 7.23 |
| playing-d | 10 | 745 | 0 | 6.59 |
| playing-e | 10 | 763 | 0 | 6.62 |
| playing-f | 10.0 | 865 | 0 | 6.06 |
| stopped-after | 5 | 348 | 0 | 5.74 |

| moment | load | status | playingSeconds | underrunQuanta | episodes | longestRunQuanta (maxQuanta) | fillMinFrames |
|---|---|---|---|---|---|---|---|
| before-play | 9.06 | running | 33.866303854875284 | 1456 | 1 | 1456 | -186154 |
| playing-a | 7.42 | running | 52.964716553287985 | 1456 | 1 | 1456 | -186154 |
| playing-b | 8.00 | running | 62.99283446712018 | 1456 | 1 | 1456 | -186154 |
| playing-c | 7.23 | running | 73.02095238095238 | 1533 | 16 | 1456 | -186154 |
| playing-d | 6.59 | running | 83.04907029478458 | 1762 | 41 | 1456 | -186154 |
| playing-e | 6.62 | running | 93.08009070294784 | 1940 | 60 | 1456 | -186154 |
| playing-f | 6.06 | running | 103.09659863945578 | 1942 | 61 | 1456 | -186154 |
| after-stop | 6.06 | running | 104.6465306122449 | 1942 | 61 | 1456 | -186154 |

- Worker busy while playing: min 745, median 824, max 894 ms/s. Underrun from the end of playing-a to the stop (52 s of playback): 486 quanta in 60 episodes (8.1 quanta per episode; one quantum = 128 frames ≈ 2.9 ms). Before the playback (the project load stalls the engine): 1456 quanta in 1 episode(s); `longestRunQuanta` 1456 is that load stall, not the playback.

### Playback summary (every run; same page, same procedure)

Counter definition: `window.__app.audio.detail()` of the page: `underrunQuanta` = AudioWorklet render quanta (128 frames ≈ 2.9 ms) in which the output ring was empty, counted cumulatively from when the audio device began to play (page start), `episodes` = maximal runs of consecutive underrun quanta (`app/src/audio/underrun.ts`, `ring-layout.ts`). This is the same counter as `underrunQuanta` in opt1's audio-b.json `detail`. **cumulative** = the value at the end (includes the stall while the project is opened); **during playback** = the difference between the reading at the end of `playing-a` (≈19 s after the first reading, 4 s after Space + 10 s playing) and the reading after the stop (≈52 s of playback).

| run | bundle | load at start → end | Worker busy ms/s while playing (min / median / max) | cumulative at the end: quanta / episodes | before the playback (project-open stall) | **during playback: quanta / episodes** | gap frames the engine wrote itself during playback (`engineGapFrames` delta; the engine wrote silence for a hole) |
|---|---|---|---|---|---|---|---|
| chrome | new | 4.1 → 4.8 (io 0.0 → 0.0) | 679 / 760 / 862 | 2342 / 94 | 1375 / 1 | **946 / 90** | 113348 |
| chrome | previous ff5ee429 | 15.2 → 9.3 (io 0.0 → 0.0) | 819 / 878 / 900 | 1809 / 32 | 1684 / 1 | **116 / 30** | 12268 |
| chrome r2 | new | 8.8 → 8.6 (io 0.0 → 0.0) | 753 / 807 / 833 | 1908 / 66 | 1432 / 2 | **444 / 61** | 51586 |
| chrome r2 | previous ff5ee429 | 8.3 → 11.4 (io 0.0 → 0.0) | 885 / 898 / 908 | 2140 / 8 | 2118 / 1 | **14 / 5** | 1362 |
| chrome r3 | new | 10.4 → 8.8 (io 0.0 → 0.0) | 683 / 798 / 885 | 2407 / 86 | 1522 / 1 | **852 / 82** | 102004 |
| chrome r3 | previous ff5ee429 | 8.8 → 5.7 (io 0.0 → 0.0) | 745 / 824 / 894 | 1942 / 61 | 1456 / 1 | **486 / 60** | 57048 |
| firefox | new | 4.8 → 6.3 (io 0.0 → 0.0) | 706 / 866 / 991 | 3223 / 70 | 2538 / 1 | **685 / 69** | 81746 |

## Mixer header drag: Worker CPU per move (previous and new alternating)

**chrome** (1920x1080 guest)

| bundle | n | median ms/move (all) | median (quiet runs: load ≤12, I/O <1) | runs (round, ms, load and I/O pressure at the start; * = load >12, reference) |
|---|---|---|---|---|
| prev | 2 | 8.75 | 8.75 (n=2) | #1 8.75 (5.97 / io 0.00); #2 8.76 (6.81 / io 0.00) |
| new | 2 | 8.68 | 8.68 (n=2) | #1 8.69 (5.01 / io 0.00); #2 8.67 (8.29 / io 0.00) |

(the runs of 3 sessions, a / b / c = ab-firefox-a.tsv / ab-firefox-b.tsv / ab-firefox.tsv, in this order in time)

**firefox** (1920x1080 guest)

| bundle | n | median ms/move (all) | median (quiet runs: load ≤12, I/O <1) | runs (round, ms, load and I/O pressure at the start; * = load >12, reference) |
|---|---|---|---|---|
| prev | 8 | 9.86 | 7.82 (n=4) | #c1 7.78 (10.35 / io 0.00); #c2 7.86 (10.54 / io 0.00); #c3 13.36 (9.08 / io 0.00); #a1 5.75 (6.17 / io 0.00); #a2 10.81 (15.46 / io 0.00)*; #b1 11.28 (17.46 / io 0.00)*; #b2 13.13 (23.83 / io 0.00)*; #b3 8.91 (17.97 / io 0.01)* |
| new | 8 | 9.96 | 8.38 (n=3) | #c1 8.38 (11.81 / io 0.00); #c2 8.34 (9.43 / io 0.00); #c3 11.25 (14.27 / io 0.00)*; #a1 11.84 (5.23 / io 0.18); #a2 13.93 (18.04 / io 0.00)*; #b1 13.76 (22.04 / io 0.00)*; #b2 8.32 (23.28 / io 0.00)*; #b3 8.66 (14.09 / io 0.00)* |

## Steps (load and I/O pressure at each step)

```
10:14:48 fetched fl-engine-ci-d9c160aa (260M) load 7.75 io avg10=0.00 avg60=0.11 avg300=0.71
10:15:03 fetched fl-engine-ci-ff5ee429 (259M) load 6.79 io avg10=0.00 avg60=0.08 avg300=0.67
10:15:03 built the page of 0b228624 load 6.79 io avg10=0.00 avg60=0.08 avg300=0.67
10:15:31 preflight ok load 6.44 io avg10=0.28 avg60=0.16 avg300=0.63
10:18:15 idle chrome x6 load 15.75 io avg10=0.00 avg60=0.01 avg300=0.36
10:20:34 idle firefox x6 load 9.36 io avg10=0.08 avg60=0.03 avg300=0.23
10:20:40 scenes chrome scn-client.json  load 9.09 io avg10=0.03 avg60=0.03 avg300=0.22
10:20:40 scenes firefox scn-client.json d1-sampler-envelope load 9.09 io avg10=0.03 avg60=0.03 avg300=0.22
10:24:16 scenes firefox scn-client.json d2-browser-folder load 16.16 io avg10=0.00 avg60=0.00 avg300=0.10
10:24:47 scenes firefox scn-client.json d3-settings-all-tabs load 26.49 io avg10=0.01 avg60=0.01 avg300=0.09
10:25:43 scenes firefox scn-client.json d4-playlist-middle-drag load 27.33 io avg10=0.00 avg60=0.01 avg300=0.07
10:26:08 scenes firefox scn-client.json d5-mixer-front load 23.83 io avg10=0.01 avg60=0.03 avg300=0.07
10:26:39 scenes firefox scn-client.json d6-playlist-close load 20.60 io avg10=0.00 avg60=0.01 avg300=0.06
10:26:59 scenes firefox scn-client.json d7-main-close load 17.23 io avg10=0.03 avg60=0.03 avg300=0.06
10:27:29 scenes firefox scn-client.json d8-file-menu-100 load 15.53 io avg10=0.03 avg60=0.06 avg300=0.06
10:30:07 scenes firefox scn-client.json d9-keys load 18.68 io avg10=0.00 avg60=0.00 avg300=0.03
10:30:58 scenes firefox scn-client.json d10-pianoroll-f12 load 25.26 io avg10=0.00 avg60=0.00 avg300=0.02
10:34:38 scenes firefox scn-client.json d11-menu-hover-escape load 24.78 io avg10=0.47 avg60=0.65 avg300=0.19
10:36:54 scenes chrome scn-plus.json  load 16.23 io avg10=0.00 avg60=0.06 avg300=0.11
10:37:24 scenes chrome scn-alpha1.json  load 14.20 io avg10=0.32 avg60=0.19 avg300=0.13
10:38:11 scenes firefox scn-plus.json  load 8.72 io avg10=0.10 avg60=0.11 avg300=0.12
10:38:41 scenes firefox scn-alpha1.json  load 11.66 io avg10=0.11 avg60=0.23 avg300=0.15
10:38:49 alpha7 load 11.60 io avg10=0.05 avg60=0.20 avg300=0.15
10:46:04 preflight ok load 6.87 io avg10=0.00 avg60=0.59 avg300=0.51
10:46:21 scenes firefox scn-client.json d11-menu-hover-escape load 9.36 io avg10=0.00 avg60=0.45 avg300=0.48
10:46:21 scenes chrome scn-client.json d11-menu-hover-escape load 9.36 io avg10=0.00 avg60=0.45 avg300=0.48
10:46:50 scenes chrome scn-client.json d1-sampler-envelope load 10.23 io avg10=0.01 avg60=0.29 avg300=0.43
10:46:52 scenes firefox scn-client.json d1-sampler-envelope load 9.89 io avg10=0.00 avg60=0.28 avg300=0.43
10:47:20 scenes chrome scn-client.json d10-pianoroll-f12 load 10.08 io avg10=0.00 avg60=0.17 avg300=0.39
10:47:26 scenes firefox scn-client.json d10-pianoroll-f12 load 10.63 io avg10=0.00 avg60=0.15 avg300=0.38
10:48:03 export start load 8.50 io avg10=0.00 avg60=0.10 avg300=0.33
10:48:16 scenes firefox scn-client.json d7-main-close load 8.26 io avg10=0.00 avg60=0.08 avg300=0.32
10:54:03 export end load 4.33 io avg10=0.00 avg60=0.00 avg300=0.07
10:54:14 play chrome load 4.06 io avg10=0.00 avg60=0.00 avg300=0.06
10:56:22 play chrome end load 4.62 io avg10=0.00 avg60=0.00 avg300=0.03
10:56:37 play firefox load 4.75 io avg10=0.00 avg60=0.00 avg300=0.03
10:58:49 play firefox end load 6.31 io avg10=0.00 avg60=0.00 avg300=0.00
11:01:14 ab chrome x2 load 7.11 io avg10=0.00 avg60=0.00 avg300=0.00
11:04:08 ab firefox x2 load 29.18 io avg10=0.00 avg60=0.00 avg300=0.00
11:09:08 ab firefox x3 load 11.71 io avg10=0.00 avg60=0.01 avg300=0.00
11:13:20 ab firefox x3 load 19.17 io avg10=0.00 avg60=0.00 avg300=0.00
11:13:53 play chrome-prev load 15.15 io avg10=0.00 avg60=0.00 avg300=0.00
11:16:04 play chrome-prev end load 9.32 io avg10=0.00 avg60=0.00 avg300=0.00
11:16:27 play chrome-r2 load 8.83 io avg10=0.00 avg60=0.00 avg300=0.00
11:18:35 play chrome-r2 end load 8.61 io avg10=0.00 avg60=0.02 avg300=0.00
11:18:43 play chrome-prev-r2 load 8.27 io avg10=0.00 avg60=0.01 avg300=0.00
11:20:56 play chrome-prev-r2 end load 11.36 io avg10=0.00 avg60=0.01 avg300=0.00
11:21:07 play chrome-r3 load 10.39 io avg10=0.00 avg60=0.01 avg300=0.00
11:23:16 play chrome-r3 end load 8.83 io avg10=0.00 avg60=0.00 avg300=0.00
11:23:16 play chrome-prev-r3 load 8.83 io avg10=0.00 avg60=0.00 avg300=0.00
11:25:26 play chrome-prev-r3 end load 5.74 io avg10=0.00 avg60=0.00 avg300=0.00
11:27:16 idle chrome x4 load 5.49 io avg10=0.00 avg60=0.00 avg300=0.00
```
