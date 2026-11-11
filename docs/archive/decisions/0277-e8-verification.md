# ci1 → PM 決定 0277（E8）: E8 の本番 bundle `fl-engine-ci-875bfd62`（Worker 4a03e043、ページ = c74b9194）の実ブラウザ確認 — 停止なし、Export 一致、File ×100、first idle、alpha7 の窓内の平行移動、最小化→`#taskbar-restore`→復元、P3（E7b 対 E8）

## 先頭の 3 点（実エンジン・実ブラウザ Chrome 156 / Firefox 158、この機械・headless・模擬入力。**依頼者の機械は 0 件**。代役・再生・推測なし。Firefox の値は全て WebGL あり（`DISPLAY=:77`、beacon の `webglRenderer` = llvmpipe、`desk.mode` = gl）。各値に load と I/O 圧 avg10 を併記）
1. **測ったこと**: 配信バケットの bundle を読み取りで取得し manifest の全ファイルの sha256 を照合（wasm `0a54e7f86358…` 142,586,692 B、glue `ddaf06700da6…` 744,344 B、seed・dll は E7b と同じ）。同じページ（c74b9194）で E7b `fl-engine-ci-d9c160aa` と E8 を配信して、15 シーン × 2 ブラウザ、File ×100、first idle（単独・冷間・交互）、Export、alpha7 の画（E7b と並べる）、最小化 → `#taskbar-restore` → 復元（マウス・キーボード）、P3（参照 zip を 60 s 再生、10 s 窓、旧新交互）。
2. **結論**: 決定 0277 の E8 の判定項目を**満たした**。停止なし（Chrome・Firefox 各 15 シーン、Export、P3 の再生）、Export の WAV が参照と一致（22,863,828 B、sha256 `381cdb5a…f979`）、File ×100 は Chrome・Firefox とも 100/100、first idle は E8 の方が速い向き（Chrome 5,156 対 5,321 ms、Firefox 4,486 対 4,823 ms）。**P3 は E7b と差がない**（Chrome、p ≥ 0.24。Firefox は数えた run 各 2 で判定できないが差も見えない）。alpha7: 主窓は @0,0 1920x1080 で不変、窓の中身は 4〜6 px 平行移動（alpha1 の予測した『FL 自身が最大化を知らされる』の帰結、崩れではない）。最小化 → `#taskbar-restore` → 復元は本番 bundle でマウス（FL の最小化ボタンのクリック）→ ページのボタン（DOM の click、実マウス、Tab → Enter）で通った。
3. **未確認**: オーナーの機械（0 件）。復元後のプレイリストの題が『Pattern 1』から『(none)』になる原因（再現条件だけ第 6 節に表にした。原因は調べていない）。Firefox の P3 は E7b 対 E8 を 3 組流したが、数えられた run（開始時 load ≤8・I/O 圧 0）は各 2 で、差は検出できない（p ≥ 0.67、run ごとのばらつきが大きい。E7b の 1 run は underrun 8 quanta、他は 400 台）。ff5ee429 対 E7b の P3 決着は別の報告。

