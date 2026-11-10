# 0295(仮): DirectSound のカーソルの契約と、FL の『遅れ』判定の native での再現 — FL は write cursor で遅れを検知するが、検知は Lock の位置とペースを変えない(mt3)

状態: **仮**(測定と読み。runtime の差分案が 1 つ: §6b、`cursors()` だけの小さな差分、E12 候補)。作業者 mt3、2026-11-06 JST。依頼: PM(0295)。基準: main 7373cd55(`lift/runtime` は 9fdc492f〔E10〕から変わっていない〔`git diff --stat 9fdc492f 7373cd55 -- lift/runtime` が空〕)。区分: **実測**=この作業の出力を見た値、**読み**=資料・コードを読んだ推論、**未確認**。

## 訂正(最初の報告の誤り。この節は訂正の記録で、本文の結論は訂正後の物)

1. **誤り**: 最初の報告(0295 報告 1)と、その時の本文・README の行の結論『FL は遅れを判定していない』『FL は write cursor を使わない』は**誤り**だった。
2. **原因(私の誤り)**: (a) write cursor を無効な値(0x7ffffff0)にした run(wrjunk)で Lock の列・+3 の数・書き込み率が完全に同じだったことから『FL は write を読まない』と結論した。この実験が示すのは『write の値は Lock の位置とペースを変えない』ことだけで、『読まない』ことは示さない。(b) FL の規則を観測だけから導き、本文に『逆アセンブルで直接は読んでいない』と書きながら、コードで確認せずに結論を言い切った。(c) Unlock と同じ仮想時刻の `GetCurrentPosition` を『判断に使われない priming』と解釈した。実際は write cursor の検査(下)だった。(d) 同じ project の `lift/harness/audio/ds_scenarios.c`(`rt_mixer`、587〜639 行)と `wall_fr15.rs`(444〜467 行)に『the engine's underrun check: the write cursor inside the half just written』が既にあり、それを見ていなかった。
3. **見つけ方**: FLEngine_x64.dll のミキサー(0x12aeeda〜0x12af266)を `objdump -d -M intel` で読み、さらに戻り先アドレスを記録する run(retaddr-40)で、トレースの呼び出し 4 点〔ポール 0x12aef14、Lock 0x12af127、Unlock 0x12af1a1、Unlock 直後の write の読み 0x12af1e8〕がコードの該当箇所と一致することを確認した。
4. **訂正後の結論**: FL は Unlock の直後に write cursor を 1 回読み、write cursor が『今書いた半分』の中にあれば underrun として UI のカウンタを +1 し、通知 `0xbd68e0(1,0)` を出す(§1・§2)。**この検知は Lock の位置・待つ/待たない・+3 の数・書き込み率を変えない**(数値表は訂正の前後で同じ: wrjunk A/B)。変わるのは『検知の頻度がリードで決まる』こと(§6)と、その結果の runtime の差分案(§6b)。
5. **影響**: 数値表(§3〜§4b、§6 の Lock の列・first の差・書き込み率・古い内容で再生された割合)は変わらない。変わるのは FL の規則の記述(§2)、Learn との対応(§5 の 2・3・7 行と新しい 12 行)、§1 の結論 1・4・6、§7 の gpu1 0289 の読みの注記。gpu1 には訂正を送った。

## 0. 冒頭の三点(正直さ)

1. **何を実行したか(実エンジン)**: native N1 = lift 済みの FLEngine と lift 済みの 7 module(dsp_ippv2・Blend2D・QuickFontCache・FreeType・ILTools・oggio・Elastique)を native に build した実体を、G3 の模擬入力(**ユーザーの操作ではない**。参照プロジェクト `serina1-serum2.flp` を開いて再生。`a12-play.g3` と `a12-play40.g3`)で走らせ、FL の `TMixerThread_DS` が DirectSound に行う `Sleep`・`GetCurrentPosition`・`Lock`・`Unlock` を全部、仮想時刻つきで記録した(alpha1 の追跡 `A12_DSTRACE`。製品に入れない)。生成 C の object は alpha1 の gen3(b42fad36)、runtime は alpha1 の t12h 木(**`fle_dsound.c` は main の物と追跡の行が違うだけ**〔diff 20 行は全部追跡〕、`fle_audio_ring.{c,h}` は main と同一)。15 回の run(40 s × 12〔再現性の確認 base40 ×3 を含む〕、100 s × 3。壁時計で 48〜109 s)。**ブラウザ・実 Windows・依頼者の機械の値は 0 件**。native の仮想時計は壁時計に歩調を合わせないので、時間はすべて仮想時間(実時間ではない)。native に VST(Serum 2 など)は無い。
2. **新しいこと**: alpha1 は 0274 で同じ追跡を取り『契約の逸脱は見えない』と書いた。ここで足したのは (a) FL のミキサーの規則を **FLEngine のコード(逆アセンブル)で読み**、全 Unlock で検証したこと、(b) 引数つきの表、(c) **契約に関わる 3 つの量を変えた感度実験**(write cursor のリード、カーソルの粒度、write cursor を無効な値にする)、(d) Learn との対応表。
3. **未確認**: 実 Windows の DirectSound が返す値(リード・粒度)の測定(計器が無い。下の対応表の『実 Windows』の列は Learn などの資料の記述だけで、**測定した値は 0 件**)、ブラウザの実スレッド版での同じ表(gpu1 の領分)、FL の通知 `0xbd68e0` の先(`0x666780` が UI 側へ何を投函するか。コードを読んだ範囲は §2)と、実ブラウザ・他の窓での表示(この場面の 3 つの surface の画素比較では、CPU メーターのグラフの一角以外に差は出ていない: §6b)。

## 1. 結論

(**この節は訂正済み**: 最初の報告の『FL は遅れを判定していない』『write cursor を使わない』は誤りだった。FLEngine のミキサーのコードを読んで、Unlock 直後の write cursor の検査を見つけた〔§2〕。)

