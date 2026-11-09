# GetStartupInfoW の cb: 決定 0018、Learn、両 DLL の呼び出し点、選択肢

調査日: 2026-10-23。HEAD e90231d（main）。**調査のみ。この文書は何も決めない**。`lift/` 配下は 1 ファイルも編集していない（読み取りのみ）。作業物は `lift/out-startupinfo/`（新規。スクリプトと出力）。この調査では runtime も DLL も**実行していない**（逆アセンブルとソースの読みと、既存ログの読み）。

背景は `stage23-two-module-real-dll.md` §5.1 段 1、§9 の 5。dsp_ippv2_x64.dll の DllMain(ATTACH) が CRT の lowio 初期化（`f_37d34b4`）で `GetStartupInfoW` を呼び、runtime が `fle: GetStartupInfoW: cb is not sizeof(STARTUPINFOW) = 0x68` で止まる。

## 0. 凡例と入力

- **実測** = このセッションで実行したコマンドの出力（`lift/out-startupinfo/` にある）。
- **読み** = 逆アセンブル（capstone、objdump）、ソース、Learn のページを読んだ結果。
- **既存ログ** = 他の作業のログ（`lift/out-stage2/` など）を読んだ。再実行していない。
- **推測**、**未確認** = そのとおり。
- 入力ファイル（SHA-256。`input_sha256.txt`）: `binaries/win64/FLEngine_x64.dll` = `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d`、`lift/out-ext/dsp_ippv2_x64.dll` = `497dd4f77e859df9e86d999b3811be3c72526b41f86c38b0c3c6e395f691c352`（ImageBase 0x180000000 のまま。段階 2・3 の run は 0x2000000 にリベースして動かすので、下の VA `0x1817d34b4` は run のログでは `0x37d34b4` と書かれる）。
- 読んだ 4 ファイル（`lift/runtime/fle_env.c`、`lift/runtime/fle_vol.c`、`lift/crates/fe-lift/tests/env.rs`、`docs/decisions/0018-virtual-fs-reg-env.md`）は HEAD と同一（`git show HEAD:<path> | cmp` で一致。実測）。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | 決定 0018 が書くこと | §7 に「GetStartupInfoW は、`cb`（0x68）以外の全メンバーを 0 にする。作成元プロセスがないので、フラグ、位置、標準ハンドルはない」だけ。**「呼び出し側の cb が 0x68 でなければ停止」という文は 0018 に無い**。その文は `fle_env.c:355` のコメントとテストにある | 読み（`grep` で確認） |
| 2 | 停止の根拠 | 0018 §決定（PM）の 1 行「仕様が定める範囲を超える入力は abort で止める」を、作業者が cb に当てはめたもの（読み）。Learn に cb の規則は無い | 読み |
| 3 | Learn（2 ページ） | `lpStartupInfo` は `[out]`、「startup information を受け取る構造体」。cb は「構造体のサイズ」。**呼び出し側が設定する必要があるとも、関数が書くとも、一次に無い** | 読み（引用: §3） |
| 4 | 呼び出し点 | FLEngine 1 か所（thunk 経由）、dsp_ippv2 1 か所（IAT 直接 `call`）。それ以外の参照は無い（走査 4 種で 0 件） | 実測 + 読み |
| 5 | バッファの出所 | **FLEngine は cb に 0x68 を入れてから呼ぶ。dsp_ippv2（CRT）は 0x68 バイトの全体が未初期化**（cb も） | 読み |
| 6 | 呼び出し後に読むもの | FLEngine: `dwFlags`（+0x3c）、`wShowWindow`（+0x40）。dsp_ippv2: `cbReserved2`（+0x42）、`lpReserved2`（+0x48）。**どちらも cb を読み返さない** | 読み |
| 7 | runtime の答え（cb 以外 0）のときの CRT | `cbReserved2 == 0` で継承ハンドルの処理を全部飛ばし、標準 3 本を「コンソールなし」にする（読み）。gdb が cb だけ書いた run（`CHAIN_MAX=1`）では `GetStartupInfoW` を通り、**次の停止は `MultiByteToWideChar`（段 2。ロケール表）**。DllMain が戻ったのは段 1〜5 を全部入れた構成だけ（`stage23` §5.1） | 読み + 既存ログ |

## 2. 決定 0018 と実装（読み取りのみ）

### 2.1 決定の文面

`docs/decisions/0018-virtual-fs-reg-env.md`:

- §決定（PM。変更しない）の 11 行目: 「仕様が定める範囲を超える入力は abort で止める（黙って成功にしない）。」
- §作業者の判断 §7（77 行目）: 「GetStartupInfoW は、`cb`（0x68）以外の全メンバーを 0 にする。作成元プロセスがないので、フラグ、位置、標準ハンドルはない。」