## 1 判定基準（E8 対 E7b、run の load と I/O 圧は各表）
| criterion | value | pass |
|---|---|---|
| Firefox has WebGL (preflight) | preflight: webglRenderer llvmpipe, or similar desk.mode gl first idle 7533 | yes |
| No stop (11 scenes + more, both browsers, play, export; d7 is the intended exit) | stops: none; not ok: none; no result at all (the harness did not start): none | yes |
| Export WAV equals the reference | 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` (expected 22863828 B, `381cdb5acf891a16…`) | yes |
| First idle ≤ previous + 500 ms (chrome, WebGL runs) | all runs: new median 5156 ms, previous 5321 ms, difference -164 ms; quiet runs only (load ≤12, I/O avg10 <1; n new 6 / prev 5): new 5156, previous 5320, difference -164 ms | yes |
| First idle ≤ previous + 500 ms (firefox, WebGL runs) | all runs: new median 4486 ms, previous 4823 ms, difference -338 ms; quiet runs only (load ≤12, I/O avg10 <1; n new 5 / prev 5): new 4474, previous 4711, difference -237 ms | yes |
| File menu ×100 (chrome) | 100/100 | yes |
| File menu ×100 (firefox) | 100/100 | yes |

## 2 15 シーン（Chrome・Firefox、各行の load と I/O 圧。2 ジョブ並行のため first idle は参考値）
| browser | scene | result | first idle ms | File ×100 | load, I/O pressure (start→end) |
|---|---|---|---|---|---|
| chrome | d1-sampler-envelope | ok | 5952 | - | load 6.6→6.9, io 0.0→0.1 |
| chrome | d2-browser-folder | ok | 5240 | - | load 6.9→8.3, io 0.1→0.0 |
| chrome | d3-settings-all-tabs | ok | 4856 | - | load 8.3→7.4, io 0.0→0.0 |
| chrome | d4-playlist-middle-drag | ok | 4975 | - | load 7.4→6.9, io 0.0→0.0 |
| chrome | d5-mixer-front | ok | 4736 | - | load 6.9→6.2, io 0.0→0.2 |
| chrome | d6-playlist-close | ok | 4779 | - | load 6.2→5.9, io 0.2→0.0 |
| chrome | d7-main-close | ok (intended exit) | 6473 | - | load 5.9→7.1, io 0.0→0.0 |
| chrome | d8-file-menu-100 | ok | 4835 | 100/100 | load 7.1→11.0, io 0.0→0.0 |
| chrome | d9-keys | ok | 4883 | - | load 11.0→14.7, io 0.0→0.0 |
| chrome | d10-pianoroll-f12 | ok | 6426 | - | load 14.7→14.9, io 0.0→0.1 |
| chrome | d11-menu-hover-escape | ok | 6334 | - | load 15.0→15.0, io 0.0→0.0 |
| chrome | add-menu-simple-fade | ok | 5436 | - | load 16.4→12.9, io 0.0→0.0 |
| chrome | picker-cost | ok | 4997 | - | load 11.6→10.3, io 0.0→0.0 |
| chrome | picker-f8-fade | ok | 4702 | - | load 12.9→11.6, io 0.0→0.0 |
| chrome | plus-more-plugins | ok | 6758 | - | load 15.2→16.4, io 0.0→0.0 |
| firefox | d1-sampler-envelope | ok | 5283 | - | load 6.6→6.9, io 0.2→0.1 |
| firefox | d2-browser-folder | ok | 4883 | - | load 8.4→8.6, io 0.1→0.0 |
| firefox | d3-settings-all-tabs | ok | 4867 | - | load 9.4→6.8, io 0.0→0.0 |
| firefox | d4-playlist-middle-drag | ok | 4987 | - | load 6.9→6.5, io 0.0→0.0 |
| firefox | d5-mixer-front | ok | 4609 | - | load 2.7→2.4, io 0.0→0.0 |
| firefox | d6-playlist-close | ok | 5267 | - | load 6.2→5.9, io 0.2→0.0 |
| firefox | d7-main-close | ok (intended exit) | 5695 | - | load 7.0→7.1, io 0.0→0.0 |
| firefox | d8-file-menu-100 | ok | 5230 | 100/100 | load 7.2→14.3, io 0.0→0.0 |
| firefox | d9-keys | ok | 5471 | - | load 14.3→15.1, io 0.2→0.0 |
| firefox | d10-pianoroll-f12 | ok | 6532 | - | load 15.0→15.6, io 0.0→0.0 |
| firefox | d11-menu-hover-escape | ok | 4929 | - | load 15.6→12.2, io 0.0→0.0 |
| firefox | add-menu-simple-fade | ok | 4638 | - | load 11.2→9.2, io 0.0→0.0 |
| firefox | picker-cost | ok | 4657 | - | load 8.5→7.3, io 0.0→0.0 |
| firefox | picker-f8-fade | ok | 4944 | - | load 9.2→8.5, io 0.0→0.0 |
| firefox | plus-more-plugins | ok | 4690 | - | load 12.3→11.2, io 0.0→0.0 |


The scenes ran as two parallel jobs (Chrome and Firefox), so the first idle values of this table are reference values (the load is next to each).

## 3 first idle（単独 run・冷間・毎回新しいプロファイル・E7b 対 E8 交互。prev = E7b、new = E8）
| browser | bundle | run | first idle ms | load, I/O pressure at the start | at the end | quiet (load ≤12, I/O <1) | webglRenderer | desk.mode |
|---|---|---|---|---|---|---|---|---|
| chrome | prev | 1 | 5322 | 7.75 / io 1.04 | 7.63 / io 0.38 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 1 | 5352 | 7.63 / io 0.38 | 7.15 / io 0.14 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 2 | 5075 | 7.15 / io 0.14 | 8.18 / io 0.05 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 2 | 5290 | 8.18 / io 0.05 | 7.61 / io 0.01 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 3 | 5320 | 7.61 / io 0.01 | 7.13 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 3 | 5238 | 7.13 / io 0.00 | 7.23 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 4 | 5299 | 7.23 / io 0.00 | 6.73 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 4 | 5577 | 6.73 / io 0.00 | 7.82 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 5 | 6006 | 7.82 / io 0.00 | 8.72 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 5 | 4991 | 8.72 / io 0.00 | 7.83 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 6 | 4955 | 7.83 / io 0.00 | 7.30 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 6 | 4984 | 7.30 / io 0.00 | 7.04 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| firefox | prev | 1 | 4711 | 7.04 / io 0.00 | 5.99 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 1 | 4411 | 5.99 / io 0.00 | 5.45 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 2 | 4417 | 5.45 / io 0.00 | 5.00 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 2 | 4665 | 5.00 / io 0.00 | 5.56 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 3 | 5097 | 5.56 / io 0.00 | 5.81 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 3 | 5002 | 5.81 / io 0.00 | 7.55 / io 0.08 | yes | llvmpipe, or similar | gl |
| firefox | new | 4 | 5288 | 7.55 / io 0.08 | 12.06 / io 0.02 | no (reference) | llvmpipe, or similar | gl |
| firefox | prev | 4 | 4935 | 12.06 / io 0.02 | 10.58 / io 0.00 | no (reference) | llvmpipe, or similar | gl |
| firefox | prev | 5 | 5098 | 10.58 / io 0.00 | 9.56 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 5 | 4474 | 9.56 / io 0.00 | 8.55 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 6 | 4497 | 8.55 / io 0.00 | 7.61 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 6 | 4623 | 7.61 / io 0.00 | 7.56 / io 0.08 | yes | llvmpipe, or similar | gl |

| browser | mode | previous median (n) | new median (n) | difference |
|---|---|---|---|---|
| chrome | gl | 5321 (6) | 5156 (6) | -164 |
| firefox | gl | 4823 (6) | 4486 (6) | -338 |

## 4 Export（Chrome、参照プロジェクトを開き Ctrl+R）
result ok; load 4.8→8.8, io 0.0→0.0; seconds 371
- WAV `serina1-serum2.wav`: 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`; expected 22863828 B / `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`: MATCH

