# 0262(仮): Mixer ヘッダードラッグの 1 手あたりの CPU — 「パネル全面の再描画」は FL 自身の RedrawWindow。runtime 側の過剰が 2 件あった(隠れた窓の古い更新矩形、RDW_NOCHILDREN の値の取り違え)(alpha8)

**依頼者の機械・実機 Windows・実 GPU・ブラウザ・本番 bundle(afd46e67 / 251dfabc / df0512b0)での測定は 0 件。** 以下は、b42fad36 の生成 C(PM の計測で既定の生成 C は不変)から作った native N1 に、HEAD の runtime(+ 環境変数で切る診断トレース `diag/a8-trace.diff`)を載せ、G3 の模擬入力(Mixer ヘッダードラッグ 40 手、`scripts/mxmove.g3`。**ユーザーの操作ではない**)で測った値。native N1 の行は本番 bundle の行ではない。Wine 10.0 の結果と私の読みは UNOFFICIAL(Wine は Windows ではない)。代役・再生・推測は含まない。

状態: **仮の決定**。作業者 `alpha1`、2026-11-05 JST 22:03〜。基準 HEAD は e4954229(0258 の alpha7 取り込み済み)。`alpha8.diff`(sha256 ea70232c…)は HEAD e4954229 の `git archive` に `git apply --check` が通り、`patch -p1 --dry-run` もオフセット・fuzz なしで通る(PM の木には既に当たっている。runtime とテスト 4 ファイルは私の検証用の複製と同一で、違うのはテストツール `move_paint.c` だけ: 後から足した `nohid` 引数は `optional/move_paint-nohid.diff`、27 行・追加のみ)。

## 結論(先頭の数行)

1. **FL の Mixer ヘッダードラッグで 1 手に出る「パネル全面(757,000 画素)の再描画」は、FL 自身の `RedrawWindow` が作っている。** 1 手あたり FL が呼ぶのは `SetWindowPos(lid 64, flags 0x14)` 1.00 回と `RedrawWindow` 6.97 回(`f_820f10+0xe7` hrgn 付き 2.62、`f_820f10+0x11c` 0.92、`f_820d60+0x7f` 1.26〔どちらも 0x41 = INVALIDATE | NOCHILDREN〕、`f_11f0270+0x131` 主フォームの 1 画素帯 2.18〔0x1〕)。`RedrawWindow(lid 64, hrgn, 0x41)` の hrgn は枠形の 5 片(57,901 画素)で、その外接矩形が (0,0,1514,500) = 757,000 画素。`BeginPaint` の rcPaint はその外接矩形(`GetUpdateRect`: 「更新領域を完全に囲む最小の矩形」。Windows でも同じ読み)。runtime の窓管理の無効化を全部止めても(`A8_NOWM=1`)FL の `RedrawWindow` の回数は 1 つも変わらない(lid 17 99、lid 14 88、lid 64 86 回/40 手)。タイマー(`TSharedAnimTimerWindow` msg 0x8ffc、約 11 ms)の直後に出る。**契約側の取りこぼしではない。**
2. **「1 手に 4 回」の 4 は `TFXForm` の `BeginPaint` ではない。** 1 手あたりの `BeginPaint` は 6.36 回(lid 64 が 1.90、lid 17 が 1.74、lid 14 が 2.21、lid 16 が 0.49)で、4.05 回/手なのは `TVectorPanel` の WM_PAINT(ValidateRgn で答えるだけの子窓)。実際に書かれる画素は `BitBlt` のクリップで 1 手 40 万画素(名目 111 万画素)。
3. **runtime 側の過剰が 2 件あった(方針の (b): 外部 API の契約の実装不足)。どちらも直した。効果は小さい。**
   - **A. 隠れた窓の古い更新矩形が、移動の「ビットのコピー」を止めていた**(`unc_pending`)。FL の `TQuickScrollBoxScroller`(lid 87)は `SWP_SHOWWINDOW` → サイズ変更(0x114)→ `SWP_HIDEWINDOW` と、一度も描かれずに隠れ、1254x19 の更新矩形が残る。S-0247-hidden(Wine 10.0: 見えない窓に更新領域は無い)に反する。この矩形が lid 64 のコピー元から下端 19 行を外し、毎手 MIX・PANA・PANB・SUB の 19 行が無効になって FL が描き直していた。Wine 10.0 の同じ呼び出し列では、移動で無効になるのは親(WP)の 15x500 の帯だけ。
   - **B. `RedrawWindow` の定数が入れ替わっていた**: `RDW_NOCHILDREN` = 0x0800、`RDW_NOFRAME` = 0x0040 としていたが、SDK のヘッダ(mingw-w64 の winuser.h を確認。UNOFFICIAL)は NOCHILDREN 0x0040、NOFRAME 0x0800。FL の 0x41(INVALIDATE | NOCHILDREN)が「子も無効化」と読まれ、Mixer の caption(lid 85)や rack の lid 96 / 98 に毎手 WM_PAINT が飛んでいた。
