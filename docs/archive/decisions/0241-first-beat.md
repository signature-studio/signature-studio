# 0241: 再生開始の 1 拍目が鳴らない — 原因は『出力リングへの先取り公開』と『FL の書き込みの遅れ』の相互作用。遅れた書き込みを、出力がまだ読んでいない間は反映する(S-ds-late-write、旗なし)

状態: **仮の決定**(作業者 aud1。作業の基準は HEAD **52034ca9** の木。`aud1.diff` と `aud1-page.diff` は HEAD **33349a3d** の `git archive` にオフセット 0・fuzz 0 で `patch -p1 --dry-run` が当たる。取り込み時に HEAD が進んでいたら `lift/runtime/fle_audio_ring.*`・`fle_dsound.*`・`lift/harness/audio/`・`wall_fr15.rs` が重なるか確かめる)。

> **先に読む**: (1) **耳での確認は 0 件**。以下の『鳴った/鳴らない』は **AudioWorklet が受け取った標本(PCM)の事実**(armCapture の 2.5 s、クリックの位置と峰値)で、ヘッドレスの音声デバイス無し(null)で取った。 (2) **依頼者の機械(Windows 10・Firefox 159・実デバイス)の値は、gpu1 が表にした 7 セッション分のビーコンの読みだけ**で、私が実機で取ったものは 0 件。**依頼者の『放置後に再発』は再現できなかった(旧実装で、Chrome 156 の放置 10/30/60 s 後 42 回は消失 0、Firefox 158 の 40 回は消失 0・欠け 1)**。 (3) 実エンジンは出荷の F2xH7(CI の段スクリプトで作った bundle。下の A・B)。**代役・再生・事前生成した波形は使っていない**(全てエンジンがその場で出した値)。 (4) この機械の load は 11〜36(他担当の負荷。表に併記)、`taskset` で Chrome 8-11・Firefox 4-7 に固定。入力は CDP / WebDriver Actions の合成入力(人の操作ではない)。

## 冒頭三点

1. **何を実測したか**: 出荷構成(F2xH7)の実エンジンを、パラメータ無しの既定の入口で隔離 Chrome 156.0.8078.47 と Firefox 158.0 に載せ、メトロノーム(Ctrl+M)を入れて Space で再生を始め、(a)起動後の最初の再生、(b)停止 0.7 s 後の再生、(c)停止 10/30/60 s 放置後の再生、で先頭 2.5 s の出力 PCM と、エンジン側の時刻表(DS バッファへの Unlock の時刻・領域の位置・出力の読み位置・公開済みの範囲)を取った。原因の見立て(下の 3・4)を確かめたうえで、**直し(F2)を CI の段スクリプトで作った 2 つの bundle(A = 素の HEAD の runtime、B = A + F2)で A/B 比較**した。
2. **結論**:
   - **経路の事実**: 音のデータは『FL(wasm、Worker)→ `fle_dsound.c` が DS バッファを混ぜる → EM_JS `fle_js_publish`(wasm→JS の同期呼び出し。1 回 128〜1024 フレーム、約 100 回/s。JS ループで wasm ヒープ → SharedArrayBuffer の Float32Array へコピー)→ AudioWorklet が SAB を読む(Atomics)』。**音のデータの経路に postMessage は無く、メインスレッドも通らない**。メインスレッドは AudioContext の生成・`resume()`・統計の受信だけ。**無音を検出しての自動 suspend/close は `app/src/audio` にも `lift/runtime` にも無い**(grep と、ログ: 起動時の Stop/Play 2 回の後はデバイスの状態が変わらない)。
   - **1 拍目が鳴らない原因**: FL は『再生カーソルが半分を越えたら反対側の半分(4096 フレーム)を書く』ので、通常は Unlock の時点で領域の先頭が再生カーソルの 83〜92 ms 先にある。一方エンジンは**再生カーソルの lead = 60 ms(S-audio-lead)先まで、その時点のバッファの中身を出力リングへ先に公開**し、公開済みのフレームは後の書き込みでは変わらなかった。Space の直後は 1 本の Worker で UI 側の仕事とミキサーが協調実行されるため、ミキサーの Unlock が 93 ms 間隔から 134〜175 ms に伸びる(測れた範囲)。領域の先頭までの距離が 60 ms を切る(34〜57 ms)と、**領域の頭の 171〜1152 フレームが古い内容(無音)で公開済み**になり、先頭 11.3 ms のアクセントのクリックが欠ける/消える。PM の最初の仮説『late_frames/skipped で捨てられる』は**当たっていない**(skipped = 0、gapFrames = 0。トランスポートの開始では DS の Play() は呼ばれない)。
   - **直し(F2)**: Unlock の後、公開済みで出力がまだ読んでいないフレームのうち、読み位置 + 256 フレーム(ガード)以降を、バッファの現在の内容から混ぜ直して上書きする。lead は変えない。**旗なし・無条件**(外部消費者がある時だけ。実機の DirectSound は『write cursor より手前は未定義』であり、我々は『出力が読む前なら反映する』を選ぶ)。**Chrome 156 の最初の再生で 1 拍目の欠け/消失 10/13 → 0/11**(P = 0.0001)。F2 で救われた周期の PCM は、遅れなかった周期とビット一致(sha256 同一)。Firefox 158 は前後とも 1/50 で差なし(この機械・この負荷では旧実装が壊れない)。
   - **F2 は契約の修正で、遅れそのものの除去ではない**。ミキサーの応答が遅れる原因(1 本の Worker での協調実行)は残る(Chrome の最初の再生の Unlock 間隔の最大値の中央値 A 133 ms、B 124 ms)。F2 で救えない遅れは残る(Space の処理だけでは説明できない大きさの Worker の 188/340 ms の停止で、出力が追い越した: B で 2 件。Space との関係は未特定)。