## 5 alpha7: 起動直後の画（Chrome、1920x1080 ゲスト）を E7b と並べた
- 窓表: 主窓 `TFruityLoopsMainForm 'FL Studio 2026'` は E7b・E8 とも **@0,0 1920x1080 z0**（可視の窓は 1 つで同一）。
- 画素: 全 2,073,600 画素中 709,324（34.2 %）が違う。原因は窓の中身の平行移動（部分ごとの最良の移動、残差 0〜8 画素）:

| 部分 | E7b から E8 への移動 |
|---|---|
| メニュー（FILE EDIT…）、Browser のアイコン | 左 6、上 6 |
| チャンネルラック、Sampler の行、プレイリストの Track 名、パターン一覧 | 左 4、上 6 |
| mixer（Master と Insert 1〜9） | 左 4、**下 6** |
| 右上の最小化/最大化/閉じる、プレイリストの閉じる | **右 5〜6**、上 6 |

- 目視: ツールバーが上端から始まる（E7b は上に 6 px の外枠があった）。右上の窓ボタンは右端に寄る（閉じるの右端 x≈1912 < 1920 で見える）。メニュー文字・transport・Browser・Channel rack・Sampler・mixer・プレイリストは全て見え、欠けて読めない物はない。下端は Browser の水平スクロールバーの下半分が 6 px 切れる。
- 解釈（ci1 の読み、PM が『許容、直さない』と判断）: FL の子窓が最大化した窓の外枠ぶん（約 6 px）だけ外へ配置された。実 Windows の最大化窓と同じ見え方。
- 3 枚の画の場所（`ci1-results/0277/e8/shots/`）: `alpha7-E7b-d9c160aa-main-1920x1080.png`（E7b）、`alpha7-E8-875bfd62-main-1920x1080.png`（E8）、`alpha7-E7b-vs-E8-diff.png`（差分: 違う画素が白、他は E7b を暗く）。拡大して上下に並べた切り出し（上が E7b、下が E8）: `cmp-top-left.png`、`cmp-top-right.png`、`cmp-bottom-left.png`、`cmp-bottom-right.png`。

