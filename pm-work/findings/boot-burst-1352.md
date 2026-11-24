# 起動直後の途切れの内訳と、切り替え点の第 1 手(#1352 段 A)

数字は runner の値(Chrome 157、nproc 4、loadavg 0.4〜1.7)。この機械とは別枠。「実測」は道具の出力、「推定」は読みと推論。耳の確認は行わない。途切れは underrun の数で書く。

## 0. 結論

- 基準(HEAD `abadafd1`。n=3): `bootLast60sAt10s` は 2,071・2,105・2,003(中央値 2,071。70% は 1,449 以下)。
- 第 1 手(後ろ向きの goto の前に切り替え点。下の 5)の後(n=3): `bootLast60sAt10s` は **4・4・25**。中央値の 70% を大きく下回る。`bootLast60sAt70s` は 6 回とも 0(基準 3 回とも 0。プロファイル付きの基準 1 回だけ 55)。
- 停止の主因の関数(`f_344fcf0`・`f_27ad5f8`)は、wasm に載っていない C(fe-lift の出力)。切り替え点を wasm の生成器に入れても届かない。C の後ろ向きの分岐に入れたことで届いた。
- 切り替えを通らない最長の区間は、基準 1,102 ms(0〜90 s の窓)→ 変更後 421 ms。残りは、C 以外(wasm の C++ 描画の `bl::Pipeline`)を含む区間。
- 段 B の目標(`bootLast60sAt10s`・`bootLast60sAt70s` が 0、`maxRefillStallMs` ≤ 60)は未達のまま。`maxRefillStallMs` は 540〜550 ms(起動からの最大の間隔。窓の値ではない)。

## 1. 基準と変更後(受け入れの窓。`measure.mjs --boot`。n=3)

基準は HEAD `abadafd1ba0b947e37cd8bf84e351d41f76f8a1c` を、この段の 3 回目のビルドで取り直した値(`pm-work/findings/boot-baseline-112.json`)。

| 版 | 回 | bootLast60sAt10s | bootLast60sAt70s | bootLast60sAt90s | maxRefillStallMs (ms) | longestRunQuanta | quantaDelta | screenDevicePx |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 基準 | 1 | 2,071 | 0 | 0 | 592.7 | 117 | 20,671 | 3240x1864 |
| 基準 | 2 | 2,105 | 0 | 0 | 592.1 | 124 | 20,671 | 3240x1864 |
| 基準 | 3 | 2,003 | 0 | 0 | 603.5 | 118 | 20,672 | 3240x1864 |
| 変更後 | 1 | 4 | 0 | 0 | 539.9 | 4 | 20,675 | 3240x1864 |
| 変更後 | 2 | 4 | 0 | 0 | 547.3 | 4 | 20,672 | 3240x1864 |
| 変更後 | 3 | 25 | 0 | 0 | 549.8 | 25 | 20,671 | 3240x1864 |

- 全 run: `error` null、`state` engine-idle、`standIn` false、`ctxState` running。`quantaDelta` は 19,000 以上。画面は `Math.round(1836 × dpr)` × `Math.round(1056 × dpr)` = 3240x1864 と一致。
- 参考(基準の前の回。同じ HEAD の別ビルド): `bootLast60sAt10s` 2,175・2,074・1,999、プロファイル付きの 1 回 2,105。ビルドを替えても基準の範囲は同じ。
- `maxRefillStallMs` は、エンジンの起動からの最大の公開の間隔(`fleAudioWorkerState.maxPubGapMs`)。窓の値ではない。基準・変更後とも 540〜600 ms。起動直後(押下の前)の間隔が含まれると推定。段 B の対象。
- 連続 underrun の最長(`longestRunQuanta`): 基準 117・124・118 → 変更後 4・4・25。

## 2. 特定した関数(a): 上位 20 関数(基準。0〜90 s の CPU profile の Worker)

「切り替え点を通らずに続いた最長」は、その関数がスタックに在り、かつ切り替え点(`fle_progress_slice` の枠。間接呼び出しの 4,096 回ごと)を含まない、連続した busy の最長。サンプル間隔は 318 µs(中央値 0.315 ms)。

スレッドは、CPU profile の根(`(root)`)の直下の枠で分けた。`fle_wasm_thread_entry` は CreateThread で作った guest のスレッドの入口。`f_1375990` は最初の guest のスレッドの入口(推定。ウィンドウ手続きを含むメッセージループの鎖が通る)。`Be.D.onmessage` は Worker の JS の入口。

