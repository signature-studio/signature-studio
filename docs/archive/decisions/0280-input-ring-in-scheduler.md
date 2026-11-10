# 決定 0280(仮): 入力リングをエンジンの内側(スケジューラの点)でも読む — 再生中の入力が『エンジンの idle』を待たない(試作。0269 の L1)

状態: **仮の決定・試作(出荷しない)**。作業者 `gpu1`(成果物名は gpu7)、2026-11-06 JST 01:00〜(サーバー再起動をはさんで 08:20〜)。基準: HEAD **d6926b4d**(`gpu7-L1.diff` は `git archive d6926b4d` の該当ファイルに `patch -p1 --dry-run` が通る)。区分: **実測**=このセッションのコマンドの出力を見た事実、**読み**=コード・文書・測定の解釈、**未確認**。**出典なしの選択**=`S-gpu7-N`(§8)。**この機械(16 vCPU の Zen 5 VM、実 GPU なし、headless Chrome 156)の値で、依頼者の機械の値は 0 件。**

## 冒頭の三点

1. **何を実行したか**: (a) C のテスト(native、本物の runtime〔`fle_thread.c`、`fle_input.c`、`fle_win.c` ほか 29 個〕+ スタブのゲスト関数。`harness/gpu7/ring_poll.c`、`tests/gpu7_ring_poll.rs`: L1 v3 の木で 4 本、L1c の木で 6 本 + 既存 `wall_fr9t` 18 本)、(b) app のテスト(**積んだ木 = e8-wip a1e96d92 + `gpu7-L1-LC.diff`**: typecheck rc=0、vitest 1,310 passed + 7 skipped、test:worker 49、test:e2e 229/229〔1 回目は load 30 前後で 175/177、taskbar の Tab / Enter の timeout 2 件〕)、(c) 試作を入れた bundle の増分ビルド(`ciL` = L1、`ciLC` = L1 + 答えの規則)、(d) **実 Chrome 156 headless で実エンジンを動かした入力遅延 T の測定**: 低 load の交互 3 往復(§6.1。T の列は下限)、**同時 2 Chrome の対 2 回(本番 ff5ee429 対 測定専用ページ。真の T)**(§6.3)、idle の交互 3 往復(§6.4)、L1c の対(§6.5)。**代役・再生なし。ただし測定専用ページ `srcLB`(答えの条件にクラス名を含む)は製品ではなく、`gpu7-L1*.diff` に入っていない。**
2. **結論**: 下の『結論』。
3. **未確認**: 依頼者の機械・実 GPU の値(0 件)。実エンジンでの node 84/55/87 行と a4(PM が回す)。Firefox。長時間(5 分以上)の再生での安定。入力の種類(ホイール・ドラッグ・複数ボタン)の網羅。高 load(15 以上)での試作(同時 2 Chrome で load 10〜15 までは測った)。UI が別スレッドに描画を非同期に任せる経路の有無(L1c の限界)。

## 結論

1. **L1(スケジューラの点で入力 ring を読む)は、再生中の入力の待ち B を取る**: 同時 2 Chrome の対 2 回(load 10〜15)で、**File クリックの真の T の中央値 201 → 30 ms(p90 401 → 40)、Esc 86 → 10 ms**、B の中央値 122 → 3.8 ms、書き込み率は同じ(§6.3)。製品の答えの規則 **L1c(settled)** の実エンジンでの対(load 17〜33): File クリック 275 → 36 ms、Esc 176 → 12 ms、物理的に速すぎる答え 0、答えなし 0、書き込み率は同じ(§6.5)。**P2 の再生中の入力遅延に対する最大の梃子**で、残りの C(UI の処理)は変わらない(P3 の課金・CPU の側。0281)。
2. **最初に出した『File クリックの T 59 → 17 ms』は撤回する**: 試作の答えの帰属の誤り(再生ヘッドなど別の変化の提示を答えにした)で下限にしかならなかった。最小 T 1.7 ms が idle の最小 5.7 ms より速いことが証拠(§6.2)。有効なのは B の短縮(122 → 3.8 ms)と書き込み率の不変。
3. **差分は 2 つ**: `gpu7-L1.diff`(L1 v3。T が下限になる答えの規則のまま)と `gpu7-L1-LC.diff`(L1 v3 + L1c = 製品の答えの規則)。**L1c を載せる**のを勧める。どちらも e8-wip a1e96d92・d6926b4d・main 93e12685 に `patch -p1 --batch --dry-run` rc=0。`-` の行は `ring.ts` の 2 行だけ。**E9(E8 + L1c + 課金の見直し)に載せる前提**(PM 10:30)。
4. **決定性**: 入力のない走行では、scheduler の点の poll と settled の語の書き込みは何も変えない(C のテスト 6: 仮想時計・スレッドの順が一致)。入力のあるとき、新経路は仮想時計を進めずに積む(§3)ので、入力の仮想時刻が idle 経路より早くなりうる。node の 84/55/87 行・a4 は PM が回す。
5. **未確認**: 依頼者の機械・実 GPU(0 件)、高 load(15 以上)の試作の再現、他の入力(ドラッグ・ホイール・複数ボタン)、長時間の再生、Firefox、syscmd の復元ボタンの実エンジンでの動作、UI が別スレッドに描画を非同期に任せる経路(L1c の限界)。

## 1. 何を変えたか(ファイル別)

