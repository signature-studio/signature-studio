# 0267(仮): メイン窓を最小化すると戻す手段がない — 契約に忠実な形で埋める(alpha10): DefWindowProc の WM_SYSCOMMAND(SC_MINIMIZE / SC_MAXIMIZE / SC_RESTORE)、最小化した窓の形、ホスト側の『タスクバー』入口。さらに、戻した後の FL のレイアウトが崩れる原因として GetWindowPlacement の長さの誤り(60 対 44)と SetWindowPlacement の欠落を見つけた(alpha10b)

**依頼者の機械・実機 Windows・実 GPU・ブラウザ・本番 bundle(`fl-engine-ci-ff5ee429` を含む)・本番 Worker(版 `a3fc8c43-a3a9-48a2-b9cf-86b1dbf5cfca`)での測定は 0 件。** 以下は、b42fad36 の生成 C から作った native N1 に、HEAD(runtime は b00932ec と同一、1c78f3c5)の runtime + 本変更を載せ、G3 の模擬入力(最小化ボタン (1863,20) のクリックと、ホストの『タスクバー』の押下。**ユーザーの操作ではない**)で測った値。native N1 の行は本番 bundle の行ではない。Wine 10.0 の結果と私の読みは UNOFFICIAL(Wine は Windows ではない)。代役・再生・推測は含まない。

状態: **仮の決定**。作業者 `alpha1`、2026-11-06 JST 00:00〜。基準 HEAD は fd518ceb〜85093eb2(runtime は b00932ec から同一。最後の確認は 85093eb2 に `git apply --check` を `alpha10.diff` → `alpha10b.diff` の順で通した)。差分は 2 本: `alpha10.diff`(A + B + C。単独で入る)と `alpha10b.diff`(D。`alpha10.diff` の上に入る。lifter の表に 1 行足す。**起動時の FL のレイアウトが変わる**)。

## 結論(先頭の数行)

