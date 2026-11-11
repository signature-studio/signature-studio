# 描画面(BGRX)を canvas に出す経路: JS 側の試作と測定(エンジンの wasm なし)

作成: 2026-11-01(基準 HEAD 0c39ffd、ブランチ main)。**何も決めない**(決めるのは PM)。決定 0126(別の作業、実行中)のワークツリー `lift/out-fruity11/` は**読み取りだけ**で使った。リポジトリの既存ファイルは編集していない。新規は `lift/harness/web/present/`(コード、契約 `CONTRACT.md`、README、小さな測定結果 `results/`)、この文書、`lift/out-git/present1-results/`(生ログ、PNG、MERGE-NOTES.md)。

## 0. 先頭の要約

凡例: **実測** = この作業で動かして得た数値(`results/` の JSON と `summary.tsv`)。**読み** = コードを読んだ解釈。**未確認**。

| 問い | 答え |
|---|---|
| 実エンジンの画素(停止時の PNG)は canvas に画素一致で出たか | **一致した**(実測)。A・B(ArrayBuffer / ImageBitmap)・C の全方式、JSPI で中断する Worker と `Atomics.wait` で塞がった Worker の両方、3 回ずつ(各 3/3)。canvas の読み戻し(`getImageData`)の SHA-256 = `a8963a48155ddc6b621bc00a7a9b53ef1d231a96693af1fe0d92de119c57f8b4` = PNG を復号した RGBA の SHA-256。最初の非背景画素の PNG も一致(`7ecc7f73f0058e19bde94312c46f91a20c508f2083e485149cdde1e78e557ebf`、全方式 3/3)。**例外: 方式 A + `Atomics.wait` の Worker は、スクリーンショット(実際の表示)が一致しなかった**(0/6。canvas の中身は一致しているが画面に出ていない。§5) |
| BGRX → RGBA(1930×1092 全面、中央値 / p95、ms) | **1 バイトずつの JS 3.08 / 3.20**、**`Uint32Array` のビット演算 2.31 / 2.61**、**wasm SIMD 0.275 / 0.585**(Worker。メインスレッドは 3.14 / 3.51、2.23 / 2.59、0.275 / 0.335)。3 通りの結果は全 3 回・両スレッドでバイト一致 |
| `putImageData`(全面 1930×1092) | 0.225 / 0.370 ms(Worker の OffscreenCanvas)、0.220 / 0.285 ms(メインスレッドの canvas)。CPU 側の呼び出しの時間で、画面への反映(合成)は含まない |
| 遅延(エンジンが面を更新 → canvas の画素が変わる。読み戻しで確認。中央値 / p95、ms。256×256 / 全面) | **A** 0.020 / 0.040、1.14 / 1.71。**B(ArrayBuffer)** 0.205 / 0.400、6.18 / 7.54。**B(ImageBitmap)** 0.210 / 0.410、6.05 / 7.20。**C** は書き手が待たないと裂ける(下)ので **C(書き手が確認を待つ)** 0.130 / 0.240、2.67 / 3.69(以上 JSPI。`Atomics.wait` の Worker でも近い値: §4) |
| 持続 fps(エンジンが 1 フレーム書くごとに表示を更新、読み戻しなし) | 256×256: A 約 23,200 回/秒(Worker のペース。page は 60 fps で合成)、B 約 3,200〜4,550(ImageBitmap の `Atomics.wait` が最低)、C(確認を待つ)約 3,900〜4,300。全面: A 約 510(JSPI)、B 約 140〜148、C(確認を待つ)約 250〜270 |
| JSPI で中断している Worker からの表示 | **表示される**(実測)。エンジンが 1.5 秒 `present()` の中で中断している間も Worker のイベントループは動き(ping に 0.26〜0.38 ms で応答(最終ビルドの 1 回))、A・B・C の全てで画面に出た。中断の前後で更新した 1 フレームも画面に出た |
| `Atomics.wait` で塞がった Worker | **A は表示されない**(再確認。スクリーンショットで 0/6、`screencast` は約 0.6 フレーム/秒)。**B(ArrayBuffer / ImageBitmap)は表示される**(`postMessage` の送信はブロック中でも届く)。**C は表示される**(ページが共有メモリを直接読む) |
| 契約案 | `CONTRACT.md`。面の表(共有メモリ上の 96 B × N の欄、seqlock + スピンロック)、`fle_surface_count/info/take_dirty`、JS が論理 id(form_key)と canvas を結ぶ |
| 0126 の実データ構造との食い違い | **`lid` は窓の作り直しをまたがない**(`CreateWindowExW` ごとに `next_lid++`)。R5 の「同じ論理窓に新しい HWND」を表すには `form_key` が要る。面は x, y、可視、gen、ダーティ矩形を持たない。詳細は `CONTRACT.md` §7 |
| G4 に対する読み | **表示経路が成立する**ことは示した。**エンジンが wasm で描いた**ことは示していない(未達。§8) |

## 1. 手順と環境

