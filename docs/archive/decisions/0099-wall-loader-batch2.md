# 0099: census の壁バッチ 2（ローダー系）— 遅延 import の `RaiseException` 2 件（Shcore の欠落と kernel32 の応答の欠落）、shell32 の序数 `GetProcAddress`、`VirtualProtect`（イメージの自己書き換え）、レジストリキー末尾の `\`

状態: **仮の決定**（応答の値と「持つ／持たない」の選択は、実機 Windows と照合していない作業者の決定。撤回条件は壁ごとの表）。ただし **W4（`VirtualProtect`）は決定しない**（調査結果と選択肢の報告だけ。実装なし）。PM の指示（census の壁バッチ 2。2026-10-27）を作業者（サブエージェント）が記録した。番号 0099 は PM の割り当て。

記入状況: すべて記入済み（決定の節・背景・範囲・範囲外は実装前に保存。実装した場所・テスト・検証・解釈が分かれた点・限界は実装と検証の後。数値は 2026-10-27 の実測）。測定の詳細は `wall-s-measurement.md`、パッチは `patches/wallS.diff`（census 適用後の木に当たる）。

**統合時の調整（merge1。この版が統合後の木に合う）**: 他のレーンとの統合（順序: 壁 I → R → S → B）で、`shcore.dll` の N を **33 → 35**（`fle_modules[]` の index 32 → 34）に振り直した。N = 33、34 は遅延モジュール（決定 0096: blend2d.dll、quickfontcache_x64.dll）が先に使っている。ハンドルの規則（`0x7FF800000000 + 0x10000 × N`）は同じで、shcore のハンドルは `0x7FF800230000`。応答表の行は、遅延モジュールの 11 行（269〜279）→ R の 10 行（280〜289）→ **S の 11 行（290〜300。1 始まりの位置）**の順になる（S の行の擬似 VA は `0x7FF900001220`〜`0x7FF9000012C0`。R の 10 行が S の前に入るため。決定 0015 のとおり既存の行の擬似 VA は動かない）。これに合わせて、既存テストの期待値（modules.rs、msctf.rs、load.rs、さらに遅延モジュールの deferred.rs）と wall_s.rs の固定数・位置の検査を直した（「テスト」の節の表）。この文書の他の数値（検証の節の全体テストの数、SHA-256、census の数）は **S 単独の木**での実測で、統合後の木では測り直す。

区分の凡例: **実測** = 実行した出力（gdb、census、テスト）。**読み** = 逆アセンブル・生成 C を読んだ結果で、実行して確かめていない部分。**Learn** = Microsoft Learn のページに書かれていること（取得日 2026-10-27）。**参考（非公式）** = Learn 以外（Wine の `shell32.spec`、Raymond Chen のブログ）で、結論の根拠にしない。**推測** = 私の知識・推論。**未確認** = 確認していない。

事実の出典:

- 決定 0097（census）、`lift/out-census/docs-new/wall-census-2.md`／`.tsv`／`wall-census-2-notes.md`（読み取り専用）。census の数は A4 の進捗ではない（決定 0046、0060）。
- 決定 0015（仮想モジュール表と `GetProcAddress`）、0029（ライブラリロード系。`fle_load_absent[]` の 9 名）、0055（absent に足してよい条件）、0081、0088（avrt／powrprof を仮想モジュールにした先例）、0016（エミュレートロケール = en-US）、0053（仮想画面の DPI = 96）、0045（wasm のページ保護の非強制）、0024（メモリ保護と VirtualProtect）、0018（仮想レジストリ）、0085（HKLM\SOFTWARE の初期キー）。
- `docs/investigations/loadlibrary-names-classification.md` §1 の 8（遅延 import 経由の `LoadLibraryA`。「absent の 9 名は NULL になると `RaiseException(0xc06d007e)` に進む形（読み）」）。
- 調査の作業物は `lift/out-wallS/investigate/`（gdb スクリプト、`delayimp.py`、`delayimp-flengine.txt`、`delay-callers.txt`、`peva.py`）と `runs/`（gdb の生の出力）。

## 先に結論: `RaiseException` の本当の原因（unit 245、265）

2 件とも **SEH を実装する話ではない**。ゲストが呼んだのは MSVC の遅延 import ヘルパー（`__delayLoadHelper2` 相当。`f_41d9a0`）で、そのヘルパーが **この環境の応答の欠落** のために失敗し、`RaiseException` に至る。

| unit | 例外コード | 失敗した遅延 import（記述子 → 関数） | 失敗した段 | 欠けていた応答 |
|---|---|---|---|---|
| 245 | `0xc06d007e` | 記述子 16 `Shcore.dll`（VA 0x1976200）→ `GetProcessDpiAwareness`（IAT スロット 0x1976490） | `LoadLibraryA("Shcore.dll")` が NULL（ret 0x41dc6f） | `shcore.dll` が **absent**（`fle_load_absent[]`、決定 0015／0029）で、`GetProcessDpiAwareness` の応答も無い |
| 265 | `0xc06d007f` | 記述子 5 `kernel32.dll`（VA 0x19760a0）→ `GetSystemDefaultLocaleName`（IAT スロット 0x1976398） | `GetProcAddress(kernel32, "GetSystemDefaultLocaleName")` が NULL（ret 0x41db6a） | 応答表に kernel32 の `GetSystemDefaultLocaleName` の行が無い |

- 例外コードの意味は **Learn** で確認した（<https://learn.microsoft.com/en-us/cpp/build/reference/error-handling-and-notification>、「Delay load exception codes」）: `LoadLibrary` 失敗は `VcppException(ERROR_SEVERITY_ERROR, ERROR_MOD_NOT_FOUND)`、`GetProcAddress` 失敗は `VcppException(ERROR_SEVERITY_ERROR, ERROR_PROC_NOT_FOUND)`、`FACILITY_VISUALCPP = 0x6d`。`0xC0000000 | (0x6d << 16) | 126 (0x7e)` = `0xc06d007e`、`| 127 (0x7f)` = `0xc06d007f`。さらに、コードの流れ（読み）が一致する: `0x41dc5d` の `mov ecx,0xc06d007e` は `LoadLibraryA`（`f_41cf90`）が NULL を返し、失敗フック（`[0x15ec2c0]`、`dliFailLoadLib` = 3）が NULL のとき。`0x41db58` の `mov ecx,0xc06d007f` は `GetProcAddress`（`f_41cfb0`）が NULL を返し、失敗フック（`dliFailGetProc` = 4）が NULL のとき。`0x41da18` の `0xc06d0057`（= `ERROR_INVALID_PARAMETER` 87）は `grAttrs` の不正。**以前の推測（遅延 import の失敗）は確定**。
- 実測（gdb、census の dsp 構成。`runs/gdb-delay.out`）: `f_41d9a0` の引数 `pidd`／`ppfnIATEntry` と、続く `RaiseException`。unit 245 は `pidd = 0x1976200`（`Shcore.dll`）、`ppfn = 0x1976490`、unit 265 は `pidd = 0x19760a0`（`kernel32.dll`）、`ppfn = 0x1976398`。遅延 import 表（17 記述子、57 関数）は `investigate/delayimp-flengine.txt`。
- **同じ種類の壁は 54 件ある**（実測の表: `investigate/delay-callers.txt`）。57 の遅延 import のうち、応答表に行があるのは 3 件（`GetLogicalProcessorInformation`、`IsWow64Process`、`GetNativeSystemInfo`）だけで、残り 54 件のうち 48 件は `call` する場所がある（propsys の 5 件と ole32 の `PropVariantClear` は呼び出し 0。静的な直接 call だけを数えた下限）。absent 9 名の遅延 import（wtsapi32 2、msimg32 2、rpcrt4 1、propsys 5、windowscodecs 1、uxtheme 5、imm32 7、dwmapi 2、shcore 2 = 27 関数）は、呼ばれれば 0xc06d007e、仮想モジュールだが応答の無い kernel32／user32／shell32／advapi32／ole32 の遅延 import は 0xc06d007f になる。この決定の後に残るのは 52 件（absent 8 名の 25 件は 0xc06d007e、他の 27 件は 0xc06d007f。shcore の `GetDpiForMonitor` は 0xc06d007f）。census は unit ごとに最初の壁しか見えないので、**この 2 件を直しても、同じ unit か他の unit で同じ種類の壁が出る**（「測定」で確認）。この決定は、census が示した 2 件だけを直す。

