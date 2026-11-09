# LoadLibraryW("Msctf.dll") の呼び出し元（f_5de9e0、f_44db40）の静的調査と、msctf.dll を仮想モジュールにした後の harness の再実行の記録（決定 0055）

実行日: 2026-10-21。作業ディレクトリ `signature-studio-develop`、HEAD 11cbdb6。DLL は `binaries/win64/FLEngine_x64.dll`。決定は `docs/decisions/0055-msctf-loadlibrary.md`。「実測」はコマンドの出力、「読み」は `lift/objdump-text.txt` の逆アセンブルの静的な読み、「推測」は未検証。成果物は `lift/out-msc/`（gitignore 済み。`TMPDIR=lift/out-msc/tmp`、`EM_CACHE=lift/out-msc/emcache`、`CARGO_TARGET_DIR=lift/out-msc/target`）。補助スクリプトは `lift/out-msc/work/`（`sites44.py` = f_44db40 の呼び出し点の引数、`nullchk.py` = LoadLibrary 系の呼び出し点と直後の命令、`mkmut.py` = 変異）。

## 1. ゲストの経路（読み。`f_5de9e0` と `f_44db40`）

### 1.1 f_44db40（0x44db40。LoadLibraryW の包み）

```
44db4c  mov [rbp+0x50],rcx        ; 名前
44db50  mov [rbp+0x58],edx        ; モード（呼び出し元は 0x8000）
44db53  mov ecx,[rbp+0x58]
44db56  call 0x423b70             ; kernel32!SetErrorMode（thunk。実測: pefile の IAT）
44db5b  mov [rbp+0x3c],eax        ; 直前のモードを保存
44db5f  mov rcx,[rbp+0x50]
44db63  call 0x414850             ; rcx が NULL ならグローバルの文字列（[0x13a8040]+0x10）に置き換える、なければそのまま
44db68  mov rcx,rax
44db6b  call 0x423990             ; kernel32!LoadLibraryW
44db70  mov [rbp+0x30],rax
44db75  mov ecx,[rbp+0x3c]
44db78  call 0x423b70             ; SetErrorMode(元のモード)
44db7d  mov rax,[rbp+0x30]         ; LoadLibraryW の結果をそのまま返す
```

- **SetErrorMode の形**: 引数は `0x8000`（SEM_NOOPENFILEERRORBOX）だけを入れて呼び、戻り値（直前のモード）を保存して、LoadLibraryW の後に**そのまま戻す**。戻り値の使われ方はこれだけ（読み）。`fle_proc.c` の `fle_imp_SetErrorMode` は応答済み（決定 0020。0x8000 を受け、直前のモードを返し、LoadLibraryW の後の復元も同じ関数）。今回の停止はその後の LoadLibraryW だった（実測: 停止時の呼び出しスタックが `f_423990` ← `f_44db40`）。
- LoadLibraryW の結果は**検査せずに返す**（NULL でも非 NULL でも同じ）。判断は呼び出し元。

### 1.2 f_5de9e0（0x5de9e0。Msctf の遅延ロード）

```
5de9e4  cmp qword [0x13d9ef8],0
5de9ec  jne 0x5dea06             ; すでに値があれば何もしない
5de9ee  lea rcx,[0x5dea18]       ; L"Msctf.dll"（UTF-16。実測: ファイルの 0x1dde18）
5de9f5  mov edx,0x8000
5de9fa  call 0x44db40
5de9ff  mov [0x13d9ef8],rax      ; 結果（NULL でも）を保存
```

- 保存先 `0x13d9ef8` はファイル上 0（実測。.data/.bss の初期値）。NULL を保存すると、**次に読むときまた `== 0` になって再び LoadLibraryW を試みる**（キャッシュされない。読み）。
- 呼び出し元は 2 つ（読み）。**f_5deae0**（0x5deae0。カウンタ `0x15f31c4` を 1 減らし、結果が -1（初回）のとき 0x5deaf4 で f_5de9e0 を呼ぶ。静的初期化の片割れ。実測: 停止時の呼び出しスタックが `f_5de9e0` ← `f_5deae0`（guest-return 0x5deaf9）← 0x412129。つまり**起動時の初期化として Msctf を読み込む**）と、f_5dea70（0x5dea7e。`[0x13d9ef8] == 0` のときの遅延呼び出し）。対になる f_5deac0（カウンタを 1 増やし、0 になったとき f_5dea30 = FreeLibrary を呼ぶ。終了処理側）は、停止点の先。

