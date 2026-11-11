# 0210: Windows API 層(ゲストが呼ぶ OS の面)の現状の実測(census)と方式の再評価、体系的網羅の最初の一歩(14 API + Wine 差分テスト + crash1 の D&D の Wine 照合)。仮の決定

状態: **仮の決定**(作業者 `api2`。判断は PM)。基準は HEAD 6f5801c の `git archive`(ワークツリー `lift/out-api2/src/`。census は 9a08d07 で一度測り、出荷の既定が SHIP2〔決定 0208〕に切り替わった 6f5801c で測り直した。fe-lift と import 表は 2 つの HEAD で無変更)。**実エンジンは 1 度も実行していない**(静的な表、fe-lift の生成 C、runtime ソースの文面解析、Wine 10.0 とのテスト用 C プログラムの差分、emcc のコンパイルテスト)。**進捗ではない**。代役・モック・画像の再生・事前生成は無い。数字は全て本作業の実測で、読み(解釈)は『読み』と書く。ロードは 35〜50 と高かったので時間は主張しない。

## 0. 冒頭の三点

**(1) 何を実測したか。** (a) FLEngine と同梱 5 DLL(dsp_ippv2・Blend2D・FreeType・QFC・ILTools)、Fruity Wrapper、Delphi 製プラグイン 7 本(Sytrus・Harmor・Edison・Fruity Compressor・Fruity Parametric EQ 2・DirectWave・FLEX)、Serum2.vst3 の 15 画像の import 表(静的 + 遅延)から **993 distinct の (DLL, API)** を取り、各 API の runtime の状態を 4 構成(既定 TABLE・出荷 ship-imports・出荷予定 ship2〔決定 0208〕・全 seeds 合算)で文面解析した。さらに fe-lift で FLEngine と 7 画像を実際に lift し、生成 C の `fle_unsupported` を数えて『閉包の中で行が無い呼び出し』を出した。(b) 窓の仕組みの網羅性を、runtime のソース、実エンジンの窓表(`docs/investigations/clip1` の実走の記録)、Wine 10.0 の headless 実行との差分で調べた。(c) 方式 A〜D を Wine 10.0 のソース(取得して数えた)とコンパイルテスト、ReactOS のソース(sparse clone して数えた)で比べた。(d) 14 API を新規ファイルで実装し、Wine 差分テストを付けた(D&D の 3 API は crash1 が持つので外し、crash1 の実装を Wine と照合した)。

**(2) 結論。**
- **『エンジン+同梱 5 DLL が import していて、出荷構成〔SHIP2、決定 0208〕で未実装』は 300 distinct(行なし 242 + 名前付き停止のみ 58)。** 母数は distinct の import 853(静的 264 + 遅延のみ 36 が未実装)。行なしのうち 233 は runtime に関数が一つも無く、9 は関数はあるが出荷の行が無い。DLL 別: user32 67、kernel32 56、gdi32 54、comctl32 23、msacm32 13、ole32 13、winmm 13、shell32 12、wininet 10、imm32 7 ほか。ファミリーで 42 の塊。見積り 12,000〜30,000 コード行(中央 14k 行)。旧出荷(F1xH/F2xH)は 310、既定の生成 C は 343。
- **行なしの呼び出しは『API 名なしの abort』になる**(`fle: unsupported instruction at 0x…`。`fle_stop` の名前付きですらない)。fe-lift の閉包で実際にそこへ届く箇所(FLEngine)は、既定 225 箇所/220 distinct、旧出荷 197/193、**SHIP2 187/184**、全 seeds 合算 181/178。SHIP2 に api2 の行を足すと 178。
- partial 437(FLEngine の SHIP2 の静的 import 738 のうち)の多数は『NULL・定義外の入力で止まる』だけで、機能そのものの欠けは 67 API(エンジン+同梱の partial/stop 523 API 中。SHIP2 構成)。
- 窓の仕組み: 実エンジンの窓表 31 クラスは**全て Delphi 製のカスタムクラス**で標準コントロール(Button・Edit…)は 0。エンジンの 4 画像に標準クラス名の文字列は 0/40。**欠けは次の点に集中**している: (i) 枠の無い窓モデル(DefWindowProc の非クライアント処理、WM_SYSCOMMAND の SC_MOVE/SIZE/MAXIMIZE/MINIMIZE、モーダルな移動・サイズ変更ループが無い)、(ii) 更新領域が窓ごとに矩形 1 つで親→子の伝播が無い(WS_CLIPCHILDREN の無い親で Wine と食い違うことを実測)、(iii) HTTRANSPARENT で兄弟へ再試行しない、(iv) ScrollWindow・ScrollDC は関数が無い(ScrollWindowEx は旧出荷に行が無く SHIP2 で解消)、(v) メニュー(14 API)・ImageList(23 API)・ダイアログマネージャが無い、(vi) WM_SIZE/WM_MOVE を CreateWindowEx の後で送らない・WM_NCCALCSIZE の wParam・WM_NCPAINT/WM_ERASEBKGND の順序が Wine と違う(Wine との差分の実測)。
- **方式の推奨(決めるのは PM)**: **A(仕様から体系的に網羅)+ D(停止を soft にする安全網)を主軸にし、Wine 10.0 を『差分テストの基準』と『挙動の仕様書』として使う。Wine のソースの丸ごとの流用(B)・ReactOS(C)は採らない。例外として、窓から独立した自己完結のライブラリ(ImageList、ACM、MIDI の入力の枠組み)は、Wine の C を runtime に取り込む小さな B' を、ファミリー単位で評価する価値がある**(§7)。
- 14 API(新規ファイル、行 14 本)は、Wine 差分で 63 行中不一致 0(意図した相違 3 件は理由つき)。窓の更新領域の 12 行は Wine と 2 行が違い、**これは runtime の窓モデルの欠け(GAP-U1)として固定した**。D&D の 3 API は crash1(決定 0209)が既に持つので api2 から外し、**crash1 の実装を Wine と照合**した(20 行中 19 行一致。違うのは `cch == 0` でバッファありの 1 行: Wine は必要長 12、crash1 は 0。ページは無言)。

**(3) 未確認。** 実エンジンで各 API が実際にいつ呼ばれるか(静的な閉包は上限)。『impl/partial』が文書の挙動を網羅している証明(サンプルの 14 API を除き未検証)。窓の症状(ドラッグ中に描画されない・ホバーで順に更新)の原因の確定(仮説 H1〜H4 と確認方法だけ)。Wine のソースを wasm にして**実行**した結果(コンパイルとリンクのテストだけ)。wasm64 化の性能への影響。実機 Windows との照合は 0 件。動的に引く名前の網羅(リテラル引数の静的読みだけ)。

## 1. 凡例と方法

