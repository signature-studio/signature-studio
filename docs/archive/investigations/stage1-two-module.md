# 段階 1: 合成 PE による 2 モジュール機構の検証

実行日: 2026-10-23。リポジトリ HEAD ef9f72f（作業開始時に指示された 7719a50 から進んでいた。ワークツリーには W6 などの未コミット変更がある）。決定 0080 と `docs/investigations/two-module-design.md`（以下「設計文書」。§4、§6、§7、§7.1）の続き。**この文書は何も決めない**。決定が要る点は §8 に列挙した。作業物は `lift/out-stage1/`（gitignore 対象の `lift/out*/`。約 45 MB）。

**範囲**: native のみ。合成 PE だけを使い、実 `dsp_ippv2_x64.dll` と FLEngine は使っていない。したがって**この文書の数値はすべて合成 PE の値で、実 DLL の値ではない**。`lift/runtime/` と `lift/crates/*/src/` は編集していない（§7）。

## 0. 凡例

- **実測** = 今回コマンドを実行して得た値（出力は `lift/out-stage1/runs/` と `build-v4/` にある）。
- **読み** = ソースや逆アセンブルを読んだ結果。実行して確かめていない。
- **未確認** = 確認できていない。
- 「試作ランタイム」= `rt-s1/`（`lift/runtime/` の作業開始時のコピーに実在モジュール表を足したもの。`PATCH-PROPOSAL.md`）。「無改変ランタイム」= `rt-base/`（コピーそのまま）。

## 1. 要約

| # | 検証項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | リベース済み DLL（基底 0x2000000）を FLEngine 役と同じバイナリにリンク（R/FL/X・dispatch 表を共有） | リンクでき、実行できた。共有しない場合の重複定義は `ld -r` で 7 個（R、X、FL、fle_dispatch_table、fle_dispatch_count、fle_resp_table、fle_resp_count）で、設計文書 §4.2 の 7 個と同じ名前。`f_<VA>` の衝突 0 | 実測 |
| 1' | `.reloc` の DIR64 が効いていること | リベースした DLL では `.data` の関数ポインタ = export VA（0x2001000）で、`call` が成功。**ImageBase だけを書き換えた対照**ではポインタが 0x180001000 のまま残り、dispatch の失敗で停止 | 実測 |
| 2 | LoadLibraryW が DllMain(hinst=基底, ATTACH=1, NULL) を LoadLibraryW が戻る前に呼ぶ。DllMain が 0 なら NULL | 順序・引数・NULL とも期待どおり（試作ランタイム）。**無改変ランタイムは `LoadLibraryW("mod2.dll")` で stop する** | 実測 |
| 3 | GetProcAddress の戻りを `call [rax]` / `call rax` で呼ぶ | 戻りは `基底 + export RVA`（0x2001000）。メモリ形（`call [rsi]`）、レジスタ形（`call rsi`）ともに呼べた。存在しない名前は NULL と 127 | 実測（試作ランタイム） |
| 4 | fle_call_guest の再入 | DllMain 内から import、同じモジュールの関数、`EnumDisplayMonitors` 経由のコールバック（第 2 モジュール内）を呼べた。第 2 モジュールから FLEngine 役の関数を呼ぶ向きも動いた | 実測 |
| 5 | FreeLibrary の参照カウント（D11 の測定のみ） | §6 | 実測 |

自動検査: `python3 check_runs.py runs/r2` が **34 件すべて PASS**（r3 でも 34、r2 と r3 の全出力が同一）。期待値は仕様（Learn の DllMain、決定 0080 D5）から書いたもので、出力の写しではない。

## 2. 構成と作り方

### 2.1 合成 PE（実測: `build-v4/`）