4. **効果**(`tables/drag-stats.tsv`、`tables/cpu-per-move.tsv`): 1 手あたり runtime 由来の update_rect の追加 840,710 → 88,033 画素(-90%)、実書込み画素 399,959 → 80,763(-80%)、lid 64 の `BeginPaint` 1.90 → 1.38、`TVectorPanel` の WM_PAINT 4.05 → 0.97、`ValidateRgn` 7.23 → 1.51。native の CPU は 1 手 HEAD 9.96 ms(6 回)→ A 修正 9.55 ms(6 回)。A + B は 9.68 対 同ラウンドの HEAD 10.54(3 回)。差は 0.4〜0.9 ms/手(4〜8%)で、同一機械の揺れ(±1 ms)と同程度。**ブラウザ(wasm)の 8〜10.5 ms/手への効果は未測定。** 残りは FL 自身の描画(lid 64 に 1.38、lid 17 に 1.74、lid 14 に 2.21 回の `BeginPaint`)。

## 冒頭の三点(正直さ)

1. **何を実測したか**: 実エンジン(`FLEngine_x64.dll` と同梱 DLL を x86-64 → C にした b42fad36 の生成 C)を native N1 として動かし、G3 の模擬入力で Mixer のヘッダードラッグ(guest (715,564) → (1015,564) → 戻り、20 歩 × 往復、16 ms 間隔)を 7 回の trace 付き走行(作業ディレクトリの `runs/mx1〜mx9`。要約は `tables/`、drag の A8 行は `logs/drag-traces/`)と 15 回の時間測定走行で測った。1 手 = `SetWindowPos(lid 64, flags 0x14)` 1 回(39 手)。Wine 10.0(mingw、headless)と runtime に同じ C テスト `harness/alpha8/move_paint.c` を走らせて比べた。**本番 bundle のバイト列での測定ではない。**
2. **結論**: 上のとおり。FL 自身の呼び出しが支配。runtime の過剰が 2 件(小)。
3. **未確認**: 依頼者の機械・実 Windows・ブラウザでの 1 手の ms(8〜10.5 ms)に対する効果。node(wasm)の 84 / 55 / 87 行、a4、全体テスト(`test.sh full`)。FL の `RedrawWindow` の輪が Windows でも同じ頻度で出るか(私は Windows を持たない: FL の呼び出しはタイマー駆動で、runtime が返す値には依存しないと読んだが、確かめてはいない)。どちらの画が実 Windows に近いか(§6.3)。

## 1. 問い

