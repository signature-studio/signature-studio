# import-GetProcAddress: 仕様の出典、読んだ箇所、確認できなかった点

作業者（M1）の調査。取得日 2026-10-20。判断は `docs/decisions/0015-virtual-modules.md`。

## 対象

- IAT VA 0x1971600、kernel32.dll!GetProcAddress（`lift/out-imports/imports.txt` 389 行）。thunk は 0x407070（`jmp *0x1971600`、objdump の読み）。
- 呼び出し箇所（objdump）: f_4099c0。

```
4099dd: lea 0x104(%rip),%rcx   # 0x409ae8   ("kernel32.dll", UTF-16LE)
4099e4: call 0x407060          # GetModuleHandleW
4099e9: mov %rax,%rcx          # 第 1 引数 hModule = 戻り値
4099ec: lea 0x10f(%rip),%rdx   # 0x409b02   第 2 引数 lpProcName
4099f3: call 0x407070          # GetProcAddress
4099f8: test %rax,%rax
4099fb: je 0x409aae            # NULL なら 0x409aae へ（eax = 0x44(%rbp) = 0 を返す）
```

- 0x409b02 のバイト列（`objdump -s` の読み）: `47 65 74 4c 6f 67 69 63 61 6c 50 72 6f 63 65 73 73 6f 72 49 6e 66 6f 72 6d 61 74 69 6f 6e 00` = ASCII `GetLogicalProcessorInformation`、NUL 終端。**第 2 引数は ANSI（LPCSTR）の名前**。序数ではない（上位ビットが 0 でない、ポインタ値 0x409b02）。
- NULL のとき: 0x409aae は `mov 0x44(%rbp),%eax`。0x44(%rbp) は 0x4099cf で `movl $0,0x44(%rbp)` と 0 に初期化されている（objdump の読み）。つまり f_4099c0 は 0 を返す。
- 非 NULL のとき（今回は通らない）: 0x409a01 以降で `call 0x407320`（`jmp *0x19762d0`。imports.txt では遅延 import の kernel32.dll!GetLogicalProcessorInformation）を呼ぶ。rax（GetProcAddress の戻り値）は 0x4099f8 の `test %rax,%rax` にだけ使われ、`call %rax` のような形で呼ばれてはいない（0x4099c0〜0x409ab6 の objdump を読んだ範囲）。つまりこの呼び出し元では、GetProcAddress は「その API があるか」の判定にだけ使われる。非 NULL を返すと、遅延 import の thunk 0x407320（対応表にない import）に進む。

## 仕様の出典

- URL: https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getprocaddress （ms.date 2024-02-05、updated_at 2025-07-01）

読んだ箇所と要約:

- Syntax: `FARPROC GetProcAddress([in] HMODULE hModule, [in] LPCSTR lpProcName);`
- hModule: 関数または変数を持つ DLL のハンドル。LoadLibrary、LoadLibraryEx、LoadPackagedLibrary、GetModuleHandle が返す。LOAD_LIBRARY_AS_DATAFILE のモジュールからはアドレスを取れない。
- lpProcName: 関数名または変数名、または関数の序数。序数のときは下位ワードに入れ、上位ワードは 0 でなければならない。
- Return value: 成功でエクスポートされた関数または変数のアドレス。失敗で NULL。拡張エラーは GetLastError。
- Remarks:
  - lpProcName の綴りと大文字小文字は、DLL の .def の EXPORTS と同一でなければならない。
  - 序数で指定すると、GetProcAddress は序数が 1 から最大序数の範囲にあることを確かめ、関数テーブルを引く。.def が 1 から N まで連番でないと、実在しない序数に対して無効な非 NULL を返すことがある。
  - 関数がその DLL に無いかもしれない場合（例: Vista 以降にしかない関数）は、序数ではなく名前で指定し、関数が無い場合に対応する。コード例は `GetProcAddress(GetModuleHandle(TEXT("kernel32.dll")), "GetNativeSystemInfo")` が NULL なら代替を使う形。

## 確認できなかった点（ページに書かれていない）

- 失敗時の具体的なエラーコード。ERROR_PROC_NOT_FOUND（127）は System Error Codes（0-499）ページ（https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499- ）に「The specified procedure could not be found.」とあるが、GetProcAddress がそれを返すとは、この GetProcAddress のページに書かれていない。127 は PM の決定（0015）。
- 序数と名前の区別の 64 ビットでの正確な規則。ページは「上位ワードが 0」と書く。ポインタ全体の上位 48 ビットが 0 なら序数、という IS_INTRESOURCE 流の読みは推論。
- hModule が無効なとき（NULL、解放済み、ハンドルでない値）の戻りとエラー。
- 名前の長さの上限。
- 成功時に last error を触るか。
- 変数（関数でないエクスポート）の扱い。

## 実装での扱い（`lift/runtime/fle_modules.c` の `fle_imp_GetProcAddress`）

- 名前は ANSI として読み、大文字小文字を区別して比較する。
- `lpProcName` の bit 16 以上が 0 のとき（序数。NULL も含む）は abort。仮想モジュールに序数のエクスポートがない。
- hModule が仮想モジュールのハンドルでない値（NULL、整列しない値、表の範囲外）と、リフトした DLL 自身のハンドル（0x400000）は abort。ページが決めていない。
- 応答表（`fle_resp_table`）に（モジュール名、API 名）がある: その擬似 VA を返す。ない: NULL と last error 127。モジュールが違えば別の行（user32 の GetVersion は kernel32 の GetVersion とは別）。
- 擬似 VA は `fle_dispatch_table` にも入っており、`fle_dispatch(va)` で応答の関数が呼ばれる（`tests/modules.rs` の `get_proc_address_result_is_called_through_dispatch`）。

## 実行での確認

- `lift/out-modules/run-modules-896.txt`: entry 47（f_407070、guest-return 0x4099f8）は abort せず返った。`GetLogicalProcessorInformation` は応答表にないので NULL（と 127）が返り、0x4099fb の `je` が 0x409aae に分岐した。その後 0x407320 は呼ばれず、f_4099c0 は 0 を返して entry 48（f_4090e0）に進んだ（entry の並びからの読み。分岐そのものは未トレース）。
- Windows 本体との照合はしていない。実在する Windows の kernel32 は GetLogicalProcessorInformation をエクスポートする（推測。この調査では確認していない）ので、この環境の「無い」は実機と違う。0015 の決定による。
