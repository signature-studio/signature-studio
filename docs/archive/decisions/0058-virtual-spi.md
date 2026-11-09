# 0058: 仮想 SystemParametersInfoW（SPI_GETWHEELSCROLLLINES だけに答え、ほかは名前付きで止める）

状態: 作業者（サブエージェント）が決定。方針（ホストの実設定・実マウス・実テーマ・実フォント・実レジストリをゲストに見せず、GET 系は仮想の固定値で答え、SET 系や fWinIni による書き込みを伴う呼び出しは、ゲストが使うなら名前付きで停止して理由を書く）は依頼者（PM）。**答える uiAction は 1 つだけで、その値（3）は一次資料が書く値**。実装は `lift/runtime/fle_spi.h`、`fle_spi.c`、`lift/crates/fe-lift/src/imports_table.rs`（1 行追記）、テストは `lift/crates/fe-lift/tests/spi.rs`、調査と実行記録は `docs/investigations/spi-caller.md`。「一次」は Microsoft Learn（2026-10-21 取得）、「非公式」はそれ以外。

## 背景

harness は `fle: unsupported instruction at 0x4255e0`（`user32.dll!SystemParametersInfoW` の thunk）で止まっていた。呼び出し元は f_620880 の `SystemParametersInfoW(0x68, 0, &[obj+0x14], 0)`（`GetSystemMetrics(75)` が真のとき。決定 0057）。DLL 全体の呼び出し点は 22 か所（thunk は 3 つ: 0x4255d0 が 18 か所、0x4255e0 が 2 か所、0x4255f0 が 2 か所。3 つとも IAT スロット 0x19710d0 への `jmp`）、静的な uiAction は 11 種（`spi-caller.md` §2）。`SystemParametersInfoA` は import に無い。

## 原則

1. ホストの実設定・実レジストリ・実マウス・実テーマ・実フォントは見せない。値は仮想で、この決定で固定する。
2. 一次資料に書かれていない値は、事実としては書かない。実装に入れる必要があるものは「仮の決定、出典なし、作業者の選択」と明記する。必要がないものは実装せず、名前付きの停止にする。
3. ゲストが使わない uiAction、値を決めていない uiAction は、`fle_stop` で止まる。値は作らない。
4. 何も書かない。SET 系の uiAction と fWinIni（ユーザープロファイルへの書き込み、`WM_SETTINGCHANGE` の送信）は、設定を保持する決定が無いので止まる。

## 一次資料で確認した規則

| ページ | 確認した内容 |
|---|---|
| [SystemParametersInfoW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow)（`updated_at` 2025-07-01、`ms.date` 2024-03-27） | 書式 `BOOL SystemParametersInfoW(UINT uiAction, UINT uiParam, PVOID pvParam, UINT fWinIni)`。uiAction の表が各 SPI_* の名前と 16 進の数値と pvParam の型を書く。**SPI_GETWHEELSCROLLLINES = 0x0068**: 「Retrieves the number of lines to scroll when the vertical mouse wheel is moved. The *pvParam* parameter must point to a **UINT** variable that receives the number of lines. **The default value is 3.**」。uiParam: 「If not otherwise indicated, you must specify zero for this parameter.」pvParam: 「If not otherwise indicated, you must specify **NULL** for this parameter.」fWinIni: 「If a system parameter is being set, specifies whether the user profile is to be updated…」（SPIF_UPDATEINIFILE、SPIF_SENDCHANGE、SPIF_SENDWININICHANGE）。戻り値: 「If the function succeeds, the return value is a nonzero value. If the function fails, the return value is zero. To get extended error information, call GetLastError.」Remarks: 「This API is not DPI aware」。**ページは、どの引数の形が失敗になるか（pvParam が NULL、uiParam が 0 でない、など）を書かない。** |

## uiAction ごとの決定（22 か所の呼び出し点の 11 種）

「出典」: **一次**＝ページが値を書く。**選択**＝ページが値を決めない。

### 答える（1 種）

| uiAction | 名前（Learn の数値） | pvParam | 値 | 出典 | 仮の決定か | 選択肢 | 撤回条件 |
|---|---|---|---|---|---|---|---|
| 0x68 | SPI_GETWHEELSCROLLLINES（0x0068。**一次**） | `UINT` の 4 バイトの出力先（**一次**。ゲストは `[obj+0x14]` とスタック上の 4 バイトを渡す） | 3 | **一次**（「The default value is 3」）。数値 0x68 も一次。ホストのマウス設定は読まない | **仮の決定ではない**（ページの既定値）。ただし**この値がホストの実設定でも、実 Windows 10 22H2 の実マシンの値でもあるとは言わない**（ページは既定値を書くだけ） | 0（スクロールしない）、`WHEEL_PAGESCROLL`（`UINT_MAX`、ページ単位）。どちらもページが挙動を書くが、既定値ではない | ゲストが 3 以外を前提にして破綻した。または決定 0057 の値が変わって、ゲストの 0x68 の前提（ホイールあり）と食い違ったとき |

