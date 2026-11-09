# ci1 → PM 決定 0277（E12）: E12 の本番 bundle `fl-engine-ci-7a9b0800`（Worker e9565033、ページ = a977401d）の実ブラウザ確認 — 停止なし、Export 一致、File ×100、first idle（E11 比、16 組と遅い 2 本の分析）、alpha7、純再生 pure（E11 対 E12、壁時計と音楽位置）

## 先頭の 3 点（実エンジン・実ブラウザ Chrome 156 / Firefox 158、この機械・headless・模擬入力。**依頼者の機械は 0 件**。代役・再生・推測なし。Firefox の preflight は webglRenderer = llvmpipe、desk.mode = gl。各値に load と I/O 圧 avg10 を併記し、first idle・pure は『開始時 load ≤8 かつ I/O 圧 0』の run だけ数え、他の担当の Chrome・CPU が重なった run は『参考（重なり）』に分けた）
1. **測ったこと**: R2 から読み取りで取得した E12 `fl-engine-ci-7a9b0800`（Worker e9565033、ページ = a977401d）の manifest の sha256 照合（wasm `d0a1a66b7ab56571…` 143,085,947 B、glue・seed・dll は E11 と同じ、全一致）。E11 `fl-engine-ci-dc5fae2b` と同じページで: 15 シーン × 2 ブラウザ、File ×100、first idle（Chrome 16 組、交互）、Export、alpha7（E11 と画素比較）、純再生 pure（E11 対 E12、Chrome 8 組、壁時計と音楽位置で揃えた窓）。
2. **結論**:
   - 停止なし（30/30 ok）、Export の WAV が参照と一致（22,863,828 B、sha256 `381cdb5a…f979`）、File ×100 は Chrome・Firefox とも 100/100。alpha7 の画は E11 と **0 画素差**（主窓 @0,0 1920x1080）。
   - **first idle は E11 とほぼ同じ（基準 +100 ms を満たす）**: 16 組の中央値 E12 4,695 / E11 4,640 ms（+54）、組ごとの差の中央値 +42。E12 の 5・6 組目が 7,098 / 7,655 ms と遅かったので外れ値の分析と追加 6 組を取った（§3.1）。5・6 組目を除く 14 組は差の中央値 +19、平均 +28 ± 25（SE）、追加 6 組は +21 ± 47（SE）で、遅い型は再現しなかった。
   - **純再生 pure は E11 と差がない**（数えた run: E11 5、E12 6。参考を含む全 8 組ずつでも同じ）: 60 s の underrun quanta は E11 6・E12 8（p 0.57）、30〜60 s は両方 0、書き込み率 0.981 / 0.963（p 0.18）、仮想 s/実 s 0.745 / 0.731（p 0.13）、Worker CPU 50.0 / 50.0 s、lagCuts/s 17.4 / 18.0。音楽位置で揃えた bin（音楽 0〜40 s）を通る壁時計の時間は E12 が +0.2〜+0.8 s（p 0.15〜0.73）で有意でない。E11 → E12 で P3 の悪化も改善も検出できない。
3. **未確認**: オーナーの機械（0 件）。**Firefox の first idle（E12 対 E11）は未実施**。E12 の 5 分再生（音楽 40 s 以降の重い区間）と lat（入力の true T）は E12 では未実施。pure の数えた run は E11 側が 5 組（参考にした 3・5・7 組目を除く）で、条件の『6 組以上』に 1 組足りない（PM の指示で補充せず）。E11・E12 とも 60 s で音楽 40 s 前後にしか届かない。

