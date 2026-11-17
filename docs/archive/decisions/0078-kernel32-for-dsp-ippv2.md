# 0078: dsp_ippv2_x64.dll が import する KERNEL32 のうち応答表に無かった 35 名（W6）

状態: 作業者（サブエージェント）が決定。依頼者（PM）の判断は含まない。**仮の決定を含む**（下の「仮の決定である点」と「撤回条件」）。方向は決定 0073、作業は `docs/investigations/dsp-ippv2-lift-feasibility.md` §6、§9 の W6。決定 0077 の「未応答 28 名」の続き。実機 Windows では確認していない（ホストは Linux）。DLL（`lift/out-ext/dsp_ippv2_x64.dll`、SHA-256 の先頭 497dd4f7…）は**実行していない**。到達の分類はすべて**逆アセンブルの読み**（capstone。下の補助スクリプト）で、実行で確かめたものではない。

## 1. 35 名の確定と分類（読み）

IAT の 72 名のうち応答表に行が無かった名前は 35 名（`ipp_imports_scan.py` で再確認。調査 §6 と同数）。呼び出し点の VA は `.pdata` の関数を順に逆アセンブルして数えた（`lift/out-w6/k32_sites.py`、`k32_rawscan.py`。出力は `sites.txt`）。FlsAlloc/FlsFree/FlsGetValue/FlsSetValue は `.pdata` に無い 8 バイトの `jmp [IAT]` 小関数（0x17d7558〜0x17d7570）としてだけ参照され、InitializeSListHead は 0x17cb2df の `jmp`、HeapSize と TlsFree も `jmp` 形。それ以外は `call [IAT]`。

到達の確認は、DllMain（0x17caca8）と、ATTACH で走る初期化表（`.rdata` 0x1ec8360〜、スロット 0x1ebd288〜0x1ebd2a0）の項目から、直接 call/jmp だけをたどる探索（`k32_paths.py`、`k32_attach.py`。`attach-reach.txt`、`paths-dllmain.txt`、`paths-exports.txt`）と、前後の命令の読みで行った。「エラー経路」は、`__security_check_cookie`（0x17ca8f0）の失敗側、`__scrt_fastfail` 系（0x17cad64、0x17cb654）、invalid parameter 系（0x17d2744、0x17d2a34、0x17d25a0）の各関数の中。