PM の問い: Mixer ヘッダードラッグの CPU 8〜10.5 ms/手の鎖のうち、「パネル全面の再描画が 1 手に 4 回」は FL 自身の `Invalidate` / `RedrawWindow` / `UpdateWindow` か、我々の `SetWindowPos` / `WM_WINDOWPOSCHANGED` / `WM_NCCALCSIZE` / `DefWindowProc` / 更新領域の扱いの過剰または不足(ビットのコピー、`SWP_NOCOPYBITS`、有効領域の扱い、Learn の SetWindowPos「クライアント領域のコピー」)か。

## 2. 1 手の中身(契約側から)

表: `tables/one-move-api-sequence-HEAD.txt`(1 手の A8 行そのまま、戻り番地に関数名を付けた)、`tables/callers-per-move.tsv`、`tables/window-messages-per-move.tsv`。

| 順 | 誰が | 呼び出し | 備考 |
|---|---|---|---|
| 1 | FL(`f_616c10+0x8a`) | `SetWindowPos(lid 64, x+15, 466, 1514x500, 0x14)` | 窓へ `WM_WINDOWPOSCHANGING` → `WM_WINDOWPOSCHANGED` → `WM_MOVE`(DefWindowProc が送る)。`WM_NCCALCSIZE` / `WM_SIZE` は来ない(サイズ不変。Learn: 「`WM_NCCALCSIZE` はサイズが変わる時だけ」) |
| 2 | runtime(窓管理、決定 0214) | ビットをコピーし、動かして露出した部分だけ無効にする | lid 16 に (x, 466, x+15, 966) = 7,500 画素 = 契約どおり。HEAD ではこれに加えて lid 64 / 86 / 83 / 85 の 19 行(§4) |
| 3 | runtime → FL | `WM_PAINT` を lid 16 に → `BeginPaint`(`f_8261a0+0xde`)→ `BitBlt` → `EndPaint` | 15x500 の帯だけ |
| 4 | FL(タイマー 0x8ffc の直後、約 11 ms 後) | `RedrawWindow(lid 17, hrgn, 0x41)`、`(lid 17, rect, 0x41)`、`InvalidateRect(lid 100)`、`RedrawWindow(lid 64, hrgn, 0x41)`、`(lid 64, rect 4,4,1510,27, 0x41)` ×2、`InvalidateRect(lid 89)` | FL の輪(枠形)の無効化。**ここで全面の外接矩形ができる** |
| 5 | runtime → FL | lid 17 / 64 の `WM_PAINT` → `BeginPaint`(rcPaint = 外接矩形)→ FL が `GetUpdateRgn` で片を読み、重なるものを描く(決定 0247 §2.1)→ `BitBlt`(名目 757,000、書込み 54,142 画素) | 書込みは BitBlt のクリップで更新領域の画素だけ |
| 6 | FL | `RedrawWindow(lid 14, (377,152,378,1022), 0x1)` | 主フォームの 1 画素帯。静止時も出る |

静止時(ドラッグなし)は手順 4 のうち lid 14 の帯だけが出る(決定 0247 §2.1 の観察と同じ)。

### 2.1 我々の無効化を止めても FL の呼び出しは変わらない

`A8_NOWM=1`(窓管理が何も無効にしない診断)の走行 `runs/mx2`(表 `tables/nowm-stats.tsv`): `RedrawWindow` の回数は lid 17 99、lid 14 88、lid 64 86(通常の走行と同じ)。`BeginPaint` lid 64 は 74 → 34 に、lid 16 の描きは消える。つまり lid 64 の `BeginPaint` のうち FL の輪由来は約 34/39 = 0.9 回/手、残りは我々の窓管理(A)由来だった。

### 2.2 メッセージを止める実験は成立しなかった

`WM_WINDOWPOSCHANGED` を窓に送らない診断(`A8_NOPOSCHANGED=1`、`runs/mx8`)では FL のドラッグ自体が始まらず(SetWindowPos の呼び出し 0)、輪が FL のメッセージ処理から来るのか FL の内部状態から来るのかは切り分けられなかった。輪は FL の内部タイマー(`TSharedAnimTimerWindow` 0x8ffc、`QFC` の tick: 決定 0247 §2.1 の鎖 `f_1e858e0 → f_1e86280 → f_80a7a0 → f_8090d0 → f_809030 → f_820f10`)から出る。我々が送るメッセージは Learn どおり(決定 0255 の全数点検 49 行、0258)で、そこに過不足は見つからなかった。

