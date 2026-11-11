# 0305c(仮): 実スレッド E11 の出力に『書かれた零』が約 60 ms(2646 フレーム)× 約 4 区間/s 出る件 — コードの読みと gpu1 の既存ログで、原因は **Worker ごとの JS 状態 `A.wr` を使う `fle_js_publish` の gap 零埋め**と読める(mt3。読み、Chrome 無し)

状態: **仮**(0305 の第 3 報の一部)。作業者 mt3、2026-11-06 JST。依頼: PM(18:55 の info。gpu1 の候補 (a)『実スレッド版でゲストが零を書く』を、`rt-on-e11.diff` の `fle_dsound.c`/`fle_js_rewrite` 周りのコードの読みで追い、file:line つきで)。区分: **実測**=既存のログ(gpu1 の `gonogo2/logs/rt-rt11-300/audio-p3.json` と `rt-e11c-300/audio-p3.json`)を私が数え直した値、**読み**=コードの解釈、**未確認**。**Chrome は起動していない。実エンジン(wasm)の新しい実行は 0 件**。

## 0. 冒頭の三点

1. **結論(読み + 既存ログの実測)**: 零を書いているのは**ゲストでも `rewrite` でもなく、Worker 0 の `fle_js_publish`**。`fle_js_publish` は『前回の publish の末尾 `A.wr` より先に書くとき、その隙間を零で埋める』が、**`A` は Worker ごとの JS オブジェクト**(`globalThis.__fleAud`)で、`audio_sync` は**全 Worker のスケジューラ点**(`poll_waiters` → `fle_audio_poll`)から呼ばれる。Worker 0 の `A.wr` は Worker 1 が進めた分を知らないので、Worker 0 が publish するたびに『Worker 1 が書いた、まだ読まれていない最大 lead(2646 フレーム)の窓』を零で上書きする。
2. **既存ログの実測が一致する**(§3): Worker 0 の JS 状態は、実スレッドで **publish 3.68 回/s、gapFrames 42,485 フレーム/s、1 回あたり平均 11,546 フレームの隙間**(協調は publish 99.8 回/s、gapFrames 0)。1 回 11.5k ≫ lead 2646 なので、毎回、未読の窓(≒ lead = 2646 フレーム = 60 ms)が全部零になる。3.68/s × 2646 = 9.7k フレーム/s = **22%**(gpu1 の実測: 約 4 区間/s、19.6%)。協調は 1 Worker で `A` が 1 つなので起きない。
3. **これは実スレッド版の本物の欠陥でも測定専用 glue だけの artefact でもなく、『段階 C(音声リングを wasm の共有メモリへ)の前の設計』が複数 Worker で動いたときの欠陥**。gpu1 の `patchglue-rt.py` は Worker ごとに `A` を遅延生成して Worker 1 から音を出せるようにした(測定専用)が、`A.wr` が Worker ごとに別になる点は**製品のコード(`fle_audio_transport.c`)の構造そのもの**。段階 C(第 1 報 I6)で状態を共有メモリに移すとき、**`wr`(gap の基準)も共有にしないと同じ欠陥が出る**。この読みが当たれば、`nonSilentQuanta` 実スレッド 81% 対 協調 96%(gonogo2 §3 の未解決)の差(約 15 ポイント)もほぼ説明できる。

## 1. 経路(file:line。読み。足場は `fle_dsound.c`・`fle_audio_ring.c`・`fle_audio_transport.c` に触れていない = E11 と同じ行)

