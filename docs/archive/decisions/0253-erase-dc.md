# 0253(仮): 『FillRect の hDC が DC でないでゲストが停止する』の原因は、RedrawWindow(RDW_ERASENOW / RDW_UPDATENOW)が WM_ERASEBKGND を DC なし(wParam = 0)で送っていたこと。DC を渡し(alpha5)、返り値の規則を揃え(alpha5b)、ソースの形を守る 2 本の試験を直した(alpha5c)

**依頼者の機械・実 GPU・実機 Windows での測定は 0 件。** 以下は、b42fad36 の生成 C(PM の計測で HEAD の生成 C と byte 同一)から作った native N1(ランタイム -O2)の G3 模擬入力と、gdb から注入した RedrawWindow(いずれも harness の模擬で、依頼者の操作ではない)での測定。代役・画像の再生・推測は使っていない。適用順は **alpha5 → alpha5c → alpha5b**(PM の複製で offset / fuzz / reject なしを確認済み)。

## 0. 先頭の 3 点

**測ったこと**

- 停止した FillRect の hDC は **0**(一度も発行されていない値)。A1(e52d024f)の steady の全 top-level 窓の一括再描画で再現し、gdb の bt は `fle_stop ← pixel_stop ← get_target ← fle_imp_FillRect ← f_614830 ← f_410090 ← f_60c3e0 ← f_612e10 ← f_612940 ← f_52a570(窓手続き)← fle_call_guest ← call_wndproc ← paint_now_tree ×5 ← fle_imp_RedrawWindow`、窓メッセージ記録の停止直前の行は `RedrawWindow lid=108 cls=TEdgeBrowser msg=0x14 wp=0`(`logs/a1-steady-stop-trace.txt`)。直前 80 件の DC 呼び出しは GetDC / BeginPaint / ReleaseDC / EndPaint が 20 / 20 / 20 / 20 で全部対(`logs/a1-steady-dc-history.txt`)。H1〜H4(保持中の DC の解放・DC の再利用・EndPaint 後の使用・古いハンドル)は値が 0 なので全部否定。
- 0 を渡したのは runtime: `fle_win.c` の `paint_now_tree`(RedrawWindow の RDW_ERASENOW / RDW_UPDATENOW)が `send_to(WM_ERASEBKGND, wParam = 0, 0)` で送っていた。Learn の WM_ERASEBKGND(2026-11-05 取得)は「wParam: A handle to the device context.」で、BeginPaint 側は GetDC(hWnd) の DC を渡している。`send_to(… FLE_WM_ERASEBKGND …)` は fle_win.c にこの 2 か所だけ。FL の `TEdgeBrowser`(lid 108)が `FillRect(Message.DC, …)` で受けると止まる。他のクラス(主フォーム、TWPControl、TSampleListForm、TVectorPanel ほか)は DC が 0 でも描かずに返す。
- **HEAD にも同じ不具合がある**: 全 top-level 窓の注入は HEAD では止まらない(注入が guest スレッド 18 の `Sleep(1)` の中で走り、UI スレッドの窓への送信が待ち行列を通って TEdgeBrowser に erase が届かない)が、TEdgeBrowser だけに `RedrawWindow(hwnd, NULL, NULL, 0x185)` を注入すると HEAD で止まる(steady でも minimize/restore のあとでも、`logs/head-before-fix-*.txt`)。『A3b2 で消えた』のではなく、再現の入口が変わっただけ(`tables/scene-matrix.tsv`)。
- **自然な場面では起きない**: 10 本の G3 場面(F8 ピッカー、Mixer、Playlist、メニュー、窓 ほか)× 3 runtime で、WM_ERASEBKGND は**全部 BeginPaint から(DC 付き)で、RedrawWindow から送られたものは 0 件**(`tables/a5b-erase-counts.tsv`)。FL がこの経路(RDW_ERASENOW / RDW_UPDATENOW で erase 旗付き)を自分で使うのは、これらの場面には無い。実環境で止まる確率は低いが、起きれば止まる契約違反。

**結論**

