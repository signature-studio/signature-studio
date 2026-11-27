実エンジン: 受け入れの chain を全文で実行した(`npm run build --prefix app`・`cargo build --release --locked`・`fe-build engine`・`fe-build bundle`・`boot-bundle.mjs`(engine-idle・standIn false・error null・elapsedMs 12179)・`welcome-flow.mjs`(流れ A〜D の 4 行が通過)・`npm test --prefix app`(75 files passed, 1 skipped / 1465 tests passed, 10 skipped)・`tools/e2e/run.sh --scenarios regression --only settings-general-scroll --displays 4k-dpr1,4k-dpr2`(R13: DPR 1 ok true・DPR 2 ok true)・`guard/scan.sh`(0 件))。R13 は、実測値の欄の内訳を出す版(`changedIn`)で、もう一度だけ走らせた(DPR 1 ok true・DPR 2 ok true。ビルドの入力は変えていない)。
測定の値は runner の値。この機械とは別枠。nproc 4。loadavg は流れ A〜D の終わりで 3.66 / 2.54 / 2.19、R13 の再実行の前で 0.87 / 1.92 / 2.15。

# settings-scroll-1433: Settings の General の窓の外の画素が変わる原因(判定: 受理)

## 0. 結論

- 窓の外が変わる原因は 2 つで、どちらも Settings の描画そのものではない。
  - (a) 683 px(DPR 1 だけ): R13 が、ホイールを送る前にポインタを窓へ動かしていなかった。ホイールの位置へ瞬間移動したため、上部バーのヒント欄(`System / miscellaneous` の文字)が、その下の部品に合わせて変わった。ポインタを先に窓の中央へ置けば出ない(PM の決定 ②)。
  - (b) 1〜26 px(DPR 1 と DPR 2 の両方): 上部バーの CPU パネル(CPU の数字・CPU のメータ・Memory・Frame Rate の欄)が、実際の値に合わせて動いた。依頼者の規則のとおり、入力が無くても変わる実測値の欄。PM の決定 ① で、この欄の矩形を判定から除いた。
- 製品(`app/`・`lift/`)は変えていない。どちらの原因も、製品の欠陥ではなかった(ヒントは、ポインタの下の部品の説明。CPU パネルは実測値)。
- R13 の判定に、(1) 前提(ポインタを先に窓の中央へ置く)、(2) 上部バーの CPU パネルの除外、を入れ、印 `#1433` を外した。期待(窓の外が変わらない・黒が 2% 未満)と、判定の式(`outDown === 0 && outUp === 0 && blank < 0.02`)は変えていない。

## 1. 変わった画素の矩形(直す前と後)

| 試行 | DPR 1 下へ / 上へ(窓の外) | DPR 2 下へ / 上へ(窓の外) | 変わった矩形 |
|---|---|---|---|
| 直す前(R13 の元の流れ。00:53・00:55・00:58 の 3 回、runner) | 696 px / 0 px(3 回とも) | DPR 2(00:54): 0 / 0 | DPR 1: (27,6)-(1037,74)。内訳: 64,64 の 322 px・0,64 の 218 px・128,64 の 143 px がヒント欄(683 px)。1024,0 の 13 px が CPU 欄 |
| 前提だけ入れた(01:25・01:27・02:28。runner) | 13 / 13 px(01:25)、0 / 0 px(01:27)、0 / 0 px(02:28) | 0 / 26 px(01:25)、0 / 0(01:27)、26 / 0(02:28) | DPR 1: (1037,6)-(1037,18)。DPR 2: (2074,12)-(2074,37)。どちらも CPU パネルの中 |
| 前提 + CPU パネルの除外(この試行。05:30・05:31。runner) | 0 / 0 px | 0 / 0 px | 窓の外には、どの DPR でも変化なし。除いた矩形の中の変化: DPR 1 は 0 / 0 px、DPR 2 は 26 / 26 px(CPU メータ) |

## 2. 原因(実測と推定を分ける)