| 区分 | 名前（呼び出し点 VA） | 読み |
|---|---|---|
| ATTACH の直線上 | InitializeCriticalSectionAndSpinCount（0x17d0a4f。0x17caca8→0x17ca988→0x17cb484→0x17cd210→0x17d071c→0x17d0a04。ほか 0x17d75d2 は初期化表スロット 0x1ebd288 の 0x17d2aa4 から）、InitializeSListHead（0x17cb2df。0x17ca9d8 から直接）、GetEnvironmentStringsW / FreeEnvironmentStringsW（0x17d716d、0x17d71df、0x17d723e。0x17cb354→0x17d2184→0x17d1efc→0x17d7154）、IsValidCodePage（0x17d6d45。0x17cb354→0x17d1d74→0x17d6c5c→0x17d6934→0x17d6cd8）、GetStringTypeW（0x17d87f9。同じ鎖の 0x17d674c→0x17d86c0）、FlsAlloc / FlsGetValue / FlsSetValue（0x17d7558、0x17d7568、0x17d7570。初期化表項目 0x17d33a8 から。決定 0081 の根拠の調査 §7 が、API セット名の照会で FlsAlloc と FlsSetValue が ATTACH で引かれることを実測している） | 実行されうる |
| 初期化表の項目（読み。条件は未確認） | GetCommandLineA（0x17d7024。表項目 0x17d7020）、GetFileType（0x17d353f、0x17d3633。表項目 0x17d36ac の lowio 初期化。GetStdHandle が NULL なら 0x17d35a4 は飛ばす（`lea rcx,[rax+1]; cmp rcx,1; jbe`）、GetStartupInfoW の cbReserved2 が 0 なら 0x17d34b4 も飛ばす（読み）） | 条件付き。通常は呼ばれない見込み |
| 条件付き | GetOEMCP（0x17d665e。`ecx == -2`（OEM コードページを要求）のときだけ。ATTACH の呼び出しは -3 で GetACP 側）、FindFirstFileExW、HeapReAlloc、HeapSize（0x17d603c、0x17d9f0d、0x17d9e96。argv のワイルドカード展開 0x17d5ecc と `_recalloc` 0x17d7264。`0x17d1d74` の引数 ecx は 0x17cb9c4 の 1 で、`cmp r14d,1; jne` の手前で戻るため、展開側の 0x17d621c は mode 2 のときだけ呼ばれる） | 通常の ATTACH では呼ばれない見込み |
| DETACH / 巻き戻し | TlsFree（0x17d095f。0x17cd5e4 から。0x17cb470 と 0x17cd260 の巻き戻し）、InterlockedFlushSList（0x17cd1ea。0x17cb2e8 から 0x17caaf0 の DETACH）、GetModuleHandleExW（0x17d1add。exit 系の 0x17d1ab8。DETACH の 0x17cb440 → 0x17d1b30 からも） | ATTACH では呼ばれない。`FreeLibrary` で DETACH を走らせると実行される |
| エラー・致命経路 | IsProcessorFeaturePresent（5 か所すべて `mov ecx,0x17`＝PF_FASTFAIL_AVAILABLE。0x17cada6、0x17cae8d、0x17cb670、0x17d25c6、0x17d2a3d）、RtlCaptureContext（0x17caf29、0x17caf97、0x17cb69d、0x17d27bd）、RtlLookupFunctionEntry（5。うち 0x17cbd26 は C++ 例外の FindHandler 0x17cbcc4）、RtlVirtualUnwind（4）、UnhandledExceptionFilter（3）、SetUnhandledExceptionFilter（3）、IsDebuggerPresent（0x17cb74c、0x17d2849） | エラー経路だけ |
| C++ 例外・終了 | RtlUnwindEx（0x17cc144、0x17cc264、0x17ccf67）、RtlPcToFileHeader（0x17cd160。0x17cd11c は例外と `terminate` 経路）、EncodePointer（0x17ce944、0x17ceb80。0x17ce8f8、0x17ceb10 の例外処理） | 例外を投げたときだけ |
| stdio・コンソール | WriteConsoleW（0x17da815、0x17da883）、GetConsoleMode（0x17d4b1b）、GetConsoleOutputCP（0x17d413b）、FlushFileBuffers（0x17d8956）、SetFilePointerEx（0x17d2c84）、SetStdHandle（0x17d7f46） | ゲストが C の入出力を呼んだときだけ（IPP の関数は呼ばない見込み） |

IPP の関数（エクスポート 446 名のうち、直接 call でたどれる 499 個の入口。`paths-exports.txt`）から、直接 call/jmp だけで届く未登録名は、FlsGetValue / FlsSetValue（0x17d3220。CRT の errno 取得 0x17d37fc の中）、IsProcessorFeaturePresent と RtlCaptureContext 系（0x17ca8f0 のクッキー失敗側）、RtlPcToFileHeader（0x17cd11c）だけだった。**通常の IPP 関数の実行が FLS を引くのは errno（`_errno`）を使うときで、FlsGetValue と FlsSetValue のスロットは ATTACH で FlsAlloc した索引**。