1. **FL は write cursor で『遅れ』を検知するが、その結果は音のペースを変えない。** Unlock の直後に `GetCurrentPosition(NULL, &write)` を呼び、write cursor が『今書いた半分』の中にあれば underrun として UI のカウンタ(`[15e16d0]` を +1。オブジェクトの仮想関数 +0x88 が真のとき)と通知(`0xbd68e0(1,0)`)を出す。**Lock の位置・待つ/待たない・+3 の数は、この検知に依らない**: write cursor を無効値にして検知を全部止めた run(検知 0/400)と基準(検知 108/400)で、Lock の列・first の差の度数・書き込み率が完全に一致(§6)。
2. **ペースを決めるのは play cursor だけ**: Unlock の後、`Sleep(1)` + `GetCurrentPosition(&play, NULL)` を 1 ms ごとに繰り返し、play の『半分の flag(play < 半分)』が前回の起点の flag と**変わる**まで待ち、変わったらミックスして、`flag ? 後ろの半分 : 前の半分` を Lock・Unlock する。Lock の offset は 662 Unlock のすべてで 0 と 16384 の厳密な交互。ミックスの間(3〜338 ms)は `GetCurrentPosition` を呼ばない。
3. **『+3 飛び』は FL の判断ではなく、遅れた書き込みの位置が次の周回になる記録上の結果**(`first = consumed + ((領域の先頭 − play) mod size)` は『その領域が出力に読まれる絶対 frame』)。ミックスが 1 ブロック(4096 frame = 92.9 ms)を超えると、play cursor は書く半分を既に通り過ぎ、play の flag が既に前回と違うので FL は待たずに始め、また遅れる → **待つ周期(+3 = 12288)と待たない周期(+1 = 4096)が交互**(§3 の表 1)。ミックスが 2 ブロックを超えると flag が前回と同じに戻って、もう 1 周待つ(20480 = +5、100 s で 15 回)。
4. **契約**: runtime の `GetCurrentPosition` は Learn の記述に反しない(§5)。違いは 2 つ: (a) write cursor のリード(runtime 60 ms〔S-audio-lead〕、Learn 『typically about 15 ms』)— **FL の検知(1)の頻度を決める唯一の量**: 検知は『Unlock までの経過が play の半分越えから (4096 − リード frame) を超えた』で出る = リード 60 ms なら C > 約 33 ms、15 ms なら C > 約 78 ms。native(40 s)の検知: リード 15 ms で 77/401(19%)、60 ms で 108/400(27%)、120 ms で 332/400(83%)。**仮想 CPU 4 倍(音は全部間に合い、古い内容で再生された frame は 0)でも、リード 60 ms では検知が 380/623(61%)出る** = runtime のリード 60 ms は、実 Windows のリード(約 15 ms)なら出ない underrun の表示を FL に出させる(UI のカウンタ。音は同じ)。(b) カーソルの粒度(runtime 1 frame、実機は資料に数値なし): 10・50 ms 刻みにしても FL の構造は変わらない。
5. **+3 を減らせるのは『1 ブロックのミックスが 92.9 ms 未満になること』だけ**: 仮想 CPU 4 倍(C の最大 82 ms)で +3 は 0 回・古い内容で再生された割合 0.0%、2 倍で +3 が 44 回、基準の C = 157〜175 ms の区間では 100 s で 280 回。契約(リード)を 15 ms に寄せても +3 は 280 → 280 で減らない。
6. **runtime の差分案が 1 つ(§6b)**: +3 に対しては無い。**FL の underrun 検知を Learn の契約に寄せる案**: FL に返す write cursor だけを play + 15 ms(Learn: typically about 15 ms)にし、手渡しのリード 60 ms(S-audio-lead)は内部に残す(変数を共有しない)。native で A/B(vis15): **Lock の列・+3 の数・書き込み率・古い内容で再生された割合は基準と同一、FL の検知は 230 → 181(100 s)、108 → 80(40 s)**。UI の CPU(仮想 CPU の slice・全スレッドの間接呼びの数)は検知 0 の run でも案の run でも測定できる差が無い(±0.3% 以内)。画面は、再生前は 3 run で完全一致、再生中は CPU メーターのグラフの一角(約 84×20 画素)以外は一致(underrun の通知・カウンタが別の場所に出た形跡なし)。実 Windows の値は Learn の記述だけで**測定していない**。
7. 付記(読み・未測定): 0241 の S-ds-late-write(`rewrite_unread`)があるので、遅れた書き込みが取り込まれる期限は『領域の先頭の `REWRITE_GUARD_FRAMES` = 256 frame(5.8 ms)手前』で、write cursor のリードに依らない。S-audio-lead の費用と利得(『60 ms は停止を吸収する、15 ms は出力が欠ける』)は PM の決定のまま。

## 2. FL の規則(FLEngine のコードを読み、全 Unlock で検証)

`TMixerThread_DS`(FLEngine_x64.dll の 0x12aeeda〜0x12af266、`objdump -d -M intel`。逆アセンブルは `tables/fle-mixer-0x12aeeda-0x12af266.dis.txt`)。バッファ: 8192 frame(32768 B、align 4)、ループ再生、通知なし(`nnotify=0`)、半分 = 4096 frame = 16384 B。変数: バッファ `[16bd498]`、サイズ `[16bd4a4]`、半分の大きさ `[16bd4a8]`、前回の flag `*[15e4d60]`、Lock の offset `*[15e3138]`、UI のカウンタ `*[15e16d0]`。

```
loop (0x12aeeda):
    do { Sleep(1); GetCurrentPosition(&play, NULL) } while (play の半分の flag が前回の flag と同じ)       # 戻り先 0x12aef14。1 ms ごと
    Lock(offset = flag ? 後ろの半分 : 前の半分, 半分の大きさ)                                        # 戻り先 0x12af127。offset は 0 / 16384 の固定値
    4096 frame をミックスして書く                                                                      # C = 3〜338 ms。この間 GetCurrentPosition を呼ばない
    Unlock                                                                                            # 戻り先 0x12af1a1
    GetCurrentPosition(NULL, &write)                                                                  # vtable +0x20、戻り先 0x12af1e8。Unlock と同じ仮想時刻
    if (Lock の offset <= write <= Lock の offset + 半分の大きさ - 1):                                  # write cursor が『今書いた半分』の中 = 遅れ
        if (object [15e4790]->+8 の仮想関数 +0x88 が真) [15e16d0] += 1                                # UI の underrun カウンタ
        0xbd68e0(1, 0)                                                                                # 通知
    (SetEvent 相当 0x423980 など。次の周回)
```

`0xbd68e0(index, param)`(読み): 配列 `0x16b6dcc[index]` が立っているときだけ、立っている flag を落として `0x666780(ctx = [16b6e00], 0, index, param)` を呼ぶ『一度きりの通知』。`0x666780` は slot を取って値を詰め、`0x423980` を呼ぶ = UI 側のキューへの投函に見える。**UI がこれで何を描くかは読んでいない・測っていない**(§6b の UI 影響は間接的な測定だけ)。カウンタ `[15e16d0]` は `0x119324b` で UI の値に読まれる。

