# RT-F (fl-engine-ci-d45afb0e) G2/G6 — ci1, 2026-11-07

## 実際に実行した物

- G2 Chrome runs: 実エンジン(rtb=協調版 E13、rtf=RT-F fl-engine-ci-d45afb0e)を実 Chrome 156 (headless/CDP) で実行。E13 run 18:34:23–18:38:26 (load 0.65→2.63, I/O avg10 0.00→0.00, swap in/out 0/0, MemAvailable 27168 MB, gate pass)。
  RT-F run 18:39:35–18:43:40 (load 1.13→3.85, I/O avg10 0.00→0.00, swap in/out 0/0, MemAvailable 37433 MB, gate pass)。他 Chrome 0、Wine 0、cargo/wasm-opt は 0 (18:44 pre-run で 0)。
  scenario は `tools/scn-lat-ff.json`。latDump JSON は `out/lat-chrome-prev-1/` (E13) と `out/lat-chrome-new-1/` (RT-F)。
- G6 Firefox runs: Firefox 158 (Mozilla Firefox 158.0、WebDriver) で RT-F (page-rt1) を実行。
  - `out/g6-firefox-rtf/` (18:44:07–18:48:43、About + 20s playback)。MemAvailable 27928 MB、cargo/wasm-opt 0、swap in/out 0/0、I/O avg10 0.00 at start。
  - `out/g6-firefox-rtf-ja/` は assets-rtf (ja variant なし) で original FLP を開いたもの。参考として残置。
  - `out/g6-firefox-rtf-ja2/` (18:53:15–18:56:52、ja-names FLP variant assets-rtf-ja、port 5123、ok/no stop)。MemAvailable 23312 MB、cargo/wasm-opt 0。
- gate 条件: load ≤8、I/O avg10 = 0、swap in/out = 0、MemAvailable ≥8 GB、他 Chrome/Wine 0 を開始前に記録し、条件を満たす run のみ counted。

## G2 — queue insertion vs T (この run の値)

Primary pointer-move queue-insertion (kind=1, Mixer drag, `tInject-tEvent`):
- E13: n=81, median 5.44, p90 11.31, p99 19.12, max 19.12 ms
- RT-F: n=81, median 5.46, p90 15.39, p99 26.14, max 26.14 ms
- diff median +0.02 ms → この指標のみ PASS (simto RT-D3 の +0.01–0.02 ms)。

ユーザー可視 T (first answered frame − `tPage`):
- Idle File click (kind=2): E13 median 5.47, RT-F 7.17, diff **+1.70 ms** (pass, p90 6.68 vs 7.78)
- Idle key down (Esc/Space, kind=5): E13 1.65, RT-F 1.80, diff **+0.15 ms** (pass)
- Drag pointermove answered T: E13 2.55, RT-F 2.27, diff **−0.29 ms** (pass)
- Playback File click (kind=2): E13 16.93, RT-F 40.50, diff **+23.58 ms** (fail、p90 25.05 vs 50.95、p99 43.94 vs 54.37)
- Playback key down (Esc/Space, kind=5): E13 3.97, RT-F 17.95, diff **+13.98 ms** (fail、p90 10.44 vs 27.07)

=> G2 overall does NOT pass the +10 ms gate. RT-D3 では playback File +28.36 / key +14.52 だった。RT-F は playback F ile +23.58 / key +13.98 と若干改善しているが、依然として gate 超過。pointermove の queue insertion 自体は同等 (+0.02 ms) なので、劣化は queue 投入ではなく「再生中の engine 側の処理・frame 提供の待ち」に寄っているとみられる (lift の実装に依存する所)。

Full table: `g2-table.md` in this directory. Percentiles computed with the same order-statistic as lat-ab.py (`sorted[max(0, ceil(p/100*n)-1)]`)。

## G6 — Firefox

- Firefox 158 started and RT-F reached engine-idle; first idle 4,075 ms (About run), 4,050 ms (ja2 run), no op errors; engine still running.
- About: F10 opened Settings; after click correction, About tab shows FL STUDIO 2026 trial mode. The About logo is still 3x3 (TLightweightLayeredControl at 757,259) and no staff-roll About text is visible, so BigFruit/AboutTxt seed assets remain missing (partial result)。
- Japanese: ja-names FLP now renders Japanese text as tofu boxes in both Channel rack first row (boxes in `pic-ja-main-8s.png` and `crop-ja-channel-zoom.png`) and Playlist header. Engine log has no msgothic.ttc seed row and only arial/tahoma/lucon among Windows Fonts. So Japanese font is NOT resolved by this RT-F seed (unchanged from RT-D3 / RT-E)。
- Playback: Firefox RT-F advanced (audio-p3 detail playingSeconds 76.9, nonSilentQuanta 5508, underrun quanta 0 in the 20 s post-click segment; no stop)。
=> G6 partial: startup/About tab/project+playback work, but About seed assets and Japanese font are not resolved. Not a full G6 pass.

## Artifacts

This directory: `g2-table.md`, `g2-analyze.py`, `MEASUREMENT-CONDITIONS.md`, `scn-lat-ff.json`, `scn-g6-d3.json`, `scn-g6-ja-d3.json`, plus copies of `out/lat-chrome-prev-1/`, `out/lat-chrome-new-1/`, `out/g6-firefox-rtf/`, `out/g6-firefox-rtf-ja/`, `out/g6-firefox-rtf-ja2/` and `steps.log`。
