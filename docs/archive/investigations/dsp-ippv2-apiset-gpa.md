# dsp_ippv2_x64.dll の CRT が GetProcAddress で引く名前と、api-ms-* の仮想モジュールが NULL を返す影響（調査）

実行日: 2026-10-24 JST（UTC では 2026-10-23 18:15〜20:10）。**調査のみ。この文書は何も決めない**。決定が要る点は §8。fe-lift・fe-decode・fe-pe のソース、lift/runtime、docs/decisions、既存のテスト、既存の docs は編集していない。作業物は `lift/out-apiset-gpa/`（gitignore 対象）。

背景は `two-module-post0083.md` §3〜§5、`dsp-ippv2-dynamic-imports.md`、決定 0015、0078、0081、0082。懸念（検証対象の仮説。結論ではない）は「実機なら `GetProcAddress` が非 NULL を返し、DLL の CRT は Ex 系の経路を取る。現在は NULL なので代替経路に入り、ATTACH が `LCMapStringW`（Locale=0）で止まるのも、その結果かもしれない」である。

## 0. 凡例

- **実測** = このセッションでコマンドを実行して得た値。ログは `lift/out-apiset-gpa/`（`run/`、`*.out`）。
- **読み** = capstone の逆アセンブル、生成 C、runtime のソースを読んだ結果。実行して確かめていない。
- **仮の値** = gdb が一時的に入れた値。**決定でも runtime の挙動でもない**。runtime には何も入れていない。
- **未確認** = 確認できていない。
- **一次に無い** = 取得した Learn のページに書かれていない。
- 引用: `> [Learn: <ファイル>] "原文"` の行だけが Learn の原文で、`lift/out-apiset-gpa/learn/<ファイル>` に実在する文だけを書いた（`check_doc_quotes.py` で機械照合。§9）。取得日は 2026-10-23 UTC（`learn/FETCHED_AT.txt` は最初の取得 18:15:31Z。最後のページは 20:10Z ごろ）。各 `.html` の SHA-256 は `learn/MANIFEST.sha256`。
- 「**APILocation**」= ページ先頭のメタデータ `<meta name="APILocation">`（`.html` の中。`.txt` には出ない）。取得ページはこの欄の意味を説明していない。表示される Requirements 表の「DLL」欄とは別物なので、区別して書く。
- 「Windows 10 に存在するか」は、Learn が**そう書いているか**と、Requirements の最小 OS からの**導出**を分けて書く（決定 0014 の環境は Windows 10 22H2。決定 0055、0078 と同じ導き方）。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | DLL が `GetProcAddress` に渡す名前 | **9 種 + `CorExitProcess`**。呼び出し点は 3 か所（CRT のラッパー A の中、ラッパー B の中、`CorExitProcess` 用の 0x17d1af5）。`dsp-ippv2-dynamic-imports.md` の一覧と**一致し、足す名前は無かった**（バイト走査で `GetProcAddress`、`LoadLibraryExW` の参照を全部拾い、ラッパー A、B の呼び元は 5 + 5 か所だけ。関数ポインタ経由の呼び出しなし） | 実測（走査）＋読み |
| 2 | ATTACH で停止までの `GetProcAddress` | 4 回、すべて NULL。hModule は `api-ms-win-core-synch-l1-2-0`（N=27、0x7ff8001b0000）が 2 回、`api-ms-win-core-fibers-l1-1-1`（N=28、0x7ff8001c0000）が 2 回。名前は `InitializeCriticalSectionEx` ×2、`FlsAlloc`、`FlsSetValue` | 実測（読み取りだけの gdb） |
| 3 | 4 回を非 NULL にしたら（仮の値 1 段。§5 の B） | 停止は**同じ**（`MultiByteToWideChar`。881 / 15,934。import 125 回）。変わったのは、`TlsAlloc` と `TlsSetValue` が各 1 → 0 回、`FlsAlloc` が 1 → 2 回、`FlsSetValue` が 2 → 3 回、`InitializeCriticalSectionAndSpinCount` は 79 回のまま（ただし**スタンドインとして同じ行の擬似 VA を返したため**。Ex 専用の行ではない）。関数エントリ +159（**内訳は未確認**: §5.2） | 実測（仮の値つき） |
| 4 | `InitializeCriticalSectionAndSpinCount` 79 回の内訳 | 1（ラッパー A5 の経路。DllMain の鎖）+ 78（ラッパー B3 の経路。0x17d5678 が 14、0x17d7cdc が 64）。Ex 経路なら 79 回が `InitializeCriticalSectionEx(lpCS, 0xfa0, 0)` の呼び出しになる。Flags は 79 回すべて 0（B で実測） | 実測＋読み |
| 5 | `LCMapStringW(Locale=0)` の経路 | **経路に依存する**。`LCMapStringEx` が NULL（現在）のときだけ、`LocaleNameToLCID` を引き（NULL）、名前 NULL → LCID 0 → `LCMapStringW(0, …)`。`LCMapStringEx` が非 NULL なら、`LocaleNameToLCID` の引きも `LCMapStringW` も呼ばれず、`LCMapStringEx(NULL, 0x100, src, 256, NULL, 0, NULL, NULL, 0)` が呼ばれる（C1 で実測: 131 回で偽の VA の呼び出しに止まる） | 実測（仮の値つき）＋読み |
| 6 | `Locale=0` の由来 | 呼び出し点 0x17d674c（setSBUpLow）は code page 1252 のとき**ロケール名 NULL** を渡す（読み。名前を持つのは 932、936、949、950 だけ）。NULL を `LocaleNameToLCID` の代替 0x17d9f1c に渡すと `xor eax,eax` で 0 | 読み |
| 7 | 非 NULL にするための runtime の機構 | **既にある**（`fle_resp_table` の `dll` 欄がモジュール名と一致する行 → 擬似 VA → `fle_dispatch`）。`api-ms-*` の `dll` 欄の行を足せば足りる（決定 0081 の「別の決定」）。B は既存行の擬似 VA をスタンドインに使い、機構が api-ms ハンドルの呼び出しでも動くことを実測 | 読み＋実測 |
| 8 | 足りない `fle_imp_*` | `InitializeCriticalSectionEx`、`LCMapStringEx`、`LocaleNameToLCID`（任意で `AreFileApisANSI`、`AppPolicyGetProcessTerminationMethod`）。`InitializeCriticalSectionAndSpinCount`、`Fls*`、`Tls*`、`LCMapStringW`、`GetLocaleInfoW` は既にある（§6） | 読み |
| 9 | 仮説の評価 | 「現在の NULL が代替経路に入れている」は**支持される**（実測）。「`Locale=0` の停止は代替経路の産物」も**支持される**（実測。非 NULL の経路には `LCMapStringW(0)` が無い）。ただし「実機で非 NULL が返る」こと自体は **Learn に書かれておらず未確認**（§4.4）。また、非 NULL にしても `LCMapStringEx` に渡る 256 単位の大文字小文字変換は、runtime の Unicode 表の無さ（0x80 以上）に当たる（読み。§5.3） | 実測＋読み |

## 2. HEAD、入力の SHA-256、手順

### 2.1 HEAD

- 作業開始時の HEAD は `8771100`（依頼の記載と一致。フル SHA `877110073bbb56941c2692ecce6ab7e6b505244a`、ブランチ main）。**作業中に main が進んだ**: 終了時の HEAD は `9b65b6696af583bbd5edfa50bda693bdc32e0dba`（途中のコミット: f8b5e6d、47118ac、d43db3e、27c8068、9b65b66）。27c8068 より後の 9b65b66 は docs 1 ファイル（`quickfontcache-lift-feasibility.md`）だけで、runtime、crates、harness は変えていない（`git diff --stat 27c8068 HEAD`）。以下の行番号は 27c8068 で確認したもので、9b65b66 でも同じ（根拠にしたファイルは両方で差が無い）。
- `git diff --stat 8771100 HEAD -- lift/runtime lift/crates lift/harness` は 4 ファイルだけ: `lift/runtime/fle_locale.c`（+26）、`lift/crates/fe-lift/tests/locale.rs`、`lift/runtime/test/cp1252_expect.txt`、`lift/runtime/test/gen_cp1252_expect.py`（コミット 27c8068、決定 0084。**私の作業ではない**）。
- この調査が根拠にする `fle_load.c`、`fle_modules.c`、`fle_imports.c`、`fle_sys.c`、`imports_table.rs` は、8771100 と HEAD で差が無く、ワークツリーも HEAD と同じ（`git diff --stat` が空。確認時点）。行番号は 27c8068（= 9b65b66 の runtime）のもの。`fle_locale.c` だけは 0084 を含む（`fle_imp_GetStringTypeW` :150、`fle_imp_IsValidCodePage` :160、`fle_imp_GetLocaleInfoW` :412、`fle_imp_MultiByteToWideChar` :546、`fle_imp_WideCharToMultiByte` :658、`fle_imp_LCMapStringW` :746）。
- **測定に使った実行ファイルは 0084 の前の runtime で作られている**（§2.2）。HEAD の runtime で作り直した実行ファイルではない。

### 2.2 入力（実測）

| 物 | 値 |
|---|---|
| `lift/out-ext/dsp_ippv2_x64.dll`（39,942,656 B） | SHA-256 `497dd4f77e859df9e86d999b3811be3c72526b41f86c38b0c3c6e395f691c352`（決定 0078、`gen_two.sh` の期待値と一致） |
| `lift/out-two-post0083/dll/dsp_ippv2_x64.2000000.dll`（リベース済み） | SHA-256 `4fd255027525f2a2e56a93fe1cc7f95442861f788179a23ac8b0401b8f4fefc5`（two-module-post0083.md と一致） |
| `binaries/win64/FLEngine_x64.dll` | SHA-256 `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d` |
| `lift/out-two-post0083/two/v-tbl2/native/two-native`（571,226,640 B） | SHA-256 `21370d7e092c923f70b2a7f9ef8fb32e5b56f29e906023d7aa90ffcba3e0760e`。2026-10-24 00:27 JST に `two-module-post0083.md` の作業（HEAD 7a9af0f）が作ったもの。**私は作り直していない**。0084 の前の runtime である証拠: gdb で `fle_cp1252_hi` のシンボルが無い。A の停止が `MultiByteToWideChar: byte of 0x80 or above …` で、0084 後の停止（`GetStringTypeW`）と違う |

### 2.3 手順（実測。実行した順。cwd は `lift/`、TMPDIR は常に `lift/out-apiset-gpa/tmp`、`PYTHONDONTWRITEBYTECODE=1`）

