# 次の停止点: user32!LoadIconW(NULL, L"MAINICON")（判断材料だけ。実装していない）

実行日: 2026-10-22。作業ディレクトリ `signature-studio-develop`、HEAD e3f54dc に決定 0067/0068 の変更を載せた状態。「実測」はコマンドの出力、「読み」は `lift/objdump-text.txt` と DLL のバイト列の静的な読み、「推測」は未検証。成果物は `lift/out-go/`（gitignore 済み）。

**実装していない。** 理由: 依頼の範囲は GetObjectW のフォントの形まで。次の停止点が何であっても、それ以外は実装せず判断材料だけを書く。

## 1. 停止点（実測。native と wasm(node) は `rsp=` の行を除いて全文一致）

```
stop   : fle: LoadIconW: lpIconName is not IDI_APPLICATION (MAKEINTRESOURCE(32512)): not answered
         top of stack: 0x4250f0 f_4250f0       (user32!LoadIconW の thunk。IAT スロット 0x19710c8)
depth  : init_count=115 / 942   last unit: index=114 run_rank=111/895 proc=0x623840 in_progress
         started=111 completed=110 in_progress=1 of 895 runnable
funcs  : executed (distinct)=627  entries=13850
```

スタック: f_4250f0（guest-return 0x867939）← f_867710 ← f_620cd0（0x620e05）← f_623840（0x62386e）← f_4120c0 ← f_412160 ← f_41d8f0 ← 入口。**a4 は 115 / 942 のまま（unit 114 の中）**。GetObjectW の前の停止（f_8651a0 の呼び出し元の f_8663b0 の GetObjectW）を越え、f_8663b0 の残り（status、menu、message、caption の f_5c1360 4 回）、f_5c1030、f_86b240 などを実行して、**別の関数 f_867710 の LoadIconW で止まった**。関数実行数は 606 → 627、入口数は 13,545 → 13,850。

## 2. 引数（実測。gdb で `fle_stop` にブレークし R を読んだ）

```
R[1] (rcx, hInstance)     = 0
R[2] (rdx, lpIconName)    = 0x867ba0   → .text 内の UTF-16 文字列 L"MAINICON"（4d 00 41 00 49 00 4e 00 49 00 43 00 4f 00 4e 00 …）
```

つまり **`LoadIconW(NULL, L"MAINICON")`**。`LoadIconW` の既存実装（決定 0054）は `hInstance == NULL` かつ `lpIconName == 32512`（IDI_APPLICATION）だけ答えるので、「lpIconName is not IDI_APPLICATION」で止まる。

## 3. 呼び出し点（読み）

`call 0x4250f0`（thunk）は 5 か所:

| 呼び出し点 | hInstance（rcx） | lpIconName（rdx） | 備考 |
|---|---|---|---|
| 0x5d26ef | `xor ecx,ecx` | `0x7f00`（32512 = IDI_APPLICATION） | 決定 0054 が答えた（`gdi-objects-caller.md` §2.3） |
| **0x867934（今回の停止）** | `[[0x15e15c8]]`（**実測で 0**） | `lea rdx,[rip+0x26c]` = 0x867ba0 = L"MAINICON" | f_867710。戻り値は `mov rdx,rax; mov rcx,rbx; call 0x5d03e0` に渡る |
| 0x86969c | `xor ecx,ecx` | `0x7f00` | `call 0x5cfff0` が 0 を返したときのフォールバック（IDI_APPLICATION） |
| 0x87fc44 | `xor ecx,ecx` | `rdi`（呼び出し元の引数） | 0x87fc3c で `call 0x5c7f20` の結果を r13 に退避 |
| 0x9919ba | `xor ecx,ecx` | `[rax+0x680]`（オブジェクトのフィールド） | |

- **hInstance の出どころ**（実測 + 読み）: gdb で停止時に読んだ（`out-go/gdb-hinst.txt`）: `*(u64*)0x15e15c8 = 0x15e6078`（.data の初期値。DLL のファイルの 0x11dffc8 にも同じ値。ポインタ）、`*(u64*)0x15e6078 = 0`（.bss。**停止の時点で 0**）。つまり hInstance は「`0x15e15c8` が指す変数 `0x15e6078` の中身」で、それが **0**。`0x15e6078` への RIP 相対の参照は DLL に **1 か所だけ**（0x412270 の `cmp qword [0x15e6078],0` = 読み。先行調査 `import-none-0x40a3b0.md` と `mxcsr-portable-run.md` が同じ命令を扱う）で、**直接の書き込みは 0 か所**（.text の RIP 相対 store の走査）。ほかの 3 か所（0x61157d、0x867923、0x989c09）は `[0x15e15c8]` を読んで二重参照する。**この変数は Delphi/Free Pascal の `HInstance` に当たる**（二重参照の形から。推測）。Windows ローダーが DLL を読むとき、その RTL の初期化が DllMain の `hinstDLL` を保存するのが通常（推測。未確認）。**この環境の DllMain の実行では、保存する経路が実行されていない**（0x412270 は 0 なら 0x41227a 以降で MXCSR を保存する分岐。実行は `0x15e6078 == 0` の側を通った）。

