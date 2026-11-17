# 0077: `call [IAT slot]` を応答表の `fle_imp_*` に結び付ける（W5）

状態: 作業者（サブエージェント）が決定。依頼者（PM）の判断は含まない。方向は決定 0073（dsp_ippv2_x64.dll を 2 つ目のモジュールとして lift する）、作業は `docs/investigations/dsp-ippv2-lift-feasibility.md` §6、§9 の W5。実装は `lift/crates/fe-lift/src/indirect.rs` と `imports_table.rs`（どちらも既存ファイルへの追記）。runtime は触っていない（新しい runtime ファイルなし、既存の runtime ファイルも変更なし）。

## 1. 何を変えたか

- `indirect::call`（`call [rip+disp]`、および fe-decode が同じ IAT スロットに解決した `mov rax,[slot]; call rax` 形）で、fe-decode の解決結果が `Resolved::Import { name }` で、その `dll!name` が応答表 `TABLE` に行を持つとき、次の C を出す。

  `{ void fle_imp_X(void); R[4] -= 8; fle_st(R[4], 8, UINT64_C(<次の命令の VA>)); fle_imp_X(); R[4] += 8; }`

  制御は `Ctl::Seq`（`call` の次の命令に続く）。ハンドラ名は `call_imp:<API 名>`（`Stats::handled`）。
- `imports_table.rs` に `lookup_call` を追加した。`TABLE` の同じ行（同じ `fle_imp_*`）を引き、ハンドラ名だけ `call_imp:<API>` にする。ハンドラ名の文字列は行ごとに 1 回だけ作る（`OnceLock`。行数分の小さなリーク）。`TABLE` の行は変えていない（応答表の行数、擬似 VA、`fle_dispatch_table` は不変）。
- 応答表に行がない API の `call [IAT]` は従来のまま: `fle_unsupported`、`Call: indirect through IAT slot (not lifted)` として数える。**実行時は `fle: unsupported instruction at <VA>` で SIGABRT**（名前付きではなく VA 付き。jmp 形 thunk の未応答も同じ `fle_unsupported` で、同じ扱い）。
- IAT 以外の間接 call（`call_ind:rip_table`、`reg_const`、`reg_table`、`dispatch`）は変えていない（テスト `indirect_calls_outside_the_iat_are_unchanged`）。`jmp [IAT]` の thunk の生成も変えていない。

## 2. 呼び出し規約の根拠（現行の jmp 形との対応）

- jmp 形 thunk（`lib.rs` の `Mnemonic::Jmp`）は `{ void f(void); f(); return; }`。`jmp` は戻りアドレスを積まないので、thunk に入った時点で `[rsp]` は呼び出し元の `call` が積んだ戻りアドレスで、`fle_imp_*` はそれを触らずに R[0] へ戻り値を置き、thunk が `return;`（= `ret`、pop しない）する（`fle_imports.h` の先頭コメント）。`fle_imp_*` は引数を rcx、rdx、r8、r9（R[1]、R[2]、R[8]、R[9]）と `[rsp + 0x28 + 8k]`（戻りアドレスが `[rsp]` にある前提）から読む。
- `call [IAT]` は、`call` 自身が戻りアドレスを `[rsp]` に積む。したがって `fle_imp_*` に入る時点のスタックの形（`[rsp]` = 戻りアドレス、その上に 32 バイトのシャドウ領域と 5 番目以降の引数）は jmp 形と同じ。生成 C は、既存の直接 call（`lib.rs`）、解決済み間接 call（`indirect::call` の `f_<target>()`）、`call_ind:dispatch`（`fle_dispatch` が応答表の擬似 VA で `fle_imp_*` を呼ぶ形。決定 0015 §4 の「`call` 経由では呼び出し元が戻りアドレスを積むので `fle_imp_*` の規約と合う」）と同じ形で、積む → 呼ぶ → pop する。`fle_imp_*` の側に変更は要らない。
- 擬似 VA（`RESP_VA_BASE` = 0x7FF9_0000_0000、決定 0015。`IMP_VA_BASE` という名前の定数は存在しない: 0015 §4）との関係: この変更は擬似 VA を使わない。IAT スロットは fe-decode が静的に API 名へ解決し、生成器が `fle_imp_*` を直接呼ぶ。`fle_dispatch`、`fle_dispatch_table`、`fle_resp_table` は変えていない。**ゲストが IAT スロットの値をレジスタに読んで解決できない形（解決できない `call reg`）で呼ぶ場合は従来どおり `fle_dispatch` を通り、スロットの値が擬似 VA でなければ `fle_dispatch_miss` で止まる**（`lift/harness/image.c` が IAT スロットを「そのスロット自身の VA」で埋める: 決定 0015 §4。この変更では触っていない）。