## 6 最小化 → `#taskbar-restore` → 復元（Chrome、本番 bundle E8）
FL の最小化ボタン（ゲスト座標 1863,20 付近）をマウスでクリックし、ページのボタンで復元:

| 時点 | 窓表（可視） | `#taskbar-restore` の DOM | フォーカス |
|---|---|---|---|
| 起動直後 | 主窓 @0,0 1920x1080 の 1 窓 | `hidden` = true、display none | `desktop` |
| 最小化の 3 s 後 | **0 窓** | `hidden` = false、display inline-block、位置 x12 y1071 幅 182 高さ 33（ページ座標。desktop は y36 から 1920x1080 なので左下）、数は 1 | `desktop` |
| Tab を押した後（キーボードの経路） | 0 窓 | 同じ（x12 y1071） | **`taskbar-restore`** |
| クリック（DOM の click）/ Enter の 3 s 後 | **主窓 @0,0 1920x1080 の 1 窓（元に戻る）** | `hidden` = true | `desktop`（フォーカスが戻る） |

- エンジンは停止せず（`engine-idle`）。復元後の画: ツールバーが右へ約 15 px ずれる（既知の AlignControls）。ほかに、復元後も最小化ボタンが青くハイライト（ポインタが残る）、ヒント欄に『Minimize』が出る。
- 復元後のプレイリストの題が『Pattern 1』から『(none)』になる件の再現条件（原因は調べていない。PM が alpha1 に native で見てもらう）。窓表（page の `deskDumpBase64(true)`）には主窓 1 つしか無く（`n` = 1、題 `FL Studio 2026`、最小化中は 0 窓）、プレイリストの題は FL が描く文字なので、**画で**比べた（復元後の画の見出し部分を、起動直後の画と、前に取った『(none)』の画と画素比較）。5 変種:

| 変種 | 復元後のプレイリスト見出し（crop 1125-1400 x 88-106）の start との差 | 既知の『(none)』との差 | 復元後と起動直後の画で違う画素（全体 / 行の範囲） | load / I/O 圧（開始） |
|---|---|---|---|---|
| dom-0.5s | 408 画素 | 0 画素 | 41691 / y 0..102 | 15.67 / 0.01 |
| dom-15s | 408 画素 | 0 画素 | 41699 / y 0..102 | 14.92 / 0.13 |
| dom-3s | 408 画素 | 0 画素 | 41691 / y 0..102 | 13.31 / 0 |
| kbd-3s | 408 画素 | 0 画素 | 41667 / y 0..102 | 16.12 / 0 |
| mouse-3s | 408 画素 | 0 画素 | 41691 / y 0..102 | 15.59 / 0 |

