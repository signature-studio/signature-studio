# 0277: verification of `fl-engine-ci-62098b85` (Worker 2f390ad3-5853-423e-89a3-fb6610a15263, page of git 38efb398) against the previous production bundle `fl-engine-ci-875bfd62`

Real engine in real browsers (Chrome / Firefox, headless; Firefox on Xvfb :77 with WebGL). No stand-in, no replay. The bundle comes from R2 (read only); every manifest sha256 is checked by `fetch`.

## Pass criteria

| criterion | value | pass |
|---|---|---|
| Firefox has WebGL (preflight) | preflight: webglRenderer llvmpipe, or similar desk.mode gl first idle 5555 | yes |
| No stop (11 scenes + more, both browsers, play, export; d7 is the intended exit) | stops: none; not ok: none; no result at all (the harness did not start): none | yes |
| Export WAV equals the reference | 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` (expected 22863828 B, `381cdb5acf891a16…`) | yes |
| First idle ≤ previous + 500 ms (chrome, WebGL runs) | all runs: new median 5128 ms, previous 5037 ms, difference +92 ms; means 5055 ± 67 (SE, n=6) / 5002 ± 67 (n=6), difference of the means +52 ± 94 (SE); quiet runs only (load ≤12, I/O avg10 <1; n new 5 / prev 5): new 5096, previous 4982, difference +114 ms | yes |
| First idle ≤ previous + 500 ms (firefox, WebGL runs) | all runs: new median 4456 ms, previous 4289 ms, difference +167 ms; means 4482 ± 92 (SE, n=6) / 4288 ± 71 (n=6), difference of the means +194 ± 116 (SE); quiet runs only (load ≤12, I/O avg10 <1; n new 6 / prev 6): new 4456, previous 4289, difference +167 ms | yes |
| File menu ×100 (chrome) | 100/100 | yes |
| File menu ×100 (firefox) | 100/100 | yes |

## Scenes (new bundle)

| browser | scene | result | first idle ms | File ×100 | load, I/O pressure (start→end) |
|---|---|---|---|---|---|
| chrome | d1-sampler-envelope | ok | 5659 | - | load 4.0→4.1, io 0.0→0.0 |
| chrome | d2-browser-folder | ok | 4867 | - | load 4.1→4.3, io 0.0→0.0 |
| chrome | d3-settings-all-tabs | ok | 5143 | - | load 4.3→7.0, io 0.0→0.0 |
| chrome | d4-playlist-middle-drag | ok | 5395 | - | load 7.0→7.6, io 0.0→0.0 |
| chrome | d5-mixer-front | ok | 4736 | - | load 7.6→9.4, io 0.0→0.0 |
| chrome | d6-playlist-close | ok | 5017 | - | load 9.4→8.6, io 0.0→0.0 |
| chrome | d7-main-close | ok (intended exit) | 5398 | - | load 8.6→7.2, io 0.0→0.0 |
| chrome | d8-file-menu-100 | ok | 4637 | 100/100 | load 7.2→12.4, io 0.0→0.0 |
| chrome | d9-keys | ok | 4540 | - | load 12.4→14.2, io 0.0→0.0 |
| chrome | d10-pianoroll-f12 | ok | 5362 | - | load 14.2→12.6, io 0.0→0.0 |
| chrome | d11-menu-hover-escape | ok | 6046 | - | load 12.6→15.4, io 0.0→0.0 |
| chrome | add-menu-simple-fade | ok | 5790 | - | load 13.9→12.8, io 0.0→0.0 |
| chrome | picker-cost | ok | 5525 | - | load 12.8→12.7, io 0.0→0.0 |
| chrome | picker-f8-fade | ok | 4970 | - | load 12.8→12.8, io 0.0→0.0 |
| chrome | plus-more-plugins | ok | 6116 | - | load 14.7→13.9, io 0.0→0.0 |
| firefox | d1-sampler-envelope | ok | 4882 | - | load 4.0→4.1, io 0.2→0.0 |
| firefox | d2-browser-folder | ok | 4813 | - | load 4.4→6.0, io 0.0→0.0 |
| firefox | d3-settings-all-tabs | ok | 4891 | - | load 6.5→8.0, io 0.0→0.0 |
| firefox | d4-playlist-middle-drag | ok | 5156 | - | load 7.6→7.8, io 0.0→0.0 |
| firefox | d5-mixer-front | ok | 4744 | - | load 9.4→8.4, io 0.0→0.0 |
| firefox | d6-playlist-close | ok | 4825 | - | load 8.1→7.0, io 0.0→0.0 |
| firefox | d7-main-close | ok (intended exit) | 5526 | - | load 8.4→9.4, io 0.0→0.0 |
| firefox | d8-file-menu-100 | ok | 5454 | 100/100 | load 9.4→12.6, io 0.0→0.0 |
| firefox | d9-keys | ok | 5365 | - | load 12.6→13.7, io 0.0→0.0 |
| firefox | d10-pianoroll-f12 | ok | 5750 | - | load 14.4→12.8, io 0.0→0.0 |
| firefox | d11-menu-hover-escape | ok | 5288 | - | load 12.8→13.5, io 0.0→0.0 |
| firefox | add-menu-simple-fade | ok | 5090 | - | load 11.5→10.4, io 0.0→0.0 |
| firefox | picker-cost | ok | 4259 | - | load 9.1→8.1, io 0.0→0.0 |
| firefox | picker-f8-fade | ok | 4658 | - | load 10.4→9.1, io 0.0→0.0 |
| firefox | plus-more-plugins | ok | 5647 | - | load 13.6→11.6, io 0.0→0.0 |


The scenes ran as two parallel jobs (Chrome and Firefox), so the first idle values of this table are reference values (the load is next to each).

## alpha7 (main window geometry, guest 1920x1080, Chrome)

result ok; load 7.3→6.8, io 0.0→0.0; windows visible at the end: 1; windows larger than 1800 px wide:
- `14 TFruityLoopsMainForm 'FL Studio 2026' @0,0 1920x1080 z0`
- expected after alpha7: the main window @0,0 1920x1080 (previous df0512b0: @-5,-6 1930x1092). Screenshots: `main-1920x1080.png, scenario-alpha7-look.png`

## First idle (single runs, cold, a new profile each; previous and new alternating)

| browser | bundle | run | first idle ms | load, I/O pressure at the start | at the end | quiet (load ≤12, I/O <1) | webglRenderer | desk.mode |
|---|---|---|---|---|---|---|---|---|
| chrome | prev | 1 | 5092 | 6.10 / io 0.20 | 6.63 / io 1.39 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 1 | 5185 | 6.63 / io 1.39 | 6.81 / io 0.51 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 2 | 4959 | 6.81 / io 0.51 | 6.68 / io 0.18 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 2 | 5200 | 6.68 / io 0.18 | 7.28 / io 0.14 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 3 | 4783 | 7.28 / io 0.14 | 6.78 / io 0.05 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 3 | 4767 | 6.78 / io 0.05 | 6.79 / io 0.01 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 4 | 5161 | 6.79 / io 0.01 | 6.08 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 4 | 4842 | 6.08 / io 0.00 | 6.08 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 5 | 4982 | 6.08 / io 0.00 | 6.91 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 5 | 5096 | 6.91 / io 0.00 | 7.08 / io 0.08 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 6 | 5161 | 7.08 / io 0.08 | 6.46 / io 0.02 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 6 | 5115 | 6.46 / io 0.02 | 5.92 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| firefox | prev | 1 | 4359 | 5.92 / io 0.00 | 5.40 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 1 | 4268 | 5.40 / io 0.00 | 5.59 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 2 | 4273 | 5.59 / io 0.00 | 6.00 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 2 | 3999 | 6.00 / io 0.00 | 6.50 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 3 | 4298 | 6.50 / io 0.00 | 6.04 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 3 | 4400 | 6.04 / io 0.00 | 6.31 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 4 | 4574 | 6.31 / io 0.00 | 5.71 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 4 | 4280 | 5.71 / io 0.00 | 5.28 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 5 | 4259 | 5.28 / io 0.00 | 4.94 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 5 | 4512 | 4.94 / io 0.00 | 5.60 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 6 | 4868 | 5.60 / io 0.00 | 4.96 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 6 | 4534 | 4.96 / io 0.00 | 4.51 / io 0.00 | yes | llvmpipe, or similar | gl |

| browser | mode | previous median (n) | new median (n) | difference |
|---|---|---|---|---|
| chrome | gl | 5037 (6) | 5128 (6) | 92 |
| firefox | gl | 4289 (6) | 4456 (6) | 167 |

## Export (Chrome, the reference project, Ctrl+R)

result ok; load 15.2→10.6, io 0.0→0.0; seconds 345
- WAV `serina1-serum2.wav`: 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`; expected 22863828 B / `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`: MATCH

