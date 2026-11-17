# 0118: CPUID は x86-64 の基準ビットを答える（決定 0009 の更新）、`GetModuleFileNameW/A(NULL)` はホスト exe `FL64.exe`、`CreateFruityInstance` の壁 7（`rpcrt4!UuidCreate`）と壁 8（`LoadIconW(MAINICON)`、`GetIconInfo`）を直し、壁 9（`SendMessageW(NULL, WM_SETICON, …)`）で停止。すべて仮の決定

状態: **仮の決定（すべての応答と選択）**。依頼は PM（2026-10-30。決定 0118 の内容は PM の仮の決定。番号 0118 は PM の仮の割り当て）。測定・実装・テスト・文書は作業者（サブエージェント）が行った。**作業は HEAD c7d65bf の `git archive` で始め、HEAD は c76a7c3 に進んだ（差は JSPI の測定の文書とスクリプトだけで `lift/runtime`・`lift/crates` は同じ。差分 `fruity5.diff` は HEAD c76a7c3 の clean な archive に当たる）。コンパイル（警告・エラーなし）、全体テスト 1 回の通し（119 test result・1,647 passed・0 failed）、生成 C の関数単位の比較、変異検査 17 件（17 件検出。対照は通る）、FRUITY 無効の独立 2 回と FRUITY 有効の独立 2 回の測定まで実施した。実機 Windows との照合は 0 件。wasm・ブラウザは未測定（native のみ）。**

**測定用の基底・種・註釈（dsp 0x2000000、Blend2D 0x4700000、QFC 0x1d00000、freetype 0x4b00000、ILTools 0x6000000、フォント 2 ファイル、レジストリの種、署名の主張の表）と、仮の応答を含む構成での値である。** 決定 0101 の表記では「A4 完走（仮の応答 N 件を含む）」の形の値で、達成の判定は PM が行う。

**肯定的に見せる点は少ない。** `CreateFruityInstance` は**戻らない**。CPU 検査を通し、その先の 3 つの壁（CPU、`rpcrt4.dll`、`LoadIconW`／`GetIconInfo`）を直したが、**38,913 関数入口目で `user32!SendMessageW(NULL, WM_SETICON, …)` に止まる**（決定 0117 の 38,241 入口目から +672 入口）。メッセージループには入っていない（目的 (a)、(b) のどちらにも到達していない）。この先は、出典のない値の選択（壁 9）と、決定 0071 の仮想ウィンドウ表の範囲外のトップレベルウィンドウ（壁 10。実験で確認）で、後者は停止条件 (iii)（大きなモデル）に当たる。

区分の凡例: **実測**＝コマンドの出力を見た事実。**読み**＝逆アセンブルやコードを読んだ上での解釈。**Learn**＝Microsoft Learn のページに書かれていること（取得日 2026-10-30。タグを除いたテキストを `learn/` に保存）。**一次(ABI)**＝x86-64 psABI のソース（Microsoft ではない）。**一次(Intel)**＝Intel SDM（Intel 公式の PDF）。**出典なしの選択**＝ページにも実測にも根拠が無く、作業者が選んだ値。**未確認**。

## 先に結論

| 項目 | 内容 |
|---|---|
| `CreateFruityInstance`（種あり・主張の表あり） | **戻らない**。最後の停止点: `fle: unsupported instruction at 0x425390`（`user32.dll!SendMessageW` の thunk `f_425390`。呼び元 `f_86b680` ← `f_5c6290` ← `f_5d03e0`（`Application.Icon` の設定）← `CreateFruityInstance`）。**38,913 関数入口目**、新規の関数 379（決定 0117 の停止点 38,241／348 から +672 入口／+31 関数）。引数 `rcx = 0`（hWnd が NULL）、`rdx = 0x80`（WM_SETICON）、`r8 = 1`（ICON_BIG）、`r9 = 0x7fba0624`（`LoadIconW` が返した HICON）。`rax` なし、`instance` = 0 のまま、終了コード 134。独立な 2 回のビルドで同一（native `48a81021c332e2271ac02546e435c97240a7eae26a4437f03597a87c7aea4259`。`fruity:` の出力は経過時間 0.06 s／0.05 s の 1 行だけが違う） |
| 直した壁 | **3**: 壁 6（CPU 検査: CPUID の基準ビット）、壁 7（`rpcrt4.dll` と `UuidCreate`）、壁 8（`LoadIconW(0x400000, "MAINICON")` と `GetIconInfo`）。壁 9 で停止（出典なしの選択と、その先の大きなモデル）。壁の通し番号は決定 0117 の続き |
| CPUID（決定 1） | leaf 1 EDX = FPU(0)・CX8(8)・CMOV(15)・MMX(23)・FXSR(24)・SSE(25)・SSE2(26) = **0x07808101**（決定 0009 は SSE|SSE2 = 0x06000000）。ECX = 0、TSC・SEP は 0 のまま。エンジンの CPU 検査（`f_13755b0`）を通り、`GetActiveWindow` と「This computer's CPU won't run FL Studio」は消えた（実測）。**a4（FRUITY 無効）の +2 の原因は FXSR（EDX bit 24）の 1 ビットだけ**（1 ビットずつ足して実測。他の 4 ビットは入口列がバイト単位で同一）。2 つの新しい入口は `f_6399d0`（FLEngine）と `f_619b390`（ILTools）で、どちらも `mov al,1; ret`（FXSR の枝の内側。想定どおり） |
| `GetModuleFileNameW/A(NULL)`（決定 2） | 旧: DLL のパス（決定 0018）→ 新: `C:\Program Files\Image-Line\FL Studio 21\FL64.exe`。`0x400000` は DLL のパスのまま、`GetModuleHandleW(NULL)` は 0x400000 のまま。エンジンの 2 回目の署名検査（ホスト exe）が `authenticode-claims.tsv` の FL64.exe の行に当たった（gdb で `WinVerifyTrust` の 4 回の呼び出しのパスを確認）。決定 0105 のフック有効化は FL64.exe でも有効のまま（実測） |
| 仮想時計（決定 3） | **直さない**。時計は実機の結果の再現ではない。チェーン・時刻を検査する段階で再検討（決定 0117 の限界のまま） |
| FRUITY 無効の新基準 | a4: 942/942、895/895、`returned from entry`、終了 0 は不変。**functions 5,306 → 5,308（+2。CPUID の FXSR）、entries 1,007,942 → 1,007,947（+5 = CPUID +2、exe パス +3）**。native `ab6c6be63c255daf24e884efaed1ac8f3e200e098191985a8d162e5f5f38f187` → **`27cdf764f10ffabc08bba2d4afb45815441cdb4e35c05221b102ce5908d23f6d`**（独立な 2 回で同一。別の OUT・別の cargo target） |
| exe パスの +3 | 原因は「パスの長さ」: FL64.exe のパス（49 文字）が DLL のパス（57 文字）より短く、Delphi のメモリマネージャのブロック管理の補助（`f_407520`、`f_407570`。呼び出し回数 3→4、31→33）が別の経路を通る。FL64.exe のパスを DLL と同じ 57 文字にした実験では入口列が決定 1 だけの実行と**バイト単位で同一**に戻る（不具合ではない） |
| 全体テスト | `cargo test --workspace --no-fail-fast --offline` を **1 回の通し**（40 分 16 秒、2026-10-30T03:20:09Z〜04:00:25Z、終了 0）で **119 test result・1,647 passed・0 failed・ignored 0**（基準 118・1,632 + 新規 15 件）。ただし通しの後に 1 件のテスト（`rpcrt4_is_no_longer_in_the_list…`）とコメントだけの変更を足した（「検証」の節） |
| 生成 C | FLEngine の生成 C（註釈あり）の SHA-256 `660b4335…`（HEAD。決定 0117 の値）→ `e7669cdc2b80f782f4c9fb3741626fab14b1ae6402580d5fdb3af812d1ccbb7d`。**関数 59,380 個のうち本体が変わったものは 0**（5 モジュールも 0）。変わるのは `fle_imp_UuidCreate` の宣言・dispatch 表・応答表の各 1 行と個数の定数（`fle_dispatch_count` 59,825 → 59,826、`fle_resp_count` 445 → 446）だけ。runtime のオブジェクトは 50 個中 7 個（`fle_cpuid`、`fle_gdi`、`fle_load`、`fle_modules`、`fle_proc`、`fle_res`、`fle_vfs`） |
| wasm | runtime が変わるので、**wasm の SHA（`docs/investigations/a4-six-module-wasm*`）は変わる**。再ビルドは別の作業（この作業では native のみ） |