| ファイル | 変更 | 行数 |
|---|---|---|
| `lift/runtime/fle_thread.c` | `schedule_ex()` のループの先頭(`poll_waiters()` の前)で、弱い関数 `fle_input_ring_poll()` があれば呼ぶ。wasm ビルド(`__EMSCRIPTEN__`)か、テストの `-DFLE_RING_POLL_HOOK` のときだけ(native の `fle_thread.o` は不変: `wall_fr9t.rs` の同一性テストが通る) | +8 |
| `lift/harness/fruity_idle.c` | `fle_input_ring_poll()`: JS の import `fle_js_input_poll(buf, 16)` でページのリングの記録を最大 16 件、wasm のメモリ上の固定バッファ(8 語 × 16)に写させ、**runtime が** `fle_input_host_mouse` / `fle_input_host_key` / `fle_input_host_display`(idle 経路の export `fle_g3_*` と同じ関数)で `evq` に積み、結果(1 = 積んだ、0 = 断った)を語 6 に書き、`fle_js_input_done` で Worker に知らせる | +30 |
| `lift/harness/web/fruity/fle_idle_lib.js` | 同期の import 2 つ(`fle_js_input_poll`、`fle_js_input_done`)。`Module.fleInputPoll` / `fleInputDone` を呼ぶだけ(方針なし) | +6 |
| `app/src/input/ring.ts` | `RingReader.drainN(max, fn)`(最大 max 件。`drain` は `drainN(Infinity)`) | +6 |
| `app/src/engine/idle-driver.ts` | `injectedInline(done)`(idle 経路の `apply()` が付ける帳簿と同じ: `stats.injected`、`awaiting`、`hooks.injected`。仮想時計は動かさない)、`takeAwaiting()` | +14 |
| `app/src/engine/fruity.ts` | `fleInputPoll` / `fleInputDone` を Module のオプションに(host が持つときだけ) | +6 |
| `app/src/engine/worker.ts` | `inputPoll`(リングから最大 16 件を staging へ。エンジンが最初の idle に達するまでは何もしない = 従来どおり idle で捨てられる)、`inputDone`(結果を帳簿へ)、`answerInline`(積んだ入力の答えの提示を 6 ms ごとに試し、画素が変わったら `input-done` と `desk` の `answers` を送る。150 ms 変わらなければ答えなしで終える) | +60 |
| テスト | `harness/gpu7/ring_poll.c`、`tests/gpu7_ring_poll.rs`(L1 v3: 4 本)、`input-ring.test.ts`(+1)、`idle-driver.test.ts`(+1) | |

**差分は 2 つ**(どちらも d6926b4d と、PM の e8-wip a1e96d92 の該当ファイルに `patch -p1 --batch --dry-run` が通る。`-` の行は `app/src/input/ring.ts` の 2 行だけ〔`drain(fn: ...) {` を `drainN` に置き換える行と、tail の更新と `fn(rec)` の順序を入れ替える `fn(rec);` の行〕。ほかはすべて追加):
- **`gpu7-L1.diff`(L1 v3)**: 上の表。v2 からの変更は (a) 『注入のとき `awaiting` が空なら `inlineSince` を今にする』の 1 行(§6.2 の欠陥)、(b) **mouse / key / display 以外の kind(task bar の syscmd)を ring に残す**(§6.5 の最後。`drainN` が `fn` の `false` で止まる)。答えの規則は v2 と同じ(『注入のあとの最初の画素が変わる提示』)なので、**T の測定で下限になる**(§6.2)。
- **`gpu7-L1-LC.diff`(L1 v3 + L1c)**: 上に、§6.5 の答えの規則(`fle_input_settled()`、`fle_thread_in_message_wait()`、settled の 1 語、`worker.ts` の旗)を足した全体。`gpu7-LC-over-L1.diff` は『L1 v3 の上に足す分』だけ。

## 2. 入力の流れの表(誰が、いつ、何に書き、誰が読むか)

| 場所 | 書く者・時 | 読む者・時 | 同期 |
|---|---|---|---|
| ページのリング(SharedArrayBuffer、`ring.ts`) | ページのメインスレッド: `RingWriter.push`(記録を書いてから `head` を Atomics で更新)。そのあと `parked` を読み、1 なら `wake` を 1 通 `postMessage` | Worker の JS: `RingReader.drain` / `drainN`(`tail` を更新)。(i) idle 経路: `onEngineIdle` → `drainRing()`、`wake` の受信。**(ii) 新: `inputPoll`(エンジンが走っている間、スケジューラの点で)** | `head` / `tail` / `parked` は Atomics(Dekker 規約: 書き手は記録 → `head` → `parked` の順、読み手は `parked` → `head` の順)。(i) と (ii) は**同じ OS スレッド**(Worker)の別の時刻に走る: (i) はエンジンが `fle_idle_wait` で止まっているとき、(ii) はエンジンが `schedule_ex` の中にいるとき。同時には走らない = リングの読み手は 1 人のまま |
| `parked` の旗 | Worker: idle で 1(`hooks.idle`)、再開で 0(`hooks.resumed`) | ページ: `push` のあと | 従来どおり。(ii) は旗に触れない: エンジンが走っている間は 0 のままで、ページは `wake` を送らず、(ii) が次のスケジューラの点で拾う。エンジンが idle に入るとき、`drainRing()` が『最初の確認のあとで書かれて、`parked` の前に』と言う 2 回目の読みで拾う(従来) |
| staging バッファ(wasm メモリ、`fruity_idle.c` の静的配列 8 語 × 16) | JS `inputPoll`(記録を写す)、C(語 6 に結果) | C(記録を読む)、JS `inputDone`(結果を読む) | 1 回の import 呼び出しの前後に直列(C → JS → C → JS)。同じスレッド |
| `evq`(`fle_input.c` の静的リング 512) | `fle_input_host_*`: (i) idle 経路の export `fle_g3_*`(JS が `IdleDriver.apply` から呼ぶ)、**(ii) 新: `fle_input_ring_poll`(C が `schedule_ex` の先頭から呼ぶ)** | ゲストのスレッドの runtime 関数: `fle_input_synth`(`PeekMessage` / `GetMessage` / `WaitMessage` の中)、`fle_input_has_event`(`poll_waiters` の起こす判定) | 単一の wasm スレッドで協調実行。スレッドの切替は『待つ・譲る関数の中』と `fle_progress_slice` だけ(`fle_thread.c` の規約)なので、(ii) の書き込みは**どのスレッドの runtime 関数の途中でもない**(`schedule_ex` の先頭 = 切替の判断の点。idle フックと同じ文脈)。ロック不要 |
| settled の語(wasm メモリ、record 領域の直後の 1 語。**L1c だけ**) | C: `fle_input_ring_poll()` が scheduler の点ごとに `fle_input_settled()` を書く | JS `inputPoll`(`Int32Array` で読むだけ。wasm の export は呼ばない)、`deskTick`(提示の前に旗を読む) | 同じ OS スレッド(Worker)で、import の呼び出しの前後に直列。ロック不要 |
| `IdleDriver.awaiting`(JS) | idle 経路の `apply()`、**(ii) 新: `injectedInline`** | idle の `processed`、**新: `answerInline`**(`takeAwaiting`) | 同じ JS スレッド |

## 3. 仮想時刻の付け方(PM の条件 (b))

