実エンジン: fl-Oz(engine 3387a473dd00)を headless Chrome で起動し、`tools/p3-audio-probe/measure-live.mjs` で段 1〜3 を各 n=3、段 0・参考を測った。出力は受け入れの 9 行の window と、各回の JSON(窓の途切れ・入力の届き・周回・`nonSilentQuanta`)。

**判定: 差し戻し**(受け入れコマンドは `ACCEPT_EXIT=1`)。
- 途切れの条件(段 1〜3 の窓の途切れが 1 以下、入力の届きが毎秒 1000 以上): 9 回すべて満たした。**受理**。窓の途切れは 9 回とも 0。
- 音の条件(段 2・3 の `nonSilentQuanta` > 0): 6 回すべて 0。**差し戻し**。音が出ていない(原因は下の 6。サンプルとプラグインが読めない)。

測定の値は runner の値。この機械とは別枠。nproc 4、loadavg(1 分値)は各回の終わりで 0.14〜1.15。画面は guest 3240x1864 device px(Issue の 1863 より 1 px 多い。JSON の `screenDevicePx` のまま)。

## 1. 受け入れの出力(最終版の道具。最後の実行の window 行)

```
window {"last60sAt10s":2170,"last60sAt130s":0,"underrunDelta":0,"nonSilentQuanta":0,"nonSilentDelta":0,"playing":true,"sent":819.5,"processed":819.1,"cycles":1712.2,"pointer":{"sent":72000,"errors":0,"maxInFlight":15},"rateOk":true}
window {"last60sAt10s":2208,"last60sAt130s":0,"underrunDelta":0,"nonSilentQuanta":0,"nonSilentDelta":0,"playing":true,"sent":0,"processed":0,"cycles":1057.9,"pointer":null,"rateOk":null}
window {"last60sAt10s":2181,"last60sAt130s":0,"underrunDelta":0,"nonSilentQuanta":0,"nonSilentDelta":0,"playing":true,"sent":809.4,"processed":809,"cycles":1701.9,"pointer":{"sent":72000,"errors":0,"maxInFlight":65},"rateOk":true}
window {"last60sAt10s":2177,"last60sAt130s":0,"underrunDelta":0,"nonSilentQuanta":0,"nonSilentDelta":0,"playing":true,"sent":806.3,"processed":805.7,"cycles":1701.4,"pointer":{"sent":72000,"errors":0,"maxInFlight":75},"rateOk":true}
window {"last60sAt10s":2139,"last60sAt130s":0,"underrunDelta":0,"nonSilentQuanta":0,"nonSilentDelta":0,"playing":true,"sent":0,"processed":0,"cycles":1057.6,"pointer":null,"rateOk":null}
window {"last60sAt10s":2166,"last60sAt130s":0,"underrunDelta":0,"nonSilentQuanta":0,"nonSilentDelta":0,"playing":true,"sent":810.2,"processed":809.8,"cycles":1703.2,"pointer":{"sent":72000,"errors":0,"maxInFlight":38},"rateOk":true}
window {"last60sAt10s":2182,"last60sAt130s":0,"underrunDelta":0,"nonSilentQuanta":0,"nonSilentDelta":0,"playing":true,"sent":802.4,"processed":801.1,"cycles":1698.9,"pointer":{"sent":72000,"errors":0,"maxInFlight":68},"rateOk":true}
window {"last60sAt10s":2129,"last60sAt130s":0,"underrunDelta":0,"nonSilentQuanta":0,"nonSilentDelta":0,"playing":true,"sent":0,"processed":0,"cycles":1057.8,"pointer":null,"rateOk":null}
window {"last60sAt10s":2249,"last60sAt130s":0,"underrunDelta":0,"nonSilentQuanta":0,"nonSilentDelta":0,"playing":true,"sent":806.2,"processed":805.3,"cycles":1702.3,"pointer":{"sent":72000,"errors":0,"maxInFlight":94},"rateOk":true}
ACCEPT_EXIT=1
```