| | host.dll（FLEngine 役） | mod2（第 2 モジュール役） |
|---|---|---|
| コンパイラ | `x86_64-w64-mingw32-gcc 13-win32`（`-O1 -msse2 -mno-avx -ffreestanding -nostdlib -shared`、`-Wl,-e,DllMain`、CRT なし） | 同じ |
| ImageBase | 0x400000（= `FLE_EXE_HANDLE`） | 0x180000000 で作り、`fe-pe rebase ... 0x2000000` で 0x2000000 |
| SizeOfImage | 0xa000 | 0xa000 |
| import | KERNEL32（6 名: FreeLibrary、GetLastError、GetModuleHandleW、GetProcAddress、LoadLibraryExW、LoadLibraryW） | KERNEL32（7 名）+ USER32（EnumDisplayMonitors） |
| lift（`fe-lift lift --pdata`） | 関数 14、命令 442、bad_nodes 0、unsupported 0 | 関数 15、命令 238、bad_nodes 0、unsupported 0 |
| `.reloc` | DIR64 1 件（`g_tabp = g_tab`。host は基底 0x400000 のままなので適用していない）+ ABSOLUTE 1 件（`objdump -p`） | DIR64 1 件（`.data` の `g_fnptr`）+ ABSOLUTE 1 件 |
| ファイルサイズ | 11,784 B | 11,797 B（リベースの前後で同サイズ。`cmp -l` の差は 4 バイト） |
| 生成 C | 89,172 B | 72,090 B |

- 実 DLL で使う `fe-pe rebase`（決定 0080、段階 0）をそのまま使った（出力: `old_base 0x180000000`、`new_base 0x2000000`、`dir64 1`、`absolute 1`）。
- 状態の共有（設計文書 §4.3 の S）は、`out-design/tools/` の `split_mod.py` と `merge_tables.py` のコピー（`tools/`）で作った。マージ後の dispatch 表は **289 行**（= host 14 + mod2 15 + 応答表の擬似 VA 260。「両方にある行」260 を表明で確認）。ツールは設計文書の試作のままで、今回は変更していない。
- host の関数は export 6 個（シナリオ）+ 非 export の `HostCb`（`.pdata` の seed で閉包に入る）+ DllMain など。mod2 の export は Add、Twice、GetStoredPtr、CallStored、RunCb とデータ 4 個。

### 2.2 import を呼ぶ形（実測。失敗から分かったこと）

最初の版（`src/host.c`、`src/mod2.c`。`build-v1`）は、`HostBasic` が `LoadLibraryW` の後で `GetProcAddress` を `call rsi`（IAT の中身を callee-saved レジスタに持った形）で呼んだため、`fle_dispatch(0x408078)` で止まった（`indirect call to 0x408078 is not a known function entry`）。0x408078 は host の `GetProcAddress` の IAT スロット（`objdump -p` の import 表で確認）で、ハーネスが IAT に入れる「自分のスロット VA」。**読み**: fe-decode が `call rsi` を import に解決できず、`fle_dispatch` に回ったと読める（`call rsi` は逆アセンブルで確認。fe-decode 側の解決条件は読んでいない）。対策として、各 import を `noinline` の 1 命令ラッパー（`src/shared.h` の `W_*`）に入れ、`call [slot]` だけになる形にした（`src/guest_host.c`、`src/guest_mod2.c`）。**実 DLL のコンパイラ出力ではこの形になるとは限らない**（未確認。実 DLL の `call_imp` と `call_ind` は決定 0077 と設計文書 §4.1 に件数がある）。

### 2.3 試作ランタイム（`PATCH-PROPOSAL.md`、`patch-rt-s1.diff`）

実在モジュール表（最大 4）を `fle_load.c`、`fle_modules.c` から引く。DllMain の呼び出しは `fle_call_guest(entry, base, 1, 0, 0)`。決定ではなく、D6 と D11 は決めていない。

## 3. 検証 1: リベースとリンク（実測）

コマンド（すべて `lift/out-stage1/` で）:

```
./build.sh                        # VARIANT=build-v4 で実行。合成 PE、lift、マージ、ビルド
ld -r dupcheck/host.o dupcheck/mod2.o -o dupcheck/both.o    # 共有しない場合の重複
```