## 背景

- 前段（決定 0117、`docs/investigations/fruity4/`）: `CreateFruityInstance(0xC0, 0xEDE95A, 偽ホスト, &instance)` は 38,241 関数入口目の `user32!GetActiveWindow`（`MessageBoxW` の所有者探し）で止まった。原因はエンジンの CPU 検査で、`f_639a20` が `CPUID` leaf 1 の EDX から作る機能語の bit 0x10（EDX bit 15 = CMOV）が、決定 0009 の答え（SSE と SSE2 の 2 ビットだけ）では立たないこと。選択肢 A1（CMOV だけ）、A2（x86-64 の基準）、A3（FRUITY のときだけ）を PM に挙げ、実験 E1〜E4 で A1 と A2 の先を見た。
- PM の仮の決定（0118、変えない）: (1) CPUID は x86-64 の基準を答える（A2）。FPU・CX8・CMOV・MMX・FXSR を足し、AVX／SSE3 以降は決定 0075／0032 のまま名乗らない。a4 の +2 の原因を特定する。(2) `GetModuleFileNameW/A(NULL)` はホスト exe FL64.exe。(3) 仮想時計と署名者証明書の有効期間の食い違いは今は直さない。(4) CPU 検査を通した後の壁を最大 12 壁まで連続して進め、(a) `CreateFruityInstance` が戻る、または (b) エンジンが初めてブロッキングのメッセージループに入るまで。Learn から導ける応答だけを実装し、導けない値は出典なしの選択として列挙。「検証成功」を偽らないと進めない壁、PM の未決の値、大きなモデル、設計の大きい lift の穴では止める。(5) FRUITY 無効の既定ビルドの挙動は CPUID と `GetModuleFileNameW(NULL)` の変更を除いて変えない。
- 方針（PM、前の決定から継続）: ふり（fail-soft）で進めない。応答表は末尾追記のみ（決定 0015）。新しい `.c`／`.h` を runtime に作らない（`tests/modules.rs` が全 `fle_*.c` をリンクする）。
- 土台（実測。HEAD c7d65bf の `git archive`）: FRUITY 無効の基準を再現した（TAG `t2331d00e8ebb`、a4 942/942、895/895、functions 5,306、entries 1,007,942、native `ab6c6be6…f187`）。FRUITY 有効（種 = nsis + app、主張の表あり）で決定 0117 の停止（`unsupported instruction at 0x424b60`、38,241 入口、新規の関数 348、native `15ab0a8e…8845`）を再現した。

## 決定（仮）

旧 → 新の表（既存の応答を変えるもの。撤回条件は「撤回条件」の節）:

| # | 場所 | 旧 | 新 | 根拠 |
|---|---|---|---|---|
| D1 | `runtime/fle_cpuid.c` の leaf 1 EDX（決定 0009 を更新） | bit 25（SSE）、bit 26（SSE2）= 0x06000000 | bit 0（FPU）、8（CX8 = CMPXCHG8B）、15（CMOV）、23（MMX）、24（FXSR）、25、26 = **0x07808101**。ECX・EBX・EAX・ベンダー文字列・拡張葉は不変。leaf 2〜0x7fffffff は leaf 1 のデータ（決定 0009 の規則）を返すので同じ EDX | **一次(ABI)**: x86-64 psABI `low-level-sys-info.tex`（取得 2026-10-30。保存した原文の SHA-256 `2d42f2ab…b705`）「Any program can expect that an x86-64 processor implements the baseline features mentioned in table」、表は CMOV・CX8・FPU・FXSR・MMX・OSFXSR・SCE・SSE・SSE2、「Most feature names correspond to CPUID bits」「Exceptions are OSFXSR and SCE」。**一次(Intel)**: Intel SDM Vol. 1 Table 21-10（`Leaf 01H Version and Features Returned in EDX Register`。取得した PDF の SHA-256 `a4a62e6a…f321`、表紙「Order Number: 325462-093US September 2026」）: EDX[0] FPU、EDX[8] CMPXCHG8B、EDX[15] CMOV、EDX[23] MMX、EDX[24] FXSR、EDX[25] SSE、EDX[26] SSE2（「If 1, supports …」）。TSC（bit 4）・SEP（bit 11）は基準の表にないので 0 のまま。AVX／SSE3 以降は名乗らない（決定 0075／0032、lifter に VEX が無い） |
| D2 | `GetModuleFileNameW/A(NULL)`（`runtime/fle_vfs.c` の `module_path`、`runtime/fle_env.h` に `FLE_VIRTUAL_EXE_PATH`） | NULL も `0x400000` も `C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.dll`（決定 0018） | **NULL → `C:\Program Files\Image-Line\FL Studio 21\FL64.exe`**（49 文字）。`0x400000` → DLL のパス（57 文字。不変）。`GetModuleHandleW(NULL)` は 0x400000 のまま（ホストの像を作らない） | **Learn**（GetModuleFileNameW、取得 2026-10-30）: 「If this parameter is NULL, GetModuleFileName retrieves the path of the executable file of the current process.」。ホストは FL64.exe（`docs/investigations/host-engine-boundary.md`）。値は PM の値（ディレクトリは決定 0018、ファイル名は決定 0117 の署名の表・レジストリの種の FL64.exe と同じ文字列）で、Windows の規則ではない |
| D3 | 仮想時計 | 2021-01-01（決定 0018） | **不変**。署名者証明書の notBefore（2026-07-28）より前。時計は実機の結果の再現ではない。チェーン・時刻を検査する段階で再検討 | PM の決定 3 |

新しい応答（これまで停止または未応答だったもの）:

| # | 場所 | 内容 |
|---|---|---|
| D4 | `runtime/fle_modules.c`、`fle_load.c` | `rpcrt4.dll` を仮想モジュール N = 36（末尾追記。ハンドル `0x7FF800240000`）にし、`fle_load_absent[]` から外す（8 → 7 名）。決定 0099 の `shcore.dll` と同じ形 |
| D5 | `runtime/fle_proc.c`／`.h`、応答表の末尾 1 行 | `rpcrt4.dll!UuidCreate`: RFC 4122 §4.4 の version 4 の UUID を、固定 seed `0x5555555555555555` の SplitMix64 から作る。戻り値 `RPC_S_OK`（0）。n 回目の呼び出しは n 番目の値（毎回の実行で同じ）。NULL ポインタは名前付き停止 |
| D6 | `runtime/fle_res.c`／`.h`（`fle_icon_group_pick`）、`runtime/fle_gdi.c` | `LoadIconW(hInstance = 0x400000 または実在モジュール, name)`: そのイメージの `RT_GROUP_ICON`（整数 id か文字列）から LookupIconIdFromDirectoryEx の規則で 1 エントリを選び、共有アイコンのハンドル（GDI 表の `FLE_GDI_KIND_ICON`、フラグ `GF_MODICON`）を返す。同じグループには同じハンドル。`GetIconInfo` はこの共有アイコンに限り答える（fIcon TRUE、ホットスポット = 中心、新しい 2 つのビットマップ） |
| D7 | `runtime/fle_sysinfo.c` | **コメントだけ**（オブジェクトは同一）。PF_COMPARE_EXCHANGE_DOUBLE（2）と PF_MMX_INSTRUCTIONS_AVAILABLE（3）は 0 のまま。CPUID が CX8・MMX を報告するのに 0 を答える（決定 0078 の「CPUID が報告しない機能はここでも報告しない」は一方向だけ）。PM の未決点 |