| 順 | 関数名 | スレッド | 自己時間 (ms) | 切り替え点を通らずに続いた最長 (ms) | wasm か C か |
| --- | --- | --- | --- | --- | --- |
| 1 | f_344fcf0 | fle_wasm_thread_entry（生成したスレッド） | 2977.4 | 56.3 | C（fe-lift の出力） |
| 2 | f_27ad5f8 | fle_dispatch | 1341.6 | 147.8 | C（fe-lift の出力） |
| 3 | fle_mx_addsd | fle_wasm_thread_entry（生成したスレッド） | 828.8 | 2.2 | C（ランタイム） |
| 4 | fle_mx_addss | f_27ad5b3 | 469.9 | 4.3 | C（ランタイム） |
| 5 | fle_mx_subsd | fle_wasm_thread_entry（生成したスレッド） | 414.4 | 0.9 | C（ランタイム） |
| 6 | fle_mx_mulsd | fle_wasm_thread_entry（生成したスレッド） | 340.1 | 0.9 | C（ランタイム） |
| 7 | f_33d6e70 | fle_wasm_thread_entry（生成したスレッド） | 330.8 | 28.2 | C（fe-lift の出力） |
| 8 | fle_mx_subss | f_27ad5b3 | 268.8 | 3.4 | C（ランタイム） |
| 9 | fle_mx_mulss | f_27ad5b3 | 267.3 | 4.2 | C（ランタイム） |
| 10 | fle_lg_psrldq | f_27ad5b3 | 245.2 | 3.2 | C（ランタイム） |
| 11 | (program) | (program) | 220.8 | 3.4 | -（ブラウザの内部） |
| 12 | f_27acd60 | fle_wasm_thread_entry（生成したスレッド） | 217.5 | 30.3 | C（fe-lift の出力） |
| 13 | fle_r_blit | f_1375990 | 208.1 | 2.8 | C（ランタイム） |
| 14 | f_863b50 | f_1375990 | 181.3 | 1.2 | wasm |
| 15 | fle_hyb_dispatch | fle_wasm_thread_entry（生成したスレッド） | 152.2 | 220.2 | C（ランタイム） |
| 16 | fle_mx_cvttss2si32 | f_27ad5b3 | 145.4 | 3.0 | C（ランタイム） |
| 17 | poll_waiters | fle_wasm_thread_entry（生成したスレッド） | 139.9 | 2.5 | C（ランタイム） |
| 18 | f_27ad5d6 | f_27ad5f8 | 130.8 | 147.8 | C（fe-lift の出力） |
| 19 | fle_mx_addpd | fle_wasm_thread_entry（生成したスレッド） | 109.9 | 4.3 | C（ランタイム） |
| 20 | fle_desk_scene | Be.D.onmessage（Worker の JS 入口） | 107.8 | 10.0 | C（ランタイム） |

- 実測: 上位 2 関数(`f_344fcf0`・`f_27ad5f8`)で busy の約 35%。`f_344fcf0` と `f_27ad5f8` は fe-lift の C 出力で、`wasm-func-names.tsv`(`lift/out-app/hyb/*.tsv`)の状態は「not selected for the new backend (hybrid)」(wasm に載せていない)か、「RSP at a return or tail jump is not its entry value」(wasm に載せられない理由)。
- 実測: `f_344fcf0` は間接呼び出しを持たない(切り替えの計数が進まない)。SSE の演算(`fle_mx_*`)の呼び出しを 1 反復に多く持つ。
- 推定: `f_344fcf0` の 1 反復は約 50 µs(後述の粒度 1,024 回で、連続 53 ms になるため)。

## 3. 切り替え点を通らずに続いた最長の区間(b)(基準。0〜90 s)

全体の busy は 12,198 ms、idle は 77,300 ms、切り替え点の枠の時間は 1.9 ms(基準のプロファイル 1 回)。窓の残りは、区間の列の上位 10 件。