- 戻り値は 1（TRUE）。last error には触れない（成功のときは書かない。ページは成功のときの last error を書かないので、触れないのが保守的）。
- uiParam と fWinIni は 0 でなければ止まる（ページが「0 を指定せよ」と書く形以外は定義されていない。「失敗で FALSE」とは**書かれていない**ので、規則に従えず名前付き停止にした。下の「規則が書かれていない形」）。
- pvParam が NULL なら止まる（同上）。
- 引数は `UINT`（32 ビット）なので、`rcx`、`rdx`、`r9` の下位 32 ビットだけを見る。pvParam は 64 ビットのポインタなので `r8` 全部。
- 書くのは 4 バイトだけ（`fle_st(R[8], 4, 3)`）。ゲストの出力先がその 4 バイトより狭いことはない（`[obj+0x14]` は DWORD。次の 4 バイトの読み手は未追跡だが、書くのは 4 バイトだけ）。
- 書き込み先の検証: wasm の `FLE_CHECK_HI32`（アドレスの上位 32 ビット）と `FLE_CHECKED_MEM` の領域検査は `fle_st` が行うので、範囲外のポインタは `fle_addr_fault` で止まる（native では SIGSEGV）。

### 値を決めない（GET 系 7 種。名前付きで停止する）

ページは、これらの値の既定値を書かない（または既定値はあるが仮想環境に対する根拠が無い）。`fle: SystemParametersInfoW: uiAction 0xNN (SPI_xxx) is used by the guest but no value is decided (decision 0058)` で止まる。

| uiAction | 名前 | pvParam の型（ページ） | 使われ方（`spi-caller.md` §2.1） | 値を決めない理由 |
|---|---|---|---|---|
| 0x1F | SPI_GETICONTITLELOGFONT | `LOGFONT`（uiParam はそのサイズ） | f_866424: uiParam = `[rbp+0x288]`（0x5C）、出力先 `[rbp+0x22c]`。後で `f_423fc0`（フォント名の取り出し）へ渡す | **フォント**。ホストのフォントは見せない。LOGFONT のフェイス名・高さ・太さの仮の値には根拠が無い（ページは値を書かない） |
| 0x26 | SPI_GETDRAGFULLWINDOWS | `BOOL` | f_9f4c60: `[obj+0x3c]` に受け、0 なら `f_425140(0)` を呼ぶ | ページは既定値を書かない。0 と 1 で分岐が変わる（0x9f4c98 `cmp [rbx+0x3c],0; jne`）ので、選択の根拠が要る |
| 0x29 | SPI_GETNONCLIENTMETRICS | `NONCLIENTMETRICS`（uiParam はそのサイズ。ゲストは 0x1f8 か 0x1f4 を渡す） | f_840020、f_8663b0、f_991cc0: 構造体の各フォントとスクロールバー幅などを使う | **フォントと寸法の塊**。ページは値を書かない。決定 0057 が決めていない SM_CXVSCROLL などとも整合させる必要がある |
| 0x30 | SPI_GETWORKAREA | `RECT` | f_8190a6、f_866cd0、f_867390、f_11c2d80: ワークエリア（タスクバーを除く領域） | ページは値を書かない。画面は 1920x1080（0053）だが、**タスクバーの有無・位置が絡む**（ページ: 「the portion of the screen not obscured by the system taskbar」）。仮想タスクバーの決定が無い。「全画面と同じ」は選択になる |
| 0x48 | SPI_GETANIMATION | `ANIMATIONINFO`（uiParam と cbSize は 8） | f_818c70: `iMinAnimate` を `!= 0` にして返す | ページは既定値を書かない。0 と 1 でウィンドウ最小化のアニメーションの有無が変わる |
| 0x1016 | SPI_GETTOOLTIPANIMATION | `BOOL` | f_61b600、f_a11c00: 0 なら即座に戻り、非 0 なら 0x1018 を続けて問う | 同上 |
| 0x1018 | SPI_GETTOOLTIPFADE | `BOOL` | 同上: 1 ならフェード（戻り値 1）、0 ならスライド（2） | 同上 |
| 0x101A | SPI_GETCURSORSHADOW | `BOOL` | f_815050: `[rbp+0x1c8]` に受ける | 同上 |

