# 0258(仮): ShowWindow / 最大化・最小化・復元の契約を、メッセージの種類と順序まで実装する(alpha7)

**依頼者の機械・実機 Windows・実 GPU・Chrome での測定は 0 件。** 以下は、b42fad36 の生成 C(PM の計測で既定の生成 C は不変)から作った native N1(ランタイムは HEAD = 4f054ecd と同一の `fle_win.c`。基準 HEAD c0a256bf で `git apply --check` 通過)と、G3 の模擬入力・gdb からの呼び出し(依頼者の操作ではない)での測定。**本番 bundle のバイト列(afd46e67 / 251dfabc / df0512b0 / 次)での測定ではない。** node(wasm のテストビルド)の 84 / 55 / 87 行は走らせていない(PM が走らせる)。代役・画像の再生・推測は使っていない。「UNOFFICIAL」は Wine 10.0 のソース(`work/` に保存した win32u/window.c の `show_window` / `window_min_maximize` / `get_min_max_info`)か私の読みで、Microsoft の文面ではない。

## 0. 先頭の 3 点(最終版)

**PM 判断(第 2 報の受領): alpha7 を採る。** 取り込み: alpha5 / 5c / 5b(HEAD に入済み)の後、opt1 / mt3 の fe-wasmgen 3 本 + hyb 一覧(+40)と同じ 1 回の全面 CI ビルドに載せる(PM の HEAD 62ca650b に `git apply --check` 清潔、5 ファイル。私も確認)。PM の木で test.sh full と a4、node の 84 / 55 / 87 は別の担当(動く行の種類と原因を `tables/expectation-changes.tsv` / `tables/adoption-and-node-predictions.tsv` と照合)。docs/0258 はこの版をコードと同時に取り込む。

**測ったこと**

- `ShowWindow` の契約を Learn で取り直した(`tables/learn/`: ShowWindow、WM_SIZE、WM_GETMINMAXINFO、MINMAXINFO、WM_WINDOWPOSCHANGING / CHANGED、WM_NCCALCSIZE、WM_SHOWWINDOW、WM_MOVE、WM_ACTIVATE、WM_SYSCOMMAND ほか 26 ページ、本文と SHA-256)。呼び出しごとのメッセージ列を `tables/showwindow-contract.tsv`(21 行: Learn の文面 / Wine 10.0〔UNOFFICIAL〕/ 変更前 / 変更後 / 判定)にした。
- 実装した(`alpha7.diff`、§1)。`ShowWindow` は最上位窓の状態(通常 / 最小化 / 最大化)が変わるとき、新しい状態の矩形へ窓を動かし、`WM_GETMINMAXINFO`(最大化の矩形を窓手続きに尋ねる)→ `WM_SHOWWINDOW`(可視が変わるとき)→ `WM_WINDOWPOSCHANGING` → `WM_NCCALCSIZE` → `WM_WINDOWPOSCHANGED` → DefWindowProc が `WM_MOVE` と `WM_SIZE`(SIZE_MAXIMIZED 2 / SIZE_MINIMIZED 1 / SIZE_RESTORED 0、client 幅高さ)を送る。`DefWindowProc(WM_GETMINMAXINFO)` は停止をやめて 0 を返し、`DefWindowProc(WM_WINDOWPOSCHANGING)` は overlapped / THICKFRAME の窓に `WM_GETMINMAXINFO` を送る(寸法の切り詰めはしない)。クラス名・窓種・番地の分岐なし、新しい選択肢なし。
- C テスト `harness/alpha7/showwin.c` 30 項目: **HEAD では 11 項目が通り 18 項目が FAIL した後、`DefWindowProc(WM_GETMINMAXINFO)` の停止で止まる。alpha7.diff の後は 30 / 30**。fe-lift テスト `alpha7_showwin.rs`(34 個の .o、alpha5_erase_dc.rs と同形)。**cargo(HEAD + alpha7.diff を `~/work/a7-cargo` に写して実行): `alpha7_showwin` 1 passed、編集した `window`〔37 passed〕と `wall_fr9`〔30 passed〕、`wall_fr17`〔20〕、`wall_ia1w`〔7〕、`win2_act`〔2〕、`txt1_createsize`〔1〕、`alpha3_updregion`〔1〕、`present2_uncover`〔16〕すべて通過**(`logs/cargo-*`)。加えて、runtime / 窓に触る 44 本のテストファイル(`fle_win` や `fle_imp_` を使うもの。`--no-fail-fast`)を HEAD + alpha7.diff と HEAD の両方で実行: **HEAD + alpha7 は 44 ファイル 740 件通過・失敗 0、HEAD は 43 ファイル 739 件通過・失敗 0**(差は新しい `alpha7_showwin` の 1 件、`logs/cargo-runtime-pack-*.txt`)。fe-lift の残りのテスト(authenticode など runtime に触れないもの)は、私の写しに `tools/` が無く走らせていない(PM の全体テスト)。
- 実エンジン(native N1、起動 16 スナップ `cmp-short.g3`、参照プロジェクト `a7-ref.g3`〔8 s アイドル + 10 s 再生〕、`a7-refw.g3`〔30 s + 30 s〕)を HEAD と alpha7.diff で走らせ、(i) 窓表と g3 行、(ii) 主フォームの画、(iii) 窓手続きに届くメッセージ表、(iv) 往復、(v) 仮想 CPU の差を測った(§3)。