## 決定（壁ごとの表）

壁の番号は PM の指示の番号とは別で、本書の中だけで使う（W1 = unit 245、W2 = unit 265、W3 = unit 238、W4 = unit 314、W5 = unit 830）。

### W1: unit 245 — `Shcore.dll!GetProcessDpiAwareness`

| 項目 | 内容 |
|---|---|
| 呼び出し元の読み | `f_80fa60`（読み）: `f_44ad60(6, 3)`（Windows 6.3 以上か。決定 0014 の 10.0 では真）が真なら `GetCurrentProcess`（`f_423440`）を `hprocess`、`[rsp+0x2c]` を `value` として遅延 import `GetProcessDpiAwareness`（stub `0x80fa10`、呼び出し `0x80fa84` の 1 か所）を呼び、`value == 2`（`PROCESS_PER_MONITOR_DPI_AWARE`）を返す。偽なら `user32!IsProcessDPIAware`（`f_424ce0`）。つまり「per-monitor DPI aware か」の問い |
| 仕様 | **Learn**: `GetProcessDpiAwareness(HANDLE hprocess, PROCESS_DPI_AWARENESS *value)`（shellscalingapi.h、Windows 8.1 以降、Shcore.dll）。`hprocess` は「NULL ならカレントプロセス」。戻り値は `S_OK`／`E_INVALIDARG`（「ハンドルまたはポインタが不正」）／`E_ACCESSDENIED`。値は `PROCESS_DPI_AWARENESS`（0 = UNAWARE、1 = SYSTEM_AWARE、2 = PER_MONITOR_AWARE。列挙体のページは未取得、**参考**: 値は私の知識で、本書の実装の根拠は 0 のみ）。`value` の中身が何であるべきかは、ページが「プロセスごとの設定」としか書かず、この環境の値は決まらない |
| 決定 | (1) `shcore.dll` を **仮想モジュール N = 35**（`fle_modules[]` の末尾。遅延モジュール N = 33、34（決定 0096）の次。S 単独の木では N = 33 だった）にし、`fle_load_absent[]` から外す（決定 0088 の avrt／powrprof と同じ形。`LoadLibrary`／`GetModuleHandle` が成功し、ハンドルは `0x7FF800000000 + 0x10000 × 35`）。(2) 応答表の末尾に `shcore.dll!GetProcessDpiAwareness` を足す。`hprocess` が NULL または `GetCurrentProcess` の擬似ハンドル（-1）で `value` が非 NULL なら、`*value = 0`（`PROCESS_DPI_UNAWARE`）を書いて `S_OK`。それ以外（NULL の `value`、他のハンドル）は `E_INVALIDARG`（ページが書く。この環境は他のプロセスのハンドルを作れないので、-1 と NULL 以外は不正なハンドル）。(3) 同じ DLL の `GetDpiForMonitor` は足さない（呼ばれれば `0xc06d007f` になる = 名前の無い停止ではなく、遅延ヘルパーの `RaiseException` で止まる。範囲外） |
| 値の選択（仮） | UNAWARE（0）。理由: 仮想プロセスにマニフェストの DPI 宣言があるという事実が無く（`FLEngine_x64.dll` は DLL で、ホスト exe は無い）、決定 0053 の仮想画面は 96 DPI の 1 モニタで、UNAWARE と PER_MONITOR は描画の数値が同じになる。選択肢: SYSTEM_AWARE（1）、PER_MONITOR_AWARE（2）。2 を選ぶとゲストは per-monitor の経路（`SetThreadDpiAwarenessContext`、`GetDpiForMonitor` ほか、どれも応答なし）へ進むので、1 件の壁を 54 件の壁の入口にする |
| 撤回条件 | ホスト exe（FL Studio 本体）の DPI 宣言が分かったとき。unit が per-monitor の経路を必要としたとき。`GetModuleHandleW("shcore.dll")` が成功してはならない根拠が見つかったとき（FLEngine に `shcore` を `GetModuleHandle` する呼び出しは無い: ファイル中の文字列 `Shcore` は遅延 import 記述子の 1 か所だけ、実測）。ゲストが `GetDpiForMonitor` を NULL でなく使う必要が出たとき |
| 副作用（実測） | **unit 145（`f_65c9f0`）が自前で `LoadLibraryW(L"SHCore.dll")` と `GetProcAddress(h, "GetDpiForMonitor")` を行う**（gdb で確認。遅延 import ではなく、avrt／PowrProf と同じ関数）。以前は `LoadLibraryW` が NULL（absent）でその先を飛ばした。今は `LoadLibraryW` が成功し、`GetDpiForMonitor` は応答表に行が無いので NULL と 127（決定 0015）になり、ゲストはその「関数が無い」経路を通る。unit 145 の入口数 422 → 440（+18）、census 無効の a4（dsp 構成）の entries 26,585 → 26,603（init_count 150、functions 943 は不変）。`GetDpiForMonitor` に行を足す・足さないは、この決定の範囲外（足すと per-monitor の経路が広がる） |
| テスト | `tests/wall_s.rs`: `shcore_is_the_35th_module_and_loads_by_every_spelling`（`LoadLibraryA`／`W`／`GetModuleHandleW` の全綴り、last error 5 のまま、参照数、`fle_module_count` 35）、`shcore_has_only_the_get_process_dpi_awareness_row`（`GetDpiForMonitor`・大文字小文字違い・kernel32 のハンドル・shell32 のハンドル・無関係な名前はどれも NULL と 127）、`get_process_dpi_awareness_answers_the_current_process_as_dpi_unaware`（-1 と NULL は 0 と `*value` の下位 4 バイトだけ 0、他のハンドル・スレッドの擬似ハンドル・NULL ポインタは `0x80070057`、`*value` は不変）、`the_two_failing_delay_imports_are_what_the_decision_says`（実 DLL の遅延 import 記述子から `Shcore.dll!GetProcessDpiAwareness` のスロット 0x1976490） |

