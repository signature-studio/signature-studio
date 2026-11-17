# 決定 0083 の後の 2 モジュール構成（実 dsp_ippv2_x64.dll）で DllMain(ATTACH) がどこまで進むか

実行日: 2026-10-23（UTC。ログの時刻は `20261023T15…Z`）。HEAD `7a9af0f15d63c350ce8ccb5858987473cbea9a8d`（main）。決定 0083（コミット c68dbce）の後。`stage23-two-module-real-dll.md`（段 2・3。以下「stage23」）の再測定。**この文書は何も決めない**。決定が要る点は §9 に列挙した。

作業物は `lift/out-two-post0083/`（gitignore 対象。追跡下のファイルは編集していない）。runtime、fe-lift、docs/decisions、既存のテスト、既存の docs は編集していない。

## 0. 凡例

- **実測** = このセッションでコマンドを実行して得た値。ログは `lift/out-two-post0083/` の下にある。
- **読み** = 逆アセンブル（capstone）、生成 C、runtime のソースを読んだ結果。実行して確かめていない。
- **仮の値** = gdb が止まった所に一時的に入れた値。**決定ではなく、runtime の挙動でもない**。runtime には何も実装していない。
- **未確認** = 確認できていない。
- 仕様（Microsoft Learn のページ）は今回取得し直していない。仕様の記述は stage23 §5.1 の読みを引用したもので、**この作業では再確認していない**。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | 構成 | stage23 の `tbl2`（種 452 個。縮小 DLL 8,030 関数）と `idx0`（種 428 個。7,992 関数）を、HEAD の `fe-pe`、`fe-lift`、`lift/runtime` で再生成・再ビルド（native）。種の定義は変えていない | 実測 |
| 2 | 生成物の同一性 | FLEngine の C、縮小 DLL の C（`tbl2`）、マージ後の `common.c`、リベース済み DLL は、stage23 の出力と **SHA-256 が一致**。種ファイル 3 つも **全バイト一致** | 実測 |
| 3 | **gdb なし**の停止（`tbl2`） | **`MultiByteToWideChar: byte of 0x80 or above in a single-byte code page: table not on the MSDN pages …`**（§3）。`GetStartupInfoW` の停止は出ない | 実測 |
| 4 | DllMain に入ったか | **入った**。`LoadLibraryW("dsp_ippv2_x64.dll")` → `fle_realmod_load` → `call_dllmain` → `f_37caca8`（rcx = 0x2000000、rdx = 1、r8 = 0）。**戻らない** | 実測 |
| 5 | DllMain 内の import | 停止まで **125 回、26 種**（125 回目が停止した `MultiByteToWideChar`）。戻りアドレスはすべてモジュール内（§4） | 実測 |
| 6 | a4 の行（`tbl2`） | init_count = **145 / 942**、最後の unit = index 144（run_rank 132/895、proc 0x65c540、in_progress）、functions_distinct = **881**、function_entries = **15,934**、all_units_done = no | 実測 |
| 7 | 種 428 個だけ（`idx0`） | stage23 §3.1 と同じ。`fle: indirect call to 0x37d23b4 is not a known function entry`、145/942、836 / 15,702 | 実測 |
| 8 | 仮の値 3 段 | `MultiByteToWideChar`、`GetStringTypeW`、`LCMapStringW` を書き換えると、次の停止は `WideCharToMultiByte`（890 / 15,957）。DllMain は戻らない（§5） | 実測（仮の値つき） |
| 9 | stage23 との差 | **GetStartupInfoW の停止が消え、その分だけ 1 段先（`MultiByteToWideChar`）で止まる**。数値（881 / 15,934）は stage23 の「仮の値 1 段」（`CHAIN_MAX` = 1）の行と一致（§7） | 実測 |

## 2. 手順（実測。実行した順）

stage23 §2 と `lift/harness/two/` のスクリプトを**無改変**で使った。TMPDIR は常に `lift/out-two-post0083/tmp`（`/tmp` は使っていない）。`lift/` から実行。