- 手順: `lift/harness/web/present/README.md`。1 フェーズ = 1 つの node プロセス = 1 つの headless Chrome(使い捨てプロファイル、`Target.createBrowserContext` の隔離コンテキスト、決定 0124 の遮断フラグ)、ローカルの COOP/COEP サーバ。
- 環境: Chrome 156.0.8078.47(V8 15.6.75.19)、**headless、`--disable-gpu`(ソフトウェア描画)**、AMD Ryzen 9 9950X 16 コア、Linux 7.0.0-38、node 24.19.0、clang 21.1.8(wasm の作成のみ)。測定中のロードアベレージは 0.0〜2.0 程度(他の作業者のジョブが動く時間帯があった。run ごとに記録していない)。
- **GPU、実ディスプレイ、vsync、HiDPI の実機は無い**。`Page.captureScreenshot` と `Page.startScreencast` は CDP の道具で、実ユーザーが見る画面ではない。
- 期待値: PNG を `png.mjs`(Node の `zlib` + 手書きのフィルタ復号)で復号した RGBA の SHA-256。**独立の確認**として、Chrome 自身の PNG デコーダ(`createImageBitmap` → `getImageData`)でも同じ SHA-256 になった(2 枚とも EQUAL。`results/pngdecode.json`)。
- 面の入力: 決定 0126 の PNG 2 枚(`lift/out-fruity11/logs/g3/surface-lid13-at-stop.png` 78,934 B、`…-first-nonbackground-pixel.png` 24,495 B。どちらも 1930×1092、8bit RGB)。**PNG は X バイトを持たないので、BGRX の X = 0 で作った**。実際の面の X が 0 だったかは PNG からは分からない(`CONTRACT.md` §7-7)。
- ネットワーク遮断(決定 0124 の基準): §9。

## 2. 契約(要約。本文は `lift/harness/web/present/CONTRACT.md`)

- 面は top-level 窓 1 枚につき 1 枚(R10。0126 の `fle_raster.c` も同じ)。画素は BGRX、top-down、stride = 幅 × 4、X = 0。
- wasm は**面の表**を線形メモリに持つ(ヘッダ 32 B + 欄 96 B × N)。欄 = `lid`、`hwnd`、`flags`(生存・可視)、`x, y`、`width, height, stride, format, pixels`、`layout_gen`、`gen`、ダーティ矩形、ヒント(`owner_lid`、`style`、`exstyle`、`form_key`)。配置部は seqlock、ダーティ部はスピンロック。
- C の関数は 4 つ: `fle_surface_table()`、`fle_surface_count()`、`fle_surface_info(i, out)`、`fle_surface_take_dirty(lid, out[5])`。JS は表を直接読んでもよい(同じ答えを返すことを `test-exports.mjs` の 15 項目で確認)。
- JS の責務: 表を読む、BGRX → RGBA、canvas / ポップアップ、**論理 id(form_key があればそれ)と canvas の対応**、DPR と CSS サイズ。
- 順序の規則: 画素を書いてから公開、**取り出してからコピー**、配置部は seqlock、`memory.grow` で `ArrayBuffer` が detach されるので鍵を作り直す。

## 3. 方式の比較(表。数値は §4〜§6 の実測)

| 観点 | A: Worker の OffscreenCanvas | B: Worker → ページ(ArrayBuffer / ImageBitmap) | C: ページが共有メモリを直接読む |
|---|---|---|---|
| JSPI で中断中の Worker | 表示される | 表示される | 表示される |
| `Atomics.wait` で塞がった Worker | **表示されない** | 表示される(ack は SAB の語。ページは `Atomics.add` + `notify`) | 表示される |
| 全面 1930×1092 の遅延 p50 / p95 | 1.14 / 1.71 ms(JSPI) | 6.18 / 7.54(AB)、6.05 / 7.20(bmp)(JSPI) | 2.67 / 3.69(確認を待つ。JSPI) |
| 256×256 の遅延 p50 / p95 | 0.020 / 0.040 ms | 0.205 / 0.400(AB)、0.210 / 0.410(bmp) | 0.130 / 0.240(確認を待つ) |
| 書き手の速度(全面、確認なし) | 約 510/s | 約 140〜148/s(エンジンは 2 フレームが転送中なら ack を待つ背圧。待たずに進む経路 `flowSkipped` は全 rate run で 0 回=未使用) | 書き手は速い(約 2,400/s)が**コピーが裂ける**(下) |
| COOP/COEP | 原理上は不要。**この harness は隔離ページでしか測っていない**(JSPI なら `postMessage` の停止でも動くはずだが未測定。`Atomics.wait` の Worker の停止・ack は SAB の語) | ブロックする Worker の ack に必要 | **必須**(共有 `WebAssembly.Memory`)。lift の wasm は共有メモリではない(引用) |
| 複数 top-level 窓 | 窓ごとに canvas を Worker へ transfer(ポップアップの canvas も可: 実測) | ページが窓ごとに canvas / ポップアップに描く | 同左 |
| 窓の再作成(canvas を取り戻す) | **Worker から canvas を取り戻せない**(転送は一方向。detach-window.md §3.3)。form_key で同じ canvas が追従した(実測)。ポップアップが閉じたあとの canvas は未測定 | canvas はページのもの。差し替えは容易(未測定) | 同左 |
| エンジンの wasm への要求 | 無し(画素は Worker のメモリ。変換も Worker) | 無し | **共有メモリでビルド**(`-pthread` 相当の `SHARED_MEMORY`)。非共有の lift の wasm には使えない |
| メインスレッドの負荷 | 無し(読み: ポップアップは別。detach-window.md §4.3(3)) | 全面で p50 1.5 ms(AB)/ 0.04 ms(bmp)の描画 + 受信 | 全面で p50 2.5 ms のコピー + 変換 |
| 実装の穴 | — | — | **配置部のコピー後の再確認が未実装**(窓の大きさが変わる瞬間にコピーが旧バッファを読む可能性。CONTRACT.md §4-3) |

## 4. 測定: 変換、putImageData、遅延、持続レート

