# C-2 描画の分離の前後で、音声の余裕を実測した結果(#1304)

測定: run 45029825845 の artifact `agent-1304`(`report.md`)。作業者は、分離の後の版(main、fc10a51e)と分離の前の版(14fad0bf。43a6eb88 の親)を、headless Chrome で bundle fl-Oz に対して `tools/p3-audio-probe/measure.mjs` で測った。値は runner の値で、この機械とは別枠。nproc は 4、loadavg(1 分平均、各実行の終わり)は 0.96〜2.12。

## 判定

改善が見えない。圧迫は、描画の分離の後も残る。

- underrunDelta は、前後とも、UI 負荷の有無に関わらず、10 秒あたり 200〜300 台。再生の quantum(10 秒で約 3,460)の約 6〜9%。
- 前後で差は見えない(範囲が重なる)。負荷の有無でも差は見えない。
- 測定の目的(分離が圧迫を減らしたか)は、この結果で答えられた。残る要因は次の節の候補。

## 結果(窓 10 秒、各条件 n=3)

負荷の定義: デスクトップ中央で半径 30 px の円を 50 ms ごとに CDP のマウス移動で動かし、10 秒の窓で続けた。

| 負荷 | 版 | underrunDelta(窓 10 秒、平均(範囲)) | engineGapFrames 増分(平均(範囲)) | presentsDelta(平均) |
|---|---|---|---|---|
| UI | 前 | 257(199〜301) | 39,298(33,268〜46,580) | 30 |
| UI | 後 | 267(238〜289) | 51,955(45,170〜55,710) | 27 |
| なし | 前 | 268(247〜292) | 39,541(36,344〜44,062) | 27 |
| なし | 後 | 262(239〜281) | 41,547(32,758〜58,140) | 21 |

- 負荷なしでも提示は 20〜28 回/10 秒起きる(再生に伴う表示の更新と見られる)。描画の仕事は両方の版に残る。
- マウス負荷を外しても underrun はほぼ変わらない。マウスによる UI の処理は原因ではない。
- engineGapFrames はエンジンが無音で埋めた量。ページ側の読みではなく、エンジン側の書き込みの遅れを示す。

### 分離の後の版・UI 負荷の 3 回

| 回 | quantaDelta | underrunDelta | engineGapFrames 増分 | presentsDelta |
|---|---|---|---|---|
| 1 | 3463 | 238 | 54984 | 21 |
| 2 | 3462 | 274 | 55710 | 29 |
| 3 | 3456 | 289 | 45170 | 30 |

### 分離の前の版・UI 負荷の 3 回

- underrunDelta: 301・199・272(平均 257)
- engineGapFrames 増分: 46,580・38,046・33,268(平均 39,298)
- presentsDelta: 3 回とも 30
- quantaDelta: 確認できない(artifact の記録に無い)

### 負荷なしの 6 回

- 個別の値は確認できない(artifact の記録は平均と範囲のみ)。
- presentsDelta は 20〜28。

## JSON 12 行

確認できない。作業者が付録に入れた JSON 12 行は、commit `4ffdd5cd`(このリポジトリにも origin にも無い。push されなかった)にあった。artifact の `report.md` と `judge.txt` には JSON が無い。

## 起動の確認

分離の後の版は boot-bundle で確認: `state` が engine-idle、`standIn` が false、`error` が null(`elapsedMs` 11315)。

## 残る要因の候補(#1334 へ)

- (a) fl-engine の Worker の中で、描画以外の作業(guest のミキサとスケジューラ)の時間。書き込みの遅れの直接の候補。
- (b) 音声の実時間の待ち(`lift/runtime/fle_audio_transport.c` の 108 行付近)と、書き込みの間隔。
- (c) 描画そのもの。推定では小さい(前回の測定で compose は提示 1 回あたり約 0.4 ms。今回は composeMs を測っていない)。

(a) と (b) は、書き込み 1 回の所要時間と間隔の分布を測ると分けられる。

## 取れなかった項目

- compose の回数と時間: 道具の JSON に出ないため、確認できない。
- 書き込みの遅延の分布: JSON には合計と直近 60 秒の値だけがある。`engineSkippedFrames`・`engineLateWrites` は JSON に出ないため、確認できない。
- quantum ごとのジッタと AudioWorklet の CPU 時間: 取れていない。
- P3(Song Mode)は判定していない。所定のプロジェクトが無く、全 12 回で `nonSilentQuanta` が 0(無音の出力)。耳の確認も行っていない。

## 手順

1. 測定の道具は `tools/p3-audio-probe/measure.mjs`(#1303)。`--load ui` は窓の間、ポインタを 50 ms ごとに動かす。`--load none` は動かさない対照。stdout に JSON 1 行、stderr にログ。engineGapFrames の増分は、stderr の from・to の行から取った。
2. 分離の後の版(main)は、app と lift をビルドし(`npm run build --prefix app`、`cargo build --release --locked --manifest-path lift/Cargo.toml`、`fe-build engine`、`fe-build bundle`)、boot-bundle で起動を確認してから、`--load ui` を 3 回、`--load none` を 3 回測った。
3. 分離の前の版は、worktree(14fad0bf)で作った。道具は main と `cmp` で同一のものを worktree にコピーして使った(main の道具で前の bundle を測ると、main の app が配信されるため)。FL のバイナリ(`lift/out-app/host`)が worktree に無いので、main の抽出物を symlink で張った。配信された dist に `fl-render`(描画の Worker)が無いことを確かめた(分離の後の dist には在る)。同じ条件で 6 回測った。
4. 判定: 各回の JSON で、`error` が null、AudioContext が running、`presentsDelta` が 0 より大きい(負荷が効いている)、全 12 回で `nonSilentQuanta` が 0 であることを確かめた。
5. 後片付け: worktree と一時ファイルを消した。製品のコード(app/・lift/・ci/)は変えていない。

## 受け入れコマンドとの差

1. `$RUNNER_TEMP` を、リポジトリ内の `.tmp-c2-run` に向けた(AGENTS.md の「リポジトリの外に作らない」)。stderr は、窓の開始値のためにログへ取った。
2. 分離の前の版は、`measure.mjs` を worktree にコピーして測った。main の道具で前の bundle を測ると、道具が main の app を配信してしまうため。
3. worktree には FL のバイナリ(`lift/out-app/host`)が無いので、main の抽出物を symlink で張った。受け入れの手順にはこの張り方が無く、判定の再実行は `error: gen-all: lift/out-app/host/lift/out-ext/dsp_ippv2_x64.dll: No such file or directory` で落ちた(judge の出力)。そのため、Issue に `failed` が付いた。
4. `--load none` の対照は、Issue の指定に無い追加。残る要因を絞るために取った。
5. 最後の判定(JSON の検査と `test -s`)は、findings を書いた後に実行し、通った。