```
export TMPDIR=$PWD/out-two-post0083/tmp
CARGO_TARGET_DIR=$PWD/out-two-post0083/target cargo build --release --offline -p fe-pe -p fe-lift
fe-pe rebase out-ext/dsp_ippv2_x64.dll 0x2000000 out-two-post0083/dll/dsp_ippv2_x64.2000000.dll
# 種（stage23 と同じ定義）
#   seeds-idx0-2000000.txt         = out-ipp/table-seeds-idx0.txt の各 VA から 0x17E000000 を引いて昇順（428 個）
#   seeds-idx0-plus-initterm.txt   = 上 + mkseeds.py --table 0x3ec8360 0x3ec8460（448 個）
#   seeds-idx0-plus-initterm2.txt  = 上 + mkseeds.py --table 0x3ebd270 0x3ebd2a8（452 個）
TAG=tbl2 SEEDS=…/seeds-idx0-plus-initterm2.txt OUT=out-two-post0083/two FEBIN=out-two-post0083/target/release bash harness/two/gen_two.sh
TAG=tbl2 OUT=out-two-post0083/two bash harness/two/build_two_native.sh
OUT=out-two-post0083/out-tbl2 NATIVE_BIN=harness/two/two-native-run.sh TWO_NATIVE=…/v-tbl2/native/two-native \
    TWO_REAL=…/dsp_ippv2_x64.2000000.dll FEPE=…/fe-pe bash harness/a4-survey.sh
# 同じことを TAG=idx0（SEEDS=seeds-idx0-2000000.txt）でも実施（比較用）
```

- 種ファイル 3 つは、stage23 の `lift/out-stage2/dll/` の同名ファイルと `cmp` で一致（読み取りだけで比較。stage23 のファイルは使っていない）。
- リベース: `fe-pe rebase` の出力 `new_base 0x2000000`、`dir64 15602`、`absolute 22`。SHA-256 `4fd25502…efc5`（stage23、決定 0080 の値と一致）。
- FLEngine の C: `gen_two.sh` の検査で SHA-256 `43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1`（期待値と一致。215,707,512 B）。
- 縮小 DLL の C（`tbl2`）: SHA-256 `6035da1d…279dbf`（266,619,561 B、3,224,242 行、8,030 関数、`bad_nodes` 0）。`out-stage2/two/v-tbl2/gen/real.c` と一致。`common.c`（マージ後 67,587 行、「両方にある行」260）も一致。`idx0` は 7,992 関数、67,549 行、260（stage23 §2 の数値と同じ）。
- native のビルドの警告は 0（`tbl2`、`idx0` とも）。実行ファイルは 571,226,640 B。2 GB 級の単一ファイルは作っていない（最大は `real.c` の 266 MB）。
- 実行は 3 回（`run/plain1〜3.out`）。停止メッセージ、スタック、a4 の行は 3 回とも同一。1 回は `/usr/bin/time -v` つきで、**壁時間 0.08 秒、最大 RSS 92,884 KB、終了コード 134**（SIGABRT。ハーネスが停止を報告して `_exit`）。
- gdb は、(a) breakpoint だけ（値を書かない。`run/ro-*`）、(b) 引数を読むだけのプローブ（`run/probe0-*`）、(c) §5 の仮の値つき（`run/post1〜3-*`）の 3 通りで使った。(a)(b) は、gdb なしの run と**同じ停止、同じ数値**（881 / 15,934）。
- `bt` を深くするため、`lift/harness/two/gdb_imports.py` の `bt 14` を `bt 80` に替えたコピーを `out-two-post0083/gdb/gdb_imports_bt80.py` に置いて使った（原本は触っていない）。

## 3. gdb なしの停止（`tbl2`。実測）

`lift/out-two-post0083/out-tbl2/a4/20261023T152730Z-native.txt`:

```
stop   : fle: MultiByteToWideChar: byte of 0x80 or above in a single-byte code page: table not on the MSDN pages (not defined by the MSDN page or outside the emulated locale; not implemented)
         top of stack: 0x37d7048 f_37d7048
depth  : init_count=145 / 942   last unit: index=144 run_rank=132/895 proc=0x65c540 in_progress
         started=132 completed=131 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=881  entries=15934
```

停止メッセージ全文（stderr。1 行）:

```
fle: MultiByteToWideChar: byte of 0x80 or above in a single-byte code page: table not on the MSDN pages (not defined by the MSDN page or outside the emulated locale; not implemented)
```

出どころ: `lift/runtime/fle_locale.c:503`（`fle_mb_to_wide`。`fle_locale_stop` が `abort()`）。

### 3.1 スタック（実測）

ハーネスの報告（`call depth at stop: 21`、先頭 10 段）:

```
#0 f_37d7048  guest-return=0x37d8738
#1 f_37d86c0  guest-return=0x37d6813
#2 f_37d674c  guest-return=0x37d6d85
#3 f_37d6cd8  guest-return=0x37d6aa1
#4 f_37d6934  guest-return=0x37d6cae
#5 f_37d6c5c  guest-return=0x37d7809
#6 f_37de9b0  guest-return=0x37d7809
#7 f_37de9d0  guest-return=0x37d7809
#8 f_37d2484  guest-return=0x37cb4b8
#9 f_37cb484  guest-return=0x37ca9fa
```

gdb の C の backtrace（`run/ro-imports.txt`）から関数名の連鎖（先頭から。`fle_*` は runtime）:

```
fle_locale_stop ← fle_mb_to_wide ← fle_imp_MultiByteToWideChar
← f_37d7048 ← f_37d86c0 ← f_37d674c ← f_37d6cd8 ← f_37d6934 ← f_37d6c5c
← fle_dispatch ← f_37de9b0 ← f_37de9d0 ← f_37d2484 ← f_37cb484 ← f_37ca988 ← f_37caca8（DllMain）
← fle_dispatch ← fle_call_guest ← call_dllmain（fle_realmod.c:117）← fle_realmod_load（fle_realmod.c:138）
← load_by_name（fle_load.c:152）← fle_imp_LoadLibraryW（fle_load.c:184）
← f_423990 ← f_44db40 ← f_6539e0 ← f_653f40 ← f_65c540（unit 144）
← fle_dispatch ← f_4120c0 ← f_412160 ← f_41d8f0 ← f_1376820（FLEngine のエントリ）← main
```

### 3.2 停止した呼び出しの引数（実測。gdb の読み取りだけ。`run/probe0-gdb.log`）

| 項目 | 値 |
|---|---|
| 呼び出し元（ゲスト） | `f_37d7048`（フラグを整えて import に `jmp` する薄い関数。読み。`L_37d709b` が `fle_imp_MultiByteToWideChar(); return;`）。その呼び出し元は `f_37d86c0` の 0x37d8733（`call 0x37d7048`。戻りアドレス 0x37d8738） |
| CodePage（cp） | **1252**（`GetACP` が返した値） |
| dwFlags | **0x1**（MB_PRECOMPOSED） |
| lpMultiByteStr | 0x7ffff7bfef90（ゲストのスタック上） |
| cbMultiByte | **256** |
| lpWideCharStr | **0（NULL）** |
| cchWideChar | **0**（個数の問い合わせ） |
| 入力バイト | 256 個。**byte[0] = 0x20、byte[1..255] = 0x01〜0xFF**（`0x00` ではない）。0x80 以上は 128 個（0x80〜0xFF の全部） |

- この呼び出しは DllMain の中で**最初の** `MultiByteToWideChar`（import の 125 回目）。個数の問い合わせ（cchWideChar = 0、出力バッファ NULL）で、runtime は `fle_mb_to_wide` の走査で 0x80 以上のバイトに当たって止まる（読み。`fle_locale.c:496-503`: 停止の判定は `dst` の有無より前にある）。
- stage23 §5.1 段 2 は入力を「バイト 0x00〜0xFF の 256 個」と書いたが、**同じ文書の gdb のログ行は `first-bytes=20 01 02 03`** で、byte[0] は 0x20 だった。今回の実測は byte[0] = 0x20。**なぜ byte[0] が 0x20 かは読んでいない（未確認）**。
- DllMain の `GetACP` から `MultiByteToWideChar` までの呼び出し: `GetACP`（120 回目）→ `HeapAlloc` → `IsValidCodePage(1252)` → `GetCPInfo`（cp 1252）→ `GetCPInfo`（rcx = 0x4e4、r8 = 9。スタック上の別のバッファ）→ `MultiByteToWideChar`（125 回目）。

## 4. DllMain の中の import（実測。`run/ro-imports.txt`。gdb は breakpoint だけ）

`f_37caca8`（DllMain）に入ってから停止までに `fle_imp_*` が **125 回、26 種**。戻りアドレスは**すべてモジュール内**（0x2000000〜0x461f000）で、FLEngine 側の戻りアドレスの呼び出しは 1 つも無い。125 回目（`MultiByteToWideChar`）は戻っていない（停止）。戻った呼び出しは 124 回。

