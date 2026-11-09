# 0312c: S2(T の B/C の梃子)の設計の読みと、I15(Worker 数 n = 2/4/8/32)の静的な先行点検(mt3。PM 指示 22:02。**コードを読んだだけ。ブラウザ・wasm の実行は 0 件。数は 0306 の gpu1 の実測を引用するか、無ければ『読み』と書く**)

## 0. 冒頭の三点

1. **B(page → UI Worker、実 5.5 ms 対 協調 0.7 ms)の根拠はコードで説明できる**: 入力 ring を読む場所は Worker 0 の (a) scheduler 点 `schedule_ex_inner` の poll(`fle_thread.c:800〜801` → `harness/fruity_idle.c:43` `fle_input_ring_poll` → JS の `fleInputPoll`)と (b) page の `wake` メッセージ(`client.ts:105`、**Worker が『parked』のときだけ**送る。`worker.ts:59` → `drainRing()`)の 2 つだけ。parked は idle のときだけ立つ(`worker.ts:121` `idle: ring.setParked(true)`、`:119` `resumed: setParked(false)`)。協調は 1 本の Worker が mixer も含めた全 scheduler 点(約 2,000 回/s = 0.5 ms 間隔)で poll する。実スレッドの Worker 0 は **UI スレッド 1 本**しか動かさず、scheduler 点は UI スレッド自身の待ち/切り替えの点だけで、しかも idle の中で present の JS(`worker.ts` の `onIdle` の `deskTick()`、gpu1 の実測で 1 present 12〜14 ms ×35/s)が同期で走る間は poll も `wake` の処理もできない。0306 §11 の B の分布(0〜約 15 ms にほぼ一様、最大 = 1 present の長さ)と合う〔gpu1 の読み。突き合わせは未〕。
2. **だから `Atomics.notify` で起こす案(0306 の候補 (1))は、B にはほとんど効かない〔読み〕**: Worker 0 の idle の待ちは既に `Atomics.waitAsync` + JSPI で、event loop が回るので `wake` メッセージは即座に処理される(`fle_thread_wasm.c:135〜163`)。起こしたい相手が **同期の JS(present)か wasm の計算の最中**のときは、`Atomics.notify` も postMessage も割り込めない。B を詰める本線は **present を Worker 0 から出す P(0311)**で、S2-B は『P の後に残る B』が 1.5 ms を超えるときだけ(下の B2)。
3. **C(エンジンの処理、実 +5 ms)の仮説は『UI スレッドの GIL の取り直しの待ち』が第一**(`fle_gil.c:71` の 2,000 回 spin のあと futex 眠り → 起床に 数十〜100 µs〔読み。V8 の Atomics.wait の起床〕)。測る道具は **D3a の `patch -R` + `-DFLE_GIL_STATS -DFLE_CS_HOLD_STATS`**(§3)で、gpu1 の `mk-rts.sh`(0289)がそのまま使える。**実エンジンでの値は未取得**。

## 1. B: 5.5 ms と 0.7 ms の差の、コードの根拠

| 項目 | 協調(E11) | 実スレッド(E11) | 根拠(file:line) |
|---|---|---|---|
| ring を読む場所 | 全 scheduler 点(UI + mixer 他の全スレッドの切り替え) | Worker 0 の scheduler 点だけ | `fle_thread.c:800〜801`(`if (MYW == 0) fle_input_ring_poll()`)、`harness/fruity_idle.c:43` |
| scheduler 点の頻度 | 約 2,000/s(0.5 ms 間隔。gpu1) | UI スレッド 1 本の待ち/切り替えの点 + idle(present の間は進まない) | 0306 §11 |
| idle の起こし方 | `wake`(parked のとき) | 同じ。idle の待ちは `Atomics.waitAsync` なので event loop が回る | `client.ts:102〜107`、`worker.ts:59,119,121`、`fle_thread_wasm.c:135〜163` |
| ring を読めない区間 | 短い(present は同じ Worker だが 1 本の Worker が 0.5 ms ごとに戻る) | **present の JS 12〜14 ms**(`onIdle` の `deskTick()`)と UI スレッドの wasm 計算 | 0306 §11 の B の分布 |

