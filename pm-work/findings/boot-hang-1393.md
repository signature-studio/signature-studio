# 起動の止まり(#1393): 測定の道具、止まりの率、止まった時の状態、機構

数字は runner の値(Chrome 157、nproc 4、loadavg 1.0〜2.2)。この機械とは別枠。「実測」は道具の出力、「推定」は読みと推論。

## 0. 結論

- 実測: 測定の道具 `tools/boot-hang/measure.mjs` を作った。HEAD の engine(`eb118ef2` 以後の main)で、269 回の起動のうち 18 回(6.7%)が 60 秒で engine-idle に届かなかった(state running のまま)。
- 実測: 止まりは DPR に従わなかった。viewport を変えた固定 DPR 2.5 と 3 は、0/45 だった。DPR 1 の変えない起動は 4/73、乱数の DPR(viewport 変更)は 8/62 だった。受け入れの「DPR 2.5 で 4/5 止まる」とは、この runner では一致しない(推定: 止まりは時間の競合で、DPR は競合の確率を上げる要因かもしれないが、この測定では確かめられない)。
- 実測: 止まった起動では、engine の Worker が busy(3 秒の CPU profile で busyMs 3,001・idleMs 0)。全体の 93% が 1 つの関数 `f_407e20`(engine の割り当てのロックの待ち)で、その次は `fle_loop_tick`・`f_409440`(割り当て)。Worker は CDP の `Runtime.evaluate` に答えない(走りっぱなしで、JS に戻らない)。ページの JS は答える(`state` は running・engine のログは 0 行)。
- 実測: 止まった時の呼び出しの鎖(profile の最も多い鎖)は、`f_407e20 < f_409440 < ... < fle_dispatch < ... < fle_hyb_dispatch < fle_call_guest < call_wndproc_fwd < call_wndproc < dispatch_message < fle_imp_DispatchMessageW`。つまり、待っている側は、ウィンドウ手続き(コールバック。切り替え点で切り替えを飛ばす深さ 2 以上)の中で、engine のロックを取ろうとして回っている。
- 機構(推定。上の実測と、`f_407e20` の機械語の読みから): 割り当ての領域は、ゲストの spin lock(`lock cmpxchg` の失敗で `pause` して、戻る。ロックを持つ間は `fle_loop_tick` の点を通る)で守られている。ロックを持つスレッドが、その領域の中の後ろ向きの分岐の切り替え点で、切り替えられる。待つ側は、深さ 2 以上のコールバックの中で回るので、`fle_loop_slice` は切り替えを飛ばす(音声の補充だけ)。持ち主は実行できず、ロックは解けず、回り続ける。仮想時計は、この回り続ける間は進まない(切り替えの点が 0 秒を進めない)。
- 未確認: 持ち主のスレッドの状態(準備完了か、待ち中か)と、持ち主が切り替えられた場所(どの関数の後ろ向きの分岐か)。これは、ビルドを 1 回増やして、持ち主の状態を止まる時に書き出す計測が要る(この Issue の作業時間内には入らなかった)。
- 直し方は、この測定では決めない(§4 に候補)。製品の変更は入れていない。
- 受け入れは通っていない。`fe-build bundle` が、この main で失敗する(§5 の原因。pack.args の OTT の wasm)。この runner の作業者では、受け入れの途中(bundle)で必ず落ちる。

## 1. 道具(`tools/boot-hang/measure.mjs`)

- 引数: `<bundle の絶対パス>`、`--dprs 1,2.5,3`、`--random LO,HI,COUNT`(繰り返し可。COUNT 点。各点は `--n` 回の起動で、起動のたびに乱数を引く)、`--n N`。
- 各起動: 新しい Chrome(新しいプロファイル)。`viewStart` が true の起動は、Page.navigate の前に viewport(2176×1092 CSS px、DPR を指定値)を変える。false の起動は、変えない(Chrome の既定。DPR 1)。true/false は起動の番号の偶奇で交互。
- 待つ: `window.__app.state` が engine-idle になるまで、上限 60 秒(WAIT_LIMIT_MS)。
- 止まった時(同じ時刻に取る): ページの state・error・engine のログの末尾 20 行(ページの JS が返らない時は null)。全ての Worker について、種類(`Module` の有無、`fleAudioWorkerState` の有無)、状態(`fleAudioWorkerState()` と `Module.fleThreadStats()`)、CPU profile(3 秒。busy と idle の時間、上位 5 関数、最も多い呼び出しの鎖 3 本。60 フレーム)。
- 出力: `{"runs":[{dpr, randomDprs?, viewStart, ok, firstIdleMs, stalled, busy, topFunctions, detail?}], "hangRate":{total, hung, rate}}`(1 行の JSON)。`dpr` は実際に適用された値(viewStart が false の起動は 1)、`randomDprs` は引いた値。