- **実測** = 本作業のコマンドの出力。**読み** = 実測からの解釈・推論。**推測** = 実機 Windows の挙動の推測。**未確認** = 確認していない。UNOFFICIAL = Wine 10.0 の挙動(Windows ではない)。
- **import の取り出し**(`tools/pe_imports.py`): pefile で静的 IAT と遅延 import を読み、`ff 15`/`ff 25` の rip 相対参照を数えた。**全 import が少なくとも 1 つの参照を持つ**ので、参照数は『到達』の指標にならない。到達の指標は fe-lift の閉包(次項)。
- **閉包**(`tools/unsup_thunks.py`): `fe-lift lift IMAGE out.c --pdata [--extra-imports ROWS]` の生成 C にある `fle_unsupported(VA)` の VA の命令(`jmp/call [rip+slot]`)を読み、IAT の import 名に戻した。FLEngine は既定で sha256 `cd9107ce…2e50`(215,811,678 B)を再現した。FLEngine の生成 C に API 以外の unsupported は 0(同梱 DLL は命令の unsupported があるので import だけを数えた)。
- **状態の判定**(`tools/rt_index.py` + `tools/api_census.py`。実行しない文面解析): fe-lift の import 表 `TABLE`(608 行)と `lift/harness/seeds/*imports*.txt` の行で『行』の有無を決め、行の指す `fle_imp_*` の本体(マクロ定義の名前付き停止 `GDI_STOP`・`FLE_PIDL_STUB`・`FLE_VOL_NAMED_STOP`・`FLE_NAMED_STOP` を展開)の `fle_stop` を数える。
  - **none** = 行なし(呼ぶと API 名なしの abort)。
  - **stop** = 名前付き停止のみ(戻り値を設定せず無条件に停止)。
  - **stub** = 引数を読まず、他を呼ばず、停止せず、8 行以下(canned な答え)。
  - **partial** = 本体または呼ぶ runtime の helper から到達する `fle_stop` が 1 つ以上。
  - **impl** = 到達する `fle_stop` が無く、stub でもない。**『文書の挙動を網羅』の証明ではない**(表の列名は `impl`、『完全』と呼ばない)。
  - テスト: 表の `tests_word_hits` は『API 名がテストソースに現れた回数』(弱い指標。エンジン+同梱の 614 API〔impl/partial/stub/stop〕のうち 602 に出現)。
- runtime(api2 の追加分を除く): 3,128 関数を索引(harness を含む)、runtime の .c の `fle_imp_*` の定義 663(マクロ定義の名前付き停止を足して索引では `fle_imp_*` が約 700)、`fle_stop(` の呼び出し 1,287(理由の文字列つき 1,245)、コード 32,008 行(コメント・空行を除く)。
- **構成**: default = `TABLE`、ship = TABLE + `ship-imports.txt`(旧出荷 SHIP_PRESET F1xH/F2xH、決定 0189)、**ship2 = TABLE + `ship2-imports.txt`(決定 0208。117 行。HEAD 6f5801c の既定の入口)**、max = TABLE + 全 seeds(ia1/ia2/dlg1/exp1/wrp1/wrp2 など。api2 の行は含めない)。
- **Learn の URL**: mingw-w64 のヘッダー名(SDK のヘッダー分割と同じ)から `…/api/<header>/nf-<header>-<api>` を作り、866 distinct を GET で確認した(**200 が 826、404 が 40**。console 系などはヘッダー名が違う)。URL の無い行 126(序数 import・COM メソッド・索引に無い名前)。404 は表の `learn_http` 列に残した。

## 2. census の結果(`tables/api-coverage.tsv` 993 行、`api-census-by-image.tsv`、`api-census-by-dll.tsv`)

### 2.1 画像ごとの import 数と SHIP2 構成の状態(distinct の (DLL, API))

| 画像 | import(静的+遅延) | impl | partial | stub | stop | none(行なし) |
|---|---:|---:|---:|---:|---:|---:|
| FLEngine 静的 | 738 | 49 | 437 | 12 | 47 | 193 |
| FLEngine 遅延 | 57 | 3 | 17 | 0 | 0 | 37 |
| dsp_ippv2 | 72 | 27 | 24 | 3 | 16 | 2 |
| Blend2D | 94 | 34 | 34 | 3 | 16 | 7 |
| FreeType | 74 | 26 | 24 | 3 | 13 | 8 |
| QFC 静的/遅延 | 217/6 | 40/0 | 143/3 | 4/0 | 21/0 | 9/3 |
| ILTools 静的/遅延 | 520/39 | 45/2 | 342/13 | 6/0 | 46/0 | 81/24 |
| Fruity Wrapper 静的/遅延 | 584/47 | 46/3 | 370/15 | 7/0 | 46/0 | 115/29 |
| Serum2.vst3 | 332 | 52 | 145 | 3 | 16 | 116 |

(旧出荷・既定・全 seeds の同じ表は `api-census-by-image.tsv`。Delphi プラグイン 7 本は同じ表の行。FLEngine 静的 738 は文書 `bundled-dll-inventory` の『738 名』と一致、33 DLL。)

### 2.2 『エンジンまたは同梱 5 DLL が import していて未実装』(`tables/unimplemented-imports.tsv`)

SHIP2 構成で none または stop の (DLL, API): **300**(none 242: 関数なし 233 + 関数ありで行が出荷に無い 9、stop 58)。母数は 853。静的 IAT 経由 264(遅延のみ 36)。FLEngine 単独 276。同じ数え方で、既定構成 343、旧出荷 310、全 seeds 291。DLL 別(300 の内訳): user32 67、kernel32 56、gdi32 54、comctl32 23、msacm32 13、ole32 13、winmm 13、shell32 12、wininet 10、imm32 7、propsys 5、winspool 5、uxtheme 4、mpr 3、comdlg32 2、imagehlp 2、msimg32 2、opengl32 2、advapi32 1、oleacc 1、oleaut32 1、shlwapi 1、windowscodecs 1、wtsapi32 1、api-ms crt 1。

『関数があるが SHIP2 の行が無い』9: FreeLibraryAndExitThread、GetUserDefaultLCID、IsProcessorFeaturePresent、TlsFree、WaitForSingleObjectEx、acmDriverClose/Enum、acmFormatSuggest、acmStreamOpen(ia2/wrp/exp1 の seeds にある)。**旧出荷にあった 19 のうち 10**(ScrollWindowEx・DoDragDrop・MessageBoxW・ShellExecuteW・InvalidateRgn・GetKeyNameTextW・Get/WritePrivateProfileStringW・SetThreadExecutionState・GetTimeZoneInformationForYear)は SHIP2 の行で解消している(fe-lift で実測: 旧出荷の閉包 193 → SHIP2 184 distinct)。

### 2.3 fe-lift の閉包で『行なし』に届く箇所(`tables/unsup-*.tsv`、`closure-unmapped-by-dll.tsv`)

| 画像/構成 | 箇所 | distinct | 内訳(distinct) |
|---|---:|---:|---|
| FLEngine 既定 | 225 | 220 | user32 66、kernel32 32、comctl32 24、gdi32 20、ole32 16、shell32 14、winmm 13、msacm32 13、wininet 10 … |
| FLEngine 旧出荷 | 197 | 193 | user32 47、kernel32 26、comctl32 23、gdi32 20、ole32 13、msacm32 13、winmm 13、shell32 12、wininet 10 … |
| **FLEngine SHIP2** | **187** | **184** | user32 43、comctl32 23、kernel32 23、gdi32 20、winmm 13、msacm32 13、ole32 12、shell32 11、wininet 10、winspool 5、mpr 3 … |
| FLEngine SHIP2 + api2 | 181 | 178 | |
| FLEngine 全 seeds | 181 | 178 | |
| FreeType/Blend2D/QFC/ILTools(既定) | 16/13/11/101 | 9/6/10/98 | |
| Fruity Wrapper/Serum2/dsp(既定) | 133/138/5 | 129/100/1 | |