**結論**

- **主フォームの矩形が変わり、画面全体が動く**。FL は主フォームを (−5, −6, 1930 x 1092) で作り(`CreateWindowExW`)、起動で `ShowWindow(主フォーム, SW_SHOWMAXIMIZED)` を 2 回呼ぶ。変更前は旗を立てるだけで矩形は (−5, −6, 1930 x 1092) のまま。変更後は 1 回目で `WM_GETMINMAXINFO`(FL の VCL ハンドラ f_808420 / f_861ff0 は追従サイズだけを書き、最大化の矩形は既定のまま)→ 矩形 **(0, 0, 1920 x 1080)**(作業領域 = モニター。主フォームは WS_POPUP + 最小化 / 最大化ボックスで枠が無いので、Windows の既定の最大化矩形も作業領域〔UNOFFICIAL: get_min_max_info の frame = 0〕)→ `WM_SIZE(SIZE_MAXIMIZED, 1920 x 1080)`。FL の `TCustomForm.WMSize`(f_85a440)が wParam から WindowState を作るのは 0255 で示した通り。2 回目は何も送らない。
- **画の差(契約どおりにした帰結)**: 起動 16 スナップのすべてで lid 14 のサイズが 1930x1092 → 1920x1080、sha256 が変わる(`tables/g3-rows-changed.tsv`)。参照プロジェクトの 10 スナップも同じ。窓の画面上の位置(5 画素・6 画素の寄り)は `tables/layout-shift-ref-closed.tsv`、図は `pics/`。例: ブラウザ枠は (1, 82) → (6, 88)(右へ 5・下へ 6、幅 −2、高さ −12)、上部ツールバー TQuickControlBar は画面 (0, 0) 幅 1920 → (6, 6) 幅 1908(**左右に 6 画素の余白が出る**)。参照プロジェクトの 'closed' では画面 2,073,600 画素のうち 953,850 画素が違う(全体が動くため)。
- **停止は出ない**(起動 + 参照プロジェクト 60 s 再生 + 往復の注入で named stop 0)。**idle の仮想 CPU は変わらない**(参照プロジェクトの closed → idle30: progress +0.00%、`tables/cpu-events.tsv`)。起動は +0.9%、再生は −0.5 〜 +1.3%(区間の仮想時間が違うノイズ範囲)。
- **既存の期待**: `win_act.c` / `win_seq.c` / `win_upd.c` / `create_seq.c` の出力は変更前後でバイト単位に同一(985 / 49 / 18 / 87 行)。変わる期待は、`window.rs` と `wall_fr9.rs` の『DefWindowProc(WM_GETMINMAXINFO) が止まる』2 件(alpha7.diff に入れた。理由は §2)、node の 84 / 55 / 87 行の g3-surface lid 14 / g3-wintable / g3-progress / g3-snap(`tables/expectation-changes.tsv`)。
- **訂正(第 1 報の §5 は誤りだった)**: 主フォームを含む FL のフォームの `WM_NCCALCSIZE` ハンドラは **f_80b590 = `Result := 0` だけ**(RECT に何も書かない)で、client = 窓矩形 = 現在の nc_zero と一致する(§5)。f_805290 / f_806da0(client を縮める)は子窓・別クラス用で、測定した全ヒットで余白 0。窓手続きが NCCALCSIZE の RECT を変えた回数は起動と参照プロジェクトで 0(`tables/nccalcsize-answers.txt`)。→ **NCCALCSIZE の答えを client に反映する作業(alpha8)は FL の画を変えない**ので実装しない(PM 判断)。上部ツールバーの左右の 6 画素は FL 自身が client の内側に確保する余白(主フォーム [+0x690..] = 6 6 6 6)で、Windows でも窓を作業領域に最大化すれば画面内に出る。