1. 先に読んだ物: §の冒頭の 2 文書、決定 0015、0078、0081、0082、`fle_load.c`、`fle_modules.c`、`fle_imports.c`、`fle_sys.c`、`fle_locale.c`、`imports_table.rs`。
2. Learn の取得: `curl` で `learn/*.html` を取り、`learn_text.py`（標準ライブラリだけ）で `.txt` を作った。404 だった 3 件（`winnls/nf-winnls-getstringtypew`、`…getstringtypeexw`、`intl/constants-used-in-national-language-support`）は `_404-*` に改名して残した（GetStringTypeW の正しい場所は `stringapiset/nf-stringapiset-getstringtypew`）。
3. DLL の読み取り専用の走査（`lift/out-apiset-gpa/`）:
   - `gpa_scan.py`: `LoadLibraryExW`、`GetModuleHandleW`、`GetModuleHandleExW`、`GetProcAddress`、`FreeLibrary`、`GetModuleFileNameW` の IAT スロットへの rip 相対参照（4 通りの整列、tail 0〜4）と、ラッパー A（RVA 0x17d0788）、B（0x17d72fc）の `call`/`jmp` rel32 の呼び元、引数の復号（出力 `gpa_scan.out`）。
   - `gpa_scan2.py`: セクションの一覧、スロットとラッパーへのデータポインタ、ラッパーへの別形式の参照、API 名の ASCII 文字列（出力 `gpa_scan2.out`）。
   - `gpa_scan3.py`: スロット RVA の裸の dword（`[base+disp32]` 形）、スロット VA の qword、`.reloc` の DIR64（15,602 件）の標的（出力 `gpa_scan3.out`）。
   - `gpa_decode.py`: 10 個の呼び元の名前表 A（3 名）、B（19 名）、API 名、モジュール添字のリスト（出力 `gpa_decode.out`）。
   - `rdis.py`、`xref_fn.py`、`imp_by_caller.py`、`final_checks.py`: 範囲の逆アセンブル、関数の呼び元の探索、gdb ログの呼び出しを呼び元関数ごとに集計、最終確認（出力 `xref_fn*.out`、`imp_by_caller.out`、`final_checks.out`）。
4. gdb（`lift/harness/two/gdb_imports.py` を**無改変**で使用。gdb スクリプトは `lift/out-apiset-gpa/gdb/`）。環境変数は `lift/harness/two/chain.sh` と同じ（`GDBI_PATCHES`、`GDBI_BIN=…/two-native`、`GDBI_REAL=…/dsp_ippv2_x64.2000000.dll`、`GDBI_FLE=binaries/win64/FLEngine_x64.dll`、`GDBI_OUT`、`GDBI_STDOUT`、`GDBI_STDERR`）。実行は `gdb -q -batch -x harness/two/gdb_imports.py`。

   | 名前 | スクリプト | 書き換え | 段数（1 段 = 1 つの API。`two-module-post0083.md` §5 と同じ数え方） |
   |---|---|---|---|
   | **A** | `patches_gpa_probe.py` | **無し**（`LoadLibraryExW`、`GetProcAddress`、`GetModuleHandleW`、`FreeLibrary` の引数と戻り値を読むだけ） | 0 |
   | **B** | `patches_gpa_stage.py`、`POST_MAX=0` | `GetProcAddress` の戻り値だけ（api-ms の 5 名） | 1 |
   | **C1** | 同、`POST_MAX=2`、`GPA_NAMES=LCMapStringEx` | `MultiByteToWideChar`、`GetStringTypeW`、`GetProcAddress`（`LCMapStringEx` の 1 名だけ） | 3 |
   | **C2** | 同、`POST_MAX=2` | 上 2 つ + `GetProcAddress`（6 名） | 3 |
   | **D** | 同、`POST_MAX=2`、`GPA_NAMES=__none__` | `MultiByteToWideChar`、`GetStringTypeW` だけ（GPA の規則は何にも当たらない。`two-module-post0083.md` §5.2 の再現） | 2 |

   **段数の注意**: `GetProcAddress` の段は 1 つの API（`GetProcAddress`）に対する 1 つの表引きの規則（名前 → 戻す値）で、B と C2 では名前ごとの分岐が 5、6 ある。数え方を「API ごとに 1 段」（`two-module-post0083.md` §5）にすれば最大 3 段の範囲に収まる。「規則（分岐）ごとに 1 段」と数えるなら B と C2 は 3 段を超える。どちらの数え方かは依頼者が決める事項として残す（§9 の未確認にも書いた）。4 つ目の停止（`WideCharToMultiByte`）は書き換えていない。
5. `check_quotes.py`、`check_doc_quotes.py`: 引用の機械照合（§9）。

## 3. DLL が `GetProcAddress` で引く名前（実測の走査＋読み）

### 3.1 名前の一覧は `dsp-ippv2-dynamic-imports.md` と一致した（実測）

- DLL の import は **KERNEL32.dll の 1 記述子だけ**（72 名。`LoadLibraryW`、`LoadLibraryA`、`LoadLibraryExA`、`GetModuleHandleA`、`GetModuleHandleExA`、`LoadPackagedLibrary` は無い）。delay import、TLS、CLR のディレクトリは空（`gpa_scan2.out`）。
- `GetProcAddress` の IAT スロット（RVA 0x1ebd178）への rip 相対参照は、バイト走査で **0x17d0899、0x17d1af5、0x17d7489 の 3 か所**（ほかに 0x33acd5 が 1 件出るが、直前のバイトが modrm にならない偽陽性）。`LoadLibraryExW`（0x1ebd180）は 0x17d080f、0x17d0847、0x17d73a1、0x17d73f5 の 4 か所（0x1dbd07e は偽陽性）。スロット RVA を裸の dword として持つ `[base+disp32]` 形、スロット VA の qword、`.reloc` の DIR64（15,602 件）の標的としての参照は **0 件**（`gpa_scan3.out`）。
- ラッパー A（RVA 0x17d0788）、B（0x17d72fc）への参照は rel32 の `call` だけで **5 + 5 か所**。`lea reg,[wrapper]`、データ中の関数ポインタ、`.reloc` の標的としての参照は 0 件（`final_checks.out` §1）。したがって `GetProcAddress` に渡る名前は、10 個の呼び元の引数（rdx）に出るものと `CorExitProcess` だけ（読み。名前を計算で作る呼び出し点は無い）。
- 名前の文字列としての ASCII 走査（`gpa_scan2.out` D）: `GetLocaleInfoEx`、`GetUserDefaultLocaleName`、`GetSystemDefaultLocaleName`、`IsValidLocaleName`、`EnumSystemLocalesEx`、`GetDateFormatEx`、`GetTimeFormatEx`、`CompareStringEx`、`LCIDToLocaleName`、`GetStringTypeExW`、`GetLocaleInfoW` など依頼が挙げた名前を含む、探した 74 名のうちの **50 名は、ファイル中のどこにも無い**（残る 24 名は、DLL が実際に引く名前、IAT の名前、`GetSystemTimeAsFileTime` などのもの）。DLL の CRT がロケールのために引く名前は `LCMapStringEx` と `LocaleNameToLCID` だけ。
- 参考（読み）: `FlsAlloc`、`FlsFree`、`FlsGetValue`、`FlsSetValue`、`TlsAlloc`、`TlsFree`、`TlsGetValue`、`TlsSetValue`、`InitializeCriticalSectionAndSpinCount`、`LCMapStringW`、`GetStringTypeW` は IAT に**静的に**もある（KERNEL32.dll）。IAT スロットへの参照を数えると、`Tls*` 4 名と `InitializeCriticalSectionAndSpinCount` は**それぞれ 1〜2 か所**（すべて A、B の代替の側。`final_checks.py` と同じ modrm 検査で確認: `InitializeCriticalSectionAndSpinCount` 0x17d0a51、0x17d75d4。`TlsAlloc` 0x17d0919、`TlsFree` 0x17d0962、`TlsGetValue` 0x17d09aa、`TlsSetValue` 0x17d09f2）。`Fls*` の IAT 参照は 0x17d7558〜0x17d7573 の 8 バイトのスタブ 4 つで、UCRT 側（0x17d33a8、0x17d3220 など）が使う。つまり **DLL は `Fls*` の静的 import を持っている**（その KERNEL32 は `FlsAlloc` を持たねばロードできない）。この事実は「api-ms-win-core-fibers-l1-1-1 のハンドルに対する `GetProcAddress` の戻り」を決めない。

### 3.2 ラッパーの規則（読み。逆アセンブルで再確認。0x17d0788 と 0x17d72fc）

`dsp-ippv2-dynamic-imports.md` §2.4 と同じ。要点だけ再掲する:

1. 関数スロットのキャッシュが非 0 かつ -1 以外ならそれを返す（B は cookie で復号）。-1 は「前に失敗」で 0。
2. モジュール添字のリストを順に見る。モジュールのキャッシュが 0 なら `LoadLibraryExW(名前, NULL, 0x800)`。NULL で last error が 0x57 かつ名前が `api-ms-`（B は `ext-ms-` も）で始まらないときだけ `dwFlags = 0` で再試行。それでも NULL なら -1 を入れて**次のモジュールへ**。
3. 得たハンドルに `GetProcAddress(ハンドル, 名前)`。**NULL なら関数スロットを -1 にして 0 を返す。リストの次のモジュールは試さない**（0x17d08af、0x17d7420）。非 NULL ならスロットに入れて返す。
4. 呼び出し側は、戻り値が非 NULL なら `call [0x1ebd258]` または `call [0x1ebd260]`（XFG 形の間接呼び出し。直前に `movabs r10, imm64` がある。r10 は runtime で使わない）、NULL なら代替を直接呼ぶ。`0x1ebd258` = RVA 0x17de9b0（`jmp rax`）、`0x1ebd260` = RVA 0x17de9d0（`jmp [0x1ebd258]`）。生成 C ではどちらも `fle_dispatch(R[0])` になる（`f_37de9b0`）。

### 3.3 名前ごとの表

「NULL の経路」「非 NULL の経路」は読み（逆アセンブル。`rdis.py`）。「ATTACH の直線」は §4 の実測（`gpa-imports.txt`）と呼び元の探索（`xref_fn*.out`）による。ハンドルの N は `fle_modules[]` の並び（`fle_modules.c:46-49`）と式 `0x7FF800000000 + 0x10000 × N`（決定 0015）による。実測したハンドル値には「実測」を付けた。

