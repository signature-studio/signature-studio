# dsp_ippv2_x64.dll の `gs:` 参照（StackLimit / PEB）に要るモデルと、仕様から言えること（調査）

実行日: 2026-10-23。HEAD 6e1bc06。調査のみ。**値は決めていない**（PM が決める）。`lift/` の既存ファイル、runtime、生成器、テストは変更していない。作業物は `lift/out-teb/`（§9）。前提の文書: `docs/investigations/dsp-ippv2-lift-feasibility.md` §7、決定 0032（`gs:[0x30]` と `gs:[0x58]`）、決定 0073。

## 0. 凡例

- **実測** = コマンドの出力。`lift/out-teb/` の同名ファイルに残してある。
- **読み** = capstone による静的な逆アセンブルの読み。DLL を実行して確かめていない。
- **導出** = 一次資料の記述から計算で得た値（一次資料に値そのものは書かれていない）。
- **非公式・仮** = ReactOS、Wine、Vergilius、Wikipedia など Microsoft 以外の出典。
- **未確認** = 確認していない。
- 入力は `lift/out-ext/dsp_ippv2_x64.dll`（ImageBase 0x180000000）。逆アセンブルは既存の `lift/out-ipp/ipp_disasm_range.py`（読み取り専用）で行った。以下のアドレスは RVA ではなく VA の下位（0x17daba0 は RVA 0x17daba0）。
- 生成器の変更や runtime の変更はしていない。ビルドが要るものは `git archive HEAD` を `lift/out-teb/stage` に展開したコピーの runtime ヘッダだけを使った。

## 1. 閉包内の `gs:` 参照（実測）

`.text` と `IPPCODE` の全 0x65 バイト（GS 接頭辞）をデコードし、基底・添字なしで変位が 0x2000 未満のものを抜いた（`teb_gs_scan3.py`、出力 `gs-sites-small.txt`）。結果は 5 件で、feasibility §7 と一致する。この条件以外の `gs:` 形（添字付きなど）は調べていない。

| 位置 | 命令 | 所在関数 | 現行生成器 |
|---|---|---|---|
| 0x17cb325 | `mov rax, gs:[0x30]` | 0x17cb318 | 対応済み（`fle_gs_ld8`） |
| 0x17d06f7 | `mov rax, gs:[0x30]` | 0x17d06e4（`.pdata` に無い葉） | 対応済み |
| 0x17d1a96 | `mov rcx, gs:[0x60]` | 0x17d1a88 | 未対応（`fle_unsupported`） |
| 0x17d573c | `mov rax, gs:[0x60]` | 0x17d5730 | 未対応 |
| 0x17dabbc | `mov r11, gs:[0x10]` | 0x17daba0（`__chkstk`） | 未対応 |

- `out-ipp/ipp-pdata.c` で 0x17cb325 と 0x17d06f7 は `fle_gs_ld8(0x30)`、他の 3 件は `fle_unsupported(...)`（実測）。生成器は `vex.rs` の `mov_seg` が `off == 0x30 || off == 0x58` のときだけ通す（読み。`vex.rs` 142 行目）。
- 注意: `.pdata` を舐める方式の走査（`teb_gs_scan.py`）は 0x17d06f7 を取りこぼした。0x17d06e4 は `.pdata` に項目が無い葉関数のため。バイト走査（`teb_gs_scan3.py`）の結果を採る。
- 比較: `FLEngine_x64.dll` の同じ走査は `gs:[0x58]`（0x1d14b）が 1 件。ほかに `gs:[0]` 形が 2 件（0x6364d9、0x636509。`gs-sites-flengine.txt`）出るが、命令の途中から読んだ偽陽性かどうかは確認していない（決定 0032 は閉包の gs を 1 件としている）。

## 2. `__chkstk`（0x17daba0）

### 2.1 全命令（実測。`chkstk-17daba0.txt`）

```
0x17daba0: sub  rsp, 0x10
0x17daba4: mov  [rsp], r10
0x17daba8: mov  [rsp+8], r11
0x17dabad: xor  r11, r11
0x17dabb0: lea  r10, [rsp+0x18]       ; 入口の rsp は戻り番地の位置。0x10 引いた後の +0x18 は call 命令の時点の rsp
0x17dabb5: sub  r10, rax              ; rax = 確保サイズ。r10 = 新しい rsp の候補
0x17dabb8: cmovb r10, r11             ; 借りが出たら 0
0x17dabbc: mov  r11, gs:[0x10]        ; StackLimit
0x17dabc5: cmp  r10, r11
0x17dabc8: jae  0x17dabe0             ; r10 >= StackLimit なら何も触らない
0x17dabca: and  r10w, 0xf000          ; r10 の下位 16 ビットだけに作用。下位 12 ビットを 0 にする
0x17dabd0: lea  r11, [r11 - 0x1000]
0x17dabd7: mov  byte [r11], 0         ; 1 バイトのゼロ書き込み
0x17dabdb: cmp  r10, r11
0x17dabde: jne  0x17dabd0
0x17dabe0: mov  r10, [rsp]            ; r10, r11 を戻す
0x17dabe4: mov  r11, [rsp+8]
0x17dabe9: add  rsp, 0x10
0x17dabed: ret
```

### 2.2 何をするか（読み。実測で裏を取った）