3. **未確認**: 耳での聴取。実機 Windows・WASAPI・Firefox 159。依頼者の『放置後に再発』(未再現)。実デバイスが無音で眠る/ストリームが止まる影響(D)。Chrome の最初の再生が Firefox より遅い原因(wasm の遅延コンパイルを切るテストでは変わらなかったが、フラグが効いたか未確認)。188/340 ms の Worker の停止の原因と Space との関係(協調実行/GC/tier-up のどれか)。F2 の早期脱出の最適化(未実装、測っていない)。

## 1 経路の事実(どこで wasm/JS・Worker/メインを越えるか)

| 段 | 場所 | 越境 | 頻度(実測) |
|---|---|---|---|
| FL が DS バッファへ書く(Lock/Unlock) | wasm のメモリ内 | なし | 4096 フレームごと(約 93 ms、約 10.8 回/s)。再生していない間も無音を書き続ける |
| DS バッファ → float の混合(`mix_chunk`) | wasm(C) | なし | 公開ごと |
| 混合 → 出力リング(C 側のミラー) | wasm 内のコピー | なし | 公開ごと |
| 出力リング → SAB | **EM_JS `fle_js_publish`: wasm → JS の同期呼び出し**。JS のループで `HEAPU8` から SAB 上の `Float32Array` へコピー | **wasm↔JS 1 回/公開**(同じ Worker のスレッド内) | 約 100 回/s、128〜1024 フレーム(多くは 384〜512) |
| SAB → 出力 | AudioWorklet(別スレッド)が `Atomics` と `Float32Array` で読む | **共有メモリの読みだけ**(コピーなしの共有、`postMessage` なし) | 128 フレームごと(約 345 回/s) |
| 読み位置 RD → エンジン | EM_JS `fle_js_consumed`(`Atomics.load`) | wasm→JS の同期呼び出し | 同期ごと |
| 統計 | worklet → メイン `postMessage` 50 ms ごと、Worker → メイン 1 Hz(`aud2-worker`)と 2 Hz(`audio-pulse`) | **データではなく統計だけ**がスレッドを越える | 20/s、1/s、2/s |
| AudioContext の生成・`resume()` | メインスレッド | ジェスチャーのたび | 無音の検出による suspend/close は **無い** |

『音のデータの経路で wasm/JS を越えるのは公開ごとの 1 回の呼び出しとコピーの 1 ループ。Worker/メインの越境は無い』が事実。コピーは JS のループ(`TypedArray.set` ではない)で、44.1 kHz × 2 ch = 88 k float/s。実測では、Unlock までの最後のスライス(コピーを含む)の平均が 6.5〜7.0 ms。変換・通信を減らす余地は、この経路にはほぼ無い(下の 5 の F2 の費用も同じ桁)。

## 2 再現(実エンジン、PCM で)

条件: a = 起動後の最初の再生(ウィザードを閉じて 20 s 待った後)、b = 停止 0.7 s 後の再生、c10/c30/c60 = 停止 10/30/60 s 放置後の再生。毎回 `armCapture`(timeline)で最初の非ゼロ標本から 2.5 s を取り、Space の keydown の瞬間の RD を記録。1 拍目 = アクセント(峰値 0.9406、11.3 ms)、2 拍目以降 = 峰値 0.6234・17 ms・429 ms 間隔(140 BPM)。正常な 1 拍目は keydown の 80〜190 ms 後。判定: 完全 = 1 拍目が 350 ms 以内に峰値 ≥ 0.8・11 ms 以上、欠け = 350 ms 以内だが切れている、消失 = 最初のクリックが 350 ms 以降(2 拍目)。

### 2.1 旧実装(F2 の前)の全走行(`tables/pre-f2-all-runs.tsv`、有効セッションのみ)

| ブラウザ | 条件 | 周期 | 1 拍目 完全 | 欠け | 消失 |
|---|---|---:|---:|---:|---:|
| Chrome 156 | **a(最初の再生)** | 21 | 5 | 8 | **8** |
| Chrome 156 | b(停止 0.7 s 後) | 42 | 41 | 0 | 1 |
| Chrome 156 | c10 / c30 / c60(放置後) | 21 / 17 / 4 | 21 / 17 / 4 | 0 | **0** |
| Firefox 158 | a | 21 | 21 | 0 | 0 |
| Firefox 158 | b | 42 | 42 | 0 | 0 |
| Firefox 158 | c10 / c30 / c60 | 20 / 15 / 5 | 20 / 14 / 5 | 0 / 1 / 0 | 0 |