| API | 回数 | | API | 回数 |
|---|---:|---|---|---:|
| InitializeCriticalSectionAndSpinCount | 79 | | GetStdHandle | 3 |
| EnterCriticalSection | 5 | | QueryPerformanceCounter | 2 |
| LeaveCriticalSection | 5 | | GetCPInfo | 2 |
| GetProcAddress | 4 | | FlsGetValue / FlsSetValue | 2 / 2 |
| LoadLibraryExW | 3 | | HeapAlloc | 3 |

各 1 回: GetSystemTimeAsFileTime、GetCurrentThreadId、GetCurrentProcessId、TlsAlloc、TlsSetValue、FlsAlloc、GetProcessHeap、GetLastError、SetLastError、GetStartupInfoW、GetCommandLineA、GetCommandLineW、GetACP、IsValidCodePage、MultiByteToWideChar。

- 順序（抜粋）: クッキー初期化（`GetSystemTimeAsFileTime`、`QueryPerformanceCounter`、`GetCurrentThreadId`、`GetCurrentProcessId`、`QueryPerformanceCounter`）→ `LoadLibraryExW`（`api-ms-win-core-synch-l1-2-0`、flags 0x800）→ `GetProcAddress`（`InitializeCriticalSectionEx`）→ `InitializeCriticalSectionAndSpinCount` → `LoadLibraryExW`（`api-ms-win-core-fibers-l1-1-1`）→ `GetProcAddress`（`FlsAlloc`）→ `TlsAlloc` → `GetProcAddress`（`FlsSetValue`）→ `TlsSetValue` → `LoadLibraryExW`（`api-ms-win-core-synch-l1-2-0`）→ `GetProcAddress`（`InitializeCriticalSectionEx`）→ `InitializeCriticalSectionAndSpinCount` ×14 → `GetProcessHeap` → `FlsAlloc` → `GetLastError` → `FlsGetValue` → `FlsSetValue` ×2 → `HeapAlloc` → 臨界領域 → `SetLastError` → `HeapAlloc` → `InitializeCriticalSectionAndSpinCount` ×64 → `LeaveCriticalSection` → **`GetStartupInfoW`**（110 回目。止まらずに戻った）→ `GetStdHandle` ×3（0xfffffff6、0xfffffff5、0xfffffff4）→ `LeaveCriticalSection` → `GetCommandLineA`、`GetCommandLineW` → `FlsGetValue` → 臨界領域 → `GetACP` → `HeapAlloc` → `IsValidCodePage` → `GetCPInfo` ×2 → **`MultiByteToWideChar`**（125 回目。停止）。
- `GetProcAddress` の 4 回はすべて **NULL**（`GpaDone` で戻り値を確認。`InitializeCriticalSectionEx` ×2、`FlsAlloc`、`FlsSetValue`）。`LoadLibraryExW` の名前は DLL 内 `.rdata`（0x3ec7ff0、0x3ec7fb0）から読んだ。
- 最初の 110 行（`GetStartupInfoW` まで）は、stage23 の `run/seq-0-imports.txt` と、名前、引数、戻りアドレスが**一致**。違うのは 2 行目の `QueryPerformanceCounter` の rcx（ホストのスタックアドレス 0x7fffffffb168 と 0x7fffffffb1f8）だけ。
- `idx0`（種 428 個）の DllMain は 13 回の import で `fle_dispatch_miss` に止まる（§6）。13 回の列は stage23 §3.1 と同じ。

## 5. 仮の値 3 段（測定。「仮の値であって決定ではない」）

**この節の値は決定ではなく、runtime の挙動でもない。runtime には何も入れていない。** 最大 3 段の上限を守った。4 つ目の停止（`WideCharToMultiByte`）は書き換えず、記録だけにした。

方法: gdb が `fle_imp_*` の入口で、**runtime が名前付きで止まる入力のときだけ**値を書き、`return` で `fle_imp_*` を飛ばす。段の関数は `lift/harness/two/patches_chain.py`（stage23 と同じコード。段 2、3、4）をそのまま `exec` して使った。**`GetStartupInfoW` の段（stage23 の段 1）は使っていない**（0083 で runtime が答える）。**1 つの「段」は 1 つの API に対する 1 つの規則**で、同じ規則が同じ API の複数回の呼び出しに作用する（stage23 と同じ数え方）。gdb のログの「(stage 2)」「(stage 3)」「(stage 4)」は patches_chain.py の旧い番号で、この節の段 1、2、3 に当たる。`POST_MAX` = 段の数。

### 5.1 書き換えた値の一覧

