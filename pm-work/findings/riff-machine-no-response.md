# リフ マシーン(Riff machine)が反応しない: 再現と原因(#1387)

実エンジン: `tools/piano-roll-tools/measure.mjs --tool riff-machine --dprs 1,<乱数>`(F7 → Tools > Riff machine → 警告が出たら閉じる → 窓の主ボタン)と、`tools/smoke/welcome-flow.mjs` の流れ A〜D を、直した後の bundle で実行した(2026-11-25 16:21〜16:24 UTC)。見た物は、道具の JSON、エンジンのログ(`--log`)、画面の PNG(`--shot`。目で読んだ。OCR は使わない)。`npm test --prefix app` は 16:26 UTC に通した。

判定: **受理**(段階 1 の範囲: 窓の表示・警告が無い・配信の資料の列挙)。窓を開いた後の止まり(#1429)と、Accept の後のノート(#1430)は未確認。

runner の値。この機械とは別枠。nproc 4。loadavg は JSON の各行(`loadavg`)。

## 1. 受け入れの判定(直した後の bundle)

| 条件 | DPR 1 | DPR 2.00 |
|---|---|---|
| `error` が null | ○ | ○ |
| `windowNew` に visible・painted の `TPRScoreCreatorForm` | ○ (751,275・334x506。dominantShare 0.371) | ○ (1669,803・334x506。0.371) |
| `warnings` が空 | ○ (`[]`) | ○ (`[]`) |
| `stopLines` に MessageBoxW が無い | ○ (0 件) | ○ (0 件) |
| `missingPaths` に配信の資料の `Riff machine\*.*` が無い | ○ (0 件) | ○ (0 件) |
| `elapsedMs` が 12,500 以下 | ○ (9,374) | ○ (9,287) |
| `preBootBytes` が 314,368,711 以下 | ○ (312,311,979) | ○ (312,311,979) |

流れ A〜D(`tools/smoke/welcome-flow.mjs`。直した後の bundle): 4 行とも `error` null・`state` engine-idle・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true。B と D は `welcomeClosed` true。`elapsedMs` は 9,289・9,504・9,817・9,602(中央値 9,553 ms)。`preBootBytes` 312,311,979 B。

`npm test --prefix app`(16:26 UTC): Test Files 74 passed | 1 skipped (75)、Tests 1439 passed | 10 skipped (1449)。

受け入れのコマンドを一本のまま通した(17:09〜17:12 UTC。engine の作り直しから。終了コード 0)。道具の DPR は 1 と 1.48。判定の式(窓・警告・資料・時間・bytes)と流れ A〜D の判定の式は、どちらも通った。流れ A〜D の `elapsedMs` は 9,092・9,145・9,171・9,762 ms(中央値 9,158)、`preBootBytes` は 312,311,979 B。

## 2. 直す前と後の道具の JSON(DPR 1)

直す前(HEAD の seed 表。Riff の資料の行は無い。run 45592009894 の記録):

```
{"tool":"riff-machine","dpr":1,"error":"no TPRScoreCreatorForm window after the item","engine":"engine-idle","stopped":false,"presentedDelta":null,"notes":null,"noteRects":null,"noteColour":null,"noteWindow":null,"windowNew":null,"warnings":[],"warningBox":null,"elapsedMs":11116,"preBootBytes":312222067,"nproc":4,"loadavg":[1.58,1.32,1.25],"logLengthAtItem":3036,"stopLines":["== stop: returned from entry","fle: MessageBoxW NOT SHOWN, answered by the virtual machine with the default button (id 1; S-ia2-msgbox-default), type 0x10, caller 0x86aadc, caption \"Error\", text \"Some required files are missing.\""],"missingPathsCount":81}
```

直した後(DPR 1。`missingPaths`・`logTail`・`steps`・`room` は省いた。`stopLines` は止まりの行だけ):

```
{"tool":"riff-machine","dpr":1,"error":null,"engine":"exited","stopped":true,"presentedDelta":0,"notes":0,"noteRects":[],"warnings":[],"windowNew":[{"cls":"TPRScoreCreatorForm","title":"Piano roll - riff machine","x":751,"y":275,"w":334,"h":506,"visible":true,"painted":true,"dominantShare":0.371}],"elapsedMs":9374,"preBootBytes":312311979,"loadavg":[1.42,1.24,1.14],"nproc":4,"stopLines":["fruity: == stop: SIGABRT (runtime abort, see runtime message)","fruity: runtime message: first non-background pixel on a drawing surface (decision 0126): written by UpdateLayeredWindowIndirect into the surface of top-level window lid 8; function entries since the program began: 0","fle: indirect call to 0 is not a known function entry"]}
```

`notes` の 0 は数えた値ではない(止まった後の画面)。§5 を見る。

## 3. 窓の表・警告・`missingPaths`

- 窓の表の差分: 直す前は新しい最上位の窓が無い(`windowNew` null。MessageBoxW の既定の答えだけ)。直した後は `TPRScoreCreatorForm "Piano roll - riff machine"` が出る(DPR 1: 751,275 の 334x506。DPR 2.00: 1669,803 の 334x506)。visible・painted(最頻の色の割合 0.371 < 0.95)。PNG(開いた直後)でも、「Step 1. Creates the progression」と右下の「Accept」を含む窓が描かれているのを確かめた。
- 警告: `warnings` は空。MessageBoxW の行は 0 件。「Sytrus couldn't be found」の文は出ていない(ログに 0 件)。Sytrus のモジュールは読み込まれている(`real module table: name=generators.sytrus.sytrus_x64.dll base=0xc580000`)。
- `missingPaths` の Riff の行(DPR 1 の一覧。配信の資料とユーザーの領域を分けた):
  - 配信の資料(`C:\Program Files\...`): `Riff machine\*.*` の失敗は 0 件。5 つのフォルダ(`Data\System\Riff machine\`・`Scores\Arpeggiator\Riff machine\`・`Quantization\...`・`Chopping\...step 1\`・`step 2\`)の列挙は、すべて成功(`enum-dir`)。
  - 配信の資料の見本の探索: `Data\Patches\Packs\Legacy\Drums\Dance\Basic 808 {Kick,Clap,HiHat,Snare}.ZGR` の `GetFileAttributesW 2`(無い)4 件。別の拡張子の探索。`.wav` の見本は seed 表に在る。推定: 正常な「無い」の応答。
  - ユーザーの領域(`C:\Users\user\Documents\Image-Line\...`)の 20 行: `Presets\Scores\Chopping\Riff machine step 1\*.*`・`step 2\*.*`・`Arpeggiator\Riff machine\*.*`・`Quantization\Riff machine\*.*` の `FindFirstFileW 3`(無い)4 件と、`Default.fsc` の `GetFileAttributesW 3` 16 件。判定に入れない(受け入れの式は `Users\` を含む行を除く)。推定: 実機でも、ユーザーの領域に Riff 用のフォルダは無いのが普通。

## 4. 止まり(窓が描かれた後。#1429 の入力として、そのまま)

窓が描かれた後、Sytrus の中で止まる(DPR 1 と DPR 2.00 で同じ形。ログは `--log` の出力):

```
fruity: == stop: SIGABRT (runtime abort, see runtime message)
fruity: runtime message: first non-background pixel on a drawing surface (decision 0126): written by UpdateLayeredWindowIndirect into the surface of top-level window lid 8; function entries since the program began: 0
fle: indirect call to 0 is not a known function entry
```

- 番地は回ごとに違う。前回までの回では、`fle: indirect call to 0xcccccccccccccccc ...`・`... to 0x8a6170`・`... to 0x48002e3e48002e3e`(記録の 22 と同じ形)を見た。未初期化の値か、読み違えた関数ポインタ(推定)。
- Accept を押さない版でも同じ位置で止まった(前回の記録の 23)。止まりは、窓を開いて Sytrus を使い始めた時の物。
- Accept の押下と止まりの前後は、この記録では分けていない。Accept の後の画面は、止まった後の画面と同じ(#1430)。

## 5. 数え方(`notes`)と、合成データの確かめ

- 実装: `tools/piano-roll-tools/measure.mjs` の `countNotes(before, after, rect)`。窓の表から取った Piano roll の矩形(guest 座標)の中で、「後の画面にあって前の画面に無い色」のうち最頻の色を、ノートの色とする(表に埋め込まない)。その色の画素の 4 方向の連結成分のうち、幅 4 px 以上・高さ 4 px 以上の物を数える。JSON は `notes`・`noteRects`(x・y・w・h、guest 座標)・`noteColour`・`noteWindow`。画素の変化の総数では判定しない。
- 実測: 止まったため、Accept の後の画面は使えない。`notes` 0 は数えた値ではない。**未確認。**
- 判定の式の確かめ(合成データ。`spec/piano-roll-notes.test.mjs`。出力):

```
ok  one block 10x6 that is new (notes): 1
ok  one block: the rectangle: [{"x":5,"y":5,"w":10,"h":6}]
ok  one block: the note colour, measured: "#40c080"
ok  one block: the judgement: "pass"
ok  two blocks; a 3x3 dot and a 10x3 bar are left out (notes): 2
ok  two blocks: the rectangles: [{"x":5,"y":5,"w":10,"h":6},{"x":30,"y":20,"w":4,"h":4}]
ok  two blocks that touch only at a corner are two parts (notes): 2
ok  no colour is new: notes 0: 0
ok  no colour is new: noteColour null: null
ok  no colour is new: the judgement: "fail"
ok  the most common new colour is the note colour: yellow 10x6 counted, the green 3x9 is not: [1,"#ffff00"]
ok  a block cut by the rectangle to 3x3 is not a note: notes 0: 0
ok  dominantShare: a uniform rectangle covers 100%: 1
ok  dominantShare: a 3x3 dot: the background covers at least 95% (not painted): true
ok  dominantShare: a 12x40 block: the background covers 80% (painted): [0.8,true]
all checks passed
```

- 窓の判定(`painted`): 最頻の色が矩形の 95% 未満なら描かれている(`dominantShare` を JSON に出す)。直す前の `paintedShare`(左上の画素と違う割合)は廃止。
- 窓の主ボタン(Accept): 窓の矩形の比率(`MAIN_BUTTON`。窓の表に文字が無いため)。押す前に窓の表を取る。比率は DPR 1 の画面の読みで決めた物。

## 6. 取り込みの衝突の解消

- 現在の作業ツリーに、前回の差分の OTT の変更は無い。`lift/crates/fe-build/src/main.rs`・`lift/crates/fe-build/tests/plan/bundle.txt`・`lift/harness/web/pack.args` は HEAD(main)の版と同じ(差分 0)。衝突の印(`<<<<<<<`)は残っていない。
- main の 4d73666a(`keep_lazies`・`PLUGINS_DIR`)が正。前回の「4. bundle を作れるようにした(OTT)」は、この版では無効。

## 7. 変えたもの

- `lift/harness/seeds/vfs-seed-idle1.tsv`: 105 行を足した(HEAD との差)。
  - `Data\System\Riff machine\`: フォルダ 1 件と、ファイル 8 件(17,346 B)。
  - `Data\Patches\Scores\` の Riff machine 系: フォルダ 4 件と、ファイル 92 件(21,119 B)。
  - ファイル 100 件の guest・sha256 は、`inputs/fl-installer-files.tsv` と全件一致(確かめた)。
  - 起動の前に取る(`@lazy-file:` にしていない。§8)。
- `tools/piano-roll-tools/measure.mjs`: `countNotes`・`dominantShare`(`paintedShare` を置換)、`warnings`(出た警告を全部記録)、`notes`・`noteRects`・`noteColour`・`noteWindow`、Accept の後の画面の取得、`--log`・`--shot`(目で読む用の PNG)。
- `spec/piano-roll-notes.test.mjs`(新規): 合成データで数え方と窓の判定を確かめる(§5)。
- `pm-work/findings/riff-machine-no-response.md`(この記録)。
- 製品のコード(`app/`・`lift/` の .rs・.c)と `ci/` は変えていない。製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足していない。

## 8. 起動の前に取る量(目標 2)

測り方: 同じ作業ツリーで、seed 表を HEAD の版と直した後の版にして、`fe-build tables` → `fe-build bundle` → `tools/smoke/welcome-flow.mjs` を取った。

| seed 表 | preBootBytes | 流れ A〜D の elapsedMs(中央値) |
|---|---|---|
| HEAD(Riff の行 無し。15:25 UTC) | 312,229,182 | 9,288.5(A〜D は 9,164〜9,497) |
| 直した後(Riff 105 行。16:24 UTC) | 312,311,979 | 9,553(A〜D は 9,289〜9,817) |

- 増加: +82,797 B(+0.027%)。直す前の記録(run 45592009894)の +82,797 B と一致する。`elapsedMs` の差(+66〜+265 ms)は、二つの回の流れ A〜D の幅が重なる範囲。推定(runner の揺れ)。
- **増やさないことは、満たしていない。** Riff の 100 件を起動の前に取る。0 にする手として `@lazy-file:`(起動の後に取る。既存の 225 行と同じ仕組み)を試した。2 つの壁に当たって、取りやめた:
  1. 生成 manifest の lazy は 256 件まで(`/lazy: must have <= 256 items`)。今は 235 件で、Riff の 100 件を足すと 335 件になり、bundle が落ちる。上限は製品のスキーマの検査で決まっている。変えるのは製品の変更なので、この Issue では行わない。
  2. `@lazy-file:` の行は、`lift/harness/web/pack.args` の `--lazy-file` の行(229 組。手で保つ)も要る。行だけを変えると、エンジンが最初の idle に届かず終わった(`state exited before the first idle`)。
- 依頼者の判断が要る: 上限を上げるか、Riff の資料を別の方法で取るか。試した作業ツリーは取り消し、この記録の版(起動の前に取る)に戻してある。

## 9. 次にやること(PM へ。この Issue では行っていない)

1. **Sytrus の呼び出しの止まり(#1429)**: §4 の行をそのまま入力にする。窓が描かれた後に、Sytrus の中で 0 か未初期化の番地を呼ぶ。Sytrus の像の中の、呼び出し元の番地(前回の記録の 4)を逆アセンブルして、呼び先の出所(IAT・スタック上の関数ポインタ)を確かめる。
2. **起動の前に取る量(§8)**: lazy の上限(256)を上げるか、Riff の 100 件の扱いを決める。
3. **受け入れの式**: `Riff machine\*.*` の判定は、配信の資料の場所(`Users\` を含まない行)に限っている。依頼者の決め。
4. **#1430(Accept の後のノート)**: #1429 の後に計る。`countNotes` は合成データで確かめ済み(§5)。