1. **FL の最小化ボタンは `ShowWindow(メイン窓, SW_MINIMIZE)` を直接呼ぶ**(ra 0x8672ca。その直前に TLightweightLayeredControl を SW_HIDE)。`WM_SYSCOMMAND` は送らない。戻すのは実 Windows ではタスクバーのボタンで、シェルが窓へ `WM_SYSCOMMAND(SC_RESTORE)` を送り、`DefWindowProc` が `ShowWindow(SW_RESTORE)` にする(Learn WM_SYSCOMMAND: 「DefWindowProc は上の表の定義済みの動作の窓メニュー要求を実行する」「アプリは WM_SYSCOMMAND を DefWindowProc に渡すことでいつでも任意のシステムコマンドを実行できる」)。HEAD の runtime は `SC_CLOSE` 以外を無視していた(`fle_input.c` の DefWindowProc)。
2. **A. `DefWindowProc` の `WM_SYSCOMMAND` を実装した**: `SC_MINIMIZE` → `ShowWindow(SW_MINIMIZE)`、`SC_MAXIMIZE` → `SW_MAXIMIZE`、`SC_RESTORE` → `SW_RESTORE`(`SC_CLOSE` → `WM_CLOSE` は従来どおり)。コマンドは wParam & 0xFFF0。Wine 10.0 の同じ C テストで、メッセージの列(WM_SYSCOMMAND → WM_WINDOWPOSCHANGING → WM_NCCALCSIZE → WM_WINDOWPOSCHANGED → WM_MOVE → WM_SIZE)・復元先(最大化されていた窓は最大化に戻る)・`WS_MINIMIZEBOX` / `WS_MAXIMIZEBOX` を見ないこと・`SC_CLOSE` が一致する(`tables/syscmd-wine-vs-runtime.tsv`)。
3. **B. 最小化した窓の形を Wine に合わせた(alpha7 の穴)**: 位置 (-32000, -32000)、クライアント 0 x 0、`WM_SIZE(SIZE_MINIMIZED)` の lParam は 0、`WM_MOVE` は (-32000, -32000)。HEAD は矩形を保ち、WM_SIZE に元のクライアントの大きさ(1920x1080)を載せていた。窓矩形は 0 x 0 のまま(Wine は 160 x 31 = SM_CXMINIMIZED x SM_CYMINIMIZED。モデルに非クライアント領域が無いので、窓矩形とクライアントを別に持たない選択。違いは GetWindowRect と WINDOWPOS の cx、cy だけ)。
4. **C. ホストの『タスクバー』の入口を作った**(クラス名・窓種の分岐なし、新オプションなし): `fle_win_taskbar_buttons(out, max)`(ボタンのある窓の一覧: 最上位・可視・`WS_EX_APPWINDOW` または所有者なしで `WS_EX_TOOLWINDOW` なし。各ボタンに hwnd、iconic、zoomed、lid)と `fle_input_host_syscommand(hwnd, cmd)`(窓へ `WM_SYSCOMMAND(cmd)` を、窓のスレッドが次にメッセージを取り出すときに SendMessage で届ける。cmd は SC_MINIMIZE / SC_MAXIMIZE / SC_CLOSE / SC_RESTORE のどれか。戻り値 1 = キューに入れた、0 = 拒否)。wasm の入口は `fle_g3_taskbar_iconic()`(最小化された窓のボタンの lid、無ければ 0)と `fle_g3_taskbar_syscommand(lid, cmd)`。メイン窓(lid 14)は `WS_EX_APPWINDOW`(exstyle 0x50000)なのでボタンが 1 つ出る。native N1 で『最小化ボタンをクリック → タスクバーの restore』が完走し、窓は (0, 0, 1920 x 1080) の最大化に戻る。ページ側の仕様は §6。
5. **しかし戻した後の FL のレイアウトが崩れる(A + B + C だけでは)**: 最小化の ShowWindow の直後に FL(VCL の AlignControls)が全部の子窓を、ゴミの幅(`cx=156103730` など)で SetWindowPos し、戻した後は Playlist が消え Mixer が横いっぱいになる(`pics/restore-without-placement-fix-AFTER-restore.png`、最小化前の画 `e03` と 1,107,824 画素違う)。**原因: `GetWindowPlacement` が FL の全呼び出し(起動時 459 回以上)で失敗している**: runtime は `WINDOWPLACEMENT` の長さ(`length`)を 60 と要求している(Learn の表の `rcDevice` まで数えた)が、SDK のヘッダでは `rcDevice` は `_MAC` だけで構造体は 44 バイト、FL(Delphi の `TWindowPlacement`)は 44 を渡す。呼び出しは `FALSE`(ERROR_INVALID_PARAMETER)を返し、構造体は書かれず、FL は初期化されていないスタックの値を使う。
6. **D(alpha10b)**: `WPL_SIZE` を 44 にし(`rcDevice` を書かない)、`ptMinPosition`((-1,-1) → 最小化後 (-32000,-32000))と `flags`(最小化された最大化窓は `WPF_RESTORETOMAXIMIZED` 2)を Wine に合わせ、**`SetWindowPlacement` を実装した**(Learn: 「表示状態と復元・最小化・最大化の位置を設定する」。Wine 10.0 の順: 復元矩形を保ち、通常状態の窓は `SetWindowPos(NOZORDER | NOACTIVATE)` でその矩形に移し〔非表示でも〕、そのあと `ShowWindow(showCmd)`)。GetWindowPlacement を直すと FL が起動時に `SetWindowPlacement` を呼ぶ(2 回: メイン窓 `SW_HIDE`、矩形 (-5,-6,1925,1086) と lid 68 `SW_HIDE`、(0,0,198,490))ようになり、**この import には lifter の表に行が無い**(`crates/fe-lift/src/imports_table.rs`)ので、GetWindowPlacement だけを直すと FL は起動時に `unsupported instruction at 0x425500`(その import の thunk)で止まる。→ alpha10b は表に 1 行足し、runtime に `fle_imp_SetWindowPlacement` を入れる。**生成 C の再生成(CI の全面ビルド)が要る**。
7. **D の効果**: 最小化 → タスクバーで復元 → 元の画との差が 1,107,824 画素 → **41,304 画素**(動いている部品の差の大きさ)。最小化前の画と比べても配置が同じ(`pics/with-placement-fix-AFTER-restore.png`)。一方で **起動時の画が HEAD と 708,177 画素違う**(`e03`。FL が本物の `WINDOWPLACEMENT` を読み、`SetWindowPlacement` で配置するため。HEAD の起動画はゴミの値を使った配置)。どちらが実 Windows に近いかは決められない(Windows を持たない)が、D は FL が読む値を契約どおりにする。PM の未決。