| 順 | 開始 (ms) | 長さ (ms) | スレッド(根の直下・時間 ms) | 鎖(根から) | 主な葉(ms) |
| --- | --- | --- | --- | --- | --- |
| 1 | 1,447 | 1,102.2 | fle_wasm_thread_entry 542、f_1375990 258、f_27ad5b3 137 | (root) > Be.D.onmessage > ot > X > scene > js-to-wasm:: > fle_desk_scene | f_344fcf0 302、f_27ad5f8 125、fle_mx_addsd 91 |
| 2 | 8,714 | 1,054.9 | fle_wasm_thread_entry 612、fle_dispatch 205、f_27ad5f8 197 | (root) > Be.D.onmessage > ot > X > scene > js-to-wasm:: > fle_desk_scene | f_344fcf0 306、f_27ad5f8 208、fle_mx_addsd 77 |
| 3 | 5,694 | 973.6 | fle_wasm_thread_entry 486、fle_dispatch 200、f_27ad5f8 143 | (root) > fle_wasm_thread_entry > f_412ae0 > fle_dispatch > f_5251b0 > fle_dispatch | f_344fcf0 245、f_27ad5f8 205、fle_mx_addsd 76 |
| 4 | 2,551 | 912.2 | fle_wasm_thread_entry 551、f_27ad590 75、f_1375990 73 | (root) > fle_wasm_thread_entry > f_412ae0 > fle_dispatch > f_5251b0 > fle_dispatch | f_344fcf0 295、f_27ad5f8 131、fle_mx_addsd 75 |
| 5 | 0 | 906.2 | fle_wasm_thread_entry 576、f_27ad5b3 133、fle_dispatch 116 | (root) > (program) | f_344fcf0 322、f_27ad5f8 113、fle_mx_addsd 89 |
| 6 | 3,937 | 890.7 | fle_wasm_thread_entry 657、f_1375990 78、fle_dispatch 67 | (root) > Be.D.onmessage > ot > X > scene > js-to-wasm:: > fle_desk_scene | f_344fcf0 264、fle_mx_addsd 80、f_27ad5f8 72 |
| 7 | 8,003 | 710.4 | fle_wasm_thread_entry 546、fle_dispatch 82、f_27ad527 56 | (root) > f_1375990 > f_1375950 > f_86a660 > wasm-function[19576] > f_86a250 | f_344fcf0 214、f_27ad5f8 77、fle_mx_addsd 63 |
| 8 | 6,667 | 612.0 | fle_wasm_thread_entry 429、f_27ad590 87、f_27ad5b3 77 | (root) > f_27ad5f8 > fle_dispatch > f_27ad590 > f_27ad5b3 > f_27ad5d6 | f_344fcf0 237、f_27ad5f8 81、fle_mx_addsd 66 |
| 9 | 7,280 | 484.4 | fle_wasm_thread_entry 231、f_27ad590 72、f_27ad5b3 64 | (root) > Be.D.onmessage > ot > X > scene > js-to-wasm:: > fle_desk_scene | f_344fcf0 107、f_27ad5f8 106、fle_mx_addsd 35 |
| 10 | 3,464 | 472.1 | fle_wasm_thread_entry 414、f_1375990 51、Be.D.onmessage 5 | (root) > Be.D.onmessage > ot > X > scene > js-to-wasm:: > fle_desk_scene | f_344fcf0 144、f_27acd60 62、fle_mx_addsd 35 |

- 実測: 上位の区間は、ほぼ全部が 0〜10 s の内(起動直後)。10〜70 s の窓の最長は 4.2 ms(基準)。
- 実測: 区間の多くは `fle_wasm_thread_entry`(生成したスレッド)の下で、`f_344fcf0`・`f_27ad5f8`・`fle_mx_addsd` を含む。
- 推定: 1 回目のビルド(同じ HEAD)のプロファイルでは、`f_1375990` の鎖が `fle_imp_DispatchMessageW` を通る区間があった。ウィンドウ手続きの中(深さ 2 以上)で切り替えが飛ばされる区間と推定。この段の表には含めていない。

## 4. 変更後の区間(切り替え点あり。0〜90 s。上位 5)

切り替え点は `fle_progress_slice` と `fle_loop_slice`(後述 5)。

