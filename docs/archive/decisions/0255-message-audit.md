# 0255(仮): runtime が窓手続きに送るメッセージの引数の全数点検(alpha6)

**依頼者の機械・実 GPU・実機 Windows での測定は 0 件。** 以下は、b42fad36 の生成 C(PM の計測で既定の生成 C は不変)から作った native N1(ランタイムは HEAD 4f054ecd = alpha5 / 5c / 5b 入り。e6433b23 は docs のみで runtime は同一)と、gdb から注入した操作・G3 の模擬入力(依頼者の操作ではない)での測定。**本番 bundle のバイト列(afd46e67 / 251dfabc / 次)での測定ではない**ので、本番の数字と混ぜていない。代役・画像の再生・推測は使っていない。「UNOFFICIAL」は Wine 10.0 のソースか私の読みで、Microsoft の文面ではない。

## 0. 先頭の 3 点

**測ったこと**

- runtime がゲストの窓手続きへ送る場所を全部拾った(`fle_win.c` の `send_to` / `call_wndproc` 40 か所、`fle_input.c` の入力・TrackMouseEvent・表示変更、`fle_appfile.c` の WM_COPYDATA、WM_PAINT / WM_TIMER の合成、ゲスト経由の素通し)。49 行の表 `tables/message-contract.tsv`(送り元、状況、メッセージ、こちらの wParam / lParam、Learn の契約、判定、10 場面での件数、FL の消費)。Learn は 72 ページを今日取得し本文と SHA-256 を保存(`tables/learn/`)。
- 実エンジン(10 本の G3 場面 + 参照プロジェクト + 右クリックの場面)で、窓手続きに届く全メッセージを (送り手 API ラベル、メッセージ、wParam の種類、lParam の種類)で数えた(新設 `A5_MSGTAB=1`。172 行 + 131 行 + 右クリック。`tables/empirical-messages*.tsv`)。
- FL がどの番号を処理するか: Delphi の `TObject.Dispatch`(f_410090)は動的メソッド表(`[VMT-0x90]`: 個数、メッセージ番号、コード番地)を引く。表を静的に探して **179 クラスの表から 333 個の番号**を得た(`tables/fl-dynamic-method-ids.tsv`、`tools/dmt-scan.py`)。TApplication の窓手続きだけは `case` 分岐(f_8689f0 付近)なので別に読んだ。
- 判定(49 行): 一致 37、不一致 4、不明(要実機)2、未送信 6。(第 3 報で S49 を『未送信』から『不一致』へ移し、S18 に実験結果を足した。)

**結論**

- **ゲストを止める/描き誤らせる、と実エンジンで言える『送っている引数の不一致』は 見つからなかった**(0253 の WM_ERASEBKGND wParam = 0 の後は 0 件)。ゲストが値を読む物は一致していた(NCCREATE / CREATE の CREATESTRUCT*、WM_SIZE の client 幅高さと SIZE_RESTORED(最大化で SIZE_MAXIMIZED が作られない点は S18 で別に扱う)、WM_MOVE の client 原点、WM_ACTIVATE / NCACTIVATE の相手窓、KILLFOCUS / SETFOCUS の相手窓、CAPTURECHANGED、NCHITTEST の画面座標、SETCURSOR / MOUSEACTIVATE の `(メッセージ << 16) | hit`、マウスの MK_* と座標系(MOUSEMOVE は client、NCMOUSEMOVE と MOUSEWHEEL は画面)、キーの lParam ビット、DISPLAYCHANGE ほか)。
- 不一致 4 件のうち 3 件(S11・S19・S46)は、**FL が値を使わない、または呼ばれる場面が実測で 0**。4 件目(S49 WM_CONTEXTMENU)は次々項:
  1. S11 `apply_pos` の WM_NCCALCSIZE を wParam = FALSE + RECT* で送る(Wine 10.0 の SetWindowPos は TRUE + NCCALCSIZE_PARAMS*。UNOFFICIAL)。FL のハンドラ 7 本は wParam を読まず lParam を『最初の RECT へのポインタ』としてだけ使い `Result := 0`(f_805290 ほか。逆アセンブルで確認)。**効果なし**。
  2. S19 owner の最小化と ShowOwnedPopups が owned window を隠さず、WM_SHOWWINDOW の lParam(SW_PARENTCLOSING 1 / SW_PARENTOPENING 3)も送らない(Learn の Window Features: 「An owned window is hidden when its owner is minimized」)。FL の TCustomForm 系ハンドラ f_861b60 は lParam を 1 と 3 で場合分けする。**注入で効果を測った**(下の §2): 全 top-level 窓に `WM_SHOWWINDOW(0, 1)` と `(1, 3)` を送ると、1 は IsIconic / IsZoomed を呼んで状態(通常 1 / 最大化 2 / 最小化 3)を窓の +0x4c3 に保存するだけ、3 は保存した状態で ShowWindow を呼び再描画する(ShowWindow を呼んだのは 27 窓のうち 8 窓、主フォームは 1 回)。停止なし(画は比較していない)。FL は ShowOwnedPopups を import するが 12 場面(10 場面 + 参照プロジェクト + 右クリック)で**呼ばれていない**。
  3. S46 WM_SETTINGCHANGE の lParam を 0 で送る(システムは文字列ポインタ)。表示変更の経路でしか送らず、10 場面で 0 件。
