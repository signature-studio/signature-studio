# ci1 → PM 決定 0277（E11）: E11 の本番 bundle `fl-engine-ci-dc5fae2b`（Worker 00769751、ページ = 7373cd55）の実ブラウザ確認 — 停止なし、Export 一致、File ×100、first idle（E10 比）、alpha7、純再生 pure（E10 対 E11、壁時計と音楽位置）

## 先頭の 3 点（実エンジン・実ブラウザ Chrome 156 / Firefox 158、この機械・headless・模擬入力。**依頼者の機械は 0 件**。代役・再生・推測なし。Firefox の値は全て WebGL あり（`DISPLAY=:77`、webglRenderer = llvmpipe、desk.mode = gl）。各値に load と I/O 圧 avg10 を併記し、first idle・pure は『開始時 load ≤8 かつ I/O 圧 0』の run だけ数え、他の Chrome が 2 本以上いた run は『参考（重なり）』に分けた）
1. **測ったこと**: R2 から読み取りで取得した E11 `fl-engine-ci-dc5fae2b`（Worker 00769751、ページ = 7373cd55）の manifest の sha256 照合（wasm `49bbf63df894fbe1…` 143,086,598 B 他、全一致。比較の相手 E10 `fl-engine-ci-432c65ec` の wasm `4d0019aaa3ee7a3b…` 142,776,280 B も一致）。15 シーン × 2 ブラウザ、File ×100、first idle（Chrome 10 組、Firefox 6 組、交互）、Export、alpha7、純再生 pure（E10 対 E11、Chrome 6 組、壁時計と音楽位置〔仮想時計〕で揃えた窓）。
2. **結論**:
   - 停止なし（30/30 ok）、Export の WAV が参照と一致（22,863,828 B、sha256 `381cdb5a…f979`）、File ×100 は Chrome・Firefox とも 100/100。alpha7 の画は E8・E9 と 65 画素（時計の欄）だけ違う。
   - **first idle は E10 より遅い向き**: Chrome（10 組）中央値 E11 4,866 / E10 4,682 ms（**+184 ms**、静かな run だけでも +171、組ごとの差の中央値 +170、10 組中 8 組が正）。9 組目だけ両方が遅い run（E10 8,102 / E11 6,368 ms）で、平均の差は +16 ± 204（SE）。9 組目を除くと +211 ± 68（SE、n=9）。**基準 +100 ms を中央値で超える**。Firefox（6 組）は中央値 +104 ms、平均の差 +236 ± 136（SE）。wasm は E10 より 310 KB 大きい。
   - **純再生（Chrome、数えた run は E10 6・E11 5（E11 の 2 組目は開始時の I/O 圧 0.18 で外れた））**: underrun は 60 s で E10 12.5・E11 9 quanta（p 0.25）、30〜60 s は 0。仮想 s/実 s 0.679 → 0.716（p 0.019）、Worker CPU 50.5 → 50.0 s、presents/s 14.8 → 15.9。**音楽位置で揃えると**、音楽 0〜30 s の 10 s bin を通る壁時計の時間が E11 の方が 0.3〜1.5 s 短い（p 0.058 / 0.009 / 0.026）。30〜40 s の bin は +0.3 s（逆向き）。underrun は音楽 10〜40 s で両方 0。つまり **E11 は音楽 40 s までは E10 より数 % 速いが、underrun の差は出ない**。
3. **未確認**: オーナーの機械（0 件）。E10・E11 とも 60 s で音楽の約 40 s にしか届かないので、**重い区間（音楽 40 s 以降）は未測定**（5 分再生が要る。gpu1 の実スレッドの窓の後）。lat（入力の true T）は E11 では取っていない。lid 8 と最小化→復元は E9 で確認済みの経路で、E11 では再実行していない。