つまり FL の『遅れ』の判定は **Unlock の直後に write cursor を 1 回読んで、今書いた半分の中にあるか**。ペース(いつ書くか)は play cursor だけで決まる: play の flag が変わるまで 1 ms ごとにポールし、変わったらミックスして書く。検知の結果はペースにも Lock の位置にも戻らない(次の周回は検知の有無に依らず同じ規則)。

(『待たない』周期は、Unlock 直後の検査の時点で play が既に次の半分の外にいて、`Sleep(1)` の後の 1 回のポールで抜ける = トレースの G は 2 回〔検査 1 回を含む。186/187。残り 1 回は 95 回〕。『待つ』周期は次の半分の中にいる間ポールが続く。)

検証(`tools/ds95.py`・`tools/underrun95.py`、全 Unlock):
- Lock の offset が 0 と 16384 の厳密な交互: base40 400/400、base100 662/662、lead15-100 662/663(例外 1 回は最初の Play から 3.2 s 後の i = 34、スレッド 7 の Unlock で、base40 でもこの位置に first の差の外れ値 8315 がある)、lead120-40 400/400、q441 406/406、q2205 406/406、wrjunk 400/400、vis15-100 662/662。
- Lock の直前の最後のポールで play が『書く半分の外』: base40 400/400、base100 662/662、lead15-100 663/663、lead120-40 400/400、q441 406/406(この条件を満たさない Lock は 0)。
- `GetCurrentPosition` の間隔の中央値 1.001 ms(`Sleep(1)` + 読みの 1 µs)、p90 も 1.001 ms。
- **write cursor の検知(Unlock 直後の検査)はペースを変えない**: 返す write cursor を 0x7ffffff0 にして検知を全部止めた run(wrjunk。検知 0/400)で、Unlock 400 回・first の差の度数(4096: 289、12288: 99、20480: 7)・古い内容で再生された割合が基準(検知 108/400)と完全に一致。
- 戻り先アドレスを記録した run(retaddr-40)で、トレースの G/L/U の呼び出し位置が上の 4 点と一致(`tools/retaddr95.py`)。
- FL は play cursor の粒度に鈍感: 10 ms 刻み(441 frame)でも 50 ms 刻み(2205 frame)でも Lock の列は交互のまま、+3 は 94 回(基準 99)。

## 3. 表 1: FL が各 Unlock の前後に読む `GetCurrentPosition` の戻り値と FL の判断(引数つき。base40、Unlock 236〜240)

`tables/excerpt-args-1ms-base40-unlock236-240.tsv`(1 ms ごとの全イベント、175 行)と `excerpt-args-10ms-…`(10 回に 1 回)が元。下は 4 周期の要点。単位は frame(バイト ÷ 4)、バッファ 8192 frame、半分 4096。時刻は仮想 ms(最初の `Play` から)。write − play = 2646 frame(60 ms)で毎回一定。

| 周期 | event | t_ms | 引数 / 戻り値 |
|---|---|---|---|
| U237(待つ) | 前の Unlock | 30164.44 | play 1825 write 4471 consumed 1330251 |
| | Unlock 直後の write cursor の検査(Unlock と同じ仮想時刻。FL は `GetCurrentPosition(NULL, &write)`) | 30164.44 | write 4471(write−play 2646)。この Unlock(U236)は半分 1〔4096〜8191〕を書いたので 4471 は**その中 → 検知が出る** |
| | 起点のポール(play が書く半分 0 の外に出た) | 30216.49 | play 4121 write 6767(53 回目のポール。この間 `Sleep(1)` + 読みを 1 ms ごと) |
| | Lock | 30390.90 | `Lock(offset 0 B, bytes 16384)` → n1 16384 B, n2 0。**C = 174.41 ms**(起点のポールから) |
| | Unlock | 30390.90 | play 3620 write 6266 consumed 1340238 → first = 1344810、**dfirst = 12288(+3 ブロック)**、play が既に書いた領域に 3620 frame 入っていた(late 3620) |
| U238(待たない) | 前の Unlock | 30390.90 | play 3620 write 6266 |
| | Unlock 直後の write cursor の検査 | 30390.90 | write 6266。この Unlock(U237)は半分 0〔0〜4095〕を書いたので 6266 は外 → 検知なし。(play 3620 は FL が捨てる〔ポインタ NULL〕。次に書く半分 1〔4096〜8191〕の外にあるので、`Sleep(1)` の後のポール 30391.90 play 3664 で抜ける) |
| | Lock | 30561.30 | `Lock(offset 16384 B, bytes 16384)`。**C = 170.40 ms** |
| | Unlock | 30561.30 | play 2943 write 5589 consumed 1347753 → first = 1348906、**dfirst = 4096(+1)**。ただし play は C の間に書く半分 1 を**全部通り過ぎて**(3620 → 4096 → 8192 → 2943)いて、半分 1 の 4096 frame は古い内容で再生された(stale 4096) |
| U239(待つ) | 起点のポール | 30588.33 | play 4135 write 6781(28 回目のポール)→ Lock 30756.73 offset 0(C = 168.40 ms)→ Unlock play 3369 → dfirst = 12288 |
| U240(待たない) | 起点 | 30756.73 | play 3369(即)→ Lock 30914.13 offset 16384(C = 157.40 ms)→ Unlock play 2119 → dfirst = 4096 |

読み方: 書く半分の選択は『起点のポール』と『Lock の offset(次の半分)』の 2 つだけ。Unlock 直後の write cursor の検査は、この例では U236・U238・U240(半分 1 を書いた周期)で write が書いた半分の中に入って検知が出て、U237・U239(半分 0)では出ない(`tools/underrun95.py`)。ミックスの間(C)は `GetCurrentPosition` を呼ばない(表 1 の 30216.49 → 30390.90 の間に G は無い)。

## 4. 表 2: Unlock ごとの全表と要約

`tables/unlocks-<run>.tsv`(1 行 = 1 Unlock。列: i、th、t_ms、off、trig_ms、trig_play、C、play_L〔Lock の直前の最後のポールの play〕、write_L、age_ms、plU/wrU/cons〔Unlock の play・write・consumed〕、first、dfirst、late_f〔Unlock の時点で play が書いた領域の中にいた frame 数〕、lost_f〔起点のポールから Unlock までに play が通り過ぎた、書いた領域の frame 数 = 古い内容で再生された frame〕、npolls、dt_u、alt、waited)と `summary-<run>.txt`、`sensitivity-summary.tsv`(§6)。