(ship5 の F2xH7〔1d438166〕と、CI の段スクリプトで作り直した A〔HEAD の runtime〕の走行を合わせた。無音セッション〔キーがページに届かず、全周期が無音〕4 件は除外: j1-chrome-s1・s5、m1-chrome-s5、p1-a1-chrome-s7。原因は未調査で、旧・新どちらの bundle にも起きる。)

**Chrome の最初の再生では 21 回中 16 回、1 拍目が欠ける/消える**。依頼者の『最初の拍は確実に鳴らない』と整合する。『すぐ停止して再度再生すれば 1 拍目から鳴る』も整合する(b: 42 回中 1 回)。**『放置後に再発』は再現しない**(Chrome 42 回、Firefox 40 回で消失 0。この機械の headless・null デバイスでの話で、実デバイスの無音時のスリープ等は含まない)。Firefox 158 は 103 周期中、欠けが 1 回(c30)で消失 0 とこの機械では壊れないが、余裕は小さい(下)。

### 2.2 余裕(Unlock 時点で、領域の先頭が出力の何 ms 先か。60 ms を切ると古い内容が公開済み。旧実装の全走行、捕捉が取れた周期、`tables/margin-pre-f2.tsv`)

| ブラウザ | 条件 | 周期 | 最小 ms | 10 パーセンタイル | 中央値 | 60 ms 未満 | 70 ms 未満 |
|---|---|---:|---:|---:|---:|---:|---:|
| Chrome | **a** | 19 | **33.9** | 42.6 | **57.1** | **12** | 15 |
| Chrome | b | 42 | 62.9 | 74.5 | 83.2 | 0 | 2 |
| Chrome | c10・c30・c60 | 42 | 64.8 | 71.6 | 86.1 | 0 | 3 |
| Firefox | a | 21 | 62.9 | 71.6 | 74.5 | 0 | 2 |
| Firefox | b | 42 | 62.9 | 71.6 | 77.4 | 0 | 2 |
| Firefox | c10・c30・c60 | 40 | 57.1 | 71.6 | 80.3 | 1 | 3 |

**Firefox の 1 拍目のブロックは出力の 57〜83 ms 先(a の中央値 74.5 ms)で、閾値 60 ms との差は 3〜23 ms**。Chrome の最初の再生は中央値 57.1 ms で、半分以上が閾値の下。依頼者の機械は first idle が 11.2 s(この機械の約 1.5 倍)で、エンジンの遅れがこれより大きければ Firefox でも同じ機構で欠けうる(**読み**。実機での確認は 0 件)。

## 3 エンジン側の時刻表(実エンジン)

`tables/timeline-before-f2.tsv`(Space の keydown を 0 とする)。Chrome の最初の再生の消失(m1-chrome-s1):

| 事象 | keydown からの ms | 領域の先頭(keydown から ms) | Unlock 時点で出力の何 ms 先か | 領域のうち公開済みの古い部分(フレーム) | 前の Unlock との間隔 ms |
|---|---:|---:|---:|---:|---:|
| Unlock(再生前から 93 ms ごと) | -226.4 | -140.3 | 86.1 | 0 | |
| Unlock | -140.3 | -47.4 | 91.9 | 0 | 86.1 |
| Unlock | -44.0 | 45.5 | 89.0 | 0 | 96.4 |
| **Unlock(1 拍目を含むブロック)** | **99.4** | 138.4 | **42.6** | **768** | **143.4** |
| Unlock | 148.3 | 231.2 | 83.2 | 0 | 48.9 |
| Unlock | 233.3 | 324.1 | 89.0 | 0 | 84.9 |
| 最初に信号のある公開(2 拍目) | 511.6 | 562.1 | 51.3 | | |

- **トランスポートの開始でデバイスの状態は変わらない**: `fle_audio_ring_set_state` の記録は、起動時の OPEN→PLAYING→OPEN→PLAYING(Stop/Play 2 回)だけで、以後 PLAYING のまま(FL の `auto close audio` OFF と整合)。ミキサーは再生していない間も 93 ms ごとに無音のブロックを Unlock する。したがって**『Play() から最初のブロックの書き込みまで』という量は無く**、量るべきは『Space から、1 拍目を含むブロックの Unlock まで』と、その時の出力からの距離。
- 同じ走行で `skipped` = 0、`gapFrames` = 0(エンジンが捨てた/無音で埋めたフレームは 0)、worklet の `UR_STARTUP`・underrun も 0。欠けは**エンジンが数えるどの量にも出ない**(古い内容を公開済みだったため。0211 §5 の『第 3 の欠け』と同じ種類)。
- Space の後 8 個の Unlock の最大間隔(A bundle、`tables/p1-ab-summary.tsv`):