## 2. 段 1〜3(受け入れと同じ条件、n=3。窓は idle の +70〜+130 秒)

| 段 | 回 | 窓の途切れ `last60sUnderruns`(+130 s) | `underrunDelta` | `inputRateOk` | 届いた毎秒の最小 `pointerEventsPerSecMin` | 届いた平均 | 送った / 処理 (/s) | 周回 (/s) | `nonSilentQuanta` | `screenDevicePx` |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 無音・マウス 1000 Hz | 1 | 0 | 0 | true | 1197 | 1200 | 819.5 / 819.1 | 1712.2 | 0 | 3240x1864 |
| 1 | 2 | 0 | 0 | true | 1194 | 1200 | 806.3 / 805.7 | 1701.4 | 0 | 3240x1864 |
| 1 | 3 | 0 | 0 | true | 1196 | 1200 | 802.4 / 801.1 | 1698.9 | 0 | 3240x1864 |
| 2 再生・マウス無し | 1 | 0 | 0 | null | — | — | 0 / 0 | 1057.9 | 0 | 3240x1864 |
| 2 | 2 | 0 | 0 | null | — | — | 0 / 0 | 1057.6 | 0 | 3240x1864 |
| 2 | 3 | 0 | 0 | null | — | — | 0 / 0 | 1057.8 | 0 | 3240x1864 |
| 3 再生・マウス 1000 Hz | 1 | 0 | 0 | true | 1196 | 1200 | 809.4 / 809.0 | 1701.9 | 0 | 3240x1864 |
| 3 | 2 | 0 | 0 | true | 1197 | 1200 | 810.2 / 809.8 | 1703.2 | 0 | 3240x1864 |
| 3 | 3 | 0 | 0 | true | 1197 | 1200 | 806.2 / 805.3 | 1702.3 | 0 | 3240x1864 |

- 送りは公称 1200 Hz(1000 Hz の 1.2 倍)で、60 秒で 72000 件を送った(`pointerErrors` 0)。未応答の最大は 15〜94 件(上限 256 には届いていない)。
- 段 2・3 の `nonSilentQuanta` は 6 回とも 0。音の条件は満たせない(6 を参照)。

## 3. 段 0(参考。判定に入れない)

- `last60sUnderrunsAtIdle10s`(idle の +10 秒): 最終版の受け入れの 9 回で 2129〜2249。参考の 3 回で 2129・2155・2173。
- 窓の終わり(+130 秒)は、どの回も 0。
- 依頼者の観察の 343(起動から約 10 秒後の画面)と、この読み(engine-idle の +10 秒)は基準の時刻が違うので、比べない。起動の山は #1352 の領域。

## 4. 参考(n=3。判定に入れない。最終版の道具)

| 条件 | 回 | 窓の途切れ(+130 s) | 届いた毎秒の最小 / 平均 | 処理 (/s) | 周回 (/s) | `nonSilentQuanta` | 備考 |
|---|---|---|---|---|---|---|---|
| 段 0 無音・無操作(+10 s の参考) | 1〜3 | 0・0・0 | — | 0 | 1057.9・1057.8・1057.6 | 0 | +10 s: 2173・2129・2155 |
| 再生・125 Hz(公称 150) | 1 | 0 | 150 / 150 | 105.7 | 1088.0 | 0 | 届いた |
| 再生・125 Hz | 2 | 0 | 148 / 150 | 105.6 | 1089.9 | 0 | 届いた |
| 再生・125 Hz | 3 | 0 | 149 / 150 | 104.7 | 1079.9 | 0 | 届いた |
| 再生・8000 Hz(公称 9600) | 1 | 0 | 5419 / 6642 | 2850.7 | 3484.8 | 0 | 届かない(判定 false)。未応答が上限 256 に達した |
| 再生・8000 Hz | 2 | 0 | 6145 / 8685 | 2616.0 | 2993.1 | 0 | 同上 |
| 再生・8000 Hz | 3 | 0 | 6422 / 8423 | 2729.2 | 3181.3 | 0 | 同上 |
| 再生・DPR 1・1000 Hz(画面 1836x1056) | 1 | 0 | 1197 / 1200 | 811.4 | 1706.8 | 0 | `+10 s`: 311 |
| 再生・DPR 1・1000 Hz | 2 | 0 | 1197 / 1200 | 810.4 | 1710.4 | 0 | `+10 s`: 388 |
| 再生・DPR 1・1000 Hz | 3 | 0 | 1197 / 1200 | 804.3 | 1706.2 | 0 | `+10 s`: 420 |

