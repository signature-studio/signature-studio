# e2e-full の初回の結果(run 45854456293・commit 8b3bdeb9)の読み取り

- 対象: `e2e-full` の run 45854456293(head 8b3bdeb9。shard 52 job と網羅の表 1 job)。成果物は `.tmp-full/`(PM が落とした shard の `e2e.jsonl` と `coverage.json`)。
- 読み取りのみ。製品・`tools/`・`.github/`・`spec/` は変えていない。`gh` は読み取りだけ使った。
- 実測と推定を分ける。推定には「推定」と書く。確認できない物は「確認できない」と書く。
- 集計の根拠: shard の各行(display 行 2 本: 4k-dpr1 と 4k-dpr2)の `scenarios[]`。判定の規則は `tools/e2e/lib/plan.mjs`(`rowPasses`)と `tools/e2e/judge-coverage.jq`。

## 0. 要点

- 985 シナリオ(ID)のうち、両方の表示で未印の合格は **361(36.6 %)**。結果 1,970 件(ID × 表示)のうち、未印の合格は 976、印つきの合格は 13、印つきの失敗は 651、未印の失敗は 261(うち error 25)、skip は 69。
- **成功した job のなかに、印つきの失敗が 540 件ある**(成功 28 job のうち 12 job)。plugins の成功 10 job は、印つき失敗だけで通っている。
- 最も重い製品の欠陥は、**言語の切替でエンジンが停止する**(exit 134)こと。`settings/language-*` の 7 本(dpr1)すべてで停止の記録が付く。印 `new`(Issue 未起票)のため受理された。
- 失敗の大半は道具の欠陥: (1) `--displays 4k-dpr1,4k-dpr2` が各シナリオの宣言を無視し、宣言に DPR 2 を含むのは 985 本のうち 1 本だけなのに、ほぼ全部を dpr2 でも走らせる、(2) dpr2 で UI が約 2 倍に描かれるのに、固定の guest px 座標を使う(settings 123 件・windows 98 件)、(3) plugins の `position-*` 277 件は、プラグインを挿入せずに窓を探すため、構造上通らない。
- plugins の `8 / 303`: 挿入の段(Plugin database の項目)で 100 % 止まり、窓・パラメータ・音声・止まらないの段に進んだ結果は **0 件**。通った 8 本は、マニフェストの確認(3 本)・picker の表示(2 本)・Plugin database の画面とログの読み取り(3 本)で、プラグインの挿入は 0 本。
- 網羅 611/1,730 は、宣言外の表示で通った結果を含む。宣言どおりに数えると 605(page の crisp の 6 機能: `dpr-1-25`・`random`・`dpr-1-5`・`dpr-2-5`・`dpr-1-75`・`dpr-3`)。
- 停止・例外の計数(pageErrors・consoleErrors・failedRequests・stuckToasts)は、全 1,970 件で 0。配信の検査(servedHeadersMatch)は 104 行すべて真。

## 1. 領域の表

ファイル数は、今(HEAD)と run 時点(8b3bdeb9)で同じ。`browser-face`(2 本)は run の後に入ったので、表に無い。`plugins` の 303 本のうち 1 本(`_generate.mjs`)は生成器で、シナリオではない(302 本)。結果は ID × 表示(2 本)。「error」の列は、error を持つ結果の数(`失敗 未印` に含む)。