基準 base100(100 s、仮想 126.5 s、Unlock 662): 書き込み率(Unlock したフレーム / 仮想秒 / 44100)0.486、first の差: 4096 が 360、**12288 が 280**、20480 が 15、-4096 が 2、その他 4(起動直後)。待った周期 475、待たなかった周期 187。C の中央値 168.4 ms、p90 264.4 ms、最大 337.9 ms。**書いた半分の 60.5% が、play が先に通り過ぎて古い内容で再生された**(全く間に合わなかった半分 304、一部 118、間に合った半分 240)。40 s の base40 では 38.4%。この割合は native の仮想 CPU の課金に依る値(alpha1 0274 の限界と同じ)で、ブラウザの値ではない。

### 4b. first の差と、実際に古い内容で再生された frame 数(lost_f)は別の量

`first` の差(+3 = 12288 など)は『書いた領域の**先頭**が出力に読まれる位置』の記録で、遅れの大きさではない。base100(662 Unlock)で `dfirst` と `lost_f`(起点のポールから Unlock までに play が通り過ぎた、書いた領域の frame 数)を数えた表(`tables/unlocks-base100.tsv`):

| dfirst \ lost_f | 0 | 1〜999 | 1000〜4095 | 4096(全部) |
|---|---|---|---|---|
| 4096(+1) | 233 | 0 | 8 | **119** |
| 12288(+3) | 0 | **1**(lost_f = 53、C = 93.11 ms) | 109 | 170 |
| 20480(+5) | 0 | 0 | 0 | 15 |
| -4096(-1) | 2 | 0 | 0 | 0 |

読み: (1) +3 の 280 回のうち 279 回は 1000 frame 以上(23 ms 以上)が古い内容で再生される大きな遅れだが、**1 回は C = 93.11 ms(ブロックより 0.2 ms 長いだけ)で、領域の先頭の 53 frame(1.2 ms)を通り過ぎられただけなのに +3 と記録された**(Unlock 465 → 466 の『+3 の次に -1』の 3 点セットの前半)。(2) 逆に、`dfirst = +1` でも 360 回のうち **119 回は 4096 frame(1 ブロック全部)が古い内容で再生**されている(『待たない周期』: 前の遅れの後で play は既に書く半分を通り過ぎつつあり、ミックス中に通り過ぎる)。→ **`first` の差だけから『+3 の 1 回 = 1 ブロックが無音』とは言えない**。取りこぼしの量は `lost_f`(または runtime の late_write の統計)で見る必要がある。(3) -1(-4096)は、遅れの直後にミックスが速く終わった周期(C = 63 ms、6 ms)で、前の遅れが先頭の 53 frame・2172 frame だけだったとき。

## 5. Learn の契約と runtime の対応表

出典は Microsoft Learn(`docs/investigations/audio1/learn/` の写し)。行は main の `lift/runtime/fle_dsound.c`(`fle_audio_ring.{c,h}`)。

| # | 契約 / 資料の記述(出典) | runtime の戻り値・動作(行) | 判定 |
|---|---|---|---|
| 1 | play cursor は『次に再生されるバイト』を指す(Play and Write Cursors〔ee418744〕) | `play = abs_pos_at(b, fle_audio_consumed())`、ループでは `mod size`(`cursors` 902〜929)。consumed = 出力が読んだ frame 数(native は仮想時計、ブラウザは AudioWorklet の RD) | 一致。分解能 1 frame(ブラウザは 128 frame の量子) |
| 2 | write cursor は『書いても安全な点』で、play cursor より常に先(ee418744、IDirectSoundBuffer::GetCurrentPosition〔mt708925〕) | `write = abs_pos_at(b, max(pulled, consumed))`、`audio_sync()` が毎回 `consumed + lead` まで出力へ渡すので write − play = lead で一定(トレースの全 G で 2646 frame。§6b の案では play + min(15 ms, 手渡し済み)) | 一致 |
| 3 | **write cursor は play cursor の『typically about 15 milliseconds』先**(mt708925。『typically』= 契約の数値ではない) | **lead = 60 ms**(`FLE_AUDIO_LEAD_MS` の既定、`fle_audio_ring.h` 23 行 S-audio-lead、PM の決定。native の環境変数 `FLE_AUDIO_LEAD_MS` で 15 にできる) | **違い**(4 倍)。意図した選択(S2 のテストで 60 ms は出力の欠け 0.2%、15 ms は 8.4%)。**FL は Unlock の直後に write を 1 回読み、今書いた半分の中なら underrun の表示を出す**(§2)ので、このリードは FL の検知の頻度を決める(§6: 15 ms で 77/401、60 ms で 108/400、120 ms で 332/400)。ペースと Lock の位置は変えない |
| 4 | write cursor は play cursor と共に動き、書いたデータでは動かない(ee418744) | 同上(consumed と共に動く) | 一致 |
| 5 | play cursor と write cursor の間は『すでに再生に回っているかもしれず、触ってはならない』(ee418744、ee418062) | **定義されない所を runtime は寛大にしている**: S-ds-late-write(決定 0241、`rewrite_unread` 337 行〜): Unlock の時点で出力がまだ読んでいない手渡し済みの frame は、現在の内容でもう一度ミックスし直して置き換える(守りの幅 `REWRITE_GUARD_FRAMES` = 256 frame 以内は取り込めず『lost』と数える) | 契約外(選択) |
| 6 | 分解能: Learn に数値なし。`DSBCAPS_GETCURRENTPOSITION2` で『できるだけ正確に』(ee418744) | 1 frame。FL がこの flag で作るかは**未確認**(トレースは作成の flag を持たない) | 数値なし。非公式の記述: 実機は約 10 ms 刻み(Handmade Network の議論、非公式)。WDK『Prefetch Offsets』〔Learn〕:『DirectSound は hardware-accelerated pin の write cursor を 10 ms 余分に先へ置く』(ページ全体を読んだ。『currently pads … by 10 milliseconds. Note that the amount of padding might change in the future.』) |
| 7 | 『大きな prefetch offset(write − play)が半バッファを超えると、play cursor が常に別の半分にいるという前提のアプリが壊れる』(WDK Prefetch Offsets、ページ全体を読んだ。例にされているのは『write cursor が半分に入ったらその半分を書く』アプリ。FL は『play cursor が半分を出たら書く』形で、write cursor は Unlock 後の検査にだけ使う) | 60 ms = 2646 frame < 半分 4096。120 ms = 5292 > 4096 の run(lead120)でも FL の Lock の列は基準と同じ | Lock の列は影響を受けない(実測)。検知の頻度は変わる(§6) |
| 8 | `Lock` / `Unlock`: 書いた後すぐ Unlock、2 つの pointer を返す(ee418073、ee418153) | FL は Lock と Unlock を同じ仮想時刻(0 ms 差)に呼ぶ。`Lock` は `DSBLOCK_FROMWRITECURSOR` なら offset を write cursor にする(1082 行)。FL の Lock の offset は 0 / 16384(固定)で、write cursor から取っていない(write は Unlock 後の検査にだけ使う) | 一致 |
| 9 | `Stop`: 『play cursor を最後に再生した sample の次にする』(ee418151) | Stop は手渡し済みの frame(リード分)を再生してから止め、後の Play が繰り返しも飛ばしもしない位置にする(1264 行 S-ds-stop-drain) | 一致 |
| 10 | 停止中のバッファ: Learn に無し | play = write = 保存したカーソル(S-ds-stopped-write、910 行)。primary は 0 / 0(S-ds-primary-cursors) | 選択(FL のミキサーは再生中) |
| 11 | `GetCurrentPosition` の呼び出しコスト: Learn に無し | 呼ぶたびに仮想時計を `FLE_CLOCK_READ_TICKS`(10 ticks = 1 µs)進める(S-ds-pos-read-tick、931 行) | 選択。FL の `Sleep(1)` のポール(1 ms)に対して無視できる(間隔の中央値 1.001 ms) |

