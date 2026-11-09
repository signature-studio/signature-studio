# GetCurrentThreadId を固定の仮想値にした後の native と wasm の再実行の記録（決定 0056）

実行日: 2026-10-21。作業ディレクトリ `signature-studio-develop`、HEAD 07d1a51。決定は `docs/decisions/0056-virtual-thread-id.md`。「実測」はコマンドの出力、「読み」は `lift/objdump-text.txt` の静的な読み、「推測」は未検証。成果物は `lift/out-tid/`（gitignore 済み。`TMPDIR=lift/out-tid/tmp`、`EM_CACHE=lift/out-tid/emcache`。cargo の target は既存の `lift/out-msc/target` を共用）。

## 1. runtime の使用箇所（grep。実測）

- `fle_imp_GetCurrentThreadId`: `lift/runtime/fle_imports.c`（定義）、`fle_imports.h`、`lift/crates/fe-lift/src/imports_table.rs`（応答表 1 行）。
- `fle_host_tid()`: `fle_glob.c`、`fle_imports.c`（TLS の持ち主確認 1 か所。以前は GetCurrentThreadId も）、`fle_ole.c`、`fle_oleaut.c`、`fle_proc.c`、`fle_sys.c`、`fle_sysinfo.c`、`fle_vmap.c`、`fle_vol.c`、`fle_event.c`、`fle_gdc.c`、`fle_gdi.c`。いずれも owner slot（0 = 未記録）との比較で、ゲストへ返す値ではない。
- `FLE_HOST_WASM_TID`: `fle_host.h` だけ（emscripten で 1）。
- TEB: `fle_vex.h` の `fle_teb`（0x2000 バイト、ゼロ埋め）。モデルしているのは 0x30（Self）と 0x58（ThreadLocalStoragePointer）だけ。ClientId（0x40、0x48）に tid を書く箇所は**ない**。ゲストの `gs:` 参照は `gs:0x58`（0x41d14b）を 1 件確認した。
- GetCurrentProcessId: `fle_proc.c` が固定値 `FLE_EMULATED_PID`（0x1000）を返す。host を見ない。TEB との整合は不要（ClientId をモデルしていない）。

## 2. ゲスト側の使われ方（読み）

`0x406f90`（GetCurrentThreadId の thunk）の呼び出し 8 か所:

| 呼び出し | 直後の使い方 |
|---|---|
| 0x410638 | `cmp ebx,eax`（`[rcx+8]` と比較。違えば 0x409730 を呼ぶ） |
| 0x4108b8、0x4109c1 | `[obj+8]` に保存し、`[obj+4] = 1`（所有者とカウント） |
| 0x410e78 | `cmp [rbx+8],eax`（等しければ `[rbx+4]` を加算） |
| 0x410e9e | `[rbx+8]` に保存、`[rbx+4] = 1` |
| 0x4121c7 | `[r13+0x34]` に保存 |
| 0x4128e0 | `cmp eax,[0x15eaddc]` |
| 0x41cee4 | `[0x15e6080]` に保存 |

保存と比較に使い、別の API の引数に直接渡す形は、この 8 か所にはない（読み）。所有者の形（0x4108b8 など）は、`[obj+8]` の値が 0 でないことを「所有者がいる」とみる可能性がある（推測。分岐を読んでいない）。これが決定で値を 0 にしない理由。

差の原因だった文字列化: `f_43aa10`（0x43aa10）は `edx` を 4 ビットずつ右に送って 16 進の桁数を数え、`f_43a890`（0x43a890）に桁数を渡す。`f_43a890` は 0x43a8b1 で `f_414ad0` を呼ぶ。`f_414ad0` は、文字列の長さが同じなら何もしない（0x414b04 `je 0x414b9f`）、違えば再確保（0x414b32 `call 0x409470`）。`f_43aa10` の呼び出し元は 10 件以上（0x43eadf、0x464d56、0x4b9674 ほか）。`f_526700`（TThread の名付け。`RaiseException(0x406d1388, 0, 6, ...)` の引数に名前と ID を入れる）は、この読みの範囲では `GetCurrentThreadId` を直接呼んでいない（`call 0x406f90` の 8 か所に含まれない）。`InitThreadSynchronization` の名前の関数は特定していない。

