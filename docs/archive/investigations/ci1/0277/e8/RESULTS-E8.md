# 0277: verification of `fl-engine-ci-875bfd62` (Worker 4a03e043-212d-4392-b35a-a1cbb60fa726, page of git c74b9194) against the previous production bundle `fl-engine-ci-d9c160aa`

Real engine in real browsers (Chrome / Firefox, headless; Firefox on Xvfb :77 with WebGL). No stand-in, no replay. The bundle comes from R2 (read only); every manifest sha256 is checked by `fetch`.

## Pass criteria

| criterion | value | pass |
|---|---|---|
| Firefox has WebGL (preflight) | preflight: webglRenderer llvmpipe, or similar desk.mode gl first idle 7533 | yes |
| No stop (11 scenes + more, both browsers, play, export; d7 is the intended exit) | stops: none; not ok: none; no result at all (the harness did not start): none | yes |
| Export WAV equals the reference | 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` (expected 22863828 B, `381cdb5acf891a16…`) | yes |
| First idle ≤ previous + 500 ms (chrome, WebGL runs) | all runs: new median 5156 ms, previous 5321 ms, difference -164 ms; quiet runs only (load ≤12, I/O avg10 <1; n new 6 / prev 5): new 5156, previous 5320, difference -164 ms | yes |
| First idle ≤ previous + 500 ms (firefox, WebGL runs) | all runs: new median 4486 ms, previous 4823 ms, difference -338 ms; quiet runs only (load ≤12, I/O avg10 <1; n new 5 / prev 5): new 4474, previous 4711, difference -237 ms | yes |
| File menu ×100 (chrome) | 100/100 | yes |
| File menu ×100 (firefox) | 100/100 | yes |

## Scenes (new bundle)

| browser | scene | result | first idle ms | File ×100 | load, I/O pressure (start→end) |
|---|---|---|---|---|---|
| chrome | d1-sampler-envelope | ok | 5952 | - | load 6.6→6.9, io 0.0→0.1 |
| chrome | d2-browser-folder | ok | 5240 | - | load 6.9→8.3, io 0.1→0.0 |
| chrome | d3-settings-all-tabs | ok | 4856 | - | load 8.3→7.4, io 0.0→0.0 |
| chrome | d4-playlist-middle-drag | ok | 4975 | - | load 7.4→6.9, io 0.0→0.0 |
| chrome | d5-mixer-front | ok | 4736 | - | load 6.9→6.2, io 0.0→0.2 |
| chrome | d6-playlist-close | ok | 4779 | - | load 6.2→5.9, io 0.2→0.0 |
| chrome | d7-main-close | ok (intended exit) | 6473 | - | load 5.9→7.1, io 0.0→0.0 |
| chrome | d8-file-menu-100 | ok | 4835 | 100/100 | load 7.1→11.0, io 0.0→0.0 |
| chrome | d9-keys | ok | 4883 | - | load 11.0→14.7, io 0.0→0.0 |
| chrome | d10-pianoroll-f12 | ok | 6426 | - | load 14.7→14.9, io 0.0→0.1 |
| chrome | d11-menu-hover-escape | ok | 6334 | - | load 15.0→15.0, io 0.0→0.0 |
| chrome | add-menu-simple-fade | ok | 5436 | - | load 16.4→12.9, io 0.0→0.0 |
| chrome | picker-cost | ok | 4997 | - | load 11.6→10.3, io 0.0→0.0 |
| chrome | picker-f8-fade | ok | 4702 | - | load 12.9→11.6, io 0.0→0.0 |
| chrome | plus-more-plugins | ok | 6758 | - | load 15.2→16.4, io 0.0→0.0 |
| firefox | d1-sampler-envelope | ok | 5283 | - | load 6.6→6.9, io 0.2→0.1 |
| firefox | d2-browser-folder | ok | 4883 | - | load 8.4→8.6, io 0.1→0.0 |
| firefox | d3-settings-all-tabs | ok | 4867 | - | load 9.4→6.8, io 0.0→0.0 |
| firefox | d4-playlist-middle-drag | ok | 4987 | - | load 6.9→6.5, io 0.0→0.0 |
| firefox | d5-mixer-front | ok | 4609 | - | load 2.7→2.4, io 0.0→0.0 |
| firefox | d6-playlist-close | ok | 5267 | - | load 6.2→5.9, io 0.2→0.0 |
| firefox | d7-main-close | ok (intended exit) | 5695 | - | load 7.0→7.1, io 0.0→0.0 |
| firefox | d8-file-menu-100 | ok | 5230 | 100/100 | load 7.2→14.3, io 0.0→0.0 |
| firefox | d9-keys | ok | 5471 | - | load 14.3→15.1, io 0.2→0.0 |
| firefox | d10-pianoroll-f12 | ok | 6532 | - | load 15.0→15.6, io 0.0→0.0 |
| firefox | d11-menu-hover-escape | ok | 4929 | - | load 15.6→12.2, io 0.0→0.0 |
| firefox | add-menu-simple-fade | ok | 4638 | - | load 11.2→9.2, io 0.0→0.0 |
| firefox | picker-cost | ok | 4657 | - | load 8.5→7.3, io 0.0→0.0 |
| firefox | picker-f8-fade | ok | 4944 | - | load 9.2→8.5, io 0.0→0.0 |
| firefox | plus-more-plugins | ok | 4690 | - | load 12.3→11.2, io 0.0→0.0 |


The scenes ran as two parallel jobs (Chrome and Firefox), so the first idle values of this table are reference values (the load is next to each).

## alpha7 (main window geometry, guest 1920x1080, Chrome)

result ok; load 9.2→8.9, io 0.0→0.0; windows visible at the end: 1; windows larger than 1800 px wide:
- `14 TFruityLoopsMainForm 'FL Studio 2026' @0,0 1920x1080 z0`
- expected after alpha7: the main window @0,0 1920x1080 (previous df0512b0: @-5,-6 1930x1092). Screenshots: `main-1920x1080.png, scenario-alpha7-look.png`

## First idle (single runs, cold, a new profile each; previous and new alternating)

| browser | bundle | run | first idle ms | load, I/O pressure at the start | at the end | quiet (load ≤12, I/O <1) | webglRenderer | desk.mode |
|---|---|---|---|---|---|---|---|---|
| chrome | prev | 1 | 5322 | 7.75 / io 1.04 | 7.63 / io 0.38 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 1 | 5352 | 7.63 / io 0.38 | 7.15 / io 0.14 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 2 | 5075 | 7.15 / io 0.14 | 8.18 / io 0.05 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 2 | 5290 | 8.18 / io 0.05 | 7.61 / io 0.01 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 3 | 5320 | 7.61 / io 0.01 | 7.13 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 3 | 5238 | 7.13 / io 0.00 | 7.23 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 4 | 5299 | 7.23 / io 0.00 | 6.73 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 4 | 5577 | 6.73 / io 0.00 | 7.82 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 5 | 6006 | 7.82 / io 0.00 | 8.72 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 5 | 4991 | 8.72 / io 0.00 | 7.83 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 6 | 4955 | 7.83 / io 0.00 | 7.30 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 6 | 4984 | 7.30 / io 0.00 | 7.04 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| firefox | prev | 1 | 4711 | 7.04 / io 0.00 | 5.99 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 1 | 4411 | 5.99 / io 0.00 | 5.45 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 2 | 4417 | 5.45 / io 0.00 | 5.00 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 2 | 4665 | 5.00 / io 0.00 | 5.56 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 3 | 5097 | 5.56 / io 0.00 | 5.81 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 3 | 5002 | 5.81 / io 0.00 | 7.55 / io 0.08 | yes | llvmpipe, or similar | gl |
| firefox | new | 4 | 5288 | 7.55 / io 0.08 | 12.06 / io 0.02 | no (reference) | llvmpipe, or similar | gl |
| firefox | prev | 4 | 4935 | 12.06 / io 0.02 | 10.58 / io 0.00 | no (reference) | llvmpipe, or similar | gl |
| firefox | prev | 5 | 5098 | 10.58 / io 0.00 | 9.56 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 5 | 4474 | 9.56 / io 0.00 | 8.55 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 6 | 4497 | 8.55 / io 0.00 | 7.61 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 6 | 4623 | 7.61 / io 0.00 | 7.56 / io 0.08 | yes | llvmpipe, or similar | gl |

| browser | mode | previous median (n) | new median (n) | difference |
|---|---|---|---|---|
| chrome | gl | 5321 (6) | 5156 (6) | -164 |
| firefox | gl | 4823 (6) | 4486 (6) | -338 |

## Export (Chrome, the reference project, Ctrl+R)

result ok; load 4.8→8.8, io 0.0→0.0; seconds 371
- WAV `serina1-serum2.wav`: 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`; expected 22863828 B / `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`: MATCH