| # | 名前 | (a) 組になる `LoadLibraryExW` と hModule（試す順） | 呼び出し点（関数） | (c) 現行 runtime の `fle_imp_*` と応答表の行 | (d) NULL のときの経路（読み） | (d) 非 NULL のときの経路（読み） | (e) ATTACH の直線（§3.2 の 125 回の範囲） |
|---|---|---|---|---|---|---|---|
| 1 | `FlsAlloc` | A: `api-ms-win-core-fibers-l1-1-1`（N=28、**0x7ff8001c0000**、実測）→ `kernel32`（N=9） | 0x17d08f8（0x17d08d8 ← 0x17cd5a7 の 0x17cd59c） | `fle_imp_FlsAlloc`（`fle_imports.c:179`）。行 `kernel32.dll` あり（`imports_table.rs:284`）。api-ms 名の行は無い | 0x17d0911 → `jmp [IAT TlsAlloc]`（0x17d0916）。A では `fle_imp_TlsAlloc` | 0x17d0902 → `jmp [0x1ebd258]` → `f_37de9b0` → `fle_dispatch(R[0])`。B では `fle_imp_FlsAlloc` | **はい**（#9 `LoadLibraryExW`、#10 GPA、#11 `TlsAlloc`／B は `FlsAlloc`） |
| 2 | `FlsFree` | 同上（A の添字 0、2） | 0x17d0942（0x17d0920 ← 0x17cd5f3 の 0x17cd5e4） | `fle_imp_FlsFree`（:194）。行 `kernel32.dll` あり | 0x17d095f → `jmp [IAT TlsFree]`。**生成 C は `fle_unsupported(0x37d095f)`**（`TlsFree` は決定 0078 §3 のとおり応答表の行が無い） | `jmp [0x1ebd258]`。`fle_imp_FlsFree` は「値があり callback がある index」で名前付き停止 | **いいえ**（0x17cd5e4 は失敗・巻き戻し・DETACH の側。0x17cd5d7、0x17cd268、0x17cd280 から。ATTACH の成功経路には無い。読み） |
| 3 | `FlsGetValue` | 同上 | 0x17d098a（0x17d0968 ← 0x17cd489 の 0x17cd470、0x17cd509 の 0x17cd4dc） | `fle_imp_FlsGetValue`（:211）。行 `kernel32.dll` あり | 0x17d099b…0x17d09a7 → `jmp [IAT TlsGetValue]`（`fle_imp_TlsGetValue`） | `jmp [0x1ebd258]` | **照会は未到達**（125 回に A3 の GPA は無い。0x17cd470、0x17cd4dc の呼び元 0x17cd252、0x17cd23c、0x17cd4c4 は 0x17cb3a0 以降の別経路で、停止までの範囲には出ない） |
| 4 | `FlsSetValue` | 同上 | 0x17d09d9（0x17d09b0 ← 0x17cd5c0 の 0x17cd59c ほか 4 か所） | `fle_imp_FlsSetValue`（:222）。行 `kernel32.dll` あり | 0x17d09f0 → `call [IAT TlsSetValue]`（`fle_imp_TlsSetValue`） | 0x17d09e8 → `call f_37de9b0`（`fle_dispatch`）。B では `fle_imp_FlsSetValue` | **はい**（#12 GPA、#13 `TlsSetValue`／B は `FlsSetValue`） |
| 5 | `InitializeCriticalSectionEx`（A5） | A: `api-ms-win-core-synch-l1-2-0`（N=27、**0x7ff8001b0000**、実測、添字 1）→ `kernel32` | 0x17d0a35（0x17d0a04 ← 0x17d072f の 0x17d071c ← 0x17cd214 の 0x17cd210 ← 0x17cb4a6 の 0x17cb484） | **無い**（`fle_imp_InitializeCriticalSectionEx` は runtime のどこにも無い。応答表の行も無い） | 0x17d0a4f → `call [IAT InitializeCriticalSectionAndSpinCount]`（rcx、edx = spin 0xfa0。**Flags は捨てる**）。A では `fle_imp_InitializeCriticalSectionAndSpinCount`（`fle_sys.c:193`） | 0x17d0a44 `mov r8d,esi` → `call [0x1ebd258]`（Ex(rcx、0xfa0、Flags)） | **はい**（#6 `LoadLibraryExW`、#7 GPA、#8 `InitializeCriticalSectionAndSpinCount`）。静的な CS（RVA 0x25615c0）を 1 個 |
| 6 | `InitializeCriticalSectionEx`（B3） | B: `api-ms-win-core-synch-l1-2-0`（同上、添字 6）→ `kernel32`（添字 15） | 0x17d75a9（0x17d7578 ← 0x17d2b50 の 0x17d2aa4、0x17d5697 の 0x17d5678、0x17d7d23 の 0x17d7cdc） | 同上（無い） | 0x17d75cd → `call [IAT InitializeCriticalSectionAndSpinCount]` | 0x17d75b3 `movabs r10` → `call [0x1ebd260]` | **はい**（#14 `LoadLibraryExW`、#15 GPA、#16〜#108 の 78 回: 0x17d5678 が 14 回（#16〜#29。間隔 40 バイト、実測）、0x17d7cdc が 64 回（#45〜#108。間隔 72 バイト、実測））。0x17d2b50 は 125 回の範囲に出ない（rcx = 0x4543610 の呼び出しが無い） |
| 7 | `AreFileApisANSI`（B2） | B: **`kernel32` だけ**（N=9、0x7ff800090000。決定 0015 の表から。実測無し） | 0x17d752f（0x17d7514 ← 0x17d5fd4、0x17d60c9 の 0x17d5ecc、0x17d63ed の 0x17d6340） | **無い**（`fle_imp_AreFileApisANSI` 無し。`kernel32.dll` の行も無し） | 0x17d754e → `mov eax,1`（ANSI） | 0x17d7539 → `jmp [0x1ebd260]` | **停止の先**。0x17d6340 は 0x17d1dc9（0x17d1d74）から呼ばれ、`GetModuleFileNameW` の後、code page が 65001 でないとき 0x17d63ed で呼ぶ。0x17d1d74 は 0x17d6c5c（停止した `MultiByteToWideChar` を含む鎖）の**後**に 0x17d1dc9 へ進む。停止までの 125 回には出ない |
| 8 | `LCMapStringEx`（B4） | B: `api-ms-win-core-localization-l1-2-1`（N=29、**0x7ff8001d0000**、実測、添字 2）→ `kernel32` | 0x17d7621（0x17d75e8 ← 0x17d9bfb、0x17d9c53、0x17d9cfd の 0x17d9a94 ← 0x17d9e20 の 0x17d9db4 ← 0x17d6841、0x17d687a の 0x17d674c） | **無い**（`LCMapStringW` は有る: `fle_locale.c:746`、`imports_table.rs:55`） | 0x17d768d → 0x17d76d4（`LocaleNameToLCID` の引き）の戻り eax を ecx に置き、`call [IAT LCMapStringW]`（6 引数。version、reserved、sortHandle は捨てる） | 0x17d7638〜 スタックの 9 引数を並べ直して `call [0x1ebd260]` | **はい**（`MultiByteToWideChar`、`GetStringTypeW`、`MultiByteToWideChar` ×2 の後。D で #130 `LoadLibraryExW`、#131 GPA）。引数は §5.3 |
| 9 | `LocaleNameToLCID`（B5） | 同上（添字 2、15） | 0x17d76fd（0x17d76d4 ← 0x17d7692 の 0x17d75e8 **だけ**） | **無い** | 0x17d7721 → `call 0x17d9f1c`（DLL 内の二分探索。名前 NULL は 0x17d9f3a で `xor eax,eax` → 0） | 0x17d7716 → `call [0x1ebd260]` | **LCMapStringEx が NULL のときだけ**（#132 GPA。D の実測。LCMapStringEx が非 NULL なら呼ばれない: C1、C2 の実測で #131 の後に GPA が無い） |
| 10 | `AppPolicyGetProcessTerminationMethod`（B1） | B: **`api-ms-win-appmodel-runtime-l1-1-2` だけ**（N=30。式から 0x7ff8001e0000。実測無し） | 0x17d74df（0x17d74bc ← 0x17d5753 の 0x17d5730 ← 0x17d1a8c の 0x17d1a88 ← 0x17d1a5c の 0x17d1a54） | **無い** | 0x17d7509 → `mov eax,0xc0000225` | 0x17d74e9〜 `rcx = -6`、`rdx = 出力` → `jmp [0x1ebd260]` | **いいえ**（0x17d1a54 は `TerminateProcess` と `ExitProcess` を呼ぶ終了処理） |
| 11 | `CorExitProcess` | `GetModuleHandleExW(0, L"mscoree.dll", &h)`（`LoadLibraryExW` ではない） | 0x17d1af5（0x17d1ab8 ← 0x17d19f6 の 0x17d198c、0x17d1a78 の 0x17d1a54） | **無い**。`GetModuleHandleExW` は行があり `fle_modules.c:140` で名前付き停止（呼ばれたら abort） | `GetModuleHandleExW` が 0 なら飛ばす | `call [0x1ebd260]`（ecx = 終了コード） | **いいえ**。0x17d198c は `GetModuleHandleW(NULL)` の PE が .NET でないと 0x17d1ab8 に進まない（FLEngine の DataDirectory[14] = (0, 0)）。0x17d1a54 は終了処理の中で 0x17d1ab8 を**無条件に**呼ぶ（0x17d1a78）が、ATTACH の直線には無い |

- 名前は 10 種（`InitializeCriticalSectionEx` は 5 と 6 の 2 か所）+ `CorExitProcess`。`dsp-ippv2-dynamic-imports.md` §3.2 の表（A1〜A5、B1〜B5、G3）と**同じ**。
- 「(b) Learn の Requirements」は §4 の表にまとめた（列が多すぎるため）。

## 4. Microsoft Learn の記述（取得 2026-10-23 UTC。`lift/out-apiset-gpa/learn/`）

### 4.1 名前ごとの Requirements と APILocation（実測。取得ページの値）

「Minimum supported client」と「DLL」は表示される Requirements 表の値。「APILocation」は `.html` のメタデータで、**この欄が何を意味するか（どの OS 版にその名前の契約があるか）は取得ページに説明が無い**。

