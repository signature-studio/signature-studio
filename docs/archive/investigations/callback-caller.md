# ランタイムからゲストのコールバックを呼ぶ機構（決定 0064）と仮想モニタ（決定 0065）の調査と再実行の記録

実行日: 2026-10-22。作業ディレクトリ `signature-studio-develop`、HEAD 0060382 に未コミットの変更を載せた状態。DLL は `binaries/win64/FLEngine_x64.dll`。「一次」は Microsoft Learn（2026-10-22 取得）、「実測」はコマンドの出力、「読み」は生成 C（`lift/out-cb/out`）と `lift/objdump-text.txt` の静的な読み、「推測」は未検証。成果物は `lift/out-cb/`（gitignore 済み。`TMPDIR=lift/out-cb/tmp`、`EM_CACHE=lift/out-cb/emcache`、`CARGO_TARGET_DIR=lift/out-cb/target`）。`next-stop-enumdisplaymonitors.md` と重なる事実（呼び出し点の一覧、コールバックの読み、Learn の EnumDisplayMonitors / MONITORENUMPROC の引用）は、そちらを参照し、ここでは繰り返さない。

## 1. 停止点の確認（実測。native と wasm が一致）

停止は `fle: unsupported instruction at 0x5dca50`（`next-stop-enumdisplaymonitors.md` §1 と同じ。開始時点の基準: 関数実行数 582、入口数 13,379、unsupported_total 540、a4 は init_count 115 / 942、最後の unit index 114 = proc 0x623840）。0x5dca50 は `ff 25 02 46 39 01`（`jmp [rip+0x1394602]`）で IAT スロット 0x1971058、import 表で `user32.dll!EnumDisplayMonitors`（テスト `the_thunks_jump_to_the_slots_which_are_user32_enumdisplaymonitors_and_getmonitorinfow`）。GetMonitorInfoW の thunk 0x5dca40 は IAT スロット 0x1970ee0。MonitorFromWindow / Rect / Point の thunk は 0x5dca10 / 0x5dca20 / 0x5dca30（スロット 0x1970dd8 / 0x1970f68 / 0x19710c0）。

## 2. コールバック 0x85df70 の閉包（読みと実測）

- **関数全体（56 バイト、実測: バイト列をテストで固定）**: `push rsi; push rbx; sub rsp,0x28; mov rbx,rcx; mov rsi,r9; mov rcx,[0x8537f0]; mov dl,1; call 0x40f9e0; mov [rax+8],rbx; mov ecx,[rsi+0x10]; mov [rax+0x10],ecx; mov rcx,rsi; mov rdx,rax; call 0x50a830; mov al,1; add rsp,0x28; pop rbx; pop rsi; ret`。rdx（hdcMonitor）と r8（lprcMonitor）は**読まない**（rdx は `mov dl, 1` と `mov rdx, rax` で書くだけ、r8 は出てこない）。
- **戻り番地・[rsp] を読まない**: 生成 C の `ret` 57,824 個は**すべて** `return;`（実測: `grep '/\* 0x[0-9a-f]* Ret \*/'` が 57,824 件で、`return;}` でないものは 0 件）。戻り番地を `fle_ld(R[4]` で読む行は `Ret` 以外にも無い（`R[4]` を使う行は push/pop/`sub rsp`/`add rsp`/`mov rbp, rsp`/`lea rsp, [rbp+..]`）。
- **静的な直接呼び出しの閉包**（生成 C の `f_xxxxxx();` を辿る。`lift/out-cb/closure-85df70.json`）: 5 関数（f_85df70、f_40f9e0、f_410320、f_410360、f_50a830）、間接呼び出し（`fle_dispatch(t)`）4 か所、`fle_imp_*` の呼び出し 0、`fle_unsupported` 0。
- **動的な閉包**（実測。native の gdb で `f_85df70` の入口から出口まで `__cyg_profile_func_enter` を記録。`lift/out-cb/gdb-cb7.cmd`、`closure-dyn-85df70.txt`）: 16 関数（f_85df70 f_40f9e0 f_410320 f_40f960 f_409410 f_407a50 f_40fb40 f_40d210 f_410360 f_410070 f_50a830 f_50aac0 f_50ade0 f_417fc0 f_417b70 f_409470）。16 関数すべての生成 C を走査し、`R[4]` を使う行で push/pop/`sub`/`add`/`mov rbp,rsp` 以外のもの（`[rsp]` や rsp 相対のロード）は 0 行。呼ぶ API: なし（`fle_imp_*` は閉包に無い）。
- **実行の確認**（実測: gdb）: `fle_imp_EnumDisplayMonitors` の入口で `R[4]=0x7ffff7bffb08`、`R[1]=0 R[2]=0 R[8]=0x85df70 R[9]=0x7ffff6bd7820`。コールバックの入口 `f_85df70` で `R[4]=0x7ffff7bffac8`、`R[1]=0x7fea0004`（hMonitor）、`R[2]=0`（hdcMonitor）、`R[8]=0x7ffff7bffaf0`（lprcMonitor。スタック上）、`R[9]=0x7ffff6bd7820`（dwData、そのまま）、`[R[4]]=0xcb000000`、`*(RECT*)R[8] = (0, 0, 1920, 1080)`。入口の R[4] は 16n+8 で 0xb08 − 0xac8 = 0x40 = 64 バイト下（シャドウ 32 + 整列と戻り番地。API の戻り番地の 8 バイトとの間に 16 バイトの RECT）。