- 『送られていない』側(S07、S08、S18、S26、S32、S33)は §3(S49 は §3b)。PM の問い(WM_GETMINMAXINFO が到達するか)の答え: 実機の Windows は作成・最大化・サイズ変更で送るが、**この runtime は送らず、ゲストも要求しない**(12 場面で 0 件)。FL は起動で `ShowWindow(主フォーム, SW_SHOWMAXIMIZED)` を 2 回呼ぶが、主フォームの矩形 `(-5, -6, 1930 x 1092)` は FL 自身が `SetWindowPos`(`ra=0x616c9a`、VCL の UpdateBounds)で決めており、GETMINMAXINFO の応答ではない。`DefWindowProc(WM_GETMINMAXINFO)` の停止にゲストが到達するのは、ゲストが自分でこのメッセージを送ったときだけで、実測で 0 件。
- **WM_CONTEXTMENU(S49): 第 1・2 報の『FL は DefWindowProc(WM_RBUTTONUP)に到達しない』は誤りだった(是正)**。DefWindowProcW への呼び出しを gdb で記録したところ、実エンジンは右クリック 5 回のうち 4 回で `WM_RBUTTONDOWN` と `WM_RBUTTONUP`(TWPControl 2 回、TEdgeBrowser、TVectorPanel。TEventEditForm は DOWN だけ)を、キーボードでは `WM_KEYUP VK_APPS` と `WM_SYSKEYUP F10`(Shift 押下中)を、焦点のあった 3 窓すべてで DefWindowProc に渡す(`logs/ctx-*-base-defproc.txt`)。契約(Learn)ではここで DefWindowProc が WM_CONTEXTMENU を作るが、runtime は作らない。**作る変更を入れた A/B の native N1 で効果を測った**: メッセージは FL の VCL ハンドラ(TControl / TWinControl の WMContextMenu)に届くが、`CreatePopupMenu` / `TrackPopupMenu` は呼ばれず、窓表は完全一致(窓の作成・表示・非表示なし)、停止なし。画は 1 回の右クリックなら**完全一致**、5 回の右クリックとキー場面では時間依存の表示(アニメする要素の 1 画素幅の縁 13 画素、カウンタの数字、Shift ボタンの淡い光彩)だけが違う(§3b)。**『契約との差、効果なし』**。
- **WM_SIZE の SIZE_MAXIMIZED(S18)は、効果が実エンジンで測れた(向きは不明)**: FL の `TCustomForm.WMSize`(f_85a440)は WM_SIZE の wParam を WindowState(通常 / 最小化 / 最大化)に写して `SetWindowState` を呼ぶ。runtime は `ShowWindow(SW_SHOWMAXIMIZED)` で zoomed 旗を立てる(IsZoomed は TRUE)のに WM_SIZE を作らず、WM_SIZE の wParam は全場面で 0 のみ(`empirical-messages.tsv`)。『ShowWindow 直後に SIZE_MAXIMIZED の WM_SIZE を 1 回送る』実験(A/B。修正ではない)で主フォームの画が **29,885 画素**変わる(bbox (5,6)-(1923,216)。上部ツールバー全体のレイアウトが左半分は 1 画素左へ、右半分は 1 画素右へ)。**どちらが実機の Windows の画かは不明**(実機の参照がない)。契約上は SIZE_MAXIMIZED が来る側。矩形の決め方(WM_GETMINMAXINFO と最大化の矩形)も要るため、**直さずに表と §4 に残した**。
- **`alpha6.diff`(必須の修正)は作っていない**。理由: PM の規則(実エンジンでゲストが止まる/描き誤る根拠がある物だけ直す)に照らし、(a) S49 は効果なし、(b) S18 は効果が測れたが『契約側の画』が実機で確かめられず、しかも本番の画(主フォームの上部 3 万画素)を動かす。**任意の差分**として `optional/alpha6-optional-ctxmenu.diff`(S49 の契約どおりの実装 + C テスト + fe-lift テスト)を付けた。取り込むかは PM の判断(§4)。