- 重複定義（`ld -r`、`dupcheck/dup-names.txt`）: `FL`、`fle_dispatch_count`、`fle_dispatch_table`、`fle_resp_count`、`fle_resp_table`、`R`、`X` の 7 個。設計文書 §4.2 と一致。weak として統合されるもの（`MM`、`YH`、`fle_teb`、`fle_mxcsr`、`fle_x87_mmx_active`、`fle_teb_set_stack`）も同じ。`f_<VA>` の共通 `T` シンボル 0。
- 共有版（`common.c` が R/FL/X と表を 1 つ定義、各 part は extern）はリンクでき、`driver-s1` と `driver-base` が動く。`nm` で R、X、FL、fle_dispatch_table、fle_resp_table が `common.o` だけにあることを確認した。
- 同じ `image.c`（無改変）を 2 回呼んで 2 つのイメージを置けた: `host base=0x400000 size=0xa000 entry=0x401089; mod2 base=0x2000000 size=0xa000 entry=0x200116a`。
- **DIR64 の効果（対照つき）**:

  | 条件 | `GetStoredPtr()`（`.data` のポインタ） | `== GetProcAddress("Add")` | `CallStored(5,6)` |
  |---|---|---|---|
  | `fe-pe rebase`（DIR64 を適用） | 0x2001000 | 1 | 11 |
  | ImageBase だけ書き換え（`tools/patch_base_only.py`。DIR64 なし） | **0x180001000** | 0 | 呼ぶ前に `fle: indirect call to 0x180001000 is not a known function entry`（SIGABRT） |

  → 設計文書 §3.4(A) の「リベース後のメモリ内ポインタと dispatch 表の VA が一致する」を、DIR64 1 件の合成 PE で確認した。

## 4. 検証 2〜4: LoadLibraryW、GetProcAddress、再入（実測。試作ランタイム）

`runs/r2/s1-basic.out` の順序（`log` はハーネスが読み出した共有ログ。タグ 4 文字）:

```
HBEF   host: LoadLibraryW の直前
DENT 1          DllMain 入口 reason=1
DHIN 0x2000000  hinstDLL = 基底
DRES 0          lpvReserved = NULL
DADD 0x2a       DllMain から同じモジュールの Add(20,22) = 42
DLE  0x1111     DllMain から SetLastError/GetLastError
DPEQ 1          DllMain から GetProcAddress(kernel32,"GetCurrentProcessId") を呼んで比較
DECB 0xabcd     DllMain から EnumDisplayMonitors → コールバックが第 2 モジュール内で動く (dwData)
DEC1 0x7fea0004 hMonitor (FLE_MONITOR_HANDLE)   DEC2 0 (hdc NULL)   DEC3 0x780 (right = 1920)
DEMR 1          EnumDisplayMonitors の戻り
HAFT 0x2000000  LoadLibraryW が host に戻り、ハンドル = 基底
HMIS 0 / HMER 0x7f   存在しない export: NULL、last error 127
HPAD 0x2001000  GetProcAddress("Add")
HADD 0x2a       call [rsi] (メモリ形) で Add(40,2)
HTWI 0x2a       call rsi (レジスタ形) で Twice(21)
HSTO 0x2001000 / HEQ 1   GetStoredPtr() == Add
HCST 0xb        CallStored(5,6)
HCB 7 / HRCB 0xf   host の HostCb を第 2 モジュールが呼ぶ (RunCb(HostCb,7) = 7*2+1)
```

- **順序**: DllMain の全ログ（DENT〜DEMR）が、host の `HAFT`（LoadLibraryW の戻りの後）より前にある（`check_runs.py` で検査）。
- **DllMain の引数**: R[1]（hinstDLL）= 0x2000000、R[2] = 1、R[8]（lpvReserved）= 0。Learn の DllMain の記述（「base address」「NULL for dynamic loads」）と合う。
- **fle_call_guest の深さ**: ドライバがシナリオを `fle_call_guest` で呼ぶので深さ 1。LoadLibraryW が DllMain を呼ぶとき `fle_callback_depth()` = 1 で、DllMain は深さ 2、EnumDisplayMonitors のコールバックは深さ 3（`FLE_CB_MAX_DEPTH` 16 の内側）。測ったのは「DllMain を呼ぶ直前の深さ」（`max callback depth at a DllMain call: 1`）だけで、コールバックの深さ 3 は**読み**（`fle_monitor.c` が `fle_call_guest` を呼ぶ）。R[4]（rsp）は各シナリオの終了後に入口と同じ（`rsp_kept=1`）。
- **無改変ランタイム**: `LoadLibraryW("mod2.dll")` で `fle: LoadLibraryW: module "mod2.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; ...)`（SIGABRT）。何も作らないで止まる。