- `[StackLimit − 0x1000]`、`[StackLimit − 0x2000]` … と、**StackLimit より下**のページを 1 バイトずつゼロで書き、目標ページ（新しい rsp を 4 KiB に切り下げた位置）を書いたら終わる。書くのは「StackLimit と目標の間」で、**StackLimit 以上は触らない**。
- 触るページ数は `(StackLimit − 目標ページ) / 0x1000`。**目標 ≥ StackLimit なら 0 回**。
- ループの終了条件は `r11 == r10`（等しいときだけ）。StackLimit と目標ページの差が 0x1000 の倍数でないと、**ループは終わらず**、アドレスが下がり続けて fault する（実測。下の §6 の「+1」「+0x800」「+0x3001」）。
- `rax`、`r10`、`r11`、`rsp` は呼び出し前後で同じ。レジスタ以外で変わるのは、メモリへのゼロ書き込みと、ステータスフラグ（最後の `add rsp, 0x10` の結果。呼び出し側が読むかは未確認）。
- 呼び出し元は 150 か所（`callers-peb-chkstk.txt`）。`call` の直後は 149 か所が `sub rsp, rax|imm` または `lea rsp, [rsp-imm]`、1 か所が `sub rsp, rcx`（`chkstk-sites.txt`）。`rax` の作り方は `mov eax, imm` が 51 か所（0x1118〜0xa0b0、最大 41,136 B）、`and rax, -16`（`alloca` / `_malloca` 形）が 98 か所、`mov rax, rcx` が 1 か所。
- 動的な 98 か所のうち、直前 14 命令に `cmp 即値; jcc` のサイズ上限があるのは 1 か所（0x17d877f の `cmp rcx, 0x400; ja`。`_malloca` の形）。**残り 97 か所の上限は読んでいない**（`chkstk-dyn-bound.txt`。この走査は上限の有無の粗い手がかりで、検査対象のレジスタまでは確かめていない）。
- Learn の `_chkstk Routine` ページ（https://learn.microsoft.com/en-us/windows/win32/devnotes/-win32-chkstk）: 「Called by the compiler when you have more than one page of local variables in your function.」「for x64 compilers it is 8K」。この DLL の `mov eax, imm` の最小は 0x1118（4,376 B）で、8K より小さい。**ページの数値とこの DLL の実測は合わない**（理由は未確認。コンパイラの版か、ページの記述の古さか）。

### 2.3 到達（読み）

- DllMain の直接 call/jmp の静的な到達集合（`out-ipp/reach-dllmain-c-funcs.txt` の 230 関数）に、`__chkstk` を呼ぶ関数は 5 つある（`chkstk-reach.txt`）: 0x17d4540、0x17d4644、0x17d4760（0x17d49f8 から。標準出力系）、0x17d86c0、0x17d9a94。
- 最短経路（`reach-paths.txt`）: 0x17d86c0 と 0x17d9a94 は `0x17d9e4c > 0x17d6c5c > 0x17d6934 > 0x17d6cd8 > 0x17d674c > …`。0x17d9e4c は `.rdata` の初期化関数表のスロット（0x1ebd298）が指す関数で、その表は DLL_PROCESS_ATTACH の中で 0x17caa4d が `0x1ebd280..0x1ebd2a8` を回して呼ぶ（`initterm-slots.txt`、`procattach-17ca9d8.txt`）。したがって **ATTACH の途中で `__chkstk` に届く経路が静的にはある**。その経路の条件分岐（0x17d6c5c の `cmp byte [0x2561e54], 0` など）は読み切っていない。実際に呼ばれるかは未確認。
- 残る 3 関数（0x17d4540 系）は DLL_PROCESS_DETACH 側（0x17cb618 経由）の経路。
- `__chkstk` が 1 度でも実行されると、現行の生成器は `fle_unsupported(0x1817dabbc)` で止まる（実測: 生成 C）。

## 3. PEB を読む 2 関数（読み。`peb-17d1a96.txt`、`peb-17d573c.txt`、`peb-17d74bc.txt`）

### 3.1 0x17d5730（`gs:[0x60]` → `[PEB+0x20]` → `[+8]`）

```
rbx = 0; [rsp+0x30] = 0
rax = gs:[0x60]                 ; PEB
rcx = [rax + 0x20]              ; PEB の 0x20
if (int32)[rcx + 8] < 0  goto ret1
call 0x17d74bc(&[rsp+0x30])     ; AppPolicyGetProcessTerminationMethod(-6, &local) を引いて呼ぶ形
if [rsp+0x30] == 1  goto ret0
ret1: return 1       ret0: return 0
```

- 0x17d74bc は 0x17d72fc で名前 `AppPolicyGetProcessTerminationMethod`（0x1ec9620 の文字列。実測 `rva-strings.txt`）の関数を引き、見つかれば `jmp` で呼ぶ。見つからなければ `eax = 0xc0000225` を返し、`local` は 0 のまま（`[rsp+0x30]` は冒頭で 0）。**つまり関数が引けない場合は `local != 1` となり 0x17d5730 は 1 を返す**。
- Learn（https://learn.microsoft.com/en-us/windows/win32/api/appmodel/ne-appmodel-apppolicyprocessterminationmethod）: `AppPolicyProcessTerminationMethod_ExitProcess`「Allows DLLs to execute code at shutdown. This value is expected for a desktop application」、`..._TerminateProcess`「Immediately ends the process. This value is expected for a UWP app.」。列挙値の数値は Learn の列挙の順（ExitProcess が先）から 0 と 1 と読めるが、数値は書かれていない（導出）。