## 2. 止まりの率(実測。HEAD の engine。runner の値)

| 条件 | 起動 | 止まり | 備考 |
| --- | --- | --- | --- |
| 固定 DPR 1、viewport を変えない | 73 | 4 | |
| 固定 DPR 1、viewport を変える | 28 | 0 | |
| 固定 DPR 2.5、viewport を変える | 29 | 0 | |
| 固定 DPR 3、viewport を変える | 16 | 0 | |
| 乱数、viewport を変えない(DPR は 1) | 61 | 6 | 乱数の値は無効(変えないため) |
| 乱数、viewport を変える | 62 | 8 | 止まった値: 1.22, 1.54, 1.62, 1.68, 2.26, 2.36, 2.67, 2.73 |
| **計** | **269** | **18** | **6.7%** |

- 起動の時間(engine-idle まで。成功した起動): DPR 1 の中央値 7.8 秒、DPR 2.5 の中央値 10.1 秒、DPR 3 の中央値 12.5 秒(17 回。12.2〜12.7 秒)。成功した 251 回の全体の 95 パーセンタイルは 12,483 ms(受け入れの上限 12,500 ms の内。ただし DPR 3 は上限の近く)。DPR が高いほど起動は遅い(実測)。
- 測定の回数: N=10 の受け入れの構成(70 回)を 3 回(古い道具の 1 回と、新しい道具の 2 回)、20 回と 30 回の追加。固定 DPR 2.5 と 3 の viewport 変更の起動は 45 回で 0 回の止まり。止まりの率が 6.7% なら 45 回で 0 回となる確率は約 5%(推定)。強い否定ではない。

## 3. 止まった時の状態(実測)

止まった 18 回のうち、新しい道具で Worker の profile を取れた 12 回の全てで、次のとおり(runner の値):

| 順 | 関数 | 自己時間 (ms, 3 秒) | busy の中の割合 |
| --- | --- | --- | --- |
| 1 | `f_407e20`(割り当てのロックの取得。`lock cmpxchg` + `pause` の spin) | 2,774〜2,798 | 92.4〜93.2% |
| 2 | `fle_loop_tick` | 82〜98 | 2.7〜3.3% |
| 3 | `f_409440`(割り当て) | 74〜93 | 2.5〜3.0% |
| 4 | `audio_sync` | 14〜22 | 0.5〜0.7% |
| 5 | `wasm-to-js` | 9〜14 | 0.3〜0.5% |

- busy: true(12/12)。engine の Worker の CDP 応答: `Runtime.evaluate` は 5 秒で timeout(12/12)。ページの state は running、engine のログは 0 行(止まった全て)。
- 最も多い呼び出しの鎖(1 回の例。60 フレームの一部):
  `f_407e20 < f_409440 < f_60c750 < f_60c3e0 < f_60a490 < f_114f520 < fle_dispatch < fle_dispatch < f_862500 < ... < fle_hyb_dispatch < fle_call_guest < call_wndproc_fwd < call_wndproc < dispatch_message < fle_imp_DispatchMessageW < f_4249a0`
  (`fle_call_guest` は `fle_callback_depth` を増やす。`fle_imp_DispatchMessageW` の中のウィンドウ手続き = 深さ 2 以上。`fle_loop_tick < f_407e20` の鎖も、同じ深さの鎖の下に在る)。