| ブラウザ | 条件 | 周期 | 中央値 ms | 最大 ms |
|---|---|---:|---:|---:|
| Chrome | **a** | 13 | **133.0** | **175.4** |
| Chrome | b | 26 | 102.1 | 121.1 |
| Chrome | c10 / c30 | 13 / 13 | 101.4 / 100.8 | 108.9 / 107.7 |
| Firefox | a / b | 10 / 20 | 107.8 / 103.0 | 122.9 / 110.0 |
| Firefox | c10 / c30 | 10 / 10 | 102.6 / 102.8 | 117.9 / 130.8 |

  93 ms が通常の間隔。Chrome の最初の再生だけが 133 ms(最大 175 ms)に伸びる。**メインスレッド(FL の UI スレッド)の CPU は、Space を含む約 500 ms で Chrome a 94.6 ms、b 80.7 ms、c10 59.5 ms、c30 72.6 ms、Firefox 98〜107 ms**(`audio-pulse` のスレッド別の累計の差〔`tables/thread-cpu-around-play.tsv`〕。スライス時間であり、待ちを含みうる)。Space の直後に新しいスレッド(thread 8)が現れ、約 1000 回/s の切替で 500 ms あたり 17〜28 ms を使う。
- Worker の『切替・待ちの呼び出し(`fle_js_now`)の間隔』で見ると、Space の後に**呼び出しの無い 45〜49 ms の連続**がある周期は Unlock が遅く(j1-chrome-s4 の a で 48.8 ms → 余裕 36.8 ms、s6 の a で 47.1 ms → 余裕 16.5 ms)、無い周期(26〜30 ms)は余裕 54〜66 ms。**ミキサーのスレッドが、UI 側のスレッドが譲るまで走れない**(協調実行)ことと整合する。どの関数が 45 ms かは、スライス別のログが無く特定していない。
- 冷えたコードの影響: Chrome の a だけが遅く、放置後(c10〜c60)は遅くない=『冷たさ』は**起動後の最初の再生**に限られる(V8 の遅延コンパイルを疑い、`--js-flags=--no-wasm-lazy-compilation` で試したが a は完全 3・欠け 1・消失 2 / 6 で変わらない。**フラグが効いたか未確認**: first idle の中央値が 10.4 s と通常の 10.6 s で同じ)。

## 4 原因の切り分け

| 候補 | 判定 | 根拠 |
|---|---|---|
| (A) エンジンの計算が遅く、最初のブロックが lead に間に合わない | **部分的に当たり**: ミキサーの 1 ブロックの CPU は mt3(決定 0234)の測定で F2xH7 の平均 58〜63 ms(周期の 63〜68%)で、通常は間に合う(Unlock の間隔 93 ms)。遅れるのは『Space の直後に UI 側の仕事が先に走ること』(1 本の Worker の協調実行)。Chrome の最初の再生で間隔が 93 → 130〜175 ms | 3 の表 |
| (B) DS の実装の規則が Learn とずれている | **当たり(契約の解釈の差)**: Learn は『play cursor と write cursor の間は、すでに再生のために積まれていて、安全に変えられない』と書く。我々は write cursor を play + 60 ms(S-audio-lead。Learn は『通常 約 15 ms』)にし、**その間の領域への後からの書き込みを失っていた**。実機の DirectSound は書き込みの時刻が遅くても『ハードウェアがその位置を読むまで』なら反映される(推測: 手前〔15 ms〕を超えない限り)。F2 はこの差を埋める | 2.2、5 |
| (C) worklet の state 遷移の順序 | **外れ**: デバイスは再生中のまま(状態の記録に遷移なし)。`UR_STARTUP` 0 | 3 |
| (D) ブラウザ/OS 側(出力が無音のときにデバイスが眠る) | **この機械では起きない**: ctx は running のまま、`outputLatency` 24〜32 ms、worklet の 50 ms 投稿の間隔と ctx 時間/実時間(Chrome 1.0002・Firefox 0.9998)は放置 10/30 s の後も変わらない(`tables/worklet-cadence.tsv`)。**null の出力で実デバイスのスリープは再現できない**=依頼者の『放置後に再発』の説明は**推測**(Windows のオーディオエンドポイント/ドライバ/Bluetooth が無音で眠る、Firefox の cubeb ストリーム)で、我々のコードに自動切断は無い | grep、ログ |
| (E) FL 自身の挙動(auto close audio OFF のとき、DS バッファを開いたままにするか) | **開いたまま**: 起動時の Stop/Play 2 回の後はデバイスの状態が変わらず、再生していない間も 93 ms ごとに無音を書く | 3 |

## 5 直し: F2 = S-ds-late-write(旗なし、無条件)

**契約の読み**: DirectSound の契約は『アプリがバッファに書いたデータは、デバイスがその位置を読む時点で再生される』。Learn は play cursor〜write cursor の間への書き込みを『安全に変えられない』とするだけで、**反映しないとは書いていない**。外部消費者(AudioWorklet のリング)では、エンジンが lead(60 ms)先まで『その時点のバッファ』を先に公開するため、その後の書き込みが出力にまだ読まれていなくても反映されなかった。これはランタイムが加えた凍結で、契約の実装ではない(方針の (a)『余計な処理の除去』に当たる: ランタイムが加えた凍結を外して DS の契約に近づける変更で、FL の挙動の補正ではない)。

