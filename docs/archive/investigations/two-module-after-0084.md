# 決定 0084 の後の 2 モジュール構成（実 dsp_ippv2_x64.dll）で DllMain(ATTACH) の次の停止

実行日: 2026-10-23（UTC。ログの時刻は `20261023T19…Z`。JST では 2026-10-24）。決定 0084（`docs/decisions/0084-mbtwc-cp1252.md`）の runtime（`lift/runtime/fle_locale.c`）で、`two-module-post0083.md`（以下「post0083」）§2 の手順を**無改変**で再実行した。**この文書は何も決めない**。

作業物は `lift/out-mbtwc-impl/`（gitignore 対象。2 モジュール構成は `lift/out-mbtwc-impl/two/`）。

## 0. 凡例

- **実測** = このセッションでコマンドを実行して得た値。
- **読み** = 逆アセンブル、生成 C、runtime のソースを読んだ結果。実行して確かめていない。
- **仮の値** = 入れていない。この文書の測定は、gdb で値を書き換えていない（読み取りだけの gdb と、gdb なしの実行）。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | 構成 | post0083 §2 の `tbl2`（種 452 個）を、決定 0084 の runtime で再ビルド（native）。作業ディレクトリは `lift/out-mbtwc-impl/two`。種の定義、手順は変えていない | 実測 |
| 2 | 生成物の同一性 | FLEngine の C は SHA-256 `43985d62…37137a1`（期待値と一致）。縮小 DLL の C（`tbl2`）は `6035da1d…279dbf`、リベース済み DLL は `4fd25502…efc5`、種ファイル 3 つ、マージ後 `common.c`（135,452 行。`merged rows 67587 rows present in both 260` はその表の行数）は post0083 の `lift/out-two-post0083/` の同名ファイルと `cmp` で全バイト一致 | 実測 |
| 3 | MultiByteToWideChar | **通った**（125 回目、126 回目の 2 回。cp = 1252、flags = 0x1、cb = 256。125 回目は個数の問い合わせ、126 回目は変換）。停止しない | 実測 |
| 4 | gdb なしの次の停止 | **`GetStringTypeW`**（127 回目の import）。予想どおり | 実測 |
| 5 | a4 の行（`tbl2`） | init_count = **145 / 942**、最後の unit = index 144（run_rank 132/895、proc 0x65c540、in_progress）、functions_distinct = **882**、function_entries = **15,937**、all_units_done = no | 実測 |
| 6 | 停止の一致 | 882 / 15,937 は post0083 §5.2 の「仮の値 1 段（`POST_MAX` = 1）」の行（882 / 15,937）と一致 | 実測 |

## 2. 手順（実測。post0083 §2 と同じ。実行した順）

`lift/` から実行。TMPDIR は `lift/out-mbtwc-impl/two/tmp` など `lift/out-mbtwc-impl/` の下（`/tmp` は使っていない）。

```
CARGO_TARGET_DIR=$PWD/out-mbtwc-impl/single/target cargo build --release --offline -p fe-pe -p fe-lift
# rebase（gen_two.sh の rebase 段）と種 3 ファイル（post0083 §2 のコメントの定義）
OUT=out-mbtwc-impl/two FEBIN=out-mbtwc-impl/single/target/release TAG=tbl2 bash harness/two/gen_two.sh rebase
#   seeds-idx0-2000000.txt        = out-ipp/table-seeds-idx0.txt の各 VA から 0x17E000000 を引いて昇順（428 個）
#   seeds-idx0-plus-initterm.txt  = 上 + mkseeds.py --table 0x3ec8360 0x3ec8460（448 個）
#   seeds-idx0-plus-initterm2.txt = 上 + mkseeds.py --table 0x3ebd270 0x3ebd2a8（452 個）
TAG=tbl2 SEEDS=…/seeds-idx0-plus-initterm2.txt OUT=out-mbtwc-impl/two FEBIN=out-mbtwc-impl/single/target/release bash harness/two/gen_two.sh
TAG=tbl2 OUT=out-mbtwc-impl/two bash harness/two/build_two_native.sh
OUT=out-mbtwc-impl/out-tbl2 NATIVE_BIN=harness/two/two-native-run.sh TWO_NATIVE=…/v-tbl2/native/two-native \
    TWO_REAL=…/dsp_ippv2_x64.2000000.dll FEPE=…/fe-pe bash harness/a4-survey.sh
```

- 種ファイル 3 つ、リベース済み DLL、`gen/flengine.c`、`v-tbl2/gen/real.c`、`v-tbl2/src/common.c` は、`lift/out-two-post0083/` の同名ファイルと `cmp` で一致（読み取りだけで比較）。
- ビルドは新しい作業ディレクトリで、runtime のオブジェクトも含め最初から作った（`build_two_native.sh` は作ったオブジェクトを飛ばす仕様なので、古い runtime のオブジェクトは使っていない）。
- gdb なしの実行は 3 回（`run/plain1〜3.*`）。停止メッセージ、スタック、a4 の行は 3 回とも同一。3 回目は `/usr/bin/time -v` つきで、壁時間 0.06 秒、最大 RSS 96,512 KB、終了コード 134（SIGABRT。ハーネスが停止を報告して終了）。
- 引数を見るために、読み取りだけの gdb プローブ（post0083 の `patches_probe.py` と `gdb_imports_bt80.py` のコピー。値を書かず、`fle_imp_*` を飛ばさない）を 1 回使った（`run/probe0-*`）。同じ停止、同じ数値（882 / 15,937）。

