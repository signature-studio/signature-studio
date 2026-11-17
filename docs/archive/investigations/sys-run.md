# 同期・時間・メモリ・システム情報の応答の導入と計測実行

作業者（M1、サブエージェント）の実行記録。日付: 2026-10-20。推測と実測を分けて書く。判断は `docs/decisions/0017-sync-time-memory.md`、仕様は `docs/investigations/import-sys-spec.md`。前回は `docs/investigations/modules-run.md`（停止点 0x407120、SetThreadLocale）。

## 1. 実装（実測）

- 新規: `lift/runtime/fle_sys.h`、`fle_sys.c`（15 API）、`lift/crates/fe-lift/tests/sys.rs`（18 件）。
- 追記: `lift/crates/fe-lift/src/imports_table.rs`（13 行 → 28 行。既存行は変更なし）。
- `fle_sys.c` は `-Wall -Wextra` で警告 0（`out-sys/build-896.txt`、テストの clang 呼び出し）。

## 2. テスト（実測）

- `sys.rs` 単体: 18 件すべて通過（`lift/out-sys/test-sys-2.txt`、EXIT=0）。1 回目（`test-sys-1.txt`）は 3 件失敗した。原因はすべてテスト側: (a) `HeapAlloc` の呼び出しで `CALL` の引数が 4 個に足りない（マクロの誤用）、(b) テスト用の `p[0x10000]` が予約 0x40000 のうちコミット前の範囲に書いて SIGSEGV（テストの誤り。本体の保護は意図どおり働いた）、(c) 実 DLL のテストが seed なしの生成を使い、thunk が閉包に入らなかった。直して 2 回目で通過。
- `cargo test --offline --workspace --no-fail-fast`: **143 件通過、4 件失敗**（`lift/out-sys/test-workspace-1.txt`、EXIT=101）。失敗はすべて `tests/modules.rs`（別の作業者の既存テスト）:
  - `thunks_call_runtime_and_tables_are_emitted`、`dispatch_table_stays_sorted_and_pseudo_vas_are_unique`、`real_dll_emits_response_table`: 応答表の行数を 13 とハードコードしている。今回の追記で 28 になった（`left: 28, right: 13`）。
  - `generated_unit_with_tables_compiles_and_links_with_runtime`: 生成 C が `fle_imp_Sleep` などを参照するが、テストのリンク対象に `fle_sys.c` がない（`undefined reference to fle_imp_Sleep`）。
  - これらは `imports_table.rs` に行を足せば必ず起きる（他の作業者の追記でも起きる）。`modules.rs` は担当外なので変更していない。必要な直しは、(1) 13 を `resp_count()` 相当の値（今は 28。他の作業者の行が入ればさらに増える）に合わせる、(2) リンクに `fle_sys.c`（と、他の作業者の追加 .c）を足す、の 2 点（推測。直してはいない）。
  - 前回の 129 件 + `sys.rs` 18 件 = 147 件 = 143 + 4 で、件数は合う（実測）。`modules.rs` 以外の失敗はない。
- cargo のターゲットは `lift/out-sys/target`、TMPDIR は `lift/out-sys/tmp` に向けた。

## 3. 生成、clang、リンク（実測）

- 生成器: `lift/out-sys/tool`（`out-modules/tool` の複製、`Cargo.toml` の説明文のみ変更）。ビルド EXIT=0（`build-tool.txt`）。seed は `out-sys/seeds-896.txt`（`out-modules/seeds-896.txt` の複製、896 件）。
- 生成: EXIT=0（`gen-sys-896.txt`）。出力 `lift/out-sys/out/after-sys-896.c`（42,510,814 バイト）。
  - functions 8427、closure_instructions 497367（前回と同じ）。c_lines 548193（前回 548148、+45）。
  - 新しい `jmp_imp` の件数: Sleep 2、InitializeCriticalSection 2、EnterCriticalSection 2、LeaveCriticalSection 2、GetTickCount 2、timeGetTime 1、QueryPerformanceCounter 2、QueryPerformanceFrequency 1、GetSystemInfo 2、VirtualAlloc 2、VirtualFree 1、GetProcessHeap 1、HeapAlloc 1、HeapFree 1。合計 22。`out-usage` の件数（未対応 jmp）と一致する。DeleteCriticalSection は 0（閉包で呼ばれない）。
  - unsupported_total 645（前回 667、−22）。
- clang（-O0）とリンク: `lift/out-sys/build.sh`（`out-modules/build.sh` の複製に `fle_sys` を追加）。`EXTRA_RT="fle_int_more fle_sse_more"`、EXIT=0（`build-896.txt`）。`lift/out-sys/bin/flengine-after-sys-896`。

## 4. 実行（実測）

コマンド: `./bin/flengine-after-sys-896 ../../binaries/win64/FLEngine_x64.dll`（TMPDIR は `lift/out-sys/tmp`）。出力 `run-sys-896.txt`、`.err`、`.exit`。EXIT=134（SIGABRT）。

