実エンジン: `npm run build`・`cargo build --release --locked`・`fe-build engine`・`fe-build bundle`(終了コード 0。bundle の出力 engine wasm 3912941db91c)・`tools/boot-check/boot-bundle.mjs`(state engine-idle・standIn false・error null・elapsedMs 8124)・`tools/smoke/welcome-flow.mjs`(流れ A〜D の 4 行が全て基準を満たす。判定の式も 0)・`npm test --prefix app`(終了コード 0。1465 passed, 10 skipped)・`tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2,4k-random`(終了コード 0。`rows 4, accepted true`)・`tools/e2e/run.mjs --only first-paint-complete --displays 4k-dpr1,4k-dpr2,4k-random --repeat 5`(終了コード 0。`rows 4, accepted true`。20 行が全て ok)・`guard/scan.sh`(0 finding)。

測定の値は、すべて runner の値。この機械とは別枠。nproc 4。loadavg(1 分値): 起動確認の直後 1.91、E2E の後 0.91。

# 起動の後の黒い領域(#1483)

## 判定

| 項目 | 値 | 判定 |
|---|---|---|
| first idle の 3 秒後に、100 px 以上の完全な黒の大きな領域が 0(修正の後・この run の測定) | DPR 1: 0/5、DPR 2: 0/5、乱数 1.95: 0/5、乱数 1.66: 0/5(計 20 行が全て ok) | 受理 |
| 修正の前の同じ測定(前の作業の記録。修正の前の HEAD。この run では再測定していない) | DPR 1: 5/5、DPR 2: 5/5、乱数 1.34: 5/5、乱数 1.78: 5/5(計 20 回に黒い領域) | 前の記録 |
| 起動確認(boot-bundle) | engine-idle・standIn false・error null・elapsedMs 8124 | 受理 |
| 流れ A〜D | 4 行とも error null・engine-idle・standIn false・trapped false・hostErrors 0・opened true。B と D は welcomeClosed true。elapsedMs 7295〜7529 | 受理 |
| `npm test --prefix app` | 終了コード 0(1465 passed) | 受理 |
| `--scenarios smoke`(この run) | 終了コード 0。4 行とも全て ok。`plugin-manager-opens` は既知の xfail(#1499) | 受理 |
| `--only first-paint-complete --repeat 5` | 終了コード 0。4 行 × 5 回 ok | 受理 |
| 黒い領域の原因の特定 | 実測の範囲で、窓の矩形と一致するところまで。描いた API は未特定(2 節) | 未確認(一部) |

## 1. 再現と修正の前後

黒い領域は、100 px 以上の完全な黒(R・G・B が 0)の連結成分。

| 版 | 表示 | 黒い領域(x, y, w x h。前の記録) | 結果 |
|---|---|---|---|
| 修正の前(前の記録) | DPR 1 | 0, 145, 364 x 2009 | 黒あり 5/5 |
| 修正の前(前の記録) | DPR 2 | 0, 290, 731 x 1858 | 黒あり 5/5 |
| 修正の前(前の記録) | 乱数 1.34 | 0, 181, 455 x 1972 | 黒あり 5/5 |
| 修正の前(前の記録) | 乱数 1.78 | 0, 252, 639 x 1897 | 黒あり 5/5 |
| 修正の後(この run) | DPR 1 | なし(黒の画素 4、1 群) | ok 5/5 |
| 修正の後(この run) | DPR 2 | なし(黒の画素 457、132 群。どれも小さい) | ok 5/5 |
| 修正の後(この run) | 乱数 1.95 | なし(黒の画素 457、132 群) | ok 5/5 |
| 修正の後(この run) | 乱数 1.66 | なし(黒の画素 265、80 群) | ok 5/5 |

修正の前は 20/20 で黒い領域あり(前の記録)。修正の後は 0/20(この run)。黒い領域の矩形の表(DPR・起動の時刻・矩形)は、修正の後は無し。行の開始の時刻(`first-paint-complete` の 5 回の開始。runner の時計): DPR 1 は 04:40:57〜04:41:58、DPR 2 は 04:42:13〜04:43:21、乱数 1.95 は 04:43:38〜04:44:44、乱数 1.66 は 04:45:01〜04:46:07。

## 2. 原因(実測と推定を分ける)

実測(前の作業の記録):
- 黒い領域の矩形は、WebView2 の controller の `put_Bounds` の矩形(親の窓の座標)と一致する。DPR 1: `bounds=0,0,364,2009 origin=0,145`。DPR 2.36(単独実行。engine の log): `bounds=0,0,910,1782 origin=0,362`。どちらも、黒い領域の origin と幅・高さに一致する。
- 黒い画素は R・G・B とも 0。その領域は、面の作成の時にゼロで、以後の描画でもゼロのまま。
- 最初の `BitBlt` は、窓が見えていない時に捨てられた(log: `first drawing call discarded because its window is not visible (decision 0126, S-draw-hidden): BitBlt into the surface of top-level window lid 14`)。
- 全面の描画(`deskDumpBase64(true)`)でも同じ矩形が黒い。初回の転送(`desk-gl.ts`)の取りこぼしではない。

推定(確かめていない):
- 黒い領域は「その領域を塗る描画が無かった」もの。WebView2 の代わりの窓は何も描かない(`lift/runtime/fle_webview2.c` の設計)。親の窓の背景の塗りが、この矩形に届いていないと推定する。
- 候補の一つは、最初の描画(lid 14 への `BitBlt`)が窓が見えていない時に捨てられ、その領域がその後に描き直されなかったこと。一致する徴候だが、確かめていない。描き直しの経路(`ShowWindow` 後の無効化)は読んでいない。根本の直し(捨てた描画の領域を無効化する等)は、この作業では行っていない。
- (a) 初回の転送の取りこぼし: 全面の描画でも黒いため、否定的。
- (b) iframe の黒: 黒い領域の原因ではない。E2E は canvas の画素を読み、iframe は canvas の上に重なる。iframe の色は判定に入らない。ただし、iframe の背景も読み込み前に黒く見えうるため、色は直した(3 節)。
- (c) 描画のタイミング: 上の「捨てられた最初の描画」が、これに当たる可能性がある。未確認。

## 3. 直した箇所(既定の動作。オプトインは無い)

- `lift/runtime/fle_win.c`: WebView2 controller の矩形の、ゼロの画素をデスクトップの色(`FLE_DESK_BACKGROUND`、#05070a)で塗る。塗る時点は三つ。(1) `put_Bounds` の時。(2) 最上位の面が作られた時。(3) その面へ書き込みが通知された後で、書かれた矩形と重なる部分だけ。
- `lift/runtime/fle_raster.c`: (2) と (3) を呼ぶ(weak 宣言)。
- `lift/runtime/fle_webview2.c`: `put_Bounds` で `fle_win_fill_unpainted` を呼ぶ。色は `fle_desk.h` の定数から求める(DPR の値は使わない)。
- `app/src/ui/webview2-surface.ts`: iframe の背景を `#142028` から `#05070a`(デスクトップの色。`fle_desk.h` の `FLE_DESK_BACKGROUND` と同じ)に変えた。
- `tools/e2e/lib/picture.mjs`: `blackRegions(pic, min)` を足す。行の長さ(stride)は `decodePicture` の値で読む。試験は `tools/e2e/test/dump-picture.test.mjs`(2 件を追加)。
- `tools/e2e/scenarios/smoke/first-paint-complete.mjs`(新規)。`tools/e2e/scenarios/smoke/index.mjs` に足す。DPR は `window.devicePixelRatio` から求める。
- `tools/e2e/run.mjs` の `--repeat`: main に既に在る(`planScenarios`。全部が ok でなければ終了コード 1。`runPasses`)。この作業では変えていない。
- `tools/e2e/scenarios/smoke/knob-right-click-permissions.mjs`: 前の作業の取り込みで衝突した。この run では main の版(`1dc942df`)を採った。この作業は、そのファイルを変えていない。

製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数は足していない(`guard/scan.sh` 0 finding)。

## 4. smoke の失敗と直し(前の作業)

- 前の作業の 1 回目・2 回目: `knob-right-click-permissions` が、4k-random の行(DPR 2.15 と 2.36)で、3 つの権限の状態の全てで menu false。原因(実測): 試験のクリック点が、メイン窓の左上から guest (406, 145) × DPR で決め打ちだった。DPR 2.36 では、ノブではなく LED の脇を叩いていた。main 側で、校正済みの DPR(1 と 2)だけ menu を要求する形に直されている(この run の版)。
- 前の作業の 3 回目: `context-menu-none` が 4k-dpr2 の行で、エンジンが終了コード 134 で止まった(ゲストの未処理の例外。原因は未確認)。この run の smoke では同じ行は ok。
- 前の作業の main 取り込み後の smoke: `plugin-database-lists-builtins` が ok 偽(前の作業の記録 7 節)。推定: 取り出しの木が古いこと。この run では、`tools/place-plugin-dlls.mjs` を走らせ、ビルドし直した後の smoke で、同じ行は ok。古い木が原因だったかは、確かめていない。

## 5. 流れ A〜D(この run の値)

A・B・C・D: error null・state engine-idle・standIn false・trapped false・hostErrors 0・opened true。B と D は welcomeClosed true。elapsedMs は 7295〜7529。

## 6. 限界

- 黒に戻す描画(どの API が書いたか)は未特定。塗った後の書き込みで、結果は消えている(書いた後の塗りだけ残る)。
- 根本の直し(捨てられた描画の後の描き直し)は行っていない。今の直しは、窓の矩形の中のゼロの画素を塗る対症の処置。
- 塗る色 #05070a は、画面では非常に暗い色で、見た目はほぼ黒のまま。E2E は完全な黒(0,0,0)を数えるので、基準は満たすが、利用者が暗い面を見る可能性はある。FL のデスクトップの色をそのまま使うという Issue の指定に従った。
- controller の矩形の中で、FL が本当に黒で描いた画素も、ゼロの画素として塗られる(非ゼロの画素は残る)。
- 書き込みのたびに、重なる矩形の画素を走査する(矩形の数の上限は 8)。その分の所要は測っていない。smoke の fps は 38〜57(runner の値)。
- 実 GPU・実 Chrome の版との差は未確認。
- 修正の前の率は、前の作業の記録。この run では修正の前を再測定していない(再ビルドが要る)。

## 7. この run の受け入れ(受理)

- 取り込み: `git pull --no-rebase --no-edit origin main`(前の作業の取り込みは、`1dc942df` の merge まで)。衝突は `tools/e2e/scenarios/smoke/knob-right-click-permissions.mjs` の 1 件だけ。main の版を採って解消した(この作業の変更の無いファイル)。
- 前の作業の差し戻し(main 取り込み後の smoke の 1 件)は、この run で解消した: `tools/place-plugin-dlls.mjs` を走らせ、ビルドし直した後、smoke は 4 行とも全て ok(xfail の 1 件を除く)。
- 次の run での確認: 木の鍵(`extract-tree-*`)は、main の取り込みで `lift/plugins/*/plugin.tsv` と `ci/extract-dlls.tsv` が変わると変わる。取り込みの後は、作業ツリーの木を作り直してから受け入れを通す。
