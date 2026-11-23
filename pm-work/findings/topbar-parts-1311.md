実エンジン: `lift/target/release/fe-build engine` と `fe-build bundle` でバンドル(raw/fl-Oz)を作り、headless Chrome で engine-idle まで起動して、既定プロジェクトを開き、トップバーの 3 部品の右クリックと項目の選択を 3 回行った(`node tools/topbar-parts/measure.mjs`。受け入れコマンドの全段を 1 回、道具を追加で 2 回)。出力は下の「実測」の JSON 行。

# topbar-parts-1311: C-6 トップバーの 3 部品のメニューと、項目を選んだ後の値

- 種別: 測定の道具の追加(`tools/topbar-parts/measure.mjs`)と、この記録。製品(app/・lift/・ci/)は変えていない。
- 判定: **受理**(受け入れの基準どおり。3 部品すべてで、メニューが出て(menuShown)、項目を選んだ後に表示が変わった(changed))。Song Position の変化は、表示の単位の変更であり、位置は変わっていない(注記)。
- 数字は、すべて runner の値。この機械とは別枠。nproc = 4。loadavg(測定の後の値)= `1.47 1.48 2.12`。

## 1. 起動後の画面と部品の位置(読んだ結果)

- 起動直後の画面(engine-idle)には、Welcome 窓と「Help us improve FL Studio by sending usage data」の確認ダイアログが出る。3 部品は見えない(画面の PNG で確認)。
- 確認ダイアログの「No thanks」を押し、Welcome 窓の「Default (Basic with limiter)」の行を、ダブルクリックで開くと、メイン窓(TFruityLoopsMainForm、780×480)に 3 部品が出る。これが、位置を読めた操作。
- 読んだ位置(ゲストの画素。画面の PNG の読み。道具の座標と一致):
  - テンポ: 「140.000」の枠。x 437〜491、y 8〜33。
  - Song Position: 「1:01:00」(B:S:T)の枠。x 500〜629、y 4〜37。
  - Pattern Selector: 番号「1」の枠。x 592〜639、y 48〜76。
- 確認ダイアログは、道具の中で「No thanks」を押して閉じた(既定の動作は変えていない。#1310 の道具と同じ手順)。

## 2. 値の読み方

- 表示の値は、部品の枠の RGBA の画素を SHA-256 にかけた先頭 16 桁で比べた(項目を選んで、メニューが閉じた後に読む)。入力が無い 1 秒の間の画素の変化(control)は、3 部品とも 0 画素。
- 文字の読み(PNG を見て読んだ。道具は文字を読まない): テンポ 140.000 → 120.000、Song Position 1:01:00(B:S:T) → 0:00:00(M:S:CS)、パターン番号 1 → 2。

## 3. メニューと、選んだ項目

| 部品 | 右クリックの点(ゲスト) | 出た窓(class・位置・大きさ) | 面積の変化 | 選んだ項目(窓の左上からの差) | 表示の前 → 後(画素の比較) | 変わった画素 | 判定 |
|---|---|---|---|---|---|---|---|
| Tempo | (462, 20) | TQuickPopupMenuWindow (457, 20) 322×290 | 1.000 | 「120 BPM」(198, 112) | e29406bb1017b0dd → bf9b556d87a37fe2(140.000 → 120.000) | 291 | 受理 |
| Song Position | (566, 20) | TQuickPopupMenuWindow (566, 20) 179×359 | 1.000 | 「Minute : Second : Centisecond」(84, 46) | fc386c8855dc75df → d0b73b2ba841dd7e(1:01:00 → 0:00:00) | 659 | 受理(注記) |
| Pattern Selector | (616, 58) | TQuickPopupMenuWindow (467, 58) 312×393 | 1.000 | 「Clone」(123, 234) | 2ae908952eb64387 → e5e2f2544e0f5a56(1 → 2) | 650 | 受理 |

- 注記(Song Position): この部品のメニューの項目は、表示の単位(Mode と Middle unit)を変えるだけで、小節の位置を動かす項目は無い。選んだ「Minute : Second : Centisecond」で、表示は「1:01:00」(B:S:T)から「0:00:00」(M:S:CS)に変わった。位置は、どちらも時刻 0 を指すと読める(推定)。
- 注記(Pattern Selector): 「Clone」は、パターン 1 を複製して、パターン 2 を選ぶ(番号 1 → 2)。プロジェクトのパターンの数が増える。
- 試して、選ばなかった項目: 「Insert one」「Find next empty...」は、名前の編集の窓(TNameEditForm)を開いた。「Pattern 1」(今の番号と同じ)は、番号を変えない。どちらも、この測定では選んでいない(項目の選び方の検討の記録)。

## 4. 実測(道具の出力。3 回とも同じ値)

受け入れコマンドの実行(`$RUNNER_TEMP/parts.json`)の行。道具は、直接の実行を 3 回(同一の値)、受け入れコマンドの中で 1 回。計 4 回で、値はすべて同じ。

```json
{"state":"engine-idle","standIn":false,"error":null,"parts":[{"name":"Tempo","menuShown":true,"valueBefore":"e29406bb1017b0dd","valueAfter":"bf9b556d87a37fe2","changed":true},{"name":"Song Position","menuShown":true,"valueBefore":"fc386c8855dc75df","valueAfter":"d0b73b2ba841dd7e","changed":true},{"name":"Pattern Selector","menuShown":true,"valueBefore":"2ae908952eb64387","valueAfter":"e5e2f2544e0f5a56","changed":true}]}
```

起動確認(`boot-bundle.mjs`): state engine-idle、standIn false、error null、chromeVersion Chrome/157.0.8103.78、navigate から engine-idle まで 10316 ms(測り方が P1 の定義と同じかは確かめていない。判定には使わない)。

## 5. 未確認と限界

- 表示の値は、画素と画面の文字で読んだ。エンジン内部のテンポの数値は読んでいない。
- Song Position は、位置そのものが変わった確認は無い(項目が表示の単位だけを変える)。小節の位置の変更は、再生かタイムラインの操作が要る(この測定の範囲外)。
- Pattern Selector の「Clone」は、プロジェクトを変える(パターンを 1 つ増やす)。3 部品の順番は、テンポ → Song Position → Pattern Selector で、Clone は最後に行うため、前の 2 つの測定には影響しない。
- 項目は各部品 1 つずつ。他の項目(「Set...」などの入力の窓)は測っていない。
- 測定の道具の座標は、この版の既定プロジェクトの画面の位置。画面の配置が変わると、項目の位置(窓の左上からの差)を直す必要がある。

## 6. 道具

- `tools/topbar-parts/measure.mjs`: 起動 → 確認ダイアログを閉じる → 既定プロジェクトを開く → 3 部品について、(1) 入力の無い 1 秒の画素の揺れ、(2) 右クリック、(3) 新しい窓の確認(窓の中に点があり、面積の半分以上が変わった)、(4) 項目を 1 つ選ぶ、(5) 残った窓を Escape で閉じる、(6) 値の画素を比べる。stdout は JSON の 1 行、stderr は部品ごとの 1 行。
