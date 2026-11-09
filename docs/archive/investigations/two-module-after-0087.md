# 決定 0087 の後の 2 モジュール構成（実 dsp_ippv2_x64.dll）の native 実行: DllMain(ATTACH) が戻り、次の停止は LoadLibraryW "avrt.dll"（測定のみ）

実行日: 2026-10-24（JST。ログの時刻は `20261024T0342…Z`）。基準の HEAD は `3d359a0a8978ff9d1d324c4936536e9fee50bd3b`（作業開始時は d810841。作業中に別の作業が 2 コミット進めた。`git diff d810841 HEAD -- lift` は空）に、決定 0087 の変更（`lift/runtime/fle_locale.c`、`fle_locale.h`、`lift/runtime/test/` の新規ファイル、`lift/crates/fe-lift/tests/locale.rs`、`k32_dsp.rs`）を足した作業コピー。決定 0087 は `docs/decisions/0087-ctype-locale-data.md`。**この文書は何も決めない**。**runtime の応答を足していない。gdb で値を入れていない**（gdb は `fle_imp_*` の入口で読むだけ。`return` も `set var` も使っていない）。

作業物は `lift/out-ctype87/`（gitignore 対象）。

## 0. 凡例

- **実測** = このセッションでコマンドを実行して得た値。ログは `lift/out-ctype87/` の下。
- **読み** = 逆アセンブル、生成 C、runtime のソース、別の文書を読んだ結果。実行して確かめていない。
- **未確認** = そのとおり。
- 「DllMain」は dsp_ippv2_x64.dll のエントリ（`f_37caca8`。基底 0x2000000 の VA）。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | 構成と生成物 | `two-module-after-0084.md` と同じ構成（`tbl2`、種 452 個）を、作業コピーから最初から再ビルド。FLEngine の C は SHA-256 `43985d62…37137a1`（`fe-lift lift … --pdata` は `unsupported_total 525`）。リベース済み DLL `4fd25502…efc5`、縮小 DLL の C `6035da1d…279dbf`、`common.c` `33779308…487d5f` は、`two-module-after-0084.md` の値と一致 | 実測 |
| 2 | **DllMain(ATTACH) が戻ったか** | **戻った。eax = 0x1**（`call_dllmain(idx, 1)` の C の戻り値。読み取りの gdb の FinishBreakpoint で測定）。DllMain の中の import は **160 回、34 種**（`DllMain` に入ってから戻るまで）。停止しなかった（`fle_realmod.c:144` の `m->state = ST_LOADED` に到達） | 実測 |
| 3 | **LoadLibraryW が 0x2000000 を返したか** | **返した**（`fle_realmod_load` の戻り値 R[0] = 0x2000000。`fle_load.c:152` の直後） | 実測 |
| 4 | DllMain の中の 3 つの API | GetStringTypeW 1 回（127 回目）、LCMapStringW 4 回（133、134、138、139 回目。Locale 0。0x100 と 0x200 のサイズ問い合わせ + 変換）、WideCharToMultiByte 6 回（135、140 回目が cp 1252、dwFlags 0。ほか 4 回は cp 0 の ASCII）。**どれも停止せず通った** | 実測 |
| 5 | DllMain の後の GetProcAddress | **424 回**（161 回目以降の import の通し番号 164〜1435）。**非 NULL 423 回、NULL 1 回**。非 NULL は hModule = 0x2000000（実在モジュール自身）に対する 423 回で、名前は `ippMalloc` に始まり `ippMul_8u_C4IRSfs` で終わる ipp の関数（420 種。`ipps*` 345、`ippi*` 71、`ipp*` 7）。NULL の 1 回は hModule = 0x7ff800130000（`GetModuleHandleW` が返した値。仮想モジュール表の index 18 = `user32.dll`。算出は `(0x7ff800130000 − 0x7ff800000000) / 0x10000 − 1`。名前は `UpdateLayeredWindowIndirect`。応答表に `user32.dll` のこの名前の行が無いため NULL） | 実測（戻り値、ハンドル）、読み（ハンドルと表の対応） |
| 6 | a4 の行（`tbl2`） | init_count = **146 / 942**（`0084` の後は 145 / 942）、last unit = index **145**（run_rank 133/895、proc `0x65c9f0`、in_progress）、functions_distinct = **935**（`0084` の後は 882）、function_entries = **25,215**（15,937）、all_units_done = no | 実測 |
| 7 | **次の停止** | **`LoadLibraryW`、モジュール名 `avrt.dll`**（`fle_load.c:174` の `fle_stop`。1436 回目の import）。メッセージ全文は §3 | 実測 |
| 8 | 再現性 | gdb なしの a4-survey 2 回: stdout の差は `calling entry … rsp=` の 1 行だけ（アドレス。ASLR）、stderr は完全に一致。gdb の読み取りの run 2 組（`ro3`/`ro4`、`ro5`/`ro6`）は `cmp` で全バイト一致 | 実測 |

