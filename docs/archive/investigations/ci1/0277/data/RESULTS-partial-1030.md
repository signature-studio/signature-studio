# 0277: verification of `fl-engine-ci-d9c160aa` (Worker 51f102d8, page of git 0b228624) against the previous production bundle `fl-engine-ci-ff5ee429`

Real engine in real browsers (Chrome / Firefox, headless; Firefox on Xvfb :77 with WebGL). No stand-in, no replay. The bundle comes from R2 (read only); every manifest sha256 is checked by `fetch`.

## Pass criteria

| criterion | value | pass |
|---|---|---|
| Firefox has WebGL (preflight) | preflight: webglRenderer llvmpipe, or similar desk.mode gl first idle 4724 | yes |
| No stop (11 scenes + more, both browsers, play, export; d7 is the intended exit) | stops: none; not ok: none | yes |
| Export WAV equals the reference | not run / no file | NO |
| First idle ≤ previous + 500 ms (chrome, WebGL runs) | all runs: new median 6514 ms, previous 7946 ms, difference -1432 ms; quiet runs only (load ≤12, I/O avg10 <1; n new 2 / prev 2): new 6132, previous 6574, difference -442 ms | yes |
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
| chrome | gl | 7946 (6) | 6514 (6) | -1432 |
| firefox | gl | 5276 (6) | 4900 (6) | -376 |

## Export (Chrome, the reference project, Ctrl+R)

not run

## Playback (the reference project zip, ~60 s)

not run

## Mixer header drag: Worker CPU per move (previous and new alternating)

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
```
