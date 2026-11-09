# RT-A G2/G3/G6 (RT-D3 bundle fl-engine-ci-83b8ed8a) — ci1, 2026-11-07

## 実際に実行した物

- G2/G3 Chrome runs: 実エンジン(rtb=協調から取り込み、rtd3=RT-D3 fl-engine-ci-83b8ed8a)を実 Chrome 156(headless/CDP)で実行。E13 run 16:38:33–16:42:51 (load 5.95→4.17, I/O avg10 0.00→1.56, swap in/out 0/0, MemAvailable 19271 MB, 他 Chrome 0, Wine 0。mt3 の cargo/wasm-opt が途中で並行したが開始前後 gate は上記)。RT-D3 run 16:45:37–16:49:59 (load 1.82→4.13, I/O avg10 0.00→0.00, swap in/out 0/0, MemAvailable 24550 MB, 他 Chrome 0, Wine 0。wasm-opt が並行は開始前後記録済み)。scenario は既存 `tools/scn-lat-ff.json`。latDump JSON は `out/lat-chrome-prev-1/` と `out/lat-chrome-new-1/`。
- G3 present runs: G2 の既存 O19 round1/round2/C テーブル(E13/RT-A) + D3 の新規 60 s shown run (`out/g3-present-rtd3-1/`, 16:50:13–16:52:17, load 3.43→3.58, I/O avg10 0.00→0.00, swap 0/0, MemAvailable 22982 MB, Chrome/Wine 0, cargo 0 at pre-run). D3 presents are deskStats at play-start/end.
- G6 Firefox run: Firefox 158 (Mozilla Firefox 158.0, WebDriver) で RT-D3 (page-rt1, port 5119)を実行。最終 run `out/g6-firefox-rtd3-3/` は 17:03:39–17:08:16 (ok no stop, load 1.7→2.39 in report load fields [note pre-run step log separately], I/O avg10 0.36 at start because of concurrent cargo [using run script], process-free at launch, MemAvailable 21422 MB, swap in/out 0/0, cargo/wasm-opt 1/2 present at start per its own log) and scenarios (About -> openProject with original FLP -> Japanese project file list -> playback), scn-g6-d3 after correcting clickIn dyFromBottom and About click y=95; first Firefox run (g6-firefox-rtd3-1) failed at clickIn because lat driver lacked dyFromBottom and reported separately.

## G2 — queue insertion vs T (この run の値)

Primary pointer-move queue-insertion (kind=1, Mixer drag, `tInject-tEvent`):
- E13: n=81, median 5.14, p90 10.22, p99 20.46, max 20.46 ms
- RT-D3: n=81, median 5.13, p90 12.48, p99 27.60, max 27.60 ms
- diff median -0.01 ms, <= +10 ms: PASS for the pointer-move queue metric.

But "input T" as user-visible/answered input (first frame - tPage; the standard gate metric) is NOT within +10 ms under playback:
- Idle File click (kind=2): E13 median 6.29 ms, D3 8.23 ms, diff +1.95 ms (pass).
- Idle key down (Esc/Space): E13 1.91, D3 2.00, diff +0.09 ms (pass).
- Drag pointermove answered T: E13 2.66, D3 2.83, diff +0.17 ms (pass).
- Playback File click (kind=2): E13 19.57, D3 47.93, diff +28.36 ms; p90 24.87 vs 57.09.
- Playback key down (Esc/Space): E13 5.15, D3 19.67, diff +14.52 ms; p90 13.70 vs 32.91.
=> G2 overall does NOT pass the +10 ms gate if T is the answered-input T (the usual RT gate). If the gate is strictly mouse queue insertion only, it passes (-0.01 ms). The report records both.

Full table: `g2-table.md` in this directory. Percentiles computed with the same order-statistic as lat-ab.py (`sorted[max(0, ceil(p/100*n)-1)]`).

## G3 — present rate

Existing O19 (60 s window; E13 vs RT-A) and C (full-song mean):
- Round1: shown E13 15.2 / RT-A 31.9; hidden 20.6 / 39.8; allhidden 20.5 / 45.7 presents/s.
- Round2: shown 16.4 / 30.2; hidden 17.1 / 38.8; allhidden 19.1 / 43.2.
- Full-song C mean: shown E13 14.4 / RT-A 27.8; allhidden E13 19.3 / RT-A 41.4.
- New D3 (60 s shown, deskStats): presents 473 -> 2368 over 60.01 s = 31.58/s, above E13 shown (~14.4-16.4/s); firstIdle 7.924 s.
=> G3: RT-D3 present rate is not below cooperative E13; shown condition value 31.58/s.

## G6 — Firefox

- Firefox 158 started, RT-D3 reached engine-idle, first idle 4.621-5.037 s (best run used for report: 4.621 s in report; second run 5.037 s), no op errors, engine still running.
- About: F10 opened Settings; after correcting click to About tab, the About panel is visible (FL STUDIO 2026, trial mode text). However the About layered logo window is 3x3 at about-8s (TLightweightLayeredControl @762,263 3x3), and the screenshot does not show the staff-roll About text; About is a partial result (BigFruit/AboutTxt are known missing seed resources, seen consistently 3x3 logo).
- Japanese: the dedicated ja-names run (`g6-firefox-rtd3-ja`) rendered Japanese text as tofu boxes in the Channel rack first row and Playlist header. The engine log shows the VFS seed only contains `arial.ttf`, `tahoma.ttf`, `lucon.ttf`; there is no `msgothic.ttc` line, so Japanese font is NOT resolved by this RT-D3 seed. Playback advanced (audio-p3 detail playingSeconds 77.4, nonSilentQuanta 5512, underrun quanta 0).
=> G6: startup / About tab / project open / playback work, but About seed assets (3x3 logo, no staff-roll) and Japanese font (tofu; no msgothic) are unresolved. This is a partial result, not a full G6 pass.

## Artifacts

This directory: `g2-table.md`, `g2-analyze.py`, `MEASUREMENT-CONDITIONS.md`, `scn-lat-ff.json`, `scn-g3-present-d3.json`, `scn-g6-d3.json`, `sdriveg2-lat.mjs`, `sdriveg2-export.mjs`, plus copies of `out/lat-chrome-prev-1/`, `out/lat-chrome-new-1/`, `out/g3-present-rtd3-1/`, `out/g6-firefox-rtd3-3/` and the O19 tables.
