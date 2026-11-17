# ci1 → PM 決定 0277（E9）: E9 の本番 bundle `fl-engine-ci-62098b85`（Worker 2f390ad3、ページ = 38efb398）の実ブラウザ確認 — 停止なし、Export 一致、File ×100、first idle、alpha7、logo、最小化→復元（実マウスクリック）、純再生 pure（E8 対 E9）、入力の true T（lat）

## 先頭の 3 点（実エンジン・実ブラウザ Chrome 156 / Firefox 158、この機械・headless・模擬入力。**依頼者の機械は 0 件**。代役・再生・推測なし。Firefox の値は全て WebGL あり（`DISPLAY=:77`、webglRenderer = llvmpipe、desk.mode = gl）。各値に load と I/O 圧 avg10 を併記し、pure・lat は『開始時 load ≤8 かつ I/O 圧 0』の run だけ数え、他の Chrome が 2 本以上いた run は『参考（重なり）』に分けた）
1. **測ったこと**: R2 から読み取りで取得した E9 `fl-engine-ci-62098b85`（Worker 2f390ad3、ページ = 38efb398）の manifest の sha256 照合（wasm `55db9bdc3363…` 142,587,278 B 他、全一致）。E8 `fl-engine-ci-875bfd62` と同じページで: 15 シーン × 2 ブラウザ、File ×100、first idle（交互、各 6 組）、Export、alpha7（E8 と画素比較）、lid 8、最小化 → `#taskbar-restore` → 復元（CDP の実マウスクリックを含む 6 変種）と題 '(none)'、**純再生 pure**（参照 zip を開き Space で 60 s、E8 対 E9 交互、10 s 窓、壁時計の窓と音楽位置〔仮想時計〕で揃えた窓）、**入力の true T lat**（再生中・idle の File クリック・Esc、E8 対 E9 交互）。
2. **結論**:
   - 決定 0277 の判定基準 4 つを**満たした**: 停止なし（30/30 ok ほか全 run）、Export の WAV が参照と一致（22,863,828 B、sha256 `381cdb5a…f979`）、File ×100 は Chrome・Firefox とも 100/100、first idle は E8 比 Chrome +52 ± 94 ms、Firefox +194 ± 116 ms（平均の差 ± 標準誤差、各 n=6、基準 +500 ms 以内）。
   - **純再生（Chrome、数えた run: E8 6、E9 6）で E9 は E8 より大幅に良い**: 60 s の underrun は E8 1,121 → E9 15 quanta（p = 0.002）、書き込み率（10〜60 s の壁時計平均）0.402 → 0.763、エンジンの gap フレーム 131,581 → 1,530、Worker CPU/wall 0.71 → 0.84、**仮想 s/実 s は 1.017 → 0.572**（lagCuts が 0 → 15 /s）。**音楽位置で揃えても**（E9 が 60 s で届く音楽 0〜30 s）E9 の方が良い: 音楽 0〜10 / 10〜20 / 20〜30 s の『壁 1 秒あたりに書いた distinct フレーム』は E8 0.720 / 0.495 / 0.411 に対し E9 0.858 / 0.797 / 0.729、同じ音楽区間の underrun は 20〜30 s で 186 → 1 quanta。ただし E9 は 60 s で音楽の約 30〜37 s にしか届かず、**E8 で最も重い音楽 30〜60 s の区間は E9 では測れていない**（5 分再生が要る）。
   - **入力の true T（再生中、Chrome、数えた run: 各 2）**: File クリック 137 → **22 ms**（中央値）、Esc 34 → **5.4 ms**。idle の File クリック 7.7 → 7.3 ms、Esc 2.0 → 2.2 ms で変わらない。
   - 復元ボタンの **CDP 実マウスクリック**は E9 でも通る。復元後の題は 6 変種すべて '(none)'（最小化から 0.1 s 後の復元を含む）。alpha7 の画は E8 と 65 画素（時計）だけ違う。
3. **未確認**: オーナーの機械（0 件）。Firefox の pure（E8 対 E9）と ff5ee429 対 E7b の P3 決着は E11 の後に回した。lid 8 は first idle より前を見ていない。lat の数えた run は各 2 組（3 組目は開始時 load・I/O 圧の条件で外れた）。E9 が届かない音楽 30 s 以降。UI の idle/s と presents/s は製品のページの `driverStats.idleCount`・`deskStats.presents`（E9 で増える）で、opt1 の『UI slice』とは別の量。