## 3. 契約側の照合: Wine 10.0 との比較

`harness/alpha8/move_paint.c`(Wine と runtime に同じ C。FL の実際の窓の矩形: MAIN > WP > { RACK, PLAY, MIX > { CAP, PANA > HID, PANB > SUB } }、MIX を PLAY の上に出した FL の z 順)。表: `tables/wine-vs-runtime-move-paint.tsv`、生の出力: `logs/wine-vs-runtime/`。

- **純粋な移動(隠れた窓の矩形なし)は HEAD の runtime も Wine と同じ**: WP の (16,466,31,966) 15x500 の帯だけが無効で、WM_PAINT は WP に 1 回。ビットのコピーと露出部分は契約どおり(Learn SetWindowPos: 「クライアント領域の有効な内容は保存され、移動後に戻される」)。
- 隠れた窓(HID)が矩形を持った状態では HEAD は WP の帯のほかに MIX (105,474,1359,493)、PANA (0,446,1167,465)、PANB (0,446,72,465) も無効にする(FL の実走行のトレースと同じ矩形)。Wine は WP の帯だけ。**これが A。**
- Wine は隠した窓の更新領域を消す(HID は無効でなくなる)。runtime は消さない。`unc_pending` が見ないようにしたので移動では無害になったが、`GetUpdateRect` には残る(直していない)。
- `SetWindowPos(SWP_SHOWWINDOW)` だけでは runtime は全面無効にしない(`ShowWindow` はする)。Wine は窓の全面と親の覆われた部分を無効にする。**隙間(直していない)**: 起動時の WM_PAINT が増える可能性があり、ドラッグには出ない。
- サイズ変更: Wine はクラスが `CS_HREDRAW | CS_VREDRAW` を持たなければビットを残す。runtime は全面を無効にする(決定 0214 の Learn BeginPaint の読み)。ドラッグには出ない(直していない)。

## 4. 見つけた A: 隠れた窓の古い更新矩形

`UNC begin pending lid=87 surf=529,1028,1783,1047 vis=0` — 診断トレースがドラッグの各手の先頭で出す。lid 87(`TQuickScrollBoxScroller`、親 lid 86、スタイル 0x4e000000 = 隠れて無効)は FL が作るとき `SetWindowPos(0x57)`(SHOWWINDOW)→ `SetWindowPos(0, 446, 1254x19, 0x114)` → `SetWindowPos(0x97)`(HIDEWINDOW)と呼ばれ、一度も描かれない。`apply_pos` は HIDEWINDOW で更新矩形を消さず、`unc_begin` は subject の子孫すべての `unc_pending` を `cp0`(コピー元)から引く。引かれた (0,0,1254,19) が lid 64 のコピー元の下端 19 行 × 1254 を欠き、`unc_end` がその 19 行を新しい位置で MIX・PANA・PANB(と ValidateRgn で答える SUB)に無効として足す。FL は毎手その 19 行の帯(1254x19 = 23,826 画素の `BitBlt`)を描き直していた。

直し方(`lift/runtime/fle_win.c` の `unc_pending` の先頭に `if (!is_visible(w)) return q;`、注釈つき): 見えない窓の更新矩形は『保留中の領域』として数えない。S-0247-hidden(`upd_add` が見えない窓に足さない)と同じ規則を、窓が見える間に持った矩形の扱いに揃える。HIDEWINDOW で矩形を消す(Wine の挙動)ことは、隠した後の `SWP_SHOWWINDOW`(全面無効にならない隙間)で窓が空白のまま残る危険があるので、今回は入れなかった。

