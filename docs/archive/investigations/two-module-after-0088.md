# 決定 0088 の後の 2 モジュール構成（実 dsp_ippv2_x64.dll）の native 実行: avrt と PowrProf は通り、次の停止は LoadLibraryW "blend2d.dll"（測定のみ）

実行日: 2026-10-24（JST。ログの時刻は `20261024T0522…Z`）。基準の HEAD は作業開始時 `2441cde`（決定 0087）。作業中に別の作業が docs だけの 2 コミット（`c1eefb5`、`ca33c3a`。`docs/investigations/wasm-locals-15-functions.md`、`wasm-two-module-o1.md`）を進めたが、`git diff 2441cde HEAD --stat -- lift` は空（`lift/` は変わっていない。実測）。測定に使ったのは、この HEAD の `lift/` に決定 0088 の変更（`lift/runtime/fle_modules.c` の 2 行と `lift/crates/fe-lift/tests/` のテスト）を足した作業コピー。決定 0088 は `docs/decisions/0088-avrt-powrprof-virtual-modules.md`。**この文書は何も決めない**。**runtime の応答を足していない。gdb で値を入れていない**（gdb は `fle_imp_*` の入口で読むだけ。`return` も `set var` も使っていない。下の §2）。**新しい停止は直していない**（範囲外）。

作業物は `lift/out-avrt88/`（gitignore 対象）。直前の測定は `docs/investigations/two-module-after-0087.md`（以下「0087 後」）。

## 0. 凡例

- **実測** = このセッションでコマンドを実行して得た値。ログは `lift/out-avrt88/` の下。
- **読み** = 逆アセンブル、生成 C、runtime のソース、別の文書を読んだ結果。実行して確かめていない。
- **未確認** = そのとおり。
- 「DllMain」は dsp_ippv2_x64.dll のエントリ（`f_37caca8`。基底 0x2000000 の VA）。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | 構成と生成物 | 0087 後と同じ構成（`tbl2`、種 452 個）を、作業コピーから最初から再ビルド（新しい OUT。runtime のオブジェクトは新しく作った）。FLEngine の C は SHA-256 `43985d62…37137a1`（`fe-lift lift … --pdata` は `unsupported_total 525`。`gen_two.sh` の検査も通過）。リベース済み DLL `4fd25502…efc5`、縮小 DLL の C `6035da1d…279dbf`、`common.c` `33779308…487d5f` は 0087 後の値と一致 | 実測 |
| 2 | **a4 の行（`tbl2`、gdb なし 2 回）** | init_count = **150 / 942**（0087 後は 146 / 942）、last unit = index **149**（run_rank 137/895、proc `0x664260`、in_progress）、functions_distinct = **943**（935）、function_entries = **26,585**（25,215）、all_units_done = no。2 回とも同一 | 実測 |
| 3 | **次の停止** | **`LoadLibraryW`、モジュール名 `blend2d.dll`**（`fle_load.c:174` の `fle_stop`。1525 回目の import）。メッセージ全文と呼び出し元のスタックは §3 | 実測 |
| 4 | **DllMain(ATTACH) が戻ったか** | **戻った。eax = 0x1**（`call_dllmain(idx, 1)` の C の戻り値。読み取りの gdb の FinishBreakpoint で測定。0087 後と同じ）。DllMain の中の import は 160 回、34 種 | 実測 |
| 5 | unit 145（`f_65c9f0`）の 4 名 | `dwmapi`: 戻り 0（absent）、`avrt`: 戻り **`0x7ff8001f0000`**（N = 31）、`PowrProf.dll`: 戻り **`0x7ff800200000`**（N = 32）、`SHCore.dll`: 戻り 0（absent）。**4 名とも通過し、145 → 次の unit に進んだ**（init_count 146 → 150）。各名前の詳細は §4 | 実測 |
| 6 | avrt の 11 名、PowrProf の 2 名の `GetProcAddress` | **13 回すべて NULL**（avrt 11 回は hModule `0x7ff8001f0000`、PowrProf 2 回は `0x7ff800200000`）。0 番地を呼ぶ経路には**届かなかった**（停止も SIGSEGV も無し） | 実測 |
| 7 | 単独構成（FLEngine だけ） | init_count **145 / 942**、functions **821**、entries **15,685**、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`。**基準と一致** | 実測 |
| 8 | 再現性 | gdb なしの a4-survey 2 回: stdout の差は `calling entry … rsp=` の 1 行だけ（アドレス。ASLR）、stderr は完全に一致。gdb の読み取りの run 2 回（`ro1`、`ro2`）は import ログが `cmp` で全バイト一致 | 実測 |

つまり、決定 0088 の runtime で、**avrt と PowrProf.dll は LoadLibraryW が成功し、GetProcAddress は全部 NULL でも実行は止まらず、unit 145 は終わって unit 149 の `Blend2D.dll` の `LoadLibraryW` で名前付き停止した**。unit 149 は決定 0086 が「実体を lift して実在モジュールにする」方向を決めた対象で、この文書は扱わない。

## 2. 手順（実測。実行した順）

`lift/` から実行。TMPDIR は `lift/out-avrt88/tmp`（`/tmp` は使っていない）。

```
CARGO_TARGET_DIR=$PWD/out-avrt88/target FLENGINE_PE=…/binaries/win64/FLEngine_x64.dll cargo test --workspace --no-fail-fast   # 95 binaries / 1,130 passed
out-avrt88/target/debug/fe-lift lift ../binaries/win64/FLEngine_x64.dll out-avrt88/fle.c --pdata       # SHA 43985d62…
OUT=$PWD/out-avrt88/a4single JOBS=2 bash harness/wasm/build_native.sh        # 生成 C の 17 分割は out-git/a4m87/out/ からコピー
OUT=$PWD/out-avrt88/a4single bash harness/a4-survey.sh
CARGO_TARGET_DIR=$PWD/out-avrt88/target cargo build --release --offline -p fe-pe -p fe-lift
# リベース済み DLL と種 3 ファイルは lift/out-ctype87/two/dll/ からコピー（SHA-256 は out-two-post0083/dll/ の 4 ファイルと一致）
OUT=$PWD/out-avrt88/two FEBIN=$PWD/out-avrt88/target/release TAG=tbl2 SEEDS=…/out-avrt88/two/dll/seeds-idx0-plus-initterm2.txt \
    REAL=$PWD/out-ext/dsp_ippv2_x64.dll bash harness/two/gen_two.sh rebase gen split merge