- 8000 Hz は届かない(届いた毎秒の最小は 5419〜6422、平均は 6642〜8685。公称の 2/3 前後)。CDP の応答が追いつかず、未応答が上限 256 に張り付いた。前回の試行で起きた Node のヒープ不足は、送りの上限を内側の繰り返しにも掛けて直した(2 の注)。
- 段 0 の `+10 s` は runner の起動の山。DPR 1 では 311〜420 で、画面が軽い。

## 5. 原因: 入力と周回の比例と、時間の内訳

**(a) 周回は入力に比例して増える(実測)。** 段 2(入力無し)の周回は 1058/s。

| 送り | 処理された入力 (/s) | 周回の増分 (/s) | 増分 ÷ 処理 |
|---|---|---|---|
| 125 Hz(公称 150) | 105 | 約 25 | 約 0.24 |
| 1000 Hz(公称 1200) | 801〜819 | 641〜654 | 約 0.8 |
| 8000 Hz(公称 9600) | 2616〜2851 | 1935〜2427 | 約 0.74〜0.85 |

1000 Hz と 8000 Hz では、入力 1 件ごとにおおよそ 1 周する(前回の試行の 0.8〜0.9 と同じ傾向)。125 Hz では、入力の間隔(約 8 ms)が engine の自前の周(約 1 ms)の中に入るため、ほとんど増えない。

**(b) 時間の内訳(プロファイル。各 1 回・n=1。CDP のプロファイルを付けたため、負荷が増えて窓の途切れが 8・4・7 出た。途切れの判定には使わず、内訳にだけ使う)。** engine の Worker の忙しい時間(60 秒あたり、自己時間):

| 項目 | 段 2 再生・無入力 | 段 1 無音・1000 Hz | 段 3 再生・1000 Hz |
|---|---|---|---|
| 忙しい時間(busy) | 1491 ms | 8850 ms | 8981 ms |
| `(program)`(ブラウザ内の処理) | 119 | 1311 | 1286 |
| `f_863b50`(wasm。名前が表に無い) | 101 | 1164 | 1173 |
| `postMessage` | 上位外 | 542 | 537 |
| `decode`(文字列の取り出し。推定: `appview.ts` の `readAppView`) | 上位外 | 476 | 475 |
| `setTimeout`(周の予約。推定) | 48 | 309 | 314 |
| `fle_win_input_view` | 上位外 | 288 | 288 |
| `poll_waiters` | 99 | 212 | 225 |
| `fle_r_blit`(描画の転送) | 95 | 上位外(増えていない) | 上位外 |
| `audio_sync`(音声の同期。無音) | 27 | 上位外 | 上位外 |
| 最長の、切り替え点の無い連続の実行 | 2.2 ms | 8.7 ms | 14 ms(うち GC 12.4 ms) |