## 1 判定基準（E12 対 E11、run の load と I/O 圧は各表）
| criterion | value | pass |
|---|---|---|
| Firefox has WebGL (preflight) | preflight: webglRenderer llvmpipe, or similar desk.mode gl first idle 4313 | yes |
| No stop (11 scenes + more, both browsers, play, export; d7 is the intended exit) | stops: none; not ok: none; no result at all (the harness did not start): none | yes |
| Export WAV equals the reference | 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` (expected 22863828 B, `381cdb5acf891a16…`) | yes |
| First idle ≤ previous + 500 ms (chrome, WebGL runs) | all runs: new median 4695 ms, previous 4640 ms, difference +54 ms; means 5007 ± 234 (SE, n=16) / 4650 ± 31 (n=16), difference of the means +357 ± 236 (SE); quiet runs only (load ≤12, I/O avg10 <1; n new 16 / prev 16): new 4695, previous 4640, difference +54 ms | yes |
| First idle ≤ previous + 500 ms (firefox, WebGL runs) | not run | not run |
| File menu ×100 (chrome) | 100/100 | yes |
| File menu ×100 (firefox) | 100/100 | yes |

## 2 15 シーン（Chrome・Firefox、各行の load と I/O 圧。2 ジョブ並行のため first idle は参考値）
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

## 3 first idle（単独 run・冷間・交互。prev = E11、new = E12。基準 +100 ms）
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

### 3.1 E12 の first idle の遅い 2 本（5・6 組目）の分析と追加 6 組
- **16 組**（Chrome、E12 対 E11 交互、開始 load 2.4〜6.8・I/O 圧 0。1〜10 組目は 18:00〜18:03、11〜16 組目は 18:09〜18:11 に他の Chrome 0 本で追加）: 中央値 E12 4,695 / E11 4,640 ms（**+54**）、平均 5,007 ± 234 / 4,650 ± 31（SE）、組ごとの差の中央値 +42、平均の差 +357 ± 228（SE）。
- 5・6 組目（E12 のみ 7,098 / 7,655 ms）を**除いた 14 組**: 中央値 4,670 / 4,636、**組ごとの差 中央値 +19、平均 +28 ± 25（SE）**。**追加の 6 組だけ**: 中央値 4,564 / 4,550、組ごとの差 中央値 −8、平均 +21 ± 47（SE）。基準 +100 ms は 16 組でも 14 組でも満たす。
- **遅い 2 本の分析**（18:01:08 と 18:01:21 開始、同時刻の他の Chrome は 0 本〔観測プロセスの 2 秒ごとの記録〕、開始・終了 load 5.65 → 5.61、5.61 → 5.37、I/O 圧 0）: ページの beacon の段階時刻で、通常の E12 run は main_start 1,255 → idle 4,608（engine 初期化 = main_start → idle が 3,353 ms）。遅い 5 組目は main_start 1,261（通常と同じ）→ idle 7,098（初期化 5,838、+2.5 s）、6 組目は download_done 1,704（+0.6 s）→ main_start 1,908 → idle 7,655（初期化 5,747、+2.4 s）。**遅れの本体は engine の初期化（Worker の CPU 作業）の +2.4〜2.5 s**。同型の外れ値は E10（9 組目、idle 8,102、初期化 5,933）と E11（9 組目、idle 6,368、初期化 4,281。別の scene run 1 本 7,364）にもある（first idle が 6 s を超えた run / その bundle の first-idle beacon 数: E10 1/22、E11 2/67、E12 2/13）。E12 の 2 本は連続する約 25 秒の窓（18:01:08〜18:01:33）に収まり、直前直後の run は通常。外乱の直接の証拠（CPU 圧の記録）は無いので『E12 固有ではない』とも『一過性の外乱』とも断定せず、**追加の 6 組で再現しなかった**ことを根拠に一過性と読む。

## 4 Export
result ok; load 8.2→8.7, io 0.0→0.0; seconds 314
- WAV `serina1-serum2.wav`: 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`; expected 22863828 B / `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`: MATCH

## 5 純再生 pure（E11 対 E12、参照 zip を開き Space で 60 s、10 s 窓、2 s ごとの snapshot、交互）
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

## 6 5 分再生
(5 分再生は未実施。gpu1 の実スレッド版の窓の後に取る)

## 7 自己申告の逸脱
1. **pure の参考にした run**: E11 側の 3 組目（PM の検証作業が重なり、終了時の I/O 圧 3.0）、5 組目（opt1 の wasm-opt の link が重なり、終了時 load 13.0）、7 組目（alpha1 の node Export、mt3 の clang 2 本、gpu1 の smoke の Chrome が約 1 分重なった）。E12 側の 7 組目（gpu1 の 2 本目の smoke Chrome 20:26:00〜20:30:16 が全体に重なった）と 8 組目の最初の約 50 秒。数えた run は E11 5、E12 6。全 run の要約も併記した。
2. **『pure end』の行を 2 回書いた**: pure の 6 組が終わった 20:24:37 の行と、補充の組 7・8 が終わった 20:33:05 の行（どちらもモード終了時に出る）。gpu1 と PM が 20:24:37 の行を ci1 の終了と読んで窓を使い、補充の組と重なった（私の説明不足）。以後、窓の合図は『ci1 extras done』の info と steps.log の行にした。
3. 18:04〜18:05 に、gpu1 が Chrome を始めていたのに追加 6 組の first idle を始めかけて 3 run で止め、結果を捨てた（重なり。追加の 6 組は 18:09〜18:11 に他の Chrome 0 本で取り直した）。
4. 他の Chrome の数の『手書きの記録』（`chrome-count-manual.json`、E9 の最初の失敗した pure の 1 run 用）を E9 の再実行と E11 の pure の最初の run に誤って当てていた。観測プロセスの記録に直し、E9 と E11 の文書の数値を更新した（結論は不変）。E12 の pure では使っていない。
5. first idle の 5・6 組目の遅れは、engine 初期化の +2.4 s で、E10・E11 にも同型の外れ値があり、追加の 6 組で再現しなかったので一過性と読んだが、直接の外乱の証拠（CPU 圧の記録）は無い。