## 1 判定基準（E9 対 E8、run の load と I/O 圧は各表）
| criterion | value | pass |
|---|---|---|
| Firefox has WebGL (preflight) | preflight: webglRenderer llvmpipe, or similar desk.mode gl first idle 5555 | yes |
| No stop (11 scenes + more, both browsers, play, export; d7 is the intended exit) | stops: none; not ok: none; no result at all (the harness did not start): none | yes |
| Export WAV equals the reference | 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` (expected 22863828 B, `381cdb5acf891a16…`) | yes |
| First idle ≤ previous + 500 ms (chrome, WebGL runs) | all runs: new median 5128 ms, previous 5037 ms, difference +92 ms; means 5055 ± 67 (SE, n=6) / 5002 ± 67 (n=6), difference of the means +52 ± 94 (SE); quiet runs only (load ≤12, I/O avg10 <1; n new 5 / prev 5): new 5096, previous 4982, difference +114 ms | yes |
| First idle ≤ previous + 500 ms (firefox, WebGL runs) | all runs: new median 4456 ms, previous 4289 ms, difference +167 ms; means 4482 ± 92 (SE, n=6) / 4288 ± 71 (n=6), difference of the means +194 ± 116 (SE); quiet runs only (load ≤12, I/O avg10 <1; n new 6 / prev 6): new 4456, previous 4289, difference +167 ms | yes |
| File menu ×100 (chrome) | 100/100 | yes |
| File menu ×100 (firefox) | 100/100 | yes |

## 2 15 シーン（Chrome・Firefox、各行の load と I/O 圧。2 ジョブ並行のため first idle は参考値）
| browser | scene | result | first idle ms | File ×100 | load, I/O pressure (start→end) |
|---|---|---|---|---|---|
| chrome | d1-sampler-envelope | ok | 5659 | - | load 4.0→4.1, io 0.0→0.0 |
| chrome | d2-browser-folder | ok | 4867 | - | load 4.1→4.3, io 0.0→0.0 |
| chrome | d3-settings-all-tabs | ok | 5143 | - | load 4.3→7.0, io 0.0→0.0 |
| chrome | d4-playlist-middle-drag | ok | 5395 | - | load 7.0→7.6, io 0.0→0.0 |
| chrome | d5-mixer-front | ok | 4736 | - | load 7.6→9.4, io 0.0→0.0 |
| chrome | d6-playlist-close | ok | 5017 | - | load 9.4→8.6, io 0.0→0.0 |
| chrome | d7-main-close | ok (intended exit) | 5398 | - | load 8.6→7.2, io 0.0→0.0 |
| chrome | d8-file-menu-100 | ok | 4637 | 100/100 | load 7.2→12.4, io 0.0→0.0 |
| chrome | d9-keys | ok | 4540 | - | load 12.4→14.2, io 0.0→0.0 |
| chrome | d10-pianoroll-f12 | ok | 5362 | - | load 14.2→12.6, io 0.0→0.0 |
| chrome | d11-menu-hover-escape | ok | 6046 | - | load 12.6→15.4, io 0.0→0.0 |
| chrome | add-menu-simple-fade | ok | 5790 | - | load 13.9→12.8, io 0.0→0.0 |
| chrome | picker-cost | ok | 5525 | - | load 12.8→12.7, io 0.0→0.0 |
| chrome | picker-f8-fade | ok | 4970 | - | load 12.8→12.8, io 0.0→0.0 |
| chrome | plus-more-plugins | ok | 6116 | - | load 14.7→13.9, io 0.0→0.0 |
| firefox | d1-sampler-envelope | ok | 4882 | - | load 4.0→4.1, io 0.2→0.0 |
| firefox | d2-browser-folder | ok | 4813 | - | load 4.4→6.0, io 0.0→0.0 |
| firefox | d3-settings-all-tabs | ok | 4891 | - | load 6.5→8.0, io 0.0→0.0 |
| firefox | d4-playlist-middle-drag | ok | 5156 | - | load 7.6→7.8, io 0.0→0.0 |
| firefox | d5-mixer-front | ok | 4744 | - | load 9.4→8.4, io 0.0→0.0 |
| firefox | d6-playlist-close | ok | 4825 | - | load 8.1→7.0, io 0.0→0.0 |
| firefox | d7-main-close | ok (intended exit) | 5526 | - | load 8.4→9.4, io 0.0→0.0 |
| firefox | d8-file-menu-100 | ok | 5454 | 100/100 | load 9.4→12.6, io 0.0→0.0 |
| firefox | d9-keys | ok | 5365 | - | load 12.6→13.7, io 0.0→0.0 |
| firefox | d10-pianoroll-f12 | ok | 5750 | - | load 14.4→12.8, io 0.0→0.0 |
| firefox | d11-menu-hover-escape | ok | 5288 | - | load 12.8→13.5, io 0.0→0.0 |
| firefox | add-menu-simple-fade | ok | 5090 | - | load 11.5→10.4, io 0.0→0.0 |
| firefox | picker-cost | ok | 4259 | - | load 9.1→8.1, io 0.0→0.0 |
| firefox | picker-f8-fade | ok | 4658 | - | load 10.4→9.1, io 0.0→0.0 |
| firefox | plus-more-plugins | ok | 5647 | - | load 13.6→11.6, io 0.0→0.0 |


The scenes ran as two parallel jobs (Chrome and Firefox), so the first idle values of this table are reference values (the load is next to each).

## 3 first idle（単独 run・冷間・交互。prev = E8、new = E9。平均 ± 標準誤差を表の上の判定行に付けた）
| browser | bundle | run | first idle ms | load, I/O pressure at the start | at the end | quiet (load ≤12, I/O <1) | webglRenderer | desk.mode |
|---|---|---|---|---|---|---|---|---|
| chrome | prev | 1 | 5092 | 6.10 / io 0.20 | 6.63 / io 1.39 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 1 | 5185 | 6.63 / io 1.39 | 6.81 / io 0.51 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 2 | 4959 | 6.81 / io 0.51 | 6.68 / io 0.18 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 2 | 5200 | 6.68 / io 0.18 | 7.28 / io 0.14 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 3 | 4783 | 7.28 / io 0.14 | 6.78 / io 0.05 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 3 | 4767 | 6.78 / io 0.05 | 6.79 / io 0.01 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 4 | 5161 | 6.79 / io 0.01 | 6.08 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 4 | 4842 | 6.08 / io 0.00 | 6.08 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 5 | 4982 | 6.08 / io 0.00 | 6.91 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 5 | 5096 | 6.91 / io 0.00 | 7.08 / io 0.08 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 6 | 5161 | 7.08 / io 0.08 | 6.46 / io 0.02 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 6 | 5115 | 6.46 / io 0.02 | 5.92 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| firefox | prev | 1 | 4359 | 5.92 / io 0.00 | 5.40 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 1 | 4268 | 5.40 / io 0.00 | 5.59 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 2 | 4273 | 5.59 / io 0.00 | 6.00 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 2 | 3999 | 6.00 / io 0.00 | 6.50 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 3 | 4298 | 6.50 / io 0.00 | 6.04 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 3 | 4400 | 6.04 / io 0.00 | 6.31 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 4 | 4574 | 6.31 / io 0.00 | 5.71 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 4 | 4280 | 5.71 / io 0.00 | 5.28 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 5 | 4259 | 5.28 / io 0.00 | 4.94 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 5 | 4512 | 4.94 / io 0.00 | 5.60 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 6 | 4868 | 5.60 / io 0.00 | 4.96 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 6 | 4534 | 4.96 / io 0.00 | 4.51 / io 0.00 | yes | llvmpipe, or similar | gl |

| browser | mode | previous median (n) | new median (n) | difference |
|---|---|---|---|---|
| chrome | gl | 5037 (6) | 5128 (6) | 92 |
| firefox | gl | 4289 (6) | 4456 (6) | 167 |

## 4 Export
result ok; load 15.2→10.6, io 0.0→0.0; seconds 345
- WAV `serina1-serum2.wav`: 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`; expected 22863828 B / `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`: MATCH