テスト: `harness/alpha8/hidden_move.c`(24 項目、`crates/fe-lift/tests/alpha8_hidden_move.rs`)。HEAD: 13 通過 / 11 FAIL(移動 3 回 × {MIX が無効、CAP・PANA・PANB が無効、WP 以外が描かれる} と RDW の 2 項目)、alpha8: 24 通過。

## 5. 見つけた B: RedrawWindow の定数の取り違え

`fle_win.c` の `RDW_NOFRAME_ 0x0040`、`RDW_NOCHILDREN_ 0x0800`。SDK は `RDW_NOCHILDREN 0x0040`、`RDW_NOFRAME 0x0800`(`tables/rdw-flags.tsv`)。関数コメントにも『NOFRAME 0x40 … NOCHILDREN 0x4000…』と誤った数字があった。FL は `RedrawWindow` を 1 手に 4.8 回 0x41 で呼ぶ(= INVALIDATE | NOCHILDREN)。HEAD は 0x41 を INVALIDATE | NOFRAME と読み、WS_CLIPCHILDREN の無い lid 64 / 17 の子へ無効化を進めた(`upd_children`)。ドラッグ中 `RedrawWindow` 由来の update_rect 追加は lid 85(Mixer の caption)に 86 回、lid 96 / 98(rack)に 272 / 168 回、lid 50 に 2 回。直すとこれらが消え、追加は lid 14 / 17 / 64(FL が名指しする窓)だけになる(1 手 32.77 → 19.21)。子の WM_PAINT(`ValidateRgn` で答えるだけ)は 1 手 5.92 → 1.51。

`present2_uncover.rs` のテスト `redrawwindow_invalidate_follows_the_same_default_and_rdw_nochildren_leaves_the_children_alone` は NOCHILDREN を 0x800 と書いていた(`harness/api2/win_upd.c` の Wine の答えと照合したテストで、数字だけが誤り)。0x40 に直した。期待値(`p 1 0 0 200 200` / `c 0 0 0 0 0`)は変えていない。

## 6. 効果

### 6.1 1 手あたりの数(`tables/drag-stats.tsv`、trace 付き native、39 手)

| 値 | HEAD | A | A + B |
|---|---|---|---|
| runtime 由来の update_rect 追加(画素の合計/手) | 840,710 | 88,033 | 88,033 |
| 実書込み画素/手(BitBlt) | 399,959 | 80,763 | 80,735 |
| `BeginPaint` lid 64 | 1.90 | 1.38 | 1.38 |
| `TVectorPanel` の WM_PAINT | 4.05 | 2.72 | 0.97 |
| `TNewCaption` の WM_PAINT | 3.13 | 3.13 | 0.51 |
| `ValidateRgn` | 7.23 | 5.92 | 1.51 |
| `RedrawWindow` 由来の upd_add | 32.77 | 32.77 | 19.21 |
| FL の `RedrawWindow` の回数 | 6.97 | 6.97 | 6.95 |

FL の呼び出し(`RedrawWindow`、lid 14 / 17 / 64 の `BeginPaint`)は変わらない。

### 6.2 CPU(`tables/cpu-per-move.tsv`)

native、gdb の batch、`g3-cpu` の user + sys(1 ms 分解能、19 手): HEAD 9.37(3 回)・10.54(3 回)、A 8.83・10.26、A + B 9.68(同じラウンドの HEAD 10.54)。同じバイナリでもラウンドで ±1 ms 動く。**差は揺れと同程度で、確かな結論には足りない。** 向きは一貫して減る側。ブラウザ(wasm、`desk` / `engineRun` のビーコン)は未測定。

### 6.3 画素(`tables/pixel-diff-native.tsv`、`pics/e03-toolbar-A-C.png`)