## 壁の表

各壁の記録は `wall-log.md`。「種類」は 未応答 import／名前付き停止／lift の穴／環境の主張／大きなモデル／メッセージループ。

| # | 停止メッセージ | 原因（区分） | 種類 | 修正（場所） | 測定（入口数／新規の関数） |
|---|---|---|---|---|---|
| 6 | `fle: unsupported instruction at 0x424b60`（`user32.dll!GetActiveWindow`） | エンジンの CPU 検査の失敗（決定 0117。`f_639a20` が EDX bit 15 から機能語 0x10 を作る。実測 + 読み） | 環境の主張 | D1 | 38,241／348 → **38,166／350**（次の停止: `RaiseException 0xc06d007e`） |
| 7 | `fle: RaiseException: exception code 0xc06d007e`（遅延ロードの `LoadLibraryA("rpcrt4.dll")` が NULL） | `f_11b2450` が `GetTempPathW`、`GetCurrentProcessId` の後に `rpcrt4!UuidCreate` を遅延ロードで呼ぶ（一時フォルダ名 `%d-%s` を作る。読み）。`rpcrt4.dll` が `fle_load_absent[]` にあった | 未応答 import（遅延ロード） | D4、D5 | 38,163／350（決定 2 を含む）→ **38,873／372**（次の停止: `LoadIconW`） |
| 8 | `fle: LoadIconW: hInstance is not NULL: icon resources of a module are not implemented (only IDI_APPLICATION is)` | `CreateFruityInstance` 自身の 0x13764eb が `LoadIconW([0x15ec298] = 0x400000, L"MAINICON")`。返った HICON を `f_5d03e0`（`Application.Icon`）が `GetIconInfo`、`GetObjectW(hbmColor)`、`DeleteObject` ×2 に使う（読み + 実測）。エンジンのイメージに `RT_GROUP_ICON` `MAINICON`（13 エントリ）が実在（pefile） | 未応答 import（資源から導ける） | D6 | 38,873／372 → **38,913／379**（次の停止: `SendMessageW`） |
| 9 | `fle: unsupported instruction at 0x425390`（`user32.dll!SendMessageW`） | `f_86b680`（`TApplication` のアイコン更新）が `SendMessageW([self+0x2b0] = **NULL**, WM_SETICON, ICON_BIG, hIcon)`、続けて `SetClassLongPtrW(NULL, GCLP_HICON, hIcon)`。`TApplication` のウィンドウ（`CreateWindowExW` を呼ぶ `f_867e10`）がまだ作られていない | **環境の主張・出典なしの選択（停止条件 (ii)）** | なし（**ここで止めた**） | 38,913／379 で停止 |
| 10（実験で確認。実装していない） | `fle: CreateWindowExW: dwStyle 0x84ca0000 is not WS_POPUP (0x80000000): only the hidden tool window of the guest is answered (decision 0071)` | `f_867e10` が `CreateWindowExW(WS_EX_TOOLWINDOW, "TApplication", "FL Studio", 0x84ca0000, X = 画面幅/2, Y = 画面高/2, 幅 0, 高さ 0, …)`。決定 0071 の「隠れた WS_POPUP の道具ウィンドウ」の範囲外 | **大きなモデル（停止条件 (iii)）** | なし | 実験 E5 で 38,928／384 |

壁の種類の件数（この作業の壁 6〜9）: 環境の主張 2（壁 6、9）、未応答 import 2（壁 7、8）、名前付き停止 0、lift の穴 0、メッセージループ 0。決定 0116〜0117 の壁 1〜5 と合わせて、壁 9 つ中 未応答 import 5、環境の主張 3、仕様に書かれていない値 1。

## 止まった壁（壁 9）の内容・選択肢・推奨

**実測**: 停止点は `SendMessageW`（38,913 入口、`logs/fin1/`）。引数は `hWnd = 0`（NULL）。シャドウスタックは `f_425390` ← `f_86b680` ← `f_5c6290` ← `f_5d03e0` ← `CreateFruityInstance`。C スタック（`logs/fin1/c-stack-symbolised.txt`）: `fle_unsupported+0x45` ← `f_425390+0x1e` ← `f_86b680+0x241` ← `fle_dispatch+0x128` ← `f_5c6290+0x19e` ← `fle_dispatch+0x128` ← `f_5d03e0+0x704` ← `f_1375990（CreateFruityInstance）+0x54be` ← `fle_dispatch+0x110` ← `fle_call_guest+0x169` ← `fruity_run+0x535` ← `main+0x1b3b`。

**Learn が書いていないこと**（取得 2026-10-30、`learn/sendmessagew.txt`、`wm_seticon.txt`、`setclasslongptrw.txt`、`syserr-1400.txt`）: SendMessageW の hWnd は「A handle to the window whose window procedure will receive the message」とだけあり、NULL の結果が書かれていない。WM_SETICON の戻り値は「a handle to the previous large or small icon … NULL if the window previously had no icon」。SetClassLongPtrW も NULL の hWnd を書かず、失敗は「0 と GetLastError」。`ERROR_INVALID_WINDOW_HANDLE` = 1400（System Error Codes の表）はどの API が返すかが書かれていない。

**選択肢**

| 案 | 内容 | 結果（実測／読み） | 注意 |
|---|---|---|---|
| X1. 止まる（推奨。この作業の選択） | 何も足さない | 38,913／379 で停止。実装は出典なしの値を増やさない | 到達点は動かない |
| X2. NULL の hWnd の `SendMessageW`／`SetClassLongPtrW` に 0 と last error 1400 を答える | **実験 E5**（`docs/investigations/fruity5/experiment-e5.diff`。差分に入れない） | 次は壁 10（`CreateWindowExW` の dwStyle 0x84ca0000、38,928／384）。0 は失敗なので「成功を偽る」ことにはならないが、0 と 1400 は**出典なしの選択** | 壁 10 は (iii) なので、(a)(b) には近づかない |
| X3. X2 + 仮想ウィンドウ表の拡張（トップレベル・可視・キャプション付き、`WM_NCCALCSIZE` の `NCCALCSIZE_PARAMS`、`WM_GETMINMAXINFO`、`WM_SIZE`、…） | 決定 0071 の拡張。メッセージキューが要るかは未調査 | 未測定 | 大きなモデル。**PM の未決**（決定 0071 の範囲の改訂） |

**推奨**: X1（止まる）。理由: X2 の 2 つの値は出典がなく、実装しても壁 10 が (iii) で必ず止まる。X3 に進む場合は、PM がウィンドウ表の範囲（可視性、フレーム、メッセージの流れ、メッセージキュー）を決めてから。

## a4 の +2 の原因の特定（実測）