| 12 | FL が write cursor を読む: Unlock 直後に `GetCurrentPosition(NULL, &write)`、write が今書いた半分の中なら underrun(§2) | 上の 2・3 行。write − play = 60 ms(2646 frame)なので、起点のポールから Unlock まで C > 約 33 ms かかる周期に出る(読み)。Learn の 15 ms なら C > 約 78 ms | **違い**(検知の頻度)。実 Windows で FL が同じ検知をどれだけ出すかは**未測定** |

実 Windows で違いうる『リード 15 ms 前後』『粒度 約 10 ms』は、どちらも**資料の記述**であって測定ではない。

## 6. 感度実験(契約に関わる量を 1 つずつ変える。`tables/sensitivity-summary.tsv`)

『FL の検知』= Unlock の直後に FL が読む write cursor が今書いた半分の中にあった Unlock の数(`tools/underrun95.py`。FL の UI カウンタ `[15e16d0]` の増加の上限〔仮想関数 +0x88 が真のときだけ増える〕で、通知 `0xbd68e0(1,0)` の呼び出し回数の上限)。

| run | 変えた物 | Unlock | FL の検知 | 書き込み率 | first の差 4096 / 12288 / 20480 | 古い内容で再生された割合 | C の中央値 / p90 |
|---|---|---|---|---|---|---|---|
| base40 | (基準 40 s。同じ run を 3 回: 結果は完全に同一) | 400 | 108 (27%) | 0.562 | 289 / 99 / 7 | 38.4% | 3.0 / 262.3 ms |
| lead15-40 | リード 15 ms | 401 | 77 (19%) | 0.564 | 290 / 98 / 8 | 38.4% | 3.0 / 262.3 |
| lead120-40 | リード 120 ms(> 半バッファ) | 400 | 332 (83%) | 0.562 | 289 / 98 / 8 | 38.4% | 3.0 / 262.3 |
| q441-40 | 返す play・write を 441 frame(10 ms)刻みに丸める | 406 | 109 (27%) | 0.570 | 294 / 94 / 10 | 38.8% | 3.0 / 262.3 |
| q2205-40 | 同 2205 frame(50 ms)刻み | 406 | 109 (27%) | 0.570 | 294 / 94 / 10 | 38.7% | 3.0 / 262.3 |
| wrjunk-40 | **返す write cursor を 0x7ffffff0(無効)に** | 400 | **0 (0%)** | 0.562 | 289 / 99 / 7 | 38.4% | 3.0 / 262.3 |
| speed2-40 | 仮想 CPU 2 倍(alpha1 の `A12_SPEED` と同じ一行。`tools/experiment-speed2-…diff`) | 544 | 166 (31%) | 0.767 | 491 / 44 / 0 | 21.6% | 81.2 / 132.2 |
| speed4-40 | 仮想 CPU 4 倍 | 623 | 380 (61%) | 0.882 | 619 / **0** / 0 | **0.0%** | 41.1 / 64.1(最大 82.3 < 92.9) |
| **vis15-40** | **§6b の案: FL に見せる write だけ play + 15 ms(手渡しのリード 60 ms は内部に残す)** | 400 | **80 (20%)** | 0.562 | 289 / 99 / 7 | 38.4% | 3.0 / 262.3 |
| base100 | (基準 100 s) | 662 | 230 (35%) | 0.486 | 360 / 280 / 15 | 60.5% | 168.4 / 264.4 |
| lead15-100 | リード 15 ms(100 s) | 663 | 179 (27%) | 0.487 | 361 / 280 / 15 | 60.4% | 168.4 / 264.3 |
| **vis15-100** | §6b の案(100 s) | 662 | **181 (27%)** | 0.486 | 360 / 280 / 15 | 60.5% | 168.5 / 264.6 |