### 1.3 `[0x13d9ef8]` の読み手（読み。6 か所）

| 場所 | 関数 | 内容 |
|---|---|---|
| 0x5dea34〜0x5dea45 | f_5dea30 | `if ([0x13d9ef8] > 0) FreeLibrary([0x13d9ef8])`（`jbe` の符号なし比較。NULL なら何もしない）。f_5deac0 が「カウンタが 0 になったとき」に呼ぶ（カウンタ `0x15f31c4`）。 |
| 0x5dea74〜0x5deaab | **f_5dea70** | `if ([0x13d9ef8]==0) f_5de9e0();` その後 `[0x13d9ef8] > 0` なら関数ポインタ `[0x13d9f00]` に **0x5dea50（`mov al,1; ret`）**、そうでなければ **0x5dea60（`xor eax,eax; ret`）** を入れて呼ぶ。つまり**「Msctf があるか」を返す関数**。 |
| 0x5e0fb2〜0x5e0fbb | f_5e0fb0（Msctf の「ある」を問う入口） | `mov rax,[0x15dfe48]; call [rax]`。`0x15dfe48` の中身は 0x13d9f00（実測: ファイルの 8 バイト）、`0x13d9f00` の**ファイル上の初期値は 0x5dea70 = f_5dea70 自身**（実測）。つまり関数ポインタは最初 f_5dea70 を指し、f_5dea70 が「0 を返すスタブ」か「1 を返すスタブ」に**自分で書き換える**。f_5e0fb0 は f_620b40（0x620b40）から呼ばれ、戻りの `al` が 0 でなければ f_5e0f30（SetInputScope）の経路に進む（`test al,al / je`、0x620b45〜0x620b47）。 |

`call 0x5dea70`（直接）は逆アセンブル上に無く、f_5dea70 へは 0x13d9f00 経由の間接呼び出しだけで届く（読み）。f_5e0fb0 の位置は直前のデータ（`SetInputScope` の UTF-16 文字列の終端、0x5e0faf まで）の後ろで、逆アセンブルの上では命令の途中から始まって見えるが、`0x620b40` の `call 0x5e0fb0` の飛び先が 0x5e0fb0 であり、そこから 0x5e0fb2 の `sub rsp,0x28`（`48 83 ec 28`）が読める（バイト列: 0x5e0faf `00` + 0x5e0fb0 `48 83` + 0x5e0fb2 `ec 28`）ので、実際には 0x5e0fb0 が関数の先頭（読み）。

### 1.4 NULL のとき・非 NULL のとき（読みで確定した分岐の違い）

| 戻り値 | 保存される値 | 以後の分岐 |
|---|---|---|
| **NULL** | `[0x13d9ef8] = 0` | f_5dea70 は 0x5dea60 を入れて呼ぶ = **「Msctf なし」を 0 で返す**。エラーにはならない。後続の初期化を飛ばす（GetProcAddress を一度も呼ばない。f_5de9e0 自体には GetProcAddress が無い）。FreeLibrary は呼ばれない（`> 0` の検査）。次の f_5de9e0 は再び LoadLibraryW を試みる。 |
| **非 NULL** | `[0x13d9ef8] = HMODULE` | f_5dea70 は 0x5dea50 を入れて呼ぶ = **「Msctf あり」を 1 で返す**。**この関数の中では GetProcAddress を呼ばない**。 |

**GetProcAddress は別の関数**（f_5e0f30、0x5e0f30）にある（読み）:

```
5e0f3b  cmp qword [0x13da2d8],0
5e0f43  jne 0x5e0f6f                    ; すでに引いてあれば引き直さない
5e0f45  mov rax,[0x15e4160]             ; 0x15e4160 の 8 バイトは 0x13d9ef8（実測）。つまり rax = &[0x13d9ef8]
5e0f4c  cmp qword [rax],0               ; Msctf のハンドルが 0 か
5e0f50  je 0x5e0f6f                     ; NULL ならスキップ（GetProcAddress を呼ばない）
5e0f59  mov rcx,[rax]
5e0f5c  lea rdx,[0x5e0f94]              ; L"SetInputScope"（UTF-16。実測: ファイルの 0x5e0f94）
5e0f63  call 0x4235f0                   ; GetProcAddress の包み
5e0f68  mov [0x13da2d8],rax
5e0f6f  cmp qword [0x13da2d8],0
5e0f76  je 0x5e0f86
5e0f7e  call [0x13da2d8]                ; hwnd=rcx、inputscope=edx をそのまま渡す
5e0f84  jmp 0x5e0f8b
5e0f86  mov eax,0x80004005              ; E_FAIL
```

- **引く名前は `SetInputScope` の 1 つだけ**（実測: 同じ DLL に `Msctf` の UTF-16 は 0x1dde18 の 1 か所、`SetInputScope` の UTF-16 は 0x1e0394 と 0x5e0f94 の 2 か所。0x1e0394 は未追跡で、別の読み手の可能性がある）。0x4235f0 は `jmp [0x1971600]`（= kernel32!GetProcAddress、実測: IAT）の thunk 0x4235e0 を呼ぶ包みで、`rdx >> 16 == 0` なら序数として扱い、そうでなければ名前を ANSI に直して呼ぶ（読み。0x423609〜0x423649）。
- 戻り値の使われ方: ポインタを `[0x13da2d8]` に保存し、NULL でなければ `call` する。**NULL のときは E_FAIL（0x80004005）を返す**。この HRESULT の呼び出し元（f_620b40 の `call 0x5e0f30`、0x620b5a）はこの結果を `jmp 0x620bbb`（戻る）に直結していて、検査しない（読み）。
- 呼び出しの届き方: f_5e0f30 の呼び出し点は 0x620b5a の 1 か所（f_620a80 の中、f_5e0fb0 が真を返したとき。f_620a80 の呼び出し元は 0x614f71 の 1 か所）。f_620a80 は引数の `sil` が 3 のとき何もしない（0x620a91）、`[0x13e2ab8]`（imm32 のハンドル）が 0 のとき何もしない（0x620a9b）、など、複数の分岐の奥にある。今回の停止点の先で、f_620a80 と f_5e0f30 に実行が届くかは確認していない。
- **終了処理側の注意（読み）**: f_5dea30 は `FreeLibrary([0x13d9ef8])` を呼ぶ（f_5deac0 経由）。msctf.dll は起動時に 1 回ロードされた形（参照 1 → LoadLibraryW で 2）なので、FreeLibrary は 1 に戻るだけで、`fle_load.c` の「参照が 0 になる」停止には当たらない（テスト `free_library_follows_the_reference_count_of_the_new_module`）。

### 1.5 まとめ（質問 (a)(b) の読み）

- (a) **ゲストは NULL を明示的に許容する**。LoadLibraryW の結果 NULL を f_5dea70 が検査し、0 を返すスタブに切り替える。f_5dea30 も `> 0` を検査する。f_5e0f30 も `[0x13d9ef8]` が 0 のとき GetProcAddress を呼ばず E_FAIL を返す。これは読みで確定（実行はしていない）。
- (b) NULL の経路は「Msctf がない環境」の設計として自然。ただし**それは「あるかもしれないし、ないかもしれない」を確かめる設計**で、Windows 10 に msctf.dll が無い場合を想定したものではない。MSDN の SetInputScope ページの要件は「Windows XP 以降」。
- 非 NULL の経路（`SetInputScope` を引く）に進んだ場合に、ゲストが SetInputScope を実際に呼ぶかは、f_5e0f30 が呼ばれたときだけ。今回の停止点までの実行で呼ばれたかは、**実測していない**（次の停止点の呼び出しスタック（§6）に f_5e0f30 は無いが、スタックは停止時の生きている呼び出しだけで、過去に呼ばれて戻った可能性は排除できない。入口ログは先頭 256 件だけ。自作の長いログ `out-msc/work/full-native.txt`（13,000 件超）では、f_5e0f30 の入口を調べていない）。

