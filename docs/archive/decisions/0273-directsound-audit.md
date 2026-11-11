# 0273(mt3w、仮): DirectSound モデルの契約監査 — ミキサーの書き込みが『1 ブロック周期 → 3 ブロック周期(書き込み率 1/3)』に落ちる理由は、モデルの返す値ではなく、ゲストのミキサーの待ち方と『検知から次のポーリングまでの遅れ C』の 2 ブロックの崖

基準: 現在の木(main 0b228624 相当。`lift/runtime/fle_dsound.c` 1474 行、`fle_audio_ring.c/.h`、`fle_audio_transport.c`)。作業: mt3w、2026-11-06 JST 09:30〜。**実エンジンの走行は 0 件(私の測定として)、ブラウザの測定は 0 件、native N1 の観測は私の測定としては 0 件(決定 0274 の alpha1 が実エンジンを native で走らせた表を §7 に引用する)、依頼者の機械は 0 件、実機 Windows との照合は 0 件**。§1〜§6 の数は、(a) runtime のソースと保存済みの Learn の 53 ページ(`lift/out-git/audio1-results/learn/`)の読み、(b) 実際の `fle_dsound.c`・`fle_audio_ring.c` を native で -O0 にビルドしたシナリオ(`tools/ds_audit.c`)の仮想時間の実行、(c) gpu1 の既存の測定(`lift/out-git/gpu7-results/`、本番 ff5ee429 の場面 b の 60 s)の読み直し、だけ。

## 冒頭の 3 点
1. **測ったこと(事実)**:
   - `fle_dsound.c` の位置・Lock/Unlock・Play/Stop/SetCurrentPosition・通知と、`fle_audio_ring` の消費位置・リードを Learn と 1 行ずつ照合した(§2 の表、23 行)。**ゲストの `TMixerThread_DS.Execute`(`f_12aeec0`)を逆アセンブルし直し**、待ち方を確認した(§1): `Sleep(1)` + `GetCurrentPosition(&play)` を、`half = ((play + lead) mod total) < block` が前回の値と変わるまで繰り返し、変わったら『play の入っていない半分』を書く。ゲストの `lead`(`[0x16bd4ac]`)は `round(total × [obj+0x3c0] × 係数)` で**モデルの値ではなくゲストの設定由来**(既定値は未確認、audio1 の読みは 0)。
   - 実際のモデル(`fle_dsound.c` + `fle_audio_ring.c`)を仮想時間で動かし、ゲストのポーリングのミキサーを写したシナリオ(`ds_audit.c` の `audit_mixer`)で、『検知してから次のポーリングまでに C ms かかる』場合の周期を測った(`tables/model-period-vs-compute-time.txt`)。**C が 190 ms 以上(2 ブロック = 186 ms を超える)と、書き込みの周期は 279 ms(3 ブロック)、`first` の差は 100% が 12288、書き込み率は 0.334**: gpu1 の本番の測定(30〜50 s の窓: 周期の中央値 279〜283 ms、`first` の差 0 件が 4096・34 件が 12288、書き込み率 0.325〜0.334)と一致。C が 150 ms なら 0.502、100 ms なら 0.858、80 ms 以下なら 1.000。
   - **この遷移は DS モデルのパラメータに依らない**: 書き込みのリード(60 ms と 15 ms)、ポーリングの間隔(Sleep(1) を 1・2・4・8・16 ms に)を変えても、C が 190〜250 ms の窓は 0.334 のまま(`tables/model-poll-interval.txt`)。リードを変えると変わるのは、ゲストの『write カーソルが書いた半分に入った』検査の発火だけ(`f_bd68e0(1,0)` は UI へのメッセージの post で、書き込みの周期には作用しない)。
   - **遷移はブロック長に比例して動く**: ゲストの設定 `Buffer length (DS)`(既定 4096 フレーム)を 8192 にすると、C = 200〜250 ms でも書き込み率は 0.87〜0.60(4096 では 0.334)。12288 では C = 250 ms まで 0.998(`tables/model-block-length.txt`)。
