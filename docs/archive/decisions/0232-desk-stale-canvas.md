# 決定 0232(仮): 出荷構成の `engine-desk` が落ちる原因 = 検査のスクリーンショットの座標(文書座標とビューポート座標)+ ページの 2 px スクロール。canvas は古くなかった

作業者 `desk3`(desk2 の続き)、2026-11-04 JST。**仮の決定**。基準 HEAD 36b1873e(`git archive HEAD` に `desk2.diff` と `desk2-ci.diff` を当てた木)。差分は `desk3.diff`(app とテストだけ。runtime・生成 C は触らない)と `desk3-docs.diff`(決定 0223 §12.6 の 9 項を閉じる一文)。

## 結論

- **どちらが古かったか: どちらも古くなかった。** Worker が canvas から読み戻した画素(getImageData)= コンポジタが保持する絵(peek)= 窓表から描き直した絵(dump)。メーターの箱の外で 0 画素(出荷構成 bundle、実エンジン、Chrome 156、5 回の run・約 25 サンプルと e2e)。(A)(B) は否定。(C)『時刻差の競合』でもない(5 s 後・10 s 後も同じ差が続く)。
- **原因: 検査の座標。** `Page.captureScreenshot` の `clip` は**文書座標**、検査が渡した `getBoundingClientRect()` は**ビューポート座標**で、ページがスクロールしていない間だけ一致する。出荷構成ではページが 1302 px で試験窓 1300 px より 2 px 高く、Chrome が 2 px スクロールすることがあり(`scrollY=2`)、スクリーンショットは絵を 2 px 下にずらした像(約 29 万画素が違う)になった。スクリーンショットを 2 行ずらすと絵と 0 画素差(他のずれは 28 万〜54 万画素)。スクロールを `window.scrollTo` で 0/2/0/2/1 に変える介入では、前任の clip は scrollY 0 で 0 画素、2 で 29 万画素、1 で 33 万画素、文書座標の clip はいずれも 0(`tables/scroll-intervention.md`)。
- **出荷構成だけ落ちた理由**: manifest の説明(`bundle.description`、393 字)が長く、`#bundle-info` が単独の行で 2 行に折り返し、header が 122 px(D8 の bundle は 284 字・1 行・72 px)。文書が 1302 px(D8 は 1300 px ちょうど=スクロール不能)になる(`tables/layout-header.md`)。
- **ユーザーの画面(既定の入口、`?screen=` なし)は影響を受けていなかった**: `100vh` + `overflow: hidden` + 1 行 36 px の header で、説明の長さに依らず文書は窓にちょうど収まる(測定: 文書 2400x1300 = 窓、scrollY 0、ゲストの画面 = 窓 − 340 − 36)。出荷構成 bundle の既定の入口でも canvas・絵・スクリーンショットは一致。はみ出していたのは legacy 配置(`?scale=` / `?screen=legacy`。実エンジンの検査が使う)だけ。
- **直し(検査を弱めない)**: (1) 検査: `shotOf`(engine-desk)と `page.shot`(browser.mjs)が毎回 `#desktop` の位置を読み直して `scrollX/Y` を足す。閾値・比較は変えない。(2) 製品側: header を 1 行に固定(`flex-wrap: nowrap`、`#bundle-info` は省略記号で切り、全文は title 属性)。FL・窓・クラスに依存しない CSS だけ。(3) 検査を足す: engine-desk に『文書が窓より高くなく、ページが先頭にある』、run.mjs の `screen-legacy` と `screen-default` に『長い manifest の説明でも header・文書の高さ・desktop の位置が変わらない』。
- **繰り返しの結果**: **(a) 修正した検査+新 CSS の製品: 出荷構成 bundle の engine-desk 10/10 run が全検査通過(51/51)、scrollY は 10/10 run で 0、文書 1300 / 窓 1300**(`tables/repeats-new-css.md`)。(b) 修正した検査+旧 CSS の製品: スクリーンショット系の 50 検査は 10/10 run で通過(scrollY 2 が 7 回、0 が 3 回)。(c) 前任の検査+新 CSS の製品: 50/50 が 2/2(CSS だけでも直る)。(d) D8 の bundle+修正した検査+新 CSS: 51/51。(e) Firefox 158: 静止後 8/8 サンプルで 0 画素(箱の外)。

## 冒頭の三点(正直さ)

