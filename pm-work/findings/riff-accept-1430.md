# リフ マシーンの Accept(#1430): 押すと窓が閉じてノートが残る。数え方は、この画面では使えない

判定: **差し戻し**。Accept の位置は見つけた(**受理**)。ノートの数(目標 2)は、数え方の規則が正しく数えないため**未確認**。道具の受け入れの式は、数えた値(36・64)で通るが、その値はノートの数ではない(下の 3)。

実エンジン: この作業のビルド(`fe-build engine` と `fe-build bundle`。12:39 UTC に終了コード 0)の bundle `lift/out-app/bundle/raw/fl-Oz` で、`tools/piano-roll-tools/measure.mjs --tool riff-machine --dprs 1,2.37`(run2・run3)と、窓の探針(一時の道具。記録には残さない)を実行した。流れ A〜D(`tools/smoke/welcome-flow.mjs`)と `npm test --prefix app` も実行した。

runner の値。この機械とは別枠。nproc 4。loadavg(JSON の各行): DPR 1 は 2.15 1.22 1.49、DPR 2.37 は 3.23 2.89 2.17。

## 1. 押す前後の窓の表

- 押す前(`windowNew`。DPR 1 と DPR 2.37 で同じ大きさ): `TPRScoreCreatorForm "Piano roll - riff machine"`。DPR 1 は 751,275・334x506、DPR 2.37 は 2008,998・334x506。visible・painted(`dominantShare` 0.391)。
- 窓の子(DPR 1。位置は guest の絶対値): `TNewCaption` "Piano roll - riff machine" (753,278 330x26)・`TVectorPanel` "Advanced settings" (758,375 320x295)・`TVectorSheet` "Prog" (758,400 267x270)・`TVectorPanel` "" (759,407 265x27)・`TVectorPanel` "Options" (759,442 265x189)・`TQuickContainer` (758,305 320x70)・`TQuickSheetSelector` (758,305 320x33)・`TVectorPanel` "" (759,691 318x82)。
- 押した後(`foundWindows`): 窓は表から消え、残るのは `TFruityLoopsMainForm` "FL Studio 2026" だけ。
- 画面(目で読んだ。OCR は使わない): `riff-accept-1430-opened-dpr1.png`(押す前)。右下の「Accept」は画面の (1037, 745) 付近。窓の矩形に対して (0.856, 0.929)。

