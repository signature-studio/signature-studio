# e2e-full の 2 回目の結果(run 46009281599・commit 24aeee99)の読み取り

- 対象: `e2e-full` の run 46009281599(head 24aeee99)。shard 52 job と網羅の表 1 job。成果物は `.tmp-full2/`(PM が取った shard の `e2e.jsonl`・`coverage.json`・失敗 job のログ。commit しない)。
- 製品・`tools/`・`.github/`・`spec/` は変えていない。`gh` は、成果物の取得と Issue の読み(`run view`・`issue list`・`issue view`)、新しい Issue の作成(`create-issues --apply` を 1 回)に使った。
- 実測と推定を分ける。推定には「推定」、確認できない物には「確認できない」と書く。
- 集計の根拠: shard の各行の `scenarios[]`。判定の規則は `tools/e2e/lib/plan.mjs`(`scenarioPasses`・`rowPasses`)と `tools/e2e/judge-coverage.jq`。どちらも commit 24aeee99 の版を `.tmp-full2/src24/` に展開して使った(現在の HEAD の版とは、`lib/plan.mjs` が同じであることを確かめた)。
- 定義: 「未印」は `expectFail` が null の本、「印つき」は null でない本。「ok 未印」は ok・pageErrors・consoleErrors・failedRequests・stuckToasts が 0。「印つき(受理)」は、受理の規則(`scenarioPasses`)で通る印つきの本。「印つき(拒否)」は通らない印つきの本。
- 静的な本の一覧(`.tmp-full2/static-set.json`): 24aeee99 の `tools/e2e/scenarios/` の 1,862 本。各本の `displays` は、run の JSON の `displays` と 1,822 本すべてで一致した(検査済み)。

## 0. 要点