- **alpha5**(runtime): `paint_now_tree` が erase の前に `GetDC(hWnd)` の DC を取って wParam に渡し、応答後に `ReleaseDC`(BeginPaint 側と同じ作り)。fle_win.c +27 -1、クラス名・アドレス分岐なし、新オプションなし。
- **alpha5b**(runtime): erase を非 0 で返したら erase 旗を下ろす(Learn: 「nonzero … this indicates that no further erasing is required. If the application returns zero, the window will remain marked for erasing.」BeginPaint 側は既にこの規則)。fle_win.c +5 -1。GetDC / ReleaseDC の数は増えない。
- **alpha5c**(試験): wall_fr9.rs / wall_fr10.rs が守る fle_win.c のソースの形(`fle_imp_GetDC()` / `fle_imp_ReleaseDC()` が 1 / 1)を 2 / 2 に直し、wall_fr9 には どの関数の中か(BeginPaint 1/0、EndPaint 0/1、paint_now_tree 1/1)も守る形を足した。
- 試験: `harness/alpha5/erase_dc.c` + `alpha5_erase_dc.rs`。直し前 11 項目中 7 FAIL → alpha5 で 0 FAIL。alpha5b の項目(erase を 1 回だけ、0 を返したら BeginPaint が再度 erase して fErase = TRUE)を足して 13 項目、alpha5 だけの runtime に当てると 1 FAIL、alpha5b で 0 FAIL。cargo で `alpha5_erase_dc` を一度だけ走らせた(別置き場、削除済み): alpha5 で ok、直し前で FAILED(`logs/cargo-test-alpha5_erase_dc.txt`)。既存の `upd_region.c`(alpha3 の 40 項目)は alpha5 のあとも 40 / 0 FAIL。
- 画素と回数: 10 場面 × {HEAD, alpha5, alpha5b} の g3-surface / 窓表の sha256 行は**全部同一**。注入した全窓の一括再描画(steady・minres・steady+200 ms)も、直後の G3 snapshot の sha256 が 3 runtime で同一、erase の回数も同一(paint_now_tree 由来 1 回、BeginPaint 由来 294 / 19、wParam 0 は HEAD で 1 回・alpha5 以後 0 回。`tables/a5b-injected-repaint.tsv`)。

**未確認**

- 実機 Windows の WM_ERASEBKGND の DC(更新領域でクリップされているか)は 0 件。runtime は BeginPaint の erase と同じ作り(クリップなし)。
- alpha5c の 2 本(wall_fr9 / wall_fr10)の cargo 実行は alpha1 側では無し。`wall_fr9` の strip_comments と新しいループを写した rustc の小さい実行ファイルで、HEAD + alpha5 の fle_win.c が通り、HEAD の元の fle_win.c が新しい期待で落ちることを手で確認した(`logs/source-guard-counts.txt`)。alpha5 / 5c / 5b を当てた木の test.sh full と a4 は PM が回す。
- alpha5b の二重 erase(erase を送ったあとに続く BeginPaint がもう一度送る)は、C の harness では再現して直ることを確認したが、**実エンジンの注入では差が出なかった**(主フォームへの erase の後、200 ms の message loop でも BeginPaint 由来の erase は増えず、v0 = v1 = v2)。実エンジンでは二重 erase を観測できていない。
- A1 の minimize/restore の全窓再描画で alpha3 が見た停止は、この build の A1 では再現しなかった(steady だけ再現)。minimize/restore のあとでも TEdgeBrowser への注入は HEAD で同じ停止をする。
- 実機の TEdgeBrowser(WebView2 の子窓)が erase を受けたときに FillRect が先に消えるかは未確認。

## 1. 切り分け

| 仮説 | 予想された証拠 | 測ったもの | 結果 |
|---|---|---|---|
| H1: 注入した RedrawWindow がゲストの GetDC〜ReleaseDC の最中に入れ子で走り、保持中の DC を解放する | 停止した hDC が直前に解放された値 | hDC = 0。直前 80 件の GetDC / BeginPaint / ReleaseDC / EndPaint は 20 / 20 / 20 / 20 で全部対 | **否定** |
| H2: 外接矩形の大きな更新領域で入れ子の WM_PAINT が DC のキャッシュを壊す | A3b で起きない | 値が 0 なので DC のキャッシュは関係しない | 否定 |
| H3: EndPaint 後の使用 | hDC が EndPaint の引数と同じ | 同上(0) | 否定 |
| H4: runtime の DC 再利用で古いハンドルを取り違える | 古い世代のハンドル | 同上(0) | **否定** |
| H5(新): runtime が WM_ERASEBKGND を DC なし(wParam = 0)で送る | 停止の直前に 0 の erase が窓手続きに届く | `RedrawWindow lid=108 cls=TEdgeBrowser msg=0x14 wp=0`、続いて `fle: FillRect … hDC is not a DC` | **確定** |

## 2. 契約(Learn)と我々の答え

