# VirtualAlloc の MEM_TOP_DOWN 受理と harness の再実行の記録（決定 0052）

実行日: 2026-10-21。作業ディレクトリ `signature-studio-develop`、HEAD 43a0123。DLL は `binaries/win64/FLEngine_x64.dll`。決定は `docs/decisions/0052-virtualalloc-top-down.md`。「実測」はコマンドの出力、「読み」は静的な読み、「推測」は未検証。成果物は `lift/out-va/`（`TMPDIR=lift/out-va/tmp`、`EM_CACHE=lift/out-va/emcache`、`CARGO_TARGET_DIR=lift/out-va/target`）。

## 1. 一次資料

Microsoft Learn の VirtualAlloc（https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualalloc、取得 2026-10-21）の flAllocationType の表: MEM_COMMIT 0x00001000、MEM_RESERVE 0x00002000、MEM_RESET 0x00080000、MEM_RESET_UNDO 0x1000000、MEM_LARGE_PAGES 0x20000000、MEM_PHYSICAL 0x00400000、MEM_TOP_DOWN 0x00100000、MEM_WRITE_WATCH 0x00200000。0x101000 = MEM_COMMIT | MEM_TOP_DOWN（実測。ページの表の値の和）。MEM_TOP_DOWN の説明は「Allocates memory at the highest possible address. This can be slower than regular allocations, especially when there are many allocations.」だけ。配置の詳細はページに無く、決定 0052 に選択として記録した。

## 2. 実装

- `lift/runtime/fle_sys.c`: `FLE_MEM_TOP_DOWN`。受理は「MEM_COMMIT、MEM_RESERVE、両方」のいずれかに MEM_TOP_DOWN を足した値で、lpAddress が NULL のときだけ。MEM_TOP_DOWN 単独、非 NULL アドレスとの組み合わせ、ほかのビット（MEM_WRITE_WATCH、MEM_RESET、未知のビット）は従来の名前付きの停止。アリーナでは `fle_host_claim(want, size, top)` が最も高い空き範囲（終端から、粒度へ切り下げ）を取る。native は配置に効かない。
- `lift/crates/fe-lift/tests/sys.rs`: 新規 `virtual_alloc_top_down_is_accepted`（native、0x101000 の呼び出しがゼロ初期化された書き込み可能な領域を返し、記録され、解放できる。0x102000、0x103000 も）。`virtual_alloc_undefined_cases_abort` の `mem-top-down`（0x100000|COMMIT|RESERVE。受理に変わったので期待が逆）を、止まるべき 5 つの組み合わせ（TOP_DOWN 単独、非 NULL アドレス、WRITE_WATCH、RESET、未知ビット）に置き換えた。
- `lift/crates/fe-lift/tests/wasm_port.rs`: 新規 `arena_top_down_takes_the_highest_free_place`（`-DFLE_VMEM_ARENA`。上端から取る、下の確保より上、粒度整列、解放した範囲の再利用、入らない確保は NULL、下からの確保は従来どおり）。

## 3. テスト（実測）

- `cargo test -p fe-lift --test sys --test wasm_port`: sys 19 件、wasm_port 6 件、すべて ok。
- `cargo test --workspace --no-fail-fast`（`lift/out-va/test-ws.txt`）: `test result` 行 70、**658 passed、0 failed、0 ignored**。直前の記録（`adjustor-thunks-run.md`）は 70 行、656 件。差 +2 は新規 2 件。
- 変異テストはしていない。

## 4. harness の再実行（実測）

生成 C は変えていない（`out-adj/out` を参照。生成の `functions 59297`、`bad_nodes 0`、`unsupported_total 554` は `out-adj/gen/gen-pdata.txt` のまま）。生成物のオブジェクト（native `obj/`、wasm `obj-base/gen`）は `out-adj` から複製し、runtime と harness だけを再ビルドした。

| | native | wasm（node） |
|---|---|---|
| 停止 | `fle: unsupported instruction at 0x424c50`（`abort kind: fle_unsupported`） | 同じ |
| 関数実行数（distinct） | 468 | 468 |
| 入口数（calls） | 12,471 | 12,471 |
| 呼び出し深さ | 7 | 7 |
| 前の停止点（VirtualAlloc 0x101000）の実行数 | 357 | 357 |

- `diff <(grep -v '^calling' run-native.txt) <(grep -v '^calling' run-node.txt)` は差なし。
- 呼び出しスタック（native と wasm で同じ）: `f_424c50` ← `f_5d19e0`（guest-return 0x5d19f3）← `f_5d2680` ← `f_4120c0` ← `f_412160` ← `f_41d8f0` ← エントリ `f_1376820`。
- `unsupported_total`: **554**（生成 C が同じ。停止点は 554 のうちの 1 つ）。
- 0x424c50 は `jmp qword ptr [rip+0x154bf5a]`（Python/capstone。バイト `ff 25 5a bf 54 01`）。IAT スロットは 0x1970bb0 で、pefile の import 表では `user32.dll!GetDC`。つまり **GetDC の import に応答が無い**（thunk の jmp が `fle_unsupported` になっている）。VirtualAlloc の 0x101000 は通った（実行数が 357 → 468 に増えた）。
- 次に必要なもの: `user32.dll!GetDC` の仕様確認と応答。ウィンドウ・GDI 系の API で、これまでの仮想 OS（0014）の範囲とは別の領域。範囲外なので実装していない。

## 5. 未確認、限界

- 0x101000 の呼び出しが実行時に返したアドレスは見ていない（native は host の mmap、wasm はアリーナの上端）。
- native は MEM_TOP_DOWN が配置に効かない。native と wasm でアドレスの値は元から違う（0045）。
- 実 Windows との比較はしていない。ゲストがアドレスの高さに依存するかは見ていない。

## 6. 触ったファイルと制約

- 変更: `lift/runtime/fle_sys.c`、`lift/crates/fe-lift/tests/sys.rs`、`lift/crates/fe-lift/tests/wasm_port.rs`。
- 新規: `docs/decisions/0052-virtualalloc-top-down.md`、本書、`lift/out-va/`（gitignore 済み）。
- `rm`、既存ファイルの `>` 上書き、git の変更操作、`/tmp`、コマンド文字列の `sleep`/`timeout`/`nohup` は使っていない。権限拒否はなし。`lift/out-va/out` は `lift/out-adj/out` へのシンボリックリンク（読み取り専用の参照）。`lift/out-va/` 全体は約 3.0 GB（`du -sh`。cargo の target、emcache、複製した obj を含む。単一ファイルは最大でも `flengine.wasm` 約 116 MB と複製した native のオブジェクト群で、2 GB 級のものはない）。emcc は `JOBS=2` 以下。
