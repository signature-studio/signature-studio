# import-GetLastError: 仕様の出典、読んだ箇所、確認できなかった点

作業者（PM）の調査。

## 対象

- IAT VA 0x19713c0、kernel32.dll!GetLastError（`lift/out-imports/imports.txt` 317 行）
- 生成 C の行: `lift/out-tls2/out/plus41d-tls2.c` の `L_41cf80`
- 呼び出し箇所（objdump の grep）: 0x41db26（`call 0x41cf80`、0x41db26〜0x41db2b で eax を -0x10(%rbp) に保存）、0x41dc2b（同様）。0x41d000〜0x41d087 の経路には呼び出しがない。

## 仕様の出典

- URL: https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror （2026-10-20 取得。ms.date 2024-02-02、updated_at 2024-02-22）

読んだ箇所と要約:

- Syntax: `_Post_equals_last_error_ DWORD GetLastError();`
- Return value: 呼び出しスレッドの last-error code。
  - 「If the function is not documented to set the last-error code, the value returned by this function is simply the most recent last-error code to have been set; some functions set the last-error code to 0 on success and others do not.」
- Remarks:
  - 「Functions executed by the calling thread set this value by calling the SetLastError function.」
  - 「You should call the GetLastError function immediately when a function's return value indicates that such a call will return useful data.」
  - 「The last-error code is maintained on a per-thread basis.」（Retrieves の説明）
  - 「Error codes are 32-bit values」。ビット 29 はアプリケーション定義のエラーに予約。

## 仕様から言えること

- 引数なし、戻り値 DWORD。
- 値は呼び出しスレッドの last-error code。
- 値は直前に設定された last-error code（SetLastError、または他の関数が設定したもの）。
- 失敗を示す関数の直後に呼ぶべき。

## 仕様から言えないこと（確認できなかった点）

- 初期値。このページは初期値を書いていない（スレッド開始時の値は未確認）。
- どの関数が last error を設定するかの一覧。ページは「Return Value section of each function」に委ねている。
- 数値の一覧は、System Error Codes のページ（別ページ）にある。この調査では 0〜499 のページのみ読んだ（ERROR_INVALID_PARAMETER 87 の確認に使用）。

## 呼び出し箇所（objdump、この実行の経路外）

- 0x41db26: `call 0x41cf80` の直後、`mov %eax,-0x10(%rbp)`。戻り値（DWORD）を保存している。
- 0x41dc2b: 同様。
- この 2 箇所の周辺の分岐（どの関数の中か、何のための取得か）は未読。

## 実装（lift/runtime/fle_imports.c）

- `fle_last_error` を返す（R[0] = fle_last_error）。
- 初期値 0 は作業者の決定（仕様は書いていない）。
- TlsGetValue が成功時に 0 を設定し、TlsSetValue と TlsGetValue の失敗時に 87 を設定する。
- TlsAlloc と LocalAlloc は last error を設定しない（既存の応答。この作業では変更していない）。

## 実行での確認

- lift/out-tls2/run2.txt では 0x41cf80 に到達していない（0x41d000〜0x41d087 の経路外）。
- よって GetLastError の応答は、この実行では未検証。構文と生成 C の置換（`L_41cf80`）のみ確認した。