**未確認**

- 実機 Windows の値(CreateWindow / SetWindowPos 時の WM_NCCALCSIZE の wParam、WINDOWPOS.flags の中身、WM_SHOWWINDOW が DestroyWindow で出るか、ShowOwnedPopups のメッセージの引数)。Wine 10.0 の挙動を UNOFFICIAL の根拠にした行は表に明記した。
- FL の動的メソッドの読みは、NCCALCSIZE(7 本)、SHOWWINDOW(2 本)、PARENTNOTIFY(1 本)、WINDOWPOSCHANGING の一部、WM_SIZE の 10 本のうち wParam を読む 3 本(f_85a440 = TCustomForm.WMSize、f_80c7f0、f_80ca40 は wParam == 2 を比較)と CONTEXTMENU の 9 本の入口だけ。WM_NCHITTEST(18)などの中身は読んでいない(件数だけ)。WM_WINDOWPOSCHANGED の 13 本は入口だけ読んだ: 中心は VCL の `TWinControl.WMWindowPosChanged`(f_6148a0。WINDOWPOS.flags `[+0x20]` を読む)で、runtime は flags に SWP_NOCLIENTMOVE / SWP_NOCLIENTSIZE を付けて渡す(Wine 10.0 に合わせた UNOFFICIAL。Learn はこの 2 旗の付け方を書かない)。この 2 旗の有無で FL の描きが変わる証拠は無い(読んだだけで、食い違い実験はしていない)。
- 最大化の画(S18): どちらが実機の画かは不明(要実機)。最大化・最小化の SC_* 経由、アプリの非活性化、表示変更の経路は測っていない。
- 時間依存の表示(カウンタの数字、アニメする縁)は、A/B の画素差がその位相ずれかを『同じ要素が基準側の別の時刻でも値を変える』ことで確かめた(1 画素の縁)。カウンタの数字と Shift ボタンの光彩は目視と位置だけで、時間依存かの測定はしていない。

## 1. 方法

| 項目 | 内容 |
|---|---|
| 送り元の全数 | `grep -n 'send_to(\|call_wndproc('` で 40 か所 + `fle_input.c` / `fle_appfile.c` / 合成(`fle_win_synth_message`)+ 素通し。各行の値は コードを読んで確認 |
| Learn の取得 | `tools/learn-fetch.py`(curl)。ページ本文の Parameters〜Return value〜Remarks と HTML の SHA-256 を `tables/learn/` に保存。WM_PARENTNOTIFY の Learn ページは 404 で取得できず(UNOFFICIAL のみ) |
| 実測 | N1 の記録器 `tools/alpha1_rec.c` に診断を足した: `A5_WINTRACE=1`(runtime の窓メッセージ記録を有効)+ `A5_MSGTAB=1`(窓手続きに届く全メッセージを種類別に数える)。種類は 0 / 1 / -1 / hwnd / dc / ptr / small / packed。10 場面 = `rp-steady / minres / mxmove / ui-pl / f8 / ui-menu / mxrep / ui-win / pull1 / mxstart`(`tools/a5b-run.sh`)、ほかに参照プロジェクト(`a4-ref.g3`)と右クリック(`a6-rclick.g3`) |
| FL の動的メソッド | `tools/dmt-scan.py FLEngine_x64.dll`。VMT は .text 内にあり、`[VMT-0x90]` の表を総当たりで探した。`... 0x83 0x86` のように番号を渡すとハンドラの番地が出る |
| 注入 | gdb から runtime の関数を直接呼ぶ(harness の模擬。依頼者の操作ではない): `pk54.gdb`(WM_SHOWWINDOW)、`pk52 / pk53 / pk55.gdb`(ShowWindow の cmd の記録) |

## 2. B1 の測定(owned window と WM_SHOWWINDOW の lParam)