`grep "caller|呼び出し側|cb が|cb は|cb の"` は 0018 の中で 0 件。つまり**停止規則そのものは 0018 の本文に書かれていない**。

### 2.2 根拠（どの Learn のどの記述か）

`docs/investigations/import-vfs-spec.md` 105 行目の表:

| API | 参照したページ | 使った記述 | 仕様に無いもの |
|---|---|---|---|
| GetStartupInfoW | `api/processthreadsapi/nf-processthreadsapi-getstartupinfow`、`...ns-processthreadsapi-startupinfow` | 「作成元が指定した構造体を返す」、メンバー一覧（x64 での大きさは 0x68） | 「作成元がないのですべて 0（決定 0018）」 |

つまり根拠は 2 つ: ① GetStartupInfoW のページの Remarks「The STARTUPINFO structure was specified by the process that created the calling process」、② STARTUPINFOW のメンバー型から計算した大きさ 0x68（§4.1 で ctypes で独立に再計算して一致を確認）。**cb が 0x68 でなければ止まる、という規則に対応する Learn の記述は、import-vfs-spec.md の表にも無い**。`fle_env.c` のコメントも「any other value is not defined by the page and stops the run」と書く（ページが定めないので止める、という形）。

### 2.3 実装（`lift/runtime/fle_env.c:351-366`）

```c
#define FLE_STARTUPINFOW_SIZE 0x68u
void fle_imp_GetStartupInfoW(void)
{
    static const char api[] = "GetStartupInfoW";
    uint64_t p = R[1];
    if (p == 0) fle_stop(api, "lpStartupInfo is NULL");
    if ((uint32_t)fle_ld(p, 4) != FLE_STARTUPINFOW_SIZE) fle_stop(api, "cb is not sizeof(STARTUPINFOW) = 0x68");
    memset((void *)fle_addr(p, FLE_STARTUPINFOW_SIZE, 1), 0, FLE_STARTUPINFOW_SIZE);
    fle_st(p, 4, FLE_STARTUPINFOW_SIZE);
}
```

- 停止条件（`fle_stop` は `fprintf(stderr, "fle: %s: %s\n")` の後 `abort()`、`fle_env.c:10-14`）: ① `rcx == 0`（`lpStartupInfo is NULL`）、② 先頭 4 バイト（cb）が 0x68 でない（`cb is not sizeof(STARTUPINFOW) = 0x68`。`fle_env.c:363`）。
- 埋める値: 0x68 バイトを 0 で埋め、その後 cb（+0）に 0x68 を書く。したがって `lpReserved`、`lpDesktop`、`lpTitle` = NULL、`dwX`〜`dwFillAttribute` = 0、`dwFlags` = 0、`wShowWindow` = 0、**`cbReserved2` = 0、`lpReserved2` = NULL**、`hStdInput`、`hStdOutput`、`hStdError` = 0（INVALID_HANDLE_VALUE ではない。`dwFlags` に STARTF_USESTDHANDLES が無いので、Learn はこれらを「ignored」と書く。§3）。戻り値は無い（VOID）。
- 既存のテスト（`lift/crates/fe-lift/tests/env.rs`）: `command_line_and_startup_info` は cb = 0x68 を入れて呼び、「cb 68」「cb 以外の非 0 が 0 個」を確かめる（152-173 行）。`undefined_input_stops_the_run` は `startup-null`（rcx = 0）と `startup-wrong-cb`（cb = 0x44）が abort することを確かめる（215-216 行）。**cb の規則を変えるなら、このテストの期待値（`startup-wrong-cb`）も変わる**。
- コメントは、0x68 が FLEngine が cb に書く値（`0x40738d`）でもあると書く。逆アセンブルで確認した（§5.2）。

### 2.4 既存の記述との食い違い（事実）

`stage23-two-module-real-dll.md` §5.3（153 行目）と `lift/harness/two/patches_stage1.py`（3 行目、`patches_chain.py` 21 行目のコメント）は、この停止を「決定 0018 の記述『caller の cb が 0x68 でなければ停止』」「decision 0018 text: "the caller's cb must be 0x68"」と書く。**この文は 0018 の本文に無い**（§2.1）。実際の出どころは `fle_env.c:355` のコード内コメント。決定 0018 の改訂が要る、という §9 の 5 の書き方は、規則が 0018 にあるかのように読めるので、この点を訂正して読むのがよい。

## 3. Microsoft Learn（取得 2026-10-23。全文は `lift/out-startupinfo/learn-excerpts.md`）