## 1 判定基準（E11 対 E10、run の load と I/O 圧は各表）
| criterion | value | pass |
|---|---|---|
| Firefox has WebGL (preflight) | preflight: webglRenderer llvmpipe, or similar desk.mode gl first idle 4428 | yes |
| No stop (11 scenes + more, both browsers, play, export; d7 is the intended exit) | stops: none; not ok: none; no result at all (the harness did not start): none | yes |
| Export WAV equals the reference | 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` (expected 22863828 B, `381cdb5acf891a16…`) | yes |
| First idle ≤ previous + 500 ms (chrome, WebGL runs) | all runs: new median 4866 ms, previous 4682 ms, difference +184 ms; means 5076 ± 156 (SE, n=10) / 5060 ± 346 (n=10), difference of the means +16 ± 380 (SE); quiet runs only (load ≤12, I/O avg10 <1; n new 10 / prev 9): new 4866, previous 4695, difference +171 ms | yes |
| First idle ≤ previous + 500 ms (firefox, WebGL runs) | all runs: new median 4485 ms, previous 4381 ms, difference +104 ms; means 4593 ± 120 (SE, n=6) / 4357 ± 64 (n=6), difference of the means +236 ± 136 (SE); quiet runs only (load ≤12, I/O avg10 <1; n new 4 / prev 4): new 4485, previous 4325, difference +160 ms | yes |
| File menu ×100 (chrome) | 100/100 | yes |
| File menu ×100 (firefox) | 100/100 | yes |

## 2 15 シーン（Chrome・Firefox、各行の load と I/O 圧。2 ジョブ並行のため first idle は参考値）
| browser | scene | result | first idle ms | File ×100 | load, I/O pressure (start→end) |
|---|---|---|---|---|---|
| chrome | d1-sampler-envelope | ok | 4699 | - | load 3.0→2.9, io 0.0→0.1 |
| chrome | d2-browser-folder | ok | 4144 | - | load 2.9→2.9, io 0.1→0.0 |
| chrome | d3-settings-all-tabs | ok | 4986 | - | load 2.9→3.4, io 0.0→0.0 |
| chrome | d4-playlist-middle-drag | ok | 4287 | - | load 3.4→3.6, io 0.0→0.0 |
| chrome | d5-mixer-front | ok | 4679 | - | load 3.6→4.6, io 0.0→0.0 |
| chrome | d6-playlist-close | ok | 4196 | - | load 4.6→4.6, io 0.0→0.0 |
| chrome | d7-main-close | ok (intended exit) | 4475 | - | load 4.6→5.4, io 0.0→1.0 |
| chrome | d8-file-menu-100 | ok | 4619 | 100/100 | load 5.4→10.3, io 1.0→0.0 |
| chrome | d9-keys | ok | 3679 | - | load 10.3→9.7, io 0.0→0.0 |
| chrome | d10-pianoroll-f12 | ok | 4569 | - | load 9.7→9.9, io 0.0→0.1 |
| chrome | d11-menu-hover-escape | ok | 5281 | - | load 9.9→10.1, io 0.1→0.0 |
| chrome | add-menu-simple-fade | ok | 5961 | - | load 15.3→15.4, io 0.1→0.0 |
| chrome | picker-cost | ok | 5309 | - | load 15.5→16.1, io 0.0→0.1 |
| chrome | picker-f8-fade | ok | 4613 | - | load 15.4→15.5, io 0.0→0.0 |
| chrome | plus-more-plugins | ok | 5855 | - | load 10.1→14.1, io 0.0→0.1 |
| firefox | d1-sampler-envelope | ok | 4259 | - | load 3.4→2.8, io 0.4→0.0 |
| firefox | d2-browser-folder | ok | 4343 | - | load 2.8→3.2, io 0.0→0.0 |
| firefox | d3-settings-all-tabs | ok | 4275 | - | load 3.0→3.6, io 0.0→0.0 |
| firefox | d4-playlist-middle-drag | ok | 4690 | - | load 3.6→4.8, io 0.0→0.0 |
| firefox | d5-mixer-front | ok | 4316 | - | load 4.6→4.7, io 0.0→0.0 |
| firefox | d6-playlist-close | ok | 4212 | - | load 4.7→6.1, io 0.0→2.0 |
| firefox | d7-main-close | ok (intended exit) | 4629 | - | load 6.1→6.2, io 2.4→3.0 |
| firefox | d8-file-menu-100 | ok | 4870 | 100/100 | load 6.2→9.9, io 3.3→0.1 |
| firefox | d9-keys | ok | 5493 | - | load 9.9→13.2, io 0.1→0.0 |
| firefox | d10-pianoroll-f12 | ok | 5508 | - | load 13.9→15.3, io 0.2→0.0 |
| firefox | d11-menu-hover-escape | ok | 5148 | - | load 15.3→16.6, io 0.0→0.0 |
| firefox | add-menu-simple-fade | ok | 4724 | - | load 14.6→14.4, io 0.0→0.0 |
| firefox | picker-cost | ok | 4304 | - | load 12.3→10.3, io 0.0→0.0 |
| firefox | picker-f8-fade | ok | 4559 | - | load 14.4→12.3, io 0.0→0.0 |
| firefox | plus-more-plugins | ok | 5532 | - | load 16.6→14.8, io 0.0→0.0 |


The scenes ran as two parallel jobs (Chrome and Firefox), so the first idle values of this table are reference values (the load is next to each).

## 3 first idle（単独 run・冷間・交互。prev = E10、new = E11。基準 +100 ms）
| browser | bundle | run | first idle ms | load, I/O pressure at the start | at the end | quiet (load ≤12, I/O <1) | webglRenderer | desk.mode |
|---|---|---|---|---|---|---|---|---|
| chrome | prev | 1 | 5304 | 6.92 / io 0.00 | 7.07 / io 0.10 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 1 | 5285 | 5.42 / io 0.00 | 4.96 / io 0.24 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 2 | 5025 | 3.12 / io 0.00 | 3.18 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 2 | 4822 | 3.18 / io 0.00 | 3.22 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 3 | 4695 | 3.22 / io 0.00 | 4.66 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 3 | 4840 | 4.66 / io 0.00 | 4.56 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 4 | 4837 | 4.56 / io 0.00 | 4.54 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 4 | 4545 | 4.54 / io 0.00 | 4.37 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 5 | 4478 | 4.37 / io 0.00 | 4.23 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 5 | 4686 | 4.23 / io 0.00 | 4.18 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 6 | 4758 | 4.18 / io 0.00 | 3.92 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 6 | 4658 | 3.92 / io 0.00 | 4.17 / io 2.35 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 7 | 4790 | 8.00 / io 0.00 | 7.84 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 7 | 4867 | 7.84 / io 0.00 | 7.85 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 8 | 4865 | 7.85 / io 0.00 | 7.63 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 8 | 4670 | 7.63 / io 0.00 | 7.44 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 9 | 8102 | 7.44 / io 0.00 | 7.29 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 9 | 6368 | 7.29 / io 0.00 | 7.60 / io 0.31 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 10 | 5231 | 5.70 / io 0.00 | 6.13 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 10 | 4536 | 6.13 / io 0.00 | 5.95 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| firefox | prev | 1 | 4394 | 6.49 / io 0.00 | 7.15 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 1 | 4299 | 7.15 / io 0.00 | 7.35 / io 0.09 | yes | llvmpipe, or similar | gl |
| firefox | new | 2 | 4723 | 6.87 / io 0.00 | 6.62 / io 0.26 | yes | llvmpipe, or similar | gl |
| firefox | prev | 2 | 4388 | 7.82 / io 0.00 | 8.08 / io 0.10 | no (reference) | llvmpipe, or similar | gl |
| firefox | prev | 3 | 4276 | 7.94 / io 0.00 | 7.94 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 3 | 4519 | 7.94 / io 0.00 | 7.78 / io 0.08 | yes | llvmpipe, or similar | gl |
| firefox | new | 4 | 4451 | 7.76 / io 0.00 | 7.89 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 4 | 4594 | 7.89 / io 0.00 | 8.28 / io 0.14 | no (reference) | llvmpipe, or similar | gl |
| firefox | prev | 5 | 4374 | 7.79 / io 0.00 | 7.96 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 5 | 4445 | 7.96 / io 0.00 | 8.48 / io 0.00 | no (reference) | llvmpipe, or similar | gl |
| firefox | new | 6 | 5120 | 7.77 / io 0.00 | 8.19 / io 0.00 | no (reference) | llvmpipe, or similar | gl |
| firefox | prev | 6 | 4117 | 7.84 / io 0.00 | 7.39 / io 0.00 | yes | llvmpipe, or similar | gl |

| browser | mode | previous median (n) | new median (n) | difference |
|---|---|---|---|---|
| chrome | gl | 4682 (10) | 4866 (10) | 184 |
| firefox | gl | 4381 (6) | 4485 (6) | 104 |

## 4 Export
result ok; load 15.5→13.0, io 0.0→0.0; seconds 309
- WAV `serina1-serum2.wav`: 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`; expected 22863828 B / `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`: MATCH