方法: 計測専用の木（`fle_cpuid.c` に環境変数 `FLE_EXP_EDX` で leaf 1 EDX を上書きする口、`trace_native.c` に全関数入口（VA と戻り先）を書き出す口。再現用の差分 `docs/investigations/fruity5/experiment-entry-dump.diff`。**差分に入れない**）で、FRUITY 無効の a4 を EDX を変えて実行し、入口列をバイト単位で比べた。

| 実行 | EDX | functions／entries | 入口列（基準との比較） |
|---|---|---|---|
| 基準（HEAD の EDX） | 0x06000000 | 5,306／1,007,942 | — |
| + FPU（bit 0） | 0x06000001 | 5,306／1,007,942 | **同一** |
| + CX8（bit 8） | 0x06000100 | 5,306／1,007,942 | **同一** |
| + CMOV（bit 15） | 0x06008000 | 5,306／1,007,942 | **同一** |
| + MMX（bit 23） | 0x06800000 | 5,306／1,007,942 | **同一** |
| + FXSR（bit 24） | 0x07000000 | **5,308／1,007,944** | 差あり |
| 5 ビットすべて（実験 E4） | 0x07808101 | **5,308／1,007,944** | 差あり。**+ FXSR の実行と入口列が同一** |

入口列のダンプの SHA-256 は `logs/entries-dumps.sha256`（各 16 MB。成果物には入れない）。差分は入口列の挿入 2 か所、各 1 入口: (1) `f_639a20`（CPU 情報の単位。InitUnits の中。gdb で `f_63a5e0` → `f_63a330` → `f_639a20` を確認）の 0x639b4c の `call 0x6399d0`。`f_639a20` は EDX bit 24（`test rax,0x1000000`、0x639b3d）が立っていると機能語に 0x40 を足し、`0x6399d0` を呼び、真ならさらに 0x20000 を足す。`0x6399d0` は `mov al,1; ret`（読み）。(2) **ILTools の同じ単位**（`ILTools_x64.6000000.dll` の 0x619b50c の `call 0x619b390`。同じく `mov al,1; ret`）。したがって 2 つの関数は `f_6399d0` と `f_619b390`（名前は未確認）。**想定どおり（エンジンが CPU 機能の分岐で別の経路に入った）**で、名前付き停止の迂回、誤った lift、偽の成功の兆候ではない。a4 の終端は変わらない。**機能語の bit 0x20000 が後で何に効くかは追跡していない（未確認）**。

**新しく有効になった機能の命令（実測 + 読み）**: FXSR の枝の呼び先は `mov al,1; ret` だけで、**FXSAVE／FXRSTOR 命令は a4 では実行されない**。静的な所在（capstone で `.text` を走査）: FLEngine に `fxsave` 2（0xadf5d9、0xadf809）、`cmpxchg8b` 1（0x4ec644）、`emms` 9、`femms` 8。dsp_ippv2 に `fxsave` 4（0x37e0531、0x37e0da1、0x3b317a1、0x3cba461）。生成器は dsp の 1 件を `Fxsave: mnemonic outside minimal set`（`gen-m1.txt`）と報告する（その関数 0x37e0514 は直接 call も rip 相対の参照も無く `.pdata` の項目だけ。IPP の CPU 振り分けが呼ぶ候補と読むが、a4 では実行されない）。FLEngine の `fxsave` 2 件と `cmpxchg8b` は `gen-flengine.txt` に未対応として出ていない（閉包の外か到達しないか: **未確認**）。FXSAVE が必要になるのは dsp の関数が実際に呼ばれる段階で、その関数は未対応（`fle_unsupported`）なので**名前付き停止になり、偽の成功にはならない**。FXSAVE／FXRSTOR の lift は仕様から導ける実装だが、まだ呼ばれていないので**この作業では実装しない**。MMX の `emms`／`femms` は決定 0028 で lift 済み。

## 実装した場所

- 新規: `lift/crates/fe-lift/tests/wall_fr5.rs`（14 件）、`docs/decisions/0118-cpuid-baseline-and-fruity-walls.md`。runtime に新しい `.c`／`.h` は作っていない。
- 変更: `lift/runtime/fle_cpuid.c`（D1。定数 7 個と、コメントの根拠）、`fle_env.h`（`FLE_VIRTUAL_EXE_PATH`）、`fle_vfs.c`（`module_path`）、`fle_modules.c`（`rpcrt4.dll` を末尾に）、`fle_load.c`（absent から外す）、`fle_proc.c`／`.h`（`fle_imp_UuidCreate`）、`fle_res.c`／`.h`（`fle_icon_group_pick`）、`fle_gdi.c`（`LoadIconW` の hInstance 非 NULL の形、`GetIconInfo` の共有アイコン、`gdi_obj` に `key`、`GF_MODICON`、`#include "fle_sysmetrics.h"`）、`fle_sysinfo.c`（コメントだけ）、`lift/crates/fe-lift/src/imports_table.rs`（末尾に 1 行）。
- 既存の関数の本体で変えたもの（指示により理由を記す）: `fle_cpuid`（定数だけ）、`module_path`（`h == 0` の分岐を分けた）、`fle_imp_LoadIconW`（`R[1] != 0` の停止を `load_icon_of_module` に置き換え。NULL の形は不変）、`fle_imp_GetIconInfo`（共有アイコンの許可。`FLE_GDI_KIND_ICONCUR` の分岐は不変）、`fle_load.c` の absent の一覧（`rpcrt4.dll` を除く）。それ以外の既存の関数は変えていない。
- 既存のテストの更新（期待値の更新漏れ。理由を記す）:
  - CPUID の値（D1）: `cpuid.rs`（`CASES` の EDX を 0x07808101 に。2 件追加）、`k32_dsp.rs`（`is_processor_feature_present_agrees_with_the_cpuid_answer`: CPUID の行を MMX 1・CX8 1 に、FPU・CMOV・FXSR の行と leaf 7 の値を追加。PF 2・3 は 0 のまま = D7）、`wall_fr4.rs`（`the_emulated_cpu_claims_sse_and_sse2_only…` を `…claims_the_x86_64_baseline…` に置き換え）。
  - exe のパス（D2）: `vfs.rs`（`module_file_name_is_the_virtual_dll_path` → `…exe_path_for_null_and_the_dll_path_for_0x400000`。バッファの規則を exe のパスの長さ 49 で書き直し）、`wall_b.rs`（`get_module_file_name_a_…`）。
  - 仮想モジュール表が 35 → 36 になった（D4）: `deferred.rs`、`load.rs`、`modules.rs`、`msctf.rs`、`wall_s.rs`（「テーブルの終端」の位置を 1 つずらし、`rpcrt4` を absent の例から外し、`the_eight_lacking_names…` を `the_seven_lacking_names…` に）。
  - 応答表の「最後の 9 行」の位置（D5）: `wall_fr4.rs`（`the_nine_new_rows…` に、その後ろに決定 0118 の行が 1 つある分を足した）。
- 既定の挙動: FRUITY 無効の既定ビルドの挙動は、CPUID（D1）、`GetModuleFileNameW/A(NULL)`（D2）、`rpcrt4.dll` の仮想モジュール化（D4。FRUITY 無効の a4 は `rpcrt4.dll` を読み込まない）、`LoadIconW`／`GetIconInfo`（D6）の変更を除いて変わらない（実測: a4 942/942、895/895、`returned from entry`、終了 0）。

## FRUITY 無効の既定ビルドの native の SHA-256（関数単位の理由）

