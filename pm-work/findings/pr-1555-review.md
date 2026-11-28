# PR #1555 の所見（パフォーマンス最適化 Waves 1-19）

作成: 2026-11-28（UTC）。作業者（PM 側）の読み。実測と推定を分けて書く。確認できなかった物は「確認できない」と書く。

## 0. 対象と事実

- PR #1555（`perf/optimize-001`、head `dd00c1fd42dc3fbe54c31e246ac4af5982fb04dc`。commit 3 つ: `34b8cd51`・`b18cc754`・`dd00c1fd`）。61 ファイル、+16,303 / −1,033 行（`gh pr diff 1555 --name-only` の 61 件と、`git diff --numstat 332cbd66 dd00c1fd` の 61 件が一致）。
- PR の基準は `332cbd66`。現在の main（`2112e22c`）は、`332cbd66` から、`AGENTS.md`・台帳（`pm-work/ledger/instructions.tsv`）の変更だけ進んでいる（PR の 61 件とは重ならない）。
- **実測**: `git merge-tree --write-tree origin/main dd00c1fd…` は衝突を出さなかった（作業ツリーは変えていない）。
- **実測**: `bash guard/flags.sh origin/main dd00c1fd…` は指摘 0 件（終了コード 0）。
- 注意（実測）: 同じクローンの `FETCH_HEAD` は、他のセッションの `git fetch` で上書きされた（この作業の途中で、`FETCH_HEAD` が dependabot の枝を指した）。取り込みの作業は、`git fetch` の直後に SHA を控えて使う。
- **未実施**: `guard/scan.sh` は作業ツリーのファイルを読むため、PR の内容には当てていない（PM の報告では 0 件）。取り込んだ後の段で当てる。
- 「PR の検証」の範囲（PR 本文）: `fe-build engine`・`fe-build bundle`、`boot-bundle.mjs`（warm 中央値 12.2 秒。PR 本文の値）、`welcome-flow.mjs` の流れ A〜D、`npm test`、`cargo test`（PR 本文の一覧）。**P3 の A/B（音声の途切れ）・1000 Hz 入力の fps は、PR の検証に無い**（PR 本文に無い）。
- 確認できない: PR 本文の検証結果（12.2 秒・流れ A〜D）は、この作業では再実行していない。

## 1. 差分の 3 群の分け方

分け方の基準: (低) 挙動が変わらないと読めて、既存の試験で守られる物。(中) 入力・描画・UI の応答時間や、試験の期待を書き換えた物、または正しさを試験で確かめるまで (低) に入れない物。(高) 音声（P3）・協調スレッドの切り替え・計時器・起床の通知・固定番地への書き込み・音声プラグインの生成物。

PR のファイルは、どの群にも入れない 2 件（`app/src/engine/hz.ts`・`app/test/unit/hz.test.ts`）を除き、1 群に 1 回ずつ入れた。混在する 3 件（`fle_sys.c`・`fle_win.c`・`worker.ts`）は、主の群に入れ、他の群の部分を注記した。

### 1.1 (低): 20 件