### 3.2 0x17d1a88（`gs:[0x60]` → `[PEB+0xbc]`）

```
call 0x17d5730
if eax == 1  goto ret0          ; 戻り値 0
rcx = gs:[0x60]
edx = [rcx + 0xbc]; edx >>= 8
if (dl & 1) goto ret0 else ret1  ; ビット 8 が立っていれば 0、立っていなければ 1
```

- **`[PEB+0xbc]` を読むのは 0x17d5730 が 1 以外（= 0）を返したときだけ**。0 を返す条件は「`[ProcessParameters+8]` が負でない」かつ「AppPolicy の結果がちょうど 1（UWP の終了方式）」。デスクトップ app として答える（結果 0）か、関数が引けない場合は、`+0xbc` は読まれない（読み）。
- 呼び出し元は 0x17d1a54 だけ（`callers-17d1a54.txt` と §3.3）。0x17d1a54 は `0x17d1a88` が 0 でなければ `GetCurrentProcess` → `TerminateProcess`、その後 0x17d1ab8（`mscoree.dll` の `CorExitProcess` を引く）、最後に `ExitProcess`（`peb-17d1a54.txt`）。

### 3.3 どこから届くか（読み）

- 0x17d1a54 の呼び出し元は 0x17d198c（共通の終了処理）の中の 1 か所だけ。0x17d198c への jmp は 0x17d1b30 と 0x17d1b40 の 2 つ（`ptr-refs.txt` で 0x17d198c を指す間接参照は 0 件）。
- 0x17d1b30 ← 0x17cb440 ← 0x17caaf0 の中の 0x17cab30（`callers-exit2.txt`、`detach-17caaf0.txt`）。0x17caaf0 はアタッチ回数を 1 減らし、状態変数 `[0x2561478] == 2` のときに 0x17cb440 を呼ぶ形（DLL_PROCESS_DETACH 側）。0x17cb440 は先頭で 0x17cb9cc（`[0x2562e40] != 0`）を呼ぶ。
- 0x17d1b40 ← 0x17d25a0（致命エラーの共通関数。直接 call 35 か所。`int 0x29` を持つ）と 0x17d7bbf（`callers-exit.txt`、`exit-17d25a0.txt`）。
- **PEB の読み取りは「この DLL がプロセスを終わらせようとする経路」でだけ起きる**（読み）。そこでは直後に `TerminateProcess` / `ExitProcess` が呼ばれ、runtime はどちらも名前付きで止める（`fle_proc.c` 211〜229 行目の `fle_imp_ExitProcess` / `fle_imp_TerminateProcess` が `fle_stop`。決定 0020）。
- したがって、その経路に実行が届いた時点で、結果として実行は止まる。今は PEB の読みが「`unsupported instruction at 0x1817d1a96`」で先に止めるので、停止の理由の表示が「プロセス終了の要求」ではなく「未対応の命令」になる（読み）。

### 3.4 `gs:[0x30]` の 2 件の到達（読み）

- 0x17cb318（0x17cb325）: 先頭で 0x17cb9cc を呼び、結果 0 なら `gs` を読まずに戻る。0x17cb9cc は `[0x2562e40] != 0` を返す。`[0x2562e40]` を参照する命令は `.text` と `IPPCODE` の rip 相対で 0x17cb9ce の `cmp` の 1 件だけ（`refs-2562e40-b.txt`）、絶対 VA を 8 バイトで持つ箇所も 0 件（`ptr-refs-flags.txt`）で、`.data` のファイルに実体の無い（ゼロ初期化）領域にある。**この関数の `gs:[0x30]` は、単一イメージ・単一スレッドの前提では実行されない**（読み。書き込みが間接に起きる可能性は排除し切れていない）。
- 0x17d06e4（0x17d06f7）: DllMain の静的な到達集合に入らない（`reach-paths.txt` の「not reached」）。呼び出し元は C++ 例外処理系の 5 関数（0x17cce58、0x17cf084 ほか。`callers-17d06e4.txt`）。読む内容は `[TEB+0x10]` と `[TEB+8]` と、`[rcx+0x98]`（x64 の CONTEXT の Rsp の位置）との比較で、範囲外なら `int 0x29`（`gs30-17d06f7.txt`）。**`gs:[0x30]` 経由で TEB の 0x8 と 0x10 が読まれる経路が存在する**。

## 4. 一次資料で言えること、言えないこと

URL は 2026-10-23 に取得した。原文は引用の範囲で示す。