## Playback (the reference project zip, ~60 s)

not run

### Playback summary (every run; same page, same procedure)

Counter definition: `window.__app.audio.detail()` of the page: `underrunQuanta` = AudioWorklet render quanta (128 frames ≈ 2.9 ms) in which the output ring was empty, counted cumulatively from when the audio device began to play (page start), `episodes` = maximal runs of consecutive underrun quanta (`app/src/audio/underrun.ts`, `ring-layout.ts`). This is the same counter as `underrunQuanta` in opt1's audio-b.json `detail`. **cumulative** = the value at the end (includes the stall while the project is opened); **during playback** = the difference between the reading at the end of `playing-a` (≈19 s after the first reading, 4 s after Space + 10 s playing) and the reading after the stop (≈52 s of playback).

| run | bundle | load at start → end | Worker busy ms/s while playing (min / median / max) | cumulative at the end: quanta / episodes | before the playback (project-open stall) | **during playback: quanta / episodes** | gap frames the engine wrote itself during playback (`engineGapFrames` delta; the engine wrote silence for a hole) |
|---|---|---|---|---|---|---|---|

## Mixer header drag: Worker CPU per move (previous and new alternating)

### P3 decider, chrome: old (previous) vs new, windows of 10 s from the play mark; counted = start load ≤8 and I/O pressure avg10 = 0

