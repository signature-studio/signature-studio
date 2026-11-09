# LoadIconW(NULL, L"MAINICON") の hInstance = 0 の出どころ（調査のみ。コードは変更していない）

実行日: 2026-10-22。HEAD a86b1cd。「実測」はコマンドの出力（gdb、a4-survey）、「読み」は DLL のバイト列と `objdump` の静的な読み、「文書」は Learn / Embarcadero の記載、「推測」は未検証。成果物は `lift/out-hi/`（gitignore 済み）: `gdb-watch.cmd/.txt`、`gdb-vals.cmd/.txt`、`gdb-null.cmd/.txt`、`scan_ptr.py`、`scan_ptr2.py`、`scan_reloc.py`、`objdump-tail.txt`、`a4-survey-run.txt`、`a4/`。

## 0. 結論

**分類は (c) ゲストの設計どおり。** 停止の引数 `hInstance = [[0x15e15c8]] = *(u64*)0x15e6078 = 0` は、lift の欠陥でも harness / runtime の欠陥でもない。

- `0x15e6078` は **`HInstance` ではない**。Delphi の **`MainInstance`**（「メインの実行ファイルのインスタンスハンドル」）に当たる（読み + 文書。§3）。DLL では `_InitExe` が走らないので、値は 0 のままになる。
- ゲストの `HInstance` は **`0x15ec298`**。DllMain の hinstDLL（`rcx = 0x400000`）を `f_41d8f0`（0x41d919）が保存しており、gdb で `0x400000` を実測した（§2）。
- 前の調査の「`0x15e6078` は `HInstance` に当たる（推測）」は **誤りだった**。§4 の選択肢 B（「HInstance が設定されない理由を調べる」）は成り立たない。
- LoadIconW の引数 `(NULL, L"MAINICON")` を直すものは無い。hInstance が 0 なのは、ホスト exe が Delphi でない（MainInstance を設定しない）実機でも同じと読める。**判断が要るのは LoadIconW の答え方だけ**（§5）。

## 1. 書き込み元の調査（最優先の依頼）

### 1.1 動的: gdb のウォッチポイント（実測）

harness の image が 0x400000 そのものにあることの確認。`a4_set_image` で止めて `*(unsigned short*)0x400000` を読むと `0x5a4d`（MZ）。`image.c:101` も `MAP_FIXED_NOREPLACE` で ImageBase に mmap し、ずれたら失敗にしている。つまり native では VA == リニアアドレスで、gdb の式に VA をそのまま使える。

コマンド（`lift/out-hi/gdb-watch.cmd`）:

```
break a4_set_image
run binaries/win64/FLEngine_x64.dll > /dev/null 2> /dev/null
awatch *(unsigned long*)0x15e6078          # 読み書き両方
commands 2 ... printf pc/value; bt 4; continue ... end
break fle_stop
continue
```

実行: `gdb -q -batch -x lift/out-hi/gdb-watch.cmd --args lift/out-go/native/flengine-native binaries/win64/FLEngine_x64.dll`（バイナリは前の作業者の `out-go/native`。生成 C は HEAD と同じ）。出力（`lift/out-hi/gdb-watch.txt`、要点）:

```
image mapped: *(u64*)0x15e15c8=0x15e6078 *(u64*)0x15e6078=0
Hardware access (read/write) watchpoint 2: *(unsigned long*)0x15e6078
ACCESS at pc=...ab7 value now=0   #1 fle_ld (a=22962296 = 0x15e6078, n=8)  #2 f_412160 () ...part-00.c:132258   #3 f_41d8f0
ACCESS at pc=...aba value now=0   （同じ fle_ld の memcpy の 2 回目のアクセス）
ACCESS at pc=...ab7 value now=0   #1 fle_ld (0x15e6078, 8)  #2 f_867710 () ...part-05.c:85257   #3 f_620cd0
ACCESS at pc=...aba value now=0
Breakpoint 3, fle_stop (api="LoadIconW", why="lpIconName is not IDI_APPLICATION ...")
STOP: *(u64*)0x15e6078=0
```