| 項目 | 一次の記述 | 区分 |
|---|---|---|
| TEB の `ProcessEnvironmentBlock` の位置 | https://learn.microsoft.com/en-us/windows/win32/api/winternl/ns-winternl-teb の構造体: 「`PVOID Reserved1[12]; PPEB ProcessEnvironmentBlock;`」。x64 で `PVOID` は 8 バイトなので 12×8 = **0x60**。ページに数値の記載は無い | 導出（`gs:[0x60]` = PEB のポインタ） |
| TEB の先頭 0x38 バイトが NT_TIB である並び | Learn の「Debugging a Stack Overflow」（https://learn.microsoft.com/en-us/windows-hardware/drivers/debugger/debugging-a-stack-overflow）の `!teb` 出力の順: 「ExceptionList、StackBase、StackLimit、SubSystemTib、FiberData、ArbitraryUserPointer、Self」。x64 の `dx` 出力（https://learn.microsoft.com/en-us/windows-hardware/drivers/debuggercmds/dx--display-visualizer-variables-）に「[+0x038] EnvironmentPointer」「[+0x058] ThreadLocalStoragePointer」「[+0x060] ProcessEnvironmentBlock」。7 個のポインタが 0x38 に収まる並びから ExceptionList 0x0、StackBase 0x8、**StackLimit 0x10**、SubSystemTib 0x18、FiberData 0x20、ArbitraryUserPointer 0x28、Self 0x30 | 導出（Learn のデバッガ資料の並びと x64 のポインタ幅から） |
| NT_TIB の専用ページ | `winnt/ns-winnt-nt_tib`、`ns-winnt-_nt_tib`、`wdm/ns-wdm-_nt_tib` は取得時に 404（試した URL）。検索でも Learn の NT_TIB の構造体ページは見つからなかった | **一次に無い**（決定 0032 は Self を mingw の `NT_TIB64` で補っていた。上の導出で Learn 側から並びは出せる） |
| StackLimit の意味 | 上の `!teb` 例は StackBase 0x01100000、StackLimit 0x01001000、「`?? int(@$teb->NtTib.StackBase) - int(@$teb->NtTib.StackLimit)` → `int 0n1044480`」。同ページ: 「The maximum stack size is stored in the field DeallocationStack」。別ページ（https://learn.microsoft.com/en-us/windows/win32/procthread/thread-stack-size）: 「The system commits additional pages from the reserved stack memory as they are needed, until either the stack reaches the reserved size minus one page (which is used as a guard page to prevent stack overflow)…」 | 例の値と、ガードページの存在は一次。「StackLimit = 現在コミット済みの下端」という定義の文は一次に無い（**未確認**。非公式の説明は §4.1） |
| `GetCurrentThreadStackLimits` | https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getcurrentthreadstacklimits: 「`LowLimit` A pointer variable that receives the lower boundary of the current thread stack.」「Callers can use … to verify that the current stack pointer is within the returned limits.」 | 一次。ただし TEB+0x10 との対応は書かれていない |
| x64 のスタック | https://learn.microsoft.com/en-us/cpp/build/stack-usage?view=msvc-170: 「All memory beyond the current address of RSP is considered volatile」 | 一次。`__chkstk` が rsp より下に書く点と整合 |
| PEB の `ProcessParameters` | https://learn.microsoft.com/en-us/windows/win32/api/winternl/ns-winternl-peb の「64-bit Windows」の Remarks の構造体: 「`BYTE Reserved1[2]; BYTE BeingDebugged; BYTE Reserved2[21]; PPEB_LDR_DATA LoaderData; PRTL_USER_PROCESS_PARAMETERS ProcessParameters;`」。2+1+21 = 24 = 0x18 に `LoaderData`、続く **0x20 が `ProcessParameters`** | 導出（一次の宣言から） |
| `ProcessParameters` の +8 | https://learn.microsoft.com/en-us/windows/win32/api/winternl/ns-winternl-rtl_user_process_parameters: 「`BYTE Reserved1[16];`」「Reserved for internal use by the operating system.」。+8 はこの `Reserved1` の中 | **一次に無い**（意味も値も） |
| PEB の `NtGlobalFlag`（+0xbc） | PEB のページに `NtGlobalFlag` は載っていない（64 ビットの Remarks は `Reserved3[520]` で 0x2a〜0x230 を覆う） | **一次に無い** |
| `NtGlobalFlag` のビット 0x100 | GFlags のフラグ表（https://learn.microsoft.com/en-us/windows-hardware/drivers/debugger/gflags-flag-table）: 「Enable application verifier \| FLG_APPLICATION_VERIFIER \| 0x0100 \| vrf \| R,K,I」。GFlags の概要（…/gflags-overview）: レジストリの `GlobalFlag`、イメージごとの設定で有効にする。同表の注: 「The symbolic name of each flag is provided for reference only」 | 0x100 のフラグの存在は一次。それが PEB の +0xbc に入るという対応と、既定値は一次に無い |
| `__chkstk` の呼ぶ条件 | §2.2 の引用 | 一次（ただし数値が実測と合わない） |
| wasm のメモリ | https://webassembly.github.io/spec/core/exec/instructions.html の load / store: 「If i + ao.offset + N/8 > \|z.mems[x].bytes\|, then: Trap.」。ページの保護やガードの概念は書かれていない | 一次。**wasm の線形メモリは範囲内なら何を書いても通り、ガードページに当たって止まる仕組みは仕様に無い** |

### 4.1 非公式・仮（補足）