## Playback (the reference project zip, ~60 s)

not run

### Playback summary (every run; same page, same procedure)

Counter definition: `window.__app.audio.detail()` of the page: `underrunQuanta` = AudioWorklet render quanta (128 frames ≈ 2.9 ms) in which the output ring was empty, counted cumulatively from when the audio device began to play (page start), `episodes` = maximal runs of consecutive underrun quanta (`app/src/audio/underrun.ts`, `ring-layout.ts`). This is the same counter as `underrunQuanta` in opt1's audio-b.json `detail`. **cumulative** = the value at the end (includes the stall while the project is opened); **during playback** = the difference between the reading at the end of `playing-a` (≈19 s after the first reading, 4 s after Space + 10 s playing) and the reading after the stop (≈52 s of playback).

| run | bundle | load at start → end | Worker busy ms/s while playing (min / median / max) | cumulative at the end: quanta / episodes | before the playback (project-open stall) | **during playback: quanta / episodes** | gap frames the engine wrote itself during playback (`engineGapFrames` delta; the engine wrote silence for a hole) |
|---|---|---|---|---|---|---|---|

## Mixer header drag: Worker CPU per move (previous and new alternating)

### P3 decider, chrome: old (previous) vs new, windows of 10 s from the play mark; counted = start load ≤8 and I/O pressure avg10 = 0