**実装**(`lift/runtime`、旗なし):
- `fle_dsound.c` `m_b_unlock`: 外部消費者があり再生中のバッファの Unlock の直後に `rewrite_unread()`。公開済みのフレーム `[consumed, wf0)`(`wf0` = この Unlock の自分の同期の前の公開の終わり)のうち、**書かれた領域に当たるもの**を数え(読み位置からの距離が ガード = 256 フレーム未満 → lost、以上 → applied)、applied > 0 のときだけ `[consumed + ガード, wf0)` を `mix_chunk` で混ぜ直して `fle_audio_ring_rewrite()` で上書きする。`wf0` を使うのは、同期が足すフレームは新しい内容から作られていて直す必要が無いため。
- `fle_audio_ring.c`: ガード(`REWRITE_GUARD_FRAMES` = 256)、`fle_audio_ring_rewrite`、カウンタ(`fle_audio_rewrite_note` と、ページが読む `fle_audio_rewrite_stat`〔events/applied/lost〕)。
- `fle_audio_transport.c`(新規): **transport の分離**(PM の判断): EM_JS(wasm)の `fle_js_*` と telemetry を `fle_audio_ring.c` から移し(`fle_tp_*`)、wasm では `fle_js_rewrite`(`rd + ガード` 以降、WR より前だけを SAB に書く。WR とページのカウンタは動かさない)を足した。native は何もしない版。**テストは自分の transport(`lift/harness/audio/sim_transport.c`)をこのファイルの代わりにリンクする**(製品のコードにテスト用の口を置かない)。
- 副作用のあるもの: lead は変えない(60 ms)。生成 C は不変(CI の lift の 44 ユニットが ci-src と byte 一致)。wasm は +1,451 B(F2 のみ)、transport の分離でさらに +105 B(6.4)。

**費用**: Unlock までの最後のスライス(スライス開始〜Unlock、`audio-pulse` の `unlockCompute`)の平均 ms の中央値 A 6.88 / B 6.99(Chrome、14 セッションずつ)、6.51 / 6.69(Firefox、10 ずつ)=**≤ 0.2 ms/Unlock**(Unlock は約 10.8/s)。数えるループは Unlock ごとに約 2,600 反復で、領域が遠い通常の Unlock でも走る。**早期脱出の最適化は入れていない**(実エンジンで測っていない物を入れない)。

### 5.1 lead を下げる案(副作用の表。既定は変えない)

`?audiolead=` で A bundle の Chrome(a・b、各 6 セッション、`tables/lead-eager-summary.tsv`)。

| lead | a: 完全/欠け/消失 | worklet underrun quanta(a / b の合計) | エンジンの無音埋め(フレーム、a / b) |
|---|---|---:|---:|
| 15 ms | 4 / 2 / 0 | 87 / 34 | 9,566 / 4,400 |
| 30 ms | 6 / 0 / 0 | 11 / 3 | 1,236 / 255 |
| 60 ms(既定。j1 の 4 セッション) | 1 / 1 / 2 | 0 / 0 | 0 / 0 |

lead を下げると 1 拍目の余裕は増えるが、Worker の一時的な停止を公開で吸収できなくなり worklet の underrun が出る(15 ms で a・b 計 121 quanta)。F2 は lead を変えずに同じ余裕を得る。

## 6 検証

### 6.1 native(`wall_fr15` に 5 件追加、計 23 passed)
- 間に合った書き込み(1000 フレームを 500 フレーム先に)は全て反映、折り返す領域(8000〜8500)も反映。
- ガード内は反映せず lost を数える(`consumed 1800`、領域 [1500, 2500): 444 applied・256 lost、[1800, 2056) は無音のまま)、ガードの境界は厳密(257 フレームで applied 1・lost 256)、全てガード内(200 フレーム)は applied 0・lost 200。トランスポートへ ガード内のフレームを渡さない(`frames offered to the transport inside the guard` = 0)。
- 遅れていない書き込み(領域が公開より先、または次の周回)は何も変えず数えない(ring の FNV-1a が 7,292 個の 0 の float の手計算と一致)。
- 正弦波(440 Hz)を 2 つの半分で書き、後ろの半分を『前半 550 フレームが無音で公開された後』に書く: ring は理想の列と **6,646 フレーム全部一致(差 0)**、隣り合うフレームの最大の段差は理想の正弦波の最大の段差と同じ(0.0153198242)=再公開した領域の境界に段差・二重ミックスが無い。
- 外部消費者のときだけ `rewrite_unread` が呼ばれる(ソース検査)。既存の native のテストは不変: `wall_fr15`、`wall_r` 31、`wall_fr11` 47、`fletest` が `fle_audio_ring.c` を読む 29 本(`call_iat callback deferred event hook_points k32_dsp load modules msctf null0 realmod site_override srw sysinfo uilang wall_b wall_c1 wall_e1 wall_fr11 wall_fr15 wall_fr18 wall_fr3 wall_fr4 wall_fr5 wall_fr6 wall_q wall_r wall_r2 wall_s`)を実行(`logs/cargo-affected.txt`)。全体の 152 results・2,118 passed の基準は**実行していない**(PM の統合時に)。
- **変異 12 件すべて検出**(`tables/mutations-late.tsv`: 呼び出しを無効、`<= guard`、ガード 0、`from = c`、終端、混ぜ直しの 1 フレームのずれ、数える条件の削除、折り返しの無視、applied/lost の入れ替え、旗なしの外部消費者、`wf0`、領域の開始)。M04(`from = c`)は、テスト用 transport が『ガード内のフレームを渡された』ことを数えるまで生き残ったので、その計数を足した。

