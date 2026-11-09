# import-TlsGetValue: 仕様の出典、読んだ箇所、確認できなかった点

作業者（PM）の調査。

## 対象

- IAT VA 0x1971778、kernel32.dll!TlsGetValue（`lift/out-imports/imports.txt` 436 行）
- 停止点（前回）: 0x41cf60 の `jmp [0x1971778]`
- 呼び出し元: 0x41d077（0x41d050 の中）、ほか 0x41d0b0、0x41d123、0x41d138 など（objdump の grep で確認）

## 仕様の出典

- URL: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-tlsgetvalue （2026-10-20 取得。ms.date 2024-08-07、updated_at 2025-07-01）

読んだ箇所と要約:

- Syntax: `LPVOID TlsGetValue([in] DWORD dwTlsIndex);`
- Return value:
  - 成功時は、呼び出しスレッドのスロットに格納された値。
  - 「If dwTlsIndex is a valid index allocated by a successful call to TlsAlloc, this function always succeeds.」
  - 失敗時は 0。拡張情報は GetLastError で得る。
  - 格納値が 0 の場合は、GetLastError が ERROR_SUCCESS かを確認して判断する、と書かれている。
- Remarks:
  - 「TlsGetValue always sets a thread's last error.」
  - 「The TlsGetValue function calls SetLastError to clear a thread's last error when it succeeds.」
  - 「it succeeds if dwTlsIndex is in the range 0 through (TLS_MINIMUM_AVAILABLE– 1). It is up to the programmer to ensure that the index is valid and that the thread calls TlsSetValue before calling TlsGetValue.」
  - Windows 11 24H2 以降は TlsGetValue2 を使うこと。TlsGetValue2 は last error を設定しない（この記述は読んだが、本件では使わない）。

## 仕様から言えること

- 引数は 1 つ（DWORD dwTlsIndex）。戻り値は LPVOID。
- 有効な索引（TlsAlloc 成功で得たもの）なら常に成功する。
- 成功時は last error を 0 に設定する（「clears」の記述）。
- 失敗（無効な索引）時は 0 を返し、GetLastError で原因を得る。
- 格納値が 0 の場合は、last error を見て成否を判断する。

## 仕様から言えないこと（確認できなかった点）

- 無効な索引のときに GetLastError が返す具体的な値。ページには書かれていない。
- 「無効な索引」の判定基準（TlsAlloc で確保されていない索引を渡したときの挙動）。ページは「the index is valid」の責任を呼び出し側に置く。
- 呼び出しスレッドが TlsSetValue を呼ぶ前の値（初期値）は、TlsAlloc の Return value（スロットは 0 に初期化）から 0 と読める。ただし TlsGetValue のページ自体には「初期値 0」の一文はない。

## 呼び出し箇所（objdump）

```
41d071: mov    0xf8c311(%rip),%ecx    ; ecx = [0x13a9388]（索引）
41d077: call   0x41cf60               ; TlsGetValue(索引)
41d07c: mov    %rax,0x11cf24d(%rip)   ; [0x15ec2d0] = 戻り値（ポインタ値）
```

- 引数は ecx（R[1]）。戻り値は rax（R[0]）で、64 ビットのポインタとして 0x15ec2d0 に保存される。
- 0x41d000 が先に TlsSetValue を呼ぶため、この時点で索引のスロットには LocalAlloc のブロックが入っている（rax != 0 の経路の場合）。
- 0x41d0b0、0x41d123、0x41d138 の箇所でも同様に索引 [0x13a9388] を ecx に入れて呼ぶ（読んだのは呼び出し前後の数命令）。
- 0x41d110 の関数（0x41d110〜0x41d15e）は、TlsGetValue の戻り値が NULL のとき 0x41d000 を呼ぶ分岐を持つ。この関数の役割（呼ぶ側）は未読。

## 実装（lift/runtime/fle_imports.c）

- 索引が TlsAlloc で払い出された範囲なら、そのスロットの値を R[0] に返し、last error を 0 にする。
- 範囲外なら R[0] = 0、last error = 87（ERROR_INVALID_PARAMETER）。
- 87 は作業者の選択（TlsSetValue のメモと同じ）。仕様は値を定めない。

## 実行での確認（lift/out-tls2/run2.txt）

- entry 14（f_41cf60）が 0x41d07c への戻り番地で呼ばれた。0x41d000 の戻り後に呼ばれている。よって TlsGetValue の呼び出しに到達し、戻りは推論（直接記録ではない）。
- 後続の停止点（0x40a3b2）は TlsGetValue の後の別関数（f_40a3b0）。TlsGetValue 自体の停止ではない。

## 残り

- 0x41d110 系の呼び出し元（未読）。
- 0x41d0a0 系（0x41d0b0 の TlsGetValue、0x41d0ca の TlsSetValue。0x41d0bd の call 0x41cf30 は IAT 0x1971298 = LocalFree）は、この実行では未到達。