| # | 場所 | 内容 |
|---|---|---|
| 1 | `fle_thread.c:481`(`poll_waiters` の中の `fle_audio_poll()`)、`:841`(`schedule_ex_inner` の中の `poll_waiters()`)、`:500`(`fle_thread_wake_check`。呼び出し元 `:1547`・`:1784`・`:1799`)。行番号は `rt-on-e11.diff` を E11 に当てた直後の木(第 1 報の直しの前) | **スケジューラ点ごとに `fle_audio_poll()` を呼ぶ**。`MYW` の条件は無い(`fle_input_ring_poll` だけ `MYW == 0` に制限した。`:836`)。= **Worker 0(UI)のスケジューラ点でも呼ばれる** |
| 2 | `fle_dsound.c:471` `void fle_audio_poll(void) { audio_sync(); }` | |
| 3 | `fle_dsound.c:416-469` `audio_sync()`。`:430-438` 『`r->write_frames < c`(poll が遅れた)なら外部消費者のとき `r->write_frames = c` に飛ばし `late_frames` に数える』。`:439-448` 『`target = c + lead` まで `mix_chunk` → `fle_audio_ring_append`』 | 共有メモリの C 状態(`r->write_frames`)は GIL の下で全 Worker 一貫。**publish が呼ばれる Worker は固定でない** |
| 4 | `fle_audio_ring.c:148-176` `fle_audio_ring_append` → `:26` `js_publish` → `fle_audio_transport.c:129` `fle_tp_publish` | **呼んだ Worker の JS** に入る |
| 5 | `fle_audio_transport.c:20-24` `fle_js_init`: `const A = { …, rdHi: 0, rdPrev: 0, wr: 0, …, gapFrames: 0 }; globalThis.__fleAud = A;` | **`A.wr` は初期値 0、`globalThis` は Worker ごと** |
| 6 | `fle_audio_transport.c:54-76` `fle_js_publish`: `:62` `if (at > A.wr) { let g = at - A.wr; A.gapFrames += g; if (g > A.cap) g = A.cap; for (…) A.d[q] = 0; A.d[q+1] = 0; }`、`:69` `A.wr = first + n;` | **隙間 `[at − g, at)` を零で埋める**。`g` は `at − (この Worker の前回の末尾)`。`rd` 以上の部分は**未読の、他 Worker が書いた実音** |
| 7 | `fle_audio_transport.c:74-84` `fle_js_rewrite`: `:81` `end = min(first + n, A.wr)` | rewrite の上限も**この Worker の `A.wr`**(別 Worker が先まで publish していると、その先は rewrite されず、publish 時の内容のまま) |
| 8 | `fle_audio_ring.h:20` リング容量 32768 フレーム、`:23` `FLE_AUDIO_LEAD_MS` 60(= 44.1 kHz で 2646 フレーム)、`fle_audio_ring.c:179` `REWRITE_GUARD_FRAMES` 256 | 零の長さの上限は『未読の窓』= **`write_frames − rd` ≒ lead = 2646**。guard(256)は別の定数(rewrite が変えられない近傍) |

**零になる機構**: Worker 1 の `audio_sync`(`GetCurrentPosition` が 107 回/s)が `c + lead` まで publish し続ける。Worker 0 のスケジューラ点の `audio_sync` が、Worker 1 の 2 回の sync の隙間(`c` が最大 約 410 フレーム進む間)に当たると、Worker 0 が**小さな塊(数百フレーム)**を publish する。その publish の `first`(= 共有の `write_frames`)は Worker 0 の前回の末尾 `A.wr`(数百 ms 前)より遥かに先なので、`:62` の分岐に入り、`[first − g, first)` のうち **未読の部分(`rd ≦ i < first` = 最大 lead)を零で上書き**する。それより前(`< rd`)は消費済みで無害。その後の `rewrite_unread`(`fle_dsound.c:337-361`、Unlock のとき)が `mix_chunk` で窓を作り直して置き直すのは **Unlock があり、かつ guard より先の部分だけ**。毎ブロック(10.77 回/s)の Unlock の rewrite が、零にされた窓のうち FL が書き終えた分を直せることもあるが、直らなかった部分が『書かれた零』として残る。

## 2. gpu1 の候補ごとの判定(PM の問い)