### 3.1 GetStartupInfoW

https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getstartupinfow（`updated_at` 2025-08-27）

> Retrieves the contents of the STARTUPINFO structure that was specified when the calling process was created.
>
> `VOID GetStartupInfoW( [out] LPSTARTUPINFOW lpStartupInfo );`
>
> `[out] lpStartupInfo` — A pointer to a STARTUPINFO structure that receives the startup information.
>
> Return value: None
>
> Remarks: The STARTUPINFO structure was specified by the process that created the calling process. It can be used to specify properties associated with the main window of the calling process.

**ページ全体に `cb` という語は無い**。呼び出し前のバッファの内容、構造体の大きさ、失敗、last error についての記述も無い。

### 3.2 STARTUPINFOW

https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/ns-processthreadsapi-startupinfow（`updated_at` 2024-11-20）

> `cb` — The size of the structure, in bytes.
>
> `cbReserved2` — Reserved for use by the C Run-time; must be zero.
>
> `lpReserved2` — Reserved for use by the C Run-time; must be NULL.
>
> `wShowWindow` — If **dwFlags** specifies STARTF_USESHOWWINDOW, this member can be any of the values that can be specified in the *nCmdShow* parameter for the [ShowWindow] function, except for SW_SHOWDEFAULT. Otherwise, this member is ignored.
>
> `dwFlags` の STARTF_USESTDHANDLES: …If this flag is specified when calling the [GetStartupInfo] function, these members are either the handle value specified during process creation or INVALID_HANDLE_VALUE.
>
> Remarks: …A process can use the [GetStartupInfo] function to retrieve the **STARTUPINFO** structure specified when the process was created.

例（作る側。`CreateProcess` へ渡す構造体）: `ZeroMemory( &si, sizeof(si) ); si.cb = sizeof(si);`。**GetStartupInfo の使い方の例ではない**。

### 3.3 cb について、一次に書いてあること / 無いこと

| 問い | Learn |
|---|---|
| 呼び出し側が GetStartupInfoW の前に cb を設定する必要があるか | **一次に無い**（GetStartupInfoW のページに cb の語が無い。`si.cb = sizeof(si)` の例は CreateProcess に渡す側） |
| 関数が cb に書くか | **一次に無い**。「receives the startup information」、cb は「The size of the structure, in bytes」。cb を書く、と読むのは**読み**（明記は無い） |
| cb が合わないときの結果（失敗、停止、無視） | **一次に無い** |
| 構造体の大きさ（0x68） | **ページに数値は無い**。メンバー型の並びから計算できる（読み。§4.1） |
| `[out]` の意味 | GetStartupInfoW のページは定義しない。SAL の `_Out_`（別ページ: 「The parameter doesn't have to be valid in pre-state but must be valid in post-state」）と同じ意味だと読むのは**読み**で、どのページも GetStartupInfoW についてそう書かない |
| 作成元プロセスが無いときの値 | **一次に無い**（「作成元が指定した」としか書かない） |
| `cbReserved2`、`lpReserved2` の値 | 「Reserved for use by the C Run-time; must be zero / must be NULL」。この「must」が、作る側（CreateProcess に渡す構造体）についてか、GetStartupInfo が返す値についてかは**ページに無い** |

## 4. 呼び出し点（全列挙）

### 4.1 方法と限界

- PE: `gsi_pe_info.py`（pefile）。構造体のレイアウト: `gsi_layout_pdata.py`（ctypes で Learn のメンバー型から独立に計算し、`gsi_annotate.py` の表と全メンバーで一致、大きさ 0x68。`layout_pdata.txt`）。
- 参照の走査（どちらの DLL にも 4 種を実行。実測）:
  1. 実行可能な全セクションの rip 相対のメモリオペランドで IAT スロットへ解決するもの（バイト走査の後 capstone で命令を確定。`gsi_callsites.py`）。
  2. `E8`/`E9` の rel32 が thunk に届くもの（同上。候補は capstone で復号して確認。`gsi_callers.py`）。
  3. スロットの VA の 8 バイト絶対値、RVA と VA 下位 32 ビットの 4 バイト定数（`gsi_callsites.py`、`gsi_imm_scan.py`。全て 0 件）。
  4. 名前なし（序数）import と delay import（`gsi_other_paths.py`、`gsi_extra_checks.py`。0 件）、`GetStartupInfo` の文字列（ASCII 1 件ずつ。**どちらも import の hint/name 表の項目そのもの**。UTF-16 は 0 件）。
  - 独立の確認として、`objdump -d` の全出力を `grep` した結果が一致した（`flengine_objdump_sweep_grep.txt`、`dsp_objdump_sweep_grep.txt`）。