### W2: unit 265 — `kernel32.dll!GetSystemDefaultLocaleName`

| 項目 | 内容 |
|---|---|
| 呼び出し元の読み | `f_7a6fe0` の手前の関数（`0x7a6fa3`、呼び出し 1 か所、読み）: Delphi の文字列 `S` を `SetLength(S, 85)`（= `LOCALE_NAME_MAX_LENGTH`）し、`n = GetSystemDefaultLocaleName(PWideChar(S), 85)`、`SetLength(S, n - 1)`。つまりシステム既定のロケール名を文字列に取る |
| 仕様 | **Learn**（取得 2026-10-27）: `int GetSystemDefaultLocaleName(LPWSTR lpLocaleName, int cchLocaleName)`、Windows Vista 以降、Kernel32.dll。成功は「終端 NUL を含むロケール名の長さ（> 0）」、失敗は 0 で `ERROR_INSUFFICIENT_BUFFER`（「バッファが足りない、または NULL」）。`LOCALE_NAME_MAX_LENGTH` が推奨サイズ。ページは `cchLocaleName` が負・0 の場合を書かない |
| 決定 | `kernel32.dll!GetSystemDefaultLocaleName` を実装して応答表の末尾に足す。エミュレートロケール（決定 0016: en-US、LCID 0x0409）の名前 `"en-US"` を書き、**6**（NUL を含む）を返す。`lpLocaleName == NULL` または `0 <= cchLocaleName < 6` は 0 と `ERROR_INSUFFICIENT_BUFFER`（122。ページの失敗の定義）。`cchLocaleName < 0` はページが書かないので名前付き停止。バッファには失敗時に触れない |
| 撤回条件 | 決定 0016 のロケールを変えるとき（この関数は `fle_emulated_locale` から名前を作る。en-US 以外は名前付き停止）。実機で `GetSystemDefaultLocaleName` の `cchLocaleName < 0` や 0 の挙動が分かったとき |
| テスト | `tests/wall_s.rs`: `get_system_default_locale_name_gives_en_us_and_fails_on_a_small_buffer`（85 → 6 と `en-US\0`、6 → 6、5・0・NULL バッファ → 0 と 122 でバッファ不変、`edx` の上位 32 ビットは無視、`GetLocaleInfoW(LOCALE_SNAME)` と同じ文字列、成功時 last error は 5 のまま）、`a_negative_locale_name_size_stops`。実行結果 unit 265 は ok（census） |

### W3: unit 238 — `GetProcAddress(shell32, 序数)`

| 項目 | 内容 |
|---|---|
| 呼び出し元の読み | `f_712080`（読み、`objdump`）: `LoadLibraryW(L"shell32.dll")`（`0x712184` の文字列）の結果を `[0x13f4da0]` に保存し、NULL でなければ `GetProcAddress(h, 序数)` を **9 回**（序数 `0x10, 0x11, 0x12, 0x13, 0x15, 0x19, 0x98, 0x99, 0x9b`）呼び、結果を `[0x13f4a28]`〜`[0x13f4a68]` に保存する。census の `hModule = 0x7ff800100000` は `fle_modules[15]` = `shell32.dll`（実測）。**9 個の保存先のうち、後から読まれるのは `[0x13f4a38]`（序数 0x12 = 18）だけ**（`.text` 全体の rip 相対参照を `grep`。呼び出し 2 か所 `0x710f5c`、`0x711050`。NULL 検査なし） |
| 序数と関数の対応 | **Learn には序数の表が無い**（未確認）。**参考（非公式）**: Wine の `dlls/shell32/shell32.spec`（取得 2026-10-27）は、16 = `ILFindLastID`、17 = `ILRemoveLastID`、18 = `ILClone`、19 = `ILCloneFirst`、21 = `ILIsEqual`、25 = `ILCombine`、152 = `ILGetSize`、153 = `ILGetNext`、155 = `ILFree` を `-ordinal` として載せる。ゲストの 9 個の序数と**ちょうど一致**するので、PIDL（`ITEMIDLIST`）を扱う関数の群をゲストが序数で引いている、という読みは強い。ただし**これは Wine の仕様で、実際の Windows 10 22H2 の shell32 で確認していない**（推測） |
| 決定 | (1) `GetProcAddress` の序数の経路を、**序数の表を持つ仮想モジュールに限って**名前に解決してから通常の引き（応答表）に渡す形にする。表は `fle_modules.c` の静的な表 `{モジュール名, 序数, 名前}`。持つのは **shell32.dll の上の 9 個だけ**。表に無い序数、表を持たないモジュール、実在モジュール、NULL（序数 0）は、従来どおり名前付き停止（メッセージは従来の文面で始め、モジュール名と序数を足す）。(2) 応答表の末尾に shell32.dll の 9 行（`ILFindLastID`、`ILRemoveLastID`、`ILClone`、`ILCloneFirst`、`ILIsEqual`、`ILCombine`、`ILGetSize`、`ILGetNext`、`ILFree`）を足す。各行の関数は**名前付き停止**（"shell32 PIDL function: the runtime has no shell namespace / ITEMIDLIST model"）で、擬似 VA が返って、呼ばれたら止まる（先例: 決定 0055 の msctf!`SetInputScope`）。名前でも引ける（`GetProcAddress(shell32, "ILClone")` は Windows 10 の shell32 にある関数で、Learn に `ILClone` の関数ページがある。名前での export は**未確認**） |
| 決めなかったこと | ILClone などの**実装**（PIDL のモデルが無く、ゲストはこの unit では呼ばない）。序数 9 個以外の shell32 の序数（hundreds ある）。他の DLL の序数 |
| 撤回条件 | 実際の shell32 で序数の対応が違うと分かったとき（その場合、表の 9 行を直す。名前の行は残る）。ゲストが PIDL 関数を呼ぶ unit に達したとき（名前付き停止が出る。`ILClone` が最初の候補） |
| 副作用（実測） | **`ILFree` は FLEngine の通常の import でもある**（IAT スロットの thunk `f_593410`。生成器の統計 `unsupported_total` 525 → 524、`jmp_imp:ILFree` 1）。応答表に `shell32.dll!ILFree` の行を足したので、この thunk は `fle_unsupported(0x593410)` ではなく名前付きの停止（`shell32!ILFree: not answered`）になる。**生成 C の関数本体が 1 つ変わる**（この thunk だけ。停止することは同じで、メッセージが変わる）。それ以外の本体の変化は 0 |
| テスト | `tests/wall_s.rs`: `the_nine_shell32_ordinals_resolve_to_the_same_pseudo_va_as_the_names`（9 個の序数が、名前で引いたときと同じ擬似 VA、last error 5 のまま）、`an_ordinal_that_is_not_in_the_table_stops_with_the_module_and_the_ordinal`（shell32 の 20・0xffff、kernel32 の 18、shcore の 16）、`the_other_ordinal_stops_keep_their_old_message`（序数 0、イメージのハンドル、不正なハンドル）、`each_shell32_pidl_function_is_found_and_stops_by_name_when_called`（9 個それぞれ）、`the_guest_asks_shell32_for_exactly_the_nine_ordinals_of_the_table`（実 DLL の `f_712080` のバイト列から 9 個の序数と文字列 `shell32.dll`）、`a_thunk_of_a_new_row_is_handled_only_under_its_own_dll`。実行結果 unit 238 は ok（census） |