| ファイル（PR の増減） | 内容（読んだ範囲） | 守る試験（既存） |
| --- | --- | --- |
| `lift/runtime/fle_raster.c`（+358/−11）・`fle_raster.h`（+3）・`fle_sse.h`（+25/−11）・`fle_mxcsr_vec.h`（+2/−2） | 差分走査の SIMD 化（`fle_r_diff_begin`・`fle_r_diff_end`）。実際の差分の全行は読んでいない | `raster.rs`・`screen1`・`alpha2_changed`・`alpha3_updregion` |
| `lift/runtime/fle_gdi.c`（+275/−75） | GDI のホットパスの最適化。全行は読んでいない | `gdi.rs`・`gdi_bulk.rs` |
| `lift/runtime/fle_flags.h`（+29/−22）・`fle_flags_nf.h`（+4/−4） | フラグの算術（`__int128` を除き、`__builtin_parity` を使う）。シフトの回数は `fle_cnt` が 6 ビットに丸める（`fle_flags.h` 138 行付近）ので、`a << c` の未定義動作は起きない（読みによる） | `flags.rs`・`mxcsr_flags.rs`・`int_ext.rs`・`size1_deadflags.rs` |
| `lift/runtime/fle_user_nowin.c`（+34/−21） | RECT の読み書きを `memcpy` にし、範囲の検査を 1 回にまとめる | 専用の試験は確認できない（`fe-lift` 全件で守る） |
| `lift/runtime/fle_runtime.h`（+6/−2） | `fle_addr` の分岐の予測（`__builtin_expect`）と、`fle_unsupported` の分割 | `fe-lift` 全件 |
| `lift/runtime/fle_sys.c`（+14/−4）【混在】 | 主: `fle_host_protect` の、予約ページの連続した範囲を 1 回の `memset` にまとめる（926 行付近）。他: `LeaveCriticalSection` の起床（412 行）は (高) | `fe-lift` 全件 |
| `lift/harness/web/cflags-ai.args`・`cflags-b2d.args`・`cflags-h.args`・`cflags-rt.args`・`wasm-opt.args`・`wasm-opt-c.cmd` | 最適化の水準。`-Oz` → `-O2`、`-O1` → `-O2` など。新しい変数は無い | 起動の確認（P1: 経過時間 20,000 ms 以下） |
| `lift/crates/fe-build/src/main.rs`（BLEND2D_CXX_FLAGS、41 行付近）・`lift/crates/fe-build/tests/plan/engine.txt`（+160/−160） | Blend2D の C++ フラグ（`-msimd128` を足し、重複を除く）。計画の試験の写し | `fe-build` の計画の試験 |
| `lift/harness/audio/sim_transport.c`（+3） | ネイティブ試験のための音声の模擬の関数 | ネイティブ試験 |
| `app/src/engine/protocol.ts`（+1/−1） | 型の注釈 | `npm test` |

### 1.2 (中): 24 件