## 5 lid 8（ロゴ窓）の起動直後
| 実行 | bundle | 時点 | 可視の窓（lid class rect） | lid 8 の大きさ | load / I/O 圧（開始） |
|---|---|---|---|---|---|
| logo-new-1 | new | wiz0 | 14 TFruityLoopsMainForm @0,0 1920x1080; 48 TWelcomeWizard @523,253 874x573 | - | 6.82 / 0 |
| logo-new-1 | new | wiz0 の 2.5 s 後 | 14 TFruityLoopsMainForm @0,0 1920x1080; 48 TWelcomeWizard @523,253 874x573 | - | 6.82 / 0 |
| logo-new-1 | new | closeWizard の後 | 14 TFruityLoopsMainForm @0,0 1920x1080 | - | 6.82 / 0 |
| logo-new-2 | new | wiz0 | 14 TFruityLoopsMainForm @0,0 1920x1080; 48 TWelcomeWizard @523,253 874x573 | - | 6.98 / 0 |
| logo-new-2 | new | wiz0 の 2.5 s 後 | 14 TFruityLoopsMainForm @0,0 1920x1080; 48 TWelcomeWizard @523,253 874x573 | - | 6.98 / 0 |
| logo-new-2 | new | closeWizard の後 | 14 TFruityLoopsMainForm @0,0 1920x1080 | - | 6.98 / 0 |
| logo-new-3 | new | wiz0 | 14 TFruityLoopsMainForm @0,0 1920x1080; 48 TWelcomeWizard @523,253 874x573 | - | 7.24 / 0 |
| logo-new-3 | new | wiz0 の 2.5 s 後 | 14 TFruityLoopsMainForm @0,0 1920x1080; 48 TWelcomeWizard @523,253 874x573 | - | 7.24 / 0 |
| logo-new-3 | new | closeWizard の後 | 14 TFruityLoopsMainForm @0,0 1920x1080 | - | 7.24 / 0 |
| logo-prev-1 | prev | wiz0 | 14 TFruityLoopsMainForm @0,0 1920x1080; 48 TWelcomeWizard @523,253 874x573 | - | 6.87 / 0 |
| logo-prev-1 | prev | wiz0 の 2.5 s 後 | 14 TFruityLoopsMainForm @0,0 1920x1080; 48 TWelcomeWizard @523,253 874x573 | - | 6.87 / 0 |
| logo-prev-1 | prev | closeWizard の後 | 14 TFruityLoopsMainForm @0,0 1920x1080 | - | 6.87 / 0 |
| logo-prev-2 | prev | wiz0 | 14 TFruityLoopsMainForm @0,0 1920x1080; 48 TWelcomeWizard @523,253 874x573 | - | 7.34 / 0.02 |
| logo-prev-2 | prev | wiz0 の 2.5 s 後 | 14 TFruityLoopsMainForm @0,0 1920x1080; 48 TWelcomeWizard @523,253 874x573 | - | 7.34 / 0.02 |
| logo-prev-2 | prev | closeWizard の後 | 14 TFruityLoopsMainForm @0,0 1920x1080 | - | 7.34 / 0.02 |
| logo-prev-3 | prev | wiz0 | 14 TFruityLoopsMainForm @0,0 1920x1080; 48 TWelcomeWizard @523,253 874x573 | - | 8.91 / 0 |
| logo-prev-3 | prev | wiz0 の 2.5 s 後 | 14 TFruityLoopsMainForm @0,0 1920x1080; 48 TWelcomeWizard @523,253 874x573 | - | 8.91 / 0 |
| logo-prev-3 | prev | closeWizard の後 | 14 TFruityLoopsMainForm @0,0 1920x1080 | - | 8.91 / 0 |

