# イベント同期 API の実装と harness の再実行の記録（決定 0050）

実行日: 2026-10-21。作業ディレクトリ `signature-studio-develop`、HEAD cc0fbc3（ワークツリーに他担当の未コミット変更は見えなかった: `git status` は本作業のファイルと既存の未追跡 `lift/iat-unsup-closure.txt` だけ）。決定は `docs/decisions/0050-event-sync.md`、調査は `docs/investigations/sync-thread-survey.md`。「実測」はコマンドの出力、「読み」は静的な読み、「推測」は未検証。実機の Windows では確認していない。成果物は `lift/out-event/`。

## 1. 結果

| 項目 | 実測 |
|---|---|
| 実装した API | **4**（`CreateEventW`、`SetEvent`、`ResetEvent`、`WaitForSingleObject`）+ `CloseHandle` のクローズフック 1 個。`WaitForSingleObjectEx` は DLL が import していないので実装していない（決定 0050 A） |
| クローズフックの使用数 | **3 / 4**（vol、vmap、event）。上限 4 に**達していない**（残り 1） |
| `tests/event.rs` | **16 件 passed、0 failed**（新規） |
| ワークスペース全テスト | **67 件の `test result` 行（64 バイナリ + doc-tests）、645 passed、0 failed、0 ignored**（`lift/out-event/test-ws.txt`、`cargo test --offline --workspace`）。直前の記録（`locale-six-run.md`）は 65 バイナリ・616 passed。差（+29 件、バイナリ数は数え方が違う可能性）のうち本作業は +16。残りは HEAD に入った他担当の変更（0047 など）によるもの（推測。切り分けていない） |
| 変異テスト | 異なる変異 **37 個**。**36 個を検出**、1 個は等価変異（§3） |
| harness の停止点 | `fle_unsupported(0x423220)`（CreateEventW）→ **`fle_dispatch_miss`: `indirect call to 0x403582 is not a known function entry`**（§5） |
| 関数実行数（distinct） | 281 → **284** |
| 入口数（`function entries (calls)`） | 5,904 → **5,916** |
| 停止時の呼び出し深さ | 7 → **6** |
| 生成の `unsupported_total` | **554**（601 → 554。内訳 §4） |
| wasm（node）の停止点 | native と**同じ**（`calling … rsp=` の 1 行以外は 1 行も違わない） |

「前」の数値（281 / 5,904 / 深さ 7）は `locale-six-run.md` の記録。ただし前の生成 C（`out-wasm2/out/`）は 0047 より前のもので、今回の生成 C は HEAD（0047 の 43 命令の実装を含む）から作り直したので、**入口数・関数実行数の差はイベントの実装だけの効果ではない**（0047 の命令の実装を除いた比較はしていない）。

## 2. 実装

### 2.1 ファイル

- 新規 `lift/runtime/fle_event.h`、`fle_event.c`。`lift/out-event/work/` で `clang -O0 -Wall -Wextra -Werror` を通し、単体テストを回してから、`runtime/` に 1 回で置いた（ヘッダ先、.c 後。`cmp` で一致を確認）。
- `lift/crates/fe-lift/src/imports_table.rs`: 末尾に 4 行追記（編集直前に読み直した。他の行は変更なし。`git diff` は +5 行: コメント 1 + 4 行）。
- `lib.rs` は変更していない。`fle_vfs.c`、`fle_vol.c`、`fle_vmap.c`、`fle_sys.c` も変更していない。

### 2.2 方式

- ハンドルは `0x7FFA80000000 + 4*(i+1)`、再利用しない。フックは領域の内側だけを引き受ける。
- 可搬性: 標準 C のみ（`fle_host.h` の `fle_host_tid()`、`fle_stop`、`fle_set_last_error`）。`long`、`uintptr_t`、asm、host の時計・スレッド・プロセスは使っていない（テストで検査）。
- 仕様の出典（Microsoft Learn、2026-10-21 取得）: CreateEventW、SetEvent、ResetEvent、WaitForSingleObject、WaitForSingleObjectEx、CloseHandle。
  - 公式: 引数の意味、自動/手動リセットと初期状態、SetEvent/ResetEvent の戻り値（成功は非 0、失敗は 0 + GetLastError）、自動リセットの消費、`WAIT_OBJECT_0` = 0 / `WAIT_TIMEOUT` = 0x102 / `WAIT_FAILED` = (DWORD)0xFFFFFFFF、ms = 0 は即座に戻る。
  - **公式に無い値（選択、決定 0050 に記載）**: 無効なハンドルの last error = 6（3 つのページは失敗時に GetLastError を呼べと書くだけで、無効ハンドルのコードを書いていない）、閉じ済みの再 Close の 6（fle_vfs.c と同じ推測）、ハンドルの値、`INFINITE` の数値 0xFFFFFFFF（windows-sys、docs.rs、非公式で確認）。
- **有限タイムアウト・未シグナルは abort**（時計を進めて `WAIT_TIMEOUT` を返さない）。理由は決定 0050 C。INFINITE・未シグナルも abort（別メッセージ）。