基準 `ab6c6be63c255daf24e884efaed1ac8f3e200e098191985a8d162e5f5f38f187`（HEAD c7d65bf 以降、FRUITY 無効）→ **`27cdf764f10ffabc08bba2d4afb45815441cdb4e35c05221b102ce5908d23f6d`**（この差分、FRUITY 無効）。a4 の数字: 942/942、895/895、functions **5,308**、entries **1,007,947**、`returned from entry`、終了 0。理由（実測）:

1. **FLEngine の生成 C**（SHA-256 `660b4335…` → `e7669cdc…ccbb7d`）: 関数 59,380 個のうち**本体が変わったものは 0**。5 モジュールの生成 C（m1〜m5）も 0 関数（`logs/gencmp-functions.txt`）。各ファイルの差は 7 行: `void fle_imp_UuidCreate(void);` の宣言、dispatch 表の 1 行、応答表の 1 行、`fle_dispatch_count`（59,825 → 59,826）、`fle_resp_count`（445 → 446）。`unsupported_total` は 379 のまま。
2. **runtime のオブジェクト**: 50 個（`fle_*.c` 全部 + `image`、`a4`、`main_two`（`-DFLE_FRUITY` なし）、`trace_native`）を 2 つの木で同じフラグでコンパイルして比べると、違うのは **`fle_cpuid.o`、`fle_gdi.o`、`fle_load.o`、`fle_modules.o`、`fle_proc.o`、`fle_res.o`、`fle_vfs.o` の 7 つ**だけ（`logs/gencmp-runtime-objects.txt`）。`fle_sysinfo.o`（コメントだけ）は同一。
3. `common.c`: 応答表と dispatch 表に 1 行ずつ、個数の定数。
4. 入口数の内訳: 5,306／1,007,942 →（D1）5,308／1,007,944 →（D2）5,308／1,007,947（`logs/c1off`、`logs/c2off`。D2 だけの差の 3 入口は `f_407520`、`f_407570` の呼び出し回数の増加で、FL64.exe のパスの長さが 49 文字であることによる）。

## Learn／一次資料の取得記録（2026-10-30。取得は `urllib`／webfetch で、タグを除いたテキスト。原文は `learn/` に保存し、成果物には入れない: SDM の PDF は 26 MB）

| ページ | URL | 取得した文言（原文の抜粋） | 使った箇所 |
|---|---|---|---|
| x86-64 psABI `low-level-sys-info.tex` | https://gitlab.com/x86-psABIs/x86-64-ABI/-/raw/master/x86-64-ABI/low-level-sys-info.tex（SHA-256 `2d42f2ab76b99a3b8456e6bc07e18314e85991119f72db90ca1175328e27b705`） | "Any program can expect that an x86-64 processor implements the baseline features mentioned in table~\ref{features}. Most feature names correspond to CPUID bits, as described in the processor manual. Exceptions are OSFXSR and SCE, which are controlled by bits in the cr4 register and the IA32_EFER MSR." 表: CMOV、CX8、FPU、FXSR、MMX、OSFXSR、SCE、SSE、SSE2（baseline）。x86-64-v2 に CMPXCHG16B・LAHF-SAHF・POPCNT・SSE3・SSE4_1・SSE4_2・SSSE3 | D1 |
| Intel SDM（結合版） | https://cdrdv2.intel.com/v1/dl/getContent/671200（PDF、26,664,929 バイト、SHA-256 `a4a62e6ba7ba11a76c7753a195b825087306812aac39b930973f9168ee599f321`。表紙「Order Number: 325462-093US September 2026」） | Vol. 1 Table 21-10 「Leaf 01H Version and Features Returned in EDX」: "EDX[0] FPU Floating Point Unit On-Chip. The processor contains an x87 FPU." "EDX[8] CMPXCHG8B If 1, supports the CMPXCHG8B (64 bits) Instruction, implicitly locked and atomic." "EDX[15] CMOV If 1, supports the Conditional Move Instructions. If CPUID.01H:EDX.FPU[0] (x87 FPU present) is 1 also, supports the FCOMI and FCMOV instructions." "EDX[23] MMX If 1, supports the Intel MMX Technology." "EDX[24] FXSR If 1, supports the FXSAVE and FXRSTOR Instructions …" "EDX[25] SSE" "EDX[26] SSE2"。補助: felixcloutier.com/x86/cpuid（非公式の転載。決定 0009 の根拠。Table 3-11 の同じ表） | D1 |
| GetModuleFileNameW | https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulefilenamew | "If this parameter is NULL, GetModuleFileName retrieves the path of the executable file of the current process." 戻り値・バッファの規則は決定 0018／0102 の実装と同じ（truncation、`ERROR_INSUFFICIENT_BUFFER`） | D2 |
| UuidCreate | https://learn.microsoft.com/en-us/windows/win32/api/rpcdce/nf-rpcdce-uuidcreate | "The UuidCreate function creates a new UUID." "Uuid: Returns a pointer to the created UUID." 戻り値 RPC_S_OK／RPC_S_UUID_LOCAL_ONLY／RPC_S_UUID_NO_ADDRESS。"generates a UUID that cannot be traced to the ethernet address of the computer" アルゴリズムとバージョンは**書かれていない** | D5 |
| GUID | https://learn.microsoft.com/en-us/windows/win32/api/guiddef/ns-guiddef-guid | `Data1`（unsigned long）、`Data2`、`Data3`（unsigned short）、`Data4[8]` | D5 |
| RFC 4122 §4.1.3、§4.4 | https://www.rfc-editor.org/rfc/rfc4122.txt | "Set the two most significant bits (bits 6 and 7) of the clock_seq_hi_and_reserved to zero and one, respectively." "Set the four most significant bits (bits 12 through 15) of the time_hi_and_version field to the 4-bit version number [0100]." "Set all the other bits to randomly (or pseudo-randomly) chosen values." | D5 |
| LoadIconW | https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-loadiconw | "If hInstance is non-NULL, lpIconName specifies the icon resource either by name or ordinal." "LoadIcon loads the icon resource only if it has not been loaded; otherwise, it retrieves a handle to the existing resource. The function searches the icon resource for the icon most appropriate for the current display." "LoadIcon can only load an icon whose size conforms to the SM_CXICON and SM_CYICON system metric values." 失敗は NULL と GetLastError（コードは書かれていない） | D6 |
| LookupIconIdFromDirectoryEx | https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-lookupiconidfromdirectoryex | "The image closest but not exceed the requested size is selected. If two or more images of that size are present, the one that matches the color depth of the display is chosen. If no images exactly match the color depth of the display, the image with the greatest color depth that does not exceed the color depth of the display is chosen. If all exceed the color depth, the one with the lowest color depth is chosen." | D6 |
| NEWHEADER、RESDIR、ICONRESDIR | https://learn.microsoft.com/en-us/windows/win32/menurc/newheader など | NEWHEADER（Reserved、ResType RES_ICON = 1、ResCount）、RESDIR（ICONRESDIR + Planes、BitCount、BytesInRes、IconCursorId）、ICONRESDIR（Width、Height: "The value 0 is accepted as representing a width of 256"、ColorCount、reserved） | D6 |
| GetIconInfo、ICONINFO | https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-geticoninfo など | "GetIconInfo creates bitmaps for the hbmMask and hbmColor or members of ICONINFO. The calling application must manage these bitmaps and delete them with DeleteObject call". "If this structure defines an icon, the hot spot is always in the center of the icon, and this member is ignored." "For color icons, the hbmMask and hbmColor bitmaps are the same size, each of which is the size of the icon." "For monochrome icons, the hbmMask is twice the height of the icon … and hbmColor is NULL." | D6 |
| SendMessageW、WM_SETICON、SetClassLongPtrW、System Error Codes（1300–1699） | https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendmessagew など | 壁 9 の節: NULL の hWnd の結果は**書かれていない**。ERROR_INVALID_WINDOW_HANDLE 1400 "Invalid window handle." | 壁 9（実装していない。実験 E5） |

