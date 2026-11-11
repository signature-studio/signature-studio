# ci1 → PM 決定 0277（E13）: E13 の本番 bundle `fl-engine-ci-0363b050`（Worker 0f8ed7c4、ページ = d0447902）の実ブラウザ確認 — 停止なし、Export 一致、File ×100、first idle（E12 比）、alpha7、純再生 pure（E12 対 E13）

## 先頭の 3 点（実エンジン・実ブラウザ Chrome 156 / Firefox 158、この機械・headless・模擬入力。**依頼者の機械は 0 件**。代役・再生・推測なし。Firefox の preflight は webglRenderer = llvmpipe、desk.mode = gl。各値に load と I/O 圧 avg10 を併記し、first idle は『開始時 load ≤8 かつ I/O 圧 0』の run を数えた）
1. **測ったこと**: R2 から読み取りで取得した E13 `fl-engine-ci-0363b050`（Worker 0f8ed7c4、ページ = d0447902）の manifest の sha256 照合（wasm `58ad4a6aa720375a…` 143,087,706 B、glue・seed・dll は E12 と同じ、全一致）。E12 `fl-engine-ci-7a9b0800` と同じページで: first idle（Chrome 10 組、交互）、15 シーン × 2 ブラウザ、File ×100、Export、alpha7（E12 と画素比較）、純再生 pure（E12 対 E13、Chrome 6 組、交互）。
2. **結論**:
   - **first idle は E13 が E12 より速い**（Chrome、静かな窓、load 1.3〜2.1・I/O 圧 0）: 中央値 E13 4,320 / E12 4,428 ms、**組ごとの差の中央値 −145 ms、平均 −108 ± 29（SE）、10 組中 9 組が負**。基準 +100 ms を満たす。
   - **Firefox の first idle（E13 対 E12 交互 6 組、開始 load 1.6〜5.9・I/O 圧 0）も同じ**: 中央値 E13 4,172 / E12 4,147 ms（+26 ms）、平均の差 −26 ± 107（SE）。
   - 停止なし（Chrome 15/15 ok、Firefox 15/15 ok〔最初の実行で 2 本が WebDriver の切断、再実行で ok、§7〕）、Export の WAV が参照と一致（22,863,828 B、sha256 `381cdb5a…f979`）、File ×100 は Chrome・Firefox とも 100/100。alpha7 の画は E12 と **0 画素差**（主窓 @0,0 1920x1080）。
   - **純再生 pure（E12 対 E13、Chrome 6 組、交互。参照 zip を開き Space で 60 s、10 s 窓）は実質の差がない**（12 run すべてが数えた run: 開始 load 1.0〜3.4・I/O 圧 0、他の Chrome 0 本〔観測プロセス〕。1 run だけ I/O の静けさを 15 s 待った。順序は prev・new・new・prev・prev・new・new・prev・prev・new・new・prev）: 60 s の underrun quanta は E12 2・E13 2.5（中央値、p 0.68）、30〜60 s は両方 0、書き込み率（10〜60 s の平均）は 0.985 / 0.992（+0.007、p 0.013）、仮想 s/実 s は 0.806 / 0.811（p 0.12）、Worker CPU は 49.53 / 49.42 s（−0.12 s、p 0.043）、presents/s は 18.48 / 18.70（p 0.032）、lagCuts/s は 16.6 / 16.4（p 0.45）。音楽位置で揃えた bin（音楽 0〜40 s、全 12 run が到達）の壁時計は −0.88〜+0.30 s（0〜10 s が −0.88 s で p 0.041、他は p 0.08〜0.28）。p < 0.05 の 5 項目（書き込み率・Worker CPU・presents/s・音楽 0〜10 s の壁時計・音楽 20〜30 s の distinct frames）はいずれも E13 が同じか僅かに良い向きで、悪化は検出できない。多重比較は補正しておらず、E13 のコード変更（Dwm と CompareStringW の句読点）が P3 に効く理由は見つかっていないので、偶然の可能性を排除できない。**同じ E12 の bundle でも前回の純再生〔0277-e12、開始 load 2.6〜8.0〕は書き込み率 0.963、今回〔開始 load 1.0〜3.4〕は 0.985 で、窓の静けさで約 2% 動く**（読み）。60 s では音楽 40 s 前後にしか届かず、重い区間（音楽 40 s 以降）は未検証。