**IsProcessorFeaturePresent と CPU の系統選択**: 5 か所すべてが PF_FASTFAIL_AVAILABLE（23）だけを問い、「1 なら `int 0x29`（fail fast）」の形で、エラー経路の中にある。IPP の実装系統の選択（選択値 0〜5）は CPUID（0x13c0、0x10c0）だけで決まり、この API は選択に関わらない（読み）。したがって 0 を返しても 1 を返しても系統は変わらない。1 を返すと（実機の Windows 8 以降と同じ）エラー経路は `int 0x29` に進む。0 を返すと RtlCaptureContext → RtlVirtualUnwind → UnhandledExceptionFilter → TerminateProcess へ進む。どちらでもエラー経路は名前付き停止かニーモニック `Int` の停止で終わる（RtlCaptureContext が名前付き停止）。

## 2. 実装したもの（12 名 + IsProcessorFeaturePresent + TlsFree。値の根拠付き）

出典はすべて Microsoft Learn の次のページ（2026-10-23 に取得。`lift/out-w6/learn/*.html`）。**一次**は「ページの文」、**導出**は「ページの文と、この環境の既存の決定からの帰結」、**仮**は「ページに値が無く、この記録での選択」。

| API | ファイル | 仕様（ページの文） | 値の根拠 |
|---|---|---|---|
| InitializeCriticalSectionAndSpinCount | `fle_sys.c` | https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-initializecriticalsectionandspincount 「On single-processor systems, the spin count is ignored」「This function always succeeds and returns a nonzero value」「Initializing a critical section that is already initialized results in undefined behavior」 | 戻り値 1（一次。実機は Windows 10 と定義済み: 決定 0014）。スピン数は保持しない（`GetSystemInfo` が 1 プロセッサ: 決定 0017 で導出）。初期化済みの再初期化は停止（既存の `fle_cs_init` と同じ）。表は既存の臨界領域の表と共用で、Enter/Leave/Delete がそのまま使える |
| InitializeSListHead、InterlockedFlushSList | `fle_sys.c` | https://learn.microsoft.com/en-us/windows/win32/api/interlockedapi/nf-interlockedapi-initializeslisthead 「This structure is for system use only」、https://learn.microsoft.com/en-us/windows/win32/api/interlockedapi/nf-interlockedapi-interlockedflushslist 「If the list is empty, the return value is NULL」 | ヘッダのバイトは読み書きしない（一次。「system use only」）。アドレスを表に覚える（臨界領域と同じ流儀）。Push が無いので初期化済みのヘッドは常に空 → 戻り値 NULL（導出）。未初期化のヘッドの Flush は停止 |
| FlsAlloc、FlsFree、FlsGetValue、FlsSetValue | `fle_imports.c`（TLS と同じファイル、同じ「1 スレッド」規則） | https://learn.microsoft.com/en-us/windows/win32/api/fibersapi/nf-fibersapi-flsalloc 「an FLS index initialized to zero」、…/nf-fibersapi-flsfree 「causes the associated callback routine to be called for each fiber, if the corresponding FLS slot contains a non-NULL value」、…/nf-fibersapi-flssetvalue 「ERROR_INVALID_PARAMETER: The index is not in range」 | 索引は不透明で、増やして渡し再利用しない（仮。ページは再利用に触れない）。スロットは 0 で始まる（一次）。FlsSetValue の範囲外は 0 と 87（一次）。FlsFree と FlsGetValue の「失敗」はコードが書かれていない → 未割り当てと解放済みの索引は停止（創作しない）。コールバックが非 NULL で値が非 NULL の FlsFree は、ゲストの関数を呼ぶ必要があるので停止（`fle_call_guest` は使っていない）。索引の最大数にページは触れないので、この実装の上限 4096 を超えたら停止（FLS_OUT_OF_INDEXES は返さない） |
| TlsFree | `fle_imports.c` | https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-tlsfree 「The TlsFree function does not free memory blocks whose addresses have been stored in the TLS slots」 | 成功は 1。未割り当てと解放済みは 0 と 87（TlsSetValue、TlsGetValue の既存の選択と同じ。ページはコードを書かない → 仮）。解放後の索引は TlsSetValue/TlsGetValue も失敗（既存の `fle_tls_allocated` に解放済みの条件を足した）。再利用しない |
| GetCommandLineA | `fle_env.c` | https://learn.microsoft.com/en-us/windows/win32/api/processenv/nf-processenv-getcommandlinea 「The command line returned by GetCommandLineA is a conversion of the Unicode command line to the 8-bit process code page」 | GetCommandLineW（決定 0018: 引用符付きの仮想パス）の全文字が ASCII で、プロセスのコードページ 1252 は ASCII を変えない（既存の `fle_locale.c` が置く前提と同じ。導出） |
| GetEnvironmentStringsW、FreeEnvironmentStringsW | `fle_env.c` | https://learn.microsoft.com/en-us/windows/win32/api/processenv/nf-processenv-getenvironmentstringsw 「Var1=Value1\0 Var2=Value2\0 … VarN=ValueN\0\0」、…/nf-processenv-freeenvironmentstringsw 「The pointer to the block must be obtained by calling the GetEnvironmentStringsW function」 | 仮想環境（決定 0018。空で始まる）の写しを毎回作る。空なら NUL 1 つ（一次の書式から）。Free に渡る他のポインタと二重解放は停止（コードが書かれていない） |
| GetOEMCP | `fle_locale.c` | https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-getoemcp 「the current OEM code page identifier for the operating system」 | 437（決定 0016 の `fle_emulated_locale.oemcp`。値はページに無い） |
| IsValidCodePage | `fle_locale.c` | https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-isvalidcodepage 「A code page is considered valid only if it is installed on the operating system」 | 環境が決めた 2 つ（1252 と 437）だけ 1。ほかの識別子はページに一覧が無い → 停止 |
| IsProcessorFeaturePresent | `fle_sysinfo.c` | https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-isprocessorfeaturepresent 「If the feature is supported, the return value is a nonzero value」、PF_PAE_ENABLED: 「All x64 processors always return a nonzero value for this feature」、https://learn.microsoft.com/en-us/cpp/intrinsics/fastfail 「Support for the native fast fail mechanism began in Windows 8」 | 下の表 |