### SET 系（2 種。名前付きで停止する）

`fle: SystemParametersInfoW: uiAction 0xNN (SPI_SETxxx) is used by the guest and sets a system parameter: this environment keeps and writes no setting (decision 0058)` で止まる。

| uiAction | 名前 | ゲストの形（`spi-caller.md` §2.1） | 止める理由 |
|---|---|---|---|
| 0x49 | SPI_SETANIMATION | f_818cb0、f_867260: uiParam = 8（`sizeof(ANIMATIONINFO)`）、pvParam = `ANIMATIONINFO{cbSize = 8, iMinAnimate}`、fWinIni = 0 | 設定を変える。この環境は設定を保持しない（保持するなら「どの値をどう持つか」の決定が要る）。**fWinIni = 0 なので、プロファイルへの書き込みと `WM_SETTINGCHANGE` の送信はない**（読み: 全 22 か所で `xor r9,r9`）。ゲストは GET してから SET し、元に戻すアニメーション抑止の形と読める（推測。f_818c70 と f_818cb0 が対） |
| 0x4B | SPI_SETFONTSMOOTHING | f_7fe2d0: `uiParam = 0`（FALSE、`xor edx,edx`）、pvParam = NULL、fWinIni = 0。呼ぶ条件（読み）: 参照カウンタ `[0x15f6e5c]` を 1 足して 0 になったとき（`add [..],1; cmp [..],0; jne skip`）、かつ `[0x1402818] == 0`（`cmp [..],0; jne skip`）のとき。`[0x1402818]` のファイル上の初期値は 0xFFFFFFFF なので、初期値のままなら呼ばない（書き手は追っていない） | フォントスムージングを**オフにする**設定。ページ: 「To disable the feature, set *uiParam* to **FALSE**」。実 Windows ならシステム全体の設定を変える。この環境は設定を持たない |

SET 系を**実装しない**理由: ゲストは SET 系を 2 種（3 か所）しか使わず、その結果（設定が変わったか）を戻り値以外で読む形が見えない。保持するなら、保持する値、スレッド規則、GET と SET の対応の決定（例: 0x49 で書いた `iMinAnimate` を 0x48 が返す）が要る。これは GET の値を決めることと一体なので、GET 系の値を決める決定と同時にする（撤回条件）。

### 規則が書かれていない形（名前付きで停止する）

ページは次を**書かない**: pvParam が NULL のとき、uiParam が 0 でないとき（0x68 に対して）、fWinIni が 0 でないとき（GET に対して）の結果。「失敗で FALSE と GetLastError」という一般規則はあるが、**どの形が失敗かは書かれていない**ので、FALSE を返すのは創作になる。名前付きで停止する（`SPI_GETWHEELSCROLLLINES with pvParam NULL: the page does not define it` など）。ゲストの 0x68 の呼び出しは 3 か所（0x6208ac、0x6209f5、0xa0a57b）で、いずれも `uiParam = 0`（`xor edx,edx`）、pvParam = スタックまたはオブジェクト内を指す `lea r8,[..]`、`fWinIni = 0`（`xor r9,r9`）で呼ぶ（読み。テスト `the_guest_call_sites_pass_the_uiactions_the_decision_lists` が 22 か所すべての `xor r9,r9` と、0x68 の 2 か所のバイト列を固定する）。

### ゲストが使わない uiAction

`uiAction 0xNN is not answered (the guest's call sites do not use it)` で止まる。SPI_SETWHEELSCROLLLINES（0x69）、SPI_GETWHEELSCROLLCHARS（0x6C。ページは「The default value is 3」と書くが、ゲストは使わない）、SPI_GETMOUSE（0x03）、SPI_GETKEYBOARDSPEED、SPI_GETDESKWALLPAPER ほか、残りすべて。**値がページに書かれていても、ゲストが使わないものは実装しない**（使われないコードは検証できない）。

## 決定 0053 / 0057 との整合