## 3. gdb なしの停止（`tbl2`。実測）

`lift/out-mbtwc-impl/out-tbl2/a4/20261023T190210Z-native.txt`:

```
stop   : fle: GetStringTypeW: the character classes of the page's C1_*, C2_*, C3_* flags are Unicode data that no MSDN page gives (not defined by the MSDN page or outside the emulated locale; not implemented)
         top of stack: 0x37d86c0 f_37d86c0
depth  : init_count=145 / 942   last unit: index=144 run_rank=132/895 proc=0x65c540 in_progress
         started=132 completed=131 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=882  entries=15937
```

停止メッセージ全文（stderr。1 行。3 回とも同一）:

```
fle: GetStringTypeW: the character classes of the page's C1_*, C2_*, C3_* flags are Unicode data that no MSDN page gives (not defined by the MSDN page or outside the emulated locale; not implemented)
```

出どころ: `lift/runtime/fle_locale.c` の `fle_imp_GetStringTypeW`（決定 0078 §3 の名前付き停止。`fle_locale_stop`。gdb の backtrace では行 152）。

### 3.1 呼び出し元（実測）

ハーネスの報告（`call depth at stop: 20`、先頭 10 段）:

```
#0 f_37d86c0  guest-return=0x37d6813
#1 f_37d674c  guest-return=0x37d6d85
#2 f_37d6cd8  guest-return=0x37d6aa1
#3 f_37d6934  guest-return=0x37d6cae
#4 f_37d6c5c  guest-return=0x37d7809
#5 f_37de9b0  guest-return=0x37d7809
#6 f_37de9d0  guest-return=0x37d7809
#7 f_37d2484  guest-return=0x37cb4b8
#8 f_37cb484  guest-return=0x37ca9fa
#9 f_37ca988  guest-return=0x37cabe8
```

gdb の C の backtrace（`run/probe0-imports.txt`）から関数名の連鎖（先頭から）:

```
fle_locale_stop ← fle_imp_GetStringTypeW ← f_37d86c0 ← f_37d674c ← f_37d6cd8 ← f_37d6934 ← f_37d6c5c
← fle_dispatch ← f_37de9b0 ← f_37de9d0 ← f_37d2484 ← f_37cb484 ← f_37ca988 ← f_37caca8（DllMain）
← fle_dispatch ← fle_call_guest ← call_dllmain（fle_realmod.c:117）← fle_realmod_load（fle_realmod.c:138）
← load_by_name（fle_load.c:152）← fle_imp_LoadLibraryW（fle_load.c:184）
← f_423990 ← f_44db40 ← f_6539e0 ← f_653f40 ← f_65c540（unit 144）
← fle_dispatch ← f_4120c0 ← f_412160 ← f_41d8f0 ← f_1376820（FLEngine のエントリ）← main
```

**呼び出し元（ゲスト）は `f_37d86c0`**（読みでは GetStringTypeA 相当のラッパー。`mbtwc-high-bytes.md` §2.3 の #5、#6 と同じ関数）。`f_37d86c0` は 1 つ前の `MultiByteToWideChar`（126 回目。戻りアドレス 0x37d87e1）の直後に、IAT 経由で GetStringTypeW を呼ぶ（戻りアドレス 0x37d87ff。post0083 §5.2 の「GetStringTypeW」の行と同じ）。

### 3.2 停止した呼び出しの引数（実測。gdb の読み取りだけ。`run/probe0-gdb.log`）

| 項目 | 値 |
|---|---|
| import の通し番号 | DllMain に入ってから 127 回目（127 回目で停止。戻った呼び出しは 126 回） |
| dwInfoType | **0x1**（CT_CTYPE1） |
| lpSrcStr | 0x7ffff7bfecc0（ゲストのスタック。直前の `MultiByteToWideChar`（126 回目）の出力） |
| cchSrc | **256** |
| lpCharType | 0x7ffff7bff290（ゲストのスタック） |
| 呼び出しの guest-return | 0x37d87ff |
| lpSrcStr の 256 単位 | 先頭 0x0020、0x0001、0x0002、…、0x007F、0x20AC、0x0081、0x201A、…、0x0178、0x00A0、…、0x00FF。先頭が 0x0020 なのは入力の `byte[0]` が 0x20 だから。**この 256 単位は、`cp1252_expect.txt`（決定 0084 のテストの期待表。zip の MBTABLE から作った）から作った `[0x20] + 表[1..255]` と一致**（`run/probe0-gdb.log` の単位の列と比べた） |