TAG=tbl2 OUT=$PWD/out-avrt88/two JOBS=4 bash harness/two/build_two_native.sh          # two-native 571,229,216 B
DLL=…/binaries/win64/FLEngine_x64.dll OUT=…/out-avrt88/out-tbl2 NATIVE_BIN=harness/two/two-native-run.sh TWO_NATIVE=…/v-tbl2/native/two-native \
    TWO_REAL=…/two/dll/dsp_ippv2_x64.2000000.dll FEPE=…/target/release/fe-pe bash harness/a4-survey.sh        # 2 回（05:22:23Z と 05:22:28Z）
GDBI_BIN=… GDBI_REAL=… GDBI_FLE=… GDBI_OUT=…/run/roN-imports.txt GDBI_STDOUT=… GDBI_STDERR=… gdb -q -batch -x out-avrt88/gdb/ro_imports.py   # 読み取りだけ。2 回（ro1、ro2）
```

- `gen_two.sh` の `rebase` 段は出力（`dll/dsp_ippv2_x64.2000000.dll`）が既にあるので「exists」と出て飛ばす（コピーした 4 ファイルは SHA-256 が一致するので、再生成と同じ内容）。FLEngine の生成 C と縮小 DLL の C、分割、合流は**この OUT で最初から**作った（`gen_two.sh` が FLEngine の C の SHA-256 を期待値と照合して通過した）。
- ビルドは新しい作業ディレクトリで、runtime のオブジェクトを含め最初から作った（`build_two_native.sh` と `build_native.sh` は作ったオブジェクトを飛ばす仕様なので、新しい OUT を使った）。`out-avrt88/two/native/rt/fle_modules.o` に `avrt.dll`、`powrprof.dll` の文字列があることを `strings` で確認した（実測）。
- a4-survey を同じ秒に 2 回続けて実行するとログのファイル名（秒単位）が衝突するので、間を空けた（05:22:23Z、05:22:28Z）。
- gdb の `ro_imports.py`（`lift/out-avrt88/gdb/`）は 0087 後の `lift/out-ctype87/gdb/ro_imports.py` の無改変のコピー。値を入れる機構を持たない（`gdb.execute` は `set pagination off`、`set confirm off`、`set print thread-events off`、`file`、`set args`、`info functions`、`run`、`bt`、`signal SIGABRT` だけ。`return`、`set var`、`call` は使っていない）。`call_dllmain` に breakpoint を置き、FinishBreakpoint で戻り値（`int`）を読む。`fle_imp_GetProcAddress`、`LoadLibraryW`、`GetModuleHandleW`、`LoadLibraryExW` の戻り値も FinishBreakpoint で読む。環境変数は `GDBI_BIN`、`GDBI_REAL`、`GDBI_FLE`、`GDBI_OUT`（存在しない）、`GDBI_STDOUT`、`GDBI_STDERR`。

## 3. 停止（gdb なし。実測）

`lift/out-avrt88/out-tbl2/a4/20261024T052223Z-native.txt`（と `…052228Z…`）:

```
stop   : fle: LoadLibraryW: module "blend2d.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)
         top of stack: 0x423990 f_423990