- **5 変種すべてで『(none)』になる（既知の『(none)』の画と 0 画素差、起動直後の『Pattern 1』とは 408 画素差）**。最小化から復元までの時間（0.5 s・3 s・15 s）にも、復元の方法（DOM の click、Tab → Enter、CDP の実マウスクリック）にも依らない。復元後の画と起動直後の画で違う画素は 41,667〜41,699 で、**全て y 0〜102 の行**（ツールバーの 15 px 右ずれ、ヒント欄の『Minimize』、プレイリストの見出し）。それ以外の窓（Browser・Channel rack・mixer・プレイリストの本体）の画素は起動直後と同一。
- 実マウスクリック（CDP の `Input.dispatchMouseEvent`、ゲスト座標 103,1051 = ボタンの上）の経路は変種 `mouse-3s`: ボタンが押され、窓が戻った（ボタン hidden、可視の窓が 1 つ、フォーカスが desktop）。これで E8 の本番 bundle でも実マウスクリックが通ったことが確認できた。

## 7 P3: E7b 対 E8（参照 zip を開き Space で 60 s、10 s 窓、旧新交互。開始時 load ≤8 かつ I/O 圧 0 の run だけ数える）
### P3 decider, chrome: old (previous) vs new, windows of 10 s from the play mark; counted = start load ≤8 and I/O pressure avg10 = 0