窓を開くと、Piano roll に窓の下書きのノート(右側の F#4 など)が出る(推定)。× で閉じると、この下書きが消える(`riff-accept-1430-cancel-dpr1.png` は空の Piano roll)。

## 2. Accept の位置を見つけた方法と、候補の数

方法(製品にも道具にも座標・比率の定数は無い。格子の比率は探索の刻みで、位置ではない): 窓の矩形を 12 列 x 32 行の格子(384 点)で刻み、中心を右下から順に押す。各押下は押したまま 120 ms(窓の矩形の画素のハッシュが変われば「押下の見た目が変わった」)、離して閉じたかを窓の表で確かめる。閉じた押下だけ `countNotes` で数え、1 以上なら Accept とする。

結果:

| 実行 | 押した数 | 閉じた押下(比率 fx, fy → 数えた notes) | 見つけた押下 |
|---|---|---|---|
| DPR 1(run2) | 13 | (0.958, 0.953) → 36 | (0.958, 0.953) |
| DPR 2.37(run2) | 15(開き直し 2) | (0.958, 0.953) → 0・(0.875, 0.953) → 0・(0.792, 0.953) → 64 | (0.792, 0.953) |
| DPR 2.37(run3) | 14(開き直し 1) | (0.958, 0.953) → 0・(0.875, 0.953) → 33 | (0.875, 0.953) |

- 押下の見た目が変わったのは、(0.958, 0.984)(窓の下の縁)と、閉じた押下(DPR 2.37 では 3 回)。
- **Accept の位置の確かめ(画面で)**: 窓の探針で、(0.958, 0.953) を押すと窓が閉じ、ノートが残った(DPR 1: `riff-accept-1430-accept-dpr1.png`。DPR 2.37: `riff-accept-1430-accept-dpr2.37-crop.png` の切り出しに、G4・C4・A3・D4 などのラベル付きノートが並ぶ)。閉じた押下の比率から見ると、Accept のボタンは、比率でおよそ x 0.79〜0.96、y 0.93〜0.96 の範囲(推定。ボタンの右下寄り)。(0.792, 0.953) と (0.875, 0.953) も、この範囲の中で窓を閉じた(run2・run3)。
- 候補の数(格子の中の押下): 13(DPR 1)、14〜15(DPR 2.37)。格子の走査は、閉じた押下を見つけるまで続く。

## 3. 閉じた後の notes・noteRects・noteColour と、数え方の結果

数え方(PM の規則。`countNotes`): 押す前の画面と後の画面の、Piano roll の矩形(`noteWindow`)で、後にだけ現れた色のうち最頻の色の、4 px 以上の連結成分。

- DPR 1(run2): notes **36**・noteColour `#32424c`。36 件の `noteRects` は、すべて y=409 の 1 行に並ぶ(幅 4〜23 px、高さ 17 px)。ノートではなく、Piano roll の 1 行の区画(行の表示の色)。
- DPR 2.37(run2): notes **64**・noteColour `#384852`。64 件は、すべて y=1252 の 1 行に並ぶ(高さ 17 px)。同じく 1 行の区画。
- 原因(確かめた): 押す前の画面は窓が Piano roll の左側を覆う。窓が消えると、その下の画素が見え、Accept で Piano roll の 1 行の表示が変わる(押す前後の画面の画素を比べると、DPR 1 の run1 では y=625 の行が `#2e3e48` から `#32424c` に変わっていた)。規則は、この行の色を「後にだけ現れた色」として最頻に選ぶ。
- 数える範囲は Piano roll の矩形(`noteWindow`)。押す前の画面で窓が覆っていた部分も、この範囲に入る(窓の矩形の外は数えない)。
- 規則を、窓で覆われていない基準(×で閉じた画面)に替えても、規則は 0 を返した(DPR 1: noteColour `#cbfed2`、4 px 以上の連結成分 0。ノートの色が画素ごとに少しずつ違い(1 件あたり 100 色以上)、同じ色の連結成分はノートの形にならない)。DPR 2.37 では、同じ基準で `#384852` の 1 行の区画 64 件。
- 同じ基準で、色を使わず「押す前と違う画素」の連結成分を数えると、DPR 1 のノートの形の区画は 35 件(高さ 18〜19 px。1 件は再生位置の線で、ノートではない。y 240〜800 の範囲)。この数は、規則ではなく探針の確かめで、道具には入れていない。

## 4. Cancel の確かめ

- × で閉じる(窓の比率 (0.955, 0.030)。探針): 窓は閉じ、Piano roll にノートが 1 つも残らないことを画面で確かめた(`riff-accept-1430-cancel-dpr1.png`)。DPR 2.37 でも、同じ × の押下で同様(探針の画面。PNG は残していない)。
- **ただし、規則の「notes 0 なら Cancel」は使えない**: DPR 2.37 の run3 で、閉じた押下 (0.958, 0.953) の前後の画面では、緑の区画が 14 件増えた(窓が覆っていた部分にノートが現れた)。探針でも、同じ比率の押下でノートが残った。道具は、この押下の notes 0 を Cancel とみなして次へ進んだ。

## 5. 道具の JSON(DPR 1 と DPR 2.37。run2。要点だけ)

受け入れの式の項目: `error`・`stopped`・`engine`・`warnings`・`notes`・`noteRects`(数と各 w・h)。

```
{"dpr":1,"error":null,"engine":"engine-idle","stopped":false,"warnings":[],"notes":36,"noteRects":36(w 4〜23, h 17, 全部 y=409),"noteColour":"#32424c","presentedDelta":133,"elapsedMs":10287,"preBootBytes":307266034,"nproc":4,"loadavg":[2.15,1.22,1.49],"acceptSearch":{"grid":"12x32","candidates":384,"tried":13,"reopened":0,"found":{"fx":0.958,"fy":0.953}}}
{"dpr":2.37,"error":null,"engine":"engine-idle","stopped":false,"warnings":[],"notes":64,"noteRects":64(w 5〜23, h 17, 全部 y=1252),"noteColour":"#384852","presentedDelta":533,"elapsedMs":10197,"preBootBytes":307266034,"nproc":4,"loadavg":[3.23,2.89,2.17],"acceptSearch":{"grid":"12x32","candidates":384,"tried":15,"reopened":2,"found":{"fx":0.792,"fy":0.953}}}
```

(JSON の 2 行は要点だけに切り詰めた。`steps`・`logTail`・`stopLines` などの全体は、実行の出力のとおり。)

道具の受け入れの式は、この 2 行で通る(`error` null・`stopped` false・`engine` engine-idle・`warnings` 空・`notes` ≥ 1・`noteRects` の数と各 w・h ≥ 4)。しかし、数えた値は行の区画で、ノートの数ではない(3)。したがって、この通過は目標 2 の達成とみなさない。

## 6. 流れ A〜D(`tools/smoke/welcome-flow.mjs`)

```
{"flow":"A","error":null,"state":"engine-idle","standIn":false,"trapped":false,"hostErrors":0,"welcomeClosed":true,"opened":true,"elapsedMs":10082,"preBootBytes":307266034,"startStalls":0,"hostErrorText":null,"imageReadsAfterIdle":[],"steps":["first idle at 10078 ms (title \"Welcome to FL Studio - in ?trial mode?\")","Welcome window at (481, 241) 874x573","No thanks at guest (993, 548)","Default row at guest (716, 358), 2 click(s)","end: state engine-idle, title \"Problems loading the project\", welcome window closed"]}
{"flow":"B","error":null,"state":"engine-idle","standIn":false,"trapped":false,"hostErrors":0,"welcomeClosed":true,"opened":true,"elapsedMs":10173,"preBootBytes":307266034,"startStalls":0,"hostErrorText":null,"imageReadsAfterIdle":[],"steps":["first idle at 10166 ms (title \"Welcome to FL Studio - in ?trial mode?\")","Welcome window at (481, 241) 874x573","Escape","end: state engine-idle, title \"Problems loading the project\", welcome window closed"]}
{"flow":"C","error":null,"state":"engine-idle","standIn":false,"trapped":false,"hostErrors":0,"welcomeClosed":true,"opened":true,"elapsedMs":9913,"preBootBytes":307266034,"startStalls":0,"hostErrorText":null,"imageReadsAfterIdle":[],"steps":["first idle at 9908 ms (title \"Welcome to FL Studio - in ?trial mode?\")","Welcome window at (481, 241) 874x573","No thanks at guest (993, 548)","Default row at guest (716, 358), 1 click(s)","end: state engine-idle, title \"Problems loading the project\", welcome window closed"]}
{"flow":"D","error":null,"state":"engine-idle","standIn":false,"trapped":false,"hostErrors":0,"welcomeClosed":true,"opened":true,"elapsedMs":10180,"preBootBytes":307266034,"startStalls":0,"hostErrorText":null,"imageReadsAfterIdle":[],"steps":["first idle at 10176 ms (title \"Welcome to FL Studio - in ?trial mode?\")","Welcome window at (481, 241) 874x573","I'm in at guest (841, 548)","after I'm in the Welcome window is still listed","Default row at guest (716, 358), 2 click(s)","end: state engine-idle, title \"Problems loading the project\", welcome window closed"]}
```

判定の式どおり: 4 行とも `error` null・`state` engine-idle・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true。流れ B と D は `welcomeClosed` true。4 流れの `elapsedMs` の中央値 10,128 ms(12,500 以下)。`preBootBytes` の最大 307,266,034 B(314,368,711 以下)。

(流れ A〜D の 4 行は、`tools/smoke/welcome-flow.mjs` の出力を、そのまま貼った。)

## 7. 単体テスト

`npm test --prefix app`: Test Files 76 passed | 1 skipped (77)、Tests 1470 passed | 10 skipped (1480)。終了コード 0。

`node spec/piano-roll-notes.test.mjs`: all checks passed(合成データ 15 件と `areaHash` の 3 件)。

## 8. 前回からの変化

- 前回(run 45911175691 の作業)の止まり(窓の作成で、ゲストのスレッド 20 が `SuspendError: trying to suspend JS frames` で落ちる)は、この build では出なかった。`fle thread` の失敗は 0 件(`engine.log`)。main の #1515(分離 wasm の中継の修正)の後の build で、#1515 の修正との関係は推定。
- 道具の変更(数え方は変えていない): 押す直前の画面(`<tool>-dpr<dpr>-before-accept.png`)と、閉じた押下ごとの前後の画面(`<tool>-dpr<dpr>-close<n>-before.png`・`-after.png`)を、`--shot` で書く。JSON の `notesNote` に、この数え方がノートを数えていないことを書いた。
- 製品(`app/`・`lift/`・`ci/`)は変えていない。

## 9. 次にやること(PM へ。この Issue では行っていない)

1. **数え方の規則を決め直す(依頼者の判断が要る)**。この画面では、色の規則が 2 通りに失敗する(行の表示の色を選ぶ・ノートの色が画素ごとに違い、連結成分にならない)。候補: (a) × で閉じた画面を基準にし、押す前後で違う画素の連結成分を数える。窓の下書きのノートは消えるので、基準に入らない。再生位置の線など、ノート以外の違う画素を除く規則が要る。(b) ゲストのメモリや、エンジンの記録から、ノートの一覧を読む(FL の画面の文字は、メモリの走査か記録で読む、という制約に合う)。
2. **#1386(コード進行ツール)も `countNotes` を使う**。同じ規則の欠陥を持つ可能性があるので、#1386 の数え方も、この結果を見て決め直す。
3. Accept の位置は、探索が見つけた(窓の比率 x 0.79〜0.96、y 0.93〜0.96 の範囲。押すと閉じてノートが残る)。ただし、探索の止め方(閉じて、数えた notes が 1 以上)は、数え方が決まるまで使えない(DPR 2.37 では、ノートが残った押下を、数えた 0 で見逃した)。
4. 受け入れ(`notes ≥ 1`)は、この数え方のままでは、行の区画を数えて通ってしまう。通過を目標 2 の達成と読まないこと。