Learn が**書いていない**こと（出典なしの選択の根拠）: UuidCreate のバージョンとアルゴリズム、LoadIconW の失敗時の last error、アイコンのホットスポットの整数値と丸め、複数エントリの「近さ」の測り方（非正方形の要求）、NULL の hWnd への SendMessageW の結果。

## 出典なしの選択の一覧

| # | 場所 | 選択 | 区分 |
|---|---|---|---|
| S1 | D1 | TSC（EDX bit 4）、SEP（bit 11）は 0 のまま（psABI の基準の表にない）。leaf 2〜0x7fffffff は leaf 1 のデータ（決定 0009 の規則）なので、すべての葉の EDX が 0x07808101。ベンダー・版・ブランド（EAX・EBX）は 0 のまま | 作業者の選択（psABI の基準に限る方針に従う） |
| S2 | D2 | exe のパスの文字列 `C:\Program Files\Image-Line\FL Studio 21\FL64.exe` | PM の値 + 設計（決定 0018 のディレクトリと決定 0117 の署名の表・レジストリの種の文字列と同じ） |
| S3 | D4 | `rpcrt4.dll` の N = 36、ハンドル `0x7FF800240000`（式は決定 0015 のとおり） | 設計（決定 0099 の先例） |
| S4 | D5 | UUID の version 4 という形式（Learn はバージョンもアルゴリズムも書かない）、PRNG = SplitMix64、seed `0x5555555555555555`、出力 2 個を下位から 8 バイトずつ並べる、戻り値 `RPC_S_OK`（`RPC_S_UUID_LOCAL_ONLY` にしない）、呼び出しごとに別の値、ホストの乱数・時計・アドレスを使わない | 出典なしの選択（PM の指示「決定的な擬似乱数（固定 seed）で v4 形式」に従う。値は環境の主張） |
| S5 | D6 | 要求サイズ = SM_CXICON × SM_CYICON = 32 × 32（Learn "About Icons"。決定 0057）、表示の色深度 = `FLE_GDC_BITSPIXEL` = 32（決定 0053） | 既存の環境の値 |
| S6 | D6 | 「closest but not exceed」の測り方 = 幅と高さがともに要求以下のものの面積の最大（非正方形の要求の扱いは Learn に無い。エンジンのエントリは正方形）。深度は `Planes × BitCount`（Planes ≠ 1 は停止） | 出典なしの選択（読み） |
| S7 | D6 | グループが無い名前、全エントリが要求サイズを超える、同サイズ・同深度が 2 つ、PNG 圧縮の RT_ICON、BITMAPINFOHEADER と RESDIR の不一致: **停止**（NULL や last error を創作しない） | 設計 |
| S8 | D6 | `GetIconInfo` のホットスポット = (幅/2, 高さ/2)（Learn は「アイコンの中心」、値は書かない。「アイコンでは無視される」とも書く）、`hbmColor` = 仮想ディスプレイの形式（32 bit、1 plane）、新しいビットマップを毎回作る | 出典なしの選択（読み） |
| S9 | D6 | 共有アイコンのハンドル値 = GDI 表の `0x7FBA0000 + 4 × (slot + 1)`（決定 0054）。同じイメージの同じグループには同じ値 | 設計 |
| S10 | D7 | `IsProcessorFeaturePresent(2)`・`(3)` は 0 のまま（CPUID は CX8・MMX を報告する） | PM の未決点（下） |

## テスト

新規 `tests/wall_fr5.rs`（14 件）と、更新（上の「実装した場所」）:

- 生成器: `UuidCreate` の行が末尾の 1 行で擬似 VA が詰まっていること、thunk が rpcrt4 の下でだけ答えられること（別の DLL の同名と `UuidCreateSequential` は未対応のまま）。
- UuidCreate: 最初の 6 個の UUID を、このテストファイルに書いた独立な SplitMix64 の実装（`expected_uuid`）と比較（version 4 のニブル、variant の 2 ビット、重複なし、戻り値 0、last error 不変、2 回の実行で同じ）、最初の 2 個の値の固定（`8c8f581c…e3ef`、`3f377f9c…cb21`。別の Python スクリプトでも計算）、GUID の並び（`1c588f8c-dce4-4d23-bfd8-2e006150e3ef`）、NULL ポインタの名前付き停止。
- `rpcrt4.dll`: 3 つの綴りで LoadLibraryW／GetModuleHandleW が同じハンドル、参照数 3、`GetProcAddress` は rpcrt4 の下でだけ見つかる、`fle_load_absent[]` の 7 名（`rpcrt4.dll` を含まない）。
- アイコン（実 `FLEngine_x64.dll`。`harness/image.c` が 0x400000 にマップ）: MAINICON の 13 エントリ（NEWHEADER と RESDIR を、テスト自身がリソースディレクトリをたどって読む）、14 通りの（要求サイズ、表示深度）→ 選ばれるエントリ（完全一致、深度の降順の規則、全部が深度を超えるときの最小、非正方形、256 = 0 の表記、全エントリが要求を超えるときの停止、グループ無しは 0 を返す）、`LoadIconW` の同じハンドル・種類 ICON・生きているオブジェクト数、`GetIconInfo` の fIcon／ホットスポット／2 つの新しいビットマップ／`GetObjectW` の 32×32、1 plane、1 bit と 32 bit、グループ無し・不明なモジュールハンドルの停止、アプリケーションアイコン（hInstance NULL）の `GetIconInfo` が停止のまま。
- `GetModuleFileNameW/A`: NULL は exe（49 文字）、0x400000 は DLL（57 文字）、`GetModuleHandleW(NULL)` は 0x400000、exe のパスが署名の表とレジストリの種の文字列に一致すること、定義が 1 か所であること。
- 実 DLL のバイト列: FXSR の枝（`test rax,0x1000000`、`call 0x6399d0`、`mov al,1; ret`）、CMOV の検査、遅延ロードの名前表（`rpcrt4.dll`、`UuidCreate`）、`LoadIconW`／`GetIconInfo` の thunk と `L"MAINICON"`、壁 9・10 の呼び出し（`SendMessageW` の WM_SETICON、`SetClassLongPtrW` の GCLP_HICON、`CreateWindowExW` の dwStyle 0x84ca0000、`L"TApplication"`）。
- CPUID（`cpuid.rs` の 2 件 + 更新）: EDX のビットを 1 つずつ（FPU、CX8、CMOV、MMX、FXSR、SSE、SSE2 は立つ、TSC・SEP ほかは立たない）、ECX・EBX・EAX は 0、leaf 0 は (1, 0, 0, 0)、拡張葉は 0。

## 検証（実測）