- idle 経路: `IdleDriver.apply()` の `moveClock()` が、**エンジンが待っていた**実時間ぶんだけ仮想時計を進めてから(`idle` の `next` を越えない、目標は `target()`: 遅れの上限 `maxLagMs` / 先行の上限 `maxLeadMs` で切る)入力を積む。入力の時刻 `e->ticks` は積んだ瞬間の仮想時計(`clk()`)。
- 新(スケジューラの点): **仮想時計を動かさない**。エンジンは待っておらず走っている(仮想時計はエンジン自身の実行で進む: 1 スライス = 仮想 1 ms、`Sleep` の待ち)ので、『待っていた実時間』は存在しない。入力の `e->ticks` は積んだ瞬間のエンジンの仮想時計(`fle_input_host_*` が `clk()` を取る、idle 経路と同じ式)。= 実時間に歩調を合わせる役目は従来どおり idle 経路が持ち、スケジューラの点は仮想時計に触れない(『仮想時間は飛ばさない』の方針のまま)。`idle` の `next` を越える心配はない(時計を動かさないので)。
- テスト: `an_input_queued_by_the_hook_reaches_...` が『フックによる積み込みで仮想時計は 0 tick しか動かない』を確かめる。入力どうしの順序は FIFO(`evq`)で、idle 経路と新経路が混ざっても到着順(リングの順)。
- 帰結(読み): idle 経路は入力の前に仮想時計を実時間の目標まで(次の期限を越えない範囲で)進めていた。新経路は進めないので、エンジンが実時間より遅れている間は、入力の仮想時刻が従来より**最大でその進め分だけ早く**なりうる(FL が入力どうしの時間差を見るのは仮想時間なので、入力どうしの整合は保たれる。タイマーとの相対関係は変わりうる)。= 差が出うる点として明記する(実エンジンで 84/55/87 行と a4 は入力を使わない行が不変であることを PM が確かめる)。

## 4. エンジンの実行中に入力を積む再入の安全性(PM の条件 (a)(d))

- **wasm の export を JS から呼ばない**: staging 経由で C が自分で `fle_input_host_*` を呼ぶ。JS は `Atomics` / 配列の読み書きと帳簿だけ(再入なし)。例外は `answerInline` → `deskTick` → `fle_desk_scene()` / `fle_desk_compose()` の export 呼び出しで、これは idle 経路でも同じ呼び出しをしている(提示)。スケジューラの点で呼ぶ点が新しい: `schedule_ex` の先頭 = idle フック(`fle_thread_idle_hook` → `fle_idle_wait` → Worker の JS が `deskTick`)と同じ文脈(ゲストのスレッドは全員、待つか譲る関数の中にいる)。
- **注入点は `schedule_ex` の先頭だけ**(`fle_thread.c` の『切替の判断の点』)。`fle_progress_slice` の『コールバックの深さ > 1 ではスキップ』の規則は `schedule_ex` を通る前の話で、注入点にはかからない(待つ・譲る関数の中は常に切替の点)。runtime の関数の途中で注入することはない。
- 呼び出しの頻度: スケジューラの点ごと(再生中に約 2,000 回/s)。1 回は JS の import 1 回 + `Atomics.load` 2 回(リングが空なら即 0 を返す)で、約 0.1〜0.3 µs の見込み。

## 5. テスト

| テスト | 内容 | 結果(実測) |
|---|---|---|
| `gpu7_ring_poll.rs` 1 | フックはスケジューラの点でだけ呼ばれる: `CreateThread` の前は 0 回、ミキサー役のスレッド(`Sleep(1)` を 6 回)が走る間は少なくとも 1 切替に 1 回、スレッドが 2 つ未満のときの呼び出しは 0 | passed |
| 2 | フックが `evq` に積んだ入力(visible な窓 300×300 の (100,100) への左ボタン押下)が、`GetMessageW` で待つ『UI スレッド』に**同じ切替で**届く(`WM_LBUTTONDOWN` 0x201、wParam 1)。ミキサー役の `Sleep(1)` ループの 0 周後(idle 経路なら 20 周)。積み込みで仮想時計は 0 tick 動く | passed |
| 3 | `fruity_idle.c` の `fle_input_ring_poll()`(`RING-POLL-BEGIN` / `END` のマーカーで切り出し、偽の import `fle_js_input_poll` / `done` でコンパイル)に 14 件の記録(移動・左右ボタン・キー〔自動リピートを含む〕・範囲外の座標・不正な mouse kind・vk 0・display 2 件〔1920×1080 は積まれ、10×10 は断られる〕・ホイール)を渡した後の状態(`pending` 件数、ポインタ、`GetAsyncKeyState` の 4 キー)が、同じ列を `fle_input_host_*` で直接積んだ状態と**一致**。11 件が積まれ 3 件を関数自身が断る。各記録の結果コードも同じ(`done_n 14 queued 11`) | passed |
| 4 | `fle_thread.c` のフックの宣言と呼び出しは `#if defined(__EMSCRIPTEN__) \|\| defined(FLE_RING_POLL_HOOK)` の中で、呼び出しは `poll_waiters()` の直前 | passed |
| 5(**L1c**) | `fle_input_settled()`: 注入前 1、注入の直後 0、UI スレッドがメッセージを持っている間 0、UI が `GetMessageW` に戻ると 1(`gpu7-L1-LC.diff`) | passed |
| 6(**L1c**) | **入力のないときのフックは何も変えない**: 同じシナリオ(`hook_counts`)を、`fle_thread.c` をフックのマクロありとなしでビルドした 2 つの実行ファイルで走らせ、最後の仮想時計(ms)とミキサー役の反復の値が**一致**(= node の 84 / 55 / 87 行のような『入力のない走行』は、scheduler の点の poll と settled の語の書き込みで変わらない、の native での確認) | passed |
| 既存 `wall_fr9t.rs`(18 本) | native の `fle_thread.o` が『フックなしの木』と同一、38 シナリオ、wasm の切替の規約、`the_js_transport_holds_no_scheduling_policy` ほか | 18 passed |
| app `vitest` | `input-ring.test.ts`(+`drainN`)、`idle-driver.test.ts`(+`injectedInline`)を含む全体 | 66 files passed + 1 skipped、1,293 passed + 7 skipped(基準 1,290) |
| **積んだ木(e8-wip a1e96d92 + `gpu7-L1-LC.diff` の写し)** | C: `gpu7_ring_poll` 6 + `wall_fr9t` 18 | 6 passed + 18 passed |
| | app: `typecheck` / `vitest` / `test:worker` / `test:e2e` | rc=0 / **1,310 passed + 7 skipped** / 49 passed / **229/229**(1 回目は 175/177: load 30 前後の窓で `taskbar-restore` の Tab / Enter の waitFor が 2 件 timeout) |
| gpu7_ring_poll.rs 合計(L1c の木) | 6 本 | 6 passed(L1 v3 の木では 4 本 passed)|
| 既存 `wall_fr9t.rs`(L1c の木) | | 18 passed |
| app(L1c の木 `srcLC/app`、emcc を PATH に) | `vitest` | 66 files passed + 1 skipped、**1,293 passed + 7 skipped**(L1 v2 と同じ。syscmd の修正と drainN の 1 本の追加の前の値。積んだ木の最終値は次の行) |
| | `typecheck` | **rc=0**(最初の 1 回は `worker.ts` の `noUncheckedIndexedAccess` で rc=1 だった。`?? 0` の 1 語で直した。e2e はこの修正の前の木で走った: 意味は同じ) |
| | `test:worker` | 49 passed |
| | `test:e2e` | **190/190** |