## 6 最小化 → `#taskbar-restore` → 復元と題 '(none)'（Chrome、E9）
| 変種 | 復元後のプレイリスト見出し（crop 1125-1400 x 88-106）の start との差 | 既知の『(none)』との差 | 復元後と起動直後の画で違う画素（全体 / 行の範囲） | load / I/O 圧（開始） |
|---|---|---|---|---|
| dom-0.1s | 408 画素 | 0 画素 | 41691 / y 0..102 | 20.57 / 0 |
| dom-0.5s | 408 画素 | 0 画素 | 41691 / y 0..102 | 18.45 / 0 |
| dom-15s | 408 画素 | 0 画素 | 41698 / y 0..102 | 10.47 / 0 |
| dom-3s | 408 画素 | 0 画素 | 41691 / y 0..102 | 8.43 / 0 |
| kbd-3s | 408 画素 | 0 画素 | 41691 / y 0..102 | 19.23 / 0 |
| mouse-3s | 408 画素 | 0 画素 | 41691 / y 0..102 | 17.7 / 0 |

## 7 純再生 pure（E8 対 E9、参照 zip を開き Space で 60 s、10 s 窓、旧新交互）
### P3 decider, chrome: old (previous) vs new, windows of 10 s from the play mark; counted = start load ≤8 and I/O pressure avg10 = 0

| run | bundle | counted | other Chrome browsers at the play mark / at +60 s / max | waited for the quiet window (s) | load start → end | I/O avg10 start → end | Worker CPU s (60 s) = CPU/wall x 60 | main CPU s | underrun quanta 0-60 s | episodes 0-60 s | engine gap frames 0-60 s | **underrun 30-60 s** | write rate mean 10-60 s | consumed rate mean 10-60 s | UI idles/s (10-60 s) | presents/s | lagCuts/s | virtual s per real s | stopped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | prev | yes | 1 / 1 / 1 | 0 | 7.4 → 2.8 | 0.00 → 0.00 | 39.6 | 0.0 | 1514 | 135 | 182175 | **1152** | 0.411 | 0.996 | 200.6 | 7.7 | 0.0 | 1.035 | False |
| 2 | prev | yes | 1 / 1 / 1 | 95 | 3.4 → 4.2 | 0.00 → 0.00 | 41.3 | 0.0 | 1167 | 112 | 137289 | **912** | 0.398 | 0.993 | 196.4 | 7.2 | 0.0 | 1.016 | False |
| 3 | prev | yes | 1 / 1 / 1 | 0 | 3.9 → 3.4 | 0.00 → 0.00 | 40.7 | 0.0 | 1249 | 121 | 149503 | **1007** | 0.403 | 0.999 | 186.6 | 7.5 | 0.0 | 1.024 | False |
| 4 | prev | yes | 1 / 1 / 1 | 65 | 7.6 → 8.2 | 0.00 → 0.12 | 43.4 | 0.0 | 1025 | 103 | 122298 | **832** | 0.409 | 0.997 | 151.4 | 7.0 | 0.0 | 1.009 | False |
| 5 | prev | yes | 1 / 1 / 1 | 115 | 8.0 → 7.4 | 0.00 → 0.00 | 43.7 | 0.0 | 1075 | 104 | 125873 | **941** | 0.401 | 1.000 | 177.8 | 7.0 | 0.0 | 1.019 | False |
| 6 | prev | yes | 1 / 1 / 1 | 25 | 7.8 → 9.1 | 0.00 → 0.03 | 43.7 | 0.0 | 975 | 88 | 117211 | **844** | 0.396 | 1.000 | 200.4 | 7.2 | 0.0 | 1.009 | False |
| 1 | new | yes | 1 / 1 / 1 | 0 | 2.6 → 3.3 | 0.00 → 0.00 | 51.2 | 0.0 | 4 | 2 | 340 | **0** | 0.793 | 0.995 | 333.7 | 13.1 | 15.1 | 0.615 | False |
| 2 | new | yes | 1 / 1 / 1 | 0 | 3.2 → 2.6 | 0.00 → 5.86 | 51.4 | 0.0 | 0 | 0 | 0 | **0** | 0.830 | 0.993 | 269.1 | 13.2 | 15.7 | 0.607 | False |
| 3 | new | yes | 1 / 1 / 1 | 0 | 3.5 → 8.9 | 0.00 → 0.00 | 50.2 | 0.0 | 18 | 5 | 1870 | **8** | 0.762 | 0.995 | 292.7 | 12.0 | 15.0 | 0.569 | False |
| 4 | new | yes | 1 / 1 / 1 | 15 | 7.9 → 11.9 | 0.00 → 0.00 | 50.2 | 0.0 | 36 | 5 | 5935 | **0** | 0.765 | 0.994 | 321.2 | 12.4 | 14.9 | 0.574 | False |
| 5 | new | yes | 1 / 1 / 1 | 0 | 6.8 → 8.2 | 0.00 → 0.00 | 50.0 | 0.0 | 12 | 4 | 1189 | **4** | 0.749 | 0.998 | 326.3 | 12.6 | 15.2 | 0.570 | False |
| 6 | new | yes | 1 / 1 / 1 | 540 | 8.0 → 8.3 | 0.00 → 0.02 | 50.1 | 0.0 | 44 | 9 | 4854 | **17** | 0.682 | 0.997 | 380.0 | 11.3 | 14.7 | 0.553 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 14 | 3 | 1460 | 0.72 | 1.00 | 1079.2 | 10.0 | 0.0 | 1.00 |
| 0-10 s | new | 6 | 9 | 1 | 1016 | 0.96 | 1.00 | 1057.4 | 12.4 | 7.1 | 0.73 |
| 10-20 s | prev | 6 | 12 | 4 | 768 | 0.50 | 1.00 | 259.8 | 8.0 | 0.0 | 1.00 |
| 10-20 s | new | 6 | 0 | 0 | 6 | 0.89 | 1.00 | 252.5 | 12.6 | 16.4 | 0.61 |
| 20-30 s | prev | 6 | 188 | 18 | 18884 | 0.41 | 1.00 | 189.6 | 7.2 | 0.0 | 1.01 |
| 20-30 s | new | 6 | 0 | 0 | 1 | 0.84 | 1.00 | 385.7 | 13.4 | 14.4 | 0.69 |
| 30-40 s | prev | 6 | 379 | 34 | 47606 | 0.33 | 1.00 | 136.3 | 6.6 | 0.0 | 1.02 |
| 30-40 s | new | 6 | 0 | 0 | 85 | 0.76 | 1.00 | 311.4 | 12.5 | 16.6 | 0.53 |
| 40-50 s | prev | 6 | 474 | 34 | 57954 | 0.33 | 1.00 | 88.1 | 5.0 | 0.0 | 1.04 |
| 40-50 s | new | 6 | 0 | 0 | 7 | 0.70 | 1.00 | 291.6 | 12.1 | 15.7 | 0.50 |
| 50-60 s | prev | 6 | 109 | 21 | 14364 | 0.43 | 0.99 | 224.6 | 9.8 | 0.0 | 1.00 |
| 50-60 s | new | 6 | 0 | 0 | 8 | 0.66 | 0.97 | 352.2 | 11.6 | 13.2 | 0.57 |