## 5 純再生 pure（E10 対 E11、参照 zip を開き Space で 60 s、10 s 窓、2 s ごとの snapshot、交互）
### P3 decider, chrome: old (previous) vs new, windows of 10 s from the play mark; counted = start load ≤8 and I/O pressure avg10 = 0

| run | bundle | counted | other Chrome browsers at the play mark / at +60 s / max | waited for the quiet window (s) | load start → end | I/O avg10 start → end | Worker CPU s (60 s) = CPU/wall x 60 | main CPU s | underrun quanta 0-60 s | episodes 0-60 s | engine gap frames 0-60 s | **underrun 30-60 s** | write rate mean 10-60 s | consumed rate mean 10-60 s | UI idles/s (10-60 s) | presents/s | lagCuts/s | virtual s per real s | stopped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | prev | yes | 1 / 1 / 1 | 105 | 7.8 → 8.6 | 0.00 → 0.00 | 49.7 | 0.0 | 18 | 7 | 1702 | **5** | 0.875 | 0.996 | 260.4 | 13.2 | 17.4 | 0.637 | False |
| 2 | prev | yes | 1 / 0 / 1 | 0 | 5.5 → 4.7 | 0.00 → 0.00 | 50.4 | 0.0 | 4 | 2 | 340 | **0** | 0.959 | 0.994 | 243.6 | 14.9 | 18.7 | 0.689 | False |
| 3 | prev | yes | 0 / 0 / 0 | 0 | 4.7 → 3.8 | 0.00 → 0.00 | 50.7 | 0.0 | 14 | 3 | 1534 | **0** | 0.951 | 1.000 | 253.9 | 14.9 | 18.7 | 0.684 | False |
| 4 | prev | yes | 0 / 0 / 0 | 0 | 4.9 → 5.1 | 0.00 → 0.00 | 50.6 | 0.0 | 8 | 2 | 852 | **0** | 0.951 | 0.998 | 255.7 | 15.2 | 18.8 | 0.692 | False |
| 5 | prev | yes | 0 / 0 / 0 | 0 | 5.1 → 5.6 | 0.00 → 0.00 | 50.4 | 0.0 | 37 | 4 | 4392 | **26** | 0.923 | 1.000 | 247.3 | 14.8 | 19.2 | 0.664 | False |
| 6 | prev | yes | 0 / 0 / 0 | 0 | 7.4 → 6.4 | 0.00 → 0.00 | 50.5 | 0.0 | 11 | 2 | 1236 | **0** | 0.933 | 1.000 | 257.8 | 14.4 | 18.3 | 0.674 | False |
| 1 | new | yes | 1 / 1 / 1 | 30 | 7.8 → 8.3 | 0.00 → 0.00 | 50.1 | 0.0 | 12 | 4 | 1192 | **6** | 0.929 | 1.000 | 270.3 | 14.5 | 18.6 | 0.676 | False |
| 2 | new | NO (not counted: load / I/O) | 1 / 1 / 1 | 5 | 7.9 → 5.5 | 0.18 → 0.00 | 49.6 | 0.0 | 17 | 3 | 1918 | **0** | 0.973 | 0.998 | 309.3 | 15.9 | 19.0 | 0.734 | False |
| 3 | new | yes | 0 / 0 / 0 | 0 | 3.8 → 4.4 | 0.00 → 0.00 | 50.1 | 0.0 | 7 | 3 | 638 | **0** | 0.964 | 1.000 | 299.4 | 16.0 | 18.6 | 0.719 | False |
| 4 | new | yes | 0 / 0 / 0 | 0 | 4.4 → 4.9 | 0.00 → 0.00 | 50.0 | 0.0 | 4 | 2 | 340 | **0** | 0.964 | 0.998 | 289.4 | 16.1 | 19.9 | 0.716 | False |
| 5 | new | yes | 0 / 0 / 0 | 0 | 5.6 → 8.0 | 0.00 → 0.00 | 50.0 | 0.0 | 10 | 3 | 1022 | **0** | 0.955 | 0.996 | 297.7 | 15.6 | 17.8 | 0.708 | False |
| 6 | new | yes | 0 / 0 / 0 | 5 | 7.8 → 7.4 | 0.00 → 0.00 | 50.0 | 0.0 | 9 | 3 | 894 | **0** | 0.977 | 1.000 | 297.1 | 15.9 | 18.6 | 0.726 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 10 | 2 | 1065 | 0.98 | 1.00 | 1126.9 | 14.6 | 7.7 | 0.77 |
| 0-10 s | new | 5 | 7 | 3 | 638 | 0.98 | 1.00 | 1146.7 | 15.1 | 8.0 | 0.78 |
| 10-20 s | prev | 6 | 0 | 0 | 0 | 0.97 | 1.00 | 250.0 | 15.6 | 21.3 | 0.66 |
| 10-20 s | new | 5 | 0 | 0 | 0 | 0.99 | 1.00 | 327.1 | 17.2 | 20.9 | 0.72 |
| 20-30 s | prev | 6 | 0 | 0 | 0 | 1.00 | 1.00 | 287.8 | 16.4 | 18.4 | 0.70 |
| 20-30 s | new | 5 | 0 | 0 | 0 | 0.99 | 1.00 | 417.6 | 17.9 | 17.6 | 0.76 |
| 30-40 s | prev | 6 | 0 | 0 | 0 | 1.00 | 1.00 | 197.0 | 15.5 | 21.6 | 0.60 |
| 30-40 s | new | 5 | 0 | 0 | 0 | 1.00 | 1.00 | 257.2 | 16.7 | 22.1 | 0.64 |
| 40-50 s | prev | 6 | 0 | 0 | 0 | 0.93 | 1.00 | 190.2 | 13.3 | 17.6 | 0.65 |
| 40-50 s | new | 5 | 0 | 0 | 0 | 0.94 | 1.00 | 216.4 | 14.6 | 19.9 | 0.68 |
| 50-60 s | prev | 6 | 0 | 0 | 0 | 0.81 | 1.00 | 341.6 | 12.8 | 14.2 | 0.76 |
| 50-60 s | new | 5 | 0 | 0 | 0 | 0.88 | 1.00 | 278.4 | 12.6 | 13.5 | 0.78 |