depth  : init_count=150 / 942   last unit: index=149 run_rank=137/895 proc=0x664260 in_progress
         started=137 completed=136 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=943  entries=26585
```

停止メッセージ全文（stderr。1 行。2 回とも同一）:

```
fle: LoadLibraryW: module "blend2d.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)
```

出どころ: `lift/runtime/fle_load.c` の `load_by_name`（:174。`fle_stop`）。`blend2d.dll` は仮想モジュールの表にも `fle_load_absent[]` にも、実在モジュールの表（`fle_realmod`。名前は `dsp_ippv2_x64.dll` の 1 つだけ）にも無い。ゲストが渡した名前は `Blend2D.dll`（gdb のログ。runtime が小文字にして `.dll` の有無を整える）。

### 3.1 呼び出し元のスタック（実測）

ハーネスの報告（`call depth at stop: 9`）:

```
#0 0x423990  f_423990  guest-return=0x44db70
#1 0x44db40  f_44db40  guest-return=0x653a56
#2 0x6539e0  f_6539e0  guest-return=0x6641be
#3 0x664190  f_664190  guest-return=0x664279
#4 0x664260  f_664260  guest-return=0x412129
#5 0x4120c0  f_4120c0  guest-return=0x41229a
#6 0x412160  f_412160  guest-return=0x41d989
#7 0x41d8f0  f_41d8f0  guest-return=0x1376875
#8 0x1376820 f_1376820 guest-return=0xdead0000
```

gdb の C の backtrace（`run/ro1-imports.txt`。`#0`〜`#4` は libc の abort）:

```
#5  fle_stop (api="LoadLibraryW", why="module \"blend2d.dll\" is not a virtual module …")   fle_env.c:13
#6  load_by_name (api="LoadLibraryW", u=…, n=11)                    fle_load.c:174
#7  fle_imp_LoadLibraryW ()                                         fle_load.c:184
#8  f_423990 ()    two/src/fle-00.c:88395          （FLEngine の LoadLibraryW のラッパー）
#9  f_44db40 ()    two/src/fle-00.c:115377         （SetErrorMode → LoadLibraryW → SetErrorMode のラッパー。調査 §3.1）
#10 f_6539e0 ()    two/src/fle-03.c:65933
#11 f_664190 ()    two/src/fle-03.c:77176
#12 f_664260 ()    two/src/fle-03.c:77203          （unit 149 の proc。0x664260）
#13 fle_dispatch (va=6701664)
#14 f_4120c0 ()    #15 f_412160 ()    #16 f_41d8f0 ()    #17 f_1376820 ()（FLEngine のエントリ）    #18 main ()
```

ゲストのレジスタ（停止時）: rcx = 0x7ffff6bd7a40、rdx = 0x8000（SEM_NOOPENFILEERRORBOX。`f_44db40` の第 2 引数）、r8 = 0x0、r9 = 0x0、rsp = 0x7ffff7bffb08、rax = 0x7ffff6bd7a40。