| run | bundle | counted | other Chrome browsers at the play mark / at +60 s / max | waited for the quiet window (s) | load start → end | I/O avg10 start → end | Worker CPU s (60 s) = CPU/wall x 60 | main CPU s | underrun quanta 0-60 s | episodes 0-60 s | engine gap frames 0-60 s | **underrun 30-60 s** | write rate mean 10-60 s | consumed rate mean 10-60 s | UI idles/s (10-60 s) | presents/s | lagCuts/s | virtual s per real s | stopped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | prev | reference (overlap: 2 other Chrome) | 2 / None / 2 | 0 | 7.4 → 2.8 | 0.00 → 0.00 | 39.6 | 0.0 | 1514 | 135 | 182175 | **1152** | 0.411 | 0.996 | 200.6 | 7.7 | 0.0 | 1.035 | False |
| 2 | prev | yes | 1 / 1 / 1 | 95 | 3.4 → 4.2 | 0.00 → 0.00 | 41.3 | 0.0 | 1167 | 112 | 137289 | **912** | 0.398 | 0.993 | 196.4 | 7.2 | 0.0 | 1.016 | False |
| 3 | prev | yes | 1 / 1 / 1 | 0 | 3.9 → 3.4 | 0.00 → 0.00 | 40.7 | 0.0 | 1249 | 121 | 149503 | **1007** | 0.403 | 0.999 | 186.6 | 7.5 | 0.0 | 1.024 | False |
| 4 | prev | yes | 1 / 1 / 1 | 65 | 7.6 → 8.2 | 0.00 → 0.12 | 43.4 | 0.0 | 1025 | 103 | 122298 | **832** | 0.409 | 0.997 | 151.4 | 7.0 | 0.0 | 1.009 | False |
| 5 | prev | yes | 1 / 1 / 1 | 115 | 8.0 → 7.4 | 0.00 → 0.00 | 43.7 | 0.0 | 1075 | 104 | 125873 | **941** | 0.401 | 1.000 | 177.8 | 7.0 | 0.0 | 1.019 | False |
| 6 | prev | yes | 1 / 1 / 1 | 25 | 7.8 → 9.1 | 0.00 → 0.03 | 43.7 | 0.0 | 975 | 88 | 117211 | **844** | 0.396 | 1.000 | 200.4 | 7.2 | 0.0 | 1.009 | False |
| 1 | new | yes | 1 / 1 / 1 | 0 | 2.6 → 3.3 | 0.00 → 0.00 | 51.2 | 0.0 | 4 | 2 | 340 | **0** | 0.793 | 0.995 | 333.7 | 13.1 | 15.1 | 0.615 | False |
| 2 | new | yes | 1 / 1 / 1 | 0 | 3.2 → 2.6 | 0.00 → 5.86 | 51.4 | 0.0 | 0 | 0 | 0 | **0** | 0.830 | 0.993 | 269.1 | 13.2 | 15.7 | 0.607 | False |
| 3 | new | yes | 1 / 1 / 1 | 0 | 3.5 → 8.9 | 0.00 → 0.00 | 50.2 | 0.0 | 18 | 5 | 1870 | **8** | 0.762 | 0.995 | 292.7 | 12.0 | 15.0 | 0.569 | False |
| 4 | new | yes | 1 / 1 / 1 | 15 | 7.9 → 11.9 | 0.00 → 0.00 | 50.2 | 0.0 | 36 | 5 | 5935 | **0** | 0.765 | 0.994 | 321.2 | 12.4 | 14.9 | 0.574 | False |
| 5 | new | yes | 1 / 1 / 1 | 0 | 6.8 → 8.2 | 0.00 → 0.00 | 50.0 | 0.0 | 12 | 4 | 1189 | **4** | 0.749 | 0.998 | 326.3 | 12.6 | 15.2 | 0.570 | False |
| 6 | new | yes | 1 / 1 / 1 | 540 | 8.0 → 8.3 | 0.00 → 0.02 | 50.1 | 0.0 | 44 | 9 | 4854 | **17** | 0.682 | 0.997 | 380.0 | 11.3 | 14.7 | 0.553 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 5 | 15 | 3 | 1736 | 0.72 | 1.00 | 1084.6 | 9.9 | 0.0 | 1.00 |
| 0-10 s | new | 6 | 9 | 1 | 1016 | 0.96 | 1.00 | 1057.4 | 12.4 | 7.1 | 0.73 |
| 10-20 s | prev | 5 | 6 | 2 | 681 | 0.49 | 1.00 | 272.2 | 7.9 | 0.0 | 1.00 |
| 10-20 s | new | 6 | 0 | 0 | 6 | 0.89 | 1.00 | 252.5 | 12.6 | 16.4 | 0.61 |
| 20-30 s | prev | 5 | 163 | 18 | 14103 | 0.41 | 1.00 | 183.5 | 7.1 | 0.0 | 1.00 |
| 20-30 s | new | 6 | 0 | 0 | 1 | 0.84 | 1.00 | 385.7 | 13.4 | 14.4 | 0.69 |
| 30-40 s | prev | 5 | 363 | 34 | 47495 | 0.33 | 1.00 | 135.4 | 6.5 | 0.0 | 1.02 |
| 30-40 s | new | 6 | 0 | 0 | 85 | 0.76 | 1.00 | 311.4 | 12.5 | 16.6 | 0.53 |
| 40-50 s | prev | 5 | 459 | 34 | 55447 | 0.33 | 1.00 | 81.9 | 4.9 | 0.0 | 1.04 |
| 40-50 s | new | 6 | 0 | 0 | 7 | 0.70 | 1.00 | 291.6 | 12.1 | 15.7 | 0.50 |
| 50-60 s | prev | 5 | 73 | 17 | 11332 | 0.43 | 1.00 | 245.1 | 9.8 | 0.0 | 1.00 |
| 50-60 s | new | 6 | 0 | 0 | 8 | 0.66 | 0.97 | 352.2 | 11.6 | 13.2 | 0.57 |