### 6.2 page(`aud1-page.diff`、PM の判断で採る)
`worker-hook.ts` が Worker の `fleAudioRewrite()` を 1 Hz の報告に載せ、`page-audio.ts` の `detail()` に `engineLateWrites/Applied/Lost`、`shared/perf.ts` のビーコン検査に同じ 3 項目(数だけ。音声データ・個人情報なし)。単体 +1(1377 passed、8 skipped)、アプリ変異 +1(`audio-late-write-not-reported`、検出)。

### 6.3 実エンジンの A/B(`tables/p1-ab-summary.tsv`、`tables/cycles-p1all.tsv`)
A・B は同じ 52034ca9 の木から、CI の段スクリプト(`tools/mk-var.sh` = ship6 の写し。lift→hybstrip→objects→link→seed→bundle、`RM_RT=1`)で作った(A: wasm `b7904bc697f8` 133,924,267 B、B: `bef8f676baee` 133,925,718 B〔A +1,451 B〕、分離後の B2: `77d8ac7af5bb` 133,925,823 B。dll・seed 同一。lift の 44 ユニットは byte 一致)。どちらも glue にだけテスト用ログ(別の assets 入口。製品に入れない)を足した。A と B のセッションを**交互に**(順序も交互に)走らせた。

| ブラウザ | 条件 | A 完全/欠け/消失 (周期) | B 完全/欠け/消失 (周期) | A→B の『欠けまたは消失』 | 片側 Fisher P |
|---|---|---|---|---|---:|
| Chrome 156 | **a** | 3 / 6 / 4 (13) | **11 / 0 / 0 (11)** | **10/13 → 0/11** | **0.0001** |
| Chrome 156 | b | 26 / 0 / 0 (26) | 22 / 0 / 0 (22) | 0/26 → 0/22 | 1 |
| Chrome 156 | c10 | 13 / 0 / 0 (13) | 11 / 0 / 0 (11) | 0 → 0 | 1 |
| Chrome 156 | c30 | 13 / 0 / 0 (13) | 10 / 0 / 1 (11) | 0 → 1 | 1 |
| Firefox 158 | a | 10 / 0 / 0 (10) | 10 / 0 / 0 (10) | 0 → 0 | 1 |
| Firefox 158 | b | 20 / 0 / 0 (20) | 20 / 0 / 0 (20) | 0 → 0 | 1 |
| Firefox 158 | c10 | 10 / 0 / 0 (10) | 9 / 0 / 1 (10) | 0 → 1 | 1 |
| Firefox 158 | c30 | 9 / 1 / 0 (10) | 10 / 0 / 0 (10) | 1 → 0 | 0.5 |
| 全部 | | Chrome 55/6/4 (65)、Firefox 49/1/0 (50) | Chrome 54/0/1 (55)、Firefox 49/0/1 (50) | Chrome 10/65 → 1/55 (P=0.009)、Firefox 1/50 → 1/50 | |

- 除外したセッション(キーが届かず全周期が無音): A の Chrome 1、B の Chrome 3。A・B とも起きる(F2 の影響とは読まない)。有効なセッションは Chrome A 13・B 11、Firefox 10・10。
- **B の消失 2 件は F2 が救えない種類**: Chrome の c30(Unlock 間隔 188.5 ms、worklet underrun 6 quanta)と Firefox の c10(間隔 340.5 ms、underrun 31 quanta)は、Space の処理だけでは説明できない大きさの Worker の停止(間隔が上の F2 の範囲約 180 ms を超える。Space との関係は未特定)で、出力が公開を追い越した(読み位置に追いつかれた)。A では 115 周期で最大 175 ms。B の 2 件は偶然差の範囲(2/110 対 0/115)で、F2 が原因とは読まない(F2 の費用は ≤ 0.2 ms)が、**原因(協調実行/GC/tier-up)は未特定**。
- **F2 が効いた周期**(領域の先頭が出力の 22〜46 ms 先で、旧実装なら 640〜1664 フレームが古い内容で公開済み): Chrome a の B で 5 周期(stale 最大 1664)、Firefox の B で 4 周期(最大 1920〔上の救えない 1 件を含む〕)。1 拍目は完全。
- **PCM の同一性**: 1 拍目が完全で捕捉が途切れなかった周期の、先頭 2.5 s(110,250 フレーム)の sha256 は、**B は 103 周期全部が同一(`06128fd08199…`)、A は 104 周期中 103 が同一**(例外 1 件は Firefox の A の 1 周期で、後ろのブロックの約 390 フレームに最大 0.002 の差=旧実装で後ろのブロックの頭が古い内容だったと読む)。Chrome と Firefox でも同一。F2 で救われた周期(Chrome a の B 5 周期、Firefox の B 3 周期)も同じ sha256=遅れなかった周期とビット一致。**旧実装の欠けた周期の PCM は、古い内容で公開済みだったフレーム数(stale)で決まる**(同じ stale なら同じ sha256: 128 フレームで 1 拍目の峰値 0.556 〔4 周期、Firefox 1 を含む〕、256 フレームで 0.2501 〔2 周期〕、384 フレームで 0.0872 〔1 周期〕、640〜1152 フレームでは 1 拍目が消え、以後は同一〔3 周期〕)=欠けは決定的に『ブロックの頭が無音になる』。
- 遅れそのもの(Space の後 8 個の Unlock の最大間隔、中央値): Chrome a A 133 / B 124 ms、b 102 / 104、c10 101 / 102、c30 101 / 103。Firefox a 108 / 110、b 103 / 105。

