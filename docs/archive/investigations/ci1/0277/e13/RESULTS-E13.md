# 0277: verification of `fl-engine-ci-0363b050` (Worker 0f8ed7c4-f576-4b40-a3b9-7fca05115164, page of git d0447902) against the previous production bundle `fl-engine-ci-7a9b0800`

Real engine in real browsers (Chrome / Firefox, headless; Firefox on Xvfb :77 with WebGL). No stand-in, no replay. The bundle comes from R2 (read only); every manifest sha256 is checked by `fetch`.

## Pass criteria

| criterion | value | pass |
|---|---|---|
| Firefox has WebGL (preflight) | preflight: webglRenderer llvmpipe, or similar desk.mode gl first idle 3857 | yes |
| No stop (11 scenes + more, both browsers, play, export; d7 is the intended exit) | stops: none; not ok: none; no result at all (the harness did not start): none | yes |
| Export WAV equals the reference | 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` (expected 22863828 B, `381cdb5acf891a16…`) | yes |
| First idle ≤ previous + 500 ms (chrome, WebGL runs) | all runs: new median 4320 ms, previous 4428 ms, difference -109 ms; means 4314 ± 19 (SE, n=10) / 4421 ± 22 (n=10), difference of the means -108 ± 29 (SE); quiet runs only (load ≤12, I/O avg10 <1; n new 10 / prev 10): new 4320, previous 4428, difference -109 ms | yes |
| First idle ≤ previous + 500 ms (firefox, WebGL runs) | all runs: new median 4172 ms, previous 4147 ms, difference +26 ms; means 4156 ± 75 (SE, n=6) / 4182 ± 77 (n=6), difference of the means -26 ± 107 (SE); quiet runs only (load ≤12, I/O avg10 <1; n new 6 / prev 6): new 4172, previous 4147, difference +26 ms | yes |
| File menu ×100 (chrome) | 100/100 | yes |
| File menu ×100 (firefox) | 100/100 | yes |

## Scenes (new bundle)

| browser | scene | result | first idle ms | File ×100 | load, I/O pressure (start→end) |
|---|---|---|---|---|---|
| chrome | d1-sampler-envelope | ok | 4638 | - | load 2.9→3.4, io 0.0→0.0 |
| chrome | d2-browser-folder | ok | 4027 | - | load 3.4→3.6, io 0.0→0.0 |
| chrome | d3-settings-all-tabs | ok | 3996 | - | load 3.6→4.2, io 0.0→0.0 |
| chrome | d4-playlist-middle-drag | ok | 4328 | - | load 4.2→4.6, io 0.0→0.0 |
| chrome | d5-mixer-front | ok | 4140 | - | load 4.6→6.8, io 0.0→0.0 |
| chrome | d6-playlist-close | ok | 4313 | - | load 6.8→7.7, io 0.0→0.0 |
| chrome | d7-main-close | ok (intended exit) | 4184 | - | load 7.7→11.8, io 0.0→0.0 |
| chrome | d8-file-menu-100 | ok | 4007 | 100/100 | load 11.8→9.6, io 0.0→0.0 |
| chrome | d9-keys | ok | 4099 | - | load 9.6→13.2, io 0.0→0.0 |
| chrome | d10-pianoroll-f12 | ok | 4795 | - | load 13.2→12.8, io 0.0→0.0 |
| chrome | d11-menu-hover-escape | ok | 4982 | - | load 12.8→11.9, io 0.0→0.0 |
| chrome | add-menu-simple-fade | ok | 5926 | - | load 11.1→11.7, io 0.0→0.0 |
| chrome | picker-cost | ok | 4808 | - | load 11.7→12.3, io 0.0→0.0 |
| chrome | picker-f8-fade | ok | 4862 | - | load 11.7→11.7, io 0.0→0.0 |
| chrome | plus-more-plugins | ok | 5022 | - | load 11.9→11.1, io 0.0→0.0 |
| firefox | d1-sampler-envelope | ok | 3781 | - | load 5.3→3.4, io 0.2→0.0 |
| firefox | d2-browser-folder | ok | 4291 | - | load 3.4→3.9, io 0.0→0.0 |
| firefox | d3-settings-all-tabs | ok | 4031 | - | load 3.9→4.6, io 0.0→0.0 |
| firefox | d4-playlist-middle-drag | ok | 4244 | - | load 4.6→6.5, io 0.0→0.0 |
| firefox | d5-mixer-front | ok | 4157 | - | load 6.8→9.2, io 0.0→0.0 |
| firefox | d6-playlist-close | ok | 3952 | - | load 3.4→3.2, io 0.0→0.0 |
| firefox | d7-main-close | ok (intended exit) | 4074 | - | load 11.6→11.6, io 0.0→0.0 |
| firefox | d8-file-menu-100 | ok | 4251 | 100/100 | load 11.3→13.2, io 0.0→0.0 |
| firefox | d9-keys | ok | 4550 | - | load 13.1→12.1, io 0.0→0.0 |
| firefox | d10-pianoroll-f12 | ok | 4407 | - | load 11.9→11.4, io 0.0→0.0 |
| firefox | d11-menu-hover-escape | ok | 4509 | - | load 11.6→11.7, io 0.0→0.0 |
| firefox | add-menu-simple-fade | ok | 5480 | - | load 13.6→14.7, io 0.0→0.0 |
| firefox | picker-cost | ok | 4247 | - | load 16.9→14.1, io 0.0→0.0 |
| firefox | picker-f8-fade | ok | 4365 | - | load 14.7→16.9, io 0.0→0.0 |
| firefox | plus-more-plugins | ok | 4692 | - | load 12.6→12.3, io 0.0→0.0 |


The scenes ran as two parallel jobs (Chrome and Firefox), so the first idle values of this table are reference values (the load is next to each).

## alpha7 (main window geometry, guest 1920x1080, Chrome)

result ok; load 6.4→6.6, io 0.0→0.0; windows visible at the end: 1; windows larger than 1800 px wide:
- `14 TFruityLoopsMainForm 'FL Studio 2026' @0,0 1920x1080 z0`
- expected after alpha7: the main window @0,0 1920x1080 (previous df0512b0: @-5,-6 1930x1092). Screenshots: `main-1920x1080.png, scenario-alpha7-look.png`

## First idle (single runs, cold, a new profile each; previous and new alternating)

| browser | bundle | run | first idle ms | load, I/O pressure at the start | at the end | other Chrome browsers during the run (max) | quiet (load ≤8, I/O <1) | webglRenderer | desk.mode |
|---|---|---|---|---|---|---|---|---|---|
| chrome | prev | 1 | 4392 | 1.38 / io 0.00 | 1.31 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 1 | 4232 | 1.31 / io 0.00 | 1.45 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 2 | 4328 | 1.45 / io 0.00 | 1.53 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 2 | 4460 | 1.53 / io 0.00 | 1.89 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 3 | 4372 | 1.89 / io 0.00 | 1.83 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 3 | 4311 | 1.83 / io 0.00 | 1.78 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 4 | 4353 | 1.78 / io 0.00 | 1.88 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 4 | 4547 | 1.88 / io 0.00 | 1.96 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 5 | 4458 | 1.96 / io 0.00 | 2.11 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 5 | 4229 | 2.11 / io 0.00 | 1.86 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 6 | 4350 | 1.86 / io 0.00 | 1.73 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 6 | 4419 | 1.73 / io 0.00 | 1.53 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 7 | 4288 | 1.53 / io 0.00 | 1.45 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 7 | 4429 | 1.45 / io 0.00 | 1.41 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 8 | 4277 | 1.41 / io 0.00 | 1.58 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 8 | 4455 | 1.58 / io 0.00 | 1.86 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 9 | 4438 | 1.86 / io 0.00 | 1.73 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 9 | 4280 | 1.73 / io 0.00 | 1.84 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 10 | 4346 | 1.84 / io 0.00 | 1.71 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 10 | 4381 | 1.71 / io 0.00 | 2.93 / io 0.00 | not observed | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| firefox | prev | 1 | 4208 | 1.55 / io 0.00 | 1.62 / io 0.16 | not observed | yes | llvmpipe, or similar | gl |
| firefox | new | 1 | 3962 | 2.51 / io 0.00 | 2.74 / io 0.00 | not observed | yes | llvmpipe, or similar | gl |
| firefox | new | 2 | 3974 | 2.74 / io 0.00 | 3.22 / io 0.00 | not observed | yes | llvmpipe, or similar | gl |
| firefox | prev | 2 | 4086 | 3.22 / io 0.00 | 3.20 / io 0.00 | not observed | yes | llvmpipe, or similar | gl |
| firefox | prev | 3 | 4331 | 3.18 / io 0.00 | 3.08 / io 0.00 | not observed | yes | llvmpipe, or similar | gl |
| firefox | new | 3 | 4195 | 3.08 / io 0.00 | 3.07 / io 0.00 | not observed | yes | llvmpipe, or similar | gl |
| firefox | new | 4 | 4172 | 3.07 / io 0.00 | 3.28 / io 0.00 | not observed | yes | llvmpipe, or similar | gl |
| firefox | prev | 4 | 4036 | 3.28 / io 0.00 | 3.63 / io 0.00 | not observed | yes | llvmpipe, or similar | gl |
| firefox | prev | 5 | 3969 | 3.63 / io 0.00 | 3.68 / io 0.00 | not observed | yes | llvmpipe, or similar | gl |
| firefox | new | 5 | 4173 | 3.68 / io 0.00 | 5.29 / io 0.00 | not observed | yes | llvmpipe, or similar | gl |
| firefox | new | 6 | 4463 | 5.29 / io 0.00 | 5.91 / io 0.00 | not observed | yes | llvmpipe, or similar | gl |
| firefox | prev | 6 | 4465 | 5.91 / io 0.00 | 6.60 / io 0.00 | not observed | yes | llvmpipe, or similar | gl |

| browser | mode | previous median (n) | new median (n) | difference |
|---|---|---|---|---|
| chrome | gl | 4428 (10) | 4320 (10) | -109 |
| firefox | gl | 4147 (6) | 4172 (6) | 26 |

## Export (Chrome, the reference project, Ctrl+R)

result ok; load 12.3→6.4, io 0.0→0.0; seconds 304
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
| 1 | prev | yes | 0 / 0 / 0 | 0 | 1.0 → 1.4 | 0.00 → 0.00 | 49.4 | 0.0 | 2 | 1 | 170 | **0** | 0.988 | 0.996 | 403.1 | 18.5 | 16.9 | 0.791 | False |
| 2 | prev | yes | 0 / 0 / 0 | 0 | 1.9 → 2.1 | 0.00 → 0.00 | 49.6 | 0.0 | 2 | 1 | 170 | **0** | 0.983 | 1.000 | 397.5 | 18.2 | 17.9 | 0.783 | False |
| 3 | prev | yes | 0 / 0 / 0 | 0 | 1.9 → 1.7 | 0.00 → 0.00 | 49.6 | 0.0 | 2 | 1 | 170 | **0** | 0.981 | 0.994 | 458.5 | 18.3 | 15.3 | 0.812 | False |
| 4 | prev | yes | 0 / 0 / 0 | 0 | 1.7 → 3.4 | 0.00 → 0.00 | 49.5 | 0.0 | 2 | 1 | 170 | **0** | 0.985 | 0.997 | 450.4 | 18.5 | 16.4 | 0.808 | False |
| 5 | prev | yes | 0 / 0 / 0 | 0 | 3.4 → 2.5 | 0.00 → 0.00 | 49.3 | 0.0 | 2 | 1 | 170 | **0** | 0.985 | 0.997 | 470.2 | 18.7 | 15.9 | 0.817 | False |
| 6 | prev | yes | 0 / 0 / 0 | 0 | 2.4 → 2.6 | 0.00 → 0.00 | 49.5 | 0.0 | 2 | 1 | 170 | **0** | 0.990 | 0.998 | 430.3 | 18.6 | 16.8 | 0.803 | False |
| 1 | new | yes | 0 / 0 / 0 | 0 | 2.3 → 1.7 | 0.00 → 0.04 | 49.3 | 0.0 | 4 | 2 | 340 | **0** | 0.986 | 0.994 | 468.1 | 19.0 | 16.4 | 0.819 | False |
| 2 | new | yes | 0 / 0 / 0 | 15 | 1.4 → 1.9 | 0.00 → 0.00 | 49.4 | 0.0 | 2 | 1 | 170 | **0** | 0.994 | 0.997 | 440.3 | 18.6 | 16.5 | 0.808 | False |
| 3 | new | yes | 0 / 0 / 0 | 0 | 1.7 → 1.1 | 0.00 → 0.00 | 49.4 | 0.0 | 0 | 0 | 0 | **0** | 0.992 | 0.995 | 454.8 | 18.5 | 15.8 | 0.814 | False |
| 4 | new | yes | 0 / 0 / 0 | 0 | 1.1 → 1.9 | 0.00 → 0.00 | 49.2 | 0.0 | 0 | 0 | 0 | **0** | 0.992 | 0.995 | 439.0 | 18.9 | 16.6 | 0.806 | False |
| 5 | new | yes | 0 / 0 / 0 | 0 | 2.5 → 1.6 | 0.00 → 0.00 | 49.4 | 0.0 | 11 | 2 | 1236 | **0** | 0.992 | 0.994 | 430.1 | 18.7 | 16.9 | 0.805 | False |
| 6 | new | yes | 0 / 0 / 0 | 0 | 1.5 → 2.4 | 0.00 → 0.00 | 49.5 | 0.0 | 3 | 1 | 298 | **0** | 0.996 | 0.996 | 496.8 | 18.7 | 14.4 | 0.834 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 2 | 1 | 170 | 0.98 | 1.00 | 1308.1 | 16.2 | 3.4 | 0.86 |
| 0-10 s | new | 6 | 2 | 1 | 234 | 0.98 | 1.00 | 1378.3 | 16.3 | 0.6 | 0.90 |
| 10-20 s | prev | 6 | 0 | 0 | 0 | 0.99 | 1.00 | 553.3 | 20.3 | 15.9 | 0.83 |
| 10-20 s | new | 6 | 0 | 0 | 0 | 0.99 | 1.00 | 581.1 | 20.2 | 15.0 | 0.85 |
| 20-30 s | prev | 6 | 0 | 0 | 0 | 1.00 | 1.00 | 714.7 | 21.3 | 9.3 | 0.90 |
| 20-30 s | new | 6 | 0 | 0 | 0 | 1.00 | 1.00 | 721.0 | 21.8 | 7.9 | 0.91 |
| 30-40 s | prev | 6 | 0 | 0 | 0 | 0.99 | 1.00 | 458.4 | 20.7 | 21.8 | 0.74 |
| 30-40 s | new | 6 | 0 | 0 | 0 | 0.99 | 1.00 | 442.1 | 20.9 | 23.4 | 0.73 |
| 40-50 s | prev | 6 | 0 | 0 | 0 | 0.99 | 1.00 | 307.2 | 17.0 | 18.8 | 0.76 |
| 40-50 s | new | 6 | 0 | 0 | 0 | 1.00 | 1.00 | 354.6 | 17.2 | 17.1 | 0.79 |
| 50-60 s | prev | 6 | 0 | 0 | 0 | 0.95 | 0.98 | 177.6 | 13.3 | 16.0 | 0.79 |
| 50-60 s | new | 6 | 0 | 0 | 0 | 0.97 | 0.97 | 137.1 | 13.7 | 17.1 | 0.78 |

Music-matched windows (the same music in both bundles): the music position is the virtual clock since the play mark (gpu1 matched-music.py); the bins are complete 10 s bins of music that each run reached within its 60 s of wall time. wall = the wall time the bin took; distinct frames = Unlock coverage per wall second (1.0 = real time); Unlocks / wall s; underrun quanta and episodes of the page and the engine gap frames over the wall interval of the bin (the 10 s snapshots and the 1 Hz engine reports, linearly interpolated). Median over the counted runs.

| music bin | bundle | n | wall time of the bin (s) | distinct frames per wall s | Unlocks per wall s | underrun quanta | episodes | engine gap frames |
|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 12.2 | 0.981 | 10.56 | 2 | 1 | 170 |
| 0-10 s | new | 6 | 11.3 | 0.978 | 10.53 | 2 | 1 | 234 |
| 10-20 s | prev | 6 | 11.2 | 0.976 | 10.73 | 0 | 0 | 0 |
| 10-20 s | new | 6 | 11.5 | 0.973 | 10.70 | 0 | 0 | 0 |
| 20-30 s | prev | 6 | 12.5 | 0.948 | 10.79 | 0 | 0 | 0 |
| 20-30 s | new | 6 | 11.9 | 0.966 | 10.80 | 0 | 0 | 0 |
| 30-40 s | prev | 6 | 13.5 | 0.917 | 10.70 | 0 | 0 | 0 |
| 30-40 s | new | 6 | 13.0 | 0.935 | 10.73 | 0 | 0 | 0 |
| 40-50 s | new | 1 | 12.6 | 0.802 | 10.61 | 0 | 0 | 0 |

Old vs new at the same music position (counted runs; exact permutation test; only bins both bundles reached):

| music bin | measure | old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|
| 0-10 s | wall time of the bin (s) | 12.158 (6) | 11.278 (6) | -0.880 | 0.041 |
| 0-10 s | distinct frames per wall s | 0.981 (6) | 0.978 (6) | -0.002 | 0.613 |
| 0-10 s | underrun quanta | 2.000 (6) | 2.500 (6) | +0.500 | 0.682 |
| 0-10 s | episodes | 1.000 (6) | 1.000 (6) | +0.000 | 1.000 |
| 0-10 s | engine gap frames | 170.000 (6) | 234.000 (6) | +64.000 | 0.682 |
| 10-20 s | wall time of the bin (s) | 11.206 (6) | 11.508 (6) | +0.302 | 0.277 |
| 10-20 s | distinct frames per wall s | 0.976 (6) | 0.973 (6) | -0.003 | 0.883 |
| 10-20 s | underrun quanta | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 10-20 s | episodes | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 10-20 s | engine gap frames | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 20-30 s | wall time of the bin (s) | 12.510 (6) | 11.869 (6) | -0.641 | 0.076 |
| 20-30 s | distinct frames per wall s | 0.948 (6) | 0.966 (6) | +0.018 | 0.032 |
| 20-30 s | underrun quanta | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 20-30 s | episodes | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 20-30 s | engine gap frames | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 30-40 s | wall time of the bin (s) | 13.457 (6) | 13.010 (6) | -0.447 | 0.076 |
| 30-40 s | distinct frames per wall s | 0.917 (6) | 0.935 (6) | +0.019 | 0.284 |
| 30-40 s | underrun quanta | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 30-40 s | episodes | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 30-40 s | engine gap frames | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 2.000 (6) | 2.500 (6) | +0.500 | 0.682 | 2.000 (6) | 2.500 (6) | +0.500 | 0.682 |
| underrun quanta 30-60 s (heavy interval) | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| episodes 0-60 s | 1.000 (6) | 1.000 (6) | +0.000 | 1.000 | 1.000 (6) | 1.000 (6) | +0.000 | 1.000 |
| engine gap frames 0-60 s | 170.000 (6) | 234.000 (6) | +64.000 | 0.682 | 170.000 (6) | 234.000 (6) | +64.000 | 0.682 |
| write rate mean 10-60 s | 0.985 (6) | 0.992 (6) | +0.007 | 0.013 | 0.985 (6) | 0.992 (6) | +0.007 | 0.013 |
| Worker CPU s (60 s) | 49.530 (6) | 49.415 (6) | -0.115 | 0.043 | 49.530 (6) | 49.415 (6) | -0.115 | 0.043 |
| UI idles/s (10-60 s) | 440.363 (6) | 447.553 (6) | +7.190 | 0.240 | 440.363 (6) | 447.553 (6) | +7.190 | 0.240 |
| presents/s (10-60 s) | 18.483 (6) | 18.703 (6) | +0.220 | 0.032 | 18.483 (6) | 18.703 (6) | +0.220 | 0.032 |
| lagCuts/s (10-60 s) | 16.582 (6) | 16.434 (6) | -0.148 | 0.446 | 16.582 (6) | 16.434 (6) | -0.148 | 0.446 |
| virtual s per real s (10-60 s) | 0.806 (6) | 0.811 (6) | +0.005 | 0.123 | 0.806 (6) | 0.811 (6) | +0.005 | 0.123 |



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
20:52:02 fetched fl-engine-ci-0363b050 (260M) load 1.65 io avg10=0.03 avg60=0.09 avg300=0.03
20:52:11 built the page of d0447902 load 1.71 io avg10=0.01 avg60=0.07 avg300=0.03
20:56:14 preflight ok load 1.71 io avg10=0.29 avg60=0.08 avg300=0.01
20:56:49 idle chrome prev 1 (quiet: load 1.38 io 0 after 35 s) load 1.38 io avg10=0.00 avg60=0.04 avg300=0.01
20:56:59 idle chrome new 1 (quiet: load 1.31 io 0 after 0 s) load 1.31 io avg10=0.00 avg60=0.03 avg300=0.00
20:57:08 idle chrome new 2 (quiet: load 1.45 io 0 after 0 s) load 1.45 io avg10=0.00 avg60=0.03 avg300=0.00
20:57:18 idle chrome prev 2 (quiet: load 1.53 io 0 after 0 s) load 1.53 io avg10=0.00 avg60=0.02 avg300=0.00
20:57:27 idle chrome prev 3 (quiet: load 1.89 io 0 after 0 s) load 1.89 io avg10=0.00 avg60=0.02 avg300=0.00
20:57:37 idle chrome new 3 (quiet: load 1.83 io 0 after 0 s) load 1.83 io avg10=0.00 avg60=0.01 avg300=0.00
20:57:47 idle chrome new 4 (quiet: load 1.78 io 0 after 0 s) load 1.78 io avg10=0.00 avg60=0.01 avg300=0.00
20:57:56 idle chrome prev 4 (quiet: load 1.88 io 0 after 0 s) load 1.88 io avg10=0.00 avg60=0.01 avg300=0.00
20:58:06 idle chrome prev 5 (quiet: load 1.96 io 0 after 0 s) load 1.96 io avg10=0.00 avg60=0.00 avg300=0.00
20:58:15 idle chrome new 5 (quiet: load 2.11 io 0 after 0 s) load 2.11 io avg10=0.00 avg60=0.00 avg300=0.00
20:58:25 idle chrome new 6 (quiet: load 1.86 io 0 after 0 s) load 1.86 io avg10=0.00 avg60=0.00 avg300=0.00
20:58:34 idle chrome prev 6 (quiet: load 1.73 io 0 after 0 s) load 1.73 io avg10=0.00 avg60=0.00 avg300=0.00
20:58:44 idle chrome prev 7 (quiet: load 1.53 io 0 after 0 s) load 1.53 io avg10=0.00 avg60=0.00 avg300=0.00
20:58:54 idle chrome new 7 (quiet: load 1.45 io 0 after 0 s) load 1.45 io avg10=0.00 avg60=0.00 avg300=0.00
20:59:03 idle chrome new 8 (quiet: load 1.41 io 0 after 0 s) load 1.41 io avg10=0.00 avg60=0.00 avg300=0.00
20:59:13 idle chrome prev 8 (quiet: load 1.58 io 0 after 0 s) load 1.58 io avg10=0.00 avg60=0.00 avg300=0.00
20:59:22 idle chrome prev 9 (quiet: load 1.86 io 0 after 0 s) load 1.86 io avg10=0.00 avg60=0.00 avg300=0.00
20:59:32 idle chrome new 9 (quiet: load 1.73 io 0 after 0 s) load 1.73 io avg10=0.00 avg60=0.00 avg300=0.00
20:59:41 idle chrome new 10 (quiet: load 1.84 io 0 after 0 s) load 1.84 io avg10=0.00 avg60=0.00 avg300=0.00
20:59:51 idle chrome prev 10 (quiet: load 1.71 io 0 after 0 s) load 1.71 io avg10=0.00 avg60=0.00 avg300=0.00
21:00:00 idle chrome x10 load 2.93 io avg10=0.00 avg60=0.00 avg300=0.00
21:00:01 scenes chrome scn-client.json  load 2.93 io avg10=0.00 avg60=0.00 avg300=0.00
21:00:01 scenes firefox scn-client.json d1-sampler-envelope load 2.93 io avg10=0.00 avg60=0.00 avg300=0.00
21:00:35 scenes firefox scn-client.json d2-browser-folder load 3.40 io avg10=0.00 avg60=0.01 avg300=0.00
21:00:56 scenes firefox scn-client.json d3-settings-all-tabs load 3.86 io avg10=0.00 avg60=0.01 avg300=0.00
21:01:40 scenes firefox scn-client.json d4-playlist-middle-drag load 4.56 io avg10=0.00 avg60=0.00 avg300=0.00
21:02:02 scenes firefox scn-client.json d5-mixer-front load 6.52 io avg10=0.00 avg60=0.00 avg300=0.00
21:02:29 scenes firefox scn-client.json d6-playlist-close load 10.11 io avg10=0.00 avg60=0.00 avg300=0.00
21:02:45 scenes firefox scn-client.json d7-main-close load 11.58 io avg10=0.00 avg60=0.00 avg300=0.00
21:03:08 scenes firefox scn-client.json d8-file-menu-100 load 11.61 io avg10=0.00 avg60=0.00 avg300=0.00
21:05:38 scenes firefox scn-client.json d9-keys load 13.23 io avg10=0.00 avg60=0.00 avg300=0.00
21:06:16 scenes chrome scn-plus.json  load 11.89 io avg10=0.00 avg60=0.00 avg300=0.00
21:06:24 scenes firefox scn-client.json d10-pianoroll-f12 load 11.89 io avg10=0.00 avg60=0.00 avg300=0.00
21:06:41 scenes chrome scn-alpha1.json  load 11.07 io avg10=0.00 avg60=0.00 avg300=0.00
21:06:58 scenes firefox scn-client.json d11-menu-hover-escape load 11.43 io avg10=0.00 avg60=0.00 avg300=0.00
21:07:29 scenes firefox scn-plus.json  load 12.62 io avg10=0.00 avg60=0.00 avg300=0.00
21:07:55 export start load 12.30 io avg10=0.00 avg60=0.00 avg300=0.00
21:07:58 scenes firefox scn-alpha1.json  load 12.30 io avg10=0.00 avg60=0.00 avg300=0.00
21:13:01 export end load 6.44 io avg10=0.00 avg60=0.00 avg300=0.00
21:13:01 alpha7 load 6.44 io avg10=0.00 avg60=0.00 avg300=0.00
21:14:11 scenes firefox scn-client.json d1-sampler-envelope load 5.96 io avg10=0.81 avg60=0.35 avg300=0.08
21:14:46 scenes firefox scn-client.json d6-playlist-close load 5.20 io avg10=0.06 avg60=0.28 avg300=0.09
21:15:04 scenes firefox scn-client.json d1-sampler-envelope load 5.26 io avg10=0.23 avg60=0.31 avg300=0.11
21:15:36 scenes firefox scn-client.json d6-playlist-close load 3.38 io avg10=0.00 avg60=0.18 avg300=0.10
21:54:20 idle firefox prev 1 (quiet: load 1.55 io 0 after 0 s) load 1.55 io avg10=0.00 avg60=0.00 avg300=0.00
21:55:01 idle firefox new 1 (quiet: load 2.51 io 0 after 30 s) load 2.51 io avg10=0.00 avg60=0.03 avg300=0.00
21:55:11 idle firefox new 2 (quiet: load 2.74 io 0 after 0 s) load 2.74 io avg10=0.00 avg60=0.02 avg300=0.00
21:55:20 idle firefox prev 2 (quiet: load 3.22 io 0 after 0 s) load 3.22 io avg10=0.00 avg60=0.02 avg300=0.00
21:55:29 idle firefox prev 3 (quiet: load 3.18 io 0 after 0 s) load 3.18 io avg10=0.00 avg60=0.01 avg300=0.00
21:55:40 idle firefox new 3 (quiet: load 3.08 io 0 after 0 s) load 3.08 io avg10=0.00 avg60=0.01 avg300=0.00
21:55:49 idle firefox new 4 (quiet: load 3.07 io 0 after 0 s) load 3.07 io avg10=0.00 avg60=0.01 avg300=0.00
21:55:58 idle firefox prev 4 (quiet: load 3.28 io 0 after 0 s) load 3.28 io avg10=0.00 avg60=0.00 avg300=0.00
21:56:08 idle firefox prev 5 (quiet: load 3.63 io 0 after 0 s) load 3.63 io avg10=0.00 avg60=0.00 avg300=0.00
21:56:17 idle firefox new 5 (quiet: load 3.68 io 0 after 0 s) load 3.68 io avg10=0.00 avg60=0.00 avg300=0.00
21:56:27 idle firefox new 6 (quiet: load 5.29 io 0 after 0 s) load 5.29 io avg10=0.00 avg60=0.00 avg300=0.00
21:56:37 idle firefox prev 6 (quiet: load 5.91 io 0 after 0 s) load 5.91 io avg10=0.00 avg60=0.00 avg300=0.00
21:56:48 idle firefox x6 load 6.60 io avg10=0.00 avg60=0.00 avg300=0.00
21:56:48 E13 firefox idle done
21:58:27 built the page of the tree /home/ubuntu/work/signature-studio-develop as page-testpage load 6.28 io avg10=0.00 avg60=0.00 avg300=0.00
00:54:38 built the page of the tree /home/ubuntu/work/signature-studio-develop/lift/out-pmchk-e14c/tree as page-e14-P load 1.50 io avg10=0.00 avg60=0.00 avg300=0.00
00:54:38 built the page of the tree /home/ubuntu/work/signature-studio-develop/lift/out-pmchk-e14d/tree as page-e14-PV4 load 1.50 io avg10=0.00 avg60=0.00 avg300=0.00
01:47:48 pure start (PM go 01:4x; wine check: no Wine process after 0 s) load 1.00 io avg10=0.00
01:47:48 p3 chrome prev 1 (quiet: load 1 io 0 after 0 s) load 1.00 io avg10=0.00 avg60=0.00 avg300=0.00
01:49:37 p3 chrome prev 1 end load 1.42 io avg10=0.00 avg60=0.00 avg300=0.00
01:49:37 p3 chrome new 1 (quiet: load 1.42 io 0 after 0 s) load 1.42 io avg10=0.00 avg60=0.00 avg300=0.00
01:51:25 p3 chrome new 1 end load 1.67 io avg10=0.03 avg60=0.18 avg300=0.06
01:51:41 p3 chrome new 2 (quiet: load 1.37 io 0 after 15 s) load 1.37 io avg10=0.00 avg60=0.14 avg300=0.06
01:53:29 p3 chrome new 2 end load 1.89 io avg10=0.00 avg60=0.01 avg300=0.03
01:53:29 p3 chrome prev 2 (quiet: load 1.89 io 0 after 0 s) load 1.89 io avg10=0.00 avg60=0.01 avg300=0.03
01:55:18 p3 chrome prev 2 end load 1.91 io avg10=0.00 avg60=0.00 avg300=0.00
01:55:18 p3 chrome prev 3 (quiet: load 1.91 io 0 after 0 s) load 1.91 io avg10=0.00 avg60=0.00 avg300=0.00
01:57:06 p3 chrome prev 3 end load 1.67 io avg10=0.00 avg60=0.00 avg300=0.00
01:57:06 p3 chrome new 3 (quiet: load 1.67 io 0 after 0 s) load 1.67 io avg10=0.00 avg60=0.00 avg300=0.00
01:58:55 p3 chrome new 3 end load 1.15 io avg10=0.00 avg60=0.00 avg300=0.00
01:58:55 p3 chrome new 4 (quiet: load 1.15 io 0 after 0 s) load 1.15 io avg10=0.00 avg60=0.00 avg300=0.00
02:00:43 p3 chrome new 4 end load 1.73 io avg10=0.00 avg60=0.00 avg300=0.00
02:00:43 p3 chrome prev 4 (quiet: load 1.73 io 0 after 0 s) load 1.73 io avg10=0.00 avg60=0.00 avg300=0.00
02:02:31 p3 chrome prev 4 end load 3.39 io avg10=0.00 avg60=0.00 avg300=0.00
02:02:32 p3 chrome prev 5 (quiet: load 3.39 io 0 after 0 s) load 3.39 io avg10=0.00 avg60=0.00 avg300=0.00
02:04:20 p3 chrome prev 5 end load 2.51 io avg10=0.00 avg60=0.00 avg300=0.00
02:04:20 p3 chrome new 5 (quiet: load 2.51 io 0 after 0 s) load 2.51 io avg10=0.00 avg60=0.00 avg300=0.00
02:06:08 p3 chrome new 5 end load 1.51 io avg10=0.00 avg60=0.00 avg300=0.00
02:06:08 p3 chrome new 6 (quiet: load 1.51 io 0 after 0 s) load 1.51 io avg10=0.00 avg60=0.00 avg300=0.00
02:07:57 p3 chrome new 6 end load 2.35 io avg10=0.00 avg60=0.00 avg300=0.00
02:07:57 p3 chrome prev 6 (quiet: load 2.35 io 0 after 0 s) load 2.35 io avg10=0.00 avg60=0.00 avg300=0.00
02:09:45 p3 chrome prev 6 end load 2.57 io avg10=0.00 avg60=0.00 avg300=0.00
02:09:45 pure end chrome (all 12 runs of the mode finished) load 2.57 io avg10=0.00 avg60=0.00 avg300=0.00
02:09:45 pure chain exit rc=0 load 2.57 io avg10=0.00
```