3. **未確認**: オーナーの機械（0 件）。E13 の 5 分再生と lat（入力の true T）は未実施。純再生は 60 s で音楽 40 s 前後にしか届かず、重い区間（音楽 40 s 以降）は未検証。

## 1 判定基準（E13 対 E12、run の load と I/O 圧は各表）
| criterion | value | pass |
|---|---|---|
| Firefox has WebGL (preflight) | preflight: webglRenderer llvmpipe, or similar desk.mode gl first idle 3857 | yes |
| No stop (11 scenes + more, both browsers, play, export; d7 is the intended exit) | stops: none; not ok: none; no result at all (the harness did not start): none | yes |
| Export WAV equals the reference | 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` (expected 22863828 B, `381cdb5acf891a16…`) | yes |
| First idle ≤ previous + 500 ms (chrome, WebGL runs) | all runs: new median 4320 ms, previous 4428 ms, difference -109 ms; means 4314 ± 19 (SE, n=10) / 4421 ± 22 (n=10), difference of the means -108 ± 29 (SE); quiet runs only (load ≤12, I/O avg10 <1; n new 10 / prev 10): new 4320, previous 4428, difference -109 ms | yes |
| First idle ≤ previous + 500 ms (firefox, WebGL runs) | all runs: new median 4172 ms, previous 4147 ms, difference +26 ms; means 4156 ± 75 (SE, n=6) / 4182 ± 77 (n=6), difference of the means -26 ± 107 (SE); quiet runs only (load ≤12, I/O avg10 <1; n new 6 / prev 6): new 4172, previous 4147, difference +26 ms | yes |
| File menu ×100 (chrome) | 100/100 | yes |
| File menu ×100 (firefox) | 100/100 | yes |

## 2 15 シーン（Chrome・Firefox、各行の load と I/O 圧。2 ジョブ並行のため first idle は参考値）
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

## 3 first idle（単独 run・冷間・交互。prev = E12、new = E13。基準 +100 ms）
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

### 3.1 E12 の first idle の遅い 2 本（5・6 組目）の分析と追加 6 組


## 4 Export
result ok; load 12.3→6.4, io 0.0→0.0; seconds 304
- WAV `serina1-serum2.wav`: 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`; expected 22863828 B / `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`: MATCH

## 5 純再生 pure（E12 対 E13）
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

## 6 5 分再生
(未実施)

## 7 自己申告の逸脱
1. **Firefox の scenes 30 本中 2 本が WebDriver の `invalid session id`**（d1-sampler-envelope: 21:00:03 開始、32 s 後。d6-playlist-close: 21:02:32 開始、13 s 後）。エンジンの停止ではない（`stopped` なし、exitText なし）。geckodriver のログは『Failed to close browser connection』のみ、RAM は 26 GB 中 10 GB 使用で OOM の痕跡は無い。切断の時刻は PM が alpha1 の node-run と mt3 の clang を SIGCONT した直後（load 3 → 10）と重なるが、因果は未確認（E9 の Firefox の d7 にも同型の切断が 1/4 あった）。この 2 場面を再実行して ok（再実行は 4 本実行し、最後の d1・d6 の結果だけが残っている〔前の 2 本は dir が上書きされた〕）。最初の失敗の記録は `data/ff-first-session-lost/`。
2. 並行して Chrome の scenes を走らせたため、Firefox の scenes の load は 3〜13（参考値）。
3. first idle の Chrome 10 組は他の Chrome が 0 本（観測プロセス）で、重なりは無い。
4. **E13 pure の自動開始の待機を、PM の go-ahead なしに張った**（23:05。gpu1 の `run-ptv6.sh` の連鎖が終わったら自動で `pure chrome 6` を始める背景の連鎖）。『開始は PM の info のみ』の規則に反する。23:06 に PM の指示を受け、23:07 に自分の pid（待機の bash と tail、23:10 の wait-until）だけを kill して止めた。**pure は 1 本も始まっていない**（steps.log に『pure start』なし、ci1 の Chrome・Firefox は 0 本）。開始は PM の info を待つ。
5. **純再生（E12 対 E13）は PM の go（01:4x）で 01:47:48 に始め、02:09:45 に完走**（12 run、rc 0）。開始前に Wine/FL64/wineserver・clang・wasm-opt・emcc・node Export が 0 本、load 1.00、I/O 圧 0.00 を自分でも確認した。全 12 run が数えた run（参考なし）。
6. `bash` という名前のツールを 8 回目に呼んだ（`echo test` だけ、変更なし。別の作業〔決定 0314〕の最中）。