Music-matched windows (the same music in both bundles): the music position is the virtual clock since the play mark (gpu1 matched-music.py); the bins are complete 10 s bins of music that each run reached within its 60 s of wall time. wall = the wall time the bin took; distinct frames = Unlock coverage per wall second (1.0 = real time); Unlocks / wall s; underrun quanta and episodes of the page and the engine gap frames over the wall interval of the bin (the 10 s snapshots and the 1 Hz engine reports, linearly interpolated). Median over the counted runs.

| music bin | bundle | n | wall time of the bin (s) | distinct frames per wall s | Unlocks per wall s | underrun quanta | episodes | engine gap frames |
|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 14.0 | 0.950 | 10.55 | 10 | 2 | 1065 |
| 0-10 s | new | 5 | 13.6 | 0.967 | 10.55 | 7 | 3 | 638 |
| 10-20 s | prev | 6 | 13.5 | 0.840 | 10.55 | 0 | 0 | 0 |
| 10-20 s | new | 5 | 12.6 | 0.879 | 10.74 | 0 | 0 | 0 |
| 20-30 s | prev | 6 | 17.0 | 0.809 | 10.73 | 0 | 0 | 0 |
| 20-30 s | new | 5 | 15.5 | 0.852 | 10.78 | 0 | 0 | 0 |
| 30-40 s | prev | 5 | 13.5 | 0.810 | 9.44 | 0 | 0 | 0 |
| 30-40 s | new | 5 | 13.8 | 0.830 | 9.99 | 0 | 0 | 0 |