| run | bundle | counted | load start → end | I/O avg10 start → end | Worker CPU s (60 s) | main CPU s | underrun quanta 0-60 s | episodes 0-60 s | engine gap frames 0-60 s | **underrun 30-60 s** | write rate mean 10-60 s | consumed rate mean 10-60 s | UI idles/s (10-60 s) | presents/s | lagCuts/s | virtual s per real s | stopped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | prev | NO (not counted) | 8.6 → 5.5 | - → - | 42.7 | 0.0 | 1105 | 99 | 132877 | **904** | 0.405 | 0.999 | - | - | - | 1.013 | False |
| 2 | prev | yes | 5.5 → 5.5 | 0.00 → 0.00 | 44.7 | 0.0 | 960 | 100 | 114215 | **809** | 0.405 | 0.998 | - | - | - | 1.007 | False |
| 3 | prev | yes | 5.3 → 5.2 | 0.00 → 0.00 | 45.6 | 0.0 | 766 | 85 | 90708 | **656** | 0.398 | 1.000 | - | - | - | 0.997 | False |
| 4 | prev | yes | 6.8 → 5.4 | 0.00 → 0.00 | 47.3 | 0.0 | 657 | 69 | 78079 | **591** | 0.392 | 0.993 | - | - | - | 1.001 | False |
| 5 | prev | yes | 5.6 → 5.4 | 0.00 → 0.00 | 45.9 | 0.0 | 785 | 85 | 92735 | **687** | 0.403 | 0.999 | - | - | - | 1.007 | False |
| 6 | prev | yes | 5.8 → 3.1 | 0.00 → 0.00 | 44.9 | 0.0 | 966 | 88 | 116072 | **810** | 0.401 | 0.994 | - | - | - | 1.006 | False |
| 1 | new | yes | 6.2 → 7.7 | - → - | 46.7 | 0.0 | 605 | 66 | 71709 | **571** | 0.398 | 0.998 | - | - | - | 1.002 | False |
| 2 | new | NO (not counted) | 7.7 → 5.5 | 0.04 → 0.00 | 44.0 | 0.0 | 927 | 90 | 110901 | **787** | 0.405 | 0.992 | - | - | - | 1.007 | False |
| 3 | new | yes | 5.2 → 10.1 | 0.00 → 0.00 | 50.1 | 0.0 | 158 | 35 | 17178 | **115** | 0.386 | 0.997 | - | - | - | 0.990 | False |
| 4 | new | NO (not counted) | 10.8 → 6.8 | 0.00 → 0.00 | 45.3 | 0.0 | 1003 | 95 | 120081 | **849** | 0.403 | 0.993 | - | - | - | 1.011 | False |
| 5 | new | yes | 5.2 → 4.9 | 0.00 → 0.00 | 45.0 | 0.0 | 845 | 89 | 100438 | **736** | 0.396 | 1.000 | - | - | - | 1.004 | False |
| 6 | new | yes | 4.9 → 6.0 | 0.00 → 0.00 | 46.0 | 0.0 | 870 | 89 | 103332 | **735** | 0.401 | 0.997 | - | - | - | 1.006 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 5 | 10 | 3 | 1015 | 0.72 | 1.00 | - | - | - | 1.00 |
| 0-10 s | new | 4 | 13 | 3 | 1436 | 0.72 | 1.00 | - | - | - | 1.00 |
| 10-20 s | prev | 5 | 0 | 0 | 7 | 0.48 | 1.00 | - | - | - | 1.00 |
| 10-20 s | new | 4 | 0 | 0 | 8 | 0.48 | 1.00 | - | - | - | 1.00 |
| 20-30 s | prev | 5 | 99 | 15 | 10850 | 0.41 | 1.00 | - | - | - | 1.00 |
| 20-30 s | new | 4 | 65 | 9 | 6692 | 0.41 | 1.00 | - | - | - | 1.00 |
| 30-40 s | prev | 5 | 244 | 29 | 28232 | 0.33 | 1.00 | - | - | - | 1.00 |
| 30-40 s | new | 4 | 185 | 23 | 20370 | 0.33 | 1.00 | - | - | - | 1.00 |
| 40-50 s | prev | 5 | 426 | 33 | 50302 | 0.33 | 1.00 | - | - | - | 1.02 |
| 40-50 s | new | 4 | 419 | 33 | 49120 | 0.33 | 1.00 | - | - | - | 1.01 |
| 50-60 s | prev | 5 | 35 | 4 | 5904 | 0.45 | 0.99 | - | - | - | 0.99 |
| 50-60 s | new | 4 | 31 | 7 | 7641 | 0.44 | 0.99 | - | - | - | 0.99 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 784.838 (5) | 724.784 (4) | -60.054 | 0.238 | 872.259 (6) | 857.463 (6) | -14.796 | 0.407 |
| underrun quanta 30-60 s (heavy interval) | 687.494 (5) | 653.023 (4) | -34.471 | 0.262 | 748.020 (6) | 735.682 (6) | -12.337 | 0.468 |
| episodes 0-60 s | 84.959 (5) | 77.421 (4) | -7.538 | 0.278 | 86.467 (6) | 88.910 (6) | +2.443 | 0.381 |
| engine gap frames 0-60 s | 92734.646 (5) | 86073.067 (4) | -6661.579 | 0.238 | 103474.769 (6) | 101884.855 (6) | -1589.914 | 0.407 |
| write rate mean 10-60 s | 0.401 (5) | 0.397 (4) | -0.005 | 0.325 | 0.402 (6) | 0.399 (6) | -0.003 | 0.558 |
| Worker CPU s (60 s) | 45.560 (5) | 46.340 (4) | +0.780 | 0.294 | 45.245 (6) | 45.635 (6) | +0.390 | 0.396 |
| UI idles/s (10-60 s) | - | - | - | - | - | - | - | - |
| presents/s (10-60 s) | - | - | - | - | - | - | - | - |
| lagCuts/s (10-60 s) | - | - | - | - | - | - | - | - |
| virtual s per real s (10-60 s) | 1.006 (5) | 1.003 (4) | -0.003 | 0.500 | 1.007 (6) | 1.005 (6) | -0.002 | 0.671 |

### P3 decider, firefox: old (previous) vs new, windows of 10 s from the play mark; counted = start load ≤8 and I/O pressure avg10 = 0