- **全体テスト**（`logs/fulltest/cargo-test-2.txt`）: 上のとおり 1 回の通しで 119 test result・1,647 passed・0 failed（警告は既存のテスト `sse_shuf`、`monitor` のものと `INIT_HEX` の未使用の定数ほか 4 件で、新規・更新のテストからは 0）。**経過の正直な記録**: (1) 最初の通しはサーバー再起動で中断した（`interrupted-run-1.txt`。数えない）。(2) 通しの後に 3 つの変更を足した: ① `fle_cpuid.c` のコメントの書き換え（オブジェクトは同一）、② `fle_sysinfo.c` のコメントの追加（オブジェクトは同一）、③ 変異検査 M10 が生き残ったため、`wall_fr5.rs` に `rpcrt4_is_no_longer_in_the_list_of_lacking_names_and_the_seven_others_are` を 1 件追加。足した後に `cpuid`、`deferred`、`k32_dsp`、`load`、`modules`、`msctf`、`vfs`、`wall_b`、`wall_fr4`、`wall_fr5`、`wall_s` の 11 本を個別に再実行して全部通った（`wall_fr5` は 14 件）が、**通しの全体を足した後の木では 1 回も走らせていない**。したがって「119 結果・1,647」は足す前の木の値で、足した後の木では +1 件（1,648）になる見込み（未確認）。(3) テストが固定パスで読む `out-*` の入力 11 ファイルは、実リポジトリの実体への symlink で与えた（差分に入らない）。
- **変異検査**（木のコピー `mut/` で 1 件ずつ変更 → 該当のテストを実行 → 落ちる → 元に戻す。`logs/mutation-run.txt`、`tools/mutate118.py`）: 対照 M0（変更なし）は通る。M1 CMOV を名乗らない、M2 FXSR を bit 22 に、M3 TSC を名乗る、M4 `GetModuleFileName(NULL)` を DLL のパスに戻す、M5 UUID の version を付けない、M6 variant を付けない、M7 seed を変える、M8 毎回同じ値、M9 NULL を受理、M10 `rpcrt4.dll` を absent に残す、M11 アイコンの選択が深度を無視、M12 要求が全エントリより小さいとき先頭を返す、M13 ホットスポットが (0, 0)、M14 毎回新しいハンドル、M15 グループが無いとき NULL を返す、M16 アプリケーションアイコンの `GetIconInfo` も答える、M17 応答表の行の位置を前にずらす: **17 件すべて検出**。M10 は 1 回目に**生き残った**（`LoadLibrary` はモジュール表を先に見るので、一覧に `rpcrt4.dll` が残っていても観測できない）ので、一覧そのものを検査するテストを足して検出した。
- **測定**（`logs/`。6 モジュール構成。ビルド gen 約 15 s + build 約 45 s、合計 約 1:00、最大 RSS 約 2.7 GB）:

| 構成 | native SHA-256 | 結果 |
|---|---|---|
| FRUITY 無効・HEAD（`base`） | `ab6c6be6…f187` | a4 942/942、895/895、functions 5,306、entries 1,007,942、終了 0 |
| FRUITY 有効・HEAD（`fr0`） | `15ab0a8e…8845` | `GetActiveWindow`、38,241／348、134 |
| 決定 1 だけ・FRUITY 無効（`c1off`） | `66a250e0…f840` | functions 5,308、entries 1,007,944、終了 0 |
| 決定 1 だけ・FRUITY 有効（`c1on`） | `93460cc4…ea157` | `RaiseException 0xc06d007e`、38,166／350 |
| 決定 1 + 2・FRUITY 無効（`c2off`） | `84df206d…5342` | functions 5,308、entries 1,007,947、終了 0 |
| 決定 1 + 2・FRUITY 有効（`c2on`） | `bf683b91…1ea4` | `RaiseException 0xc06d007e`、38,163／350 |
| 壁 7 を直した（`w7`） | `e0dbb79c…cea` | `LoadIconW`、38,873／372 |
| 壁 8 を直した（`w8`） | `48a81021…4259` | `SendMessageW`、38,913／379 |
| **FRUITY 無効・最終**（`fin-off1`、`fin-off2`） | **`27cdf764f10ffabc08bba2d4afb45815441cdb4e35c05221b102ce5908d23f6d`** | a4 942/942、895/895、functions **5,308**、entries **1,007,947**、`returned from entry`、終了 0。別の OUT・別の cargo target、fe-lift／fe-pe の SHA-256 同一 |
| **FRUITY 有効・最終**（`fin1`、`fin2`） | **`48a81021c332e2271ac02546e435c97240a7eae26a4437f03597a87c7aea4259`** | `unsupported instruction at 0x425390`（`SendMessageW`）、**38,913／379**、134。`fin2` は別の OUT・別の cargo target。`fruity:` の 133 行は経過時間の 1 行以外同一 |

- **gdb**（`logs/c2on/gdb-wintrust-paths.txt`）: `WinVerifyTrust` の 4 回の呼び出しのパスは #1・#2 が `…\FLEngine_x64.dll`、#3・#4 が `…\FL64.exe`（決定 0117 では 4 つとも DLL）。`GetModuleFileNameW/A` は FRUITY 無効の a4（DllMain まで）で 42 回呼ばれ、hModule が NULL のものが 27 回、0x400000 が 6 回、実在モジュール（0x1d00000 = QFC、0x6000000 = ILTools）が 9 回（`logs/gdb-fxsr/gdb-gmfn.txt`。gdb のブレークポイントで数えた）。NULL の 27 回が D2 で答えが変わった呼び出し。
- **決定 0105 のフック有効化**: `f_8e0ef0`（有効化の呼び出し、戻り先 0x8e4af6）が FL64.exe でも呼ばれる（入口列に 1 回。`0x414e70`（文字列比較）の呼び出し数は 203 で、D2 の前後で不変）。
- **差分の当たり方**: `patch -p1 --dry-run` は成果物の `fruity5.diff` を HEAD b8ed117（c76a7c3 の後に他の作業者の docs だけが入った。`lift/` は同じ）の clean な archive に当てて、終了 0、33 ファイルとも当たり、当てた木の 33 ファイルがワークツリーと `cmp` で一致することを確認した（`logs/patch-verify.txt`、`logs/patch-apply.txt`）。

## 解釈が分かれた点

1. **a4 の `entries +3`（D2）**: 依頼は「FRUITY 無効の a4 の functions／entries／native SHA を測り、942/942 が保たれること」で、+3 の予想はなかった。原因を実験で切り分けた（FL64.exe のパスを DLL と同じ 57 文字にすると +3 が消える）。パスの長さに依存する Delphi のメモリマネージャの経路の違いと読み、不具合とは扱わない。新しい FL64.exe のパス（PM の値）が別の長さになれば、entries はまた変わる。
2. **壁 9 で止める判断**: 依頼は「導けない値は出典なしの選択として列挙して進む」と「PM が決める値（仕様から導けず選択肢が分かれるもの）は止める」の両方を書く。(引用の語を、決定を求める言い回しから事実の言い回しに直した)壁 9 の答え（NULL の hWnd への `SendMessageW`／`SetClassLongPtrW`）は仕様に無く、0 と last error（1400 か不変か）の選択肢が分かれ、しかも実装しても壁 10 が (iii) で止まる。**止める側**に倒した。進める側に倒すなら実験 E5 の差分がそのまま使える。
3. **`LoadIconW` の失敗の扱い**: Learn は「失敗は NULL と GetLastError」と書く。グループが無い名前に NULL を答えると、コードと値を創作することになるので停止にした（実機は NULL）。
4. **`GetIconInfo` の範囲**: 依頼は「資源に実在するアイコンをハンドルで返す」と「`GetIconInfo`・描画は DIB セクションのモデル（決定 0108/0111）で答えられるか名前付き停止か分類」。`GetIconInfo` は、この共有アイコンに限りピクセルを持たないビットマップ 2 つ（0108 の「CreateBitmap の形」）を返すだけにした（`GetObjectW` と `DeleteObject` が使える）。ピクセルの内容を読む呼び出し（`GetDIBits`、`DrawIcon`、`DrawIconEx`）は従来どおり名前付き停止（`DrawIcon`／`DrawIconEx` は `GDI_STOP`）。アイコンを描くには RT_ICON の XOR／AND ビットマップの解釈（DIB の bottom-up、色表、マスクの合成）が要り、決定 0111 の DIB の演算の範囲外。
4b. **`IsProcessorFeaturePresent(2, 3)`（D7）**: CPUID が CX8・MMX を報告するのに 0 を答える。決定 0078 の方針（CPUID に合わせる）を逆方向にも当てるなら 1 にすべきだが、`fle_sysinfo.c` の挙動を変えると、依頼の「既存応答の変更は表に旧→新を明記」の範囲が広がる。**変えずに、コメントとテストで現状を固定**した。ゲストの呼び出し元が問うのは PF 23 だけ（決定 0078）なので、a4 には影響しない。
5. **決定 3（仮想時計）**: 直さない。決定 0117 の `unverified = chain,revocation,time` が全出力に明記されている。壁 9 までの実行で、署名検査の結果（`WinVerifyTrust` = 0、FL64.exe の行も使われた）は 0117 と同じ意味のまま。
6. **全体テストの数**: 「基準 118 件・1,632 passed」に新規を足す。新規 15 件の内訳は上のとおり（通しの後に 1 件足したため、通しの値とは 1 件ずれる。下の「検証」の注記）。