## 2. 一次資料（Microsoft Learn。2026-10-21 取得）

| ページ | 確認した内容 |
|---|---|
| [LoadLibraryW](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryw)（`updated_at` 2025-07-01） | 成功は「a handle to the module」。失敗は NULL で、拡張エラーは GetLastError。**ページは失敗時のコードを書かない**。拡張子の省略は ".DLL" を補う。「If the function cannot find the module, the function fails」。Remarks: 「To enable or disable error messages displayed by the loader during DLL loads, use the SetErrorMode function」。Security Remarks: 「Do not make assumptions about the operating system version based on a LoadLibrary call that searches for a DLL」。最小の対応: Windows XP。 |
| [System Error Codes (0-499)](https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499-)（2025-07-16） | `ERROR_MOD_NOT_FOUND` = 126 (0x7E)「The specified module could not be found.」`ERROR_PROC_NOT_FOUND` = 127。**LoadLibraryW のページが 126 を名指ししない**ので、「存在しない DLL で 126」は一次資料の LoadLibraryW ページからは出ない。126 という値は決定 0015（PM）の選択で、System Error Codes のページが値と意味を与える。 |
| [SetErrorMode](https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-seterrormode)（2024-06-26） | 引数は uMode。値 0（システム既定）、SEM_FAILCRITICALERRORS 0x1、SEM_NOGPFAULTERRORBOX 0x2、SEM_NOALIGNMENTFAULTEXCEPT 0x4、**SEM_NOOPENFILEERRORBOX 0x8000**（「The OpenFile function does not display a message box when it fails to find a file」）。戻り値は「the previous state of the error-mode bit flags」。 |
| [SetInputScope](https://learn.microsoft.com/en-us/windows/win32/api/inputscope/nf-inputscope-setinputscope)（2025-07-01） | `HRESULT SetInputScope(HWND hwnd, InputScope inputscope)`。要件の DLL は **Msctf.dll**、最小の対応 Windows XP。戻り値は S_OK。ウィンドウとスレッドが同じスレッドでないとき E_INVALIDARG。`api_location` に `msctf.dll` と `ext-ms-win-tsf-msctf-l1-1-x.dll`。 |
| [Text Services Framework](https://learn.microsoft.com/en-us/windows/win32/tsf/text-services-framework)（2025-04-15） | 「TSF is a system service available as a redistributable for Windows」。適用は「Windows XP or later versions of the operating system」。 |
| [GetProcAddress](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getprocaddress)（2024-02-05） | 名前の綴りと大文字小文字は EXPORTS と同じ。失敗は NULL。「If the function might not exist in the DLL module ... design your application to handle the case when the function is not available」。 |

確認できなかったこと: (1) 「Windows 10 に msctf.dll が必ずある」ことそのものを書いたページ。SetInputScope と TSF のページが「Windows XP 以降」と書くことと、決定 0014 の OS（Windows 10）から**導いた**。(2) LoadLibraryW が存在しない DLL に対して付ける GetLastError の値（ページは書かない）。(3) msctf.dll の他の export の仕様と、DLL の内部の挙動。

## 3. f_44db40 を通る全呼び出し点と、他の外部 DLL の呼び出し（読み。静的に追える範囲）

`call 0x44db40` は 26 か所（実測: `grep`）。第 1 引数が直前でリテラルとして読まれるのは 11 か所、残り 15 か所は動的（`mov rcx,[rbp+..]` など。名前は追えていない）。すべて `edx = 0x8000`（実測: `sites44.py`）。

「NULL 許容」は、LoadLibraryW の直後 6 命令以内に 0 との比較（`cmp [..],0`、`test rax,rax`）がある場合を「検査あり」とした**ヒューリスティック**（`nullchk.py`。検査があることは NULL の許容の証明ではなく、検査の先で何をするかは読んでいないものがほとんど）。

### 3.1 f_44db40 経由のリテラル（11 か所）

| 呼び出し点 | 名前 | 直後の検査 | 読み |
|---|---|---|---|
| 0x5de9fa | Msctf.dll | なし（保存のみ） | §1。NULL を許容（f_5dea70、f_5dea30、f_5e0f30 が検査） |
| 0x620a47 | imm32.dll | 後続の f_620a80（0x620a9b）が `cmp [0x13e2ab8],0 / je` | NULL を許容（読み）。ただし imm32 は決定 0029 が「ない」と定義した名前 |
| 0x6f68ce | oleaut32.dll | `cmp [0x15f60d0],0 / je` | 検査あり |
| 0xc07a0f | WS2_32.DLL | `cmp [0x14fd348],0 / je` | 検査あり |
| 0xc12afb | MSWSOCK.DLL | `cmp [0x14fd350],0 / jne`（NULL のとき GetLastError を使ってエラー処理へ） | 検査あり |
| 0xc14b47 | Wship6.dll | なし（`[0x14fea48]` に写して `c05820` に渡す） | **NULL 許容かは追えていない**（c05820 → 0x4235f0 に NULL の hModule を渡す形。後で `test rbx,rbx` がある） |
| 0xc14c5e | Fwpuclnt.dll | `cmp [0x14fea50],0 / je` | 検査あり |
| 0xc171fa | IdnDL.dll | `cmp [0x14fedd0],0 / je` | 検査あり |
| 0xc17278 | Normaliz.dll | `cmp [0x14fedd8],0 / je` | 検査あり |
| 0xc192fa | iphlpapi.dll | なし（保存のみ。読み手 0xc18f52 などが `cmp [0x14fee78],0`） | 読み手が検査する形（読み。先は未読） |
| 0xf360bc | ss2wav.dll | `cmp [0x16bac08],0 / jne` | 検査あり（ss2wav はゲスト自身のプラグイン DLL） |

### 3.2 f_44db40 経由で名前が追えない 15 か所

0x653a51、0x84344e、0x95eb37、0x9da84a、0xa5ff09、0xa65b24、0xac50ec、0xb665b7、0xb692c1、0xb6e9ce、0xb7645c、0xb953d6、0xc525c9、0xd8cb65、0xf06c93。第 1 引数は `[rbp+..]` や `lea rcx,[rdx+0x38]` で、直前に名前を作る関数の戻り値。**名前は確定していない**（決定 0029 の「動的 9 か所」と同じ種類の未確定）。直後に検査があるのは 12 か所（`nullchk.tsv`）、ないのは 0x653a51、0x95eb37、0x9da84a の 3 か所。

### 3.3 f_44db40 を通らない LoadLibraryW の直接の呼び出し（外部 DLL の部分。SHFolder 21 か所と kernel32 系は決定 0029 で表にある）

| 名前 | 呼び出し点 | 直後の検査 | 決定 0029 の扱い |
|---|---|---|---|
| dwmapi（拡張子なし） | 0x65ca23 | `cmp [0x15f4158],0 / je` | NULL + 126 |
| SHCore.dll | 0x65cf89 | `cmp [0x15f42b0],0 / je` | NULL + 126 |
| avrt | 0x65ccd1 | `cmp [0x15f4288],0 / je` | 止める |
| PowrProf.dll | 0x65cf38 | `cmp [0x15f42a8],0 / je` | 止める |
| advapi32（拡張子なし） | 0x65cee7 | `cmp [0x15f42a0],0 / je` | 表にある |
| PSAPI.dll | 0x4263e1 | `cmp [0x15ec320],0 / jne` | 止める |
| RICHED20.DLL | 0x6d644d | `cmp [..],0x20 / ja`（値が 0x20 以下なら 0 に直す）。NULL 許容の形ではあるが、失敗値を `<= 0x20` で見る旧い慣習 | 止める |
| URLMON.DLL | 0x707bfa | `cmp [0x13f3e78],0 / je` | 止める |
| WebView2Loader.dll | 0xbe3539 | `cmp [0x16b6ea0],0 / jne` | 止める |
| 動的 | 0x8b419d、0x8b41c1（thunk 0x4239a0）、0x9b594b、0xca1008、0xcad9c4、0x11336a4 | 後ろ 3 つは検査あり。前 3 つは検査なし | 動的。止める（名前が表・9 DLL に無いとき） |

### 3.4 依頼が列挙した DLL の読み

dwmapi、uxtheme、imm32、shcore、propsys、rpcrt4、windowscodecs、wtsapi32、msimg32 は、PE の **delay import**（実測: pefile の `DIRECTORY_ENTRY_DELAY_IMPORT`。dwmapi は `DWMAPI.DLL`、shcore は `Shcore.dll`）。ゲストの LoadLibraryW のリテラルとして現れるのは dwmapi（0x65ca23）、SHCore.dll（0x65cf89）、imm32（f_44db40 経由 0x620a47）の 3 つだけで、**いずれも直後で NULL を検査する**（読み）。uxtheme、propsys、rpcrt4、windowscodecs、wtsapi32、msimg32 は LoadLibraryW のリテラルが見つからない（delay load の補助コードが LoadLibraryA で引く形と**推測**。0x41dbd1 の `LoadLibraryA(動的)` がそれらしいが、確認していない）。

ゲストが NULL を許容するか、この調査では「許容する形」までしか言えない。**そのDLLが実際に無いときのゲストの動作の検証は、実行を伴うので何もしていない**。

## 4. 判断（決定 0055。要点）

- 決定 0029 の「欠けている」9 名の一覧に msctf.dll を足す案は、PM の条件 (c) を満たさない。エミュレート OS は Windows 10 22H2（決定 0014）で、msctf.dll は一次資料のページが「Windows XP 以降」としている。「この環境は msctf.dll を持たない」と定義すると、環境の定義が OS の定義と矛盾する。
- 採用: msctf.dll を**仮想モジュール**にする（`fle_modules.c` の表の末尾、N = 26）。`LoadLibraryW("Msctf.dll")` と `GetModuleHandleW` が同じハンドルを返す（決定 0015 の規則）。ゲストが GetProcAddress で引く名前 `SetInputScope` だけに応答表の行を作り、**呼ばれたら名前付きで停止**する（HRESULT を作らない）。詳細は決定 0055。

## 5. 実装

- `lift/runtime/fle_modules.c`: 表の末尾に `"msctf.dll"` を 1 行（N = 26）。
- `lift/runtime/fle_msctf.h`、`fle_msctf.c`（新規）: `fle_imp_SetInputScope`（`fle_stop` で止まる）。
- `lift/runtime/fle_load.c`: コメント 1 行（表の説明に msctf.dll を足した）。コードは変更なし。
- `lift/crates/fe-lift/src/imports_table.rs`: 末尾に応答表の行 1 つ（コメント付きで 2 行）。
- テスト: `lift/crates/fe-lift/tests/msctf.rs`（新規、11 件）。既存の `modules.rs`（4 行: モジュール数 25→26、ハンドルの範囲の上限 25→26、`handle-past-table` の番号 25→26、`lines[26]`→`lines[27]`）と `load.rs`（1 行: `fle_load_refcount(25)`→`(26)`）を、モジュール数が 25 から 26 になった分だけ更新した（期待値の理由はコメントに書いた）。
- 他の runtime ファイル（`fle_vol.c`、`fle_event.c`、`fle_gdc.c`、`fle_gdi.c` ほか）のハンドル領域は、`0x7FF800000000 + 0x10000 * 26` より上にあることを確認済みのテスト（`vol.rs`、`event.rs`、`glob.rs` は `mods_end = ... * 26` で、もともと 26 まで見込んでいた。変更なし）で通った（§7）。

## 6. harness の再実行（実測）

生成 C は変わった（応答表が 1 行増えた）ので `lift/out-msc/out` に新規生成した（`OUT=lift/out-msc bash harness/wasm/build.sh gen split`、次いで `rt gen-c harness link`）。`functions 59297`、`bad_nodes 0`、`unsupported_total` **547 → 547**（`SetInputScope` は IAT の thunk ではないので、`jmp_imp` の行は増えず、unsupported も減らない。応答表の行が 1 つ増えただけ）。全 17 パーツと runtime 36 ファイルが `errors=0`（`build-wasm.txt`）。native は `build_native.sh`（clang）で同じパーツをビルド。

| | native | wasm（node） |
|---|---|---|
| 停止 | `fle: unsupported instruction at 0x424ee0`（`SIGABRT`、`abort kind: fle_unsupported`） | 同じ |
| 関数実行数（distinct） | **572**（前 535） | 572 |
| 入口数（calls） | **13,227**（前 12,913） | **13,230** |
| 呼び出し深さ | 10（前 8） | 10 |
| unsupported_total | 547 | 547 |

- 0x424ee0 は `user32!GetSystemMetrics` の thunk（実測: IAT）。呼び出し元は `f_620860`（0x620872。呼び出しは `mov ecx,0x13` の `GetSystemMetrics(19)`）。**SM_MOUSEPRESENT = 19 は Learn の GetSystemMetrics で確認**（「Nonzero if a mouse is installed; otherwise, 0. This value is rarely zero」）。結果を `test eax,eax / setne al / mov [rbx+0x10],al` に保存する（1 バイトのブール値）。隣の f_620880（0x620880）は `mov ecx,0x4b; call 0x424ee0`（`GetSystemMetrics(0x4b)`）の結果を `[rbx+0x28]` に保存し、真のとき `call 0x4255e0`（thunk の先の API は**未確認**。`thunks.py` を当てていない）を呼ぶ。0x4b が何の定数かは**未確認**（Learn の表を読んでいない）。呼び出しスタック: `f_424ee0` ← `f_620860` ← `f_6209a0` ← `f_620640` ← `f_620cd0` ← `f_623840` ← `f_4120c0`（0x412129）← … ← エントリ。
- Msctf の `LoadLibraryW` は通った（読み: 停止時のスタックに `f_5de9e0` が無く、`f_5deae0` も無い。`f_5de9e0` は戻った。`[0x13d9ef8]` の値は gdb で読んでいない）。仮想ハンドルは非 NULL なので、`[0x13d9ef8]` は 0x7FF8001A0000（N = 26 の値）のはず（推測）。停止点の先で `f_5dea70` が「1 を返すスタブ」を呼ぶかは確認していない。

### 6.1 native と wasm の入口数が一致しない（今回新しく出た差。原因は読みと一部の実測）

> **解消済み: 決定 0056**（GetCurrentThreadId を固定の仮想値 0x1001 に）。以後、native と wasm の入口数はどちらも 13,230 で一致する（`virtual-thread-id.md` §5 の実測）。この節の本文は書いた時点のまま残している。

- `diff run-native.txt run-node.txt`（`rsp=` の行を除く）の差は、**入口数（13,227 と 13,230）と、その数字が出る 2 行だけ**。関数実行数（distinct）、停止、深さ、呼び出しスタックは同じ。基準の記録（GDI 時点: 12,913 で一致）より前の停止点ではこの差は出ていなかった。
- 入口の列を取り直して比べた（`LOG_MAX` を 20,000 に上げた自作の trace で native と wasm を実行。`out-msc/work/full-*.txt`）。**13,159 番目だけが違う**。native は `f_414ad0`（0x43a8b6 から呼ばれる）の後に `f_414850` へ進み、wasm は `f_414ad0` の後に **`f_409470`、`f_408040`、`f_414850`** の 3 入口が追加される（`f_414ad0` が文字列の長さを変えるときの再確保の経路。0x414b32 の `call 0x409470`。読み）。
- 原因: `f_43aa10`（16 進数に変換する関数）の引数 `edx` に、native では **`GetCurrentThreadId` の戻り値（Linux の tid）**が入る（実測: gdb で停止し `R[2] == syscall(SYS_gettid)`、`0x2a09a8` と `0x2a1f9f`。実行ごとに変わるが、6 桁の 16 進数）。wasm では `fle_host_tid()` が定数 **1**（`runtime/fle_host.h` の `FLE_HOST_WASM_TID`。決定 0045）で、16 進の桁数が 1 桁になる。桁数が違うので、`f_43a890` → `f_414ad0` に渡す長さが違い、`f_414ad0` が「長さが同じなら何もしない」（0x414b04 の `je 0x414b9f`）経路に入るかが変わる（native の 1 回目の `edx = 0x1000`、2 回目が tid）。**wasm 側を gdb で追ってはいない**（wasm のデバッガを使っていない）ので、wasm の分岐を実測で確かめたわけではない。native の tid が 6 桁の間（`0x100000` 以上）は同じ差になると推測するが、tid が 1 桁になる環境での native 実行はしていない。
- 実行ごとの再現性: native を 2 回、wasm を 2 回実行して、それぞれ 13,227、13,230 で、同じだった（実測）。ただし native の値は tid の桁数に依存する。
- これは `GetCurrentThreadId` の決定 0017/0045 の設計の帰結で、今回の変更によるものではない（今回の変更は msctf だけ）。次の依頼者の方針に従うべき事項: native と wasm の入口数は、tid を使う経路が増えるたびにずれる可能性がある。ずれを防ぐなら、native 側でも GetCurrentThreadId を定数にする、harness の比較で入口数を除く、のどちらかが要る（この作業では変更していない）。

## 7. テスト（実測）

- `cargo test -p fe-lift --test msctf`: **11 件すべて ok**（`out-msc/test-msctf.txt`）。内訳: 生成器 1（応答表の行が msctf.dll にだけある）、runtime 9（26 番目のモジュール、LoadLibraryW のハンドルと last error、他のハンドルが変わらないこと、GetProcAddress の見つかる・見つからない、`fle_dispatch` を通した SetInputScope の停止、FreeLibrary の参照カウント、「ない」9 名が 126 のままであること、近い名前 6 通りの停止、範囲外のハンドル）、実 DLL のバイト列 1（§1 の読みの根拠となる命令列と文字列。DLL が無いときは skip）。
- 変異テスト（`out-msc/work/mkmut.py` が `out-msc/mut/m1`〜`m7` に runtime の複製を作り、`FLE_MSCTF_RT_DIR` で指す）: **7 変異すべてが失敗で検出された（生き残り 0）**（`out-msc/mut/summary.txt`）。m1 表に msctf.dll を入れない（6 件失敗）、m2 msctf.dll を「ない」名前の一覧に入れる（6）、m3 SetInputScope が S_OK で返る（1）、m4 ntdll と msctf の並びを入れ替える（6）、m5 「ない」9 名のうち 1 名を外す（1）、m6 前方一致で msctf のハンドルを返す（2）、m7 LoadLibraryW が msctf のとき成功でも last error を 126 にする（3）。
- `cargo test --workspace --no-fail-fast`（`out-msc/test-ws.txt`、shell のタイムアウト 3,600,000 ms、フォアグラウンド）: awk の集計は `test result` 行 **73**、**707 passed、0 failed、0 ignored**（cargo の終了コード 0）。直前の基準は 72 行、696 件。差は +1 行、+11 件（新規 `msctf.rs` の 11 件）。

## 8. その先の見通し（読みと推測。実装していない）

1. `user32!GetSystemMetrics`（今回の停止点）。f_620860 が 19（SM_MOUSEPRESENT）、f_620880 が 0x4b、その先が別の値。値の意味は Learn の表で確認できる（19 は確認済み）が、仮想環境の値（マウスの有無）の決定は PM（ホストを見せない方針の対象）。
2. f_5e0f30 が呼ばれたら `SetInputScope` で名前付きに停止する（窓の表が要る）。ここに届くのは f_620a80 経由で、停止点の先。
3. f_44db40 を通る未解決の 15 か所（§3.2）と、f_5dea70 の「1 を返す」経路で `[0x13d9f00]` が何を呼ぶか。

## 9. 触ったファイルと制約

- 変更: `lift/runtime/fle_modules.c`（1 行）、`lift/runtime/fle_load.c`（コメント 1 行）、`lift/crates/fe-lift/src/imports_table.rs`（行 1）、`lift/crates/fe-lift/tests/modules.rs`、`lift/crates/fe-lift/tests/load.rs`。
- 新規: `lift/runtime/fle_msctf.h`、`fle_msctf.c`、`lift/crates/fe-lift/tests/msctf.rs`、`docs/decisions/0055-msctf-loadlibrary.md`、本書、`lift/out-msc/`（gitignore 済み）。
- `rm`、既存ファイルの `>` 上書き、git の変更操作、`/tmp`、コマンド文字列の `sleep`/`timeout`/`nohup` は使っていない。権限拒否はなし。`lift/iat-unsup-closure.txt` には触っていない。emcc は `JOBS=2` 以下。