| 注入 | FL の反応(API 呼び出し数。`logs/b1-show-effect.txt`) | 窓表・画 |
|---|---|---|
| 各 top-level 窓に `SendMessageW(h, WM_SHOWWINDOW, 0, 1)`(SW_PARENTCLOSING) | `IsIconic` 1、`IsZoomed` 1(状態の保存)。ShowWindow なし | 変化なし(窓表の比較はしていない) |
| 続けて `(h, WM_SHOWWINDOW, 1, 3)`(SW_PARENTOPENING) | 保存した状態で `ShowWindow` を呼び(27 窓中 8 窓)、再描画(BeginPaint / BitBlt ほか)。主フォームは `ShowWindow` 1 回 | 停止なし(画は比較していない) |

読み: FL の VCL は『owner が最小化されたら状態を覚え、復元されたらその状態で ShowWindow し直す』を実装しており、runtime が owned window を隠さない限り この処理は走らず、害もない。もし runtime が最小化で owned window を隠すようにするなら、復元で SW_PARENTOPENING を送る必要がある(送らないと FL の復元が走らず、ShowOwnedPopups(TRUE) は『ShowOwnedPopups で隠した窓だけ』を表示する Learn の規則で、最小化で隠した窓は対象にならない)。

## 3. 『送られていない』メッセージと FL の消費(全表 `tables/fl-handled-not-sent.tsv`: 63 番号)

| 番号 | メッセージ | 実機 Windows(Learn) | この runtime | FL | 判定・測定 |
|---|---|---|---|---|---|
| S07 0x24 | WM_GETMINMAXINFO | 作成・最大化・サイズ変更の前に送られる | 送らない。DefWindowProc は『未モデル化』で停止 | 2 クラス(f_808420、f_861ff0) | ゲストの到達 0 件。最大化の矩形は FL 自身が決める |
| S08 0x210 | WM_PARENTNOTIFY | 子窓の作成・破棄・クリックで親へ(Learn は取得不可。Wine は送る) | マウス押下だけ(61 件) | 1 クラス(f_615f20 = VCL の TWinControl.WMParentNotify: event が WM_CREATE / WM_DESTROY のとき子を調べる) | 作成・破棄の通知は無い。FL は WebView2 の子窓が無い環境では使わない見込み(未測定) |
| S18 | WM_SIZE(SIZE_MAXIMIZED)ほか | `ShowWindow(SW_SHOWMAXIMIZED)` で矩形が変わり WM_SIZE が来る | zoomed 旗だけ。WM_SIZE / WINDOWPOS なし。FL は起動で 2 回呼ぶ(主フォームに SIZE_MAXIMIZED は 0 回) | WM_SIZE 10 クラス。f_85a440(TCustomForm.WMSize)は wParam → WindowState | **(訂正)第 2 報までの『VCL の WindowState は IsZoomed で読めるので状態は一致』は誤り**: VCL は WM_SIZE の wParam から WindowState を作る。実験で主フォームの画が 29,885 画素変わる(§0)。向きは不明/要実機 |
| S26 0x1c | WM_ACTIVATEAPP | 別アプリの窓が活性化されるとき | 送らない | TApplication.WndProc の `case 0x1c`(f_868f80)が FActive(+0x139)を更新し CM_ACTIVATE / CM_DEACTIVATE を送る | FActive は TApplication.Create が True にする(f_8678ef)ので単一アプリでは一致。タブが非活性になる場合だけ欠ける |
| S32 0x85 | WM_NCPAINT | 枠の描画が必要なとき | 送らない(枠が無いモデル。ゲストが自分で 677 回送る) | 8 クラス | 枠の描画が無いので影響なし |
| S33 0x7c / 0x7d | WM_STYLECHANGING / CHANGED | SetWindowLong で | 送らない | 0 クラス | 影響なし |
| S49 0x7b | WM_CONTEXTMENU | DefWindowProc が WM_RBUTTONUP / WM_NCRBUTTONUP / Shift+F10 / VK_APPS から作る(Learn 公式) | DefWindowProc は 0 を返すだけ | 9 クラス | **(訂正)** 実エンジンは右クリックの 4/5 と VK_APPS・Shift+F10 のキー解放を DefWindowProc に渡す(§0)。契約との差。効果なし(§3b) |

## 3b. S49 WM_CONTEXTMENU の効果の測定(A/B の native N1。ビルド・手順は `MERGE-NOTES.md`)

入力はすべて G3 の模擬入力(依頼者の操作ではない)。基準側 = HEAD の runtime、変更側 = `optional/alpha6-optional-ctxmenu.diff` の変更。同じ 2 本の binary を環境変数で切り替えた(実験用の `A6_NOCTX=1`。製品の差分には無い)。基準側は gdb 付き・なしで同じ画になり(再現性あり)、変更側も同じ。