Music-matched windows (the same music in both bundles): the music position is the virtual clock since the play mark (gpu1 matched-music.py); the bins are complete 10 s bins of music that each run reached within its 60 s of wall time. wall = the wall time the bin took; distinct frames = Unlock coverage per wall second (1.0 = real time); Unlocks / wall s; underrun quanta and episodes of the page and the engine gap frames over the wall interval of the bin (the 10 s snapshots and the 1 Hz engine reports, linearly interpolated). Median over the counted runs.

| music bin | bundle | n | wall time of the bin (s) | distinct frames per wall s | Unlocks per wall s | underrun quanta | episodes | engine gap frames |
|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 5 | 10.0 | 0.715 | 7.70 | 15 | 3 | 1848 |
| 0-10 s | new | 6 | 15.2 | 0.858 | 10.31 | 9 | 1 | 1023 |
| 10-20 s | prev | 5 | 10.0 | 0.495 | 5.33 | 6 | 2 | 682 |
| 10-20 s | new | 6 | 14.5 | 0.797 | 9.01 | 1 | 0 | 119 |
| 20-30 s | prev | 5 | 10.0 | 0.412 | 4.43 | 162 | 18 | 18630 |
| 20-30 s | new | 6 | 19.1 | 0.729 | 8.01 | 1 | 1 | 306 |
| 30-40 s | prev | 5 | 9.9 | 0.332 | 3.57 | 358 | 33 | 42334 |
| 40-50 s | prev | 5 | 9.6 | 0.334 | 3.60 | 436 | 33 | 53604 |
| 50-60 s | prev | 5 | 10.0 | 0.440 | 4.77 | 127 | 20 | 12940 |