**呼び出し元の読み**: `f_664260` は unit 149 の proc（`units.tsv` の index 149、proc_va 0x664260、second_va 0x664250、run_rank 137。実測）。`f_664190` ← `f_664260`、`f_6539e0` を経て `f_44db40` が `Blend2D.dll` を引く。この呼び出し元の連鎖は、決定 0086 の表の 149 行（Blend2D 調査 §4.1 の読みの `f_664190` ← `f_664260`）と一致する。`f_6539e0` は 0x653a48 の `SetDllDirectoryW`（1523 回目）、0x653a56 で `f_44db40` を呼ぶ（戻りアドレス 0x653a56。実測の `guest-return`）。Blend2D の扱いは決定 0086 の方向の決定（実在モジュールにする）の範囲で、この文書は決めない。

## 4. unit 145（`f_65c9f0`）の 4 名と、DllMain の戻り（実測。読み取りの gdb。`run/ro1-imports.txt`）

### 4.1 DllMain

| 項目 | 値 |
|---|---|
| `call_dllmain(idx, 1)`（DLL_PROCESS_ATTACH）の戻り値 | **0x1** |
| `fle_realmod_load` | `m->state = ST_LOADED`（`fle_realmod.c:144`）に到達 |
| `LoadLibraryW` が呼び出し元に返した値 | **0x2000000** |
| DllMain の中の import | **160 回、34 種**（0087 後と同じ） |
| DllMain の後（161 回目以降）の import | **1,365 回**（161〜1525 回目。1525 回目が停止の `LoadLibraryW`）。0087 後は 161〜1436 回目（1,276 回） |

### 4.2 `f_65c9f0` が引く 4 名と、それぞれの通り方

`f_65c9f0` の `LoadLibraryW`／`GetModuleHandleW` の実測（`ro1-imports.txt`。名前は gdb が `rcx` の先の文字列を読んだ値。戻りは FinishBreakpoint で読んだ R[0]）:

| 通し番号 | 呼び出し | 引数の名前 | 戻り | 通り方 |
|---|---|---|---|---|
| 1431 | `LoadLibraryW` | `dwmapi` | **0x0** | absent（`fle_load_absent[]`。決定 0015/0029）。直後に `GetModuleHandleW`（1432）へ進んだ（0087 後と同じ） |
| 1432 | `GetModuleHandleW` | （`user32`。読み） | `0x7ff800130000`（仮想 `user32.dll`） | 1435 で `GetProcAddress(0x7ff800130000, "UpdateLayeredWindowIndirect")` が NULL（0087 後と同じ） |
| **1436** | `LoadLibraryW` | **`avrt`** | **`0x7ff8001f0000`**（N = 31） | **成功**（0087 後はここで名前付き停止） |
| 1439〜1469 | `GetProcAddress` × 11 | `AvSetMmMaxThreadCharacteristicsW`、`AvSetMmThreadCharacteristicsW`、`AvSetMmThreadPriority`、`AvQuerySystemResponsiveness`、`AvRevertMmThreadCharacteristics`、`AvRtCreateThreadOrderingGroup`、`AvRtCreateThreadOrderingGroupEx`、`AvRtDeleteThreadOrderingGroup`、`AvRtJoinThreadOrderingGroup`、`AvRtLeaveThreadOrderingGroup`、`AvRtWaitOnThreadOrderingGroup`（hModule はすべて `0x7ff8001f0000`） | すべて **0x0** | 応答表に avrt の行が無いので NULL と 127（0088 の決定 3 のとおり）。**調査 §3.1 の 11 名と順序・綴りが一致** |
| 1470 | `GetModuleHandleW` | （`kernel32`。読み） | `0x7ff800090000`（仮想 `kernel32.dll`） | 1473〜1491 の 7 名（`InitializeSRWLock` ほか SRW ロックの 7 名）の `GetProcAddress` は、hModule `0x7ff800090000` で**すべて NULL**（応答表に `kernel32.dll` のこれらの名前の行が無い） |
| 1492 | `LoadLibraryW` | `advapi32` | `0x7ff800010000`（N = 1） | 成功（PE の import 表の DLL）。1495 `RegCopyTreeW`（kernel32 の hModule `0x7ff800090000`）と 1498 `CheckTokenMembership`（advapi32 の hModule）は NULL |
| **1499** | `LoadLibraryW` | **`PowrProf.dll`** | **`0x7ff800200000`**（N = 32） | **成功** |
| 1502、1505 | `GetProcAddress` × 2 | `PowerGetActiveScheme`、`PowerSetActiveScheme`（hModule `0x7ff800200000`） | どちらも **0x0** | NULL と 127。**調査 §3.1 の 2 名と一致** |
| 1506 | `LoadLibraryW` | `SHCore.dll` | **0x0** | absent（`fle_load_absent[]`）。直後の 1507 `GetFileAttributesW` へ進んだ |