IsProcessorFeaturePresent の値（CPUID の応答、決定 0009 と 0047: 葉 1 の EDX は SSE と SSE2 のみ、ECX は 0、葉 7 と拡張葉なし、と矛盾しない）:

| PF 番号 | 値 | 根拠 |
|---|---|---|
| 6（SSE）、10（SSE2） | 1 | CPUID の EDX bit 25、26（導出） |
| 9（PAE） | 1 | ページの文（一次） |
| 23（FASTFAIL） | 1 | __fastfail のページ（Windows 8 以降）と、環境が Windows 10（決定 0014）から（導出） |
| 2（CMPXCHG8B）、3（MMX）、7（3DNow!）、8（RDTSC）、13（SSE3）、14（CMPXCHG16B）、17（XSAVE）、22（RDFSBASE）、36（SSSE3）、37、38（SSE4.1、4.2）、39（AVX）、40（AVX2）、41（AVX-512F）、60（BMI2） | 0 | CPUID が報告しない（EDX bit 4、8、23 も 0、ECX は 0、葉 7 なし）ので、同じ環境が「あり」と答えない（導出）。**実機の x64 Windows は 2、3、8、13、14、17 などを 1 と答えるので、実機とは違う** |
| 上以外（0、1、12、15、20、21、24 など、ARM、未掲載の番号） | 停止 | ページに値が無く、環境の決定も及ばない |

## 3. 実装しなかった（名前付き停止）21 名

応答表には行を足し、runtime 関数は `fle_stop` の流儀で「`fle: <API>: <理由>`」と abort する。**呼ばれたら止まる。戻り値は作らない**。理由は 1 の表の読みに基づく。