| 候補 | 判定 | 根拠 |
|---|---|---|
| (a1)『Worker 1 が Lock した領域を Worker 0 の **rewrite** が零で上書きする』 | **該当しない** | `rewrite_unread` は `m_b_unlock`(`fle_dsound.c:1137`)からだけ呼ばれ、書き込み元は `mix_chunk` で作った**現在の DS バッファの内容**。零を作る経路ではない。Unlock は Worker 1(ミキサー)が呼ぶ |
| (a2)『Worker 0 の **publish** が Worker 1 の書いた窓を零で上書きする』 | **該当する(主因と読む)** | 上の 1〜7 と §3 の実測 |
| (a3) Lock〜Unlock の間に Worker 0 の `audio_sync` が**書きかけのブロック**を publish する | **起きうる(副次)**。協調では譲らないので起きなかった(gpu1 0289 §9 の行 9)。FL が書き終えていない部分は零/古い内容で publish され、Unlock の `rewrite_unread` が guard より先なら直す | `fle_dsound.c:331-361`。ただし、これは設計(決定 0241)で、直せなかった分は `lost` に数える(`fle_dsound.c:349`、実測の Lost 120 k〔実スレッド〕対 121 k〔協調〕でほぼ同じ → 主因ではない) |
| (a4)『GIL 区間で Unlock の publish が**遅れる**』 | **書かれた零は作らない(underrun 側)** | 遅れると `audio_sync` は `:430-438` で `write_frames = c` に飛び `late_frames` に数える。Worklet は零を再生する(underrun。実スレッドで 15 quanta・2 回、少ない)が、リングに零を書くのではない。飛んだ publish の gap 零埋め(`:62`)は**消費済み**の位置(`< rd`)に書くので無害 |
| (b) 測定専用 glue の artefact | **半分正しい**: `A` が Worker ごとに別になるのは gpu1 の glue(遅延 init)の結果だが、**`A.wr` が Worker ごとの JS 状態であるのは製品のコードの構造**。製品の段階 C が状態を共有メモリへ移すとき `wr`/`rdHi` も共有にする必要がある(§4) |

## 3. 既存ログの実測(`tables/0305c-worker0-audio-state.tsv`)

`audio-p3.json` の `reports`(Worker 0 の JS 状態〔`fleAudioWorkerState`〕を 1 Hz でページに送った系列)を、再生の最後の 200 s で数え直した:

| 腕 | W0 の publish/s | W0 の gapFrames/s | 1 publish あたりの平均 gap |
|---|---|---|---|
| **実スレッド**(rt11) | **3.68** | **42,485** | **11,546 フレーム(262 ms)** |
| 協調(e11c) | 99.81 | 0 | 0 |

- **隙間 11.5k ≫ lead 2646** なので、W0 の publish 1 回ごとに未読の窓全部(≒ 2646 フレーム = 60 ms)が零になる。**零の塊の長さが lead に一致**する(gpu1 の観測: 約 2646)。
- 頻度 3.68/s は gpu1 の『約 4 区間/s』に一致。零のフレーム数の期待値 3.68 × 2646 ≒ 9.7 k フレーム/s = 44.1 kHz の **22%**(gpu1 の実測 19.6%)。
- gapFrames 42,485/s ≒ 44.1k の 96% = **W0 の `A.wr` が、W1 が publish した分(実時間のほぼ全部)を全然知らない**ことの直接の証拠。協調は 1 Worker なので gap が 0(115,349 は起動時の 1 回)。
- gpu1 の 0289 §2 は『`engineGapFrames` は Worker 0 の `A.gapFrames` で Worker 1 の状態を見ないので意味が無い(値 2.5 M は読まない)』と書いたが、**この値こそ W0 が publish して隙間を埋めていた証拠**だった(2.5 M は 60 s の run で 約 42k/s × 60 s)。
- 副次: 同じ機構が**逆向き**にも働く(W0 の publish の後の W1 の publish は、W1 の `A.wr` が W0 の塊より前なので、W0 の塊を零で消す)。大きさは W0 の塊(数百フレーム)で、W1 側の `gapFrames` は報告に出ていない(**未確認**)。

## 4. 製品形(段階 C)への含意と、gpu1 が安く確かめる方法