| 段 | 対象 | ガード（この入力のときだけ作用） | 書き換えた値（gdb が入れた） | 作用した回数（`POST_MAX` = 3 の run） |
|---|---|---|---|---:|
| 1 | `MultiByteToWideChar` | cp が 1252 または 437（`fle_emulated_locale` の acp、oemcp）で、入力に 0x80 以上のバイトがある | cchWideChar = 0 のとき R[0] = 個数（256）。cchWideChar ≥ 個数のとき、各バイト b を UTF-16 単位 **U+00bb**（恒等。**cp1252 の表ではない**）として出力バッファに書き、R[0] = 256。いずれも `fle_imp_*` を飛ばす | 4 |
| 2 | `GetStringTypeW` | なし（runtime は常に停止する） | 256 単位すべてに種類 **0** を書き、R[0] = 1 | 1 |
| 3 | `LCMapStringW` | 単一の大小文字フラグ（0x100、0x200）で、lcid が既知でない、または 0x80 以上の単位がある | cchDest = 0 のとき R[0] = cchSrc（256）。cchDest = 256 のとき、**ASCII の大文字・小文字だけ変換**（0x41〜0x5a → +0x20。フラグ 0x100）、他の単位はそのまま、R[0] = 256 | 2 |

- 書き換えたのは、ゲストのレジスタ R[0]（戻り値）と、ゲストのスタック上の出力バッファ（MBTWC の出力、GetStringTypeW の種類の配列、LCMapStringW の出力）だけ。DllMain の戻り値、ゲストのコード、runtime のソース、FLEngine 側の呼び出しは書き換えていない。
- これらの値の出典は無い（Learn のページに表も値も無い。stage23 §5.1 の読み。**再確認していない**）。DllMain が戻った後の DLL の `isalpha` 系の挙動は実機と違う可能性が高い（推測）。

### 5.2 各段で次に止まった所（実測。`run/post1〜3-*`）

| `POST_MAX` | 停止メッセージ（`fle:` の後。全文） | 呼び出し元（ゲスト） | 引数（実測） | init_count | 関数（distinct） | 入口 | DllMain 後の import 回数 |
|---:|---|---|---|---:|---:|---:|---:|
| 0（§3） | `MultiByteToWideChar: byte of 0x80 or above in a single-byte code page: table not on the MSDN pages (not defined by the MSDN page or outside the emulated locale; not implemented)` | `f_37d7048` ← `f_37d86c0`（戻り 0x37d8738） | cp = 1252、flags = 0x1、cb = 256、cchWide = 0、バイト 0x20、0x01〜0xFF | 145 | 881 | 15,934 | 125 |
| 1 | `GetStringTypeW: the character classes of the page's C1_*, C2_*, C3_* flags are Unicode data that no MSDN page gives (not defined by the MSDN page or outside the emulated locale; not implemented)` | `f_37d86c0`（戻り 0x37d87ff。IAT 経由の呼び出し） | dwInfoType = 0x1（CT_CTYPE1）、cchSrc = 256、lpSrcStr = 段 1 の出力（先頭 0x0020、0x0001、0x0002、…、0x00ff）、lpCharType = 0x7ffff7bff290（スタック） | 145 | 882 | 15,937 | 127 |
| 2 | `LCMapStringW: locale other than the emulated one (not defined by the MSDN page or outside the emulated locale; not implemented)` | `f_37d75e8`（戻り 0x37d76bf。`call [rip+0x6e5979]`）← `f_37d9a94` ← `f_37d9db4` ← `f_37d674c` | **Locale = 0x0**、dwMapFlags = **0x100**（LCMAP_LOWERCASE）、cchSrc = 256、cchDest = 0（lpDestStr NULL）。src は段 1 の出力（再度の `MultiByteToWideChar` の結果） | 145 | 889 | 15,950 | 133 |
| 3 | `WideCharToMultiByte: UTF-16 unit of 0x80 or above for a single-byte code page: table not on the MSDN pages (not defined by the MSDN page or outside the emulated locale; not implemented)`（`fle_locale.c:601`） | `f_37d70a4`（戻り 0x37d9d31。`call 0x37d70a4`）← `f_37d9a94` ← `f_37d9db4` ← `f_37d674c` | CodePage = **1252**、dwFlags = **0**、cchWideChar = 256、lpMultiByteStr = 0x7ffff7bff090（スタック）、cbMultiByte = **256**、lpDefaultChar = **NULL**、lpUsedDefaultChar = **NULL**。src は段 3 の出力（0x41〜0x5a が 0x61〜0x7a に変わり、0x80 以上は恒等のまま） | 145 | 890 | 15,957 | 135 |

