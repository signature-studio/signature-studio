# システム情報の応答（GetNativeSystemInfo ほか）の実装と all-pdata harness の再実行

日付: 2026-10-20。決定は `docs/decisions/0034-sysinfo.md`。出力はすべて `lift/out-sysinfo/`（`tmp/` は TMPDIR）。前回の記録は `docs/investigations/pdata-seeds-run.md`（停止点 0x41cfa0、`RaiseException(0xc06d007f)`）。

「実測」はツールの出力、「推測」は未検証の解釈。

## 1. 閉包の IAT にある近縁 API の一覧（実測）

調べ方: `lift/out-imports/imports.txt`（DLL の import 表、normal 738 + delay 57）から名前を検索。`name_xrefs.py` と `grep -a` で DLL 内の文字列を検索（ASCII と UTF-16）。`iat_unsup.py` で、生成 C の `fle_unsupported` になっている `jmp [IAT]` を import ごとに数えた。`k32-missing.txt` は kernel32 の import 194 件のうち TABLE にない 72 件。

| 名前 | IAT（normal / delay） | 実装前の状態 | 今回 |
|---|---|---|---|
| GetSystemInfo | normal 0x19715e8 | 実装済み（`fle_sys.c`、0017） | 重複させず。変更なし |
| GetNativeSystemInfo | delay 0x1976390 | 未対応（GetProcAddress で NULL） | 実装 |
| GetLogicalProcessorInformation | delay 0x19762d0 | 未対応 | 実装 |
| IsWow64Process | delay 0x1976358 | 未対応 | 実装 |
| GlobalMemoryStatus | normal 0x19717d8 | 未対応（`jmp [IAT]` が `fle_unsupported`、1 件） | 実装 |
| GetProcessAffinityMask | normal 0x1971638 | 未対応（同、1 件） | 実装 |
| SetProcessAffinityMask | normal 0x1971680 | 未対応（同、1 件） | 実装 |
| GetComputerNameW / GetComputerNameExW | normal 0x1971790 / 0x1971440 | 未対応（同、各 1 件） | 未実装（依頼の列挙外。決定 E） |
| GetLogicalProcessorInformationEx、GetNumaHighestNodeNumber、IsProcessorFeaturePresent、GlobalMemoryStatusEx、GetPhysicallyInstalledSystemMemory、IsWow64Process2、GetActiveProcessorCount、GetMaximumProcessorCount | import 表になし。DLL 内に ASCII、UTF-16 とも文字列なし（`grep -a -c`、`name_xrefs.py`） | — | 未実装（実在しない） |
| GetSystemTimes | import 表になし。文字列が DLL 内に 1 つある（オフセット 0xdacbc、`GetCPUUsage`、`PrevSystemTimes` の近く） | — | 未実装（使われ方は未確認） |

- `GetCPUUsage` の周辺は、文字列が 0xdacbc（ファイル先頭からのバイト位置）にあり、直前のバイトが長さではなく名前テーブルの形（`\x0eGetSystemTimes`）に見える。何のテーブルかは見ていない（推測）。
- 実装前の `GetSystemInfo` の thunk は 2 つ（0x407230、0x423700）。

呼び出し元（`callers.py` で `call rel32` を走査）: GetNativeSystemInfo の thunk 0x4236f0 は 2 か所（0x45413e、0x115d144）、GetLogicalProcessorInformation の 0x407320 は 4 か所（0x409a07、0x409a42、0x41052d、0x410555）、IsWow64Process の 0x425a80 は 1 か所（0x84333b）、GlobalMemoryStatus の 0x4238d0 は 1 か所（0x12622b3）、GetProcessAffinityMask の 0x423690 は 3 か所、SetProcessAffinityMask の 0x423bf0 は 9 か所。

## 2. 実装したもの（6 API）