### W4: unit 314 — `VirtualProtect` がイメージの `.text` に対して呼ばれる（**決定しない。報告**）

| 項目 | 内容 |
|---|---|
| 実際の引数 | `lpAddress = 0x41c820`、`dwSize = 5`、`flNewProtect = 0x40`（`PAGE_EXECUTE_READWRITE`）、`lpflOldProtect` = ゲストのスタック（実測、gdb。`runs/gdb-abort.out`）。0x41c820 は **FLEngine の `.text` の中**（`.text` = 0x401000〜0x13a44b0、特性 0x60000020 = コード・実行・読み取り。実測）で、ゲストの関数 `f_41c820`（`push rbp; push rsi; push rbx; …` のプロローグ。文字列を 0x1000 ずつコピーする関数。読み）の**先頭** |
| ゲストの意図 | `f_8e0c60`（コンストラクタ。`rcx` = クラス記述子、`dl = 1`、`r8` = 対象関数、`r9` = 置き換え先、`[rsp+0x20] = 0`）→ `f_8e0de0`（読み）: **インライン フック（先頭 5 バイトを `jmp rel32` に書き換える）の準備**。`[obj+0x24]` に対象、`[obj+0x18..0x1c]` に `0xe9 + rel32(置き換え先 − 対象 − 5)`、`[obj+0x1d..0x21]` に対象の元の 5 バイトを保存し、`VirtualProtect(対象, 5, PAGE_EXECUTE_READWRITE, &old)`。失敗（戻り 0）なら `f_44cb40`（エラー）。**パッチの書き込み（有効化）は別の関数**: `f_8e0ef0` が 3 つのオブジェクトに `f_8e0de0` をもう一度掛け、`f_8e0db0` が `[obj+0x18..0x1c]` の 5 バイトを対象 `[obj+0x24]` に書く（`f_8e0d80` は元の 5 バイトを戻す）。`f_8e0ef0` の呼び出しは `0x8e4af1` の 1 か所で、**`ParamStr(0)`（`f_40a060`）のファイル名を小文字にした文字列が `delphi32.exe`（`0x8e4b94`）と等しくないときだけ**（`f_414e70` は等しいと 0 を返し、`je` で飛ばす。読み）。この環境の `GetModuleFileNameW(NULL)` は `FLE_VIRTUAL_DLL_PATH` で、`delphi32.exe` ではないので有効化される（実測、下の参考実験）。`VirtualProtect` はコンストラクタで、有効化の条件に関係なく呼ばれる。unit 314 の proc `f_8e48a0` は静的初期化で、このコンストラクタを 3 回呼ぶ: (対象, 置き換え先) = (0x41c820, 0x8dad80)、(0x43b130, 0x8dac70)、(0x43b150, 0x8dace0)（`lea r8`／`lea r9` の読み）。置き換え先は 1 つの大域オブジェクト（`[0x15f93c8]`、`f_8dc210` で作る）を経由する関数で、**文字列クラスの関数を横取りする仕組み**（翻訳・ローカライズ層ではないか、は推測。未確認） |
| runtime が追跡しているもの | `VirtualProtect` は `fle_sys_region_at`（`VirtualAlloc` の領域の表）に当たるアドレスだけ扱う（`fle_vmap.c`、決定 0024）。ファイルビュー（`MapViewOfFile`）は「未実装」で停止、**それ以外（ロードしたイメージ、ホストのヒープ、スタック）は追跡せず停止**（メッセージ: "address is not in a VirtualAlloc region (the runtime does not track other memory)"）。`VirtualQuery` も同じ（イメージは領域に無い）。native のハーネスはイメージを `PROT_READ\|PROT_WRITE\|PROT_EXEC` で mmap する（`harness/image.c`。読み）ので、ホストは `.text` への書き込みを止めない。決定 0045 の wasm 版はページ保護を**記録するだけで強制しない**（fail-open）。したがって、イメージへの `VirtualProtect` に「成功」と答えても、native でも wasm でも書き込みは通り、**フックのバイトが `.text` に入るだけで、リフトされた C は書き換えられたコードを実行しない**（`f_41c820()` は C の関数呼び出しで、メモリ上の命令を読まない） |
| 選択肢 | (A) 現状の停止のまま（自己書き換えコードの停止として妥当）。(B) イメージのセクションを追跡し（`.text` は `PAGE_EXECUTE_READ` 等。セクションの特性から導く。Learn の "Memory Protection Constants" と PE 形式）、`VirtualProtect` は旧保護の返却と新保護の記録だけ行う（強制しない）。**ゲストのフックは黙って無効になる**（フック対象の文字列クラスの関数は元のまま動く）。(C) (B) に加えて、フックのバイトを検出して名前付き停止する（書き込みの検出には、`.text` を読み取り専用にして SIGSEGV を使うか、実行の最後に比較する、など別の設計が要る）。(D) フックを実際に効かせる（生成 C の関数入口に置き換え表を引く検査を足す、または対象を静的に特定して生成器が呼び出しを置き換える）。生成器の設計変更で、この決定の範囲外 |
| 決定 | **しない**。理由: (B) は fail-soft（黙って意味が変わる。決定 0046 の方針 4）で、A4 の「InitUnits を通す」を満たしても挙動の同一性が崩れる。(C)(D) は設計の判断が要る（PM）。コードは変えない（`fle_vmap.c` は無変更）。最小案: **(A) のまま、フック 3 件の置き換え先が何をするか（`f_8dad80`、`f_8dac70`、`f_8dace0`、`f_8dfb80`）を読んで、無効でも影響が無い（またはある）ことを示してから、(B) を PM が決める**。(B) にする場合の最小の実装は、`fle_vmap.c` の `VirtualProtect` に「イメージの範囲は旧保護をセクションの特性から返し、新保護を記録する」を足すことと、**黙らせない**ために、イメージの `PAGE_EXECUTE_READWRITE`（コードを書き換え可能にする要求）を 1 行の注記（stderr）に出すこと。もう 1 つの選択肢 (E): ゲスト自身が持つスイッチ（`ParamStr(0)` が `delphi32.exe` のときフックしない）を環境の定義で使う（`GetModuleFileNameW(NULL)` の値を変える）。ゲストが自分でフックを無効にするので偽の成功は無いが、`VirtualProtect` には答える必要が残り、環境の定義を変えることになる（0018 の `FLE_VIRTUAL_DLL_PATH`） |
| 参考実験（実測。パッチに含めない） | `exp/src/fle_vmap_exp.c`（`fle_vmap.c` のコピー）で、`VirtualProtect` が `[0x400000, 0x400000 + 0x18c0000)` に完全に収まる範囲なら、旧保護 `0x20`（`PAGE_EXECUTE_READ`）を書いて 1 を返す（何も記録しない）だけの使い捨てビルド（`exp/two-native-exp`）で dsp 構成の census を実行した（`runs/exp/`）。結果: **unit 314 は ok（入口数 890、次の壁なし）**、他の unit の状態と入口数は変わらない（ok 879／stop 16）。gdb で見ると、unit 314 の中で `f_8e0ef0`（有効化）が 1 回呼ばれ、census の終わり（終了フェーズの停止点）で **`0x41c820`、`0x43b130`、`0x43b150` の先頭 5 バイトが `e9 rel32`（`0x8dad80`、`0x8dac70`、`0x8dace0` への `jmp`）になっている**。リフトされた C は書き換えられたコードを実行しないので、フックは効かない。有効化の**後に** `f_41c820`／`f_43b130`／`f_43b150` が呼ばれた回数は、この census の範囲（放棄した unit が多い）で **0**（有効化の前は `f_41c820` が 63 回）。つまり、この census の範囲では、(B) にしても結果は変わらないが、これは「フックの対象が後で呼ばれない」ことの証明ではない（放棄した unit の先は見えない） |

