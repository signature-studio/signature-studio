# e2e-full の 3 回目の結果(e2e-full の最新の run。checkout は eea3c21)の読み取り

- 対象: `e2e-full` の最新の run(`gh run view` の headSha は b73d5f8f・結論 failure)。job は 58(shard 57 + 網羅の表 1)。成果物は `.tmp-full3/`(shard の `e2e.jsonl`・`coverage.json`・job の全ログ。commit しない)。
- **重要な訂正: 58 job すべてが checkout したのは eea3c21 である**(ログの `HEAD is now at eea3c21`・58 件とも)。`gh run view` の headSha(b73d5f8f)は、試験した版ではない。この所見の判定と集計は、eea3c21 の `tools/e2e/`(`git archive` で `.tmp-full3/src/` に展開)で行った。eea3c21 と b73d5f8f の間で、E2E の本は 91 ファイル、`lift/runtime/` と `app/` も変わっている(後述 §0-2)。
- 製品・`tools/`・`.github/`・`spec/` は変えていない。`gh` は、成果物とログの取得、open の Issue の読み、新しい Issue の作成に使った。
- 実測と推定を分ける。推定には「推定」、確認できない物には「確認できない」と書く。
- 判定の規則: `tools/e2e/lib/plan.mjs` の `scenarioPasses`・`rowPasses`(eea3c21 の版)。shard の計画は `planScenarios`・`rowsFor` を、eea3c21 の本の宣言(テキストから読んだ 1,869 本)に当てて再現した。

## 0. 要点