- 例外・アンワインド（`fle_proc.c`）: RtlCaptureContext、RtlLookupFunctionEntry、RtlVirtualUnwind、RtlUnwindEx、RtlPcToFileHeader、UnhandledExceptionFilter、SetUnhandledExceptionFilter。**実行されるのはエラー経路と C++ 例外経路だけ**で、呼び出し点の周辺はどれも `__security_check_cookie` の失敗側（0x17ca8f0→0x17cad98→0x17caf8c）、`__scrt_fastfail` 系（0x17cad64）、invalid parameter 系（0x17d2744 など）、C++ の FindHandler（0x17cbcc4）と `_CxxThrowException` 系（0x17cc05c 以下）。ここへ入ると、機械コンテキストの CONTEXT（`R[]` は CONTEXT ではない）と `.pdata` の関数表が要るが、どちらもこの runtime に無く、値を創作しない。なお RtlCaptureContext の手前で 5 か所の IsProcessorFeaturePresent（PF 23）が先に呼ばれ、1 を返すと `int 0x29` に進む（1 の末尾）。
- IsDebuggerPresent、EncodePointer（`fle_proc.c`）: 前者はエラー経路の 2 か所だけ。後者は C++ の例外処理だけ。
- stdio・コンソール・ファイル（`fle_vol.c`）: GetFileType、SetStdHandle、GetConsoleMode、GetConsoleOutputCP、WriteConsoleW、FlushFileBuffers、SetFilePointerEx、FindFirstFileExW。仮想環境にコンソールも標準ハンドルもない（GetStdHandle は NULL を返す: 決定 0024）。
- HeapSize、HeapReAlloc（`fle_sys.c`）: HeapAlloc は `malloc` のブロックで、ページの「requested size」を記録していない。
- GetStringTypeW（`fle_locale.c`）: 文字ごとのクラスは Unicode のデータで、ページに無い。
- GetModuleHandleExW（`fle_modules.c`）: exit 系と DETACH でだけ。ページのフラグ（PIN、UNCHANGED_REFCOUNT、FROM_ADDRESS）は仮想モジュール表が持たない参照数とアドレス検索を使う。

**実装したが応答表の行にしなかった 2 名**: TlsFree と IsProcessorFeaturePresent。既存のテストが「行が無い」ことを表明している（`tests/imports_table.rs` は TlsFree を「表に無い名前」の例に使い、`tests/sysinfo.rs` 918 行は `GetProcAddress("IsProcessorFeaturePresent")` が NULL と 127 を返すことを表明する。決定 0034）。この依頼では既存テストを編集しない。結果、DLL の `call [IAT]` 5 か所（IsProcessorFeaturePresent）と `jmp [IAT]` 1 か所（TlsFree）は従来どおり `fle_unsupported(VA)` のまま（`Call: indirect through IAT slot (not lifted)` 5、`Jmp: indirect through IAT slot (not lifted)` 1）。どちらも runtime 関数はあり、`tests/k32_dsp.rs` が直接呼んで確かめている。行を足すには、その 2 つのテストの期待値は、更新の採否が未決（実行される経路がエラー経路と DETACH だけなので、緊急ではない）。

## 4. 生成物への影響（実測）

### dsp_ippv2_x64.dll（`fe-lift lift lift/out-ext/dsp_ippv2_x64.dll /dev/null --pdata`）

「変更前」は、同じワークツリーから、この作業で足した行だけを `imports_table.rs` から除いたコピーで測り直した値（`lift/out-w6/base-src`、`dsp-base.stats.txt`）。決定 0077 の 138 → 54 に、他者の変更（functions 32,162 → 32,648）が加わっているため、0077 の表とは直接比べない。

| 項目 | 変更前 | 変更後 |
|---|---:|---:|
| functions | 32,648 | 32,648 |
| unsupported_total | 974,582 | 974,527（-55） |
| `Call: indirect through IAT slot (not lifted)` | 54 | 5（IsProcessorFeaturePresent） |
| `Jmp: indirect through IAT slot (not lifted)` | 7 | 1（TlsFree） |
| `call_imp:*` の件数 | 84 | 133 |
| `jmp_imp:*` の件数 | 12 | 18 |