### 4.1 DllMain が 0 を返す場合（実測）

`runs/r2/s1-fail-always.out`（DllMain は毎回 `SetLastError(0x5678)` して 0）:

- 1 回目の `LoadLibraryW` は 0。直後の `GetLastError()` = 0x5678（DllMain が設定した値のまま。**試作は last error に触っていない**。Learn は LoadLibrary の失敗時の last error を明示していない）。
- DllMain の呼び出しは `reason 1`、`reason 0`（lpvReserved NULL）の順で、DETACH は `LoadLibraryW` が戻る前（Learn: 「システムは直ちに PROCESS_DETACH で呼ぶ」と合う。試作の `S1_DETACH_ON_FAIL=1`。**DETACH を呼ばない形は測っていない**）。
- 2 回目の `LoadLibraryW` も ATTACH から再実行され、0（ログの reason 列: 1, 0, 1, 0）。
- `runs/r2/s1-fail-first.out`（DllMain が最初の ATTACH だけ 0）: 1 回目 0、2 回目 0x2000000。reason 列は 1, 0, 1（2 回目の成功の後に DETACH なし）。

### 4.2 DllMain の中から LoadLibraryW（実測）

`runs/r2/s1-basic-selfload.out`（mode 2）: DllMain が `LoadLibraryW("mod2.dll")`（自分自身）を呼ぶと基底 0x2000000 が返り、DllMain への 2 回目の入口はない（reason 列は 1 のみ）。`LoadLibraryW("kernel32.dll")` はトークン 0x7ff800090000、`FreeLibrary` は 1。その後のシナリオは変わらない。Learn は DllMain から LoadLibrary を呼ぶことを禁じており（設計文書 §5.4）、これは機構が壊れないかの測定で、推奨ではない。「初期化中の同名ロードで DllMain を再度呼ばない」扱いは作業者の選択で、Learn に明記を**見つけていない**。

## 5. 検証 3 の補足: `GetProcAddress` が返す値（読み）

試作は export の実アドレス（基底 + RVA）を返し、`fle_dispatch` の共有表にあるので呼べた（実測）。擬似 VA（0x7FF9…）か実アドレスかをゲストが区別しないという設計文書 §5.2 の読みは、合成 PE の呼び出し側（`call [rax]`、`call rax`）が区別しなかったという範囲で**支持された**。ゲストが戻りを範囲検査や比較に使う場合の差は、合成 PE では試していない（**未確認**。比較は同じ実アドレス同士の `==` だけ）。

## 6. 検証 5: FreeLibrary の参照カウント（D11 は未決。現状の挙動の測定のみ）

**仕様は決めない**。無改変ランタイムの挙動と、試作の 2 つのスイッチ（`S1_UNLOAD_AT_ZERO` 0 / 1）の測定結果を並べる。

### 6.1 無改変ランタイム（`runs/r2/base-*.out`）

| 操作 | 結果 | 区分 |
|---|---|---|
| `LoadLibraryW("kernel32.dll")` | トークン 0x7ff800090000（仮想モジュール。参照 1 → 2） | 実測 |
| 1 回目の `FreeLibrary(その値)` | 1（参照 2 → 1） | 実測 |
| 2 回目の `FreeLibrary(その値)` | stop: `the reference count would reach zero, which would unload a module the process imports (not modelled)` | 実測 |
| `FreeLibrary(0x2000000)`（誰も LoadLibrary していない値） | stop: `hLibModule is not a virtual module handle (the page names no error code for it)` | 実測 |

### 6.2 試作・既定（`S1_UNLOAD_AT_ZERO=0`。`runs/r2/s1-refs.out`）