| 名前 | Minimum supported client / server | Requirements の DLL | APILocation に載る api-ms 名（メタデータの全値のうち、DLL が引く名前に関わるもの） | DLL が引く名前が載っているか |
|---|---|---|---|---|
| `InitializeCriticalSectionEx` | Windows Vista / Windows Server 2008 | Kernel32.dll | `API-MS-Win-Core-Synch-l1-1-0.dll`、`API-MS-Win-Core-Synch-l1-2-0.dll`、`API-MS-Win-Core-Synch-l1-2-1.dll`（ほか `Kernel32.dll`、`KernelBase.dll`、`API-MS-Win-DownLevel-Kernel32-l1-1-0.dll`、`MinKernelBase.dll`、`vertdll.dll`） | `synch-l1-2-0`: **載る** |
| `InitializeCriticalSectionAndSpinCount` | Windows XP / Windows Server 2003 | Kernel32.dll | 同上（`l1-1-0`、`l1-2-0`、`l1-2-1`） | `synch-l1-2-0`: **載る** |
| `FlsAlloc`、`FlsFree`、`FlsGetValue`、`FlsSetValue`（4 ページとも同じ） | Windows Vista / Windows Server 2003 | Kernel32.dll | `api-ms-win-core-fibers-l1-1-2.dll`、`API-MS-Win-Core-fibers-l1-1-0.dll`、`API-MS-Win-Core-fibers-l1-1-1.dll`（ほか `Kernel32.dll`、`KernelBase.dll`、`API-MS-Win-DownLevel-Kernel32-l1-1-0.dll`、`MinKernelBase.dll`） | `fibers-l1-1-1`: **載る** |
| `LCMapStringEx` | Windows Vista / Windows Server 2008 | Kernel32.dll | `api-ms-win-core-localization-l1-2-4.dll`、`…-l1-2-3.dll`、`API-MS-Win-Core-Localization-l1-1-0.dll`、`…-l1-2-0.dll`、`…-l1-2-1.dll`、`…-L1-2-2.dll`（ほか `Kernel32.dll`、`KernelBase.dll` など） | `localization-l1-2-1`: **載る** |
| `LocaleNameToLCID` | Windows Vista / Windows Server 2008 | Kernel32.dll | 同上に `api-ms-win-core-localization-l1-2-3.dll` を含む | `localization-l1-2-1`: **載る** |
| `AreFileApisANSI` | Windows XP（desktop apps only） | Kernel32.dll | `api-ms-win-core-file-l1-2-5.dll`、`…-l1-2-4.dll`、`…-l1-2-3.dll`、`API-MS-Win-Core-file-l1-2-2.dll`（ほか `Kernel32.dll`、`KernelBase.dll`、`MinKernelBase.dll`） | DLL は `kernel32` だけを試す。`kernel32`: 載る（`Kernel32.dll`） |
| `AppPolicyGetProcessTerminationMethod` | **Requirements 表に最小 OS の行が無い**（メタデータの `req.target-min-winverclnt` は空） | **Api-ms-win-appmodel-runtime-l1-1-2.dll** | `api-ms-win-appmodel-runtime-l1-1-7.dll` … `l1-1-2.dll`（6 つ）、`kernel.appcore.dll`、`kernel32.dll`、`kernelbase.dll` | `appmodel-runtime-l1-1-2`: **載る（Requirements の DLL 欄にも）** |
| `CorExitProcess` | Platforms: See System Requirements.（OS の行なし）。.NET Framework: Available since 1.0 | `MSCorEE.dll`（Library 行） | — | — |
| `GetProcAddress`（参考） | Windows XP / Windows Server 2003 | Kernel32.dll | `api-ms-win-core-libraryloader-l1-2-3.dll`、`API-MS-Win-Core-LibraryLoader-l1-1-0.dll` ほか | — |
| `LCMapStringW`（代替の側。参考） | Windows 2000 Professional | Kernel32.dll | `api-ms-win-core-localization-l1-2-4.dll`、`…-l1-2-3.dll`、`…-l1-1-0.dll`、`…-l1-2-0.dll`、`…-l1-2-1.dll`、`…-L1-2-2.dll`、`API-MS-Win-Core-misc-l1-1-0.dll` ほか | — |
| `GetStringTypeW`（参考。ATTACH の次の停止） | Windows 2000 Professional | Kernel32.dll | `API-MS-Win-Core-String-l1-1-0.dll` ほか | — |

### 4.2 Windows 10 に存在するか（一次に書かれている範囲と導出）

- **Learn は、上の API が「Windows 10 に存在する」とは書いていない**（各 API ページに "Windows 10" の語は無い。`LoadLibraryExW` ページの "Windows 8.1, Windows 10 or later" は `LOAD_LIBRARY_REQUIRE_SIGNED_TARGET` の記述で、API の存在とは別）。
- 導出（決定 0055、0078 と同じ導き方。Learn の記述ではない）: Requirements の最小 OS が Vista、Server 2003/2008、XP である 8 API（`InitializeCriticalSectionEx`、`InitializeCriticalSectionAndSpinCount`、`Fls*` 4 名、`LCMapStringEx`、`LocaleNameToLCID`、`AreFileApisANSI`）は、Windows 10 22H2（決定 0014）に存在する、と導ける（それ以降の OS で除かれたとは、どのページも書いていない）。
- `AppPolicyGetProcessTerminationMethod` は**この導出もできない**（最小 OS の行が無い）。`CorExitProcess` も OS の行が無い。
- 取得ページは、api-ms 名（`api-ms-win-core-synch-l1-2-0` など）が **LoadLibrary で解決できる OS 版**を書いていない（一次に無い）。API セットのページ（下）は一般論だけを書く。

### 4.3 API セットの一般論（原文）

> [Learn: windows-apisets.txt] "The info in this topic applies to all versions of Windows 10, and later."
> [Learn: windows-apisets.txt] "You can pass an API set name to LoadLibrary, or use it as a delay-load target. The operation succeeds when the schema on the current device maps that contract to a usable host; there isn't necessarily an actual file with that name anywhere on the PC. If the contract isn't mapped on the current device, a direct LoadLibrary fails."
> [Learn: windows-apisets.txt] "Either way, a successful link or load isn't by itself evidence that an implementation is present."
> [Learn: windows-apisets.txt] "Don't infer availability from the prefix."
> [Learn: windows-apisets.txt] "If the API belongs to an API set, the requirements table in the article lists the API set name and the Windows version in which the API was first introduced"
> [Learn: api-set-loader-operation.txt] "Your binary imports an API set contract name, or passes one to LoadLibrary."
> [Learn: api-set-loader-operation.txt] "The loader loads that host binary and binds the imported function to the host's export."
> [Learn: api-set-loader-operation.txt] "For a call through an API set to reach an implementation, all of the following have to hold:"
> [Learn: api-set-loader-operation.txt] "The contract is present in the schema on the current device."
> [Learn: api-set-loader-operation.txt] "The host exports the specific function your binary is calling."
> [Learn: api-set-loader-operation.txt] "A missing export is reported separately from a missing contract; a binary that imports a function the host doesn't export fails with a missing entry point error."
> [Learn: detect-api-set-availability.txt] "In some cases, a given API set contract name might be intentionally mapped to an empty module name on some Windows devices."
> [Learn: detect-api-set-availability.txt] "However, these are not a reliable means for testing API sets, because of reverse forwarding."
> [Learn: detect-api-set-availability.txt] "Don't use LoadLibrary or GetProcAddress as a substitute for the availability query."

### 4.4 これらの原文から**言えること**と**言えないこと**

言える（原文どおり）:

- API セット名は `LoadLibrary` に渡せ、スキーマが契約をホストに結ぶと成功する。
- ローダーは「契約 → ホスト DLL → ホストの export」と解決する。**ホストがその関数を export しているか**は別の条件（上の 3 条件）。

**一次に無い**（取得したどのページも書いていない）:

1. 実機 Windows 10 22H2 で `api-ms-win-core-synch-l1-2-0`、`-fibers-l1-1-1`、`-localization-l1-2-1`、`-appmodel-runtime-l1-1-2` が `LoadLibraryExW(…, NULL, 0x800)` で**解決するか**。
2. 解決したとき、そのハンドルに対する `GetProcAddress(ハンドル, "InitializeCriticalSectionEx")` などが**非 NULL を返すか**（上の「ホストが export しているか」に当たる）。API セットのページは `GetProcAddress` の戻りを明記しない。ページの原文は「`LoadLibrary` や `GetProcAddress` は API セットの可用性の検査に信頼できない」と言うだけで、戻り値を言わない。
3. `dwFlags = 0x800`（`LOAD_LIBRARY_SEARCH_SYSTEM32`）と API セット名の組み合わせの扱い。
4. 各名前が最初に導入された Windows の版（`APILocation` の意味は説明が無い。`windows-apisets` ページは「Requirements 表に API セット名と導入された版が載る」と書くが、8 API の Requirements の DLL 欄は Kernel32.dll で、api-ms 名は表に出ない）。

したがって懸念の前半（「実機では、これらの関数は存在する。実機なら非 NULL が返る」）のうち、**API が Windows 10 に存在すること**は導出できる（§4.2）が、**`GetProcAddress(api-ms ハンドル, 名前)` が非 NULL を返すこと**は、Learn からは**確認できない**。この調査は、実機の挙動を**事実として書かない**。

### 4.5 `GetProcAddress` ページの原文（runtime の応答規則との関係）

> [Learn: libloaderapi-getprocaddress.txt] "A handle to the DLL module that contains the function or variable. The LoadLibrary, LoadLibraryEx, LoadPackagedLibrary, or GetModuleHandle function returns this handle."
> [Learn: libloaderapi-getprocaddress.txt] "If the function fails, the return value is NULL."
> [Learn: libloaderapi-getprocaddress.txt] "design your application to handle the case when the function is not available"

ページは「API セット名のモジュールのハンドルに対して、どの関数に非 NULL を返すか」を書かない。決定 0015 の規則（応答表に行が無ければ NULL + 127）は、この点でページと矛盾しないが、ページが決める規則でもない。

## 5. 「NULL のときの経路」と「非 NULL のときの経路」で、ATTACH が呼ぶ import がどう変わるか

### 5.1 測定 A（読み取りだけ。実測）: 現行の NULL の経路

`lift/out-apiset-gpa/run/gpa-imports.txt`（`imp_by_caller.out`）。DllMain（f_37caca8）に入ってから停止まで **125 回、26 種**。**`two-module-post0083.md` §4 と回数、順序が一致**。`gpa-gdb.log` の `(probe)` 行:

| # | 呼び出し | 引数 | 戻り（runtime） | 呼び元（RVA） |
|---|---|---|---|---|
| 6 | `LoadLibraryExW` | `api-ms-win-core-synch-l1-2-0`、hFile 0、dwFlags 0x800 | 0x7ff8001b0000 | 0x17d0813（ラッパー A） |
| 7 | `GetProcAddress` | 0x7ff8001b0000、`InitializeCriticalSectionEx` | **NULL** | 0x17d089d |
| 9 | `LoadLibraryExW` | `api-ms-win-core-fibers-l1-1-1`、0、0x800 | 0x7ff8001c0000 | 0x17d0813 |
| 10 | `GetProcAddress` | 0x7ff8001c0000、`FlsAlloc` | **NULL** | 0x17d089d |
| 12 | `GetProcAddress` | 0x7ff8001c0000、`FlsSetValue` | **NULL** | 0x17d089d |
| 14 | `LoadLibraryExW` | `api-ms-win-core-synch-l1-2-0`、0、0x800 | 0x7ff8001b0000 | 0x17d73a5（ラッパー B） |
| 15 | `GetProcAddress` | 0x7ff8001b0000、`InitializeCriticalSectionEx` | **NULL** | 0x17d748d |

`api-ms-win-core-synch-l1-2-0` の `LoadLibraryExW` は 2 回目も 0x7ff8001b0000（キャッシュではなく、A と B で別のキャッシュ表（0x25615f0、0x2561e90）を持つため）。**`kernel32` の `LoadLibraryExW` は 1 回も呼ばれない**（NULL の `GetProcAddress` の後は次のモジュールへ進まない。§3.2 手順 3）。

125 回のうち、呼び元関数別（`imp_by_caller.out`）の要点:

| API | 回数 | 呼び元関数（RVA） |
|---|---:|---|
| `InitializeCriticalSectionAndSpinCount` | 1 | 0x17d0a04（A5。ret 0x17d0a55） |
| 同上 | 78 | 0x17d7578（B3。ret 0x17d75d8）。さらに 14 回は 0x17d5678 の 0x17d5697（#16〜#29）、64 回は 0x17d7cdc の 0x17d7d23（#45〜#108）から |
| `TlsAlloc` | 1 | 0x17cd59c（0x17cd5ac。`FlsAlloc` の代替） |
| `TlsSetValue` | 1 | 0x17d09b0（0x17d09f6。`FlsSetValue` の代替） |
| `FlsAlloc` | 1 | 0x17d33a8（**IAT の静的な `FlsAlloc`**。0x17d7558 のスタブ経由。UCRT の初期化表のスロット 0x1ec83d0 から） |
| `FlsGetValue` ×2、`FlsSetValue` ×2 | 4 | 0x17d3220（UCRT の errno 取得 0x17d37fc など）、0x17d317c |