- 本数: 最も多い鎖 8,947/9,658 サンプル(93%)。`fle_loop_tick` を直接に持つ鎖は 266 サンプル(2.8%)。
- もう一方の Worker(render-worker)は、ずっと `(idle)`。

## 4. 原因(機構)と、直し方の候補

### 4.1 実測と推定の分け

- 実測: 止まりは、engine の Worker の、割り当ての spin lock `f_407e20` の回り続け。それを呼ぶ鎖は、ウィンドウ手続き(深さ 2)の中。Worker の JS は戻らない。
- 実測(機械語の読み。`lift/out-app/gen/src/fle-00.c` の `f_407e20`): `lock cmpxchg` が失敗すると、`pause` の後で、[0x15e6a35] が 0 でなければ、切り替えの点(`fle_loop_tick`)を通って、`cmpxchg` に戻る。0 のときだけ、`f_4073e0` を呼んでから戻る(`f_4073e0` の中身は未読)。つまり、ロックの持ち主が実行されない限り、この spin は抜けない。
- 実測: 持ち主の関数は、`f_407e20` の臨界域の中の `fle_loop_tick`(L_407f84・L_407f8e・L_407fa0・L_407faa)と、臨界域から呼ぶ `f_409c80`(10 個の切り替え点を持つ)。持ち主は、これらの点で切り替えられうる。
- 推定(強い): 持ち主は、臨界域の後ろ向きの分岐で切り替えられ、準備完了の状態で待つ。ウィンドウ手続き(深さ 2)のスレッドが同じロックを取ろうとして、`fle_loop_slice` が切り替えを飛ばし、持ち主へ戻れない。仮想時計は進まない(スレッドは走り続けるため、待ち時間の締切は来ない)。
- 推定(未確認): 持ち主の状態が準備完了か、待ち中か。準備完了なら、深さ 1 の切り替えで戻れるはずなので、深さ 2 の切り替えの禁止が直接の原因と言える。待ち中なら、待ちの締切(仮想時計)が進まないことが原因。どちらも、止まりの瞬間の `fle_thread` の状態を書き出すと決まる。

### 4.2 #1352 の段 A との関係

- 段 A の切り替え点は、この臨界域の中に入った(f_407e20 の tick の数は 8、f_409c80 は 10)。段 A より前は、ロックの持ち主は間接呼び出しの点でしか切り替えられず、臨界域を跨ぐことは稀だった。段 A の切り替え点が、この止まりを増やした、と推定する。
- 段 A の効果(起動直後の途切れ)は、切り替えを残す限り保つ必要がある。

### 4.3 直し方の候補(未実装。どれも未確認)

1. 割り当ての spin の戻りの点で、深さに関わらず「明示的な譲り」(SwitchToThread 相当)をする。`pause` を含む後ろ向きの分岐(spin)だけを、譲りの点にする。理由: 明示的な待ち・譲り API(Sleep(0)・SwitchToThread)は、深さ 2 以上でも `schedule` に入っている(`fle_thread_sleep`)。spin の点を同じ扱いにすれば、持ち主へ戻れる。危険: 深さ 2 の runtime の枠が状態を持つ(決定 0147 の理由)。実装は fe-lift の後ろ向きの分岐の出し方(`succ`)に、spin の判定を入れる形。
2. 臨界域で切り替えない: ロックを取る命令(`lock` 前置の cmpxchg)と、その後に呼ぶ関数に、切り替えの点を置かない。不完全(`f_409c80` の点のように、臨界域から呼ぶ関数が残る)。
3. 持ち主が準備完了なら、深さ 2 の spin で、持ち主へ切り替える(深さ制限の緩和)。危険が大きい(決定 0147 の「深さ 2 以上で切り替えない」の目的に反する)。

推奨: 候補 1 を、次の順で確かめる: (a) 止まりの瞬間の持ち主の状態を書き出す計測を入れて、持ち主の準備完了を確かめる。(b) 候補 1 を fe-lift に入れて、止まりの率を 0 にできるか、道具で N=10 を 2 回(70 起動 × 2)で見る。(c) 段 A の `boot` の基準(`bootLast60sAt10s` ≤ 30)を確かめる。

## 5. 受け入れのブロッカー(この main の `fe-build bundle`)