| 順 | 開始 (ms) | 長さ (ms) | スレッド(根の直下・時間 ms) | 主な葉(ms) |
| --- | --- | --- | --- | --- |
| 1 | 1,891 | 421.0 | f_1375990 266、f_27ad590 53、f_27ad5b3 45 | bl::Pipeline::Reference（Blend2D の塗り。wasm の C++。切り替え点なし）123、f_27ad5f8 51、f_3370290 37 |
| 2 | 8,635 | 374.0 | fle_wasm_thread_entry 240、fle_dispatch 70、f_27ad5f8 54 | f_344fcf0 126、f_27ad5f8 74、fle_mx_addsd 33 |
| 3 | 5,375 | 369.7 | fle_wasm_thread_entry 240、fle_dispatch 49、f_1375990 28 | f_344fcf0 123、f_27ad5f8 46、fle_mx_addsd 37 |
| 4 | 7,364 | 342.0 | fle_wasm_thread_entry 211、f_27ad5b3 46、f_27ad590 45 | f_344fcf0 123、f_27ad5f8 55、fle_mx_addsd 27 |
| 5 | 3,410 | 329.0 | fle_wasm_thread_entry 262、f_1375990 54、(garbage collector) 5 | f_27acd60 72、f_344fcf0 47、f_33d6e70 28 |

関数ごとの最長(基準 → 変更後): `f_344fcf0` 56.3 → 53.4 ms、`f_27ad5f8` 147.8 → 93.5 ms、`f_27acd60` 30.3 → 34.1 ms、`f_33d6e70` 28.2 → 28.3 ms、`fle_hyb_dispatch` 220.2 → 198.3 ms。自己時間の合計は、0〜90 s で 12,198 → 11,901 ms(プロファイラ付きの 1 回ずつ。誤差の範囲)。

## 5. 切り替え点の場所・方式・理由