読み: (0) **仮想 CPU を 4 倍にして C を 92.9 ms 未満(最大 82.3 ms)にすると、+3 は 0 回、古い内容で再生された割合は 0.0%、全 623 周期が『待つ』周期で first の差は全て 4096**(FL・契約の値は一切変えていない)。2 倍(C の中央値 81 ms、p90 132 ms)では +3 が 44 回(= C が 92.9 ms を超えた周期)。= alpha1 0274 §8 の再現で、+3 の有無はミックスの速さだけで決まる。**4 倍でも FL の検知は 380/623(61%)出る**: 音は全部間に合っているのに、リード 60 ms では C > 約 33 ms の周期に検知が出る。(C の中央値が run ごとに違うのは、仮想 CPU が速いと曲を開く段階が早く終わり、40 s の窓の中で重い区間が占める割合が増えるため。) (1) リード 15・60・120 ms で FL の Lock の列・書き込み率は 1〜2 回の揺れ(境界の 1 周期)を除いて同一 → write cursor のリードは +3 の数を変えない。**変わるのは検知の頻度だけ**(77 / 108 / 332)。(2) write cursor を無効な値にして検知を全部止めても、Lock の列・first の差の度数・書き込み率・古い内容で再生された割合は完全に同一 → **検知はペースに作用しない**(FL が write を読まないという意味ではない。訂正の節)。(3) play cursor の粒度を 10〜50 ms にすると、起点のポールが最大 50 ms 遅れて周期がわずかにずれ(書き込み率 +0.008、+3 が 99 → 94)るが、構造(交互の Lock、+3 と +1 の交互)は変わらない。q441 と q2205 が同一なのは、play が 4096 を越えて最初に取る値が両方で 4410(= 441 × 10 = 2205 × 2)になるため。(4) **vis15 は、Lock の列(360/280/15/2)・+3 の数・書き込み率(0.486)・古い内容で再生された割合(60.5%)が基準と同一で、FL の検知だけが 230 → 181(−21%)、40 s では 108 → 80(−26%)**。リード 15 ms の run(lead15)と検知の数がほぼ同じ(77 と 80、179 と 181)。差の 2〜3 回は、lead15 では手渡しのリードも 15 ms になるため手渡しの時刻がずれることによる。(5) 検知が 0 にならないのは、native の仮想 CPU の課金で 1 ブロックのミックスが 78 ms を超える周期が多い(base100 の C の中央値 168 ms)ため。実機のミックスの速さではない。

## 6b. runtime の差分案: S-ds-write-cursor(PM 承認の方針。小さな差分、E12 候補)

差分は 3 つ(`impl/`)。PM は必要な物を取る:
1. **`0295-ds-write-cursor.diff`(承認された本体)**: `lift/runtime/fle_dsound.c` の `cursors()` だけ。+8 / −1 行、定数 1 つ。基準 main 7373cd55 に offset 0 で当たる(`tables/verify-patch-0295-on-main-7373cd55.log`)。
2. `0295-ds-write-cursor-test.diff`(任意): `crates/fe-lift/tests/wall_fr15.rs` にテストを 1 本(+18 行、削除 0)。『write cursor が play + 15 ms〔661 frame〕で、手渡しのリードが 60 ms〔2646〕でも 120 ms〔5292〕でも変わらない』を既存の `play` の scenario で固定する。**変更後のコードで通り、変更前のコードでは落ちる**(変更前は write = (consumed + 2646) mod 882 = 176 バイトで、期待の 2820 バイトに合わない)ことを確認した。既存のテストは修正なしで全て通る(下)。
3. `0295-ds-write-cursor-comments.diff`(任意、コメントだけ): この変更で不正確になる 3 か所のコメント(`fle_dsound.c` の先頭、`fle_audio_ring.h` の 4 行目と 23 行目の『write cursor の lead = 手渡しの lead』)を『手渡しのリード。GetCurrentPosition が返す write cursor は別の量』に直す。`fle_audio_ring.h` に触れるので、C の部分のキャッシュキーが変わる(E12 は冷間ビルドなので影響は無いはず)。取らなくても動作は同じ。

```c
#define DS_WRITE_CURSOR_MS 15u      /* Learn, GetCurrentPosition: "typically about 15 milliseconds' worth of audio data" */
...
    pl = abs_pos_at(b, c);
    {
        uint64_t vis = (uint64_t)b->fmt.rate * DS_WRITE_CURSOR_MS / 1000u, handed = b->pulled > c ? b->pulled - c : 0;
        wr = abs_pos_at(b, c + (vis < handed ? vis : handed));     /* 手渡し済みの frame を越えない */
    }
```

- **変数を共有しない(条件 1)**: 手渡しのリード(`fle_audio_lead_frames()` = `FLE_AUDIO_LEAD_MS` の 60 ms)は `audio_sync()` の中の `lead` / `r->lead_frames` で、差分は `audio_sync` に触らない。`cursors()` の write は `DS_WRITE_CURSOR_MS` と `b->fmt.rate` だけから作り、`fle_audio_lead_frames()` も `r->lead_frames` も読まない(上のコードで確認できる)。手渡しは 60 ms のままなので、S2 のテストの『60 ms は出力の欠け 0.2%』は変わらない。
- **契約の実装であって補正ではない(条件 2・方針)**: Learn の GetCurrentPosition の記述の実装(外部 API の契約。(b) の拡充)。旗・環境変数・class/アドレスによる分岐は無い。FL の挙動を補う・代行する処理ではない。
- **S-ds-late-write は変わらない**: play と write の間の書き込みは `rewrite_unread`(守りの幅 256 frame)が取り込む。write cursor が手前に来るだけで、取り込める範囲は変わらない。
- **write を使う他の経路**: `Lock(DSBLOCK_FROMWRITECURSOR)` の offset(1082 行)だけ(FL は使わない。play + 60 ms → play + 15 ms になり、Learn に近づく)。Unlock の `cursors()` の呼び出し(1135 行)は play だけを使う。
- **native A/B(条件 3。`tables/sensitivity-summary.tsv`、vis15 と base):**

| | base40 | vis15-40 | base100 | vis15-100 |
|---|---|---|---|---|
| Unlock | 400 | 400 | 662 | 662 |
| Lock の列(offset の交互) | 400/400 | 400/400 | 662/662 | 662/662 |
| first の差 4096 / 12288 (+3) / 20480 / -4096 | 289 / 99 / 7 / 0 | 289 / 99 / 7 / 0 | 360 / 280 / 15 / 2 | 360 / 280 / 15 / 2 |
| 書き込み率 | 0.562 | 0.562 | 0.486 | 0.486 |
| 古い内容で再生された割合 | 38.4% | 38.4% | 60.5% | 60.5% |
| 待った周期 / 待たない周期 | 336 / 64 | 336 / 64 | 475 / 187 | 476 / 186 |
| **FL の検知(カウンタ `[15e16d0]` の増加の上限)** | 108 (27%) | **80 (20%)** | 230 (35%) | **181 (27%)** |

  100 s の『待った周期 475 / 476』の 1 周期の違いは境界の揺れ(lead15-100 の 475/188 と同種)。**FL の検知の頻度が下がり、Lock の列・+3 の数・書き込み率は変わらない**。vis15-40 のスナップ #1・#3 の画面の sha256 は base・wrjunk と同一(再生前)。