| run | bundle | counted | load start → end | I/O avg10 start → end | Worker CPU s (60 s) | main CPU s | underrun quanta 0-60 s | episodes 0-60 s | engine gap frames 0-60 s | **underrun 30-60 s** | write rate mean 10-60 s | consumed rate mean 10-60 s | UI idles/s (10-60 s) | presents/s | lagCuts/s | virtual s per real s | stopped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | prev | NO (not counted) | 8.6 → 5.5 | - → - | 42.7 | 0.0 | 1105 | 99 | 132877 | **904** | 0.405 | 0.999 | - | - | - | 1.013 | False |
| 2 | prev | yes | 5.5 → 5.5 | 0.00 → 0.00 | 44.7 | 0.0 | 960 | 100 | 114215 | **809** | 0.405 | 0.998 | - | - | - | 1.007 | False |
| 3 | prev | yes | 5.3 → 5.2 | 0.00 → 0.00 | 45.6 | 0.0 | 766 | 85 | 90708 | **656** | 0.398 | 1.000 | - | - | - | 0.997 | False |
| 4 | prev | yes | 6.8 → 5.4 | 0.00 → 0.00 | 47.3 | 0.0 | 657 | 69 | 78079 | **591** | 0.392 | 0.993 | - | - | - | 1.001 | False |
| 5 | prev | yes | 5.6 → 5.4 | 0.00 → 0.00 | 45.9 | 0.0 | 785 | 85 | 92735 | **687** | 0.403 | 0.999 | - | - | - | 1.007 | False |
| 6 | prev | yes | 5.8 → 3.1 | 0.00 → 0.00 | 44.9 | 0.0 | 966 | 88 | 116072 | **810** | 0.401 | 0.994 | - | - | - | 1.006 | False |
| 1 | new | yes | 6.2 → 7.7 | - → - | 46.7 | 0.0 | 605 | 66 | 71709 | **571** | 0.398 | 0.998 | - | - | - | 1.002 | False |
| 2 | new | NO (not counted) | 7.7 → 5.5 | 0.04 → 0.00 | 44.0 | 0.0 | 927 | 90 | 110901 | **787** | 0.405 | 0.992 | - | - | - | 1.007 | False |
| 3 | new | yes | 5.2 → 10.1 | 0.00 → 0.00 | 50.1 | 0.0 | 158 | 35 | 17178 | **115** | 0.386 | 0.997 | - | - | - | 0.990 | False |
| 4 | new | NO (not counted) | 10.8 → 6.8 | 0.00 → 0.00 | 45.3 | 0.0 | 1003 | 95 | 120081 | **849** | 0.403 | 0.993 | - | - | - | 1.011 | False |
| 5 | new | yes | 5.2 → 4.9 | 0.00 → 0.00 | 45.0 | 0.0 | 845 | 89 | 100438 | **736** | 0.396 | 1.000 | - | - | - | 1.004 | False |
| 6 | new | yes | 4.9 → 6.0 | 0.00 → 0.00 | 46.0 | 0.0 | 870 | 89 | 103332 | **735** | 0.401 | 0.997 | - | - | - | 1.006 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 5 | 10 | 3 | 1015 | 0.72 | 1.00 | - | - | - | 1.00 |
| 0-10 s | new | 4 | 13 | 3 | 1436 | 0.72 | 1.00 | - | - | - | 1.00 |
| 10-20 s | prev | 5 | 0 | 0 | 7 | 0.48 | 1.00 | - | - | - | 1.00 |
| 10-20 s | new | 4 | 0 | 0 | 8 | 0.48 | 1.00 | - | - | - | 1.00 |
| 20-30 s | prev | 5 | 99 | 15 | 10850 | 0.41 | 1.00 | - | - | - | 1.00 |
| 20-30 s | new | 4 | 65 | 9 | 6692 | 0.41 | 1.00 | - | - | - | 1.00 |
| 30-40 s | prev | 5 | 244 | 29 | 28232 | 0.33 | 1.00 | - | - | - | 1.00 |
| 30-40 s | new | 4 | 185 | 23 | 20370 | 0.33 | 1.00 | - | - | - | 1.00 |
| 40-50 s | prev | 5 | 426 | 33 | 50302 | 0.33 | 1.00 | - | - | - | 1.02 |
| 40-50 s | new | 4 | 419 | 33 | 49120 | 0.33 | 1.00 | - | - | - | 1.01 |
| 50-60 s | prev | 5 | 35 | 4 | 5904 | 0.45 | 0.99 | - | - | - | 0.99 |
| 50-60 s | new | 4 | 31 | 7 | 7641 | 0.44 | 0.99 | - | - | - | 0.99 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 784.838 (5) | 724.784 (4) | -60.054 | 0.238 | 872.259 (6) | 857.463 (6) | -14.796 | 0.407 |
| underrun quanta 30-60 s (heavy interval) | 687.494 (5) | 653.023 (4) | -34.471 | 0.262 | 748.020 (6) | 735.682 (6) | -12.337 | 0.468 |
| episodes 0-60 s | 84.959 (5) | 77.421 (4) | -7.538 | 0.278 | 86.467 (6) | 88.910 (6) | +2.443 | 0.381 |
| engine gap frames 0-60 s | 92734.646 (5) | 86073.067 (4) | -6661.579 | 0.238 | 103474.769 (6) | 101884.855 (6) | -1589.914 | 0.407 |
| write rate mean 10-60 s | 0.401 (5) | 0.397 (4) | -0.005 | 0.325 | 0.402 (6) | 0.399 (6) | -0.003 | 0.558 |
| Worker CPU s (60 s) | 45.560 (5) | 46.340 (4) | +0.780 | 0.294 | 45.245 (6) | 45.635 (6) | +0.390 | 0.396 |
| UI idles/s (10-60 s) | - | - | - | - | - | - | - | - |
| presents/s (10-60 s) | - | - | - | - | - | - | - | - |
| lagCuts/s (10-60 s) | - | - | - | - | - | - | - | - |
| virtual s per real s (10-60 s) | 1.006 (5) | 1.003 (4) | -0.003 | 0.500 | 1.007 (6) | 1.005 (6) | -0.002 | 0.671 |