## 3. テストと変異テスト

### 3.1 `tests/event.rs`（16 件）

- 4 通り（手動/自動 × 初期シグナル/非シグナル）の状態遷移（32 行の出力を固定）、BOOL の下位 32 ビット（高位ビットだけ非 0、0xFFFFFFFF、0x80000000、ms の 64 ビット幅）。
- ハンドル値（領域内、重複なし、再利用なし、成功時に last error を変えない、仮想時計が動かない）、`WaitForSingleObject` の各結果、無効ハンドル（NULL と閉じ済み）、CloseHandle 後の再利用、二重・三重 Close、vol/vmap/ファイルのフックとの共存。
- abort: 名前付き（4 通り）、属性付き、INFINITE 未シグナル（4 通り）、有限未シグナル（5 通り）、発行していないハンドル（10 種 × 3 API）、領域内の未発行・非整列の Close、DuplicateHandle（イベント）、別スレッド（5 API）。
- 領域: 0x7FFA80000000、サイズ 0x10000000、他の領域（0x7FFA00000000〜0x7FFA70000000、予約 0x7FFA30000000/0x7FFA40000000）と重ならない、ヒープ・モジュール・応答 VA より上、47 ビット未満。
- ソース検査: ホストの時計・スレッド・プロセス・ファイル・`long`・asm を使わない。クローズフックの登録が runtime 全体でちょうど 3 個（4 以下）。

### 3.2 変異テスト（`lift/out-event/make_mutants.py`、`make_mutants2.py`、`run_mutants*.sh`。ログ `mut-logs/`、`mut2-logs/`）

`fle_event.c`（または `.h`）を 1 か所変えた変異体を `FLE_EVENT_SRC_DIR` で差し替えて `tests/event.rs` を走らせた。runtime 本体は変えていない。

- 1 回目 34 個（ソース 30 + ヘッダ 4）: 28 個が assertion の失敗で検出。6 個（`initial-ignored`、`initial-always`、`no-close-hook`、`attrs-ignored`、`name-ignored`、`foreign-handle-fails-softly`）は未使用変数の `-Werror` でコンパイルに失敗しただけだったので、検出とは数えず、`(void)` を足して作り直した（2 回目）→ 全部 assertion の失敗で検出。
- 1 回目の生存 3 個（`unissued-close-claims-nothing`、`region-lower-bound-inclusive`、`region-too-big`）: テストの穴だった。`tests/event.rs` を局所変更（領域サイズの固定、領域内の未発行の Close がこのフックの停止であること、領域の境界 2 点がフックの外であること）→ 3 個とも検出。
- 2 回目の新規 3 個: `region-upper-bound-inclusive`、`alignment-unchecked` は検出。`closed-wait-after-signal`（`if (e->signaled || !e->live)`）は**生存**: `event_arg` が閉じ済みを NULL で返すので、その条件に届かない（**等価変異**、読み。到達しない）。
- 基準線（変異なし）: 16 passed、0 failed。

## 4. harness の再実行（all-pdata）

手順は `wasm-port-run.md` / `locale-six-run.md` と同じ。出力は `lift/out-event/nb/`、`lift/out-event/wasm/`（`TMPDIR=lift/out-event/tmp`、`EM_CACHE=lift/out-event/wasm/emcache`。`emcache` は `out-loc4/wasm/emcache`（28 MB）のコピー）。

1. 生成: `fe-lift lift FLEngine_x64.dll out/wasm-pdata.c --pdata`（`CARGO_TARGET_DIR=lift/out-event/nb/target`）。`functions 58695`、`c_bytes 215,540,788`（`out-wasm2` の 215,539,555 より 1,233 B 多い）。`handled` に `jmp_imp:CreateEventW 1`、`SetEvent 1`、`ResetEvent 1`、`WaitForSingleObject 1`（各 thunk 1 つ）。分割 16 本は `split_c.py`（約 430 MB。2 GB 級ではないので複製せず、ここで生成）。
2. `unsupported_total`: 601 → **554**。`Jmp: indirect through IAT slot (not lifted)` が 557 → 553（**−4 = 本作業の 4 thunk**）。他の差 −43（Psubw 10、Pcmpgtd 8、Rcpps 8、Bswap 7、Test high-byte 7、And 1、Andnpd 1、Xgetbv 1）は **0047（A2a、HEAD 00858ec）の効果**で、本作業のものではない（0047 は「601 → 558」と記録。558 − 4 = 554 で一致）。残りは `Jmp: indirect through IAT slot` 553 と `Jmp: indirect target unresolved` 1。
3. native: `OUT=lift/out-event/nb bash lift/harness/wasm/build_native.sh` → `nb/native/flengine-native`。実行は `run-native.txt`、`run-native.err`。
4. wasm: `OUT=lift/out-event/wasm bash lift/harness/wasm/build.sh rt gen-c harness link`。runtime 33 本（`fle_event.c` を含む）は errors 0、warnings 0。生成 C 16 本は errors 0。`flengine.wasm` は 115,741,731 B（前回 115,724,550 B）。実行: `node bin-base/run_node.cjs binaries/win64/FLEngine_x64.dll`（node は PATH の v24.19.0）→ `wasm/run-node-base.txt`。
5. 比較: `diff <(grep -v '^calling' nb/run-native.txt) <(grep -v '^calling' wasm/run-node-base.txt)` は**差なし**。`calling … rsp=` の 1 行だけ違う（native `0x7079a29fffc0`、wasm `0x11c4bf60`）。

