# 0274(仮): P3 の梃子の切り分け — native N1 で FL の TMixerThread_DS が DirectSound に何をしているか(alpha12)

**依頼者の機械・実機 Windows・ブラウザ(本番 ff5ee429 / Worker)での測定は 0 件。** 以下は native N1(b42fad36 の生成 C、HEAD 0b228624 の runtime〔`lift/runtime` は b00932ec 以降同一〕+ **診断の追跡**〔`diag/`、製品に入れない〕、G3 の模擬入力〔**ユーザーの操作ではない**〕、参照プロジェクト `serina1-serum2.flp` を開いて再生)の値。**native の仮想時計は壁時計に歩調を合わせない**ので、すべて『仮想時間あたり』で書く(実時間ではない)。native のプラグインは lift 済みの FLEngine・Elastique・dsp_ippv2・Blend2D・ILTools・oggio・QFC・freetype だけで、**Serum 2 などの VST は無い**。測定は構造の測定(性能の値ではない)。測定中の load average は他の担当と共有(6〜10)。逆アセンブル・プロファイルの読みは UNOFFICIAL。代役・再生・推測は含まない。

状態: **仮の決定**。作業者 `alpha1`、2026-11-06 JST 09:35〜。成果: `tables/`(根拠の表)、`logs/`(追跡)、`tools/`(解析)、`diag/diag-trace-prio-speed-NOT-FOR-MERGE.diff`(**入れない**)。runtime の差分案は無い(下の結論 6)。

## 冒頭の三点(正直さ)

1. **何を測ったか**: native N1 を診断の追跡付きで走らせ、FL のミキサースレッド(スレッド 18、2 本目の `TMixerThread_DS`)の `Sleep`・`GetCurrentPosition`・`Lock`・`Unlock` を全部、仮想時刻つきで記録した(再生の前 25 s と再生の 100 s)。1 回のミックス(最後のポールから次の `Lock`)の間に仮想 CPU の 1 ms のスライスをどのスレッドが使ったか(`Q` 行)、ゲストの間接呼び出しの数(`fle_progress_count`)、スライスが課金された数と callback の中で飛ばされた数、をその区間ごとに出した。次に変数を 1 つずつ変えた: (a) `SetThreadPriority` を守らせる(runtime の選択肢、既定は off)、(b) 仮想 CPU を 2・3・4 倍速くする、(c) 再生ボタンを 20 仮想秒遅らせる。最後に標本採取のプロファイル(`A4_PROF`、CPU 時間 2 ms ごと)でミキサースレッドの CPU の行き先を出した。
2. **結論**(第 1 の結論 = native でも 3 ブロック周期に落ちるか): **落ちる。** 仮想時間で、再生を押して 4.9 s 後にミックスの時間 C(最後のポール → `Lock`)が 3 ms → 165 ms に、24.4 s 後に 265 ms に跳び、書き込み率(Unlock したフレーム / 仮想秒 / 44100)が 0.99 → 0.49 → **0.33**、`Unlock` の間隔が 93 → 180 → 275 ms(= 1.94 → 2.95 ブロック)、`first` の差が 4096(連続)から 12288 が主になる(ブラウザで gpu1 が見た『12288 の 3 ブロック周期 279〜283 ms、1/3』と同じ形)。**原因はブラウザの IdleDriver / 壁時計の写しではなく、DirectSound のモデル(play / write カーソル、リード 60 ms 一定)でもなく、ミキサー自身の計算量**: 1 ブロック(4096 フレーム = 92.88 ms)のミックスに仮想 CPU で 165〜265 ms かかる(ゲストの間接呼び出し 250 万〜410 万回)。再生中は仮想 CPU のスライスの **83〜97% をスレッド 18 が使い、UI スレッド(0)は 10 s に 4〜27 ms**。優先度を守らせても同じ(競争相手がいない)。仮想 CPU を 3〜4 倍にすると書き込み率は 0.99〜1.00 に戻り、`first` の差はすべて 4096(2 倍では 0.50〜0.97 で足りない)。→ **P3 の天井は『音声 1 秒あたりに必要な CPU(約 2.4〜2.9 秒)』で、梃子はミキサーの計算量(プロファイルではその 70% が Elastique)か並列化**。この形と数は gpu1 / opt1 の『Unlock 1 回あたり Worker CPU 202〜245 ms、音声 1 秒あたり 2.17〜2.64 s』と同じ桁。
3. **未確認**: ブラウザ(本番の Worker・wasm)での同じ分解(native と wasm の関数ごとの CPU の割合は違う)。**『Worker は 66〜87% しか埋まっていない』の説明は native では出ない**(native のミキサーは仮想 CPU の 83〜97% を使い、待ちの W は 1〜28 ms)。ブラウザの空き(13〜34%)の候補は、ミックスが 2 ブロックを少し超えるときの `Sleep(1)` のポールの待ち(1 周期の 12% 程度)と、IdleDriver が仮想時刻を壁時計に合わせて待つ分だが、**測っていない**(`driverStats` の `timerWakes` / `maxLeadTicks` を見るのが次の一手)。Elastique の CPU の内訳(mt3 の 0275 の範囲)。実 Windows での FL の『音声 1 秒あたり』の CPU。