実測:
- (a) ヒント欄(683 px)。元の R13 は、ホイールを送る前にポインタを窓の中央へ動かしていない。CDP の mouseWheel は位置が変わってもポインタの移動イベントを出さない。製品の `app/src/input/tracker.ts` の `wheel()` は、ホイールの位置へ MOVE を先に送る(`app/test/unit/tracker.test.ts` が仕様として固定している)。元の流れのエンジンの記録は、`move at screen (1920, 1080)` の後に `wheel -120` が 6 回。前提を入れた流れは、`wheel` の 6 回だけで move は無い。ヒント欄の文字は MOVE の後に消えた(`settings-scroll-1433-hint-idle.png`・`settings-scroll-1433-hint-down.png`・`settings-scroll-1433-hint-diff.png`)。
- (b) CPU パネル(13 px / 26 px)。上部バーのメータ(`settings-scroll-1433-gauge-diff.png` の赤い縦線)が、入力が無い時も変わる。Settings の窓が無い時も変わった(16 サンプルのうち 3 回)。音は無関係(`audio.detail()` で peak 0・rms 0・nonSilentQuanta 0 を、読んだ 10 回の時点で確認)。
- 窓の内側(Settings の頁)は、スクロールで大きく変わる(約 150,000 px。窓の中の画素は判定の対象外)。前提を入れた後の試行では、窓の外で CPU パネル以外に変わった画素は見つからなかった。