注意: `GetKeyState` / `GetAsyncKeyState` の整合は『直接積んだ状態との一致』(テスト 3)で見ている。`GetKeyState`(メッセージ処理済みの状態)は `evq` を取り出す側の挙動で、新経路は取り出しに触れない。

## 6. 測定

> **訂正(09:42 JST、PM に報告済み)**: 最初に出した『File クリックの T 59 → 17 ms(−71%)』は、**試作の T が下限にしかならない**ので、P2 の結果として使わない。理由は §6.2。**有効なのは B(エンジンに渡るまで)の短縮と書き込み率の不変**(§6.1)。T の真の値は §6.3(測定専用ページ)、答えの帰属の規則の案は §6.5。

### 6.1 交互 3 往復(場面 b の再生中、File クリックと Esc 各 35 回 × 3、load 4〜12)— B と書き込み率は有効、試作の T は下限

**方法**: 同じ場面(`tools/scn-latb.json`: 参照プロジェクトを開き、Space で再生、10 s 待ってから File ボタンのクリック + Esc を 35 回。`tools/run-alt.sh 3 4990 4992`)を、**ベースライン**(本番の bundle ff5ee429〔配信バケットから読み取り〕+ 本番のページ HEAD 91cbd9b0)と**試作**(L1 入りの bundle `ciL`〔wasm 142,381,342 B、本番より +416 B〕+ 試作のページ)で、**交互に 3 往復**(base-1, proto-1, base-2, proto-2, base-3, proto-3)。各 run の始めと終わりの `loadavg`(1 分)を併記。サーバー再起動の直後で load が低い窓(4〜12)で取った。T = ページが送ってから答えの画の提示が終わるまで(0269 と同じ定義)。**試作の `tIdle` は『エンジンが idle に戻った時刻』でなく『答えの提示の時刻』**なので、試作の C は『注入から答えの提示まで』(FL の処理 + 提示の試行の間隔 6 ms)で、ベースラインの C(注入から idle まで)と定義が違う。T と B が比べられる量。

| round | arm | load (start→end) | write rate | input | n | **T median / p90 / p99** | B median / p90 | C median / p90 | D median | mean T | mean B | mean C |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | base | 9.48→4.56 | 0.413 | click | 35 | **69 / 156 / 188** | 16 / 116 | 16 / 21 | 0.2 | 72 | 44 | 28 |
| 1 | base | 9.48→4.56 | 0.413 | esc | 35 | **8 / 97 / 134** | 5 / 95 | 2 / 3 | 0.2 | 28 | 25 | 3 |
| 1 | proto | 4.56→5.55 | 0.410 | click | 35 | **17 / 33 / 56** | 0 / 9 | 1 / 17 | -0.0 | 21 | 3 | 7 |
| 1 | proto | 4.56→5.55 | 0.410 | esc | 15 | **4 / 6 / 7** | 0 / 3 | 3 / 4 | -0.1 | 4 | 1 | 3 |
| 2 | base | 5.55→4.29 | 0.414 | click | 35 | **59 / 159 / 224** | 36 / 142 | 16 / 18 | 0.2 | 72 | 52 | 20 |
| 2 | base | 5.55→4.29 | 0.414 | esc | 35 | **14 / 86 / 145** | 12 / 83 | 2 / 3 | 0.1 | 31 | 28 | 3 |
| 2 | proto | 4.29→4.20 | 0.410 | click | 35 | **17 / 27 / 53** | 1 / 6 | 16 / 22 | -0.0 | 17 | 2 | 14 |
| 2 | proto | 4.29→4.20 | 0.410 | esc | 35 | **3 / 7 / 45** | 0 / 3 | 3 / 4 | -0.0 | 5 | 1 | 4 |
| 3 | base | 4.20→12.06 | 0.415 | click | 35 | **38 / 149 / 194** | 16 / 133 | 17 / 28 | -0.1 | 71 | 44 | 27 |
| 3 | base | 4.20→12.06 | 0.415 | esc | 35 | **8 / 98 / 116** | 6 / 96 | 2 / 3 | 0.1 | 34 | 31 | 3 |
| 3 | proto | 12.06→7.21 | 0.405 | click | 35 | **12 / 25 / 53** | 3 / 7 | 4 / 23 | -0.0 | 14 | 3 | 11 |
| 3 | proto | 12.06→7.21 | 0.405 | esc | 33 | **4 / 9 / 19** | 0 / 6 | 3 / 5 | -0.0 | 6 | 2 | 3 |

pooled over the rounds:

| arm | input | n | T median / p90 / p99 | B median / mean | C median / mean | D median | mean T |
|---|---|---|---|---|---|---|---|
| base | click | 105 | **59 / 155 / 194** | 17 / 47 | 16 / 25 | 0.1 | 72 |
| base | esc | 105 | **9 / 97 / 134** | 6 / 28 | 2 / 3 | 0.1 | 31 |
| proto | click | 105 | **17 / 27 / 54** | 1 / 3 | 4 / 11 | -0.0 | 17 |
| proto | esc | 83 | **4 / 7 / 45** | 0 / 2 | 3 / 3 | -0.0 | 5 |