| 領域 | ファイル(今 = run) | 載った ID | 結果 | ok 未印 | ok 印つき | 失敗 印つき | 失敗 未印(うち error) | skipped | error の有る結果 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| page | 84 | 84 | 168 | 165 | 0 | 0 | 3 (0) | 0 | 0 |
| input | 3 | 3 | 6 | 6 | 0 | 0 | 0 | 0 | 0 |
| file | 3 | 3 | 6 | 6 | 0 | 0 | 0 | 0 | 0 |
| menus | 3 | 3 | 6 | 6 | 0 | 0 | 0 | 0 | 0 |
| windows | 251 | 251 | 502 | 376 | 2 | 2 | 122 (24) | 0 | 24 |
| audio | 3 | 3 | 6 | 2 | 0 | 4 | 0 | 0 | 0 |
| visual | 3 | 3 | 6 | 5 | 0 | 0 | 1 (0) | 0 | 0 |
| settings | 150 | 150 | 300 | 165 | 0 | 16 | 115 (0) | 4 | 0 |
| plugins | 302 | 302 | 604 | 15 | 0 | 588 | 1 (0) | 0 | 0 |
| topbar | 63 | 63 | 126 | 68 | 5 | 41 | 12 (0) | 0 | 0 |
| resilience | 42 | 42 | 84 | 18 | 4 | 0 | 2 (1) | 60 | 1 |
| perf | 31 | 31 | 62 | 50 | 2 | 0 | 5 (0) | 5 | 0 |
| delivery | 47 | 47 | 94 | 94 | 0 | 0 | 0 | 0 | 0 |
| **合計** | **985** | **985** | **1,970** | **976** | **13** | **651** | **261 (25)** | **69** | **25** |

- 「ok 未印」は、判定と同じ規則(ok・pageErrors 0・consoleErrors 0・stuckToasts 0・failedRequests 0)。
- 表示別(未印の失敗): windows は dpr1 が error の 12 件だけ、dpr2 が 98 + error 12。settings は dpr2 の 115 件だけ、page は dpr2 の 3 件、visual は dpr2 の 1 件。

## 2. 失敗の指紋(件数の多い順。1 件は 1 種類に割り当て)

区分: (i) 道具・シナリオの欠陥、(ii) 製品の欠陥、(iii) runner の環境、(iv) 揺れ。

