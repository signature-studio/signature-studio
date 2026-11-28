# plugin-database-rows: Installed の行が一覧に描かれない原因の観測(Issue #1535)

状態: 実測(この作業の runner。bundle は `fe-build engine` 1 回、`fe-build bundle` 2 回で作った)。推定は「推定」と書く。

## 1. 画面(実測。`--shots` の画像を `read` で見た)

- 実行 1(項目は `Presets\Plugin database\Installed\<名前>.nfo` の直下。分類データ 3 本あり): Browser は `Plugin database` > `Installed`(折りたたみ。左に ▸)。その下に行は無い。
- 実行 2(項目を `Installed\Effects\`・`Installed\Generators\` に置いた): `Installed` > `Effects` が展開され、Fruity Balance・Fruity Delay Bank・Fruity Delay・Fruity Fast Dist・Fruity Fast LP・Fruity Filter・Fruity Flanger・Fruity Flangus・Fruity NoteBook 2・Fruity PanOMatic・Fruity Parametric EQ・Fruity Phaser・Fruity Reeverb 2・Fruity Reeverb の 14 行が描かれた。`Generators` > Sytrus も描かれた。
- 実行 2 の画像に `Directory is empty!` は見えない。

## 2. FL が探した物(実測。エンジンのログ。Installed 行をクリックして右で開いた後)

| 順 | 要求 | 結果 | 実行 |
|---|---|---|---|
| 1 | `Data\Patches\Plugin database\Installed\`・`Effects\`・`Generators\` | 無い(コード 2) | 1・2 |
| 2 | `Presets\Plugin database\Installed.nfo`・`Effects.nfo`・`Generators.nfo` | 無い(コード 2) | 1 |
| 3 | `Presets\...\Installed\.Plugins.ini` | 無い | 1・2 |
| 4 | `Presets\...\Installed\*.*`(列挙) | 15 件(項目の .nfo) | 1 |
| 5 | `Presets\...\Installed\<名前>.fst`(項目ごと 15 本) | 全部無い | 1 |
| 6 | `Presets\...\Installed\*.*`(列挙) | 4 件(`Effects`・`Generators` の 2 つの dir と、`Effects.nfo`・`Generators.nfo`) | 2 |
| 7 | `Presets\...\Installed\Effects\*.*`(列挙) | 28 件(14 本 × .nfo と .fst) | 2 |
| 8 | `Presets\...\Installed\Generators\*.*`(列挙) | 2 件(Sytrus の .nfo と .fst) | 2 |
| 9 | `Presets\...\Installed\Effects.fst`・`Generators.fst` | 無い | 2 |
| 10 | `Presets\...\INSTALLED\EFFECTS`・`INSTALLED\GENERATORS`(と Data\Patches 側) | 無い | 1 |
| 11 | `Presets\...\Installed\FLEX.NFO`・`Emphasizer.NFO`(と Effects\・Generators\ の下) | 無い | 2 |

- 読み: 実行 1 で FL は、項目の `.nfo` を列挙し、項目ごとに同じ名前の `.fst` を探して、無かった(5)。さらに、カテゴリの folder(`Installed\Effects`・`Installed\Generators`)も無かった(10)。実行 2 は、カテゴリの folder と、その下の `.nfo`・`.fst` を読んで、行が描かれた(7・8)。
- 見つかったファイルの読み(`CreateFileW`・`ReadFile`)は、このログに出ない(製品のログは「無い」と「列挙」だけ。窓の API の trace は利用者のプロファイル配下の開く・属性だけを出す)。**確認できない**。

## 3. 原因の判定

- (a) 分類データの欠落: **当てはまった**(実測。前の作業の記録: `Data\Patches\Plugin database\Installed.nfo`・`Effects.nfo`・`Generators.nfo` が seed に無く、missing だった)。3 本を seed に足した。足した後は missing にならなかった(実測)。
- (a') 項目の置き場と形の不一致: **当てはまった**(実測。実行 1 → 実行 2 で、同じ分類データのまま、行の有無が変わった)。原因は、項目を `Installed\` の直下に置いたこと(カテゴリの folder が無い。各項目に `.fst` が無い)。
- (c) 測り方: **当てはまった**(実測)。`databaseHas` は名前をログに求める。ログに名前は出ないので、製品の状態に関係なく偽になる。
- (b) 被り・(d) 時機: 画像で、行が描かれていない実行 1 は WebView2 の面と無関係の位置にあった。確認できない(時機は未測定)。

## 4. 直したこと

- 分類データ 3 本の seed 行(`lift/harness/seeds/vfs-seed-idle1.tsv`): `Data\Patches\Plugin database\Effects.nfo` 71 B `04fb2887968ad6fe5b273e99415e310fb4bb197c655e40100d480b51f6ec520a`、`Generators.nfo` 61 B `bb1e16ab4d84cb6e5e1d42bea44c3d25a1a724f8950d259532e8b13e2d7b57fd`、`Installed.nfo` 67 B `3a52f215ae83cc9a1e5246be86f50e7f1381a71f38bc98abc14eb992d36d6ec2`。(`inputs/fl-installer-files.tsv` の値と一致。実物は `Plugin database (alphabetical)` の同名の物と同じ SHA-256)
- 項目の形(`lift/crates/fe-build/src/plugins.rs` の `plugin_db_rows`): 項目は `Installed\<Effects|Generators>\<名前>.nfo`(中身は `Tip=<名前>`。`Bitmap=` は書かない。理由: 実験で `Bitmap=` が web の画像の取得を起こした)と、`<名前>.fst`(空)。カテゴリは、本体 DLL のパスの上の folder から決める(製品のコードに名前を書かない)。`Installed.nfo`・`Effects.nfo`・`Generators.nfo` の 3 行は、分類データの同じ 3 本を指す(新しい中身は書かない)。
- `.fst` を空にした理由: 実物の `.fst` は、分類データの木の全部に在るわけではない(`Fruity Delay`・`Fruity Fast LP`・`Fruity Reeverb` は `Plugin database (alphabetical)` に無い)。中身の形式を、この作業で確かめていない。**確認できない**: 空の `.fst` を挿入したときの動作。
- E2E(`tools/e2e/lib/plugins.mjs`・`tools/e2e/scenarios/smoke/plugin-database-rows-drawn.mjs`): 判定は、`Installed\Effects\*.*` の列挙が、内蔵のエフェクトの数の 2 倍(.nfo と .fst)以上であること、engine-idle。`Directory is empty!` の文字列は、guest のメモリに常に在る(FL の文字列表)ので、判定に使わない(実測: 実行 2 でも真になり、画像には出ていない)。画像は `read` で見て報告する。

## 5. 確認できない・未確認

- 内蔵プラグインの一覧は、`lift/plugins/` の 15 件だけ(Installed は 15 項目)。インストーラの 100 を超える内蔵、外部の Serum 2・OTT・Vital は含まれない。
- 実行 2 の `FLEX.NFO`・`Emphasizer.NFO` の探索(11)は、名前の一覧の外の物を FL が探す様子。原因は未確認。
- 挿入(Mixer の picker から項目を選ぶ)は未確認。
- 読む時機(列挙の後に行が出るまでの時間)は未測定。