| ファイル（PR の増減） | 内容 | 注意 |
| --- | --- | --- |
| `app/src/engine/desk-gl.ts`（+48/−50） | WebGL2 の提示。`preserveDrawingBuffer: false`（165 行）。部分描画の scissor をやめ、更新の矩形をまとめて 1 回で描く（293 行の説明）。不透明な最下段の窓では色の消去を省く | 試験の期待の書き換えを伴う（下の `desk-gl.test.ts`） |
| `app/test/unit/desk-gl.test.ts`（+21/−15） | 期待の書き換え。「移動していない窓の部分描画は scissor で 30 px だけ描く」の確認を、「更新の矩形を 1 回で描き、描画の画素は 1920×1080」に置き換えた | **期待を弱める書き換え**。元の確認（部分描画は更新の矩形だけ）が消えた |
| `app/src/engine/desk-link.ts`（+5/−1） | 描画の要求の枠（`WANT`）を、`GEN`・`ACK` の次に足し、`MODE` を 3 にずらす | 描画の担当との共有の枠の位置が変わる |
| `app/src/engine/render-worker.ts`（+25/−6） | 眠りの猶予 100 → 500 ms（23 行）。要求の出し方（`ask`・`frame`）。'want' の応答の省略（`worker.ts` の 606 行と合わせる） | 描画の応答が止まらないことの確認が要る（確認できない） |
| `app/src/engine/idle-driver.ts`（+10/−2）・`app/test/unit/idle-driver.test.ts`（+37） | `takeAwaiting(count)`（入力の答えを、先頭の n 件だけ取る）。新しい試験 | 試験は追加の方向（期待の弱めは無い） |
| `app/src/engine/client.ts`（+3/−2） | マウスの MOVE では、エンジンを起こさない（171 行。描画の枠の時だけ起こす） | 入力から提示までの遅れ（P2）に効く。P2 の A/B で見る |
| `app/src/main.ts`（+38/−3） | カーソル画像のキャッシュ（ハッシュの鍵。128 件で消す）、表示文字の差分更新、描画の時刻の記録（`displayFrameTimes`） | 計測用の `displayFrameTimes` は製品の状態に足す（旗ではない） |
| `app/src/ui/desktop.ts`（+21/−2） | 描画の矩形をキャッシュする（`resize`・`scroll`（capture）・`ResizeObserver` で無効化）。カーソルの重複を省く | **ポインタの座標の対応**に効く。レイアウトの変化が上の 3 つ以外で起きたときに古い矩形を使う（推定。確認できない） |
| `app/src/engine/desk.ts`（+2/−1）・`app/src/engine/fruity.ts`（+2） | `sceneDirty` を任意の関数として足す（`fle_desk_scene_dirty`） | `fle_desk.c` と対になる |
| `lift/runtime/fle_desk.c`（+157/−31）・`lift/runtime/fle_desk.h`（+3） | シーンの差分走査（`trim_row`）を 16 バイト単位にする。窓ごとの矩形を最大 16 件（554 行 `SCENE_WIN_MAX_RECTS`）にする | `trim_row` の境界は読んで確かめた（`x0`・`x1` は `int32_t`。減算の折り返しは無い）。**16 件を超えたときの扱いは読んでいない（確認できない）** |
| `lift/runtime/fle_input.c`（+11/−1） | 続けて届いた MOVE を、キューの末尾の 1 件にまとめる（148–158 行）。`ev_n >= EV_MAX` の検査を、まとめた後へ移す | 入力の間引き。P2 の A/B で見る |
| `lift/runtime/fle_win.c`（+90/−65）【混在】 | 主: `CM_ACTIONUPDATE`（0xB03F）の掃引を、状態を変える通知が無いときは 200 ms に 1 回にする（1040–1058 行）。`unc_move_rows`（3509 行）は、窓の内容の移動をインプレースで行う（低の部分。読みでは順序は正しい）。他: 計時器の走査範囲（`timer_hi`、5683 行など）は (高) | `CM_ACTIONUPDATE` の間引きは、メニューの有効・無効の表示を最大 200 ms 遅らせる（推定） |
| `lift/runtime/fle_x87.c`（+1025/−145） | x87 の高速パス。**正しさは読んでいない（確認できない）** | 試験で確かめるまで (低) に入れない |
| `lift/crates/fe-wasmgen/src/lower.rs`（+497/−157）・`lib.rs`（+52/−18）・`hybrid.rs`（+4/−7） | 生成するコードの変更（条件付きの `flush_lc`、呼び出しごとの `reload_lc` の省略、レジスタ・フラグの最適化）。**正しさは読んでいない（確認できない）** | 全 Wasm 生成に効く。`cargo test -p fe-wasmgen` と、流れ A〜D で見る |
| `lift/harness/b2d1/b2d4-tick.patch`（+29/−8）・`lift/harness/b2d1/b2d_swap.cpp`（+46/−47） | Blend2D の tick の通知をまとめる | 通知の時刻が変わる（確認できない） |
| `lift/harness/seeds/reg-seed-app.tsv`（+3）・`lift/crates/fe-lift/tests/wall_fr3.rs`（+7/−2） | 既定値を 3 行足す（`MIDIForm` の `AnimRefreshRateCombo`=2・`PollBox`=0・`PBTrackingModeBox`=1）。試験の「値は 1 件だけ」を 4 件に広げた | **製品の既定値の変更**（画面の設定に効く）。試験の期待の書き換え |
| `app/src/engine/worker.ts`（+231/−55）【混在】 | 主: 入力の MOVE の間引き（`MOVE_PACE_MS = 8`、252 行。`pendingMove`・`mergedMoveSeqs`）、入力の答えの扱い（`inlineSettledCount`・`pushDeferredAnswers`）、描画の要求（`serviceRenderWant`）、カーソル画像の送り（`sendCursorPicture(shared)`）、駆動の上限（`maxLeadMs: 1, maxLagMs: 10`、237 行）、'want' の応答の省略（606 行）。他: 計時器の枠（160–178 行）は (高) | 入力の遅れ（P2）と描画の応答に効く |

### 1.3 (高): 15 件（+ 混在の 3 件の部分）