- 段 2 の `f_37d75e8` の第 1 引数（ロケール名）、LCID の出どころは**今回測っていない**。stage23 §5.1 は「`LCMapStringEx` が NULL（`GetProcAddress`）なので `f_37d76d4` → `f_37d9f1c` が名前 NULL に対して `xor eax,eax` で 0 を返す」と読んだ（読み。**再確認していない**）。
- 段 3 の停止後（`WideCharToMultiByte`）は書き換えていない。その先（stage23 では DllMain が `eax = 1` で戻る所まで 2 段先）は今回測っていない。
- **DllMain は 3 段の範囲では戻らなかった**。`LoadLibraryW` は戻っていない。

## 6. 種 428 個だけ（`idx0`。実測）

`out-idx0/a4/20261023T153540Z-native.txt`: `fle: indirect call to 0x37d23b4 is not a known function entry`。スタック先頭は `f_37de9b0`（`call depth at stop: 15`）← `f_37de9d0` ← `f_37d2484` ← `f_37cb484` ← `f_37ca988` ← `f_37caca8`（DllMain）← `f_423990` ← `f_44db40` ← `f_6539e0` ← `f_653f40` ← `f_65c540`。init_count 145 / 942、unit 144 in_progress、functions_distinct **836**、function_entries **15,702**。DllMain 内の import は 13 回で、停止に至る。**stage23 §3.1 と数値、メッセージ、スタックが同一**（0083 の影響なし。`GetStartupInfoW` に届く前の停止）。

## 7. stage23 との差分

| 項目 | stage23（HEAD 87bc74d。0083 の前） | 今回（HEAD 7a9af0f。0083 の後） |
|---|---|---|
| 生成物（FLEngine C、縮小 DLL C、`common.c`、リベース済み DLL、種） | — | stage23 と **SHA-256 一致**（§2）。生成器、リフトの結果は変わっていない |
| 追跡下の変更（`git diff --stat 87bc74d HEAD -- lift/runtime lift/crates lift/harness/wasm lift/harness/a4.c`） | — | `lift/runtime/fle_env.c`（GetStartupInfoW）と `lift/crates/fe-lift/tests/env.rs` の 2 ファイルだけ（0083） |
| `tbl2`、gdb なしの停止 | `GetStartupInfoW: cb is not sizeof(STARTUPINFOW) = 0x68`。867 / 15,909 | **`MultiByteToWideChar: byte of 0x80 or above …`。881 / 15,934** |
| `tbl2` の init_count、最後の unit | 145/942、unit 144 | 同じ |
| `GetStartupInfoW`（DllMain の 110 回目） | 停止（cb = 0x65d0ab08 の残りの値） | **止まらず戻る**（rcx = 0x7ffff7bff930。runtime が cb = 0x68 を書く） |
| 仮の値の最初の段 | `GetStartupInfoW` の cb に 0x68（gdb） | **不要**（0083 の runtime が答える） |
| `MultiByteToWideChar` の停止の数値 | `CHAIN_MAX` = 1: 881 / 15,934 | **881 / 15,934（一致）** |
| `GetStringTypeW` | `CHAIN_MAX` = 2: 882 / 15,937 | `POST_MAX` = 1: 882 / 15,937（一致） |
| `LCMapStringW` | `CHAIN_MAX` = 3: 889 / 15,950 | `POST_MAX` = 2: 889 / 15,950（一致） |
| `WideCharToMultiByte` | `CHAIN_MAX` = 4: 890 / 15,957 | `POST_MAX` = 3: 890 / 15,957（一致） |
| 仮の値の段数（DllMain が戻るまで） | 5 | 4 段（`MultiByteToWideChar`、`GetStringTypeW`、`LCMapStringW`、`WideCharToMultiByte`）で戻ると読める（**今回は 3 段までしか測っていない**。4 段目は stage23 の `CHAIN_MAX` = 5 の行に相当。推測） |
| `idx0`（428 個） | 836 / 15,702、`0x37d23b4` | 同じ |
| `MultiByteToWideChar` の入力バイト | 「0x00〜0xFF」（ログの行は `20 01 02 03`） | byte[0] = 0x20、byte[1..255] = 0x01〜0xFF を実測 |