**未確認**

- 実機 Windows の値すべて(Wine 10.0 の実走は UNOFFICIAL の参照: `tables/wine-vs-runtime.tsv`)。node(wasm)の行。本番 bundle。実機での作業領域(タスクバー分)は我々の画面(タスクバーなし)とは違う。
- 最小化: 矩形はそのまま(アイコン位置 −32000 を作っていない。`SM_CXMINIMIZED` は GetSystemMetrics が答えない)。最小化中の窓表の主フォームが (32, 0, 570 x 412) になる(570 x 412 は診断で見た FL の ptMinTrackSize と同じ値なので、FL の VCL が `SetWindowPos(cx = 0, cy = 0)` を最小サイズ制約で丸めたと読む。WINDOWPOS の書き換えの確認は未了)。WM_QUERYOPEN、SIZE_MAXHIDE / MAXSHOW、SW_OTHERZOOM / OTHERUNZOOM、SC_MAXIMIZE などの WM_SYSCOMMAND の ShowWindow への対応(製品で最小化すると戻す手段が無いので意図して外した)、CreateWindow 時の WM_GETMINMAXINFO(0255 S07)は実装していない。
- 往復(§3 iv): 主フォームの矩形は戻るが、最初の『復元 → 最大化』の後で上部ツールバーの小窓が 15 画素ほど動く(34 行)。2 回目以降は同一(0 行)。**ShowWindow に依らない FL の履歴依存**と読む(HEAD の runtime で SetWindowPos だけの往復でも、起動時の配置と縮めて戻した後の配置は 12 行違う: §5b iii)。
- `DefWindowProc(WM_WINDOWPOSCHANGING)` の寸法の切り詰め(Wine は ptMaxTrackSize / ptMinTrackSize で切る。UNOFFICIAL)はしていない: 主フォームの作成時の 1930 x 1092 が仮想画面 1920 x 1080 に切られるため。

## 1. 変更(`alpha7.diff`、`lift/runtime/fle_win.c` ほか 5 ファイル。+434 / −35)

| ファイル | 内容 |
|---|---|
| `lift/runtime/fle_win.c` | ① 窓表に `restore_max`(最大化から最小化した窓の復元で最大化に戻す)。② `get_min_max_info`: MINMAXINFO(ptReserved = 窓の原点、ptMaxSize = 作業領域、ptMaxPosition = 作業領域の原点、ptMinTrackSize 0 x 0、ptMaxTrackSize = 仮想画面)を作って `WM_GETMINMAXINFO`(wParam 0)を送り、窓手続きの書いた値を読む(戻り値は使わない)。③ `fle_imp_ShowWindow`: 最上位窓の状態遷移で矩形(最大化 = ②、復元 = 窓表の rest 矩形、最小化 = 矩形そのまま)を決め、`apply_pos(…, SWP_FRAMECHANGED | …)` に渡す。可視の変化・WM_SHOWWINDOW・活性化の規則は変更前のまま。④ `DefWindowProcW`: `WM_GETMINMAXINFO` は 0 を返す(NULL lParam は停止)。`WM_WINDOWPOSCHANGING` は overlapped / WS_THICKFRAME の窓に `WM_GETMINMAXINFO` を送る(THICKFRAME の停止は除去)。 |
| `lift/harness/alpha7/showwin.c` | 新規。30 項目(順序、wParam / lParam、矩形、IsZoomed / IsIconic、往復、窓手続きの上書き、隠れた窓、DefWindowProc の 2 case) |
| `lift/crates/fe-lift/tests/alpha7_showwin.rs` | 新規。alpha5_erase_dc.rs と同形(34 個の .o を -O1 で作り C テストを走らせる) |
| `lift/crates/fe-lift/tests/window.rs`、`wall_fr9.rs` | §2 の 2 件 |

方針の適合: 新しい処理はすべて (b) 外部 API の契約の実装(ShowWindow の状態遷移とメッセージ、WM_GETMINMAXINFO の送出と既定値、DefWindowProc の既定処理)。FL の挙動の補正・代行・クラス名や窓種による分岐・新オプションは無い。主フォームの矩形は `get_min_max_info` が窓手続きに尋ねた結果であり、FL の窓の位置を我々が決めてはいない。

## 2. `-` 行の説明(全 35 行を読んだ)