Old vs new at the same music position (counted runs; exact permutation test; only bins both bundles reached):

| music bin | measure | old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|
| 0-10 s | wall time of the bin (s) | 10.003 (5) | 15.237 (6) | +5.234 | 0.002 |
| 0-10 s | distinct frames per wall s | 0.715 (5) | 0.858 (6) | +0.143 | 0.002 |
| 0-10 s | underrun quanta | 14.878 (5) | 8.984 (6) | -5.894 | 0.758 |
| 0-10 s | episodes | 2.975 (5) | 1.498 (6) | -1.477 | 0.500 |
| 0-10 s | engine gap frames | 1848.076 (5) | 1023.000 (6) | -825.076 | 0.771 |
| 10-20 s | wall time of the bin (s) | 9.985 (5) | 14.508 (6) | +4.522 | 0.002 |
| 10-20 s | distinct frames per wall s | 0.495 (5) | 0.797 (6) | +0.302 | 0.002 |
| 10-20 s | underrun quanta | 6.003 (5) | 0.520 (6) | -5.483 | 0.004 |
| 10-20 s | episodes | 1.997 (5) | 0.260 (6) | -1.738 | 0.011 |
| 10-20 s | engine gap frames | 682.000 (5) | 118.575 (6) | -563.425 | 0.080 |
| 20-30 s | wall time of the bin (s) | 9.954 (5) | 19.080 (6) | +9.126 | 0.002 |
| 20-30 s | distinct frames per wall s | 0.412 (5) | 0.729 (6) | +0.317 | 0.002 |
| 20-30 s | underrun quanta | 162.267 (5) | 1.045 (6) | -161.222 | 0.002 |
| 20-30 s | episodes | 17.825 (5) | 0.523 (6) | -17.302 | 0.002 |
| 20-30 s | engine gap frames | 18630.103 (5) | 306.425 (6) | -18323.678 | 0.002 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 1075.430 (5) | 14.966 (6) | -1060.464 | 0.002 | 1121.102 (6) | 14.966 (6) | -1106.136 | 0.002 |
| underrun quanta 30-60 s (heavy interval) | 912.166 (5) | 1.998 (6) | -910.168 | 0.002 | 926.679 (6) | 1.998 (6) | -924.681 | 0.002 |
| episodes 0-60 s | 103.776 (5) | 4.489 (6) | -99.287 | 0.002 | 107.777 (6) | 4.489 (6) | -103.288 | 0.002 |
| engine gap frames 0-60 s | 125873.139 (5) | 1529.635 (6) | -124343.505 | 0.002 | 131581.240 (6) | 1529.635 (6) | -130051.606 | 0.002 |
| write rate mean 10-60 s | 0.401 (5) | 0.763 (6) | +0.362 | 0.002 | 0.402 (6) | 0.763 (6) | +0.361 | 0.002 |
| Worker CPU s (60 s) | 43.400 (5) | 50.205 (6) | +6.805 | 0.002 | 42.335 (6) | 50.205 (6) | +7.870 | 0.002 |
| UI idles/s (10-60 s) | 186.640 (5) | 323.760 (6) | +137.120 | 0.002 | 191.505 (6) | 323.760 (6) | +132.255 | 0.002 |
| presents/s (10-60 s) | 7.154 (5) | 12.485 (6) | +5.331 | 0.002 | 7.194 (6) | 12.485 (6) | +5.291 | 0.002 |
| lagCuts/s (10-60 s) | 0.000 (5) | 15.036 (6) | +15.036 | 0.002 | 0.000 (6) | 15.036 (6) | +15.036 | 0.002 |
| virtual s per real s (10-60 s) | 1.016 (5) | 0.572 (6) | -0.444 | 0.002 | 1.017 (6) | 0.572 (6) | -0.445 | 0.002 |