## 3. FLEngine への影響: なし

- `fe-lift lift binaries/win64/FLEngine_x64.dll OUT.c --pdata` の出力 SHA-256 は変更前も変更後も `67827424301459e6f2da1ccb46a5fd081b7de9a38c64e1bca44ba9bf5d15584d`（`lift/out-w5/flengine-base.c`、`flengine-after.c`）。functions 59,297、unsupported_total 531。
- 理由: FLEngine の `import_calls` は 0（IAT は `jmp [IAT]` の thunk からだけ使われる。調査文書 §6）。`Call: indirect through IAT slot` は FLEngine の統計に出ない（`lift/out-w5/flengine-base.stats.txt`。`Jmp: indirect through IAT slot` が 530 件）。
- この比較の間、他の subagent の `fe-lift/src/vex.rs` などの変更がワークツリーにあったが、SHA は一致したまま（切り分けの必要は出なかった）。

## 4. dsp_ippv2_x64.dll への影響

`fe-lift lift lift/out-ext/dsp_ippv2_x64.dll /dev/null --pdata`（ImageBase 0x180000000 のまま。統計だけ）:

| 項目 | 変更前 | 変更後 |
|---|---:|---:|
| functions | 32,162 | 32,162 |
| emitted_instructions | 6,504,638 | 6,504,638 |
| unsupported_total | 974,004 | 973,920（-84） |
| `Call: indirect through IAT slot (not lifted)` | 138 | 54 |
| `call_imp:*`（新規ハンドラ） | 0 | 84 |
| `Jmp: indirect through IAT slot (not lifted)` | 7 | 7（変更なし） |

（注）最後に再実行した時点（他の subagent の `vex.rs` / `fle_vex.h` の変更がワークツリーにある状態）では unsupported_total が 973,919 だった（上の表の 973,920 は作業の途中の実測）。`Call` / `Jmp` の IAT の件数は 54 と 7 のままで、差の 1 は gs 関連の他者の変更による（`gs:` の未対応が 3 か所とあったもの。原因の切り分けは未実施、推測）。この作業の数値は「138 → 54、`call_imp` 84」。

`call [IAT]` の 138 か所は fe-decode の `import_calls` 138（`fe-decode closure ... --pdata`）と一致。API 名別（`lift/out-w5/dsp-by-api.txt`。テスト `call_iat_dll.rs` が出す）:

応答表に行があり 0 になった API（31 名、84 か所）: GetLastError 23、SetLastError 6、WriteFile 6、DeleteCriticalSection 5、FreeLibrary 4、LoadLibraryExW 4、CloseHandle 3、GetCurrentProcess 3、GetProcAddress 3、CreateFileW 2、FindClose 2、HeapAlloc 2、RaiseException 2、ExitProcess 1、FindNextFileW 1、GetACP 1、GetCPInfo 2、GetCommandLineW 1、GetCurrentProcessId 1、GetCurrentThreadId 1、GetModuleFileNameW 1、GetModuleHandleW 1、GetProcessHeap 1、GetStartupInfoW 1、GetStdHandle 1、GetSystemTimeAsFileTime 1、HeapFree 1、LCMapStringW 1、QueryPerformanceCounter 1、TerminateProcess 1、TlsSetValue 1。

応答表に行がなく未応答のまま残る API（28 名、54 か所）: IsProcessorFeaturePresent 5、RtlLookupFunctionEntry 5、RtlCaptureContext 4、RtlVirtualUnwind 4、RtlUnwindEx 3、SetUnhandledExceptionFilter 3、UnhandledExceptionFilter 3、EncodePointer 2、FreeEnvironmentStringsW 2、GetFileType 2、InitializeCriticalSectionAndSpinCount 2、IsDebuggerPresent 2、WriteConsoleW 2、FindFirstFileExW、FlushFileBuffers、GetCommandLineA、GetConsoleMode、GetConsoleOutputCP、GetEnvironmentStringsW、GetModuleHandleExW、GetOEMCP、GetStringTypeW、HeapReAlloc、InterlockedFlushSList、IsValidCodePage、RtlPcToFileHeader、SetFilePointerEx、SetStdHandle（各 1）。

調査文書 §6.1 が「`DLL_PROCESS_ATTACH` の直線上で実際に呼ばれる可能性が高い未登録の API」とした `InitializeCriticalSectionAndSpinCount` は 2 か所とも未応答のまま（W6 の対象。この作業では応答表に行を足していない）。

