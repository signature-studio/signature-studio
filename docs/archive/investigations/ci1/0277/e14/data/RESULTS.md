# 0277: verification of `fl-engine-ci-0363b050+PV7` (Worker ?, page of git e14-PV7) against the previous production bundle `fl-engine-ci-0363b050`

Real engine in real browsers (Chrome / Firefox, headless; Firefox on Xvfb :77 with WebGL). No stand-in, no replay. The bundle comes from R2 (read only); every manifest sha256 is checked by `fetch`.

## Pass criteria

| criterion | value | pass |
|---|---|---|
| Firefox has WebGL (preflight) | preflight: webglRenderer llvmpipe, or similar desk.mode None first idle 4149 | yes |
| No stop (11 scenes + more, both browsers, play, export; d7 is the intended exit) | stops: none; not ok: none; no result at all (the harness did not start): none | NO or not complete |
| Export WAV equals the reference | 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` (expected 22863828 B, `381cdb5acf891a16…`) | yes |
| First idle ≤ previous + 500 ms (chrome, WebGL runs) | all runs: new median 4570 ms, previous 4591 ms, difference -22 ms; means 4555 ± 27 (SE, n=10) / 4602 ± 43 (n=10), difference of the means -46 ± 51 (SE); quiet runs only (load ≤12, I/O avg10 <1; n new 10 / prev 10): new 4570, previous 4591, difference -22 ms | yes |
| First idle ≤ previous + 500 ms (firefox, WebGL runs) | all runs: new median 4107 ms, previous 4206 ms, difference -100 ms; means 4127 ± 56 (SE, n=6) / 4213 ± 127 (n=6), difference of the means -86 ± 139 (SE); quiet runs only (load ≤12, I/O avg10 <1; n new 6 / prev 6): new 4107, previous 4206, difference -100 ms | yes |
| File menu ×100 (chrome) | 100/100 | yes |
| File menu ×100 (firefox) | not run | not run |

## Scenes (new bundle)

| browser | scene | result | first idle ms | File ×100 | load, I/O pressure (start→end) |
|---|---|---|---|---|---|
| chrome | d1-sampler-envelope | ok | 5230 | - | load 1.4→1.7, io 0.0→0.0 |
| chrome | d2-browser-folder | ok | 4649 | - | load 1.7→1.7, io 0.0→0.0 |
| chrome | d3-settings-all-tabs | ok | 4761 | - | load 1.7→3.5, io 0.0→0.0 |
| chrome | d4-playlist-middle-drag | ok | 4888 | - | load 3.5→3.4, io 0.0→0.0 |
| chrome | d5-mixer-front | ok | 4867 | - | load 3.4→3.6, io 0.0→0.1 |
| chrome | d6-playlist-close | ok | 4973 | - | load 3.6→3.4, io 0.1→0.0 |
| chrome | d7-main-close | ok (intended exit) | 4950 | - | load 3.4→3.2, io 0.0→0.0 |
| chrome | d8-file-menu-100 | ok | 4644 | 100/100 | load 3.2→3.4, io 0.0→0.0 |
| chrome | d9-keys | ok | 4025 | - | load 3.4→5.0, io 0.0→0.0 |
| chrome | d10-pianoroll-f12 | ok | 5657 | - | load 5.0→5.7, io 0.0→0.0 |
| chrome | d11-menu-hover-escape | ok | 5250 | - | load 5.7→7.8, io 0.0→0.0 |
| chrome | add-menu-simple-fade | ok | 5252 | - | load 8.1→7.0, io 0.0→0.0 |
| chrome | picker-cost | ok | 4630 | - | load 6.8→5.2, io 0.0→0.0 |
| chrome | picker-f8-fade | ok | 4546 | - | load 7.0→6.8, io 0.0→0.0 |
| chrome | plus-more-plugins | ok | 5902 | - | load 7.8→8.1, io 0.0→0.0 |


The scenes ran as two parallel jobs (Chrome and Firefox), so the first idle values of this table are reference values (the load is next to each).

## alpha7 (main window geometry, guest 1920x1080, Chrome)

result ok; load 3.0→3.2, io 0.0→0.0; windows visible at the end: 1; windows larger than 1800 px wide:
- `14 TFruityLoopsMainForm 'FL Studio 2026' @0,0 1920x1080 z0`
- expected after alpha7: the main window @0,0 1920x1080 (previous df0512b0: @-5,-6 1930x1092). Screenshots: `main-1920x1080.png, scenario-alpha7-look.png`

## First idle (single runs, cold, a new profile each; previous and new alternating)

| browser | bundle | run | first idle ms | load, I/O pressure at the start | at the end | other Chrome browsers during the run (max) | quiet (load ≤8, I/O <1) | webglRenderer | desk.mode |
|---|---|---|---|---|---|---|---|---|---|
| chrome | prev | 1 | 4645 | 1.17 / io 0.00 | 1.52 / io 0.16 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 1 | 4637 | 0.99 / io 0.00 | 1.14 / io 0.20 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 2 | 4544 | 0.70 / io 0.00 | 0.89 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 2 | 4541 | 0.89 / io 0.00 | 1.20 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 3 | 4522 | 1.10 / io 0.00 | 1.46 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 3 | 4421 | 1.34 / io 0.00 | 1.28 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 4 | 4414 | 1.00 / io 0.00 | 1.22 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 4 | 4609 | 1.22 / io 0.00 | 1.63 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 5 | 4486 | 1.38 / io 0.00 | 1.32 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 5 | 4596 | 1.11 / io 0.00 | 1.17 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 6 | 4639 | 1.17 / io 0.00 | 2.63 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 6 | 4933 | 2.42 / io 0.00 | 2.34 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 7 | 4601 | 2.34 / io 0.00 | 2.21 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 7 | 4504 | 2.21 / io 0.00 | 2.40 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 8 | 4571 | 2.36 / io 0.00 | 2.52 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 8 | 4581 | 2.52 / io 0.00 | 2.29 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 9 | 4665 | 2.29 / io 0.00 | 2.31 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 9 | 4568 | 2.11 / io 0.00 | 2.01 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 10 | 4660 | 2.01 / io 0.00 | 2.07 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 10 | 4434 | 2.07 / io 0.00 | 1.98 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| firefox | prev | 1 | 3941 | 1.03 / io 0.00 | 1.10 / io 0.00 | 0 | yes | llvmpipe, or similar | gl |
| firefox | new | 1 | 4115 | 1.01 / io 0.00 | 1.57 / io 0.00 | 0 | yes | llvmpipe, or similar | gl |
| firefox | new | 2 | 4393 | 1.44 / io 0.00 | 1.37 / io 0.00 | 0 | yes | llvmpipe, or similar | gl |
| firefox | prev | 2 | 3887 | 1.37 / io 0.00 | 1.47 / io 0.00 | 0 | yes | llvmpipe, or similar | gl |
| firefox | prev | 3 | 4208 | 1.35 / io 0.00 | 1.32 / io 0.00 | 0 | yes | llvmpipe, or similar | gl |
| firefox | new | 3 | 4103 | 1.22 / io 0.00 | 1.33 / io 0.00 | 0 | yes | llvmpipe, or similar | gl |
| firefox | new | 4 | 4002 | 1.33 / io 0.00 | 1.35 / io 0.00 | 0 | yes | llvmpipe, or similar | gl |
| firefox | prev | 4 | 4271 | 1.35 / io 0.00 | 1.60 / io 0.00 | 0 | yes | llvmpipe, or similar | gl |
| firefox | prev | 5 | 4205 | 1.60 / io 0.00 | 1.65 / io 0.00 | 0 | yes | llvmpipe, or similar | gl |
| firefox | new | 5 | 4039 | 1.60 / io 0.00 | 1.95 / io 0.00 | 0 | yes | llvmpipe, or similar | gl |
| firefox | new | 6 | 4111 | 1.79 / io 0.00 | 1.73 / io 0.00 | 0 | yes | llvmpipe, or similar | gl |
| firefox | prev | 6 | 4764 | 1.46 / io 0.00 | 1.54 / io 0.00 | 0 | yes | llvmpipe, or similar | gl |

| browser | mode | previous median (n) | new median (n) | difference |
|---|---|---|---|---|
| chrome | gl | 4591 (10) | 4570 (10) | -22 |
| firefox | gl | 4206 (6) | 4107 (6) | -100 |

## Export (Chrome, the reference project, Ctrl+R)

result ok; load 5.2→3.0, io 0.0→0.0; seconds 319
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
| 1 | prev | yes | 0 / 0 / 0 | 2 | 0.9 → 2.7 | 0.00 → 0.00 | 50.3 | 0.0 | 8 | 2 | 852 | **0** | 0.962 | 1.000 | 284.9 | 15.8 | 19.3 | 0.707 | False |
| 2 | prev | yes | 0 / 0 / 0 | 7 | 3.2 → 3.1 | 0.00 → 0.00 | 50.1 | 0.0 | 2 | 1 | 170 | **0** | 0.910 | 1.000 | 298.0 | 15.0 | 18.4 | 0.675 | False |
| 3 | prev | yes | 0 / 0 / 0 | 2 | 2.9 → 2.9 | 0.00 → 0.00 | 51.4 | 0.0 | 8 | 2 | 852 | **0** | 0.879 | 1.000 | 262.4 | 12.4 | 16.3 | 0.636 | False |
| 4 | prev | yes | 0 / 0 / 0 | 7 | 2.0 → 2.0 | 0.00 → 0.00 | 51.2 | 0.0 | 14 | 3 | 1534 | **0** | 0.884 | 1.000 | 283.4 | 12.9 | 16.3 | 0.652 | False |
| 5 | prev | yes | 0 / 0 / 0 | 2 | 2.0 → 1.4 | 0.00 → 0.00 | 51.2 | 0.0 | 3 | 1 | 298 | **0** | 0.914 | 1.000 | 237.8 | 13.2 | 18.1 | 0.652 | False |
| 6 | prev | yes | 0 / 0 / 0 | 7 | 3.7 → 4.1 | 0.00 → 0.00 | 50.7 | 0.0 | 7 | 2 | 724 | **0** | 0.916 | 1.000 | 246.9 | 14.0 | 18.5 | 0.661 | False |
| 1 | new | yes | 0 / 0 / 0 | 2 | 2.5 → 3.6 | 0.00 → 0.00 | 70.5 | 0.0 | 4 | 2 | 340 | **0** | 0.953 | 0.997 | 856.7 | 29.9 | 0.2 | 0.978 | False |
| 2 | new | yes | 0 / 0 / 0 | 2 | 3.3 → 3.4 | 0.00 → 0.00 | 70.7 | 0.0 | 2 | 1 | 170 | **0** | 0.947 | 0.995 | 859.6 | 30.1 | 0.9 | 0.973 | False |
| 3 | new | yes | 0 / 0 / 0 | 2 | 2.9 → 2.8 | 0.00 → 0.00 | 74.3 | 0.0 | 7 | 3 | 638 | **0** | 0.918 | 0.995 | 717.9 | 29.4 | 4.7 | 0.886 | False |
| 4 | new | yes | 0 / 0 / 0 | 2 | 2.6 → 2.3 | 0.00 → 0.00 | 74.7 | 0.0 | 10 | 1 | 1194 | **0** | 0.877 | 0.995 | 730.2 | 29.2 | 4.6 | 0.864 | False |
| 5 | new | yes | 0 / 0 / 0 | 2 | 1.3 → 3.7 | 0.00 → 0.00 | 74.0 | 0.0 | 10 | 3 | 1022 | **0** | 0.927 | 1.000 | 759.2 | 29.3 | 4.9 | 0.911 | False |
| 6 | new | yes | 0 / 0 / 0 | 2 | 3.7 → 4.2 | 0.00 → 0.00 | 73.0 | 0.0 | 18 | 3 | 2046 | **0** | 0.947 | 0.995 | 736.6 | 29.5 | 3.0 | 0.913 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 8 | 2 | 788 | 0.98 | 1.00 | 1135.1 | 13.8 | 7.1 | 0.77 |
| 0-10 s | new | 6 | 8 | 2 | 830 | 0.98 | 1.00 | 1516.1 | 20.1 | 0.0 | 0.96 |
| 10-20 s | prev | 6 | 0 | 0 | 0 | 0.97 | 1.00 | 238.3 | 13.2 | 19.0 | 0.64 |
| 10-20 s | new | 6 | 0 | 0 | 0 | 0.97 | 1.00 | 889.8 | 29.3 | 0.5 | 0.98 |
| 20-30 s | prev | 6 | 0 | 0 | 0 | 0.97 | 1.00 | 302.3 | 14.8 | 17.1 | 0.71 |
| 20-30 s | new | 6 | 0 | 0 | 0 | 0.99 | 1.00 | 755.8 | 29.5 | 1.3 | 0.93 |
| 30-40 s | prev | 6 | 0 | 0 | 0 | 0.97 | 1.00 | 200.0 | 14.0 | 19.6 | 0.59 |
| 30-40 s | new | 6 | 0 | 0 | 0 | 0.98 | 1.00 | 576.9 | 29.7 | 7.1 | 0.79 |
| 40-50 s | prev | 6 | 0 | 0 | 0 | 0.92 | 1.00 | 196.9 | 12.6 | 18.2 | 0.63 |
| 40-50 s | new | 6 | 0 | 0 | 0 | 0.92 | 1.00 | 614.6 | 29.6 | 6.7 | 0.86 |
| 50-60 s | prev | 6 | 0 | 0 | 0 | 0.75 | 1.00 | 397.2 | 12.9 | 13.7 | 0.75 |
| 50-60 s | new | 6 | 0 | 0 | 0 | 0.79 | 0.98 | 991.4 | 29.2 | 0.3 | 1.03 |

Music-matched windows (the same music in both bundles): the music position is the virtual clock since the play mark (gpu1 matched-music.py); the bins are complete 10 s bins of music that each run reached within its 60 s of wall time. wall = the wall time the bin took; distinct frames = Unlock coverage per wall second (1.0 = real time); Unlocks / wall s; underrun quanta and episodes of the page and the engine gap frames over the wall interval of the bin (the 10 s snapshots and the 1 Hz engine reports, linearly interpolated). Median over the counted runs.

| music bin | bundle | n | wall time of the bin (s) | distinct frames per wall s | Unlocks per wall s | underrun quanta | episodes | engine gap frames |
|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 14.1 | 0.938 | 10.53 | 8 | 2 | 788 |
| 0-10 s | new | 6 | 10.4 | 0.953 | 10.46 | 8 | 2 | 830 |
| 10-20 s | prev | 6 | 14.0 | 0.857 | 10.45 | 0 | 0 | 0 |
| 10-20 s | new | 6 | 10.4 | 0.887 | 10.48 | 0 | 0 | 0 |
| 20-30 s | prev | 6 | 17.1 | 0.803 | 10.45 | 0 | 0 | 0 |
| 20-30 s | new | 6 | 11.1 | 0.828 | 10.73 | 0 | 0 | 0 |
| 30-40 s | prev | 5 | 13.6 | 0.758 | 8.77 | 0 | 0 | 0 |
| 30-40 s | new | 6 | 12.6 | 0.786 | 10.65 | 0 | 0 | 0 |
| 40-50 s | new | 6 | 10.1 | 0.775 | 9.02 | 0 | 0 | 0 |

Old vs new at the same music position (counted runs; exact permutation test; only bins both bundles reached):

| music bin | measure | old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|
| 0-10 s | wall time of the bin (s) | 14.119 (6) | 10.428 (6) | -3.690 | 0.002 |
| 0-10 s | distinct frames per wall s | 0.938 (6) | 0.953 (6) | +0.015 | 0.139 |
| 0-10 s | underrun quanta | 7.500 (6) | 8.500 (6) | +1.000 | 0.654 |
| 0-10 s | episodes | 2.000 (6) | 2.500 (6) | +0.500 | 0.732 |
| 0-10 s | engine gap frames | 788.000 (6) | 830.000 (6) | +42.000 | 0.639 |
| 10-20 s | wall time of the bin (s) | 13.995 (6) | 10.432 (6) | -3.563 | 0.002 |
| 10-20 s | distinct frames per wall s | 0.857 (6) | 0.887 (6) | +0.029 | 0.240 |
| 10-20 s | underrun quanta | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 10-20 s | episodes | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 10-20 s | engine gap frames | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 20-30 s | wall time of the bin (s) | 17.121 (6) | 11.144 (6) | -5.977 | 0.002 |
| 20-30 s | distinct frames per wall s | 0.803 (6) | 0.828 (6) | +0.025 | 0.416 |
| 20-30 s | underrun quanta | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 20-30 s | episodes | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 20-30 s | engine gap frames | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 30-40 s | wall time of the bin (s) | 13.600 (5) | 12.608 (6) | -0.992 | 0.071 |
| 30-40 s | distinct frames per wall s | 0.758 (5) | 0.786 (6) | +0.028 | 0.582 |
| 30-40 s | underrun quanta | 0.000 (5) | 0.000 (6) | +0.000 | 1.000 |
| 30-40 s | episodes | 0.000 (5) | 0.000 (6) | +0.000 | 1.000 |
| 30-40 s | engine gap frames | 0.000 (5) | 0.000 (6) | +0.000 | 1.000 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 7.500 (6) | 8.500 (6) | +1.000 | 0.654 | 7.500 (6) | 8.500 (6) | +1.000 | 0.654 |
| underrun quanta 30-60 s (heavy interval) | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| episodes 0-60 s | 2.000 (6) | 2.500 (6) | +0.500 | 0.732 | 2.000 (6) | 2.500 (6) | +0.500 | 0.732 |
| engine gap frames 0-60 s | 788.000 (6) | 830.000 (6) | +42.000 | 0.677 | 788.000 (6) | 830.000 (6) | +42.000 | 0.677 |
| write rate mean 10-60 s | 0.912 (6) | 0.937 (6) | +0.025 | 0.348 | 0.912 (6) | 0.937 (6) | +0.025 | 0.348 |
| Worker CPU s (60 s) | 50.955 (6) | 73.530 (6) | +22.575 | 0.002 | 50.955 (6) | 73.530 (6) | +22.575 | 0.002 |
| UI idles/s (10-60 s) | 272.927 (6) | 747.919 (6) | +474.992 | 0.002 | 272.927 (6) | 747.919 (6) | +474.992 | 0.002 |
| presents/s (10-60 s) | 13.584 (6) | 29.441 (6) | +15.857 | 0.002 | 13.584 (6) | 29.441 (6) | +15.857 | 0.002 |
| lagCuts/s (10-60 s) | 18.248 (6) | 3.774 (6) | -14.474 | 0.002 | 18.248 (6) | 3.774 (6) | -14.474 | 0.002 |
| virtual s per real s (10-60 s) | 0.656 (6) | 0.912 (6) | +0.255 | 0.002 | 0.656 (6) | 0.912 (6) | +0.255 | 0.002 |



## Steps (load and I/O pressure at each step)

```
03:39:20 E14 PV7 functional checks start (scenes chrome, export, alpha7; not timing) load 1.45 io avg10=0.00
03:39:20 scenes chrome scn-client.json  load 1.45 io avg10=0.00 avg60=0.06 avg300=0.02
03:45:47 scenes chrome scn-plus.json  load 7.77 io avg10=0.00 avg60=0.00 avg300=0.00
03:46:13 scenes chrome scn-alpha1.json  load 8.10 io avg10=0.00 avg60=0.00 avg300=0.00
03:47:26 export start load 5.16 io avg10=0.00 avg60=0.00 avg300=0.00
03:52:48 export end load 3.02 io avg10=0.00 avg60=0.05 avg300=0.23
03:52:48 alpha7 load 3.02 io avg10=0.00 avg60=0.05 avg300=0.23
03:53:05 E14 PV7 functional checks end load 3.24 io avg10=0.00
04:18:12 mem note
Mem:           20863        9115        2344         374       10628       11747
Swap:           4095        2678        1417
 1  0 2743128 2406848 1444324 9439000    0    0     0     0 2185 1114  7  0 93  0  0  0
 1  0 2743128 2388792 1444324 9439000    0    0     0  1420 3273 2594  6  1 93  0  0  0
