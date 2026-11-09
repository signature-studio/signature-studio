# import の応答: 仕様の出典と決定の一覧

## API ごとの一覧

| VA | 名前 | 仕様の出典 | 応答の決定 | 状態 |
|---|---|---|---|---|
| 0x19714f0 | kernel32!GetCurrentThreadId | https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getcurrentthreadid | 呼び出しスレッドの tid（`gettid()` の 32 ビット切り詰め）を R[0] に返す。詳細は `docs/decisions/0001-getcurrentthreadid.md` | 実装済み、thunk 0x406f90 は実行で通過を確認 |

## 出典の読み方

- 上の出典は 2026-10-20 に取得し、Syntax、Return value、Remarks を読んだ。
- 仕様を確認できなかった import には、応答を作っていない。

## 停止のまま残したもの

- 件数: 実測で数えていない。生成 C の `fle_unsupported` 行は、元の `lift/out-string/after.c` で 1,576 件、`lift/out-imp/after.c` で 1,575 件（差は 0x406f90 の 1 件）。このうち import thunk の件数は未分類。
- 理由: 次の停止点（0x41d050）は import ではなく、`f_412160` の間接 call で計算された値。import の応答追加の担当範囲外のため、ここで止めた。