- 場所: fe-lift が出す C の後ろ向きの goto(分岐先の番地が分岐元の番地以下の分岐)。全ての関数。生成された C の例: `if (fle_cc(2)) { fle_loop_tick(); goto L_332f0e0; }`。後ろ向きの分岐は、エンジンのビルドの fe-lift の 11 回の実行の合計で 72,845 本(ビルドのログの `back_edges`。全 `goto` は 580,176 本)。jecxz・jrcxz と、switch の表(jumptab)も同じ規則。wasm の生成器(fe-wasmgen)は変えない。
- 方式: `fle_loop_tick()`(`lift/runtime/fle_dispatch.h`)は、ループ用の計数 `fle_loop_count` を進め、上限 `fle_loop_limit` に達したら `fle_loop_slice()` を呼ぶ。計数は間接呼び出しの `fle_progress_count` と別で、仮想時計は進めない。上限は 1,024 回ごと(`FLE_LOOP_SLICE_BACKEDGES`。`lift/runtime/fle_thread.h`)。最初の CreateThread から有効(それまでは上限が UINT64_MAX)。
- `fle_loop_slice()`(`lift/runtime/fle_thread.c`): 深さ 1(スレッドの最も外の guest の呼び出し)なら、準備完了のスレッドへ切り替える(`schedule_ex(1)`。その中で `poll_waiters` と音声の補充が走る)。深さ 2 以上(ウィンドウ手続きなどの中)なら切り替えは飛ばし、音声の補充だけ(`fle_audio_poll_in_callback`)。
- 理由(実測・読み):
  - 停止の主因は C の関数(上の 2。実測: 上位の関数は wasm に載っていない)。wasm 側の切り替え点は届かない。前回の wasm 側の試みが効かなかった理由と一致する。
  - 仮想時計を進めると、guest の時間が余計に進み、起動が長くなる(前回の試行で 139.7 s)。そのため、時計は進めない。
  - 深さ 2 以上で切り替えを飛ばすと補充も止まる(#1334 の読み)。補充だけは行う。
- 粒度: 1,024 回。`f_344fcf0` の 1 反復が約 50 µs(推定)で、連続 53 ms になった。音声の先読み(60 ms)に近い。段 B で粒度を細かくする候補(この段では変えない。ビルドの回数の上限のため)。

## 6. 再生中の性能(窓の +10〜+70 s。engine の Worker の busy ÷ 再生の周)

| 版 | busy (ms) | 再生の周 | 1 周あたり (µs) | 主スレッドの busy (ms) |
| --- | --- | --- | --- | --- |
| 基準 | 1,824.4 | 20,675 | 88.2 | 614.8 |
| 変更後 | 1,623.3 | 20,676 | 78.5 | 401.4 |

- 実測: プロファイラ付きの 1 回ずつ。悪化は見えない。プロファイラ無しの周あたりは未測定(道具に無い)。
- 実測: 窓の中の最長の切り替え無し区間は、基準 4.2 ms、変更後 3.4 ms(10〜70 s)。
- 推定: 後ろ向き分岐ごとの追加は、カウンタの加算と比較 1 回。1,024 回ごとの切り替えの検査(`poll_waiters`)は、窓の busy に対して小さい。

## 7. 取り方(c)

- 道具: `tools/p3-audio-probe/measure.mjs <bundle> --boot`(n=3。判定の値)、`--boot --profile FILE`(CPU profile。engine の Worker は FILE、ページの主スレッドは FILE の `.main` 付き。この段で追加)。`tools/p3-audio-probe/burst-report.mjs`(この段で追加: 関数の自己時間、関数ごとの切り替え無しの最長、切り替え無しの区間の上位、`--from-ms`・`--to-ms` の窓、wasm か C かの印)。
- 窓: profile の時刻 0 は engine-idle の付近(profile の開始は押下の前)。全体は 0〜90 s、再生中は 10〜70 s。
- 切り替え点の枠: `fle_progress_slice`(基準・変更後とも)、`fle_loop_slice`(変更後)。
- 「wasm か C か」: `lift/out-app/hyb/*.tsv`(fe-wasmgen hybrid の状態。`ok` は wasm、それ以外は C(fe-lift の出力))。C ランタイム(`fle_*`)は C。
- 基準の profile は HEAD のビルドで取った(この段の 3 回目のビルド。表の名前は同じビルドの `wasm-func-names.tsv`)。変更後の profile は変更後のビルドで取った。

## 8. 未確認・限界

- 未確認(推定): 0〜10 s の補充が回復した経路。(a) 切り替えの点で `poll_waiters` → `fle_audio_poll` → `audio_sync` が呼ばれ、補充される。(b) 音声を作る guest のスレッドが、準備完了になった点で走り、DirectSound の API(Lock・GetCurrentPosition など)から補充される。どちらが効いたかは分けていない。
- 未確認: 421 ms の区間(`bl::Pipeline`、Blend2D の塗り。wasm の C++。後ろ向き分岐の無い経路)の間に補充が止まらなかった理由。窓の underrun は 4〜25 quanta で、補充の停止は観測されなかった。トレース(`--trace`)は変更後のビルドで取っていない(ビルドを 3 回に抑えたため)。
- 限界: wasm 側(fe-wasmgen が直接に生成した関数)の後ろ向き分岐は対象外。wasm の C++ 版の描画(`bl::Pipeline`)も対象外。
- 限界: `maxRefillStallMs` は窓の値ではない(起動からの最大)。段 B の目標(60 以下)は未達(540〜550 ms)。
- 限界: n=3。プロファイルは 1 回ずつ。
- fe-lift のテスト: 基準(HEAD)と変更後で、失敗は同じ 63 件(失敗の名前の集合も同じ)。失敗の理由は環境に由来するように見える(python3 の補助ファイルの欠落、`archive/` の欠落、ホストとの比較表、既存の件数)。切り替えの追加に関係する名前は無い。前回の記録の 55 件とは数が違う(原因は未確認)。
- 耳の確認は行わない。「音が途切れない」とは書かない。

## 9. 変えたパス

- `lift/crates/fe-lift/src/lib.rs`(`succ`: 後ろ向きの goto の前に `fle_loop_tick();`。`Stats.back_edges`)、`jumptab.rs`(`succ` の引数)、`int_more2.rs`(jecxz・jrcxz)、`main.rs`(統計の 1 行)、`tests/msvc_switch.rs`(後ろ向きの switch の期待値の形を追加)。
- `lift/runtime/fle_dispatch.h`(`fle_loop_count`・`fle_loop_limit`・`fle_loop_slice`・`fle_loop_tick`)、`fle_thread.h`(`FLE_LOOP_SLICE_BACKEDGES`)、`fle_thread.c`(`fle_loop_slice`。`preempt_arm` で上限を設定)。
- `tools/p3-audio-probe/measure.mjs`(`--profile` で主スレッドも取る)、`tools/p3-audio-probe/burst-report.mjs`(新規)。
- `pm-work/findings/boot-baseline-112.json`、`pm-work/findings/boot-burst-1352.md`(この文書)。
- `app/` は変えていない。`lift/` を変えたので、main への push で `deploy.yml` が動く。
- 新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数は足していない(`FLE_LOOP_SLICE_BACKEDGES` は `FLE_PREEMPT_*` と同じ種類の定数)。