| # | 種類 | 件数(dpr1 / dpr2) | 代表 3 件 | 見立て(実測 / 推定) | 区分 |
|---|---|---:|---|---|---|
| 1 | plugins: 内蔵・外部・プリセットの項目が Plugin database の一覧に無い(印 `#14xx`・`new`)。picker が開かない 13 件を含む | 309 (148 / 161) | `plugins/builtin-effects-fruity-7-band-eq-fruity-7-band-eq-x64`・`…fruity-delay-3-…`・`…fruity-limiter-…` | 実測: 名前の行が 1 件も無い(一覧の行は 4 行。`enum-dir` の件数は 2)。dpr1 は 1 件目の後の 134 件で一覧の行が出ない(§3 を参照)。推定: 内蔵の wasm の配信がまだ(#1434〜#1476 は open) | (ii) 推定 + (i) |
| 2 | plugins `position-*`: 挿入せずに、プラグインの窓を探す(`no window named`) | 277 (145 / 132) | `plugins/position-effects-edison-edison-x64`・`…fruity-big-clock-…`・`…patcher-…` | 実測: 窓が無いのは当然。推定: `positionSource` は `openPluginDatabase` の後に窓を探すだけで、挿入をしない。構造上、通らない | (i) |
| 3 | settings dpr2: 頁・節・tab の固定座標が外れる(`no panel "…" under …`・`no Presets grid`) | 123 (0 / 123) | `settings/about-control`・`settings/check-input-click-and-hold`・`settings/choice-open-colorful-waveforms` | 実測: dpr1 は 0 件、dpr2 は 123 件。推定: `TAB_X`(dpr1 で測った guest px の定数)で頁を押す。dpr2 では UI が約 2 倍に描かれ、別の頁を押す | (i) |
| 4 | windows dpr2: 固定の guest px で、step・fader・playlist・knob・mute を操作 | 98 (0 / 98) | `windows/step-r1-c05`・`windows/mixer-fader-back-3`・`windows/playlist-t3-b03` | 実測: dpr1 は 0 件。UI の大きさ: メニュー欄 208x298 → 402x604、メニューバー 293x33 → 586x66、picker 176x113 → 325x227(guest px、dpr1 → dpr2)。推定: `RACK_LAYOUT`・`mx.x + 127 + 44k` が外れる | (i) |
| 5 | 予算切れ(skip) | 69 (18 / 51) | `resilience/wait-idle-30min`・`resilience/new-after-storm`・`perf/render-ab-vs-base` | 実測: `--budget 35m` は shard 全体で 1 つの時計。dpr1 行で予算が切れ、dpr2 行の resilience は 42 件すべて skip。settings-5 の 4 件・perf-2 の 5 件も、末尾の skip | (i) |
| 6 | topbar 印 `new`: wheel・click・右クリックが効かない(`no effect (picture unchanged; opened none; menu none)`) | 41 (21 / 20) | `topbar/click-tquickedittoolbarpanel-8`・`topbar/rclick-tquickedittoolbarpanel-10`・`topbar/rclick-twpcontrol-3` | 実測: dpr1 と dpr2 の両方で同じ(製品の状態は同じ)。推定: 機能の未対応か、位置の誤り(固定の座標の可能性)。画像が無く確認できない | 確認できない |
| 7 | windows: `windows/plugin-f8` の後、同じ shard の rack 系 12 件が error(`Channel rack (TStepSeqForm) is not shown: vis false`) | 24 (12 / 12) | `windows/rack-knob-r3-small`・`windows/rack-menu-r2`・`windows/rack-plus` | 実測: plugin-f8 を含む windows-6 でだけ起きる。他 8 shard の rack 系は全部 ok。推定: F8 の後の窓の状態が、次のシナリオに残る(後始末が無い) | (i) 推定 |
| 8 | topbar 印なし: wheel・click・drag が効かない | 12 (3 / 9) | `topbar/wheel-tquickedittoolbar-2`・`topbar/wheel-tquickedittoolbarpanel-11`・`topbar/wheel-tquickedittoolbarpanel-3` | 実測: 印なしのまま失敗。原因は #6 と同じ系統か未確認 | 確認できない |
| 9 | settings `language-*`: 言語の切替で engine が停止(exit 134)。印 `new` | 7 (7 / 0) | `settings/language-fr`・`settings/language-ja`・`settings/language-ko` | 実測: 停止の報告は 2 種類。`language-fr`・`language-es` は `fle_dispatch_miss`(呼び出し元 0x66dd61。`CreateFruityInstance` の中で SIGABRT)。残り 5 件は、停止の報告に実行時の原因が無く、初画素の記録だけ。`language-vi`・`language-zh` は、直前の停止の後に始まった。推定: 既知の `#1382`・`#1417`(閉じている)と同じ系統 | (ii) |
| 10 | perf: 基準との比の超過 | 5 (1 / 4) | `perf/cpu-playing-busy`・`perf/latency-ab-vs-base`・`perf/latency-menu-open-options` | 実測: busy 1.77 % / 基準 0.80 % = 2.2 倍(上限 1.5)。クリック 12.5 ms / 基準 8.2 ms。load 2.4〜2.9(4 コア)。推定: 負荷の影響。基準は別の機械の値 | (iii)・(iv) |
| 11 | audio: 音声のクォンタが増えない(印 `new`) | 4 (2 / 2) | `audio/quanta-while-open`・`audio/no-underrun-while-idle` | 実測: `device playing`・`nodeReady true`・quanta +0(両表示)。推定: runner の音声出力か製品。確認できない | 確認できない |
| 12 | page・visual: 黒い区画の割合が 10 % を超える | 3 (0 / 3) | `page/frame-no-black-in-windows`・`page/reload-picture-after`・`visual/no-black-regions` | 実測: dpr1 7.4 %(150 / 2,040)、dpr2 15.1 %(308 / 2,040)。計数は全区画を分母にする(`blackInWindows`)。画像が無く、空の机か描画の欠けかは確認できない | 確認できない |
| 13 | plugins: その他(Plugin Manager の項目が窓を出さない・Plugin database の項目を選んでも画面が変わらない) | 3 (1 / 2) | `plugins/plugin-manager-opens`・`plugins/plugin-database-item-select` | 実測: Plugin Manager は外部の実行ファイル(`#1499`、open)。項目の選択は、dpr1 では画面が変わり、dpr2 では変わらない | (ii) 推定 |
| 14 | windows: F8 の後の F12 で Plugin list が残る(印 `new`) | 2 (1 / 1) | `windows/zz-f12-plugin` | 実測: `TPlugListForm` が残り、top-level の一覧に出る | 確認できない |
| 15 | page: 宣言外の表示で走る crisp(`dpr:1` の宣言、dpr2 で実行) | 1 (0 / 1) | `page/crisp-screen-pixels` | 実測: dpr2 で 7,954 / 8,285 の画素が違う。スクリーンショットは CSS の大きさ(1920x1080)。シナリオ自身が「DPR 1 だけ」と書いている | (i) 確定 |

残り(各 1 件): resilience の `demo-open-replace-50`(Ctrl+O の箱を答えない。error。履歴では 2 件が既に answered)、`resilience/tab-hidden-repeat`(隠す・表示の繰り返しで描画が戻らない 0 回。確認できない)、settings の `choice-pick-colorful-waveforms`(Display の選択。印 `new`)。

印つきで ok になった 13 件(印が古い可能性): `perf/render-min-fps-new-project`(印 `new`)、`resilience/random-ops-01・05・07・09`(印 `new`)、`topbar/click-tquickedittoolbar-2`・`topbar/drag-twpcontrol-2`・ほか(印 `new`)、`windows/zz-mixer-insert-7`(印 `new`)。

## 3. plugins の `8 / 303` の理由

- シナリオは 302 本(`_generate.mjs` を除く)。結果は 604(2 表示)。通ったのは 8 本: `balance-lazy-entry`・`sytrus-lazy-entry`・`manifest-lists-plugins`(マニフェストの読み取り。UI は使わない)、`picker-opens`・`picker-position-random`(picker だけ)、`plugin-database-browser-opens`・`plugin-database-count`・`plugin-database-item-select`(Plugin database の画面と、ログ全体の読み取り)。
- **実測**: 4 段(窓・パラメータ・音声・止まらない)に進んだ結果は 0 件。内蔵・外部の 145 本は、すべて「Plugin database に項目が無い」で止まる。`position-*` の 145 本は、挿入をしないので窓が無い(277 件)。
- **Plugin database の一覧(実測)**: 項目の名前は 1 件も無い。`plugin-database-count` が見た `enum-dir` の件数は 2(dpr1・dpr2 とも)。`#1484`(「Plugin Database が空」)は閉じているが、この run では同じ状態。
- **列挙の行の数(実測)**: dpr2 の行は、毎回 4 行。dpr1 の行は、各 shard の 1 件目だけ 4 行で、以後の 134 件は 0 行。推定: 道具の観測(`engineLog` の開始位置)か製品の再列挙。確認できない。どちらでも、「Installed folder is empty in this build」という note の文言は、観測から言い切れない(`lib/plugins.mjs`)。
- **ログの上限**: `EngineLogView` は 50,000 行まで保持する(`app/src/ui/panels.ts`)。ただし、dpr1 の DB の行は全体で 29 件しか無い(ログは上限に達していない)。上限の仮説は、この数字からは支持されない。
- **閉じた Issue の印**: `#1315`・`#1336`・`#1337`・`#1338`・`#1400`・`#1484` は閉じているが、印の付いた 34 件は、すべて ok にならない。

## 4. windows の failure の理由

- 4k-dpr1 は 237 件が合格。失敗は、plugin-f8 の後の連鎖(12 件の error)と、印 `zz-f12-plugin`(1 件)だけ。
- 4k-dpr2 は 139 件が合格。98 件が、固定の guest px の座標で外れる(#4 の表)。
- **実測**: 画の大きさは 3840x2160 で dpr 間で同じ。一方、UI の要素は約 2 倍(上の #4)。`lib/windows.mjs` の冒頭の前提(「the guest picture is the same size at every DPR」)は、画の大きさについてだけ正しく、座標の定数には当てはまらない。
- **連鎖(#7)**: `windows/plugin-f8` を含む shard(windows-6)だけで起きる。両表示で、同じ位置から 12 件。推定: F8 で開く Plugin list が、Channel rack の可視を変えたまま戻さない(製品か後始末の不足かは確認できない)。

## 5. e2e-full のワークフローの課題

- **予算(`--budget 35m`)の扱い**: 時計は shard 全体で 1 つ。dpr1 行が予算を使い切ると、dpr2 行は全部 skip(resilience の dpr2 は 42 件すべて skip)。job の所要は 36.8 分と 41.6 分(resilience)。予算は、シナリオを始める前にだけ見るため、最後のシナリオが超える。
- **表示の上書き**: `--displays 4k-dpr1,4k-dpr2` を付けると、`rowsOf` がシナリオの宣言を無視する(`lib/plan.mjs` の `explicit`)。宣言が `4k-dpr1` だけ、または `4k-dpr1,4k-random` のシナリオは、ほぼ全部が dpr2 で走る。宣言が `dpr:1.25`・`dpr:2.5`・`dpr:3` などのシナリオは、宣言の DPR では一度も走らない(網羅の 6 機能が、宣言と違う表示で覆われる)。`4k-random`(乱数の DPR)も、この run では一度も走らない。
- **印つきの規則**: `rowPasses` は、印つきのシナリオで engine の停止(`engineStops`)を見ない。このため、印 `new` の言語の停止 7 件は、受け入れの判定で理由にならなかった(settings-4・settings-5 の job が落ちたのは、別の未印の失敗による)。
- **job の成否**: 成功 28 job のうち 12 job に、印つきの失敗 540 件がある。失敗 24 job の未印の失敗は 261 件。
- **網羅の表(`report` job)の失敗**: 失敗の理由は `網羅の判定`(`judge-coverage.jq`)。網羅の最小(page 80・windows 250・plugins 200 など)に届かない、という設計どおりの不合格。表(job summary)は書かれている。ただし、テストの失敗と網羅の不足が、同じ `failure` の結論に混ざる。
- **job の割り方**: input・file・menus・visual・audio は、各 3 本のシナリオを 3 job に分けたため、1 本の job が 2〜3 分(計 9 job)。逆に plugins の 11 job は 14〜18 分、settings は 27〜37 分。
- **長い待ち**: `resilience/wait-idle-30min` は 30 分の固定の待ち。予算の中で 2 表示の行に入ると、job の上限(`timeout-minutes: 60`)を超えうる(推定。この run では skip で、実測は無い)。
- **証拠の不足**: `--shots` と `--engine-logs` は run で使っていない。黒い区画・停止の前後・Channel rack の状態を、画像やログで確かめられない。JSON にシナリオごとの所要時間も無いため、予算の使い方は確認できない。

## 6. 提案(実装はしない)

### (A) `tools/e2e/` で直せる欠陥

1. `lib/plan.mjs` の `rowsOf`(`run.mjs`): `--displays` を付けても、シナリオの `displays` の宣言を守る(宣言に無い表示では走らせない)。
2. `lib/plan.mjs` の `rowPasses`: 印つきのシナリオでも `engineStops`・`engineStuckRunning` を見る(印は「既知の失敗の内容」に限る)。
3. `lib/windows.mjs`(`RACK_LAYOUT`・`mx.x + 127 + 44k`)・`lib/settings.mjs`(`TAB_X`)・`lib/plugins.mjs`(`DB_ITEM`)・`lib/topbar.mjs`: 固定の guest px を、窓の表の矩形から求め直す。冒頭の前提文も直す。
4. `lib/plugins.mjs`(`databaseLines`・`pluginSteps`): 一覧の観測を、`engineLog` の開始位置に頼らない。note の「the Installed folder is empty in this build」を、計数と名前の有無に置き換える。
5. `scenarios/plugins/position-*.mjs`(`_generate.mjs` の `positionSource`): 挿入(Plugin database の項目)を通してから窓を探す。今は 277 件が構造上通らない。
6. `scenarios/plugins/plugin-database-count.mjs`・`plugin-database-browser-opens.mjs`(ほか): `databaseLines(guest)`(ログ全体)を使わず、開始位置から読む。
7. `scenarios/page/crisp-*.mjs`・`zoom-*.mjs`: 宣言した DPR と違えば失敗させる。今は `crisp-dpr-3` が DPR 1 と 2 で通り、網羅の 6 機能が過大になる。
8. `scenarios/resilience/demo-open-replace-50.mjs`: Ctrl+O の箱を `lib/flow.mjs` の `openProjectFile` で答える。
9. `scenarios/resilience/wait-idle-30min.mjs`: 予算の残りが 30 分より少なければ始めない。2 表示の行で走らせない。
10. `scenarios/windows/plugin-f8.mjs`・`zz-f12-plugin.mjs`: 後始末(Plugin list を閉じ、Channel rack を出す)。
11. `lib/picture.mjs`(`blackTiles`)・`lib/page.mjs`(`blackInWindows`): 分母を窓の中の区画にする(今の `blackInWindows` は、窓の外の区画も分母に入れる)。空の机の扱いを決め、失敗時は画像を保存する。
12. `scenarios/perf/*`: 基準(`tools/e2e/golden/perf/ab-base.tsv`)は別の機械の値。比は同じ run の A/B で取る。
13. 閉じた Issue の印(`#1315`・`#1336`・`#1337`・`#1338`・`#1400`・`#1484`、34 件)を見直す(印を外すか、Issue を開き直す)。
14. `scenarios/settings/language-*.mjs`: 停止と重なる再読込の判定を、停止の無い時だけ判定する。停止の時は停止として数える。
15. `scenarios/topbar/*`: 位置を窓の表から求め直し、dpr2 の `no effect` が位置の誤りでないかを確かめる。

### (B) 製品の欠陥(Issue の題名の案・見込みの優先度)

1. **高**: 言語の切替で engine が停止する(exit 134)。`settings/language-fr`・`-ja`・`-ko`・`-de`・`-es`(停止)、`-vi`・`-zh`(直前の停止の後)。`fle_dispatch_miss` は `CreateFruityInstance` の中。`#1382`・`#1417` の系統の再発か。印 `new` で、Issue は無い。
2. **高**: 内蔵・外部のプラグインが Plugin database の一覧に出ない。`plugins/builtin-*`(142 本)・`external-*`(3 本)。`#1484` は閉じているが、この run では再現。`#1434`〜`#1476` が対応の Issue。
3. **中**: トップバーの部品(TQuickEditToolbar・TWPControl・TNewMenu・分割線)の wheel・click・右クリックが効かない。`topbar/*`(印 `new` 41 件・印なし 12 件)。`#1346`(閉じている)と関係があるか。位置の誤りの除外が先(A-15)。
4. **中(確認が先)**: 音声のクォンタが増えない(`audio/quanta-while-open`・`audio/no-underrun-while-idle`、両表示)。runner の音声出力か製品かを、先に確かめる。P3 に関わるので、製品なら優先度を上げる。`#1349`(閉じている)と関係あり。
5. **中(確認が先)**: 言語を選んで再読込すると、言語の欄が英語の時と同じ(7 本の dpr1)。`#1383`(閉じている)と合わない。停止と重なるため、停止を直した後に確かめる。
6. **低〜中(確認が先)**: F8 で Plugin list を開くと、Channel rack の可視が `false` のまま(`windows/plugin-f8` の後の 両表示で各 12 件)。F12 で Plugin list が残る(`zz-f12-plugin`)。
7. **確認が先**: 黒い区画が dpr2 で 15 %、dpr1 で 7.4 %。画像で、空の机か描画の欠けかを確かめる(`--shots`)。
8. **確認が先(低)**: busy の比 2.2 倍・クリックの比 1.5 倍超。runner の負荷の影響が大きいと見る(推定)。

### (C) `e2e-full` の改善

1. `--displays 4k-dpr1,4k-dpr2` を外す(宣言どおりに走らせる。乱数の DPR は既定の `4k-random` で動かす)。
2. 予算を、表示ごとに分ける(または、表示ごとに shard を分ける)。resilience の dpr2 行が全部 skip されている。
3. `wait-idle-30min` は、制限時間を長く取った別の job に出すか、表示 1 本だけで走らせる。
4. input・file・menus・visual・audio(各 3 本)は、1 job にまとめる(9 job → 3 job)。
5. 結果の表に、印の番号ごとの件数と、engine の停止の一覧を入れる。
6. 網羅の判定(`report`)を、テストの合否とは別の結論にする。
7. 失敗したシナリオだけ、`--engine-logs` と `--shots` を artifact にする(黒い区画・停止の前後・Channel rack の状態を確かめるため)。
8. JSON に、シナリオごとの所要時間を入れる(予算の見積もりに必要)。
9. `timeout-minutes: 60` は、予算 35 分と最後のシナリオの長さを合わせて決め直す。

### (D) 全体の健全度

- 985 本のうち 361 本(36.6 %)が、両方の表示で未印の合格。615 本は dpr1 行で合格。
- 未印の失敗 261 件の内訳: settings 115(すべて dpr2。固定の座標、#3)、windows 122(dpr2 の座標 98(#4)と、連鎖の error 24(#7))、topbar 12(#8)、perf 5、page 3、resilience 2(error 1 と tab-hidden 1)、plugins 1、visual 1。skip の 69 件は、この 261 件には入れていない。
- **重大な所見(クラッシュ)**: 言語の切替での停止(製品の欠陥 (B)-1)。language-* の 7 本の記録すべてに停止が付き、印 `new`(Issue 無し)のため、依頼者が見える一覧には出ていない。
- **重大な所見(音)**: 音声のクォンタが増えない(2 本・両表示で計 4 件)。runner の音声出力か製品か、確認できない。音の途切れ(P3)に直結するため、先に確かめる。
- **重大な所見(黒い領域)**: dpr2 で黒い区画が 15 %。画像が無いので、欠けか机かは確認できない。
- 道具の欠陥が、失敗の大半を占める(#2・#3・#4・#5・#7・#15 と (C) の表示の上書き)。製品の欠陥として数えるのは (B) の 1・2 が中心。

## 7. 確認できなかったこと

- 画像(`--shots`)とエンジンのログ(`--engine-logs`)は、この run に無い。黒い区画・停止の前後・topbar の操作の結果は、推定にとどまる。
- シナリオごとの所要時間は、JSON に無い。予算の切れ目の正確な時刻は確認できない。
- runner の音声出力の有無は、この run からは分からない(`audio/*` の 4 件)。
- 閉じた Issue(`#1484`・`#1382`・`#1383` など)が、この run の状態と合わないのは、印や scenario の誤りか、修正の不足かは確認できない。
- 乱数の DPR(`4k-random`)と、宣言の DPR(`dpr:1.25`・`dpr:2.5` など)の結果は、この run に無い。

## 付録: 集計の方法

- shard の `e2e.jsonl`(52 本。各 2 行)を読み、各 `scenarios[]` を `{area, id, display, ok, expectFail, pageErrors, consoleErrors, failedRequests, stuckToasts, engineStops, engineState, engineStop, error, notes, skipped}` で見た。
- 判定の分類は、1 件を 1 種類に割り当てる順序の規則で行った(成功合格 → 予算切れ → 印つきで ok → 領域・表示・文言の順)。集計は node の一時の script(作業後に削除)で行い、`coverage.mjs` の出力(611/1,730)を再現して、数字の定義を確かめた。
- `gh run view 45854456293 --json jobs` と `--log-failed` を、読み取りだけに使った。