## 冒頭の三点(正直さ)

1. **何を実測したか**: 実エンジン(`FLEngine_x64.dll` と同梱 DLL を x86-64 → C にした b42fad36 の生成 C)の native N1 を G3 の模擬入力で走らせ、メイン窓の最小化ボタン (1863,20) のクリックの鎖(API ログ・メッセージ・gdb)と、HEAD / A + B + C / A + B + C + D の最小化 → 復元の画(lid 14 の面の 1920 x 1080 の生ピクセル)を比べた。D の『SetWindowPlacement の行』は生成 C に無いので、native N1 では実験用のリンク設定(`--wrap=f_425500`、`tools/wrap-f425500.c`)でその thunk を `fle_imp_SetWindowPlacement` に繋いで測った(製品は表の行で同じことをする)。同じ C テスト(`harness/alpha10/syscmd_seq.c`、`setplace_seq.c`)を Wine 10.0(mingw、headless)と runtime に走らせた。**本番 bundle のバイト列での測定ではない。**
2. **結論**: 上のとおり。A + B + C は契約どおりで単独で入るが、戻した後の FL の見え方は D に頼る。
3. **未確認**: 実 Windows での最小化 / 復元の見え方、ブラウザ(wasm)での動作、ページの復元ボタン(PM が別担当に回す)、D が起動時のレイアウトを変えること(実 Windows との照合、node の 84 / 55 / 87 行)、`SetWindowPlacement` の lifter 表の行を足した生成 C での通し(native では `--wrap` で代用した)、全体テスト(`test.sh full`)と a4。

## 1. FL の最小化ボタンの鎖(`logs/fl/`)

クリック (1863,20) → TQuickEditToolbarPanel(lid 80)の WM_LBUTTONUP → FL が `ShowWindow(TLightweightLayeredControl, SW_HIDE)`(ra 0x868766)→ `ShowWindow(TFruityLoopsMainForm, SW_MINIMIZE = 6)`(ra 0x8672ca)。窓へは WM_WINDOWPOSCHANGING → WM_NCCALCSIZE → WM_WINDOWPOSCHANGED → WM_MOVE → WM_SIZE(SIZE_MINIMIZED)(`logs/fl/minimize-click-messages.txt.gz`)。WM_SYSCOMMAND は 0 件。直後に VCL が子窓を並べ直す(`logs/fl/minimize-click-api-calls.txt`: `SetWindowPos(TQuickControlBar, cx = 156103730)`、`TFruityLoopsMainForm cx = 156103742, cy = -16794059` ほか)。幅のゴミは、`GetWindowPlacement`(459 回以上、先頭 40 回は `logs/fl/getwindowplacement-first40.txt`: 全部 `length=44`)が失敗して構造体が書かれないことから来る(§4)。

## 2. A. DefWindowProc の WM_SYSCOMMAND