## 4. LoadIconW の既存の規則と、この呼び出しが違う点

- Learn（`LoadIconW`、2026-10-22 取得は 0054 の記録）: hInstance が NULL なら lpIconName は IDI_ の識別子、そうでなければ「a handle to the instance of the module whose executable file contains the icon」で lpIconName は「the name of the icon resource」（文字列。または MAKEINTRESOURCE の識別子）。
- **今回の hInstance は NULL なのに lpIconName が文字列**: Learn に従えば、hInstance が NULL で lpIconName が文字列の場合の定義は書かれていない（NULL は「事前定義のシステムアイコン」の意味で、識別子 IDI_ だけ）。**実 Windows ならおそらく失敗（NULL を返す）**か、現在のプロセスの実行ファイル（FLEngine の DLL ではなくホストの exe）のリソースを探す。ゲストが hInstance = 0 を渡しているのは、`HInstance` が DllMain で設定されていない（実行がまだ DllMain の途中）ためかもしれない（推測）。
- 戻り値の使われ方（読み）: 0x867939 の後 `call 0x5d03e0(rbx, rax)`（f_5d03e0）が hIcon を受ける。

## 5. 選択肢（判断は PM）

- **A. hInstance = NULL、lpIconName = 文字列を NULL（失敗）で返す**。last error はページが書かない。ゲストがその後 NULL をどう扱うか（f_5d03e0）は未確認。「Learn が書かない形を答える」ことになるので 0054 の規則（書かない形は停止）に反する。
- **B. `HInstance`（0x15e6078）が設定されない理由を調べる**。実 Windows では DllMain の hinstDLL がどこかに保存されるはず（推測）。**ゲストの DllMain からの経路で、その書き込みが今の実行で行われていない**のなら、問題は LoadIconW ではなく、その前の値（0）の方にある。0x15e6078 への書き込みを動的に調べる（gdb の watchpoint）のが先。
- **C. 停止のまま**。
- **D. FLEngine のリソースの "MAINICON"（RT_GROUP_ICON。決定 0025 の `docs/decisions/0025-resource-apis.md:57` に、DLL に `MAINICON`（名前側）があると書かれている）を、仮想アイコンとして答える**。hInstance が DLL の HMODULE（仮想）であれば LoadIconW の規則に合う。別の表（アイコンハンドル、リソース読み込み）が必要で、0054 と 0061（LoadImage）にまたがる。

推奨（推測）: **B を先にやる**。hInstance が 0 のまま来ていること自体が、ゲストの初期化の前提（DllMain が渡す hinstDLL の保存）が、この環境で成り立っていない可能性を示す。`0x15e6078` への直接の書き込みが DLL に無い（RIP 相対の走査）ので、書き込みは間接（`mov [rax],rcx` で rax がこの番地。`0x15e15c8` 経由）か、RTL の初期化（Delphi/FPC なら `Sysinit` の `InitLib`。DllMain 相当の入口 0x1376820 の引数 rcx = `hinstDLL`、実測 `calling entry (no export name) rcx=0x400000`）にある。**この環境の harness は入口を `rcx=0x400000 rdx=1 r8=0` で呼んでいる**ので、hinstDLL = 0x400000 が渡っているのに `HInstance` が 0 のままなのは、保存の経路が実行されていないか、別の値（`[0x15e15c8]` 経由）を読んでいる、のどちらか。値を創作する前に、書き込み元を確認する（gdb の watchpoint で `0x15e6078` を監視。今回はやっていない）。

## 6. 限界

- `0x15e6078` の書き込みは、直接の RIP 相対では 0 か所（読み 1 か所: 0x412270。走査は mod=00 rm=101 の 0x80/0x81/0x83/0x88/0x89/0x8a/0x8b/0x8d/0xc6/0xc7/0x38〜0x3b の形だけ）。間接書き込み（`mov [rax],rdx` で rax がこの番地）と、`0x15e15c8` を介した書き込みは追っていない。watchpoint は置いていない。
- f_867710 の全体（0x867710〜）と f_5d03e0 は読んでいない。
- 実 Windows の `LoadIconW(NULL, "MAINICON")` の戻り値は、Learn にも書かれず、比べていない。
- 呼び出し点 0x87fc44 と 0x9919ba の引数の文字列は追っていない（今回の停止ではない）。