- 限界（未確認）: レジスタ相対（イメージ基底をレジスタに持つ `[reg+disp]`）は定数走査で RVA 0 件なので見つかっていない。実行時に作ったアドレス、`.pdata` に載らないコードへの間接 call は静的には追えない。`GetStartupInfoA` は両 DLL の import に無い（`StartupInfo` を含む import 名は W 版 1 件だけ）。

### 4.2 FLEngine_x64.dll（ImageBase 0x400000）

- import: `kernel32.dll!GetStartupInfoW`、IAT スロット VA 0x1971588（RVA 0x1571588）。名前の import（序数ではない）。実測。
- IAT スロットを参照する命令は **0x407080 の thunk 1 つだけ**: `jmp qword ptr [rip+0x156a502]  # 0x1971588`。thunk を指す `call`/`jmp` は **0x40739a の `call 0x407080` が 1 つだけ**（8 バイト絶対の参照 0、`.pdata` に thunk の項目なし）。実測。
- 呼び出し元の関数 0x407380〜0x4073ba（`.pdata` の項目あり、大きさ 0x3a）は **0x41cec4 の `call 0x407380`（関数 0x41ce10〜0x41cefb の中）から 1 回だけ**呼ばれる（8 バイト絶対の参照 0）。実測。
- 逆アセンブル（読み。`flengine_407330.asm`、`annotated.txt`）:

```
407380: push rbx
407381: sub  rsp,0x90                       ; バッファは [rsp+0x28 .. rsp+0x90)、直上が保存した rbx
407388: mov  ebx,0xa                        ; 既定値 10 (= SW_SHOWDEFAULT。ShowWindow のページ)
40738d: mov  dword ptr [rsp+0x28],0x68      ; cb = 0x68    ← 呼び出し側が cb を設定する
407395: lea  rcx,[rsp+0x28]
40739a: call 0x407080                       ; → jmp [0x1971588] = kernel32!GetStartupInfoW
40739f: mov  eax,dword ptr [rsp+0x64]       ; dwFlags (+0x3c)
4073a3: test eax,1                          ; STARTF_USESHOWWINDOW
4073a8: je   4073af
4073aa: movzx ebx,word ptr [rsp+0x68]       ; wShowWindow (+0x40)
4073af: mov  eax,ebx / add rsp,0x90 / pop rbx / ret
```

- バッファの出所: **スタック。cb（+0）だけ 0x68 を入れ、残り 0x64 バイトは未初期化のまま渡す**。
- 呼び出し後に読むもの: `dwFlags`（+0x3c）の bit 0、立っていれば `wShowWindow`（+0x40）。`cbReserved2`、`lpReserved2`、`hStd*`、cb は読まない。戻り値（ebx、既定 10）は呼び出し元 0x41cec9 の `mov [rip+0x11c91b9],eax`（グローバル 0x15e6088）に保存される。0x15e6088 への rip 相対の参照はこの書き込み 1 件だけ（読み手はレジスタ相対かもしれず**未確認**）。
- runtime の答え（`dwFlags` = 0）では `ebx` は 10 のまま返る（読み）。
- 実行（既存ログ）: `lift/out-vfs/run-vfs-896.txt` 87-88 行に `callee=0x407380 … guest-return=0x41cec9`、`callee=0x407080 … guest-return=0x40739f`。`vfs-run.md` §4: `GetStartupInfoW` 1 回が通過。cb = 0x68 を渡すので、現行の runtime で停止しない（読み）。

### 4.3 dsp_ippv2_x64.dll（ImageBase 0x180000000）

- import: `KERNEL32.dll!GetStartupInfoW`、IAT スロット VA 0x181ebd0e8（RVA 0x1ebd0e8）。名前の import。実測。
- IAT スロットを参照する命令は **0x1817d34d4 の `call qword ptr [rip+0x6e9c0e]  # 0x181ebd0e8` 1 つだけ**。jmp thunk は無い（他の参照 0、8 バイト絶対 0、定数 0）。実測。
- 呼び出し元の関数 `0x1817d34b4〜0x1817d35a1`（`.pdata`、大きさ 0xed）は `0x1817d36c9` の `call 0x1817d34b4` から呼ばれ、その呼び出し元 `0x1817d36ac〜0x1817d36e7` は `.rdata` の 8 バイト絶対の項目 `0x181ec83e0`（近傍 `0x181ec83c8〜0x181ec83f8` はコードポインタの表）が指す（実測。リベース後の `0x3ec83e0`、`stage23` §5.1 の「初期化表の項目 0x3ec83e0」と一致）。`GetStartupInfoW` はこの 1 回のみ（`stage23` §4 の列、各 1 回と一致）。
- 逆アセンブル（読み。`dsp_f37d34b4.asm`、`annotated.txt`。バッファは `lea rcx,[rax-0x78]`）:

```
1817d34b4: mov  rax,rsp                      ; rax = 入口の rsp (R)。[R] は戻りアドレス
1817d34b7..c3: mov [rax+8..0x20], rbx/rbp/rsi/rdi   ; R+8.. = 呼び出し元のホーム領域。バッファの外
1817d34c7: push r14                          ; R-8
1817d34c9: sub  rsp,0x90                     ; rsp = R-0x98
1817d34d0: lea  rcx,[rax-0x78]               ; rcx = R-0x78 = rsp+0x20。バッファは [rsp+0x20 .. rsp+0x88)
1817d34d4: call [0x181ebd0e8]                ; kernel32!GetStartupInfoW   ← この前に cb を含めバッファへの書き込みは無い
1817d34da: xor  r14d,r14d
1817d34dd: cmp  word ptr [rsp+0x62],r14w     ; cbReserved2 (+0x42) == 0 ?
1817d34e3: je   0x1817d3583                  ; はい → 何もせず戻る
1817d34e9: mov  rax,qword ptr [rsp+0x68]     ; lpReserved2 (+0x48)
1817d34ee: test rax,rax
1817d34f1: je   0x1817d3583                  ; NULL → 何もせず戻る
         （ここから先は cbReserved2 != 0 かつ lpReserved2 != NULL のときだけ）
1817d34f7: movsxd rbx,[rax]                  ; handle_count = *(int *)lpReserved2
1817d34fa: lea  rsi,[rax+4]                  ; osfile のバイト列
1817d34fe..1817d3508: edi = min(handle_count, 0x2000)
1817d350d: call 0x1817d7dd4(edi)             ; lowio の配列を確保（ecx >= 0x2000 なら errno = 9）
1817d3512..1817d351f: edi = min(edi, _nhandle) ; 0 なら戻る
1817d3526: 各 fh: [rbx] == -1 または -2 なら飛ばす / (byte [rsi] & 1) == 0 なら飛ばす /
           (byte [rsi] & 8) == 0 なら call [0x181ebd1b8] (= KERNEL32!GetFileType) し、0 なら飛ばす
1817d3565..6f: __pioinfo の fh 番目の osfhnd (+0x28) = [rbx]、osfile (+0x38) = [rsi]
```

- バッファの出所: **スタック。0x68 バイトの全体が未初期化（cb を含む）**。入口から `call` までの命令でバッファへ書くものは無い（保存先は R+8〜R+0x20 と R-8）。測定値と整合: `rcx = 0x7ffff7bff930`、`[rcx]` = 0x65d0ab08、別の run で 0x65d0bb08（実行ごとに違う。`lift/out-stage2/run/tbl-4-gdb.log`、`seq-3-gdb.log`。既存ログ）。
- 呼び出し後に読むもの: **`cbReserved2`（+0x42、WORD）と `lpReserved2`（+0x48）だけ**。cb、`dwFlags`、`wShowWindow`、`hStd*` は読まない。cb を呼び出し後に読み返す命令も無い（`[rsp+0x20]` を読む命令が関数内に無い）。したがって **runtime が cb に何を書いても、この DLL の挙動は変わらない**（読み）。
- `lpReserved2` の先（`cbReserved2 != 0` のときだけ）: 先頭 4 バイトのハンドル数 N、続く N バイトの osfile、続く N 個の 8 バイトのハンドル。これが「CRT が継承ファイルハンドルを受ける」処理（読み。UCRT の `initialize_inherited_file_handles_nolock` の形と一致するが、ソースとの照合は非公式資料のみ。§8）。
- 同じ初期化の次（`0x1817d35a4`）: 標準 3 本の `osfhnd` が -1/-2 のままなら `GetStdHandle(-10/-11/-12)`（IAT 0x181ebd1b0）を呼び、結果が -1 か 0 なら、`osfile |= FDEV(0x40)`、`osfhnd = -2`、`__piob` があれば `FILE::_file = -2`（「コンソールなし」の扱い）。それ以外なら `GetFileType`（0x181ebd1b8）。runtime は `GetStdHandle` に NULL を返す（`fle_vol.c:48-56`）ので「コンソールなし」の枝に入る（読み）。

### 4.4 呼び出し点の対比