1. **何を実測したか**: 実エンジン(lift した FLEngine と同梱 DLL の wasm、出荷構成 F2xH4 + `-DFLE_DESKTOP`、CI の段スクリプト製の bundle `fl-engine-ci-F2xH4-4646ef5c`)を隔離 Chrome 156 と Firefox 158(headless)で実行した。代役・画像の再生・期待値の貼り付けは使っていない。この機械は共用(load 5〜33)で、依頼者の機械ではない。
2. **結論**: 上の『結論』のとおり。製品の canvas・コンポジタの絵に不具合は見つからず、原因は検査の座標系と、legacy 配置(`?scale=` / `?screen=legacy`)のページが manifest の説明の長さで 2 px はみ出すこと。**既定の入口(`?screen=` なし)は影響を受けていなかった**(§3)。
3. **未確認**: Chrome が 2 px スクロールする契機(スクリプトの scroll / focus / scrollIntoView は hook して呼ばれていない。クリックでも再現しない。ページの高さが窓を 2 px 超えると起きることは確か)。前任の n8〜n17(10 連続の失敗)の各 run の scrollY(記録が無い)。19:20 の run が通った理由(同じ理由で scrollY 0 だったと読むが、記録は無い)。依頼者の実機(Windows の Firefox 159)の値は 0 件。

## 1. 依頼と、見るべき対立仮説

PM の依頼: 出荷構成 bundle の engine-desk で『ページの canvas のスクリーンショット = コンポジタが保持した絵』の 4〜5 検査が再現して落ちる(決定 0223 §12.6 の 9)。どちらが古いか — (A) canvas が古い絵のまま(Worker が提示していない = D の本物の不具合)/(B) 絵のほうが古い(コンポジタが窓の変化を拾っていない)/(C) 検査の競合。PM の読み(未確認)は、`fle_r_blit` の FAST_COPY や AI_UNITS の always_inline が `fle_raster_wrote` → `fle_desk_wrote` を呼ばない書き込みをしている、というもの。

## 2. 実測(同じ時刻の 4 者。`tools/stale.mjs`、`tables/four-way.md`)

出荷構成 bundle の engine-desk を HEAD + desk2 + desk2-ci の木で実行して再現した(45/50、同じ 5 検査、crop `52520ae83297` / 絵 `da1d1697dd1f` / 差 290,923 画素)。同じページ・同じ bundle で、同じ時刻に次を取った: (i) canvas = Worker が OffscreenCanvas 自身の 2D context に `getImageData` した値(hook を CDP で Worker の前に差し込む。app は変えない)、(ii) 絵 = `deskDumpBase64(true)`(compose しない)、(iii) 窓表から描き直した絵 = `deskDumpBase64(false)`、(iv) Worker の `putImageData` の記録(時刻・矩形)とプレゼンタの数、(v) CDP のスクリーンショット(前任の検査と同じ clip、と文書座標の clip)。

| 比較 | 結果(静止後、メーターの箱 (1037,6)-(1122,38) の外) |
|---|---|
| canvas vs 絵 | **0 画素**(5 run・約 25 サンプル。静止前は取得の時刻差で食い違う) |
| 絵 vs 窓表から描き直した絵 | 0 画素(`resync` 後の canvas も 0) |
| スクリーンショット(前任の clip)vs canvas | scrollY 0 の run は 0、**scrollY 2 の run は 290,351 画素**(5 s・10 s 後も同じ。追いつかない) |
| スクリーンショット(文書座標の clip)vs canvas | **どの run でも 0 画素** |

`putImageData` は 1 サンプルの間に 1〜2 回進み、canvas は絵に追いついていた。したがって (A)(B) は否定、(C) 時刻差の競合でもない(差は時間が経っても消えない)。PM の仮説(`fle_desk_wrote` の欠落)は、窓の面→絵→canvas の全段が一致しているので成り立たない(`lift/runtime` の読みも §6)。FAST_COPY 等の二分(手順 2)は、原因が runtime でないと分かったので行っていない(PM の了承あり)。

## 3. 原因の確定(3 つの独立した証拠)

1. **2 px のずれ**(`tables/shift-search.md`): 失敗したスクリーンショットを絵と比べると、(dx, dy) = (0, +2) だけが 0 画素で、他のずれは 28 万〜54 万画素。スクリーンショットは絵を 2 px 下にずらした像。
2. **介入実験**(`tables/scroll-intervention.md`): 同じページ・同じ canvas で `window.scrollTo(0, s)` だけを変えると、前任の clip は scrollY 0 で 0 画素、1 で 325,785、2 で 290,351 画素。文書座標の clip は常に 0。
3. **e2e 自身の記録**: engine-desk の run に scrollY / docHeight / innerHeight を記録させると、落ちた形では `scrollY 2, docHeight 1302, innerHeight 1300`。