## Steps (load and I/O pressure at each step)

```
13:27:10 fetched fl-engine-ci-62098b85 (260M) load 7.06 io avg10=0.00 avg60=0.00 avg300=0.00
13:27:18 built the page of 38efb398 load 6.65 io avg10=0.00 avg60=0.00 avg300=0.00
13:28:21 preflight ok load 6.45 io avg10=0.46 avg60=0.19 avg300=0.04
13:30:30 idle chrome x6 load 5.92 io avg10=0.00 avg60=0.08 avg300=0.06
13:32:34 idle firefox x6 load 4.51 io avg10=0.00 avg60=0.00 avg300=0.03
13:32:44 scenes chrome scn-client.json  load 3.97 io avg10=0.00 avg60=0.00 avg300=0.03
13:32:44 scenes firefox scn-client.json d1-sampler-envelope load 3.97 io avg10=0.00 avg60=0.00 avg300=0.03
13:33:18 scenes firefox scn-client.json d2-browser-folder load 4.36 io avg10=0.00 avg60=0.01 avg300=0.03
13:33:41 scenes firefox scn-client.json d3-settings-all-tabs load 5.98 io avg10=0.00 avg60=0.01 avg300=0.02
13:34:28 scenes firefox scn-client.json d4-playlist-middle-drag load 7.99 io avg10=0.00 avg60=0.00 avg300=0.01
13:34:52 scenes firefox scn-client.json d5-mixer-front load 7.78 io avg10=0.00 avg60=0.00 avg300=0.00
13:35:22 scenes firefox scn-client.json d6-playlist-close load 8.43 io avg10=0.00 avg60=0.00 avg300=0.00
13:35:42 scenes firefox scn-client.json d7-main-close load 7.04 io avg10=0.00 avg60=0.00 avg300=0.00
13:36:08 scenes firefox scn-client.json d8-file-menu-100 load 9.36 io avg10=0.00 avg60=0.00 avg300=0.00
13:39:05 scenes firefox scn-client.json d9-keys load 12.62 io avg10=0.00 avg60=0.00 avg300=0.00
13:39:34 scenes chrome scn-plus.json  load 14.69 io avg10=0.00 avg60=0.00 avg300=0.00
13:39:53 scenes firefox scn-client.json d10-pianoroll-f12 load 14.38 io avg10=0.00 avg60=0.00 avg300=0.00
13:40:00 scenes chrome scn-alpha1.json  load 13.95 io avg10=0.00 avg60=0.00 avg300=0.00
13:40:29 scenes firefox scn-client.json d11-menu-hover-escape load 12.85 io avg10=0.00 avg60=0.00 avg300=0.00
13:41:02 scenes firefox scn-plus.json  load 13.46 io avg10=0.00 avg60=0.00 avg300=0.00
13:41:31 scenes firefox scn-alpha1.json  load 11.62 io avg10=0.00 avg60=0.00 avg300=0.00
13:43:09 alpha7 load 7.29 io avg10=0.00 avg60=0.00 avg300=0.00
13:43:42 logo new 1 load 6.82 io avg10=0.00 avg60=0.00 avg300=0.00
13:44:02 logo new 1 end load 6.87 io avg10=0.00 avg60=0.00 avg300=0.00
13:44:02 logo prev 1 load 6.87 io avg10=0.00 avg60=0.00 avg300=0.00
13:44:23 logo prev 1 end load 6.45 io avg10=0.00 avg60=0.00 avg300=0.00
13:44:23 logo new 2 load 6.45 io avg10=0.00 avg60=0.00 avg300=0.00
13:44:45 logo new 2 end load 7.34 io avg10=0.02 avg60=0.02 avg300=0.00
13:44:45 logo prev 2 load 7.34 io avg10=0.02 avg60=0.02 avg300=0.00
13:45:06 logo prev 2 end load 7.24 io avg10=0.00 avg60=0.01 avg300=0.00
13:45:06 logo new 3 load 7.24 io avg10=0.00 avg60=0.01 avg300=0.00
13:45:27 logo new 3 end load 8.91 io avg10=0.00 avg60=0.00 avg300=0.00
13:45:27 logo prev 3 load 8.91 io avg10=0.00 avg60=0.00 avg300=0.00
13:45:48 logo prev 3 end load 8.43 io avg10=0.00 avg60=0.00 avg300=0.00
13:45:48 title load 8.43 io avg10=0.00 avg60=0.00 avg300=0.00
13:48:21 title end load 15.24 io avg10=0.00 avg60=0.00 avg300=0.00
13:48:21 export start load 15.24 io avg10=0.00 avg60=0.00 avg300=0.00
13:54:08 export end load 10.61 io avg10=0.00 avg60=0.00 avg300=0.00
14:06:00 p3 chrome prev 1 (quiet: load 7.8 io 0 after 0 s) load 7.80 io avg10=0.00 avg60=0.01 avg300=0.08
14:16:04 p3 chrome prev 1 end load 10.04 io avg10=0.00 avg60=0.01 avg300=0.04
14:18:04 p3 chrome new 1 (quiet: load 7.73 io 0 after 120 s) load 7.73 io avg10=0.00 avg60=0.00 avg300=0.01
14:26:47 built the page of 9fdc492f load 2.95 io avg10=0.00 avg60=0.02 avg300=0.00
14:28:09 p3 chrome new 1 end load 1.43 io avg10=0.12 avg60=1.58 avg300=0.65
14:40:00 p3 chrome prev 1 (quiet: load 7.37 io 0 after 0 s) load 7.37 io avg10=0.00 avg60=0.00 avg300=0.05
14:41:55 p3 chrome prev 1 end load 2.58 io avg10=0.00 avg60=0.00 avg300=0.02
14:41:55 p3 chrome new 1 (quiet: load 2.58 io 0 after 0 s) load 2.58 io avg10=0.00 avg60=0.00 avg300=0.02
14:43:44 p3 chrome new 1 end load 3.24 io avg10=0.00 avg60=0.00 avg300=0.00
14:43:44 p3 chrome new 2 (quiet: load 3.24 io 0 after 0 s) load 3.24 io avg10=0.00 avg60=0.00 avg300=0.00
14:45:35 p3 chrome new 2 end load 2.84 io avg10=5.52 avg60=3.55 avg300=0.99
14:47:10 p3 chrome prev 2 (quiet: load 3.36 io 0 after 95 s) load 3.36 io avg10=0.00 avg60=1.19 avg300=1.02
14:49:00 p3 chrome prev 2 end load 3.85 io avg10=0.00 avg60=0.19 avg300=0.69
14:49:00 p3 chrome prev 3 (quiet: load 3.85 io 0 after 0 s) load 3.85 io avg10=0.00 avg60=0.19 avg300=0.69
14:50:49 p3 chrome prev 3 end load 3.54 io avg10=0.00 avg60=0.03 avg300=0.47
14:50:49 p3 chrome new 3 (quiet: load 3.54 io 0 after 0 s) load 3.54 io avg10=0.00 avg60=0.03 avg300=0.47
14:52:40 p3 chrome new 3 end load 8.43 io avg10=0.00 avg60=0.00 avg300=0.31
14:52:55 p3 chrome new 4 (quiet: load 7.89 io 0 after 15 s) load 7.89 io avg10=0.00 avg60=0.00 avg300=0.29
14:54:45 p3 chrome new 4 end load 12.13 io avg10=0.00 avg60=0.00 avg300=0.19
14:55:50 p3 chrome prev 4 (quiet: load 7.63 io 0 after 65 s) load 7.63 io avg10=0.00 avg60=0.00 avg300=0.14
14:57:41 p3 chrome prev 4 end load 8.25 io avg10=0.09 avg60=0.02 avg300=0.09
14:59:36 p3 chrome prev 5 (quiet: load 7.97 io 0 after 115 s) load 7.97 io avg10=0.00 avg60=0.00 avg300=0.05
15:01:30 p3 chrome prev 5 end load 6.77 io avg10=0.00 avg60=0.00 avg300=0.02
15:01:30 p3 chrome new 5 (quiet: load 6.77 io 0 after 0 s) load 6.77 io avg10=0.00 avg60=0.00 avg300=0.02
15:03:20 p3 chrome new 5 end load 8.46 io avg10=0.00 avg60=0.00 avg300=0.00
15:12:21 p3 chrome new 6 (quiet: load 8 io 0 after 540 s) load 8.00 io avg10=0.00 avg60=0.00 avg300=0.00
15:14:12 p3 chrome new 6 end load 8.34 io avg10=0.02 avg60=0.02 avg300=0.00
15:14:37 p3 chrome prev 6 (quiet: load 7.82 io 0 after 25 s) load 7.82 io avg10=0.00 avg60=0.01 avg300=0.00
15:16:31 p3 chrome prev 6 end load 9.05 io avg10=0.02 avg60=0.05 avg300=0.01
15:32:09 lat chrome prev 1 (quiet: load 7.67 io 0 after 305 s) load 7.67 io avg10=0.00 avg60=2.84 avg300=3.95
15:36:54 lat chrome prev 1 end load 9.13 io avg10=0.00 avg60=0.07 avg300=1.59
15:39:35 lat chrome new 1 (quiet: load 7.77 io 0 after 160 s) load 7.77 io avg10=0.00 avg60=0.00 avg300=0.90
15:44:27 lat chrome new 1 end load 18.96 io avg10=0.06 avg60=0.17 avg300=0.41
15:55:52 lat chrome new 2 (quiet: load 7.21 io 0 after 686 s) load 7.21 io avg10=0.00 avg60=0.05 avg300=0.04
16:00:39 lat chrome new 2 end load 10.52 io avg10=0.02 avg60=0.04 avg300=0.01
16:01:29 lat chrome prev 2 (quiet: load 7.77 io 0 after 50 s) load 7.77 io avg10=0.00 avg60=0.01 avg300=0.00
16:06:17 lat chrome prev 2 end load 15.06 io avg10=0.00 avg60=0.10 avg300=0.16
16:07:12 lat chrome prev 3 (quiet: load 7.93 io 0 after 55 s) load 7.93 io avg10=0.00 avg60=0.03 avg300=0.13
16:11:56 lat chrome prev 3 end load 10.97 io avg10=0.00 avg60=0.00 avg300=0.04
16:13:36 lat chrome new 3 (quiet: load 7.86 io 0 after 100 s) load 7.86 io avg10=0.00 avg60=0.00 avg300=0.01
16:18:06 lat chrome new 3 end load 5.81 io avg10=0.00 avg60=0.00 avg300=0.00
16:19:13 fetched fl-engine-ci-dc5fae2b (260M) load 4.48 io avg10=0.00 avg60=0.00 avg300=0.00
16:19:30 fetched fl-engine-ci-432c65ec (260M) load 4.24 io avg10=0.00 avg60=0.00 avg300=0.00
16:19:41 built the page of 7373cd55 load 3.67 io avg10=0.00 avg60=0.00 avg300=0.00
```