| diff の `-` 行 | 何か | 置き換え |
|---|---|---|
| `window.rs` 2 行、`wall_fr9.rs` 1 行 | 『DefWindowProc(WM_GETMINMAXINFO) が止まる』ことのテスト | 有効な MINMAXINFO は 0 を返す(C テストが固定)。NULL lParam だけが停止(テストを NULL lParam に変え、理由の文字列を新しい停止に合わせた)。**理由**: 停止の根拠は『モデルが枠と画面を持たない』で、最大化の実装でその根拠が無くなる。これを直さないと VCL の既定処理(DefaultHandler → DefWindowProc)で最初の最大化に停止する |
| `fle_win.c` の `WM_WINDOWPOSCHANGING` のコメント 2 行 + THICKFRAME の停止 1 行 | 『GETMINMAXINFO は未実装なので THICKFRAME で停止』 | Learn の文面どおり overlapped / THICKFRAME の窓に WM_GETMINMAXINFO を送る |
| `fle_win.c` の `case 0x0024u` の停止 2 行 | 同上(DefWindowProc の停止) | `FLE_WM_GETMINMAXINFO` で 0 を返す |
| `fle_imp_ShowWindow` の旗 12 行(iconic / zoomed の設定)と `if (was != now) {` | 状態の旗だけを立てる | 同じ旗を `fin`(最終状態)から立てる。子窓は従来と同じ旗(`want` が同じ)。最上位窓の `SW_SHOWNOACTIVATE` は通常化に加わる(Learn: 『most recent size and position』) |
| 同 `swp = NOMOVE \| NOSIZE \| NOZORDER \| NOACTIVATE \| (SHOW/HIDE)` ほか 9 行 | 可視が変わるときの SetWindowPos の旗と WM_SHOWWINDOW と隠すときの snapshot | インデントを 1 段深くして同じ内容で残す。旗は `NOZORDER \| NOACTIVATE \| (SHOW/HIDE)` に、状態遷移で FRAMECHANGED、矩形を渡さないとき NOMOVE \| NOSIZE を足す(値は変更前と同じ: 可視だけの変化では 0x97 / 0x57 のまま。win_act.c の出力が同一であることで確認) |
| 同 `apply_pos(api, i, 0, 0, 0, 0, 0, swp);` ほか 3 行 | 常に空矩形で SetWindowPos | 矩形が決まるときは新しい矩形を渡す。それ以外は 0, 0, 0, 0(同じ) |

## 3. 測定

**(i) 窓表と g3 行**: `tables/g3-rows-changed.tsv`(種類ごと)、`g3-rows-changed-detail.tsv`(行ごと)。cmp: lid 14 のサイズとハッシュが 16 / 16 行で変わる。wintable 16 / 16。progress 16 / 16。snap 16 / 16(clock_ms が 1〜3 仮想 ms)。lid 8 と g3-win は 0 行。lid 48(ウィザード)は最初の 1 行だけ(アニメの瞬間)。`fruity:` 行の warp_bilin の画素数 +146(最後の行)。ref / refw: lid 14、wintable、progress が 10 / 10、snap 9 / 10。他の種類(g3-win 23 行、他の lid の surface、timers、layered)は 0 行。**native の行は node(wasm)の行ではない**(ハッシュは opt1 の `g3-lines-*-b42fad36-*` と違う)。変わる『種類』と原因が引き継がれる。

**(ii) 主フォームの画**: 起動の最終スナップ(cmp e3300)で画面 790,481 画素が違う。参照プロジェクト closed で 953,850、play_b で 969,979。全体が (5, 6) 画素ずれ、右下の端が内側に入るため。ブラウザ枠は同じ絵が (+5, +6) に移っただけ(照合で 0 行差)。`pics/ref-closed-topleft.png`(基準 | 変更後 | 差)、`pics/cmp-e3300-topleft.png`、`pics/ref-closed-{base,new}-half.png`(全体の半分の縮小)。

**(iii) メッセージ表**: `tables/msg-delta.tsv`。cmp: `WM_GETMINMAXINFO` 0 → 1(ShowWindow 起点)、**WM_SIZE の wParam が 2 の行が 0 → 1**(`CallWindowProcW 0x5 small packed`)、ShowWindow 起点の WM_NCCALCSIZE 0 → 1、WM_WINDOWPOSCHANGING / CHANGED が各 +1。そのほか FL の再配置で WM_SIZE +29、WM_MOVE +9、WM_PAINT(Dispatch)1208 → 1289、WM_NCCALCSIZE(SetWindowPos 起点)309 → 340 など。