- **含意**: 段階 C(第 1 報 I6、0234l)は『リングを wasm の共有メモリへ、JS を C に』で、`fle_tp_publish` が C の `memcpy` + `__atomic_store_n(&hdr[1])` になる。**gap の基準を共有の `hdr[1]`(WR)か `r->write_frames` にしないと、Worker ごとに別の状態を持つ今の欠陥が残る**。0234l の設計は『ギャップの無音埋めも C』と書いてあり、C 化すればそれは共有の `r->write_frames` を使う形に自然になる(**要確認**: 設計に『Worker ごとの `wr` を持たない』と明記する)。遅延 init(gpu1 の glue の方式)は**製品の形にしない**。
- **安い確認(Chrome 1 本 60 s、私は起動しない)**: gpu1 の `patchglue-rt.py` に『`A.wr` を Worker 間で共有する』1 か所を足す。`fleUlog` の SAB(全 Worker で共有済み)に Float64 を 1 つ取り、`fle_js_publish` の `A.wr` の読み書き(`:62`、`:69`)と `fle_js_rewrite` の `A.wr`(`:81`)をその共有スロットに置き換える(GIL の下なので atomics は不要)。**これで零が消えれば原因確定**、消えなければ (a3)(a4)(`rewrite` の上限)を疑う。
- **もう一つの確認(ログだけ)**: 既存の rt11 の `audio-p3.json` の `unlocks` と PCM 捕捉の零の位置を突き合わせ、**零の塊の開始が『Worker 0 の publish の時刻』**に一致するか(W0 の publish の時刻はログに無いので、次の run で `A.publishes` を 10 s ごとでなく publish ごとに記録するか、W0 の `fle_js_publish` で `console` に first/n/gap を出す)。
- **別の直し方(参考、判断は PM)**: `fle_audio_poll` を『音声のスレッド(ミキサー)がいる Worker』に限る。ただし契約(FL の通知イベントを仮想/実時計の進みに合わせて立てる)の観点で、どの Worker でも `audio_sync` が呼べることは設計の前提(協調では全部同じ Worker)なので、**状態を共有にするほうが正しい**。

## 4b. 確定(PM 19:10 の info)

gpu1 の `WRFIX=1`(`A.wr` を Worker 間で共有した run `rt11wr`)の PCM を PM が `eval-pcm.json` で数えた: **零の塊 実スレッド修正前 212 本・19.63%、修正後 3 本・0.036%、協調 4 本・0.038%**。= §1〜§3 の読み(Worker 0 の `fle_js_publish` の gap 零埋めが Worker 1 の未読の窓を零で上書きする)は**確定**。以下の §5 の『限界』のうち『PCM の零の開始と Worker 0 の publish の時刻の一致は直接見ていない』は、介入(`wr` の共有)で零が消えたことで置き換わった。段階 C の設計への明記は 第 1 報の表 I6 に入れた。

## 5. この読みの限界(正直さ、確定前の記述)

- 『W0 の publish が零で窓を上書きする』は**コードの読みと W0 の JS 状態の実測**で、PCM の零の塊と W0 の publish の時刻が一致することは**直接見ていない**(上の確認)。頻度 3.68/s と 19.6%・長さ 2646 の一致は強い状況証拠だが、独立の原因が重なっている可能性は残る(例: (a3) の書きかけの publish、`rewrite` の上限が W1 の `A.wr` で切れる経路)。
- 3.68/s の W0 の publish が、W0 のどのスケジューラ点で起きるか(UI の起床 24〜37 回/s のうち約 1/8 が publish に至る理由)は**未確認**。
- Worker 1 側の `A.gapFrames` は報告に無い。
- 協調の 99.8 publish/s は、協調の `GetCurrentPosition`(107 回/s)ごとの `audio_sync` と整合する。

## 6. 自己申告

- 最初、零の原因を GIL の遅れ(a4)と書きかけの publish(a3)から探したが、`gapFrames` の値(gpu1 が『意味が無い』とした値)を数え直して初めて主因に気づいた。
- 『3.68/s × 2646 = 22%』は粗い見積り(窓の重なり・部分的に消費済みの分を無視)で、gpu1 の 19.6% との一致は『桁と比が合う』程度に読む。


## PM の決定と取り込み(2026-11-06 19:45、docs のみ)

取り込み: mt3 の最終(19:43)の一部として `docs/investigations/mt3w/0305/` の表・道具とともに置いた。判断は決定 0305 の PM の節にまとめた(E13 = Dwm + gp-extension、着手順 S0〜S6、D1 = (a')・D2・D3 の確定の範囲、担当)。0305c(実スレッドの PCM の零は Worker ごとの JS 状態の `A.wr` が古いための測定用 glue の欠陥)は gpu1 の決定 0306 の PM の節と合わせて読む。**依頼者の機械・wasm の実行は 0 件**。