| run | bundle | counted | load start → end | I/O avg10 start → end | Worker CPU s (60 s) | main CPU s | underrun quanta 0-60 s | episodes 0-60 s | engine gap frames 0-60 s | **underrun 30-60 s** | write rate mean 10-60 s | consumed rate mean 10-60 s | UI idles/s (10-60 s) | presents/s | lagCuts/s | virtual s per real s | stopped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | prev | yes | 2.4 → 4.1 | 0.00 → 0.00 | 49.0 | 0.0 | 443 | 60 | 51520 | **399** | 0.394 | 0.996 | - | - | - | 1.000 | False |
| 2 | prev | yes | 6.7 → 12.3 | 0.00 → 0.01 | 56.8 | 0.0 | 8 | 2 | 837 | **8** | 0.312 | 0.996 | - | - | - | 0.892 | False |
| 3 | prev | NO (not counted) | 13.3 → 11.9 | 0.18 → 0.00 | 49.0 | 0.0 | 451 | 64 | 52144 | **436** | 0.381 | 1.000 | - | - | - | 1.000 | False |
| 1 | new | yes | 4.0 → 4.8 | 0.00 → 0.00 | 49.6 | 0.0 | 425 | 62 | 49055 | **379** | 0.388 | 1.000 | - | - | - | 1.000 | False |
| 2 | new | yes | 4.8 → 5.7 | 0.00 → 0.00 | 50.7 | 0.0 | 299 | 45 | 34270 | **289** | 0.385 | 0.999 | - | - | - | 1.000 | False |
| 3 | new | NO (not counted) | 12.1 → 13.8 | 0.00 → 0.00 | 53.8 | 0.0 | 64 | 19 | 10197 | **64** | 0.359 | 0.997 | - | - | - | 0.997 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 2 | 0 | 0 | 0 | 0.69 | 1.00 | - | - | - | 1.00 |
| 0-10 s | new | 2 | 0 | 0 | 0 | 0.72 | 1.00 | - | - | - | 1.00 |
| 10-20 s | prev | 2 | 0 | 0 | 0 | 0.43 | 1.00 | - | - | - | 1.00 |
| 10-20 s | new | 2 | 0 | 0 | 0 | 0.44 | 1.00 | - | - | - | 1.00 |
| 20-30 s | prev | 2 | 22 | 3 | 2490 | 0.34 | 1.00 | - | - | - | 0.84 |
| 20-30 s | new | 2 | 28 | 5 | 1626 | 0.42 | 1.00 | - | - | - | 1.00 |
| 30-40 s | prev | 2 | 76 | 12 | 8752 | 0.29 | 1.00 | - | - | - | 0.86 |
| 30-40 s | new | 2 | 114 | 18 | 13619 | 0.33 | 1.00 | - | - | - | 1.00 |
| 40-50 s | prev | 2 | 114 | 13 | 13485 | 0.33 | 1.00 | - | - | - | 1.01 |
| 40-50 s | new | 2 | 196 | 26 | 22528 | 0.33 | 1.00 | - | - | - | 1.00 |
| 50-60 s | prev | 2 | 13 | 2 | 1451 | 0.37 | 0.98 | - | - | - | 1.03 |
| 50-60 s | new | 2 | 25 | 4 | 3889 | 0.41 | 1.00 | - | - | - | 0.99 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 225.328 (2) | 362.185 (2) | +136.858 | 1.000 | 442.798 (3) | 299.472 (3) | -143.326 | 0.800 |
| underrun quanta 30-60 s (heavy interval) | 203.542 (2) | 333.946 (2) | +130.403 | 1.000 | 399.227 (3) | 288.583 (3) | -110.644 | 0.700 |
| episodes 0-60 s | 30.969 (2) | 53.445 (2) | +22.476 | 0.667 | 59.973 (3) | 44.914 (3) | -15.059 | 1.000 |
| engine gap frames 0-60 s | 26178.645 (2) | 41662.521 (2) | +15483.876 | 1.000 | 51520.437 (3) | 34269.953 (3) | -17250.484 | 0.800 |
| write rate mean 10-60 s | 0.353 (2) | 0.386 (2) | +0.033 | 1.000 | 0.381 (3) | 0.385 (3) | +0.004 | 0.800 |
| Worker CPU s (60 s) | 52.865 (2) | 50.160 (2) | -2.705 | 1.000 | 49.030 (3) | 50.690 (3) | +1.660 | 1.000 |
| UI idles/s (10-60 s) | - | - | - | - | - | - | - | - |
| presents/s (10-60 s) | - | - | - | - | - | - | - | - |
| lagCuts/s (10-60 s) | - | - | - | - | - | - | - | - |
| virtual s per real s (10-60 s) | 0.946 (2) | 1.000 (2) | +0.054 | 0.667 | 1.000 (3) | 1.000 (3) | -0.000 | 0.800 |