- Wikipedia の Win32 Thread Information Block（https://en.wikipedia.org/wiki/Win32_Thread_Information_Block。2026-08-10 付の版を検索結果で確認。本文全体は取得していない）の表: GS:[0x08] Stack Base（高アドレス）、GS:[0x10] Stack Limit（低アドレス）、GS:[0x30] TEB の線形アドレス、GS:[0x60] PEB。「Stack Limit / Ceiling of stack (low address)」。同ページは StackLimit と DeallocationStack（0x1478）の差を「ガードページ分」とする。**非公式・仮**。
- Vergilius Project の x64 `_PEB`（https://www.vergiliusproject.com/kernels/x64/windows-10/22h2/_PEB。Windows 10 22H2）: ProcessParameters 0x20、`NumberOfProcessors` 0xb8、**`NtGlobalFlag` 0xbc**。Wine の `winternl.h`（https://raw.githubusercontent.com/wine-mirror/wine/master/include/winternl.h）の `PEB`: 「`ULONG NtGlobalFlag; /* 068/0bc */`」。**非公式・仮**。
- Wine の `RTL_USER_PROCESS_PARAMETERS`: 先頭が `ULONG AllocationSize; ULONG Size; ULONG Flags; …` なので +8 は `Flags`（**非公式・仮**）。この DLL が `Flags` の符号を見る意味は未確認。
- 以上から、DLL が読む `[PEB+0xbc]` は NtGlobalFlag、そのビット 8 は FLG_APPLICATION_VERIFIER、という読みになるが、**対応は非公式の出典に依る**（一次に無い）。

## 5. 現行 FLEngine 側のモデルとゲストのスタック

### 5.1 TEB / TLS（読み）

- 決定 0032 と `runtime/fle_vex.h` 85〜110 行目: ホスト側のバッファ `fle_teb[0x2000]`（weak、0 初期化）。`fle_gs_ld8(off)` は呼ぶたびに Self（0x30）と TLS 配列の先頭（0x58）を `fle_teb` に書き込み、`off` が 0x30 か 0x58 ならその値を返す。**それ以外のオフセットは `fle: GS:[...] is not modeled` で `abort()`**。0x8（StackBase）、0x10（StackLimit）、0x60 は `fle_teb` の中でゼロのまま。
- 生成器側は `vex.rs` の `mov_seg` が 0x30 と 0x58 以外を生成時に `Err`（= `fle_unsupported`）にする（二重の fail-closed）。
- TLS 配列 `fle_tls_values[]`（`fle_imports.c` 69 行目）は 1 スレッド用の TlsAlloc 表。TlsSlots（TEB+0x1480）は `fle_teb` の範囲内（0x2000）だがモデルは無い。
- **Self ポインタ経由で `fle_teb` を直接読める**（0x30 の値が `fle_teb` のアドレス）。したがって `gs:[0x10]` を別の変数で答えると、`gs:[0x30]` 経由で `[Self+0x10]` を読んだ値と食い違い得る（§3.4 の 0x17d06f7 がその読み方）。決定 0032 が 0x30 と 0x58 を `fle_teb` の同じ位置にも書いたのと同じ理由で、**StackBase（0x8）、StackLimit（0x10）、PEB（0x60）も 1 つのバッファに置くのが整合する**（読み）。
- `fle_teb` は静的変数なので、wasm ではモジュールのデータ領域（`GLOBAL_BASE` 以上）に入り、`FLE_CHECKED_MEM` の領域検査を通る（決定 0045）。

### 5.2 ゲストのスタック（実測と読み）

| 場面 | 領域 | 出典 |
|---|---|---|
| native の boot | `mmap(NULL, 16 MiB, PROT_READ\|PROT_WRITE, MAP_PRIVATE\|MAP_ANONYMOUS\|MAP_NORESERVE)`。ガードページなし。`R[4] = (base + 16 MiB − 64) & ~15`、その後 `R[4] -= 8` で番兵 0xdead0000 | `harness/main.c` 55〜72 行目、`boot.h` の `HARNESS_STACK_BYTES`（読み） |
| wasm の boot | `malloc(16 MiB)`、`R[4]` の計算は native と同じ | `harness/wasm/main.c` 48〜65 行目（読み） |
| `fle_call_guest` | **専用のスタックは持たない**。呼び出し時の `R[4]` から `(R[4] − 32) & ~15` を呼び出し点の rsp とし、同じスタックを下へ使う。入れ子は 16 段まで（`FLE_CB_MAX_DEPTH`、決定 0064） | `fle_callback.c` 23〜70 行目（読み） |
| 「unit のスタック」 | 専用のものは見つからなかった。`R[4] =` を代入する箇所は harness の main 2 つ、`fle_callback.c`、`fle_win.c`、`fle_monitor.c`、`fle_ole.c`（C のローカル配列を一時的に `R[4]` にする）、オラクルのドライバだけ（grep）。unit は boot のスタックの上で回ると読める | grep（読み） |
| オラクル（別系統のテスト用） | `OR_STACK` 0x30000000、サイズ 0x100000、`OR_STACK_TOP` は +0xC0000 | `harness/oracle/oracle_common.h`（読み） |
| wasm の C スタック | `-sSTACK_SIZE=8MB`（emscripten の C 側のスタック）。**ゲストの `R[4]` のスタックとは別物** | `harness/wasm/build.sh` 92 行目（読み） |

- native の mmap は下端がページ境界。wasm の `malloc` の戻りは、ホストの glibc で 16 MiB を確保した実測が下位 12 ビット 0x10（`malloc_align.c`）。**wasm（emscripten の malloc）の下端がページ境界かは未確認**。境界でなければ §2.2 のとおり、StackLimit を下端にした場合に「触る必要が出たとき」ループが終わらない。
- どちらの harness も、スタックの**下**に何があるか（別の mmap、別の `malloc` ブロック）を保証しない。native でも「ガードページ」は無い。

## 6. StackLimit の値ごとの `__chkstk` の挙動（実測）