## 結論(続き)

4. **FL の判断の根拠(1 例、`tables/transition-example.tsv`)**: FL は 2 つの半バッファ(8192 フレームのバッファ、4096 フレームずつ)を交互に埋める。`Unlock` の後は `Sleep(1)` + `GetCurrentPosition` のポール(1 ms ごと)で、**再生カーソルが片方の半分を出るのを待ち**(play ≥ 4096 または 0 に折り返し)、出たらその半分をミックスして `Lock`(16384 B = 4096 フレーム)・`Unlock` する。`write` カーソルは常に play + 2646 フレーム(60 ms)。ミックスが 93 ms 以内なら次の半分に着く前に書き終わる。ミックスが 151 ms かかった例では、`GetCurrentPosition` が play = 4112(境界を越えた)を返した 274 ms でミックスが始まり、`Lock` が 425 ms、そのとき play = 2589 で、**書こうとする半分(オフセット 0〜4095)の中を再生カーソルがもう 2589 フレーム進んでいる**(= ブロックの 63% が書く前に再生された)。FL は遅れに気づかず、次の半分を順に書く。`tables/native-late-frames-per-write.tsv`: 30 s 以降は書き込みの 17〜63% が一部遅れ(最大 4076 / 4096 フレーム)。
5. **『30 s』は曲の位置に依る**(`tables/c-steps-follow-the-song-position.tsv`): 再生ボタンを 20 仮想秒遅らせても、C の跳び(3 → 165 ms、165 → 265 ms)は押してから同じ 4.9 s / 24.4 s 後に来る(窓ごとの中央値が ±3 ms で一致)= 仮想時間ではなく再生位置(曲の内容: 参照プロジェクトのどこで重いクリップが始まるか)。ブラウザで書き込みが遅いと曲の進みも遅いので、壁時計の『30 s』は曲の位置が重い場所に着く時刻で、固定の時刻ではない。
6. **契約の逸脱(DirectSound のモデル)は見えなかったので、runtime の差分案は無い**: `GetCurrentPosition`(play = 消費フレーム、write = 配布済みまで、リード 60 ms 一定)・`Lock` / `Unlock`(範囲・周回)は Learn のとおりで、ミキサーの 100 s のすべての周期で一貫(`write - play` = 2646 フレームの一定)。**他に見えた runtime の事実(逸脱だが P3 に効かない)**: (i) `SetThreadPriority` は runtime が記録するだけでスケジューラは見ない(決定 0234 の選択肢は既定 off)。FL は、2 本のミキサー(スレッド 7、18)に自分で TIME_CRITICAL(15)を設定し、ほかのスレッドには LOWEST(-2)・BELOW_NORMAL(-1)を設定し、QFC のタイマースレッド(1、3)は -2 → ABOVE_NORMAL(1)にし、プロセスのクラスは ABOVE_NORMAL にする(`tables/thread-priorities-and-creation.txt`)。守らせても native の書き込み率は変わらなかった(競争相手がいない)。ブラウザで UI が同じ Worker の CPU を取る場合の効果は未測定。(ii) 仮想 CPU の課金: 再生中のミックスでは**スライスの終わりの 73% が callback の中で飛ばされ(課金されない)**、課金は 27%(1 課金 ms あたり約 15,300 回の間接呼び出し、モデルの定数 4096 回 / ms の約 3.7 倍)。これは契約ではなく時間モデルの選択(`S-vclock-depth`)で、native の C の絶対値(165〜265 ms)はこの課金に依る。ブラウザでは壁時計が支配するので C の絶対値は移らない(移るのは『音声 1 秒あたり約 2.4〜2.9 倍の計算が要る』という比)。
7. **ミキサースレッドの CPU の行き先(native -O2、仮想 3630〜3728 s、20,246 標本 = 40.5 CPU 秒)**(`tables/native-mixer-cpu-by-module.tsv`、`native-mixer-profile-top-functions.txt`): **Elastique 70.3%**、FLEngine 14.1%、runtime / libc 12.5%(うち `fle_*` 6.0%、その他 6.5%: `bf_mul`・`bf_div_small`・`round_pack` など x87 / 多倍長の補助、`sha_update` 0.6%)、dsp_ippv2 3.1%。自己時間の上位は Elastique の `f_b34aca0` 10.7%、`f_b34eaf0` 9.1%、`f_b2f3650` 6.9%、`f_b1ce5e0` 4.6%(mt3 の 0270 の対象)、`f_b2f23c0` 3.4%、`f_b2f4490` 2.9%、`f_b2b2360` 2.5%、`f_b25ce20` 2.2%(0270 の対象)ほか。native の CPU の割合で、wasm の割合ではない。
8. **仮想 CPU の倍率の実験**(`tables/native-unlock-per-10s-virtual-cpu-x{2,3,4}.tsv`): ×4 なら C = 41〜71 ms で書き込み率 0.99〜1.00、`first` の差は全部 4096。×3 でもほぼ 1.0(最も重い区間で C の中央値 89 ms、p90 99 ms と閾値に接し、`first` に 12288 が 11 回・-4096 が 10 回)。×2 は足りない(書き込み率 0.97 / 0.60 / 0.77 / 0.50)。= 必要な高速化は**最悪区間で約 2.9 倍**。