### W5: unit 830 — `RegOpenKeyExW` のキーパス末尾の `\`

| 項目 | 内容 |
|---|---|
| 実際のパス | `hKey = HKEY_CURRENT_USER`（`0xffffffff80000001`）、`lpSubKey = L"Software\Image-Line\Registrations\"`（34 文字、**末尾に `\`**）、`ulOptions = 0`、`samDesired = 0xf003f`（`KEY_ALL_ACCESS`）（実測、gdb。`runs/gdb-reg.out`。この呼び出しの 1 つ前までに `Software\Image-Line\Shared` と `General` が作られているが、`Registrations` は作られていない） |
| ゲストの意図 | `f_5b0cd0`（読み）は **Delphi の `TRegistry.OpenKey(const Key: string; CanCreate: Boolean)`**: `IsRelative`（先頭が `\` か。`f_5aeec0`）、絶対なら先頭の `\` を `Delete(S, 1, 1)`、`GetBaseKey`、そして `RegOpenKeyEx(base, PChar(S), 0, access, &result)`（`f_4230d0` が thunk）。ゲストのアプリ側が `Registrations\` と**末尾に `\` を付けたキー名**を渡している（登録情報の列挙のため、と推測。未確認）。`OpenKey` は戻り値が `ERROR_SUCCESS` かで真偽を返すだけ（Delphi の標準実装。読みではなく既知の実装。`f_5b0cd0` の構造と一致） |
| 仕様 | **Learn**（`RegOpenKeyExW`）: キー名は大文字小文字を区別しない、`lpSubKey` が NULL または空で `hKey` が定義済みキーならそのハンドルを返す、キーが無ければ `ERROR_FILE_NOT_FOUND`（ページの例）。**空要素（先頭・末尾・二重の `\`）の扱いはページに書かれていない**。**参考（非公式、Microsoft の社員のブログ）**: Raymond Chen「Why does RegOpenKey sometimes (but not always) fail if I use two backslashes instead of one?」（2012-10-04）は「二重の `\` はレジストリのキーパスでは正しくない」「レジストリ木の検索はパス `Blah\\Oops` を『サブキー `Blah` を探し、次にサブキー `""` を探し、次に `Oops`』と解釈し、`""` の段で失敗する（キーは空の名前を持てない）」「検索経路の不一致は Windows Vista で解消した」と書く。**エラーコードはそこに書かれていない**（記事の XP の話は `ERROR_INVALID_ARGUMENT`）。先頭の `\` が `ERROR_BAD_PATHNAME`（161）になる、という報告は Stack Overflow の質問（非公式、実機の報告）。**末尾の `\` が成功するか失敗するかは、Learn にも上の記事にも書かれていない（未確認）** |
| 決定 | `RegOpenKeyExW`／`RegOpenKeyExA` のキーパスを**先頭から 1 要素ずつ順に引く**（Raymond Chen の記事が書く検索の順序）形にし、**空要素に「到達した」ときだけ**名前付き停止する。到達する前に存在しない要素があれば、その要素で `ERROR_FILE_NOT_FOUND`（2）を返す。今回の `Software\Image-Line\Registrations\` は `Registrations` が無いので 2 になり、空要素（末尾）には到達しない。先頭の `\`、二重の `\`、**存在するキーの後ろの末尾の `\`** は、空要素に到達するので従来どおり名前付き停止（実機の挙動が未確認。上の「仕様」）。`RegCreateKeyExW` ほか、`split_key` を使う他の関数の挙動は変えない（空要素は従来どおり、パスを分割する時点で停止） |
| この決定が言っていること／いないこと | 言う: 「前の要素が無ければ、空要素に関係なく『無い』」（Windows が要素の順に検索するなら成り立つ。事前にパス全体を検査して拒否する実装なら、エラーコードは 2 ではなく 87／161 などになりうる = ゲストにとっては同じ「開けなかった」で、`TRegistry.OpenKey` は `ERROR_SUCCESS` でないことだけを見る）。言わない: 末尾の `\` を Windows が受理するか。受理しても、この仮想レジストリに `Registrations` は無いので結果は同じ（FILE_NOT_FOUND） |
| 撤回条件 | 実機で、末尾の `\` が存在するキーに対して成功する／失敗する（コード）が分かったとき（その場合、到達後の停止を応答に置き換える）。仮想レジストリに `Software\Image-Line\Registrations` が作られる経路が出たとき（その時点で末尾の `\` が決定を要する） |
| テスト | `tests/wall_s.rs`: `a_trailing_backslash_after_a_missing_key_is_file_not_found_like_the_guest_call`（ゲストの呼び出しそのもの（`Software\Image-Line\Shared` を作った後の `Software\Image-Line\Registrations\`）が 2、出力ハンドルは不変、`RegOpenKeyExA` も同じ、先頭の要素が無い `Nope\`・`Nope\\Deeper`・`Software\Nope\`・`Software\Image-Line\Nope\\` も 2、末尾の `\` の無い同じパスも 2、存在するキーと空文字列は 0）、`an_empty_component_that_the_walk_reaches_still_stops_by_name`（先頭の `\`（キーの有無に関係なく）、`\` だけ、存在するキーの後ろの末尾の `\`（1 段・2 段・A 版）、存在するキーの後ろの二重の `\`、および `RegCreateKeyExW`／`RegDeleteKeyW` の末尾の `\` は従来どおり停止）。既存の `reg.rs` の `open-leading-backslash`／`open-trailing-backslash` は変更なしで通る（`T` が存在するため）。実行結果 unit 830 は、この壁を過ぎて**次の壁**（下の測定）に進む |

## 範囲（今回扱う）

- W1、W2、W3、W5 の実装と、W4 の調査・選択肢の報告・参考実験。census による前後比較（`wall-s-measurement.md`）。

## 範囲外（今回扱わない）

- 未応答 import（acmMetrics ほか）と `SPI_GETWORKAREA`／`SM_DIGITIZER`（レーン R）、`indirect call to 0x477e73` と終了フェーズの `jmp [rcx]`（レーン L）、Blend2D／QFC／iltools のロード（遅延モジュール。決定 0096）。
- SEH の実装。**不要と分かった**（上の結論）。遅延 import ヘルパーが `RaiseException` に至る**前**の応答を足した。
- 他の遅延 import の応答（54 件の壁。`investigate/delay-callers.txt`）。absent の 8 名（dwmapi、imm32、msimg32、propsys、rpcrt4、uxtheme、windowscodecs、wtsapi32）を仮想モジュールにするかは PM の方針。
- `ILClone` ほか shell32 の PIDL 関数の実装。`GetDpiForMonitor` の応答。
- 新しい `.c`／`.h`（`tests/modules.rs` が全 `fle_*.c` をリンクする）。

## 実装した場所

新しい `.c`／`.h` は作っていない。パッチ `patches/wallS.diff`（census 適用後の木に当たる。`git archive` の HEAD f1bcba7（2026-10-27 時点）に census.diff を当てた木へ `patch -p1 --dry-run` で当たることを確認済み）は 13 ファイル。

- `lift/crates/fe-lift/src/imports_table.rs`（+15）: `TABLE` の**末尾**に 11 行（`kernel32.dll!GetSystemDefaultLocaleName`、`shcore.dll!GetProcessDpiAwareness`、`shell32.dll!ILFindLastID`、`ILRemoveLastID`、`ILClone`、`ILCloneFirst`、`ILIsEqual`、`ILCombine`、`ILGetSize`、`ILGetNext`、`ILFree`）。既存の行の擬似 VA は動かない（決定 0015）。他のレーン（遅延モジュール、R、B）も末尾に行を足すので、マージの順序で擬似 VA が決まる。統合後の順序は 遅延モジュールの 11 行 → R の 10 行 → この 11 行 → B の 9 行。行の位置を固定するテストは、`the_new_rows_are_the_last_rows_of_the_response_table_in_order` を `the_new_rows_follow_the_rows_before_them_in_the_response_table_in_order` に直し、「最後の 11 行」ではなく「`GetSystemDefaultLocaleName` の行から始まる連続した 11 行で、直前の行が R の最後の行（`version.dll!GetFileVersionInfoSizeW`）」を見る（後のレーンが末尾に足しても壊れない）。
- `lift/runtime/fle_modules.c`（+69 −8。**PM がマージする箇所。遅延モジュール（決定 0096）と重なりうる**）:
  - 行 52〜59: `fle_modules[]` の末尾に `"shcore.dll"`（N = 35。遅延モジュールの次）。**配列の末尾だけ**。
  - 行 190〜230 と 232〜310: `fle_imp_GetProcAddress` の**直前**に追加（`fle_ordinals[]`（198〜208）、`shell32_pidl_stop`、`FLE_PIDL_STUB` による 9 つの `fle_imp_IL*`）。`fle_imp_GetProcAddress`（232〜310）の変更は 2 か所だけ: (1) 先頭の序数の経路（`if ((np >> 16) == 0)` の中。仮想モジュールのハンドルで、表に行があれば名前に解決。それ以外は従来のメッセージで停止）、(2) 名前の読み取り（`if (ord_name != NULL) strcpy(name, ord_name); else { 従来のループ }`）。実在モジュールの経路（`fle_realmod_*`）と、ハンドルの検査は無変更。
- `lift/runtime/fle_modules.h`: 9 つの `fle_imp_IL*` の宣言（+11）。
- `lift/runtime/fle_load.c`（+3 −2。**PM がマージする箇所**）: 行 51〜61 の `fle_load_absent[]`（データ）とそのコメントだけ。`"shcore.dll"` を外した（9 → 8 名）。関数は無変更。
- `lift/runtime/fle_proc.c`（+21、`#include "fle_com.h"` 1 行を含む）、`fle_proc.h`（+5）: `fle_imp_GetProcessDpiAwareness`（末尾）。
- `lift/runtime/fle_locale.c`（+28、末尾）、`fle_locale.h`（+4）: `fle_imp_GetSystemDefaultLocaleName`。名前は `fle_locale_info_value(FLE_LC_SNAME, …)`（`GetLocaleInfoW(LOCALE_SNAME)` と同じ 1 つの出所）。
- `lift/runtime/fle_reg.c`（+12 −6）: `split_key` に引数 `allow_empty`（`RegOpenKeyEx` の `open_common` だけ 1。`RegCreateKeyExW`、`RegDeleteKeyW` は 0 = 従来どおり）。`open_common` の歩きで、要素の長さが 0 ならそのときに停止（到達しなければ停止しない）。
- テスト: 新規 `tests/wall_s.rs`（15 件）。更新した既存テストは下の「テスト」。
- 生成 C への影響: 下の「検証」。