測り方（2 つ）:

1. `chkstk_probe.c`: 生成器が出した `__chkstk` の C（`out-ipp/ipp-pdata.c` の 5051513〜5051531 行目）を `chkstk_lifted.inc` に写し、**`gs:[0x10]` の 1 行だけ**を変数の読み出しに替えた。ゲストと同じ形の 16 MiB のスタックの上で動かし、書き込みを記録して、範囲外の書き込みは拒否して報告する（`chkstk-probe-run.txt`）。
2. `chkstk_oracle.c`: DLL の元のバイト列（0x17daba0〜0x17dabed）をホスト CPU でそのまま実行し（`arch_prctl(ARCH_SET_GS)` で偽の TEB を指し、+0x10 に StackLimit を置く）、同じ状態の生成 C と突き合わせる。r10、r11、rsp、6 個のステータスフラグ、領域のメモリ（戻り番地の 8 バイト以外）を比較。**14 ケース全部で一致**（`chkstk-oracle-run.txt`）。ケース: 確保量 0、0x300、0x800、0x1800、0x2000、0x3000、借りが出る巨大値、限界の手前・上・同ページ・同ページ内、整列していない限界（+1、+0x800）、領域の下端を超える確保。

これは**ループのモデル（生成 C）が元のバイト列と同じ挙動をする**ことの確認で、StackLimit に何を置くべきかの根拠ではない。

### 6.1 結果（`chkstk-probe-run.txt`。stack = 16 MiB、rsp は上端 − 64）

| StackLimit | 確保量 | 結果 |
|---|---|---|
| 下端（ページ境界） | 0 〜 0x100000（rsp が上端付近） | 触るページ 0。何も書かずに戻る |
| 下端 + 1 / + 0x800 | 同上 | 触るページ 0（目標 ≥ 限界なので） |
| 0、0x10000 | 同上（rsp が下端の 0x3000 上で確保 0x5000 でも） | **触るページ 0。常に何も書かない**（目標 ≥ 限界が常に成り立つ） |
| 下端 | rsp が下端の 0x3000 上、確保 0x5000（フレームが下端をまたぐ） | 下端のすぐ下（下端 − 0x1000）を書こうとして**範囲外**。実行側ではスタックの外への 1 バイトのゼロ書き込み |
| 下端 + 1 | 同上 | 範囲外（ページ境界でないアドレス）を書こうとする。以降もループは終わらない |
| 下端 + 0x2000 | 同上 | 2 ページ書いた後、範囲外 |
| 確保量 20 MiB（> スタック） | 下端 | 範囲外 |
| 0x7fffffffffff | どれでも | 最初の書き込みが範囲外（0x7fffffffefff）。即座に止まる／fault |
| rsp の 1 ページ上（`above-rsp`。限界が rsp より高い） | 0x1118 〜 0x100000 | 2、257 などのページ数を書いて戻る。この測定で書いたアドレスはすべて rsp 以下（実測）。限界が rsp より 1 ページ超高いと、rsp より上の（生きている）領域に 0 を書き得る（読み。実測していない） |

- 「範囲外」は probe が拒否した書き込みで、実行側の native では SIGSEGV（隣が未マップのとき）か、隣の領域への黙った書き込み（隣がマップ済みのとき）、wasm では**黙って成功**（隣は線形メモリの別のデータ）になる（読み。wasm 側は実行していない）。
- 触るバイトは 1 バイトのゼロで、フレームの**未使用領域**（目標ページ〜限界）に当たる。限界が rsp 以下であれば、生きているデータは壊さない（読み。実測の範囲は上の表）。

## 7. 選択肢（値は決めない）

共通の前提: どの案も `mov r11, gs:[0x10]` を生成器が通す変更が要る（案 C を除く）。生成器・runtime・harness は今回は変更していない。