- バックトレースの `fle_ld`（読み）しか出ない。`fle_st` 経由の書き込みは **1 回も無い**。ヒットは 2 地点の読みだけ: `f_412160` の C 132258 行 = 0x412270 の `cmp qword [0x15e6078],0`、`f_867710` の C 85257 行 = 0x86792a の `mov rcx,[rax]`（rax = `[0x15e15c8]` = 0x15e6078）。
- 起動から停止点まで、値は 0 のまま。**書き込みは無い（事実）**。ハードウェアウォッチポイントは、プロセス内のメモリへの store を全て捕まえる（lift した store は全て実メモリへ書く）。

### 1.2 静的: 書き込みの走査（読み）

- `.text` の RIP 相対の `0x15e6078` 参照は **1 か所だけ**（0x412270、読み）。`objdump-text.txt` は 0x11fb4ae で途切れているので、残り（0x11fb400〜`.text` の終端 0x13a84b0）を `objdump -d` で別に出して（`lift/out-hi/objdump-tail.txt`、488,983 行）、`0x15e6078` と `0x15e15c8` の参照を grep した。**0 件**。したがって `.text` 全体で 1 か所。
- `0x15e15c8` を読む `mov rax,[rip+disp]` は **4 か所だけ**（0x61157d、0x867923、0x867987、0x989c09。前の記録は 0x61157d、0x867923、0x989c09 の 3 か所としていて、0x867987 を挙げていなかった）。直後の命令（読み）:

| 場所 | 直後 | 書き込み |
|---|---|---|
| 0x61157d | `mov rcx,[rax]` → `call 0x424ba0`（GetClassInfoW）の hInstance 引数 | なし |
| 0x867923 | `mov rcx,[rax]`、`lea rdx,0x867ba0`、`call 0x4250f0`（LoadIconW） | なし（今回の停止） |
| 0x867987 | `mov rcx,[rax]`、`lea rdx,[rbp+0x48]`、`mov r8d,0x100`、`call 0x4235a0`（GetModuleFileNameW） | なし |
| 0x989c09 | `mov rax,[rax]`、`mov [rbp+0x58],rax`（ローカルに退避）、`call 0x4233a0`（FindResourceW） | なし（`[rbp+0x58]` へ書くだけ） |

  `mov [rax…],reg` の形（`rax` が 0x15e6078 を指す書き込み）は、どの 4 か所の直後 10 命令以内にも無い。いずれも `[rax]` の**読み**で、その値を API の第 1 引数にするだけ。
- 0x15e15c8 は `.data` のポインタ表（`+0 = 0x15e6078`、`+8 = 0x5b6f20`（.text 内）、`+0x10 = 0x15f28a8`、…）。ファイル内で 8 バイトの `0x15e6078` は `.data` の 0x15e15c8 に **1 回だけ**（`scan_ptr.py`）。4 バイトの一致も同じ位置だけ。`0x15e15c8` 自体を値にもつポインタは無い。`.reloc` にも 0x15e6078 付近のエントリは無い（`scan_reloc.py`: 該当 `[]`）。
- 経路が静的に見えない書き込み（別の変数のアドレスを計算して `0x15e6078` に当てる、のような形）は走査していない（限界）。ただし §1.1 の動的な実測で、**この実行では書き込みが 0 回**と確定している。

## 2. ゲストの本来の初期化経路（実測 + 読み）

入口（`CreateFruityInstance` のほか、エントリの DllMain 相当は 0x1376820。export は `CreateFruityInstance` 0x1375990、`__dbk_fcall_wrapper` 0x41d5a0、`dbkFCallWrapperAddr` 0x15ec2b0 の 3 つで、DllMain は名前なし）。

