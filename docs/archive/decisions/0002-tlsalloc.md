# 0002: TlsAlloc の応答

状態: 作業者（PM）が決定。依頼者の判断は含まない。実装は未変更（案のコードのみ記載）。

## 文脈

停止点は `0x41cf40` の `jmp [IAT 0x19712b8]`（kernel32!TlsAlloc）。呼び出し元は `0x41d050`（唯一の呼び出し箇所 `0x41d061`）。戻り番地は `0x41d066`。詳細と逆アセンブルは `docs/investigations/import-TlsAlloc.md`。

## 仕様の出典

- 出典: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-tlsalloc （2026-10-20 に取得）
- Syntax: `DWORD TlsAlloc();`。引数なし。
- Return value: 成功時は TLS index を返し、そのスロットは 0 に初期化される（原文 "The slots for the index are initialized to zero."）。失敗時は `TLS_OUT_OF_INDEXES` を返し、詳細は `GetLastError` で得る。
- Remarks: 索引の値は不透明（"should be treated as an opaque value; do not assume that it is an index into a zero-based array."）。
- 上限の出典: https://learn.microsoft.com/en-us/windows/win32/procthread/thread-local-storage （2026-10-20 に取得）。「The maximum number of indexes per process is 1,088.」

### この出典から言えること

- 引数なし、戻り値は DWORD。
- 索引の数は 1 プロセスあたり最大 1,088。
- 確保された索引の各スロットは 0（NULL）に初期化される。

### この出典から言えないこと

- 索引の値の範囲。依頼文にあった「0 から 1088 の範囲」は仕様に書かれていない（1,088 は索引の数の上限であり、値の範囲ではない）。値の範囲は仕様が定めないので、作業者が決める（後述）。
- 「プロセス内で一意」の明文。読んだ箇所にはない。
- `TLS_OUT_OF_INDEXES` の数値（読んだページには書かれていない。一般に 0xFFFFFFFF とされるが、この調査では未確認）。

## 戻り値の使われ方（0x41d050 の周辺、objdump で確認）

- 0x41d061 の `call 0x41cf40` の後、0x41d066 で `mov %eax,[0x13a9388]`。戻り値（32 ビット）をそのまま保存する。
- その後 0x41d06c で `0x41d000` を呼ぶ。0x41d000 は [0x13a9388] を読み、
  - `cmpl $-1` で比較する（-1 の場合は `0x412a60` を呼ぶ。失敗時の記録と推測）。
  - 0x41d077 で `TlsGetValue`（0x41cf60）に索引を渡す。
- 結論: **戻り値は使われる**。索引として以後の TLS 呼び出しに渡される。したがって、ゼロを返すだけの応答では後続の TLS 操作が同じ索引に対して動く。仕様が未定義のまま決めてはならない。

## 決定

案 A を採る: **呼ばれるたびに、harness 内の単調増加の索引を返す。上限は 1,088 個まで。上限に達したら `0xFFFFFFFF`（TLS_OUT_OF_INDEXES と仮定）を返す。**

### 理由（仕様の出典に基づく）

- 仕様は索引を「不透明な値」と定める。値の内容は決まっていないので、harness が決めても仕様違反にはならない。
- 仕様は「1 プロセスあたり最大 1,088 個」の索引を定める。したがって、確保のたびに新しい索引を返し、上限で止める案は仕様の範囲内。
- 各索引のスロットは 0 に初期化される（Return value）。したがって、確保直後の `TlsGetValue` 相当は NULL を返すことが仕様から言える。
- 案 A は「TLS の索引は確保ごとに別物である」という TLS の用途（"each thread receives its own slot for the index"）と矛盾しない。

### 限界（仕様から言えないこと）