### 4.1 BGRX → RGBA と putImageData(`results/conv.json`、3 回の p50 の中央値と p95 の中央値、ms / 回)

| スレッド | 矩形 | JS バイト | JS `Uint32Array` | wasm SIMD | `putImageData`(p50 / p95) |
|---|---|---|---|---|---|
| Worker | 全面 1930×1092 | 3.080 / 3.200 | 2.310 / 2.610 | **0.275 / 0.585** | 0.225 / 0.370 |
| Worker | 256×256 | 0.099 / 0.139 | 0.064 / 0.085 | 0.0075 / 0.0113 | 0.0063 / 0.0100 |
| Worker | 64×64 | 0.006 / 0.007 | 0.004 / 0.005 | 0.0006 / 0.0008 | 0.0008 / 0.0010 |
| メインスレッド | 全面 | 3.140 / 3.510 | 2.225 / 2.590 | 0.275 / 0.335 | 0.220 / 0.285 |
| メインスレッド | 256×256 | 0.100 / 0.159 | 0.065 / 0.124 | 0.0075 / 0.0113 | 0.0063 / 0.0100 |

- **SIMD は JS のビット演算の約 8 倍、1 バイトずつの約 11 倍速い**。全面の変換 0.275 ms は 60 fps の予算(16.7 ms)の 1.6 %。JS のビット演算でも 2.3 ms(14 %)で、60 fps には足りる。
- 3 通りの変換で得た全面の RGBA の SHA-256 は 3 通り × 3 回 × 2 スレッド、すべて同じ(`a8963a48…57f8b4`。`results/summary.tsv` の「converters」)。
- `new ImageData(view, w, h)`(線形メモリの上のビュー)は 0.0005 ms(コピー無し)。共有メモリの上には作れない(`TypeError`。browser-event-loop.md §7.1 の実測の引用。この作業では C 方式で、共有バッファから非共有の配列へ JS で変換しながらコピーしたので、この制約は再確認していない)。
- 方式 B の Worker 側の費用(全面、p50): ArrayBuffer への切り出し 3.65 ms、`transferToImageBitmap` 3.65 ms(256×256 は 0.094 / 0.101 ms)。エンジン代役の更新(XOR + `wrote()`)は全面 0.16 ms、256×256 は 0.006 ms。
- `putImageData` は CPU 側の呼び出しの時間(読み戻しを足しても 0.22 ms で変わらない = 呼び出しの時点で完了している)。**GPU へのアップロードと合成は含まない**。

### 4.2 遅延と持続レート(`results/summary.tsv` の rate。中央値は 3 回の中央値。**読み戻し行**は、フレームごとに canvas からマーカー(64×24 の 24 ビット)を `getImageData` で読んで、エンジンの更新時刻との差を取る)

遅延 = エンジンが面を更新した時刻(`performance.timeOrigin + performance.now()`、ページと Worker で共通)から、canvas の画素を読み戻して更新後のフレームだと確認した時刻まで。**ページ側(B・C)は読み戻しのコスト(`getImageData` 64×24)を含む**。時計の刻みは 5 µs(隔離)。

| 方式 / ループ | 矩形 | 遅延 p50 | 遅延 p95 | エンジンの present 回数/秒 | ページの描画回数/秒 |
|---|---|---|---|---|---|
| A / JSPI | 256×256 | 0.020 | 0.040 | 19,153(読み戻しあり)/ 23,225(なし) | —(Worker が直接) |
| A / JSPI | 全面 | 1.140 | 1.710 | 485 / 508 | — |
| B 配列 / JSPI | 256×256 | 0.205 | 0.400 | 4,282 / 4,156 | 同じ |
| B 配列 / JSPI | 全面 | 6.175 | 7.540 | 150 / 148 | 同じ |
| B ImageBitmap / JSPI | 256×256 | 0.210 | 0.410 | 4,383 / 4,451 | 同じ |
| B ImageBitmap / JSPI | 全面 | 6.050 | 7.195 | 143 / 144 | 同じ |
| **C 確認を待つ**(Csync) / JSPI | 256×256 | 0.130 | 0.240 | 3,615 / 3,922 | 同じ(エンジンが待つ) |
| C 確認を待つ / JSPI | 全面 | 2.670 | 3.685 | 263 / 270 | 同じ |
| A / `Atomics.wait` | 256×256 | 0.020 | 0.035 | 31,023 / 41,107 | —(**画面には出ない**) |
| A / `Atomics.wait` | 全面 | 0.790 | 1.150 | 795 / 822 | — |
| B 配列 / `Atomics.wait` | 256×256 | 0.210 | 0.780 | 4,343 / 4,517 | 同じ |
| B 配列 / `Atomics.wait` | 全面 | 6.845 | 8.710 | 140 / 147 | 同じ |
| B ImageBitmap / `Atomics.wait` | 256×256 | 0.345 | 0.710 | 3,040 / 3,210 | 同じ |
| B ImageBitmap / `Atomics.wait` | 全面 | 7.185 | 8.960 | 140 / 140 | 同じ |
| C 確認を待つ / `Atomics.wait` | 256×256 | 0.130 | 0.255 | 3,957 / 4,261 | 同じ |
| C 確認を待つ / `Atomics.wait` | 全面 | 2.905 | 4.025 | 253 / 253 | 同じ |

**読み方(正直に)**:

1. **A の遅延は「OffscreenCanvas に putImageData してから、同じ Worker が同じ canvas を読み戻した」時間で、ページに見える時間ではない**。0.02 ms は表示の遅延ではない。表示の遅延は §5 の別の測定(上限の見積もり)で見る。`Atomics.wait` で塞がった A は、**canvas の中身は更新されている(読み戻しが成功する)のに画面に出ない**ので、この表の A/`Atomics.wait` の遅延は表示とは無関係。
2. B と C の遅延は「ページの canvas に `putImageData` / `drawImage` した後のマーカーの読み戻し」までで、**合成(画面への反映)は含まない**。
3. エンジンの present 回数は、**B と C(確認を待つ)ではエンジンがページの確認を待つ**ので、表示側の速度で決まる(背圧)。A は待たないので Worker の速度。
4. **B の全面は 143〜150 回/秒で頭打ち**。Worker 側のコピー(3.65 ms)+ 転送 + ページの描画(AB で 1.5 ms)+ 確認の往復が律速(読み)。60 fps の表示には余裕がある(2.4 倍)。
5. **C(待たない版)の数値は表に載せていない**(裂けたコピーの遅延は意味が無い)。数値は `results/summary.tsv` にある: 256×256 で遅延 p50 0.065 ms、エンジン 27,271 回/秒、ページのコピー 9,159 回/秒、**検査したコピーの 97.7 %(104,760 / 107,201)が裂けていた**。全面でも 99.8 %(3,333 / 3,341)。`Atomics.wait` の C も同じ(99.9 %)。
6. **C(待たない版)で、飽和した実行の終わりに書き手が止まったあとの最終状態は、canvas がエンジンの最後の画素に一致した**(§6)。裂けるのは実行中の中間のコピーだけ。
7. 「確認を待つ」C(Csync)は、書き手が読み手の確認を待つので、**検査した 199,000 回のコピーで裂け 0**。検査は 4 点の画素で、見逃しはありうる。

### 4.3 rAF と compositor(レート測定の副産物)

`requestAnimationFrame` の回数は、読み戻しなしの全方式でほぼ 56〜60 回/秒、`Page.startScreencast` のフレームは 52〜59 回/秒(`Atomics.wait` の A だけ 0.5〜0.7 回/秒)。レンダラは 60 Hz で回っている。**GPU は無いので、これは CPU ラスタライズの上限の目安にすぎない**。

## 5. 測定: 表示の正しさ、JSPI の中断と `Atomics.wait`、画面への反映

### 5.1 正しさ(`results/correct.json`、`summary.tsv` の correctness)

48 実行(2 画像 × 4 方式 × 2 ループ × 3 回)すべてで **canvas の読み戻しの SHA-256 が PNG から復号した RGBA の SHA-256 と一致**(48/48)。スクリーンショット(`Page.captureScreenshot` の PNG の RGBA)の一致:

| 方式 | JSPI で中断 | `Atomics.wait` で塞がった Worker |
|---|---|---|
| A | 6/6(実行中)、6/6(停止後) | **実行中 0/6**、停止後 6/6 |
| B 配列、B ImageBitmap、C | 各 6/6 | 各 6/6(実行中)、6/6(停止後) |

- **A + `Atomics.wait`**: canvas の中身は一致(Worker の `getImageData`)だが、**実行中のスクリーンショットは一致しない(画面に出ていない)**。Worker が `Atomics.wait` から戻って自分のイベントループに戻った後(停止後)は、画面に出て一致した。browser-event-loop.md §7.2 と detach-window.md §4.3 の再確認(Chrome 156)。
- 検査の感度(変異): R と B を入れ替えない変換にすると A・B・C のすべてで SHA-256 が不一致になった(検出)。ダーティ矩形の幅を半分にすると A・B・C のすべてで最終の canvas がエンジンの画素と不一致になった(検出)。
- 2 枚の PNG を**スクリーンショット**にして私が画像を見た(`lift/out-git/present1-results/results/canvas-stop-A-jspi.png`、`canvas-first-C-block.png`): 停止時の画像は、決定 0126 の PNG そのもの(上部のトランスポート、左の黒いブラウザ欄、チャンネルラック風のパネル、プレイリストの格子、24 本のミキサー)が canvas に出ている。最初の非背景画素の画像は、トランスポートの帯と灰色の枠だけで、ほかは黒(0126 の記述どおり)。**文字は 1 つも描かれていない**(0126 の記述と一致。エンジンの文字描画が未実装のため。**これは表示経路の問題ではない**)。

### 5.2 JSPI で中断中の Worker と、`Atomics.wait` で塞がった Worker(`results/suspend.json`)

エンジン代役は、`present()` の中で 1.5 秒待つ(JSPI: `WebAssembly.Suspending` の import が Promise を待つ。`block`: `Atomics.wait(…, 1500)`)。待っている間に Worker のイベントループが回っているかを、ページから ping(`postMessage`)で見る。待っている間に表示された画像と、1 フレーム更新した後の画像を、スクリーンショットで確かめる。

| ループ | 方式 | 待っている間の ping | 0.7 秒の画面 = 実エンジンの画像 | フレーム 1(1.5 秒後に更新)が 2.1 秒の画面に出た |
|---|---|---|---|---|
| JSPI | A | **応答あり(0.38 ms)** | 一致 | **出た** |
| JSPI | B 配列 / B ImageBitmap / C | 応答あり(0.26〜0.37 ms) | 一致 | 出た |

(各 1 回の測定。ping は 300 ms で諦める設定で、塞がった Worker では 300 ms 経っても応答が無かった。)
| `Atomics.wait` | A | **応答なし** | **不一致**(画面に出ていない) | **出ない** |
| `Atomics.wait` | B 配列 / B ImageBitmap / C | 応答なし | 一致 | 出た |