Old vs new at the same music position (counted runs; exact permutation test; only bins both bundles reached):

| music bin | measure | old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|
| 0-10 s | wall time of the bin (s) | 13.971 (6) | 13.633 (5) | -0.338 | 0.058 |
| 0-10 s | distinct frames per wall s | 0.950 (6) | 0.967 (5) | +0.018 | 0.212 |
| 0-10 s | underrun quanta | 10.000 (6) | 7.000 (5) | -3.000 | 0.253 |
| 0-10 s | episodes | 2.000 (6) | 3.000 (5) | +1.000 | 0.242 |
| 0-10 s | engine gap frames | 1065.000 (6) | 638.000 (5) | -427.000 | 0.162 |
| 10-20 s | wall time of the bin (s) | 13.546 (6) | 12.574 (5) | -0.973 | 0.009 |
| 10-20 s | distinct frames per wall s | 0.840 (6) | 0.879 (5) | +0.039 | 0.030 |
| 10-20 s | underrun quanta | 0.000 (6) | 0.000 (5) | +0.000 | 1.000 |
| 10-20 s | episodes | 0.000 (6) | 0.000 (5) | +0.000 | 1.000 |
| 10-20 s | engine gap frames | 0.000 (6) | 0.000 (5) | +0.000 | 1.000 |
| 20-30 s | wall time of the bin (s) | 16.950 (6) | 15.487 (5) | -1.463 | 0.026 |
| 20-30 s | distinct frames per wall s | 0.809 (6) | 0.852 (5) | +0.043 | 0.128 |
| 20-30 s | underrun quanta | 0.000 (6) | 0.000 (5) | +0.000 | 1.000 |
| 20-30 s | episodes | 0.000 (6) | 0.000 (5) | +0.000 | 1.000 |
| 20-30 s | engine gap frames | 0.000 (6) | 0.000 (5) | +0.000 | 1.000 |
| 30-40 s | wall time of the bin (s) | 13.503 (5) | 13.793 (5) | +0.290 | 0.048 |
| 30-40 s | distinct frames per wall s | 0.810 (5) | 0.830 (5) | +0.020 | 0.397 |
| 30-40 s | underrun quanta | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |
| 30-40 s | episodes | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |
| 30-40 s | engine gap frames | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 12.500 (6) | 9.000 (5) | -3.500 | 0.253 | 12.500 (6) | 9.500 (6) | -3.000 | 0.370 |
| underrun quanta 30-60 s (heavy interval) | 0.000 (6) | 0.000 (5) | +0.000 | 0.727 | 0.000 (6) | 0.000 (6) | +0.000 | 0.727 |
| episodes 0-60 s | 2.500 (6) | 3.000 (5) | +0.500 | 0.909 | 2.500 (6) | 3.000 (6) | +0.500 | 0.909 |
| engine gap frames 0-60 s | 1385.000 (6) | 894.000 (5) | -491.000 | 0.201 | 1385.000 (6) | 958.000 (6) | -427.000 | 0.351 |
| write rate mean 10-60 s | 0.942 (6) | 0.964 (5) | +0.022 | 0.126 | 0.942 (6) | 0.964 (6) | +0.022 | 0.061 |
| Worker CPU s (60 s) | 50.455 (6) | 50.010 (5) | -0.445 | 0.071 | 50.455 (6) | 49.995 (6) | -0.460 | 0.032 |
| UI idles/s (10-60 s) | 254.805 (6) | 297.053 (5) | +42.248 | 0.002 | 254.805 (6) | 297.393 (6) | +42.588 | 0.002 |
| presents/s (10-60 s) | 14.811 (6) | 15.863 (5) | +1.052 | 0.028 | 14.811 (6) | 15.895 (6) | +1.084 | 0.011 |
| lagCuts/s (10-60 s) | 18.710 (6) | 18.609 (5) | -0.101 | 0.677 | 18.710 (6) | 18.623 (6) | -0.087 | 0.545 |
| virtual s per real s (10-60 s) | 0.679 (6) | 0.716 (5) | +0.036 | 0.019 | 0.679 (6) | 0.717 (6) | +0.038 | 0.009 |