- 開始値（0 か 1 か）は仕様にない。作業者の決定（下の実装案では 0 から始める。0 が有効な索引かどうかは仕様から言えない）。
- 上限到達時の `0xFFFFFFFF` は、TLS_OUT_OF_INDEXES の数値を未確認のまま使っている。確認するまでは仮定。
- TlsFree は未実装のため、解放された索引の再利用は行わない。

## 代替案 B との比較

B: 固定値を返す（例: 常に同じ値）。

- B の利点: 実装が最も単純。結果が決定的。
- B の欠点: 2 回の `TlsAlloc` が同じ索引を返すと、仕様の「各確保は別のスロットを持つ」という用途と合わない。将来 `TlsSetValue` を実装したとき、別々の TLS 変数が衝突する。
- A の利点: 確保ごとに別の索引を返せる。上限を仕様（1,088）から取れる。
- A の欠点: 索引の値を決める作業者の判断が入る（開始値）。値は実行ごとに同じだが、仕様はこの値を保証しない。

採った理由: 仕様の "each thread receives its own slot for the index"（スロット）と "maximum number of indexes per process is 1,088"（上限）の両方に沿うのは A。B は 2 回目以降の確保を区別できない。

## 実装案（未変更。`lift/runtime/fle_imports.c` に追記する案）

```c
/* kernel32!TlsAlloc: DWORD TlsAlloc(void). The index is opaque (MSDN: TlsAlloc
 * Remarks). The harness hands out a fresh index per call, up to the per-process
 * maximum of 1,088 (MSDN: Thread Local Storage). */
#define FLE_TLS_MAX_INDEXES 1088u
#define FLE_TLS_OUT_OF_INDEXES 0xFFFFFFFFu

static uint32_t fle_tls_next_index;

void fle_imp_TlsAlloc(void)
{
    uint32_t idx = __atomic_fetch_add(&fle_tls_next_index, 1, __ATOMIC_SEQ_CST);
    if (idx >= FLE_TLS_MAX_INDEXES) {
        __atomic_fetch_sub(&fle_tls_next_index, 1, __ATOMIC_SEQ_CST);
        R[0] = FLE_TLS_OUT_OF_INDEXES;
        return;
    }
    R[0] = idx;
}
```

`lift/runtime/fle_imports.h` に追加する宣言:

```c
/* kernel32!TlsAlloc: DWORD TlsAlloc(void). */
void fle_imp_TlsAlloc(void);
```

接続（生成 C の書き換え、`lift/out-seed41d/build.sh` と同じ方式の案）:

```
s|^L_41cf40: { /\* 0x41cf40 Jmp \*/ fle_unsupported(UINT64_C(0x41cf40)); }$|L_41cf40: { /* 0x41cf40 jmp [0x19712b8] kernel32!TlsAlloc */ fle_imp_TlsAlloc(); return; }|
```

注意:
- 上の案は未コンパイル・未実行。コードは案にとどまる。
- `__atomic_fetch_sub` の巻き戻しは、上限を超えた分を戻すための処理。並行呼び出しの厳密さは未検証。
- `fle_tls_next_index` の 0 始まりは作業者の決定（仕様から導けない）。
- 上限到達時の `0xFFFFFFFF` は、仕様の数値を未確認のまま使っている。

## 失敗時の扱い（TLS_OUT_OF_INDEXES）

- 0x41d000 は [0x13a9388] が -1 のとき `0x412a60` を呼ぶ。この分岐は、失敗時の応答が必要かどうかを示す（直接の判定は次項）。
- 上限到達は本件（1 回の起動）では起きない見込み（推測）。0x41cf40 の呼び出しは 0x41d061 の 1 箇所のみで、TlsAlloc の呼び出しは 0x41d050 の 1 回だけと見える。

## 次の作業

- 案 A の実装（未実施。依頼に従い docs のみ）。
- 0x41cf20 の LocalAlloc、0x41cf70 の TlsSetValue、0x41cf60 の TlsGetValue。いずれも未対応。
- 0x412a60 と 0x412a40 の中身（未読）。