- 停止点 VA: 0x407120、関数 f_407120。
- 命令: `jmp *0x1971800`（`kernel32.dll!SetThreadLocale`、IAT 0x1971800）。前回記録と同じ。
- abort の種類: `fle_unsupported`。
- 関数実行数（distinct）35、関数の呼び出し 51、呼び出し深さ 6。

### 前回（modules-run.md）との比較

| 項目 | 前回 | 今回 |
|---|---|---|
| 停止点 | 0x407120（SetThreadLocale） | 0x407120（SetThreadLocale） |
| abort の種類 | fle_unsupported | fle_unsupported |
| 関数実行数（distinct） | 35 | 35 |
| 関数の呼び出し | 51 | 51 |
| 呼び出し深さ | 6 | 6 |

実行の出力は、スタックのアドレスと終了コードの行を除いて前回と一致する（`diff` で確認。差は `EXIT=134` の行のみ）。**停止点は進んでいない。**

### 理由（実測と推測）

- 実測: 停止までの 51 回の entry に、担当 API の thunk を含む関数（0x407360 は GetModuleHandleW 側の別の thunk）で、今回の 15 API を呼ぶものは出ていない。entry log に出る関数（0x40f470、0x40f490、0x409bf0 など）の呼び出し元の中に、`sys-callers.txt` の 199 件のいずれかがあるかは照合していない。`Enter/LeaveCriticalSection` の呼び出し元は 119/120 関数あるが、停止までの経路ではそれらに入っていない（推測。entry log は関数の入口のみで、thunk は関数内に展開されるため、thunk 呼び出しは entry log に出ない。したがって「呼ばれていない」とは言い切れない）。
- 判断: 停止点（SetThreadLocale）は担当外（ロケール系の別の作業者）。今回の追記で停止点が動かないのは、この API が経路の先にあるため（推測）。

### 担当 API のゲスト関数単位の実行（実測。`run-entry-*.txt`）

harness の第 2 引数でゲスト関数を単独に呼んだ（スタックと引数の状態は実行の途中とは違う。応答が動くことの確認で、正しさの確認ではない）。

| 入口 | 関数の内容（objdump の読み） | 結果 |
|---|---|---|
| 0x41ca30 | `GetSystemInfo` を呼び、`[rsp+0x40]` を返す | 正常に戻り、`rax=1`（オフセット 0x20 + 0x20 = dwNumberOfProcessors = 1。応答の構造体配置と整合） |
| 0x4087e0 | `VirtualAlloc(NULL, 0x10000, MEM_COMMIT, PAGE_READWRITE)` | 正常。`rax=0x71232a170001`（上位はポインタ、下位バイトは `setne al` の結果）。ゲストが返りを 64 ビットのまま保持 |
| 0x407680 | `VirtualAlloc(NULL, 0x13fff0, ...)` の後に 0 番地へ書く | `SIGSEGV`（fault address nil）。呼び出しの後、グローバルのリスト先頭 `[0x15e6b90]` が未初期化（0）で `mov [rax], rcx` を実行したため（読み。VirtualAlloc は 0x4076a4 に戻っている）。単独呼び出しの状態不足であり、応答の失敗ではないと読む（推測） |
| 0x40a0e0 | `QueryPerformanceCounter` を呼び、成功なら `GetTickCount` を呼ばない | 正常に戻る。`rax=8`（呼び出し先の `0x40a140` の結果。内容は未確認） |
| 0x4074d0 | `Sleep` を含む。フラグ `[0x15e60a9]` が 0 なので即 return | 正常（Sleep は呼ばれていない） |
| 0x406f90、0x423940、0x423750 | 各 1 関数で戻る（EnterCriticalSection 等の thunk を含む可能性。内容は読んでいない） | 正常に戻る |

## 5. 次の停止点（報告）

- VA 0x407120、命令 `jmp *0x1971800`、IAT `kernel32.dll!SetThreadLocale`、未対応の種類は「IAT 経由の jmp（対応表にない import）」。前回の記録と同じ。呼び出し元は f_41ce10 の 0x41ce46、引数 `ecx = 0x400`（modules-run.md の読み）。
- ロケール系の作業者が担当。今回は繰り返さない。

## 6. 残りの作業量（推測を含む）

- 担当 API はすべて閉包の thunk 22 か所を応答済み。担当の範囲での追加は、実行がこれらに到達するまで確認できない。
- 停止点が動かないので、担当 API の実行結果への効果は、全体の実行ではまだ確認できていない。到達した時点で、Enter/Leave の対応、Sleep の引数、VirtualAlloc の呼び出し側の使い方（0x40769f 周辺の自由リスト）を確認する必要がある。
- `modules.rs` の 4 件の直し（§2）が必要。
- 生成物の未対応は 645 か所（前回 667）。IAT 経由の jmp は 420 → 398（実測。`gen-sys-896.txt`）。他の作業者の行が入ればさらに減る。