`call [IAT]` 138 か所は、133 か所が応答する（`tests/call_iat_dll.rs` の表。IsProcessorFeaturePresent の 5 か所だけが未応答）。`jmp [IAT]` は 19 か所のうち 18 か所が応答（TlsFree が未応答）。

### FLEngine_x64.dll（生成 C は変わった）

`fe-lift lift binaries/win64/FLEngine_x64.dll OUT.c --pdata` の SHA-256 は、基準の `67827424301459e6f2da1ccb46a5fd081b7de9a38c64e1bca44ba9bf5d15584d` から **`43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1` に変わった**（runtime の最終版でも同じ。生成 C は runtime に依存しない）。**依頼の条件（不変）は満たしていない**。理由は、応答表に 33 行を足すと、FLEngine の `fle_resp_table` と `fle_dispatch_table` の擬似 VA の末尾にも 33 行が付くことと、FLEngine の IAT の `jmp [IAT]` のうち、DLL と共通の名前を持つスロット 6 つが応答するようになったことの 2 つ。

- 差分（基準 `lift/out-w5/flengine-base.c` との `diff`。`lift/out-w6/flengine/diff-vs-base.txt`）: 134 行（`<` 8 行、`>` 107 行）。内訳: 関数の宣言の追加 33 行、`fle_dispatch_table` の擬似 VA 33 行、`fle_resp_table` の 33 行、`fle_dispatch_count` 59,524→59,557 と `fle_resp_count` 227→260 の 2 行、**6 つの `jmp [IAT]` の thunk** が `fle_unsupported(VA)` から `fle_imp_*(); return;` になった 6 行。6 つ: 0x406ea0（GetFileType）、0x406ff0（RtlUnwindEx）、0x407000（UnhandledExceptionFilter）、0x4070c0（GetConsoleOutputCP）、0x4235c0（GetOEMCP）、0x423aa0（IsDebuggerPresent）。
- `unsupported_total` 531 → 525（`Jmp: indirect through IAT slot` 530 → 524）。functions 59,297、`c_lines` 2,885,313 → 2,885,412、`c_bytes` 215,700,923 → 215,707,512。
- **FLEngine の実行の停止点は変わらない**（a4 survey。変更後の生成 C を 17 個に分割（`split_c.py` 3700 関数ずつ）→ `build_native.sh` → `a4-survey.sh`。OUT は `lift/out-w6`、絶対パス。runtime の最終版で再リンクして再測定）: 停止は `LoadLibraryW "dsp_ippv2_x64.dll"`、init_count 145 / 942、最後の unit は index 144、run_rank 132 / 895、**関数実行数（distinct）821、エントリ 15,685**（決定 0077 と同じ）。6 つの thunk は、FLEngine の実行ではまだ呼ばれていない。

## 5. テスト