| 項目 | Windows の契約 | 直し前 | 直し後 |
|---|---|---|---|
| WM_ERASEBKGND の wParam | Learn WM_ERASEBKGND: 「wParam: A handle to the device context.」 | BeginPaint から: DC / RedrawWindow(ERASENOW・UPDATENOW)から: **0** | どちらも GetDC(hWnd) の DC(alpha5) |
| その DC の寿命 | Learn GetDC / ReleaseDC: 共通 DC は使い終えたら ReleaseDC する | - | メッセージの応答後に ReleaseDC(CS_OWNDC / CS_CLASSDC の窓は ReleaseDC が何もしない) |
| erase の返り値 | Learn WM_ERASEBKGND: 非 0 なら以降の erase は不要、0 なら窓は erase 待ちのまま | BeginPaint 側: 規則どおり(0 のときだけ fErase = TRUE)/ RedrawWindow 側: 返り値を見ず旗が残る | どちらも規則どおり(alpha5b) |
| 無効な hDC を FillRect に渡す | Learn FillRect: 失敗は 0(このページは今回は取得していない。記憶)。無効な DC の詳細は書かれていない | runtime は『定義されていない』ので停止(決定 0126) | 変更なし(この経路では起きなくなった) |

## 3. 3 本の diff と試験の対応(適用順 alpha5 → alpha5c → alpha5b)

| diff | ファイル | `-` の行 | 中身 | それを守る試験 |
|---|---|---|---|---|
| alpha5 | `lift/runtime/fle_win.c`(+27 -1)、新規 `lift/harness/alpha5/erase_dc.c`、新規 `lift/crates/fe-lift/tests/alpha5_erase_dc.rs` | `(void)send_to(api, i, FLE_WM_ERASEBKGND, 0, 0);` | erase の前に GetDC(hWnd)、wParam に渡し、応答後に ReleaseDC | alpha5_erase_dc(11 項目: wParam ≠ 0、その窓の DC、FillRect 成功、解放済み、ERASENOW のみ、親と子 2 つ、BeginPaint 側は不変)。直し前 7 FAIL → 0 FAIL |
| alpha5c | `lift/crates/fe-lift/tests/wall_fr9.rs`、`wall_fr10.rs` | 期待 `count(), 1` ×4(`fle_imp_GetDC()` / `fle_imp_ReleaseDC()`)と 2 本のコメント | 1 / 1 を 2 / 2 に。wall_fr9 に関数別(BeginPaint 1/0、EndPaint 0/1、paint_now_tree 1/1) | 自分自身(ソースの形の守り)。手での数え直し: HEAD 1/1、HEAD + alpha5 2/2、関数別は上のとおり(`logs/source-guard-counts.txt`) |
| alpha5b | `lift/runtime/fle_win.c`(+5 -1)、`erase_dc.c`、`alpha5_erase_dc.rs`(下限 13) | `(void)send_to(api, i, FLE_WM_ERASEBKGND, dc, 0);` ほか harness の 3 行 | 返り値が非 0 なら erase 旗を下ろす | alpha5_erase_dc の追加 2 項目(erase は 1 回、0 を返したら BeginPaint が再度送り fErase = TRUE)と、erase 1 回に直した 1 項目。alpha5 だけだと 1 FAIL、alpha5b で 0 FAIL |

## 4. 場面の表(抜粋。全体は `tables/scene-matrix.tsv`、`tables/a5b-erase-counts.tsv`、`tables/a5b-injected-repaint.tsv`)

| runtime | 場面 | 注入 | 結果 |
|---|---|---|---|
| A1(e52d024f) | steady | 全 27 窓に 0x185 | **停止**(主フォームの再描画中、TEdgeBrowser の erase、hDC = 0) |
| A1 | minimize/restore | 同上 | 停止せず |
| HEAD | steady / minres | 同上 | 停止せず(注入が thread 18 の中) |
| HEAD | steady / minres | TEdgeBrowser だけに 0x185 | **停止**(同じメッセージ) |
| HEAD + alpha5 | 同上 | 同上 | 停止せず。erase の wParam は DC(0x7fa919a0) |
| 10 場面(F8、steady、Mixer、Playlist、メニュー、窓 ほか) | 注入なし | - | erase は BeginPaint から だけ。HEAD / alpha5 / alpha5b で件数・sha256 同一 |

## 5. 逸脱の開示

- 1 回、`timeout` を含むコマンドを送って弾かれた(未実行)。
- 実エンジンの再現のため実 DLL の置き場 `work/dll`(7 本の rebase 済み)を作り直した(dsp は他の調査の置き場への symlink、残りはコピー)。
- cargo は、PM の「alpha1 側では作らない」の指示の前に、alpha5_erase_dc だけを `lift/out-alpha3` の外の一時置き場で 2 回(alpha5 後と直し前)走らせた。置き場は削除済み。以後は走らせていない。
- `tools/alpha1_rec.c` に診断用の入口を足した(`A5_WINTRACE=1`: 窓メッセージの記録と WM_ERASEBKGND の API 別の計数、`A5_DCHIST=1`: DC 呼び出しの履歴。N1 の記録器だけ)。
- PM の tree には書いていない。`lift/out-alpha3` は 500 MB 以下(最後に確認)。