(Esc の n は『答えのあった入力』の数。試作の 1 往復目は、積んだ入力の答えを『自分の提示でだけ』受け取る版のページで、別の経路〔rAF・idle の fallback〕が先に画素の変化を提示すると答えが付かなかった: Esc 15/35。2 往復目からは『どの経路の提示でも、最初に何かを見せた提示が答える』版〔`completeInline` を `deskTick` から呼ぶ〕で、Esc は 35/35、33/35。1 往復目のページは v1、2〜3 往復目は v2。`gpu7-L1.diff` は v3〔v2 + `inlineSince` の修正 + syscmd を ring に残す修正〕。)

**読み(訂正後)**:
- **B(エンジンに渡るまで。`tInject` は C が注入した時刻)**: File クリック 中央値 17 → 1 ms、平均 47 → 3 ms。Esc の中央値 6 → 0 ms。= 『エンジンの idle を待つ』時間がほぼなくなった。**有効**。
- **書き込み率(音)**: 本番 0.413 / 0.414 / 0.415、試作 0.410 / 0.410 / 0.405(平均 0.414 → 0.408 = −1.4%)。雑音(0263 で ±0.013)と同程度で、増えてはいない。**有効**。
- 6 run とも `expectRunning`(エンジンが動いている)、操作のエラー 0。
- **T の列の試作の値(17 ms など)は下限**(§6.2)。本番の T は有効(答えが『エンジンが処理して idle に戻ったあとの最初の提示』で、idle の最小 5.7 ms より速い答えが 0/105)。
- load が高いとき(15〜37)に試作を測っていない(窓がなかった)。本番は 185〜298 ms(0269)。B が idle の間隔に比例していたので差は大きくなる見込み(読み)。**未測定**。

### 6.2 T の答えの帰属の誤り(第3報の『59 → 17』が下限にしかならない理由)

- v2 の答えの規則(`completeInline`): 『注入のあとの最初の、画素が変わった提示』。再生中は UI が絶えず変わる(再生ヘッドなど)ので、**エンジンが入力をまだ処理していない時点の提示が答えになりうる**。本番の答えは『エンジンが処理して idle に戻ったあとの最初の提示』で、この混入はない。
- 証拠(実測、場面 b の再生中の File クリックの T):

| | n | 最小 | p10 | p25 | 中央値 | T < 5.7 ms(idle の File クリックの最小 = 本番)| T < 11.4 ms |
|---|---|---|---|---|---|---|---|
| 本番 | 105 | 15.6 | 16.9 | 20.5 | 59.1 | 0 (0%) | 0 |
| 試作 v2 | 105 | **1.7** | 4.0 | 7.2 | 17.1 | **20 (19%)** | 34 (32%) |

  idle の File クリックの T の最小は本番 5.7 / 試作 6.2 ms(メニューを作る処理が最低それだけかかる)なので、それより速い答えは物理的に不可能 = **偽の答え**。Esc でも 5/83(6%)が idle の最小(1.5 ms)より速い。
- もう 1 つの欠陥(同じ run の idle の調査で見つけた): v2 は、idle 経路が `awaiting` を空にしても `inlineSince` を戻さなかったので、次の入力が注入された瞬間に『150 ms 経過』と判定されて**答えなしで終わる**(`tIdle − tInject` が 0.2 ms で `frames {}`、outcome `late`)。idle の Esc の答えのあった数が試作 22/35・24/35・30/35(本番 35/35)だったのはこれ。**v3(`gpu7-L1.diff`)で直した**: 注入のとき `awaiting` が空なら `inlineSince` を今にする。

### 6.3 真の T(測定専用ページ。期待した結果が画面に出た時点)— P2 の再生中の入力遅延に対する結果

**方法**: 本番 ff5ee429(本番のページ HEAD 91cbd9b0)と、L1 試作 bundle `ciL` + **測定専用ページ `srcLB`** を、**同時に走らせた 2 つの Chrome の対**で 2 回(`tools/run-latb.sh base-<n>` と `truth-<i>`、場面 `scn-latb.json`: 参照プロジェクトを開き、Space で再生、10 s 後に File クリック + Esc 各 35 回)。`srcLB` = 試作ページ(v3)+ 『注入した入力の答えは、その提示が描いた窓表に**期待した結果**があるときだけ』(File クリックなら `TQuickPopupMenuWindow` がある、Esc ならない。`api.describe()` の窓表を読む)。**この条件はクラス名を含む測定専用のもので、製品に入れない(`gpu7-L1.diff` に入っていない)**。2 つ同時のため load は 10〜15(他の担当のビルドを含む。run ごとの始め→終わりを表に記録)。両者が同じ load を見る代わりに、**絶対値は低 load の測定より大きい**。

| pair | arm | load (start→end) | write rate | input | sent | answered | **T median / p90 / p99** | T min | B median / mean | C median |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | base | 10.31→14.94 | 0.344 | click | 35 | 35 | **202 / 382 / 474** | 32.3 | 137.9 / 120.5 | 28.0 |
| 1 | base | 10.31→14.94 | 0.344 | esc | 35 | 35 | **105 / 288 / 547** | 4.0 | 77.1 / 84.3 | 3.8 |
| 1 | truth | 10.31→15.55 | 0.337 | click | 35 | 35 | **31 / 43 / 50** | 18.6 | 2.2 / 5.3 | 26.7 |
| 1 | truth | 10.31→15.55 | 0.337 | esc | 35 | 35 | **9 / 29 / 34** | 3.4 | 2.9 / 6.3 | 6.2 |
| 2 | base | 14.31→11.12 | 0.351 | click | 35 | 35 | **185 / 439 / 541** | 26.7 | 120.1 / 116.8 | 38.2 |
| 2 | base | 14.31→11.12 | 0.351 | esc | 35 | 35 | **67 / 296 / 426** | 4.9 | 26.6 / 47.3 | 4.2 |
| 2 | truth | 14.31→11.48 | 0.347 | click | 35 | 35 | **29 / 38 / 56** | 20.0 | 4.3 / 4.4 | 24.6 |
| 2 | truth | 14.31→11.48 | 0.347 | esc | 35 | 33 | **11 / 33 / 46** | 4.0 | 3.1 / 6.9 | 6.6 |

pooled:

| arm | input | answered | T median / p90 / p99 | T min | B median / mean |
|---|---|---|---|---|---|
| base | click | 70 | **201 / 401 / 541** | 26.7 | 122.1 / 118.6 |
| base | esc | 70 | **86 / 288 / 547** | 4.0 | 40.7 / 65.8 |
| truth | click | 70 | **30 / 40 / 56** | 18.6 | 3.8 / 4.9 |
| truth | esc | 68 | **10 / 29 / 46** | 3.4 | 3.0 / 6.6 |