2. **結論**:
   - **契約と違う点で、3 ブロック周期を説明するものは無い**。照合表の 23 行のうち、書き込みの周期に効くのは『write カーソルのリード』(S-audio-lead = 60 ms、Learn は『typically about 15 ms』)と『ポーリングの間隔』(kernel32 の Sleep、0266)だけで、どちらも C の窓を動かさない(上の測定)。リードが 60 ms であることの実効は、ゲストの underrun 検査が発火する遅れの閾値が 33 ms(15 ms なら 78 ms)になること。
   - **1/3 は『C ≥ 2 ブロック』の崖**: ゲストの `half` の比較は、検知から次のポーリングまでに play が偶数個の境界を越えると変化を見逃し、次の境界まで待つ(C ∈ [2, 3) ブロックなら周期が 3 ブロック)。C が 186 ms を切るだけで書き込み率は 0.334 → 0.502 に跳ねる(階段)。gpu1 の 0〜10 s(0.73)・10〜20 s(0.49)・30〜50 s(0.33)・50〜60 s(0.46)は、この階段の上を C が動いた形に読める。
   - **C の中身は DS モデルではない**(検知 → EnterCriticalSection → ミキサー本体 `f_d0ece0` → Lock → Unlock → 次のポーリング)。gpu1 の測定では、Lock → Unlock の計算は平均 0.5〜0.9 ms(最大 31 ms。`gpu7-results/logs/p3q-base/audio-p3.json` の `unlockCompute` を窓ごとに読み直した)、ミキサースレッド(t18)は 976〜1012 スライス/s・平均 0.54〜0.61 ms で Worker の壁時計の 54〜62%(`tables/p3q-base-unlocks.md`。同じ形の別の run `thread-slices-r60c` では 94,338 スライス/99 s・平均 0.61 ms・57.9%)= ほぼポーリング。**ミキサー本体 `f_d0ece0` の時間(Lock の前)は測られていない**。C が 190 ms を超える内訳(本体の計算、CS の待ち、他スレッドのスライス)は未確認。
   - 直す差分: **fle_dsound.c / fle_audio_ring.c に契約の逸脱は見つからず、直す差分は無い**。テスト用の道具として `audit_mixer` のシナリオ(`0273-ds-audit-scenario.diff`、`lift/harness/audio/ds_scenarios.c` に +77/−1)を添える。
3. **未確認**: (i) ブラウザの C の実測(検知から次のポーリングまで。内訳)。(ii) ~~native N1 で 3 ブロック周期になるか~~ → **alpha1 の native N1(決定 0274、実エンジン・仮想時間)で確認された(§7)**: C(最後のポーリング → Lock)の中央値は再生中 0〜3 ms の間 1 ブロック周期(書き込み率 0.994)、C=165 ms で周期 180 ms(0.49)、C=264 ms で周期 274 ms = 2.95 ブロック(0.34)。私の予測(C ≥ 約 186 ms で 3 ブロック周期)と一致。ブラウザの値は未測定のまま。(iii) ゲストの `lead`([obj+0x3c0])の実行値と、`Buffer length (DS)` の設定の置き場(レジストリ/ini のキー。この監査では特定していない)。(iv) 実機 Windows の write − play の値列(Learn は『typically about 15 ms』しか書かない)。(v) ゲストの underrun の UI メッセージ(`f_bd68e0(1,0)` → `f_666780`)が UI スレッドに足す費用。

## 1. ゲストのミキサーの判断(`f_12aeec0`)と、そこに入る入力
ポーリングの形(`audio-m5.md` §4.2 の読みを今回の逆アセンブルで確認):
```
repeat: Sleep(1); EnterCS; GetCurrentPosition(&pos, NULL); pos = (pos + lead) mod total; half = (pos < block); LeaveCS
until half != prev ; prev = half
off = half ? block : 0        ; 書く半分 = play の入っていない半分(lead = 0 のとき)
EnterCS; f_d0ece0(...)        ; ミキサー本体(1 ブロック分)
Lock(off, block); copy; Unlock
GetCurrentPosition(NULL, &write); if (write ∈ [off, off + block - 1]) { 計数; f_bd68e0(1, 0) }   ; underrun の検査
LeaveCS; (ループの先頭へ)
```
- 入力は 3 つ: `GetCurrentPosition` の play(半分の判定)、write(underrun の検査だけ)、ゲスト自身の設定(`lead` = `round(total × [obj+0x3c0] × k)`〔`0x12af6af`〜`0x12af6d5`〕、block = `[0x16bd4a8]`、total = `[0x16bd4a4]`、`Buffer length (DS)` = `[0x16bd4a0]`)。**バッファに何ブロック書くか**は、この判定が決める: 毎回 1 ブロックで、書く間隔は『`half` が変わるまで』。
- `f_bd68e0(1, 0)`(`0xbd68e0`): `if (flag[cmd]) { flag[cmd] = 0; f_666780(obj, 0, cmd, arg) }` = コマンドを 1 つ渡す呼び出し(`f_666780` の中身は読んでいない。UI へのメッセージと読む)。旗で合体するので、underrun が毎ブロック起きても post は旗が立っている間に 1 回。**書き込みの周期には作用しない**。
- **崖の仕組み**: 検知から次のポーリングまでの遅れ C の間に、play は C/ブロック個の境界を越える。奇数個なら次のポーリングで `half` が前回と違い、すぐ書く(周期 = C)。**偶数個(C ∈ [2, 3) ブロックなど)だと `half` が前回と同じに戻っており、見逃して、次の境界まで待つ**(周期 = 3 ブロック)。