- **UI への影響(条件 4。`tables/ui-impact-40s.tsv`、`tables/pixdiff-*.txt`、`tables/cpu-meter-crop-p040-base-wrjunk-vis15.png`、native N1 の 40 s、同じ G3 入力・同じ binary で、検知 108〔基準〕/ 0〔wrjunk〕/ 80〔案〕の 3 run)**:
  - **CPU**: 全スレッドの間接呼びの数(g3-progress)789,666,656 / 789,728,259(+0.008%)/ 789,580,563(−0.011%)、仮想 CPU の 1 ms の slice のうち th 0(最初のスレッド = UI と読む)が 1853 / 1854 / 1848、ミキサー(th 18)が 34031 / 34046 / 34020。**検知を 0 にしても案にしても、UI スレッドの CPU に測定できる差は出ない**(検知 0 の run のほうが基準より UI の slice が 1 つ多く、案の run は 5 つ少ない = 揺れの範囲)。→ P2/P3 への副次的な利得(検知を減らして UI の CPU・描画を減らす)は **N1 の仮想 CPU の課金では観測されなかった**。
  - **画面(PM 15:44 の (3)。G3 の各スナップで `FLE_G3_DUMP_DIR` により全 surface の生の画素を比べた、同じ 3 run を再実行。進捗の数は元の run と完全に一致〔決定的〕)**: 主ウィンドウ(lid 14、1920×1080)・もう 1 つの surface(lid 48、874×573)・wizard(lid 8)・open_a/open_b の lid 137 の全スナップのうち、**再生前の全スナップ(wiz0・e03・open_a・open_b・closed・idle30)は 3 run で画素が完全に一致**。再生中(p010・p020・p030・p040)と end の主ウィンドウは、基準と wrjunk の間で 968 / 19 / 1046 / 985 画素(全体の 0.001〜0.05%)、基準と案の間で 972 / 22 / 30 / 46、wrjunk と案の間で 8 / 20 / 1046 / 980 画素違う。**違う画素は全て、主ウィンドウ上端の x 1043〜1126、y 12〜31(約 84×20 画素)に収まる = ツールバーの CPU メーター(値 `95` と赤い CPU 履歴のグラフ)の一角**。`95` の値は 3 run で同じで、違うのは赤い履歴のグラフの数画素の揺れ(切り抜き: `tables/cpu-meter-crop-p040-base-wrjunk-vis15.png`、6 倍に拡大)。**ほかの画素は全スナップで 3 run が一致**。= underrun カウンタ `[15e16d0]` や通知 `0xbd68e0(1,0)` の効果が、この場面(参照プロジェクトの再生 40 s)・この 3 つの surface の中で**目に見える別の場所に出た形跡は無い**(カウンタの値自体を描く場所は、この画面の中には見つかっていない)。CPU メーターのグラフの違いは、検知の有無そのものか仮想時刻のわずかなずれ(上の slice の差)のどちらによるかを分けられていない(検知の無い run と有る run の差の大きさが、案と基準の差と同程度)。
  - 残る未確認: ほかの窓(Mixer・Performance・プラグインの窓など)を開いた状態での表示、実ブラウザでの表示(gpu1 の領分)。
- **実 Windows の値(条件 5)**: Learn の『typically about 15 ms』と WDK の『10 ms の padding』は資料の記述だけで、**実 Windows の write cursor のリード・粒度は測定していない**。DS_WRITE_CURSOR_MS = 15 は契約の『typically』の値を採った選択で、実 Windows の測定値ではない。
- **検証(条件 6)**: 下の『検証』の表。

### 検証(main 7373cd55 の木 + 差分、冷間の木)

| 検証 | 木 | 結果(実測) | ログ(`tables/`) |
|---|---|---|---|
| 差分 1 の適用 | main 7373cd55 | `patch --dry-run` rc=0、offset 0 | verify-patch-0295-on-main-7373cd55.log |
| `cargo test --no-run` | main 7373cd55 + 差分 1 | rc=0、214 test executables | verify-cargo-test-no-run-main-7373cd55-0295.txt |
| `test.sh full`(`FLE_TEST_NOCACHE=1 --jobs 2`) | main 7373cd55 + 差分 1 | 218 results、**2,408 passed、0 failed**(7373cd55 の基準と同数)、壁時計 581.5 s | verify-testfull-main-7373cd55-0295.log |
| a4(build 段階まで、5 module) | main 7373cd55 + 差分 1 | units 942/942、runnable 895/895、distinct 5,310、entries 1,009,737、終了コード 0(基準と同一) | verify-a4-main-7373cd55-0295.log、verify-a4run.sh |
| `cargo test --no-run` | **積んだ木**(main 7373cd55 + 0284c + 0284b-on-0284c + 差分 1) | rc=0、214 test executables | verify-cargo-test-no-run-stack-0284c-0284b-0295.txt |
| `test.sh full` | 積んだ木 | 218 results、**2,410 passed、0 failed**(0284c + 0284b の基準と同数)、壁時計 572.8 s | verify-testfull-stack-0284c-0284b-0295.log |
| a4 | 積んだ木 | units 942/942、runnable 895/895、distinct 5,310、entries 1,009,737、終了コード 0(同一) | verify-a4-stack-0284c-0284b-0295.log |
| 積んだ木 + 差分 1 + 2 + 3 | 同上 | `cargo test --no-run` 完走、`wall_fr15` 24 本(既存 23 + 新 1)が全部通る | verify-stack-all3-cargo-and-wall_fr15.log |
| `test.sh full` | 積んだ木 + 差分 1 + 2 + 3 | 218 results、**2,411 passed(= 2,410 + 新しいテスト 1)、0 failed**、壁時計 446.1 s | verify-testfull-stack-all3.log |