**読み**:
- **File クリック(再生中)の T の中央値 201 → 30 ms(約 1/7)、p90 401 → 40 ms、p99 541 → 56 ms**(70 対 70)。**Esc**: 86 → 10 ms、p90 288 → 29 ms。B(エンジンに渡るまで)の中央値 122 → 3.8 ms、41 → 3.0 ms。書き込み率は同じ(0.344 / 0.351 対 0.337 / 0.347)。
- **物理的に速すぎる答えがない**: 試作の File クリックの T の最小は 18.6 ms(idle の最小 5.7〜6.2 ms より長い。§6.2 の v2 は 1.7 ms)。
- C(注入から答えの提示まで)の中央値は本番 28〜38 ms、試作 25〜27 ms でほぼ同じ = **L1 は B(エンジンの idle を待つ時間)だけを取り、C(UI の処理)は変えない**。
- idle の File クリックの T は 7〜12 ms(§6.4)で、再生中の試作 30 ms はその約 3 倍(load 10〜15)。残りは C(UI の処理がミキサーの課金に押される分。P3 の文書)。
- Esc の答えは 68/70(2 件は答えなし)。
- **第3報の『59 → 17』は §6.2 のとおり下限で、使わない**。低 load(4〜12)の窓で本番の T が 59 ms だったのに対し、ここは load 10〜15 の本番 201 ms で条件が違うので、この表と §6.1 の本番の値は直接比べない。

### 6.4 idle の退行(File メニューのクリック・Esc・Mixer ヘッダーのドラッグ。プロジェクトなし、本番と試作を交互 3 往復。`tools/run-alti.sh`)

- load は 3.9 → 23.5 の間で動いた(他の担当のビルドが走った窓。run ごとの始め/終わり: base-1 3.9→5.9、proto-1 5.9→23.5、base-2 23.5→12.6、proto-2 12.6→18.0、base-3 18.0→12.5、proto-3 12.5→8.3)。**load が条件を揃えていないので、結論は『悪化は見えない、雑音が大きい』まで**。

| 入力 | 指標 | 本番 1 / 2 / 3 | 試作 1 / 2 / 3 |
|---|---|---|---|
| File クリック(n 36) | T 中央値 / p90 | 7/11、8/23、9/17 | 11/31、12/23、8/19 |
| Mixer ヘッダーのドラッグの移動(n 80) | T 中央値 / p90 | 3/17、4/8、4/12 | 6/20、8/50、4/10 |
| Esc(答えのあった数) | T 中央値 / p90(n) | 2/2(35)、2/9(35)、3/4(35) | 3/8(22)、3/6(24)、2/3(30) |

  File クリックの中央値は 3 往復中 2 往復で試作が 3〜4 ms 長く、1 往復で 1 ms 短い(load が往復で違う)。Esc の『答えのあった数』の欠けは §6.2 の `inlineSince` の欠陥で、v3 で直した(v3 での再測定は §6.3 の測定専用ページと同じ修正を含む)。


### 6.5 製品側の答えの帰属の規則(L1c。PM 09:42 の (b))

**規則**: 注入した入力の答えは、**『その入力をゲストのキューから取り出したスレッドが、そこから作られたメッセージを全部取り出し、再び message の待ち(GetMessage / WaitMessage / MsgWaitForMultipleObjects)に入った(= 処理し終えた)あとに始まった、最初の画素が変わる提示』**。クラス名・窓の種類の分岐なし、新オプションなし。v2 の『注入のあとの最初の画素が変わる提示』は、エンジンが処理する前の提示(再生ヘッドの変化など)を答えにしていた(§6.2)。

**実装**(`gpu7-L1-LC.diff` = L1 v3 + L1c、`gpu7-LC-over-L1.diff` = L1 v3 に足す分。+約 40 行、いずれも外部 API の契約の実装でも FL の挙動の代行でもなく、**通信/計測の帳簿**):

| ファイル | 変更 |
|---|---|
| `lift/runtime/fle_input.c` / `.h` | `fle_input_settled()`(read-only): evq が空、outq が空、最後に取り出したスレッド(`fle_input_synth` が取り出した / 返したときの `t`)が message の待ち、のとき 1 |
| `lift/runtime/fle_thread.c` / `.h` | `fle_thread_in_message_wait(idx)`: そのスレッドが `W_MSG_GET` / `W_MSG_WAIT`(と msg 付きの `W_OBJECTS`)で待っているとき 1 |
| `lift/harness/fruity_idle.c` | `fle_input_ring_poll()` が、scheduler の点ごとに settled を record 領域の直後の 1 語に書いてから import を呼ぶ(import の署名は不変) |
| `app/src/engine/worker.ts` | `inputPoll` がその語を読む。注入のあとの scheduler の点で 1 を見てから(`inlineSettled`)始まった提示だけが答え(`deskTick` は提示の**前**に旗を読む)。150 ms で settled にならなければ答えなしで終える |
| テスト | C: `an_injected_input_is_settled_only_when_the_thread_that_handled_it_waits_in_its_message_loop_again`(注入前 1、直後 0、メッセージを持っている間 0、UI が GetMessage に戻ると 1)、`a_hook_with_nothing_queued_changes_neither_the_virtual_clock_nor_the_thread_order`(入力のないときのフックは仮想時計もスレッドの順も変えない) |

**限界の 2 点(PM 10:18 の (5))の確認**:
1. **効果が settled より前の提示で既に出ていた場合**: settled のあとの提示は変化なし → 答えなし(150 ms であきらめる)。FL の実際の入力経路で起こりにくい根拠: 入力の処理(WM_LBUTTONDOWN など)は UI スレッドが message ループの中で同期に描くので、効果は『UI が待ちに戻る』より前には画面状態に完成しているが、**提示(ブラウザへ見せる)は別の経路(rAF・idle)で、settled の前に出ることがある**。この場合は答えなしになる(測定で率を見る)。
2. **入力の処理が別スレッドに非同期に任される場合**: 決定 0234 の測定(参照プロジェクトの再生 30 仮想 s)で、ミキサースレッドが呼ぶ API は『ウィンドウ・GDI・メッセージ・ファイル・レジストリの API は 0』、UI スレッドが 47 種・112 万回/s。= **FL のウィンドウとメッセージは UI スレッドが処理する**(読み。0234 の測定による)。settled は『最後に取り出したスレッド』で判定するので、UI が別スレッドに非同期に描かせる経路があれば早すぎる settled になる。この経路の有無は測っていない。