つまり **現行の NULL の経路で DLL は「Fls と Tls の両方」を呼ぶ**: ラッパー A の 2 つ（`FlsAlloc`、`FlsSetValue`）が `Tls` に落ち、UCRT の別の初期化（0x17d33a8 と 0x17d3220）は IAT の静的な `Fls*` を直接呼ぶ。後者は NULL でも非 NULL でも**同じ**（ラッパーを通らない）。

### 5.2 測定 B（仮の値 1 段）: 4 回を非 NULL にしたとき

仮の値: 4 回の `GetProcAddress` に、**既存の応答表の行の擬似 VA**（gdb が実行中の `fle_resp_table` から読む）を返した。`InitializeCriticalSectionEx` → `kernel32!InitializeCriticalSectionAndSpinCount` の行の VA（0x7ff900000e40）、`FlsAlloc`、`FlsSetValue` → 同名の行の VA（0x7ff900000e70、0x7ff900000ea0）。**創作した値は「`InitializeCriticalSectionEx` → ICSASC の行」の対応だけ**（第 1・2 引数が同じ。第 3 引数 Flags は ICSASC の行が読まない）。

結果（`stageB-*`、`final_checks.out` §3）:

| 項目 | A（NULL） | B（非 NULL の仮の値） |
|---|---:|---:|
| 停止 | `MultiByteToWideChar`（byte ≥ 0x80） | **同じ** |
| import の回数 | 125 | **125**（位置ごとの比較: API 名が違う位置は #11 と #13 の 2 か所だけ） |
| 関数 distinct / エントリ | 881 / 15,934 | **881 / 16,093**（+159） |
| `InitializeCriticalSectionAndSpinCount` | 79 | 79（スタンドイン。guest の戻りが 0x37d75d8 → 0x37d75cb（78）、0x37d0a55 → 0x37d0a4d（1）に変わった: `call IAT` ではなく `call [0x1ebd2xx]` の側を通った証拠） |
| `TlsAlloc`、`TlsSetValue` | 1、1 | **0、0**（#11 が `FlsAlloc`、#13 が `FlsSetValue` に変わった。#13 は guest の戻り 0x37d09ee） |
| `FlsAlloc` | 1 | 2 |
| `FlsSetValue` | 2 | 3 |
| ICS の第 3 引数（r8 = Flags） | A: stale（1 回 0x800、78 回 0xc80e9a4e1297。代替では捨てられる値） | **79 回すべて 0**（Ex の Flags = 0 が ICSASC の行に渡る） |
| `InitializeCriticalSectionAndSpinCount` の第 2 引数（rdx） | 0xfa0 | 0xfa0 |

- **「79 回が Ex 経路に変わると何になるか」**（依頼の問い）の答え（読み＋B の実測）: DLL 側は 79 回すべて `InitializeCriticalSectionEx(lpCS, 0xfa0, 0)` の呼び出しになる。`InitializeCriticalSectionAndSpinCount` 79 回は、**Ex 専用の `fle_imp_InitializeCriticalSectionEx` を作れば 79 回すべてがその関数の呼び出しに変わり**、`InitializeCriticalSectionAndSpinCount` は 0 回になる（B は ICSASC の行を流用したので数字は 79 のまま。DLL が Ex 経路を取ったことは、guest の戻りアドレスと r8 = 0 で分かる）。
- **FLS 経路**（依頼の問い）: 非 NULL のとき ATTACH の #11 は `FlsAlloc`、#13 は `FlsSetValue`。**TLS 方式への分岐は ATTACH の範囲では消える**（`TlsAlloc`、`TlsSetValue` の呼び出しが 0 回）。ただし ATTACH の別の箇所（0x17d33a8、0x17d3220）が IAT の静的な `Fls*` を呼んでいるので、`fle_imp_Fls*` は NULL のときも使われる（A でも `FlsAlloc`、`FlsGetValue`、`FlsSetValue` が呼ばれる）。**停止の位置は変わらない**。
- エントリ +159（15,934 → 16,093）の**内訳は未確認**。distinct は同じ（881）。`fle_dispatch` 経由の呼び出しが `f_37de9b0` の入口を増やすことは読めるが、79 + 2 = 81 回の経由では 159 にならない（81 × 2 = 162 とも一致しない）。数え方（`trace.c` のカウンタ）を読んでいないので、この数字の説明は書かない。

### 5.3 測定 C1、C2、D（仮の値 3 段、2 段）: `LCMapStringEx` と `LocaleNameToLCID`

**D**（`MultiByteToWideChar`、`GetStringTypeW` の 2 段だけ。GPA の規則は何にも当たらない）: 停止は `fle: LCMapStringW: locale other than the emulated one`（Locale=0x0、dwMapFlags=0x100、cchSrc=256、cchDest=0、lpDestStr=NULL）。889 / 15,950。import 133 回。**`two-module-post0083.md` §5.2 の段 2 の行と一致**（再現）。DllMain 側の呼び出し列は:

```
#128 MultiByteToWideChar ← #129 MultiByteToWideChar
#130 LoadLibraryExW("api-ms-win-core-localization-l1-2-1", NULL, 0x800)   0x37d73a5 → 0x7ff8001d0000
#131 GetProcAddress(0x7ff8001d0000, "LCMapStringEx")                     → NULL
#132 GetProcAddress(0x7ff8001d0000, "LocaleNameToLCID")                  → NULL     ← LCMapStringEx が NULL のときだけ引かれる
#133 LCMapStringW(Locale=0, 0x100, src, 256, NULL, 0)                    停止
```

**C1**（D の 2 段 + `GetProcAddress("LCMapStringEx")` に**偽の VA 0x7ff9dead0010** を返す。この VA はどの表にも無い。`LCMapStringEx` の実装ではない）: 停止は `fle: indirect call to 0x7ff9dead0010 is not a known function entry`。887 / 15,949。**import は 131 回**（#131 の GPA で終わる）。**`LocaleNameToLCID` の GPA も `LCMapStringW` も呼ばれない**（D の #132、#133 が出ない）。停止時の guest のレジスタとスタック（`stageC1-gdb.log` の `(stop-probe)`、読み取り）:

| 項目 | 値 |
|---|---|
| rcx（lpLocaleName） | **0x0（NULL）** |
| rdx（dwMapFlags） | 0x100（LCMAP_LOWERCASE） |
| r8（lpSrcStr） | 0x7ffff7bfec60（スタック。前の `MultiByteToWideChar` の出力） |
| r9（cchSrc） | 0x100（256） |
| 5 番目（lpDestStr） | 0x0（NULL） |
| 6 番目（cchDest） | **0x0** |
| 7、8、9 番目（lpVersionInformation、lpReserved、sortHandle） | 0、0、0 |
| guest の戻りアドレス | 0x37d768b |

**C2**（B の 6 名すべて + 2 段）: C1 と**同じ**停止と数（887 / 16,108、131 回）。`InitializeCriticalSectionEx`、`Fls*` の非 NULL が、その先の停止点に影響しない。

読み（0x17d75e8 = `LCMapStringEx` 呼び出し側のラッパー）:

- **NULL のとき**: 0x17d768d で `edx = 0`、`rcx = rbp`（ロケール名 NULL）を渡して 0x17d76d4（`LocaleNameToLCID` のラッパー B5）を呼ぶ。B5 は `GetProcAddress("LocaleNameToLCID")` が NULL なら DLL 内の 0x17d9f1c を呼ぶ。0x17d9f1c は名前が NULL なら 0x17d9f3a で 0x17d9f86（`xor eax,eax`）に飛ぶ。つまり **名前 NULL → LCID 0**。その eax を ecx に置き、`LCMapStringW(0, flags, src, cchSrc, dst, cchDest)` を呼ぶ（実測 D の #133）。
- **非 NULL のとき**: 0x17d7638 以降、スタックの 9 引数をそのまま並べ直して `call [0x1ebd260]` → `fle_dispatch`。`LocaleNameToLCID` も `LCMapStringW` も呼ばれない（実測 C1、C2）。**つまり「Locale=0 の停止」は、`LCMapStringEx` が NULL のときの代替経路の産物**で、非 NULL の経路には無い（依頼の問い「非 NULL のときも同じか」の答え: **同じではない**）。
- **ただし**、非 NULL のときも `LCMapStringEx(NULL, 0x100, src, 256, NULL, 0, NULL, NULL, 0)` が実行される。`LCMapStringEx` の仕様（下）に従って実装すると、`lpLocaleName` = NULL の扱いが問題になる。**取得した `LCMapStringEx` のページは NULL の意味を書いていない**: `lpLocaleName` は「`[in, optional]`」で、「Pointer to a locale name, or one of the following predefined values.」として `LOCALE_NAME_INVARIANT`、`LOCALE_NAME_SYSTEM_DEFAULT`、`LOCALE_NAME_USER_DEFAULT` を挙げる。`LOCALE_NAME_USER_DEFAULT` の定義（`LOCALE_NAME* Constants` のページ）は「Name of the current user locale, …」で、値（NULL か文字列か）は**ページに書かれていない**。ヘッダ（mingw-w64 の `winnls.h`。**Learn ではない。参考**）は `#define LOCALE_NAME_USER_DEFAULT NULL`、`LOCALE_NAME_INVARIANT L""`、`LOCALE_NAME_SYSTEM_DEFAULT L"!x-sys-default-locale"`。つまり **NULL = ユーザー既定ロケール** は、Learn の原文ではなく、SDK ヘッダの値による読みになる。
- 非 NULL でも、**256 単位の小文字化**は `LCMapStringW` の現行実装（`fle_locale.c:746`。0x80 以上は `Unicode case table not on the MSDN pages` で停止）と同じ壁に当たる。0x80〜0xFF の単位が入力に含まれるため（src は `MultiByteToWideChar` の 1252 の出力。0084 のあと、0x80〜0x9F は U+20AC などになる）。この壁は `LCMapStringEx` を実装しても残る（読み）。

### 5.4 `setSBUpLow` がロケール名を NULL で渡す理由（読み）

停止した `LCMapStringW` の呼び出し元は 0x37d76bf ← 0x17d9a94 ← 0x17d9db4 ← **0x17d674c**（`setSBUpLow`）。0x17d674c は code page ごとの文字クラス表を作る関数で、`GetCPInfo` で情報を取り、256 バイト（byte[0] = 0x20、byte[1..255] = 0x01〜0xFF）を `MultiByteToWideChar` → `GetStringTypeW`（`CT_CTYPE1`）→ 0x17d9db4（`__crtLCMapStringA` 形）に 2 回（flags 0x100、0x200）渡す。ロケール名は `[rdi + 0x220]`（`setSBCS`/`setSBUpLow` が詰める LocaleName）から取る（0x17d6823）。この値は 0x17d6cd8 の中の 0x17d6e10〜0x17d6e50 で決まる: `ecx = code page - 0x3a4`、0 なら（932）0x17d6e49、`-4` で 0 なら（936、0x3a8）0x17d6e40、さらに `-0xd` で 0 なら（949、0x3b5）0x17d6e37、さらに `ecx == ebp`（ebp は 1。950、0x3b6）なら 0x17d6e2e。それぞれ `.rdata` 0x1ec9060〜0x1ec9078 のポインタから `ja-JP`、`zh-CN`、`ko-KR`、`zh-TW`（読み: 0x1ec9060 = `ja-JP`、0x1ec9068 = `zh-CN`、0x1ec9070 = `ko-KR`、0x1ec9078 = `zh-TW`）。**どれにも当たらなければ 0x17d6e29 の `mov rax, rbx`（rbx は 0x17d6d05 の `xor ebx,ebx` で 0）つまり NULL**。1252（0x4e4）はどれにも当たらない。実測: 1252 で `LCMapStringW(0, …)`（#133）、すなわち名前 NULL → 0x17d9f1c が 0。UTF-8（65001、0x17d6d53〜0x17d6d60）の経路も `[rsi+0x220] = rbx`（NULL）。