## PM 記入欄（ci1 は書かない）


## PM の決定と取り込み(2026-11-06 21:18、docs のみ。製品の差分なし)

取り込み: ci1 の E13 本番確認(第 1 報、21:18)の文書・結果・画像を `docs/investigations/ci1/0277/e13/` に置いた(`*-audio.json` は入れない)。実エンジン・実 Chrome 156 / Firefox 158 の測定。**依頼者の機械は 0 件、本番 R2 の `perf/` に beacon は 0 件。FL 内蔵エフェクト 7 種 25 インスタンスと Fruity Wrapper が無視された負荷での値(G9 未到達)**。

事実として受け取る(E13 = bundle `fl-engine-ci-0363b050`、Worker `0f8ed7c4-f576-4b40-a3b9-7fca05115164`、wasm `58ad4a6aa720375a…` 143,087,706 B):
1. manifest は R2 の読み取りで全一致(glue `07e7646f280b…`、seed・dll は E12 と同じ)。
2. **first idle(Chrome、E13 対 E12 の交互 10 組、開始 load 1.3〜2.1、I/O 圧 0、他の Chrome 0 本)**: 中央値 E13 4,320 ms 対 E12 4,429 ms、組ごとの差の中央値 −145 ms、平均 −108、10 組中 9 組が負(PM が idle-chrome.tsv から再計算: 差の平均 −107.5、標準偏差 107.4、SE 34.0。ci1 の第 1 報も SE 34 だが、文書本体の表は ±29 とあり、数え方の差。結論は同じ)。基準 +100 ms を満たす。E13 のコードは Dwm の行と CompareStringW の表だけなので、この差は E13 の効果とは読まない(揺れ)。
3. **Export: 22,863,828 B、sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`** で E8〜E12 と一致。alpha7: 主窓 @0,0 1920x1080、E12 の画と 0 画素差。File ×100: Chrome 100/100、Firefox 100/100。
4. 停止なし: Chrome 15/15。Firefox は 13/15 + 2 本が WebDriver の `invalid session id`(d1-sampler-envelope と d6-playlist-close、最初の 13〜32 s、エンジンの停止ではない〔stopped なし〕)。この 2 場面を load 3〜6 で再実行して ok(= 30/30 ok、ただし再実行 2 本つき)。切断の時刻は私が alpha1 の node-run と mt3 の clang を SIGCONT して load が 3 → 10 に上がった時と重なる(ci1 の記録どおり)。E9 の Firefox d7 にも同型の切断が 1/4 であり、原因は不明のまま。E13 固有とは断定しない。

判断:
1. **E13 は撤回しない**。first idle・停止なし(再実行つき)・Export・alpha7・File ×100 は E12 と区別できない。**E13 の本番確認(Chrome)はこれで閉じる**。
2. Firefox の WebDriver 切断は、load が高い窓(私の段取りの責任)で出た。今後 ci1 の scenes(Chrome ‖ Firefox)の最中は、PM が他の worker に重い job を許さない(今回は許してしまった)。
3. 持ち越し(優先度は gpu1 の E14 の窓の後): E13 対 E12 の pure(Chrome 6 組、counted、約 22 分)、Firefox の first idle(E13 対 E12、6 組、約 3 分)、E12 の Firefox first idle、E11・E12 の 5 分再生、Firefox の pure(E8 対 E9)、ff5ee429 対 E7b の P3、lat。
4. 本書の P2/P3 の値はすべて内蔵エフェクト無視の負荷での値。

### 追記(2026-11-06 21:58、ci1 の Firefox first idle の取り込み)

ci1 の追報(21:58)で **Firefox の first idle(E13 対 E12 交互 6 組、開始 load 1.6〜5.9、I/O 圧 0、Firefox のみ、約 3 分)**: 中央値 E13 4,172 ms 対 E12 4,147 ms(+26 ms)、平均の差 −26 ± 107(SE)。Chrome の −145 ms(中央値)と合わせ、E13 の first idle は両ブラウザで E12 と区別できない(基準 +100 ms の内)。**E13 の本番確認は Chrome・Firefox とも閉じる**。残り: pure(E12 対 E13、Chrome 6 組。22:40 以降の窓、PM の info で)、5 分再生・lat(優先度低)。E13 のコードは Dwm と locale だけなので、これらで E13 を撤回する条件は想定しない。


### 追記(2026-11-06 23:30、ci1 の自己申告の取り込み)

ci1 の 23:08 の版(自己申告の追加)に差し替えた。追加の内容: ci1 は 23:05 に、PM の合図なしに『gpu1 の chain が終わったら E13 の pure を自動で始める』待機を置き、23:07 に自分で止めた(pure は 0 run)。私の 23:06 の info で取り消し済み。**E13 の pure(E12 対 E13、Chrome 6 組、約 25 分、counted)は Elastique のバッチ窓と gpu1 の Chrome 窓(00:00〜00:25 見込み)の後、PM の info で**。E13 の本番確認(manifest・first idle・scenes・Export・alpha7・File ×100)の結論は変わらない。


### 追記(2026-11-07 02:13、ci1 の純再生 pure〔E12 対 E13、Chrome 6 組・12 run〕の取り込み、docs のみ)

ci1 の 02:11 の版に差し替えた(先頭三点と §5)。**実エンジン・Chrome 156・この機械・headless・模擬入力。依頼者の機械は 0 件。FL 内蔵エフェクト 7 種 25 インスタンスと Fruity Wrapper が無視された負荷での値**。PM の検証: `data/P3-chrome.md` の 12 run の行を自分で読んだ。12 run とも開始 load 1.0〜3.4・I/O 圧 0.00(1 run だけ 15 s 待った)・他の Chrome 0 本。60 s の underrun quanta は E12(prev)が 2 を 6 run、E13(new)が 4・2・0・0・11・3、30〜60 s の underrun は両方 0、書き込み率(10〜60 s の平均)は prev 0.981〜0.990、new 0.986〜0.996、消費率 0.994〜1.000、presents/s は 18.2〜18.7 対 18.5〜19.0、仮想 s/実 s は 0.783〜0.817 対 0.805〜0.834。**判断: E13 は E12 に対し P3 の指標で悪化を検出しない**(Dwm と句読点だけの変更で、改善は偶然を排除できない: 多重比較の補正なし)。E13 の本番確認(manifest・first idle・scenes・Export・alpha7・File ×100)の結論は変わらない。**同じ E12 でも窓の静けさ(開始 load 2.6〜8.0 → 1.0〜3.4)で書き込み率が 0.963 → 0.985 と約 2% 動く**ので、E 間の差は同じ窓で交互に測ることが条件。**この窓でも 60 s で音楽 40 s 前後にしか届かず、P3(Song Mode が途切れない)の重い区間は未検証・未達**(P3 の書き込み率の目標 0.95 は満たしているが、音楽の重い区間と 5 分再生は B1 の局所の測定だけ)。E14 の first idle・E13 の 5 分再生と lat は未実施。