Music-matched windows (the same music in both bundles): the music position is the virtual clock since the play mark (gpu1 matched-music.py); the bins are complete 10 s bins of music that each run reached within its 60 s of wall time. wall = the wall time the bin took; distinct frames = Unlock coverage per wall second (1.0 = real time); Unlocks / wall s; underrun quanta and episodes of the page and the engine gap frames over the wall interval of the bin (the 10 s snapshots and the 1 Hz engine reports, linearly interpolated). Median over the counted runs.

| music bin | bundle | n | wall time of the bin (s) | distinct frames per wall s | Unlocks per wall s | underrun quanta | episodes | engine gap frames |
|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 10.0 | 0.720 | 7.75 | 14 | 3 | 1537 |
| 0-10 s | new | 6 | 15.2 | 0.858 | 10.31 | 9 | 1 | 1023 |
| 10-20 s | prev | 6 | 10.0 | 0.495 | 5.33 | 12 | 4 | 1209 |
| 10-20 s | new | 6 | 14.5 | 0.797 | 9.01 | 1 | 0 | 119 |
| 20-30 s | prev | 6 | 9.9 | 0.411 | 4.42 | 186 | 18 | 22258 |
| 20-30 s | new | 6 | 19.1 | 0.729 | 8.01 | 1 | 1 | 306 |
| 30-40 s | prev | 6 | 9.8 | 0.332 | 3.57 | 372 | 33 | 44622 |
| 40-50 s | prev | 6 | 9.6 | 0.334 | 3.60 | 444 | 33 | 55361 |
| 50-60 s | prev | 6 | 10.0 | 0.442 | 4.80 | 147 | 23 | 15384 |

Old vs new at the same music position (counted runs; exact permutation test; only bins both bundles reached):

| music bin | measure | old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|
| 0-10 s | wall time of the bin (s) | 9.996 (6) | 15.237 (6) | +5.241 | 0.002 |
| 0-10 s | distinct frames per wall s | 0.720 (6) | 0.858 (6) | +0.138 | 0.002 |
| 0-10 s | underrun quanta | 14.383 (6) | 8.984 (6) | -5.399 | 0.771 |
| 0-10 s | episodes | 2.977 (6) | 1.498 (6) | -1.480 | 0.260 |
| 0-10 s | engine gap frames | 1536.816 (6) | 1023.000 (6) | -513.816 | 0.838 |
| 10-20 s | wall time of the bin (s) | 9.988 (6) | 14.508 (6) | +4.520 | 0.002 |
| 10-20 s | distinct frames per wall s | 0.495 (6) | 0.797 (6) | +0.301 | 0.002 |
| 10-20 s | underrun quanta | 11.966 (6) | 0.520 (6) | -11.447 | 0.004 |
| 10-20 s | episodes | 3.982 (6) | 0.260 (6) | -3.722 | 0.011 |
| 10-20 s | engine gap frames | 1209.396 (6) | 118.575 (6) | -1090.821 | 0.041 |
| 20-30 s | wall time of the bin (s) | 9.938 (6) | 19.080 (6) | +9.142 | 0.002 |
| 20-30 s | distinct frames per wall s | 0.411 (6) | 0.729 (6) | +0.318 | 0.002 |
| 20-30 s | underrun quanta | 186.217 (6) | 1.045 (6) | -185.172 | 0.002 |
| 20-30 s | episodes | 18.256 (6) | 0.523 (6) | -17.734 | 0.002 |
| 20-30 s | engine gap frames | 22257.545 (6) | 306.425 (6) | -21951.119 | 0.002 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 1121.102 (6) | 14.966 (6) | -1106.136 | 0.002 | 1121.102 (6) | 14.966 (6) | -1106.136 | 0.002 |
| underrun quanta 30-60 s (heavy interval) | 926.679 (6) | 1.998 (6) | -924.681 | 0.002 | 926.679 (6) | 1.998 (6) | -924.681 | 0.002 |
| episodes 0-60 s | 107.777 (6) | 4.489 (6) | -103.288 | 0.002 | 107.777 (6) | 4.489 (6) | -103.288 | 0.002 |
| engine gap frames 0-60 s | 131581.240 (6) | 1529.635 (6) | -130051.606 | 0.002 | 131581.240 (6) | 1529.635 (6) | -130051.606 | 0.002 |
| write rate mean 10-60 s | 0.402 (6) | 0.763 (6) | +0.361 | 0.002 | 0.402 (6) | 0.763 (6) | +0.361 | 0.002 |
| Worker CPU s (60 s) | 42.335 (6) | 50.205 (6) | +7.870 | 0.002 | 42.335 (6) | 50.205 (6) | +7.870 | 0.002 |
| UI idles/s (10-60 s) | 191.505 (6) | 323.760 (6) | +132.255 | 0.002 | 191.505 (6) | 323.760 (6) | +132.255 | 0.002 |
| presents/s (10-60 s) | 7.194 (6) | 12.485 (6) | +5.291 | 0.002 | 7.194 (6) | 12.485 (6) | +5.291 | 0.002 |
| lagCuts/s (10-60 s) | 0.000 (6) | 15.036 (6) | +15.036 | 0.002 | 0.000 (6) | 15.036 (6) | +15.036 | 0.002 |
| virtual s per real s (10-60 s) | 1.017 (6) | 0.572 (6) | -0.445 | 0.002 | 1.017 (6) | 0.572 (6) | -0.445 | 0.002 |