SHIP2 で閉包に届く物の例: user32 の CreateMenu・CreatePopupMenu・TrackPopupMenu・DestroyMenu・InsertMenuItemW・GetDlgItem・**ScrollWindow・ScrollDC**・SendInput・ToUnicode・FindWindowExW・SetWindowPlacement、gdi32 の TextOutW・CreateFontW・BeginPath・StartDocW、kernel32 の CreateProcessW・OpenProcess・HeapDestroy・VirtualQueryEx、comctl32 の ImageList_*(23 全部)。**これらは踏むと API 名なしで abort する**。

### 2.4 動的に引く名前(`tables/dynamic-names.tsv`、`loadlibrary-names.tsv`、`dyn-sites-*.tsv`)

生成 C のリテラル引数(直前に代入された rcx/rdx の定数)から読んだ。FLEngine: GetProcAddress は 12 箇所(リテラル 6 = kernel32 の 5 名〔GetLogicalProcessorInformation・GetThreadPreferredUILanguages・SetThreadPreferredUILanguages・GetThreadUILanguage・GetLongPathNameW〕、非リテラル 6 = 遅延ロードヘルパほか)、LoadLibraryW のリテラル 63 箇所(うち runtime のモジュール表に無く**名前付き停止 21**、absent 2、仮想 40)、GetModuleHandleW 34 箇所(NULL と 126 を返し停止しない)。Serum2 の GetProcAddress は 18 名(user32 の DPI 系 9、shell32 の SHGetKnownFolderPath/SHCreateItemFromParsingName、dcomp、ntdll の `wine_get_version` など)。dsp の動的名は過去調査(`gpa_decode.out`)の表を再利用した 9 名。**限界**: dsp 以外の名前表(インデックスで引く物)・非リテラルは読んでいない。

## 3. 状態の限界と停止の理由別内訳(`tables/stop-reasons-by-dll.tsv`、`stop-reasons-examples.txt`)

エンジン+同梱の partial/stop 523 API(SHIP2 構成)の `fle_stop` 789 箇所をキーワード規則(S-api2-1)で分けた: NULL 引数 250、文書が挙動を定義しない入力 167、機能・フラグ・状態を『未実装/未モデル』と明記 99、固定上限・メモリ 39、その他 234(決定の言及・環境の記述・名前付き停止マクロ)。API 単位の最も強い理由: 未モデル 67、定義外入力 76、上限 13、その他 111、NULL のみ 66、helper 経由のみ 190。読み: **partial の多くは『NULL/定義外入力』で止まる安全弁で、実エンジンが踏む可能性は低い。踏みうるのは『未モデル』の 67 API と、行なしの 300**。

## 4. 窓の仕組みの網羅性(`tables/window-semantics.tsv` 21 行、`window-classes-observed.tsv`、`class-strings.tsv`)

### 4.1 実測した事実

- 実エンジンの窓表(`docs/investigations/clip1/logs/runs/wintable-idle.txt` の 107 窓、31 クラス)は**全て T で始まる Delphi のクラス**(TVectorPanel 21、TPUtilWindow 19、TQuickEditToolbarPanel 13 …、TFruityLoopsMainForm、TEdgeBrowser〔WebView2〕)。スタイルはパネル類(TVectorPanel・TQuickEditToolbarPanel・TVectorSheet・TNewCaption …)が `WS_CHILD|WS_VISIBLE|WS_CLIPSIBLINGS|WS_CLIPCHILDREN`。**メインフォームは `0x9c030000` = WS_POPUP|WS_VISIBLE|WS_DISABLED(0x08000000。モーダルな TWelcomeWizard が開いている時の記録)|WS_CLIPSIBLINGS|MINIMIZEBOX|MAXIMIZEBOX で、WS_CAPTION も WS_CLIPCHILDREN も無い**(自前の TNewCaption を子に持つ)。**WS_CLIPCHILDREN が無い窓**(窓表 107 のうち 43。うち TPUtilWindow 19 はメッセージ用の POPUP): メインフォーム TFruityLoopsMainForm、TMIDIForm、TStepSeqForm、TFXForm、TSampleListForm、TEventEditForm、TGraphEditorForm、TPluginForm、TPythonForm、TTestForm、TWelcomeWizard、TWPControl(6)、TWinControlProxy(5)。= **メインフォームと道具フォームの全てが CLIPCHILDREN を持たない**。
- 標準コントロールのクラス名(Button・Edit・ComboBox・ListBox・ScrollBar・Static・SysTabControl32・SysListView32・SysTreeView32・msctls_*・ToolbarWindow32・tooltips_class32・RichEdit …)の文字列は **FLEngine・QFC・ILTools・Fruity Wrapper に 0/40**、Serum2.vst3 に `Edit` と `tooltips_class32`(コードから参照あり)。= エンジン自身は標準コントロールを作らない。サードパーティの VST3 とファイルダイアログ・印刷には残る。
- runtime のクラス表は標準クラス名を『答えない』(`fle_class.c:96-113`、`GetClassInfoW(NULL, …)` は『the system classes are not modelled』で停止)。
- エンジンは WM_SYSCOMMAND を SC_MINIMIZE `0xF020`(0x8697d7)・SC_RESTORE `0xF120`(0x8698bd)・SC_MAXIMIZE `0xF030`(0x8612ff、0x868082)で送り、フォームの手続きで SC_MOVE `0xF010`(0x861ad9)を比較している(出荷の生成 C の命令アドレス)。runtime の DefWindowProc は WM_SYSCOMMAND の SC_CLOSE だけ実行し、他は『枠の無いモデルで効果なし』(`fle_input.c:731`)、非クライアントのマウス(0xA0〜0xA9)の既定結果は 0(`fle_input.c:743-746`)。`WM_ENTERSIZEMOVE` は runtime のどこにも無い。

### 4.2 意味論ごとの状態(要約。全 21 行は `window-semantics.tsv`)

| 意味論 | 状態 | 一言 |
|---|---|---|
| メッセージ取得の順序(送信→投稿→入力→WM_PAINT→WM_TIMER) | 実装 | 文書の順。WM_TIMER は 1 取得に 1 回(S-timer) |
| 更新領域・WM_PAINT | 部分 | 窓ごとに矩形 1 つ。親子の伝播なし(GAP-U1)。InvalidateRect(NULL) と GetUpdateRect(bErase) は停止 |
| 描画の切り抜き(子・兄弟) | 実装 | `fle_visrgn.c`(決定 0142) |
| DefWindowProc: クライアント側 | 実装 | S- の選択つき |
| DefWindowProc: 非クライアント(枠・システムメニュー) | **欠け** | 枠の無いモデル(S-nc-zero) |
| モーダルな移動/サイズ変更ループ(SC_MOVE/SIZE) | **欠け** | `WM_NCLBUTTONDOWN(HTCAPTION)` と `WM_SYSCOMMAND` は結果 0 |
| マウスの経路: ヒットテスト・HTTRANSPARENT・キャプチャ | 部分 | HTTRANSPARENT は兄弟を試さず捨てる(S-httransparent) |
| キャプチャ・フォーカス・アクティブ化 | 実装 | AttachThreadInput は行なし |
| ScrollWindow/ScrollWindowEx/ScrollDC | **欠け** | ScrollWindowEx は関数あり(SHIP2 には行がある。描画面が無いと停止)、ScrollWindow・ScrollDC は関数も行も無い |
| 標準スクロールバー | 部分(状態のみ) | SetScrollInfo/GetScrollInfo/ShowScrollBar は状態のみ、Get/SetScrollPos・EnableScrollBar は行なし |
| 標準コントロールのクラス | **欠け** | クラス表に無い(エンジンは使わない) |
| メニュー(HMENU) | **欠け** | システムメニューだけ。14 API が閉包 |
| ダイアログマネージャ・MessageBox | 部分 | MessageBoxW は既定ボタンの答え(ia2)、他は行なし |
| レイヤード窓 | 実装 | S-ulw-no-messages |
| SetWindowPos/MoveWindow/z 順 | 実装 | 位置だけの変更は何も無効化しない(Windows と同じ) |
| タイマー・フック・クリップボード | 実装/部分 | フックは一部のメッセージだけ、遅延描画は停止 |
| IME・他スレッドの窓 | 欠け/部分 | imm32 は absent |