04:51:08 G9-a test bundle Export (PM window): pre-run memory note
Mem:           21771        7434        4986         333       10246       14336
Swap:           4095        3008        1087
 1  0 3080396 5102752 1392948 9099836    0    0     0    48 4161 3239  7  1 93  0  0  0
 1  0 3080392 5201340 1392948 9099832    4    0     4     0 3179 1490 11  0 88  0  0  0
04:52:48 G9-a test bundle Export start check
Mem:           21771        4877        7518         334       10270       16893
Swap:           4095        3008        1087
 0  0 3080304 7703444 1393188 9124304    0    0     0     0 1099  915  0  0 100  0  0  0
 0  0 3080300 7709312 1393188 9124304    8    0     8     0 2878 3047  1  1 99  0  0  0
04:53:17 G9-a test bundle Export start (gpu1 port 4969, assets-g9a = fl-engine-test-g9a-28f6f631; PM go; vmstat last 4 samples si/so 0; wait-quiet swap 0/0) load 0.52 io avg10=0.00 avg60=0.00 avg300=0.00
04:58:29 G9-a test bundle Export end rc=0 load 0.89 io avg10=0.00 avg60=0.00 avg300=0.00
04:59:50 mem note after G9-a Export
Mem:           29401        4923       14964         394       10500       24477
Swap:           4095        3005        1090
05:54:12 E14 first idle window start check (PM go)
Mem:           24321        5638        6253         489       13491       18682
Swap:           4095        2921        1174
 0  0 2991252 6410848 2142740 11672648    0    0     0    80 2556 2635  0  0 100  0  0  0
 0  0 2991252 6442412 2142740 11672648    0    0     0     4 2091 2200  0  0 100  0  0  0