つまり、決定 0087 の runtime で、**DllMain(ATTACH) は戻り、LoadLibraryW は 0x2000000 を返し、ゲストは DllMain の後に ipp の API の名前を GetProcAddress で引き続け、`unit 145`（`f_65c9f0`）の `LoadLibraryW("avrt")` で名前付き停止した**。

## 2. 手順（実測。実行した順）

`lift/` から実行。TMPDIR は `lift/out-ctype87/tmp`（`/tmp` は使っていない）。

```
CARGO_TARGET_DIR=$PWD/out-ctype87/target FLENGINE_PE=…/binaries/win64/FLEngine_x64.dll cargo test --workspace --no-fail-fast   # 95 binaries / 1,125 passed
CARGO_TARGET_DIR=$PWD/out-ctype87/target cargo build --release --offline -p fe-pe -p fe-lift
# 種 3 ファイルとリベース済み DLL は lift/out-ctype/two/dll/ からコピー（SHA-256 は §1 の 1 と一致）
OUT=$PWD/out-ctype87/two FEBIN=$PWD/out-ctype87/target/release TAG=tbl2 SEEDS=…/out-ctype87/two/dll/seeds-idx0-plus-initterm2.txt \
    REAL=$PWD/out-ext/dsp_ippv2_x64.dll bash harness/two/gen_two.sh rebase gen split merge
TAG=tbl2 OUT=$PWD/out-ctype87/two JOBS=4 bash harness/two/build_two_native.sh          # two-native 571,229,232 B
DLL=…/binaries/win64/FLEngine_x64.dll OUT=…/out-ctype87/out-tbl2 NATIVE_BIN=harness/two/two-native-run.sh TWO_NATIVE=…/v-tbl2/native/two-native \
    TWO_REAL=…/two/dll/dsp_ippv2_x64.2000000.dll FEPE=…/target/release/fe-pe bash harness/a4-survey.sh        # 2 回
gdb -q -batch -x out-ctype87/gdb/ro_imports.py     # 読み取りだけ。4 回（ro3〜ro6）
```

- `gen_two.sh` の `rebase` 段は、`REAL`（`lift/out-ext/dsp_ippv2_x64.dll`）の SHA-256 を検査して `fe-pe rebase` で基底 0x2000000 のファイルを作る。作ったファイルは調査の `out-ctype/two/dll/` のものと SHA-256 が一致する。
- ビルドは新しい作業ディレクトリで、runtime のオブジェクトを含め最初から作った（`build_two_native.sh` は作ったオブジェクトを飛ばす仕様なので、新しい OUT を使った）。
- 単独構成（FLEngine だけ）は §6。
- a4-survey を同じ秒に 2 回続けて実行すると、ログのファイル名（秒単位の時刻）が衝突して 2 回目が失敗する（`File exists`）。1 回目は 03:42:07Z、2 回目は 03:42:17Z に実行した（失敗した 1 回は結果に数えていない）。
- gdb の `ro_imports.py`（`lift/out-ctype87/gdb/`）は `gdb_imports.py` の読み取りだけの版（値を入れる機構を持たない）。`call_dllmain` に breakpoint を置き、FinishBreakpoint で戻り値（`int`）を読む。`fle_imp_GetProcAddress`、`LoadLibraryW`、`GetModuleHandleW`、`LoadLibraryExW` の戻り値も FinishBreakpoint で読む。いずれも `return` や `set var` を使わない。

## 3. 停止（gdb なし。実測）

`lift/out-ctype87/out-tbl2/a4/20261024T034207Z-native.txt`（と `…034217Z…`）:

```
stop   : fle: LoadLibraryW: module "avrt.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)
         top of stack: 0x423990 f_423990
depth  : init_count=146 / 942   last unit: index=145 run_rank=133/895 proc=0x65c9f0 in_progress
         started=133 completed=132 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=935  entries=25215
```

停止メッセージ全文（stderr。1 行。2 回とも同一）:

```
fle: LoadLibraryW: module "avrt.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)
```