| ファイル（PR の増減） | 内容 | 根拠の行（PR 側） |
| --- | --- | --- |
| `lift/runtime/fle_thread.c`（+344/−20） | (a) UI の順番: `pick_ui_pacing` が 2.4〜4.2 ms の枠を、音声の余裕（headroom）で決める（820 行）。(b) 仮想時計を壁時計へ寄せる（`vclock_wall_sync`、885 行。`fle_sys_clock_advance_to(target)` 914 行）。(c) 起床の通知型: `fle_thread_wake_cs`（485 行）・`fle_thread_wake_event`（506 行）。(d) `is_dwm`（1416 行） | 協調スレッドの切り替えと仮想時計（P3・P2 の両方） |
| `lift/runtime/fle_thread.h`（+3/−1） | 後退辺の切り替え（`FLE_LOOP_SLICE_BACKEDGES`）を 1024 → 4096（98 行）。起床の宣言（187–188 行） | 切り替えの頻度が 4 倍に下がる（推定） |
| `lift/runtime/fle_audio_ring.c`（+16/−1） | 外部の消費者の下で、仮想時計とフレームの対応を変える（量子の端へ合わせる。`fle_audio_tick_of_frame` 119 行付近）。`fle_audio_wait_until` の再アンカー | 仮想時計と音声の対応が変わる |
| `lift/runtime/fle_audio_transport.c`（+37/−1）・`fle_audio_transport.h`（+4） | 消費位置の経過時間（`rdAt`・`fle_js_rd_age_ms`）、量子の大きさ（128）、空きの計時 | 音声の読み出しの計時 |
| `lift/runtime/fle_dsound.c`（+180/−46） | (a) 外部の消費者の下で、looping バッファは **アンロック済みの範囲の外を無音にする**（`mix_chunk` 339 行: `if (i0 >= b->buffered_until) continue;`）。(b) 28 ms の余裕（`fle_audio_has_slack`）。(c) 音声の再計算を、変化があったときだけにする（`audio_dirty`） | 音（P3）に直接効く。途切れの原因になりうる（推定） |
| `lift/runtime/fle_event.c`（+2/−1） | `SetEvent` が、対象のイベントを待つ thread だけを起こす（240 行） | 起床の通知型 |
| `lift/runtime/fle_sys.c`（混在の部分）| `LeaveCriticalSection` が、その CS の待ちだけを起こす（412 行） | 起床の通知型 |
| `lift/runtime/fle_win.c`（混在の部分）| 計時器の走査範囲を `timer_hi` に限る（`SetTimer` 5683 行、`fle_win_host_timer` 7716 行、`KillTimer`・`timers_of_window_gone`）。読みでは正しい（枠の使用状況と上限が一致する） | 計時器 |
| `app/src/engine/worker.ts`（混在の部分）| **計時器の枠（`DEADLINE_SLOTS = 16`、160 行）**。下の 2 節を参照 | 計時器（遅れた発火） |
| `lift/harness/web/threads/fle_thread_lib.js`（+27/−1） | JSPI の切り替えを、4 ms ごとに MessageChannel の送信で返す（72 行。`queueMicrotask` だけの連鎖をやめる） | 協調スレッドの切り替え（ホストの message を占有しない意図。推定） |
| `lift/harness/two/main_two.c`（+21） | (a) 起動の時に `fle_thread_set_priorities(1)`（318 行）。(b) `fle_host_on_idle`（58–72 行）は、**固定の番地**（`0x86b500` の 7 バイトの照合、`0x15f7190` の読み、`app + 0x164` への 100 の書き込み）で guest のメモリを書く | (b) は版に依る決め打ちの書き込み。目的は PR 本文に無い。**確認できない** |
| `lift/harness/hyb/hyb-args.tsv`（+1）・`lift/harness/hyb/lists/lowered-fruity_delay2.txt`（+12,356、生成物）・`lift/harness/hyb/n0.args`（+1/−1）・`lift/harness/hyb/c-imports-engine.txt`（+2/−2）・`lift/harness/hyb1/hybrid_glue.c`（+1） | `fruity_delay2`（音声のエフェクト）を、直接 Wasm にする。`n0.args` は `--hybrid-mx` を外し、`--mem-import` を `c.memory` にする。`fle_hyb_loop_slice` を足す | **音声の出力（P3）に効く**。生成物の正しさは読んでいない（確認できない） |
| `lift/harness/seeds/elastique-seeds-all3.txt`（+1） | 音声プラグイン（elastique）の seed を 1 行足す | 音声（確認できない） |

### 1.4 取り込まない: 2 件