- **JSPI で中断しているエンジンの Worker は、`OffscreenCanvas`(方式 A)でも表示される**(実測)。中断の間は Worker のタスクが走る(ping に答える)ので、描画のフレームが提出される。
- **`Atomics.wait` で塞がった Worker は A では表示されず、B(`postMessage` の送信)と C(ページが直接読む)なら表示される**(実測)。

### 5.3 画面への反映の遅延(上限の見積もり。`results/display.json`)

エンジンが 500 ms ごとに 256×256 を 1 フレーム更新し、ランナーが 64×64 のスクリーンショット(1 回約 50 ms)を連続して撮って、フレーム番号が見えた最初のスクリーンショットの終わりの時刻とエンジンの更新時刻の差を取る。**1 回のスクリーンショットが約 50 ms かかるので、分解能は約 50 ms で、「上限の見積もり」にすぎない**。スクリーンショット自体が compositor のフレームを要求しうるので、vsync 駆動の実表示の遅延ではない。

| 方式 / ループ | 見えたフレーム(3 回 × 10) | 3 回の中央値の中央値(ms) | 最悪の回の中央値 |
|---|---|---|---|
| A / JSPI | 30/30 | 12.7 | 33.8 |
| B 配列 / JSPI | 30/30 | 25.3 | 27.6 |
| B ImageBitmap / JSPI | 30/30 | 26.4 | 29.1 |
| C / JSPI | 30/30 | 32.5 | 49.9 |
| A / `Atomics.wait` | **0/30** | — | — |
| B 配列 / `Atomics.wait` | 30/30 | 26.3 | 27.7 |
| B ImageBitmap / `Atomics.wait` | 30/30 | 25.9 | 27.2 |
| C / `Atomics.wait` | 30/30 | 35.0 | 45.2 |

読み: どの方式も、更新から画面に出るまで 1〜2 フレーム(60 Hz で 16.7〜33 ms)以内に見える範囲で、**方式の差は分解能(50 ms)と run 間の振れ(A / JSPI は 11.5〜33.8 ms)より小さい**。方式を順位づける根拠にはならない。

## 6. ダーティ矩形、複数窓、`memory.grow`、最終状態

### 6.1 ダーティ矩形(`results/dirty.json`)

エンジンが 64×64 / 256×256 / 全面を毎フレーム書く。**取り出し(`take`)で矩形を空にして、そのあとコピーする**。最終の canvas が、エンジンが書いた画素(最終フレーム N の期待値)と一致するかと、1 フレームあたりにコピーした画素数:

| 方式 | 矩形 | 最終の canvas == エンジンの画素 | 1 フレームのコピー(最初の全面コピーを除く) |
|---|---|---|---|
| A、B 配列、B ImageBitmap、C | 64×64 | 一致(4/4) | 4,096 画素(面の 0.19 %) |
| 同上 | 256×256 | 一致(4/4) | 65,536 画素(3.11 %) |
| 同上 | 全面 | 一致(4/4) | 2,107,560 画素(100 %) |

- **最初に作った「世代の ack でクリアする」方式は誤りだった**。書き手がコピーより速いと矩形が縮まず、**最初の全面書き込みが矩形に残り続けて毎回全面をコピーした**(2,107,560 画素/回)。さらに、裂けの検査がマーカーを「コピーした矩形の原点」で読んでいたので、全面の矩形では基準画像を見て「裂け 0」と報告した(目が見えていなかった)。`take` 方式に変え、裂けの検査をエンジンの矩形で読むようにして取り直した。誤った最初の結果は `lift/out-git/present1-results/first-pass-flawed-dirty-protocol/` に残してある(使っていない)。
- ダーティ矩形の効果(CPU 側): 256×256 の変換 + put は全面の約 3 %(§4.1: 0.0075 + 0.0063 ms 対 0.275 + 0.225 ms)。**画面への反映(GPU のアップロード)の効果は未測定**。

### 6.2 複数の top-level 窓(`results/multi.json`)

| テスト | 結果(実測) |
|---|---|
| 窓 2(form_key 7)をポップアップ(`window.open`、512×384)に出す | 方式 A・B・C のすべてで、メインの canvas もポップアップの canvas も画素一致(canvas の読み戻しとスクリーンショットの両方)。ポップアップは `crossOriginIsolated`、`opener` あり |
| 窓 2 を作り直す(lid 21 → 22、512×384 → 640×480、HWND も変わる)。form_key あり | A・B・C のすべてで**同じ canvas が追従**し、640×480 に変わって画素一致 |
| 同じテストで form_key なし(lid だけ) | 新しい lid に canvas が無く、**旧い canvas に旧い画像(512×384)が残った**(`presenter.unbound` 74) |

- ポップアップは CDP の合成クリックの直後に開いた。ポップアップの inner 高さを要求より 1 px 大きく開くと、headless で canvas の寸法とぴったり合う(detach-window.md §4.3(6) の headless の 1 px の差の再確認)。
- **未測定**: ポップアップを閉じる / 開き直す、複数のポップアップ、`window.open` の拒否(活性切れ)時のフォールバック、ポップアップへの入力の往復(この作業は表示だけ)。

### 6.3 `memory.grow`(`results/grow.json`)

実エンジンの wasm は `ALLOW_MEMORY_GROWTH` でリンクされる(`jspi-real-wasm.md`、`link.sh`。引用)ので、実行中に `memory.grow` が起きる。非共有メモリでは、grow が古い `ArrayBuffer` を **detach** し、それを元にした `ImageData` が死ぬ。代役を伸長可能なメモリ(64 → 128 MiB)にして、実行中に 3 回 grow した。

