# 起動の後の無入力の途切れ(R18 `startup-no-dropout-10s`、DPR 1)の原因と、先読みの変更(#1432)

数字は runner の値(nproc 4。無負荷の loadavg は 1.7〜2.7、4 並列の負荷では 12〜13)。この機械とは別枠。「実測」は道具の出力、「推定」は読みと推論。耳の確認は行わない。途切れは underrun の quanta で書く(1 quantum は 128 frame = 2.9 ms)。

## 0. 結論

- 実エンジン: `bash tools/e2e/run.sh --scenarios regression --only startup-no-dropout-10s --displays 4k-dpr1` を、無負荷で 5 回続けて走らせ、5 回とも `ok: true`(underrun quanta 0、status running、context running、error none)。`node tools/boot-check/boot-bundle.mjs` と `node tools/smoke/welcome-flow.mjs` の流れ A〜D、`npm test --prefix app`、`bash guard/scan.sh` も通った。
- 途切れは、engine が音声を補充しない時間(切り替えが無い区間)が、先読み(lead)の 60 ms を超えたときに出る(実測)。負荷を掛けると、その停止は 63〜313 ms になった(実測。19 回の基準の途切れから)。
- 直したのは、先読みの既定値 `FLE_AUDIO_LEAD_MS` を 60 ms から 300 ms にしたこと(1 つの定数)。負荷を掛けた R18 は、基準が 19 回中 19 回途切れ(中央値 46 quanta)、変更後は 8 回中 0 回。再生中の p3 A/B は、基準の途切れ中央値 7(最大 8)から 0 に下がった。
- 判定: 受理(受け入れの全判定基準を満たした。push はしていない。`lift/` を変えたので、main への push で `deploy.yml` が動く)。

## 1. 再現の表(途切れの時刻と engine の状態)

時刻は、first idle(`audio.marks['first-idle']`)からの ms。負荷は R18 を 4 本同時に走らせた条件(loadavg 7〜13)。「停止」は、途切れの quanta から推定した、engine が補充しなかった時間(60 ms + 途切れ × 2.9 ms)。

| 回 | 途切れの時刻 (ms) | 途切れ(quanta・推定の停止) | 直前の Unlock からの時間 (ms) | engine の状態 |
| --- | --- | --- | --- | --- |
| 1(基準・負荷 par-5) | 2,835〜3,045 | 60 quanta(≈ 234 ms) | 2,399 | 補充が 60 ms を超えて止まった(実測)。何をしていたかは、この回では未取得 |
| 2(基準・負荷 par-6) | 3,059〜3,266 | 73 quanta(≈ 272 ms) | 2,823 | 同上 |
| 3(基準・負荷 parB-2) | 3,281〜3,432 | 66 quanta(≈ 252 ms) | 1,009 | 同上 |
| 4(基準・負荷 par-4) | 2,681〜2,790、9,540〜9,590 | 32 quanta(≈ 153 ms)、42 quanta(≈ 182 ms) | 916、1,413 | 同上 |
| 5(基準・負荷 par-1) | 2,686、3,836〜3,946 | 1 quantum(≈ 63 ms)、31 quanta(≈ 150 ms) | 692、574 | 同上 |

- 無負荷の R18(基準 20 回、変更後 5 回)は、どちらも途切れ 0 だった(実測)。負荷を掛けない限り、途切れは出なかった。
- 途切れは、最初の idle の後 0.1〜4 秒(一部は 9.5 秒)に集中した。直前に出た Unlock(音声のバッファへの書き込み)とは関係が無かった(実測。直前の Unlock は 28〜2,858 ms 前)。
- 負荷下の trace(`measure.mjs --boot --trace`、50 ms ごと)では、途切れの各時点で、リングの埋まりが 60.0 ms(先読みの全量)だった(実測)。つまり、補充が止まり、読み出しが先読みを使い切ったときに途切れている。

## 2. 原因の特定

実測:
- 補充(`audio_sync`)は、エンジンの切り替え点(スケジューラの切り替え、ループの区切り 1,024 回ごと、間接呼び出し 4,096 回ごと、DirectSound の API)でだけ呼ばれる(`lift/runtime/fle_dsound.c`)。切り替え点の無い区間が 60 ms を超えると、読み出しが先読みを使い切る。
- 負荷下の CPU profile(`measure.mjs --boot --profile`、0〜40 s、CDP の計測のため負荷は増える)で、切り替え点の無い区間が 150 ms 以上続いたものは 40 件、計 11.8 s。その 84%(9.9 s)は guest のスレッド(`fle_wasm_thread_entry`)の計算で、主な葉は fe-lift の C(`f_344fcf0`・`f_27acd60`・`f_33d6e70`)と、C のランタイムの SSE 演算(`fle_mx_*`)。16%(1.8 s)は UI のスレッドのメッセージ処理(`DispatchMessageW` → ウィンドウ手続き `f_4249a0`)。GC は 1 件(34 ms)だけだった。
- 停止の長さは、負荷下の途切れから 63〜313 ms(19 回の基準)。