`fle_input.c` の `fle_input_defproc`: `SC_CLOSE` → `WM_CLOSE`(従来)、`SC_MINIMIZE` / `SC_MAXIMIZE` / `SC_RESTORE` → `fle_win_input_showwindow`(`fle_win.c`。`ShowWindow` を呼ぶ)。Wine 10.0 は `WS_MINIMIZEBOX` / `WS_MAXIMIZEBOX` を見ないので同じ。他のコマンド(`SC_SIZE`、`SC_MOVE`、メニュー系)は『枠の無いモデル』のため従来どおり何もしない。

## 3. B. 最小化した窓の形

`fle_imp_ShowWindow` の最小化(`fin == 1`)で最上位窓の矩形を (-32000, -32000, 0 x 0) にする。復元矩形(`rest_*`)は `apply_pos` が、窓が最小化・最大化の間は書き換えない既存の仕組みで守られる。復元は `fin == 0`(`rest_*`)または `fin == 2`(最小化前に最大化されていた窓: `restore_max`、alpha7)。

## 4. D. GetWindowPlacement と SetWindowPlacement(alpha10b)

(`logs/setplace/`、`tables/getwindowplacement-setwindowplacement.tsv`)
- `WPL_SIZE` 60 → 44。Learn の WINDOWPLACEMENT の表は `rcDevice` を載せるが、SDK のヘッダ(mingw-w64 の winuser.h を確認、UNOFFICIAL)では `_MAC` だけ。実 Windows の `GetWindowPlacement` は `length != sizeof(WINDOWPLACEMENT)` で失敗し、x64 の sizeof は 44。
- `SetWindowPlacement`: 上の結論 6。Wine 10.0 で 12 の場面を比べた(非表示の窓への `SW_HIDE` / `SW_SHOWNORMAL` / `SW_SHOWMAXIMIZED` / `SW_SHOWMINIMIZED` / `SW_SHOWNOACTIVATE`、表示中の窓への各種、`WPF_RESTORETOMAXIMIZED`、誤った長さ)。メッセージの列と状態は runtime と一致する(WINDOWPOS の flags、NCCALCSIZE の wParam、活性化の細部は従来どおり違う)。Wine 自身の揺れ(最大化の直後に別の WM_WINDOWPOSCHANGED が来て最大化が外れる)が 3 回に 1 回ほど出る場面があり、その場面は一致の根拠にしていない。
- import の行: `("user32.dll", "SetWindowPlacement", "fle_imp_SetWindowPlacement", "jmp_imp:SetWindowPlacement")`(`imports_table.rs`)。

## 5. 画(`tables/min-restore-pictures.tsv`、`pics/`)

| 比較(lid 14、1920 x 1080、BGR 画素) | 違う画素 | 最大のチャネル差 |
|---|---|---|
| A + B + C: 最小化前 `e03` と復元後 `rst2` | 1,107,824 | 247 |
| A + B + C + D: 最小化前 `e03` と復元後 `rst2` | 41,304 | 247 |
| 起動直後 `e03`: A + B + C と A + B + C + D | 708,177 | 247 |

41,304 は FL の動く部品(CPU メーターの数字、再生位置など)の大きさの差と読む(中身は未分解)。

起動時の配置の差(`tables/startup-layout-windows.tsv`、113 窓のうち 27 窓の矩形が違う。可視性は同じ): HEAD(+ A + B + C)では内容がメイン窓の 6 px の余白の内側から始まる(例: lid 50 TQuickControlBar (6, 6, 1908 x 82)、lid 49 (6, 88, 362 x 986)、lid 64 TFXForm (16, 454, ...))。D では (0, 0) から最大化メイン窓の 1920 x 1080 いっぱいに使う(lid 50 (0, 0, 1920 x 82)、lid 49 (0, 82, 364 x 998)、lid 64 (16, 466, ...)。Mixer の y は決定 0240 のドラッグの場面と同じ 466)。D は、runtime が窓表の値を契約どおりに GetWindowPlacement で答え、FL が渡す矩形(lid 14 は (-5, -6, 1925, 1086)、lid 68 は (0, 0, 198, 490))を契約どおりに SetWindowPlacement で適用した結果で、runtime は配置の値を決めていない(クラス名・窓種の分岐なし)。FL の配置が変わる因果(FL がどの値をどう使うか)は gdb で追っていない。見た目が変わるので PM の未決。