## 6 5 分再生（E11）
(5 分再生は未実施。gpu1 の実スレッド版の窓の後に取る)

## 7 自己申告の逸脱
1. 14:06〜14:28 の E9 の pure の失敗（サーバー停止）は E9 の文書に記載。E11 の run は全て、サーバーが 200 を返すことを run の前に検査した。
2. E11 の first idle の Chrome の最後の数組（7〜10 組目）は、alpha1 の最後の Chrome の run と一部重なった可能性がある。重なりの数は観測プロセス（2 秒ごと）の記録から後で付ける（idle の表には『他の Chrome の数』の列がまだ無い）。first idle の判定は『開始時 load ≤8・I/O 圧 0』の gate で通った run。
3. E11 の pure の最初の 1 run（E10 の 1）を、最初は『他の Chrome 2 本の重なり』として参考に分けた（E10 n=5 で報告）が、**E9 の手書きの記録の誤適用で、観測プロセスの記録では他の Chrome は 1 本（opt1）で重なりなし**。E10 を n=6 に直した（E11 は 2 組目が I/O 圧 0.18 で外れて n=5）。結論は不変（仮想 s/実 s 0.679 → 0.716、p 0.019 など）。
4. 5 分再生は未実施（上記）。
5. pure で、10 s ごとだった snapshot を 2 s ごとに変えた（E9 の pure は 10 s ごと）。音楽 bin の underrun は snapshot の線形補間、gap フレームは 1 Hz のエンジン報告の線形補間。
6. この文書の first idle の表の列見出し『quiet (load ≤12, I/O <1)』は誤記で、E11 の判定に使った閾値は **load ≤8・I/O 圧 <1**（PM の指示、report の閾値を環境変数で 8 にしていた）。表の『yes / no (reference)』は ≤8 で付いている（キットの report は以後、見出しも閾値に合わせる）。