1. **job**: shard 57 のうち **28 が成功、29 が失敗**。網羅の表(report)は失敗(網羅の判定。12 領域以上が最小に届かない)。job の合否は、各行の `rowPasses` の再計算と GitHub の結論が **57 件すべてで一致**した。
2. **試験した版は eea3c21**(上の訂正)。eea3c21 の後の commit で、この所見の対象の外のもの: `#1509` の修正(2ce02866・22bbdf73。topbar の 4k-dpr2 の宣言、shard の page 共有の読み直し)、`#1537` の修正(5a0c95a1。VirtualQuery の画像の表 16 → 193、unwinder)。この run では、どちらも入っていない(**実測**)。よって、topbar・perf・resilience の未印の失敗と、VirtualQuery の停止は、この run の結果で HEAD の状態を判定できない(§4)。
3. **計画と実際の行は 57 shard すべてで一致**(行の名前・本の数・本の並び。`planOk`)。shard の数は前回の 52 から 57 に増えた(menus 3 → 8。`e2e-full.yml`)。
4. **結果は 2,227 件**(1,829 ID。宣言外 40 本)。未印で ok 1,537・印つき ok 42・印つきの失敗(受理)399・印つきの失敗(拒否)2・未印の失敗 228(うち error 35)・予算切れ(skip)19。前回(1,824 件)とは、shard の数・表示の行の数が違うので、件数は直接比べない(§9)。
5. **前回からの主な変化**: 予算切れが 268 → **19**(menus の 8 shard で、予算の中に全本が入った。#1527 の予算の部分は直った)。audio の未印の失敗が 35 → **4**(#1526 の書き出しの連鎖は直った)。visual の描画の停止(error 34)は 1 に減った(#1528)。一方、menus の未印の失敗は 73 → **125**(項目の効果が無い群は 61 → 94。#1527 の項目の部分は直っていない)。file は 52 → **62**。
6. **起動(page started → project open)**: shard の最初の値の中央値 **40.7 s**(57 件。範囲 27.2〜47.2 s)。全 76 組(表示ごと)の中央値 41.3 s。e2e の出力に `elapsedMs` は無いので、job のログの時刻差から求めた(推定。確認できない: 前回は未計測)。起動の待ちは、読み込み・Welcome・project 開き・待ちを含む(P1 の 10 秒とは別の測り方)。
7. **エンジンの停止の記録(`engineStops > 0` の本は 22 本)**: 6 型(A〜F。14 本)と、起動の失敗の連鎖(error の本 8 本)。最多は VirtualQuery(6 本)で、eea3c21 の表の上限 16 によるもの(§4)。error の状態の本は全部で 33 本(§3-1)。
8. **plugins**: 302 本のうち ok は 7 本。挿入の段を越えて窓・パラメータ・音声の段に届いた結果は **0**(前回と同じ)。原因の大半は、**道具の欠陥**: 一覧の読みが本の開始位置から始まり、共有の page では一覧が page の開始時に 1 回しか書かれないため、282 本が一覧の段で止まる(§8)。
9. **新しい Issue**: 3 件(#1549・#1550・#1551。下書き 263・264・265)。閉じた Issue を指す印の残りは §5。

## 1. shard の結果と、落ちた規則

- 判定の規則は job ごとに、行の `rowPasses`(servedHeadersMatch・error が null・全本が `scenarioPasses`)。
- 29 の失敗 job の内訳(実測。`analysis.json` の `jobs`):

| 失敗の型 | job 数 | job |
|---|---:|---|
| 未印の失敗のみ | 19 | file 1/3・2/3・page 2/3・menus 1/8・2/8・3/8・4/8・7/8・8/8・topbar 1/3・2/3・3/3・plugins 7/11・input 3/3・audio 1/3・2/3・windows 5/9・7/9・perf 2/2 |
| 未印の失敗 + error | 6 | menus 5/8・6/8・settings 1/5・2/5・4/5・visual 3/3 |
| 未印の失敗 + 印つき(拒否)+ error | 2 | file 3/3・settings 5/5 |
| 未印の失敗 + 予算切れ + error | 1 | resilience 2/2 |
| 予算切れのみ | 1 | resilience 1/2 |

- 印つき(拒否)は 2 本だけ(`file/demo-songs-mehran-abbasi-with-you`: 停止・印 `new` の宣言が無い。`settings/language-zh`: 印 `new` の本で、失敗したリクエストが 1)。
- 成功 28 job: page 1・3、input 1・2、windows 1〜4・6・8・9、audio 3、settings 3、plugins 1〜6・8〜11、visual 1・2、perf 1、delivery 1・2。

## 2. 領域の表(eea3c21 の宣言。1 本の表示 + dpr2 の行)

「実行 ID」は結果に現れた本の数。「宣言外」は、宣言が `dpr:<n>` か `4k-random` だけの本(`--displays 4k-dpr1,4k-dpr2` では走らない)。「全結果 ok の ID」は、その ID の全結果が未印で ok の数。「網羅」は `coverage.json`(覆った features / 全 features)。「通った / 最小」は coverage の `scenarios` / 最小。

| 領域 | 本 | 実行 ID | 宣言外 | 結果 | ok 未印 | ok 印つき | 失敗 印つき(受理) | 失敗 印つき(拒否) | 失敗 未印(error) | skip | 全結果 ok の ID | 網羅 | 通った / 最小 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| page | 85 | 62 | 23 | 63 | 62 | 0 | 0 | 0 | 1 (0) | 0 | 61 | 61/88 | 61/80 |
| input | 121 | 118 | 3 | 118 | 97 | 9 | 10 | 0 | 2 (0) | 0 | 97 | 67/81 | 106/120 |
| file | 118 | 118 | 0 | 120 | 42 | 0 | 15 | 1 | 62 (1) | 0 | 40 | 43/123 | 43/100 |
| menus | 420 | 406 | 14 | 408 | 282 | 1 | 0 | 0 | 125 (15) | 0 | 280 | 132/211 | 281/400 |
| windows | 251 | 251 | 0 | 502 | 494 | 1 | 3 | 0 | 4 (0) | 0 | 247 | 30/75 | 248/250 |
| audio | 84 | 84 | 0 | 84 | 76 | 1 | 3 | 0 | 4 (0) | 0 | 76 | 35/43 | 77/80 |
| settings | 150 | 150 | 0 | 292 | 257 | 9 | 8 | 1 | 17 (17) | 0 | 126 | 80/120 | 136/150 |
| plugins | 302 | 302 | 0 | 302 | 7 | 0 | 294 | 0 | 1 (0) | 0 | 7 | 7/736 | 7/200 |
| topbar | 63 | 63 | 0 | 63 | 37 | 1 | 22 | 0 | 3 (0) | 0 | 37 | 53/108 | 38/60 |
| resilience | 42 | 42 | 0 | 42 | 16 | 4 | 0 | 0 | 3 (1) | 19 | 16 | 20/35 | 20/40 |
| visual | 155 | 155 | 0 | 155 | 93 | 16 | 43 | 0 | 3 (1) | 0 | 93 | 29/54 | 109/120 |
| perf | 31 | 31 | 0 | 31 | 27 | 0 | 1 | 0 | 3 (0) | 0 | 27 | 28/32 | 27/30 |
| delivery | 47 | 47 | 0 | 47 | 47 | 0 | 0 | 0 | 0 (0) | 0 | 47 | 288/288 | 47/40 |
| **合計** | **1,869** | **1,829** | **40** | **2,227** | **1,537** | **42** | **399** | **2** | **228 (35)** | **19** | **1,154** | | |

- 網羅の判定(`judge-coverage.jq`)は、最小に届かない領域が 12(delivery 以外。windows は 248/250 で届かない)。
- windows の結果が 502 なのは、宣言の dpr1・dpr2 の両方で走る本が 251 本あるため(表示の行が 2 つ)。
- resilience の skip 19 は、`resilience/wait-idle-30min` 系(予算の 35 分の中で、30 分の本が始められない)。#1509 が受け持つ(この run には入っていない)。

## 3. 印の無い失敗(228 件)を、原因で分類

分類は `.tmp-full3/groups.mjs` の規則(error → 箱 → 停止 → 項目 → file → その他の順)。件数は実測。原因の見立ては、実測と推定を分けて書く。

| 群 | 件数 | 領域 | 見立て | 受け持つ Issue | この所見の扱い |
|---|---:|---|---|---|---|
| G1 エンジンの error の連鎖(`no desktop compositor`) | 33 | menus 15・settings 17・file 1 | **製品**(起動の OPFS の失敗が先)。§3-1 | 無し | **新規 #1549**(下書き 263) |
| G2 箱が Esc・OK・Cancel で閉じない | 17 | menus 12・windows 4・input 1 | **製品(推定)**。同じ窓の型が、どの答えでも閉じない。§3-2 | 無し(#1527 は「別の Issue にする」と書いて閉じた) | **新規 #1550**(下書き 264) |
| G3 一覧の段で止まる(印つきの受理の中) | 282 | plugins | **道具**(一覧を本の開始位置から読む)。§8 | 無し(#1535 は閉じた。その後も残る) | **新規 #1551**(下書き 265) |
| G4 項目を選んでも効果が無い | 94 | menus(file 18・patterns 17・view 15・options 13・tools 11・help 9・key 7・add 4) | 不明。外部の効果(ブラウザ)・未実装・キーの指定の誤りのいずれか(**確認できない**)。§3-4 | #1527(閉じた。項目の部分は直っていない) | 未起票(切り分けが先) |
| G5 file: テンプレート・デモ・Open・メニューの箱 | 56 | file | 混在(§3-5) | #1530(閉じた) | 未起票(切り分けが先) |
| G6 エンジンが止まる(error でない) | 8 | file 5(デモ 4・open-box-shows 1)・resilience 1・menus 2 | file 5 は VirtualQuery の表の上限 16(eea3c21)。menus 2 と resilience 1 は §4 の型 B・E・F | #1537(閉じた。HEAD では 193)・#1420・#1546・#1428・#1529 | HEAD で再確認(§4) |
| G7 topbar・perf・resilience | 7 | topbar 3・perf 3・resilience 1 | #1509 の版の前の結果(§3-6) | #1509(閉じた。この run の後の commit) | 確認できない(この run は #1509 の前) |
| G8 audio | 4 | audio | 書き出しの画面が開かない 2・メトロノーム停止後も音が出る 2(推定。確認できない) | #1526(閉じた)・無し | 未起票(4 本。5 本未満) |
| G9 visual(golden の差) | 2 | visual | テーマの画素の差(golden の鍵は dpr1)。画像は無い(確認できない) | 無し | 未起票(2 本) |
| G10 その他(error ではない) | 5 | page 1・plugins 1・input 1・menus 2 | 個別。§3-7 | — | 未起票 |
| G11 エラー(error) の残り | 2 | resilience 1(Ctrl+O の箱が出ない)・visual 1(`Target crashed`) | 個別 | #1519・#1528(閉じた) | 未起票 |

合計: 33 + 17 + 94 + 56 + 8 + 7 + 4 + 2 + 5 + 2 = **228 件**(G6 の 8 件には、`menus/tools-16-choose`・`menus/view-11-1-choose` の 2 件を含む。分類の順で箱の規則に先に当たったため、停止の 2 件は、この表では G6 に数える)。

### 3-1. G1: engine の error の連鎖(33 件)→ 新規 #1549(下書き 263)

- 最初の原因(実測。`engineStopReason`。9 本に記録がある): `InvalidStateError: Failed to execute 'getSize' on 'FileSystemSyncAccessHandle': getSize failed`。本は `menus/view-11-16-choose`・`menus/view-11-17-choose`・`resilience/reload-20`(20 回の再読込の中で 1 回 error)・`settings/theme-ash`(dpr1・dpr2)・`settings/theme-dedzii`(dpr1・dpr2)・`settings/theme-dedzii-light`(dpr2)・`settings/theme-dark-orange`(dpr2)。
- 以後の連鎖(実測): 同じ page の後続の本が、全て `this engine build has no desktop compositor`(`dumpWaiter` の応答 `desk-dump-error`)で落ちた(33 件: menus 15・settings 17・file 1)。eea3c21 の `run.mjs` は、非 file の領域では、engine が error・exited でも page を読み直さない(#1509 で直した。この run の後)。
- 製品の箇所(実測。コード): `app/src/engine/regstore.ts` の `openRegState` は、OPFS の `storage.getDirectory`・`createSyncAccessHandle` の失敗は捕まえて「保存しない」として続けるが、その後の `h.getSize()`(71 行)は捕まえていない。`getSize` の失敗は起動の `init` の例外になり、worker が `error` を出す。`app/src/engine/` は eea3c21 から HEAD まで変わっていない(実測)。
- 原因の見立て(推定): 読み直しの直後に、前の page の worker が同じ `hkcu.log` を持っている。`acquire`(50 回・100 ms)の後に取れた handle が、`getSize` で無効になる。確認できない: 無効になる正確な条件。
- 別の見方(推定): 起動の失敗を、engine の error として扱い、page の読み直しをしない限り戻らない。この run では、読み直しが無い領域で 33 本が連鎖した。

### 3-2. G2: 箱が閉じない(17 件)→ 新規 #1550(下書き 264)

| 箱(窓のクラス) | 件数 | 本 | 答え |
|---|---:|---|---|
| TSpeechForm[Speech properties] | 3 | `menus/add-1-esc`・`add-1-ok`・`add-1-cancel` | Esc・OK・Cancel |
| TPlugListForm[PlugList] | 4 | `menus/add-6-esc`・`add-6-ok`・`add-6-cancel`・`view-6-cancel` | Esc・OK・Cancel |
| TPlugListForm(F8) | 5 | `windows/plugin-esc`(dpr1・dpr2)・`windows/plugin-reopen`(dpr1・dpr2)・`input/key-escape-closes-plugin-list` | F8 のあとの Esc |
| TWAVRenderForm[Render options] | 3 | `menus/file-9-8-esc`・`file-9-8-ok`・`file-9-8-cancel` | Esc・OK・Cancel |
| TTapTempoForm[Tempo tapper] | 1 | `menus/view-8-ok` | OK |
| TPythonForm[Script output] | 1 | `menus/view-10-ok`(同時に engine の停止) | OK |

- 実測: 同じ窓は、どの答えでも閉じない(TPlugListForm は 3 種の答え、TSpeechForm も 3 種、TWAVRenderForm も 3 種)。Esc が他の窓では効く本が多い(実測。`menus/add-14-choose` などは OK で閉じる)。
- 見立て(推定): 窓ごとの閉じる処理(製品)か、窓が閉じる前の確認の箱(製品)。道具の座標の誤りは、同じ窓で 3 種の答えが全て落ちるので考えにくい。確認できない: 製品の閉じる処理の正確な所在。
- #1527(閉じた)は、この群を「別の Issue にする」と書いて閉じている(実測。Issue 本文)。

### 3-3. G3: 一覧の段(282 件)→ 新規 #1551(下書き 265。§8)

### 3-4. G4: 項目を選んでも効果が無い(94 件)→ 未起票

- 内訳(実測): file 18・patterns 17・view 15・options 13・tools 11・help 9・key 7・add 4。同じ領域の他の項目は、多くが効果を出している(例: view は 72 本中 38 本が合格、patterns は 75 本中 57 本が合格)。
- 観測(実測): `windows: none; submenus open: 0; changed: none; title: same`(効果が無い)が多い。`help` は 13 本中 9 本が効果が無い。
- 見立て(推定): (a) 外部の効果(ブラウザ・ヘルプの URL)を、窓の表で見ると効果が無いと数える。(b) 項目が未実装。(c) キーの `Down x N` の指定がずれる(区切り線・無効な項目)。確認できない: どれが多いか(項目の名前と効果の対応を取っていない)。
- #1527 は閉じたが、この群は 61 本(前回)から 94 本(今回)に増えた。前回との比の分母は、項目の数(前回 403 本・今回 406 本)で大きく変わらない。

### 3-5. G5: file(56 件)→ 未起票(切り分けが先)

- テンプレートを開く本(`file/template-*`): 原因の型が混ざっている。
  - 実測 4 本: `no project file <host>/lift/out-app/host/resources/Data/Templates/<名前>.flp`(`openProjectFile` が存在を確かめて投げる)。e2e-full の shard は、`lift/out-app/host/resources/Data/` に **デモの曲だけ**を置く(`e2e-full.yml` の「demo の曲を host/resources に置く」)。`agent.yml` は host-extra の資源全体(テンプレートを含む)を host に置く。よって、テンプレートの資源が shard に無い可能性が高い(推定。確認できない: shard の中のファイル一覧)。
  - 実測 13 本: 開く途中で `pageerror: A requested file or directory could not be found`(OPFS ��� NotFound)。原因は確認できない。
  - その他: `title "null"`・`engine-idle`・`the file's name in the engine log false`(約 35 本)。原因は確認できない。
- デモ(DPR 1 のみ): 11 本が「タイトルが変わらない」「別の曲の名前」。うち 5 本は engine の停止(§4 の VirtualQuery)。残りは確認できない。
- Ctrl+O の箱(`open-folder`・`open-upload-multiple`・`save-as-dialog` など)と、File の項目の箱(`export-master` など)は、G4 と同じ型の項目の効果の問題(推定)。
- **道具の欠陥の候補として、host の資源(テンプレート)の欠けは 4 本で実測**。5 本未満で、残りの原因が確認できないため、起票しない(§6 で扱う)。

### 3-6. G7: topbar・perf・resilience(7 件)→ この run では確認できない

- topbar 3(`click-twpcontrol`・`click-tquickedittoolbarpanel-8`・`wheel-tquickedittoolbar-2`: no effect)・perf 3(CPU の比 1.6〜1.9。上限 1.5)・resilience 1(`tab-hidden-repeat`: 0 回)。
- 受け持ちは #1509(閉じた)。その修正(2ce02866・22bbdf73)は eea3c21 の後の commit で、この run には無い(実測)。**確認できない**: 修正の効果。

### 3-7. その他(G10・G11)

- `page/visibility-presents-resume`(presents 72 → 76。File メニューが描かれない)・`plugins/plugin-database-browser-opens`・`input/mouse-right-click-mixer-hold`(ミキサーの右クリックの押し続け)・`menus/key-ctrl-n`(Ctrl+N の後の `TInAppShopForm`)・`menus/patterns-1-again`(名前の窓)。個別。
- `resilience/demo-open-replace-50`(error。Ctrl+O の箱が 40 秒出ない。#1519 の後の箱の変化の可能性。推定)。`visual/black-dpr-random-2`(error。`Target crashed`。#1528 の撤収の後も 1 本残る)。

## 4. エンジンが止まった本(停止の記録は 22 本)と、型

実測(`engineStop` の `stop.lastFle`・`engineStopReason`)。停止は `engineStops > 0` か、状態が error・exited の本。

| 型 | 本数 | 本 | 状態 | 印 | 見立て | 受け持ち |
|---|---:|---|---|---|---|---|
| A `fle: VirtualQuery: more images than the runtime holds` | 6 | file/demo-songs-edlan-ella-no-l-song-for-you・-lollievox-lezina-j-i-ll-be-your-madonna・-asher-postman-future-house・-dedzii-night-terror・-mehran-abbasi-with-you(印 `new`・拒否)・file/open-box-shows | exited | 無し(5)・`new`(1) | **eea3c21 の `lift/runtime/fle_vmap.c` の `VQ_MAX_IMAGES` が 16**(実測)。停止は 17 本目の image の登録で起きる。#1537(5a0c95a1。eea3c21 の後)で 193 に広げ、満杯は名前で止める。**HEAD では直っている(推定。この run で確かめていない)** | #1537(閉じた) |
| B `fle: indirect call to 0 is not a known function entry` | 2 | menus/view-11-1-choose(無し)・settings/language-fr(`new`) | exited | — | 間接呼び出しの先が 0 | 推定: #1420・#1546(開いている) |
| C `fle: indirect call to 0x4e0022323c003444 ...` | 1 | menus/view-10-ok(無し) | exited | — | 間接呼び出しの先が文字・画素の形(推定) | 推定: #1546(開いている) |
| D `fle: CompareStringW: lcid=0x400 ...`(ドイツ語・ベトナム語・日本語の言語) | 3 | settings/language-de・-vi・-ja(印 `new`・受理) | exited | `new` | 言語の切替で停止する既知の型。印が `new` のまま | #1395(開いている)。印を `#1395` にする |
| E `fle: PathCleanupSpec: pszSpec holds an illegal character` | 1 | menus/tools-16-choose(無し) | exited | 無し | パスの予約文字 | #1428(開いている)。印 `#1428` を付ける |
| F `act1-net #5 InternetOpenUrlW ... last error 12007` | 1 | resilience/reload-20(無し) | engine-idle | 無し | 外部の取得の失敗が呼び出し元で扱えない | #1529(閉じた)。印が無い |
| (error) state error(起動の失敗。停止の記録があるのは 8 本) | 8(全体 33) | §3-1 | error | — | getSize(G1) | #1549(下書き 263) |

- 停止の記録の全数: 22 本(型 A〜F の 14 本と、error の 8 本)。前回の 9 件(外部の取得の失敗 3・Sytrus の demo 2・Edit > #3 2・書き出しの形式 1・閉じた #1409 の本 1)のうち、この run に出たのは外部の取得の失敗 1 本(`resilience/reload-20`。印が無い)だけ。Sytrus の demo・Edit > #3・書き出しの形式は、この run では出ていない(実測)。言語の 3 本(D)は前回には無く、この run で出た。
- 型 A(6 本)は、HEAD で直っている可能性が高い(推定)。次の run で確かめる。

## 5. 印(`expectFail`・`expectStop`)の状態

- 閉じた Issue を指す印(eea3c21 の本。`grep` の集計と `gh issue list --state all` の照合。実測):

| 印 | 本数(結果) | 結果 | 扱い |
|---|---:|---|---|
| `#1440`・`#1441`・`#1445`・`#1447`(plugins) | 24 | 全て失敗(受理)。大半は一覧の段で止まる(G3) | 印の見直し。G3 の修正の後に、印を外すか開いた Issue に付け替える |
| `#1535`(plugins の生成器・効果) | 24 | 全て失敗(受理)。大半は一覧の段で止まる(G3) | 同上。#1535 は閉じたが、一覧の読みが残る(§8) |
| `#1527`(file/import-midi) | 1 | 失敗(受理) | 印の見直し(項目の効果の群 G4 と同じ型) |
| `#1529`(resilience/random-ops-01・-07・-09) | 3 | **合格**(ok)。警告の対象 | **印を外す** |

- 開いている Issue の印: `#1395`(言語。16 本。うち言語の 3 本は停止 D)、`#1428`(パス。停止 E は印が無い)、`#1434`〜`#1477`(内蔵プラグイン群。plugins の G3 の下にある)、`#1546`・`#1420`(停止 B・C の推定)。
- 印 `new` の受理の結果: 105 件(前回は 112 件)。`new` の付け替え先は、各本の内容で決める。

## 6. 起票(この所見の作業で作った Issue)と、未起票

- 作った下書き(`pm-work/issue-drafts/wave1bx/`):

| 下書き | 群 | 種類 | Priority | Size | 作成 |
|---|---|---|---|---|---|
| 263 → #1549 | G1: 起動の OPFS の getSize の失敗が、engine の error を連鎖させる(33 件) | 製品 | High | M | 作成済み(open。M4。親 #1377) |
| 264 → #1550 | G2: 箱が Esc・OK・Cancel で閉じない(17 件) | 製品(推定) | High | M | 作成済み(open。M2。親 #1377) |
| 265 → #1551 | G3: 一覧の段で止まる(282 件。道具) | 道具 | High | S | 作成済み(open。M5。親 #1377) |

- 未起票(切り分けが先。件数と理由):
  - G4 項目の効果が無い 94 件(外部の効果・未実装・キーの指定のいずれか。確認できない)。**最大の群**。#1527 の項目の部分は直っていない(前回 61 → 94)。
  - G5 file 56 件(テンプレート・デモ・Open の箱。原因が 4 つ以上に分かれる。host の資源の欠け 4 本は実測)。
  - G8 audio 4 件・G9 visual 2 件・G10 5 件・G11 2 件(5 本未満。原因は個別)。
  - 停止 B・C(間接呼び出し 3 本。#1420・#1546 の推定)・D(言語 3 本。#1395 の印の付け替え)・E(パス。#1428 の印の付け替え)・F(取得の失敗 1 本。#1529 は閉じた)。

## 7. 既存の Issue が受け持つ群(起票しなかった)

- #1395(言語): 停止 D(3 本)の印の受け持ち。
- #1428(パス): 停止 E(1 本)。印が無いので、印 `#1428` を付けるのが先。
- #1420・#1546(間接呼び出し): 停止 B・C(推定)。
- #1508・#1535(plugins): 挿入の段。G3 が先に止めているので、挿入の段の結果は出ていない(§8)。
- #1509(topbar・perf・resilience): この run に入っていない(§3-6)。

## 8. plugins の段(実測)

- 302 本のうち ok は 7 本。
- 原因の大半(282 本): 一覧の段で止まる。`no listing of Installed in the engine log since the scenario started (the listing is written once per page)`(実測)。`insertFromDatabase`(`tools/e2e/lib/plugins.mjs`)は、一覧の行を `databaseLines(guest, from)` で、本の開始位置 `from` から読む。共有の page(shard の 1 page を本が続けて使う)では、一覧は page の開始時の 1 回しか書かれないので、本の開始後には無い。
- 残り: 「行が見つからない」(`the row of … is not located`)11 本・その他 9 本。挿入の段を越えた結果(insert・window・param・audio・stop のいずれかが値を持つ)は **0 本**(実測)。
- 同じ読み方は HEAD でも変わっていない(`git log eea3c21..HEAD -- tools/e2e/lib/plugins.mjs` が空。実測)。#1535 は「一覧の列挙で判定する」に変えたが、本の開始位置からの読みは残っている(実測)。
- 見立て(推定): 共有の page では、一覧を本の開始時(page の開始時)以降の行から探せばよい。確認できない: 一覧が page の開始時に 1 回だけ書かれることの確認(この run のログ以外)。

## 9. 前回(e2e-full-2。run 46009281599)との差

- 訂正(実測): 前回の run の shard も、headSha(24aeee99)ではなく **4ebdb56** を checkout していた(job のログの `HEAD is now at 4ebdb56`。windows 1/9・menus 1/3 で確認)。`tools/e2e/` は 4ebdb56 と 24aeee99 で同一なので、前回の判定の道具は正しかった。ただし、前回の製品の版の記録(24aeee99)は誤りで、試験された製品は 4ebdb56。

| 項目 | 前回 | 今回(eea3c21) | 差の読み |
|---|---|---|---|
| shard の数 | 52 | 57(menus 3 → 8) | 増えた |
| 結果の件数 | 1,824 | 2,227 | 表示の行(windows・settings・file の dpr2)と本の数の増加 |
| 未印で ok(ID) | 900 / 1,862 | 1,154 / 1,869 | 全結果 ok の ID |
| 未印の失敗 | 246(error 35) | 228(error 35) | 件数は減ったが、群は入れ替わった(§3) |
| 印つきの受理(失敗) | 387 | 399 | plugins の G3 が増えた |
| 印つきの拒否 | 6 | 2 | 停止 A の 1 本(印 `new`)と、取得の失敗 1 本 |
| 予算切れ | 268 | 19 | **直った**(#1527 の予算の部分) |
| audio の未印の失敗 | 35 | 4 | **直った**(#1526。書き出しの連鎖) |
| visual の error | 34 | 1 | **直った**(#1528。描画の停止の連鎖) |
| menus の未印の失敗 | 73 | 125(項目の効果 61 → 94) | **直っていない**(#1527 の項目の部分) |
| file の未印の失敗 | 52 | 62 | 増えた(テンプレート・デモ) |
| settings の未印の失敗 | 14(dpr1 の退行) | 17 error(getSize の連鎖) | 前回の退行とは原因が違う(推定。前回の退行の本は、この run では error の連鎖に入った) |
| plugins の挿入の段を越えた結果 | 0 | 0 | 変わらない。G3 が先に止める |
| エンジンの停止の記録 | 9 | 22(型 A〜F の 14 本と、error の 8 本) | 増えた(VirtualQuery 6 は eea3c21 の表の上限) |
| 網羅の判定 | 不合格 | 不合格 | 変わらない |

- **直ったこと(実測)**: 予算切れ・audio の書き出しの連鎖・visual の描画の停止の連鎖。
- **直っていないこと(実測)**: 項目の効果(menus 94)・plugins の一覧の段(282)・file のテンプレート・デモ(約 56)。
- **新しく出たこと(実測)**: engine の error の連鎖(33)。前回には無い。

## 10. e2e-full のワークフローと、道具の課題

- **試験した commit と、表示される headSha の違い(実測)**: `gh run view` の headSha は b73d5f8f だが、58 job すべてが eea3c21 を checkout した。前回の run も、headSha(24aeee99)と違う 4ebdb56 を checkout していた(§9)。e2e-full の checkout の ref は `workflow_run.head_sha` で、この表示とは違う(推定。確認できない: 再実行の経緯)。今後の所見は、job のログの `HEAD is now at` で確かめてから、版を決める。
- **予算**: menus の 8 shard は、35 分の中に全本が入った(skip は resilience のみ)。
- **受理の穴(前回の指摘は直っている)**: eea3c21 の `scenarioPasses` は、印つきの本の `error` も失敗にする(`errored(s)` を先に見る)。この run の印つき 441 本(受理 399 + ok 42)のうち、error を持つ本は 0 件(実測)。
- **印つきの受理が 399 件**: 印の中身(一覧の段・停止)が見えなくなる。G3 のように、道具の欠陥が印の後ろに隠れる。
- **shard のページ共有**: 共有の page の読み直しは、eea3c21 では file 領域だけ(実測)。その他の領域では、error の後も page が続き、連鎖する(G1)。#1509 で直した(HEAD)。

## 11. 確認できなかったこと

- 起動の getSize の失敗の正確な条件(G1)。読み直しとの関係は推定。
- 箱が閉じない原因(製品の閉じる処理か、窓の確認の箱か)(G2)。
- 項目の効果が無い 94 件の内訳(G4)。外部の効果・未実装・キーの指定の別。
- テンプレートとデモの 56 件の原因の内訳(G5)。
- topbar・perf・resilience の 7 件(#1509 の修正の前の結果)(G7)。
- HEAD での VirtualQuery の停止(型 A)が直っているか。この run では、HEAD の版を試していない。
- 起動の時間(elapsedMs)の正式な値(e2e の出力に無い。ログの時刻差から求めた。前回は未計測)。

## 付録: 集計の方法

※ 作業用の `.tmp-full3/`(成果物・ログ・分析の表・スクリプト)は、この所見の後に削除した。以下は、その手順の記録。

- 成果物: `gh run download 46128658286 -D .tmp-full3`(58 の artifact)。job のログ: `gh run view --job <id> --log`(58 件。`.tmp-full3/logs/`)。`gh run view --log`(run 全体)は API の上限で取れない。
- 試験した版: `git archive eea3c21 tools/e2e | tar -x -C .tmp-full3/src`。判定の関数は `.tmp-full3/src/tools/e2e/lib/plan.mjs`(eea3c21)。
- 静的な本の一覧: `.tmp-full3/static.mjs`(eea3c21 の `scenarios/*.mjs` を、テキストで読む。playwright が無いため import しない。1,869 本。結果に現れた ID は全て一覧に在る)。
- 計画の再現: `.tmp-full3/analyze.mjs`(`planScenarios`・`rowsFor` を、静的な一覧に当てた。57 shard すべてで、行の名前・本の数・並びが一致)。
- 起動: `.tmp-full3/startup.mjs`(job のログの `the page started` から `project open` までの時刻差)。
- 群の分類: `.tmp-full3/groups.mjs`。分析の結果は `.tmp-full3/analysis.json`(commit しない)。