## 6. ページ側へ渡す仕様(ホスト API)

C(`lift/runtime/fle_input.h`):
- `typedef struct { uint64_t hwnd; uint32_t iconic; uint32_t zoomed; uint32_t lid; uint32_t pad; } fle_taskbar_button;`
- `unsigned fle_win_taskbar_buttons(fle_taskbar_button *out, unsigned max)` — タスクバーにボタンのある窓(最上位・可視・`WS_EX_APPWINDOW`、または所有者なしで `WS_EX_TOOLWINDOW` なし)を作成順に最大 `max` 件書き、件数を返す。最小化された窓もボタンを持つ。
- `int fle_input_host_syscommand(uint64_t hwnd, uint32_t cmd)` — 1 = キューに入れた、0 = 拒否(生きた最上位窓でない、cmd が 0xF020 / 0xF030 / 0xF060 / 0xF120 のどれでもない、キューが一杯)。配信は窓のスレッドの次のメッセージ取得で、窓手続きに `SendMessage` として `WM_SYSCOMMAND(wParam = cmd, lParam = 0)` が届く。

wasm の入口(`lift/harness/fruity_idle.c`、`fle_g3_display` と同じ型。**`EMSCRIPTEN_KEEPALIVE` の関数を 2 つ足しただけで、emscripten ではコンパイルしていない**: `-D__EMSCRIPTEN__` の構文確認だけ):
- `unsigned fle_g3_taskbar_iconic(void)` — 最小化された窓のボタンの lid(窓表・G3 の記録・ブラウザのビーコンと同じ番号)。無ければ 0。ページは『メイン窓が最小化されている間だけ』復元ボタンを出す: エンジンが 1 ステップ進むたび(または `desk` のシーン更新のたび)に呼び、非 0 の間ボタンを出す。メイン窓(lid 14)は、最小化されていると描画シーンから消える(黒いデスクトップ)ので、ボタンはその上に出す。
- `int fle_g3_taskbar_syscommand(unsigned lid, unsigned cmd)` — ボタンが押されたら `(lid, 0xF120)` で呼ぶ。戻り 1 = 受け付けた。復元は次のメッセージ取得で行われ、数 ms 後に `desk` のシーンにメイン窓が戻る(native N1 の G3 では 1 秒の間に完了、`g3-surface` が 570x412 → 1920x1080 に戻る)。
- G3 スクリプトに `taskbar restore|minimize|maximize|close`(`fruity_g3.c`)。

export の許可リスト(wasm のリンク行)にこの 2 つを足す必要があるかは、ビルドスクリプトを見ていない(`fle_g3_display` が足された決定 0212 の差分に倣うこと)。

## 7. 直していないもの

1. 最小化時の `WM_KILLFOCUS`、復元後の `WM_SETFOCUS` / `WM_ACTIVATE` の順(Wine: SETFOCUS が先)、WINDOWPOS の `flags`(SWP_STATECHANGED 0x8000)と `WM_NCCALCSIZE` の wParam TRUE、窓矩形の 160 x 31(結論 3)。FL が読む値ではない(決定 0255 の監査 S11)。
2. 他の窓の自動活性化(最小化した窓の代わりに次の窓を活性にする)。この環境の最上位窓は FL のメイン窓が中心。
3. 複数のモニター・アイコンの並び・`WPF_SETMINPOSITION` による最小化位置(`ptMinPosition` は保持しない)。
4. ページ側の復元ボタン(PM が別担当に回す)。

## 8. 変更とテスト

