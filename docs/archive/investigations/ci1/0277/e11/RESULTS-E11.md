# 0277: verification of `fl-engine-ci-dc5fae2b` (Worker 00769751-6314-4262-8464-151fcd043354, page of git 7373cd55) against the previous production bundle `fl-engine-ci-432c65ec`

Real engine in real browsers (Chrome / Firefox, headless; Firefox on Xvfb :77 with WebGL). No stand-in, no replay. The bundle comes from R2 (read only); every manifest sha256 is checked by `fetch`.

## Pass criteria

| criterion | value | pass |
|---|---|---|
| Firefox has WebGL (preflight) | preflight: webglRenderer llvmpipe, or similar desk.mode gl first idle 4428 | yes |
| No stop (11 scenes + more, both browsers, play, export; d7 is the intended exit) | stops: none; not ok: none; no result at all (the harness did not start): none | yes |
| Export WAV equals the reference | 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` (expected 22863828 B, `381cdb5acf891a16…`) | yes |
| First idle ≤ previous + 500 ms (chrome, WebGL runs) | all runs: new median 4866 ms, previous 4682 ms, difference +184 ms; means 5076 ± 156 (SE, n=10) / 5060 ± 346 (n=10), difference of the means +16 ± 380 (SE); quiet runs only (load ≤12, I/O avg10 <1; n new 10 / prev 9): new 4866, previous 4695, difference +171 ms | yes |
| First idle ≤ previous + 500 ms (firefox, WebGL runs) | all runs: new median 4485 ms, previous 4381 ms, difference +104 ms; means 4593 ± 120 (SE, n=6) / 4357 ± 64 (n=6), difference of the means +236 ± 136 (SE); quiet runs only (load ≤12, I/O avg10 <1; n new 4 / prev 4): new 4485, previous 4325, difference +160 ms | yes |
| File menu ×100 (chrome) | 100/100 | yes |
| File menu ×100 (firefox) | 100/100 | yes |

## Scenes (new bundle)

| browser | scene | result | first idle ms | File ×100 | load, I/O pressure (start→end) |
|---|---|---|---|---|---|
| chrome | d1-sampler-envelope | ok | 4699 | - | load 3.0→2.9, io 0.0→0.1 |
| chrome | d2-browser-folder | ok | 4144 | - | load 2.9→2.9, io 0.1→0.0 |
| chrome | d3-settings-all-tabs | ok | 4986 | - | load 2.9→3.4, io 0.0→0.0 |
| chrome | d4-playlist-middle-drag | ok | 4287 | - | load 3.4→3.6, io 0.0→0.0 |
| chrome | d5-mixer-front | ok | 4679 | - | load 3.6→4.6, io 0.0→0.0 |
| chrome | d6-playlist-close | ok | 4196 | - | load 4.6→4.6, io 0.0→0.0 |
| chrome | d7-main-close | ok (intended exit) | 4475 | - | load 4.6→5.4, io 0.0→1.0 |
| chrome | d8-file-menu-100 | ok | 4619 | 100/100 | load 5.4→10.3, io 1.0→0.0 |
| chrome | d9-keys | ok | 3679 | - | load 10.3→9.7, io 0.0→0.0 |
| chrome | d10-pianoroll-f12 | ok | 4569 | - | load 9.7→9.9, io 0.0→0.1 |
| chrome | d11-menu-hover-escape | ok | 5281 | - | load 9.9→10.1, io 0.1→0.0 |
| chrome | add-menu-simple-fade | ok | 5961 | - | load 15.3→15.4, io 0.1→0.0 |
| chrome | picker-cost | ok | 5309 | - | load 15.5→16.1, io 0.0→0.1 |
| chrome | picker-f8-fade | ok | 4613 | - | load 15.4→15.5, io 0.0→0.0 |
| chrome | plus-more-plugins | ok | 5855 | - | load 10.1→14.1, io 0.0→0.1 |
| firefox | d1-sampler-envelope | ok | 4259 | - | load 3.4→2.8, io 0.4→0.0 |
| firefox | d2-browser-folder | ok | 4343 | - | load 2.8→3.2, io 0.0→0.0 |
| firefox | d3-settings-all-tabs | ok | 4275 | - | load 3.0→3.6, io 0.0→0.0 |
| firefox | d4-playlist-middle-drag | ok | 4690 | - | load 3.6→4.8, io 0.0→0.0 |
| firefox | d5-mixer-front | ok | 4316 | - | load 4.6→4.7, io 0.0→0.0 |
| firefox | d6-playlist-close | ok | 4212 | - | load 4.7→6.1, io 0.0→2.0 |
| firefox | d7-main-close | ok (intended exit) | 4629 | - | load 6.1→6.2, io 2.4→3.0 |
| firefox | d8-file-menu-100 | ok | 4870 | 100/100 | load 6.2→9.9, io 3.3→0.1 |
| firefox | d9-keys | ok | 5493 | - | load 9.9→13.2, io 0.1→0.0 |
| firefox | d10-pianoroll-f12 | ok | 5508 | - | load 13.9→15.3, io 0.2→0.0 |
| firefox | d11-menu-hover-escape | ok | 5148 | - | load 15.3→16.6, io 0.0→0.0 |
| firefox | add-menu-simple-fade | ok | 4724 | - | load 14.6→14.4, io 0.0→0.0 |
| firefox | picker-cost | ok | 4304 | - | load 12.3→10.3, io 0.0→0.0 |
| firefox | picker-f8-fade | ok | 4559 | - | load 14.4→12.3, io 0.0→0.0 |
| firefox | plus-more-plugins | ok | 5532 | - | load 16.6→14.8, io 0.0→0.0 |


The scenes ran as two parallel jobs (Chrome and Firefox), so the first idle values of this table are reference values (the load is next to each).

## alpha7 (main window geometry, guest 1920x1080, Chrome)

result ok; load 13.0→15.0, io 0.0→0.0; windows visible at the end: 1; windows larger than 1800 px wide:
- `14 TFruityLoopsMainForm 'FL Studio 2026' @0,0 1920x1080 z0`
- expected after alpha7: the main window @0,0 1920x1080 (previous df0512b0: @-5,-6 1930x1092). Screenshots: `main-1920x1080.png, scenario-alpha7-look.png`

## First idle (single runs, cold, a new profile each; previous and new alternating)

| browser | bundle | run | first idle ms | load, I/O pressure at the start | at the end | quiet (load ≤12, I/O <1) | webglRenderer | desk.mode |
|---|---|---|---|---|---|---|---|---|
| chrome | prev | 1 | 5304 | 6.92 / io 0.00 | 7.07 / io 0.10 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 1 | 5285 | 5.42 / io 0.00 | 4.96 / io 0.24 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 2 | 5025 | 3.12 / io 0.00 | 3.18 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 2 | 4822 | 3.18 / io 0.00 | 3.22 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 3 | 4695 | 3.22 / io 0.00 | 4.66 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 3 | 4840 | 4.66 / io 0.00 | 4.56 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 4 | 4837 | 4.56 / io 0.00 | 4.54 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 4 | 4545 | 4.54 / io 0.00 | 4.37 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 5 | 4478 | 4.37 / io 0.00 | 4.23 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 5 | 4686 | 4.23 / io 0.00 | 4.18 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 6 | 4758 | 4.18 / io 0.00 | 3.92 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 6 | 4658 | 3.92 / io 0.00 | 4.17 / io 2.35 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 7 | 4790 | 8.00 / io 0.00 | 7.84 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 7 | 4867 | 7.84 / io 0.00 | 7.85 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 8 | 4865 | 7.85 / io 0.00 | 7.63 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 8 | 4670 | 7.63 / io 0.00 | 7.44 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 9 | 8102 | 7.44 / io 0.00 | 7.29 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 9 | 6368 | 7.29 / io 0.00 | 7.60 / io 0.31 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 10 | 5231 | 5.70 / io 0.00 | 6.13 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 10 | 4536 | 6.13 / io 0.00 | 5.95 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| firefox | prev | 1 | 4394 | 6.49 / io 0.00 | 7.15 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 1 | 4299 | 7.15 / io 0.00 | 7.35 / io 0.09 | yes | llvmpipe, or similar | gl |
| firefox | new | 2 | 4723 | 6.87 / io 0.00 | 6.62 / io 0.26 | yes | llvmpipe, or similar | gl |
| firefox | prev | 2 | 4388 | 7.82 / io 0.00 | 8.08 / io 0.10 | no (reference) | llvmpipe, or similar | gl |
| firefox | prev | 3 | 4276 | 7.94 / io 0.00 | 7.94 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 3 | 4519 | 7.94 / io 0.00 | 7.78 / io 0.08 | yes | llvmpipe, or similar | gl |
| firefox | new | 4 | 4451 | 7.76 / io 0.00 | 7.89 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 4 | 4594 | 7.89 / io 0.00 | 8.28 / io 0.14 | no (reference) | llvmpipe, or similar | gl |
| firefox | prev | 5 | 4374 | 7.79 / io 0.00 | 7.96 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 5 | 4445 | 7.96 / io 0.00 | 8.48 / io 0.00 | no (reference) | llvmpipe, or similar | gl |
| firefox | new | 6 | 5120 | 7.77 / io 0.00 | 8.19 / io 0.00 | no (reference) | llvmpipe, or similar | gl |
| firefox | prev | 6 | 4117 | 7.84 / io 0.00 | 7.39 / io 0.00 | yes | llvmpipe, or similar | gl |

| browser | mode | previous median (n) | new median (n) | difference |
|---|---|---|---|---|
| chrome | gl | 4682 (10) | 4866 (10) | 184 |
| firefox | gl | 4381 (6) | 4485 (6) | 104 |

## Export (Chrome, the reference project, Ctrl+R)

result ok; load 15.5→13.0, io 0.0→0.0; seconds 309
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
| 1 | prev | reference (overlap: 2 other Chrome) | 2 / None / 2 | 105 | 7.8 → 8.6 | 0.00 → 0.00 | 49.7 | 0.0 | 18 | 7 | 1702 | **5** | 0.875 | 0.996 | 260.4 | 13.2 | 17.4 | 0.637 | False |
| 2 | prev | yes | 1 / 0 / 1 | 0 | 5.5 → 4.7 | 0.00 → 0.00 | 50.4 | 0.0 | 4 | 2 | 340 | **0** | 0.959 | 0.994 | 243.6 | 14.9 | 18.7 | 0.689 | False |
| 3 | prev | yes | 0 / 0 / 0 | 0 | 4.7 → 3.8 | 0.00 → 0.00 | 50.7 | 0.0 | 14 | 3 | 1534 | **0** | 0.951 | 1.000 | 253.9 | 14.9 | 18.7 | 0.684 | False |
| 4 | prev | yes | 0 / 0 / 0 | 0 | 4.9 → 5.1 | 0.00 → 0.00 | 50.6 | 0.0 | 8 | 2 | 852 | **0** | 0.951 | 0.998 | 255.7 | 15.2 | 18.8 | 0.692 | False |
| 5 | prev | yes | 0 / 0 / 0 | 0 | 5.1 → 5.6 | 0.00 → 0.00 | 50.4 | 0.0 | 37 | 4 | 4392 | **26** | 0.923 | 1.000 | 247.3 | 14.8 | 19.2 | 0.664 | False |
| 6 | prev | yes | 0 / 0 / 0 | 0 | 7.4 → 6.4 | 0.00 → 0.00 | 50.5 | 0.0 | 11 | 2 | 1236 | **0** | 0.933 | 1.000 | 257.8 | 14.4 | 18.3 | 0.674 | False |
| 1 | new | yes | 1 / 1 / 1 | 30 | 7.8 → 8.3 | 0.00 → 0.00 | 50.1 | 0.0 | 12 | 4 | 1192 | **6** | 0.929 | 1.000 | 270.3 | 14.5 | 18.6 | 0.676 | False |
| 2 | new | NO (not counted: load / I/O) | 1 / 1 / 1 | 5 | 7.9 → 5.5 | 0.18 → 0.00 | 49.6 | 0.0 | 17 | 3 | 1918 | **0** | 0.973 | 0.998 | 309.3 | 15.9 | 19.0 | 0.734 | False |
| 3 | new | yes | 0 / 0 / 0 | 0 | 3.8 → 4.4 | 0.00 → 0.00 | 50.1 | 0.0 | 7 | 3 | 638 | **0** | 0.964 | 1.000 | 299.4 | 16.0 | 18.6 | 0.719 | False |
| 4 | new | yes | 0 / 0 / 0 | 0 | 4.4 → 4.9 | 0.00 → 0.00 | 50.0 | 0.0 | 4 | 2 | 340 | **0** | 0.964 | 0.998 | 289.4 | 16.1 | 19.9 | 0.716 | False |
| 5 | new | yes | 0 / 0 / 0 | 0 | 5.6 → 8.0 | 0.00 → 0.00 | 50.0 | 0.0 | 10 | 3 | 1022 | **0** | 0.955 | 0.996 | 297.7 | 15.6 | 17.8 | 0.708 | False |
| 6 | new | yes | 0 / 0 / 0 | 5 | 7.8 → 7.4 | 0.00 → 0.00 | 50.0 | 0.0 | 9 | 3 | 894 | **0** | 0.977 | 1.000 | 297.1 | 15.9 | 18.6 | 0.726 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 5 | 11 | 2 | 1236 | 0.98 | 1.00 | 1113.0 | 14.6 | 7.7 | 0.76 |
| 0-10 s | new | 5 | 7 | 3 | 638 | 0.98 | 1.00 | 1146.7 | 15.1 | 8.0 | 0.78 |
| 10-20 s | prev | 5 | 0 | 0 | 0 | 0.98 | 1.00 | 249.4 | 16.2 | 21.4 | 0.67 |
| 10-20 s | new | 5 | 0 | 0 | 0 | 0.99 | 1.00 | 327.1 | 17.2 | 20.9 | 0.72 |
| 20-30 s | prev | 5 | 0 | 0 | 0 | 1.00 | 1.00 | 286.2 | 16.5 | 18.5 | 0.71 |
| 20-30 s | new | 5 | 0 | 0 | 0 | 0.99 | 1.00 | 417.6 | 17.9 | 17.6 | 0.76 |
| 30-40 s | prev | 5 | 0 | 0 | 0 | 1.00 | 1.00 | 189.7 | 15.5 | 21.7 | 0.61 |
| 30-40 s | new | 5 | 0 | 0 | 0 | 1.00 | 1.00 | 257.2 | 16.7 | 22.1 | 0.64 |
| 40-50 s | prev | 5 | 0 | 0 | 0 | 0.92 | 1.00 | 208.7 | 13.4 | 17.2 | 0.66 |
| 40-50 s | new | 5 | 0 | 0 | 0 | 0.94 | 1.00 | 216.4 | 14.6 | 19.9 | 0.68 |
| 50-60 s | prev | 5 | 0 | 0 | 0 | 0.82 | 1.00 | 331.5 | 12.9 | 14.6 | 0.76 |
| 50-60 s | new | 5 | 0 | 0 | 0 | 0.88 | 1.00 | 278.4 | 12.6 | 13.5 | 0.78 |

Music-matched windows (the same music in both bundles): the music position is the virtual clock since the play mark (gpu1 matched-music.py); the bins are complete 10 s bins of music that each run reached within its 60 s of wall time. wall = the wall time the bin took; distinct frames = Unlock coverage per wall second (1.0 = real time); Unlocks / wall s; underrun quanta and episodes of the page and the engine gap frames over the wall interval of the bin (the 10 s snapshots and the 1 Hz engine reports, linearly interpolated). Median over the counted runs.

| music bin | bundle | n | wall time of the bin (s) | distinct frames per wall s | Unlocks per wall s | underrun quanta | episodes | engine gap frames |
|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 5 | 14.1 | 0.942 | 10.56 | 11 | 2 | 1236 |
| 0-10 s | new | 5 | 13.6 | 0.967 | 10.55 | 7 | 3 | 638 |
| 10-20 s | prev | 5 | 13.5 | 0.839 | 10.60 | 0 | 0 | 0 |
| 10-20 s | new | 5 | 12.6 | 0.879 | 10.74 | 0 | 0 | 0 |
| 20-30 s | prev | 5 | 16.8 | 0.815 | 10.73 | 0 | 0 | 0 |
| 20-30 s | new | 5 | 15.5 | 0.852 | 10.78 | 0 | 0 | 0 |
| 30-40 s | prev | 5 | 13.5 | 0.810 | 9.44 | 0 | 0 | 0 |
| 30-40 s | new | 5 | 13.8 | 0.830 | 9.99 | 0 | 0 | 0 |

Old vs new at the same music position (counted runs; exact permutation test; only bins both bundles reached):

| music bin | measure | old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|
| 0-10 s | wall time of the bin (s) | 14.128 (5) | 13.633 (5) | -0.495 | 0.040 |
| 0-10 s | distinct frames per wall s | 0.942 (5) | 0.967 (5) | +0.025 | 0.198 |
| 0-10 s | underrun quanta | 11.000 (5) | 7.000 (5) | -4.000 | 0.310 |
| 0-10 s | episodes | 2.000 (5) | 3.000 (5) | +1.000 | 0.206 |
| 0-10 s | engine gap frames | 1236.000 (5) | 638.000 (5) | -598.000 | 0.175 |
| 10-20 s | wall time of the bin (s) | 13.476 (5) | 12.574 (5) | -0.902 | 0.024 |
| 10-20 s | distinct frames per wall s | 0.839 (5) | 0.879 (5) | +0.040 | 0.056 |
| 10-20 s | underrun quanta | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |
| 10-20 s | episodes | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |
| 10-20 s | engine gap frames | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |
| 20-30 s | wall time of the bin (s) | 16.773 (5) | 15.487 (5) | -1.286 | 0.048 |
| 20-30 s | distinct frames per wall s | 0.815 (5) | 0.852 (5) | +0.036 | 0.214 |
| 20-30 s | underrun quanta | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |
| 20-30 s | episodes | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |
| 20-30 s | engine gap frames | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |
| 30-40 s | wall time of the bin (s) | 13.503 (5) | 13.793 (5) | +0.290 | 0.048 |
| 30-40 s | distinct frames per wall s | 0.810 (5) | 0.830 (5) | +0.020 | 0.397 |
| 30-40 s | underrun quanta | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |
| 30-40 s | episodes | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |
| 30-40 s | engine gap frames | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 11.000 (5) | 9.000 (5) | -2.000 | 0.405 | 12.500 (6) | 9.500 (6) | -3.000 | 0.370 |
| underrun quanta 30-60 s (heavy interval) | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 | 0.000 (6) | 0.000 (6) | +0.000 | 0.727 |
| episodes 0-60 s | 2.000 (5) | 3.000 (5) | +1.000 | 0.683 | 2.500 (6) | 3.000 (6) | +0.500 | 0.909 |
| engine gap frames 0-60 s | 1236.000 (5) | 894.000 (5) | -342.000 | 0.270 | 1385.000 (6) | 958.000 (6) | -427.000 | 0.351 |
| write rate mean 10-60 s | 0.951 (5) | 0.964 (5) | +0.013 | 0.190 | 0.942 (6) | 0.964 (6) | +0.022 | 0.061 |
| Worker CPU s (60 s) | 50.480 (5) | 50.010 (5) | -0.470 | 0.008 | 50.455 (6) | 49.995 (6) | -0.460 | 0.032 |
| UI idles/s (10-60 s) | 253.916 (5) | 297.053 (5) | +43.137 | 0.008 | 254.805 (6) | 297.393 (6) | +42.588 | 0.002 |
| presents/s (10-60 s) | 14.864 (5) | 15.863 (5) | +0.999 | 0.040 | 14.811 (6) | 15.895 (6) | +1.084 | 0.011 |
| lagCuts/s (10-60 s) | 18.715 (5) | 18.609 (5) | -0.106 | 0.921 | 18.710 (6) | 18.623 (6) | -0.087 | 0.545 |
| virtual s per real s (10-60 s) | 0.684 (5) | 0.716 (5) | +0.032 | 0.032 | 0.679 (6) | 0.717 (6) | +0.038 | 0.009 |



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
16:49:49 idle chrome prev 1 (quiet: load 6.92 io 0 after 40 s) load 6.92 io avg10=0.00 avg60=0.01 avg300=0.01
16:50:24 idle chrome new 1 (quiet: load 5.42 io 0 after 25 s) load 5.42 io avg10=0.00 avg60=0.02 avg300=0.01
16:51:14 idle chrome new 2 (quiet: load 3.12 io 0 after 40 s) load 3.12 io avg10=0.00 avg60=0.06 avg300=0.03
16:51:24 idle chrome prev 2 (quiet: load 3.18 io 0 after 0 s) load 3.18 io avg10=0.00 avg60=0.05 avg300=0.03
16:51:33 idle chrome prev 3 (quiet: load 3.22 io 0 after 0 s) load 3.22 io avg10=0.00 avg60=0.04 avg300=0.02
16:51:43 idle chrome new 3 (quiet: load 4.66 io 0 after 0 s) load 4.66 io avg10=0.00 avg60=0.04 avg300=0.02
16:51:53 idle chrome new 4 (quiet: load 4.56 io 0 after 0 s) load 4.56 io avg10=0.00 avg60=0.03 avg300=0.02
16:52:02 idle chrome prev 4 (quiet: load 4.54 io 0 after 0 s) load 4.54 io avg10=0.00 avg60=0.02 avg300=0.02
16:52:12 idle chrome prev 5 (quiet: load 4.37 io 0 after 0 s) load 4.37 io avg10=0.00 avg60=0.02 avg300=0.02
16:52:22 idle chrome new 5 (quiet: load 4.23 io 0 after 0 s) load 4.23 io avg10=0.00 avg60=0.01 avg300=0.01
16:52:31 idle chrome new 6 (quiet: load 4.18 io 0 after 0 s) load 4.18 io avg10=0.00 avg60=0.01 avg300=0.01
16:52:41 idle chrome prev 6 (quiet: load 3.92 io 0 after 0 s) load 3.92 io avg10=0.00 avg60=0.01 avg300=0.01
16:54:46 idle chrome prev 7 (quiet: load 8 io 0 after 115 s) load 8.00 io avg10=0.00 avg60=0.07 avg300=0.07
16:54:55 idle chrome new 7 (quiet: load 7.84 io 0 after 0 s) load 7.84 io avg10=0.00 avg60=0.06 avg300=0.07
16:55:05 idle chrome new 8 (quiet: load 7.85 io 0 after 0 s) load 7.85 io avg10=0.00 avg60=0.05 avg300=0.07
16:55:15 idle chrome prev 8 (quiet: load 7.63 io 0 after 0 s) load 7.63 io avg10=0.00 avg60=0.04 avg300=0.06
16:55:24 idle chrome prev 9 (quiet: load 7.44 io 0 after 0 s) load 7.44 io avg10=0.00 avg60=0.03 avg300=0.06
16:55:38 idle chrome new 9 (quiet: load 7.29 io 0 after 0 s) load 7.29 io avg10=0.00 avg60=0.02 avg300=0.06
16:56:25 idle chrome new 10 (quiet: load 5.7 io 0 after 35 s) load 5.70 io avg10=0.00 avg60=0.06 avg300=0.06
16:56:36 idle chrome prev 10 (quiet: load 6.13 io 0 after 0 s) load 6.13 io avg10=0.00 avg60=0.04 avg300=0.06
16:56:45 idle chrome x10 load 5.95 io avg10=0.00 avg60=0.04 avg300=0.06
17:02:21 scenes firefox scn-client.json d1-sampler-envelope load 2.97 io avg10=0.00 avg60=0.00 avg300=0.00
17:02:21 scenes chrome scn-client.json  load 2.97 io avg10=0.00 avg60=0.00 avg300=0.00
17:02:57 scenes firefox scn-client.json d2-browser-folder load 2.77 io avg10=0.02 avg60=0.05 avg300=0.01
17:03:19 scenes firefox scn-client.json d3-settings-all-tabs load 3.21 io avg10=0.00 avg60=0.03 avg300=0.00
17:04:03 scenes firefox scn-client.json d4-playlist-middle-drag load 3.60 io avg10=0.00 avg60=0.01 avg300=0.00
17:04:26 scenes firefox scn-client.json d5-mixer-front load 4.82 io avg10=0.02 avg60=0.03 avg300=0.00
17:04:53 scenes firefox scn-client.json d6-playlist-close load 4.74 io avg10=0.00 avg60=0.01 avg300=0.00
17:05:12 scenes firefox scn-client.json d7-main-close load 6.06 io avg10=2.01 avg60=0.55 avg300=0.12
17:05:37 scenes firefox scn-client.json d8-file-menu-100 load 6.24 io avg10=3.17 avg60=1.46 avg300=0.37
17:08:14 scenes firefox scn-client.json d9-keys load 9.89 io avg10=0.08 avg60=0.37 avg300=0.51
17:08:45 scenes chrome scn-plus.json  load 10.10 io avg10=0.00 avg60=0.21 avg300=0.45
17:09:02 scenes firefox scn-client.json d10-pianoroll-f12 load 13.89 io avg10=0.00 avg60=0.16 avg300=0.43
17:09:11 scenes chrome scn-alpha1.json  load 14.06 io avg10=0.09 avg60=0.16 avg300=0.42
17:09:37 scenes firefox scn-client.json d11-menu-hover-escape load 15.35 io avg10=0.00 avg60=0.10 avg300=0.38
17:10:09 scenes firefox scn-plus.json  load 16.64 io avg10=0.01 avg60=0.08 avg300=0.34
17:10:27 export start load 15.46 io avg10=0.04 avg60=0.08 avg300=0.33
17:10:39 scenes firefox scn-alpha1.json  load 14.76 io avg10=0.01 avg60=0.06 avg300=0.31
17:12:41 preflight ok load 9.50 io avg10=0.00 avg60=0.00 avg300=0.19
17:15:38 export end load 12.98 io avg10=0.00 avg60=0.00 avg300=0.09
17:15:38 alpha7 load 12.98 io avg10=0.00 avg60=0.00 avg300=0.09
17:17:44 p3 chrome prev 1 (quiet: load 7.84 io 0 after 105 s) load 7.84 io avg10=0.00 avg60=0.00 avg300=0.05
17:19:34 p3 chrome prev 1 end load 8.56 io avg10=0.00 avg60=0.00 avg300=0.02
17:20:04 p3 chrome new 1 (quiet: load 7.77 io 0 after 30 s) load 7.77 io avg10=0.00 avg60=0.00 avg300=0.02
17:21:55 p3 chrome new 1 end load 8.35 io avg10=0.00 avg60=0.00 avg300=0.00
17:22:00 p3 chrome new 2 (quiet: load 7.92 io 0 after 5 s) load 7.92 io avg10=0.00 avg60=0.00 avg300=0.00
17:23:50 p3 chrome new 2 end load 5.45 io avg10=0.00 avg60=0.00 avg300=0.00
17:23:50 p3 chrome prev 2 (quiet: load 5.45 io 0 after 0 s) load 5.45 io avg10=0.00 avg60=0.00 avg300=0.00
17:25:39 p3 chrome prev 2 end load 4.72 io avg10=0.00 avg60=0.00 avg300=0.00
17:25:39 p3 chrome prev 3 (quiet: load 4.72 io 0 after 0 s) load 4.72 io avg10=0.00 avg60=0.00 avg300=0.00
17:27:29 p3 chrome prev 3 end load 3.75 io avg10=0.00 avg60=0.02 avg300=0.00
17:27:29 p3 chrome new 3 (quiet: load 3.75 io 0 after 0 s) load 3.75 io avg10=0.00 avg60=0.02 avg300=0.00
17:40:23 idle firefox prev 1 (quiet: load 6.49 io 0 after 0 s) load 6.49 io avg10=0.00 avg60=0.00 avg300=0.04
17:40:34 idle firefox new 1 (quiet: load 7.15 io 0 after 0 s) load 7.15 io avg10=0.00 avg60=0.00 avg300=0.03
17:41:05 idle firefox new 2 (quiet: load 6.87 io 0 after 20 s) load 6.87 io avg10=0.00 avg60=0.02 avg300=0.03
17:41:55 idle firefox prev 2 (quiet: load 7.82 io 0 after 35 s) load 7.82 io avg10=0.00 avg60=0.05 avg300=0.04
17:42:36 idle firefox prev 3 (quiet: load 7.94 io 0 after 30 s) load 7.94 io avg10=0.00 avg60=0.04 avg300=0.04
17:42:47 idle firefox new 3 (quiet: load 7.94 io 0 after 0 s) load 7.94 io avg10=0.00 avg60=0.03 avg300=0.04
17:43:17 idle firefox new 4 (quiet: load 7.76 io 0 after 20 s) load 7.76 io avg10=0.00 avg60=0.03 avg300=0.04
17:43:28 idle firefox prev 4 (quiet: load 7.89 io 0 after 0 s) load 7.89 io avg10=0.00 avg60=0.03 avg300=0.03
17:44:03 idle firefox prev 5 (quiet: load 7.79 io 0 after 25 s) load 7.79 io avg10=0.00 avg60=0.03 avg300=0.03
17:44:14 idle firefox new 5 (quiet: load 7.96 io 0 after 0 s) load 7.96 io avg10=0.00 avg60=0.02 avg300=0.03
17:44:40 idle firefox new 6 (quiet: load 7.77 io 0 after 15 s) load 7.77 io avg10=0.00 avg60=0.01 avg300=0.02
17:50:21 idle firefox prev 6 (quiet: load 7.84 io 0 after 330 s) load 7.84 io avg10=0.00 avg60=0.00 avg300=0.00
17:50:30 idle firefox x6 load 7.39 io avg10=0.00 avg60=0.00 avg300=0.00
17:50:46 fetched fl-engine-ci-7a9b0800 (260M) load 7.92 io avg10=0.00 avg60=0.00 avg300=0.00
17:50:46 built the page of a977401d load 7.92 io avg10=0.00 avg60=0.00 avg300=0.00
```