- 0057 の SM_MOUSEWHEELPRESENT = 1 とこの決定の SPI_GETWHEELSCROLLLINES = 3 は整合する（ホイールがあり、1 目盛り 3 行）。0057 の「撤回条件」にあった「スクロール行数の既定値（ページは 3 と書く）と整合しない結果が出たとき」には**当たらない**。
- 0057 が決めない SM_CXVSCROLL、SM_CYHSCROLL など（スクロールバーの寸法）と、0x29（NONCLIENTMETRICS の `iScrollWidth`、`iScrollHeight`）は同じ値を指す。**どちらかを決めるときは他方も決める**（片方だけ決めると食い違う）。このため 0x29 は止めたままにした。
- 画面の 1920x1080（0053）と 0x30（SPI_GETWORKAREA）。ワークエリアは画面からタスクバーを引いたもの。**タスクバーの決定が無いので 0x30 は止めた**。決めるときは「ワークエリア = (0, 0, 1920, 1080) か、タスクバー分を引くか」を 0053 と一緒に決める。
- 96 DPI（0053）。0x1F、0x29 のフォントの高さは DPI に依存する（ページ: 「This API is not DPI aware」）。

**追記（決定 0066）**: 上の表の 0x1F（SPI_GETICONTITLELOGFONT）と 0x29（SPI_GETNONCLIENTMETRICS）は、決定 0066 で仮の値を決めて答えるようになった（停止ではない）。0x29 の uiParam は、ゲストが 3 か所とも 0 を渡すので、0 と cbSize を受ける。この節の表は 0058 の時点の記録として残す。

## 選択肢（全体）

- A. 0x68 だけに答え、ほかのゲストが使う 10 種と未使用の uiAction は止まる（採用）。
- B. ゲストが使う 11 種全部に値を決める（Windows 10 の既定値の記憶を使う）。**不採用**: 0x68 以外はページが既定値を書かない。「Windows 10 の既定値は X」と言える根拠がない。フォントやタスクバーなど、他の決定と整合させる必要がある値が多い。
- C. 未使用・未決の uiAction に黙って FALSE を返す。**不採用**: 失敗の戻り値を創作して続行することになる。ページは失敗する形を書いていない。
- D. ホストの値を読む。**不採用**: PM の方針（ホストの環境を見せない）に反する。
- E. SET 系を受け付けて何もしない（TRUE を返す）。**不採用**: 後続の GET が SET の値を返すとは限らない。「成功したが何も変わらない」は創作。
- F. SET 系の値を保持する。**不採用（今は）**: 保持の仕様（どの値をどう持つか）を決める根拠が、GET 系の値と一体で必要。ゲストがまだそこへ届いていない。

## 撤回条件

- 実行が、止まった 10 種のどれかに届いた（そのたびに、その uiAction の用途を読み、ページが値を書くなら答える。書かないなら、選択として決定に追記する）。**次に届く可能性が高いのは、GET 系の 0x1016／0x1018（ツールチップ）、0x1F（アイコンタイトルのフォント）、0x29（NONCLIENTMETRICS）、0x30（ワークエリア）**。
- ゲストが 0x68 の値 3 で破綻した（行数に依存する計算で `f_a0a4d0` が `0xffffffff` との比較を持つ。`WHEEL_PAGESCROLL` の分岐。値を変えたいなら理由を書く）。
- 0053 が解像度、DPI、モニタ数を変えた。0x30、0x1F、0x29 が止まっていれば影響はない。
- 0057 が SM_MOUSEWHEELPRESENT を 0 にした。f_620880 の経路（0x6208ac）は 0x68 を呼ばなくなる。ただし f_6209a0 の `edx == 0x68` の経路（0x6209f5。`[obj+0x28]` が真のときだけ）と f_a0a4d0（0xa0a57b）は SM_MOUSEWHEELPRESENT と無関係に 0x68 を呼び得るので、この決定の 0x68 は残る（読み。f_a0a4d0 の呼び出し元は追っていない）。
- Microsoft の別の文書が、止まっている uiAction の既定値を Windows 10 の値として示した。

## 確認できていないこと

- 実 Windows が SystemParametersInfoW で返す値（比較していない）。ホストの Windows 10 22H2 の実機でも 0x68 は 3 とは限らない（ユーザー設定）。
- 0x68 の出力先 `[obj+0x14]` の読み手（f_6209a0 の後で使われると読めるが追っていない）。
- 動的な uiAction の呼び出し（間接呼び出しの先。`GetProcAddress` 経由で SystemParametersInfoW を得る形は、DLL の文字列に `SystemParametersInfo` が 1 つ（import 名のみ）あるだけなので無いと読めるが、確認は静的）。
- 数値の照合: 11 種の数値は Learn の表で確認した。mingw-w64 の `winuser.h`（非公式）との照合は 11 種すべてが一致。値の根拠には使っていない。