**測定**(`ciLC` = L1 + 答えの規則の bundle、wasm 142,381,520 B。ページは `srcLC`。**製品の規則そのもの**で、測定専用ページのクラス名の条件は使っていない。本番 ff5ee429 と**同時 2 Chrome の対**を 1 回。場面 b を再生 → 10 s 後に File クリック + Esc 各 35 回。**load は 16.7 → 32.9**〔他の担当のビルドで高い〕):

| pair | arm | load (start→end) | write rate | input | sent | answered | **T median / p90 / p99** | T min | B median / mean | C median |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | base | 16.65→32.93 | 0.263 | click | 35 | 35 | **275 / 465 / 657** | 32.8 | 75.6 / 125.5 | 178.0 |
| 1 | base | 16.65→32.93 | 0.263 | esc | 35 | 35 | **176 / 394 / 627** | 16.0 | 104.5 / 114.1 | 5.3 |
| 1 | lc | 16.65→32.93 | 0.255 | click | 35 | 35 | **36 / 68 / 85** | 24.7 | 1.9 / 9.0 | 30.9 |
| 1 | lc | 16.65→32.93 | 0.255 | esc | 35 | 35 | **12 / 30 / 78** | 5.7 | 2.6 / 9.3 | 7.8 |

- **物理的に速すぎる答えは 0**: File クリックの T の最小 24.7 ms(idle の最小 5.7〜6.2 ms より長い。v2 は 1.7 ms で 19% が 5.7 ms 未満)、5.7 ms 未満は 0/35。Esc の最小 5.7 ms(idle の最小 1.5 ms)。
- **答えなしの率 0**: 70/70 に答えが付き、outcome は全部 `window`(v2 の idle の Esc は 22〜30/35)。
- 真の答え(測定専用ページ、§6.3)の File クリック T 30 ms(load 10〜15)と L1c の 36 ms(load 17〜33)は同じ水準(別の run の比較。読み)。同じ対の中での直接比較はしていない。
- 書き込み率: 本番 0.263、L1c 0.255(同じ)。load が高いので本番の T は §6.3 より大きい(275 ms)。この表の本番の値は §6.3 と直接比べない。
- **まだ見ていない**: 効果が settled より前の提示で出て答えなしになる場合(この run では 0)、他の入力(ドラッグ・ホイール・複数ボタン)、syscmd の復元ボタンが実エンジンで動くか。

**task bar の syscmd との関係(PM 10:30 の (1)(a))**: e8-wip の ring には syscmd(`#taskbar-restore`、kind 8)が入る。v2/v3 の `inputPoll` は『mouse / key 以外は display』として積んでいたので、**syscmd が display(0×0)として誤って取り出され、失われる**ところだった(PM の問いで見つけた)。直した: `worker.ts` の `inputPoll` の callback が mouse / key / display 以外で `false` を返し、`ring.ts` の `drainN` は `fn` が `false` を返した記録を取り出さずに止まる(その記録と後ろの記録は順序どおり ring に残る)。syscmd は従来どおり idle で `IdleDriver.apply` が処理する。ring に syscmd が残っていても、それまでに取った入力の答え(`answerInline`)は止まらない。vitest 1 本を足した。**積んだ木(e8-wip a1e96d92 + `gpu7-L1-LC.diff`)**: C 6 本 + `wall_fr9t` 18 本 passed、typecheck rc=0、vitest 1,310 passed + 7 skipped、test:worker 49 passed、test:e2e は 1 回目 175/177(load 30 前後の窓で `taskbar-restore` の Tab / Enter の waitFor が 2 件 timeout。DOM のフォーカスのテストで ring と無関係。その後の group は走らなかった)、2 回目 **229/229**(taskbar-restore を含む)。

## 7. 残るリスクと、この試作が確かめていない点

| # | 内容 | 状態 |
|---|---|---|
| R1 | **仮想時刻の差**(§3): 新経路は仮想時計を進めずに積むので、エンジンが実時間より遅れている間、入力の仮想時刻が従来より『idle 経路が進めていた分』だけ早くなりうる。FL が見る入力どうしの時間差は保たれるが、タイマーとの相対は変わりうる | 読み。入力を使わない node の 84/55/87 行は不変のはず(PM が確かめる)。G3 の入力列(a4)は変わりうる |
| R2 | **計器の意味**: 注入された入力の `tIdle` は『答えの提示の時刻』で、idle 経路の `tIdle`(エンジンが idle に戻った時刻)と別の量。C の定義が違うので、新旧の C は比べられない(T と B は比べられる) | 実測(§6) |
| R3 | **答えの帰属**: v2 の規則(『注入のあとの最初の画素が変わる提示』)は、エンジンが処理する前の提示を答えにして T を過小評価する(§6.2: 最小 T 1.7 ms < idle の最小 5.7 ms)。v3 は `inlineSince` の欠陥だけ直した。**製品の規則は L1c の settled**(§6.5)。測定専用ページ(クラス名を含む)で真の T を出した(§6.3) | 実測。v3 は T を下限にする。L1c の実測は §6.5 |
| R4 | **提示の頻度**: 入力が待つ間の 6 ms ごとの提示の試行(最大 150 ms = 25 回)は、再生中の UI の仕事を増やす。書き込み率は 0.414 → 0.408(−1.4%)で、雑音(±0.013)と同程度 | 実測、3 往復。有意ではないが、増えてはいない |
| R5 | **高 load**: 低 load(4〜12)の交互(§6.1)に加えて、同時 2 Chrome で load 10〜15 の対(§6.3: 本番の File クリックの T は 201 ms)で測った。load 15〜37(本番が 185〜298 ms の窓)の試作は未測定。B はエンジンの idle の間隔に比例していたので、さらに高い load では差が大きくなる見込み(読み) | 15 以上は未測定 |
| R6 | **長時間**: 5 分以上の再生、入力の種類(ホイール、ドラッグ、複数ボタン、キーの自動リピート)の網羅、Firefox | 未確認。キーの自動リピートと範囲外の座標は C のテスト 3 の範囲 |
| R7 | **テスト用のマクロ**: `FLE_RING_POLL_HOOK` は native の C テストでフックを有効にするコンパイル時の定義(`__EMSCRIPTEN__` の代わり)。製品の設定項目ではない。製品の wasm ビルドは `__EMSCRIPTEN__` で有効 | 事実 |
| R8 | **idle の退行**: idle の入力は新経路を通らない(エンジンが idle の間は従来の `onEngineIdle`)。ただし idle 経路でも `schedule_ex` の先頭でフックが呼ばれる | §6.4: 悪化は見えない(File クリックの中央値 本番 7 / 8 / 9、試作 11 / 12 / 8 ms。load が往復で 4〜23 と揃わず雑音が大きい) |
| R9 | **L2(動的ブースト)は勧めない**: C の主因が『ミキサーの描画ロックの待ち』なら、保持者(ミキサー)が走らない限り UI の優先度は効かない(Windows の CS に優先度継承はない)。C を縮めるのは P3(ミキサーの速さ)の側 | 読み(0234 の測定から。この build では未確認) |
| R10 | **L1c(settled)の限界**: (a) 効果が settled より前の提示で出ていると答えなし(150 ms)、(b) UI が別スレッドに描画を非同期に任せる経路があると settled が早すぎる。FL のウィンドウとメッセージは UI スレッドが処理する(0234 の測定。読み) | §6.5 |
| R11 | **測定専用ページのクラス名の条件**(`TQuickPopupMenuWindow`)は製品に入れない。真の T の表(§6.3)は、この条件で『期待した結果が画面に出た時点』を測った値 | 製品ではない |