- **最初のビルドは失敗した**: A と B ImageBitmap が `InvalidStateError: Failed to execute 'putImageData' … The source data has been detached.` で止まった(Worker が `ImageData` を作り直さずキャッシュしていた)。キャッシュの鍵に `memory.buffer` を足して直した。直した後は **A・B 配列・B ImageBitmap・C の 4 方式とも、grow 3 回の実行で最終の canvas がエンジンの画素と一致**。
- 共有メモリの grow(C)では `buffer` オブジェクトが新しくなるので、JS の `Int32Array` の表のビューを取り直す(`SurfaceTable.i32`)。

### 6.4 飽和後の最終状態(`results/final.json`)

10 方式・ループ × 2 矩形(256×256 / 全面)で、エンジンが待たずに 2 秒書き続けて止まったあと、canvas の最終の画素がエンジンの最後の画素と一致するか: **20/20 一致**。待たない C は実行中のコピーが裂けていても、書き手が止まったあとの最後の `take` で拾うので最終状態は正しい(「実行中に裂けている」ことと「最終的に正しい」ことは別)。

## 7. DPR、CSS サイズ、`image-rendering`(`results/dpr.json`)

| 設定 | canvas の内部サイズ | CSS サイズ | `devicePixelRatio` | 結果(実測) |
|---|---|---|---|---|
| 等倍、CSS サイズ指定なし | 1930×1092 | 1930×1092 | 1 | スクリーンショットが PNG と一致(`a8963a48…`) |
| `width/height` を CSS で 1930×1092 に固定 | 同 | 同 | 1 | 一致 |
| **DPR 2、CSS 965×546(1 canvas 画素 = 1 デバイス画素)** | 同 | 965×546 | 2 | **一致**(1930×1092 のスクリーンショット) |
| DPR 2、CSS 1930×1092(2 倍に拡大)、`image-rendering: auto` | 同 | 同 | 2 | 拡大で補間される: サンプルした 2×2 ブロックの 78 %(102,917 / 131,859)だけが一様 |
| DPR 2、CSS 1930×1092、**`image-rendering: pixelated`** | 同 | 同 | 2 | **全ブロック一様(131,859 / 131,859)**= 画素が崩れない |
| CSS 0.5 倍(965×546)、DPR 1、`auto` / `pixelated` | 同 | 965×546 | 1 | 縮小(`lift/out-git/present1-results/results/canvas-css-half-pixelated.png` は `pixelated`。縮小では画素が間引かれる) |
| CSS 1.5 倍(2895×1638)、DPR 1、`auto` / `pixelated` | 同 | 2895×1638 | 1 | 非整数倍。比較は未実施(画像は `results/dpr.json` の SHA のみ) |

- **等倍(1 canvas 画素 = 1 デバイス画素)で見せる限り `image-rendering` は要らない**。**拡大して見せる(DPR 2 で CSS 1930×1092)なら `image-rendering: pixelated`(または `crisp-edges`: 未測定)が要る**。
- **canvas の内部サイズは面の寸法(1930×1092)で、CSS サイズは別**。HiDPI(DPR 2)で画素を 1:1 にするには CSS サイズを内部サイズ ÷ DPR にする(965×546)。`devicePixelRatio` が窓ごとに違う場合(ポップアップだけ DPR 2 など: detach-window.md §4.3(6))は窓ごとに CSS サイズを決める。
- 実 HiDPI の画面は無い。CDP の `Emulation.setDeviceMetricsOverride` で DPR を変えただけで、実機のズーム、OS のスケーリング、複数画面は未測定。

## 8. G4 に対する読み

- **表示経路が成立する**: 実エンジンの停止時の描画面(決定 0126 の PNG)を、wasm の線形メモリの上の BGRX の面として、A・B・C のいずれでも canvas に**画素一致**で出せる。JSPI で中断するエンジンの Worker でも、`Atomics.wait` で塞がった Worker でも(A を除く)出る。
- **「エンジンが wasm で描いた」ことは別**で、**未達**。この作業の画素は、決定 0126 の **native** の実行が描いた面を PNG にしたものを、代役の wasm が面として持っているだけである。エンジンの wasm はこの作業で一度も動かしていない。C 側の API(`fle_surface_*`)は代役に実装しただけで、決定 0126 のワークツリー(`fle_raster.c`)には**無い**(実装していない。足し方は `CONTRACT.md` §3)。
- G4 の受け入れ条件は「エンジンが描いた画素が canvas に出る(エンジンの GDI 呼び出し結果と画素が一致する)」。**画素一致は「PNG = canvas」であって、「エンジンの GDI 呼び出し結果 = canvas」ではない**。後者は 0126 の native の面(メモリ)と canvas を直接比較して初めて言える(この作業では PNG を経由している: PNG は 8bit RGB のロスレスなので、PNG と面は一致すると読んでいるが、**PNG が面から正しく書かれたことは 0126 の担当の記述(「PNG から数え直しても一致」)を引用しているだけで、再確認していない**)。
- したがって G4 の状況は「表示経路は先に成立を確かめた。エンジンの wasm 側(FRUITY 有効の wasm ビルド、面の表の実装、共有メモリか JSPI か)が残っている」。

## 9. ネットワーク遮断(決定 0124 の基準)