**(iv) 往復**(`tables/roundtrip.tsv`。gdb から主フォームに ShowWindow を呼ぶ harness の模擬): 最大化(FL が起動で済ませた状態)→ 復元 → 最小化 → 復元 → 最大化 → 最小化 → 復元 → 通常、と 復元 → 最大化 → 復元 → 最大化 → 最小化 → 復元 → 最大化。主フォームの矩形は期待どおり (0,0,1920x1080) ⇄ (−5,−6,1930x1092)(復元は作成時の矩形)で、最小化から最大化の窓を復元すると最大化に戻る。**窓表全体は戻らない**: 最初の『復元 → 最大化』のあと上部ツールバーの小窓 34 行が動く(2 回目以降は 0 行差)。変更前は全 9 ステップで窓表が同一(何もしないため)。

**(v) CPU / イベント**(`tables/cpu-events.tsv`): idle(参照プロジェクト closed → idle30、8 s と 30 s)の progress は +0.00% / −0.00%(2 本とも)。起動の最初の 6 s は +2.3%(layout が 1 回増える)。再生 10 s は +1.30%、30 s は −0.49%(区間の仮想時間が 10035 / 10124 ms と違う)。slices / switches は ±2%。

## 4. 期待が変わる所

`tables/expectation-changes.tsv`(16 行)。要点: node 84 行 = lid 14 の surface 16 行・wintable 16 行・progress 16 行・snap 16 行が変わる(原因: ShowWindow(SW_SHOWMAXIMIZED) の矩形と WM_SIZE。契約: ShowWindow の SW_SHOWMAXIMIZED、WM_GETMINMAXINFO の既定の最大化矩形)。55 行・87 行は lid 14 / wintable / progress が全スナップ(snap は 9 / 10)。他の種類は変わらない。変わる行を黙って更新していない。

## 5. FL の非クライアント余白(第 1 報の記述を訂正)

- **測定**(`tables/nccalcsize-answers.txt`、`scripts/pk64〜pk67.gdb`、診断 N1): FL の `WM_NCCALCSIZE`(動的メソッド 0x83)のハンドラ 7 本のうち、主フォームと TStepSeqForm など FL のフォームで走るのは **f_80b590**(`mov [rdx+0x18], 0 ; ret` = Result := 0。RECT に何も書かない)。Windows の契約では wParam TRUE で rgrc[0] を変えずに 0 を返すと client = 窓矩形(非クライアントなし)。f_805290(client を左 +[0x524]、上 +[0x528]、右 −[0x52c]、下 −[0x530] 縮める)と f_806da0(+[0x690] ...)は子窓・別クラスの物で、起動の 11 回のヒットはすべて余白 0。`apply_pos` が送る WM_NCCALCSIZE の RECT を窓手続きが変えた回数は、起動(cmp-short)と参照プロジェクト(a7-ref)のどちらでも 0。
- 6 画素(主フォーム。[+0x690..+0x69c] = 6 6 6 6)・4 画素(TStepSeqForm)は、FL が `SetWindowPos(607, 92, 620 x 677)`(作成時の (613, 98, 608 x 665) に 6 を足す)のように**自分で窓を client + 余白 x 2 に広げる**もの。余白は client の中にあり、FL がそこを自分で塗る/空ける。
- したがって、第 1 報の『FL の NC 余白を runtime が捨てるので、変更前も後も Windows の画ではない』『Windows の画は client (5, 6, 1910, 1068) に近い』は取り下げる。FL のフォームの NC は 0 で nc_zero は合っている。窓を作業領域 (0, 0, 1920, 1080) に最大化する alpha7 の画は、Windows(と Wine 10.0 の実走、`tables/wine-vs-runtime.tsv`)の最大化の結果と同じ矩形・同じ client になる。画面内の上部ツールバーの左右の 6 画素は FL の余白がそのまま見えるもの。
- `nc_zero` を使う箇所の一覧(別決定で NC を持つときの見積り): `fle_win.c` の窓表の pcx / pcy(client サイズ = 窓サイズ)、`client_origin`(R2)、`fle_imp_GetClientRect`(0, 0, w, h)、`fle_imp_AdjustWindowRectEx`(矩形をそのまま返す)、`apply_pos` の WM_NCCALCSIZE(RECT を送るだけで答えを使わない)、`defproc_windowposchanged` の WM_MOVE / WM_SIZE(格納された位置とサイズ)、`fle_win_input_view` の原点とサイズ、`fle_input.c` の NCHITTEST(窓矩形の内側 = HTCLIENT)、`fle_dwm.c` の拡張フレーム、表面の原点(窓の左上)。NCCALCSIZE の既定(DefWindowProc の答え)が従来と同じなら、これらは変わらない。FL の画が理由では実装しない。