### 6.4 transport の分離の検証(PM の判断 1。B2 = B から `fle_audio_ring.c` の transport を `fle_audio_transport.c` に分けた木)
- **glue はバイト一致**: B と B2 の `engine-glue.824fddaaa68b.js` は同一(sha256 の先頭 12 桁が同じ、`fle_js_*` 11 関数の本文の差 0)=EM_JS の本文と関数の集合は不変。生成 C は不変(lift の 44 ユニットを再利用)。`fle_dsound.o` はバイト一致(sha256 同一)。
- **wasm の sha256 は一致しない**(B `bef8f676baee…` 133,925,718 B、B2 `77d8ac7af5bb…` 133,925,823 B、**+105 B**。A は 133,924,267 B。S1 の線 141,145,856 B の下であることは変わらない)。セクションの差は 2 つだけ: 関数セクション −1 B(関数が 1 個減)、コードセクション +105 B(データ +1 B)。**差の説明**: ランタイムの定義済みシンボルは、B の `fle_audio_ring.o` に新しい `fle_tp_*` 8 関数が加わっただけ(`tables/syms-b1.txt`・`syms-b2.txt`)。旧 ring の中で同じ翻訳単位内だった薄い `js_*` ラッパが翻訳単位をまたぐ呼び出しになり、link 後の `wasm-opt` のインライン化の結果が変わった(読み。bundle は関数名を持たないので、減った 1 関数の名前は特定していない)。
- **挙動**(Chrome 156 と Firefox 158 の各 6 セッション、a・b。`tables/cycles-q2.tsv`): Chrome a **6/6 完全**(うち 4 周期は stale 128〜1152 フレームで F2 が効いた)、b 12/12。Firefox a 6/6(stale 256 が 1 周期)、b 12/12。完全な周期の PCM の sha256 は全て `06128fd08199…`(A・B と同一)。F2 のカウンタは動いている(セッション末で events 26〜46、applied 23,653〜60,025 フレーム。lost 112〜3,213 フレームは主に起動時)。

## 7 出典のない選択(S-)

| 名前 | 値 | 理由 | 代替 | 撤回条件 |
|---|---|---|---|---|
| **S-ds-late-write**(新) | Unlock の後、公開済みで未読のフレームのうちガード以降を、バッファの現在の内容で上書きする | 契約は『書いたデータはデバイスがその位置を読むときに再生される』。Learn は write cursor 手前を『安全に変えられない』とするのみ(反映しないとは書かない) | (i) lead を下げる〔副作用の表 5.1: underrun が出る〕、(ii) 出力を遅らせる〔`audiodelay`: 遅延が増える〕、(iii) 何もしない | 実機の Windows で FL の最初のブロックが write cursor の手前に来ないことが示されたとき。または F2 が PCM を変える反例 |
| **S-ds-rewrite-guard**(新) | 256 フレーム(5.8 ms = worklet の 2 quantum) | worklet が今読んでいる 1 quantum と次の 1 quantum には触らない(読み出しと上書きの競合を避ける) | 128、512 | 競合による標本の傷が観測されたとき(今回の走行では F2 の前後で PCM の sha256 が一致し、0 件) |
| **S-ds-late-lost-count**(新) | 書かれた領域のうち、次の再生が読み位置から ガード未満の距離のフレームを lost とする。周回の手前に戻ったフレーム(すでに古い内容で読まれた)は数えない | `fle_audio_unlock_note` と同じ『領域は次の周回』の規約 | 読まれたフレームも lost に入れる | 診断の見え方が不足したとき |
| S-audio-lead(既存) | 60 ms | 0139 のまま。**変えない**(PM) | 15/30(5.1) | — |

## 8 未解決・PM の判断点・他の決定との関係

- **原因は残る**: Space の直後のミキサーの応答の遅れ(1 本の Worker での協調実行)は F2 で隠れるだけ。除去は mt3(B′ 段階 2、ミキサーを別 Worker に)の領域。F2 の効く範囲は、Unlock の間隔が約 180 ms(= 通常の 93 ms + (92.9 − 5.8) ms)以内の遅れまで。それを超える停止(B の 188/340 ms の 2 件)は救えず、worklet の underrun として P3 に現れる。
- **別の問題(gpu1 のビーコン)**: 依頼者の 1 セッション(e44b7eec4981)は 255 s の再生中ほぼ全部が underrun(10,174 quanta)・非無音 0(エンジンが実時間に追いつけない)。これは P3 の主題で、F2 では直らない。
- **page の差分**(`aud1-page.diff`)は別 hunk。F2 の効き(events/applied/lost)を依頼者の機械のビーコンで読める。
- 取り込みの順序: `aud1.diff` は `lift/runtime/fle_audio_ring.*`・`fle_dsound.*` と新規 3 ファイル(`fle_audio_transport.c/.h`、`lift/harness/audio/sim_transport.c`)。新規ファイルが `git ls-files` で追跡されることを、差分を当てた `git archive` の木で確認した。mt3 が音声リングを共有メモリ化するときは、`fle_audio_transport.c`(EM_JS)の側を触ることになる。opt1 が畳むオプションには、この変更は触れない(旗・環境変数・URL パラメータ・プリセットを足していない)。
- `harness/testfast/deps.tsv`(『generated … do not edit』)は触っていない。新規の runtime 2 ファイルの行は `test.sh record` が足す。
- 全体のテスト(152 results・2,118 passed)は未実行。生成 C は不変(lift の 44 ユニットが byte 一致)。wasm の大きさ: A から +1,556 B(F2 +1,451 B、分離 +105 B)。