## 8 入力の true T（lat、再生中と idle の File クリック・Esc、E8 対 E9 交互）
### Input latency T (ms, page sent -> answer presented), chrome; counted = start load <=8 and I/O pressure avg10 = 0

| run | bundle | counted | other Chrome at the start / end / max during the run | waited (s) | load start -> end | I/O avg10 start -> end | section | input | n sent | answered | T median / p90 / p99 / max |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | prev | NO (load / I/O) | 0 / 0 / 0 | 305 | 8.66 -> 9.13 | 0 -> 0 | a-idle-menu | Esc (key down) | 35 | 35 | 2.2 / 3.5 / 16.6 / 16.6 |
| 1 | prev | NO (load / I/O) | 0 / 0 / 0 | 305 | 8.66 -> 9.13 | 0 -> 0 | a-idle-menu | File click (button down) | 36 | 36 | 7.9 / 11.6 / 25.2 / 25.2 |
| 1 | prev | NO (load / I/O) | 0 / 0 / 0 | 305 | 8.66 -> 9.13 | 0 -> 0 | b-play-menu | Esc (key down) | 36 | 36 | 26.2 / 105.0 / 179.5 / 179.5 |
| 1 | prev | NO (load / I/O) | 0 / 0 / 0 | 305 | 8.66 -> 9.13 | 0 -> 0 | b-play-menu | File click (button down) | 35 | 35 | 35.6 / 161.6 / 204.0 / 204.0 |
| 2 | prev | yes | 1 / 1 / 1 | 50 | 7.77 -> 15.06 | 0 -> 0 | a-idle-menu | Esc (key down) | 35 | 35 | 2.1 / 2.7 / 3.3 / 3.3 |
| 2 | prev | yes | 1 / 1 / 1 | 50 | 7.77 -> 15.06 | 0 -> 0 | a-idle-menu | File click (button down) | 36 | 36 | 7.8 / 12.3 / 27.8 / 27.8 |
| 2 | prev | yes | 1 / 1 / 1 | 50 | 7.77 -> 15.06 | 0 -> 0 | b-play-menu | Esc (key down) | 36 | 36 | 50.2 / 134.3 / 303.5 / 303.5 |
| 2 | prev | yes | 1 / 1 / 1 | 50 | 7.77 -> 15.06 | 0 -> 0 | b-play-menu | File click (button down) | 35 | 35 | 137.3 / 232.9 / 356.2 / 356.2 |
| 3 | prev | yes | 1 / 1 / 1 | 55 | 7.93 -> 11.32 | 0 -> 0 | a-idle-menu | Esc (key down) | 35 | 35 | 1.9 / 2.7 / 6.8 / 6.8 |
| 3 | prev | yes | 1 / 1 / 1 | 55 | 7.93 -> 11.32 | 0 -> 0 | a-idle-menu | File click (button down) | 36 | 36 | 7.3 / 11.1 / 19.6 / 19.6 |
| 3 | prev | yes | 1 / 1 / 1 | 55 | 7.93 -> 11.32 | 0 -> 0 | b-play-menu | Esc (key down) | 36 | 36 | 19.6 / 113.1 / 202.0 / 202.0 |
| 3 | prev | yes | 1 / 1 / 1 | 55 | 7.93 -> 11.32 | 0 -> 0 | b-play-menu | File click (button down) | 35 | 35 | 137.9 / 216.5 / 391.8 / 391.8 |
| 1 | new | NO (load / I/O) | 0 / 1 / 1 | 160 | 7.63 -> 17.57 | 0.18 -> 0.07 | a-idle-menu | Esc (key down) | 35 | 34 | 1.9 / 2.5 / 5.0 / 5.0 |
| 1 | new | NO (load / I/O) | 0 / 1 / 1 | 160 | 7.63 -> 17.57 | 0.18 -> 0.07 | a-idle-menu | File click (button down) | 36 | 36 | 7.0 / 12.8 / 24.8 / 24.8 |
| 1 | new | NO (load / I/O) | 0 / 1 / 1 | 160 | 7.63 -> 17.57 | 0.18 -> 0.07 | b-play-menu | Esc (key down) | 36 | 34 | 6.3 / 21.1 / 40.6 / 40.6 |
| 1 | new | NO (load / I/O) | 0 / 1 / 1 | 160 | 7.63 -> 17.57 | 0.18 -> 0.07 | b-play-menu | File click (button down) | 35 | 35 | 23.9 / 37.1 / 61.5 / 61.5 |
| 2 | new | yes | 1 / 0 / 1 | 686 | 7.21 -> 10.52 | 0 -> 0.02 | a-idle-menu | Esc (key down) | 35 | 29 | 2.2 / 3.0 / 4.6 / 4.6 |
| 2 | new | yes | 1 / 0 / 1 | 686 | 7.21 -> 10.52 | 0 -> 0.02 | a-idle-menu | File click (button down) | 36 | 36 | 7.8 / 12.2 / 25.6 / 25.6 |
| 2 | new | yes | 1 / 0 / 1 | 686 | 7.21 -> 10.52 | 0 -> 0.02 | b-play-menu | Esc (key down) | 36 | 36 | 7.0 / 17.6 / 36.1 / 36.1 |
| 2 | new | yes | 1 / 0 / 1 | 686 | 7.21 -> 10.52 | 0 -> 0.02 | b-play-menu | File click (button down) | 35 | 35 | 23.7 / 36.7 / 49.5 / 49.5 |
| 3 | new | yes | 1 / 1 / 1 | 100 | 7.86 -> 5.97 | 0 -> 0 | a-idle-menu | Esc (key down) | 35 | 34 | 2.2 / 3.0 / 5.3 / 5.3 |
| 3 | new | yes | 1 / 1 / 1 | 100 | 7.86 -> 5.97 | 0 -> 0 | a-idle-menu | File click (button down) | 36 | 36 | 7.0 / 9.5 / 23.6 / 23.6 |
| 3 | new | yes | 1 / 1 / 1 | 100 | 7.86 -> 5.97 | 0 -> 0 | b-play-menu | Esc (key down) | 36 | 35 | 4.4 / 11.1 / 29.3 / 29.3 |
| 3 | new | yes | 1 / 1 / 1 | 100 | 7.86 -> 5.97 | 0 -> 0 | b-play-menu | File click (button down) | 35 | 35 | 21.2 / 29.2 / 39.5 / 39.5 |