- job は、shard 52 のうち **26 が成功、26 が失敗**。網羅の表(report)は失敗(網羅の判定が exit 1。12 領域が最小に届かない)。
- **行の作り方が変わった(#1506 の効果)**: 各 shard の行数は、計画(宣言と shard の割り方から求めた数)と 52 shard すべてで一致した。表示は宣言どおり。`--displays 4k-dpr1,4k-dpr2` でも、dpr2 で走るのは宣言に 4k-dpr2 がある 2 本(`windows/plugin-f8`・`windows/zz-f12-plugin`)だけ。
- 結果は 1,824 件(1,822 本。2 本だけ 2 表示)。宣言した表示で、未印の本が全結果で ok だった ID は **900 / 1,862(48.3 %)**。前回の 361 / 985(36.6 %)とは母集団(表示の数・本の数)が違うので、直接は比べない。
- 印の無い失敗は **246 件**(error 35 件を含む)。印つきの失敗は **393 件**(受理 387・拒否 6)。予算切れの skip は **268 件**(menus 246・resilience 19・audio 3)。
- 前回の dpr2 の失敗(settings 115・windows 98 など)は、**直ったのではなく、dpr2 で走らなくなった**(宣言に dpr2 が無い本は dpr1 だけで走る)。
- 新しく落ちた群(前回に無かった領域): audio 35、menus 73(と予算切れの skip 246)、file 52、input 16、visual 46(うち 34 は描画の停止の連鎖)。前回から退行した群: settings のテーマ・表示など 14(前回は dpr1 で合格)。
- エンジンの停止は 9 件(製品の停止 8 件 + 閉じた印 #1409 の本 1 件)。系統は 4(§4)。
- plugins: 302 本のうち ok は 8 本(前回の 8 / 303 と同じ)。挿入の段を越えて窓・パラメータ・音声の段に進んだ結果は **0 件**(前回と同じ)。印つきの受理 294 件の内訳は、挿入の段で止まる 146・窓を探す段で止まる 119・picker が開かない 26・その他 3。
- 閉じた Issue の印を使う結果は 22 件(§5)。`menus/add-14-choose`(印 #1411・閉じている)は ok になっており、印を外す対象。
- 起票: 6 件(#1526〜#1531)。受け持ちの無い群を、§6 のとおり。残りは「未起票」(§6)。

## 1. shard の結果と、落ちた規則

判定の規則: job は、各行が `rowPasses`(servedHeadersMatch・error が null・全本が `scenarioPasses`)を満たすと成功。
実測で、servedHeadersMatch の偽・行の error・dprOk の偽は、一件も無い(0)。

| job | 結論 | 行(計画=実際) | 未印で ok | 印つき(受理) | 落ちた規則 |
|---|---|---:|---:|---:|---|
| audio 1/3 | 失敗 | 28 | 9 | 1 | 未印の失敗 18 |
| audio 2/3 | 失敗 | 28 | 8 | 0 | 未印の失敗 17・予算切れ 3 |
| delivery 2/2 | 失敗 | 23 | 22 | 0 | 未印の失敗 1 |
| file 1/3 | 失敗 | 39 | 19 | 4 | 未印の失敗 15・印つきの停止 1 |
| file 2/3 | 失敗 | 38 | 16 | 3 | 未印の失敗 18 |
| file 3/3 | 失敗 | 38 | 14 | 3 | 未印の失敗 19・印つきの停止 1 |
| input 1/3 | 失敗 | 41 | 33 | 3 | 未印の失敗 5 |
| input 2/3 | 失敗 | 37 | 28 | 2 | 未印の失敗 7 |
| input 3/3 | 失敗 | 40 | 31 | 5 | 未印の失敗 4 |
| menus 1/3 | 失敗 | 134 | 27 | 0 | 予算切れ 84・未印の失敗 22 |
| menus 2/3 | 失敗 | 134 | 30 | 1 | 予算切れ 75・未印の失敗 28 |
| menus 3/3 | 失敗 | 135 | 25 | 0 | 予算切れ 87・未印の失敗 23 |
| perf 1/2 | 失敗 | 16 | 12 | 1 | 未印の失敗 3 |
| resilience 1/2 | 失敗 | 21 | 4 | 0 | 予算切れ 17 |
| resilience 2/2 | 失敗 | 21 | 13 | 0 | 予算切れ 2・未印の失敗 2(error 1)・印つきの停止 4 |
| settings 1/5 | 失敗 | 30 | 27 | 1 | 未印の失敗 2 |
| settings 2/5 | 失敗 | 30 | 26 | 2 | 未印の失敗 2 |
| settings 3/5 | 失敗 | 30 | 25 | 1 | 未印の失敗 4 |
| settings 4/5 | 失敗 | 30 | 25 | 2 | 未印の失敗 3 |
| settings 5/5 | 失敗 | 30 | 25 | 2 | 未印の失敗 3 |
| topbar 1/3 | 失敗 | 21 | 16 | 3 | 未印の失敗 2 |
| topbar 2/3 | 失敗 | 21 | 10 | 9 | 未印の失敗 1 |
| topbar 3/3 | 失敗 | 21 | 10 | 9 | 未印の失敗 1 |
| visual 1/3 | 失敗 | 52 | 28 | 14 | 未印の失敗 5 |
| visual 2/3 | 失敗 | 52 | 32 | 8 | 未印の失敗 7 |
| visual 3/3 | 失敗 | 51 | 2 | 15 | 未印の失敗 34(すべて error) |
| 網羅の表 | 失敗 | — | — | — | 網羅の判定(12 領域が最小に届かない) |

- 失敗 26 job の内訳: 未印の失敗があるのが 25 job(うち 7 job は、予算切れ(menus 1〜3・audio 2/3・resilience 2/2)か印つきの停止(file 1/3・file 3/3・resilience 2/2)も併せて持つ)。予算切れだけで落ちたのは resilience 1/2 の 1 job。印つきの停止だけで落ちた job は無い。
- 成功 26 job: page 1〜3・windows 1〜9・plugins 1〜11・delivery 1/2・audio 3/3・perf 2/2。
- 網羅の判定(`judge-coverage.jq`、exit 1): 網羅 759 / 1,990 features。最小に届かない領域は 12 個(page 61/80・input 92/120・file 51/100・menus 83/400・windows の網羅 31/75・audio 43/80・settings 128/150・plugins 8/200・topbar 38/60・resilience 17/40・visual 72/120・perf 27/30)。delivery だけが両方の最小を満たす。

## 2. 領域の表(宣言どおりの 1 表示。前回と同じ列)

「全結果 ok の ID」は、その ID の全ての結果(宣言した表示)が未印で ok の数。「網羅」は `coverage.json` の覆った features / 全 features。「通った / 最小」は、覆った本の数(合格した ID)/ 最小。

| 領域 | 本(ファイル) | 実行 ID | 宣言外(実行せず) | 結果 | ok 未印 | ok 印つき | 失敗 印つき(受理) | 失敗 印つき(拒否) | 失敗 未印(うち error) | skip | 全結果 ok の ID | 網羅 | 通った / 最小 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| page | 84 | 61 | 23 | 61 | 61 | 0 | 0 | 0 | 0 (0) | 0 | 61 | 62/88 | 61/80 |
| input | 121 | 118 | 3 | 118 | 92 | 0 | 10 | 0 | 16 (0) | 0 | 92 | 65/81 | 92/120 |
| file | 115 | 115 | 0 | 115 | 49 | 2 | 10 | 2 | 52 (0) | 0 | 49 | 49/120 | 51/100 |
| menus | 417 | 403 | 14 | 403 | 82 | 1 | 1 | 0 | 73 (0) | 246 | 82 | 33/210 | 83/400 |
| windows | 251 | 251 | 0 | 253 | 250 | 1 | 2 | 0 | 0 (0) | 0 | 249 | 31/75 | 250/250 |
| audio | 84 | 84 | 0 | 84 | 43 | 0 | 3 | 0 | 35 (0) | 3 | 43 | 26/43 | 43/80 |
| settings | 150 | 150 | 0 | 150 | 128 | 0 | 8 | 0 | 14 (0) | 0 | 128 | 76/120 | 128/150 |
| plugins | 302 | 302 | 0 | 302 | 8 | 0 | 294 | 0 | 0 (0) | 0 | 8 | 7/736 | 8/200 |
| topbar | 63 | 63 | 0 | 63 | 36 | 2 | 21 | 0 | 4 (0) | 0 | 36 | 52/108 | 38/60 |
| resilience | 42 | 42 | 0 | 42 | 17 | 0 | 0 | 4 | 2 (1) | 19 | 17 | 21/35 | 17/40 |
| visual | 155 | 155 | 0 | 155 | 62 | 10 | 37 | 0 | 46 (34) | 0 | 62 | 22/54 | 72/120 |
| perf | 31 | 31 | 0 | 31 | 27 | 0 | 1 | 0 | 3 (0) | 0 | 27 | 27/32 | 27/30 |
| delivery | 47 | 47 | 0 | 47 | 46 | 0 | 0 | 0 | 1 (0) | 0 | 46 | 288/288 | 46/40 |
| **合計** | **1,862** | **1,822** | **40** | **1,824** | **901** | **16** | **387** | **6** | **246 (35)** | **268** | **900** | | |

- 「宣言外」は、宣言が `dpr:<数>` か `4k-random` だけの本(40 本)。`--displays` では走らない(run.mjs の `rowsFor`)。page の crisp・zoom(dpr:1 など)が 23 本、menus の乱数の DPR が 14 本。
- 「error」の列の 35 件は、未印の失敗のうち error を持つ本(visual 34・resilience 1)。印つきで error を持つ本は 15 件あり、すべて visual 3/3 の本で、受理された(§3-2)。

## 3. 印の無い失敗(246 件)を、原因で分類

件数・領域・代表 ID・見立て・受け持つ Issue を、群ごとに書く。実測は「実測」、推定は「推定」と書く。

### 3-1. audio(35 件)→ 新しい Issue #1526

- 書き出しが保存されない: `audio/export-wav-length`・`audio/export-wav-nonsilent`(2 件。`no file was saved within 90000 ms`)。
- 書き出しの後の再生の判定が落ちる(22 件。`transport false`・`Space to the first non-silent quantum none ms`・`pause-resume`・`play-3s`・`resume-after-stop` など)。書き出しの前の `audio/device-playing-on-play`(audio 2/3)は transport が動いて合格(**実測**)。書き出しを含まない audio 3/3 は、同じ判定がほぼ全部合格(**実測**)。
- 画の判定 4 件(`loop-button-toggle`・`metronome-button-toggle`・`pattern-song-toggle`・`tempo-value-changes`)。
- 取り込みが 0 件 5 件(`repeat-4096`・`repeat-8192`・`repeat-16384`・`repeat-32768`・`finite-samples`)。
- 音量の判定 2 件(`master-fader-down-200`・`master-fader-up-60`。`peak undefined`)。
- 見立て(推定): `exportWav`(`tools/e2e/lib/audio.mjs`)が、保存されない時に `TWAVRenderForm` を閉じずに返す。窓が残り、以後の入力が届かない。製品の書き出しの不具合か、Start の位置の誤りかは、確認できない。

### 3-2. 描画プロセスの停止(visual 3/3 の 49 件)→ 新しい Issue #1528

- `visual/black-dpr-random-2`(DPR 2.06 へ変えた後の読み。約 65 秒)で、描画プロセスが落ちた(`page.evaluate: Target crashed`、**実測**)。
- その後の 49 本は、すべて `Target crashed` で 0〜1 ms。ページが死んだまま走った(**実測**)。内訳は未印 34 本・印 `new` 15 本。
- 受理の穴(**コードで確認**): `scenarioPasses` は印つきの本の `error` を見ない。印 `new` の 15 本は error のまま受理され、row の判定は通る。
- 見立て(推定): 描画プロセスの停止は、DPR の変更の後の読みで起きた。製品か実行環境の資源かは、確認できない(画像も engine のログも取っていない)。

### 3-3. menus(73 件)→ 新しい Issue #1527

- 項目を選んでも効果が出ない: 38 件(`windows: none; submenus open: 0; changed: none; title: same`。例: `menus/add-3-choose`・`menus/add-5-4-choose`・`menus/file-7-again`)。続きの `no window opened: no dialog to answer` が 23 件。
- 箱が閉じない: 8 件(`TSpeechForm` の OK・Cancel・Esc(`menus/add-1-*`)、`TInAppShopForm` の OK(`menus/file-4-ok`・`file-5-ok`・`file-6-ok`)、`TMissingPluginsForm` の Esc(`menus/file-1-esc`・`file-2-5-esc`))。
- ドロップダウンが開かない: 2 件(`menus/file-8-submenu-escape`・`menus/file-9-10-submenu-escape`)。同じ仕組みの file の 5 件(`file/export-submenu`・`file/menu-new-from-template-submenu`・`file/exit-not-chosen`・`file/export-all-playlist-tracks`・`file/import-submenu`)も含めて、#1527 で扱う。
- その他 1 件(`menus/file-2-2-again`)。停止 1 件(`menus/edit-3-choose`)は §4 の系統 C。
- 道具の状態(**実測**): 項目は `Down x N` で指定し、名前は位置(`index`)で付けている。項目の名前は選ぶ前に確かめていない。見立て(推定): 位置のずれ(区切り線・無効な項目)が一部。残りは製品の未対応か、確認できない。
- 予算切れ(skip 246 件、**実測**): menus の 1 shard は 134〜135 本。35 分の行の予算で、1 shard は 50 本ほどで打ち切られる(menus 1/3 は 34.5 分で 50 本目まで)。`.github/workflows/e2e-full.yml` の shard の数(3)が足りない。#1527 で扱う。

### 3-4. file(52 件)→ 新しい Issue #1530

- テンプレートを開く 37 件(`file/template-*`。notes は `title "null"`・`state engine-idle`・`the file's name in the engine log false`)。見立て(推定): `Problems loading the project` の箱が残り、主の窓の題名が読めず、`boxLeft` で偽になる。結果に `boxLeft` が残っていないので、確認できない。
- ファイルの箱を待つ本(7 本): Ctrl+O を使う `file/new-after-demo`(`no open box ... 40 s after Ctrl+O`)・`file/open-box-shows`・`file/open-upload-multiple`・`file/save-as-dialog`・`file/save-as-template`(demo を `openDataFile` で開く)と、File メニューの箱を待つ `file/import-midi`・`file/open-menu-item`。#1519(File → Open を OPFS の窓にした。閉じている)は、この run の commit の後に入った(**コードで確認**)。箱の前提は古い可能性がある(推定。今の製品での結果は、確かめていない)。
- 閉じた印 `file/z-persist-recent-list`(印 `#1409`・閉じている)は、再読込の後に engine が止まった(状態 `error`、§4 の系統 E。受理の規則で拒否)。#1502(閉じている。Recent を再読込をまたいで保つ)の後、印の前提は古い可能性がある。
- 停止 2 件(`file/demo-product-demos-olbaid-sytrus-essentials`・`file/demo-product-demos-synth-dfr-overcome-sytrus`。系統 B)は、#1529 で扱う。
- drop-downs 0(5 件)と、項目を選んでも新しい窓が出ない 3 件(`file/export-master`・`file/export-video`・`file/export-project-data-files`。`menus/item/file-9-7`・`file-9-11`・`file-9-12` を指す)は、#1527 の「項目の効果」と同じ仕組みとして扱う。

### 3-5. input(16 件)→ 未起票

- クリックが記録されない(desk 上の左・中・右・ダブルクリック、ダブルクリックの trace。計 5 件。`records down false, up false`)。
- タッチの長押し・タップ(2 件)。
- ホイール(`input/mouse-wheel-down-piano-roll`。`records wheel -120 processed 3`。ピアノロールの画は変わらず)。
- キーの切り替え 3 件(F5・F6・F9。1 回目は画が変わり、2 回目で元に戻らない)、ヒントバー 3 件(`top bar 3840x82: no pixel of the bar changed`)、フォーカス 1 件、カーソル(I ビーム)1 件。
- 見立て(推定): desk の座標(1920,1080)の操作が記録されない(道具の測定点か、窓の下か)。確認できない。残りは道具の判定(画素の比較)の可能性。Issue にできるだけの切り分けが、この run では無い(**未起票**)。

### 3-6. settings(14 件)→ 新しい Issue #1531(前回からの退行)

- テーマ 11 件(`settings/theme-ash` など)・表示 2 件(`check-display-force-refreshes`・`check-display-transparent-menus`)・波形 1 件(`choice-open-colorful-waveforms`)。
- **前回の run(8b3bdeb9)では、これらの本は dpr1 で合格していた**(前回の所見の表。settings の未印の失敗は全部 dpr2)。`tools/e2e/scenarios/settings/` と `lib/settings.mjs` は、2 つの commit の間で変わっていない(**実測**)。変わったのは `lib/common.mjs` の `resetProject`(New を Enter で行う・Problems の箱を先に閉じる)と `closeNew`、`lib/flow.mjs`(4741abb8 と #1510)。見立て(推定): 本の始まりの状態が変わった。製品の変更(`app/` の 8 件)は、テーマや表示に関わる物が見当たらない。

### 3-7. visual(12 件)→ 未起票

- `visual/theme-*-golden`(6 件)・`visual/theme-*-edge`・`visual/picture-start`・`visual/window-tedgebrowser-golden` など。画素の比較(ゴールデン)と縁の判定。前回の run には無かった(155 本は 17d4f781 以後の新しい領域)。notes の一部は `display 4k-dpr1 only (the golden keys carry the ratio...)`。画像を取っていないので、原因は確認できない。

### 3-8. その他

- perf 3 件(`perf/cpu-after-audio-busy`(比 1.545)・`perf/cpu-idle-busy-60s`(1.66)・`perf/startup-engine-idle`(1.529)。上限は 1.5。**#1509 が受け持つ**。負荷(loadavg 1.2〜2.9)の影響が大きいと見る(推定)。
- topbar 4 件(`topbar/wheel-tquickedittoolbar-2`・`wheel-twpcontrol`・`click-twpcontrol`・`click-tquickedittoolbarpanel-8`。`no effect`)。**#1509 が受け持つ**。
- resilience 2 件(`resilience/demo-open-replace-50`(error。Ctrl+O の箱が 40 秒出ない。#1519 の後の箱の変化の可能性。推定)・`resilience/tab-hidden-repeat`(0 回))。**#1509 が受け持つ**。
- delivery 1 件(`delivery/lazy-module-served`: `status null, bytes null (manifest null)`)。**未起票**(1 件)。

## 4. エンジンが止まった本(9 件)と、系統

実測の停止の報告(`engineStop` の `message`・`lastFle`)。外部の取得の host は、本文から外した。

| 系統 | 本 | 印 | 終了 | 停止の報告(実測) | 見立て | Issue |
|---|---|---|---|---|---|---|
| A 外部の取得の失敗 | `resilience/random-ops-05`・`-07`・`-09` | `new`(拒否) | 134 | `act1-net #5 InternetOpenUrlW scheme=https ... object=<パス>.png ... -> NULL, last error 12007 (decision 0132)`(`returned from entry`) | 推定: 取得の失敗(12007)を呼び出し元が扱えず止まる。接続の有無は確認できない | #1529 |
| B Sytrus の demo | `file/demo-product-demos-olbaid-sytrus-essentials`・`file/demo-product-demos-synth-dfr-overcome-sytrus` | なし | 134 | `fle: more than 32768 live critical sections`(SIGABRT) | 推定: demo を読む時の重要区間の数の上限。前回の run の Vocodex・Mehran の demo(`#1420` は別の原因)と同じ種類 | #1529 |
| C Edit > #3 | `menus/edit-3-choose`(なし)・`resilience/random-ops-01`(`new`) | 混在 | 134 | `fle: DefWindowProcW: message 0x304: the default processing is not answered (decision 0071 ...)` | 推定: ドロップダウン(Edit > #3)の窓の作成の途中で、メッセージ 0x304 の応答が無い | #1529 |
| D 書き出しの形式 | `file/export-last-format` | `new`(拒否) | 5 | `fle: pthread_mutex_lock: the mutex is held by this guest thread ... libc's mutexes are not recursive` | 推定: 同じ mutex を同じスレッドが二度取り、解放が飛んだ | #1529 |
| E 閉じた印の本 | `file/z-persist-recent-list` | `#1409`(閉じている・拒否) | error | 報告の本文は無い(state `error`) | 再読込の後の停止。#1502 の後の前提の確認が要る | #1530 |

- 系統 A の 3 本は、印 `new`(停止の宣言が無い)なので、受理の規則で拒否された(**実測**)。#1509(resilience の印の見直し)も、この 3 本の印に触れる。
- 系統 B は、印が無い(未印)ので、停止の報告で落ちた。
- 印つきで止まっても、停止の宣言(`expectStop`)が無ければ拒否される規則は、正しく働いた(**実測**)。
- 前回の run の停止(言語の切替の 7 本: `settings/language-*`)は、この run では出ていない(言語の本の印 `#1395` は開いている。`visual` の 9 本が ok)。

## 5. 印(`expectFail`・`expectStop`)の状態

- 閉じた Issue の番号を使う結果は 22 件。
  - `#1315`(1)・`#1336`(3)・`#1337`(2)・`#1338`(2)・`#1400`(1)・`#1484`(8)・`#1499`(1): すべて plugins。ok は 0。
  - `#1409`(1・file・拒否)、`#1411`(1・menus・**ok**)、`#1415`(1・menus)、`#1416`(1・menus)。
- `#1499`(`plugins/plugin-manager-opens` など。閉じている)は、#1508 の本文の閉じた番号の一覧(#1315・#1319・#1336・#1337・#1338・#1400・#1484)に入っていない。**未起票**(#1508 の範囲に足すか、別に起票する)。
- `#1411`(`menus/add-14-choose`)は、ok になった(印が古い)。印を外す。
- 印 `new` の結果は 112 件(ok 6)。`#1395`(言語)は 14 件(ok 9。すべて visual)。`#1461`・`#1484`(plugins)など、開いている plugins の印は、挿入の段で止まる(§6)。

## 6. 起票(この run から作った Issue)と、未起票

作った Issue(`pm-work/created-issues.tsv` に追記済み)。マイルストーン・親・Size・Priority は、作成ツールの表示どおり。

| 下書き | Issue | 領域 | 群(件数) | Priority | Size | Milestone |
|---|---|---|---|---|---|---|
| 240 | #1526 | audio | 書き出しの失敗の連鎖と、取り込みの 0 件(35) | High | M | M1 |
| 241 | #1527 | menus | 予算の skip(246)・項目の効果(73)・閉じた印 #1411・#1415・#1416 | High | L | M2 |
| 242 | #1528 | visual | 描画の停止の連鎖(49)・error を受理する穴(15) | High | M | M2 |
| 243 | #1529 | 製品 | エンジンの停止 4 系統(8) | **Urgent** | L | M4 |
| 244 | #1530 | file | テンプレートの判定(37)・Ctrl+O の箱の前提(7)・閉じた印 #1409(1) | Medium | M | M2 |
| 245 | #1531 | settings | 前回 dpr1 で合格した本の退行(14) | High | S | M2 |

- 作成ツールの仕様: `--apply` は、`pm-hold` を付けて作り、最後に外す。外した時点でワーカーの起動が始まる(`agent.yml` の設計どおり)。#1529 には、外した後の時点で既に `running` が付いていた(確認できた)。
- Priority: 製品の停止は、作成ツールの選択肢(Urgent・High・Medium・Low)にある **Urgent** を使った(AGENTS.md の「Critical か High」の表記に当たる物がツールに無い)。
- 上限 6 件のため、次の群は**未起票**(所見に残す):
  - input 16 件(§3-5)。切り分けが先。
  - visual の golden・edge 12 件(§3-7)。
  - delivery 1 件(`delivery/lazy-module-served`)。
  - `#1499` の閉じた印(plugins 1 件。#1508 の一覧に無い)。
  - `file/z-persist-recent-list`(`#1409`)の停止は #1530 に入れた。

## 7. 既存の Issue が受け持つ群(起票しなかった)

- #1507(windows・settings の DPR 2 の位置): この run では dpr2 の本が無いので、印の無い失敗の受け持ちは無い。DPR 2 の本を増やす作業として残る。
- #1508(plugins): 挿入の段の作業。この run では、挿入の段を越えた結果は 0(§8)。閉じた印の一覧は、#1499 を含まない(§5)。
- #1509(topbar・page・resilience・perf): perf 3・topbar 4・resilience 2(error 1・未印 1)・resilience の予算切れ 19 を受け持つ。random-ops の印の見直しは、§4 の系統 A の 3 本と重なる。

## 8. plugins の段(実測)

- 302 本(1 表示)のうち ok は 8 本。
- 挿入の段で止まった: 146 本(`the Plugin database has no entry for …`)。窓を探す段まで行かない: 119 本(`no window named`)。picker が開かない: 26 本(`the picker did not open`)。その他 3 本(Plugin Manager の窓・プリセットの読み込み)。
- 挿入の段を越えて、窓・パラメータ・音声・止まらないの段に進んだ結果: **0 件**(前回と同じ)。
- 印つきの受理 294 件(挿入の段の失敗が、印で受理されている)。

## 9. 前回(24aeee99 の前の run。commit 8b3bdeb9)との差

| 項目 | 前回 | 今回 |
|---|---|---|
| 行の作り方 | 宣言を無視し、`--displays 4k-dpr1,4k-dpr2` の 2 表示で全本 | 宣言どおり(#1506)。dpr2 は宣言のある 2 本だけ |
| 本の数 | 985 | 1,862(新しい領域: menus 417・input 121・file 115・visual 155・audio 84) |
| 結果 | 1,970(985 × 2) | 1,824 |
| 未印の合格 | 361 本(両表示で合格。36.6 %) | 900 本(宣言した表示で全部合格。48.3 %) |
| 未印の失敗 | 261(うち error 25) | 246(うち error 35) |
| 印つきの失敗 | 651 | 393(受理 387・拒否 6) |
| 印つきの合格 | 13 | 16 |
| skip | 69(resilience の dpr2 が大半) | 268(menus の 246 が新しく入った) |
| 宣言外 | 0 | 40 本 |
| 停止 | 言語の切替 7 本(印 `new`) | 9 件(§4。言語は出ていない) |
| 網羅 | 611 / 1,730 | 759 / 1,990 |
| 網羅の判定 | 不合格 | 不合格(12 領域) |

- **直ったこと**: 宣言外の表示で走っていた本(dpr2 の失敗 約 240 件)が、宣言の表示でしか走らないことで消えた(#1506 の効果。**直ったのではなく、走らなくなった**)。言語の切替の停止 7 件は、この run では出なかった(原因は確認できない。言語の本は印 #1395 のまま)。
- **新しく落ちた群**: audio 35・menus 73 と skip 246・file 52・input 16・visual 46(うち 34 は描画の停止)・settings 14(退行)・停止 9 件。
- **残る群**: plugins の挿入の段(§8)・topbar 4・perf 3・resilience(予算 19・未印 1)・delivery 1・input 16・visual 12。

## 10. e2e-full のワークフローと、道具の課題

- **予算(実測)**: menus の 3 shard は、1 shard に 134〜135 本。35 分の行の予算で打ち切られる(246 本が skip)。shard の数を増やす(#1527)。resilience の `wait-idle-30min` は #1509 で扱う。
- **受理の穴(コードで確認)**: `scenarioPasses` は、印つきの本の `error` を見ない。15 件の visual の本が、error のまま受理された(#1528)。
- **網羅の判定と、テストの判定が同じ `failure` に混ざる**: 前回の所見の指摘のまま(report job)。
- **resetProject の変更(推定)**: 4741abb8 の New を Enter で行う・Problems の箱を Enter で閉じる、の変更が、settings の退行(#1531)と、audio の連鎖(#1526)の候補。
- **dpr1 だけの run(実測)**: 宣言が dpr2 の本が無いため、DPR 2 の確認は、この run では 2 本だけ。DPR 2 の確認の範囲が狭まったことは、報告の書き方に注意する。
- **画像とエンジンのログが無い(実測)**: `--shots`・`--engine-logs` を付けていない。描画の停止・書き出し・黒い区画・クリックの判定は、推定にとどまる。

## 11. 確認できなかったこと

- 書き出しが保存されない原因(製品か Start の位置か)。描画プロセスの停止の原因(製品か実行環境か)。外部の取得の失敗が、実行環境の接続によるか。
- テンプレートの Problems の箱の有無(結果に記録が無い)。Ctrl+O の箱の現在の動き(#1519 の後の製品での結果は、この run に無い)。
- input の desk 上のクリックが記録されない原因(道具の測定点か、窓か、製品か)。
- visual の golden の 12 件の原因(画像が無い)。
- 前回の run で settings の dpr1 が合格していた原因が、道具か製品かの切り分け(#1531 の作業で決める)。

## 付録: 集計の方法

- 成果物: `gh run download 46009281599` で shard の `e2e.jsonl`(52 本)と `e2e-full-coverage`、`gh run view --job <id> --log` で失敗 job のログ(27 本)を取った。
- node のスクリプト(`.tmp-full2/`。読み取りのみ・commit しない): `analyze.mjs`(結果の分類)、`static-set.mjs`(本の宣言を、テキストから読む。import は app/ の TypeScript の連鎖が解けないため使わない)、`analysis2.mjs`(shard ごとの計画の行数と、実際の行数の一致)、`table.mjs`(領域の表)、`detail.mjs`(失敗の一覧)。
- 分類の順序: 成功 → 予算切れ(skipped)→ 未印 → 印つき(受理)→ 印つき(拒否)。`scenarioPasses` と同じ規則。
- 計画の行数: `planScenarios` と `rowsFor`(commit 24aeee99 の `lib/plan.mjs`)を、静的な本の一覧に当てた。52 shard すべてが、実際の行数と一致した。