## 2. 照合表: `fle_dsound.c`・`fle_audio_ring` ↔ Learn(23 行)
判定: **一致**(Learn の文に一致)、**選択**(Learn に値が無い、`S-` の出典なしの選択)、**逸脱**(Learn の文と違う)、**不明**(Learn が書かない)。『周期への影響』= §1 の判断に効くか(測定 = シナリオ、読み = ソース)。行番号は `fle_dsound.c`。

| # | 項目 | 実装 | Learn(ページ ID) | 判定 | 周期への影響 |
|---|---|---|---|---|---|
| 1 | play カーソルの値 | `cursors()` 902〜929: `abs_pos_at(consumed)` を size で割った余り × nBlockAlign。『出力が今鳴らしている次のフレーム』 | Play and Write Cursors(ee418744): 『always points to the next byte of data to be played』 | 一致 | 無し(これが `half` の入力。モデルの値は単調で境界を越えるたびに 1 回変わる) |
| 2 | play カーソルの更新の粒度 | native: 仮想時計の 100 ns 刻みから計算。ブラウザ: `fle_js_consumed()` = AudioWorklet の RD(`Atomics.load`)。RD は 128 フレーム(2.9 ms)ごと | 同ページ『DSBCAPS_GETCURRENTPOSITION2 で、できるだけ正確に』(FL は `GETCURRENTPOSITION2` を付ける: audio-m5 §2) | 一致(粒度は Learn に値が無い) | 検知の遅れ ≤ 2.9 ms。崖には効かない(poll 16 ms でも崖は動かない: `model-poll-interval.txt`) |
| 3 | write カーソル | `cursors()`: `abs_pos_at(max(pulled, consumed))`。`pulled` = 出力に渡した先頭のフレーム。呼ぶたびに `audio_sync()` が consumed + lead まで渡すので、write − play = lead(定数) | GetCurrentPosition(mt708925): 『always leads the play cursor, typically by about 15 milliseconds』。Play and Write Cursors: 『the block between the play cursor and the write cursor is already committed』 | **選択**(S-audio-lead = 60 ms。『typically』なので違反ではないが 4 倍) | 周期に無し(シナリオ: リード 15/60 で 190〜250 ms の窓は同じ 0.334)。underrun 検査の閾値だけ: 60 ms なら play が半分の頭から 33 ms を過ぎてから書き終わると発火(15 ms なら 78 ms) |
| 4 | write カーソルが play の後ろに来ない | `max(pulled, consumed)` | 同上(write は play より先) | 一致 | 無し |
| 5 | 両方の出力ポインタが NULL でもよい | 945/946: `if (R[2])`、`if (R[8])` | GetCurrentPosition: 『can be NULL』 | 一致 | 無し |
| 6 | 協調レベル未設定の `GetCurrentPosition` | 常に `DS_OK` | GetCurrentPosition: 戻り値に `DSERR_PRIOLEVELNEEDED` を挙げる | 逸脱(小) | 無し(FL は `SetCooperativeLevel` の後に呼ぶ: audio-m5 §2) |
| 7 | 読み取りが仮想時計を進める | `read_tick()` 931: `FLE_CLOCK_READ_TICKS` = 10 ticks = 1 µs | (Learn に無い) | 選択(S-ds-pos-read-tick。ポーリングだけのループを終わらせるため) | 無視できる(1 µs) |
| 8 | プライマリのカーソル | 0 / 0(907) | (Learn に無い) | 選択(S-ds-primary-cursors) | 無し |
| 9 | 止まったバッファのカーソル | play = write = 止まった位置(912) | Play and Write Cursors: 『When the buffer is stopped, the cursor remains where it is』(play の分) | play は一致、write は選択(S-ds-stopped-write) | 無し(ミキサーは止まったバッファを見ない) |
| 10 | 非ループの終端 | カーソルを size に丸め、バッファを止める(460) | GetStatus(ee418070): 『If this value [LOOPING] is not set, the buffer will stop when it reaches the end』 | 一致 | 無し |
| 11 | Lock の範囲・巻き戻し | `FROMWRITECURSOR` = write カーソル、`ENTIREBUFFER` = 全体、先頭へ巻き戻すと第 2 ポインタ(`pp2`/`pn2` が NULL なら巻き戻さない) | Lock(ee418073) の各項 | 一致 | 無し |
| 12 | Lock の範囲外 | offset ≥ size、bytes = 0、bytes > size は `DSERR_INVALIDPARAM`、二重 Lock は `DSERR_INVALIDCALL` | (Learn に無い) | 選択(S-ds-lock-range、S-ds-lock-twice) | 無し(FL は常に 1 ブロック = size/2) |
| 13 | Unlock のバイト数 | Lock より多い/別のポインタは `DSERR_INVALIDPARAM`。少なくてもよい | Unlock(ee418153): 『must specify the number of bytes actually written, which might be less than the size of the lock』 | 一致(検査は選択 S-ds-unlock-check) | 無し |
| 14 | 書いたデータが出力に渡る時点 | デバイスは `audio_sync()` のたびにバッファの**その時の内容**を consumed + lead まで読む。Lock → Unlock の区切りでは読む範囲を絞らない | Unlock: 『DirectSound uses these values to determine how much data to commit to the device』。Lock: 『must immediately call Unlock』 | **逸脱(小)** | 無し(ミキサーは Lock → Unlock の間で譲らない協調スレッド。他スレッドの `GetCurrentPosition` はその間に走らない) |
| 15 | commit 済み(play〜write)の領域への遅れた書き込み | `rewrite_unread()`: 出力がまだ読んでいなければ、guard 256 フレーム以降を現在の内容で上書きして渡し直す | Play and Write Cursors: 『Data between [play and write] may already have been queued for playback by DirectSound and should not be touched』(定義されない) | 選択(S-ds-late-write、決定 0241、S-ds-rewrite-guard) | 周期に無し。実機より寛容(実機では失われる) |
| 16 | Play | LOOPING はプライマリで必須、優先度 0、再生中の Play はフラグを更新、未設定の協調レベルでは無音 | Play(ee418074) の各項 | 一致 | 無し |
| 17 | 再生中の `SetCurrentPosition` / 止まっていた Play の開始点 | 新しい位置は直ちに play カーソルの値になるが、出力に渡し済みのフレーム(最大 lead)を鳴らし終わるまで**カーソルは動かない**(`restart_geometry`: `base = max(write_frames, consumed)`) | SetCurrentPosition(ee418076): 『If the buffer is playing, the cursor immediately moves to the new position and play continues from that point』 | **逸脱(小)**: 動き出しが最大 lead 遅れる | 定常に無し。FL は起動時の `SetCurrentPosition(block); Play(LOOPING)` の 1 回だけ(`write_frames == consumed` なので遅れは 0) |
| 18 | Stop | 渡し済みのフレームを鳴らし、カーソルはその次のフレーム(S-ds-stop-drain)。`DSBPN_OFFSETSTOP` を立てる | Stop(Learn の記述はコメントに引用) 『the sample that follows the last sample played』 | 一致 / 選択 | 無し |
| 19 | GetStatus | PLAYING、LOOPING。Play/Stop の直後に即反映 | GetStatus: 『Because of latency, a call to Play or Stop might not immediately change the status』 | 一致(許される範囲) | 無し |
| 20 | 通知(`SetNotificationPositions`) | play カーソルがオフセットを越えたらイベントを立てる。多くの周回は 64 周に合体 | DSBPOSITIONNOTIFY(ee416819)、SetNotificationPositions(ee418245) | 一致 | 無し(この構成のゲストはポーリング形式) |
| 21 | エンジンが遅れて出力が先に進んだ時 | 外部消費者: 遅れたフレームは捨て(無音を鳴らし)、write_frames を consumed に進める(`late_frames`) | (Learn に無い。実機はバッファを鳴らし続けるので、ループなら古い内容が繰り返される) | 選択(名前のない処理: `audio_sync()` 429〜436 のコメント) | 周期に無し(カーソルは時間で動く)。音には効く |
| 22 | 消費位置の出どころ | 1 本: `fle_audio_consumed()`(native = 仮想時計、ブラウザ = AudioWorklet の RD) | (Learn に無い) | 選択(決定 0139 の構造) | **これが C を作る**: ブラウザでは play が実時間で進むのに、エンジンの計算も実時間を食う(§3) |
| 23 | ポーリングの間隔 | `Sleep(1)` は仮想 1 ms ちょうど(kernel32 の契約: `timeBeginPeriod(1)` で 1〜2 ms、既定のタイマー分解能は約 15.6 ms。決定 0266) | Sleep(Learn) | 選択(0266) | 崖に無し(間隔 1・2・4・8・16 ms で同じ。`model-poll-interval.txt`)。**CPU には効く**: 本番の測定でミキサーのスレッドが Worker の壁時計の 54〜62% を使い、スライスは 976〜1012 回/s(ほぼポーリング) |