### 4.3 Wine 10.0 との差分の実測(headless Wine。`DISPLAY` なしで窓・更新領域・メッセージが動く)

**更新領域**(`harness/api2/win_upd.c`。同じ 12 行を Wine の user32 と runtime の本物の窓層〔`fle_win.c` ほか 30 ファイル、g3 と同じ足場〕に通した。Wine の出力が基準〔UNOFFICIAL〕):
- 親が `WS_CLIPCHILDREN` を持つ場合(エンジンのパネル類): Invalidate(親)・Invalidate(子)・Invalidate(親の一部)の 5 行**全て一致**。
- 親が `WS_CLIPCHILDREN` を持たない場合: **Wine は親の無効化を子へ伝える**(Invalidate(親) で子の更新矩形 (0,0,100,100)、Invalidate(親,[30,30,60,60]) で子の (10,10,40,40))。runtime は伝えない(子の更新矩形 0)。= **GAP-U1**。**メインフォームと道具フォームは全て CLIPCHILDREN が無い**(§4.1)ので、フォームを無効化(例: 他の窓の下から現れる、サイズ変更、Invalidate)しても子の部品が再描画の対象にならない。S1・S2 の症状(フォーム全体が更新されず、ホバーした部品から順に更新される)と整合する。
- ScrollWindowEx は runtime が『窓の描画面が無い』で停止するので Wine だけの行(表に記録)。

**メッセージの並び**(`harness/api2/win_seq.c`。作成・表示・移動・サイズ・非表示・破棄の 10 手。`tables/window-sequence-diff.txt`、`expected/win-seq-*.txt`): 一致する物 = WM_NCCREATE→WM_NCCALCSIZE→WM_CREATE、WM_SHOWWINDOW→WINDOWPOSCHANGING→CHANGED、位置変更の WM_MOVE、サイズ変更の WM_NCCALCSIZE→WM_SIZE、破棄の WM_DESTROY→子の WM_DESTROY→子の WM_NCDESTROY→親の WM_NCDESTROY。**違う物**(UNOFFICIAL):
1. CreateWindowEx の WM_CREATE の後に Wine は **WM_SIZE と WM_MOVE** を送る。runtime は送らない(窓を作った直後にレイアウトを受け取る VCL のコードが、最初の WM_SIZE を待つ場合に影響しうる)。
2. 表示のとき Wine は WINDOWPOSCHANGING と CHANGED の間に **WM_NCPAINT と WM_ERASEBKGND** を送る。runtime は WM_PAINT の中で BeginPaint が WM_ERASEBKGND を送るだけ。
3. サイズ変更の WM_NCCALCSIZE の wParam は Wine が TRUE(NCCALCSIZE_PARAMS)、runtime は FALSE(RECT)。
4. 同じ位置・サイズへの SetWindowPos で、Wine は CHANGED を送らず、runtime は送る。WINDOWPOSCHANGED の flags の内部ビット(0x800/0x1000/0x8000)も違う。

### 4.4 症状への当てはめ(仮説。**確認はしていない**)

依頼者の症状 (S1)『ウインドウのドラッグ中に描画が更新されない』(S2)『ドラッグ後に更新されず、ホバーした部品から順に更新される』。読み:
- **H1(モーダルな移動ループの欠け)**: エンジンが窓の移動を `WM_NCLBUTTONDOWN(HTCAPTION)`/`WM_SYSCOMMAND(SC_MOVE|HTCAPTION)` の既定処理に任せているなら、runtime の DefWindowProc は何もしない。ただし FL のメインフォームには WS_CAPTION が無く、出荷の生成 C に SC_DRAGMOVE の定数 0xF012 は 1 つも無かった(0xF010 は比較のみ)。FL 自身が SetWindowPos で動かす可能性が高く、その場合 H1 は当たらない。確認: native の fork プローブでキャプションを掴むドラッグを入力し、SetWindowPos/MoveWindow とその WM_* を記録する(`FLE_G3_TRACE`、`FLE_RASTER_TRACE` が既にある)。
- **H2(更新領域の伝播の欠け)**: CLIPCHILDREN の無いフォーム(メインフォーム含む)の再描画漏れ(§4.3)。**実測で裏づいた欠けのうち症状と整合するのはこれ**(ただし S1/S2 の原因がこれだとは確認していない)。S2『ホバーした部品から順に更新』と整合する(ホバーで個々の部品が自分を無効化するので、そこから順に描画される、という読み)。確認: 当該フォームを別の窓の上で動かす native プローブで、移動後の子の更新矩形を記録する。
- **H3(HTTRANSPARENT)**: VCL の部品が WM_NCHITTEST で HTTRANSPARENT を返して親にクリックを渡している場合、クリックが捨てられる。確認: エンジンの WM_NCHITTEST ハンドラが -1 を返すか(生成 C を grep)。
- **H4(ホスト側)**: 位置だけの SetWindowPos は何も無効化しない(Windows と同じ)ので、ページが窓の canvas を動かしたときに面の内容を保持していない場合、エンジンは正しく何も再描画せず、見かけだけ空になる。確認: ページで面表の更新(x, y, 世代)を記録し、native で位置だけの SetWindowPos の前後で面のバイトが変わらないことを見る。
- **別の事実**: 決定 0191 の壁 ia2-W23 は『ScrollWindowEx — プレイリストのホイールとスクロールバー』、ia2-W31 は『エンジンのスクロール helper(`f_825d20`)が ScrollWindowEx の埋めた領域を無効化する』と記録している(`fle_wall_ia2w.c`、`ia2-imports.txt`)。依頼者の『Playlist をマウスホイール押下でドラッグ』がこの経路だとは確認していない(読み)が、旧出荷(ship-imports.txt)では ScrollWindowEx が行なし→ abort する構成だった(確認済み)。**SHIP2(現在の既定の入口)では ScrollWindowEx は通るが、ScrollWindow と ScrollDC は行なしで関数も無い**(閉包に届く)。ScrollWindowEx の本体は『窓の描画面が無い』と停止する経路を持つ(native の足場で再現。実エンジンでの確認は未)。

## 5. 方式の再評価(§7 に比較表と推奨)

### 5.1 数字(実測は全て本作業)