- `fe-build bundle` は、`lift/harness/web/pack.args` の `--lazy-file lift/out-app/plugins/ott/vst-ext.wasm=plugins/ott.wasm` で止まる: `--lazy-file lift/out-app/plugins/ott/vst-ext.wasm: no such file`(`app/scripts/pack-assets.mjs` の検査)。
- この行は、0688430b(11:42、#1337 の途中)で入った。そのファイルは、`lift/harness/web/mt1/plugin-worker/build.sh ott` が作る(`MT1_OTT_DIR` の OTT の資料が要る)。受け入れの鎖(`agent.yml`・`deploy.yml`)には、その build が無い。
- 影響: この Issue の受け入れは、bundle の段で必ず落ちる。`deploy.yml`(main への push)も、同じ理由で落ちる(推定: 0688430b 以後の main への push の deploy は落ちている。ログで確かめていない)。
- 測定のための回避(この作業だけ): `lift/out-app/plugins/ott/vst-ext.wasm` を空のファイルとして置いた(`lift/out-app/` は git の無視の対象。製品の変更ではない)。この回避で `fe-build bundle` は通った(exit 0)。製品の起動は、このファイルに依らない(起動の後の取得)。
- 直し方の選択(PM の判断): (a) OTT の行を、無くても bundle が作れるように任意にする(`pack-assets.mjs` の検査)、(b) 受け入れと deploy の鎖に `build.sh ott` を入れる(OTT の資料を取る手順が要る)、(c) 行を一時的に外す(#1337 の取得が消える)。

## 6. 受け入れの各段の値(HEAD。空の OTT の回避の上。runner の値)

- `boot-bundle.mjs`(1 回): `error` null、`state` engine-idle、`standIn` false、`elapsedMs` 8,088。受け入れの上限 12,500 の内。
- 流れ A〜D(`welcome-flow.mjs`): 4 行とも `error` null、`state` engine-idle、`standIn` false、`trapped` false、`hostErrors` 0、`opened` true、`welcomeClosed` true(B と D も)。`elapsedMs` 7,810〜7,999、`preBootBytes` 311,111,837(上限 314,368,711 の内)、`startStalls` 0。
- `hang.json` の 35 回の受け入れの構成(DPR 1・2.5・3 と乱数 4 点、各 5 回)は、この測定では実行していない(N=10 の 70 回を 3 回、の測定で代えた。§2)。受け入れの基準(止まり 0)は、HEAD では満たさない(§2: 6.7%)。
- `measure.mjs --boot` の 3 回、`npm test --prefix app`: 実行していない(時間の制限。bundle の段が落ちるため、受け入れは先に進まない)。

## 7. 未確認・限界

- 未確認: 持ち主の状態(§4.1)。持ち主の切り替えられた関数(臨界域の中の tick が、実際にその時の持ち主か)。
- 未確認: 候補 1 の効果と、段 A の効果の保持(boot の基準)。
- 未確認: (ii) の実験(`fle_loop_slice` を何もしない版)。ビルドを 1 回増やす余裕が無かった。
- 限界: 止まりの率 6.7%(18/269)。DPR ごとの率は、固定 DPR の回数が少ない(2.5 と 3 は各 16〜29 回)。DPR 依存の有無は、この runner では言えない。
- 限界: 受け入れの `hang.json` の 35 回のうち、止まりが 0 になる確かさは、この率では低い(0.933^35 ≈ 0.08)。

## 8. 変えたパス

- `tools/boot-hang/measure.mjs`(新規。計測の道具。製品ではない)。
- `pm-work/findings/boot-hang-1393.md`(この文書)。
- 製品(`app/`・`lift/`・`ci/`)は変えていない。main の deploy は動かない。
- 一時の計測の出力(`.tmp-*`)は、作業の終わりに消す。`lift/out-app/` のビルドの出力は git の無視の対象。

## 9. 直しの試行(2 回目の作業者。ビルド 2 回。受け入れは差し戻し)

数字は runner の値(nproc 4、loadavg 1.4〜2.3)。この機械とは別枠。「実測」は道具の出力、「推定」は読みと推論。

### 9.1 直しの内容(製品)

- fe-lift(`lift/crates/fe-lift/src/lib.rs` の `is_spin_loop` と `succ`): 後ろ向きの分岐の範囲(目標から分岐元まで、番地順)に `pause` があれば `fle_spin_tick(); goto L_…` を出す。他は従来の `fle_loop_tick()`。
- runtime(`lift/runtime/fle_dispatch.h`・`fle_thread.c`・`fle_thread.h`):
  - `fle_spin_tick()` は `fle_loop_count` を数え、`fle_loop_limit` に届くと `fle_spin_slice()` を呼ぶ。
  - 深さ 1 以下(窓の手続きの外。`FLE_PREEMPT_MAX_DEPTH`)は、従来の `fle_loop_slice` と同じに準備完了のスレッドへ譲る。`fle_thread_set_priorities` は製品では呼ばれない(既定で無効)ので、`schedule()` と `schedule_ex(1)` は同じに動く。
  - 深さ 2 以上(窓の手続きの中)は、`FLE_SPIN_DEEP_BACKEDGES`(1,048,576)回の後ろ向きの分岐の後だけ譲る。それより短い spin は、従来どおり譲らない。
  - `fle_preempt_stats` に `spins`・`spins_deep` を足した(読むのは g3 の harness だけ)。
- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数は足していない。DPR の値を使う分岐は無い。

### 9.2 生成コードの確認(実測)

- 現在の main の生成コードで、`f_407e20`(割り当ての spin lock)の構造: `L_407ebe` の `cmpxchg` が成功すると `L_407ec2`(`fle_loop_tick`。取得後の後ろ向き)。失敗すると `L_407ec4` の `pause`、`L_407ec6` で `[0x15e6a35]` を読み、0 でなければ `L_407ed0` の `fle_spin_tick()` で `L_407eb9` に戻る(止まりの経路)。0 のときは `f_4073e0` を呼ぶ(`L_407eef`・`L_407f09` も `fle_spin_tick`)。
- つまり、止まりの経路の戻りは全て `fle_spin_tick` になっている。
- 番地は前回の一覧(`L_407f84` など)と違う。main が変わったため(前回の一覧は古いビルドのもの)。
- `fle_spin_tick` の出力は、生成コード全体で 46 行(`lift/out-app/gen/src/`)。

### 9.3 受け入れの結果(2 回)

| 項目 | ビルド 1(深さ 2 以上で、spin ごとに譲る) | ビルド 2(深さ 2 以上で、長い spin だけ譲る。現在の版) |
| --- | --- | --- |
| hang(99 起動。DPR 1・2.5・3 と乱数 8 点、各 9 回。DPR 1 の変えない起動 49) | 止まり 0/99。`ok` 99/99。p95 16,399 ms(min 10,319、max 16,478) | 止まり 0/99。`ok` 99/99。p95 16,273 ms(min 10,370、median 10,756、max 16,564) |
| 判定の hang(p95 ≤ 12,500) | **不合格** | **不合格** |
| boot.json: `error`・`state`・`standIn`・`elapsedMs` | null・engine-idle・false・12,450 | null・engine-idle・false・11,172 |
| 流れ A〜D: `elapsedMs`・`preBootBytes` | 10,653〜10,822・311,112,684 | 10,494〜10,649・311,112,755 |
| 流れ A〜D の他の項目 | 4 行とも合格(`trapped` false、`hostErrors` 0、`opened` true、B と D は `welcomeClosed` true) | 同左 |
| boot×3: `bootLast60sAt10s`(≤ 30) | 14・25・15(合格) | 14・**46**・14(**不合格**) |
| boot×3: `bootLast60sAt70s`・`bootLast60sAt90s` | 0・0(×3) | 0・0(×3) |
| boot×3: `quantaDelta`(≥ 19,000) | 20,675〜20,676 | 20,672〜20,676 |
| `npm test --prefix app` | 1,374 passed | 1,374 passed |

- 実測: 止まりは 198 起動で 0 件(ビルド 1 と 2 の計)。前回の実測(HEAD。269 起動で 18 件、6.7%)と比べると、同じ率なら 0 件となる確かさは小さい(0.933^198 ≈ 1.5×10^-6)。ただし前回の実測は古い main で、今の main での止まりの率は測っていない(下の 9.5)。
- 実測: ビルド 1 と 2 の差は、深さ 2 以上の spin の譲りの間隔だけ。p95 は 16,399 → 16,273 ms で、ほぼ同じ。
- 推定: 深さ 2 以上の譲りは、起動の遅さの原因ではない。深さ 2 以上で早く譲る版(ビルド 1)と、長い spin だけ譲る版(ビルド 2)の差が p95 に出ていない。
- 未確認: `bootLast60sAt10s` の 46(ビルド 2 の 2 回目)が、直しによる悪化か、runner の揺れか。同じビルドの 1 回目と 3 回目は 14 で、揺れの範囲の可能性がある。回数が少ないので確かめられない。

### 9.4 起動の遅さ(p95 16 秒 対 上限 12.5 秒)

- 前回の実測(p95 12,483 ms。DPR 1 の中央値 7.8 秒)は、古い engine の時点。その後 main に、engine の変更が多く入った(#1362・#1365・#1366・#1367・#1368・#1376 など。`git log eb118ef2..HEAD -- lift/runtime lift/crates/fe-lift/src lift/harness/web app/src/engine`)。
- 推定: 遅さは main の変更か runner の性能。この直しの経路は、通常の起動では働かない(深さ 1 は従来と同じ。深さ 2 以上の止まりは、譲らなければ必ず止まるので、通常の起動では起きない)。
- 未確認: 直しを入れない HEAD の、この runner での起動の時間(基準)。ビルドを増やす余裕がなかった。

### 9.5 止まった時の記録(計画の 1 番)の未実施と理由

- 未実施: 持ち主の状態(準備完了か待ちか)の記録。ビルドを 2 回使い、受け入れ優先にしたため。
- 分かったこと: エンジンの stdout・stderr は、Worker の中で 100 ms のタイマー(`app/src/engine/worker.ts` の `pushLog` と `flushLogs`)で束ねて送る。止まった Worker は、イベントの処理に戻らないので、タイマーが動かず、ログが 1 行も届かない。前回の「ログ 0 行」は、この仕組みによる(エンジンが何も出していないのではない)。止まった時の記録には、Worker のイベント処理を待たない経路が要る(runtime から直接 `postMessage` する、またはページ側が読む共有メモリ)。
- `fle_preempt_stats` の `spins`・`spins_deep` は、止まった Worker からは読めない(`Module.fleThreadStats` は Worker の JS が答える時だけ)。

### 9.6 未実施(時間の制限)

- 実験(ii)(`fle_loop_slice` を何もしない版)。
- 直しを入れない HEAD の `measure.mjs`(基準。今の main での止まりの率と起動の時間)。
- 持ち主の状態の確認(9.5)。

### 9.7 変えたパス

- `lift/crates/fe-lift/src/lib.rs`・`lift/runtime/fle_dispatch.h`・`lift/runtime/fle_thread.c`・`lift/runtime/fle_thread.h`(`lift/` を変えたので、push すると main の deploy が動く。受け入れは不合格のため、判定は push しない)。
- `tools/boot-hang/measure.mjs`(前回の差分。この作業では変えていない)。
- `pm-work/findings/boot-hang-1393.md`(この節)。
- `app/` は変えていない(`npm run build` と `npm test` のみ)。

### 9.8 次にやること

1. 基準を 1 回測る(ビルドは 1 回): 直しを入れない HEAD で、`tools/boot-hang/measure.mjs` を今の受け入れと同じ条件(N=9、DPR 1・2.5・3 と乱数 8 点)で。止まりの率と p95 を、今の main で測る。p95 が 16 秒前後なら、起動の遅さは直しではなく main の問題で、別の Issue にする。
2. 止まった時の記録: 9.5 の経路で、持ち主の状態を読めるようにする(計測のための製品の変更は、既定の動作を変えない形で)。
3. `bootLast60sAt10s` の揺れ: 同じビルドで 5 回以上測り、直しの前後で比べる。