出どころ: `lift/runtime/fle_load.c` の `load_by_name`（:174。`fle_stop`）。`avrt.dll` は仮想モジュールの表にも `fle_load_absent[]` にも無い。

### 3.1 呼び出し元（実測）

ハーネスの報告（`call depth at stop: 6`）:

```
#0 f_423990  guest-return=0x65ccd6
#1 f_65c9f0  guest-return=0x412129
#2 f_4120c0  guest-return=0x41229a
#3 f_412160  guest-return=0x41d989
#4 f_41d8f0  guest-return=0x1376875
#5 f_1376820 guest-return=0xdead0000
```

gdb の C の backtrace（`run/ro5-gdb.log`。`#0`〜`#4` は libc の abort）:

```
#5  fle_stop (api="LoadLibraryW", why="module \"avrt.dll\" is not a virtual module …")
#6  load_by_name (api="LoadLibraryW", u=…, n=4)                    fle_load.c:174
#7  fle_imp_LoadLibraryW ()                                        fle_load.c:184
#8  f_423990 ()    two/src/fle-00.c:88395          （FLEngine の LoadLibraryW のラッパー）
#9  f_65c9f0 ()    two/src/fle-03.c:70078          （unit 145 の proc。0x65c9f0）
#10 fle_dispatch (va=6670832)
#11 f_4120c0 ()    #12 f_412160 ()    #13 f_41d8f0 ()    #14 f_1376820 ()（FLEngine のエントリ）    #15 main ()
```

ゲストのレジスタ（停止時）: rcx = 0x65d444（名前の文字列 `avrt` の位置）、rdx = 0x7ffff6bdc000、r8 = 0x65d40c、r9 = 0x1b、rsp = 0x7ffff7bffbd8、rax = 0x0。

**呼び出し元の読み**: `f_65c9f0` は unit 145 の proc で、`docs/investigations/windows-dll-names-virtual-or-absent.md` §1 の 1 が「dwmapi と shcore と同じ関数」と書いたものと同じ（読み）。この実行（実測）では、`LoadLibraryW("dwmapi")`（1431 回目。戻り 0 = NULL。`fle_load_absent[]` の名前）、`GetModuleHandleW`（1432 回目。戻り 0x7ff800130000 = 仮想 `user32.dll`）、`GetProcAddress(0x7ff800130000, "UpdateLayeredWindowIndirect")`（1435 回目。NULL）の後に `LoadLibraryW("avrt")`（1436 回目）を呼ぶ。この順は上の文書 §250 の引用（`dwmapi` → `user32` → `avrt` …）と一致する。`avrt.dll` の扱い（仮想か absent か）は別の文書の範囲で、この文書は決めない。

## 4. DllMain の戻りと、その後のゲスト（実測。読み取りの gdb。`run/ro5-gdb.log`）

### 4.1 DllMain

| 項目 | 値 |
|---|---|
| `call_dllmain(idx, 1)`（DLL_PROCESS_ATTACH）の戻り値 | **0x1** |
| `fle_realmod_load` | `m->state = ST_LOADED`（`fle_realmod.c:144`）に到達。`call_dllmain(idx, 0)`（DETACH）は呼ばれなかった |
| `LoadLibraryW` が呼び出し元に返した値 | **0x2000000** |
| DllMain の中の import | **160 回、34 種** |

DllMain の中の import の種類と回数（160 回の内訳。実測）: InitializeCriticalSectionAndSpinCount 82、GetProcAddress 7、HeapAlloc 7、MultiByteToWideChar 6、WideCharToMultiByte 6、EnterCriticalSection 6、LeaveCriticalSection 6、LoadLibraryExW 5、LCMapStringW 4、GetStdHandle 3、QueryPerformanceCounter 2、GetCPInfo 2、FlsGetValue 2、FlsSetValue 2、その他 20 種が各 1 回（GetSystemTimeAsFileTime、GetCurrentThreadId、GetCurrentProcessId、TlsAlloc、TlsSetValue、GetProcessHeap、FlsAlloc、GetLastError、SetLastError、GetStartupInfoW、GetCommandLineA、GetCommandLineW、GetACP、IsValidCodePage、**GetStringTypeW**、InitializeSListHead、GetModuleFileNameW、GetEnvironmentStringsW、FreeEnvironmentStringsW、HeapFree）。