- 実測のとおり、**4 名とも `f_65c9f0` は先へ進んだ**（0087 後は 1436 の `avrt` で停止し unit 145 が未完のまま、init_count 146。今回は 1506 の `SHCore.dll` の後に 1507〜1525 を実行し、unit 149 の `Blend2D.dll` に届いて init_count 150）。
- **avrt と PowrProf の `GetProcAddress` は 13 回すべて NULL**。その後に、NULL のスロットを読んで 0 番地を呼ぶ経路には**届かなかった**（停止も SIGSEGV も無い）。ただしこれは**この実行が停止するまでの範囲**の観察で、停止点より先、特に avrt の 8 スロットの検査の有無（調査 §4.1、決定 0088 の「理由」）は**未確認のまま**。
- 通し番号 1507〜1525（`f_65c9f0` の後、unit 145 の終わりから 149 の `Blend2D.dll` まで）の import: `GetFileAttributesW` 4 回、`GetLastError` 5 回、`GetModuleFileNameW` 1 回（rcx = NULL）、`RegOpenKeyExW` 6 回（rcx = 0xffffffff80000001（HKCU）と 0xffffffff80000002（HKLM）、各 3 回）、`SetDllDirectoryW` 1 回、`SetErrorMode` 2 回（0x8000。`f_44db40` の前後）、`LoadLibraryW` 1 回（`Blend2D.dll`）。この 1507〜1525 が unit 145 の残りか、146〜149 のどれの import かは、gdb のログ（import の入口）だけからは**切り分けていない**（unit の境界は import のログに出ない。未確認）。a4 の行（init_count 150、last unit 149）と `units.tsv`（index 146 = proc 0x662e50、147 = 0x6634a0、148 = 0x664180、149 = 0x664260。実測）が、145 から 149 まで進んだことを示す。
- DllMain の後の `GetProcAddress` の集計（実測。`ro1-imports.txt` の 161 回目以降）: 446 回（0087 後は 424 回）。**非 NULL 423 回、NULL 23 回**（0087 後は 423 / 1）。非 NULL は hModule = `0x2000000`（実在モジュール）の 423 回（0087 後と同じ）。NULL 23 回の内訳: `0x7ff800130000`（user32）1、`0x7ff800090000`（kernel32）8、`0x7ff800010000`（advapi32）1、`0x7ff8001f0000`（avrt）11、`0x7ff800200000`（PowrProf）2。

## 5. 単独構成（FLEngine だけ。実測）

`OUT=…/out-avrt88/a4single JOBS=2 bash harness/wasm/build_native.sh` → `OUT=… bash harness/a4-survey.sh`。生成 C の分割（17 ファイル）は `lift/out-git/a4m87/out/` からコピー。**`fle_modules.o` を含む runtime のオブジェクトは新しい OUT で最初から作った**（`build_native.sh` は既存の `.o` を飛ばすため）。

```
stop   : fle: LoadLibraryW: module "dsp_ippv2_x64.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)
         top of stack: 0x423990 f_423990
depth  : init_count=145 / 942   last unit: index=144 run_rank=132/895 proc=0x65c540 in_progress
funcs  : executed (distinct)=821  entries=15685
```