| | FLEngine_x64.dll 0x40739a | dsp_ippv2_x64.dll 0x1817d34d4 |
|---|---|---|
| 呼び方 | `call` → thunk 0x407080 → `jmp [IAT]` | `call [IAT]`（直接） |
| 呼び出し元 | 0x407380（← 0x41cec4） | 0x1817d34b4（← 0x1817d36c9 ← 初期化表 0x181ec83e0） |
| バッファ | スタック `[rsp+0x28, +0x90)` | スタック `[rsp+0x20, +0x88)` |
| cb の設定 | **あり（0x68。0x40738d）** | **なし（未初期化）** |
| 他のメンバー | 未初期化 | 未初期化 |
| 呼び出し後に読む | `dwFlags`（+0x3c）、`wShowWindow`（+0x40） | `cbReserved2`（+0x42）、`lpReserved2`（+0x48）（と、その先） |
| cb を読み返すか | いいえ | いいえ |
| 現行 runtime | 通る（cb = 0x68） | **停止**（cb = 残りの値） |

## 5. runtime の答えのときの実行経路（既存ログ + 読み）

runtime が `cbReserved2 = 0`、`lpReserved2 = NULL` を返すと、`f_37d34b4` は `0x1817d34e3` の `je` で 0x1817d3583 へ飛び、**継承ハンドルの処理に入らない**（`GetFileType`、`__pioinfo` への書き込み、`lpReserved2` の参照はいずれも実行されない。読み）。その後 `GetStdHandle` を 3 回呼び（0xfffffff6、5、4）、NULL を得て 3 本とも「コンソールなし」にする（読み）。

既存の測定との整合: `stage23` §5.1 段 1 は gdb が `[rcx]` に 0x68 を書いて runtime の通常の答えを使う。**段 1 だけ（`CHAIN_MAX=1`）の停止は `MultiByteToWideChar`（init_count 145、関数 881、入口 15,934。`stage23` §5.1 の表）**で、DllMain は戻らない。run のログ `lift/out-stage2/run/seq-1-imports.txt`（既存ログ。このセッションで読んだ）: 110 番が `GetStartupInfoW`（ret=0x37d34da）、111〜113 番が `GetStdHandle`（0xfffffff6、5、4）、`GetFileType` は 0 回、125 番の `MultiByteToWideChar` で停止（`fle_locale.c:33`）。`tbl-1-imports.txt` は同じ名前の別構成で 110 番の `GetStartupInfoW`（cb の停止）で止まっており、パッチなしの run に見える（構成は読んでいない。未確認）ので引用しない。DllMain が eax = 1 で戻ったのは段 1〜5（ロケール表の仮の値 4 段を含む）を全部入れたとき（`stage23` §5.1 の表）。したがって、cb の規則を直しても DllMain 完走には**ロケールの仮の値（`stage23` §9 の 4）がまだ要る**。`lift/out-stage2/run/final-imports.txt` 110〜113 行は `GetStartupInfoW`（ret=0x37d34da）、`GetStdHandle` ×3（ret=0x37d3623）、`GetFileType` は 0 回。`stage23` §4 も「DllMain の間に `GetFileType` は呼ばれなかった」。決定 0078 の表（14 行目）と `fle_vol.c:58-65` のコメントも、`cbReserved2` が 0 のとき `GetFileType` を飛ばすと読んでおり、一致する。**ただし、runtime そのものを変えた run はまだ無い**（gdb の仮の値だけ）。

## 6. 選択肢（決めない）

| | (a) 現状維持 | (b) cb を出力として扱い 0x68 を書いて続行 | (c) その他 |
|---|---|---|---|
| 内容 | `cb != 0x68` で停止（`fle_env.c:363`） | 0x68 の比較を外す。cb の書き込みと全メンバー 0（`fle_env.c:364-365`）は今のまま。NULL の停止は残す | 下記 c1〜c5 |
| FLEngine の呼び出し | 通る | 通る（変化なし） | — |
| dsp_ippv2 の呼び出し | **止まる**（実測。`stage23` §3.2、§5.1）。CRT の命令（`cbReserved2` の比較以降）に届かない。DllMain は戻らない | `GetStartupInfoW` は通る。`cbReserved2 == 0` で継承処理を**飛ばす**。標準 3 本は GetStdHandle = NULL で「コンソールなし」。**停止点は次の `MultiByteToWideChar` に進む**（段 2。gdb が cb だけ書いた `CHAIN_MAX=1` の run と同じ入力。DllMain はまだ戻らない） | c4 は (b) と同じ経路。c5 は別 |
| CRT が `lpReserved2 = NULL`、`cbReserved2 = 0` 前提の処理に入るか | 入る前に止まる | **入る**（その枝が実行される。`lpReserved2` の先は読まれない） | — |
| Learn が支持する範囲 | 無い。Learn に cb の規則も、cb が違うときの結果も無い | `[out]` の引数、「receives the startup information」、cb は「The size of the structure, in bytes」。cb に構造体の大きさを書くのは、取得した構造体の 1 メンバーとして自然（読み）。runtime が cb を書くこと自体は既に現行の実装 | — |
| Learn が支持しない範囲 | 「呼び出し側が cb を 0x68 にしておく」（ページに無い）。停止そのもの | 「関数は入力の cb を無視する」「関数が cb を書く」と**明記した記述は無い**。バッファが 0x68 未満のときの結果も無い。作成元が無いときの値 0（これは (a) でも同じで 0018 の判断） | — |
| 副作用の範囲 | なし | runtime は呼び出し側のバッファの大きさを知らず 0x68 バイト書く。両 DLL のバッファはどちらも 0x68 バイト以上と読める（FLEngine `[rsp+0x28,+0x90)`、dsp `[rsp+0x20,+0x88)`。§4.2、4.3）。静的に見つかった呼び出し点はこの 2 つだけ（§4.1 の限界あり） | — |
| 他に変わるもの | なし | `env.rs` の `startup-wrong-cb`（cb = 0x44 で abort を期待）、`fle_env.c:355` のコメント、0018 §7 の記述、`stage23` §5.3 と §9 の 5 の扱い | — |