- `lift/runtime/fle_sysinfo.h`、`fle_sysinfo.c`（新規。`lift/out-sysinfo/work/` で `clang -Wall -Wextra -Werror` を通してから、ヘッダ、`.c` の順に `lift/runtime/` へ 1 回で置いた。`cmp` で一致を確認）。
- `lift/crates/fe-lift/src/imports_table.rs`: TABLE の末尾に 6 行を追記（コメント 2 行つき）。167 行 → 173 行（`fle_resp_count = 173`、生成 C で確認）。
- `lift/crates/fe-lift/tests/sysinfo.rs`（新規、16 件）。
- `lift/crates/fe-lift/tests/modules.rs`: 1 か所の局所変更。`generated_unit_with_tables_compiles_and_links_with_runtime` が「応答表にない名前」の例として `GetLogicalProcessorInformation` を使っていた（今回、表に入ったため NULL でなくなった）。名前を `GetLogicalProcessorInformationEx`（表にも DLL にもない）に変え、期待値は変えていない。テストの意図（表にない名前は NULL と 127）を保つ変更。
- `fle_modules.c` は編集していない（API 名を持たず、応答表は生成器が TABLE から出す。0015 §5）。

## 3. テスト（実測）

- 新規 `sysinfo.rs`: **16 件 passed**。構成: 行がテーブルとヘッダにある、thunk と応答表の行（合成 PE）、実 DLL と all-pdata seed での行（200 MB の C を保持せず走査）、GetNativeSystemInfo が GetSystemInfo と同じバイト列、設定値、GetLogicalProcessorInformation のバッファ規約（サイズ問い合わせ、足りない、ぴったり、余る）と 7 レコードの内容とキャッシュの値、IsWow64Process、GlobalMemoryStatus の整合と設定値、アフィニティの規則（87）、未定義入力 12 件の abort、別スレッドの abort、GetProcAddress が 6 行を見つけて引き続き表にない名前（大文字小文字違い、別モジュール、GetNumaHighestNodeNumber、IsProcessorFeaturePresent、GlobalMemoryStatusEx、GetLogicalProcessorInformationEx）を NULL と 127 にする、ソースがホストの状態を読まない。
- 応答表の行数を固定値でテストしていない（下限だけ）。
- ワークスペース全体（`cargo test --offline --workspace --no-fail-fast`、`out-sysinfo/test-ws2.txt`、`test-ws2.exit` は EXIT=0）: **56 バイナリ、518 件 passed、0 failed、0 ignored**。うち私の分は `sysinfo.rs` 16 件。
- 1 回目（`test-ws.txt`、EXIT=101）: 517 passed、**1 failed**（`null0.rs` の `runtime_answers`、`run failed: Os { code: 26, kind: ExecutableFileBusy, message: "Text file busy" }`）。原因は、同じ生成物のバイナリ（`out-modules/test`〔推測: `null0.rs` の `dirs()`〕の実行ファイル）を作って直後に実行するときの競合で、テストの中身の失敗ではない（失敗の文面から。競合の相手は特定していない）。`cargo test -p fe-lift --test null0` の単独再実行で 6 件すべて通過（`test-null0-rerun.txt`）、ワークスペース全体の 2 回目で 0 failed。集計には 2 回目を使った。
- 途中の `modules.rs` の失敗（§2）: `generated_unit_with_tables_compiles_and_links_with_runtime`。応答表に `GetLogicalProcessorInformation` を入れた結果 `miss 140707423586960`（擬似 VA）が返り、期待の `miss 0` と `le 127` に合わなくなった。テスト側の例の名前を変えて通過（`test-modules-2.txt`、12 件）。
- 同じテスト名の実 DLL の検査（`real_dll_handles_...`）は、DLL が無いと黙ってスキップする作りなので、今回 DLL がある環境で実行された（スキップではない）。

## 4. 再生成と再ビルド（all-pdata、実測）