Per bundle (all answered records of the counted runs pooled):

| section | input | bundle | runs | answered | T median / p90 / p99 / max |
|---|---|---|---|---|---|
| a-idle-menu | Esc (key down) | prev | 2 | 70 | 2.0 / 2.7 / 6.8 / 6.8 |
| a-idle-menu | Esc (key down) | new | 2 | 63 | 2.2 / 3.0 / 5.3 / 5.3 |
| a-idle-menu | File click (button down) | prev | 2 | 72 | 7.7 / 11.1 / 27.8 / 27.8 |
| a-idle-menu | File click (button down) | new | 2 | 72 | 7.3 / 11.4 / 25.6 / 25.6 |
| b-play-menu | Esc (key down) | prev | 2 | 72 | 33.8 / 117.7 / 303.5 / 303.5 |
| b-play-menu | Esc (key down) | new | 2 | 71 | 5.4 / 16.9 / 36.1 / 36.1 |
| b-play-menu | File click (button down) | prev | 2 | 70 | 137.3 / 226.0 / 391.8 / 391.8 |
| b-play-menu | File click (button down) | new | 2 | 70 | 22.1 / 31.4 / 49.5 / 49.5 |

## 9 自己申告の逸脱
1. **14:06〜14:28 の pure の最初の 2 run（E8 の 1、E9 の 1）は失敗した（私の誤り）**。13:56 の tool セッションの再起動（『location shut down』）で page のサーバー 2 本と Xvfb :77 が落ちており、ページが起動せず（`waitIdle timeout, state no-app`）、10 分ずつ待って NOT OK。20 分を失い、気づいたのは 14:28。サーバーを立て直し、`pure`/`lat` の run の前に『サーバーが 200 を返さなければ即停止』の検査を足した。失敗した run は集計に入れていない（`out-failed-noapp/`）。13:56 より前に取った物（scenes・first idle・Export・logo・title・alpha7）は有効。
2. pure の最初の 1 run（E8 の 1）を、最初は『開始時に他の Chrome が 2 本いた（手で ps から読んだ）』として参考に分け E8 n=5 で報告したが、**これは E9 の最初の失敗した pure（14:06）の手書きの記録が、E9 の再実行（14:40）の同名の run に誤って当たっていた私の誤り**。観測プロセス（2 秒ごと）の記録では、再実行の E8 の 1 の run 中の他の Chrome は 1 本（opt1）で重なりなし。**E8 を n=6 に直し、表と数値を更新した**（E8 の中央値: underrun 1,075 → 1,121、書き込み率 0.401 → 0.402、仮想 s/実 s 1.016 → 1.017 ほか。結論と p 値は不変）。
3. lat の数えた run は E8 2、E9 2 組。E8 の 1 は開始時 load が 8.66、E9 の 1 は I/O 圧 avg10 が 0.18 で外れた（規則どおり）。3 組目は取れていない。
4. PM の連絡前に ff5ee429 対 E7b の P3 決着を『静かな窓待ち』で起動していた（12:54）。PM の指示（連絡までは重い Chrome の run を始めない）を見て 13:11 に停止。その 1 run（ff5ee429、開始 load 8.8 で数えない）は決着に使っていない。
5. 音楽位置で揃えた窓の underrun・episodes は、10 s ごとのページの snapshot を線形補間したもの（1 Hz のエンジン報告から gap フレーム）。bin が壁時計の 10 s 境界と合わない所は誤差を持つ（E10/E11 の pure は 2 s ごとの snapshot に変えた）。
6. 走っている `verify-bundle.sh` を編集して bash が構文エラーを出した（E8 の stretch、run は完了済みで影響なし）。それ以降は run 中に編集していない。
7. 並列に起動したため、サーバー起動が page のビルドより先に走って 1 回失敗した（ビルド後に起動し直した）。
8. PM の指示（alpha1 の A/B のため最大 12 分待つ）に合わせて、`wait-until.mjs`（開始時刻まで待つ小さな道具）で pure の開始を決めた。待ちの間に私は Chrome を動かしていない。

