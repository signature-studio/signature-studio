# #1426 測定の表: 通知位置のバイト単位の受け付けと、再生の順序(P3)

判定:
- 受け入れ: 差し戻し。`tools/ab/run.mjs p2` の出力の行: `pass false`・`minWindowFps: changed median >= 0.90 x base median: false {"base":7.98,"changed":6.99}`(0.90 × 7.98 = 7.18 を下回る)。
- 変更: 残す(残す条件を満たす)。`wall_fr15` の `sc_notify_bytes` が通知の位置をバイト単位で返す(仕様の修正)。p3 の `pcmRepeat8192` は基準 0・変更後 0(0.01 未満)。`underrunDelta` の変更後の中央値 11 は、基準の最大 14 + 4 以下。

## 受け入れの出力の行

- `ab: p3 base 1..3`・`ab: p3 changed 1..3`: 6 run とも `exit 0, error null`。
- `tools/ab/run.mjs p3` の判定(pass true): `every changed run: error is null [null,null,null]`・`playing is true [true,true,true]`・`inputRateOk is true [true,true,true]`・`nonSilentDelta is above 0 [1146,1146,1146]`・`pcmRepeat8192 is null or below 0.01 [0,0,0]`・`underrunDelta: changed median <= base max + 4 {"base":11,"baseMax":14,"changed":11}`・`last60sUnderruns: changed median <= base max + 4 {"base":11,"baseMax":14,"changed":11}`。
- `tools/ab/run.mjs p2` の判定(pass false): `every run: error is null: true`・`every run: inputRateOk is true: true`・`minWindowFps: changed median >= 0.90 x base median: false`・`inputToPresentMsP95: changed median <= 1.20 x base median: true {"base":189.425,"changed":196.075}`。
- `tools/ab/base.mjs --verify .tmp-base`: `{"ok":true,"changed":[]}`。
- `wall_fr14`: 26 passed; 0 failed。`wall_fr15`: 25 passed; 0 failed(`sc_notify_bytes` を含む)。
- 起動確認(`boot-bundle.mjs`): state `engine-idle`・standIn false・error null・elapsedMs 11292。
- 流れ A〜D(`welcome-flow.mjs`): 4 流れとも error null・engine-idle・standIn false・trapped false・hostErrors 0・opened true。elapsedMs 11587〜11758(上限 12500)。B と D は welcomeClosed true。`flow judge: true`。
- engine の sha256: 基準 `8a5b07b6…`(167567118 B)、変更後 `be2dfe21…`(167567152 B)。変更は engine に入っている。

## p3 の 6 run(`tools/ab/run.mjs p3 ... 3`。run の順は 基準 1・変更後 1・基準 2・変更後 2・基準 3・変更後 3)

| 版 | run | pcmRepeat8192 | pcmEligible | underrunDelta | last60sUnderruns | nonSilentDelta |
|---|---|---|---|---|---|---|
| 基準 | 1 | 0 | 34718 | 11 | 11 | 1146 |
| 変更後 | 1 | 0 | 34718 | 15 | 15 | 1146 |
| 基準 | 2 | 0 | 34718 | 14 | 14 | 1146 |
| 変更後 | 2 | 0 | 34718 | 11 | 11 | 1146 |
| 基準 | 3 | 0 | 34718 | 11 | 11 | 1146 |
| 変更後 | 3 | 0 | 34718 | 11 | 11 | 1146 |

中央値(基準 / 変更後): pcmRepeat8192 0 / 0、underrunDelta 11 / 11、last60sUnderruns 11 / 11、nonSilentDelta 1146 / 1146。基準の最大(underrunDelta・last60sUnderruns): 14。

## p2 の 6 run(`tools/ab/run.mjs p2 ... 3`。p3 の run には minWindowFps が無いので、この列は p2 から取った)

| 版 | run | minWindowFps | inputToPresentMsP95 (ms) | inputDropped | error |
|---|---|---|---|---|---|
| 基準 | 1 | 7.98 | 197.88 | 110 | null |
| 変更後 | 1 | 7.05 | 197.83 | 97 | null |
| 基準 | 2 | 9.99 | 189.425 | 102 | null |
| 変更後 | 2 | 5.52 | 110.125 | 98 | null |
| 基準 | 3 | 6.01 | 118.31 | 99 | null |
| 変更後 | 3 | 6.99 | 196.075 | 102 | null |

中央値(基準 / 変更後): minWindowFps 7.98 / 6.99(比 0.88。gate は 0.90 以上)、inputToPresentMsP95 189.425 / 196.075(比 1.03)、inputDropped 102 / 98。

## 所見

- 効果は測れていない。`pcmRepeat8192` は全 6 run で 0(`pcmEligible` 34718)。この評価用プロジェクトでは、基準でも音の繰り返しが出ていない。
- `underrunDelta` と `last60sUnderruns` は、基準 11・14・11、変更後 15・11・11。中央値は両方 11 で、差は無い。
- `nonSilentDelta` は 6 run とも 1146(変わらない)。
- p2 の minWindowFps は、この測定の雑音の中にある。基準の 3 run は 6.01〜9.99、変更後は 5.52〜7.05 で、範囲が大きく重なる。中央値の差 0.99(約 12%)は、基準の run の散らばり(約 4)より小さい。前回の試行で同じ変更を測った p2 は、基準 8.04・変更後 8.73 で通った。符号が測り直しごとに変わるので、n=3 では回帰の有無を判定できない。
- この試行の判定は、受け入れのコマンドの 1 回の出力(上の差し戻しの行)による。落ちた後に測り直した結果で差し替えない。雑音を減らす測り直し(run の数を増やす)は、次の試行で行う。

## 変更の内容(main からの差分)

- `lift/runtime/fle_dsound.c`: 通知位置(バイト)を `notify_frame` で、切り上げてサンプル(フレーム)に直す。`fire_notifications`・`fle_audio_next_tick` で使う。`m_n_setpos` は、サンプル境界の検査を外し、範囲(`off < size`)だけを検査する。`m_b_play` は、`audio_sync()` を `fle_audio_ring_set_state(FLE_AUDIO_PLAYING)` より先に呼ぶ。
- `lift/harness/audio/ds_scenarios.c`: 試験 `sc_notify_bytes`(通知位置 1 と 3527 バイト)を足し、`S[]` に登録する。
- `lift/crates/fe-lift/tests/wall_fr15.rs`: `the_list_of_scenarios` の期待する名前の列に `notify_bytes` を足す(既存の試験の期待値は変えていない。列に 1 つ足しただけ)。`byte_offsets_round_up_to_frames_and_the_last_byte_notifies_at_wrap` を足す(1 バイトは 227 ticks、3527 バイトは 882 フレーム = 20 ms の 36000200000 ticks)。
- `lift/crates/fe-lift/tests/wall_fr14.rs`: main のまま(変更なし)。前回の取り込みで衝突した箇所は、main の版を採った。
- 計測用の unlock の情報(`fle_audio_unlock_note` の引数)は入れていない。

## 注意

- `lift/` の変更は、push で main の `deploy.yml` を動かす。judge が通って push されるときは、デプロイが走る。
- 前回の試行の表(p2 が通った版)は、この表で置き換えた。