fe-lift を `out-sysinfo/target`（`CARGO_TARGET_DIR`）にビルドして使った（workspace 外の拒否はなかった）。

| 項目 | 実装前（`base`） | 実装後（`after`） |
|---|---|---|
| 関数 | 58,695 | 58,695 |
| 閉包の命令 | 2,523,194 | 2,523,194 |
| `bad_nodes` | 0 | 0 |
| `c_bytes` | 215,654,688 | 215,656,107 |
| `unsupported_total` | **2,944**（48 種類） | **2,941**（48 種類） |
| `Jmp: indirect through IAT slot (not lifted)` | 591 | 588 |
| `Jmp: indirect target unresolved` | 62 | 62 |
| `fle_resp_count` / 表の行数 | 167 | 173 |
| 生成の所要、最大 RSS | 2.64 秒、320 MB | 2.77 秒、320 MB |

- `unsupported_total` -3 は、GlobalMemoryStatus、GetProcessAffinityMask、SetProcessAffinityMask の 3 つの `jmp [IAT]` が処理済み（`jmp_imp:*` 各 1）に変わったもの。他の 3 つ（delay 版）は、IAT thunk ではなく GetProcAddress 経由なので `unsupported_total` に影響しない（実測: `jmp_imp:GetNativeSystemInfo` などは出力に現れない）。
- 前回の記録（pdata-seeds-run.md §5.3）の 2,991 との差（-47）は、その後のワークツリーの更新による（他の担当の変更。私は未確認）。この作業の `base`（実装前の同じツリー）との差だけが私の変更。
- 分割（`out-pdata/split_c.py` 3,700 関数、16 個）、ビルド（`out-sysinfo/build-parts.sh`、`out-pdata/build-parts.sh` の出力先を `out-sysinfo/` に変えた複製、`EXTRA_RT` は生成 C が参照する `fle_imp_*` 173 個の定義元 21 ファイル）: **44.0 秒、最大 RSS 1,203 MB、EXIT=0**。2 GB 級の中間ファイルなし（最大の C は 216 MB、`out-sysinfo/` 全体は `target` を含めて 3.8 GB）。生成 C が参照する `fle_imp_*` は 173 個すべてが runtime のどれかに定義されていることを確認してからビルドした（`UNDEFINED` の出力なし）。

## 5. 実行（実測）

コマンド: `./bin/flengine-after ../../binaries/win64/FLEngine_x64.dll`（`out-sysinfo/` で）。

### 5.1 実装前（`flengine-base`、同じツリーの実装前）

`run-base.txt`: `fle: RaiseException: exception code 0xc06d007f ...`、**停止点 0x41cfa0**、関数実行数（distinct）140、関数入口 708、深さ 9。前回の記録と同じ。

### 5.2 実装後（`flengine-after`、EXIT=134）

- **停止点 0x41cfa0 を通過せず、手前の別の場所で止まるようになった: `fle: unsupported instruction at 0x4072ff`、`abort kind: fle_unsupported`**。`f_407320` の中。関数実行数（distinct）**38**、関数入口 **55**、深さ 8、実行 0.02 秒。
- スタック: `#0 0x407320 f_407320`、`#1 0x4099c0`、`#2 0x409bf0`、`#3 0x41ce10`、`#4 0x4120c0`、`#5 0x412160`、`#6 0x41d8f0`、`#7 0x1376820`（エントリ）。
- 数字だけ見ると前回より後退している（140 → 38 関数、708 → 55 入口）。これは、応答表に `GetLogicalProcessorInformation` を入れたことで、ゲストが以前とは違う経路に進んだため。実装前は `GetProcAddress("GetLogicalProcessorInformation")` が NULL で、`f_4099c0` は 0 を返して先へ進んでいた（`modules-run.md` の既存の記録、`import-GetProcAddress.md`）。今は非 NULL が返り、`f_4099c0`（0x4099f8 の `test rax,rax` の後）が遅延ロードの thunk `0x407320` を呼ぶ。