推定(実測ではない):
- (a) は実機の利用では起きないと推定する。実機ではポインタの移動が pointermove として先に届き、ホイールの位置が前回と同じになって MOVE が出ない(`tracker.ts` の move は同じ位置なら何もしない)。R13 の瞬間移動だけが MOVE を出す。
- (b) は CPU 負荷の表示と推定する(`topbar-cpu-fps-1402.md` の §1 の左上の欄)。負荷が動けば変わる。スクロールの描画で負荷が上がるため、スクロールの前後で変化が出やすいと推定する。変化の源(負荷の測り方)は未確認。
- DPR 2 で (a) が出なかった理由は未確認(ヒントが出ていなかった、または位置が違うと推定する)。
- カーソル(#1392 の画像カーソル)は、画素の比較に入らないと推定する。根拠: `app/src/ui/desktop.ts` の `setCursor` は `#desktop` の CSS の `cursor` を変えるだけで、ブラウザが描く。比べている画素は `deskDumpBase64`(コンポジタの画素)。実測はしていない。ツールチップ(ヒント欄の文字)は、(a) のとおり、画素の比較に入っていて、前提で消えた。

## 3. 変えた箇所

- 製品(`app/`・`lift/`): 変更なし。
- `tools/e2e/scenarios/regression/r13-settings-general-scroll.mjs`:
  - ホイールの前に、ポインタを窓の中央へ動かし、1 秒待ってから top の画素を取る(前提 ②)。
  - 上部バーの CPU パネルを、窓の外の判定から除く(`topbarMeasuredRect`。①)。除く矩形は窓の表から取る(DPR の値は決め打たない)。CPU パネルが表に無ければ、除かない(判定は厳しいまま)。
  - 印 `expectFail: '#1433'` を外した。
  - 結果に、`excludedRects`・`excludedChanged`(除いた矩形の中の変化の数)・`outDownBox`・`outUpBox`(窓の外の変化の外接矩形)を足した。
- `tools/e2e/scenarios/regression/common.mjs`: `changedOutside` は、除く矩形を受け取り、数と外接矩形を返す(呼び出しは R13 だけ)。`topbarMeasuredRect` を足した。`OPEN_ISSUES` から `#1433` を外した。
- `tools/e2e/test/regression-outside.test.mjs`(新規。4 件): 除外と外接矩形、CPU パネルの特定(DPR 1 と DPR 2 の表)。
- `pm-work/findings/settings-scroll-1433*`: この記録と、画像 4 枚。

CPU パネルの特定: 表の中の、表示中の `TQuickEditToolbarPanel` の 13 番目(lid の順。`tools/e2e/lib/topbar.mjs` の `splitter-1` と同じ数え方)が仕切りで、その左に隣接する同じ行の欄を CPU パネルとする。DPR 1 で (1033,0) 156x41、DPR 2 で (2066,0) 312x82。欄の数え方が変わると「not found」になり、除外はされない(判定は厳しいまま)。

## 4. 流れ A〜D(`tools/smoke/welcome-flow.mjs`、受け入れの chain の出力)

```
A: error null, state engine-idle, standIn false, trapped false, hostErrors 0, welcomeClosed true, opened true, elapsedMs 12219, preBootBytes 307266034
B: error null, state engine-idle, standIn false, trapped false, hostErrors 0, welcomeClosed true, opened true, elapsedMs 12163, preBootBytes 307266034
C: error null, state engine-idle, standIn false, trapped false, hostErrors 0, welcomeClosed true, opened true, elapsedMs 12203, preBootBytes 307266034
D: error null, state engine-idle, standIn false, trapped false, hostErrors 0, welcomeClosed true, opened true, elapsedMs 12214, preBootBytes 307266034
```

起動の確認(`boot-bundle.mjs`): state engine-idle, standIn false, error null, elapsedMs 12248(12,500 以下)。

## 5. R13 の JSON の要点(受け入れの chain の出力)

- DPR 1(4k-dpr1): ok true、outDown 0・outUp 0・blank 0、excludedRects `[{x:1033,y:0,w:156,h:41}]`、excludedChanged `{down:13,up:13}`(除いた欄の中の CPU メータの変化。窓の外には出ていない)、pageErrors 0・consoleErrors 0・failedRequests 0、engineState engine-idle、achievedFps 38.33(runner の値。120 Hz 目標。P2 の測定ではない)。
- DPR 2(4k-dpr2): ok true、outDown 0・outUp 0・blank 0、excludedRects `[{x:2066,y:0,w:312,h:82}]`、excludedChanged `{down:0,up:26}`、pageErrors 0・consoleErrors 0・failedRequests 0、engineState engine-idle、achievedFps 39.54(runner の値)。
- 窓の外の変化は、どの DPR でも 0 と読めた。除いた欄(CPU パネル)の中の変化は、実測値の欄が動いたもの(DPR 1 は 13 / 13 px、DPR 2 は 0 / 26 px。前回の試行では逆の組み合わせもあった)。

## 6. 受け入れ

- 起動確認: 通過(engine-idle・standIn false・error null・elapsedMs 12248)。
- 流れ A〜D: 通過(上の 4)。
- `npm test --prefix app`: 通過(exit 0)。
- R13(DPR 1 と DPR 2): 両方 ok true。印 `#1433` は外した(`expectFail: null`)。
- `guard/scan.sh`: 0 件。
- 道具の確認(この変更の分): `tools/e2e` の `node --test test/` 83 件 pass。`spec/guard.test.sh` 104 件 ok、`spec/workflow.test.sh` 602 件 ok(exit 0)。

判定: **受理**(R13 が DPR 1 と DPR 2 で ok true。窓の外の変化は 0)。ただし判定は「実測値の欄を除く」の版で、製品の変化を直したものではない(§0・§3)。

## 7. PM の決定の扱い

1. 実測値の欄(CPU パネル)を窓の外の判定から除く: 実施(矩形は窓の表から取り、JSON に出す。他の画素は、今までどおり失敗する)。
2. 前提(ポインタを先に窓へ置く): 実施。
3. P1(10 秒以内の起動): 扱わない。起動の elapsedMs は 12179(受け入れの上限 12500 に入る)。

## 8. 未確認

- カーソル(#1392)の画素への写り: コードからの推定のみ。
- DPR 2 で (a) が出なかった理由。
- CPU パネルの変化の源(負荷の測り方)。
- 除いた欄の中で、CPU・Memory・Frame Rate 以外の画素(枠・背景)が変わらないこと: 差分画像(`gauge-diff.png`)で、赤い縦線の変化だけを見た。全数の確認はしていない。