## 3. シナリオ(`tools/ds_audit.c`、`tools/build.sh`)と結果
- **道具**: 実際の `fle_thread.c`・`fle_dsound.c`・`fle_audio_ring.c`ほか 20 ファイルを native -O0 で、`lift/harness/audio/ds_scenarios.c`(audio1 の既存のシナリオ集)に `audit_mixer <秒> <C ms>` を足した物とリンクする。仮想時計で決定的。ゲストと同じ手順(Lock(ENTIREBUFFER)→Unlock→SetCurrentPosition(block)→Play(LOOPING)、`Sleep(poll)` + `GetCurrentPosition` + `half` の比較、Lock、**C ms を仮想時計に課金**、Unlock、write の検査)。環境変数 `FLE_AUDIO_LEAD_MS`・`AUDIT_POLL_MS`・`AUDIT_BF`(ブロック長)。出力 = 周期の中央値と p10/p90、連続する Unlock の `first` の差(4096 = 連続 / 12288 = 2 ブロック飛ばし / その他)、書き込み率、underrun 検査の発火数。
- **C を掃引**(リード 60 ms、30 s の仮想時間、ブロック 4096 フレーム = 92.9 ms): 

| C (ms) | 周期の中央値 (ms) | `first` の差: 4096 / 12288 | 書き込み率 | gpu1 の本番との対応 |
|---|---|---|---|---|
| 0〜80 | 93 | 291〜296 / 0 | 0.997〜1.000 | (native の仮想時間で計算が C に入らなければここ) |
| 100 | 101 | 231 / 17 | 0.858 | — |
| 120 | 121 | 149 / 48 | 0.669 | |
| 140〜180 | 141〜181 | 73〜59 / 72〜58 | 0.502 | 10〜20 s の窓(0.49)と同じ値 |
| 190〜250 | 279 | **0 / 93〜97** | **0.334** | 30〜50 s の窓(周期 279〜283 ms、0 / 34、0.325〜0.334) |
| 300 | 301 | 0 / 68 | 0.294 | |
| 330 | 411 | 0 / 39 | 0.251 | |
  リード 15 ms でも 190〜250 ms の行は同じ(`first` の差、周期、書き込み率が同一)。違うのは underrun 検査の発火数だけ。