```
0x1376820 (rcx=hinstDLL, edx=reason, r8=reserved):
  mov [rbp+0x30],rcx ; mov [rbp+0x3c],edx ; mov [rbp+0x40],r8
  call 0x412760        ; 戻り eax
  ...
  1376858 lea rdx,[0x13768b0]        ; InitTable ヘッダ（942 unit）
  137685f mov r8,[rbp+0x30]          ; r8  = hinstDLL
  1376863 mov r9d,[rbp+0x3c]         ; r9d = reason
  1376867 mov rax,[rbp+0x40] ; mov [rsp+0x20],rax    ; 第 5 引数 = reserved
  1376870 call 0x41d8f0              ; = f_41d8f0(rcx=&ctx, rdx=table, r8=hinst, r9d=reason, [rsp+0x20]=reserved)

0x41d8f0 (_InitLib 前段):
  mov rbx,rcx ; mov rsi,rdx ; mov rdi,r8 ; mov r13d,r9d
  cmp r13d,1 ; jne 41d94d             ; reason == DLL_PROCESS_ATTACH のとき
  41d90b mov byte [0x15ec290],1       ; ModuleIsLib 相当（推測）
  41d912 mov byte [0x15ec2d8],1
  41d919 mov [0x15ec298],rdi          ; ★ HInstance = hinstDLL
  41d920 mov [0x13a93b8],rdi          ; ★ TLibModule.Instance（推測）= hinstDLL
  41d927/32 mov qword [0x13a93c0],0 ; [0x13a93c8],0
  ...
  41d984 call 0x412160                ; f_412160 (_InitLib 本体)

0x412160 (_InitLib):  ... 0x412270 cmp qword [0x15e6078],0 ; jne 41228c
                          41227a mov byte [0x15e6084],1   ; 0 のときだけ
                          412281 call 0x40a3b0            ; stmxcsr → eax
                          412286 mov [0x13a5054],eax
```

- **harness の呼び出し**（`harness/main.c:63-66`、`wasm/main.c:54-59`）: `R[1] = img.base`（0x400000）、`R[2] = 1`、`R[8] = 0`。`R[9]` は設定していない。DllMain は `rcx = hinstDLL`、`edx = reason`、`r8 = reserved` なので、**引数は正しい**（reason = DLL_PROCESS_ATTACH = 1、reserved = NULL。`calling entry (no export name) rcx=0x400000 rdx=1 r8=0` と一致）。
- **保存先の実測**（`gdb-vals.cmd`）: `watch *(unsigned long*)0x15ec298` が `WRITE 0x15ec298 -> 0x400000`（`f_41d8f0`、C 145240 行）、`watch 0x13a93b8` が `WRITE 0x13a93b8 -> 0x400000`（C 145241 行）。停止時: `0x15ec298=0x400000 0x13a93b8=0x400000 0x15e6078=0 0x15ec290=0x1 0x15ec2d8=0x1`。**hinstDLL は正しく保存されている。保存先は 0x15e6078 ではない**。
- **0x15ec298 が HInstance である根拠**（読み）: これを読む箇所は、hInstance を要求する API の引数に使う。0x448956: `mov rcx,[0x15ec298]; lea rdx,[rbp+0x2ec]; mov r8d,0x105; call 0x4235a0`（GetModuleFileNameW(HInstance, buf, MAX_PATH)）。0x611566: `mov rcx,[0x15ec298]; … call 0x424ba0`（GetClassInfoW(HInstance, name, wc)）。0x605920 は `mov rax,[0x15ec298]; ret`（`HInstance` の読み出し関数に見える）。
- **0x15e6078 の相手**（読み）: 0x611550 の関数は、`GetClassInfoW(HInstance, …)`、失敗したら `GetClassInfoW([[0x15e15c8]] = MainInstance, …)`、さらに失敗したら `GetClassInfoW(NULL, …)`、の順に探す。Delphi の VCL が「HInstance → MainInstance → NULL（システムクラス）」の順に探す形に一致する。0x867923 / 0x867987 は VCL の `TApplication` の中（`LoadIcon(MainInstance, 'MAINICON')`、`GetModuleFileName(MainInstance, …)`）に当たる（推測。名前は DLL のシンボルに無いので、形からの読み）。`0x15e15c8` は「他モジュールの変数（System の `MainInstance` など）を指す間接参照表」に見える（推測）。
- **`MainInstance` は DLL では 0**（文書 + 読み）: Embarcadero の System ページは `MainInstance` を「Indicates the instance handle for the main executable.」、`HInstance` を（Writing Dynamically Loaded Libraries で）「During a library's lifetime, HInstance contains its instance handle.」と書く。RTL の `_InitExe` が `MainInstance` を設定するのは EXE のとき（推測。RTL のソースは未確認。取得できた KOL の SysInit のコピーは古い 32 ビット版で、`MainInstance` は載っていなかった）。**この DLL の `_InitLib` が `cmp [0x15e6078],0` で 0 のときだけ `[0x15e6084] = 1` を立てるのは（実行済み。0x15e6084 を書く箇所は 0x41227a だけ）、0 が DLL の通常の状態（IsLibrary 相当の判定）という形に合う**（推測）。

## 3. 分類