## 3. 生成 C の `call` の形（実測）

- 直接呼び出し: `{ R[4] -= 8; fle_st(R[4], 8, UINT64_C(<次の命令の VA>)); f_<target>(); R[4] += 8; }`。
- 間接呼び出し: `{ uint64_t t = <オペランド>; R[4] -= 8; fle_st(R[4], 8, UINT64_C(<次の命令>)); fle_dispatch(t); R[4] += 8; }`。
- `ret`: `return;`（R[4] を動かさない）。
- API（import の thunk、`jmp [slot]`）: `{ void fle_imp_X(void); fle_imp_X(); return; }`。**API は呼び出し元の `call` が積んだ戻り番地のスロットが `[R[4]]` にある状態で始まり、R[4] を動かさず戻る**（`fle_imports.h` の規約）。
- 引数は R[1]（rcx）、R[2]（rdx）、R[8]（r8）、R[9]（r9）。5 番目以降は `[R[4] + 0x28 + 8(n−5)]`（`fle_env.h` の `fle_arg`）。

これを `fle_call_guest`（決定 0064）が再現する。テスト `a_generated_call_and_fle_call_guest_give_the_callee_the_same_stack` は fe-lift に小さい PE（`sub rsp,0x28; call 0x1040; add rsp,0x28; ret` と、`mov rax, rsp; ret`）を生成させ、生成 C の呼び出しとの callee の rsp の距離が 48 バイト、16n+8 で、どちらも R[4] が戻ることを実行で確かめる。

## 4. Learn の確認事項（一次。2026-10-22 取得）

URL と取得日の一覧と引用は、決定 0064 と 0065 の表に書いた。このファイルが新しく取ったもの:

- [x64 calling convention](https://learn.microsoft.com/en-us/cpp/build/x64-calling-convention)（`ms.date` 2025-03-19、`updated_at` 2026-05-21）: 「The caller must always allocate sufficient space to store four register parameters, even if the callee doesn't take that many parameters」「Integer arguments are passed in registers RCX, RDX, R8, and R9」「The stack pointer must remain 16-byte aligned in any region of code that isn't part of an epilog or prolog, except within leaf functions」「The x64 ABI considers the registers RBX, RBP, RDI, RSI, RSP, R12, R13, R14, R15, and XMM6-XMM15 nonvolatile. They must be saved and restored by a function that uses them」「A scalar return value that can fit into 64 bits … is returned through RAX」。**ページは「call の時点で 16 バイト境界」「callee の入口は 16n+8」とは明記しない**（整列の規則から導いた。callee の入口が 16n+8 であることは「Prologs and epilogs are highly restricted」と「rsp は 16 バイト整列」と call の 8 バイトの戻り番地からの導出）。
- [MONITORINFO](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-monitorinfo)（`updated_at` 2024-02-22）、[MONITORINFOEXW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-monitorinfoexw)、[GetMonitorInfoW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmonitorinfow)（`updated_at` 2024-11-20）: 構造体と cbSize の規則。**MONITORINFOF_PRIMARY の数値はページに無い**（名前だけ）。**GetMonitorInfoW の失敗条件は書かれていない**。MONITORINFOEXW の `szDevice[CCHDEVICENAME]` の CCHDEVICENAME の値もページに無い（SDK ヘッダでは 32。`/usr/share/mingw-w64/include/multimon.h` 37 行）ので、sizeof(MONITORINFOEXW) = 0x68、MONITORINFOEXA = 0x48 は**ヘッダの値からの計算**。
- [EnumDisplayMonitors](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaymonitors)、[MONITORENUMPROC](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nc-winuser-monitorenumproc): 引用は `next-stop-enumdisplaymonitors.md` §3 と決定 0065。**コールバックが FALSE を返したときの EnumDisplayMonitors の戻り値は書かれていない**（Wine の `NtUserEnumDisplayMonitors` は、コールバックの戻り値をそのまま返す。`win32u/sysparams.c` の `ret = *(BOOL *)ret_ptr` を読んだ。非公式。Learn の `EnumWindows` のページは「If EnumWindowsProc returns zero, the return value is also zero」と明記する。同種の API の類推）。
- [MonitorFromWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-monitorfromwindow)（`updated_at` 2024-02-22）: 「retrieves a handle to the display monitor that has the largest area of intersection with the bounding rectangle of a specified window」。dwFlags は「Determines the function's return value if the window does not intersect any display monitor」: MONITOR_DEFAULTTONEAREST（nearest）、MONITOR_DEFAULTTONULL（NULL）、MONITOR_DEFAULTTOPRIMARY（primary）。数値は書かれていない。**hwnd が無効なときの規則は書かれていない**。

## 5. 実装と検証（実測）

- `lift/runtime/fle_callback.h`、`fle_callback.c`、`fle_monitor.h`、`fle_monitor.c`（新規。`lift/out-cb/work/` で完成させ、`clang -Wall -Wextra -Werror` と `emcc -Wall -Wextra -Werror` の両方で警告なし。ヘッダ、`.c` の順に `lift/runtime/` に置いた。その後、実装の確認でテストが 3 件落ちたため work と runtime の**両方を同じ編集**で直した（`cbSize` の 0x48/0x68、`%#x` の 0 の表記、`<inttypes.h>`）。直した後に `cmp` で 4 ファイルとも work と runtime が一致することを確かめた。最後に `fle_callback.c` の説明のコメントと不要な `R[4] += 8` を 1 回直し、再び `cmp` で一致。clang と emcc は直した後に再度 `-Werror` で通した。全 runtime の emcc ビルド（`build.sh rt`）の警告は `fle_proc.c` の 1 件（既存。`fle_thread_prio` の未使用）だけで、`fle_callback.c`、`fle_monitor.c` は `errors=0 warnings=0`）。
- `lift/crates/fe-lift/src/imports_table.rs`: 末尾にコメント付きで 3 行追記（EnumDisplayMonitors、GetMonitorInfoW。編集の直前に読み直した）。`lib.rs` は変更なし。
- 既存テストの更新（「答えない」の表明を外した。理由をコメントに書いた）: `tests/kbd.rs`（隣接 API の表から `EnumDisplayMonitors` を除外）、`tests/sysmetrics.rs`（同じ）。
- テスト: `tests/callback.rs` 18 件、`tests/monitor.rs` 25 件（新規）。ワークスペース全体 `cargo test --workspace --no-fail-fast --offline`: **83 バイナリ、896 件、失敗 0**（基準は 81 バイナリ・853 件。差は新規 2 バイナリ・43 件。`lift/out-cb/test-ws.txt`）。
- 変異テスト: 66 件を作り（`lift/out-cb/mkmut.py`）、最初の実行（`mut-summary.txt`、66 件）で生き残ったのは 2 件。`c05-return-slot-not-popped`（ランタイムの `R[4] += 8` を消す変異。実装の `R[4] = entry_sp` が後で全部を戻すので、この `+= 8` は**冗長で、除いた**。除いた版に対する別の変異で置き換える必要は無いと判断）と `m27-cbsize-8-bytes`（`uint32_t cb` を `uint64_t` にするだけの**同値変異**）。最終ソースから作り直した 2 回目（`mut-summary2.txt`。c05 と m27 を除き m41 を足した 65 件）を実行し、`m41-cbsize-read-as-8-bytes`（8 バイトを読んで `(uint32_t)` に切る**同値変異**）が生き残った。同値なので捨て、cbSize を 64 ビットで比べる `m42-cbsize-64-bit-compare` に置き換えた（検出: 4 テスト失敗）。最終の集計: 最終ソースに対して作った変異は 66 件（`mut2` の 65 件 + m42）。同値の m41 を除く **65 件がすべて検出**（`mut-summary2.txt` の 65 件中 64 件が検出 + m42）（clang のコンパイル失敗による検出は 0。`clang_failed_mentions=0`）。
- **a4 の再実行**（`OUT=lift/out-cb WASM=1 bash lift/harness/a4-survey.sh`。生成 C は `lift/out-cb/out` に再生成、native は `build_native.sh`、wasm は `build.sh rt gen-c harness link`）:

```
stop   : fle: SystemParametersInfoW: uiAction 0x1f (SPI_GETICONTITLELOGFONT) is used by the guest but no value is decided (decision 0058)
         top of stack: 0x4255d0 f_4255d0
depth  : init_count=115 / 942   last unit: index=114 run_rank=111/895 proc=0x623840 in_progress
         started=111 completed=110 in_progress=1 of 895 runnable   all_units_done=no
funcs  : executed (distinct)=602  entries=13541
wasm   : same a4 lines as native (exit 134)
```

  スタック（実測。`call depth at stop: 9`）: f_4255d0（guest-return 0x866429）← f_8663b0（0x865379）← f_8651a0（0x620dea）← f_620cd0（0x62386e）← f_623840（0x412129）← f_4120c0 ← f_412160 ← f_41d8f0 ← エントリ f_1376820。**native と wasm は、`rsp=` の行（スタックのアドレス。native は 0x729a547fffc0、wasm は 0x11c50330）を除いて、281 行の出力全体が一致**（実測: `diff <(grep -v 'rsp=' native) <(grep -v 'rsp=' node)` が空）。a4 の値は前回と同じ（init_count 115 / 942、unit index 114、started 111、completed 110）。関数実行数は 582 → 602、入口数は 13,379 → 13,541。unit 114 の中で進んだ（f_8651a0 が EnumDisplayMonitors の後、`0x5c0d40` × 5 と `0x8663b0` に入った）。

## 6. 次の停止点とその先の見通し（実装していない）

- **次の停止点（実測）**: `SystemParametersInfoW(SPI_GETICONTITLELOGFONT = 0x1f)`（f_8663b0 の 0x866424）。決定 0058 が名前付き停止にしている。実装していない。判断材料は `next-stop-spi-iconmetrics.md`。
- **MonitorFrom\* の判断材料**（これらは停止点の先で、まだ届いていない。実装していない。依頼の規則は「次の停止点が MonitorFrom\* のときだけ、追加で最大 2 つまで実装してよい」で、次の停止点は SPI 0x1f なので該当しなかった）:
  - **MonitorFromWindow**（5 か所: 0x85dfc8、0x86748a、0x86a999、0x86a9b8、0x8805a9）。dwFlags は、0x85dfc8、0x86a999、0x86a9b8、0x8805a9 が `edx = 2`（MONITOR_DEFAULTTONEAREST の数値は SDK ヘッダで 2。Learn は数値を書かない）、0x86748a は `[0x1409ca8 + r8*4]` の表から。表（.data、0x1409ca8）の先頭 4 要素は `2, 0, 1, 0`（実測: DLL のバイト列。表の添字は呼び出し元の引数 `r8b`）。hwnd の出どころ: 0x85dfc8 は `f_617920()` の戻り値（`[rbx+0x45c]` を返す）、0x86a999 は `[rbp+0x98]`、0x86a9b8 は `[[rbp+0xe0]+0x2b0]`、0x8805a9 は `[rbx+0x1c0]`、0x86748a は引数 `rdx`。**どれも hwnd が非 NULL の可能性がある形**で、窓の表（HWND の実在検査）が要る。hwnd が NULL でも実在しなくても DEFAULTTONEAREST/PRIMARY なら（Learn の規則から）primary を返してよいが、Learn は「hwnd が無効なとき」の規則を書かない。**窓の表を作る前に、ゲストが hwnd として何を渡すか（仮想の窓のハンドルが先に作られるか）を見る必要がある**。
  - **MonitorFromRect**（2 か所: 0x807849、0x867451）、**MonitorFromPoint**（1 か所: 0x86741a）: 窓の表は要らず、「モニタが 1 つ」で答えられる形（dwFlags が DEFAULTTONEAREST/PRIMARY なら、矩形／点がモニタと交わらなくても primary、DEFAULTTONULL なら交わらないときだけ NULL）。ただし**交わる／交わらないを判定する規則**（RECT がモニタの何ピクセルと交わるか）を決める必要がある。0x807849 は `[rbp+0xb0]` の RECT、0x867451 は `[rsp+0x38]` が指す RECT（読み）。
- **GetMonitorInfoW の 6 か所**（0x807862、0x864fbd、0x864ffd、0x865037、0x86a9ea、0x8805c4）は、cbSize 0x28 のとき実装済み。0x864fbd、0x864ffd、0x865037 は引数が `[rcx+8]`（要素の hMonitor）で、`EnumDisplayMonitors` のコールバックが作ったノード（`[node+8] = hMonitor`）の値、つまり仮想ハンドル 0x7FEA0004。他の 3 か所は MonitorFromWindow/Rect の結果。**MonitorFrom\* が NULL（`DEFAULTTONULL` のとき）や別の値を返したら、GetMonitorInfoW が止まる**（ハンドルが仮想ハンドルと違うので）。
- **0x1f の先**: 0x8663b0 の SPI 0x29（NONCLIENTMETRICS）、`CreateFontIndirectW`（IAT スロット 0x19719d0、thunk 0x423fc0。5 回）、`GetStockObject(13)`（0054 が答える）、`CreateHalftonePalette`（0x1971ba0）など。

## 7. 限界

- コールバックの閉包の動的な実測は 1 回の実行（1 モニタ）のもの。モニタが 2 つ以上のとき（この環境では起きない）の閉包は見ていない。
- `fle_call_guest` の整合検査は callee-saved の汎用レジスタと rsp と戻り番地のスロットだけ（xmm6-15、x87、MXCSR は検査しない）。
- 実 Windows の実機との比較はしていない。
- `lprcMonitor` のスタック上の RECT は、コールバックの戻りの後で他の用途に使われる（API の戻りの後は未定義）。コールバックがその値を保持するポインタを持ち続けると壊れる。ゲストの 0x85df70 は RECT を読まない。