- **ブロック長を掃引**(リード 60 ms、60 s): C = 250 ms で 4096 フレームは 0.334、6144 は 0.499、**8192 は 0.601**、12288 は 0.998、16384 は 0.997。C が 200 ms なら 8192 で 0.867。= 崖は 2 ブロックの位置にあり、ブロックを長くすれば同じ C で崖の上を保てる。

### 3b. gpu1 の測定から C を粗く見積もる(読み。測定ではない)
- gpu1 の窓ごとの値(`p3q-base-unlocks.md`): ミキサースレッドの時間 = 窓の 57〜62%(5.7〜6.2 s/10 s)、スライス 976〜1012/s、Unlock(ブロック)は 0〜10 s が 79 回、30〜40 s が 35 回。時間 = ポーリング(スライス数 × 1 回の費用 c_p)+ ブロック数 × ブロック本体の費用 B と置くと、c_p を 0.14 ms とすれば B は 0〜10 s で約 54 ms、30〜40 s で約 134 ms(c_p = 0.55 ms なら B は 4 ms と 17 ms)。**c_p が分からないので B は決まらない**(上の 2 組はどちらも 2 窓の合計時間と矛盾しない)。
- したがって、C = 190 ms 以上の内訳(本体の計算 B、ミキサー用 CS の待ち、他スレッドの協調スライス、UI への同期メッセージ)は**この監査では分けられない**。分かる範囲で: (1) ゲストのミキサーは高い優先度のスレッドで、実機では Sleep(1) の期限が来れば UI スレッドの途中で割り込む。このモデルの協調スレッドは優先度を無視する(決定 0122)ので、UI の 1 スライス(gpu1: 平均 6 ms、500 ms ごとの平均の最大 19.9 ms、累計の最大は 8.5 s)が終わるまでミキサーの wake が待つ。これは kernel32 のスレッドの契約の側の差で、DS の契約ではない。(2) ミキサー本体は `f_d0ece0`(Lock の前)で、gpu1 の `unlockCompute`(Lock → Unlock)には入らない。測る価値が最も大きいのは、この本体の時間と CS の待ちの時間。