### 5.3 原因（実測）

- gdb（`gdb-stop2.cmds`、`gdb-stop2.txt`）: `GetProcAddress hmod=0x7ff800090000 name=GetLogicalProcessorInformation` が 2 回（`LoadLibraryA name=kernel32.dll` の前後）。2 回目の戻りは擬似 VA 0x7ff900000a90（応答表の行。`fle_resp_table` の `GetLogicalProcessorInformation` と一致）。abort 時の R[0] もこの値。
- 0x407320 は `jmp [0x19762d0]`（delay の IAT スロット）。生成 C は `f_407320` の先頭が `goto L_407310` で、`L_407310: push rdx; lea rdx,[0x19762d0]; jmp 0x4072b0` の遅延ロードのヘルパ（0x4072b0）に入る。ヘルパは 0x4072d8 で `call 0x41cfc0`（f_41d9a0 を呼ぶ）を呼び、戻り値の rax へ **0x4072ff の `jmp rax`**（`ff e0`、レジスタ間接 jmp）で飛ぶ。
- `jmp rax` は fe-decode が解決できない（`Jmp: indirect target unresolved`、62 件の一種）。生成 C は `fle_unsupported(0x4072ff)`。`fle_dispatch` で解決する経路は call 用（`dispatch::call`）にだけあり、`jmp` には無い。
- 同じ種類の `jmp rax` は、今の all-pdata の生成 C に **18 か所**（0x4071bf、0x4072ff、0x42304f、0x42316f、0x423e6f、0x42474f、0x42572f、0x43916f、0x4692af、0x46932f、0x5933cf、0x59396f、0x5dc61f、0x5dcacf、0x5e103f、0x5f1499、0x741fdf、0x80f9ef）。遅延ロードのヘルパのどれかの末尾と読めるが、1 つずつは見ていない（推測）。`jmp` を `Jmp` の fle_unsupported のままにしているサイトは全体で 610 か所（IAT の `jmp [rip+d]` 588、`jmp rax` 18、`jmp rcx` 2、他 2。実測、`iat_unsup.py` と上の走査）。

### 5.4 反事実（GetNativeSystemInfo の行だけで先へ進むか。実測、診断用）

`GetProcAddress` の `GetLogicalProcessorInformation` の引数だけを gdb で書き換えて NULL を返させた実行（`gdb-nullglpi-run.cmds`、`run-nullglpi.txt`、ゲストの実装は変えていない。診断だけで、成果物ではない）:

- 停止点: **0x42316f**（`f_4236f0` 内、`GetNativeSystemInfo` の遅延ロードのヘルパ 0x423120 の末尾の `jmp rax`）、`fle: unsupported instruction at 0x42316f`。関数実行数 **138**、入口 **706**、深さ 6。
- スタック: `#0 0x4236f0`、`#1 0x454090`（呼び出し元 0x454143。0x45413e の call）、`#2 0x4120c0`、...。
- 読み: `GetProcAddress(kernel32, "GetNativeSystemInfo")` は今度は非 NULL（0x7ff900000a80）を返し、`RaiseException(0xc06d007f)`（0x41cfa0）には行かなかった。0x41cfa0 は通過している。ただし同じ遅延ロードのヘルパの `jmp rax` で止まる。

### 5.5 結論（事実のみ）

