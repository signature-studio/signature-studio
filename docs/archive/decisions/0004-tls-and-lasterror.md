# 0004: TlsSetValue、TlsGetValue、GetLastError の応答

状態: 作業者（PM）が決定。依頼者の決定は含まない。実装は `lift/runtime/fle_imports.c` と `.h` に追記済み（既存の関数は変更していない）。

## 文脈

前回（`lift/out-localalloc/run1.txt`）の停止点は 0x41cf70 の `jmp [0x19717a8]`（kernel32!TlsSetValue）。呼び出し元は 0x41d000（0x41d044）。同じ経路の TlsGetValue（0x41cf60、IAT 0x1971778）と GetLastError（0x41cf80、IAT 0x19713c0）も未対応だった。

詳細: `docs/investigations/import-TlsSetValue.md`、`import-TlsGetValue.md`、`import-GetLastError.md`。実行結果は `docs/investigations/tls-run.md`。

## 仕様の出典

- TlsSetValue: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-tlssetvalue （2026-10-20 取得）
- TlsGetValue: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-tlsgetvalue （2026-10-20 取得）
- GetLastError: https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror （2026-10-20 取得）
- ERROR_INVALID_PARAMETER（87）: https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499- （2026-10-20 取得）
- TLS_OUT_OF_INDEXES（0xFFFFFFFF）: Microsoft の TlsAlloc ページには数値が書かれていない（この調査で未確認）。ローカルの mingw ヘッダ `/usr/share/mingw-w64/include/processthreadsapi.h:21` の `#define TLS_OUT_OF_INDEXES ((DWORD)0xffffffff)` を二次資料として確認した。

## 決定

1. **TlsSetValue**: 索引が TlsAlloc で払い出された範囲（`fle_tls_next_index` 未満）なら値を保存して TRUE（R[0] = 1）。範囲外なら FALSE（R[0] = 0）、last error = 87。
2. **TlsGetValue**: 払い出された範囲なら値を返し、last error = 0（仕様の「clears」に従う）。範囲外なら NULL、last error = 87。
3. **GetLastError**: `fle_last_error` を返す。初期値 0。
4. **スレッド**: 値は索引ごとに 1 つ。持ち主スレッドは最初の呼び出しの gettid で決まる。別スレッドからの呼び出しは stderr にメッセージを出して abort（fail-closed）。

## 仕様から言えること／言えないこと

- 言えること:
  - TlsSetValue は 2 引数、BOOL。成功 nonzero、失敗 0。
  - TlsGetValue は 1 引数。有効な索引なら常に成功。成功時に last error を 0 にする。
  - 索引が払い出された後のスロットは 0（NULL）に初期化される（TlsAlloc Remarks）。
  - GetLastError は呼び出しスレッドの last-error を返す。
- 言えないこと:
  - 無効な索引のときの具体的な error code（3 つのページとも書いていない）。87 は作業者の選択。
  - GetLastError の初期値。
  - 範囲外の索引の挙動（TlsSetValue の Remarks は「0 から TLS_MINIMUM_AVAILABLE − 1 の範囲で検証なしに成功する」と書くが、範囲外の失敗条件は書かれていない。TLS_MINIMUM_AVAILABLE の数値は未確認（TLS ページは「少なくとも 64」）。「0〜63」は推論）。

## 作業者が決めたこと（仕様の外）

- 無効な索引の error code = 87（ERROR_INVALID_PARAMETER）。仕様は値を定めない。
- GetLastError の初期値 = 0。
- 持ち主スレッドを最初の呼び出しの gettid で決め、別スレッドは停止させる。
- 失敗時に stderr へ出すメッセージの文言。

## 制限（判断記録として明記）

- **スレッドは 1 つだけ**。値は「スレッドごと」ではなく「索引ごと」に 1 つ持つ。別スレッドの呼び出しは abort する。単一の実行で十分な範囲に限る。
- gettid は保持場所の鍵ではなく、持ち主の確認に使う。GetCurrentThreadId の応答と同じ gettid を使うが、鍵の仕組みは別。
- TlsFree は未実装。解放された索引の再利用はしない。
- last error は全スレッドで 1 つの変数（持ち主スレッドの前提のため）。
- TlsAlloc と LocalAlloc は last error を設定しない（既存の応答。変更していない）。

## -1 比較（0x41d010）の判定

- 0x41d010: `cmpl $0xffffffff, [0x13a9388]`。[0x13a9388] は 0x41d066 で保存された TlsAlloc の戻り値。
- 仕様との照合: TLS_OUT_OF_INDEXES は TlsAlloc の失敗値。Microsoft のページは数値を書いていないが、mingw ヘッダで 0xFFFFFFFF と確認。よって比較値は失敗値と一致する。
- ただし、不一致時（jne 0x41d023）も一致時（0x41d019 → 0x412a60）も、いずれも LocalAlloc と TlsSetValue の経路に進む。失敗時に処理を打ち切らない。0x412a60 は、スタック上の引数を [0x13a5008] に保存し、cl（0xe2）を 0x412a40 に渡して [0x13a5000] に保存する（読んだ範囲）。0x4128b0 以降の分岐は一部のみ読んだ。失敗の記録という役割は推測。
- 実行: 今回は索引が 0 のため -1 比較は不一致。0x412a60 は呼ばれていない（entry log に現れない）。

## 代替案

- B: TlsSetValue と TlsGetValue を常に成功させ、値を索引に関係なく 1 つだけ持つ。
  - 利点: 実装が最も単純。
  - 欠点: 索引ごとの分離が仕様（"each thread receives its own slot for the index"）と合わない。将来の複数の索引で衝突する。
- C: 別スレッドの値も保持する（スレッドごとの配列）。
  - 利点: 仕様どおり。
  - 欠点: 単一の実行には不要。スレッド生成の経路も今はない。範囲外。
- 採った案 A（索引ごと、持ち主 1 スレッド）は、仕様の索引分離に沿い、今の実行で必要な範囲に限る。

## 検証の状態

- ビルド: 警告 0 で成功（`lift/out-tls2/build.sh`）。
- 実行: 停止点は 0x40a3b2（thunk 0x40a3b0 の `stmxcsr`）。TlsSetValue と TlsGetValue の経路は通過（entry log による推論。詳細は `tls-run.md`）。
- GetLastError: 未到達。応答の正しさは未検証。
