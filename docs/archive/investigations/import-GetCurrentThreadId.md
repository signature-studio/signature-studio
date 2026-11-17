# kernel32!GetCurrentThreadId の仕様確認と、0x41219a の通過確認

## 1. 0x41219a の通過（実測）

- 実行: `lift/out-trace/flengine_probe`（`lift/out-trace/build.sh` で作成）。入力は `lift/out-string/after.c`。
- 結果: 停止は前回と同じ `0x406f90`（`jmp [IAT 0x19714f0]` = kernel32!GetCurrentThreadId）。`fle: unsupported instruction at 0x406f90`。
- 関数実行 7、呼び出し深さ 4。
- 根拠（実測）: 新しい trace は関数入場時に guest の戻り番地（`R[4]` の先頭 8 バイト）を記録する。
  - 停止時の `f_406f90` の戻り番地は `0x4121cc`。これは `0x4121c7` の `call` の次の命令である。
  - `0x412160` の戻り番地は `0x41d989`。これは `f_412160` の呼び出し元である。
  - よって、`0x4121c7` の `call` は実行されている。
- 補強（生成コードの読取）:
  - `0x41219a` は `rep movsq`（`while (R[1] != 0)` のループ）。外部呼出しを含まない。
  - `0x41219a` から `0x4121c7` までは分岐命令がなく、この区間の `goto` も生成コードにない（`out-string/after.c` を検索して確認）。
  - 以上から、`0x41219a` は通過している。これは実測（戻り番地）と読取の両方で一致する。

## 2. GetCurrentThreadId の仕様（一次資料）

- 出典: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getcurrentthreadid （取得日 2026-10-20、ms.date 2024-02-02、updated_at 2025-07-01）
- 読んだ該当部分の要約:
  - Syntax: `DWORD GetCurrentThreadId();`。引数はない。
  - Return value: 「呼び出しスレッドのスレッド識別子」を返す。
  - Remarks: スレッドが終了するまで、その識別子はシステム全体で一意である。
  - DLL: Kernel32.dll。
- 確認できた点:
  - 引数なし。
  - 戻り値の型は DWORD（32 ビット符号なし）。
  - 戻り値は呼び出しスレッドの ID。
  - 一意性はスレッド終了までの範囲で保証される。
- 確認できなかった点:
  - 「失敗しない」は、この記述ページでは明示されていない。エラー条件や `GetLastError` の扱いの記述もない。「失敗しない」は仕様として確定できない。
  - 具体的な ID の値（数値の範囲や割当規則）は規定されていない。

## 3. 応答の設計案（仕様が確認できた範囲のみ。実装はしない）

- 確認できた仕様に基づく応答: 引数を読まず、`R[0]` に呼び出しスレッドの ID を 32 ビットで返す（上位 32 ビットは 0）。
- 設計上の論点（決定が必要）:
  - 値の決め方: 仕様は値を規定しない。固定値を既定として埋めると、ADR-0010 が採らないとする「名前付き既定値」に当たる恐れがある。
    - 候補 A: 実行中の harness のスレッドを表す値（例: ホストの `gettid()`）を返す。仕様の「スレッドごとに一意」に合う。
    - 候補 B: harness が割り当てる単一のスレッド ID を、起動時に一度だけ決めて全呼出しで返す。
    - どちらを採るかは依頼者が決める事項。本調査では選ばない。
  - 失敗の扱い: 仕様は失敗を記述していないため、失敗の応答を harness で作らない。未知の失敗は fail-closed（停止）のままにする。
  - 適用範囲: import の stub は `0x406f90`（thunk）に対応する。`out-imports/thunk_map.txt` の 0x19714f0 の行で対応を確認した。

## 4. 残りの作業量

- 実測: 0x41219a は通過。停止は `0x406f90`（GetCurrentThreadId）で、ここで止まっている。
- 未確認: この import を越えた後の挙動。依頼文の「未対応 1,576 件」は本調査で数えていない。次の停止点はこの import を越えて初めて分かる。
- 前提数（依頼文と `out-imports/thunk_map.txt` の記載）: 通常 import 409 件、delay import 14 件、call 0 件。これらが IAT 経由 jmp の内訳であり、仕様を一件ずつ確認する必要がある。
- ADR-0010: `docs/decisions/` は存在せず、リポジトリ内に ADR-0010 は見つからなかった。A4 の定義は依頼文の記述のみを根拠にしている。