## PM 記入欄（ci1 は書かない）


## PM の決定と取り込み(2026-11-06 18:25、docs のみ。製品の差分なし)

取り込み: ci1 の E11(本番 bundle `fl-engine-ci-dc5fae2b`、Worker 00769751、ページ 7373cd55)の確認の文書・結果・画・小さな表を `docs/investigations/ci1/0277/e11/` に置いた(音声の生データ `*-audio.json` は入れない。原本は `lift/out-git/ci1-results/0277/e11/data/`)。実ブラウザ(Chrome 156 / Firefox 158、この機械、模擬入力)で、**依頼者の機械は 0 件**、本番 R2 の `perf/` に beacon は 0 件。

事実として受け取る(詳細は本文):
1. 停止なし 30/30、Export 22,863,828 B・sha256 `381cdb5a…f979`、File ×100 が両ブラウザで 100/100、manifest 全一致、alpha7 の画は E8・E9 と 65 画素(時計の欄)だけ違う。
2. first idle(E11 − E10、本番): Chrome 10 組の中央値 +184 ms(静かな run だけで +171)、9 組目の外れ値を除く平均 +211 ± 68 ms(n=9)。Firefox 6 組の中央値 +104 ms、平均 +236 ± 136 ms。**基準 +100 ms を超える**(opt1 の非本番の測定は +79 ± 53 ms で、本番のほうが大きい)。
3. 純再生(Chrome、E10 対 E11、各 5): 60 s の underrun は両方約 10 quanta で差なし、仮想 s/実 s 0.684 → 0.716(p 0.032)、Worker CPU 50.5 → 50.0 s、presents/s 14.9 → 15.9。音楽位置で揃えると 0〜30 s の bin は E11 が壁時計で 4〜8% 短い。E11 は音楽 40 s までは数 % 速いが underrun の差は出ない。

判断:
1. **E11 の first idle の超過(+184 ms)は撤回の理由にしない**。P1 の目標は 10 s で、実測は約 4.5〜4.9 s、E10 とのたった +0.2 s。E11 の価値(Worker の CPU −5%、純再生の仮想時計 +5%)に見合う。ただし超過の原因(typed アダプタ 603 本と wasm +310 KB が engine 初期化 3.3 s を伸ばす、と読む。未確認)は E13 以降の候補として残す(初期化の内訳は ci1 の beacon の段階時刻で取れる)。E12 は E11 比で +28 ± 25 ms(外れ値除く 14 組)で超過は無い。
2. 音楽 40 s 以降(重い区間)は、E10・E11 の本番では未測定のまま。実スレッド版の判断(決定 0306)と E11 の 5 分再生で扱う。5 分再生の本番(E11)は、E12 が本番なので E12 の 5 分で代える。
3. 未確認: E11 の 5 分再生、lat(入力の true T)、lid 8 と最小化→復元(E9 で確認した経路で E11 では再実行していない)。

(訂正、2026-11-06 20:35、PM: ci1 の 20:36 の訂正〔手書きの重なり記録の取り違え〕で、本文の e11 の pure の数値が直った。上の PM の節の pure の数字のうち E8/E10 側の n・underrun・書き込み率・仮想 s/実 s は、本文の訂正後の表が正。結論は不変。)