DllMain の中の GetProcAddress 7 回（通し番号 7、10、12、15、131、132、150）は**すべて NULL を返した**（実測。引数と戻り値は `ro5-gdb.log`）: 7 と 15 は `InitializeCriticalSectionEx`（hModule は `api-ms-win-core-synch-l1-2-0` = 0x7ff8001b0000）、10 は `FlsAlloc`、12 は `FlsSetValue`（`api-ms-win-core-fibers-l1-1-1` = 0x7ff8001c0000）、131 は `LCMapStringEx`、132 は `LocaleNameToLCID`（`api-ms-win-core-localization-l1-2-1` = 0x7ff8001d0000）、150 は `AreFileApisANSI`（`kernel32` = 0x7ff800090000）。決定 0081 のとおり、api-ms の下に応答表の行が無く、`kernel32` にこの名前の行も無い。この点は調査 `dsp-ippv2-ctype-locale-data.md` §3.2 の読み（Locale 0 の LCMapStringW は、GetProcAddress("LCMapStringEx") が NULL のときに入る）と整合する。

調査の gdb 測定（仮の値）と、この実行（決定 0087 の runtime）の比較: 調査の `wineall` run は import 160 回で DllMain が eax = 1。この実行も 160 回で eax = 1。**import の列の名前、引数の先頭 4 つ、戻りアドレスが 127〜140 回目の GetStringTypeW、LCMapStringW、WideCharToMultiByte で調査 §3.1、§3.2 の表と一致**（`ro3-gdb.log` の 127、133、134、135、138、139、140 回目。GetStringTypeW は dwInfoType 1、cchSrc 0x100。LCMapStringW は Locale 0、dwMapFlags 0x100／0x200、cchSrc 0x100。WideCharToMultiByte は cp 0x4e4（1252）、dwFlags 0、cch 0x100）。

### 4.2 DllMain の後（161 回目以降。LoadLibraryW が戻った後。1436 回目で停止）

| API | 回数 | 備考（実測） |
|---|---:|---|
| `SetErrorMode` | 1 | 161 回目 |
| `WideCharToMultiByte` | 848 | cp 0x4e4（1252）、dwFlags 0（848 回すべて。実測）。**停止せず通った**。1 つの名前につき 2 回の組が 424 組で、各組の直後に GetProcAddress が 1 回続く（実測の順）。2 回はサイズ問い合わせと変換と読める（読み。cbMultiByte などのスタック引数は読んでいない） |
| `GetProcAddress` | **424** | **非 NULL 423、NULL 1**（下） |
| `GetModuleHandleW` | 1 | 1432 回目。引数（rcx）0x65d3fe は FLEngine 内の文字列。`external-modules-usage.md` の 0x65cc9a の行（`GetModuleHandleW(L"user32")`）と同じ呼び出し点の組（読み。引数の文字列そのものは gdb で読んでいない）。戻り 0x7ff800130000 = 仮想モジュール `user32.dll`（上の算出） |
| `LoadLibraryW` | 2 | 1431 回目 `dwmapi`（戻り 0 = NULL）、1436 回目 `avrt`（停止） |

GetProcAddress の 424 回（実測。戻りアドレスはすべて 0x42364e = FLEngine の GetProcAddress のラッパー）:

- hModule = **0x2000000**（実在モジュール dsp_ippv2 自身）: **423 回、すべて非 NULL**。名前は `ippMalloc`、`ippFree`、`ippSetFlushToZero`、…、`ippiRotateCenter_8u_C4R`、`ippiMul_8u_C4IRSfs`（異なる名前 420 種。同じ名前が 3 回引かれたものがある）。戻り値は 0x273c5c0、0x273c600 など、**実在モジュールの中のアドレス**（`fle_realmod_getproc` が DLL のエクスポート表を引いた結果）。
- hModule = 0x7ff800130000（`GetModuleHandleW` の戻り。仮想モジュール `user32.dll`）: 1 回、名前 `UpdateLayeredWindowIndirect`、**NULL**（応答表にこの名前の行が無い）。

この GetProcAddress の連続は、FLEngine が `dsp_ippv2_x64.dll` の IPP のエクスポートを関数ポインタの表に集める処理（読み。1 つの名前につき WideCharToMultiByte 2 回 + GetProcAddress 1 回の組が繰り返される）。**ゲストが実在モジュールの関数を引いた結果は、すべて非 NULL だった**。

## 5. 決定 0087 の runtime の入力の確認（実測）