### P3 decider, firefox: old (previous) vs new, windows of 10 s from the play mark; counted = start load ≤8 and I/O pressure avg10 = 0

| run | bundle | counted | load start → end | I/O avg10 start → end | Worker CPU s (60 s) | main CPU s | underrun quanta 0-60 s | episodes 0-60 s | engine gap frames 0-60 s | **underrun 30-60 s** | write rate mean 10-60 s | consumed rate mean 10-60 s | UI idles/s (10-60 s) | presents/s | lagCuts/s | virtual s per real s | stopped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | prev | yes | 2.4 → 4.1 | 0.00 → 0.00 | 49.0 | 0.0 | 443 | 60 | 51520 | **399** | 0.394 | 0.996 | - | - | - | 1.000 | False |
| 2 | prev | yes | 6.7 → 12.3 | 0.00 → 0.01 | 56.8 | 0.0 | 8 | 2 | 837 | **8** | 0.312 | 0.996 | - | - | - | 0.892 | False |
| 3 | prev | NO (not counted) | 13.3 → 11.9 | 0.18 → 0.00 | 49.0 | 0.0 | 451 | 64 | 52144 | **436** | 0.381 | 1.000 | - | - | - | 1.000 | False |
| 1 | new | yes | 4.0 → 4.8 | 0.00 → 0.00 | 49.6 | 0.0 | 425 | 62 | 49055 | **379** | 0.388 | 1.000 | - | - | - | 1.000 | False |
| 2 | new | yes | 4.8 → 5.7 | 0.00 → 0.00 | 50.7 | 0.0 | 299 | 45 | 34270 | **289** | 0.385 | 0.999 | - | - | - | 1.000 | False |
| 3 | new | NO (not counted) | 12.1 → 13.8 | 0.00 → 0.00 | 53.8 | 0.0 | 64 | 19 | 10197 | **64** | 0.359 | 0.997 | - | - | - | 0.997 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 2 | 0 | 0 | 0 | 0.69 | 1.00 | - | - | - | 1.00 |
| 0-10 s | new | 2 | 0 | 0 | 0 | 0.72 | 1.00 | - | - | - | 1.00 |
| 10-20 s | prev | 2 | 0 | 0 | 0 | 0.43 | 1.00 | - | - | - | 1.00 |
| 10-20 s | new | 2 | 0 | 0 | 0 | 0.44 | 1.00 | - | - | - | 1.00 |
| 20-30 s | prev | 2 | 22 | 3 | 2490 | 0.34 | 1.00 | - | - | - | 0.84 |
| 20-30 s | new | 2 | 28 | 5 | 1626 | 0.42 | 1.00 | - | - | - | 1.00 |
| 30-40 s | prev | 2 | 76 | 12 | 8752 | 0.29 | 1.00 | - | - | - | 0.86 |
| 30-40 s | new | 2 | 114 | 18 | 13619 | 0.33 | 1.00 | - | - | - | 1.00 |
| 40-50 s | prev | 2 | 114 | 13 | 13485 | 0.33 | 1.00 | - | - | - | 1.01 |
| 40-50 s | new | 2 | 196 | 26 | 22528 | 0.33 | 1.00 | - | - | - | 1.00 |
| 50-60 s | prev | 2 | 13 | 2 | 1451 | 0.37 | 0.98 | - | - | - | 1.03 |
| 50-60 s | new | 2 | 25 | 4 | 3889 | 0.41 | 1.00 | - | - | - | 0.99 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 225.328 (2) | 362.185 (2) | +136.858 | 1.000 | 442.798 (3) | 299.472 (3) | -143.326 | 0.800 |
| underrun quanta 30-60 s (heavy interval) | 203.542 (2) | 333.946 (2) | +130.403 | 1.000 | 399.227 (3) | 288.583 (3) | -110.644 | 0.700 |
| episodes 0-60 s | 30.969 (2) | 53.445 (2) | +22.476 | 0.667 | 59.973 (3) | 44.914 (3) | -15.059 | 1.000 |
| engine gap frames 0-60 s | 26178.645 (2) | 41662.521 (2) | +15483.876 | 1.000 | 51520.437 (3) | 34269.953 (3) | -17250.484 | 0.800 |
| write rate mean 10-60 s | 0.353 (2) | 0.386 (2) | +0.033 | 1.000 | 0.381 (3) | 0.385 (3) | +0.004 | 0.800 |
| Worker CPU s (60 s) | 52.865 (2) | 50.160 (2) | -2.705 | 1.000 | 49.030 (3) | 50.690 (3) | +1.660 | 1.000 |
| UI idles/s (10-60 s) | - | - | - | - | - | - | - | - |
| presents/s (10-60 s) | - | - | - | - | - | - | - | - |
| lagCuts/s (10-60 s) | - | - | - | - | - | - | - | - |
| virtual s per real s (10-60 s) | 0.946 (2) | 1.000 (2) | +0.054 | 0.667 | 1.000 (3) | 1.000 (3) | -0.000 | 0.800 |