既存の write cursor 参照のテスト(PM 15:44 の (2)): `lift/harness/audio/ds_scenarios.c` の `rt_mixer`(587〜639 行)を呼ぶ `wall_fr15.rs` の `the_engines_polling_mixer_runs_against_the_model_in_virtual_time_without_an_underrun`(444〜467 行。リード 15 / 60 の両方で『the engine's underrun check fired 0 times』)と、write cursor の戻り値の手計算を持つ `play_moves_the_play_cursor_..._by_15_ms`・`stop_keeps_the_frames_...`(2992 バイト)・`the_default_lead_is_60_ms_...`(手渡し write_frames = 2646 など)は、**期待値を直さずに全部通る**(`FLE_AUDIO_LEAD_MS=15` で走るテストは、手渡し済みが 661 frame で write = play + 661 と変わらないため。リード 60 の `rt_mixer` は検知が 0 のまま)。変更の効果を固定するテストは差分 2。実機の値でなく Learn の記述『typically about 15 ms』を期待値の根拠にしている。

## 7. gpu1(0289)・alpha1(0274)との関係

- gpu1 の 0289 §4 の読み(『FL は次の Unlock を書き込み位置の先に置く、その後飛ばした 2 ブロックのうち 1 つを書き戻す』)は、**FL が書き込み位置を見て『先に置く』のではない**(FL は write cursor を Unlock 直後に 1 回読むが、それは underrun の検知用で、書く半分の選択には使わない。§2)。FL は常に『最後に見た play の外の半分』を書くだけで、+3 は遅れた書き込みが次の周回に落ちる記録上の結果、-1/+1 は C の揺れ(実時間のジッタ。native の仮想時間では C が一定に近いので -4096 は 100 s に 2 回だけ)で書く半分が変わった周期。gpu1 自身の 0289 の `first` の定義の節の読み(『+3 は play が既に入っている半分を書いたときの記録』)と同じ。
- gpu1 から受けた情報: ブラウザの 0289 に GetCurrentPosition の戻り値の列は無い(計器が無い)。この文書の表が native の同じ列。実機ブラウザの観測は gpu1(分担:私 = native の再現と runtime の対応表、gpu1 = 実機ブラウザ)。
- alpha1 の 0274: 契約の逸脱なし(write − play = 2646 の一定)。この文書はそれを 100% の Unlock と感度実験で補強した。

## 8. 方針との適合(必須の記載)

- **lift の忠実性**: 分析(§1〜§6)はコードの変更 0 行。差分案(§6b)の新しい処理は、(b) 外部 API の契約の実装(Learn の GetCurrentPosition の write cursor)で、近道・FL の挙動の補正・代行ではない。class/アドレスによる分岐も無い。runtime が FL の遅れを隠すための近道(FL のミキサーを待たせる、ブロックを前倒しで渡す、等)は採らない。感度実験の小さな差分(`tools/experiment-*-NOT-FOR-MERGE.diff`)は native の使い捨てで、製品に入れない。
- **no-options**: 旗・環境変数・オプションの追加は無い(差分案は定数 1 つ)。実験で使った `FLE_AUDIO_LEAD_MS` は既存の native 専用の環境変数(`FLE_TEST_GETENV`。wasm の製品は読まない)。
- **進捗の誠実性**: 代役・再生・推測を含まない。実エンジン(native N1)で実行した範囲と、していない範囲(ブラウザ・実 Windows・画面への表示)を §0 と §6b に分けて書いた。誤り(最初の報告の結論)は『訂正』の節に書いた。

## 9. 再現手順(`tools/`)

- `build95.sh VARIANT`: alpha1 の N1 の recipe(`out-alpha3/tools/build-n1c.sh`、mt3w 0285 の `build85.sh`)で、alpha1 の t12h 木(runtime + 追跡)から native N1 を build(基準は約 40 秒、変種は `fle_dsound.c` だけ入れ替える)。
- `run95.sh TAG BIN G3SCRIPT CPU [VAR=value …]`: G3 の模擬入力で 1 run(`A12_DSTRACE` に追跡)。40 s の run は壁時計 48〜66 s、100 s の run は 109 s。
- `ds95.py TRACE --tsv OUT`: Unlock ごとの表と要約。`excerpt95.py TRACE FIRST N [--every K]`: 引数つきの時系列。`summ95.py RUNDIR…`: 感度の表(FL の検知の列つき)。`underrun95.py TRACE…`: FL の Unlock 直後の検査が出た数。`retaddr95.py`: 戻り先アドレスの run の集計。
- 追跡(gz): `logs/ds-trace-{base40,base100,lead15-100}.trace.gz`。他の run の追跡は大きいので置かない(要約と表は `tables/`)。


## PM の判断と取り込み(2026-11-06 16:15、docs のみ。runtime の差分案は E12 候補で未投入)

取り込み: mt3 の最終(訂正を含む、16:03)の文書・表(約 1 MB)・道具と、差分 3 本(`0295-ds-write-cursor.diff`、`-test.diff`、`-comments.diff`)を `docs/investigations/mt3w/` に置いた(生のログ 4.6 MB は入れない。原本は `lift/out-git/mt3w-results/0295/logs/`)。実行したのは native N1 に G3 の模擬入力(18 回)で、**ブラウザ・実 Windows・依頼者の機械は 0 件**。

判断:
1. **最初の読み(「FL は遅れを判定しない・write cursor を使わない」)の訂正を受理する**: FL は Unlock 直後に write cursor だけを読み、write cursor が書いた半分の内側にあれば UI の underrun カウンタを進めて通知する。ただしこの検出は Lock の位置・待ち・+3 の数・書き込み率を変えない(無効な write cursor を返しても同一)。pacing は play cursor だけで決まる。「+3 飛び」はミックスが 92.9 ms を超えたときの記録上の結果、という結論は不変。
2. **runtime の write cursor のリードは 60 ms(S-audio-lead)で、Learn の「typically about 15 ms」の 4 倍**。そのため FL の underrun 検出が実 Windows より頻繁に発火する(native 40 s で 108/400 対 リード 15 ms の 77/401)。UI の通知が画面に出るかは今回の場面で観測できず、CPU への影響は ±0.3% で測れない。実 Windows の値は未測定。
3. **差分案(FL に見せる write cursor を play + 15 ms にし、内部の 60 ms の受け渡しリードは別変数のまま)は外部 API の契約の実装(忠実)として採る方向**。条件: 旗・環境変数・class/アドレス分岐なし(満たす)、native の A/B で Lock の列・+3・書き込み率は同一(満たす)、検出回数が減る(108 → 80、230 → 181)。mt3 の積み重ね検証(main + 0284c + 0284b-on-0284c + 差分 1)は test.sh full 218/2,410/0 failed、a4 942/942・895/895・5,310・1,009,737。PM は E12b の上に差分 1 + 2 を載せた木(`000091ae`)で `cargo test --no-run`、`test.sh full`(218 results/2,411 passed/0 failed)、a4 を検証する。
4. E12 に入れるかは (a) PM の a4 の結果、(b) opt1 の FS 付きテストビルドの node 行(検出頻度が変わるので、underrun 通知の描画があれば g3-surface に出うる)で決める。入れない場合は E13 として別 push。実 Windows の write cursor のリード・粒度は契約の文章だけが根拠(15 ms は「typically」の選択)。