`HostRefs`: `GetModuleHandleW("mod2.dll")`（未ロード）→ 0、last error 126。`LoadLibraryW("mod2.dll")`（ATTACH 1 回、参照 1）、`LoadLibraryW("MOD2")`（参照 2。名前の正規化は既存の規則）、`LoadLibraryExW("mod2.dll", NULL, 0x800)`（参照 3）は、すべて 0x2000000。`GetModuleHandleW("mod2.dll")` = 0x2000000。`FreeLibrary` 1 回目、2 回目は 1（参照 3 → 2 → 1）。`GetProcAddress` は動く。**3 回目の `FreeLibrary`（参照 1 → 0）は stop**: `the reference count would reach zero: the unload of a real module is not modelled (D11)`（試作の書いた文言で、既存の仮想モジュールの stop と同じ振る舞い）。

### 6.3 試作・`S1_UNLOAD_AT_ZERO=1`（`build-d11/`、`runs/d11-HostRefs-0.out`）

同じ列で 3 回目の `FreeLibrary` が 1 を返し、その前に `DllMain(reason 0, NULL)` が実行される（ログ `DENT 0`、`DRS0 0`）。続く `GetModuleHandleW("mod2.dll")` は 0、次の `LoadLibraryW` は ATTACH を再実行して 0x2000000（ログ reason 列: 1, 0, 1）。**イメージのメモリは再初期化しない**ので、静的データは前回の値のまま（読み。再ロード後に `g_attach_count` が 1 に戻るかは測っていない。実機は再マップするので戻ると読めるが未確認）。

### 6.4 参照カウントと DllMain 内のロード（実測。注意）

`HostRefs` を mode 2（DllMain が自分自身を `LoadLibraryW`）で実行すると（`runs/r2/s1-refs-selfload.out`）、**既定のスイッチでも 3 回目の `FreeLibrary` が stop せず**、`GetModuleHandleW` は 0x2000000 のまま、最後の `LoadLibraryW` も 0x2000000 で終わる（exit 0）。DllMain 内の自己ロードが参照を 1 つ足し、ATTACH 完了時の参照が 2 になるため（読み。試作の「初期化中は参照 +1」の帰結）。実機の Windows が DllMain 内の同名ロードをどう数えるかは**未確認**。

### 6.5 その他（実測）

- `FreeLibrary(0x2000000)`（未ロードの実在モジュール）: 試作は stop（`FreeLibrary of a real module that is not loaded (not modelled)`）。`S1_UNLOAD_AT_ZERO=1` でも同じ（`runs/d11-HostFreeBase-0.out`）。
- 仮想モジュールの測定（`HostRefsVirtual`）は試作でも 6.1 と同じ。

## 7. 変更していないものと作業物

- **`lift/runtime/`、`lift/crates/*/src/`、`lift/crates/*/tests/`、`lift/harness/` は編集していない**。作業の終わりに `lift/runtime/` の `*.c`、`*.h` 103 ファイルのハッシュを作業開始時（`out-stage1/rt-base-sha256.txt`）と比べ、**差 0**（2 回確認）。`git status` に出る変更（W6 の `imports_table.rs`、runtime の 15 ファイル、`k32_dsp.rs` ほか）は作業開始時に既にあったもので、今回のものではない。`lift/runtime/` 以外（`crates/*/src/`、`harness/`）は作業開始時のハッシュを取っておらず、`git status` の ` M`/`??` の一覧が作業開始時（最初の `git status`）と同じであることだけを確認した（`harness/` の `.new` ファイルも開始時からあった）。
- `harness/image.c`、`harness/boot.h` は無改変のまま、ビルド時に直接コンパイルした。
- `cargo` は `CARGO_TARGET_DIR=lift/out-stage1/target` で `fe-pe`、`fe-lift` を release ビルドしただけ（`--locked --offline`）。**`cargo test` は実行していない**。a4（`harness/a4-survey.sh`）も実行していない（FLEngine を使わないため）。

### 7.1 作ったファイル（すべて `lift/out-stage1/` と本文書）

