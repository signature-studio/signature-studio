# #1335 の音の出る入力の探索(判定: 差し戻し。音の出る入力なし)

実エンジン: 実行あり。音の出る入力なし。展開の木の `Data\` の playlist を持つ `.flp` 51 件を、`tools/p3-audio-probe/measure.mjs`(run モード。Space の後 10 秒)で 1 件ずつ測った。51 件とも `nonSilentQuanta` が 0。

runner の値。この機械とは別枠。nproc 4。loadavg(1 分値)は 0.62〜2.86(51 件の各回の終わり)。1 件あたりの wall は 39〜66 秒。

## 0. 結論

- 候補 51 件のどれも音が出なかった(`nonSilentQuanta` 0)。`inputs/p3-project.tsv` は、#1401 の手順の行(1 行目)と、Song Mode の手順の行(2 行目)の 2 行にした(見出し行は無い。9 節)。
- 同じエンジンで、`--make-sound`(#1401 の手順。Default の Channel Rack の Kick を 16 個押して Space)は音が出た: `litSteps` 16、`nonSilentQuanta` 3,580、`quantaWhilePlaying` 16,041、`underrunDelta` 0、`error` null(runner の値。runner nproc 4、loadavg 1.74)。エンジンは音を出せる。demo の無音は、入力の内容の側(欠けたプラグインの DLL と素材)と推定する(未確認。下の 3)。
- 受け入れの 3 回の測定(n=3)は、選んだ 1 件が無いので実行していない。
- Song Mode の判定(P3)は、この探索では未確認。

## 1. 前提

- #1365(18a7fded「DEDZII を開くと engine が止まる」の直し)と #1373(a2959e94「demo の素材の配信」)は main に入っている。
- 候補: `Data\` の `.flp` 80 件のうち、playlist(イベント 233)を持つ 51 件。件数は、イベント 233 の大きさを 32 バイトで割った値(割れない 3 件は、バイト数を 32 で割った概数。表で `*`)。
- 開けたか: `error` が null なら「開けた」(題名の待ちが終わり、窓の測定が済んだ)。`error` が「the engine is exited during the open」の物は「開けない」。
- 終了時の state: `engine-idle` のまま終わった物と、`exited`(エンジンが止まった)。`exited` の 4 件(Exoplanet・Mehran・Steptime・Vocodex)は、エンジンの終了コードが 134(SIGABRT)。
- Problems の窓: 起動の後の「Problems loading the project」(TMissingPluginsForm)の有無。
- DLL: 測定の `dll`(エンジンが読もうとして無かった `.dll` の要求の数)。

## 2. 候補ごとの結果(51 件。件数の多い順)

| 件数 | 曲(`Data\` からの相対) | 開けたか | 終了時の state | Problems の窓 | DLL の要求(無かった数) | nonSilentQuanta | underrunDelta | error |
|---|---|---|---|---|---|---|---|---|
| 4440 | Demo songs/Eric Kauffmann & NrgMind - Exoplanet | 開けた | exited | あり | 64 | 0 | 3445 |  |
| 3850 | Demo songs/DEDZII - NIGHT TERROR | 開けた | engine-idle | あり | 78 | 0 | 0 |  |
| 3735 | Demo songs/Ookay - Thief | 開けた | engine-idle | あり | 51 | 0 | 0 |  |
| 2500 | Demo songs/Cuebur & Thandi Draai - Africa's Calling | 開けた | engine-idle | あり | 73 | 0 | 0 |  |
| 2260 | Demo songs/Astes - Bien Duro | 開けた | engine-idle | あり | 79 | 0 | 0 |  |
| 2158 | Visual/ZGameEditor Visualizer/JuiceBass - SeamlessR - Menagerie | 開けた | exited | あり | 47 | 0 | 3445 |  |
| 2117 | Demo songs/Bombs Away ft KARRA - I'm Awake | 開けた | engine-idle | あり | 67 | 0 | 0 |  |
| 1617* | Demo songs/Mehran Abbasi - With You | 開けた | exited | あり | 66 | 0 | 3445 |  |
| 1152* | Demo songs/Chan Gibbons x Ceebeaats - Time | 開けた | engine-idle | あり | 58 | 0 | 0 |  |
| 1085 | Demo songs/Jay Cactus x Confz - Mic Check Ready | 開けた | exited | あり | 61 | 0 | 3446 |  |
| 1080 | Visual/ZGameEditor Visualizer/Astes - Raubana - LIFE | 開けた | engine-idle | あり | 47 | 0 | 2667 |  |
| 1056 | Product demos/Synth_dfr - Overcome - Sytrus | 開けない | exited | なし | 23 | - | - | the engine is exited during the open (the title is "Song inf |
| 980 | Demo songs/Edlan & Ella Noël - Song For You | 開けた | engine-idle | あり | 80 | 0 | 0 |  |
| 924 | Demo songs/Mehdi Ebrahiminejad - Never Out | 開けた | engine-idle | あり | 59 | 0 | 0 |  |
| 880 | Demo songs/Purps On The Beat Feat Luclover - Stroll Up | 開けた | engine-idle | あり | 67 | 0 | 0 |  |
| 839 | Product demos/Olbaid - Harmless Essentials | 開けた | engine-idle | あり | 35 | 0 | 0 |  |
| 823 | Product demos/Olbaid - Sytrus Essentials | 開けない | exited | なし | 23 | - | - | the engine is exited during the open (the title is "Song inf |
| 811 | Product demos/Square a Saw - Lightning (VEELA Vocals Resynthesized) | 開けた | engine-idle | あり | 54 | 0 | 0 |  |
| 777* | Demo songs/LeriQ - SHINE | 開けた | engine-idle | あり | 49 | 0 | 0 |  |
| 760 | Demo songs/P.Priime - Use Me | 開けた | engine-idle | あり | 51 | 0 | 2574 |  |
| 705 | Demo songs/LollieVox & Lezina J - I'll Be Your Madonna | 開けた | engine-idle | あり | 78 | 0 | 0 |  |
| 693 | Demo songs/Atthar Feat Mehran Abbasi & Elise Chantelle - Right Night feeling | 開けた | engine-idle | あり | 61 | 0 | 0 |  |
| 537 | Product demos/Square a Saw - Oxygen (Tevlo Vocals Resynthesized) | 開けた | engine-idle | あり | 51 | 0 | 0 |  |
| 534 | Demo songs/Asher Postman - Future House | 開けた | engine-idle | あり | 75 | 0 | 0 |  |
| 398 | Visual/ZGameEditor Visualizer/Treben - Gimbal and Sinan - RawFL | 開けた | engine-idle | あり | 51 | 0 | 0 |  |
| 260 | Other/Blocks/Blocks | 開けた | engine-idle | なし | 13 | 0 | 0 |  |
| 218 | Product demos/Electric Loop - The Ogun Dream | 開けた | engine-idle | あり | 37 | 0 | 0 |  |
| 160 | Visual/ZGameEditor Visualizer/Sacco - Exploding Chips | 開けた | engine-idle | あり | 19 | 0 | 0 |  |
| 115 | Performance/Touchscreen/Touchscreen | 開けた | engine-idle | あり | 29 | 0 | 0 |  |
| 112 | Visual/ZGameEditor Visualizer/Sacco - FlashyWaves | 開けた | engine-idle | あり | 19 | 0 | 0 |  |
| 108 | Visual/ZGameEditor Visualizer/Treben - Umziky - All the same | 開けた | engine-idle | あり | 64 | 0 | 576 |  |
| 94 | Demo songs/Sid De La Cruz - Secret Agent Rendezvous | 開けた | engine-idle | あり | 57 | 0 | 0 |  |
| 89 | Product demos/nucleon - Fruity Delay 3 | 開けた | engine-idle | あり | 29 | 0 | 0 |  |
| 83 | Performance/8x8/8x8 | 開けた | engine-idle | あり | 23 | 0 | 0 |  |
| 81 | Performance mode/SeamlessR - Bass Antics Performance | 開けた | engine-idle | あり | 43 | 0 | 0 |  |
| 70 | Visual/ZGameEditor Visualizer/Sacco - Expanding universe | 開けた | engine-idle | あり | 19 | 0 | 0 |  |
| 68 | Product demos/nucleon - Harmless Galactic | 開けた | engine-idle | あり | 23 | 0 | 0 |  |
| 65 | Product demos/nucleon - Transient Processor | 開けた | engine-idle | あり | 45 | 0 | 0 |  |
| 64 | Visual/ZGameEditor Visualizer/Sacco - Pumping objects | 開けた | engine-idle | あり | 17 | 0 | 0 |  |
| 63 | Demo songs/Electroconductor - Inner Days - SH-1 Rework (feat. Maja Mannila) | 開けた | engine-idle | あり | 53 | 0 | 0 |  |
| 51 | Demo songs/Electroconductor - Inner Days - Original | 開けた | engine-idle | あり | 43 | 0 | 0 |  |
| 51 | Visual/Razer Chroma/nucleon - Razer Chroma | 開けた | engine-idle | あり | 47 | 0 | 194 |  |
| 48 | Performance mode/Drumloop Masher | 開けた | engine-idle | あり | 31 | 0 | 0 |  |
| 41 | Performance mode/nucleon - Steptime | 開けた | exited | あり | 43 | 0 | 3449 |  |
| 36 | Performance mode/Charles Deluxe - Performance Demo | 開けた | engine-idle | あり | 17 | 0 | 0 |  |
| 26 | Performance mode/nucleon - Housework | 開けた | engine-idle | あり | 23 | 0 | 0 |  |
| 26 | Performance/4x4/4x4 | 開けた | engine-idle | あり | 23 | 0 | 0 |  |
| 25 | Performance mode/nucleon - Complextreon | 開けた | engine-idle | あり | 27 | 0 | 0 |  |
| 25 | Utility/SFX transitions/SFX transitions | 開けた | engine-idle | あり | 23 | 0 | 0 |  |
| 15 | Product demos/Vocodex demo | 開けた | exited | あり | 25 | 0 | 3445 |  |
| 10 | Utility/Create a chord progression/Create a chord progression | 開けた | exited | あり | 31 | 0 | 3449 |  |

- 開けた 49 件のうち 48 件で Problems の窓が出た(欠けたプラグインを含む)。窓のない 1 件は Blocks(260)。
- 開けない 2 件(Synth_dfr・Olbaid Sytrus)は、「Song info」の窓の後にエンジンが終了した(`exited`。Issue #1365・#1372 の範囲)。
- 終了時の `exited` は 9 件。そのうち 7 件は窓の途中で止まり(`underrunDelta` 3,445〜3,449)、2 件(Synth_dfr・Olbaid Sytrus)は開く途中で止まった。窓の途中で止まった 7 件の `underrunDelta` は、途切れではなく、エンジンが止まった後の数字として読む。

## 3. 無音の原因(推定。切り分けは未了)

- 実測: 開けた 49 件の DLL の要求の欠けた数は 13〜80 件。Problems の窓は 48 件に出た。
- 実測: `--make-sound`(Default の Kick)は音が出た。エンジンの出力の経路は、この VM で動く。
- 推定(未確認): demo の曲は、欠けた DLL(FLEX・Fruity の効果など)を使うため、音の経路が働かない。Blocks は窓が出ないが、同じく無音(サンプルの読みの側と推定)。
- 切り分けには、曲ごとに欠けた DLL の一覧を取り、音源の有無と照らす作業が要る。この Issue の範囲外。

## 4. 道具の直し(この作業)

- `tools/p3-audio-probe/measure.mjs`: 題名の待ちは、開く前の題名から変わった時も終わる(`Song info` は除く)。例: Electric Loop の題名は「The Ogun Dream - FL Studio 2026」で、ファイル名では始まらないが、`error` null で読めた(前回の待ちは、ファイル名で始まる題名だけを認めていた)。`underrunSpans`(AudioContext 開始からの途切れの区間。ms と quanta)を、run モードの出力に足した。
- `tools/p3-audio-probe/measure-live.mjs`: 変更なし。TSV の 1 行目は手順の行で、1 列目が `.flp` の路径ではないので、既定の Default が使われる(#1401 の判定のまま)。
- 前回の差分の版(3-way の衝突)は、main の版(#1401 の `--make-sound` を含む)に、上の 2 点を足して解消した。

## 5. 受け入れの状態

- 1 段目(`npm run build --prefix app`・`cargo build --release --locked`・`fe-build engine`・`fe-build bundle`)は同じコマンドで実行し、EXIT 0(21:22 UTC)。`fe-build engine` は約 44 分。
- 起動の確認: `tools/boot-check/boot-bundle.mjs` の結果は `state` engine-idle、`standIn` false、`error` null、`elapsedMs` 10,423。
- 2 段目(TSV の行から `.tmp-p3-project.flp` を作る)は、見出し行だけの TSV では行が無いので落ちる。n=3 の測定は、この段の後なので実行されていない。

## 6. 次にやること(PM の判断)

1. P3 の入力を、demo(playlist を持つ曲)から、#1401 の Default の手順(音が出る)に変えるか決める。Default の手順は Song Mode の再生ではない(Channel Rack の Pattern の再生)。Song Mode にするには、Playlist に Pattern を置く操作を足す必要がある。
2. demo を使い続けるなら、欠けた DLL の配信(Plugins の束)と、曲が使う素材の配信が要る。その後に、この探索を同じ道具で繰り返す。
3. Synth_dfr・Olbaid Sytrus の「Song info」の窓(開けない)と、`exited`(exit 134)の 9 件は、#1365・#1372 の範囲。

## 7. 以前の試行(run 45315923203)

- Sacco - Exploding Chips(160)を 3 回測った前回の結果は、すべて `nonSilentQuanta` 0、`quantaWhilePlaying` 11,063〜11,074、`underrunDelta` 0。今回の表の同じ曲も `nonSilentQuanta` 0。
- 前回の表(51 件のうち 30 件を開けた)の結果は、今回の表に含めた。道具が違うので、今回の表の値で置き換える。


## 8. #1401 の手順(main の版の inputs/p3-project.tsv から移した)

#1401(音の出る入力を FL の操作で作る)が main に入れた行。この Issue の TSV の形(見出し行と、demo の 1 行)に合わせるため、TSV からは外した。手順の道具は `tools/p3-audio-probe/measure.mjs --make-sound`。

```
default-channel-rack-steps	1) 起動して engine-idle を待つ。部屋は 1836 x 1056 CSS px・DPR 1 にする(tools/p3-audio-probe/measure.mjs --make-sound)。2) ステータス行をクリックして AudioContext を開く。3) Welcome の確認ダイアログの「No thanks」を、Welcome の窓の左上から (512, 307) でクリックする。4) Welcome の開始リストを、(235, 180) で 12 目盛り上げてから 3 目盛り下げ、Default の行を (235, 117) でダブルクリックする(Default の窓が開き、Welcome が閉じるまで待つ)。5) 開いた後の「Problems loading the project」の箱は Enter、「Remix a song」の窓は Escape で閉じる。6) Channel rack(クラス TStepSeqForm)の窓の矩形(左上 (x0, y0)、幅 w、高さ h)で、最初の行(Kick)の 16 個のステップを、左から 1 つずつ 1 回ずつクリックする。行の中心の y は y0 + 0.256 h、ステップ i(0〜15)の中心の x は x0 + (0.448 + 0.0321 i) w。実測(窓が (872, 82) 499 x 246 の時)は x = 1096 + 16 i、y = 145。各クリックの前後で窓の画素を比べ、変われば点灯。7) Space で再生し、10 秒後に nonSilentQuanta を読む。素材は Data\Patches\Packs\Legacy\Drums\Dance\Basic 808 *.wav(4 件。配信済み)。
```


## 9. #1335 の Song Mode(この作業。受け入れの式どおり)

実エンジン: 受け入れコマンドを全段実行し、EXIT 0(14:56 UTC)。起動確認(`tools/boot-check/boot-bundle.mjs`)は engine-idle・`standIn` false・`error` null。流れ A〜D(`tools/smoke/welcome-flow.mjs`)は 4 行とも `error` null・engine-idle・`trapped` false・`hostErrors` 0・`opened` true、B と D は `welcomeClosed` true。Pattern の 3 回と Song Mode の 3 回(`measure.mjs --make-sound` と `--make-sound --song`)は `error` null。

runner の値。この機械とは別枠。nproc 4。loadavg(1 分値)は 1.3〜2.4(各回の終わり)。

| 回 | songMode | playlistClips | quantaWhilePlaying | nonSilentQuanta | nonSilentDelta(窓) | underrunDelta |
|---|---|---|---|---|---|---|
| Pattern 1 | — | — | 16,928 | 3,580 | 3,449 | 0 |
| Pattern 2 | — | — | 16,972 | 3,601 | 3,448 | 0 |
| Pattern 3 | — | — | 16,851 | 3,585 | 3,446 | 0 |
| Song 1 | true | 1 | 18,629 | 3,574 | 3,445 | 0 |
| Song 2 | true | 1 | 18,164 | 3,595 | 3,448 | 0 |
| Song 3 | true | 1 | 18,650 | 3,581 | 3,449 | 0 |

- Pattern の `underrunDelta` の中央値は 0。Song の `underrunDelta` は 0・0・0(基準 + 4 = 4 以下)。
- Song の画素の証拠(変わった画素の数。箱は guest px の DPR 1 の部屋): PAT/SONG の箱(303, 4, 37 x 30)は 904 px 変化。道具の箱(窓の左上から (40, 4), 60 x 22)は 1,320 px 変化(Paint の刷毛から鉛筆へ)。クリップの箱(窓の左上から (280, 72), 58 x 40)は 2,320 px 変化(箱の全画素。Pattern 1 のクリップが 1 小節目・Track 1 に置かれた)。
- 直した点(この作業の途中): 1 回目は、Paint の道具のまま、クリックの x 1649 が格子(小節の線 x 1650)の左に当たり、クリップが置かれなかった(変化は、曲の先頭の印の 2 px の線 80 px だけ)。鉛筆を押し、クリックを x 1652 に直すと、クリップが置かれた。判定の閾値は、クリップ 400 px・PAT/SONG 100 px(印の線は 80 px なので、閾値は線を外す)。

判定: 受理(受け入れの式どおり。Song の 3 回は songMode true・playlistClips 1 以上・音が出る・`underrunDelta` が基準 + 4 以下)。ただし、この 6 回の窓の `underrunDelta` は全部 0 で、途切れの変動は見ていない(推定。runner の基準の版どうしの 4〜14 の変動は、この回には現れなかった)。Song の再生が playlist のクリップを鳴らしていることは、音の量が Pattern と同じ程度(nonSilentDelta 3,445〜3,449)であること以外には確かめていない(推定)。

未確認: demo の曲での Song Mode(この作業の範囲外。#1365・#1372)。P3 の「音が途切れない」の達成は、この判定では言えない。
