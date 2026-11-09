# RT-F GIL song (300 s, FLE_DIAG_ATOM=1 FLE_DIAG_GIL=1) — 2026-11-07

## 実際に実行した物

- 実 Chrome 156 (page-rt1) で RT-F bundle (fl-engine-ci-d45afb0e) を manifest copy with FLE_DIAG_ATOM=1 FLE_DIAG_GIL=1 (assets-rtf-gil2) で実行。scenario scn-o19c-gil.json (o19c-song-shown)。
- run o19-songfull-gil2-shown-1-rtf-gil2: 19:30:47–19:36:49 start (load 0.30 io avg10 0.00 after 72s, MemAvail 20544 MB, swap 0/0), end load 4.23. seconds 360, first idle 7137 ms, ok/no stop.
- Also earlier run without GIL diag (assets-rtf-gil, FLE_DIAG_GIL=1 only) at 19:21:31–19:27:34 for comparison (o19-songfull-gil-shown-1-rtf-gil).

## Results

- O19C window (0/30): both runs have underrun 0 quanta, lag cuts 0, engine gap 0 frames, present/s 36.0 and 36.7, silent runs 2, worker 1.73 cores N=8.
- Atom diagnostics (ATOM=1): fl-diag: atom lines 400 (GlobalAddAtomW / GlobalFindAtomW).
- GIL diagnostics (GIL=1): fl-diag: gil lines 0. No gil wait/histogram line appeared in the 954-line engine-log despite env flag. The GIL histogram (wait/held, top-held) did NOT appear — either the song run did not enter the GIL-instrumented path or the diag counter line only emits under a spike condition.
  - Lines matching "gil" (case-sensitive) only include g9-diag indirect calls (6 lines at 65536, 131072, 262144, 524288, 1048576 over song).
- Without GIL histogram, we cannot attribute the playback File T +23.58 ms / key T +13.98 ms from the FLE_DIAG_GIL data alone. The GIL didn't show contended waits in this run; the lag is likely elsewhere (API holding duration or Worker scheduling).