- 新規 `lift/crates/fe-lift/tests/k32_dsp.rs`（14 件）。全 `runtime/fle_*.c` とリンクして clang で native 実行（生成 C と、合成 PE の `call [IAT]`）:
  - 33 名すべてに行が 1 行ずつあり、`fle_imp_*` が runtime のヘッダで宣言されている。TlsFree と IsProcessorFeaturePresent に行が無い。実 DLL があれば、33 名の `call_imp` / `jmp_imp` が各 1 以上、未応答が上の 5 と 1 だけであること（`lift/out-ext` は gitignore。無ければ何もせず通る）。
  - InitializeCriticalSectionAndSpinCount: 戻り値 1、Enter/Leave/Delete が動く、スピン数 0 と 0xffffffff、削除後の再初期化。生きた領域の再初期化、InitializeCriticalSection で作った領域への重ねがけ、NULL は停止。
  - SList: ヘッダのバイトが変わらない、Flush は NULL、再初期化後も NULL。未初期化の Flush と NULL の初期化は停止。
  - Fls: 索引が異なる、初期値 0、値の往復、他の索引に影響しない、範囲外の Set は 0 と 87、解放が 1、解放しても他の値は残る。未割り当て、二重解放、解放後の Get/Set は停止。コールバックと値があるときの Free は停止、値が NULL かコールバックが無いときは 1。
  - GetCommandLineA が GetCommandLineW と同じ ASCII 文字列（長さ、全文字）で、繰り返し呼んで同じポインタ。
  - 環境ブロック: 空で NUL のみ、`FLE_A=x` と `B=yz` を足した後は 2 項目と終端 NUL で 14 ユニット。Free は 1、ほかのポインタと二重解放は停止。
  - GetOEMCP 437、GetACP 1252、IsValidCodePage 1252 と 437 は 1、65001、932、0、99999、0xffffffff は停止。
  - IsProcessorFeaturePresent: PF 19 通りの値（上の表）を、同じ runtime の CPUID（葉 1 の EDX、ECX、MMX と TSC と CMPXCHG8B が 0、葉 7 が葉 1 と同じデータ）と並べて確かめる。表に無い 12 個の番号は `feature <n>:` を含む名前付き停止。
  - 停止する 21 名は、`fle: <API>` が stderr に出て SIGABRT になり、呼び出しの後ろの printf が実行されない。
  - 合成 PE の `call [IAT]`（ICSASC、FlsAlloc、GetOEMCP の 3 つ）が `call_imp:*` になり、続けて走って R[0] が 437、`rsp` が保たれる。`call [RtlCaptureContext]` は `fle: RtlCaptureContext:` で SIGABRT。
  - 追加した C のソースがホストの状態（`getenv(`、`getpid`、`sysconf`、`/proc`、`cpuid`、`clock_gettime`、`fopen`、`open(`）を使わない。
- **ホストの実 API との突合**: ホストの Windows API は無い。参考として、Wine 10.0（`wine64`、mingw-w64 でビルドした `lift/out-w6/probe/pf.c`、`k32.c`）で同じ呼び出しを実行した（**Wine は Windows ではなく、根拠にしていない**。「非公式・参考」）。一致: InitializeCriticalSectionAndSpinCount が 1、Enter/Leave/Delete、FlsAlloc の 2 つの索引が異なる、初期値 0、Set/Get の往復、FlsFree が 1、InterlockedFlushSList が空で 0、GetEnvironmentStringsW が非 NULL で `FLE_A=x` を含む、FreeEnvironmentStringsW が 1、TlsFree が 1 と、再度 0 と 87、IsValidCodePage の 1252、437、65001 が 1 で 99999 が 0。**違い**: Wine の IsProcessorFeaturePresent は PF 2、3、8、13、14、17、36〜40 を 1 と答える（この環境は CPUID に合わせて 0。上の表のとおり実機の x64 Windows とも違う）。Wine は FlsSetValue の範囲外の索引で last error を設定しない（0）、この実装は Learn の ERROR_INVALID_PARAMETER 87。Wine の GetACP と GetOEMCP は 932（ホストのロケール）で、437 とは比べられない。
- 全体: `cd lift && CARGO_TARGET_DIR=…/lift/out-w6/target FLENGINE_PE=…/binaries/win64/FLEngine_x64.dll cargo test --workspace --no-fail-fast`。1 回目は 1 件失敗（`sysinfo.rs` の `source_uses_no_host_processor_memory_time_or_file_state`）。原因は**この作業**の `IsProcessorFeaturePresent` の停止メッセージに書いた英単語 `environment` が、そのテストの禁止語 `environ` に当たったこと。メッセージの文言を直した（テストは直していない）。2 回目は **1,090 件すべて成功、失敗 0、94 結果**（バイナリ 91 + doc-tests 3）。依頼文の基準（90 バイナリ・1,050 件）との差は、`k32_dsp.rs` の 14 件と、この作業の外で他の作業者が足したテスト（`gs_stacklimit.rs` など）。
- 補助スクリプト（読み取り専用。すべて `lift/out-w6/`）: `k32_sites.py`（IAT スロットの参照点）、`k32_rawscan.py`（バイト走査）、`k32_paths.py`（DllMain とエクスポートからの直接 call/jmp の到達）、`k32_attach.py`（ATTACH だけ）、`k32_xrefs.py`（関数の参照元）、`k32_hooktab.py`、`k32_data.py`、`learn_text.py`（Learn の HTML から本文）。