## E8: minimize / restore through the page button (Chrome; FL own minimize button clicked with the mouse)

**min-restore-click**: ok True, stopped False; load 3.3→2.9, io 0.0→0.0
- tb-0-start: `{"hidden":true,"display":"none","text":"最小化された窓を元に戻す","title":"Restore the minimized window","rect":[0,0,0,0],"desktopRect":[0,36,1920,1080],"active":"desktop","count":1}`
- tb-1-after-minimize: `{"hidden":false,"display":"inline-block","text":"最小化された窓を元に戻す","title":"Restore the minimized window","rect":[12,1071,182,33],"desktopRect":[0,36,1920,1080],"active":"desktop","count":1}`
- press-restore: `clicked`
- tb-3-after-restore: `{"hidden":true,"display":"none","text":"最小化された窓を元に戻す","title":"Restore the minimized window","rect":[0,0,0,0],"desktopRect":[0,36,1920,1080],"active":"desktop","count":1}`
- windows 0-start: ["14 TFruityLoopsMainForm 'FL Studio 2026' @0,0 1920x1080 z0"]
- windows 1-after-minimize: none visible
- windows 3-after-restore: ["14 TFruityLoopsMainForm 'FL Studio 2026' @0,0 1920x1080 z0"]

**min-restore-keyboard**: ok True, stopped False; load 2.7→4.3, io 0.0→0.0
- tb-0-start: `{"hidden":true,"display":"none","text":"最小化された窓を元に戻す","title":"Restore the minimized window","rect":[0,0,0,0],"desktopRect":[0,36,1920,1080],"active":"desktop","count":1}`
- tb-1-after-minimize: `{"hidden":false,"display":"inline-block","text":"最小化された窓を元に戻す","title":"Restore the minimized window","rect":[12,1071,182,33],"desktopRect":[0,36,1920,1080],"active":"desktop","count":1}`
- tb-2-after-tab: `{"hidden":false,"display":"inline-block","text":"最小化された窓を元に戻す","title":"Restore the minimized window","rect":[12,1071,182,33],"desktopRect":[0,36,1920,1080],"active":"taskbar-restore","count":1}`
- tb-3-after-restore: `{"hidden":true,"display":"none","text":"最小化された窓を元に戻す","title":"Restore the minimized window","rect":[0,0,0,0],"desktopRect":[0,36,1920,1080],"active":"desktop","count":1}`
- windows 0-start: ["14 TFruityLoopsMainForm 'FL Studio 2026' @0,0 1920x1080 z0"]
- windows 1-after-minimize: none visible
- windows 3-after-restore: ["14 TFruityLoopsMainForm 'FL Studio 2026' @0,0 1920x1080 z0"]