## 4. 結論の使い方(PM の判断材料。実装はしていない)
- 1/3 の天井を動かすには、(i) C を 186 ms 未満に下げる(= ミキサー本体の計算・CS の待ち・他スレッドのスライスの合計を減らす。0270 の Elastique の直 wasm 化は、本体の計算を減らす側)、または (ii) **ゲストのブロック長を伸ばす**(設定 `Buffer length (DS)`、既定 4096。FL を実機で遅い PC に使う利用者が伸ばす設定そのもので、モデルの変更ではない。代償 = 音の遅れが 1 ブロック 93 ms → 8192 で 186 ms、12288 で 279 ms。再生位置の表示は `GetCurrentPosition` の補間なので追従する〔読み〕。設定の置き場は未特定)。
- ポーリング自体の費用(Worker の約 58%)は、Sleep の丸め(間隔 2 ms)で半減できる見込みで、シナリオでは音に影響しない(間隔 1〜16 ms で崖が動かない)。ただし 0266 は kernel32 の領分で、DS の契約の話ではない。
- 次の測定(C の内訳): ブラウザの Worker で、ミキサーのスレッドの『`GetCurrentPosition(&play)` が半分の変化を返した時刻』と『その次の `GetCurrentPosition(&play)` の時刻』の差(= C)を 500 ms ごとの pulse に足す(約 20 行の計測。**今回は入れていない**)。alpha1 の native N1 の API ログでも同じ 1 つの数が取れる。

## 5. 差分と道具
- `0273-ds-audit-scenario.diff`(90 行、`lift/harness/audio/ds_scenarios.c` に `audit_mixer` を足す。`-` は 1 行 = シナリオの表の行)。PM の木のコピーに `patch -p1 --dry-run` が offset/fuzz なしで通る。
- `tools/`: `build.sh`(シナリオのビルド。`nice -n 15`)、`sweep.sh`・`sweep2.sh`・`sweep3.sh`(C・ポーリング間隔・ブロック長の掃引)、`ds_audit.c`。`tables/model-*.txt` はその出力。
- runtime 本体(`fle_dsound.c`・`fle_audio_ring.c`)への差分は無い。

## 6. 未確認・限界・自己申告
- 私の測定としては実エンジン 0 件、ブラウザ 0 件、native N1 0 件(alpha1 の 0274 の表を §7 に引用)、実機 Windows 0 件。シナリオは**ゲストのミキサーの写し**(audio-m5 §4.2 の読みと今回の逆アセンブル)で、ゲストの実物ではない。実物のループの細部(`[0x15e4d60]` を他の関数が書くか、`Terminated`、時計の記録)は写していない。
- C を課金する場所(Lock の後)は、『検知から次のポーリングまで』の遅れの置き方の 1 つ。実物の C は ミキサー本体(Lock の前)+ CS の待ち + 他スレッドのスライスで、ブラウザの値は未測定。gpu1 の測定との一致は『周期・`first` の差・書き込み率が C ∈ [190, 280) ms の窓の値と同じ』という形の一致で、C の値そのものの確認ではない。
- ゲストの `lead`([obj+0x3c0] 由来)の既定値は読んでいない(audio1 は 0 と読んだ。`audio-m5.md` §…『実行では見ていない』)。0 でなければ半分の判定の位相が変わるが、崖(C が偶数個の境界を越える)の位置は変わらない〔読み〕。
- 自己申告: (a) 作業の最初に、`lift` の下を再帰 grep して 120 秒のタイムアウトに当たった(不要な負荷)。(b) `while read` の空ループを 1 回書いた(0270 の後片付けの一覧。禁止に抵触)ことは 0270 の最終報告で申告済み。(c) 0275 の作業中に、存在しない `bash` という名前のツールを 1 回呼んだ(結果は shell として実行された。0270 で 4 回申告済みの再発)。