- B を入れると起動直後(`e03`)の主フォームの 65 画素(x 1042〜1124、y 12〜24)が変わる。FL の CPU メーターの数字が 0 から 1 に変わる(時間に依存する値。C 3 回は同じ、A 2 回は同じ)。描画の取りこぼしではない。
- A を入れるとドラッグの後(`mx_end`)に、Mixer の右部分(x 1700〜1900、y 550〜650 と 1000〜1050)の約 2,000 画素が最大 10/255(中央値 1)変わる。HEAD が毎手描き直していた帯と枠。**どちらが実 Windows に近いかは決めていない**(Windows を持たない)。

## 7. 直していないもの

1. 隠した窓の更新矩形を消すこと(Wine の挙動)。`GetUpdateRect` にだけ見える。
2. `SetWindowPos(SWP_SHOWWINDOW)` だけで窓の全面を無効にすること。
3. サイズ変更でクラスの `CS_HREDRAW | CS_VREDRAW` を見ること。
4. 新規作成された見える窓の最初の WM_PAINT(Wine は描く、runtime は描かない)。
どれもドラッグの CPU には出ない。1 と 2 は対にして入れる必要がある(1 だけ入れると、隠れた窓を `SWP_SHOWWINDOW` で見せたとき、偶然残っていた矩形による描画も失う)。

## 8. 変更とテスト