- フラグ: `--host-resolver-rules="MAP * 127.0.0.1:9, EXCLUDE 127.0.0.1"`、`--no-proxy-server`、`--disable-ipv6`、`--disable-quic`、バックグラウンド機能・Safe Browsing 等の無効化、新しいブラウザコンテキスト、使い捨てプロファイル(`/tmp/opencode/present1/profile-*`。終了時に削除)。決定 0124 の `run_chrome_scen.mjs` と同じ(コピーして使った。`--force-color-profile=srgb --disable-lcd-text` だけ足した)。
- **最終ビルドの 11 フェーズ分の net-log(11 ファイル、357,282 イベント、取得モード `Default`。`pngdecode` は別に 1 回走らせ、同じ検査で TCP 非ループバック 0・UDP_BYTES_SENT 0 を `results/netlog-per-phase.txt` に記録)の集計**(`results/netlog-check.txt`):
  - **TCP の接続試行: 非ループバック 0**(ループバック 204 回。そのうち 170 回は `127.0.0.1:9`(何も listen していない。`MAP *` の写し先)、残りは自前のサーバ)。
  - **UDP の `connect()`: `[2001:4860:4860::8888]:443` に 52 回(ホストリゾルバの IPv6 到達性チェックと読む。決定 0124 が記述したものと同じ)、`127.0.0.1:443` に 72 回。`UDP_BYTES_SENT` のイベントは 0**(= パケットは送っていない)。
  - Chrome 自身が他のホスト(`accounts.google.com`、`www.google.com`、`update.googleapis.com`)へ起こした URL 要求は、全部 net error −102(接続拒否)で終わった(名前は `127.0.0.1:9` に写される)。
- **決定 0124 の PM の基準(TCP 非ループバック 0、UDP の送信パケット 0)は満たした**。**「UDP の `connect()` を 1 回も出さない」は満たしていない**(決定 0124 と同じ状況: `--disable-ipv6` ほか各種のフラグでも消えなかった。この作業で追加の試みはしていない)。
- **取得モードを `Default` にした理由と限界**: 最初の実行は `Everything` で、`rate` の net-log だけで 992 MB になり(応答の本体まで記録される)、検査スクリプトが文字列に読み込めず落ちた。work dir も 1.6 GB まで膨らんだ。`Default`(応答の本体を取らない)に変えた。`Default` のログは TCP / UDP の接続試行と `UDP_BYTES_SENT` を記録する。**`Everything` との比較**: `pngdecode` と `suspend` の 2 フェーズだけ `Everything` で取り直して同じ検査をした(TCP 非ループバック 0、UDP_BYTES_SENT 0)。`Everything` の `suspend` は 225 MB で、検査後に削除した。
- **依頼者のブラウザのプロファイル・資格情報には触れていない**。`~/.config/google-chrome/` を Chrome が更新しうること(決定 0124 の注)は確認していない。`--disable-breakpad` と `--crash-dumps-dir` は使い捨てプロファイルの下に向けた。

## 10. 限界・未確認

1. **エンジンは代役**。FL のコードでも wasm でもない。面の更新は XOR と 64×64 のマーカーで、実エンジンの GDI の書き方(小さな矩形の大量の更新、BitBlt、重なり)ではない。実エンジンの描画面の更新の頻度・矩形の分布は未測定。
2. **GPU 無し、headless、`--disable-gpu`**。画面への反映(アップロード、合成)、vsync、複数モニター、HiDPI の実機は未測定。`putImageData` の時間は CPU 側の呼び出しだけ。
3. **画面への反映の遅延は上限の見積もり**(分解能 50 ms。§5.3)で、方式の順位づけには使えない。
4. **方式 C の配置部の再確認(窓の大きさが変わる瞬間のコピー)は未実装**。C は共有メモリの wasm が前提で、現在の lift の wasm は非共有(引用)。
5. **ポップアップを閉じる / 開き直す、`window.open` の拒否のフォールバック、入力の往復、複数のポップアップは未測定**。方式 A でポップアップの canvas を Worker に渡すと、閉じたあとに canvas を取り戻せない(detach-window.md §3.3)。
6. **`form_key` を 0126 のエンジンから取れるかは未確認**(`SetPropW` の値からと読む)。
7. **X バイトの実値**: PNG は X を持たない。0126 の面の X が 0 かは PNG から分からない。
8. **Firefox、Safari、headed の Chrome、実ユーザーのクリックによる `window.open`、Windows / macOS の Chrome は未測定**。JSPI も Chrome 156(既定で有効)だけ。
9. **`Atomics.wait` の Worker で A が表示されない原因**は Chrome の内部実装を見ていない(フレームの提出がイベントループの 1 ターンの終わりに結びついていると読むのは browser-event-loop.md の読み)。
10. C の「裂けの検査」は 4 点の画素。「裂け 0」は下限の意味(見逃しは否定できない)。
11. 3 回の中央値と p95 は run ごとの 3 点の中央値で、p95 は各 run の p95 の中央値。run 間の振れ(例: A / JSPI の全面の表示遅延は 11.5〜33.8 ms)は大きい場合がある。
12. 他の作業者のジョブが同じ機械で動いていた時間帯がある。run ごとの loadavg は記録していない。

## 11. 未決の点