| | 内容 | 値・動作の根拠 | native | wasm | テスト | 失敗時 |
|---|---|---|---|---|---|---|
| **A** | StackLimit（と StackBase）= ゲストのスタックの**実際の下端（と上端）**。`fle_teb` の +0x10、+0x8 に置き、`gs:[0x10]`、`gs:[0x30]` 経由のどちらでも同じ値が読める | 値は環境の事実（boot が確保した領域）。「下端」が StackLimit の意味と合うかは一次に無い（§4）が、Learn の `!teb` 例（StackBase 0x01100000 > StackLimit 0x01001000）と、`GetCurrentThreadStackLimits` の「lower boundary」の語とは整合（導出）。**仮**の部分: 下端をページに揃えるか、boot のスタックだけを正とするか | フレームが下端をまたがない限り触るページ 0（実測）。またぐと下端の下へ 1 バイトの 0（隣が未マップなら fault、そうでなければ黙って壊す）。boot が mmap のため下端はページ境界 | フレームが収まる限り同じ（実測）。またぐと**黙って隣の線形メモリに 0 を書く**。下端がページ境界でないと、またいだときループが終わらず走り続ける（読み）。現行 `FLE_CHECKED_MEM` はスタックを含む領域を通すので止めない | probe/oracle の流儀: 限界をスタック下端にして、収まる / またぐ / ページ未整列 / 確保 0 を native で比較。DLL を載せた実行では ATTACH の完走 | **fail-closed ではない**（またぐと黙って壊れるか、native で運が良ければ fault）。ただし現行の FLEngine は `__chkstk` の形を持たず（プロローグのバイト列の検索 0 件。`flengine-chkstk-pattern.txt`。ほかの形の確認はしていない）、`gs:[0x10]` も読まない（`gs-sites-flengine.txt`）ので、この穴は今は表に出ていない |
| **B** | 十分低い固定値（0、または 0x10000 など） | **値は創作**（一次にも環境の事実にもならない）。「StackLimit より下を触る」ループを起こさない、という効果だけが根拠 | 触るページ 0（実測）。**フレームが下端をまたいでも検知しない** | native と同じ | 限界を 0 にして `__chkstk` が戻ることと、フラグ・レジスタが保たれることを oracle 流で | fail-open（検知なし）。他の読み手（0x17d06f7 が StackBase と比べる範囲検査）に対しては、下側の検査が常に通る。StackBase を別途決めないと上側の検査（`rcx <= [TEB+8]`）が通らない（§3.4。読み） |
| **C** | `__chkstk`（VA 0x17daba0）を**名前付きの検査関数に置換**: 「`rsp − rax` がゲストのスタックの下端以上か」を見て、足りなければ `fle_stop`（名前付き）。TEB には依らない | 根拠は「`__chkstk` の目的は新しい rsp が確保済みの範囲に入ることの確認」（§2.2 の読み + Learn の `_chkstk` ページ）。`gs:[0x10]` は読まない | 同じ | 同じ。**wasm でも検知できる唯一の案**（`fle_st` のガードに頼らない） | 置換した関数を oracle 流で「収まる / またぐ」を比較。ただし DLL のバイト列を lift しない箇所ができる | **fail-closed**。ただし、検索した範囲では**生成器に VA 指定の置換・上書きの仕組みは見つからなかった**（`grep` で `override`、`replace` 系を検索して 0 件。網羅ではない）ので、新規の仕組み。置換は r10、r11、rax を保ち、フラグが `add rsp, 0x10` の結果になることまで合わせないと元と違う（呼び出し側が読むかは未確認） |
| **D** | A に加えて、native の boot のスタックの下にガード（`PROT_NONE`）を置く | 環境側の追加 | またぐと SIGSEGV（fail-closed） | **効かない**（ガードの概念が無い。§4） | native だけ追加でテスト | native のみ fail-closed。wasm との差が残る |

### 7.1 推奨（決めない）

- **A を、StackBase と StackLimit を 1 つの `fle_teb` に置く形で**勧める。理由: 値が環境の事実で、創作を含まない（B は値が創作）。`gs:[0x30]` 経由の直接の読みとも食い違わない（§5.1）。
- A だけでは「またぐと黙って壊す」が残る。その危険の大きさは、**動的な 97 か所の確保量の上限が未確認**（§2.2）で、静的な最大は 41 KiB、スタックは 16 MiB という比でしか言えない。深さ 16 の入れ子（`FLE_CB_MAX_DEPTH`）と合わせて、実行で届くかは未確認。
- wasm でも「またぐ」を止めたいなら C が要る。A と C は排他ではない（C を入れた上で `gs:[0x10]` も A で答える）。
- どの案でも `gs:[0x10]` を通す生成器の変更と、`StackBase`/`StackLimit` を boot が runtime に渡す手段（今は `fle_teb` にも harness にも無い）が要る。**これは新しい決定**。

## 8. PEB の参照（`[PEB+0x20]`、`[PEB+0xbc]`）

### 8.1 答えを決めずに済む可能性（読み）

- §3.3 のとおり、PEB を読む関数は**プロセス終了の経路**にだけある。`ExitProcess` / `TerminateProcess` は runtime が名前付きで止める（決定 0020）。ATTACH の直線上では届かない（読み）。
- したがって PEB の値を決めなくても、「終了の経路に届いたら止まる」ことは変わらない。違うのは停止の表示（今は `unsupported instruction at 0x1817d1a96` または `0x1817d573c`。§3.3）。
- ただし 0x17d25a0（致命エラー）から届く経路は、**ATTACH の途中でも通り得る**（DLL が不正な状態を検出したとき）。その場合も同じく終了で止まる。

### 8.2 経路に届いたときに要る最小のモデル

PEB を読む命令を通す（生成器と runtime の変更が要る）なら、必要なのは次だけ:

| 読み | 必要な値 | 仕様から言えるか |
|---|---|---|
| `gs:[0x60]` | PEB のアドレス（`fle_teb` の +0x60 にも同じ値を置く） | オフセットは導出（Learn の TEB 構造体。§4）。アドレスは環境の選択 |
| `[PEB+0x20]` | `ProcessParameters` のポインタ | オフセットは導出（Learn の PEB の 64 ビット構造体）。指す先は環境の選択 |
| `[ProcessParameters+8]`（4 バイト、符号を見る） | 負か否か | **一次に無い**（Reserved）。Wine の `Flags` なら非公式・仮。値は「0 = 負でない」を置くと次の AppPolicy の分岐に進む。これは創作 |
| AppPolicy の答え | 関数が引けない（`local == 0`）か、`ExitProcess` 相当（0）か、1 か | 「デスクトップ app では ExitProcess が期待値」は一次（§3.1）。関数を `api-ms-win-appmodel-runtime-l1-1-2` から引く経路を runtime が答えるかは未確認（`fle_modules[]` の規則に `api-ms-win-*` は無い。feasibility §6.1） |
| `[PEB+0xbc]` | 上の AppPolicy の答えが 1 のときだけ読まれる。ビット 8 を見る | `NtGlobalFlag` は一次に無い。ビット 0x100 が AppVerifier の意味は一次（§4）。既定の値は一次に無い |