## 5b. alpha7 の採否の論点(PM の質問への答え)

(i) **(−5, −6) を保つ別機構はあるか**: 無い。主フォーム(hwnd 0x7f990038)への位置・大きさの呼び出しは、`CreateWindowExW(−5, −6, 1930 x 1092)`、`SetWindowLongPtrW`(style)、`ShowWindow`(1 回目 cmd 1、続けて cmd 3 を 2 回)、`SetWindowPos(0,0,0,0, SWP_NOMOVE|SWP_NOSIZE, ra=0x80761a)`(z 順/活性化のみ)で全部。**`SetWindowPos` / `MoveWindow` / `DeferWindowPos` / `SetWindowPlacement` による位置・大きさの設定は、作成後に 1 回も無い**(HEAD と alpha7 の `api.log`、cmp / ref)。VCL の `WMGetMinMaxInfo`(f_861ff0。f_808420 が呼ぶ)の逆アセンブル: `ptMinTrackSize`(+0x18, +0x1c)と `ptMaxTrackSize`(+0x20, +0x24)を Constraints から書くだけで `ptMaxSize`(+0x8)と `ptMaxPosition`(+0x10)は書かない。診断 N1: 主フォームの答えで既定から変わったのは ptMinTrackSize = (570, 412) と ptMaxTrackSize = (16396, 16396) のみ。→ Windows の最大化の既定矩形(枠なしの WS_POPUP = 作業領域)がそのまま使われる。
(ii) **契約と Wine 10.0 の実走の一致**: `tables/wine-vs-runtime.tsv`(同じ Win32 プログラム `harness/alpha7/show_seq.c` を Wine 10.0 の user32 と runtime の両方で走らせた)。最大化: Wine は GETMINMAXINFO(maxsize 1920, 1080、maxpos 0, 0、mintrack 0, 0)→ WINDOWPOSCHANGING → NCCALCSIZE → WINDOWPOSCHANGED → MOVE → SIZE(2, 1920 x 1080)、窓 = client = 画面。runtime(alpha7)も同じ順・同じ値で、窓 = client = 画面。違い: ptMaxTrackSize(Wine 1932 x 1092)、NCCALCSIZE の wParam(S11)、WINDOWPOSCHANGING の旗(Wine 0x8020、runtime 0x34)、Wine の再描画メッセージ、最小化の矩形(Wine は (−32000, −32000, 160 x 31) で SIZE lParam 0。runtime は矩形を保つ)、隠れた窓の最大化で WM_SHOWWINDOW(Wine は送らない。Learn は送る)。UI が右 5・下 6 に動く・ツールバーの左右に 6 画素の余白、は **FL の窓を (−5, −6, 1930 x 1092) から作業領域へ動かしたことの帰結**で、Windows の契約に従う。画面が 1366 x 768 のとき(製品でページが表示サイズを渡す場合に相当。`tables/screen-sizes.tsv`)も同じで、FL は主フォームを (−5, −6, 1376 x 780) で作り、alpha7 では (0, 0, 1366 x 768) = 画面になる(HEAD は画面からはみ出す)。依頼者の画面で退行に見える可能性は残る(FL の余白が見えるようになるため)。
(iii) **alpha7 が直す物 / 直らない物**: 直る — FL の `TCustomForm.WMSize`(f_85a440)が wParam から WindowState を作る経路(SIZE_MAXIMIZED で wsMaximized、SIZE_MINIMIZED で wsMinimized、0 で wsNormal)が、ShowWindow の状態遷移で正しい値を受け取る。**測定**(`scripts/pk69.gdb`、cmp-short.g3): f_85a440 は HEAD で 52 回呼ばれ SizeType はすべて 0、alpha7 で 55 回呼ばれ SizeType = 2 が 1 回(Self = 主フォーム、lParam 0x4380780 = 1080 << 16 | 1920)。WM_SIZE の wParam が 2 の行が 0 → 1(f_80c7f0 / f_80ca40 の `wParam == 2` の比較が初めて真になりうる)。`IsZoomed` と WM_SIZE の状態が食い違わない。直らない — SIZE_MAXHIDE / MAXSHOW(Learn が挙げる。FL に読むハンドラは見つかっていない)、SW_OTHERZOOM / OTHERUNZOOM、最小化のアイコン矩形と lParam 0、WM_QUERYOPEN、SC_* → ShowWindow。往復で上部ツールバーの小窓が動く件は、**ShowWindow に依らない FL の履歴依存**と読む: HEAD の runtime で `SetWindowPos` だけを使い主フォームを (0,0,1920,1080) ⇄ (−5,−6,1930,1092) と往復させる注入(`scripts/pk68.gdb`、`tables/roundtrip.tsv` の rt3-base)でも、起動時の配置と『一度縮めて戻した』後の配置は 12 行違い(窓表の z を除く)、往復を重ねると同じ(0 行差)。alpha7 の往復の 34 行(最初の復元 → 最大化の後)も同じ性質で、最大化メッセージ列は Wine の実走と同じ(`wine-vs-runtime.tsv`)。最小化中の窓表の (32, 0, 570 x 412) は、FL の VCL が最小サイズ制約(診断の ptMinTrackSize = 570 x 412)で `SetWindowPos(cx = 0, cy = 0)` の大きさを丸めた値と読める(WM_GETMINMAXINFO の答えと一致)。