| 項目 | 結果 |
|---|---|
| GetStringTypeW の入力 | 127 回目。dwInfoType 1、cchSrc 256、lpSrcStr 0x7ffff7bfecc0（スタック）、lpCharType 0x7ffff7bff290。**調査 §3.1 と同じ**。停止せず、戻り値は次の import に進んだこと（実測）から非 0（`ro3-gdb.log` は GetStringTypeW の R[0] を読んでいない。**戻り値そのものは未測定**。決定 0087 のテスト `gstw_c1_all_at_once_the_attach_input_and_the_page_rules` が ATTACH の 256 単位の入力で 1 を返すことを確かめる） |
| LCMapStringW | 4 回（133、134、138、139）。Locale 0、dwMapFlags 0x100／0x200。**D の外の文字は渡されなかった**（停止しなかった。決定 0087 の「D の外は停止」に当たらない） |
| WideCharToMultiByte（cp 1252） | 2 回（135、140）。D の中の 256 単位。停止せず通った |
| DllMain の後の WideCharToMultiByte 848 回 | cp 1252、dwFlags 0、**停止せず通った**（実測）。入力が D の中だったことは、停止しなかったこと（D の外なら名前付き停止）から言える。入力が ASCII の関数名であることは読み（名前の長さ r9 は 0xf〜0x14 など。文字列そのものは読んでいない） |

## 6. 単独構成（FLEngine だけ。実測）

`OUT=…/out-ctype87/a4single JOBS=2 bash harness/wasm/build_native.sh` → `OUT=… bash harness/a4-survey.sh`。生成 C の分割（17 ファイル）は `lift/out-git/a4m85/out/` からコピー。**`fle_locale.o` を含む runtime のオブジェクトは新しい OUT で最初から作った**（`build_native.sh` は既存の `.o` を飛ばすため、古いオブジェクトを使わないように新しい OUT にした）。

```
stop   : fle: LoadLibraryW: module "dsp_ippv2_x64.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)
         top of stack: 0x423990 f_423990
depth  : init_count=145 / 942   last unit: index=144 run_rank=132/895 proc=0x65c540 in_progress
funcs  : executed (distinct)=821  entries=15685
```

基準（init_count 145/942、functions 821、entries 15,685、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`）と**一致**。

## 7. 検証（実測）

- `cargo test --workspace --no-fail-fast`（`FLENGINE_PE=binaries/win64/FLEngine_x64.dll`）: **95 バイナリ、1,125 件通過、0 失敗、0 無視**（基準 95 / 1,113 / 0 / 0。+12 は決定 0087 の `locale.rs` の新規 12 件）。
- FLEngine の生成 C: `fe-lift lift ../binaries/win64/FLEngine_x64.dll out-ctype87/fle.c --pdata` の SHA-256 `43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1`（基準と一致。215,707,512 B。`unsupported_total 525`）。
- 2 モジュール構成の生成物: §1 の 1。

## 8. 未確認、限界

- **DllMain の後の実行は 1,276 回の import で `LoadLibraryW("avrt")` に達した**。その先（avrt の扱い）は測っていない。**新しい停止は直していない**（範囲外）。
- 実機 Windows での挙動は未確認（ホストは Linux）。決定 0087 の値（特に C1 の UPPER と LOWER 以外のビット）は非公式で、実機の値ではない。
- GetStringTypeW の戻り値そのもの（R[0]）は、この実行の gdb では読んでいない（次の import に進んだこと、テストの 1 から読んだ）。
- GetProcAddress の戻り値（非 NULL 423）が、実機で同じ名前に対して同じように非 NULL かは未確認（実機のエクスポート表は同じファイルだが、api-ms の NULL は決定 0081 の仮の選択）。
- DllMain の後の ipp の関数名の連続が何をしているかは読み（関数ポインタの表の作成）。実行して確かめていない。
- wasm は使っていない（native のみ）。

## 9. 作ったファイル

- 本文書 `docs/investigations/two-module-after-0087.md`（追跡下。新規）。
- `lift/out-ctype87/`（gitignore 対象）: `test-ws.txt`（`cargo test --workspace` の出力）、`test-pre*.txt`、`test-mutants*.txt`（テストの変異検査）、`fle.c`（FLEngine の生成 C）、`a4single/`（単独構成。`native/`、`a4/`）、`two/`（2 モジュール構成。`dll/`、`gen/`、`src/`、`native/`、`v-tbl2/`）、`out-tbl2/a4/`（a4-survey のログ。gdb なし 2 回）、`run/ro3〜ro6-*`（読み取りの gdb のログ）、`gdb/ro_imports.py`、`quotecheck.py`（決定 0087 の Learn の引用の機械照合）、`mut/`（変異検査用の runtime のコピー）、`NOTE-sha.txt`、`tmp/`。