つまり**変わったのは `GetStartupInfoW` の段が消えたことだけ**で、その後の停止の並び、数値は stage23 と一致する。

## 8. 検証

- 追跡下のファイルは `docs/investigations/two-module-post0083.md`（新規）以外に変更が無い（作業の最後に `git status`、`git diff --stat` で確認。完了報告に出力を載せる）。作業前から未追跡だった `lift/harness/wasm/trace.c.new`、`lift/harness/wasm/trace_native.c.new`、`lift/iat-unsup-closure.txt` は、触っていない（`trace_native.c`（追跡下）を `build_two_native.sh` が使う。`.new` は使っていない）。
- `cargo test --workspace` は実行していない（runtime、crates を編集していないため）。
- 単独構成（FLEngine だけ。realmod 表なし）の a4 は**今回測り直していない**。コミット c68dbce のメッセージは「a4 unchanged 145/942」。stage23 §8 は 145/942、関数 821、入口 15,685。
- wasm は今回使っていない（native のみ）。

## 9. 決定が要る点（決めない）

1. **ロケールの表**（`MultiByteToWideChar`、`GetStringTypeW`、`LCMapStringW`、`WideCharToMultiByte`）。DllMain が戻るには、cp1252 の 0x80〜0xFF の対応、`GetStringTypeW` の文字ごとの種類、`LCMapStringW` の LCID = 0 の扱い、逆方向の変換が要る。stage23 §9-4 と同じで、仕様（Learn のページ）に値は無い（stage23 §5.1 の読み。再確認していない）。値の出典の決定が先。別の調査（`mbtwc-high-bytes.md`）が進行中のため、ここでは扱わない。
2. **個数の問い合わせ（cchWideChar = 0、出力 NULL）の扱い**。今回の停止は DllMain の最初の `MultiByteToWideChar` で、**個数の問い合わせ**（lpWideCharStr = NULL）だった。runtime は問い合わせでも 0x80 以上のバイトで止まる（読み。`fle_locale.c:496-503`）。単一バイトの code page での個数の問い合わせに、表なしで答えてよいかは、ページの記述（stage23 §5.1 の読みは「表は無い」）からは決められない。**決める人が仕様を確認する必要がある**（私は仕様を再確認していない）。
3. **`MultiByteToWideChar` の入力の byte[0] = 0x20**（§3.2）。stage23 §5.1 の「0x00〜0xFF」の記述は正確でなかった。なぜ 0x20 かは未確認。決定ではないが、表を作る調査（1）が知っておく必要がある。
4. **縮小 DLL の種の定義**（stage23 §9-1。変わらない）。今回も `idx0`（428 個）は DllMain の CRT 初期化表の項目（`0x37d23b4` など）に届かず停止し、`tbl2`（452 個）は届く。種を渡す定義は PM の決定待ち。
5. **0083 の保留**（`GetStartupInfoW` の入力 cb を実 kernel32 が読むか）。今回は 0083 のとおり動いて停止しない。一次資料の確認は 0083 の「provisional」のまま。

## 10. 未確認

- DllMain が `eax = 1` で戻る所（4 段目以降）の今回の数値。stage23 では仮の値 5 段で戻った。
- ロケール名が NULL になる理由、LCID = 0 の出どころの再確認（stage23 の読みの再確認）。
- Windows 実機での挙動（ホストは Linux）。
- 単独構成の a4 の今回の値、wasm、DETACH、`FreeLibrary`。
- Learn のページの再取得（今回は行っていない）。

## 11. 作ったファイル

- 本文書 `docs/investigations/two-module-post0083.md`（追跡下。新規）。
- `lift/out-two-post0083/`（gitignore 対象）: `target/`（`fe-pe`、`fe-lift`）、`dll/`（リベース済み DLL、種ファイル 3 つ）、`two/`（`gen/`、`src/`、`native/`、`v-tbl2/`、`v-idx0/`）、`out-tbl2/a4/`、`out-idx0/a4/`（a4-survey のログ）、`run/`（gdb と plain 実行のログ）、`gdb/`（`patches_probe.py`（読み取りだけのプローブ）、`patches_post0083.py`（§5 の仮の値。`POST_MAX` ≤ 3 の上限つき）、`gdb_imports_bt80.py`）、`log/`（生成、ビルド、a4-survey の出力）、`tmp/`（TMPDIR）。
