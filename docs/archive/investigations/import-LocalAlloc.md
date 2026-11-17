# import-LocalAlloc: 仕様の出典、読んだ箇所、確認できなかった点

作業者（PM）の調査。

## 対象

- VA 0x19717c8（IAT）、kernel32.dll!LocalAlloc（`lift/out-imports/imports.txt` の 446 行）
- 停止点: 0x41cf20 の `jmp [0x19717c8]`（`lift/out-tlsalloc/run1.txt`）
- 呼び出し元: 0x41cfd0（0x41cfdd の `call 0x41cf20`。戻り番地 0x41cfe2）。0x41cfd0 の呼び出し元は 0x41d025（戻り番地 0x41d02a）。

## 仕様の出典

- URL: https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-localalloc （2026-10-20 取得。ms.date 2018-12-05、updated_at 2024-02-22）

読んだ箇所と要約:

- Syntax: `DECLSPEC_ALLOCATOR HLOCAL LocalAlloc([in] UINT uFlags, [in] SIZE_T uBytes);`
- uFlags: 既定値は LMEM_FIXED。LMEM_FIXED（0x0000）は「メモリオブジェクトへのポインタ」を返す。LMEM_MOVEABLE（0x0002）はハンドルを返し、LocalLock でポインタに変換する。LMEM_ZEROINIT（0x0040）は「内容を 0 に初期化する」。LPTR（0x0040）は LMEM_FIXED と LMEM_ZEROINIT の組み合わせ。LHND（0x0042）は LMEM_MOVEABLE と LMEM_ZEROINIT。LMEM_DISCARDABLE / LMEM_NOCOMPACT / LMEM_NODISCARD は無視される。
- uBytes: 確保するバイト数。0 のとき、LMEM_MOVEABLE なら「破棄済みとマークされたメモリオブジェクトへのハンドル」を返す。LMEM_FIXED の場合の 0 バイトの扱いは、読んだ箇所に書かれていない。
- Return value: 成功時は新しく確保したメモリオブジェクトへのハンドル。失敗時は NULL。詳細は GetLastError で得る。
- Remarks: 確保が失敗すると NULL。「virtual address zero is never allocated」（仮想アドレス 0 は割り当てられない。NULL はエラーの印）。成功時は要求以上を確保することがある（LocalSize で実際の量を得る）。解放は LocalFree。
- Remarks: 新しいアプリは heap 関数を使うのが推奨（LocalAlloc は「ドキュメントが指定する場合を除く」）。

## 仕様から言えること

- LMEM_FIXED（既定）の戻り値はポインタ。LMEM_MOVEABLE を指定しない限り、戻り値は後続の処理でそのままポインタとして使われる。
- LMEM_ZEROINIT（0x40）は確保した領域を 0 にする。
- 失敗時は NULL（0）。
- 1 バイト以上の確保は要求以上の量を確保してよい（"at least the amount requested"）。

## 仕様から言えないこと（確認できなかった点）

- uBytes = 0 で LMEM_FIXED を指定したときの戻り値（読んだ箇所に明記なし。実装は 1 バイトの確保として扱う。これは作業者の仮定）。
- GetLastError で得られる失敗コードの値（未読）。
- LMEM_ZEROINIT が 0x40 であること（表で確認した。LPTR と同じ値）。

## 0x41cfd0 の引数（objdump で確認）

```
41cfd0: sub    $0x28,%rsp
41cfd4: mov    $0x40,%eax        ; eax = 0x40
41cfd9: mov    %ecx,%edx         ; edx = ecx（0x41d023 で ecx = ebx = 0x32a8）
41cfdb: mov    %eax,%ecx         ; ecx = 0x40
41cfdd: call   0x41cf20          ; LocalAlloc(0x40, 0x32a8)
41cfe2: add    $0x28,%rsp
41cfe6: ret
```

- uFlags = 0x40（LMEM_ZEROINIT。LPTR と同じ値。LMEM_FIXED との組み合わせ）。Windows x64 の呼び出し規約で rcx = uFlags、rdx = uBytes。
- uBytes = 0x32a8（12968 バイト）。0x41d023 の `mov %ebx,%ecx` と、0x41cff0 の `lea 0x32a8,%rax` から来る。
- 生成 C 側の確認: `R[0] = (uint32_t)(0x40)`、`R[2] = (uint32_t)R[1]`、`R[1] = (uint32_t)R[0]`。引数は rcx（R[1]）と rdx（R[2]）に入る。

## 0x41d000 の戻り値の使われ方（0x41d02a 以降、objdump で確認）

- 0x41d02a `test %rax,%rax`。rax == 0 なら 0x41d034 で `0x412a60`（引数 0xe2）を呼んで 0x41d049 で終了。
- rax != 0 なら 0x41d03b〜0x41d044 で `TlsSetValue([0x13a9388], rax)`（0x41cf70 → IAT 0x19717a8）。戻り値のポインタは TlsSetValue の引数 rdx に入る。
- 結論: LocalAlloc の戻り値（ポインタ）は、そのまま TlsSetValue の値として保存される。したがって、単調増加の番号を返すと、TlsSetValue の後で guest がその値をポインタとして使い、壊れる。

## 戻り値の型（ハンドルかポインタか）

- 依頼文は「戻り値（ハンドル）を一意に返せるなら、単調増加のハンドル値を返す案」としていた。
- しかし uFlags に LMEM_MOVEABLE がないため、既定の LMEM_FIXED が適用される。仕様によれば戻り値はポインタ（"The return value is a pointer to the memory object"）。
- したがって、単調増加の番号は仕様に合わない。採った案（実体のあるポインタを返す）は `docs/decisions/0003-localalloc.md` に記録。

## 確認した事実

- 0x41cf20 の jmp と、0x41cfd0 の `call 0x41cf20`（0x41cfdd）は objdump の該当範囲で確認。
- 0x41cf20 の呼び出し箇所が 0x41cfdd の 1 件だけかは未検索（バイナリ全体の検索はしていない）。

## 推測（実行で未確認の部分）

- 0x41d000 の -1 比較（0x41d010）は、TlsAlloc の失敗値（TLS_OUT_OF_INDEXES）と関係すると思われる。ただし本件の実行では未到達。
- 0x412a60 は失敗時の記録関数と思われる。中身は未読。