- `app/src/engine/hz.ts`（+4/−5）と `app/test/unit/hz.test.ts`（+12/−12）。**上限を 120 から 300 に上げる変更**（`STANDARD_HZ` の 13 行目に 144・165・180・240・300 を足す）。依頼者の決めた条件（ミキサーが追いついたら見直す）を満たしていない。`hz.test.ts` の期待の書き換えも、この変更に伴う。両方とも main の版のまま残す。

## 2. 計時器の枠の取り消しが、生きた待ちを発火しない（(高) の前に直す）

PM の読み（「計時器のスロットを 16 個で循環させ、キャンセルの notify が生きた待ちを ok で起こし、発火されない疑い」）を、コードで確かめた。**枠は計時器（`fle_win.c` の `timers`、256 件、空きの最小を取る）ではなく、描画・駆動の `deadlineWait`（`worker.ts`）の枠**。

PR 側（`FETCH_HEAD:app/src/engine/worker.ts`）:

- 160 行: `const DEADLINE_SLOTS = 16;`。161 行: `deadlineWords`（16 要素）。164 行: `nextDeadlineSlot++ & (DEADLINE_SLOTS - 1)`。
- `deadlineWait` の中の行: `if (w.async) void w.value.then((v) => { if (v === 'timed-out') fire(); });`。**`'ok'`（notify で起きた）では発火しない**。
- 173–178 行: `cancelDeadlineWait` は `Atomics.add(deadlineWords, h.slot, 1)` の後、`Atomics.notify(deadlineWords, h.slot)`。**同じ枠の、他の生きた待ちも `'ok'` で起きる**。

main 側（`origin/main:app/src/engine/worker.ts`）: 単一の `deadlineWord`（158 行）を使い、`.then(fire)`（どの解決でも発火）。生きた待ちが別の取り消しで起きても、早く発火するだけで、失われない。

**実測（node v24.19.0、`node -e` の 1 回。リポジトリのファイルは作らない）**: 同じ枠に 2 つの `Atomics.waitAsync` を置き、一方を `Atomics.add` と `notify` で取り消すと、**両方とも `"ok"` で解決した**。

→ PR の版では、取り消しのあとに生きたままの待ちが、どの発火も受けないまま止まる（失われた起床）。枠は、`nextDeadlineSlot` が 16 を回るたびに同じ枠に戻るので、生きた待ちが 16 回分の作成の前に作られていれば、同じ枠になる。`moveFlushTimer`（MOVE の間引き。最大 8 ms 生きる）は、その間に駆動の待ちが多く作られるとき、この条件に入りやすい（推定。実際の頻度は確認できない）。

**直し方の方向（作業者が決める。試験を先に赤にする）**: 'ok' で起きたときは、`h.live` と期限（時刻）を見て、期限を過ぎていれば発火し、過ぎていなければ待ち直す。または枠を取り消しごとに分けない（枠を待ちの数だけ用意する）。製品に分岐の旗は足さない。

## 3. P3 の A/B（音声の途切れ）の関門

### 3.1 道具（確認済み）

- `tools/ab/run.mjs <p2|p3> <base の bundle raw/fl-Oz> <base の app> <changed の bundle raw/fl-Oz> <changed の app> [n=3]`。base と changed を交互に測り、中央値と判定（`pass`）を出す。
- 計測: p2 は `tools/p2-frames/measure.mjs --dpr 1.75 --load a --app <dir>`（pointer を 1000 Hz 以上で送る。30 秒）。p3 は `tools/p3-audio-probe/measure-live.mjs --dpr 1.75 --play 1 --hz 1000 --mouse 20 --sound --app <dir>`（窓は t0+70〜130 秒の 60 秒）。
- 判定（`tools/ab/gates.mjs`）: p2 は、どの run も `error` が null で `inputRateOk`。`minWindowFps` の changed の中央値 ≥ base の中央値 × 0.90。`inputToPresentMsP95` の changed の中央値 ≤ base の中央値 × 1.20。p3 は、changed の各 run の `error`・`playing`・`inputRateOk`・`nonSilentDelta > 0`・`pcmRepeat8192`（null か 0.01 未満）。`underrunDelta` と `last60sUnderruns` の changed の中央値 ≤ base の最大 + 4（NOISE_BAND）。
- **base の bundle**: `.github/workflows/e2e.yml` が `bundle-${{ github.sha }}` を main の push で保存する（`runs-on/cache`）。しかし **作業ジョブ（`agent.yml` の `work`）は、その cache を取らない**（取るのは抽出物と言語・プラグインだけ）。したがって、base の bundle は作業ジョブの中で作る。既存の作り方（`pm-work/issue-drafts/wave1be/229-separate-wasm-suspend-js-frames.md`・`wave1as/183-preempt-depth2-dispatch-priority-default.md`）: `node tools/ab/base.mjs "$(git merge-base HEAD origin/main)" .tmp-base` の後、`.tmp-base` で `npm ci`・`npm run build`・`cargo build --release`・`fe-build engine`・`fe-build bundle` を行う。`node tools/ab/base.mjs --verify .tmp-base` で、リンクした入力が書き換えられていないことを確かめる。
- **changed の bundle**: 作業の木で `fe-build engine`・`fe-build bundle`。
- 時間（推定）: 1 回のビルドは 30 分台（`pm-work/findings/perf-windows-browser/README.md` の「35 分の再ビルド」の記録から）。base と changed で 2 回。A/B の run は p3 が約 3 分 × 6、p2 が約 1 分半 × 6（推定）。**1 回の試行で (d) まで届く見込みは低い**（推定）。