- 先行する 2 回の `MultiByteToWideChar`（125 回目、126 回目）はどちらも `cp = 1252`、`dwFlags = 0x1`、`cbMultiByte = 256`、`src` は 0x20、0x01〜0xFF の 256 バイト（`byte[0] = 0x20`）。125 回目は `lpWideCharStr = NULL`、`cchWideChar = 0`（戻り値 256）、126 回目は `lpWideCharStr` が 0x7ffff7bfecc0、`cchWideChar = 256`（戻り値 256）。戻り値は gdb の出力ログからは読んでおらず、次の import（`GetStringTypeW`）に進んだこと（実測）と、決定 0084 のテスト（256 を返す）からの読み。
- DllMain の import は停止まで **127 回、27 種**（`run/probe0-imports.txt` の集計。post0083 の 125 回、26 種に、`MultiByteToWideChar` の 2 回目と、新しい種類の `GetStringTypeW` の 1 回が加わった。1 回目の `MultiByteToWideChar` は post0083 でも 125 回目）。順序は post0083 §4 と同じで、125 回目の `MultiByteToWideChar` が戻って 126 回目の `MultiByteToWideChar`、127 回目の `GetStringTypeW` が続く（実測。`run/probe0-imports.txt`）。

## 4. post0083 との差分

| 項目 | post0083（HEAD 7a9af0f） | 今回（決定 0084 の runtime） |
|---|---|---|
| gdb なしの停止 | `MultiByteToWideChar: byte of 0x80 or above …`（125 回目）。881 / 15,934 | **`GetStringTypeW: the character classes …`**（127 回目）。**882 / 15,937** |
| `tbl2` の init_count、最後の unit | 145/942、unit 144 | 同じ |
| 生成物（FLEngine C、縮小 DLL C、リベース済み DLL、種） | — | post0083 と SHA-256 一致 |
| DllMain の import 回数（停止まで） | 125 | 127 |
| 停止の呼び出し元 | `f_37d7048` ← `f_37d86c0`（戻り 0x37d8738） | `f_37d86c0`（その呼び出し元 `f_37d674c` への guest-return は 0x37d6813）。GetStringTypeW は 0x37d87ff の戻りアドレスの IAT 経由 |

つまり、**変わったのは `MultiByteToWideChar` が通ったことだけ**。数値（882 / 15,937）は post0083 §5.2 の `POST_MAX` = 1 の行（`MultiByteToWideChar` を gdb の恒等変換で通した行）と一致する。cp1252 の表（恒等変換と違う 0x80〜0x9F）で通っても、停止の数値は変わらない。

## 5. 読み（実測ではない）

- 停止の先は `mbtwc-high-bytes.md` §5.3 の連鎖と同じ: GetStringTypeW（C1 の文字分類。決定 0078 §3 の名前付き停止）→ LCMapStringW（lcid = 0。runtime は 0x400、0x800、0x409、0x7F だけ）→ WideCharToMultiByte(1252)。post0083 §5.2 の仮の値の段と同じ順序。今回は MultiByteToWideChar の段だけを決定 0084 で実装し、残りの 3 つは名前付き停止のまま。**ATTACH は完走しない**（GetStringTypeW で止まる）。
- GetStringTypeW の `lpSrcStr` は、決定 0084 の表で変換した 256 単位（0x80〜0x9F が恒等でない）。次の API（分類）の出典は、調査 §5.3、§8 のとおり確定していない。この文書は決めない。

## 6. 検証（実測）

- FLEngine の生成 C: `fe-lift lift ../binaries/win64/FLEngine_x64.dll lift/out-mbtwc-impl/fle.c --pdata` の SHA-256 `43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1`（期待値と一致。215,707,512 B）。`gen_two.sh` の検査（`out-mbtwc-impl/two/gen/flengine.c`）も同じ値。
- FLEngine 単独構成（realmod 表なし。`build.sh gen split` → `build_native.sh` → `a4-survey.sh`）の a4: `LoadLibraryW: module "dsp_ippv2_x64.dll" is not a virtual module …` で停止、init_count 145/942、functions 821、entries 15,685（変化なし）。
- `cargo test -p fe-lift --no-fail-fast`: 74 テストバイナリ、passed 984、failed 0（完了報告の数値。`lift/out-mbtwc-impl/cargo-test-full.txt`）。

## 7. 未確認

- DllMain が戻る所（GetStringTypeW 以降）の今回の数値。仮の値は入れていない。
- 実機 Windows での挙動（ホストは Linux）。
- 126 回目の `MultiByteToWideChar` の戻り値そのもの（gdb の読み取りでは R[0] を読んでいない）。次の import に進んだことと、テストの 256 から読んだ。

## 8. 作ったファイル

- 本文書 `docs/investigations/two-module-after-0084.md`（追跡下。新規）。
- `lift/out-mbtwc-impl/`（gitignore 対象）: `target/`（cargo test 用）、`single/`（FLEngine 単独構成のビルドと a4、`fe-lift`、`fe-pe`）、`two/`（2 モジュール構成。`dll/`、`gen/`、`src/`、`native/`、`v-tbl2/`）、`out-tbl2/a4/`（a4-survey のログ）、`run/`（plain 実行と gdb プローブのログ）、`gdb/`（プローブのコピー）、`fle.c`（FLEngine の生成 C）、`cargo-test-full.txt`、`tmp/`（TMPDIR）。