- 停止点: **0x41cfa0（RaiseException）→ 0x4072ff（`jmp rax`、遅延ロードのヘルパ 0x4072b0 の末尾）**。`GetNativeSystemInfo` の NULL が原因で出ていた `RaiseException(0xc06d007f)` は、この実装で出なくなった（反事実の実行 §5.4 でも、`GetNativeSystemInfo` が非 NULL なら 0x41cfa0 を通らない）。
- 次の停止点の原因: **命令名は `jmp rax`（レジスタ間接 jmp、VA 0x4072ff。同種は 0x42316f ほか全 18 か所）**。API 名ではない。遅延ロードのヘルパ（IAT スロットの解決後に、得たアドレスへ飛ぶ）の末尾。範囲外（生成器の `jmp` の間接解決は、このタスクの範囲外）のため実装していない。候補（事実のみ）: `jmp reg` を call と同様に `fle_dispatch` で解決する（pseudo-VA は dispatch 表にある）、または遅延ロードのヘルパを置き換える。
- 実装前（0x41cfa0）より先へ進んだかは、この計測では「停止点が別の原因に移った」ことしか言えない。実行の関数数は 140 → 38 に減った（経路が変わったため。§5.2）。
- `unsupported_total`: 2,944 → 2,941（-3）。

## 6. 制約の遵守と残置物

- `rm`、`>` による既存ファイルの上書き、git 操作、`/tmp`、sleep/timeout は使っていない。TMPDIR は `lift/out-sysinfo/tmp`。権限拒否はなし（cargo は `CARGO_TARGET_DIR=lift/out-sysinfo/target` で動いた。`out-sysinfo/Cargo.toml` は不要だった）。外部通信は Microsoft Learn のページ取得のみ（GetNativeSystemInfo、GetLogicalProcessorInformation、SYSTEM_LOGICAL_PROCESSOR_INFORMATION、LOGICAL_PROCESSOR_RELATIONSHIP、CACHE_DESCRIPTOR、PROCESSOR_CACHE_TYPE、IsWow64Process、GlobalMemoryStatus、MEMORYSTATUS、GetProcessAffinityMask、SetProcessAffinityMask、SYSTEM_INFO、GetSystemInfo、VirtualAlloc、Memory Management Functions）。
- `fle_modules.c`、`fle_sys.c`、`fle_sys.h`、他の担当の未コミットのファイル（imports_table.rs の既存行、fle_glob/fle_vol/fle_vmap/fle_load/fle_winmm/fle_user_nowin、vex.rs など）は触っていない。
- 後始末できていないもの（削除禁止）: `lift/out-sysinfo/` 全体（約 3.8 GB。`target` が大半）。`out/base*.c`、`out/after*.c`（各 216 MB と分割 16 個ずつ）。`k32-all.txt` などの作業ファイル。`gdb-stop.cmds`（`finish` が後続コマンドを打ち切ったため役に立たなかった失敗作。内容は `gdb-stop2.cmds` に置き換え済み）。`gdb-nullglpi.cmds`（SIGABRT で gdb が止まり harness の停止レポートが出なかった失敗作。`gdb-nullglpi-run.cmds` で `handle SIGABRT pass` に直した）。`run-nullglpi.*` の最初の空の出力は上書きされて残っていない（`gdb-nullglpi-run.cmds` は上書き更新した。これは自分が作った計測用ファイルの上書きで、既存の他者のファイルではない）。
- 触ったファイル（`lift/out-sysinfo/` 配下の生成物は除く）:
  - 新規: `lift/runtime/fle_sysinfo.h`、`lift/runtime/fle_sysinfo.c`、`lift/crates/fe-lift/tests/sysinfo.rs`、`docs/decisions/0034-sysinfo.md`、`docs/investigations/sysinfo-run.md`
  - 編集（追記または局所変更）: `lift/crates/fe-lift/src/imports_table.rs`（TABLE の末尾に 6 行 + コメント 2 行）、`lift/crates/fe-lift/tests/modules.rs`（1 行: テストが「表にない名前」として使う例を差し替え）
  - `lift/out-sysinfo/`: 計測ツール（`iat_unsup.py`、`name_xrefs.py`、`callers.py`、`build-parts.sh`）、`work/`（`fle_sysinfo.h`、`fle_sysinfo.c` の開発用の元）、各ログ、`target/`、`out/`、`parts/`、`obj/`、`bin/`、`test/`、`tmp/`。