## 7. native N1(alpha1、決定 0274)との照合 — 2026-11-06 追記

出典: `lift/out-git/alpha12-results/tables/native-mixer-cycle-per-10s.tsv` と `native-unlock-per-10s.tsv`(**実エンジン**を native -O2 で走らせた仮想時間の観測。私の測定ではない。区間 = 実行開始からの仮想秒で、再生を押すのは約 25 s)。C = 最後の `GetCurrentPosition` → 次の `Lock`(ミックスの区間)、W = `Unlock` → 最後のポーリング。

| 区間 (s) | cycles | W 中央値 (ms) | C 中央値 (ms) | cycle / block | 書き込み率 | `first` の差 |
|---|---|---|---|---|---|---|
| 10〜30 | 64〜106 | 90 | 3.0 | 1.00 | 0.59〜0.994 | 4096 |
| 30〜40 | 54 | 11.0 | 165.4 | 1.94 | 0.492 | 4096: 27 / 12288: 26 |
| 40〜50 | 52 | 11.0 | 165.4 | 1.96 | 0.483 | 12288: 26 / 4096: 25 |
| 50〜60 | 36 | 12.0 | 264.5 | 2.95 | 0.344 | 12288: 30 / 4096: 4 |
| 60〜70 | 36 | 5.0 | 267.3 | 2.96 | 0.325 | 12288: 23 / 20480: 6 / 4096: 6 |
| 70〜100 | 43〜44 | 1〜28 | 190〜207 | 2.4〜2.5 | 0.40 | 12288 が多数 |

- **予測は当たった**: 仮想時間だけの native でも、C が 186 ms(2 ブロック)を超える区間で周期が 3 ブロック(0.34)に落ちる。C=165 ms は 0.49(私のシナリオの C=140〜180 ms の行 0.502 と同じ段)、C=264 ms は 0.344(C=190〜250 ms の行 0.334 と同じ段)。**= ブラウザの pacing でも DirectSound のモデルでもなく、エンジン内の『ミックス区間の仮想時間』が原因**(alpha1 の読みと同じ)。
- `write − play` は全区間 2646 フレーム(60 ms)で一定(alpha1 の追跡)=『契約の逸脱は見えない』という §2 の結論と整合。
- C の中身(ミックス本体の仮想 ms を何が作るか)は決定 0275 に回す。

## PM の決定と取り込み(2026-11-06 10:25、docs のみ。差分は製品の木に当てていない)

取り込み: mt3 の 0273 最終(表・道具・シナリオ差分)を `docs/investigations/mt3w/0273/` に置いた(差分 `patches/0273-ds-audit-scenario.diff` は `lift/harness/audio/ds_scenarios.c` へのテスト用の追加だけ)。測定は runtime のソースと保存した Learn 53 ページの読み、native -O0 の仮想時間のシナリオ(ゲストのミキサーの写し)、gpu1 の既存データの読み直しで、**実エンジン・ブラウザ・実機 Windows・依頼者の機械は 0 件**。

判断:
1. **DirectSound のモデル(`fle_dsound.c` / `fle_audio_ring`)に、書き込みの周期へ効く契約の逸脱は無い**(照合 23 行: 一致 12 / 選択 8 / 小さな逸脱 3)。runtime への修正差分は無い。
2. **3 ブロック周期は『mix 区間 C(検知 → 次のポーリング)が 2 ブロック = 186 ms 以上』の閾値**(C = 190〜250 ms で周期 279 ms、書き込み率 0.334)で、**alpha1 の native N1(決定 0274)の実測が予測どおり(C 3 ms → 165 → 265 ms)だった**(§7)。C の中身はミキサー自身の課金で、DS モデルではない。決定 0275 と 0279 がこの先を扱う。
3. 未確認: ブラウザでの C の内訳、ゲストの lead の実行値、「Buffer length (DS)」設定の置き場(利用者設定で伸ばせば閾値が動く。代償は遅延)。