## 6. 規則・逸脱

- PM の木に書いていない(`git archive` で `lift/out-alpha3/work/t7base`、`t7` に写した)。sleep・timeout・末尾 `&`・`( … ) &`・pkill -f・while は使っていない。`rm -rf` は自分の `work/` と `runs/` の実ディレクトリだけ(symlink 先を消さない)。cargo は走らせていない。
- 重い処理は 2 本まで。`lift/out-alpha3` は終了時に 500 MB 以内(最終時点 337 MB。N1 binary 2 本、診断 binary、Wine prefix を消した)。cargo は PM の木ではなく `~/work/a7-cargo`(git archive の写し)で実行した。`sleep 1` を 1 回打ってしまい、拒否された(実行されていない)。
- 作った画像の dump(約 1 GB)は枠外の `~/work/a7-dump` に置き、終了時に消した。

## 7. 成果物

`docs/`(この文書、`README-0258-line.txt`)、`alpha7.diff`、`tables/`(`showwindow-contract.tsv`、`adoption-and-node-predictions.tsv`、`wine-vs-runtime.tsv`、`screen-sizes.tsv`、`nccalcsize-answers.txt`、`expectation-changes.tsv`、`g3-rows-changed*.tsv`、`msg-delta.tsv`、`roundtrip.tsv`、`cpu-events.tsv`、`layout-shift-ref-closed.tsv`、`learn/`)、`logs/`、`scripts/`(G3 場面 `a7-*.g3`、gdb `pk60`〜`pk63.gdb`)、`tools/`(`build-showwin-test.sh`、`build-winact.sh`、`a7-vis-diff.py`、`a7-shot.py`、`learn-fetch.py`)、`pics/`。

## PM の判断と取り込み(2026-11-05 22:15)

取り込み: alpha1 の最終報告(22:05)の文書・表・道具・差分を `docs/investigations/alpha7/` に、決定文書を本ファイルに置いた。`alpha7.diff`(35,781 B、sha256 `00a9b648…`、5 ファイル: `lift/runtime/fle_win.c`、`lift/crates/fe-lift/tests/{wall_fr9.rs,window.rs,alpha7_showwin.rs}`〔後者は新規〕、`lift/harness/alpha7/showwin.c`〔新規〕)を、HEAD 62ca650b の `git archive` 写しに `patch --batch -p1 --dry-run` で確認(rc=0、offset・fuzz・reject なし)、PM の木に当てた。`-` 行 35 本を読んだ: `wall_fr9.rs` の `stop-defproc-minmax`(1 行)、`window.rs` の `win-abort-def-minmax`(2 行)、`fle_win.c` の 32 行(WS_THICKFRAME に対する WM_WINDOWPOSCHANGING の停止、`case 0x0024`〔WM_GETMINMAXINFO〕の `fle_stop`、ShowWindow の状態ブロック〔iconic/zoomed の旗、`was != now` の可視判定、`apply_pos(0,0,0,0)`、WM_SHOWWINDOW の送出〕を、契約の列〔WM_GETMINMAXINFO → WM_SHOWWINDOW → WM_WINDOWPOSCHANGING → WM_NCCALCSIZE → WM_WINDOWPOSCHANGED → DefWindowProc の WM_MOVE / WM_SIZE〕に置き換える)。FL のクラス名・窓名・アドレスの分岐、新しいオプションは無い。