**(c) ゲストの設計どおり。**

- (a) lift の欠陥ではない: ゲストに「hinstDLL を 0x15e6078 に書く命令」は無い（`.text` 全体の静的走査と、起動から停止までの動的な実測の両方で 0 回）。hinstDLL を書く命令（0x41d919、0x41d920）は正しく実行されて 0x400000 を保存している。
- (b) harness / runtime の欠陥ではない: 入口の引数は正しい。`GetModuleHandleW(NULL)` などの runtime は、この値の経路に関与していない。値は `.bss` の 0 のまま誰も触らない。
- 実機でも、DLL が MainInstance を設定せず、`LoadIconW(NULL, L"MAINICON")` が同じ形で呼ばれる、と読める。ただしホストの FL64.exe が Delphi の RTL で、同じ変数を共有するなら別（推測。FLEngine は自前の RTL を持つ（DLL の中に System/SysInit がある）ので、共有しない）。

## 4. 実機の LoadIconW(NULL, 文字列)（Learn を再取得。2026-10-22）

https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-loadiconw より（原文）:

- 「`[in, optional] hInstance` … A handle to the module of either a DLL or executable (.exe) file that contains the icon to be loaded. … **To load a predefined system icon, set this parameter to NULL.**」
- 「`[in] lpIconName` … If *hInstance* is non-NULL, *lpIconName* specifies the icon resource either by name or ordinal. … **If *hInstance* is NULL, *lpIconName* specifies the identifier (beginning with the IDI_ prefix) of a predefined system icon to load.**」
- 戻り値: 「If the function succeeds, the return value is a handle to the newly loaded icon. **If the function fails, the return value is NULL.** To get extended error information, call GetLastError.」

**書かれていないこと**: hInstance が NULL で lpIconName が IDI_ 以外の**文字列**のときの結果。このページは「NULL のときは IDI_ の識別子」としか書かず、文字列が来たときに失敗するとも、呼び出し元の exe を探すとも書いていない。実機の戻り値は、この調査では確認できない（推測: NULL が返る。根拠は「文字列名の事前定義アイコンが無い」ことだけで、文書の根拠ではない）。

## 5. LoadIconW の次の判断材料（選択肢 A〜D の再評価。判断は PM）

ゲスト側の読み（`f_5d03e0`、0x5d0411）: `LoadIconW` の戻り値 hIcon を受ける関数は、`cmp qword [rbp+0x98],0; je 0x5d0470` で **hIcon が NULL なら設定をスキップ**して通常の戻りに進む（読み）。また 0x86969c の別の呼び出し点は、`call 0x5cfff0` が 0 を返したときに `LoadIconW(NULL, IDI_APPLICATION)` に落とす。つまり**ゲストは LoadIcon の失敗（NULL）を想定している**（読み）。

- **B は削除。** 「HInstance が設定されない理由を調べる」は、HInstance は設定されている（0x400000）ので成り立たない。前の作業者の推測（0x15e6078 = HInstance）は誤り。
- **A. NULL を返す（失敗）。** 根拠: ゲストは NULL を処理する（上）。実機でも `MainInstance` が 0 なら同じ形で呼ばれる。Learn は戻り値「失敗なら NULL」を書く。問題: 「hInstance が NULL で lpIconName が文字列」の結果は Learn が書かない。0054 の「書かない形は停止」の規則に反する。last error をどうするかも書かれていない（GetLastError の値は未確認。触らない、が最小）。**確認用の探り**（`lift/out-hi/gdb-null.cmd`。gdb で `fle_imp_LoadIconW` を R[0]=0 で即 return。実装の変更ではない）: 0x867934 の後は次の停止まで進み、**次の停止は `GetClassInfoW`（IAT スロット 0x1970a90、thunk 0x424ba0）の未実装 import（`f_52a7f0` から。引数 rcx=0x400000、rdx=0x52a7c8）**。この探りでは、途中の GetModuleFileNameW(NULL)（0x867987）が呼ばれて答えられたかは、スタックの途中を見ていないので未確認（`f_52a7f0` は別の呼び出し元で、`f_867710` の続きかも確認していない）。
- **C. 停止のまま。** 根拠: 規則どおり。問題: 次に進めない。上の探りは、NULL なら先にも進める（GetClassInfoW が次）ことを示す。
- **D. MAINICON（DLL の `.rsrc`、決定 0025 の RT_GROUP_ICON）を仮想アイコンとして返す。** hInstance = NULL なので、**実機の規則（NULL = システムアイコン）に合わない**。ゲストの意図は「ホスト exe の MAINICON」（MainInstance はホストの exe。DLL の MAINICON ではない）。ホスト exe（FL64.exe）の MAINICON の有無は分からない。DLL の MAINICON を返すのは、実機が返すもの（おそらく NULL）と食い違う可能性がある。別の表（アイコンハンドル、リソース読み込み）が要り、0054 / 0061 にまたがる。推奨しない。
- **E（新規）. `MainInstance` を DLL とは別の値にする、ではない**: `[0x15e6078]` を runtime が書いて 0x400000 にするのは、ゲストの設計（DLL では 0）を変える。IsLibrary 相当の判定（0x412270）も変わる（0x15e6084 が立たない、MXCSR の保存が飛ぶ）。しない。