## Steps (load and I/O pressure at each step)

```
11:37:48 fetched fl-engine-ci-d9c160aa (260M) load 8.70 io avg10=0.00 avg60=0.10 avg300=0.13
11:37:51 fetched fl-engine-ci-875bfd62 (260M) load 8.70 io avg10=0.00 avg60=0.09 avg300=0.13
11:37:57 built the page of c74b9194 load 8.60 io avg10=0.00 avg60=0.08 avg300=0.13
11:38:24 preflight ok load 8.42 io avg10=2.83 avg60=1.07 avg300=0.36
11:40:37 idle chrome x6 load 7.04 io avg10=0.00 avg60=0.10 avg300=0.21
11:42:55 idle firefox x6 load 7.56 io avg10=0.08 avg60=0.04 avg300=0.13
11:43:02 scenes chrome scn-client.json  load 6.63 io avg10=0.04 avg60=0.03 avg300=0.13
11:43:02 scenes firefox scn-client.json d1-sampler-envelope load 6.63 io avg10=0.04 avg60=0.03 avg300=0.13
11:43:37 scenes firefox scn-client.json d2-browser-folder load 8.36 io avg10=0.07 avg60=0.12 avg300=0.14
11:44:00 scenes firefox scn-client.json d3-settings-all-tabs load 8.55 io avg10=0.00 avg60=0.08 avg300=0.13
11:44:47 scenes firefox scn-client.json d4-playlist-middle-drag load 6.87 io avg10=0.00 avg60=0.03 avg300=0.10
11:45:10 scenes firefox scn-client.json d5-mixer-front load 6.48 io avg10=0.00 avg60=0.02 avg300=0.09
11:45:11 scenes firefox scn-client.json d6-playlist-close load 6.48 io avg10=0.00 avg60=0.02 avg300=0.09
11:45:31 scenes firefox scn-client.json d7-main-close load 5.93 io avg10=0.02 avg60=0.03 avg300=0.09
11:45:57 scenes firefox scn-client.json d8-file-menu-100 load 7.15 io avg10=0.02 avg60=0.04 avg300=0.08
11:48:53 scenes firefox scn-client.json d9-keys load 14.28 io avg10=0.00 avg60=0.00 avg300=0.04
11:49:42 scenes firefox scn-client.json d10-pianoroll-f12 load 15.07 io avg10=0.00 avg60=0.02 avg300=0.04
11:49:47 scenes chrome scn-plus.json  load 14.99 io avg10=0.00 avg60=0.02 avg300=0.04
11:50:14 scenes chrome scn-alpha1.json  load 16.39 io avg10=0.04 avg60=0.06 avg300=0.05
11:50:20 scenes firefox scn-client.json d11-menu-hover-escape load 15.56 io avg10=0.02 avg60=0.05 avg300=0.05
11:50:51 scenes firefox scn-plus.json  load 12.20 io avg10=0.01 avg60=0.04 avg300=0.05
11:51:18 scenes firefox scn-alpha1.json  load 11.16 io avg10=0.00 avg60=0.02 avg300=0.04
11:51:43 alpha7 load 9.23 io avg10=0.00 avg60=0.01 avg300=0.03
11:53:37 minrestore load 4.10 io avg10=0.00 avg60=0.00 avg300=0.01
11:54:01 minrestore end load 3.96 io avg10=0.00 avg60=0.00 avg300=0.00
11:54:19 minrestore load 3.34 io avg10=0.00 avg60=0.00 avg300=0.00
11:55:06 minrestore end load 4.34 io avg10=0.00 avg60=0.00 avg300=0.00
11:55:43 export start load 4.77 io avg10=0.00 avg60=0.00 avg300=0.00
12:01:56 export end load 8.77 io avg10=0.00 avg60=0.01 avg300=0.00
12:02:06 p3 chrome prev 1 load 8.56 io avg10=0.00 avg60=0.01 avg300=0.00
12:03:58 p3 chrome prev 1 end load 6.24 io avg10=0.00 avg60=0.00 avg300=0.00
12:03:58 p3 chrome new 1 load 6.24 io avg10=0.00 avg60=0.00 avg300=0.00
12:05:50 p3 chrome new 1 end load 7.66 io avg10=0.04 avg60=0.02 avg300=0.00
12:05:50 p3 chrome new 2 load 7.66 io avg10=0.04 avg60=0.02 avg300=0.00
12:07:42 p3 chrome new 2 end load 5.55 io avg10=0.00 avg60=0.00 avg300=0.00
12:07:42 p3 chrome prev 2 load 5.55 io avg10=0.00 avg60=0.00 avg300=0.00
12:09:34 p3 chrome prev 2 end load 5.34 io avg10=0.00 avg60=0.01 avg300=0.00
12:09:34 p3 chrome prev 3 load 5.34 io avg10=0.00 avg60=0.01 avg300=0.00
12:11:25 p3 chrome prev 3 end load 5.19 io avg10=0.00 avg60=0.00 avg300=0.00
12:11:25 p3 chrome new 3 load 5.19 io avg10=0.00 avg60=0.00 avg300=0.00
12:13:22 p3 chrome new 3 end load 10.12 io avg10=0.00 avg60=0.00 avg300=0.00
12:13:22 p3 chrome new 4 load 10.12 io avg10=0.00 avg60=0.00 avg300=0.00
12:15:16 p3 chrome new 4 end load 6.85 io avg10=0.00 avg60=0.00 avg300=0.00
12:15:16 p3 chrome prev 4 load 6.85 io avg10=0.00 avg60=0.00 avg300=0.00
12:17:09 p3 chrome prev 4 end load 5.59 io avg10=0.00 avg60=0.01 avg300=0.00
12:17:09 p3 chrome prev 5 load 5.59 io avg10=0.00 avg60=0.01 avg300=0.00
12:19:03 p3 chrome prev 5 end load 5.15 io avg10=0.00 avg60=0.00 avg300=0.00
12:19:03 p3 chrome new 5 load 5.15 io avg10=0.00 avg60=0.00 avg300=0.00
12:20:55 p3 chrome new 5 end load 4.93 io avg10=0.00 avg60=0.00 avg300=0.00
12:20:55 p3 chrome new 6 load 4.93 io avg10=0.00 avg60=0.00 avg300=0.00
12:22:47 p3 chrome new 6 end load 5.77 io avg10=0.00 avg60=0.00 avg300=0.00
12:22:47 p3 chrome prev 6 load 5.77 io avg10=0.00 avg60=0.00 avg300=0.00
12:24:39 p3 chrome prev 6 end load 3.10 io avg10=0.00 avg60=0.10 avg300=0.05
12:24:51 scenes firefox scn-client.json d5-mixer-front load 2.70 io avg10=0.00 avg60=0.08 avg300=0.05
12:25:24 p3 firefox prev 1 load 2.35 io avg10=0.00 avg60=0.06 avg300=0.05
12:27:22 p3 firefox prev 1 end load 4.06 io avg10=0.00 avg60=0.00 avg300=0.02
12:27:22 p3 firefox new 1 load 4.06 io avg10=0.00 avg60=0.00 avg300=0.02
12:29:20 p3 firefox new 1 end load 4.84 io avg10=0.00 avg60=0.00 avg300=0.00
12:29:20 p3 firefox new 2 load 4.84 io avg10=0.00 avg60=0.00 avg300=0.00
12:31:18 p3 firefox new 2 end load 6.74 io avg10=0.00 avg60=0.00 avg300=0.00
12:31:18 p3 firefox prev 2 load 6.74 io avg10=0.00 avg60=0.00 avg300=0.00
12:33:22 p3 firefox prev 2 end load 12.27 io avg10=0.01 avg60=0.03 avg300=0.00
12:33:22 p3 firefox prev 3 load 12.27 io avg10=0.01 avg60=0.03 avg300=0.00
12:35:31 p3 firefox prev 3 end load 11.88 io avg10=0.00 avg60=0.04 avg300=0.01
12:35:31 p3 firefox new 3 load 11.88 io avg10=0.00 avg60=0.04 avg300=0.01
12:37:35 p3 firefox new 3 end load 13.84 io avg10=0.00 avg60=0.00 avg300=0.00
12:38:01 title load 13.31 io avg10=0.00 avg60=0.00 avg300=0.00
12:40:15 title end load 15.18 io avg10=0.00 avg60=0.01 avg300=0.00
```