## PM 記入欄（ci1 は書かない）


## PM の決定と取り込み(2026-11-06 16:2x、docs のみ。製品の差分なし)

取り込み: ci1 の E9 の最終(本番 bundle `fl-engine-ci-62098b85`、Worker 2f390ad3、ページ 38efb398)の文書・結果・画・小さな表を `docs/investigations/ci1/0277/e9/` に置いた(音声の生データ `*-audio.json` 約 3.4 MB は入れない。原本は `lift/out-git/ci1-results/0277/e9/data/`)。実ブラウザ(Chrome 156 / Firefox 158、この機械、模擬入力)で、**依頼者の機械は 0 件**、本番 R2 の `perf/` に beacon は 0 件。

事実として受け取る:
1. 判定基準の 4 つを満たした: 停止なし(30/30 ほか)、Export 22,863,828 B・sha256 `381cdb5a…f979`(ci1 の sha256sum)、File ×100 が Chrome・Firefox とも 100/100、first idle は E8 比 Chrome +52 ± 94 ms、Firefox +194 ± 116 ms(各 n=6、基準 +500 ms 以内)。復元ボタンの CDP 実マウスクリックは通り、復元後の題は 6 変種すべて `(none)`(決定 0291 = FL 自身の挙動)。
2. 純再生(Chrome、数えた run: E8 5・E9 6): 60 s の underrun 1,075 → 15 quanta(p = 0.002)、書き込み率(壁時計 10〜60 s)0.401 → 0.763、仮想 s/実 s 1.016 → 0.572、lagCuts 0 → 15/s。**音楽位置(仮想時計)で揃えても E9 の方が良い**: 音楽 0〜10 / 10〜20 / 20〜30 s の壁 1 秒あたりの distinct フレームは E8 0.715 / 0.495 / 0.412、E9 0.858 / 0.797 / 0.729、同じ区間の underrun は 20〜30 s で 162 → 1 quanta。**ただし E9 は 60 s で音楽の約 30〜37 s にしか届かない**ので、E8 で最も重い音楽 30〜60 s は E9 では測れていない(5 分再生が要る → E11 の項目。opt1 の E11 の 5 分再生では最大 CPU/Unlock 85〜86 ms で壁の 92.9 ms 未満、単発の静かな run)。
3. 入力の true T(再生中、Chrome、数えた run: 各 2 組。3 組目は開始 load 8.66・I/O 圧 0.18 で外れた): File クリックの中央値 137 → 22 ms(p90 226 → 31、p99 392 → 50)、Esc 34 → 5.4 ms、idle の File クリック 7.7 → 7.3 ms、Esc 2.0 → 2.2 ms で不変。
4. UI の idle/s・presents/s(製品のページの `driverStats.idleCount`・`deskStats.presents`)は E9 で増える(7.2 → 12.5 presents/s)。opt1 の「UI slice」(65〜68 → 30〜38/s、計測用ページ)とは別の量で、E9 で減るのは仮想時間あたりの tick(仮想 s/実 s 0.572)。

判断:
1. **E9 の本番確認は合格**。P2(再生中の入力遅延)と P3(underrun・音楽 0〜30 s の書き込み)は E8 より大幅に改善した。P2 の「120 fps」と P3 の重い区間(音楽 30 s 以降)は E9 では未達/未測定で、E11 以降の本番確認(ci1 の E11 キット、5 分再生)と gpu1 の 0297 で取る。
2. 以後の P3 の比較はすべて**音楽位置で揃えた窓を併記**する(決定 0289 の PM 節)。
3. 未確認: Firefox の pure(E8 対 E9)と ff5ee429 対 E7b の P3 の決着(E11 の後)、lid 8 の first idle より前、E9 が届かない音楽 30 s 以降。

(訂正、2026-11-06 20:35、PM: ci1 の 20:36 の訂正〔手書きの重なり記録の取り違え〕で、本文の e9 の pure の数値が直った。上の PM の節の pure の数字のうち E8/E10 側の n・underrun・書き込み率・仮想 s/実 s は、本文の訂正後の表が正。結論は不変。)