## 1. 表(`tables/`)

- `native-unlock-per-10s.tsv`: 10 仮想秒ごとに Sleep・GetCurrentPosition・Lock・Unlock の回数、Unlock したフレーム、書き込み率 wr、Unlock の間隔の中央値、`first` の差の分布(`first` = 消費フレーム + Lock のオフセットの再生カーソルからの距離 = ランタイムが `fle_audio_unlock_note` でページに渡す値と同じ定義)、write − play の中央値。再生ボタンは t0 + 25.1 s。
- `native-mixer-cycle-per-10s.tsv`: 1 周期 = Unlock → 次の Lock を W(Unlock → 最後のポール、待ち)と C(最後のポール → Lock、ミックス)に分ける。再生前は W 90 / C 3 ms(周期 1.00 ブロック)、30〜50 s は W 11 / C 165 ms(1.94〜1.96)、50〜70 s は W 5〜12 / C 265 ms(2.95)、80〜120 s は C 190〜207 ms(2.25〜2.5)。
- `native-vcpu-slices-per-thread.tsv`: 10 仮想秒ごとのスライス(仮想 ms)をスレッド別に。再生中の合計 8,300〜9,650 のうちスレッド 18 が 8,310〜9,650(83〜97%)、スレッド 0(UI)は 4〜27。
- `native-mix-intervals-and-summary.tsv`(**mt3 向け**): ミックスごと(645 行)に C、スレッド別のスライス(仮想 ms: `ms_th0`、`ms_th18`)、間接呼び出しの数、課金されたスライス、飛ばされたスライス、`first` の差。末尾に 10 秒ごとの中央値(例: 30〜40 s は C 165 ms、ミックスあたり 2,520 千回、課金 165・飛ばし 450、他スレッド 0 ms。50〜70 s は C 265 ms、4,040 千回、課金 265・飛ばし 725)。**ミックスの間に他のスレッドが取ったスライスは(中央値で)0 ms**(C > 20 ms の 425 回のミックスのうち UI のスライスがあったのは 55 回、最大でも 9 ms。C の全部がミキサー自身)。
- `native-progress-slices-deferred.tsv`: スナップ時点の `progress`・`slices`・`deferred`(10 仮想秒あたり約 1.5 億回、課金 8,900、飛ばし 27,500)。
- `native-unlock-per-10s-priorities-honoured.tsv`、`native-vcpu-slices-per-thread-priorities-honoured.tsv`: `SetThreadPriority` を守らせた変種(30 s 以降の 10 s ごとの書き込み率: 守らせない 0.49 / 0.48 / 0.34 / 0.33 / 0.40 / 0.41 / 0.40 / 0.44 / 0.42 / 0.25、守らせる 0.49 / 0.49 / 0.33 / 0.33 / 0.42 / 0.45 / 0.43 / 0.43 / 0.43 / 0.24)。
- `native-unlock-per-10s-play-press-20s-later.tsv`、`c-steps-follow-the-song-position.tsv`: 再生を 20 s 遅らせた実験。
- `native-late-frames-per-write.tsv`、`transition-example.tsv`: 判断の例。
- `thread-priorities-and-creation.txt`: FL の `SetPriorityClass`(ABOVE_NORMAL)、`SetThreadPriority` の全呼び出し(呼んだスレッド、ハンドル、水準)と `CreateThread` の列(n 番目 = スレッド番号、開始アドレス 0x412ae0 = ワーカー、0x1d0fdb0 = QFC のタイマー)。ミキサーはスレッド 7 と 18 で、自分に TIME_CRITICAL(15)を設定する。

## 2. 解釈と、ブラウザへの対応

