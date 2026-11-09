# 0003: LocalAlloc の応答

状態: 作業者（PM）が決定。依頼者の判断は含まない。

## 文脈

停止点は `0x41cf20` の `jmp [IAT 0x19717c8]`（kernel32!LocalAlloc）。呼び出し元は `0x41cfd0`（`mov $0x40,%eax; mov %ecx,%edx; mov %eax,%ecx; call 0x41cf20`）。引数は uFlags = 0x40、uBytes = 0x32a8。詳細は `docs/investigations/import-LocalAlloc.md`。

## 仕様の出典

- 出典: https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-localalloc （2026-10-20 取得）
- uFlags の既定は LMEM_FIXED。LMEM_FIXED の戻り値は「メモリオブジェクトへのポインタ」。LMEM_MOVEABLE はハンドル（LocalLock で変換）。
- LMEM_ZEROINIT（0x0040）は内容を 0 に初期化する。
- 失敗時は NULL。詳細は GetLastError。

## 決定

**単調増加のハンドル値は返さない。uFlags に LMEM_MOVEABLE が無いので、実体のあるポインタを返す。LMEM_ZEROINIT（0x40）なら 0 で初期化し、無ければ初期化しない（malloc）。失敗時は NULL。**

### 理由（仕様の出典と、逆アセンブルに基づく）

- 0x41cfd0 の uFlags は 0x40 で、LMEM_MOVEABLE（0x0002）を含まない。したがって仕様の既定（LMEM_FIXED）が適用され、戻り値はポインタ。
- 依頼文の「ハンドルを返す案」は、LMEM_MOVEABLE 時の戻り値（ハンドル）を前提にしていた。この前提は本件の uFlags と合わない。ハンドルの番号を返すと、guest はそれをポインタとして使う（0x41d02a の test と 0x41d041 の TlsSetValue の値になる）。
- harness のメモリは identity map（`fle_ld` / `fle_st` が host ポインタを直接読み書きする）。したがって host の `malloc` / `calloc` が返すポインタは、guest のアドレスとして有効。
- 既存の heap は harness に無い（`lift/harness/` を grep。heap / malloc の実装なし）。既存に合わせる対象が無いので、最小限の `malloc` / `calloc` を使う。

### 代替案との比較

- 案 A（採用）: `calloc`（LMEM_ZEROINIT あり）/ `malloc`（なし）で実体を確保し、ポインタを返す。仕様の既定（LMEM_FIXED）と一致。
- 案 B: 単調増加の番号を返す（依頼文の案）。仕様の LMEM_FIXED（ポインタ）と合わない。guest がその値をポインタとして使うと壊れる。却下。
- 案 C: 固定の非 NULL 値を返す。実体が無く、guest の読み書きで壊れる。却下。

## 限界（仕様から言えないこと）

- uBytes = 0 の LMEM_FIXED の戻り値は仕様の抜粋に明記がない。実装は 1 バイトとして確保する（作業者の仮定）。本件の uBytes は 0x32a8 で、この仮定は実行に影響しない。
- GetLastError は未実装（失敗時に設定しない）。失敗の詳細は読んでいない。本件の実行では失敗は起きていない（後述）。
- LMEM_MOVEABLE（ハンドル）は未実装。指定されたら `fle_imp_LocalAlloc` が abort する。
- LocalFree は未実装（本件の経路では呼ばれていない）。

## 実装

`lift/runtime/fle_imports.c` に `fle_imp_LocalAlloc` を追記。`lift/runtime/fle_imports.h` に宣言を追記。既存の関数は変更していない。

## 次の作業

- 0x41cf70 の TlsSetValue（IAT 0x19717a8）。未対応。
- 0x41cf60 の TlsGetValue（IAT 0x1971778）、0x41cf80 の GetLastError（IAT 0x19713c0）。未対応。