## 3. 仕様（Microsoft Learn）

出典: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getcurrentthreadid （2026-10-21 に取得。ページの `ms.date` 2024-02-02）。

- 説明: Retrieves the thread identifier of the calling thread.
- Syntax: `DWORD GetCurrentThreadId();`
- Return value: The return value is the thread identifier of the calling thread.
- Remarks: Until the thread terminates, the thread identifier uniquely identifies the thread throughout the system.
- Requirements: Windows XP 以降、Windows Server 2003 以降、`processthreadsapi.h`、Kernel32.lib、Kernel32.dll。

値の範囲、0 の扱い、桁数、プロセス ID との関係はページに書かれていない。Windows の実スレッド ID の経験則は使っていない。

## 4. 変更

- `lift/runtime/fle_imports.h`: `FLE_EMULATED_TID 0x1001u` を追加。
- `lift/runtime/fle_imports.c`: `fle_imp_GetCurrentThreadId` が `FLE_EMULATED_TID` を返す（以前は `(uint32_t)fle_host_tid()`）。
- `lift/runtime/fle_host.h`: コメントの更新（コードは変更なし）。
- 生成 C は変えていない。

## 5. 再実行（実測）

手順は `msctf-caller.md` のとおり（`OUT=lift/out-tid`）。生成 C の 17 パーツは `lift/out-msc/out` と `obj-base/gen`、`native/obj` のオブジェクトを `lift/out-tid/` に複製して再利用し、runtime 36 ファイルと harness を再コンパイルして、wasm をリンク（`build.sh rt harness link`、39 行すべて `errors=0`）、native は `build_native.sh`（clang）で再リンクした。

| | native | wasm（node） |
|---|---|---|
| 停止 | `fle: unsupported instruction at 0x424ee0`（SIGABRT） | 同じ |
| 関数実行数（distinct） | 572 | 572 |
| 入口数（calls） | **13,230** | **13,230** |
| 呼び出し深さ | 10 | 10 |

- `diff <(grep -v '^calling' run-native.txt) <(grep -v '^calling' run-node.txt)`: **差なし**。全体の diff は `calling entry … rsp=` の 1 行だけ（rsp の値はホストのスタックの住所）。
- 各 2 回実行（`run-native.txt`、`run-native2.txt`、`run-node.txt`、`run-node2.txt`）。native 同士、wasm 同士、native と wasm（2 回目）がすべて一致。
- 入口数 13,230 は、以前の wasm（tid = 1）の値と同じ。native が 13,227 から 13,230 になった。13,159 番目の入口列は、今回は取り直していない（全出力と入口数が一致したことから、差は消えたと読む。推測）。
- wasm 側の分岐は今回も gdb で追っていない。

## 6. テスト（実測）

- `tests/tid.rs`（新規、7 件）: 値 4097（引数を読まない、上位 32 ビットが 0）、実 tid と違う、別のホストスレッドから同じ、`__EMSCRIPTEN__` を手で定義したビルドと同じ、2 プロセスで同じ、ソースが host tid を使わない（owner の検査は使う）、ヘッダの値。
- `tests/wasm_port.rs`: 2 件を更新（native: host tid は gettid、ゲスト値は 4097。wasm 定義: host tid は 1、ゲスト値は 4097）。件数は変わらない。
- 変異（`out-tid/work/mkmut.py` が `out-tid/mut/m1`〜`m7` に runtime の複製を作り、`FLE_TID_RT_DIR` で指す。`out-tid/mut/summary.txt`）: **7 変異すべてが失敗で検出された（生き残り 0）**。m1 native が実 tid を返す（6 件失敗）、m2 値が 0（5）、m3 emscripten だけ host tid に従う（2）、m4 上位 32 ビットが立つ（4）、m5 引数がゲスト値に漏れる（2）、m6 wasm の host tid が 0（2）、m7 ヘッダと違う固定値（5）。
- `cargo test --workspace --no-fail-fast`（`out-tid/test-ws.txt`、shell のタイムアウト 3,600,000 ms、フォアグラウンド）: awk の集計は `test result` 行 **74**、**714 passed、0 failed、0 ignored**（cargo の終了コード 0）。直前の基準は 73 行、707 件。差は +1 行、+7 件（新規 `tid.rs` の 7 件）。