| 場面 | 基準側の DefWindowProc 到達(gdb) | 変更側 | 結果 |
|---|---|---|---|
| `a6-rclick.g3`(Basic.flp、5 か所を右クリック) | 0x204+0x205 を 4 回、0x204 のみ 1 回 | WM_CONTEXTMENU が 37 回 DefWindowProc に入る(子 → 親の連鎖。VCL の 2 回呼びで祖先ごとに 2 倍: 1,2,4,8,16。有限) | `CreatePopupMenu` / `TrackPopupMenu` 呼び出し 0、窓表 7 枚すべて一致、停止なし。画: base・r1 一致、r2 以降は 13 画素(1 画素幅の縁 x=1042, y=12..24。基準側でも同じ画素が時間で 5e666a → 424a4f → 545c61 と変わる = アニメの位相) |
| `a6-rc-one.g3`(1 回だけ右クリック) | - | - | r2 は**画素まで完全一致** |
| `a6-kbctx.g3`(クリックで焦点を 4 窓に動かし、各窓で VK_APPS と Shift+F10) | VK_APPS の keyup と Shift+F10 の SYSKEYUP が 4 回の焦点位置(窓は 3 枚)すべてで DefWindowProc に到達 | WM_CONTEXTMENU(lParam = -1)が 56 回 DefWindowProc に入る(同じ連鎖) | popup API 呼び出し 0、窓表 10 枚すべて一致、停止なし。画: base〜p2sf10 は一致、p3 以降はカウンタの数字(基準 "0"・変更 "1")と淡い光彩(Shift ボタン 28 x 31)だけ |

読み: FL の VCL ハンドラは WM_CONTEXTMENU を受けて PopupMenu を探すが、FL の窓には VCL の TPopupMenu が付いていないらしく(OS のメニューを作る呼び出しが 0。推定)、FL 自身のメニューは WM_RBUTTONDOWN / UP の処理の中で自分で開くらしい(右クリックの 1 回で `TQuickPopupMenuWindow` が開いた観測による推定)。つまり実機でも WM_CONTEXTMENU は FL では目に見える物を開かない、と推定する(実機の確認なし)。画の差は時間の位相。

## 4. 直す候補(必須の修正は無し。順位は『実機で見える害が出たら』)

1. **DefWindowProc が WM_RBUTTONUP / VK_APPS / Shift+F10 から WM_CONTEXTMENU を作る**(Learn 公式、FL 9 クラスが処理)。**実装・テスト・A/B の測定まで済み**(`optional/alpha6-optional-ctxmenu.diff`: 5 行の追加 case + テスト 17 項目。HEAD で 11 項目が FAIL、変更後 17/17、`git apply --check` 通過)。契約の欠落(外部 API の不足 (b))の穴埋めで、効果は測れなかった。**推奨: 今は取り込まない**(効果が見えない上、FL の VCL ハンドラの経路が新たに走る = 他の場面での停止の可能性が増える。10 場面で停止 0 は確かめたが、全場面ではない)。FL が実機でメニューを開くのに我々で開かない、という報告が出たら取り込む。
2. **owner の最小化・復元で owned window を隠し、SW_PARENTCLOSING / SW_PARENTOPENING を送る**(Learn 公式、FL が lParam を読む)。必要な根拠: FL の最小化の経路(SC_MINIMIZE がモデルで無効)と、依頼者の画面でどう見えるか。
3. `ShowWindow(SW_MAXIMIZE)` が WM_GETMINMAXINFO・矩形・WM_SIZE(SIZE_MAXIMIZED) を作る。**効果が実エンジンで測れた(上部ツールバー 29,885 画素の 1 画素ずれ)が、どちらが実機の画かが不明**。必要な根拠: 実機の FL の同じ場所の画(依頼者の機械)。実験の変更は 3 行(`fle_imp_ShowWindow` の `apply_pos` の後で、zoomed でなかった窓に `send_to(WM_SIZE, 2, 現在の client 幅高さ)`)だが、契約の全体(最大化の矩形)ではない。**この項目が本番の画に最も効く**ので、依頼者の実機の画との照合を優先する。

## 5. 逸脱の開示

