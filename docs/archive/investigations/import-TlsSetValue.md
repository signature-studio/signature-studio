# import-TlsSetValue: 仕様の出典、読んだ箇所、確認できなかった点

作業者（PM）の調査。

## 対象

- IAT VA 0x19717a8、kernel32.dll!TlsSetValue（`lift/out-imports/imports.txt` 442 行）
- 呼び出し元: 0x41d041〜0x41d044（0x41d000 の中。`call 0x41cf70`）
- 生成 C の行: `lift/out-tls2/out/plus41d-tls2.c` の `L_41cf70`

## 仕様の出典

- URL: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-tlssetvalue （2026-10-20 取得。ms.date 2024-02-02、updated_at 2025-07-01）

読んだ箇所と要約:

- Syntax: `BOOL TlsSetValue([in] DWORD dwTlsIndex, [in, optional] LPVOID lpTlsValue);`
  - dwTlsIndex は TlsAlloc が返した索引。lpTlsValue は呼び出しスレッドのスロットに格納する値。
- Return value: 成功時は nonzero。失敗時は 0。拡張情報は GetLastError で得る。
- Remarks:
  - 「Each thread of a process has its own slot for each TLS index.」（description）
  - 「it succeeds if dwTlsIndex is in the range 0 through (TLS_MINIMUM_AVAILABLE – 1). It is up to the programmer to ensure that the index is valid」。つまり、この範囲では検証なしで成功する。範囲外の失敗条件は書かれていない。
  - Windows Store app の FlsSetValue への置換（Windows Phone 8.1、Windows 8.1 など）の記述あり。対象は Store app。

## 仕様から言えること

- 引数は 2 つ。dwTlsIndex（DWORD）、lpTlsValue（LPVOID）。
- 戻り値は BOOL。成功 nonzero、失敗 0。
- 失敗時は GetLastError で原因を得る、と書かれている。
- 成功時に SetLastError を呼ぶとは書かれていない（一般則として Return value 周辺の GetLastError の記述と整合する。ただし TlsSetValue 固有の記述は読んだ範囲にない）。

## 仕様から言えないこと（確認できなかった点）

- 失敗時にどの値を GetLastError が返すか。TlsSetValue のページには書かれていない。
- 「索引が無効な場合」の具体的な失敗条件。Remarks は 0〜(TLS_MINIMUM_AVAILABLE−1) の範囲で検証なしに成功すると書くのみ。
- 範囲外の索引を渡したときの挙動（失敗するのか、未定義なのか）。ページには書かれていない。
- TLS_MINIMUM_AVAILABLE の値（ページ TLS の記述には 64 以上とあるが、この調査では数値の詳細を読んでいない）。

## 呼び出し箇所（objdump、0x41d000 の内部）

```
41d03b: mov    0xf8c347(%rip),%ecx   ; ecx = [0x13a9388]（索引。TlsAlloc の戻り値）
41d041: mov    %rax,%rdx             ; rdx = LocalAlloc の戻り値（確保したブロック）
41d044: call   0x41cf70              ; TlsSetValue(索引, ブロック)
```

- 第 1 引数（dwTlsIndex）は ecx（R[1]）。第 2 引数（lpTlsValue）は rdx（R[2]）。
- 0x41d03b に入るのは、0x41d02a の `test %rax,%rax; jne` で LocalAlloc の戻り値が非ゼロのとき。
- 戻り値 eax（BOOL）は 0x41d044 の後で使われない。0x41d049 で `add`、`pop`、`ret` に進む。
- 戻り値の使われ方: 読んだ範囲では、戻り値を検査していない。失敗時の処理もない。

## 実装（lift/runtime/fle_imports.c）

- 索引が TlsAlloc で払い出された範囲か（fle_tls_next_index 未満か）を確認する。範囲外は FALSE（R[0] = 0）、GetLastError の値を 87（ERROR_INVALID_PARAMETER）に設定する。
- 範囲内なら値を保存し、R[0] = 1。
- 87 の値は Microsoft の System Error Codes (0-499) ページで確認した（`ERROR_INVALID_PARAMETER 87 (0x57) The parameter is incorrect.`、https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499- ）。ただし「この API がこの値を返す」とは仕様に書かれていない。作業者の選択である。

## 実行での確認（lift/out-tls2/run2.txt）

- entry 13（f_41cf70）が呼ばれたことを確認。以後、0x41d000 から戻った後に TlsGetValue が 0x41d07c への戻り番地で呼ばれている（entry 14）。よって TlsSetValue は戻っている（推論。戻りを直接記録してはいない）。

## 残り

- GetLastError の値の確認（0x41cf80）。この実行では到達していない（別調査）。