### 3.2 確認できないこと

- 実行環境の google-chrome の版と、この runner での p2・p3 の値（PR 本文の値とは別）。
- `fruity_delay2` を Wasm にした物の音（耳の確認は行わない。道具の値だけ）。

## 4. 起動の時間と規則（PR 本文と道具）

- PR 本文: 起動（warm）中央値 12.2 秒。P1 の目標は 10 秒以内。受け入れの上限は 20,000 ms（#1525 の見本）。
- 規則の検査: `guard/scan.sh` は、この段では未実施（取り込み後に当てる）。`guard/flags.sh origin/main FETCH_HEAD` は 0 件（実測）。

## 5. 作業の範囲（この所見が決めたこと）

1. 群の分けは、§1 のとおり。ファイル単位で `git checkout FETCH_HEAD -- <path>` で取り込めるのは、低と中の大半。混在の 3 件（`fle_sys.c`・`fle_win.c`・`worker.ts`）は、ハンク単位で取り込む。
2. **Hz の上限（`hz.ts`・`hz.test.ts`）は取り込まない。**
3. 計時器の枠（§2）は、(高) の前に、試験で赤を出してから直す。
4. P3 の A/B（§3）は、(高) の取り込みの後で行う（P2 の A/B は、(中) の後）。
5. PR #1555 は閉じない・触らない（依頼者が後で閉じる）。

## 6. 確認できなかった物（まとめ）

- `fle_x87.c`・`lower.rs`・`lib.rs`・`hybrid.rs` の正しさ（読んでいない。試験で確かめる）。
- `fle_gdi.c`・`fle_raster.c`・`fle_desk.c`・`fle_sse.h` の全行（読みは一部。試験の結果で確かめる）。
- `fruity_delay2` の Wasm 化と、`elastique` の seed の音への影響。
- 描画の要求の往復（`'want'` の応答の省略）が止まらないこと（読みでは正しそうだが、実行で確かめていない）。
- `desktop.ts` の矩形キャッシュの古さ（レイアウトの変化の経路）。
- `CM_ACTIONUPDATE` の間引きが、メニューの表示に与える遅れ。
- P3 の A/B と、P2 の A/B の値（この作業では実行していない）。
- PR 本文の検証（12.2 秒・流れ A〜D）の再現。

## 7. 起票の結果

- Issue #1556（下書き 269。`pm-work/issue-drafts/wave1ca/269-perf-pr-1555-import-with-ab.md`）。Task・priority High・size XL・`blocked_by` なし・親 #1305（下書き 005）・マイルストーン M2（#1548 の下書きと同じ）・ラベル hidpi・verify。
- 確認(読み取りのみ): Issue の状態 OPEN・Type Task・Priority High(Issue field)・Size XL(プロジェクト)・pm-hold は外れている。`pm-work/created-issues.tsv` に 269 の行を追記した。
- 作業ツリーに残した物: 本文・所見。`.tmp-*` の作業用の物は消した。