## PM 記入欄（ci1 は書かない）


## PM の判断と取り込み(2026-11-06 20:36、docs のみ。製品の差分なし)

取り込み: ci1 の E12 の本番確認(先頭 3 点・判定表・15 シーン・Export・File ×100・first idle・alpha7・pure)の文書と data(各 run の `*-audio.json` は 300 KB 近いので入れない。原本は `lift/out-git/ci1-results/0277/e12/data/`)を `docs/investigations/ci1/0277/e12/` に置いた。同時に、ci1 の 20:36 の訂正(手書きの重なり記録の取り違えで、E9 の再実行と E11 の pure の最初の run に他 Chrome の重なりがあったとしていた誤り。E8 側を n=5 → 6、E10 側を n=5 → 6 に直した)を `0277-e9-verification.md`・`0277-e11-verification.md` と investigations の e9・e11 に反映した(各 PM の節は残し、末尾に訂正を足した)。実 Chrome 156 と Firefox 158(llvmpipe)、この機械の headless、模擬入力。**依頼者の機械は 0 件、本番 R2 の `perf/` に beacon は 0 件**。

事実として受け取る(本番 E12 = bundle `fl-engine-ci-7a9b0800`、Worker `e9565033-8c44-4395-a6a4-53abee97001d`):
1. manifest は全一致(wasm `d0a1a66b7ab56571…` 143,085,947 B)。15 シーン × Chrome・Firefox = 30/30 停止なし。File ×100 は両ブラウザとも 100/100。Export は 22,863,828 B・sha256 `381cdb5a…f979` で参照と一致(私の sha256 でも同じ)。alpha7 の画は E11 と 0 画素差(主窓 @0,0 1920x1080)。
2. first idle は E11 とほぼ同じ: 16 組の中央値 E12 4,695 / E11 4,640 ms(+54 ms)。遅い 2 組を除く 14 組は差の平均 +28 ± 25、追加 6 組は +21 ± 47 で、遅い型は再現しなかった。基準 +100 ms を満たす。
3. 純再生 pure(Chrome、E11 対 E12): **数えた run は E11 側 5・E12 側 6、参考(他担当の重なり)を含む全 8 組ずつでも結論は同じ**。60 s の underrun quanta は 6 → 8(p 0.57)、30〜60 s は両方 0、書き込み率は 0.981 → 0.963(p 0.18、全 8 組 0.971 → 0.971)、仮想 s/実 s は 0.745 → 0.731(p 0.13)、Worker CPU は 50.0 → 50.0 s。音楽 0〜40 s の 10 s ごとの bin を通る壁時計の時間は E12 が E11 より 0.2〜0.8 s 長い向きだが有意でない(p 0.15〜0.73)。= **E11 → E12 の P3 の変化は検出できない**(E12 の変更は DirectSound の書き込みカーソルと CompareStringW の重みで、ミキサーの CPU には効かない見込みどおり)。

PM の補足(重なりの記録): pure の補充の組(7・8)の最中に、私の運用の誤りで他の担当の Chrome・CPU(gpu1 の smoke の Chrome 約 4 分、alpha1 の node Export と mt3 の clang 約 1 分)が重なった。「pure end」の行を補充の終わりと読み違えて窓を配った私の誤りで、ci1 は該当 run を『参考』に分けて表に残した。数えた run が E11 側 5 組で条件の『6 組以上』に 1 組足りないが、E12 は既にデプロイ済みで、この比較は関門ではないため補充しない。

判断:
1. **E12 は撤回しない**。first idle・停止なし・Export・File ×100・alpha7・P3(pure)のいずれも E11 と区別できない。E12 の本番確認はこれで閉じる(Firefox の first idle と 5 分再生は E13 の確認の後に回す)。
2. 持ち越し(優先度は E13 の本番確認が先): E12 の Firefox first idle(対 E11)、E12・E11 の 5 分再生(音楽 40 s 以降)、Firefox の pure(E8 対 E9)、ff5ee429 対 E7b の P3、lat(入力の true T)。
3. 全体の注意: 本書のすべての P2/P3 の値は、FL 内蔵エフェクト 7 種 25 インスタンスと Fruity Wrapper が無視された負荷での値(G9 未到達、決定 0296 の訂正 2)。
