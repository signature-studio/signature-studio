# mt3: ハイブリッド + GIL の壁の除去(MXCSR とスクラッチバッファのスレッドごと化)と、セマフォの実装(PM 指示 2(b)(d)。決定番号は PM)

状態: **ワークツリー h6(6eb1737d 基準)での実装と単体テストまで**。ビルド・ブラウザでの測定は 0234p に書く。

## 1. `--hybrid-mx-tls`(fe-wasmgen)と C 側
- 壁(0234n §5): 実スレッド版は MXCSR / x87 / `fle_landing_top` / SEH を Worker ごとのブロック `fle_ctx`(`FLE_TLS_MORE`)に置く。直 wasm(fe-wasmgen のハイブリッド)は MXCSR の語(`fle_mxcsr`)とスローパスのスクラッチバッファ(`fle_hyb_buf`、16 バイト × 4)を**固定番地**で読み書きするので、(i) リンクマップに番地が無くビルドが止まる、(ii) 番地があっても Worker 間で共有になり競合する(バッファは並行して使われる)。
- 直し(スレッドごとにする。グローバルの MXCSR は着地/SEH がスレッドごとなので不可):
  - `lower.rs`: MXCSR の語(`mx_addr`)の読み 2 か所、バッファ(`mx_buf`)の書き 1・読み 6 か所を `mx_base(addr)` 経由にした。`--hybrid-mx-tls` のとき `reg_base`(R/FL/X と同じ仕組み)で `global.get __tls_base` + `(addr − hybrid_r + tls-ctx のオフセット)` になる。`--hybrid-mx-tls` が無いときは従来と**同じバイト列**(`i32.const 0` + offset=addr)。
  - `Config::hybrid_mx_tls` と CLI の `--hybrid-mx-tls`(`--tls-ctx` が要る)。`--hybrid-mx` の 2 つの番地は『レジスタページの番地空間』: `hybrid_r + FLE_CTX_MXCSR`(0x4CC)と `hybrid_r + FLE_CTX_HYBBUF`(0x500)。
  - C: `fle_lowregs.h` に `FLE_CTX_HYBBUF 0x500`、`FLE_CTX_SIZE 0x540`。`fle_ctx_more.h` に `fle_hyb_buf` の定義(`fle_ctx` の中の `fle_xmm[4]`)。`harness/hyb1/hybrid_glue.c` は `FLE_TLS_MORE` のとき自前の配列を持たない。
  - ビルドスクリプト(`app/scripts/build-engine-bundle.sh`、h6 の作業版): `GIL=1` のとき hybrid 段が `MXA = RB + 0x4CC`、`BUFA = RB + 0x500` を使い `--hybrid-mx-tls` を足す。リンクマップの番地の検査(`hyb_guard cpart/merged`)は、この 2 つについては不要(`i32.const <番地>` が無い)なので外す。`fle_ctx` の大きさの正規表現を `300|540` に。
- テスト(`crates/fe-wasmgen/tests/mx_hybrid_tls.rs`): `mx_hybrid.rs`(CVTSI2SS / CVTSD2SI / CVTSS2SI の窓の経路と C 経路の呼び出しをホスト CPU と比較)の同じ内容を、TLS の MXCSR/バッファで通す。`__tls_base` を渡し、MXCSR を `TLSBASE+OFF+0x4CC` に置く。**感度の確認**: MXCSR をわざと別の番地に書くと、『窓の状態なのに速い経路を取る入力が 1 つも無い』の主張で失敗する(元の `mx_hybrid.rs` にはこの主張が無い)。`mx_hybrid` / `tls_ctx` / `mx_hybrid_tls` は通る。
- 未確認: 2 つのインスタンスを同じメモリの上で別の `__tls_base` で動かして互いの MXCSR を見ないことのテスト(レジスタページと同じ写像なので同じ機構。ハイブリッド版のブラウザ測定で見る)。

## 2. セマフォ(`CreateSemaphoreW` / `ReleaseSemaphore`。PM 許可、Learn 確認後)
- Learn で確認した契約(2026-11-05 に読んだ): `lMaximumCount` は 0 より大きい; `lInitialCount` は 0 以上で `lMaximumCount` 以下; 状態は count > 0 で signaled; 待ちが 1 つ満たされるごとに count が 1 減る; `lReleaseCount` は 0 より大きい; 加えて最大値を超える場合は count を変えず FALSE; `lpPreviousCount`(NULL 可)に前の値; 名前つきは既存のイベント・ミューテックス等と名前空間を共有(今は名前つきは止まる)。
- ページが**書いていない**もの(選択と印を付けた): 範囲外の引数のエラー番号(ERROR_INVALID_PARAMETER 87: assumption)、最大値超えのエラー番号(ERROR_TOO_MANY_POSTS 298: System Error Codes による推定)、イベントのハンドルに ReleaseSemaphore / セマフォのハンドルに SetEvent を渡したとき(ERROR_INVALID_HANDLE 6: assumption)。失敗時に `lpPreviousCount` は書かない(ページは書かない: 変えないと読める)。
- 実装(`runtime/fle_event.c`): イベントの表に種別『セマフォ』(`sem`、`count`、`max`)。待ち(`WaitForSingleObject`、スケジューラの `fle_event_wait_state` 経由の `WaitForMultipleObjects`)と `CloseHandle` はイベントの経路に載る。`ReleaseSemaphore` は `fle_thread_wake_check` で待ち手を起こす。`fle_event_host_set`(DS の通知)はセマフォを拒否。ハンドルはイベントと同じ領域。
- `imports_table.rs` に 2 行(`kernel32.dll`)。**import 表への追加なので生成 C が変わる**: 再 lift が要る(opt1 の push の後)。この木では未実施(`fe-lift lift --pdata` の既定の sha と変更行数は、再 lift をした時点で前後を数える)。
- テスト: `crates/fe-lift/tests/semaphore.rs`(6 件: 表とヘッダ、count と待ち、前の値と最大値超え、境界値と上位 32 ビット、種別違いと CloseHandle、名前つき/属性つきの停止)、`wall_fr8.rs` に 1 件追加(2 つのスレッドが待つセマフォを ReleaseSemaphore(1) → (3) で順に起こす、WaitForMultipleObjects(all) が各セマフォから 1 ずつ取る/取らない)。`wall_fr8.rs` の『未実装のままの名前』の一覧から `CreateSemaphoreW` を外した。event(16)・wall_fr8(46)・semaphore(6)は通る。
- プロセッサ数: 設計は 0234n §4 のまま(ページが `navigator.hardwareConcurrency` を事実として渡し、GetSystemInfo 等が n を返す)。セマフォが入ってから、n ≥ 2 で FL がその先で止まる API があるかを見る。**未実施**。