## 5. テスト

- `lift/crates/fe-lift/tests/call_iat.rs`（3 件、合成 PE）。`mov ecx,77; call [SetLastError]; call [GetLastError]; add rax,1; call <jmp [GetLastError] の thunk>; mov rax,[slot]; call rax; add rax,3; ret` を生成し、全 `runtime/fle_*.c` とリンクして clang で native 実行する。確認した値: R[0] = 80（77 + 1 + 3。値は runtime の応答から出る）、実行後の `R[4]` が実行前と同じ、最後の `call rax` が積んだ戻り番地（次の命令の VA）が `[rsp - 8]` の位置に残っている（`ret` は pop しない）、ゲストの戻りアドレスのスロットは不変。生成 C の本文（push → `fle_imp_X()` → pop）、`call_imp:*` の件数（メモリ形とレジスタ形を含む）、`jmp_imp:GetLastError` が 1 のまま、unsupported_total 0。
- 未応答 API（`k.dll!Fn`）: `Call: indirect through IAT slot (not lifted)` 1、`call_imp:` なし、実行すると `fle: unsupported instruction at <VA>` で SIGABRT、以降の出力なし。
- IAT 以外の間接 call（`call rbx`、未解決）は `call_ind:dispatch` のまま。
- `lift/crates/fe-lift/tests/call_iat_dll.rs`（1 件）: 実 DLL があれば API 名別の表（上記）を出し、全件が `call_imp` か未応答のどちらかになること、合計が 138 であることを確かめる。DLL がなければ（`lift/out-ext` は gitignore）何もせず通る。
- 既存の表明は変えていない。`indirect.rs` 内の単体テスト（`call [IAT]` が `Call: indirect through IAT slot (not lifted)` 1）は、そこで使う IAT 名 `k.dll!Fn` が応答表に行を持たないので、そのまま通る。
- 全体: `cargo test --workspace --no-fail-fast`（`FLENGINE_PE` 指定、`CARGO_TARGET_DIR=lift/out-w5/target`）は 86 バイナリ + doc-tests 3 = 89 結果、**1,044 件 すべて成功**（基準の 87 結果・1,040 件に、新規 2 バイナリ 4 件を足した値と一致。基準の 87 は doc-tests を含む数と読んだ）。
- a4 survey（`OUT=lift/out-w5`。変更後の FLEngine の生成 C を分割 → `build_native.sh` → `a4-survey.sh`）: 停止点は `LoadLibraryW "dsp_ippv2_x64.dll"`（決定 0060 の基準と同じ）、init_count 145 / 942、最後の unit は index 144、run_rank 132 / 895、関数実行数（distinct）821、エントリ 15,685。変化なし（生成 C が不変なので当然の結果）。

## 6. 未確認・限界

- 実行時の検証は合成 PE と native（Linux、clang）だけ。DLL の `call [IAT]` が実際に実行されたとき `fle_imp_*` が期待どおりに応じるかは、DLL を実行していないので未確認（W2 以降）。
- 84 か所のうち、実行時に引数を 5 個以上取る API（`[rsp+0x28+…]` 参照）や、DLL 側がシャドウ領域をどう使うかは、各 `fle_imp_*` の既存の仕様どおりで、この変更では検証していない。
- 未応答 API は名前付きの停止ではなく `fle_unsupported(VA)`（jmp 形 thunk の未応答と同じ。VA から API 名を引ける）。調査文書 §9 W6 の「呼ばれたら名前付きで止まる」にするには、`fle_stop` 系の runtime が要る。この作業では行わなかった。
- `call [reg+disp]` で IAT でないテーブル、スロットの値を書き換える DLL（遅延ロードのヘルパなど）の形は変えていない。
- `jmp [IAT]`（DLL に 19 か所。うち 7 か所が行なしで未応答）、間接 jmp の表（W3(d)）は対象外。
- 他の subagent が同時に編集した `vex.rs`、`fle_vex.h`、`harness/main.c`、`harness/wasm/main.c`、`tests/gs_stacklimit.rs` は、この作業と無関係（触っていない）。FLEngine の SHA（`vex.rs` の変更がワークツリーにある状態で再計算して一致）には影響が見えなかった。全体テストの実行時点では `tests/gs_stacklimit.rs` はまだ無く、実行結果（1,044 件）にはそのテストを含まない（`vex.rs` などの変更が全体テストの実行時点でワークツリーにあったかは確認していない）。