**採用(PM の判断)**: alpha7 を採る。根拠: (1) 最大化後に主フォームを (−5,−6) に保つ仕組みが lifted コードに無い(HEAD にも alpha7 にも作成後の SetWindowPos / MoveWindow / DeferWindowPos / SetWindowPlacement が無く、VCL の WMGetMinMaxInfo `f_861ff0` は ptMinTrackSize / ptMaxTrackSize だけを書く)。(2) Wine 10.0(UNOFFICIAL)の同じプログラムの実走が、順序と値(GETMINMAXINFO → WINDOWPOSCHANGING → NCCALCSIZE → WINDOWPOSCHANGED → MOVE → SIZE(2, 1920x1080)、窓 = client = 画面)で一致。(3) Learn の ShowWindow / WM_GETMINMAXINFO / WM_SIZE の契約。(4) FL のフォームの NC は 0(主フォームの WM_NCCALCSIZE ハンドラは `f_80b590` の Result := 0 のみ)で現在の nc_zero と一致し、NCCALCSIZE の答えを client に反映する案(alpha8)は画を変えない(診断で変化 0)ので実装しない。(5) 小さい画面(1366x768)で HEAD は主フォームが溢れ、alpha7 は収まる。(6) cargo の fe-lift テスト: HEAD 43 ファイル 739 passed、alpha7 を当てて 44 ファイル 740 passed / 0 failed(alpha1 の測定)。(7) 停止なし、idle の仮想 CPU は不変。

**画の帰結(契約どおりにした結果として記録)**: UI 全体が右へ 5・下へ 6 画素動く(ブラウザ枠 (1,82)→(6,88))、上部ツールバーの左右に 6 画素の余白が画面内に出る。この 6 画素は FL 自身が client の内側に持つ余白(主フォームの [+0x690..0x69c] = 6 6 6 6)で、変更前は作成矩形 (−5,−6,1930x1092) のために画面外に隠れていた。依頼者の実機 Windows との照合は 0 件(要実機)で、待たない。

PM が測った値(この機械、PM の木 = 62ca650b + `opt1-f.diff` → `mt3-wasmgen-small.diff` → `mt3-x87-lower.diff` → `opt1-f-lists.diff`〔+40〕→ `alpha7.diff`、`lift/out-pmchk-stk2/`):
- cargo build(fe-lift・fe-wasmgen)rc=0。FLEngine の既定の生成 C の sha256 **`34a376dbf23a30c62df5ce6d04c07ed9beb5b651dd82d608cf5bcc59af78062a`**(215,740,414 B)不変(alpha7 は runtime のみ)。
- shellcheck と `ci/test-{hybrid,needs,lifecycle,wrangler-config,b2d-key}.sh`・`test-real-deploy.py` すべて rc=0(`test-hybrid` は 3,853 / elastique 155 / oggio 46)。
- `test.sh full`: **206 results / 2,363 passed / 0 failed / 1 ignored**(`alpha7_showwin` を含む。1930x1092 を固定する既存のテストは落ちなかった)。
- a4(FRUITY 無効): **init_count 942/942、run_rank 895/895、functions_distinct 5,310、function_entries 1,009,737、started_but_not_entered 0**。

node の 84/55/87 行は、alpha1 の予測表(`tables/adoption-and-node-predictions.tsv`)のとおり、次の**種類**の行が変わる想定: 84 行は g3-surface の lid 14(1930x1092 → 1920x1080 とハッシュ)・g3-wintable・g3-progress・g3-snap が各 16 行、lid 48 は最初の 1 行だけ、`fruity:` の warp_bilin の計数; 55/87 行は lid 14・wintable・progress が全 10 スナップ、snap が 9/10; g3-win(23 行)・他の lid の surface・timers・layered は不変。原因: ShowWindow(SW_SHOWMAXIMIZED) が主フォームを作業領域 (0,0,1920x1080) にし WM_SIZE(2) を送ること。保存行を黙って更新しない: 最終木の実エンジンの行を opt1 が取り(増分のテストビルド)、予測と**種類と原因**で照合してから、保存行の更新を別に判断する。

未実装の穴(表に残す): 最小化のアイコン矩形と WM_SIZE の lParam、ptMaxTrackSize の値(Wine 1932x1092、こちら 1920x1080)、WM_QUERYOPEN、SIZE_MAXHIDE / MAXSHOW、SW_OTHERZOOM / OTHERUNZOOM、WM_SYSCOMMAND(SC_*)から ShowWindow、CreateWindow 時の GETMINMAXINFO、DefWindowProc の寸法の切り詰め、NCCALCSIZE の wParam TRUE。往復(復元 → 最大化)で上部ツールバーの小窓 34 行が約 15 画素動くのは FL の履歴依存(SetWindowPos だけの HEAD でも同じ)。