推定:
- 主因は、起動の後にも続く guest の計算(切り替えの無い長い区間)が、先読みの 60 ms より長いこと。計算の中身の切り分けは、関数名だけでは確定できなかった。
- 描画(ウィンドウのメッセージ処理)は、切り替えの無い区間の約 16%(推定の上限)。GC は主因ではない(実測)。lazy の取得とプラグインの読み込みの寄与は、この道具では分けていない(未確認)。

## 3. 変えた箇所

- `lift/runtime/fle_audio_ring.h`: `FLE_AUDIO_LEAD_MS` 60u → 300u(コメントを更新)。これが効果のある変更。製品に環境変数・`#ifdef`・URL パラメータ・ビルド変数は足していない。
- `lift/runtime/fle_audio_ring.c`・`lift/runtime/fle_dsound.c`: コメントの 60 ms を 300 ms に直した(動作は変えない)。
- `lift/crates/fe-lift/tests/wall_fr15.rs`: 既定値のテスト(`the_default_lead_is_300_ms_and_the_environment_changes_it`)と、書き込みカーソルのテストの既定値を 2646 → 13230 frame に直した。`cargo test --release -p fe-lift --test wall_fr15 -- the_default_lead_is_300_ms the_write_cursor_that_the_guest_sees` は 2 件とも通った。
- `tools/e2e/scenarios/regression/r18-startup-no-dropout-10s.mjs`: 判定の式は変えず、結果の JSON に記録(`record`・`spans`)と、報告用の 1 行を足した。起動からの途切れの時刻、直前の engine の公開・Unlock・スレッドの値。

300 ms にした理由: 負荷下の停止の実測(63〜313 ms)の大半を覆うため。60 ms では、負荷下の途切れが 19 回すべてで出た。

## 4. A/B(変更の前後)

| 比較 | 基準 | 変更後 | 判定 |
| --- | --- | --- | --- |
| R18・無負荷・DPR 1 | 途切れ 0(20 回) | 途切れ 0(5 回。受け入れの回) | 通過 |
| R18・4 並列の負荷・DPR 1 | 途切れあり 19/19 回。5〜87 quanta(中央値 46) | 途切れ 0(8/8 回) | 大きく改善 |
| `tools/ab` p3(再生・マウス・音あり、n=3) | underrunDelta 中央値 7(最大 8)。nonSilentDelta 1,146 | underrunDelta 中央値 0。nonSilentDelta 1,150 | 通過(全ゲート pass) |
| `tools/ab` p2(n=2) | minWindowFps 11.49 → 11.98。inputToPresentMsP95 150.3 → 139.2 ms | 同左 | 通過(全ゲート pass) |

負荷下の基準の 19 回(R18):
par 1〜8 は 32・42・48・74・60・73・45・32、parB 1〜8 は 32・66・11・87・63・46・36・57(いずれも 4 本同時)、measure の profile と並行した 3 回は 53・5・31(quanta)。

## 5. 流れ A〜D(受け入れ、変更後のビルド)

`node tools/smoke/welcome-flow.mjs` の 4 行(`error` null、`state` engine-idle、`standIn` false、`trapped` false、`hostErrors` 0、`opened` true。B と D は `welcomeClosed` true):
- A: error null・engine-idle・standIn false・trapped false・hostErrors 0・welcomeClosed true・opened true・elapsedMs 9,692・preBootBytes 306,216,760
- B: error null・engine-idle・standIn false・trapped false・hostErrors 0・welcomeClosed true・opened true・elapsedMs 9,845・preBootBytes 306,216,760
- C: error null・engine-idle・standIn false・trapped false・hostErrors 0・welcomeClosed true・opened true・elapsedMs 9,870・preBootBytes 306,216,760
- D: error null・engine-idle・standIn false・trapped false・hostErrors 0・welcomeClosed true・opened true・elapsedMs 9,757・preBootBytes 306,216,760

起動確認(`boot-check`): error null、state engine-idle、standIn false、elapsedMs 10,169(12,500 以下)。

## 6. 未確認・限界

- 未確認: lazy の取得とプラグインの読み込みのどちらが、停止の主体か。関数名から、描画・lazy・プラグインを切り分けていない。
- 未確認: 先読みを 300 ms にした代償。書き直し(`rewrite_unread`)は、Unlock ごとに先読みの範囲を再ミックスするため、CPU の負担は先読みに比例して増える。p3 A/B の再生では、負担による途切れは出ず、フレームも減らなかった(1,146 → 1,150)が、負担の量は計っていない。開始から音までの遅れも、計っていない(推定: 新しい書き込みは書き直しの経路で早く出るため、遅れは増えないはず)。
- 限界: 負荷下の試験は、4 本の並列の R18 による人工の負荷。実機の負荷とは別枠。
- 限界: profile は CDP の計測付きの 1 回。負荷下の途切れの時刻と profile の時刻の対応は、確定していない。
- 限界: p3 の A/B は n=3、p2 は n=2。