- `alpha10.diff`: `lift/runtime/fle_win.c`(最小化の矩形、`fle_win_input_showwindow`、`fle_win_taskbar_buttons`)、`fle_input.c` / `fle_input.h`(DefWindowProc の SC_*、ホストの syscommand、型と宣言)、`lift/harness/fruity_g3.c`(G3 の `taskbar` コマンド)、`lift/harness/fruity_idle.c`(wasm の 2 入口)、`lift/harness/alpha10/syscmd_seq.c`(新規。Wine と runtime の両方で動く)、`lift/crates/fe-lift/tests/alpha10_syscmd.rs`(新規)、既存の期待の変更(`wall_fr17`: 最小化された祖先の画素の期待 `000000` → `ff0000`〔本体で最小化の前に BLUE を塗るようにした。0 x 0 の窓は面を作らないので、最小化の前の面の色が読めるようにテストを変えた〕、`sweep1` のリンク用スタブ 2 か所〔`sweep1.rs` と `sweep1_cases.c`〕に `fle_win_input_showwindow`)。
- `alpha10b.diff`: `fle_win.c` / `fle_win.h`(WPL_SIZE 44、GetWindowPlacement の値、SetWindowPlacement)、`crates/fe-lift/src/imports_table.rs`(1 行)、`harness/alpha10/syscmd_seq.c`(`WPL_LEN` 44)、`harness/alpha10/setplace_seq.c`(新規)、`crates/fe-lift/tests/alpha10b_setplace.rs`(新規)、既存の期待の変更(`wall_fr9`: GetWindowPlacement の length は 44 が有効で 60 が誤り。表の末尾の行の位置を固定していたテスト 7 本〔`wall_fr4` / `5` / `13` / `15` / `16` / `19` / `20`〕は、決定 0147 のときと同じ方法で `tests/rows_0267/mod.rs`〔新規。1 行〕の分を数えない〔決定 0015: 追記専用〕。`wall_fr6` と `window` の `fle_set_last_error(` の数 37 → 38〔SetWindowPlacement の長さ違い 1 か所、引数は既存の集合のまま〕、`vcall_thunk_dll` の未対応 thunk 225 → 224〔FLEngine の thunk 0x425500 が答えられる。測定値〕)。
- cargo(`~/work/a7-cargo` = HEAD の runtime の複製。PM の木ではない。ログは `logs/cargo/`):

| 走らせた束(最終の木) | ファイル数 | passed | failed |
|---|---|---|---|
| A + B + C の通し(最初。`alpha10_syscmd` を含む) | 47 | 740 | 3(`sweep1` x 2 のスタブ、`wall_fr17` x 1 → 直して再確認) |
| A + B + C + D の通し | 48 | 737 | 7(表の末尾の位置を固定したテスト 5 本、`fle_set_last_error(` の数 2 本 → 上のとおり直した) |
| 直したテストを含む再実行(`wall_fr4` `5` `6` `13` `15` `16` `19` `20` `window` `vcall_thunk_dll` `imports_table` `alpha10b_setplace`) | 12 | 183 | 0 |
| 表を読むテストで最初の 48 に無かったもの(`wall_r2` `wall_s` `wall_fr3` `wall_c1` `wall_dec1` `wall_dlg1` `wall_e1` `wall_exp1` `wall_fpick1` `wall_opfs1` `wall_q_lift` `call_iat` `deferred` `event` `gdc` `gdi` `glob` `k32_dsp` `loadimage` `locale` `modules` `null0` `oleaut_ole` `osver` `proc` `spi` `srw` `sys` `sysinfo` `sysmetrics` `time` `uilang` `ver` `vfs` `vmap` `vol`) | 36 | 699 | 0 |

  最終の木に対して『全ファイルを 1 回で』は走らせていない(束を 3 回に分けた。D の後に変えたのはテストの期待と `rows_0267` だけで、runtime と lifter は 48 ファイルの通しと同じ)。走らせたのは 157 ファイル中の 88。残りの 69 ファイルは grep で表・窓・入力を読まないと見たもので、**走らせていない**。全体テスト(`test.sh full`)、a4、node(wasm)の 84 / 55 / 87 行も未実行。

