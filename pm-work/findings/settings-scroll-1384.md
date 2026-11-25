# settings-scroll (#1384): F10 の General 頁をスクロールすると描画が崩れる(判定: 受理)

実エンジン: `npm run build --prefix app`・`cargo build --release`・`fe-build engine`(約 20 分)・`fe-build bundle`(約 9 分)の後、`tools/boot-check/boot-bundle.mjs`(state engine-idle, standIn false, error null, elapsedMs 7111)、`tools/settings-scroll/measure.mjs`(DPR 1 と乱数の DPR 1.4 の 2 行を観測。下の「最終の値」)、`tools/smoke/welcome-flow.mjs`(流れ A〜D がすべて通過)、`npm test --prefix app`(71 files passed, 1401 tests passed)。
測定の値は runner の値。この機械とは別枠。nproc 4。loadavg は測定の前 1.33 / 1.76 / 1.97、後 2.22 / 2.12 / 2.08。

## 最終の値(runner の値。`tools/settings-scroll/measure.mjs`)

受け入れの 2 行(DPR 1 と乱数の DPR 1.58。この run の出力の抜粋):

    {"dpr":1,"state":"engine-idle","standIn":false,"error":null,"elapsedMs":6997,"preBootBytes":311125557,"mismatchPath":0,"mismatchRedraw":0,"blankMax":0,...}
    {"dpr":1.58,"state":"engine-idle","standIn":false,"error":null,"elapsedMs":7233,"preBootBytes":311125557,"mismatchPath":0,"mismatchRedraw":0,"blankMax":0,...}

受け入れコマンドの run(乱数の DPR 1.4)も、同じく `mismatchPath` 0、`mismatchRedraw` 0、`blankMax` 0。

## 直す前後の値(DPR 1)

| 試行 | 内容 | mismatchPath (a) | mismatchRedraw (b) | blankMax (c) |
|---|---|---|---|---|
| 直す前(前回の試行の最終の木。前回の記録) | 測定の道具の初期 | 0.5484 | 0.6804 | 0 |
| 1 回目 | 子の複写を直接の子だけ・移動後の無効化(効かなかった) | 0.5460 | 0.6923 | 0 |
| 2 回目 | 子孫まで複写を止める・移動時の親の無効域 | 0.0199 | 0.0277 | 0 |
| 3 回目 | 2 回目 + 露出した帯に消去を付ける | 0.0187 | 0.0267 | 0 |
| 最終 | 露出した帯の無効化で、窓と子孫を消去付きで描き直す | 0 | 0 | 0 |

乱数の DPR の結果: 2 回目で DPR 2.23 が (a) 0.0155、(b) 0.0191。3 回目で DPR 2.x が (a) 0.0152、(b) 0.0287。最終で DPR 1.4 と 1.58 が 0・0。

## 原因(実測の出力)

- 呼び出し(trace。`window-api:` の行。測定の時だけ出る):
  - `ScrollWindowEx H dx=0 dy=-1 scroll=1 clip=1 hrgn=1 prcUpdate=0 flags=0`(毎フレーム。アプリは露出域の矩形 hrgn を渡す)
  - `InvalidateRect H rect=(6,893,645,894) erase=0`(その直後。アプリが露出した帯を無効にする。消去は無し)
  - `move lid=160 parent=159 old=(24,555) new=(24,548) cx=0 cy=0 flags=0x1d pm=1 pmd=(0,-7)`(項目は頁 lid 22 の子ではなく、中間の窓 lid 159 の子)
- H1(ビットが動かずに返る): 外れ。`fle_win_scroll` は毎フレーム答え(`scroll-model lid=22 dx=0` が 190 回)。
- H2(後備えに落ちる): 外れ。後備えの経路は通らない。
- H3(子が親と別に描かれ、移動後に描き直されない): 当たり。ただし次の 2 つが重なっていた。
  1. 子孫の画素の二重移動。頁(lid 22)に WS_CLIPCHILDREN が無いので、`ScrollWindowEx` が項目の画素も一緒に動かす(T2b)。その後、項目の `EndDeferWindowPos`(flags 0x1d。NOCOPYBITS 無し)が同じ量だけ項目のビットを再び複写する。直接の子だけを対象にした 1 回目は効かなかった(項目は中間の窓 lid 159 の子のため)。子孫まで対象にすると効いた。
  2. 露出した帯の再描画の差。帯の無効化だけでは、描き直した項目の文字の縁が、消去なしで重ね描きされ、1 段ずれる(画素の値の例: `475055/485055`、`5d6467/5c6366`。全体に ±1 のずれ)。強制の再描画(消去あり)との差はこれ。消去を帯に付けるだけ(3 回目)では減らなかった。窓と子孫を消去付きで描き直すと 0 になった。
- 項目(lid 160〜168)は `BeginPaint` を呼ばない。描画は頁(lid 22)の描き直しと、項目自身の描き直しで行われる(`BeginPaint lid=22 erase=0` が 1862 回)。

## 直した箇所(`lift/runtime/fle_win.c`)

1. `fle_win_scroll`: 親(WS_CLIPCHILDREN の無い窓)の `ScrollWindowEx` で動いた、全ての子孫に印(`pm_set`・`pm_dx`・`pm_dy`)を付ける。頁の画素は子孫の画素と一緒に動くため(T2b)。
2. `apply_pos`: 印の付いた窓が、画面上の絶対座標で同じ量だけ動くときは、ビットを複写しない(`pm_carried`)。サイズの変更を伴う移動は対象外。
3. `fle_win_scroll` で露出域を記録(`scroll_unc`)。`fle_imp_InvalidateRect` で、その露出域を覆う無効化が来たら、`fle_win_redraw_tree` で窓と可視の子孫を消去付きで描き直す。強制の再描画(`settingsRedraw`)と同じ処理。
4. `fle_win_redraw_tree`: 測定の道具の口から呼ばれるほか、3 から呼ばれる(既定の動作で呼ばれる)。

理由: 2 は、子のビットが親のスクロールで既に正しい位置にあるため(二重の複写を避ける)。3 は、露出域の描き直しが、消去なしでは文字を重ね描きして色がずれるため。窓と子孫を消去付きで描き直すと、描く結果が描き直しの経路によらず同じになる(強制の再描画と同じ列)。

## 測定の道具(`tools/settings-scroll/measure.mjs`)

- つまみの列を、スクロールバーの中央(`scroller.x + floor(scroller.w / 2)`)で探す。DPR 2.23 では、+8 が縁に当たって「つまみが見つからない」で失敗していた。
- 測定のための一時の記録(trace の出力、差分の診断、画像の保存、DPR の固定)は戻した。

## 残り・注意

- 性能(P2 の 120 fps)は未測定。最終の 3. の規則では、露出域を覆う無効化のたびに、その窓と子孫を描き直す。設定窓のスクロールは 1 歩ごとに頁全体を描くことになり、描画量が増える。ほかの窓がスクロールで露出域を無効にする場合も同じ。範囲を絞る(露出域に重なる子だけを消去付きで描く)案は、描き直しの一致を確かめてから、#1348・#1350 の測定の道具で見る。
- ホイールは DPR ≠ 1 で頁を動かさない(測定の `range.M` が 0)。今回は未確認。測定は、つまみの経路で位置を決めている。
- 画像: 直す前の見本は `pm-work/findings/settings-scroll-1384-dpr1-p50-{down,up,redraw}.png`(前回の試行。DPR 1、位置 50%)。直した後は、ずれの大きい位置の画像は出ない。
- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数は足していない。強制の再描画の口(`window.__app.settingsRedraw`)は、前回の試行から測定の道具の口として残る。