(c) の中身:

- **c1: cb の値が 0x68 のときだけ通し、他は止める、の判別を「未初期化」と「誤った値」で分ける** — 成立しない。未初期化の値は実行ごとに違い（0x65d0ab08、0x65d0bb08）、誤った値と区別できない（読み）。
- **c2: cb が 0x68 以外のときは cb だけ書いて他を触らない**（0x68 未満のバッファへの書き込みを避ける形） — Learn に根拠が無い。dsp の呼び出し点では `cbReserved2`、`lpReserved2` が未初期化のまま CRT に読まれ、`lpReserved2` の先を読みに行く（読み。実行では危険）。成立しない。
- **c3: cb の値に応じて書く量を変える** — Learn に書く量の規則が無く、未初期化の値に対して意味が無い。成立しない。
- **c4: 生成側（lift）か harness で呼び出し点（`0x1817d34d0`）の前に cb を書く** — runtime の規則は保たれるが、ゲストの命令列と違う動作を足す。Learn の範囲外。`patches_stage1.py` の gdb 仮の値はこの形の一時版。
- **c5（別軸）: `cbReserved2`、`lpReserved2` に非 0 を入れる** — cb の選択とは独立。Learn は「must be zero / must be NULL」と書く（誰についてかは §3.3 のとおり不明）。非 0 にすると CRT が `GetFileType`（0x1817d353f）に進み、runtime は名前付きで停止する（`fle_vol.c:69`）。0018 の判断（0 / NULL）は変えなくてよい。

## 7. 意見（推奨。事実ではない）

**(b) の最小形が、Learn と実行経路の両方から一番整合すると考える。** 理由:

1. Learn は `lpStartupInfo` を `[out]` にし、呼び出し側が cb を設定するとは書かない。0018 §決定の 11 行目（「仕様が定める範囲を超える**入力**は abort」）は、cb が**入力**であることが前提になる。Learn は引数全体を出力と書くので、未初期化の cb を「仕様を超えた入力」として止めるのは、規則の当てはめ方として弱い。
2. 同梱の DLL の CRT が cb を設定せずに呼ぶ（§4.3）。DLL は実際に配布され、Windows で動いているはず（推測。ここで実行はしていない）なので、実 kernel32 は入力の cb を要求していないと考えるのが自然（推測）。非公式の ReactOS と Wine の実装も cb を読まず書くだけ（§8。根拠にはしない）。
3. dsp の呼び出し点は呼び出し後に cb を読まない（§4.3）。cb に何を書くかは DLL の挙動を変えない。変わるのは「止まるか通るか」だけ。
4. (b) にすると CRT は `cbReserved2 = 0` の枝に入り、継承ハンドル処理は実行されない（読み）。既存の gdb の run（cb だけ書いた `CHAIN_MAX=1`）は `GetStartupInfoW` を通り、`GetFileType` は呼ばれず、次の停止（`MultiByteToWideChar`）まで進んだ。**(b) は DllMain 完走の十分条件ではない**（ロケールの 4 段が残る。§5）。

残す方がよいもの: NULL の停止（Learn に NULL の記述が無いので、0018 §決定の 11 行目に従う）、全メンバー 0（0018 の判断。変えない）、`hStd*` の 0（`dwFlags` が 0 なので Learn は「ignored」）。

