# 0277: verification of `fl-engine-ci-0363b050` (Worker ?, page of git e14-PV7) against the previous production bundle `fl-engine-ci-0363b050`

Real engine in real browsers (Chrome / Firefox, headless; Firefox on Xvfb :77 with WebGL). No stand-in, no replay. The bundle comes from R2 (read only); every manifest sha256 is checked by `fetch`.

## Pass criteria

| criterion | value | pass |
|---|---|---|
| Firefox has WebGL (preflight) | not run | NO: Firefox values are 2D values |
| No stop (11 scenes + more, both browsers, play, export; d7 is the intended exit) | stops: none; not ok: none; no result at all (the harness did not start): none | NO or not complete |
| Export WAV equals the reference | 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` (expected 22863828 B, `381cdb5acf891a16…`) | yes |
| First idle ≤ previous + 500 ms (chrome, WebGL runs) | not run | not run |
| First idle ≤ previous + 500 ms (firefox, WebGL runs) | not run | not run |
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

| browser | mode | previous median (n) | new median (n) | difference |
|---|---|---|---|---|

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
```