## テスト

新規 `tests/wall_s.rs` 15 件（各壁の行に内訳）。期待値は決定 0099、決定 0015（ハンドル = `0x7FF800000000 + 0x10000 × N`）、Learn のページ（GetProcessDpiAwareness、GetSystemDefaultLocaleName、RegOpenKeyExW、GetProcAddress、遅延ロードの例外コードのページ）から書き、C のコードから読み戻していない（序数 9 個の名前は Wine の `shell32.spec`、非公式）。

- 生成器: 新しい 11 行が応答表の**連続した 11 行**（`GetSystemDefaultLocaleName` から。統合後は後ろに B の 9 行が続く）で、順序どおり（擬似 VA は行ごとに 0x10 ずつ増える。古い行は動かない）、`fle_imp_*` のプロトタイプが出る。別の DLL 名の同名 API には行が無い。
- runtime: 模擬 PE の生成 C と全 `fle_*.c` をリンクしたドライバ（`tests/msctf.rs` と同じ形）で、`GetProcAddress` が返す擬似 VA を `fle_dispatch` で呼ぶ。
- レジストリ: `fsx_common`（`reg.rs` と同じ形）。
- 実 DLL（`FLEngine_x64.dll` が無ければ飛ばす）: 遅延 import 記述子（57 関数、`Shcore.dll` は `GetDpiForMonitor` と `GetProcessDpiAwareness` の 2 つだけ、スロット 0x1976490／0x1976398）、`f_712080` のバイト列（序数 9 個の `mov edx, imm32` + `call` と、文字列 `shell32.dll`）。
- **変異検査**（実施。元に戻して `cmp` で一致を確認。ログ `logs/mut-*.txt`）: DPI の値を 2 にする → 1 件失敗、`shcore.dll` を表から外す → 4 件失敗、`GetSystemDefaultLocaleName` の最小サイズを 1 小さくする → 1 件失敗、序数 18 を `ILFree` に取り違える → 2 件失敗、序数の経路でモジュール名を見ない → 1 件失敗、`RegOpenKeyEx` を従来の `split_key`（空要素で即停止）に戻す → 1 件失敗、空要素に到達したときの停止を 2 に置き換える → 1 件失敗（7 件とも検出）。

**更新した既存テスト**（原因はどれも「`shcore.dll` が仮想モジュールになった」で、新しい期待値は決定 0099 から書いた。統合後の木では、遅延モジュール（決定 0096）が N = 33、34 を使っているので、shcore は N = 35（index 34）、「表の外」は index 35（N = 36）になる。下の表は統合後の値。S 単独の木での旧→新は、括弧の中に S 単独の値を書く）:

| ファイル | 行 | 内容 |
|---|---|---|
| `tests/modules.rs` | `name_rows()` | `shcore.dll` が「見つからない」→ N = 35（index 34。S 単独: N = 33、index 32）。`SHCore`、`Shcore.dll` の行と `shcore2.dll`（見つからない）を足した |
| `tests/modules.rs` | `module_handles_follow_the_formula_and_are_distinct` | `count 34` → `count 35`、`lines[1..35]` → `lines[1..36]`、`lines[35]` が `shcore.dll`、`lines[36]` が表の外（S 単独: 32 → 33） |
| `tests/modules.rs` | `undefined_cases_abort` の `handle-past-table` | `h(34)` → `h(35)`（表の外のハンドルは 1 つ後ろへ） |
| `tests/msctf.rs` | シナリオ `table` | `fle_module_name(34)` → `fle_module_name(35)`、`count 34` → `count 35` |
| `tests/msctf.rs` | シナリオ `lack` と テスト `the_nine_lacking_names_still_fail_with_126` | 9 名 → 8 名（`shcore` を外す）、テスト名を `the_eight_lacking_names_still_fail_with_126` に |
| `tests/msctf.rs` | シナリオ `avrt-powrprof` | 末尾の absent の例を `shcore` → `uxtheme` |
| `tests/msctf.rs` | シナリオ `handle-35` → `handle-36` | 表の外のハンドル（N = 36） |
| `tests/load.rs` | `load_library_w_of_a_delay_only_dll_fails_with_126` | `"SHCore.dll"` を absent の例から外す（9 名） |
| `tests/load.rs` | `reference_counts_follow_load_and_free`、`avrt_and_powrprof_follow_the_free_library_rule_of_the_other_virtual_modules` | 「表の外」の参照数を `fle_load_refcount(34)` → `(35)`（0 を期待） |
| `tests/load.rs` | `avrt_and_powrprof_have_nothing_to_find_and_dwmapi_and_shcore_stay_absent` | 名前を `…_dwmapi_and_uxtheme_stay_absent` に、absent の例を `SHCore.dll` → `UxTheme.dll` |
| `tests/load.rs` | `avrt_and_powrprof_follow_the_free_library_rule…` 内の `past-the-new-end` | `fle_module_handle(34)` → `(35)` |
| `tests/deferred.rs`（決定 0096。**統合で加わった 4 か所**） | シナリオ `table`、`refs`、`free-past-the-table`、`gpa-past-the-table`、テスト `the_table_ends_with_the_two_deferred_modules_n33_and_n34` | `fle_module_name(34)` → `(35)`、`fle_load_refcount(34)` → `(35)`、`M(35)` → `M(36)`、`count 34` → `count 35`。テスト名を `the_table_has_the_two_deferred_modules_n33_and_n34_and_shcore_after_them` に（N = 33、34 の遅延モジュールの検査は変えない） |
| `tests/wall_s.rs`（新規ファイルの中の統合時の調整） | `N_SHCORE`（C と Rust）、シナリオ `shcore-module` の `fle_load_refcount(32)`／`fle_module_name(32)` | 33 → 35、32 → 34、期待する行 `count 33 name32 shcore.dll` → `count 35 name34 shcore.dll`、テスト名 `shcore_is_the_33rd_module…` → `shcore_is_the_35th_module…`、「最後の 11 行」の検査を上の位置の検査に |

固定数を決め打ちして**壊れた**のは、S 単独の木では上の表の 11 行だけ（全体テストで確認）。統合後の木では、遅延モジュールの `deferred.rs` が同じ理由で加わる（読みで見つけて直した。統合後の木は未コンパイル）。`reg.rs`、`realmod.rs`（序数の停止メッセージ `GetProcAddress by ordinal (lpProcName 0x1) is not supported`）、`census.rs`、`imports_table.rs` ほかは変更なしで通る。

## 検証

手順と生の出力は `lift/out-wallS/`（`logs/`、`runs/`、`test-ws*.txt`）。census の前後の詳細は `wall-s-measurement.md`。