推奨（推測）: **A（NULL を返し、last error は触らない）を、「文書に書かれない形」として決定に明記し、撤回条件（実機の実測ができたら）を付ける**。または、この形のときだけ停止を残して PM が判断する。どちらも実装は今回していない。

## 6. 周辺の発見（今回は実装していない。次の停止の予告）

- `0x15e6078`（`MainInstance`）が 0 のままなので、**GetModuleFileNameW(NULL, …) が 0x867987 で来る**。runtime は NULL に virtual DLL path を返す（決定 0018 の値。実機では「ホストの exe のパス」。Learn: hModule が NULL なら「the file used to create the calling process」）。値の食い違いは既に 0018 の決定の範囲。
- `FindResourceW(MainInstance = NULL, …)`（0x989c09 → 0x4233a0）も 0 で来る。決定 0025 の NULL の扱い（`fle_load.c`）を確認していない。
- 次の停止の予想（探りの結果）: GetClassInfoW（`f_52a7f0`、0x424ba0）。Delphi の `RegisterClass` の前の `GetClassInfo` で、クラスの有無の問い合わせ。クラス表（RegisterClassW の仮想）の設計が要る（決定の対象）。

## 7. a4 と native / wasm（実測。`lift/out-hi/a4-survey-run.txt`）

`OUT=lift/out-hi WASM=1 bash lift/harness/a4-survey.sh`（ビルドは前の作業者の `out-go` の native と wasm を `NATIVE_BIN`、`FLE_WASM_DIR`、`FEPE` で指定して再利用。コードを変えていないので再生成なし）。終了コード 0。

```
stop   : fle: LoadIconW: lpIconName is not IDI_APPLICATION (MAKEINTRESOURCE(32512)): not answered
         top of stack: 0x4250f0 f_4250f0
depth  : init_count=115 / 942   last unit: index=114 run_rank=111/895 proc=0x623840 in_progress
         started=111 completed=110 in_progress=1 of 895 runnable   all_units_done=no
funcs  : executed (distinct)=627  entries=13850
wasm   : same a4 lines as native (exit 134)
```

停止点は変わらない（コードを変えていない）。native と wasm は a4 の 6 行とも一致。

## 8. 確定できなかったこと（限界）

- 実機の `LoadIconW(NULL, L"MAINICON")` の戻り値と last error（Learn に無い。実機の実測も無い）。
- `0x15e6078` が `System.MainInstance` であることは、使われ方（`HInstance` → `MainInstance` → NULL の探索順、`TApplication` に当たる文脈、`_InitLib` の IsLibrary 相当の分岐）からの**読みと推測**。DLL にシンボル名は無く、RTL のソースとの対応は取っていない（Embarcadero の System / SysInit ページは変数の説明だけで、`MainInstance` が EXE のときだけ設定されることの明文は取得できなかった: docwiki の `System.MainInstance` は 403）。
- `.text` の走査は objdump の逆アセンブルによる（データがコードに混じる箇所の誤りは未確認）。間接の計算経由の書き込みは静的には追っていない。動的な実測（書き込み 0 回）が確定の根拠。
- ホスト（FL64.exe）側の MAINICON の有無、LoadIconW の戻り値が実機で何になるか。
- 0x867987 以降（GetModuleFileNameW(NULL) の後の `f_867710` の残り）と GetClassInfoW の先は読んでいない。
