# RT-F G2 re-measurement — ci1, 2026-11-07

## 実際に実行した物

- 実エンジン(rtb=協調版 E13、rtf=RT-F 本番形 fl-engine-ci-d45afb0e)を実 Chrome 156 headless で実行。E13 run 19:42:04–19:46:05 (load 0.08→3.41, io avg10 0.00→0.00, swap in/out 0/0, MemAvail 17481 MB)、RT-F run 19:46:11–19:50:17 (load 3.34→6.33, io avg10 0.00→0.00, swap in/out 0/0, MemAvail 27236 MB)。他 Chrome 0、Wine 0、cargo/wasm-opt 0 (pre-run 0)。scenario は `tools/scn-lat-ff.json`。latDump JSON は `out/lat-chrome-prev-1/` (E13) と `out/lat-chrome-new-1/` (RT-F)。gate 条件 (load ≤8、I/O avg10=0、swap 0/0、MemAvail ≥8 GB) を満たす run のみ counted。

## 結果 (post-play window の値のみ)

Primary pointer-move queue-insertion (kind=1, Mixer drag, `tInject-tEvent`):
- E13: n=81, median 5.39 ms (p90 10.50, p99 19.60)
- RT-F: n=81, median 5.46 ms (p90 11.38, p99 21.53)
- diff +0.06 ms → この指標だけ PASS。

ユーザー可視 T (first answered frame − `tPage`):
- Idle File click (kind=2): E13 median 5.50, RT-F 7.20, diff **+1.70 ms** (pass, p90 14.21 vs 19.25)
- Idle key down (Esc/Space, kind=5): E13 1.61, RT-F 1.83, diff **+0.22 ms** (pass)
- Drag pointermove answered T: E13 2.21, RT-F 2.41, diff **+0.21 ms** (pass)
- Playback File click (kind=2): E13 16.46, RT-F 39.83, diff **+23.37 ms** (fail、p90 23.12 vs 49.97)
- Playback key down (Esc/Space, kind=5): E13 3.85, RT-F 14.55, diff **+10.71 ms** (fail、p90 12.10 vs 25.81)

=> G2 overall is NOT PASS. RT-F は +23.37 / +10.71 ms で、RT-D3 (+28.36 / +14.52) よりは改善しているが、まだ +10 ms を超えている。drag 3/4 修正と WM_CONTEXTMENU 修正が G2 の playback T を全体として改善する方向には働いているものの、gate には届いていない。劣化は queue 投入ではなく engine 側の処理・frame 提供の待ちに寄っている (queue insertion は +0.06 ms と同等)。

Full table: `g2-table.md`。Percentiles は lat-ab.py と同じ order statistic。