## 6. 仮の決定である点

- 到達の分類は**読みで、DLL を実行していない**。特に、(a) 初期化表の項目が ATTACH で全部走るか、(b) GetFileType の飛ばし条件（GetStdHandle が NULL、cbReserved2 が 0）、(c) mode 1 のとき argv のワイルドカード展開と `_recalloc` に入らない、は読み。
- 索引の払い出し順と再利用しないこと、索引の上限（FLS 4096）、TlsFree の失敗コード 87、FlsGetValue/FlsFree の失敗の扱い（停止）は、ページに値が無いための選択。
- IsProcessorFeaturePresent の 0 と 1 は、CPUID の応答（決定 0009、0047）とこの環境の OS（決定 0014）からの導出で、実機の x64 Windows の答えとは違う。
- IsValidCodePage は 1252 と 437 だけ 1（決定 0016 のコードページ）。
- ホストは Linux。Windows 本体との照合はしていない。Wine の結果は根拠にしていない。

## 7. 撤回条件

- 実機 Windows（決定 0014 の Windows 10 22H2）で、この記録の値（IsProcessorFeaturePresent の 0 または 1、FlsSetValue の失敗コード、TlsFree の 87 など）が違うと確認されたとき: 該当の値を直し、決定を改める。
- Learn のページが、停止にした入力の値を書き足したとき: 該当の停止を値に置き換える。
- IsProcessorFeaturePresent を CPUID と切り離して実機と同じ値にする判断を PM がしたとき: 表の 0 を 1 に改める（その場合は決定 0009、0047 との関係を先に決める）。
- 到達の読みが実行で覆ったとき（例: ATTACH が GetFileType、HeapReAlloc、FindFirstFileExW を呼ぶと分かったとき）: その名前を実装に切り替える。

## 8. 未確認・限界

- **ATTACH は完走しない見込み**。読みでは、IsValidCodePage（1252 → 1）と GetCPInfo の先で、`0x17d674c` が 256 バイトを `MultiByteToWideChar` に渡し（0x80 以上のバイトを含む）、既存の `fle_locale.c` が「0x80 以上のバイトは表が無い」と停止する。これは既存の決定 0016 の限界で、この作業では直していない（編集してよい範囲の外。ページに無い表が要る）。その先の GetStringTypeW も名前付き停止にしてある。実行して確かめていない。
- InitializeCriticalSectionAndSpinCount と FlsAlloc などは、API セットの `LoadLibraryExW`（決定 0081）が非 NULL を返す場合、`GetProcAddress(api-ms-…)` が NULL を返し、DLL は `Tls*` と `InitializeCriticalSectionAndSpinCount` の IAT 側に落ちる（調査 §7）。この記録の FLS の行は、その場合でも FlsSetValue などの `jmp` のスタブ（0x17d7558〜）が呼ばれたときの答えになる。
- DETACH（`FreeLibrary` でゲストが DLL を解放したとき）は、GetModuleHandleExW で止まる。
- `tests/call_iat_dll.rs` の合計 138 と `answered_total + unanswered` の表明は変えていない。応答数（84 → 133）は表に出る。
- 他者が同時に編集した `fe-decode` / `fe-lift` の間接 jmp（決定 0079）、`fe-pe` の変更（決定 0080）は、この作業と無関係（触っていない）。