## 8 自己申告の逸脱
1. Firefox の d5 の最初の起動失敗（geckodriver のポート衝突 `Address already in use`、ハーネスの起動失敗）を最初の集計で見落とした（『全て ok』は結果のある行だけを数えていた）。報告前に気づき、集計に『結果なし』の行を足して再実行した（ok）。
2. P3 の最初の 1 run（Chrome）は I/O 圧の記録が無い（ドライバの記録漏れ）。以降は記録あり。
3. 復元の経路の一つは DOM の `.click()`（ページのボタンの実クリックではない）。実マウスクリックは第 6 節の変種 `mouse-3s`。

## PM 記入欄（ci1 は書かない）

### PM の決定と取り込み(2026-11-06 13:00、docs のみ)

取り込み: ci1 の E8 本番 bundle の確認(第 1 報 12:27、ingest 用文書 12:41)の文書・結果表・データ(各 run の report と P3・first idle・minrestore の表。サイズの大きい生の audio JSON は入れない)・画 4 枚を `docs/investigations/ci1/0277/e8/` に置いた。測定は Chrome 156 と Firefox 158(Xvfb :77、WebGL あり)の本番 E8 bundle `fl-engine-ci-875bfd62`(Worker `4a03e043-212d-4392-b35a-a1cbb60fa726`)で、依頼者の機械・実 Windows は 0 件。
判断:
1. **E8 の本番 bundle は判定基準を満たした**(15 シーン × 2 ブラウザと Export と 60 s 再生で停止なし、Export 22,863,828 B sha256 `381cdb5a…f979`、File メニュー ×100 が両ブラウザで 100/100、first idle は E7b に対し Chrome 5,156 対 5,321 ms、Firefox 4,486 対 4,823 ms で悪化なし)。
2. **P3 は E7b と E8 で差が無い**(Chrome 6 組: 書き込み率 10〜60 s の平均 0.401 対 0.397、underrun 785 対 725 量子、episodes 85 対 77。Firefox は run ごとのばらつきが大きく判定できない)。E8 は hyb リストが E7b と同じなので、これは想定どおり。
3. **起動直後の窓の中身が E7b に対し 4〜6 px 平行移動する**(FL が最大化と知らされるため、alpha1 の予測)。主窓の矩形 @0,0 1920x1080 は不変、読めなくなる物は無い。**許容する**(PM 判断。FL の挙動であり、直せば FL を補うことになる)。
4. **最小化 → `#taskbar-restore` → 復元は本番 bundle で通った**(DOM click、Tab → Enter、CDP の実マウスクリックの 3 経路。窓表 1 → 0 → 1)。復元後の toolbar の +15 px は FL 自身の AlignControls(決定 0282。HEAD でも出る。直さない)。**プレイリストの題が『Pattern 1』から『(none)』になる件は、5 変種(待ち 0.5 / 3 / 15 s、Tab → Enter、実マウス)すべてで再現し、原因は未調査**(alpha1 に native で調べさせる)。
5. 未確認: 『(none)』の原因、Firefox の P3、ff5ee429 対 E7b の underrun の決着(静かな窓が要る)、E9 の本番 bundle での同じ確認(デプロイ後)。