- **Wine 10.0**(git `wine-10.0`、374 MB): user32 129,029 行、win32u 93,940、gdi32 83,398、comctl32 133,788、ole32 72,081、shell32 85,667(6 つで 598k 行)、kernelbase 44,952、kernel32 95,882、ntdll 178,366、oleaut32 80,425、wininet 40,615、winmm 27,314、server/ 55,574(窓・キュー・フック・クラス・winstation・clipboard・region・timer・user で 11,302)。win32u の server 要求の呼び出し箇所 158、server の要求 292 種、win32u の syscall 表 862 エントリ(NtUser と NtGdi。`win32syscalls.h`)、`struct user_driver_funcs` の関数ポインタのメンバー 61。
- **コンパイルテスト**(`tools/wine_compile_try.sh`): `./configure --enable-win64` と `make __tooldeps__`、`make -C include` で widl 生成ヘッダーを作り(bison/flex は deb をユーザー領域へ展開)、`emcc -c -O1 -sMEMORY64=1 -fshort-wchar -D__x86_64__`(CONTEXT 用に CPU を偽装、winnt.h の x86 inline asm 3 か所を `!__wasm__` で無効にした scratch 木)で 16 ディレクトリの 301 ファイル中 **285 が単体で通る**(95%、オブジェクト合計 8.75 MB。初回は 268〔89%〕で、ole32 の `dcom.h`〔`make -C dlls/ole32 dcom.h`〕と `-include winternl.h`〔`struct _TEB` の定義〕で 17 本増えた)。win32u 39/41(落ちる 2 本は clang のクラッシュと winstation.c)、comctl32 35/35、user32 37/38〔png.h が無い 1 本〕、shell32 44/44、gdi32 9/9、kernelbase 13/16〔x86 inline asm〕、ole32 26/27、oleaut32 11/15、wininet 6/9。server/ 36/42(落ちるのは Linux 依存の fd・sock・process・change・ptrace・unicode)。**wasm32 では通らない**(`unsigned long` が 4 バイトで winnt.h の 64 ビットのビットフィールドが崩れる)。
- **-Oz でリンクした実測**: win32u 0.69 MB(オブジェクト 1.71 MB)、comctl32 0.67 MB(1.24 MB)、gdi32 0.16 MB(0.27 MB)。残りは同じ比(0.4〜0.57)の外挿で、16 ディレクトリ合計 **約 3.5〜5.0 MB(読み)**。出荷予定 SHIP2 の S1 の余裕は 10.9 MB(決定 0208)なので、コードの大きさだけなら収まる。server・ntdll の一部は別。
- **Wine の PE を lift する案(B2)**: Wine の組み込み PE のコード部 comctl32 705 KB、shell32 552 KB、user32 560 KB、gdi32 314 KB、ole32 332 KB、oleaut32 365 KB、win32u 55 KB(Unix 側への syscall の薄い殻だけ)。comctl32.dll の import は 381(user32 181、gdi32 89、kernelbase 53、ucrtbase 29 …)で、**現行 runtime に行が無い物が 159**(`tables/wine-pe-imports-coverage.tsv`)。上位の DLL を lift しても下位(user32/gdi32/kernel32)の網羅が要る。
- **ReactOS**(sparse clone): win32ss 371,789 行(user/ntuser 79k、user/user32 62k、gdi/ntgdi 45k、gdi/gdi32 35k)、comctl32 87k、shell32 95k、kernel32 81k。win32ss は GPL-2.0-or-later の NT カーネルモード(win32k.sys)で、ntoskrnl の Ob/Ke/Mm に依存し、ユーザーモードのライブラリとしては取り出せない。

### 5.2 B の論点(読みと実測の境界)

- **表示ドライバ**: win32u は `struct user_driver_funcs`(約 61 メンバー)を介して画面とやりとりし、`driver.c` に `nulldrv_*` の既定実装がある。本機の headless Wine(`DISPLAY` なし)で窓の作成・更新領域・メッセージ列・ScrollWindowEx が動いたことは、nulldrv の経路で窓の意味論が成立する実測。ブラウザ用ドライバは、窓の面(window_surface)の flush を現行の面表(`app/src/engine/surface-table.ts`)へ渡す形になる(読み。未実装)。
- **wineserver の同一プロセス化**: 窓・キュー・フック・クラス・winstation・clipboard・region・timer・user の server 側 11,302 行を含む 36/42 ファイルは wasm64 で通る。通らない 6(fd・sock・process・change・ptrace・unicode)は poll・ソケット・プロセス管理の Linux 依存で、`wine_server_call` の同期 RPC を直接の関数呼び出しにする置換と、待ち(select 要求)をゲストの単一スレッドのスケジューラ(`fle_thread.c`)へつなぐ設計が要る(未設計)。
- **スレッド**: win32u は pthread と server の待ちを使う。現行エンジンは Worker + 仮想時間のゲストスレッドで、wasm のスレッド対応(`fle_thread_wasm.c`)は別途ある。Wine の待ちを仮想時間に載せ替える作業は未測定。
- **ポインタ幅**: wasm32 でコンパイルするとポインタが 4 バイトで、`ULONGLONG` などが ILP32 になり winnt.h が崩れる(実測)。x64 のゲストの構造体と配置を合わせるには wasm64(memory64)にするか、wow64win 相当の変換層(Wine に約 10k 行ある)を書く。**エンジン本体は wasm32**(2 GiB)なので、Wine の層だけを別の wasm64 モジュールにするとゲストのメモリを共有できない。= B は事実上『エンジン全体の memory64 化』を伴う(未測定、性能への影響は不明)。 memory64 自体は node 24.19 で `emcc -sMEMORY64=1` の小さなプログラムが実行できる(`sizeof(void*)=8`、`sizeof(long)=8`=LP64。実測)。memory64 は Firefox 134・Chrome 133 で出荷済み(SpiderMonkey 開発者のブログ 2025-01-15 `spidermonkey.dev/blog/2025/01/15/is-memory64-actually-worth-using.html`、caniuse)で、依頼者の Firefox 159 と本機の Chrome 156 は対応する範囲。**ただし同じブログは『64 ビットモードは 32 ビットより遅く、負荷により 10% から 100% 超』と書く**(32 ビットはガードページで境界検査を消せるが 64 ビットは消せない)。= B のためにエンジン全体を memory64 にする案は、P2/P3 の性能に直接響く(一次資料。数字はその負荷の話で、本エンジンでの値は未測定)。

### 5.3 ライセンスの読み(法的助言ではない)

