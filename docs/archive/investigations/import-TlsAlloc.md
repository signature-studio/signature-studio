# import-TlsAlloc: 仕様の出典、読んだ箇所、確認できなかった点

作業者（PM）の調査。実装は変更していない。

## 対象

- VA 0x19712b8（IAT）、kernel32.dll!TlsAlloc（`lift/out-imports/imports.txt` の 284 行）
- 停止点: 0x41cf40 の `jmp [0x19712b8]`（`lift/out-seed41d/run1.txt`）
- 呼び出し元: 0x41d050（唯一の呼び出し箇所は 0x41d061 の `call 0x41cf40`）。戻り番地 0x41d066 が保存先。

## 仕様の出典

### 1. TlsAlloc

- URL: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-tlsalloc （2026-10-20 取得。ページの ms.date は 2024-12-05、updated_at は 2025-07-01）
- 読んだ箇所と要約:
  - Syntax: `DWORD TlsAlloc();`。引数なし。
  - Return value: 成功時は TLS index を返し、「The slots for the index are initialized to zero.」。失敗時は TLS_OUT_OF_INDEXES を返し、詳細は GetLastError で得る。
  - Remarks: 「The value of the TLS index should be treated as an opaque value; do not assume that it is an index into a zero-based array.」（索引の値は不透明。0 始まりの配列の添字と仮定しないこと）。「When a TLS index is allocated, its storage slots are initialized to NULL.」（確保時、スロットは NULL に初期化される）。TlsFree / TlsSetValue / TlsGetValue を後続で使う、とも書かれている。
  - Remarks の Windows 8.1 以前の記述: Windows Store app では FlsAlloc にインライン置換される。対象は Store app のみ。

### 2. TLS の上限

- URL: https://learn.microsoft.com/en-us/windows/win32/procthread/thread-local-storage （2026-10-20 取得）
- 読んだ箇所: 「The constant TLS_MINIMUM_AVAILABLE defines the minimum number of TLS indexes available in each process. This minimum is guaranteed to be at least 64 for all systems. The maximum number of indexes per process is 1,088.」

## 仕様から言えること

- 引数なし、戻り値は DWORD。
- 成功時は索引、失敗時は TLS_OUT_OF_INDEXES。
- 確保された索引のスロットは 0（NULL）に初期化される。
- 1 プロセスあたりの索引の最大数は 1,088。
- 索引の値は不透明。0 始まりの添字とみなしてはならない。

## 仕様から言えないこと（確認できなかった点）

- 索引の値の範囲。「0 から 1088」とは書かれていない。1088 は索引の「数」の上限であって、値の範囲ではない。
- 「プロセス内で一意」の文字どおりの記述。読んだ箇所には明記がない。一つの索引が確保されている間はその索引が他と区別される、という意味は読み取れるが、2 回の TlsAlloc が必ず異なる値を返すとは明記されていない。
- TLS_OUT_OF_INDEXES の数値。読んだページには書かれていない。一般的に 0xFFFFFFFF と言われるが、この調査では未確認。
- GetLastError で得られる失敗コードの値（未読）。
- TlsFree、TlsSetValue、TlsGetValue の仕様（未読。別の調査で読む）。
- 対象が Windows デスクトップ DLL であること（Store app の FlsAlloc 置換の記述が対象外と言えるかは未確認）。

## 逆アセンブル（binaries/win64/FLEngine_x64.dll、objdump を直接使用）

### 0x41d050（呼び出し元）

```
41d050: sub    $0x28,%rsp
41d054: lea    0x32a8,%rax        ; rax = 0x32a8（定数）
41d05c: test   %rax,%rax
41d05f: je     0x41d083            ; rax は非ゼロなので取られない
41d061: call   0x41cf40            ; TlsAlloc
41d066: mov    %eax,0xf8c31c(%rip) ; [0x13a9388] = 戻り値（32 ビット）
41d06c: call   0x41d000
41d071: mov    0xf8c311(%rip),%ecx ; ecx = [0x13a9388]
41d077: call   0x41cf60            ; TlsGetValue
41d07c: mov    %rax,0x11cf24d(%rip) ; [0x15ec2d0] = TlsGetValue の戻り値
41d083: add    $0x28,%rsp
41d087: ret
```