0x17d9f1c の fallback 表（RVA 0x1ecb780、228 エントリ。`final_checks.out` §2）には `en-us`（index 75、LCID 0x409）、`en`（index 8、0x9）、空文字列（index 66、0x7f）があるが、**名前 NULL では表を引かない**（0x17d9f37 `test rcx,rcx`、0x17d9f3a `je 0x17d9f86`、0x17d9f86 `xor eax,eax`）。

## 6. 非 NULL を返すために runtime に必要な変更の範囲（読み取りだけの見積もり。実装していない）

行番号は 27c8068（= 終了時の HEAD 9b65b66 の runtime と同じ）。

### 6.1 擬似アドレスの機構は既にある（読み＋B、C の実測）

1. `fle_imp_GetProcAddress`（`fle_modules.c:168`）は、`hModule` が `FLE_MODULE_HANDLE_BASE + STEP × N`（N ≤ `FLE_MODULE_N`。**N=27〜30 を含む**。`fle_modules.c:199-205`）なら `fle_modules[N-1]` の名前を引き、`fle_resp_table[i].dll` がその名前と `strcmp` で一致し、`.name` が一致する行の `va` を返す（`fle_modules.c:218-225`）。無ければ `NULL` と 127（:226-227）。
2. 応答表は `imports_table.rs` の `TABLE`（260 行。`("dll", "API", "fle_imp_*", "jmp_imp:*")` の 4 つ組。`dll` は比較時に小文字化。:17-316）が**唯一の元**。擬似 VA は `RESP_VA_BASE (0x7FF9_0000_0000) + 0x10 × (行 k + 1)`（:323、:357）。生成 C の `fle_dispatch_table`（全関数エントリ + 全行の擬似 VA）に載る（:336-348）。`fle_dispatch(va)` は表を二分探索して `fn()` を呼ぶ（`fle_dispatch.h:33-47`）。
3. **api-ms ハンドルへの適用**: 表の `dll` 欄が `api-ms-win-core-synch-l1-2-0.dll` などの行を足せば、機構はそのまま動く。**B で実測**: api-ms ハンドル（0x7ff8001b0000、0x7ff8001c0000）への `GetProcAddress` が擬似 VA（既存行のもの）を返し、DLL の `call [0x1ebd258]` → `f_37de9b0` → `fle_dispatch(R[0])` → `fle_imp_FlsAlloc` などが呼ばれた（#11、#13、#31 以降。`TlsAlloc`、`TlsSetValue` は 0 回）。
4. **ABI**: `fle_imp_*` は Windows x64 の引数をゲストのレジスタ R[1]（rcx）、R[2]（rdx）、R[8]（r8）、R[9]（r9）と、`R[4] + 8 + 8 × (n-1)` のスタック（5 番目以降。`fle_locale.c` の `fle_stack_arg`）から読み、R[0] に返す。`jmp [0x1ebd258]`／`call [0x1ebd260]` が積む戻りアドレスと、決定 0015 §4 の `fle_dispatch` の規約は合う（B で `TlsSetValue` → `FlsSetValue` の入れ替えで guest の戻りが 0x37d09f6 → 0x37d09ee に変わっても動いた）。
5. 注意: `movabs r10, imm64`（XFG のハッシュ）は R[] に写らない。runtime は r10 を使わない（読み。B、C1 で問題は出ていない）。

### 6.2 どの `fle_imp_*` が既にあり、どれが無いか（`grep` と読み）

| DLL が使う API | 既にある `fle_imp_*`（HEAD の行） | `kernel32.dll` の応答表の行 | api-ms の行 | 備考 |
|---|---|---|---|---|
| `InitializeCriticalSectionAndSpinCount` | `fle_sys.c:193`（決定 0078） | あり（`imports_table.rs:281`） | — | 戻り 1。`fle_cs_init` と同じ表。初期化済みの再初期化は停止。Flags の引数は無い |
| `InitializeCriticalSectionEx` | **無い** | 無い | 無い | §7.1 |
| `FlsAlloc`、`FlsFree`、`FlsGetValue`、`FlsSetValue` | `fle_imports.c:179、194、211、222`（決定 0078） | あり（:284-287） | 無い | B で実測した非 NULL の応答の本体 |
| `TlsAlloc`、`TlsSetValue`、`TlsGetValue` | `fle_imports.c:27、97、115` | あり（:19、21、22） | — | NULL 経路の `FlsAlloc`、`FlsSetValue`、`FlsGetValue` の代替 |
| `TlsFree` | `fle_imports.c:135`（決定 0078） | **無い**（既存テストが「無い」と表明。決定 0078 §3） | — | `FlsFree` が NULL のとき `fle_unsupported(0x37d095f)`（生成 C） |
| `LCMapStringW` | `fle_locale.c:746` | あり（:55） | — | ASCII の大文字小文字だけ。0x80 以上と他のロケールは停止 |
| `GetLocaleInfoW` | `fle_locale.c:412` | あり（:54） | — | 依頼が挙げた名前だが、**DLL は `GetLocaleInfoW` を呼ばない**（IAT にも文字列にも無い） |
| `GetStringTypeW` | `fle_locale.c:150`（名前付き停止） | あり（:314） | — | 実装は停止だけ |
| `LCMapStringEx` | **無い** | 無い | 無い | §7.2 |
| `LocaleNameToLCID` | **無い** | 無い | 無い | §7.3 |
| `AreFileApisANSI` | **無い** | 無い | 無い | NULL のとき 1（ANSI）。ATTACH の先 |
| `AppPolicyGetProcessTerminationMethod` | **無い** | 無い | 無い | NULL のとき `0xc0000225`。終了経路だけ（ATTACH に無い） |
| `GetModuleHandleExW` | `fle_modules.c:140`（名前付き停止） | あり（:315） | — | `CorExitProcess` の経路（ATTACH に無い） |

### 6.3 変更の範囲（見積もり。**決めない**）

「A. 行を足すだけ（既存の `fle_imp_*` を使う）」「B. 新しい `fle_imp_*`」「C. 構造の変更」に分ける。

**A. `imports_table.rs` の `TABLE` に api-ms の行を足す**（既存の `fle_imp_*` を指す）: 例 `("api-ms-win-core-fibers-l1-1-1.dll", "FlsAlloc", "fle_imp_FlsAlloc", "jmp_imp:FlsAlloc")` を 4 行。同じ `fle_imp_*` を指す行が 2 つ（`kernel32.dll` と api-ms）になる。**この方法の見積もりでの注意点**（読み。確認は `cargo test` が要る。**この調査は走らせていない**）:
- `handler` 名（4 番目）は `Stats::handled` のキーで、`lookup(import)`／`lookup_call` は `dll!name` の `TABLE.iter().find` で**最初の行**を返す（`imports_table.rs:368-374`、:385-388）。同じ `API 名` を持つ行が増えても、`kernel32.dll` の行が先にあれば挙動は変わらない（api-ms の行は `lookup` からは見えない）。ただし `call_handlers()` は行ごとに `call_imp:<API>` を作るので、同名のハンドラ名が 2 行に付く。`Stats::handled` のキーは `lookup`／`lookup_call` が返した**最初の行**のハンドラ名（`lib.rs:813`）だけが数えられるので、キーの個数は変わらないと読める（**未確認**: 実行していない）。`k32_dsp.rs:194-195` は行文字列全体（`"kernel32.dll"` を含む）の一致が 1 回であることを表明するので、api-ms の行は数えない（読み）。`call_iat_dll.rs` は `st.handled["call_imp:<API>"]` の回数と DLL の 138 か所の呼び出し点の総数を表明する（`call_iat_dll.rs:39-48`）。呼び出し点の総数は行の追加で変わらない（行は IAT スロットの `dll!name` で引かれ、api-ms の行は IAT に現れない）と読める（**未確認**）。
- 行が増えると `fle_resp_count` と `fle_dispatch_table` の擬似 VA が増え、**FLEngine の生成 C の SHA-256 が変わる**（決定 0078 §4 の前例: 33 行で `67827424…` → `43985d62…`）。`gen_two.sh` の `EXPECT_FLE_SHA`（`43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1`）と、FLEngine の単独構成の a4（145/942、821 関数、15,685 エントリ）の基準を更新する必要がある（決定の対象。§8）。
- 既存テストの固定: `modules.rs:331-336`（N=26〜29 の `GetModuleHandleW`）、`realmod.rs:30、478、555`（`API_SET_HANDLES`）。これらは**ハンドルの値**を固定するだけで、行の有無を表明しない（読み。`GetProcAddress` を api-ms ハンドルで呼ぶテストは grep で見つからなかった）。`resp_entries` の期待値は `>= ROWS.len() + 1` など下限（`sysinfo.rs:331`、`modules.rs:553`）で、絶対値の固定は見つからなかった（**未確認**: 全テストを走らせていない）。

**B. 新しい `fle_imp_*`**（各 1 関数。実装は Learn の仕様（§7）で書ける範囲）:
- `InitializeCriticalSectionEx`: 第 3 引数 Flags は 0 または `CRITICAL_SECTION_NO_DEBUG_INFO`（値は**ページに書かれていない**。表の「Meaning」の説明だけ）。`fle_imp_InitializeCriticalSectionAndSpinCount` と同じ本体で足りる（Flags は観測できない）。
- `LCMapStringEx`: 引数 9 つ。`LCMapStringW` の実装を共有できる範囲は、locale が名前（NULL を含む）で、flags が単一の大小文字、0x80 未満の ASCII だけ（`LCMapStringW` と同じ制限）。
- `LocaleNameToLCID`: `LCMapStringEx` が非 NULL のとき DLL は呼ばない（実測）ので、**ATTACH のためには不要**。呼ばれるのは `LCMapStringEx` が NULL で `LocaleNameToLCID` だけ非 NULL の組み合わせ（応答表が「`LCMapStringEx` だけ無い」ときの DLL の経路）だけ。
- `AreFileApisANSI`、`AppPolicyGetProcessTerminationMethod`: ATTACH の直線に無い（停止までの 125 回に出ない。`AreFileApisANSI` は ATTACH の先で到達する: §3.3 #7）。