- 読み取った 2 つの値（`+8` と `+0xbc`）のどちらも、**デスクトップとして答える設計では `+0xbc` は読まれない**（読み）。`+8` の符号だけは必ず読まれる。
- 0 で埋めた PEB と ProcessParameters を置く案は、`+8 = 0`（負でない）→ AppPolicy を引く経路に進む。**0 を置くのは「0 という値を決めた」ことになる**。決定 0066 は 0 を「書かずに置く場所を 0 にした」と区別して記録しているが、ここは分岐に使われるので同じ言い方はできない。
- 代替: PEB の読みを通さず現状のまま（未対応）にして、終了の経路に届いたら止まることを受け入れる。ATTACH の直線上では届かないので、A4 の測定には影響しない可能性が高い（読み）。

## 9. 分かったこと

### 実測
- 閉包の `gs:` 参照は 5 件（§1）。生成 C は 0x17d1a96、0x17d573c、0x17dabbc の 3 件が `fle_unsupported`。
- `__chkstk` の呼び出しは 150 か所、確保量の即値は最小 0x1118、最大 0xa0b0（§2.2）。
- 生成 C の `__chkstk` と元のバイト列は、14 ケースで r10、r11、rsp、フラグ、メモリが一致（`chkstk-oracle-run.txt`）。
- StackLimit を下端・0・0x10000 にすると、フレームが収まる限り触るページは 0。またぐと下端の下を書く（§6）。限界が整列していないと、またいだときループは終わらない。
- 一次資料: TEB の PEB ポインタ位置（導出）、PEB の ProcessParameters 位置（導出）、`AppPolicy...` の値の意味、GFlags の 0x100、wasm の load/store の trap 条件（§4）。

### 読み
- `__chkstk` は ATTACH の途中に静的な経路を持つ（初期化表の 0x17d9e4c 経由）。条件は読み切っていない。
- PEB を読む 2 関数は、プロセス終了の経路（DETACH 側の終了、致命エラー）にだけある。`+0xbc` は AppPolicy が UWP 型（1）のときだけ読まれる。
- 0x17cb318 の `gs:[0x30]` は `[0x2562e40]` が常に 0 のため実行されない。0x17d06e4 の `gs:[0x30]` は C++ 例外処理系で、DllMain の静的な到達の外。後者は TEB の 0x8 と 0x10 を読む。
- 動的な確保の 97 か所の上限は読んでいない。

### 未確認
- `__chkstk` が実際に ATTACH で呼ばれるか、何回か、確保量。
- wasm の `malloc` の下端がページ境界か。wasm で「またぐ」ときの実際の挙動（実行していない）。
- Learn の NT_TIB の専用ページ（見つからない）。「StackLimit = 現在コミット済みの下端」という一次の文。
- `_chkstk` の Learn の「x64 は 8K」と、実測の最小 0x1118 の食い違いの理由。
- `[ProcessParameters+8]` の意味（非公式では `Flags`）と、DLL が符号を見る理由。
- 呼び出し側が `__chkstk` の戻りのフラグを読むか。
- FLEngine の `gs:[0]` 2 件が偽陽性か。
- `api-ms-win-appmodel-runtime-l1-1-2` を runtime がどう答えるか。

### 仮説
- 単一イメージ・単一スレッドでは、`[0x2562e40]` は常に 0 のまま（書き込む命令が見つからない）。間接の書き込みは排除していない。
- ATTACH の直線上で PEB の読みには届かない。終了の経路でも、届けば直後に runtime が名前付きで止める。

## 10. 作ったファイル（`lift/out-teb/`。既存のファイルは変更していない）

- ツール（読み取り専用）: `teb_gs_scan.py`、`teb_gs_scan2.py`、`teb_gs_scan3.py`、`teb_path.py`、`ripref_scan.py`、`ptr_refs.py`、`rva_strings.py`、`chkstk_sites.py`、`chkstk_dyn_bound.py`。
- 測定プログラム: `chkstk_probe.c`（+ ビルド結果 `chkstk_probe`）、`chkstk_oracle.c`（+ `chkstk_oracle`）、`chkstk_lifted.inc`（生成器の出力から 1 行だけ替えた写し）、`malloc_align.c`（+ `malloc_align`）。ビルドは `clang -O0 -w -I stage/lift/runtime`。
- 測定の出力: `chkstk-17daba0.txt`、`chkstk-sites.txt`、`chkstk-reach.txt`、`chkstk-dyn-bound.txt`、`chkstk-probe-run.txt`、`chkstk-oracle-run.txt`、`gs-sites-small.txt`（ほか `gs-sites*.txt`）、`reach-paths.txt`、`callers-*.txt`、`peb-*.txt`、`refs-*.txt`、`ptr-refs*.txt`、`rva-strings.txt`、`initterm-slots.txt`、`exit-*.txt`、`gs30-*.txt`、`detach-17caaf0.txt`、`procattach-17ca9d8.txt` ほか。
- `stage/`（`git archive HEAD` の展開。runtime ヘッダ用）、`target/`（空）、`tmp/`（TMPDIR）。