## 9 gpu1 が読んだ依頼者のビーコン(Firefox 159 / Windows 10、7 セッション、`lift/out-gpu1/tables/client-beacon-audio.tsv`)との突き合わせ

- ctxState = running、sampleRate 44,100、outputLatency 22〜32 ms(この機械の headless: Chrome 24〜32 ms、Firefox 31 ms)、resume の所要 58〜85 ms。**アクセントのクリックの峰値 0.941 が 3 セッションで worklet に届いている**(音のデータは依頼者のブラウザの worklet まで届いていた)。
- **1 拍目の欠けを直接示す列(`UR_STARTUP`、late_frames、worklet の skipped)はビーコンの版に無い**。F2 の `engineLateWrites/Applied/Lost`(page 差分)が本番に届けば、依頼者の機械で『遅れた書き込みがあったか・F2 が効いたか・ガード内で失ったか』を読める。
- 放置後の再発(ctx の状態・outputLatency の変化・worklet の止まり)は、この 7 セッションからは読めない(1 セッションあたりの記録が少ない)。

## 10 成果物

`lift/out-git/aud1-results/`: `aud1.diff`(runtime・テスト・harness)、`aud1-page.diff`(page)、`MERGE-NOTES.md`、`docs/0241-first-beat.md`、`docs/README-0241-line.txt`、`tables/`(上の表の元)、`logs/`、`tools/`(`first-beat.mjs`/`first-beat2.mjs` = テスト運転手、`make-instr-glue-j.py` = テスト用の glue、`mk-var.sh` = bundle の作り方、`run-pairs.sh`、`analyze-cycles.py`/`table.py`/`ab-summary.py`/`timeline.py`/`slices.py`、`mutate-late.py`、`build-scen.sh`、`mkdiff.sh`)。

## PM の決定と取り込み(2026-11-05)

取り込んだ基準: HEAD c8cdfd07 に `aud1.diff`(10 ファイル、`-` 行 153 本のうち `fle_audio_ring.c` の 148 本は新ファイル `fle_audio_transport.c/.h` へ移した行で、担当が行単位の存在をスクリプトで確認、残り 5 本は読んだ)と `aud1-page.diff`(5 ファイル、`-` 行 8 本は全部読んだ)を `patch -p1`(offset・fuzz・reject なし)。PM が測った値(この機械、`lift/out-pmchk-u1/`): FLEngine の既定の生成 C の sha256 `cd9107ce…`(215,811,678 B)不変、typecheck rc=0、vitest 1,228 passed + 7 skipped(+1)、全体テスト **196 results / 2,392 passed / 0 failed / 1 ignored**(+5 は wall_fr15 の新規 5 件)、a4 FRUITY 無効 942/942・895/895・functions 5,310・entries 1,009,737 不変。

1. **F2(S-ds-late-write)を契約の修正として採る**(旗なし、無条件、lead 60 ms は変えない)。Chrome 156 の最初の再生で 1 拍目の欠け/消失 10/13 → 0/11(P=0.0001)。完全な周期の PCM は前後で同一。Firefox 158 は 1/50 対 1/50 で差なし。F2 は遅れそのもの(1 本の Worker での協調実行)の除去ではない。
2. **transport の分離**(EM_JS と telemetry を `fle_audio_transport.c/.h` へ。テストは `sim_transport.c` を差し込む)。製品のコードにテスト用の weak フックを置かない(no-options.md §2.1 の 6)。glue はバイト一致、wasm は +105 B(薄いラッパのインライン化の差と読む。未特定)。
3. **ビーコンに F2 のカウンタ(events/applied/lost)を載せる page の差分を採る。** ビーコンが本番に届くようになった(a61760ba)ので、依頼者の機械で F2 が効いたか・遅れが残るかを読める。数だけ(音声データ・個人情報なし)。
4. **未解決**: 依頼者の『放置後に再発』は再現できていない(実機 Windows・WASAPI・Firefox 159 の値は 0)。F2 が救えない Unlock 間隔 約 180 ms 超の停止(Chrome 188 ms、Firefox 340 ms)の原因は未特定(協調実行/GC/tier-up)。依頼者のビーコンの 1 セッション(255 s の再生でほぼ全部 underrun)はエンジンが実時間に追いつけない別の問題で P3 の主題(mt3 の領域)。キーがページに届かず全周期が無音のセッションが Chrome で A 1/14、B 3/14(原因未調査、ツールの不具合と読む)。