## 8. S-gpu7-N と自己申告

- **S-gpu7-8**: 入力リングを読む点は `schedule_ex` の先頭(1 か所)。1 回に最大 16 件(`FLE_RING_MAX`)。出典なし。
- **S-gpu7-9**: 注入した入力の答えの試行間隔 6 ms(`INLINE_TRY_MS`)、あきらめる時間 150 ms(`INLINE_GIVEUP_MS`)。出典なし。
- **S-gpu7-10**: 仮想時計を注入で動かさない(§3)。idle 経路との差は、エンジンが実時間より遅れている間の入力の仮想時刻(読み)。
- **S-gpu7-11**: 測定専用ページ(`srcLB`)の『期待した結果』= File クリックなら `TQuickPopupMenuWindow` が窓表にある、Esc ならない。クラス名の分岐を含むので**製品に入れない**。
- **S-gpu7-12**: 交互測定は低 load の窓(4〜12)だけ。高 load の試作は未測定。

**自己申告**:
1. **第3報の『File クリック T 59 → 17 ms』は、試作の答えの帰属の誤りで下限にしかならなかった**(§6.2)。試作の T の最小 1.7 ms が idle の最小 6.2 ms より速いことに、報告の前に気づくべきだった。第3報で『`tIdle` の意味を変えた』と自分で書きながら、その結果の帰属の妥当性(物理的に速すぎる答えの有無)を確かめなかった。idle の調査の中で見つけ、09:42 に PM に訂正を送った。
2. 試作 v2 に `inlineSince` を戻さない欠陥(§6.2)があり、idle の Esc の一部が答えなしになった。v3 で直した。
3. ビルドの起動で、`background:true` の 1 行の中に末尾 `&` を書いた(規則違反)。5 秒後に自分の PID だけ止めて、起動し直した。
4. 測定専用 bundle の `bundle` 段で、`BROTLI_QUALITY` を渡し忘れて brotli q11 の圧縮に数分待たされた。自分の PID(`pack-assets.mjs`)を止めて、`BROTLI_QUALITY=1` で `bundle` 段だけ再実行した(2 本とも)。
5. 並列の 2 つのツール呼び出しで、1 つ目が作るファイルを 2 つ目が使い、exit 127 になった。もう一度実行した。
6. P3 の対照(4096 回)の最初の run は、自分の `vite build` と重なったので、やり直した(対照 B)。load は run ごとに記録。
7. 配信バケットの読み取りは `wrangler dev`(remote の R2 の list / get のみ)。書き込み操作は使っていない。終わったら PID で止めた。
8. 見つけた文書と実装の食い違い(直していない): 決定 0147 §4.3 と `fle_thread.h` のコメントは『環境変数 `FLE_PREEMPT_DISPATCHES` / `FLE_PREEMPT_TICKS` で 2 つの数を替えられる』『`FLE_PREEMPT=0` で機構を止める』と書くが、d6926b4d の `lift/` にその `getenv` はない(`git grep FLE_PREEMPT` はコメントと `wall_fr20.rs` の `env_remove` だけ)。

## PM の判断と取り込み(2026-11-06 10:55、docs のみ。差分は製品の木に当てていない)

取り込み: 差分 3 本(`gpu7-L1.diff`、`gpu7-L1-LC.diff`、`gpu7-LC-over-L1.diff`)を `docs/investigations/gpu7/patches/` に置いた。`gpu7-L1-LC.diff` は e8-wip(a1e96d92)の `git archive` に `patch --batch -p1` が rc=0(offset 12、fuzz 2、reject 0)で当たることを PM が確認した。PM は C のテスト・app の連鎖・a4・node 行をこの差分ではまだ走らせていない(gpu1 の積んだ木でのテスト: C 24 本、vitest 1,310 + 7 skip、worker 49、e2e 229/229〔1 回目は load 30 前後で 175/177、focus の waitFor 2 件の timeout〕)。
判断:
1. **L1c(スケジューラの点で入力 ring を読む + 製品の答えの規則)は E9 の部品にする**(E8 には入れない: 停止の切り分けが効かなくなるため)。実エンジンの対(load 17〜33、同時 2 Chrome): File クリック 275 → 36 ms、Esc 176 → 12 ms、B 75.6 → 1.9 ms、物理的に速すぎる答え 0、答えなし 0/70、書き込み率は不変(0.263 → 0.255)。load が高い窓の値は参考で、採用判断は E9 の低 load の窓の値で行う。
2. **必須の修正を含める**: v2/v3 の `inputPoll` は mouse/key 以外を display として取り、alpha11 の復元ボタン(`syscmd`)を失う欠陥があった。gpu1 が自分で見つけ、scheduler の点が取る kind は mouse/key/display だけとし、それ以外は ring に残して IdleDriver が処理する形に直した(`drainN` が false で止まる)。復元ボタンの実エンジンでの動作は E8 の bundle の上で gpu1 が確認する。
3. 未確認: ドラッグ・ホイール・複数ボタン、低 load の再現、Firefox、実 GPU、node 84/55/87 行と a4(PM)。