**C. 構造の変更**: 不要と読める（機構は §6.1 のとおり既にある）。`fle_modules.c` のテーブルの変更も不要（N=27〜30 は決定 0081 で実装済み）。

### 6.4 非 NULL にしたときの副作用（読み）

- ラッパーは一度 NULL を得ると**関数スロットを -1** にする（再試行しない）。非 NULL なら関数スロットに擬似 VA を保存し、以後は `call [0x1ebd258]` で `fle_dispatch` を通す。ゲストが擬似 VA を保持する前提は決定 0015 §4（保存されない前提）。
- **DETACH**: `InitializeCriticalSectionEx` の関数スロット（A5、B3）は DETACH では使われない。`FlsFree` は `fle_imp_FlsFree` が「値があり callback がある」index で停止する（0x17cd5e4。DETACH、巻き戻し、失敗経路。決定 0082 D11 で `FreeLibrary` は停止のままなので、DETACH には届かない）。
- `FlsAlloc` の callback は 0x17cd450（`FlsCallback`）。`fle_imp_FlsFree` は、値が非 NULL でその index に callback があると停止する（`fle_imports.c:201-203`）。ATTACH の範囲では `FlsFree` は呼ばれない（#9〜#125 に無い）。

## 7. 足りない関数を Learn の仕様でどこまで実装できるか（引数と戻り値の規則。実装していない）

### 7.1 `InitializeCriticalSectionEx`

> [Learn: synchapi-initializecriticalsectionex.txt] "This parameter can be 0 or the following value."
> [Learn: synchapi-initializecriticalsectionex.txt] "If the function succeeds, the return value is nonzero."
> [Learn: synchapi-initializecriticalsectionex.txt] "If the function fails, the return value is zero (0)."
> [Learn: synchapi-initializecriticalsectionex.txt] "Initializing a critical section that is already initialized results in undefined behavior."
> [Learn: synchapi-initializecriticalsectionandspincount.txt] "This function always succeeds and returns a nonzero value."
> [Learn: synchapi-initializecriticalsectionandspincount.txt] "Starting with Windows Vista, the InitializeCriticalSectionAndSpinCount function always succeeds, even in low memory situations."

- 規則が書かれている: `Flags` は 0 か `CRITICAL_SECTION_NO_DEBUG_INFO`、成功は非 0、失敗は 0。再初期化は未定義（既存の `fle_cs_init` の停止と同じ扱いにできる）。
- **書かれていない**: `InitializeCriticalSectionEx` が「常に成功する」とは**書いていない**（`InitializeCriticalSectionAndSpinCount` のページだけが「always succeeds」と書く。`InitializeCriticalSectionEx` のページに "always" の語は無い）。`CRITICAL_SECTION_NO_DEBUG_INFO` の**数値**もページに無い（DLL は 0 しか渡さない: 実測で 79 回すべて Flags = 0。B）。Flags が 0 と `NO_DEBUG_INFO` 以外のときの結果もページに無い（停止にできる）。
- 実装の見積もり: ATTACH で観測される範囲（Flags = 0、spin = 0xfa0、79 回、未初期化の領域）では、`fle_imp_InitializeCriticalSectionAndSpinCount`（`fle_sys.c:193`）と**同じ戻り 1 と同じ表の更新**で足りる。戻り 1（「成功は非 0」の範囲。ICSASC の「always succeeds」を Ex に写すのは**導出**）は Learn の `InitializeCriticalSectionEx` のページからは「成功のとき非 0」までしか言えない。

### 7.2 `LCMapStringEx`

> [Learn: winnls-lcmapstringex.txt] "Pointer to a locale name, or one of the following predefined values."
> [Learn: winnls-lcmapstringex.txt] "The application can set cchDest to 0. In this case, the function does not use the lpDestStr parameter and returns the required buffer size for the mapped string or sort key."
> [Learn: winnls-lcmapstringex.txt] "Windows Vista, Windows 7: Reserved; must set to NULL."
> [Learn: winnls-lcmapstringex.txt] "Reserved; must be NULL."
> [Learn: winnls-lcmapstringex.txt] "Reserved; must be 0."
> [Learn: winnls-lcmapstringex.txt] "CompareStringEx and LCMapStringEx can specify a sort handle (if the locale name is null)."

- 規則が書かれている: `cchDest = 0` なら出力バッファを使わず必要な大きさを返す（`LCMapStringW` と同じ。`fle_imp_LCMapStringW` の `cd == 0` の経路を共有できる）。`lpVersionInformation`（NULL）、`lpReserved`（NULL）、`sortHandle`（0）は予約（DLL は 0、0、0 を渡す: C1 の `(stop-probe)`）。
- **書かれていない**: `lpLocaleName` = NULL の意味（上は「locale name, or one of the following predefined values」と書き、`LOCALE_NAME_USER_DEFAULT` の**値を書かない**。`LOCALE_NAME* Constants` のページも名前の説明だけ）。「sort handle が指定される場合は locale name が null」という書き方は、**NULL が locale name の位置で許される**ことを示唆するが、NULL が何を意味するかは書いていない。**ここは Learn では決められず、決定が要る点**（§8）。
- 大文字小文字の変換（flags = 0x100）の 0x80 以上の単位の結果は、`LCMapStringW` と同じく Learn に**表が無い**（`fle_locale.c:776` の停止）。ATTACH で DLL が渡す 256 単位の入力は 0x80〜0xFF を含む（`MultiByteToWideChar` の 1252 の出力）ので、`LCMapStringEx` を実装しても**この壁は残る**（読み。0084 で 0x80〜0x9F の対応は入ったが、0xA0〜0xFF は恒等で、Unicode の小文字化表は無い）。
- 実装の見積もり: 引数の並べ替えと、ロケール名 → 既存の `fle_locale_resolve` への接続（NULL、`en-US` を emulated に、他は停止）が要る。**変換そのものは `LCMapStringW` の制限（ASCII の大小文字だけ）のまま**。

### 7.3 `LocaleNameToLCID`

> [Learn: winnls-localenametolcid.txt] "If successful, returns the locale identifier corresponding to the locale name."
> [Learn: winnls-localenametolcid.txt] "The function returns 0 if it does not succeed."

- 規則が書かれている: 成功は LCID、失敗は 0。`lpName` は「locale name, or one of the following predefined values」（`LOCALE_NAME_INVARIANT`、`LOCALE_NAME_SYSTEM_DEFAULT`、`LOCALE_NAME_USER_DEFAULT`）。`dwFlags` は「Beginning in Windows 7: Can be set to LOCALE_ALLOW_NEUTRAL_NAMES」（DLL は `dwFlags` = 0 を渡す: 0x17d768d の `xor edx,edx` の後に 0x17d7692 で 0x17d76d4 を呼び、0x17d76d4 は `mov edi,edx`（0x17d76de）で保持して非 NULL の経路の呼び出しにも渡す。**読み**。#132 の gdb ログの `rdx` は GPA の第 2 引数（名前のポインタ 0x3ec9608）で、`dwFlags` ではない）。
- **書かれていない**: `en-US` が 0x0409 になること（`LocaleNameToLCID` のページの例は 0x10407 で、en-US の LCID は書かない）。`locale-names` のページは `en-US` を locale name の例に挙げるが（`LOCALE_SNAME`）、LCID との対応は書かない。LCID 側の値は決定 0016、0042 が「環境の値」として採っている（`fle_emulated_locale.lcid = 0x0409`）。
- ATTACH では**不要**（`LCMapStringEx` が非 NULL なら DLL は呼ばない。C1、C2 の実測）。

### 7.4 `AreFileApisANSI`、`AppPolicyGetProcessTerminationMethod`

> [Learn: fileapi-arefileapisansi.txt] "If the set of file I/O functions is using the ANSI code page, the return value is nonzero."
> [Learn: fileapi-arefileapisansi.txt] "If the set of file I/O functions is using the OEM code page, the return value is zero."
> [Learn: fileapi-setfileapistoansi.txt] "The 8-bit console functions use the OEM code page by default. All other functions use the ANSI code page by default."
> [Learn: appmodel-apppolicygetprocessterminationmethod.txt] "If no known process termination policy was found for the process token, the function raises a STATUS_ASSERTION_FAILURE exception and returns ERROR_NOT_FOUND."

- `AreFileApisANSI`: 戻り値の規則は書かれている。**初期状態**は `SetFileApisToANSI` ページの "by default"（ANSI）から読める。DLL も FLEngine も `SetFileApisToOEM`／`SetFileApisToANSI` を import しない（`grep -c -a` が両方 0）ので、状態は変わらず、非 0 を返すのが整合する。DLL の NULL 経路の固定値 1 と、非 NULL の経路の戻り（非 0）は**同じ**で、差が出ない（ATTACH の先）。
- `AppPolicyGetProcessTerminationMethod`: ページは戻り値（`ERROR_SUCCESS`、`ERROR_NOT_FOUND`、`ERROR_INVALID_PARAMETER`）と、`policy` が `AppPolicyProcessTerminationMethod` 型であることを書くが、**その列挙値の一覧は取得ページに書いていない**（ページ内の記述は型名と「a value indicating the method used to end the process」だけ）。ATTACH で呼ばれず（終了処理 0x17d1a54 のみ）、NULL の経路の `0xc0000225`（STATUS_NOT_FOUND）と、非 NULL で `ERROR_SUCCESS` を返す経路で、0x17d5730 は `[rsp+0x30] == 1`（列挙値 1）のときだけ 0 を返す（読み）。ATTACH に出ない名前なので、行を足さない選択ができる。

## 8. 決定が要る点（決めない。選択肢と事実。推奨は「意見」と明記）

### 8.1 `GetProcAddress(api-ms ハンドル, 名前)` が NULL か非 NULL か（決定 0081 の「当面、行を作らない」の再訪）

**事実**

- 現行（決定 0081 のとおり。行を作らない）: DLL は §5.1 の NULL 経路（`Tls*`、`InitializeCriticalSectionAndSpinCount` 79 回、`LCMapStringW`）を取る（実測 A）。
- 非 NULL にしたとき（実測 B、C1、C2）: DLL は Ex 経路と FLS 経路を取る（§5.2）。**ATTACH の停止の位置は変わらない**（`MultiByteToWideChar`。B）。その先の `LCMapStringW(Locale=0)` の停止は**消え**、代わりに `LCMapStringEx` の呼び出しに止まる（C1。§5.3）。ただし 256 単位の小文字化の壁（0x80 以上の Unicode 表が無い）は**残る**（§7.2）。
- Learn は、実機で api-ms ハンドルへの `GetProcAddress` が非 NULL を返すかを**書いていない**（§4.4 の 1、2）。API の存在は Windows 10 22H2 について導出できる（§4.2）。
- `LCMapStringEx` の `lpLocaleName` = NULL の意味は Learn に**書かれていない**（§7.2）。

**選択肢**