## 6. 成果物

`alpha5.diff`、`alpha5c.diff`、`alpha5b.diff`、`docs/`(この文書、`README-0253-line.txt`)、`MERGE-NOTES.md`、`tables/`(scene-matrix、a5b-erase-counts、a5b-injected-repaint)、`logs/`(A1 の停止 bt と窓メッセージ記録、DC 履歴、HEAD の直し前、直し後、cargo の結果、ソースの形の数え直し)、`scripts/`(G3 場面と gdb スクリプト)、`tools/`(記録器、ビルド/実行、`build-erase-test.sh`、`a5b-*.sh`、`source-guard-check.rs`)。

## PM の判断と取り込み(2026-11-05 19:50)

取り込んだ基準: HEAD ba7619f9(runtime は cd4923f5 = 本番 bundle 251dfabc と同一)に、`alpha5.diff` → `alpha5c.diff` → `alpha5b.diff` の順で `patch --batch`(offset・fuzz・reject なし。git archive の写しで dry-run も rc=0)。`-` 行を読んだ: `fle_win.c` は `(void)send_to(api, i, FLE_WM_ERASEBKGND, 0, 0)` の 1 行(alpha5)と、その置換後の `(void)` を外して戻り値を受ける 1 行(alpha5b)だけ。試験は `wall_fr9.rs`・`wall_fr10.rs` の GetDC/ReleaseDC の件数の期待(1→2)とコメント、ハーネスの 3 行。FL のクラス名・窓名・アドレスの分岐、新しいオプションは無い。

PM が測った値(この機械、`lift/out-pmchk-a5c/`、alpha5+5c+5b の全体):
- FLEngine の既定の生成 C の sha256 **`34a376dbf23a30c62df5ce6d04c07ed9beb5b651dd82d608cf5bcc59af78062a`**(215,740,414 B)不変(lifter は触っていない)。
- shellcheck、`ci/test-*.sh` と `test-real-deploy.py` rc=0。typecheck rc=0、vitest 1,269 passed + 7 skipped、`test:worker` rc=0、エンジン無しの `test:e2e` rc=0。
- `test.sh full`: **201 results / 2,348 passed / 0 failed / 1 ignored**。alpha5 だけを当てた最初の実行では wall_fr9 と wall_fr10 の 2 件が落ちた(GetDC/ReleaseDC の件数を数える原文の形の試験が 1→2 に追従していなかった)。alpha5c がその 2 件の期待を直し、直後の全体で 0 失敗になった。
- a4(FRUITY 無効): **init_count 942/942、run_rank 895/895、functions_distinct 5,310、function_entries 1,009,737、started_but_not_entered 0**(不変。alpha5 だけの最初の実行も同じ数字)。

判断:
1. **採る(alpha5、5c、5b の 3 つとも)。** RedrawWindow の RDW_ERASENOW/UPDATENOW 経路が WM_ERASEBKGND の wParam に DC を渡さなかったのは、Learn の契約(wParam = デバイスコンテキストのハンドル)への違反で、我々の欠落(b)だった。FL の TEdgeBrowser(f_614830 が Message.DC を使う)が `FillRect(0, ...)` で止まる直接の原因。H1〜H4(DC の寿命の仮説)は alpha1 が DC の履歴 20/20/20/20 の対で除外した。HEAD でも TEdgeBrowser だけに RedrawWindow(0x185)を注入すると同じ停止が出るので、alpha3 前後に関係しない既存の不具合。
2. alpha5b(WM_ERASEBKGND が非 0 を返したら erase 要求を消す)も Learn の規則(BeginPaint が既にそうしている)の実装なので採る。
3. 効果の範囲: 自然な 10 場面では RedrawWindow 由来の WM_ERASEBKGND は 0 件(すべて BeginPaint 由来)で、g3-surface と窓表の sha256 は HEAD / alpha5 / alpha5b で同一。つまりこれは「この経路を叩く入力(minimize/restore の別順、注入された再描画)で FL が止まる」穴を塞ぐ変更で、現行の 10 場面の画面や性能は変えない。依頼者の機械でこの停止が起きていたかは 0 件の測定のため分からない。
4. 未確認(alpha1 の報告のまま): 実 Windows で erase の DC が更新領域にクリップされるか、二重 erase は C のハーネスでだけ再現(実エンジンでは出ない)、alpha5c の cargo 試験は alpha1 の側では走らせていない(PM が全体で確認した)。