## 9. 再現

- Wine との比較: `tools/build-syscmd.sh TREE OUTDIR`、`tools/build-setplace.sh TREE OUTDIR`(native.out / wine.out)。
- FL の鎖: `tools/build-n1b.sh`(GENOBJ = b42fad36 のコンパイル済み生成 C、NO_COPY=1、TAG)で N1 を作り(D は `tools/build-n1c.sh`: `--wrap=f_425500`)、`tools/run-gdb.sh BIN scripts/a10-restore.g3 scripts/pk70.gdb OUT`(`A5_WINTRACE=1 A5_MSGTAB=1`)、画は `tools/run-n1.sh BIN scripts/a10-restore.g3 OUT DUMPDIR`。

## 10. 次の手

1. PM: `alpha10.diff` を入れる(単独で壊れない)。`alpha10b.diff` を入れるかは、起動時のレイアウトが変わる点(結論 7)の判断と、CI の全面ビルド(lifter の表の変更)で。ブラウザの確認(ci1)は、最小化 → 復元の前後と起動直後の画。
2. ページ側: §6 の 2 入口で復元ボタン。
3. 実 Windows が使えるなら、起動時の配置と最小化 → 復元の見え方の照合。

## PM の決定と取り込み(2026-11-06 09:45、docs のみ。差分は製品の木に当てていない)

取り込み: alpha1 の最終(0267 + 0272 の訂正)の文書・表・道具・差分を `docs/investigations/alpha10/`(差分は `patches/`)に置いた。`alpha10.diff`(A + B + C: DefWindowProc の SC_MINIMIZE / MAXIMIZE / RESTORE → ShowWindow、最小化した窓の形、ホストの「タスクバー」入口)と `alpha10b-v2.diff`(D: GetWindowPlacement の長さ 60 → 44 と SetWindowPlacement、lifter の表に 1 行)は、e7b-wip(= main 0b228624)の `git archive` に順に dry-run が offset・fuzz なしで通る。D の `vcall_thunk_dll` の期待は 224 でなく **223**(`alpha10b-v2.diff`、alpha1 の訂正。FL の SetWindowPlacement の thunk 2 本が同じ IAT スロットを共有する)。測定は alpha1 の native N1 と Wine 10.0(UNOFFICIAL)で、ブラウザ・実 Windows・依頼者の機械は 0 件。PM は全体テスト・a4・node 行を未実行。
判断:
1. **alpha10(A + B + C)と alpha10b-v2(D)を採り、次の冷間ビルド(E8)に載せる**(いまの CI 42137885173 = E7b には載せない: 停止の切り分けが効かなくなるため)。D の採用条件だった「garbage 幅の SetWindowPos の原因の説明」は決定 0272 で満たされた(FL の VCL の UpdateBounds〔0x617f10〕が最小化時に GetWindowPlacement の rcNormalPosition を戻り値を見ずに読む。HEAD でも同じ garbage が出る。D は構造体が書かれるので消える)。D は lifter の表に行を足すので **既定の FLEngine の生成 C の sha が変わる**(PM が新しい sha を記録して取り込む)。
2. 本文の「GetWindowPlacement が失敗すると FL が未初期化のスタックの値を使う」は一般には誤り(FL の呼び出し 16 か所のうち 9 か所は戻り値を見る)。決定 0272 で訂正済み。
3. 未確認: 実 Windows での最小化 / 復元の見え方、ブラウザ(wasm)での通し、test.sh full・a4・node 84/55/87 行、D の SetWindowPlacement の行の入った生成 C の CI ビルド。E8 の完了条件にする。