## 5. 次の停止点

- `fle: indirect call to 0x403582 is not a known function entry`（`abort kind: fle_dispatch_miss`）。API の未対応ではなく、**間接 call の先が関数入口の表に無い**。
- 呼び出しスタック（native と wasm で同じ）: `f_41a7b0` ← 0x527d6a（`f_527d20`）← `f_4120c0` ← `f_412160` ← `f_41d8f0` ← エントリ `f_1376820`。`f_527d20` は InitTable index 42（`out-inv/units_first_api.tsv` の 42 行目）の proc（読み）。
- 読み: `f_527d20` が 0x527d4b で `TObject.Create` 相当（`f_40f9e0`）をした後、0x527d5a〜0x527d65 で `rdx = rbx + 0x10`（nil なら nil）にして `f_41a7b0` を呼ぶ。`f_41a7b0`（0x41a7d2）は `rdx` が指すオブジェクトの vtable の `[+0x8]` を呼ぶ。Delphi の `_IntfCopy`（インターフェース変数の代入: 新しい側の `_AddRef`、古い側の `_Release` を `[+0x8]`、`[+0x10]` で呼ぶ）の形（推測）。
- 0x403582 は **関数入口と認識されていない小さな thunk**: `add rcx, -0x10; jmp 0x41c1a0`（バイト `48 83 c1 f0 e9 15 8c 01 00`）。隣の 0x403579（`add rcx,-0x10; jmp 0x41c180`）、0x40358b（`…; jmp 0x41c1b0`）も同じ形で、データ側の 3 要素の表（0x403598: 0x403579、0x403582、0x40358b。実測、DLL のバイト列）が指す。生成 C の `fle_dispatch_table` に 0x403579、0x403582、0x40358b は無い（`grep` で 0 件。0x41c180、0x41c1a0 は有る）。
- 読み: 3 つはインターフェースの vtable（IUnknown の QueryInterface/AddRef/Release 相当）が指す「Self をインターフェースからオブジェクトへ補正する thunk」で、直接 call されず、`.pdata` にも無いため、lift の関数入口の探索（`--pdata` の seed）に入っていない（推測。`.pdata` を引いて確かめてはいない）。
- 範囲外なので実装していない。直すなら、過去の `0007`/`0008`/`0031`（extra seed、pdata seeds）と同種の入口 seed の追加で、0x403579、0x403582、0x40358b の 3 つ（読み）。他にも同形の thunk が vtable にあるはずで、全数は未調査。

## 6. 未確認、限界

- 停止点より先の実行で、`CreateEventW` が何回、どの属性の組み合わせで呼ばれたかは見ていない（`entry log` は最初の 256 件）。この停止点までに `SetEvent`/`ResetEvent`/`WaitForSingleObject` は呼ばれていない可能性が高いが、数えていない。
- 4 つの手動/自動 × 初期状態の組み合わせの `CreateEventW` が実際に呼ばれる経路（調査 §4.4: index 45、314 など）は、まだ届いていない。
- 有限タイムアウトで未シグナルの `WaitForSingleObject` をゲストが踏むかは未確認（踏めば abort）。
- ワークスペース全テストは、HEAD に他担当の変更が入った状態で 0 failed。切り分けは不要だった。

## 7. 触ったファイルと制約

- 新規: `lift/runtime/fle_event.h`、`lift/runtime/fle_event.c`、`lift/crates/fe-lift/tests/event.rs`、`docs/decisions/0050-event-sync.md`、本書、`lift/out-event/`（`work/`、`mut/`、`mut2/`、`mut-logs/`、`mut2-logs/`、`mut-summary.txt`、`mut2-summary.txt`、`make_mutants*.py`、`run_mutants*.sh`、`test/`、`tmp/`、`target/`、`nb/`、`wasm/`、`test-ws.txt`）。
- 変更: `lift/crates/fe-lift/src/imports_table.rs`（末尾に 4 行 + コメント 1 行）。
- 触っていない: `lib.rs`、`fle_vfs.c`、`fle_vol.c`、`fle_vmap.c`、`fle_sys.c`、`fle_time.c`、既存のテスト。
- `rm`、`>` による既存ファイルの上書き、git 操作、`/tmp`、コマンド文字列の `sleep`/`timeout` は使っていない。生成 C の 2 GB 級の複製はしていない（`out-wasm2` は参照のみ。今回の生成 C は約 430 MB を `out-event/nb/out/` に新規生成し、wasm 側は同じファイルへのシンボリックリンク）。権限拒否はなし。`lift/iat-unsup-closure.txt` は本作業の前からある未追跡ファイル。