| | 内容 | この調査で分かる範囲の帰結 |
|---|---|---|
| (a) | 現状維持（行を作らない。api-ms は「ある」が中身は空） | 決定 0081 のとおり。ATTACH は `MultiByteToWideChar`、`GetStringTypeW`、`LCMapStringW`（Locale=0）、`WideCharToMultiByte` の表の決定が要る（`two-module-post0083.md` §9-1）。実機との差は断言できない（§4.4） |
| (b) | `api-ms-win-core-synch-l1-2-0.dll` と `…fibers-l1-1-1.dll` に行を足す（`InitializeCriticalSectionEx`、`Fls*`）。`localization`、`appmodel` は足さない | A（既存 `fle_imp_*`）＋ `InitializeCriticalSectionEx` の新規。ATTACH の停止の位置は変わらない。`LCMapStringW(0)` の停止は残る（`LCMapStringEx` が NULL のまま）。FLEngine の生成 C の SHA-256 が変わる |
| (c) | (b) に加えて `…localization-l1-2-1.dll` に `LCMapStringEx`（と `LocaleNameToLCID`）の行を足す | `LCMapStringW(Locale=0)` の停止は消え、`LCMapStringEx` の実装が要る。NULL の意味の決定（8.2）と、0x80 以上の小文字化の表の決定（`two-module-post0083.md` §9-1）が要る |
| (d) | 4 名すべてに行を足す（`AreFileApisANSI` は kernel32 だけなので足さない） | (c) に加えて `AppPolicyGetProcessTerminationMethod` の行。ATTACH に出ない（行を足す意味は「実機との整合」だけ） |

**意見**（決めるのは依頼者）: この調査の事実だけでは (a)〜(d) の優劣は付けられない。**判断の分かれ目は「実機の `GetProcAddress` が非 NULL を返す」という前提を採るか**で、それは Learn に書かれておらず、この調査では確認できない。**(b) の `Fls*` と `InitializeCriticalSectionEx` は、Windows 10 に API が存在すること（§4.2。導出）と、行が既存の `fle_imp_*`（`Fls*`）か ICSASC と同じ本体で足りること（§6.3）から、(c)、(d) より新しい主張が少ない**。ただし (b) でも 79 回の `InitializeCriticalSectionAndSpinCount` が `InitializeCriticalSectionEx` に変わるので、DLL の観測する経路は変わる（それが望ましいかは前提次第）。

### 8.2 `LCMapStringEx(NULL, …)` の NULL の扱い（8.1 の (c) を採る場合）

- 事実: Learn に値の記述が無い（§7.2）。SDK ヘッダ（Learn ではない参考）では `LOCALE_NAME_USER_DEFAULT` が NULL。runtime は現在 `LCMapStringW` で `LOCALE_USER_DEFAULT`（0x400）を emulated locale（0x0409）に解決する（`fle_locale.c:60-72`）。
- 選択肢: (i) NULL を `LOCALE_USER_DEFAULT` と同じく emulated locale に解決する（SDK ヘッダの値による読み。Learn に無い）、(ii) NULL は停止にする（fail-closed。ATTACH はここで止まる）。
- 意見: (ii) は Learn に書かれていないことを環境の振る舞いとして固定しない。ただし ATTACH は進まない。

### 8.3 FLEngine の生成 C の SHA-256 と a4 の基準（8.1 の (b)〜(d) を採る場合）

- 事実: `TABLE` に行を足すと `fle_resp_count`、`fle_dispatch_table` が増え、FLEngine の生成 C が変わる（決定 0078 §4 の前例）。`gen_two.sh` は `EXPECT_FLE_SHA = 43985d62…` を検査する。単独構成の a4（145/942、821 関数、15,685 エントリ）の基準にも影響しうる（決定 0078 §4 は「停止点は変わらない」と実測した。今回は**未測定**）。
- 決定が要る: 期待 SHA の更新、a4 の基準の再測定の手順（決定 0078 の前例どおり再測定）。

### 8.4 既存テストの固定（8.1 の (b)〜(d) を採る場合）

- 事実: §6.3 A のとおり、`GetProcAddress` を api-ms ハンドルで呼ぶテストは見つからず、行文字列の一意性を数えるテストは `kernel32.dll` の行文字列で数える。ただし `cargo test --workspace` は**走らせていない**（編集禁止の範囲で作業した）。
- 決定が要る: 実装の作業で全テストを走らせる手順（決定 0081 の実装の節と同じ扱いを推奨するが、これは意見）。

### 8.5 DLL が 79 回呼ぶ `InitializeCriticalSectionEx` の戻り値（8.1 の (b) を採る場合）

- 事実: `InitializeCriticalSectionEx` のページは「成功のとき非 0、失敗のとき 0」と書き、「always succeeds」とは書かない。`InitializeCriticalSectionAndSpinCount` のページは「always succeeds」と書く（§7.1）。
- 選択肢: (i) 戻り 1（ICSASC と同じ。ATTACH の範囲で観測される値）、(ii) 失敗の経路（0 を返す条件）を作らない。(i) と (ii) は同じ実装になる（失敗の入力が ATTACH に無い）。

### 8.6 測定の前提の再確認（決定ではなく、決める人が知るべき点）

- 測定に使った実行ファイルは 0084（`MultiByteToWideChar` の cp1252 の 0x80〜0x9F）の**前**の runtime で作られている（§2.2）。HEAD の runtime で作り直すと、A の停止は `MultiByteToWideChar` から `GetStringTypeW`（882/15,937。27c8068 のコミットメッセージ）に進む。**NULL と非 NULL の経路の差は 0084 と独立**（`GetProcAddress` の戻り値の差であり、`MultiByteToWideChar` の実装の差ではない）が、停止の位置の数値は変わる。作り直した実行ファイルでの再測定は**していない**（作り直しは `build_two_native.sh` の長いビルドで、依頼の範囲外と判断した。**必要な測定**に書く）。

## 9. 検証、未確認、必要な測定

### 9.1 検証（実測）

- **引用の機械照合**: `lift/out-apiset-gpa/check_doc_quotes.py` が、この文書の `> [Learn: …]` 行のすべて（ほか `check_quotes.py` の 38 件）を `lift/out-apiset-gpa/learn/*.txt` の本文に対して照合し、missing 0（出力は完了報告に載せる）。
- **gdb が読み取りだけの測定 A**: 停止、数値（881 / 15,934、import 125 回、26 種）が `two-module-post0083.md` §3、§4 と一致。`HOOK` 行は出るが `PATCH` と `skip` の行は 0。プローブは R[] とゲストのメモリを書かない（`patches_gpa_probe.py`）。
- **D**: `two-module-post0083.md` §5.2 の「`POST_MAX` = 2」の行（`LCMapStringW`、Locale=0、cchDest=0、889 / 15,950、133 回）を再現。

### 9.2 未確認

- 実機 Windows 10 22H2 での、`LoadLibraryExW(api-ms-win-core-synch-l1-2-0 など, NULL, 0x800)` の結果と、そのハンドルへの `GetProcAddress` の結果（Learn に無い。ホストは Linux。実機を持たない）。
- B の「+159 エントリ」（15,934 → 16,093）の内訳（79 + 2 = 81 回の `fle_dispatch` の経由では説明がつかない）。`trace.c` のカウンタが何を数えるかを読んでいない。
- 非 NULL にしたときの ATTACH の**先**（`WideCharToMultiByte` の後、DllMain が `eax = 1` で戻るまで）。C1、C2 は `LCMapStringEx` の呼び出しで止めたので、その先は測っていない。
- `FlsFree` と `FlsGetValue` の GPA 照会が実行されるか（ATTACH の 125 回の範囲に無い。0x17cd470、0x17cd4dc、0x17cd5e4 の呼び元は 0x17cb3a0 以降の別経路に見える）。
- `AreFileApisANSI` の照会（0x17d752f）の到達の確認（ATTACH の先にあると読んだが、実行していない）。
- 段数の数え方（§2.3）: B と C2 の「GetProcAddress の段」を 1 つと数えるか、名前ごとの分岐と数えるか。**後者なら B は 5 段、C2 は 2 + 6 = 8 段で、依頼の「最大 3 段」を超える**。依頼者が決める事項として残す。
- B のスタンドイン（`InitializeCriticalSectionEx` → ICSASC の行）が、実際の `fle_imp_InitializeCriticalSectionEx` の応答（戻り 1、同じ表の更新）と同じ結果を与えるかは、実装していないので**未確認**（ICSASC の本体は同じ `fle_cs_init` と戻り 1 で、Flags を読まない: `fle_sys.c:193-197`）。
- `cargo test --workspace` は実行していない。`GetProcAddress` を api-ms ハンドルで呼ぶテストが無いことは grep の結果であり、全文走査ではない。

### 9.3 必要な測定（やっていない）

1. **HEAD の runtime で作り直した 2 モジュールの実行ファイルでの A、B、C1**: `gen_two.sh`、`build_two_native.sh`（長いビルド）が要る。数値（881 → 882 など）が 0084 の後でどう変わるかを見るため。
2. **api-ms の行を実際に足したコピーでのビルドと a4**: 依頼は「実装しない」ので、`imports_table.rs` のコピーを `lift/out-apiset-gpa/` に置いてビルドする作業が要る（本体は編集しない）。FLEngine の生成 C の SHA-256、a4 の 145/942、821 関数、15,685 エントリの変化を測る。
3. **`LCMapStringEx` を ASCII だけ実装したコピーでの ATTACH の先**: 0x80 以上の壁（§7.2）の位置と、その先の停止（`WideCharToMultiByte` ほか）を測る。
4. **実機の Windows 10 22H2 での `GetProcAddress(LoadLibraryExW("api-ms-win-core-synch-l1-2-0", NULL, 0x800), "InitializeCriticalSectionEx")`**。依頼の範囲外。

## 10. 作ったファイル（すべて `lift/out-apiset-gpa/` と本文書だけ）

- 本文書 `docs/investigations/dsp-ippv2-apiset-gpa.md`（追跡下。新規）。
- `lift/out-apiset-gpa/`（gitignore 対象。約 2.6 MB。2 GB 級の単一ファイルは無い）:
  - `learn/`: Learn の取得ページ `.html` 36 件（うち 3 件は 404 のエラーページで、名前が `_404-*`）と、抜き出し `.txt`、`FETCHED_AT.txt`、`MANIFEST.sha256`。`_404-*` の `.txt` は 1 件（`constants-used-in-national-language-support`）だけ。
  - `gpa_scan.py`、`gpa_scan2.py`、`gpa_scan3.py`、`gpa_decode.py`、`rdis.py`、`xref_fn.py`、`imp_by_caller.py`、`final_checks.py`、`learn_text.py`、`check_quotes.py`、`check_doc_quotes.py`（読み取り専用のスクリプト）と、それぞれの `*.out`。
  - `gdb/patches_gpa_probe.py`（読み取りだけ）、`patches_gpa_stage.py`（B、C1、C2、D。仮の値）。`patches_gpa_nonnull.py`、`patches_gpa_lcmapex.py` は `patches_gpa_stage.py` の前の版（B と C の初期の書き方。**測定には使っていない**）。
  - `run/`: A（`gpa-*`）、B（`stageB-*`）、C1（`stageC1-*`）、C2（`stageC2-*`）、D（`stageD-*`）の gdb のログと import 列。
  - `tmp/`（TMPDIR）。`/tmp` は使っていない。