- **逸脱 2 件(開示)**: (1) 背景コマンドの中で `( … ) &` と `wait` を 1 回使った(AGENTS.md は末尾 `&` を拒否と定めるが、受理されて走った。以後は別の背景シェルに分けた)。(2) `rm -rf work/t6b/lift/harness/alpha6` が、`t6b/lift/harness` が `out-alpha3/t6/lift/harness` へのシンボリックリンクであるため **共有のテストソース `ctxmenu.c` を消した**(自分の木。PM の木ではない)。同じ内容を作り直して HEAD 側・変更側のテストをやり直した(11 FAIL / 17 OK を再取得)。
- sleep・timeout・pkill -f・while は使っていない。cargo は走らせていない。
- 第 1・2 報の『FL は DefWindowProc に到達しない』(S49)と『VCL の WindowState は IsZoomed で読めるので状態は一致』(S18)は誤りだった。上に是正した。
- `lift/out-alpha3` は 500 MB 以下に保った(途中で 601 MB になった時点で runs / 古い木を削除)。
- 実 DLL(`work/dll` の 7 本)は 0253 のとき作り直したものを使った。

## 6. 成果物

`docs/`(この文書、`README-0255-line.txt`)、`optional/alpha6-optional-ctxmenu.diff`(任意。取り込み判断は PM)、`tables/`(`message-contract.tsv`、`fl-dynamic-method-ids.tsv`、`fl-handled-not-sent.tsv`、`runtime-stops-on-guest-results.tsv`、`empirical-messages*.tsv`、`learn/`)、`logs/`(B1 の注入結果、`ctx-*-defproc.txt` = DefWindowProc 到達の記録、`pixel-ab.txt` = A/B の画素比較、`ct-before.log` / `ct-after.log` = C テスト、`pics/` = 差の拡大)、`scripts/`、`tools/`(`learn-fetch.py`、`dmt-scan.py`、`a6-msgtab.py`、`a6-mk-table.py`、`a6-mk-notsent.py`、記録器)。

## PM の判断と取り込み(2026-11-05 20:50、docs のみ)

取り込み: alpha1 の最終報告(20:41)の文書・表・道具・任意差分を `docs/investigations/alpha6/` に、決定文書を `docs/decisions/0255-message-audit.md` に置いた。**コードの変更は 0**(`optional/` の 2 本は取り込まない)。測定は alpha1 のもの(native N1、b42fad36 の生成 C、HEAD 4f054ecd の runtime、G3 の模擬入力。依頼者の機械・実機 Windows・実 GPU・本番 bundle の値は 0 件)で、PM は再測定していない。

判断:
1. **採る: 49 行の表(一致 37 / 不一致 4 / 不明 2 / 未送信 6)と、「0253 の後、ゲストを止める/描き誤らせる引数の不一致は見つからない」という結論。** S11・S19・S46 は FL が値を使わない、または場面で 0 件なので、今は直さない。
2. **取り込まない: `optional/alpha6-optional-ctxmenu.diff`(S49 WM_CONTEXTMENU)。** 契約との差は本物(FL は DefWindowProc に WM_RBUTTONUP / WM_KEYUP VK_APPS を渡し、Windows ならそこで WM_CONTEXTMENU が作られる)だが、A/B で CreatePopupMenu / TrackPopupMenu の呼び出しは 0、窓表は完全一致、画の差は時間の位相だけで、効果が無い。FL の VCL の経路が新たに走る費用(wall_fr* の形のテスト、全 84/55/87 行の確認)に見合わない。差分は保管する(`docs/investigations/alpha6/optional/`)。実機で見える害(右クリックでメニューが出ない)が出たときに取り込む。`ab-experiment-NOT-FOR-MERGE.diff` は実験用で取り込まない。
3. **S18(最大化で WM_SIZE(SIZE_MAXIMIZED) が来ない)は直す。** 契約(Learn の ShowWindow / WM_SIZE)は SW_SHOWMAXIMIZED で WM_SIZE(SIZE_MAXIMIZED) を作ると定める。実エンジンで主フォームの画が 29,885 画素(上部ツールバーの寄りが 1 画素)動く。「どちらが実機の画か要実機」は、依頼者に頼めないので待たない。lift の忠実性の方針では、外部 API の契約を実装していないことが不具合の原因の 2 つのうちの 1 つであり、契約に書いてある挙動を欠いたまま画を比べない。次の担当(決定 0258 仮、alpha1)に回す。
4. alpha1 の是正 2 件(S49 の「FL は DefWindowProc に到達しない」は誤り、S18 の「WindowState は IsZoomed で読める」は誤り)を、そのまま事実として記録する。