- idle の `onIdle` は present の後に `drainRing()` する(`worker.ts` の `onIdle` の末尾のコメント『an input that was written after the first look but before this idle was announced as parked』)ので、present の最中に着いた入力は **present の残り**だけ待つ = 一様分布の理由〔読み〕。
- **S2-B1(本線)= P(0311)**: present を Worker 0 の idle から外す。B の大半が消える〔gpu1 の読み〕。S2 の側で作る物は無い。
- **S2-B2(P の後に B の中央値が 1.5 ms を超えるときだけ)**: ring を **メッセージキューの問い合わせの入口**でも読む。根拠は Windows の契約: 入力は OS が(アプリの CPU が計算中でも)スレッドのメッセージキューに積み、アプリは `PeekMessage`/`GetMessage`/`GetQueueStatus`/`MsgWaitForMultipleObjects*`/`GetInputState` で見る。いまの実装は『scheduler 点で ring → キュー』で、UI スレッドが計算中は積まれない。問い合わせの入口で `fle_input_ring_poll()` を呼ぶ(Worker 0、GIL 保持下、既存の関数)のは**契約の実装**で、class/アドレスの分岐も旗も要らない。新しい処理の区分: (b)。**効果の見積りは持っていない**(P の後の B の分布を測ってから)。
- **『`Atomics.notify` で起こす』の位置づけ**: S1 の後は page が engine の共有メモリの SAB(`audio-ring` の `sab`)を持つので、page から Worker 0 の `wake_seq_` に `Atomics.notify` できる(`fle_thread.c:172`)。ただし event loop を回す待ちは postMessage でも同じ程度に即応するので、**得るのは 1 hop の差**〔読み。測定なし〕。やらない案。

## 2. C: +5 ms の内訳の仮説(0306 §11 を引き継ぐ。順位は私の読み)

差は File(21.9 対 16.9)と Esc(7.6 対 3.1)で**同じ約 +5 ms** = 処理量に比例しない入力ごとの固定の追加(0306 の観察)。

| # | 仮説 | コードの根拠 | 反証/確認 |
|---|---|---|---|
| C1 | **UI スレッドが GIL を取るとき、mixer が持っていると spin 2,000 回(約 20 µs〔読み〕)で眠り、起床に数十〜100 µs**。1 入力の処理で『持ち主が違う』取り直しが数十回あれば +数 ms | GIL は『guest が runtime 関数を呼ぶたび』に取る(`fle_gil.h:1〜11` の不変条件、生成される `fle_gil_wrap.c`)。取れなければ `fle_gil.c:71` の spin → `memory.atomic.wait32`(`:90`) | `-DFLE_GIL_STATS`: Worker ごとの『slow acquire の数・待ち時間の合計・ヒストグラム(<10 µs/<100 µs/<1 ms/<10 ms)』。File クリックの 35 回の区間の差分で、UI Worker の slow acquire 数 × 平均待ち が約 5 ms/入力か |
| C2 | mixer の GIL の保持が長い(heavy window の Unlock、`fle_tp_publish` が GIL 下で 100 frames を書く〔S1 で JS から C へ移り、1 公開あたりの複写は短くなった〕) | `FLE_CS_HOLD_STATS` の『import ごとの hold 時間 top10』(`fle_gil_wrap_report`) | hold の max/平均を import 別に |
| C3 | 『答えの present の機会』(`INLINE_TRY_MS = 6 ms` ごと、`worker.ts` の `answerInline`)の待ち。B と同じ原因で settled の検出が遅れる | 0306 §11 C の 1 | P が直す(present が別 Worker になると `answerInline` の待ちが変わる) |
| C4 | TLS・Worker 間の同期の固定費 | `FLE_WORKERS=1` の腕は協調と同じだった(0289)ので、コードの費用ではなく待ち | C1 で説明がつくか |

**梃子の案(C1 が主だった場合。いずれも『契約の実装/除去/通信の削減』の区分で、優先度の特別扱い〔UI スレッドだけ GIL を優先する〕は作らない)**:
- **L1: lock-free の import を増やす**(`lift/harness/gil/gen_wrap.py:7〜17` の `lockfree` 集合、`runtime/fle_gil_lockfree.inc`〔現在 13 import〕、docs 0234q: 『他の Worker が書き得る状態を読み書きしない import は GIL を取らない』)。`FLE_CS_HOLD_STATS` の top10(保持回数 × 時間)から、**状態に触れない呼び出しを足す**。区分: (c) 通信/待ちの削減。契約は変えない。
- **L2: mixer が runtime に入る回数を減らす**: S1 で `fle_tp_publish` は C に移ったが、`audio_sync` が全 scheduler 点から呼ばれる構造(0305c)は残る。毎 scheduler 点の `fle_audio_poll` を、publish が要るときだけに絞れるなら GIL の保持回数が減る〔読み。`fle_thread.c:804` `poll_waiters` → `:482` `fle_audio_poll`、未検討〕。
- **L3: spin の長さ**(2,000)は定数の調整であって契約の実装ではない。**測らずには変えない**(区分に当たらない)。

## 3. 測る道具(D3a の `patch -R`)

