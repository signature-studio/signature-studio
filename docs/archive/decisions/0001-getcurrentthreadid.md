# 0001: GetCurrentThreadId の応答

状態: 作業者（PM）が決定。依頼者の決定は含まない。

## 文脈

停止点は `0x406f90` の `jmp [IAT 0x19714f0]`（kernel32!GetCurrentThreadId）。生成 C では fe-lift が `fle_unsupported` を出していた。生成 C には import を接続する仕組み（`imp_name_table`、`imp_tab`）が存在しなかった。

## 決定

候補 A を採る。呼び出しスレッドの実際の ID を `R[0]` に返す。

- 値: Linux の `gettid()` を 32 ビットに切り詰めた値（`(uint32_t)syscall(SYS_gettid)`）
- 引数は読まない（関数は引数を取らない）
- 失敗応答は作らない
- 実装: `lift/runtime/fle_imports.c` の `fle_imp_GetCurrentThreadId`
- 接続: 生成 C の thunk 行を `lift/out-imp/build.sh` が `sed` で書き換えた `lift/out-imp/after.c` の中で、`fle_imp_GetCurrentThreadId(); return;` に置き換える。元の `lift/out-string/after.c` は変更しない。

## 根拠（仕様の出典）

- 出典: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getcurrentthreadid （2026-10-20 に取得）
- 読んだ箇所: Syntax は `DWORD GetCurrentThreadId();`。Return value は「呼び出しスレッドのスレッド識別子」。Remarks は「スレッドが終了するまで、その識別子は system 全体でそのスレッドを一意に識別する」。
- この出典から言えること: 引数なし、戻り値は DWORD、呼び出しスレッドの ID を返す。失敗の記述はない。
- この出典から言えないこと: 値が Linux の tid と一致すべきという仕様はない。切り詰めた値が一意かどうかも保証されない（tid は 32 ビット以内に収まるが、切り詰めの一意性は未検証）。

## 代替案 B との比較

B: harness が起動時に値を決め、それを返す。

- B の利点: 値を決定的にできる。再現性が高い。
- B の欠点: 複数スレッドの区別ができず、呼び出しスレッドごとの値にならない。仕様（呼び出しスレッドの ID）と合わない。
- A の利点: 仕様の文に沿う（呼び出しスレッドごとに値が変わる）。
- A の欠点: 実行ごとに値が変わる。値の一意性は切り詰めに依存する。

作業者は A を選んだ。仕様は「呼び出しスレッドの ID」であり、決定的な値は要求していないため。

## 検証の範囲（実測）

- ビルド: `sh lift/out-imp/build.sh` が成功（`lift/out-imp/flengine_imp`）。
- 実行: `lift/out-imp/run1.txt`。停止点は `0x406f90` を通過し、次は `0x41d050` の dispatch miss（`fle_dispatch_miss`、SIGABRT）。`f_406f90` は戻った。
- 未検証: 返した値が実際の呼び出しスレッド ID と一致することは、この実行では確認していない。