1. **窓の論理 id が作り直しをまたぐか**: 0126 の `lid` は作り直しのたびに変わる。canvas を窓に結ぶには `form_key`(フォームのインスタンスで安定する値)が要る。0126 に足させるか、JS 側で別の鍵(例: クラス + 親)を使うか。
2. **表示方式の選択**: JSPI で中断する前提(決定 0124)なら **A が最も単純で速い**(Worker 内で完結。メインスレッドの負荷なし)。ただし**エンジンが `Atomics.wait` で塞がる場面(M-A の保険、一部の同期 API)があると A は表示されない**。B は A の保険として動く(遅延は全面で約 6 ms 大きい)。C は共有メモリの wasm の再ビルドと、裂けの対策が要る。JSPI だけを前提にするなら A、保険を持つなら A + B、のように読めるが、決めるのは PM。
3. **C(共有メモリ)を採るなら**、書き手が読み手の確認を待つ(エンジンの描画が表示に律速される)か、2 枚持つか。待たない C はコピーが 99 % 裂ける。
4. **面の表と `fle_surface_*` を 0126 の `fle_raster.c` / `fle_win.c` に足す作業**を誰がいつやるか(`CONTRACT.md` §3 の表。約 100 行規模と読むが、見積もりの根拠は弱い)。
5. **ポップアップの扱い**(detach-window.md §7 の継続): この作業は表示だけで、入力と閉じる動作は未測定。

## 12. 規則違反の自己申告と、途中の誤り

- **AGENTS.md の「作業は task agent を通じて」**: 私は task ツールを持たないサブエージェントなので、調査・実装・測定・文書をすべて自分で行った。
- **git の変更系操作はしていない。commit していない。** 既存ファイルは編集していない。決定 0126 のワークツリー `lift/out-fruity11/` は読み取りだけ(PNG を `cp` で `lift/out-git/present1-results/png/` にコピーした。元は変更していない)。`lift/runtime`、`lift/crates`、`lift/harness` の C・シェル、`lift/out-diag*`、他の `out-*` は触れていない。
- **Chrome の遮断**: 最初のスモークテストから `--host-resolver-rules` ほかの遮断フラグを付けた(遮断なしの Chrome は起動していない)。net-log の検査は全フェーズで行った(§9)。
- **work dir `/tmp/opencode/present1/` が一時的に 300 MB の上限を超えた**: 最初の測定で net-log を `Everything` にしたため 1.6 GB(`rate` の net-log が 992 MB)になった。気づいた時点で net-log とプロファイルを削除し、`Default` に変えた。**その後、`Everything` の再検査で `suspend` の net-log が 225 MB になり、work dir が 279 MB に達した(上限 300 MB 以内)**。検査後に削除。
- **コマンドの実行**: 最初のスモークテストが `Browser.close` の後も node が終了せず(タイマーが残った)、2 回タイムアウトした。`withTimeout`(タイマーを消す)と `process.exit` で直した。**この 2 回は shell の timeout で止まり、出力が失われた**(`cut` のパイプに出力が溜まったまま。以後は出力をファイルに書いて読んだ)。`sleep`、`timeout`(コマンドとして)、`nohup`、末尾 `&`、`pgrep -f` のループは使っていない。python は標準ライブラリだけで、`PYTHONDONTWRITEBYTECODE` は設定していなかったが、スクリプトの編集にしか使わず(`python3 - <<EOF`)、`__pycache__` / `.pyc` は作られていない(`find` で確認)。標準ライブラリと衝突するファイル名は作っていない。
- **途中の誤りと訂正**(結果に影響したもの):
  1. 「世代の ack でダーティ矩形をクリア」方式の誤り(§6.1)。誤った結果は `first-pass-flawed-dirty-protocol/` に残した。
  2. C の裂け検査が、全面の矩形のとき基準画像を見て「裂け 0」と報告した(§6.1)。
  3. 方式 C の最初のページループが、`Atomics.waitAsync` が同期で返るとき(書き手が速いとき)イベントループに戻らず rAF と CDP を飢えさせ、`P.stop()` が 120 秒でタイムアウトした(この実行の net-log が 992 MB になった原因の一部。取得モードの変更は §9)。コピー後に `MessageChannel` で譲るように直した。最初の `rate` の記録は `out-git/present1-results/logs/rate-first-attempt-naive-C-loop.txt` に残した(使っていない)。
  4. 最初の `display` の判定が、マーカーを 24 行しか切り出さなかったので、「灰色セルが見えている」の検査が空振りした(白い canvas を有効と読んだ)。64×64 の切り出しに直し、A / `Atomics.wait` が「0/30」と出ることを確かめた。
  5. 最初のポップアップのスクリーンショットが不一致だったのは、headless のポップアップの inner 高さが要求より 1 px 小さい(383)ためだった。1 px 大きく開く(§6.2)。
  6. `memory.grow` の最初の実行(§6.3)と、伸長可能な wasm の名前の誤り(`engine-shared-grow.wasm` を引いた)。ログは `out-git/present1-results/logs/grow-first-attempt-…txt`。
- **最終ビルドの wasm**(SHA-256 は `results/wasm.sha256`)で、`rate` 以外の全フェーズと `rate` を取り直した。`pngdecode` だけ、最終のあとで足したフェーズで、`test-exports.mjs` は最終の `engine.wasm` に対して実行した。**`engine.c` の最後の変更はコメントだけ**(`lid` の記述。再ビルドして 6 個の wasm がバイト同一であることを `cmp` で確認)。

## 13. 成果物の場所

- 本文: `docs/investigations/canvas-present.md`(この文書)
- 契約: `lift/harness/web/present/CONTRACT.md`
- コード・README・小さな結果: `lift/harness/web/present/`(`results/` に最終ビルドの JSON と `summary.tsv`)
- 生ログ・PNG・スクリーンショット・MERGE-NOTES.md: `lift/out-git/present1-results/`(gitignore 対象)