- D3a(`0305-d3-remove-instrumentation.diff`)を **逆向きに当てる**と、`-DFLE_GIL_STATS`(Worker ごとの保持/待ちの合計と最大、待ちのヒストグラム)と `-DFLE_CS_HOLD_STATS`(import ごとの保持時間、`fle_gil_wrap_report`)の計器が戻る。**S0+S1 の木に対し `patch --batch -R --dry-run -p1` が clean**(実測: 7 ファイル、`fle_thread.c` の 4 hunk が offset 2 行で成功、reject 0。`tables/0312c-d3a-reverse-dry-run.txt`)。
- 使い方: 測定用の使い捨ての木で `patch -R -p1 < 0305-d3-remove-instrumentation.diff`、gpu1 の `lift/out-gpu2/tools/mk-rts.sh`(`SPIKE_CFLAGS="-DFLE_GIL_STATS -DFLE_CS_HOLD_STATS"` 付き。0289 で使用)の形で build。報告は `FLE_DBG_REPORT_S=n`(manifest の `engine.env`)で **stderr に n 秒ごと**に出る(`fle_thread.c:751` `dbg_report_maybe`、累積値なので **File クリックの区間の前後の差**を取る)。engine の stderr は page の `engineLog` に来る〔既存の経路〕。
- **製品には入れない**(方針 D3: 測定 build は製品の木から除く)。測定用の木は使い捨て。

## 4. S2 の範囲の案(P(E14)が B/C を同時に直す見込みなので)

1. **まず P だけで測る**(S0+S1+P の bundle を gpu1 が 1 本。B1 の後)。T・B・C の分解(page の `latency` 記録)。
2. **P の後に残る B/C だけを S2 にする**:
   - 残る B の中央値 ≤ 1.5 ms → S2-B は不要。> 1.5 ms → B2(問い合わせの入口で ring を読む)。
   - 残る C(協調との差)≤ 2 ms → 受け入れ(協調 + 2 ms 以内)に収まる。> 2 ms → §3 の計器の build を 1 本(約 20 分の cold build + 5 分 × 数本の測定)で C1/C2 を決め、L1(lock-free の拡大)、次に L2。
3. 工数の読み: B2 約 0.5 人日(関数 1 つの呼び出し点を 5 つほど)、L1 約 1 人日(top10 の安全性の確認が主)、L2 約 1 人日。いずれも『測ってから』。

## 5. I15 の静的な先行点検(n = 2 / 4 / 8 / 32。**測定は gpu1 の B1 の後。ここは読みだけ**)

結論: **範囲外の添字になる箇所は見つからなかった**(配列は `FLE_MAXW = 32` で足りる)。n ≥ 8 で心配なのは**添字ではなく性能の壁**(§5b)。

### 5a. 配列・添字(file:line)

| 箇所 | 大きさ | n ≤ 32 での判断 |
|---|---|---|
| `fle_thread.c:162` `FLE_MAXW 32u`、`:164` `thr T[FLE_THREAD_MAX + FLE_MAXW]`(256 + 32)、`:169` `wk_started[FLE_MAXW]`、`:172` `wake_seq_[FLE_MAXW * 16]` | 32 | `:343` で `nworkers_cfg` は 32 に頭打ち。ホストの論理 CPU が 64 でも 32 |
| `BOOTW(w) = FLE_THREAD_MAX + w`(`:163`)の枠 `T[BOOTW(w)]`(`:1203〜1208`)、`cur = BOOTW(w)`(`:1158`) | 256..287 | `T` は足りる。`fle_audio_switch_note` の呼び出しは `cur < FLE_THREAD_MAX`(`:529`)、`fle_thread_current` 系は `c >= FLE_THREAD_MAX` で 0(`:564`)。`fle_thread_wasm.c:50,97` の `idx >= FLE_THREAD_MAX` の guard で `region[]`(256)の範囲外を読まない |
| `pick_worker`(`:1168〜`)の `live[FLE_MAXW]` と 3 つの loop | 32 | `w < nworkers_cfg`(≤ 32)。**飢餓**: Worker 0 は最初のスレッド専用、他は『live が 0 の起動済みの Worker → まだ起動していない最小の Worker → live が最小(同数なら番号の小さい方)』。FL のスレッド数が n−1 を超えると、**超えた分は同じ Worker で協調に交代**(`pick_worker` の前のコメント: 『他のスレッドが持つ lock への spin は slice が終わるまで抜けられない』)。n = 2 は mixer 他の全スレッドが Worker 1 に同居 = これが E11 の構成。n を増やすほど改善する方向だが、**どのスレッドがどの Worker に行くかは作成順**なので mixer と重い UI 補助スレッドが同居する可能性は残る〔読み。測定で確認〕 |
| `fle_gil.c`: `gil_key[32]`、`me = fle_worker_id & 31`(`:53`)、他の Worker の key を見る `for (w < 32)`(`:69,80`) | 32 | n ≤ 32 で足りる。**1 回の slow acquire の中に 32 回の atomic load の走査**(spin の中の 64 回に 1 回、`:78〜80`)= n の増加でも定数 |
| `fle_audio_transport.c`(S1): `sl_total[SL_MAX = 64]`、`fle_js_pulse` は `n = min(nthr, 32)` | 64 / 32 | 範囲内。thread index ≥ 32 の slice は pulse に出ない(集計は SL_MAX まで)。表示だけの欠け |
| `tp_ulog[4000]`、`tp_stat[8]` | — | n に依らない |