`Page.captureScreenshot` の `clip` は**文書座標**で、`getBoundingClientRect()` は**ビューポート座標**。ページがスクロールしていなければ同じだが、スクロールしている間は clip がスクロール量だけ上にずれる。

**なぜ出荷構成だけ落ちたか**(`tables/layout-header.md`): 出荷構成 bundle の manifest の説明(`bundle.description`、393 字)が長く、`#bundle-info` が単独の行で 2 行に折り返し、header が 122 px、文書が 1302 px(試験窓は 1300 px)になって 2 px の縦スクロール範囲ができた。D8 の bundle の説明は 284 字で 1 行、header 72 px、文書 1300 px ちょうどでスクロールできない。スクロールは run ごとに起きたり起きなかったりする(旧 CSS・修正した検査での 10 run: scrollY 2 が 7 回、0 が 3 回。どちらも 50 検査すべて通る)。前任の `ident3.mjs` が通ったのは、その run が scrollY 0 だったからと読む(記録なし)。

**ユーザーの画面への影響**(PM の問い): 既定の入口(`?screen=` なし = `screen-fit`)は `body { height: 100vh; overflow: hidden }` と 1 行 36 px の header で、説明の長さに依らず文書は窓にちょうど収まる(`tables/layout-default-entry.md`: 文書 2400x1300 = 窓、scrollY 0、ゲストの画面 = 窓 − 340 − 36 と一致)。出荷構成 bundle の既定の入口で canvas・絵・スクリーンショットは一致(`tables/default-entry-four-way.md`)。2 px のはみ出しは legacy 配置だけ。

## 4. 直し(`desk3.diff`。runtime なし。FL の窓・クラスに依存しない)

| 場所 | 変更 |
|---|---|
| `test/e2e/engine-desk.mjs` | `shotOf` が毎回 `#desktop` の位置を読み直して `scrollX/Y` を足す(clip を文書座標にする)。呼び出し 7 か所の `rect` 引数を除く。ページの高さ・スクロール量の検査を 1 つ足す(`docHeight <= innerHeight`、`scrollY === 0`) |
| `test/e2e/browser.mjs` | `page.shot(x, y, w, h)` の x, y は client 座標と決め、`scrollX/Y` を足す(全呼び出し元が `getBoundingClientRect` を渡す) |
| `src/style.css` | header を 1 行に固定(`flex-wrap: nowrap`。`#bundle-info` は `flex: 1 1 0; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap`)。`?screen=` 配置にあった同じ規則を基本に移した |
| `src/main.ts` | `#bundle-info` の `title` 属性に全文 |
| `test/e2e/run.mjs` | `screen-legacy` と `screen-default` に『3,000 字級の説明にしても header の高さ・文書の高さ・desktop の位置が変わらず、省略記号で切れ、title に全文がある』 |
| `test/mutation/run.mjs` | 3 件: `header-note-wraps-again`、`header-note-title-not-set`、`desk_presenter-never-shows-a-whole-picture`(canvas が絵より古い = (A) の模擬) |

**検査を弱めていない**: 閾値(200 画素・箱 85x32)も比較も変えていない。変えたのは clip の座標だけ。それを示す 2 つ: (1) 旧 CSS の製品(scrollY 2 を含む)で修正した検査が通る(§5)、(2) (A) を模擬した製品(Worker が whole の提示を canvas に出さない)では、修正した engine-desk が落ちる(`desk_presenter-never-shows-a-whole-picture`: 『ウィンドウの更新が画面に届く』の 2 検査が落ちて検出)。

## 5. 検証