- 本文書 `docs/investigations/stage1-two-module.md`。
- `PATCH-PROPOSAL.md`、`patch-rt-s1.diff`（`rt-base` と `rt-s1` の差。88 行）。
- `src/`: `shared.h`、`guest_host.c`、`guest_mod2.c`（現行）、`driver.c`、`host.c`、`mod2.c`（最初の版。`call rsi` の失敗を起こした。`rm` 禁止で残した。使っていない）。
- `rt-base/`（`lift/runtime` のコピー）、`rt-s1/`（試作。差は `fle_load.c`、`fle_modules.c`、新規 `fle_realmod.c`、`fle_realmod.h`）、`rt-base-sha256.txt`、`snapshot-sha256.txt`。
- `build.sh`、`run_all.sh`、`check_runs.py`、`tools/`（`split_mod.py`、`merge_tables.py` は `out-design/tools/` のコピー、`patch_base_only.py`）。
- `build-v1`（失敗した最初の版）、`build-v2`、`build-v3`（途中版）、`build-v4`（現行。`runs/r2`、`r3` はこれ）、`build-d11`（`S1_UNLOAD_AT_ZERO=1`）、`build-neg`（ImageBase だけ書き換えた対照）、`dupcheck/`、`runs/`（`r1` は `build-v3`、`r2`、`r3` は `build-v4`、`check-r2b.txt` が最終の検査出力）、`target/`、`tmp/`。`build/` は最初の試し（生成 C の確認）。

## 8. 決定が必要な点（決めない）

1. **D11**: 参照 0 の `FreeLibrary` の扱い（stop / DETACH を呼んで未ロードに戻す / ロード済みのまま）。§6 のとおり、無改変ランタイムの仮想モジュールは stop。実在モジュールを仮想モジュールと同じ扱いにするかが論点。DETACH して未ロードに戻す場合、イメージのメモリを再初期化するか（§6.3）も決定が要る。
2. DllMain が FALSE のとき `LoadLibraryW` が last error をどうするか（試作は DllMain の設定値のまま）。DETACH を呼ぶか（試作は呼ぶ。Learn の記述に沿う）。
3. DllMain 内の同名 `LoadLibraryW`（初期化中）の参照の数え方（§4.2、§6.4）。実 DLL の DllMain がこれを呼ぶかは未確認。
4. 実在モジュール表と `fle_load.c` の `refs[]`（仮想モジュール、`LOAD_MODULES_MAX` 64）を統合するか。
5. `GetModuleHandleW(NULL)` は `FLE_EXE_HANDLE`（0x400000）のまま、`FreeLibrary(FLE_EXE_HANDLE)` は stop のまま、という前提の維持（基底 0x400000 の実在モジュールは作れない）。
6. D6（`GetModuleFileNameW` の DLL の仮想パス）は今回扱っていない。
7. 試作の差分を `lift/runtime/` に入れる時期と方法（W6 と `fle_modules.c`、`fle_modules.h` が重なる）。

## 9. 未確認

- **wasm**: 合成 PE の 2 モジュール構成を wasm でビルド・実行していない（設計文書 §4.6 の縮小 DLL の実測はあるが、今回の機構の wasm 版は未実施）。
- 実 DLL（縮小含む）、FLEngine と組み合わせた a4 の停止点、`cargo test --workspace`。
- 実 DLL のコンパイラ出力で `GetProcAddress`、`LoadLibraryW` などが `call [slot]` に解決される割合（§2.2。合成は 1 命令ラッパーに揃えた）。
- 合成 PE の `call` 先が dispatch 表の外（ゲストが戻りアドレスを範囲検査する形）の挙動。
- `GetProcAddress` の序数指定、フォワーダ（試作は stop）。
- `S1_DETACH_ON_FAIL=0`、複数の実在モジュール（`RM_MAX` 4）、基底 0x400000 との衝突、FLEngine の実イメージとの重なり（FLEngine は 0x400000〜0x1cc0000 を使う。今回の host は 0x400000〜0x40a000）。
- コンパイラは mingw gcc 13（`-O1`）の出力 1 種類だけ。clang など他のコンパイラの出力、SSE の命令を含む関数は試していない（合成 PE に SSE なし）。
- 機構が正しいことの保証範囲は設計文書 §7.1 の限界のとおり（CRT、`__chkstk`、`api-ms-win-*`、AVX、locals 上限は検出できない）。