- `alpha8.diff`(5 ファイル): `lift/runtime/fle_win.c`(`unc_pending`、RDW の 2 つの #define と関数コメント)、`lift/harness/alpha8/hidden_move.c`(新規、24 項目)、`lift/harness/alpha8/move_paint.c`(新規、Wine と runtime の両方で動くテストツール)、`lift/crates/fe-lift/tests/alpha8_hidden_move.rs`(新規)、`lift/crates/fe-lift/tests/present2_uncover.rs`(0x800 → 0x40)。`-` の行は 5 行(テストの古い注釈 1、テストの古い呼び出し行 1、関数の古い注釈 1、`#define` 2)で、いずれも上の理由。wall_fr* の期待値の変更はなかった。
- cargo(`git archive` した HEAD + alpha7 の複製 `~/work/a7-cargo` に alpha8 を載せたもの。PM の木ではない): `alpha8_hidden_move` 1 passed、`present2_uncover` 16 passed、`alpha3_updregion` 1 passed。runtime / 窓に触る 45 テストファイル(alpha7 の 44 + `alpha8_hidden_move`、`--no-fail-fast`)は **741 件通過・失敗 0・rc=0**(alpha7 のときの 44 ファイルは 740 件。+1 は新しいテスト)。`logs/cargo-pack-alpha8.txt`。

## 9. 再現

- Wine との比較: `tools/build-movepaint.sh TREE OUTDIR`(native.out / wine.out と、`top` 引数の native-top.out / wine-top.out)。HEAD 側は runtime を元に戻した木で。
- C テスト: `tools/build-hiddenmove-test.sh TREE OUTDIR`。
- ドラッグ: `tools/build-n1b.sh`(GENOBJ = b42fad36 のコンパイル済み生成 C、RMC = realmod_ref.c、NO_COPY=1、TAG)に診断トレースを足した木(`diag/a8-trace.diff`)を渡し、`tools/run-gdb.sh BIN scripts/mxmove.g3 scripts/pk70.gdb OUT`(`A8_TRACE=1 A5_MSGTAB=1 A5_WINTRACE=1`)。集計は `tools/a8-drag-stats.py OUT/gdb.err`。

## 10. 次の手

1. PM: alpha8.diff を alpha5/5c/5b・alpha7 に続けて 1 回の CI ビルドへ。ブラウザの Mixer ドラッグ(ci1 の 8.12 → 7.98 ms/手と同じ手順)で A / B の効果を測る。
2. 残りの CPU は FL 自身の描画(lid 64 / 17 / 14 の `BeginPaint` と中身)。ここを減らすには FL の描画コード(guest)の速さ(fe-wasmgen の直 wasm 化)が対象で、更新領域の側にはもう過剰が見つかっていない。
3. 7 の 1・2 を対で入れるかどうかは、起動時の WM_PAINT の増減を測ってから。

## PM の決定と取り込み(2026-11-05 23:02)

取り込み: `alpha8.diff`(5 ファイル: `lift/runtime/fle_win.c`、`lift/crates/fe-lift/tests/present2_uncover.rs`、新規 `alpha8_hidden_move.rs`、新規 `lift/harness/alpha8/{hidden_move.c,move_paint.c}`)を全体 e4954229(opt1-f + mt3 の 0254/0259 + opt1-f-lists +40 + alpha7)の上に `patch --batch -p1`(`git archive` の写しで dry-run も rc=0、offset・fuzz なし)。`-` 行 5 本をすべて読んだ: 旧テストコメント 1、旧テストの呼び出し `0x1u | 0x800u`(NOCHILDREN として 0x800 を使っていた)を 0x40 に、旧関数コメント 1、`#define RDW_NOFRAME_ 0x0040u` / `#define RDW_NOCHILDREN_ 0x0800u` の入れ替え(SDK の winuser.h は NOCHILDREN 0x0040、NOFRAME 0x0800)。追加は `unc_pending` が不可視の窓を数えない 1 行(`if (!is_visible(w)) return q;`)とコメント。FL のクラス名・窓種・アドレスの分岐、新オプションは無い。

PM が測った値(この機械、`lift/out-pmchk-a8/`):
- FLEngine の既定の生成 C の sha256 **`34a376dbf23a30c62df5ce6d04c07ed9beb5b651dd82d608cf5bcc59af78062a`**(215,740,414 B)不変。
- shellcheck と `ci/test-*.sh` / `test-real-deploy.py` rc=0。
- `test.sh full`: **207 results / 2,364 passed / 0 failed / 1 ignored**(直前の全体 206 / 2,363 に `alpha8_hidden_move` が加わった)。
- a4(FRUITY 無効): **init_count 942/942、run_rank 895/895、functions_distinct 5,310、function_entries 1,009,737、started_but_not_entered 0**(不変)。
- alpha1 自身の cargo(git archive の HEAD + alpha7 + alpha8 の写し、45 ファイル): 741 passed / 0 failed。

判断:
1. **採る(alpha8)。** 契約(winuser.h の RDW_* の値、Learn の不可視窓には更新領域が無いこと)に対する我々側の 2 つの誤りの修正で、native N1 で測った実測は、ランタイムが起こす無効化 840,710 → 88,033 画素/移動(−90%)、BitBlt の書き込み 399,959 → 80,763 画素/移動(−80%)、BeginPaint の領域 440,934 → 122,095 画素/移動(−72%)、TVectorPanel の WM_PAINT 4.05 → 0.97/移動。Wine 10.0 と同じ形(不可視窓の無い純粋な移動では HEAD も Wine と一致、不可視窓がある場合は alpha8 が Wine と一致)。
2. **効果の読み**: Mixer のヘッダードラッグで毎移動パネル全面(1514x500)が描き直される主因は **FL 自身の RedrawWindow**(QFC のタイマー連鎖 f_1e858e0 → … → f_820f10、1 移動あたり 6.97 回。決定 0247 §2.1)で、ランタイム側の過剰は 2 つとも直した。native の CPU は 1 移動あたり約 0.4〜0.9 ms(4〜8%)でノイズの水準。ブラウザ(wasm)の 8〜10.5 ms/移動への効果は未測定で、P2 を満たしたとは言わない。
3. 画素: 起動直後の主窓 約 65 画素(CPU メーターの桁、時間依存)と、ドラッグ後の Mixer 右部 約 2,000 画素(最大 10/255)が変わる。どちらが実機の Windows に近いかは未決。
4. node 84/55/87 行は opt1 の E6(E5 + alpha8)の FS テストビルドで、種類と原因で比べる(更新領域・WM_PAINT の数に関わる surface/wintable/progress/snap の変化は想定内、停止・g3-win・timers の変化は想定外)。結果が出る前に push する場合は、デプロイ後に ci1 がブラウザで確かめる。
5. 直さなかった Wine との差(表 `tables/wine-vs-runtime-move-paint.tsv`): (1) 不可視窓の更新矩形を消す、(2) SetWindowPos(SWP_SHOWWINDOW) 単独では全面を無効化しない(ShowWindow は無効化する)— (1) と (2) は対で直す必要がある、(3) リサイズの無効化が CS_HREDRAW|CS_VREDRAW を見ない、(4) 新しく作った可視窓の最初の WM_PAINT。いずれもドラッグの CPU に効かない。

### 追記(2026-11-05 23:25): opt1 の E6(= E5 + alpha8)の実エンジンの確認(push 後に届いた。CI 42043420797 は b00932ec で開始済み)

出典: opt1 の報告(23:20、23:22)と `docs/investigations/alpha8/tables/0265-e6-alpha8-node-rows-vs-e5.txt`(PM が表を読んだ)。E6 は HEAD e4954229 の冷間 CI ビルド(wasm 142,380,705 B、sha256 `ed049a99167f…` = E5 と byte 同一)を製品側の作業ディレクトリにして、`rt/fle_win.o` だけ alpha8 で作り直した FS 付きテストビルド(製品形ではない)。
- node の行 E6 − E5: 起動 84 は 29 行(g3-progress 16/16、g3-snap 5/16〔clock_ms −2〕、g3-surface lid 14 8/16〔sha256 のみ、nonzero 不変〕)、参照 55 と 87 は各 26 行(progress 10/10、snap 9/10〔clock_ms −1、87 の再生中 3 スナップは −25〕、lid 14 7/10〔sha256 のみ〕)。**不変**: g3-wintable、g3-win の 23 行、lid 48・lid 8・lid 139 の surface、timers、fruity。**停止・想定外の種類の変化は 0**。progress の差は最大 −0.23%(多くは 0.01% 台)。
- 参照プロジェクトは開き、node の参照 run は最後まで走る。Export(Chrome 156、FS 付きテスト bundle、参照プロジェクトを開いて Ctrl+R)は 22,863,828 B・sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` で、本番・E2〜E5 と一致。1 本。
- 未確認のまま: lid 14 の sha256 だけが変わる画素差(alpha1 の native では「CPU メーターの数字」約 65 画素)の画素単位の確認、Mixer ドラッグ場面(node の行には無い)、製品形の E6 bundle のブラウザ確認(デプロイ後に ci1)。

### 訂正(2026-11-05 23:40、alpha1 の決定 0266 第 1 報による)

本文書の「1 手あたり」の表(BeginPaint 6.36、RedrawWindow 6.97、lid 14 の BeginPaint 2.21、ValidateRgn 7.23 など)は、run の最後まで数えていて、ボタンを離した後の 700 ms の idle tick を含んでいた(alpha1 の誤り)。ドラッグ窓だけに切ると、HEAD の runtime で BeginPaint 4.51、RedrawWindow 5.08、lid 14 の BeginPaint 1.05、lid 17 の BeginPaint 1.05、ValidateRgn 5.82、alpha8 では BeginPaint 4.00 / lid 64 の BeginPaint 1.38。lid 64(1.90 → 1.38)、lid 16(0.49)、CPU/手(g3-cpu の m001〜m020 はドラッグ区間だけ)は不変。結論(パネル全面の再描画の本体は FL 自身の RedrawWindow、runtime 側の過剰は 2 件で alpha8 が直した、効果は小さい)は変わらない。表: `docs/investigations/alpha8/tables/0262-correction-drag-window-counts.tsv`。