05:54:41 E14 first idle chain start (A=5053 E13, B=5063 P+V7; idle chrome 10, preflight, idle firefox 6) load 1.17 io avg10=0.00
05:54:43 idle chrome prev 1 (quiet: load 1.17 io 0 after 2 s; mem avail 18892 MB, swap used 2921 MB, swap in/out 0/0 pages (2 s)) load 1.17 io avg10=0.00 avg60=0.00 avg300=0.00
05:55:50 idle chrome new 1 (quiet: load 0.99 io 0 after 57 s; mem avail 14838 MB, swap used 2922 MB, swap in/out 0/0 pages (2 s)) load 0.99 io avg10=0.00 avg60=0.03 avg300=0.00
05:56:31 idle chrome new 2 (quiet: load 0.76 io 0 after 32 s; mem avail 22460 MB, swap used 2892 MB, swap in/out 0/0 pages (2 s)) load 0.70 io avg10=0.00 avg60=0.04 avg300=0.00
05:56:43 idle chrome prev 2 (quiet: load 0.89 io 0 after 2 s; mem avail 22835 MB, swap used 2891 MB, swap in/out 0/0 pages (2 s)) load 0.89 io avg10=0.00 avg60=0.03 avg300=0.00
05:57:00 idle chrome prev 3 (quiet: load 1.1 io 0 after 7 s; mem avail 22997 MB, swap used 2890 MB, swap in/out 0/0 pages (2 s)) load 1.10 io avg10=0.00 avg60=0.02 avg300=0.00
05:57:11 idle chrome new 3 (quiet: load 1.46 io 0 after 2 s; mem avail 22815 MB, swap used 2889 MB, swap in/out 0/0 pages (2 s)) load 1.34 io avg10=0.00 avg60=0.02 avg300=0.00
05:57:38 idle chrome new 4 (quiet: load 1 io 0 after 17 s; mem avail 23128 MB, swap used 2887 MB, swap in/out 0/0 pages (2 s)) load 1.00 io avg10=0.00 avg60=0.01 avg300=0.00
05:57:50 idle chrome prev 4 (quiet: load 1.22 io 0 after 2 s; mem avail 22863 MB, swap used 2887 MB, swap in/out 0/0 pages (2 s)) load 1.22 io avg10=0.00 avg60=0.00 avg300=0.00
05:58:06 idle chrome prev 5 (quiet: load 1.5 io 0 after 7 s; mem avail 23257 MB, swap used 2886 MB, swap in/out 0/0 pages (2 s)) load 1.38 io avg10=0.00 avg60=0.00 avg300=0.00
05:58:28 idle chrome new 5 (quiet: load 1.11 io 0 after 12 s; mem avail 23327 MB, swap used 2886 MB, swap in/out 0/0 pages (2 s)) load 1.11 io avg10=0.00 avg60=0.00 avg300=0.00
05:58:39 idle chrome new 6 (quiet: load 1.17 io 0 after 2 s; mem avail 22964 MB, swap used 2886 MB, swap in/out 0/0 pages (2 s)) load 1.17 io avg10=0.00 avg60=0.00 avg300=0.00
05:58:51 idle chrome prev 6 (quiet: load 2.63 io 0 after 2 s; mem avail 22994 MB, swap used 2886 MB, swap in/out 0/0 pages (2 s)) load 2.42 io avg10=0.00 avg60=0.00 avg300=0.00
05:59:03 idle chrome prev 7 (quiet: load 2.34 io 0 after 2 s; mem avail 23092 MB, swap used 2886 MB, swap in/out 0/0 pages (2 s)) load 2.34 io avg10=0.00 avg60=0.00 avg300=0.00
05:59:14 idle chrome new 7 (quiet: load 2.21 io 0 after 2 s; mem avail 23050 MB, swap used 2886 MB, swap in/out 0/0 pages (2 s)) load 2.21 io avg10=0.00 avg60=0.00 avg300=0.00
05:59:26 idle chrome new 8 (quiet: load 2.4 io 0 after 2 s; mem avail 22659 MB, swap used 2886 MB, swap in/out 0/0 pages (2 s)) load 2.36 io avg10=0.00 avg60=0.00 avg300=0.00
05:59:38 idle chrome prev 8 (quiet: load 2.52 io 0 after 2 s; mem avail 22916 MB, swap used 2885 MB, swap in/out 0/0 pages (2 s)) load 2.52 io avg10=0.00 avg60=0.00 avg300=0.00
05:59:49 idle chrome prev 9 (quiet: load 2.29 io 0 after 2 s; mem avail 23059 MB, swap used 2885 MB, swap in/out 0/0 pages (2 s)) load 2.29 io avg10=0.00 avg60=0.00 avg300=0.00
06:00:06 idle chrome new 9 (quiet: load 2.21 io 0 after 7 s; mem avail 23239 MB, swap used 2884 MB, swap in/out 0/0 pages (2 s)) load 2.11 io avg10=0.00 avg60=0.00 avg300=0.00
06:00:17 idle chrome new 10 (quiet: load 2.01 io 0 after 2 s; mem avail 22900 MB, swap used 2884 MB, swap in/out 0/0 pages (2 s)) load 2.01 io avg10=0.00 avg60=0.00 avg300=0.00
06:00:29 idle chrome prev 10 (quiet: load 2.07 io 0 after 2 s; mem avail 22920 MB, swap used 2884 MB, swap in/out 0/0 pages (2 s)) load 2.07 io avg10=0.00 avg60=0.00 avg300=0.00
06:00:39 idle chrome x10 load 1.98 io avg10=0.00 avg60=0.00 avg300=0.00
06:00:52 PREFLIGHT FAILED: no WebGL in Firefox (rc 2): stop and report load 1.91 io avg10=0.42 avg60=0.14 avg300=0.03
06:01:34 idle firefox prev 1 (quiet: load 1.03 io 0 after 42 s; mem avail 23641 MB, swap used 2882 MB, swap in/out 0/0 pages (2 s)) load 1.03 io avg10=0.00 avg60=0.06 avg300=0.02
06:01:45 idle firefox new 1 (quiet: load 1.1 io 0 after 2 s; mem avail 23437 MB, swap used 2882 MB, swap in/out 0/0 pages (2 s)) load 1.01 io avg10=0.00 avg60=0.05 avg300=0.01
06:01:57 idle firefox new 2 (quiet: load 1.57 io 0 after 2 s; mem avail 22838 MB, swap used 2882 MB, swap in/out 0/0 pages (2 s)) load 1.44 io avg10=0.00 avg60=0.04 avg300=0.01
06:02:09 idle firefox prev 2 (quiet: load 1.37 io 0 after 2 s; mem avail 22918 MB, swap used 2882 MB, swap in/out 0/0 pages (2 s)) load 1.37 io avg10=0.00 avg60=0.03 avg300=0.01
06:02:20 idle firefox prev 3 (quiet: load 1.47 io 0 after 2 s; mem avail 23045 MB, swap used 2882 MB, swap in/out 0/0 pages (2 s)) load 1.35 io avg10=0.00 avg60=0.02 avg300=0.01
06:02:32 idle firefox new 3 (quiet: load 1.32 io 0 after 2 s; mem avail 22941 MB, swap used 2882 MB, swap in/out 0/0 pages (2 s)) load 1.22 io avg10=0.00 avg60=0.02 avg300=0.00
06:02:43 idle firefox new 4 (quiet: load 1.33 io 0 after 2 s; mem avail 22705 MB, swap used 2882 MB, swap in/out 0/0 pages (2 s)) load 1.33 io avg10=0.00 avg60=0.01 avg300=0.00
06:02:55 idle firefox prev 4 (quiet: load 1.35 io 0 after 2 s; mem avail 22656 MB, swap used 2882 MB, swap in/out 0/0 pages (2 s)) load 1.35 io avg10=0.00 avg60=0.01 avg300=0.00
06:03:07 idle firefox prev 5 (quiet: load 1.6 io 0 after 2 s; mem avail 22804 MB, swap used 2881 MB, swap in/out 0/0 pages (2 s)) load 1.60 io avg10=0.00 avg60=0.00 avg300=0.00
06:03:24 idle firefox new 5 (quiet: load 1.6 io 0 after 7 s; mem avail 22906 MB, swap used 2881 MB, swap in/out 0/0 pages (2 s)) load 1.60 io avg10=0.00 avg60=0.00 avg300=0.00
06:03:35 idle firefox new 6 (quiet: load 1.95 io 0 after 2 s; mem avail 22791 MB, swap used 2881 MB, swap in/out 0/0 pages (2 s)) load 1.79 io avg10=0.00 avg60=0.00 avg300=0.00
06:03:52 idle firefox prev 6 (quiet: load 1.59 io 0 after 7 s; mem avail 23116 MB, swap used 2881 MB, swap in/out 0/0 pages (2 s)) load 1.46 io avg10=0.00 avg60=0.00 avg300=0.00
06:04:02 idle firefox x6 load 1.54 io avg10=0.00 avg60=0.00 avg300=0.00
06:04:02 E14 first idle chain end rc=0 load 1.54 io avg10=0.00
06:05:04 E14 pure chrome 6 start check
Mem:           28821        5243       10606         485       14047       23577
Swap:           4095        2881        1214
 0  0 2950252 10865044 2214968 12169228    0    0     0     0 1160 1140  0  0 100  0  0  0
 0  0 2950252 10868396 2214968 12169228    0    0     0     0 2317 2812  0  0 99  0  0  0