| 層 | 結果(最終の木、実測) |
|---|---|
| engine-desk 繰り返し(出荷構成 bundle、Chrome 156) | 新 CSS+修正した検査 **10/10 run が 51/51**(scrollY 0、文書 1300/窓 1300)。旧 CSS+修正した検査 10 run はスクリーンショット系 50 検査が 10/10 通過(scrollY 2 が 7 回を含む)。前任の検査+新 CSS 2/2(50/50)。D8+新 CSS 1/1(51/51)。表は `tables/repeats-*.md`・`tables/controls.md` |
| Firefox 158(要素のスクリーンショット vs 絵、5+5 サンプル) | 静止後 8/8 で箱の外 0 画素(`tables/firefox.md`) |
| e2e 全群・全 target(bundle なし、emcc あり、`--no-build`) | **893/893**(dev 335 + preview 287 + worker 271。887 + 新検査 6)。`compositor-synth` は dev で 48 PASS 行、SKIPPED 0 |
| e2e 出荷構成 bundle 付き(preview の全群 = スタンドインの群と実エンジンの群) | **462/462** |
| vitest | 1377 通過 + 7 skip(変化なし) |
| tsc(4 構成) | エラーなし |
| Worker(`test/worker/run.mjs`) | 50/50 |
| mutation(e2e 層、追加 3 件) | 3/3 検出(`tables/mutation.md`) |
| runtime・生成 C・フル試験・a4 | 触っていないので再実行していない(影響する経路が無いという読み) |


## 6. 点検表(`docs/policies/lift-fidelity.md` §2)

- 余計な処理の除去・追加: runtime・ランタイムの合成器・Worker の提示は変更していない。FL のクラス名・窓の種類・位置による分岐は足していない。CSS とテストだけ。新しい処理の分類は『検査の座標系の訂正』と『ページの配置の偶然(説明の長さ)への依存の除去』で、Win32 の契約・通信/変換の削減のどれにも当たらない(エンジンの動作に関わらない)。
- 窓の全書き込み経路が `fle_raster_wrote` を通るか(PM の手順 2 の『まず読み』): `lift/runtime` の `fle_raster_wrote` の呼び出しは `fle_gdi.c:1897`、`fle_layered.c:250`、`fle_win.c:2908`、`fle_wall_ia2w.c:111`(weak)。`fle_raster.c:769` が `fle_desk_wrote` を呼ぶ。窓の面への他の書き込み経路は網羅していない(原因が違うと分かったため中断)。**canvas = 絵 = 全再描画の一致が、損傷の欠落が無いことの実測**(静止後。メーターを除く)。

## 7. 出典のない選択(S-desk3-…)

| # | 選択 | 理由・代替・撤回条件 |
|---|---|---|
| S-desk3-header | legacy 配置の header を 1 行にして `#bundle-info` を省略記号で切る | 出典なし(ページ自身の見た目)。代替: (a) legacy 配置も `100vh` + `overflow: hidden`(却下: 1920x1080 の desktop が小さい窓で切れる)、(b) manifest の説明を短くする(却下: ビルドの記録)、(c) 折り返しを許し高さを予約(却下: 説明の長さに依存し続ける)。撤回条件: legacy 配置を使わなくなったとき |
| S-desk3-title | 切った説明の全文を `title` 属性に | 代替: クリックで開く(却下: 操作が増える)。撤回条件: なし |
| S-desk3-fit | engine-desk の『文書が窓より高くない・ページが先頭にある』の検査 | 試験窓 2400x1300 が legacy 配置の高さに収まることを守る。撤回条件: 試験窓の大きさを変えるとき |
| S-desk3-rereads | `shotOf` が毎回位置を読み直す | 最初に一度だけ読んだ rect は、その後のスクロールで古くなる。撤回条件: なし |

## 8. 自己申告・見つけたこと

- 作業中、実行中の繰り返しスクリプト(`rep.sh` の 10 回ループ)の途中でテストファイル(`engine-desk.mjs`)を書き換えた。3 回目以降の run は新しい検査(docHeight)を含み、旧 CSS の製品では 1 検査だけ落ちた(これは意図した検出で、50 検査の通過は変わらない)。`tables/repeats-old-css.md` に run ごとの内訳を書いた。
- 新 CSS の 10 回ループの最後の run(new-10)の実行中に、テストとスタイルのコメントだけを編集した(コード・検査は変えていない。new-10 は編集の前に読み込まれていた)。
- 一度 `sleep 1` を打って拒否された(規則違反。実行はされていない)。
- 初回の報告の『約 20 サンプル』は、`stale-1`〜`5` の静止後サンプル(k≥1)の概数で、表は `tables/four-way.md`。stale-1・2 は scrollY を記録していない(scrollY = 2 は rc.y の 208 と 2 行ずれから読む)。`tools/stale.mjs` の後の run は browser.mjs の修正後に走るため、`ssPage` 列は二重に補正される(scrollY 0 の run だけで使った)。
- PM の見立て『本番で縦スクロールバーが出る』は、既定の入口には当たらなかった(§3)。