- 計算量の比 C / ブロック = 1.8〜2.9 は、`write rate` の逆数にほぼ等しい(0.49 ≈ 1/1.96、0.33 ≈ 1/2.95)。つまり**書き込み率は 1 / (ミックス時間 ÷ ブロック時間)**で、周期の量子化(2・3 ブロックへの丸め)による損は小さい(W が 1〜12 ms)。
- 必要な CPU は音声の長さに比例する(ブロックを大きくしても C / ブロック は変わらない)ので、FL のバッファ長の設定で P3 は救えない(遅延と欠けの粒度が変わるだけ)。
- ブラウザの Worker CPU『音声 1 秒あたり 2.17〜2.64 s』(performance-targets §4e)= native の C / ブロック(1.8〜2.9)と同じ量。P3(0.95)には**計算量を 2.3〜2.6 分の 1 に減らすか、ミキサーを別の CPU に出す**必要がある。

## 3. 直していないもの / 逸脱

- 逸脱: なし。PM の木には書いていない。コミット・push なし。runtime の製品の差分なし(診断の追跡は `diag/` に、入れない)。
- 実験の限界: native N1 の C は仮想 CPU の課金に依る(結論 6 の ii)。ブラウザの実時間の C ではない。

## 4. 再現

- N1 のビルド: `GENOBJ=$PWD/gen3 RMC=$PWD/keep/rebuild/realmod_table_gen.c NO_COPY=1 JOBS=2 UPSRC=/nonexistent TAG=<t> bash tools/build-n1b.sh <bin> <tree>`。木は `git archive HEAD lift/runtime lift/harness` に `diag/diag-trace-prio-speed-NOT-FOR-MERGE.diff` を当てたもの。
- 実行: `A12_DSTRACE=<trace> [A12_PRIO=1] [A12_SPEED=k] SEED=…/vfs-seed-nested.tsv SRC=<tree> CPU_LIMIT=6000 bash tools/run-n1.sh <bin> scripts/a12-play.g3 <out>`(`a12-play40.g3` は 40 s、`a12-playlate.g3` は再生を 20 s 遅らせる、`a12-playprof.g3` は `A4_PROF=<file>` 用に `cpu` 行つき)。
- 解析: `tools/a12-ds.py TRACE [--t0-ms MS]`、`a12-cycle.py`、`a12-q.py`、`a12-intervals.py`、`a12-prof-modules.py`、`a4-prof.py`。gdb: `a12-prio.gdb`(優先度と CreateThread)。



## PM の決定と取り込み(2026-11-06 10:15、docs のみ)

取り込み: alpha1 の 0274 最終(10:05)の文書・表・道具・G3 スクリプトを `docs/investigations/alpha12/` に置いた。8 本の native の追跡ログ(各 700〜1,100 KB の gzip)は 400 KB の上限を超えるので入れず、`lift/out-git/alpha12-results/logs/` に残す。診断差分(`diag-trace-prio-speed-NOT-FOR-MERGE.diff`)は入れない。測定は native N1(b42fad36 の生成 C + HEAD の runtime + 追跡)・G3 の模擬入力・参照プロジェクトで、**ブラウザ・実機 Windows・依頼者の機械は 0 件**。native の仮想時計は壁時計に合わせないので、すべて仮想時間の値。

判断:
1. **P3 の 3 ブロック周期(書き込み率 1/3)は、ブラウザの IdleDriver でも DirectSound のモデルでもなく、エンジン内の仮想時間の上でミキサー自身の計算が 2 ブロック(186 ms)を超えることで決まる**(native で再現。mt3 の 0273 の閾値の予測と一致)。ミキサーが仮想 CPU のスライスの 83〜97% を使い、mix 区間に他のスレッドのスライスは中央値 0 ms なので、複数の仮想 CPU に並列化する案(0275 の (b))は C を縮めない。
2. 仮想 CPU を k 倍にする診断では、×4 で書き込み率 1.0、×3 でほぼ 1.0、×2 は不足。**必要な加速は最悪区間で約 2.9 倍**。これは (i) 仮想時間の課金(決定 0147 の 4096 回 = 1 ms。スライスの終わりの 73% は callback の中で課金されず、課金分は約 15,300 回 / 課金 ms で、モデルの定数の約 3.7 倍)の較正と、(ii) 実 CPU の削減(Elastique が native のミキサー CPU の 70.3%)の両方の問題。(i) は mt3 の 0275 で、(ii) は alpha1 の 0276 で、設計の表を取る。
3. 契約の逸脱は見えなかった(write − play = 2,646 フレームで一定)ので、runtime への差分は無い。SetThreadPriority を守らせても P3 は変わらない(協調 1 CPU では競争相手がいない)。
4. 未確認: ブラウザでの同じ分解。native に出ない「Worker が 66〜87% しか埋まらない」ブラウザの空き(13〜34%)の理由(周期の丸めの待ち、IdleDriver の lead の返済のどちらか。opt1 の c の内訳が要る)。「30 s」は曲の位置(参照プロジェクトの重いクリップの開始)に依る。