基準（init_count 145/942、functions 821、entries 15,685、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`）と**一致**。単独構成は unit 144 で止まり、avrt（unit 145）に届かないので、決定 0088 の影響を受けない。

## 6. 検証（実測）

- `cargo test --workspace --no-fail-fast`（`FLENGINE_PE=binaries/win64/FLEngine_x64.dll`）: **95 バイナリ、1,130 件通過、0 失敗、0 無視**（基準 95 / 1,125 / 0 / 0。+5 は決定 0088 の新規 5 件）。出力は `lift/out-avrt88/test-ws.txt`。
- FLEngine の生成 C: `fe-lift lift ../binaries/win64/FLEngine_x64.dll out-avrt88/fle.c --pdata` の SHA-256 `43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1`（基準と一致。215,707,512 B。`unsupported_total 525`）。**変わらなかった**。
- 2 モジュール構成の生成物: §1 の 1。
- 変異検査（決定 0088 の「テスト」）: `lift/out-avrt88/mut/` の runtime のコピー 2 通りで、新規テストが落ちる（決定の根拠ではない自己検査）。

## 7. 未確認、限界

- **新しい停止は直していない**（範囲外）。次の停止は `LoadLibraryW "blend2d.dll"`（unit 149）で、決定 0086 の方向の決定の対象。Blend2D の lift と実在モジュール化は別の作業。
- **停止点より先の avrt、PowrProf の使い手の動作は測っていない**。特に avrt の 8 スロット（`AvSetMmMaxThreadCharacteristicsW`、`AvQuerySystemResponsiveness`、`AvRtCreateThreadOrderingGroup`、`…Ex`、`AvRtDeleteThreadOrderingGroup`、`AvRtJoinThreadOrderingGroup`、`AvRtLeaveThreadOrderingGroup`、`AvRtWaitOnThreadOrderingGroup`）の検査の有無は未確認（調査 §4.1、§8 の 2）。この実行で 0 番地呼びが起きなかったのは、実行がそこまで届いていないだけかもしれない（avrt の使い手 4 関数は、この実行では呼ばれたかを測っていない。関数単位の計数をしていない）。
- **ゲストが実機と違う経路を通る**: 実機の Windows 10 では avrt、PowrProf の関数が存在し `GetProcAddress` は非 NULL を返す。この runtime（決定 0088）では NULL なので、ゲストは「機能が無い」経路を通る（決定 0088 の「限界」）。この実行は、その経路で先へ進めることを示すだけで、実機の挙動の再現ではない。
- 実機 Windows での挙動は未確認（ホストは Linux）。
- gdb は読み取りだけ。DllMain の戻り（0x1）は 0087 後と同じ値で、決定 0088 は DllMain の前に効かない（avrt は DllMain の後）。
- a4 の行の 146 → 150 は、unit 145 が完了し 146〜149 が started になったことを示す。**init_count が動いたのは決定 0071（10-23 00:18、115 → 145）以来**という `docs/progress.md` の指標の文脈では、2 モジュール構成の init_count 146 → 150 は「停止点の移動」であり、FLEngine 単独構成（145 のまま）の進捗ではない。progress.md に書くときの扱いは未決（この文書は書かない）。
- wasm は使っていない（native のみ）。

## 8. 作ったファイル

- 本文書 `docs/investigations/two-module-after-0088.md`（追跡下。新規）。
- `lift/out-avrt88/`（gitignore 対象）: `test-ws.txt`（`cargo test --workspace` の出力）、`test-focus.txt`（対象 7 ファイル）、`test-mut-*.txt`（変異検査）、`fle.c`（FLEngine の生成 C）、`fle-lift.log`、`a4single/`（単独構成。`native/`、`out/`、`a4/`）、`a4single-build.txt`、`a4single-survey.txt`、`two/`（2 モジュール構成。`dll/`、`gen/`、`src/`、`native/`、`v-tbl2/`）、`two-gen.txt`、`two-build.txt`、`two-survey-1.txt`、`two-survey-2.txt`、`two-cargo-release.txt`、`out-tbl2/a4/`（a4-survey のログ。gdb なし 2 回）、`run/ro1〜ro2-*`（読み取りの gdb のログ）、`gdb/ro_imports.py`、`mut/`（変異検査用の runtime のコピー）、`target/`、`tmp/`。