| 項目 | 結果 |
|---|---|
| `cargo test --workspace --no-fail-fast`（`CARGO_TARGET_DIR=lift/out-wallS/target`、`FLENGINE_PE` 指定、`src` の木） | **100 binaries / 1,252 passed / 0 failed / 0 ignored**。基準 = 98 binaries / 1,228 passed（HEAD 単体）+ census の 9 件 + `wall_s.rs` の 15 件 = 100 / 1,252 で**一致**。**1 回の実行では完了していない**: 1 回目（`test-ws.txt`）の途中で**ディスクが満杯になり**（`/` が 100%、空き 1.7〜2.4 GB。他の作業の書き込みによる。この作業の全成果物は 8 GB）、`tailjmp`・`tid`・`time`・`uilang`・`ver` が `No space left on device` で失敗し、以降の出力が切れた。`test-ws.txt` には 71 binaries（955 件）が `ok` で入っている。残りを同じ木・同じ環境変数で分けて再実行し、すべて ok: `test-ws-rest1.txt`（tailjmp 4、tid 7、time 24、uilang 4、ver 2、verify 3、versioninfo 3、vex 13 = 8 binaries、60 件）、`test-ws-rest2.txt`（vfs 22、vmap 23、vol 22、wall_s 15、wasm_port 6、window 36、x87 14、x87_trans 9、x87ctl 7 = 9 binaries、154 件）、`test-ws-rest3.txt`（fe-pe の lib／main／7 つの integration／doc-test = 10 binaries、83 件）、`test-ws-rest4.txt`（doc-test 3。`fe_pe` は rest3 と重複）。合計 71 + 8 + 9 + 10 + 2 = 100 binaries、955 + 60 + 154 + 83 = 1,252 件 |
| FLEngine の生成 C の SHA-256 | **`62146e105d4f0fea2bff31b36aa5fa29649e6949bff1a23bc2e3994af79207d9`**（旧 `feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f`）。215,709,232 → 215,711,199 バイト（+1,967）。functions 59,297、bad_nodes 0 のまま。`unsupported_total` 525 → 524（IAT thunk 524 → 523）、`jmp_imp:ILFree` 1。`fle_dispatch_count` 59,565 → 59,576、`fle_resp_count` 268 → 279 |
| 旧 C との diff（`runs/fle-c-diff.txt`） | 6 つの差分ブロック: (1) 11 個のプロトタイプ（`void fle_imp_…(void);`）、(2) `fle_dispatch_table` の 11 行、(3) `fle_dispatch_count`、(4) `fle_resp_table` の 11 行、(5) `fle_resp_count`、(6) **関数本体の変化は 1 行**: `L_593410: { /* 0x593410 Jmp */ fle_unsupported(UINT64_C(0x593410)); }` → `{ void fle_imp_ILFree(void); fle_imp_ILFree(); return; }`（`shell32.dll!ILFree` の IAT thunk。FLEngine の通常の import が応答表の新しい行に当たった）。**テーブルだけ、という予想とは違い、本体が 1 関数（thunk）変わる**。どちらも停止する（`unsupported instruction at 0x593410` が `shell32!ILFree: not answered` になる） |
| dsp_ippv2（縮小。種 452 個、基底 0x2000000）の生成 C | SHA `24305e0c…a025`（旧 `45690ae2…ad13`）。diff（`runs/real-c-diff.txt`）は応答表と dispatch／resp の個数だけ（37 行）。functions 8,030、bad_nodes 0 のまま |
| census 無効の a4（単独） | init_count **145/942**、last unit 144、functions **821**、entries **15,685**、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`。**基準と一致** |
| census 無効の a4（FLEngine + 縮小 dsp_ippv2） | init_count **150/942**、last unit 149、functions **943**、停止 `LoadLibraryW "blend2d.dll"`。**entries は 26,585 → 26,603（+18）**。基準と一致しない点がこれ。原因（実測）: unit 145（`f_65c9f0`）が自前で `LoadLibraryW(L"SHCore.dll")` と `GetProcAddress(h, "GetDpiForMonitor")` を行い、`shcore.dll` が仮想モジュールになったため成功の経路（NULL と 127 を受けて「関数が無い」に進む）に入る（W1 の表の「副作用」）。最初の 256 エントリのログと停止点は一致。census の unit 145 の入口数 422 → 440 と同じ +18 |
| census（dsp 構成。診断。A4 の進捗ではない） | ok **876 → 878**、stop **19 → 17**（fault 0、hang 0、empty 46、noproc 1）。unit 238、265 が ok。unit 245、830 は壁を過ぎて次の壁へ。unit 314 は変化なし。2 回実行して、経過時間の列を除いて同一（再現性） |
| census（単独構成） | ok **873 → 875**、stop **22 → 20** |
| 参考実験（W4。パッチに含めない） | イメージへの `VirtualProtect` を成功させる使い捨てビルドで unit 314 が ok、ok 879／stop 16 |

census の数は A4 の進捗ではない（決定 0046、0060、0097）。

## 解釈が分かれた点

1. **`shcore.dll` を仮想モジュールにするか、遅延ヘルパーが `RaiseException` するのを許すか**: 決定 0015／0029 は `shcore.dll` を「持たない名前」にした。これは `GetModuleHandleW` が 126 を返す（遅延 import の DLL は DllMain の時点では未ロード）という PM の決定を、作業者が `LoadLibrary` に広げたもの。Windows 10（決定 0014）には shcore.dll がある。**ゲストの遅延 import は NULL を許さない**（Learn: 遅延ロードの失敗は未処理の例外）。そこで、決定 0088（avrt／powrprof）と同じ形（仮想モジュール、応答は行がある関数だけ）にした。`GetModuleHandleW("shcore.dll")` が成功するようになる副作用は、FLEngine にその呼び出しが無い（文字列 `Shcore` は記述子の 1 か所だけ）ので影響しない。unit 145 の自前のロードには影響する（上の「副作用」）。
2. **DPI awareness の値**: 0（UNAWARE）を選んだ。2（PER_MONITOR）にするとゲストは `SetThreadDpiAwarenessContext`、`GetDpiForMonitor` ほか（どれも応答なし）へ進む。実際のホスト exe の宣言は分からない（仮の決定。撤回条件は W1）。
3. **序数の表に Wine の `shell32.spec`（非公式）を使った**: Learn に序数の表が無い。9 個の序数が PIDL 関数の群にちょうど一致する、という強い整合はあるが、実際の Windows 10 の shell32 では確認していない。表の外の序数、表を持たないモジュールは停止のまま（NULL と 127 にはしない）にした。ゲストの `GetProcAddress` は 9 個とも NULL 検査をしないで保存し、後で読むのは `ILClone` の 1 つだけ（静的な読み）。
4. **`ILxxx` の 9 関数は名前付き停止**: 実装（PIDL のモデル）は範囲外。行があるので、`GetProcAddress` は NULL でなく擬似 VA を返す（Windows 10 にあるものを「無い」と言わない）。`ILFree` は通常の import でもあるため、生成 C の thunk 1 関数の本体が変わった（検証）。この副作用を避けるには `ILFree` を表から外し、序数 155 の解決を別の仕組みにするしかないが、そうしなかった。
5. **W4 を決めない**: 「`VirtualProtect` に成功と答える」のは API としては正しい答えだが、ゲストはその直後に自分のコードを書き換える。リフトされたプログラムはそれを実行しないので、答えるだけで**フックが黙って無効になる**。census の範囲では、フック対象は有効化の後に呼ばれないが、これは証明ではない。PM の判断（選択肢 A〜E）に残した。
6. **W5 の「到達したときだけ停止」**: 先頭の `\` は（ベースキーが前のキーなので）常に停止、二重の `\` と存在するキーの後ろの末尾の `\` も停止。末尾の `\` を受理する実装が普通なら、停止ではなく受理にすべき（未確認）。今回のゲストの呼び出しは、停止も受理も要らない（`Registrations` が無い）。
7. **`census` の分類**: unit 830 の次の壁（`indirect call to 0`）を census は「e: null 呼び、二次の疑い」にした。実際は**一次**: ゲストが `GetProcAddress(SHFolder.dll, "SHGetFolderPathW")` を NULL 検査せずに呼ぶ。この `GetProcAddress` が NULL と 127 を返すのは、応答表に `shfolder.dll!SHGetFolderPathW` の行が無いため（測定の文書）。

## 限界

- **実機の Windows と照合していない**（ホストは Linux）。応答はすべて仮の決定。
- **各 unit で見えるのは最初の壁だけ**。unit 245、830 の次の壁の先は見えない。unit 238、265 は ok（戻った）だが、census の ok は「戻った」だけで、放棄した unit の後の状態で動いている（決定 0097）。
- **同じ種類の壁が 52 件残る**（遅延 import。上の「先に結論」）。この決定は census が示した 2 件だけを直した。absent 8 名を仮想モジュールにするか、遅延 import ごとに応答するか、遅延ヘルパーに失敗フック（`__pfnDliFailureHook2`）を持たせるか（`0x15ec2c0` に値を入れると `RaiseException` の前に戻れる。ゲストのコードを変える形ではなく、環境が関数ポインタを置く形になる）は PM の方針。
- `GetProcessDpiAwareness` の値、`GetSystemDefaultLocaleName` の `cchLocaleName <= 0` の扱い、レジストリの空要素の扱い、序数 9 個の名前、はいずれも Learn にも実機にも根拠が無いか、非公式。
- W4 の参考実験は census の範囲（放棄した unit の先は見えない）の観察で、フックの影響が無いことの証明ではない。
- 全体テストは 1 回の実行では完了していない（ディスクの満杯。検証の表）。分けて再実行した結果の合計。
- 応答表の行は、他のレーン（R、L）の行と末尾で重なる。マージ後の擬似 VA は並べ方で決まる（新しい 11 行の位置を固定するテストは、マージ後に直す）。
- native のみ。wasm は使っていない。`GetProcessDpiAwareness` ほかの wasm での動作は未確認。

## 限界

（実装の後に記入）