## 未決の点(PM)

1. **壁 9（`SendMessageW(NULL, …)`／`SetClassLongPtrW(NULL, …)`）**: 案 X1（止まる。推奨）、X2（0 と 1400。実験 E5）、X3（X2 + 仮想ウィンドウ表の拡張）。X2 は出典なしの選択を 2 つ足し、(a)(b) に近づかない（次が壁 10）。
2. **壁 10 以降の範囲（ウィンドウのモデル）**: `CreateWindowExW(WS_EX_TOOLWINDOW, "TApplication", "FL Studio", 0x84ca0000, X = 画面幅/2, Y = 画面高/2, 幅 0, 高さ 0)` は、決定 0071 の「隠れた WS_POPUP の道具ウィンドウ」を超える（キャプション・システムメニュー、位置が (0, 0) でない）。可視性（`ShowWindow`）、非クライアント領域（`WM_NCCALCSIZE` の `NCCALCSIZE_PARAMS`）、`WM_GETMINMAXINFO`・`WM_SIZE`・`WM_MOVE`・`WM_SHOWWINDOW`、メッセージキュー（`GetMessageW`／`PeekMessageW`）を段階的にどこまで入れるか。目的 (b)（ブロッキングのメッセージループ）に到達するには、このモデルの一部が必要と読む（未測定）。
3. **`IsProcessorFeaturePresent(2, 3)`（D7）**: 0 のまま（推奨: 呼ばれない。決定 0078 の撤回条件「CPUID と切り離して実機と同じ値にする」と同時に決める）／CPUID に合わせて 1 にする（決定 0078 の方針の逆方向）。
4. **FL64.exe のパス（D2）**: 49 文字の文字列が entries を +3 動かす。実機の値（`C:\Program Files\Image-Line\FL Studio 21\FL64.exe` ではない場合）が分かれば更新する。a4 の基準はそのとき動く。
5. **CPUID の機種像**: psABI の基準だけを名乗る（TSC・SEP・CLFSH・HTT などは名乗らない）。ファミリ・モデル・ベンダー文字列は 0 のまま。エンジンの検査（`f_639a20`）が読むほかのビット（TSC（bit 4）→ 0x2、SEP（bit 11）→ 0x8 など）が別の分岐で要るようになったら、そのときに決める。
6. **wasm の再ビルド**: runtime が変わるので wasm の SHA が変わる（別の作業）。
7. **docs の索引**: `docs/README.md` の 0118 の行は `docs/README-0118-line.txt`。注記の件数（決定ファイル 118 → 119、番号の種類 117 → 118、索引の行 118 → 119、仮の決定 49 → 50）は、足す側の確認を要する（この作業では `docs/README.md`、`docs/progress.md` を編集していない）。

## 撤回条件

- **D1（CPUID）**: 実機の Windows 上の CPUID の値、または PM の別の機種像の決定 → `FLE_CPUID_LEAF1_EDX` の更新（決定 0009 の撤回条件と同じ）。エンジンが TSC・SEP などの別のビットを必要とする壁が出たとき。AVX／SSE3 以降を名乗る決定（lifter の VEX 対応、決定 0075/0032 の撤回）のとき。
- **D2（`GetModuleFileNameW/A(NULL)` = FL64.exe）**: 実機でホストの exe のパスが別の値だと分かったとき → `FLE_VIRTUAL_EXE_PATH` の更新（+ 署名の表の FL64.exe の行のゲストのパス + a4 の基準）。ホストが FL64.exe でないと分かったとき（`docs/investigations/host-engine-boundary.md` の読みが外れたとき）。`GetModuleHandleW(NULL)` を exe の像にする決定をしたとき（そのとき NULL と 0x400000 が別のモジュールになっている現状を整理する）。
- **D3（時計）**: 証明書チェーン・時刻を検査する決定（決定 0117 の撤回条件）のとき再検討。
- **D4（`rpcrt4.dll` の仮想モジュール化）**: 実機で `rpcrt4.dll` の扱いが別だと分かったとき。他の `fle_load_absent[]` の 7 名を仮想モジュールにするかは、それぞれ呼ばれたときに（決定 0099 の流儀で）決める。
- **D5（`UuidCreate`）**: 実機の値（version 1 かどうか）が分かったとき、または乱数の主張を PM が別に決めたとき → seed・アルゴリズムの更新。UUID が識別子（一時フォルダ名）にだけ使われるのは読み。
- **D6（`LoadIconW`／`GetIconInfo`）**: 実機で選ばれるエントリが違うと分かったとき（例: 要求サイズ 32 × 32 で実機が 32 bit の別の扱い）。アイコンの描画（DIB の演算）を実装する決定のとき、ピクセルを持つビットマップに置き換える。ホットスポットの値が実機と違うと分かったとき。
- 壁 9: PM が X2／X3 を決めたとき。

## 限界

- 実機 Windows との照合は 0 件。wasm・ブラウザは未測定（native のみ）。
- 目的 (a)（`CreateFruityInstance` が戻る）にも (b)（ブロッキングのメッセージループ）にも**到達していない**。停止は壁 9（出典なしの選択）。その先の壁 10 は実験で見ただけ（出典なしの選択を含む実験）。壁 10 の先は見ていない。
- FXSR を名乗ったあとの `fxsave`／`fxrstor` の lift は、a4 では実行されないため試していない。dsp_ippv2 の `fxsave` 4 箇所のうち生成器が報告するのは 1 件だけ（残りが閉包の外か未到達かは未確認）。
- 機能語の bit 0x20000（FXSR の枝の中の `f_6399d0` が真のとき）が後で何に効くかは未追跡。
- アイコンのピクセルは読まない（描画は名前付き停止のまま）。`GetIconInfo` のビットマップはピクセルを持たない。
- `LoadIconW` の選択規則は、エンジンの `MAINICON`（正方形・1 plane・32 bit が最深）に対して検査した。他のイメージの形（PNG 圧縮、非正方形、Planes ≠ 1）は停止にしてある。
- 署名の検証は決定 0117 のまま（ダイジェストと署名値のみ。チェーン・失効・時刻は未検証）。
- 時間と RAM: 1 回の 6 モジュールのビルド（gen + build）が約 1:00、最大 RSS 約 2.7 GB。実行は 0.06 秒の呼び出し + 約 1 秒のハーネス。全体テストは約 40 分。