- 増分は約 7.4 秒/60 秒(約 12% of one core)。内訳は、入力の読み・通知・周の予約の段(`fle_win_input_view`・`postMessage`・`decode`・`setTimeout`・`f_863b50`)が大半。描画の転送(`fle_r_blit`)は増えていない。
- 推定: 入力 1 件ごとに周が回るのは、engine が待機中(parked)のときに、移動 1 件ごとに Worker へ `wake` を送るため(`app/src/engine/client.ts` の 126〜129 行)。wake の回数は数えていない。
- 音声を作る側の時間は、無音なので測れていない。音が出る条件で測るまで、音声の書き込みへの影響は未確認。

## 6. 直し方(この回では未実施)

- 途切れは、段 1〜3 で既に 0(9 回すべて)。途切れを直す根拠は、この runner では出ていない。
- 音が出ないので、P3 の判定が出せない。音が出ない測定では、直しの効果を音の途切れで示せない。
- 直しの候補(未実施。決めるのは PM): (1) 待機中の wake をまとめる(入力 1 件ごとに 1 周しない。入力の遅れは数 ms 増える)、(2) 描画の通知(`postMessage` の `frame`)を、画面が変わった周だけにする、(3) `readAppView` の文字列の取り出しを、seq が変わった時だけにする(推定の確認が要る)。
- 直す前に、音の出る入力で段 2・3 を測る(#1366・#1365 の後)。

## 7. 道具の変更(前回の staged の版からの差分。`tools/p3-audio-probe/measure-live.mjs`)

- 送りの公称を 1.2 倍にした(`SEND_FACTOR`)。前回は 1.0 倍で、届いた毎秒の最小が 992 だった。
- 判定を仕様どおりにした(毎秒の件数が `--hz` 以上)。前回の「合計 0.99 倍・毎秒 0.95 倍」の式は取り消した。
- 未応答の上限 256 を、まとめて送る内側の繰り返しにも掛けた。8000 Hz の試行では、これが掛かっておらず、未応答が積もって Node のヒープ不足で落ちた。送りに締め切り(窓の 3 倍)も付けた。
- JSON に `pointerEventsPerSecMean`・`pointerMaxInFlight` を足した。
- `--project` は未確認。Ctrl+O でファイルの窓が開かず(前回の試行)、この枝には #1335 の開く経路(`d5696f7a`)が無い。受け入れは `--project` を使わない。

## 8. 取り除いた経路・変えたパス・製品の変更

- 製品(`app/`・`lift/`・`ci/`)は変えていない。main の deploy は動かない。
- 製品への計測の追加は無い(取り除く物も無い)。
- 変えたパス: `tools/p3-audio-probe/measure-live.mjs`(前回の staged の版を直した)、`pm-work/findings/measure-109-attempt2.md`(この報告)。
- 変更の前後の値: 製品を変えていないので、前後の値は同じ。道具の変更の前後では、段 1・3 の送りの届きが 992〜997(前回の 1.0 倍)から 1194〜1197(1.2 倍)に変わった。
- テスト: `spec/guard.test.sh`・`spec/workflow.test.sh` は通った(`exit 0`)。

## 9. 未達と残り(PM へ)

1. **音が出ない(差し戻しの理由)。** 既定の Default は、サンプル 4 件とプラグイン 2 件が読めず、出力が無音(前回の試行と同じ)。#1335 の Blocks も同じく `nonSilentQuanta` が 0 だった。前提は #1366(内蔵サンプルの配信)と #1365(demo の 2 件が開けない)。この機械では、それらの Issue の状態は確かめていない。
2. 8000 Hz は届かない(届いた頻度は 5400〜6400/s。公称 9600 の 2/3 前後)。送りは CDP の応答で頭打ちになる。
3. 窓の途切れは 0 だが、音が無いので、音声の負荷は小さい状態の測定。実際に音が出る時の余裕は未確認。
4. 原因の内訳はプロファイルの 1 回ずつ(n=1)。プロファイルの負荷で途切れが出た点に注意。
5. 注意: 依頼者の 343(起動から約 10 秒後)と、この道具の段 0 の読み(engine-idle の +10 秒。2129〜2249)は、基準の時刻が違う。比べない。
