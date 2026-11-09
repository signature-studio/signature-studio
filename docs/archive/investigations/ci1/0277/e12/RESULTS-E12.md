# 0277: verification of `fl-engine-ci-7a9b0800` (Worker e9565033-8c44-4395-a6a4-53abee97001d, page of git a977401d) against the previous production bundle `fl-engine-ci-dc5fae2b`

Real engine in real browsers (Chrome / Firefox, headless; Firefox on Xvfb :77 with WebGL). No stand-in, no replay. The bundle comes from R2 (read only); every manifest sha256 is checked by `fetch`.

## Pass criteria

| criterion | value | pass |
|---|---|---|
| Firefox has WebGL (preflight) | preflight: webglRenderer llvmpipe, or similar desk.mode gl first idle 4313 | yes |
| No stop (11 scenes + more, both browsers, play, export; d7 is the intended exit) | stops: none; not ok: none; no result at all (the harness did not start): none | yes |
| Export WAV equals the reference | 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` (expected 22863828 B, `381cdb5acf891a16…`) | yes |
| First idle ≤ previous + 500 ms (chrome, WebGL runs) | all runs: new median 4695 ms, previous 4640 ms, difference +54 ms; means 5007 ± 234 (SE, n=16) / 4650 ± 31 (n=16), difference of the means +357 ± 236 (SE); quiet runs only (load ≤12, I/O avg10 <1; n new 16 / prev 16): new 4695, previous 4640, difference +54 ms | yes |
| First idle ≤ previous + 500 ms (firefox, WebGL runs) | not run | not run |
| File menu ×100 (chrome) | 100/100 | yes |
| File menu ×100 (firefox) | 100/100 | yes |

## Scenes (new bundle)

| browser | scene | result | first idle ms | File ×100 | load, I/O pressure (start→end) |
|---|---|---|---|---|---|
| chrome | d1-sampler-envelope | ok | 4761 | - | load 5.7→10.7, io 0.0→0.0 |
| chrome | d2-browser-folder | ok | 4152 | - | load 10.7→10.0, io 0.0→0.0 |
| chrome | d3-settings-all-tabs | ok | 5949 | - | load 10.1→12.2, io 0.0→0.0 |
| chrome | d4-playlist-middle-drag | ok | 4560 | - | load 12.2→12.3, io 0.0→0.0 |
| chrome | d5-mixer-front | ok | 4802 | - | load 12.3→12.2, io 0.0→0.0 |
| chrome | d6-playlist-close | ok | 5040 | - | load 12.2→11.4, io 0.0→0.1 |
| chrome | d7-main-close | ok (intended exit) | 4961 | - | load 11.4→11.8, io 0.1→0.0 |
| chrome | d8-file-menu-100 | ok | 4557 | 100/100 | load 11.9→13.6, io 0.0→0.0 |
| chrome | d9-keys | ok | 4080 | - | load 13.6→12.7, io 0.0→0.0 |
| chrome | d10-pianoroll-f12 | ok | 4338 | - | load 12.7→13.7, io 0.0→0.0 |
| chrome | d11-menu-hover-escape | ok | 4650 | - | load 14.4→11.8, io 0.0→0.0 |
| chrome | add-menu-simple-fade | ok | 5121 | - | load 10.4→10.5, io 0.0→0.0 |
| chrome | picker-cost | ok | 4073 | - | load 9.0→7.9, io 0.0→0.0 |
| chrome | picker-f8-fade | ok | 4341 | - | load 10.5→9.0, io 0.0→0.0 |
| chrome | plus-more-plugins | ok | 5151 | - | load 11.8→10.4, io 0.0→0.0 |
| firefox | d1-sampler-envelope | ok | 4856 | - | load 6.1→10.3, io 0.3→0.0 |
| firefox | d2-browser-folder | ok | 5172 | - | load 10.3→11.4, io 0.0→0.0 |
| firefox | d3-settings-all-tabs | ok | 4962 | - | load 13.0→12.3, io 0.0→0.0 |
| firefox | d4-playlist-middle-drag | ok | 4483 | - | load 12.3→12.2, io 0.0→0.0 |
| firefox | d5-mixer-front | ok | 4680 | - | load 12.2→12.0, io 0.0→0.1 |
| firefox | d6-playlist-close | ok | 4115 | - | load 12.0→11.4, io 0.0→0.0 |
| firefox | d7-main-close | ok (intended exit) | 4664 | - | load 12.4→13.1, io 0.0→0.0 |
| firefox | d8-file-menu-100 | ok | 4863 | 100/100 | load 13.1→13.7, io 0.0→0.0 |
| firefox | d9-keys | ok | 4660 | - | load 13.7→11.0, io 0.0→0.0 |
| firefox | d10-pianoroll-f12 | ok | 4417 | - | load 10.6→10.1, io 0.0→0.0 |
| firefox | d11-menu-hover-escape | ok | 4154 | - | load 10.1→8.9, io 0.0→0.0 |
| firefox | add-menu-simple-fade | ok | 3967 | - | load 9.4→8.0, io 0.0→0.0 |
| firefox | picker-cost | ok | 5649 | - | load 8.7→11.4, io 0.0→0.0 |
| firefox | picker-f8-fade | ok | 4045 | - | load 8.0→8.7, io 0.0→0.0 |
| firefox | plus-more-plugins | ok | 4175 | - | load 8.9→9.3, io 0.2→0.0 |


The scenes ran as two parallel jobs (Chrome and Firefox), so the first idle values of this table are reference values (the load is next to each).

## alpha7 (main window geometry, guest 1920x1080, Chrome)

result ok; load 8.5→8.3, io 0.0→0.0; windows visible at the end: 1; windows larger than 1800 px wide:
- `14 TFruityLoopsMainForm 'FL Studio 2026' @0,0 1920x1080 z0`
- expected after alpha7: the main window @0,0 1920x1080 (previous df0512b0: @-5,-6 1930x1092). Screenshots: `main-1920x1080.png, scenario-alpha7-look.png`

## First idle (single runs, cold, a new profile each; previous and new alternating)

| browser | bundle | run | first idle ms | load, I/O pressure at the start | at the end | other Chrome browsers during the run (max) | quiet (load ≤8, I/O <1) | webglRenderer | desk.mode |
|---|---|---|---|---|---|---|---|---|---|
| chrome | prev | 11 | 4386 | 3.34 / io 0.00 | 3.13 / io 0.00 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 11 | 4569 | 3.13 / io 0.00 | 2.80 / io 0.00 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 12 | 4673 | 2.80 / io 0.00 | 2.97 / io 0.00 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 12 | 4547 | 2.97 / io 0.00 | 2.82 / io 0.00 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 13 | 4554 | 2.82 / io 0.00 | 3.05 / io 0.00 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 13 | 4463 | 3.05 / io 0.00 | 2.80 / io 0.00 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 14 | 4559 | 2.80 / io 0.00 | 2.59 / io 0.00 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 14 | 4514 | 2.59 / io 0.00 | 2.55 / io 0.00 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 15 | 4628 | 2.42 / io 0.00 | 2.55 / io 0.00 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 15 | 4552 | 2.55 / io 0.00 | 2.68 / io 0.00 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 16 | 4584 | 2.68 / io 0.00 | 2.87 / io 0.00 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 16 | 4645 | 2.87 / io 0.00 | 2.97 / io 0.00 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 1 | 4783 | 5.06 / io 0.00 | 4.81 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 1 | 4821 | 4.81 / io 0.00 | 4.67 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 2 | 4778 | 4.67 / io 0.00 | 5.31 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 2 | 4781 | 5.31 / io 0.00 | 5.72 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 3 | 4580 | 5.72 / io 0.00 | 5.38 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 3 | 4775 | 5.38 / io 0.00 | 5.31 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 4 | 4608 | 5.31 / io 0.00 | 5.48 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 4 | 4538 | 5.48 / io 0.00 | 5.48 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 5 | 4801 | 5.48 / io 0.00 | 5.65 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 5 | 7098 | 5.65 / io 0.00 | 5.61 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 6 | 7655 | 5.61 / io 0.00 | 5.37 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 6 | 4636 | 5.37 / io 0.00 | 6.79 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 7 | 4729 | 6.79 / io 0.00 | 6.51 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 7 | 4667 | 6.51 / io 0.00 | 6.35 / io 0.00 | 0 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 8 | 4717 | 6.35 / io 0.00 | 7.00 / io 0.18 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 8 | 4750 | 6.75 / io 0.00 | 6.56 / io 0.00 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 9 | 4732 | 6.56 / io 0.00 | 6.32 / io 0.00 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 9 | 4798 | 6.32 / io 0.00 | 6.10 / io 0.00 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 10 | 4800 | 6.10 / io 0.00 | 6.02 / io 0.00 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 10 | 4800 | 6.02 / io 0.00 | 5.78 / io 0.00 | 1 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |

| browser | mode | previous median (n) | new median (n) | difference |
|---|---|---|---|---|
| chrome | gl | 4640 (16) | 4695 (16) | 54 |

## Export (Chrome, the reference project, Ctrl+R)

result ok; load 8.2→8.7, io 0.0→0.0; seconds 314
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
| 1 | prev | yes | 0 / 0 / 0 | 0 | 4.3 → 3.3 | 0.00 → 0.00 | 49.7 | 0.0 | 0 | 0 | 0 | **0** | 0.996 | 0.996 | 407.2 | 18.0 | 16.5 | 0.791 | False |
| 2 | prev | yes | 0 / 0 / 0 | 65 | 3.7 → 4.6 | 0.00 → 0.00 | 50.1 | 0.0 | 8 | 4 | 680 | **4** | 0.973 | 0.996 | 316.9 | 16.0 | 17.4 | 0.731 | False |
| 3 | prev | reference (PM verification work overlapped, I/O pressure avg10 3.0 at the end) | 0 / 0 / 0 | 0 | 4.6 → 6.3 | 0.00 → 3.00 | 50.4 | 0.0 | 12 | 2 | 1364 | **0** | 0.938 | 0.998 | 258.8 | 14.2 | 18.2 | 0.678 | False |
| 4 | prev | yes | 0 / 0 / 0 | 0 | 7.6 → 7.6 | 0.00 → 0.00 | 50.0 | 0.0 | 29 | 3 | 3454 | **0** | 0.968 | 0.997 | 326.0 | 15.8 | 17.5 | 0.732 | False |
| 5 | prev | reference (opt1 wasm-opt link overlapped, load 13.0 at the end) | 0 / 0 / 0 | 0 | 7.7 → 12.0 | 0.00 → 0.00 | 49.1 | 0.0 | 46 | 12 | 4836 | **34** | 0.890 | 1.000 | 330.2 | 15.0 | 17.0 | 0.677 | False |
| 6 | prev | yes | 0 / 0 / 0 | 0 | 6.9 → 6.0 | 0.00 → 0.00 | 50.1 | 0.0 | 6 | 3 | 510 | **0** | 0.981 | 0.998 | 337.4 | 16.6 | 18.7 | 0.745 | False |
| 7 | prev | reference (alpha1 node Export + mt3 clang x2 + gpu1 smoke Chrome overlapped (about 1 minute, PM 20:26)) | 1 / 1 / 1 | 0 | 6.0 → 8.4 | 0.00 → 0.00 | 49.9 | 0.0 | 4 | 2 | 340 | **0** | 0.957 | 0.996 | 354.9 | 16.0 | 17.1 | 0.737 | False |
| 8 | prev | yes | 0 / 0 / 0 | 0 | 1.7 → 1.1 | 0.00 → 0.00 | 49.4 | 0.0 | 0 | 0 | 0 | **0** | 0.983 | 0.999 | 431.2 | 18.7 | 17.1 | 0.801 | False |
| 1 | new | yes | 0 / 0 / 0 | 0 | 3.3 → 6.2 | 0.00 → 0.11 | 50.8 | 0.0 | 71 | 8 | 8400 | **0** | 0.884 | 1.000 | 325.8 | 14.5 | 18.4 | 0.661 | False |
| 2 | new | yes | 0 / 0 / 0 | 140 | 5.4 → 4.7 | 0.00 → 0.17 | 49.9 | 0.0 | 6 | 1 | 682 | **0** | 0.966 | 1.000 | 339.2 | 16.8 | 18.9 | 0.744 | False |
| 3 | new | yes | 0 / 0 / 0 | 531 | 2.6 → 8.0 | 0.00 → 0.00 | 50.0 | 0.0 | 6 | 3 | 510 | **0** | 0.959 | 0.996 | 285.9 | 15.2 | 17.9 | 0.707 | False |
| 4 | new | yes | 0 / 0 / 0 | 65 | 6.3 → 7.6 | 0.00 → 0.00 | 50.2 | 0.0 | 8 | 2 | 852 | **0** | 0.960 | 0.997 | 317.5 | 15.0 | 17.9 | 0.717 | False |
| 5 | new | yes | 0 / 0 / 0 | 255 | 8.0 → 7.3 | 0.00 → 0.00 | 49.4 | 0.0 | 8 | 2 | 852 | **0** | 0.975 | 1.000 | 366.2 | 17.2 | 17.3 | 0.759 | False |
| 6 | new | yes | 0 / 0 / 0 | 0 | 7.3 → 6.9 | 0.00 → 0.00 | 50.0 | 0.0 | 10 | 1 | 1194 | **0** | 0.992 | 0.997 | 331.5 | 16.7 | 18.1 | 0.753 | False |
| 7 | new | reference (gpu1 smoke (rt-psmoke-p2, Chrome, 20:26:00-20:30:16) overlapped) | 1 / 1 / 1 | 70 | 3.8 → 2.1 | 0.00 → 0.00 | 49.8 | 0.0 | 5 | 1 | 554 | **0** | 0.988 | 1.000 | 365.1 | 17.6 | 17.9 | 0.772 | False |
| 8 | new | reference (gpu1 smoke (until 20:30:16) overlapped the first 50 s of this run (PM 20:31)) | 1 / 0 / 1 | 0 | 2.1 → 1.7 | 0.00 → 0.00 | 49.7 | 0.0 | 3 | 1 | 298 | **0** | 0.992 | 1.000 | 410.1 | 18.4 | 17.7 | 0.791 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 5 | 4 | 2 | 340 | 0.98 | 1.00 | 1218.5 | 15.6 | 5.8 | 0.81 |
| 0-10 s | new | 6 | 8 | 2 | 852 | 0.98 | 1.00 | 1158.4 | 14.9 | 7.2 | 0.78 |
| 10-20 s | prev | 5 | 0 | 0 | 0 | 0.99 | 1.00 | 424.9 | 18.6 | 19.3 | 0.77 |
| 10-20 s | new | 6 | 0 | 0 | 0 | 0.99 | 1.00 | 399.2 | 16.5 | 20.1 | 0.72 |
| 20-30 s | prev | 5 | 0 | 0 | 0 | 1.00 | 1.00 | 589.3 | 18.9 | 13.8 | 0.84 |
| 20-30 s | new | 6 | 0 | 0 | 0 | 1.00 | 1.00 | 455.2 | 18.4 | 15.7 | 0.79 |
| 30-40 s | prev | 5 | 0 | 0 | 0 | 0.99 | 1.00 | 329.3 | 18.4 | 22.3 | 0.67 |
| 30-40 s | new | 6 | 0 | 0 | 0 | 1.00 | 1.00 | 306.2 | 18.5 | 23.3 | 0.66 |
| 40-50 s | prev | 5 | 0 | 0 | 0 | 0.98 | 1.00 | 282.1 | 15.3 | 19.8 | 0.73 |
| 40-50 s | new | 6 | 0 | 0 | 0 | 0.97 | 1.00 | 232.2 | 15.5 | 18.4 | 0.71 |
| 50-60 s | prev | 5 | 0 | 0 | 0 | 0.92 | 0.99 | 219.9 | 12.9 | 15.1 | 0.77 |
| 50-60 s | new | 6 | 0 | 0 | 0 | 0.86 | 0.99 | 270.6 | 12.3 | 13.8 | 0.76 |

Music-matched windows (the same music in both bundles): the music position is the virtual clock since the play mark (gpu1 matched-music.py); the bins are complete 10 s bins of music that each run reached within its 60 s of wall time. wall = the wall time the bin took; distinct frames = Unlock coverage per wall second (1.0 = real time); Unlocks / wall s; underrun quanta and episodes of the page and the engine gap frames over the wall interval of the bin (the 10 s snapshots and the 1 Hz engine reports, linearly interpolated). Median over the counted runs.

| music bin | bundle | n | wall time of the bin (s) | distinct frames per wall s | Unlocks per wall s | underrun quanta | episodes | engine gap frames |
|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 5 | 12.9 | 0.976 | 10.51 | 4 | 2 | 340 |
| 0-10 s | new | 6 | 13.5 | 0.965 | 10.55 | 8 | 2 | 852 |
| 10-20 s | prev | 5 | 11.5 | 0.947 | 10.75 | 0 | 0 | 0 |
| 10-20 s | new | 6 | 12.3 | 0.918 | 10.75 | 0 | 0 | 0 |
| 20-30 s | prev | 5 | 14.3 | 0.941 | 10.76 | 0 | 0 | 0 |
| 20-30 s | new | 6 | 15.0 | 0.918 | 10.74 | 0 | 0 | 0 |
| 30-40 s | prev | 5 | 13.4 | 0.874 | 10.41 | 0 | 0 | 0 |
| 30-40 s | new | 5 | 13.6 | 0.861 | 10.43 | 0 | 0 | 0 |

Old vs new at the same music position (counted runs; exact permutation test; only bins both bundles reached):

| music bin | measure | old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|
| 0-10 s | wall time of the bin (s) | 12.939 (5) | 13.483 (6) | +0.545 | 0.154 |
| 0-10 s | distinct frames per wall s | 0.976 (5) | 0.965 (6) | -0.011 | 0.489 |
| 0-10 s | underrun quanta | 4.000 (5) | 8.000 (6) | +4.000 | 0.216 |
| 0-10 s | episodes | 2.000 (5) | 2.000 (6) | +0.000 | 0.424 |
| 0-10 s | engine gap frames | 340.000 (5) | 852.000 (6) | +512.000 | 0.223 |
| 10-20 s | wall time of the bin (s) | 11.542 (5) | 12.344 (6) | +0.802 | 0.277 |
| 10-20 s | distinct frames per wall s | 0.947 (5) | 0.918 (6) | -0.028 | 0.195 |
| 10-20 s | underrun quanta | 0.000 (5) | 0.000 (6) | +0.000 | 1.000 |
| 10-20 s | episodes | 0.000 (5) | 0.000 (6) | +0.000 | 1.000 |
| 10-20 s | engine gap frames | 0.000 (5) | 0.000 (6) | +0.000 | 1.000 |
| 20-30 s | wall time of the bin (s) | 14.311 (5) | 15.007 (6) | +0.696 | 0.214 |
| 20-30 s | distinct frames per wall s | 0.941 (5) | 0.918 (6) | -0.023 | 0.580 |
| 20-30 s | underrun quanta | 0.000 (5) | 0.000 (6) | +0.000 | 0.182 |
| 20-30 s | episodes | 0.000 (5) | 0.000 (6) | +0.000 | 0.182 |
| 20-30 s | engine gap frames | 0.000 (5) | 0.000 (6) | +0.000 | 0.182 |
| 30-40 s | wall time of the bin (s) | 13.376 (5) | 13.591 (5) | +0.215 | 0.730 |
| 30-40 s | distinct frames per wall s | 0.874 (5) | 0.861 (5) | -0.013 | 0.532 |
| 30-40 s | underrun quanta | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |
| 30-40 s | episodes | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |
| 30-40 s | engine gap frames | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 6.000 (5) | 8.000 (6) | +2.000 | 0.569 | 7.000 (8) | 7.000 (8) | +0.000 | 0.832 |
| underrun quanta 30-60 s (heavy interval) | 0.000 (5) | 0.000 (6) | +0.000 | 0.455 | 0.000 (8) | 0.000 (8) | +0.000 | 0.200 |
| episodes 0-60 s | 3.000 (5) | 2.000 (6) | -1.000 | 0.686 | 2.500 (8) | 1.500 (8) | -1.000 | 0.646 |
| engine gap frames 0-60 s | 510.000 (5) | 852.000 (6) | +342.000 | 0.578 | 595.000 (8) | 767.000 (8) | +172.000 | 0.785 |
| write rate mean 10-60 s | 0.981 (5) | 0.963 (6) | -0.018 | 0.177 | 0.971 (8) | 0.971 (8) | +0.000 | 0.811 |
| Worker CPU s (60 s) | 49.990 (5) | 49.965 (6) | -0.025 | 0.450 | 49.945 (8) | 49.905 (8) | -0.040 | 0.522 |
| UI idles/s (10-60 s) | 337.387 (5) | 328.680 (6) | -8.707 | 0.190 | 333.786 (8) | 335.378 (8) | +1.592 | 0.915 |
| presents/s (10-60 s) | 16.641 (5) | 15.944 (6) | -0.697 | 0.165 | 15.997 (8) | 16.763 (8) | +0.766 | 0.860 |
| lagCuts/s (10-60 s) | 17.376 (5) | 18.019 (6) | +0.643 | 0.149 | 17.249 (8) | 17.887 (8) | +0.638 | 0.086 |
| virtual s per real s (10-60 s) | 0.745 (5) | 0.731 (6) | -0.015 | 0.128 | 0.734 (8) | 0.748 (8) | +0.014 | 0.941 |



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
17:59:42 preflight ok load 5.06 io avg10=0.00 avg60=0.00 avg300=0.00
17:59:42 idle chrome prev 1 (quiet: load 5.06 io 0 after 0 s) load 5.06 io avg10=0.00 avg60=0.00 avg300=0.00
17:59:51 idle chrome new 1 (quiet: load 4.81 io 0 after 0 s) load 4.81 io avg10=0.00 avg60=0.00 avg300=0.00
18:00:01 idle chrome new 2 (quiet: load 4.67 io 0 after 0 s) load 4.67 io avg10=0.00 avg60=0.00 avg300=0.00
18:00:11 idle chrome prev 2 (quiet: load 5.31 io 0 after 0 s) load 5.31 io avg10=0.00 avg60=0.00 avg300=0.00
18:00:20 idle chrome prev 3 (quiet: load 5.72 io 0 after 0 s) load 5.72 io avg10=0.00 avg60=0.00 avg300=0.00
18:00:30 idle chrome new 3 (quiet: load 5.38 io 0 after 0 s) load 5.38 io avg10=0.00 avg60=0.00 avg300=0.00
18:00:40 idle chrome new 4 (quiet: load 5.31 io 0 after 0 s) load 5.31 io avg10=0.00 avg60=0.00 avg300=0.00
18:00:49 idle chrome prev 4 (quiet: load 5.48 io 0 after 0 s) load 5.48 io avg10=0.00 avg60=0.00 avg300=0.00
18:00:59 idle chrome prev 5 (quiet: load 5.48 io 0 after 0 s) load 5.48 io avg10=0.00 avg60=0.00 avg300=0.00
18:01:08 idle chrome new 5 (quiet: load 5.65 io 0 after 0 s) load 5.65 io avg10=0.00 avg60=0.00 avg300=0.00
18:01:21 idle chrome new 6 (quiet: load 5.61 io 0 after 0 s) load 5.61 io avg10=0.00 avg60=0.00 avg300=0.00
18:01:34 idle chrome prev 6 (quiet: load 5.37 io 0 after 0 s) load 5.37 io avg10=0.00 avg60=0.00 avg300=0.00
18:01:44 idle chrome prev 7 (quiet: load 6.79 io 0 after 0 s) load 6.79 io avg10=0.00 avg60=0.00 avg300=0.00
18:01:54 idle chrome new 7 (quiet: load 6.51 io 0 after 0 s) load 6.51 io avg10=0.00 avg60=0.00 avg300=0.00
18:02:03 idle chrome new 8 (quiet: load 6.35 io 0 after 0 s) load 6.35 io avg10=0.00 avg60=0.00 avg300=0.00
18:02:43 idle chrome prev 8 (quiet: load 6.75 io 0 after 30 s) load 6.75 io avg10=0.00 avg60=0.01 avg300=0.00
18:02:53 idle chrome prev 9 (quiet: load 6.56 io 0 after 0 s) load 6.56 io avg10=0.00 avg60=0.01 avg300=0.00
18:03:02 idle chrome new 9 (quiet: load 6.32 io 0 after 0 s) load 6.32 io avg10=0.00 avg60=0.01 avg300=0.00
18:03:12 idle chrome new 10 (quiet: load 6.1 io 0 after 0 s) load 6.10 io avg10=0.00 avg60=0.00 avg300=0.00
18:03:22 idle chrome prev 10 (quiet: load 6.02 io 0 after 0 s) load 6.02 io avg10=0.00 avg60=0.00 avg300=0.00
18:03:31 idle chrome x10 load 5.78 io avg10=0.00 avg60=0.00 avg300=0.00
18:04:39 idle chrome prev 11 (quiet: load 2.9 io 0 after 0 s) load 2.90 io avg10=0.00 avg60=0.19 avg300=0.08
18:04:49 idle chrome new 11 (quiet: load 3.36 io 0 after 0 s) load 3.36 io avg10=0.00 avg60=0.16 avg300=0.08
18:04:59 idle chrome new 12 (quiet: load 3.14 io 0 after 0 s) load 3.14 io avg10=0.00 avg60=0.14 avg300=0.07
18:09:25 idle chrome prev 11 (quiet: load 3.34 io 0 after 0 s) load 3.34 io avg10=0.00 avg60=0.02 avg300=0.02
18:09:34 idle chrome new 11 (quiet: load 3.13 io 0 after 0 s) load 3.13 io avg10=0.00 avg60=0.02 avg300=0.02
18:09:44 idle chrome new 12 (quiet: load 2.8 io 0 after 0 s) load 2.80 io avg10=0.00 avg60=0.01 avg300=0.01
18:09:54 idle chrome prev 12 (quiet: load 2.97 io 0 after 0 s) load 2.97 io avg10=0.00 avg60=0.01 avg300=0.01
18:10:03 idle chrome prev 13 (quiet: load 2.82 io 0 after 0 s) load 2.82 io avg10=0.00 avg60=0.01 avg300=0.01
18:10:13 idle chrome new 13 (quiet: load 3.05 io 0 after 0 s) load 3.05 io avg10=0.00 avg60=0.00 avg300=0.01
18:10:22 idle chrome new 14 (quiet: load 2.8 io 0 after 0 s) load 2.80 io avg10=0.00 avg60=0.00 avg300=0.00
18:10:32 idle chrome prev 14 (quiet: load 2.59 io 0 after 0 s) load 2.59 io avg10=0.00 avg60=0.00 avg300=0.00
18:10:42 idle chrome prev 15 (quiet: load 2.42 io 0 after 0 s) load 2.42 io avg10=0.00 avg60=0.00 avg300=0.00
18:10:51 idle chrome new 15 (quiet: load 2.55 io 0 after 0 s) load 2.55 io avg10=0.00 avg60=0.00 avg300=0.00
18:11:01 idle chrome new 16 (quiet: load 2.68 io 0 after 0 s) load 2.68 io avg10=0.00 avg60=0.00 avg300=0.00
18:11:10 idle chrome prev 16 (quiet: load 2.87 io 0 after 0 s) load 2.87 io avg10=0.00 avg60=0.00 avg300=0.00
18:11:20 idle chrome x6 load 2.97 io avg10=0.00 avg60=0.00 avg300=0.00
19:29:40 scenes firefox scn-client.json d1-sampler-envelope load 5.68 io avg10=0.00 avg60=0.00 avg300=0.02
19:29:40 scenes chrome scn-client.json  load 5.68 io avg10=0.00 avg60=0.00 avg300=0.02
19:30:17 scenes firefox scn-client.json d2-browser-folder load 10.35 io avg10=0.01 avg60=0.03 avg300=0.03
19:30:41 scenes firefox scn-client.json d3-settings-all-tabs load 11.39 io avg10=0.00 avg60=0.02 avg300=0.02
19:31:28 scenes firefox scn-client.json d4-playlist-middle-drag load 12.29 io avg10=0.00 avg60=0.00 avg300=0.01
19:31:50 scenes firefox scn-client.json d5-mixer-front load 12.25 io avg10=0.00 avg60=0.00 avg300=0.00
19:32:19 scenes firefox scn-client.json d6-playlist-close load 11.98 io avg10=0.04 avg60=0.02 avg300=0.00
19:32:37 scenes firefox scn-client.json d7-main-close load 11.45 io avg10=0.00 avg60=0.01 avg300=0.00
19:33:02 scenes firefox scn-client.json d8-file-menu-100 load 13.11 io avg10=0.01 avg60=0.03 avg300=0.00
19:35:40 scenes firefox scn-client.json d9-keys load 13.67 io avg10=0.00 avg60=0.00 avg300=0.00
19:36:09 scenes chrome scn-plus.json  load 11.81 io avg10=0.00 avg60=0.00 avg300=0.00
19:36:26 scenes firefox scn-client.json d10-pianoroll-f12 load 11.01 io avg10=0.00 avg60=0.00 avg300=0.00
19:36:34 scenes chrome scn-alpha1.json  load 10.40 io avg10=0.00 avg60=0.00 avg300=0.00
19:36:59 scenes firefox scn-client.json d11-menu-hover-escape load 10.10 io avg10=0.00 avg60=0.00 avg300=0.00
19:37:29 scenes firefox scn-plus.json  load 8.90 io avg10=0.00 avg60=0.00 avg300=0.00
19:37:47 export start load 7.89 io avg10=0.03 avg60=0.02 avg300=0.00
19:37:57 scenes firefox scn-alpha1.json  load 9.32 io avg10=0.01 avg60=0.01 avg300=0.00
19:43:03 export end load 8.50 io avg10=0.00 avg60=0.00 avg300=0.00
19:43:03 alpha7 load 8.50 io avg10=0.00 avg60=0.00 avg300=0.00
19:45:06 pure start (wine check: no Wine process after 105 s) load 4.35 io avg10=0.00
19:45:06 p3 chrome prev 1 (quiet: load 4.35 io 0 after 0 s) load 4.35 io avg10=0.00 avg60=0.00 avg300=0.00
19:46:55 p3 chrome prev 1 end load 3.30 io avg10=0.00 avg60=0.00 avg300=0.00
19:46:55 p3 chrome new 1 (quiet: load 3.3 io 0 after 0 s) load 3.30 io avg10=0.00 avg60=0.00 avg300=0.00
19:48:45 p3 chrome new 1 end load 6.17 io avg10=0.09 avg60=0.21 avg300=0.06
19:51:05 p3 chrome new 2 (quiet: load 5.38 io 0 after 140 s) load 5.38 io avg10=0.00 avg60=0.26 avg300=0.19
19:52:55 p3 chrome new 2 end load 4.74 io avg10=0.14 avg60=0.34 avg300=0.28
19:54:00 p3 chrome prev 2 (quiet: load 3.7 io 0 after 65 s) load 3.70 io avg10=0.00 avg60=0.21 avg300=0.26
19:55:50 p3 chrome prev 2 end load 4.64 io avg10=0.00 avg60=0.11 avg300=0.22
19:55:50 p3 chrome prev 3 (quiet: load 4.64 io 0 after 0 s) load 4.64 io avg10=0.00 avg60=0.11 avg300=0.22
19:57:39 p3 chrome prev 3 end load 6.49 io avg10=3.00 avg60=1.98 avg300=0.77
20:06:30 p3 chrome new 3 (quiet: load 2.59 io 0 after 531 s) load 2.59 io avg10=0.00 avg60=0.83 avg300=4.42
20:08:20 p3 chrome new 3 end load 8.03 io avg10=0.18 avg60=0.33 avg300=3.13
20:09:25 p3 chrome new 4 (quiet: load 6.26 io 0 after 65 s) load 6.26 io avg10=0.00 avg60=0.22 avg300=2.55
20:11:15 p3 chrome new 4 end load 7.59 io avg10=0.00 avg60=0.02 avg300=1.72
20:11:15 p3 chrome prev 4 (quiet: load 7.59 io 0 after 0 s) load 7.59 io avg10=0.00 avg60=0.02 avg300=1.72
20:13:04 p3 chrome prev 4 end load 7.70 io avg10=0.00 avg60=0.00 avg300=1.18
20:13:04 p3 chrome prev 5 (quiet: load 7.7 io 0 after 0 s) load 7.70 io avg10=0.00 avg60=0.00 avg300=1.18
20:14:54 p3 chrome prev 5 end load 13.03 io avg10=0.00 avg60=0.00 avg300=0.80
20:19:09 p3 chrome new 5 (quiet: load 7.95 io 0 after 255 s) load 7.95 io avg10=0.00 avg60=0.00 avg300=0.31
20:20:58 p3 chrome new 5 end load 7.32 io avg10=0.00 avg60=0.00 avg300=0.20
20:20:58 p3 chrome new 6 (quiet: load 7.32 io 0 after 0 s) load 7.32 io avg10=0.00 avg60=0.00 avg300=0.20
20:22:48 p3 chrome new 6 end load 6.87 io avg10=0.00 avg60=0.00 avg300=0.13
20:22:48 p3 chrome prev 6 (quiet: load 6.87 io 0 after 0 s) load 6.87 io avg10=0.00 avg60=0.00 avg300=0.13
20:24:37 p3 chrome prev 6 end load 6.02 io avg10=0.00 avg60=0.00 avg300=0.07
20:24:37 pure end chrome (all 12 runs of the mode finished) load 6.02 io avg10=0.00 avg60=0.00 avg300=0.07
20:24:37 pure extra pairs 7-8 start (wine check: no Wine process after 0 s)
20:24:37 p3 chrome prev 7 (quiet: load 6.02 io 0 after 0 s) load 6.02 io avg10=0.00 avg60=0.00 avg300=0.07
20:26:27 p3 chrome prev 7 end load 8.42 io avg10=0.00 avg60=0.00 avg300=0.04
20:27:37 p3 chrome new 7 (quiet: load 3.77 io 0 after 70 s) load 3.77 io avg10=0.00 avg60=0.11 avg300=0.07
20:29:27 p3 chrome new 7 end load 2.11 io avg10=0.00 avg60=0.01 avg300=0.04
20:29:27 p3 chrome new 8 (quiet: load 2.11 io 0 after 0 s) load 2.11 io avg10=0.00 avg60=0.01 avg300=0.04
20:31:16 p3 chrome new 8 end load 1.70 io avg10=0.00 avg60=0.00 avg300=0.02
20:31:16 p3 chrome prev 8 (quiet: load 1.7 io 0 after 0 s) load 1.70 io avg10=0.00 avg60=0.00 avg300=0.02
20:33:05 p3 chrome prev 8 end load 1.13 io avg10=0.00 avg60=0.00 avg300=0.00
20:33:05 pure end chrome (all 4 runs of the mode finished) load 1.13 io avg10=0.00 avg60=0.00 avg300=0.00
20:33:05 extra pairs done
```