### 5b. n を増やしたときの性能の壁の候補(**読み。測定は無い**)

1. **GIL の競合は n に比例して悪化しうる**: guest が runtime 関数を呼ぶたびに GIL(§2 C1)。Worker が awake で spin する数が増えると、`best` の判定(`:69,80`)と spin(2,000 回)の CPU が O(待っている Worker の数)で増える。n が物理コア数を超えると、spin している Worker が GIL を持つ Worker の CPU を奪う(ホストの論理 CPU より多くは作らない = D2 の方針で既に頭打ち)。
2. **Worker の起動**: `-sPTHREAD_POOL_SIZE=0`(`harness/web/fruity/link-web.sh:23`)なので `pthread_create` は Worker 0 の event loop で実際の Worker を作る(`wk_spawn`、`fle_thread.c:1187〜1195`)。n = 32 の最悪は FL が起動中に 31 本の Worker を順に起こす = **起動 P1(10 s 以内)への影響**〔読み。Worker 1 本の起動費の実測がまだ無い〕。`pick_worker` は『まだ起動していない最小の Worker』を優先するので、スレッドが作られる都度 Worker が増える。**『host CPU 数 = n』を上限にするだけでなく、起動の遅延の測定(n = 2/4/8 の first idle)が要る**。
3. **メモリ**: Worker の C スタックは `pthread_attr_setstacksize(512 KiB)`(`fle_thread.c:1193`)で、PM の『shadow stack 8 MiB × n』は**当たらない**(8 MB の `-sSTACK_SIZE` は主スレッドだけ。`link-web.sh:18`)。guest スレッドの shadow stack は `fle_thread_wasm.c` の `region[]`(malloc、thread 数に比例、Worker 数には依らない)。共有メモリは初期 640 MiB・最大 2 GiB(`link-web.sh:23`)で、Worker を増やしても共有。**増えるのは Worker ごとの V8 isolate + JS ヒープ**(n = 32 で数百 MB〔読み。Chrome の Worker 1 本の常駐は数十 MB の見積り〕)。
4. **時計・待ち**: `fle_thread_wasm_idle_wait` は Worker 0 が `Atomics.waitAsync`、他は `memory.atomic.wait32`(`fle_thread_wasm.c:155〜167`)。Worker が n 本あっても各 1 本の wait で、**起こす側は `wake_seq_[w * 16]` を bump + notify**(`fle_thread.c:172`)= n に比例する走査は無い。
5. **n = 4 は 0234s で最後まで走った**(0305b の引用)。**n = 8 / 32 は未実行**(下の測定)。

### 5c. 測定の計画(gpu1 の B1 の後、manifest の `engine.env` の `FLE_WORKERS` だけ変える。再 build 不要)

- n = 2 / 4 / 8 / 16 / 32 で: ① FL が最後まで起動する(first idle の到達と P1)、② 5 分再生の PCM の零 chunk(S1 の共有 ring)、③ UI Worker・CPU/Unlock、④ stop/trap の有無(`fle_stop` の名前)。
- 見るべきログ: `stopReport`(`fle_gil` の assert、`scheduler`)、`W%u has waited 3 s for the lock`(`fle_gil.c:92`)。

## 6. 提案(PM の決定が要る点)

1. S2 は P の後に決める(B1 の測定 → S0+S1+P の bundle の測定)。それまで S2 のコードは作らない(B2・L1・L2 は測定値次第)。
2. `-R` の D3a は『測定用の木』専用とし、製品の積み重ね(`0312-stack-on-main-e08b4589.diff`)には入れない。
3. I15 は B1 の測定の中で n = 2/4/8 をまず取り、16/32 は 8 が通ったときに。


## PM の判断と取り込み(2026-11-06 22:32、docs のみ)

取り込み: mt3 の最終(0312 の一部)として `docs/investigations/mt3w/0312/` の表・道具・差分とともに置いた。判断は決定 0312 の PM の節にまとめた(段階 C は採る、T の基準が通るまで実スレッドは main に出さない、P を先に、S0+S1+P の bundle の測定、Worker 数の式は「区分に当たらない選択」として測定用の既定)。**ブラウザでの実行は 0 件、依頼者の機械は 0 件**。