Wine は LGPL-2.1-or-later(全ソースの `COPYING.LIB` と各ファイルの表示)。依頼者だけがアクセスする私的なデプロイでも、wasm をブラウザへ送ることは『複製の提供』にあたりうる。その受け手が依頼者本人(ソースを持つ)なので義務は事実上満たされると読めるが、公開する場合は LGPL の義務(ソースの提示、再リンク可能性、変更の明示)が要る。本リポジトリの方針(決定 0158 #8: 無ライセンスは参考のみ)は LGPL には当てはまらない。ReactOS の win32ss は GPL(LGPL ではない)で、組み込むと全体が GPL になりうる(公開しなければ義務は生じない、と読む)。

## 6. 体系的網羅の最初の一歩: 14 API(新規ファイルのみ。既存を触らない。opt-in)

PM の指示に従い **kernel32/shell32 の文字列・ロケール・時刻・コンソール・リンクに限定**(user32/gdi32/comctl32 の窓・メニュー・ImageList は触らない)。dec1(決定 0206)が実装中の GetFileSizeEx・GetUserDefaultLCID・EnumSystemLocalesW・CompareFileTime・GetDiskFreeSpaceW・CreateFileA 、crash1(決定 0209)の DragQueryFileW・DragQueryPoint・DragFinish(最初は実装したが、重複シンボルになるので外した)、既存の関数(ia2/exp1 の行なし出荷物)は避けた。

| ファイル | API |
|---|---|
| `lift/runtime/fle_api2_str.c` | CompareStringA、LCMapStringA、GetStringTypeA、GetLocaleInfoA(W 版へ CP_ACP で変換して委譲)、TzSpecificLocalTimeToSystemTime(夏時間規則を TIME_ZONE_INFORMATION の文書から実装。曖昧な時刻・存在しない時刻は文書どおり夏時間) |
| `lift/runtime/fle_api2_sys.c` | GetLargePageMinimum(0: 環境の設定)、VirtualQueryEx(現プロセスの疑似ハンドルのみ)、OutputDebugStringA(何もしない)、WriteConsoleA・ReadConsoleW・GetConsoleCP(コンソール無し→失敗+ERROR_INVALID_HANDLE)、CreateSymbolicLinkW(非昇格→失敗+ERROR_PRIVILEGE_NOT_HELD 1314) |
| `lift/runtime/fle_api2_shell.c` | ShellExecuteA・ShellExecuteExW(ia2 の S-ia2-shellexecute-noassoc に合わせ SE_ERR_NOASSOC 31 / ERROR_NO_ASSOCIATION 1155。『ブラウザが開いた』ではない) |
| `lift/runtime/fle_api2_call.c/.h` | runtime 内から別の `fle_imp_*` をゲスト ABI で呼ぶ小さな道具(ゲストのスタックの下に枠を作る) |
| `lift/harness/seeds/api2-imports.txt` | 14 行(新しい行だけ。`cat ship2-imports.txt api2-imports.txt` を `--extra-imports` に) |

**Wine 差分テスト**(`lift/harness/api2/`): `api2_cases.c` を (a) mingw(`-mwindows`、コンソール無し)で作って Wine 10.0 の本物の API に、(b) native で runtime の関数に通し、1 ケース 1 行の出力を比べる(`build.sh diff`)。TZ=UTC・en-US の Wine。**63 行を比べて不一致 0**。意図した相違 3 件(`expected/known-divergence.txt`、理由つき): GetStringTypeA の NULL 入力(文書: ERROR_INVALID_PARAMETER、Wine は成功)、TzSpecific の存在しない日付 2023-02-29(Wine は正規化、runtime は失敗 87)、WriteConsoleA 失敗時の `*lpNumberOfCharsWritten`(runtime は触らない、Wine は 0)。Wine が基準にならない 6 件(大ページ・シンボリックリンク・ShellExecute・OutputDebugString)は `expected/native-only.txt` に文書から手書き。Wine との最初の相違から直した物が 1 件: GetLocaleInfoA の LOCALE_RETURN_NUMBER の戻り値(バイト数 4: S-api2-8、UNOFFICIAL)。

**D&D の Wine 照合(crash1 の実装に対して)**: `api2_drag.c`(HDROP を GlobalAlloc(GMEM_FIXED) で作り、DragQueryFileW 16 ケース・DragQueryPoint 3 ケース・DragFinish 1 行の計 20 行)を Wine 10.0 と、crash1 の `fle_crash1_drag.c` にそれぞれ通した(`build.sh drag WORKDIR IMPL.c`、`DRAG_RT` と `DRAG_REG` で crash1 の木と `fle_drop_register` を指定)。**20 行中 19 行が一致**。違う 1 行: `DragQueryFileW wide-cch0-buf`(有効な添字、バッファあり、cch 0)は Wine が必要長 12 を返し、crash1 は 0 を返す。ページは cch 0 を書かない(S-api2-7、UNOFFICIAL)。crash1 の ANSI リストは 0x80 以上のバイトで停止する(CP_ACP 変換は未実装)。Wine の出力は `expected/wine-10.0-drag.txt`。

**Rust テスト**(新規 `crates/fe-lift/tests/api2.rs`、4 件): 行が新しくて宣言・定義がある、Wine の出力と一致(意図した相違を除く)、定義外の入力は名前付きで停止、窓の更新領域の 12 行が Wine と一致(GAP-U1 の 2 行を除く。直ると『エントリを消せ』で落ちる)。変異 13 本中 10 本を検出(`logs/mutation-api2.txt`。M1〜M7 は D&D を api2 から外す前の変異〔M3 と M6 は今は crash1 側の関数〕、M8〜M13 は最終の木)。生き残り 3 本: M4 LCMapStringA のサイズ問い合わせの UTF-16 数(単一バイトの code page で常に等しい〔等価〕)、M7 CompareStringA の第 2 長さを無視する(未終端の確保の後ろが 0 のとき区別できない〔不安定〕)、M13 LCMapStringA の LCMAP_SORTKEY の分岐(ケースが無い=**テストしていない経路**)。

**wasm32 での実行**: `api2_cases.c` と runtime の 13 ファイル(api2 の 4 本を含む)を emcc(wasm32、-O1)でビルドし node 24.19 で実行した出力が、native の出力と 68 行すべて一致した(`logs/wasm-node-api2.txt`)。= 新しい C はゲストのポインタ=ホストのポインタ(同一視の地図、32 ビット)の下でも同じ答えを返す。

**生成 C への影響**: 既定の生成 C は不変(fe-lift 無変更、新規 runtime 4 ファイルは行が無ければ呼ばれない)。FLEngine の既定 sha256 `cd9107ce…` を再現(`logs/gen-default.sha256`)。SHIP2 + api2 では閉包の『行なし』が 184 → 178(CompareStringA・GetConsoleCP・ShellExecuteA・ShellExecuteExW・TzSpecificLocalTimeToSystemTime・VirtualQueryEx の 6)。他の 8 行は FLEngine の閉包には届かない(同梱 DLL や動的名の側)。**出荷の import 行に加えるのは PM の判断**(行を加えるビルドは `fle_api2_*.c` を runtime のファイルとして取り込む必要がある)。

## 7. 方式の比較と推奨(決めるのは PM)

| 観点 | A: 仕様から体系的に網羅(現行 runtime を拡張) | B: Wine の C を wasm にして API 層にする | B2: Wine の PE(上位 DLL)を lift | C: ReactOS | D: 現行維持 + 停止を soft に(crash1) |
|---|---|---|---|---|---|
| 『踏んだら止まる』 | SHIP2 の閉包 184(未実装全体 300)を潰す。系統単位で漸減 | 原理的に消える(user32/win32u/gdi32 の全 API)。ただし下位(ntdll・server)の穴が移る | 上位の穴は消えるが下位 159+ API が要る | 同左+NT カーネル依存 | 止まらないが挙動は欠けたまま(失敗が返る) |
| 窓の正しさ | 欠けは §4 の 6〜8 点。Wine 差分で個別に潰せる(更新領域・メッセージ列は実測済み) | 完全(Windows 互換の 25 年分)。枠・モーダルループ・メニュー・標準コントロール込み | 同左(user32 は B と同じ問題) | 同左(別実装) | 変わらない |
| 作業量 | 300 API = 12k〜30k 行(中央 14k)、読み 7〜15 チーム日 + Wine 差分 +30〜50%。最初の 17 API〔後に 14〕の実装とテストに約 25 分(読みと設計を除く。W 関数への委譲が多い簡単な物で 1〜2 分/API) | 598k 行 + server 56k + ntdll 一部。コンパイル 95% は実測、**リンク・実行・server の同一プロセス化・関数ポインタの橋渡し・wasm64 化は未測定**。読み: 数週間〜数か月、失敗リスク大 | 上位 DLL の lift は容易だが下位の網羅が先 | B より重い | 小(設計中) |
| 性能(P2/P3) | 現行(Delphi の GDI 呼び出しは runtime の C) | Wine の dibdrv(ソフト描画)の速度は未測。wasm64 は境界検査を消せず 10%〜100% 超遅いと SpiderMonkey の開発者が書く(§5.2。本エンジンでは未測定) | 上位は lift の 2〜6 倍遅い(0158 の合格線) | 未測 | 変わらない |
| サイズ(S1) | 増えは数百 KB | +3.5〜5.0 MB(読み、-Oz 外挿)+ server。SHIP2 の余裕 10.9 MB に収まる見込み | PE 1 本 = コード部 ×(1.84 の比)で comctl32 だけで約 1.3 MB | 未測 | 0 |
| ポインタ幅・メモリ | 現行(ゲストのポインタは 64 ビット値、wasm32) | **構造体の配置が合わない**(wasm32 の Wine はポインタ 4 バイト)。wasm64 にするか変換層(wow64win 相当)が要る。現行エンジンは wasm32 | lift 側に統一される | — | 変わらない |
| ライセンス | 自作 | LGPL-2.1+(§5.3) | LGPL | GPL | 自作 |
| 既存の検証済み資産 | 維持(約 15 日分の実装・決定・テストを持つ約 15k 行。依頼者の実機では 12 セッション中 10 が停止した、という欠けも同じ層にある) | **捨てる**(fle_win/gdi/raster/visrgn/input の約 15k 行と、その上の決定・テスト) | 維持 | 捨てる | 維持 |

**D の対象 300 の戻り値の型**(`tools/ret_types.py`、mingw ヘッダー。`tables/api-return-types.tsv`): BOOL/BOOLEAN 114、ハンドル/ポインタ 40、HRESULT 15、整数/コード 70、LRESULT 7、COLORREF 4、void 6 + 戻らない 2、ヘッダー外で型不明 39(msacm の MMRESULT 13、propsys・uxtheme・wininet など)。= **約 170(57%)が FALSE/NULL/失敗 HRESULT という通常の失敗値を持つ**ので、『文書で失敗が定義された呼び出しは停止でなく失敗を返す』道が使える。void の 8 は失敗を返せず、呼び出し側が結果を見ない API(多くの描画補助)では失敗が黙って無視される=画面が欠けるだけで気づけない(読み)。

**推奨**: (1) **A を主軸にする**。census が『300 の未実装』と『42 ファミリー』を有限の作業にした。ファミリー順(§8)に、文書から実装し、Wine を差分の基準にする。差分テストは実測で成立した(headless Wine で窓・更新領域・メッセージ列まで比べられる)。(2) **D を安全網として並行**(crash1)。停止を失敗に変えるだけでは UI は壊れたままなので A の代わりにならない。ERROR_CALL_NOT_IMPLEMENTED(120)は System Error Codes にある『この関数はこのシステムでは未対応』で、行なしの呼び出しに返す文書上の失敗値の候補(読み。偽の成功を返さない規則と矛盾しない)。(3) **B・C は採らない**: 598k 行 + server の全体を持ち込むと、検証済みの窓層 15k 行を捨て、wasm64 化(エンジン全体の memory64 化は性能に影響しうる)と server の同一プロセス化という未測定の大工事を負う。(4) 自己完結のライブラリ(ImageList 3,849 行、ACM 7k 行、MIDI 入力の枠組み)は、Wine の C をそのまま runtime に取り込み、下位 API は逆向きの shim(`fle_imp_*` を C から呼ぶ)で受ける B' が安い可能性がある。ImageList の公開 API は整数と不透明ハンドルが中心で構造体の幅の問題が小さい(読み)。**実測(`tools/wine_deps.py`、`tables/wine-deps-imagelist.txt`)**: `comctl32/imagelist.c`(3,849 行)を wasm64 でコンパイルした物の未定義シンボル 57 のうち、Win32 API は 41(runtime に関数がある 36〔impl 4・partial 31・stop 1〕、無い 3〔CopyImage・GetBkColor・GetTextColor〕、census の外 2〔GdiAlphaBlend・GetNearestColor〕)、Wine 内部・libc・GUID が 14、他 2。= **ImageList は Wine の C で動かせるが、下の gdi32(BitBlt・CreateDIBSection・GetDIBits・StretchDIBits・CreateIconIndirect …)の partial が ImageList の使い方に耐えるかが本当の仕事**(下位の穴が移るだけで消えない)。**ファミリー単位に評価**してから採否を決める。

## 8. ファミリー別の着手順の提案(`tables/api-families.tsv`)

(SHIP2 基準の 42 ファミリー、上位 20 は表。『閉包』は FLEngine の閉包に届く数。)

| ファミリー | API | 停止のみ | 閉包 | 見積り(行) | 参考: Wine 10.0 の対応ソース |
|---|---:|---:|---:|---|---|
| comctl32 ImageList | 23 | 0 | 23 | 920〜2,300 | imagelist.c 3,849 |
| user32 メニュー | 14 | 0 | 14 | 560〜1,400 | win32u/menu.c 4,668 |
| gdi32 ベクタ描画 | 13 | 13 | 0 | 520〜1,300 | dibdrv/primitives |
| msacm32 | 13 | 0 | 13 | 520〜1,300 | msacm32/*.c 約 7,200 |
| ole32(COM・ストリーム・ストレージ・D&D) | 13 | 0 | 12 | 520〜1,300 | compobj/stg_stream/storage32/ole2 |
| winmm MIDI | 13 | 0 | 13 | 520〜1,300 | midi.c + winmm.c |
| gdi32 メタファイル | 12 | 10 | 2 | 480〜1,200 | emfdrv.c 581 + |
| kernel32 ファイル | 12 | 4 | 4 | 480〜1,200 | kernelbase/file.c |
| kernel32 同期・メモリ | 12 | 0 | 7 | 480〜1,200 | sync.c, memory.c |
| user32 窓・メッセージ配管 | 12 | 0 | 11 | 480〜1,200 | window.c 6,255 + message.c 4,583 + mdi.c |
| gdi32 フォント・文字 | 11 | 4 | 7 | 440〜1,100 | font.c 7,115 |
| user32 描画補助・アイコン | 11 | 9 | 2 | 440〜1,100 | uitools.c 1,752 + cursoricon.c |
| gdi32 DC・ビットマップ・リージョン | 10 | 6 | 3 | 400〜1,000 | dc.c + region.c 2,685 |
| kernel32 ロケール・時刻 | 10 | 0 | 6 | 400〜1,000 | locale.c |
| user32 タッチ・ジェスチャ・DPI | 10 | 0 | 0 | 400〜1,000 | sysparams.c |
| wininet | 10 | 0 | 10 | 400〜1,000 | http.c 6,407 + internet.c 5,326 |
| user32 キーボード・SendInput | 9 | 0 | 9 | 360〜900 | input.c 2,713 |
| kernel32/ntdll 例外・unwind | 8 | 6 | 1 | 320〜800 | signal_x86_64.c |
| imm32 | 7 | 0 | 0 | 280〜700 | imm.c 3,343 |
| user32 スクロール(ScrollWindow・ScrollDC・Get/SetScrollPos …) | 7 | 0 | 3 | 280〜700 | scroll.c 1,519 |

着手順の提案: (1) **窓の穴の系統**(影響大・クラッシュ報告に直結): スクロール 7、メニュー 14、ダイアログ/MessageBox 4、窓・メッセージ配管 12(合計 37 API、1.5k〜3.7k 行)+ 窓の意味論の欠け(GAP-U1、WM_SIZE/WM_MOVE、SC_*、HTTRANSPARENT。API 数に入らない)。**user32 は crash1 と調整が要る**(PM 指示)。(2) comctl32 ImageList 23(閉包 23。B' の候補)。(3) gdi32: フォント・文字 11、パス 3、DC・ビットマップ・リージョン 10、ベクタ描画 13(停止のみ)、印刷 5。(4) ole32 13(COM・ストリーム)。(5) kernel32 の残り(ファイル 12・同期 12・ロケール 10・プロセス 7・コンソール 7・例外 8)。api2 の 14 API は kernel32/shell32 のこの塊から既に済ませた。(6) msacm32 13・winmm MIDI 13・wininet 10(音声・MIDI・ネットワーク。『偽の成功を返さない』ので多くは文書の失敗値)。全 42 行は `tables/api-families.tsv`(API 名つき)。

## 9. 作業者の選択(出典のない選択。S-api2-N)

- S-api2-1: 停止理由の分類(キーワード規則)。TzSpecific の不正な SYSTEMTIME を ERROR_INVALID_PARAMETER 87(SystemTimeToFileTime の失敗と同じ)にした。代替: 停止(ページが無定義)。
- S-api2-2: LOC の見積り密度(40〜100 コード行/API、観測 48)。GetLargePageMinimum を 0(環境に大ページが無い)にした。代替: 2 MiB(Wine)。
- S-api2-3: VirtualQueryEx と WriteConsoleA/ReadConsoleW の失敗コードを ERROR_INVALID_HANDLE 6(ページはコードを書かない)。
- S-api2-4: GetConsoleCP のコンソール無しを 0 + 6(Wine と一致、UNOFFICIAL)。
- S-api2-5: SHELLEXECUTEINFOW の x64 の配置(ページはメンバーの順を書き、オフセットは書かない。DROPFILES の 20 バイトの配置は照合用ハーネスの側で同じ扱い)。
- S-api2-6/7: (D&D 3 API は api2 から外したが、照合の結果として残す)DragQueryFileW の範囲外の添字は 0、`cch == 0` でバッファありは Wine が必要長を返す(UNOFFICIAL)。crash1 は 0 を返す。
- S-api2-8: GetLocaleInfoA の LOCALE_RETURN_NUMBER の戻り値は 4(Wine、UNOFFICIAL)。
- S-api2-9: WriteConsoleA 失敗時に `*written` を触らない。S-api2-10: GetStringTypeA の NULL 入力は 87(ページ)で、Wine と相違。
- S-api2-11: コンパイルテストで `__x86_64__` を偽装し、winnt.h の x86 inline asm を scratch 木で無効にした(Wine のソースの意味は変えないテスト用の改変。成果物には含めない)。
- S-api2-12: census の『impl』を『到達する fle_stop が無い』と定義した(『完全』と呼ばない)。

## 10. 規則違反の自己申告と確認できなかったこと

- 存在しない bash ツールを 2 回呼んだ(エラーのみ、副作用なし)。
- ロード 35〜55 の中で重い処理を最大 2 並列で走らせた(全体テストの 2 並列 + emcc の連続コンパイル 1 本など。同時に重い物 3 本になった時間帯がある)。`nice -n 15` を使用。作業ディレクトリは最大 約 6 GB(生成 C 約 2 GB、Wine の木 374 MB と build、ReactOS の sparse、全体テストの target)。終了時に削除して 500 MB 以内にした(`logs/disk-after.txt`)。
- 網羅表の URL は mingw のヘッダー名から機械的に作ったもので、866 件中 40 件が 404。
- 確認できなかったこと: §0 の(3)。加えて、Wine のソースの**実行**(コンパイルとリンクだけ)。全体テストは、HEAD 0a3ab9a の木 + api2.diff の途中版(17 API 版)で 176 実行ファイル results=180 passed=2,281 failed=0(入力 DLL をリンクしなかった 11 実行ファイルの SKIPPED 15 は、リンクして再実行し 15 results/212 passed/0 failed/0 skipped)。最終版(14 API)では api2 の 4 件だけを再実行した(`MERGE-NOTES.md`)。a4 は触っていないので再実行していない。

## 11. 成果物

`lift/out-git/api2-results/`: `api2.diff`(新規ファイルのみ)、`MERGE-NOTES.md`、`docs/0210-api-layer-survey.md`(本文)、`docs/README-0210-line.txt`、`tables/`(api-coverage ほか)、`tools/`(census の道具、Wine のコンパイルテスト)、`logs/`。作業: `lift/out-api2/`。

---

## PM の決定と取り込み(2026-11-04)

- **取り込み**: `api2.diff`(20 ファイル、新規のみ、削除 0 行)を当てた。FLEngine の既定の生成 C は `cd9107ce…` のまま。PM 実測: 全体テスト 183 results / 2,307 passed / 0 failed(基準は 180 / 2,287。+api2 + dec1 wall_dec1 7 + act1_net 9)、a4 FRUITY 無効 942/942、895/895、関数 5,310、入口 1,009,731(不変)(dec1・act1 と同時の 1 回)。census の数字(993 distinct、SHIP2 の未実装 300、42 ファミリー)は担当の実測(PE の import と生成 C の静的読み、実エンジン未実行)。
- **方式の判断**: 担当の推奨(A: 仕様から網羅 + D: 停止を soft に、Wine は差分の基準のみ)を採る。B(Wine の C を wasm に)・C(ReactOS)は採らない。依頼者の『個別のケースに個々に対応しているのは不自然だ』への答えとして、次の作業はファミリー単位(ImageList 23、user32 の menu 14、gdi32 の描画 13、msacm32 13、winmm 13、ole32 14…)で、文書(Learn)の挙動と Wine 差分を根拠に進める。ImageList は自己完結のライブラリなので、Wine の `imagelist.c` を取り込む案(B')を評価する価値がある(担当の読み。実行は未確認)。
- **D&D の 3 API は crash1(`fle_crash1_drag.c`)に任せる**(api2 から外してある)。crash1 の実装を担当の Wine 出力つきオラクルに通すと 20 行中 19 行が一致し、違うのは `cch == 0` でバッファありの 1 行(Wine は必要長 12、crash1 は 0)。crash1 に伝える。
- **出荷の行**: `api2-imports.txt` を ship3 の EXTRA_IMPORTS に足すと FLEngine の閉包の『行なし』が 184 から 178 に減る。足してよい(テストは ship3 の 16 スナップで確認)。
- **限界**: 各 API が実エンジンで実際に呼ばれる時機は閉包の上限。impl / partial が文書の挙動を網羅している証明ではない。窓の症状の原因(H1〜H4)のうち GAP-U1 は present2 が runtime に反映する。