(b) を取る場合の手順（意見）: 0018 §7 の改訂（作業者の判断の部分。PM 決定の部分には cb の規則が無い）→ `fle_env.c:355` のコメントと 363 行目の削除 → `env.rs` の `startup-wrong-cb` を外す（または「cb がどんな値でも 0x68 を書く」に変える）→ `stage23` §5.3 と §9 の 5 の記述の訂正（§2.4）。`stage23` §9 の 5 が書くとおり、**決定 0018 の改訂が先**。この調査ではどれも行っていない。

## 8. 非公式の参考（根拠にしない）

Microsoft の Windows の実装ではない。出典と引用: `lift/out-startupinfo/unofficial-excerpts.md`。

- ReactOS `dll/win32/kernel32/client/proc.c` の `GetStartupInfoW`: 入力の cb を読まず、`lpStartupInfo->cb = sizeof(STARTUPINFOW)` を書く。`hStd*` は `dwFlags` に STARTF_USESTDHANDLES、STARTF_USEHOTKEY、STARTF_SHELLPRIVATE のいずれかがあるときだけ書く。`cbReserved2`、`lpReserved2` は `Params->RuntimeData` から書く。
- Wine `dlls/kernelbase/process.c` の `GetStartupInfoW`: 入力の cb を読まず、`info->cb = sizeof(*info)` を書く。`lpReserved2` は `cbReserved2 != 0` のときだけ非 NULL。`hStd*` は `STARTF_USESTDHANDLES` のときだけ書く。
- ReactOS `sdk/lib/ucrt/lowio/ioinit.cpp`（ファイルの先頭に `Copyright (c) Microsoft Corporation`。Windows SDK の UCRT ソースとは未照合）: `STARTUPINFOW startup_info; GetStartupInfoW(&startup_info);` と cb を設定せずに呼び、`cbReserved2 == 0 || lpReserved2 == nullptr` なら戻る。DLL の `f_37d34b4` の命令列（§4.3）と形が一致する（照合したのは構造だけで、版は特定していない）。
- mingw-w64 ヘッダ（ローカル）: `GetStartupInfoW (LPSTARTUPINFOW lpStartupInfo)`。SAL の注記なし。

## 9. 未確認

- 実 Windows の `GetStartupInfoW` が入力の cb を読むか。Learn は書かない。ここでは Windows で実行していない。
- cb が 0x68 でない、またはバッファが 0x68 未満のときの実 Windows の挙動。
- 作成元が無いプロセスで、実 Windows が `lpReserved`、`lpDesktop`、`lpTitle`、`hStd*` に何を返すか（runtime は 0）。
- FLEngine の 0x15e6088 の読み手（レジスタ相対）。
- レジスタ相対の IAT 参照（§4.1）、`GetStartupInfoA` が他のモジュールから呼ばれる経路。
- `__acrt_*` の名前の対応は、命令列の形が一致するという読みで、DLL にシンボルは無い。
- `GetStartupInfoA` の Learn ページ（`nf-processthreadsapi-getstartupinfoa` は 404。本調査の対象外）。

## 10. 作ったファイル

文書: `docs/investigations/getstartupinfow-cb.md`（この文書）。

`lift/out-startupinfo/`（新規）:

- スクリプト（読み取り専用）: `gsi_pe_info.py`、`gsi_callsites.py`、`gsi_callers.py`、`gsi_other_paths.py`、`gsi_imm_scan.py`、`gsi_extra_checks.py`、`gsi_iat_names.py`、`gsi_annotate.py`、`gsi_layout_pdata.py`
- 出力: `pe_info.txt`、`flengine_callsites.txt`、`dsp_callsites.txt`、`flengine_callers.txt`、`dsp_callers.txt`、`flengine_other_paths.txt`、`dsp_other_paths.txt`、`imm_scan.txt`、`extra_checks.txt`、`dsp_iat_names.txt`、`flengine_refs_15e6088.txt`、`annotated.txt`、`layout_pdata.txt`、`flengine_objdump_sweep_grep.txt`、`dsp_objdump_sweep_grep.txt`、`input_sha256.txt`
- 逆アセンブル: `flengine_407330.asm`、`flengine_41ce60.asm`、`dsp_f37d34b4.asm`、`dsp_f37d36ac.asm`、`dsp_f37d7dd4.asm`、`dsp_f37d7cdc.asm`
- 引用: `learn-excerpts.md`、`unofficial-excerpts.md`
- `tmp/`（空。TMPDIR）、`__pycache__/gsi_annotate.cpython-314.pyc`（`gsi_layout_pdata.py` が `gsi_annotate` を import した時に作られた。規則で `rm` 禁止のため残している）