06:05:12 pure start (E14 PM go; A=5053 E13, B=5063 P+V7) load 0.87 io avg10=0.00
06:05:14 p3 chrome prev 1 (quiet: load 0.87 io 0 after 2 s; mem avail 23527 MB, swap used 2881 MB, swap in/out 0/0 pages (2 s)) load 0.87 io avg10=0.00 avg60=0.00 avg300=0.00
06:07:04 p3 chrome prev 1 end load 2.71 io avg10=0.00 avg60=0.00 avg300=0.00
06:07:06 p3 chrome new 1 (quiet: load 2.71 io 0 after 2 s; mem avail 23830 MB, swap used 2881 MB, swap in/out 0/0 pages (2 s)) load 2.49 io avg10=0.00 avg60=0.00 avg300=0.00
06:08:56 p3 chrome new 1 end load 3.34 io avg10=0.00 avg60=0.00 avg300=0.00
06:08:58 p3 chrome new 2 (quiet: load 3.34 io 0 after 2 s; mem avail 24056 MB, swap used 2877 MB, swap in/out 0/0 pages (2 s)) load 3.34 io avg10=0.00 avg60=0.00 avg300=0.00
06:10:47 p3 chrome new 2 end load 3.44 io avg10=0.00 avg60=0.00 avg300=0.00
06:10:54 p3 chrome prev 2 (quiet: load 3.17 io 0 after 7 s; mem avail 24406 MB, swap used 2877 MB, swap in/out 0/0 pages (2 s)) load 3.17 io avg10=0.00 avg60=0.00 avg300=0.00
06:12:44 p3 chrome prev 2 end load 3.13 io avg10=0.00 avg60=0.00 avg300=0.00
06:12:46 p3 chrome prev 3 (quiet: load 3.13 io 0 after 2 s; mem avail 24328 MB, swap used 2877 MB, swap in/out 0/0 pages (2 s)) load 2.88 io avg10=0.00 avg60=0.00 avg300=0.00
06:14:38 p3 chrome prev 3 end load 2.89 io avg10=0.00 avg60=0.00 avg300=0.00
06:14:40 p3 chrome new 3 (quiet: load 2.89 io 0 after 2 s; mem avail 24236 MB, swap used 2877 MB, swap in/out 0/0 pages (2 s)) load 2.89 io avg10=0.00 avg60=0.00 avg300=0.00
06:16:30 p3 chrome new 3 end load 2.59 io avg10=0.00 avg60=0.00 avg300=0.00
06:16:32 p3 chrome new 4 (quiet: load 2.59 io 0 after 2 s; mem avail 24261 MB, swap used 2875 MB, swap in/out 0/0 pages (2 s)) load 2.59 io avg10=0.00 avg60=0.00 avg300=0.00
06:18:24 p3 chrome new 4 end load 2.33 io avg10=0.00 avg60=0.00 avg300=0.00
06:18:31 p3 chrome prev 4 (quiet: load 2.14 io 0 after 7 s; mem avail 24355 MB, swap used 2875 MB, swap in/out 0/0 pages (2 s)) load 1.97 io avg10=0.00 avg60=0.00 avg300=0.00
06:20:22 p3 chrome prev 4 end load 2.02 io avg10=0.00 avg60=0.00 avg300=0.00
06:20:24 p3 chrome prev 5 (quiet: load 2.02 io 0 after 2 s; mem avail 25318 MB, swap used 2875 MB, swap in/out 0/0 pages (2 s)) load 2.02 io avg10=0.00 avg60=0.00 avg300=0.00
06:22:15 p3 chrome prev 5 end load 1.44 io avg10=0.00 avg60=0.00 avg300=0.00
06:22:17 p3 chrome new 5 (quiet: load 1.44 io 0 after 2 s; mem avail 25340 MB, swap used 2875 MB, swap in/out 0/0 pages (2 s)) load 1.33 io avg10=0.00 avg60=0.00 avg300=0.00
06:24:07 p3 chrome new 5 end load 3.70 io avg10=0.00 avg60=0.00 avg300=0.00
06:24:09 p3 chrome new 6 (quiet: load 3.7 io 0 after 2 s; mem avail 25406 MB, swap used 2874 MB, swap in/out 0/0 pages (2 s)) load 3.70 io avg10=0.00 avg60=0.00 avg300=0.00
06:26:02 p3 chrome new 6 end load 4.00 io avg10=0.00 avg60=0.00 avg300=0.00
06:26:09 p3 chrome prev 6 (quiet: load 3.68 io 0 after 7 s; mem avail 24269 MB, swap used 2874 MB, swap in/out 0/0 pages (2 s)) load 3.68 io avg10=0.00 avg60=0.00 avg300=0.00
06:27:59 p3 chrome prev 6 end load 4.07 io avg10=0.00 avg60=0.00 avg300=0.00
06:27:59 pure end chrome (all 12 runs of the mode finished) load 4.07 io avg10=0.00 avg60=0.00 avg300=0.00
06:27:59 E14 pure chain end rc=0 load 4.07 io avg10=0.00
```