### 0x41d000（0x41d06c から呼ばれ、保存済みの索引 [0x13a9388] を使う）

0x41d000 は TlsAlloc の戻り値を直接読むのではなく、0x41d066 で保存された [0x13a9388] を読む（0x41d06c の呼び出し前に ecx へ入れる処理はなく、関数内で `[0x13a9388]` を参照する）。

- 0x41cff0 は `lea 0x32a8,%rax; ret`。定数 0x32a8 を返す。ebx に格納し、0 なら 0x41d049 で終了。
- 0x41d010: `cmpl $0xffffffff,[0x13a9388]`。等しければ `0x412a60` を引数 0xe2 で呼ぶ（失敗時の記録と推測）。その後も分岐せず 0x41d023 へ進む。
- 0x41d023〜0x41d025: `0x41cfd0` を呼ぶ。`0x41cfd0` は `mov $0x40,%eax; mov %ecx,%edx; mov %eax,%ecx; call 0x41cf20`。0x41cf20 は IAT 0x19717c8 = kernel32!LocalAlloc（`imports.txt` 446 行）。したがって `LocalAlloc(0x40, 0x32a8)`。
- rax != 0: 0x41d041 で `TlsSetValue([0x13a9388], rax)`（0x41cf70 → IAT 0x19717a8 = TlsSetValue）。
- rax == 0: 0x41d034 で `0x412a60` を引数 0xe2 で呼び、0x41d049 で終了。

注意: 0x41d010 の -1 比較の後も処理は続くため、索引が -1 のままでも TlsSetValue（0x41cf70）が呼ばれる。その結果は TlsSetValue の仕様（未読）に依存する。

### 確認した事実

- IAT 0x19712b8 を参照する命令は、objdump 全体の grep で 0x41cf40 の jmp の 1 件のみ。
- `call 0x41cf40` は 0x41d061 の 1 件のみ。
- 0x41d050〜0x41d087 の範囲では GetLastError（0x41cf80）を呼ばない。

### 推測（実行で未確認）

- 0x41d000 の -1 比較は「未確保」の印として書かれていると思われる（TLS_OUT_OF_INDEXES と同じ値が使われている可能性）。
- 0x412a60 は「エラーの記録または報告」の関数と思われるが、中身は未読。
- LMEM_ZEROINIT が 0x40 であることは一般知識（この調査では未確認）。

## 0x412241 周辺（戻り番地）

- 0x412241 は `cmpq $0x0,0x160(%rbp)` で始まる。0x41d050 は `je 0x41d083` で戻ることはなく、0x41d087 の `ret` で 0x412241 に戻る（trace の guest-return=0x412241 と一致）。
- 0x412241 の直前の命令群（0x412226〜0x41223e）は、0x41d050 の戻り値とは無関係な分岐（0x170(%rbp) の値で `call *(%rbx,%rsi,8)` を分ける）。0x41d050 の戻り値を直接は読んでいない。

## 次の停止点（予測。未実行）

- 0x41d066 を通過すると、次の import は 0x41cf20 の LocalAlloc（IAT 0x19717c8）。TlsSetValue（0x41cf70）と TlsGetValue（0x41cf60）が続く予定。
- これは逆アセンブルから読んだ順序であり、実行で確認するまでは確定ではない。

## 残りの作業量（この調査の範囲）

- TlsAlloc の応答（本件）。
- 同じ経路の import: LocalAlloc、TlsSetValue、TlsGetValue の 3 件。いずれも未対応。
- 0x412a60 と 0x412a40 の中身（未読）。
- 全体の残り件数は、既存メモの 1,575 件（未分類）から見積もれない。import 以外の fle_unsupported も含むため。
