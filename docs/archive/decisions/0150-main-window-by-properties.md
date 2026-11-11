# 0150: 主窓を窓の lid の数値でなく窓の性質で決める — クラス名 `TFruityLoopsMainForm`(窓リスト)→ 無ければ最大の可視・オーナー無し・非 layered 窓 → 無ければ名前付きエラー。仮の決定

状態: **仮の決定**。依頼は PM(決定 0150)。調査・設計・実装・テスト・文書は作業者(`app4`)。作業は HEAD **2b3a2f2**(ブランチ main)、`git archive 2b3a2f2` の木に対する差分 `app4.diff`(`diff -ruN clean src`、`patch -p1` の形)。**`lift/runtime/`・`lift/crates/` は 0 行、`lift/harness` の C は 0 行、面の表の ABI(version 1/2)は変えていない**。

区分の凡例は決定 0121〜0148 と同じ。**実測**＝このセッションのコマンドの出力を見た事実(2026-11-03 JST)。**読み**＝コード・文書を読んだ解釈で、走らせていない。**出典なしの選択**＝ページにも実測にも根拠が無く作業者が選んだ値(`S-main-…`、§5 に撤回条件つき)。**未確認**＝確かめていない。

## 0. 先に書くこと(進捗の誠実性、AGENTS.md / `docs/policies/progress-integrity.md` に従う)

**(1) 実エンジンが実行された事実(実測)**: 決定 0147(vclock1)の作業途中の objs(`lift/out-vclock1/wb2/w-Oz/obj`、**読み取りだけ**、mtime 00:05)を `link-web.sh`(`-sEXPORT_ES6=1`)で自分の dir に再リンクした `flengine6.wasm`(320,731,834 B、SHA-256 `1c0eb51a8b08…`。vclock1 の `dist2/web/flengine6.wasm` と同じ大きさ。PM の連絡では、PM はこの bundle へのハードリンクを配信している)と、vclock1 の `dist2/web/data` の DLL 6 個・seed(224 ファイル)で bundle を作り、**隔離した headless Chrome 156 で自分のアプリ(HEAD 2b3a2f2 + この差分)に実エンジンを実行させた(`lift/out-app4/real/main-check.mjs`、6 回(1 回目は下の欠陥で失敗)と、HEAD のアプリで 2 回、e2e で 3 回)**。これは vclock1 の**途中成果物**で、最終 bundle ではない。観測したこと:

- エンジンは **最初の idle まで 37.1〜37.7 s**(`main_start` 3.05 s から約 34 s)で idle に達した。窓リスト(`fle_av_poll`)で **主フォーム `TFruityLoopsMainForm` = lid 14**(1930x1092、オーナー無し、exstyle 0x50000)、ロゴ `TLightweightLayeredControl` = **lid 8**(320x448、オーナー = `TApplication`、exstyle 0x80800ac)、ウィザード `TWelcomeWizard` = lid 48(874x570)、`TMIDIForm` = lid 22。アプリの `__app.mainWindow()` = `{lid:14, rule:'class', className:'TFruityLoopsMainForm', candidates:1}`。
- **主フォームの lid が 13 でなく 14、ロゴが 18 でなく 8 だった**(決定 0147 の PM の説明どおり。実エンジンで確認した)。
- **発見(このタスクの範囲外だが重要)**: エンジンの first-pixel イベント(`wf1-surface: event=first-non-background-pixel`)は**主窓でなくロゴ(lid 8、334x474)で出る**。主窓の first pixel はエンジンから一度も報告されない。従って **HEAD 2b3a2f2 のアプリ(`ev.lid === mainLid`)に、この bundle の manifest へ `mainLid=14` を手で足したものを与えると、`first_pixel` の印は付かない**(**実測**: `lift/out-app4/clean/app`(HEAD 2b3a2f2 のそのままのアプリ)で `real/old-app-check.mjs` を実行。state は `engine-idle`、36.7 s で idle に達したが、`timeline` に `first_pixel` が無く、`checks` は 0、ページの『First pixel』の段は未完了のまま。窓リストは `TWelcomeWizard` lid 48 と `TFruityLoopsMainForm` lid 14)。私の最初の実行も、自分のコードが同じ理由で `first_pixel` を付けず、待ちのスクリプトが 15 分で時間切れになった。そこで、主窓の first pixel が報告されなかったときは**エンジンの最初の idle で主窓に印を付ける**ことにした(§3)。実測: 印は 37.1 s(idle の時点)で `{lid:14, 1930x1092, source:'first-idle'}`。
- End session(エンジンが idle のときに `stop()`)の後: stop イベント(kind 2)と最終チェック(kind 255)は**主窓 lid 14**で、**エンジン自身の SHA-256 = メモリから読んだ SHA-256(`8d1a0148fec4…`)で一致**(method B なので canvas の読み戻しは無し)。終了後も窓リストは読めた(§5 の `S-main-remembered` の想定は、この 1 回では起きなかった)。
- **実エンジンの e2e(`engine-desktop`、preview)**: 3 回実行(44/50、45/50、最終版のスクリプトで **44/47**)。最終版の失敗 3 件は**どれも logo の検査**(Enter でウィザードは消えるが、302 s 待っても logo が可視にならなかった。原因は未調査。同じ bundle で Enter だけの流れでは 7.0 s で可視になる、§8)。**窓をクラスで引く部分は全部通った**(主窓・ウィザード・ロゴ・Settings の検索、`__app.mainWindow()` = lid 14 の class、Z 順・位置・スクリーンショットとの照合、ウィザード面の SHA-256 の 3 値)。
- 実エンジンのページに STAND-IN の帯は**出ない**(`hidden`・空・`display:none`、`engineKind` = `emscripten-fruity-v1`。上の e2e の 1 件)。

**(2) 代役・合成データ・再生の範囲**: (a) `vitest` の単体テスト(この変更で +102 件)は**合成データ**: `WebAssembly.Memory` に本物の `SurfaceTable` と `fle_av_poll` の構造を手で書いたもので、FL の窓の大きさ・クラス名・オーナーは決定 0145 の実エンジンのログから写した。(b) エンジン無しの e2e(dev・preview・Worker)は**代役のエンジン(`surface-table-v1`、PNG の画素を窓の面として出すだけ)**で、表示経路と manifest の扱いのテスト。実エンジンではない。いずれも『進捗』ではなく、この変更の回帰の確認である。(c) 『d8aacf4 の最終構成(主窓 lid 13)の bundle で同じアプリが動く』は、**その bundle が無いので実エンジンでは確かめていない**。確かめたのは、その構成の窓リストを写した合成データでの規則の答え(§6)。

**肯定的に見せる点は限られる。**

1. 実エンジンでの確認は **vclock1 の途中成果物 1 つ、Chrome 1 台**だけ。実機の FL が同じ窓を『主窓』とするかは**未確認**。クラス名 `TFruityLoopsMainForm` は Delphi のフォームのクラスで、エンジンの窓リストで最大・オーナー無し・題 `FL Studio`/`FL Studio 2026` だった窓の名前、というだけ(読み)。
2. **面の表の `owner_lid` と `form_key` の語は、実エンジンでは常に 0**(`lift/harness/fruity_surface.c` はこの 2 語に代入していない。読み。CONTRACT.md の『`fle_win.c` が反映する弱い hook』は実装されていない)。依頼文の『面の表は owner_lid/form_key を既に持つ』は欄としてはその通りだが値は入っていない。クラス名・題は面の表に無い。窓リスト(`fle_av_poll`、決定 0141)が、クラス名・題・オーナー・style・exstyle・可視を持つ唯一の既存のチャネルだった。**C は変えずに**これを使った(§2)。
3. `lift/harness/web/{perf1,perf2,final0129}` の研究用 worker は決定 0128〜0138 の wasm(窓リストの export が無い)を相手に書かれている。**新しい窓リストがあればクラス名で、無ければ 13 を返す**ようにした。13 は『その古い bundle の主窓の lid』という決定 0126 の実測で、新しい bundle には使われない。**これらの道具は走らせていない**(必要な古い wasm が無い)。構文検査と、コピーした関数本体の単体テスト(8 件)だけ。
4. 実エンジンの e2e の期待 SHA(決定 0136 のウィザード面など)は、この途中成果物でも**一致した**(下)。が、最終 bundle で一致するかは別。

## 1. 先に結論

| 項目 | 結果 |
|---|---|
| 主窓の規則 | 規則 1: 窓リストのクラス名が `TFruityLoopsMainForm` で画素(幅・高さ > 0)のある窓。複数なら 可視 → 面積が大きい → Z 順で上 → 面の表で新しい(後ろの欄)。規則 2: 1 が無ければ、可視・オーナー無し・非 layered で画素のある窓のうち最大面積(同点は 1 と同じ)。規則 3: 無ければ無し(`NoMainWindowError`、ログに理由)。**lid の数値は規則にも実装にも出てこない**(§3、`no-window-numbers.test.ts` が原文を読んで拒否する) |
| 実エンジン(**実測**、途中成果物) | 主窓 = lid 14 をクラスで選んだ。first pixel は主窓でなくロゴで出る→最初の idle で印を付ける(§0) |
| 後方互換(合成データ、**実測**) | 同じ窓の集合に lid を 13 / 14 / 7777 / 1 と付けた 4 通り × 面の表 v1/v2 × 窓リストあり/なし = 16 通りで、イベントの『主窓か』が同じ(§6)。再作成・不可視・複数・主窓無しの各場面も |
| manifest | `mainLid` は任意(書いてあれば無視)。`expect.surfaces[]` は `lid`(旧・その bundle の番号)か `window:'main'` のどちらか一方。新しい `pack-assets.mjs` は `mainLid` を書かず、`--expect main:WxH:SHA` を書く |
| テスト(**実測**) | vitest **476 passed(32 ファイル)**(基準 374)、Worker 結合 **29 passed**(基準 29)、型検査 4 つの tsconfig 通る、e2e(エンジン無し)dev・preview・Worker 各 **174** = **522/522**(同じ機械・同じ HEAD の clean 木は各 156 = 468。依頼の基準 271〜273 は実エンジンの 82 を含む値と読める) |
| 変異検査(**実測**) | **39/39 検出(単体層 36/36、e2e 層 3/3)**(途中で 2 件が生き残り、強化した。§7) |
| 追加依頼(STAND-IN の帯) | `engine.kind = surface-table-v1` のときだけ、ページ最上部に常時の帯。`window.__app.engineKind`。エンジン無しの e2e で存在、実エンジンの e2e で不在(§9)。変異 4 件 |
| 時間 | 開始 00:58 JST、本文書の時点 02:33 JST(1 時間 35 分) |

## 2. 背景と、主窓を決める手段の選び方

**依頼の根拠**: アプリと測定ツールが窓の論理 id `lid`(主窓 13、ロゴ窓 18)を決め打ちしていた。lid は `fle_win.c` の `next_lid++`(`CreateWindowExW` ごと)で付く窓の作成順の番号で、決定 0147(vclock1)で主窓が 14、ロゴ窓が 8 になる(PM の調査。**§0 の実エンジンの実行で、途中成果物では 14 と 8 であることを確認した**)。窓が作り直されても変わる(detach-window.md R5)。

**lid を使っていた場所**(`grep` で洗い出した。アプリ以外も含む):

| 場所 | 使い方 | この決定での扱い |
|---|---|---|
| `app/src/engine/worker.ts` | `ev.lid === fruityOptions.mainLid` で first pixel・stop/idle のチェック、最後の読み戻しに `mainLid` | 窓の性質で決める `makeMainRouting`(§3) |
| `app/src/bundle/manifest.ts` | `mainLid` が必須の欄 | 任意・無視 |
| `app/src/main.ts` `verifyExpected` | `expect` の `lid` と canvas の lid を突き合わせ | `window:'main'` を足した。`lid` は旧 manifest 用に残す(その bundle の数値という**データ**) |
| `app/scripts/pack-assets.mjs` | `mainLid` 既定 13、スタンドインの `--expect 13:...` | 既定をやめた。スタンドインの scene の lid(13/115/18...)は代役が自分で決めた窓の番号なので**そのまま** |
| `app/scripts/build-engine-bundle.sh` | `--expect "13:1930x1092:..."` | `--expect "main:..."` |
| `app/test/e2e/{engine-desktop,layered-engine,explore-engine}.mjs` | 13・48・18・22 | クラス名(`TFruityLoopsMainForm`・`TWelcomeWizard`・`TLightweightLayeredControl`・`TMIDIForm`)で引く |
| `lift/harness/web/{perf1,perf2,final0129}` | `lid === 13` | §4 |
| `lift/harness/web/present/*` | `lid === 13`(代役のエンジンの窓) | 変えない(実エンジンの主窓ではない) |
| `lift/harness/a4-fruity.sh` | 注釈の『lid 13』 | 注釈 3 行を足した |
| `lift/harness/web/perf2/perf2_probe.c`(C) | `lid == 13` | 依頼の『C は変えない』に従い**変えていない**(決定 0138 の古い測定用) |
| `app/src/ui/desktop.ts` | 窓リストに無い面の Z を `lid` 順にする(`z: s.lid`) | **変えていない**。窓を選ぶ規則ではなく、窓リストが無いときの重なり順の既定。代役の layered scene の『lid が大きいほど手前』に依っている |

**主窓の手段の候補と選択**:

| 候補 | 内容 | 採否 |
|---|---|---|
| A. 面の表の `form_key`/`owner_lid` | 欄はあるが実エンジンは書かない(§0 の 2) | 不可(C を変える必要が出る) |
| B. 面の表を C 側で拡張(クラス名の語を足す) | version 3 | 不採用。決定 0145 の互換規則(新アプリは v1/v2 を読む、古いアプリは新しい版を読めない)を守る必要があり、アプリと bundle を一緒に作り直さないと動かない。依頼の『C は最小に』にも反する |
| C. エンジンのログ行 `g3-wintable:` | `fruity_g3.c` が出すのは窓表のテキストの**行数と SHA-256** と、可視の top-level の `g3-win:` 行(FLE_G3 の診断用)。ブラウザの bundle では通常出ない | 不可 |
| D. 停止時のログの窓表(`summarizeEngineLog`) | クラス名はあるが**停止の後**だけ。first pixel(起動中)には間に合わない | 不可(停止後の表示には使われている) |
| **E. `fle_av_poll`(窓リスト)** | クラス名・題・オーナー・style・exstyle・可視・Z 順。決定 0141 以降の bundle が export している。アプリは既に idle のたびに読んでいる | **採用**。C の変更 0 行。first-pixel イベントの最中(エンジンの stack の中)から呼んでも動いた(**実測**、§0) |

## 3. 規則(実装: `app/src/engine/main-window.ts`、`surface-events.ts`)

入力は 2 つ。**面**(面の表の窓: lid・幅・高さ・可視・layered・オーナー・exstyle)と**窓リスト**(`fle_av_poll`: 全 top-level 窓のクラス名・オーナー・可視・exstyle、Z 順で上が先頭。無い bundle では無し)。

- 候補: 画素のある面(幅 > 0 かつ高さ > 0)。**窓リストがあるときは、リストに無い窓の面は候補にしない**(破棄された窓の最後の絵が表に残る。`desktop.ts` の既存の扱いと同じ)。
- 可視・オーナー・exstyle は、窓リストにあればそちらを、無ければ面の表の値を使う(面の表の可視は最後の書き込みの時点の値で古くなりうる。オーナーは常に 0)。
- **規則 1**: クラス名が `TFruityLoopsMainForm` の候補。複数あれば順位: 可視 → 面積が大きい → Z 順で上 → 面の表で後ろの欄(新しい面)。可視でなくても主窓。
- **規則 2**: 規則 1 の候補が無いとき、**可視・オーナー無し・layered でない**候補のうち最大面積(同点の順位は規則 1 と同じ)。layered は面の表の layer 語または exstyle の `WS_EX_LAYERED`(0x80000)。
- **規則 3**: どちらも無ければ無し。`chooseMainWindow` は `null`、`requireMainWindow` は `NoMainWindowError`(メッセージに面の数・窓リストの有無・リストと対応する面の数・クラス名が無いこと)。
- **題(title)は使わない**: エンジンの窓リストで起動直後は `FL Studio`、後は `FL Studio 2026` と変わる(決定 0145 のログ)。**`form_key` も使わない**(常に 0)。
- **lid の数値は規則のどこにも出ない**。同点の最後の決め手も lid の大小ではなく『面の表の欄の順(作成順)』。

**Worker の使い方**(`makeMainRouting`、`worker.ts`): エンジンの面イベント(`fleSurfaceEvent`)のたびに、面の表(`SurfaceTable.entries()`)と窓リスト(`readAppView()`)を読み、規則で主窓を決めて、**そのイベントの窓が主窓か**を答える。再作成を拾うため、**毎回決め直す**(コストは既存の `pumpAll()` と同程度の桁。イベントごとに表を読み JSON 化している)。窓リストの読み出しが例外になってもエンジンの stack に投げ返さない(規則 2 に落とし、ログに 1 回)。
- ブラウザのビルドが受けるイベントは **kind 1(first pixel)と kind 2(stop)だけ**(kind 3 は `fle_g3_snapshot` が出し、node のハーネスだけが呼ぶ。読み)。実エンジンの実行(§0)でも kind 1 と kind 2 だけだった。
- first pixel(kind 1): 主窓のイベントなら `first_pixel` の印とチェック(従来どおり)。**主窓でない窓に first pixel が来たら**、印を付けず警告を 1 回出す(以前は黙って無視していた)。
- **エンジンの最初の idle で**、`first_pixel` がまだなら、主窓(あれば)に `first_pixel` の印を付ける。`detail.source` = `'first-idle'`、チェックは作らない(エンジンはその窓の SHA-256 を出していない)。イベントで付いた印は `'event'`。**実エンジン(途中成果物)では first pixel がロゴで出るので、この経路が働いた**(§0)。
- stop(kind 2): 印は全窓、チェックは主窓だけ(従来どおり)。
- 実行の最後の読み戻し: 今決まる主窓、無ければ**最後に見つけた主窓**。
- 主窓が変わったら(再作成・初めて見つかった・見つからなくなった・また見つかった)ログに 1 行ずつ出す: `main window: lid 29, class TFruityLoopsMainForm (was: lid 13, class TFruityLoopsMainForm)`。

**ページ側**(`main.ts`): `window.__app.mainWindow()` が同じ規則を `client.currentSurfaces`(`SurfaceInfo` に `style`/`exstyle` を足した)と最後の窓リストに当てる。`verifyExpected` の `window:'main'` が使う。

## 4. 測定ツール(`lift/harness/web`)

`perf1`・`perf2`・`final0129` の worker に `mainLidOf(instance, memory)`(同じ本文のコピー 3 つ。classic script で import できない)を足した: `fle_av_poll` があればクラス名 `TFruityLoopsMainForm` の窓の lid、**無ければ 13**(決定 0126 の実測、決定 0128〜0138 の wasm だけに真)、あるが主窓が無ければ null。面イベントに `main` の旗を付け、page → worker の入力は `lid:'main'`(worker が引く)、runner と summarizer は旗で選ぶ(旗が無い古い結果の JSON は lid 13 とみなす)。**走らせていない**。3 つの本文が一致すること・合成の窓リストでの答え・export 無しなら 13・不正な構造なら null を `harness-main-lid.test.ts` が確かめる(8 件)。

## 5. 出典なしの選択(S-main-…、撤回条件つき)

| 名前 | 値 | 理由 | 撤回条件 |
|---|---|---|---|
| `S-main-class` | 主窓のクラス名 = `TFruityLoopsMainForm` | エンジンの窓リストの実測(決定 0126/0141/0145 のログと今回の実エンジンの実行。サイズ 1930x1092、オーナー無し、exstyle 0x50000)で最大かつ唯一のフォーム。Delphi の命名から『主フォーム』と読む。**実機の FL が主窓とするかは未確認** | 別の build で別のクラス名が主窓になったとき(規則 2 が受ける) |
| `S-main-tiebreak` | 複数のとき 可視 → 面積 → Z 順 → 新しい面 | 実エンジンで同じクラスの窓が 2 つ以上同時にあるのを見ていない。順序は作業者の選び | 実際に 2 つ以上見えたとき、実機の主フォームの選び方(Delphi の `Application.MainForm`)と比べる |
| `S-main-hidden-class` | クラスが合えば不可視でも主窓 | 決定 0145 のエンジンログの窓表には、主窓の `visible 0` の行と `visible 1` の行がある(実測。起動中に不可視の時間がある)。可視を要求すると取りこぼしうる | 不可視の主窓のイベントでチェックが誤る例が出たとき |
| `S-main-rule2` | 可視・オーナー無し・非 layered の最大面積 | PM の依頼文の規則。窓リストが無い古い bundle と、別のクラス名の build のため | 実機で主窓が最大でないと分かったとき |
| `S-main-leftover` | 窓リストがあるとき、リストに無い面は候補にしない | `desktop.ts` が『破棄された窓の最後の絵』と扱うのと同じ。再作成の旧い面を主窓にしない | 窓リストに載らない主窓があると分かったとき |
| `S-main-title-unused` | 題は使わない | 起動直後と後で変わる(ログ) | — |
| `S-main-remembered` | 最後の読み戻しは『最後に見つけた主窓』に頼ってよい | エンジン終了後に窓リストが空になりうる(読み)。**実エンジンの 1 回の実行では、終了後も窓リストが読めた**ので、この頼りは働かなかった | 終了後も窓リストが常に残ると分かればこの頼りは要らない |
| `S-main-first-idle` | 主窓の first pixel が報告されないときは、**エンジンの最初の idle で主窓に `first_pixel` の印**(`source:'first-idle'`)を付ける | 実エンジンの実行で first-pixel イベントがロゴで出て、主窓のは報告されなかった(§0)。印を付けないと進捗表示の『First pixel』が idle になっても未完了のまま残る。idle の時点で主窓には絵がある(実測: 窓リストの 1930x1092、canvas の SHA-256 `dbdae00102ef…`) | エンジン側(`fruity_surface.c`)が主窓の first pixel を報告するようになったとき(`'event'` が働き、この経路は要らない) |
| `S-main-first-pixel-wait` | 主窓でない窓の first pixel は印を付けず待つ(警告 1 回) | 従来と同じ(lid が主窓でなければ無視)。ロゴの first pixel で『First pixel』を完了にすると、見える絵が無いのに完了と言うことになる(ロゴは idle の時点で不可視、実測) | first pixel が主窓以外に来続ける build が出たとき(印の付け方を決め直す) |
| `S-main-old-13` | 測定ツールの古い wasm(窓リスト無し)では 13 | 決定 0126 の実測。その wasm だけ | 古い wasm を捨てたとき、この分岐も捨てる |

## 6. テスト(手計算の期待値、件数は §1)

- `main-window.test.ts`(31 件): 規則 1(クラスが大きさに勝つ、13/14/7777 で同じ、小さい番号・大きい番号、複数の同クラスで 可視 → 面積 → Z 順、不可視でも主窓、面が無い・サイズ 0 は候補でない、破棄された窓の面、再作成)、規則 2(窓リスト無し、リストのオーナーと exstyle、`WS_EX_LAYERED`、不可視、リストの可視が面の可視に勝つ、同面積の Z 順/後ろの欄)、無し(空、全部 layered/owned/hidden/空、`requireMainWindow` のメッセージ)、**全 lid を別の数に置き換えた 5 通り(恒等・+12・×100・反転・40 億台)で答えが対応する窓になる**、`MainWindowTracker`(1 回だけ言う、変わったら言う、見つからない/また見つかった/初めて見つかった)。
- `main-window-engine.test.ts`(41 件): `WebAssembly.Memory` に**本物の `SurfaceTable`(v1/v2)と `fle_av_poll` の構造**を書き(`test/helpers/synthetic-engine.ts`)、アプリの読み手で読んで、`makeMainRouting` にイベントを流す。FL の窓(TApplication・主フォーム 1930x1092・ロゴ 334x474 layered・ウィザード 874x570)の数を 13/48/18、14/49/8、7777/3/90000、1/400/300 の 4 通りにして、面の表 v1/v2 × 窓リストあり/なし。再作成(旧い面が残る/窓リストが空になる)、不可視、複数、主窓無し、窓リストの読み出しが例外、ロゴだけ大きい、page 側の `SurfaceInfo` 経由、**first pixel がロゴで出る場合の idle での印(§3)**。
- `expect.test.ts`(9 件)、`manifest.test.ts`(純増 +1)、`pack-assets-fruity.test.ts`(+2)、`harness-main-lid.test.ts`(8 件)、`no-window-numbers.test.ts`(7 件: `src/**/*.ts`・`pack-assets.mjs`・`build-engine-bundle.sh`・実エンジンの e2e 4 本に窓の数を書いた行が無いことを原文で検査)、`standin-banner.test.ts`(3 件)。
- e2e(エンジン無し): stopscreen の manifest の `expect` を `window:'main'` にした 4 場面(合う・面の順を逆にした・合わない)と、layered scene の `mainWindow()`、STAND-IN の帯。

**限界**: 『d8aacf4 の最終構成』を相手にした確認は、決定 0145 の記録から写した窓リストでの合成データ。実際の bundle での確認ではない。

## 7. 変異検査

`test/mutation/run.mjs` に決定 0150 の変異を足した(規則の各条件を 1 つずつ壊す: クラス名、可視を優先しない、小さい面を選ぶ、Z 順の逆、同点で古い面、規則 2 がオーナー付き・layered・不可視を取る、破棄された窓の面を候補にする、サイズ 0 を候補にする、オーナー/可視/layered をリストでなく面の表から取る、規則 1 を飛ばす、追跡が最後の主窓を忘れる、変化を黙る、最後の読み戻しが覚えを使わない、first pixel が全窓で印を付ける、チェックが全窓、`isMain` を `lid === 13` にする、idle での印を消す/二重に付ける/出所を偽る、主窓の大きさを入れ替える、`surfaceInfo` が style を落とす、worker/page の fact が exstyle を無視、`expect` が `lid` を比べる、manifest が `lid` と `window` の併記を受ける、`mainLid` を必須に戻す、`pack-assets` が `mainLid` を書く、build script が `13:` を期待する、窓リストの例外を通す、page の `verifyExpected` が先頭の面を取る、STAND-IN の帯を消す/条件を逆にする/ページが隠す/ページが文字を入れない)。木のコピー(`/tmp/opencode/app4/work/mut/app`)で 1 件ずつ壊し、vitest(e2e 層は `run.mjs` の該当場面)が落ちることを見て戻す。

**結果**: 単体層 36/36、e2e 層 3/3 = **39/39 検出**(`logs/final/mutation-unit.txt`・`mutation-e2e.txt`、各通しの最後に『木のコピーが実木と同一・単体テストが緑』を確認)。

| 変異 | 木 | 結果 |
|---|---|---|
| `main-class-name-changed` | `src/engine/main-window.ts` | 検出 |
| `main-visible-not-preferred` | `src/engine/main-window.ts` | 検出 |
| `main-smallest-area-wins` | `src/engine/main-window.ts` | 検出 |
| `main-z-order-reversed` | `src/engine/main-window.ts` | 検出 |
| `main-oldest-surface-wins-a-tie` | `src/engine/main-window.ts` | 検出 |
| `main-rule2-takes-owned-windows` | `src/engine/main-window.ts` | 検出 |
| `main-rule2-takes-layered-windows` | `src/engine/main-window.ts` | 検出 |
| `main-rule2-takes-hidden-windows` | `src/engine/main-window.ts` | 検出 |
| `main-destroyed-windows-leftover-is-a-candidate` | `src/engine/main-window.ts` | 検出 |
| `main-empty-surface-is-a-candidate` | `src/engine/main-window.ts` | 検出 |
| `main-owner-from-the-table-not-the-list` | `src/engine/main-window.ts` | 検出 |
| `main-visible-from-the-table-not-the-list` | `src/engine/main-window.ts` | 検出 |
| `main-layered-style-of-the-list-ignored` | `src/engine/main-window.ts` | 検出 |
| `main-class-rule-skipped` | `src/engine/main-window.ts` | 検出 |
| `tracker-forgets-the-last-main-window` | `src/engine/main-window.ts` | 検出 |
| `tracker-silent-when-the-main-window-changes` | `src/engine/main-window.ts` | 検出 |
| `final-main-ignores-the-remembered-window` | `src/engine/surface-events.ts` | 検出 |
| `router-first-pixel-for-any-window` | `src/engine/surface-events.ts` | 検出 |
| `router-checks-every-window` | `src/engine/surface-events.ts` | 検出 |
| `router-main-is-lid-13` | `src/engine/surface-events.ts` | 検出 |
| `worker-surface-info-drops-the-style` | `src/engine/surface-info.ts` | 検出 |
| `page-fact-ignores-the-exstyle` | `src/engine/main-window.ts` | 検出 |
| `window-list-error-reaches-the-engine` | `src/engine/surface-events.ts` | 検出 |
| `standin-banner-removed` | `src/ui/standin-banner.ts` | 検出 |
| `standin-banner-condition-inverted` | `src/ui/standin-banner.ts` | 検出 |
| `router-idle-fallback-removed` | `src/engine/surface-events.ts` | 検出 |
| `router-idle-marks-again-after-the-event` | `src/engine/surface-events.ts` | 検出 |
| `router-idle-mark-says-event` | `src/engine/surface-events.ts` | 検出 |
| `routing-main-size-swapped` | `src/engine/surface-events.ts` | 検出 |
| `routing-failure-reaches-the-engine` | `src/engine/surface-events.ts` | 検出 |
| `worker-fact-ignores-the-exstyle` | `src/engine/main-window.ts` | 検出 |
| `expect-main-compares-the-entry-lid` | `src/bundle/expect.ts` | 検出 |
| `manifest-accepts-lid-and-window-together` | `src/bundle/manifest.ts` | 検出 |
| `manifest-requires-mainlid-again` | `src/bundle/manifest.ts` | 検出 |
| `pack-assets-writes-mainlid-13` | `scripts/pack-assets.mjs` | 検出 |
| `build-script-expects-lid-13` | `scripts/build-engine-bundle.sh` | 検出 |
| `standin-banner-hidden-by-the-page` | `src/main.ts` | 検出 |
| `standin-banner-text-not-set-by-the-page` | `src/main.ts` | 検出 |
| `page-verify-expected-takes-the-first-surface` | `src/main.ts` | 検出 |

偽陰性の記録: (1) `page-fact-ignores-the-layered-style` は**等価変異**(`layered` を面の表の語と exstyle の両方から二重に計算していた)。式を整理し、変異を exstyle の受け渡しの 2 件に置き換えて検出。(2) `page-verify-expected-takes-the-first-surface` は**テストの穴**(代役の scene では主窓が先頭の欄だったので先頭を取っても通った)。面の順を逆にした manifest の場面 `expect-main-reordered` を足して検出。

## 8. 実エンジンでの確認の詳細

§0 に要約。道具: `lift/out-app4/real/main-check.mjs`(リポジトリの差分には入れない。`logs/real/` に結果)。bundle の作り方: `link-web.sh`(`OBJ=…/out-vclock1/wb2/w-Oz/obj EM_CACHE=<自分の写し> LINK_EXTRA=-sEXPORT_ES6=1`)で再リンク(3 s)、`pack-assets.mjs --kind emscripten-fruity-v1 --seed-dir <dist2/web/data/seed の写し> --dll-dir <6 DLL>`。環境変数は `build-engine-bundle.sh` のもの(`FLE_CALL_FRUITY` ほか)。**フォントの seed(`vfs-seed-fonts.tsv`)は入れていない**(画の文字に影響しうる。未確認)。

| 実行 | 結果 |
|---|---|
| run1 | first pixel の印が付かず、スクリプトの待ち(900 s)で時間切れ。**自分のコードの欠陥**(主窓でない窓の first pixel を無視)と、待ち条件が first_pixel だけだったスクリプトの欠陥 |
| run2 | 診断つき。idle 37.7 s、主窓 lid 14(class)、first pixel は lid 8 |
| run3 | 修正後。印は 37.1 s に `source:'first-idle'` |
| run4 | idle で End session。stop・最終チェックは lid 14、エンジン SHA = メモリ SHA(`8d1a0148fec4`) |

エンジンの窓リスト(idle 後、可視または大きさのある窓): `TWelcomeWizard` lid 48(874x570、可視、オーナー = 主窓)、`TFruityLoopsMainForm` lid 14(1930x1092、可視、オーナー無し)、`TApplication` lid 5(0x0、可視)、`TLightweightLayeredControl` lid 8(320x448、不可視、オーナー = `TApplication`)、ほか `TPUtilWindow` など多数(不可視)。面の表の窓: lid 8、14、48 の 3 枚。canvas の SHA-256: lid 48 = `1223c73ff0b5…`(**決定 0136 のウィザード面と同じ**)、lid 14 = `dbdae00102ef…`。ネットは非 loopback の TCP 0、UDP 送信 0。

**実エンジンの e2e(`engine-desktop`、preview、`FL_ENGINE_BUNDLE=real/bundle/fl-vc`)**: 3 回実行した。

| 回 | 結果 | 失敗の内訳 |
|---|---|---|
| 1(旧版のスクリプト) | 44/50 | (a)『first pixel の時点で starting 段階』を前提にした 3 件(first pixel が idle の時点で付くので成り立たない)、(b) logo の検査 3 件(Enter の後 302 s 待っても logo が可視にならなかった) |
| 2(starting 段階の検査を『未検査』の記録に直した版) | 45/50 | (a) 『pointerdown/up が 4 件以上』(starting 段階の click を省いたので 2 件。スクリプトの欠陥)、(b) logo の canvas を `lid` で引き直さず、logo が(約 1 分後に)消えた後で `null` を触った(スクリプトの欠陥) |
| 3(最終版。先頭の `record:` に『first pixel は `first-idle`』、`NOT CHECKED` に starting 段階の検査を省いたことが残る) | 44/47 | logo の検査 3 件(上の 1 回目と同じ。Enter でウィザードは消えるが、302 s 待っても logo が可視にならない)。ログ: `logs/real/e2e-engine-desktop-run3-final.txt` |

- **窓をクラスで引く部分は、実エンジンで全部通った**: 主窓・ウィザード・ロゴ・Settings をクラス名で引き、`__app.mainWindow()` が lid 14 を class で選び、DOM の canvas・Z 順・位置・スクリーンショットとの一致(ウィザード面 `1223c73ff0b5…`、主窓の一部 `7256376c6b87…`)も通った。決定 0136 のウィザードの SHA-256(押す前 `1223c73ff0…`、押した `f285760a56…`、離した `7bf902bdd6…`)は**この途中成果物でも一致した**。
- **logo の挙動(実測、`main-check.mjs --enter 1 --watch 60`)**: Enter の約 7.0 s 後に logo(lid 8)が可視になり、`data-layered="ulw"` の canvas が出て、320x448 → 318x446 と縮み、**約 58〜61 s 後に窓リストから消えた**(面・canvas も同時に消える。再作成はされなかった)。HEAD のアプリでも同じ Enter だけの流れで可視になるまで 7.03 s(私のアプリ 7.02 s)で、可視になる点は同じ。e2e の 1 回目で logo が 302 s 見えなかった理由は**未調査**(e2e の操作列が長いことによる挙動の違いと読むが、確かめていない)。logo の検査は、logo が約 1 分で消えるので、操作の間隔に依存する。


## 9. 追加依頼: STAND-IN の帯

PM の依頼(依頼者が既定のページに出ていた代役を実エンジンと取り違えかけた)。`engine.kind` が `surface-table-v1` のとき、ページ最上部に常時の帯『STAND-IN: テスト用の代役です。FL Studio のエンジンは動いていません(画像の再生)』(`index.html` の `#standin-banner`、`src/ui/standin-banner.ts`、`style.css` の `position: sticky`)。`emscripten-fruity-v1` では出ない(hidden・空)。`window.__app.engineKind` に manifest の `engine.kind`。stand-in の bundle は名前が `standin-` で始まり、説明が `STAND-IN, not the real engine:` で始まる(`pack-assets.mjs --standin`)。単体 3 + 1 件、e2e: animate-A・stopscreen-A・layered-A で存在(位置 top < 100 px、高さ 40 px)、実エンジンの e2e で不在、変異 4 件。

**限界**: 帯は DOM のテキストなので、画面の画像を撮った場合は写る(位置が最上部)が、manifest を直接読む道具には出ない。配信側(`vite dev`/`preview`/Worker が `surface-table-v1` の manifest を既定のパスで返さない)は**この変更に含まれない**(PM の別担当)。

## 10. 失敗・やり直し・不備(正直に)

- 存在しない `bash` ツールを 2 回呼んだ(出力は shell に回されたが規則違反)。`sleep` を 1 回書いた(拒否された)。最初に `git log --oneline` を端末に出した(コミットメッセージが長大で出力が大きかった)。
- **実エンジンの最初の実行は失敗した**(上、run1)。原因は自分のコードが『主窓でない窓の first pixel を無視する』ことで、旧コードの `lid === mainLid` も同じ挙動だった(HEAD のアプリで同じ bundle を実行して確認、§0)。待ちのスクリプトも first_pixel だけを待っていたのが不備(以後 idle も待つ)。
- 変異検査で 2 件が生き残った(§7)。
- 最初の `no-window-numbers` の検査が自分の `expect.ts` の引数名 `mainLid` と e2e のローカル変数を誤検出した。引数名を `main` にし、`mainLid` を禁じる範囲を `src/` に絞った。
- 主窓が見つかった最初の通知が『(found again)』と出ていた(まだ一度も見つけていないのに)。『一度見つけた後で、また見つかった』ときだけ付けるよう直した。
- 実エンジンの e2e の最初の通し(スクリプトが旧い版)では、『エンジンが first pixel の時点で starting 段階』を前提にした 3 件が失敗した(first pixel が idle の時点で付くため)。前提が成り立たない場合はそれらを検査せず記録するよう直した。

## 11. PM の未決の点

1. **`first pixel` の意味**(重要): 実エンジン(途中成果物)ではエンジンの first-pixel イベントが主窓でなくロゴで出る。HEAD 2b3a2f2 のアプリ + `mainLid=14` ではこの印は付かない(実測、§0)。私の差分は『最初の idle で主窓に印』を足したが、これは**エンジン側(`fruity_surface.c` の first-pixel の証拠規則が主窓を指す)で直すのが本筋**かもしれない(C の変更。vclock1 の担当と衝突するので私はしなかった)。どちらにするか。
2. **manifest の互換**: 古いアプリ(F3)は新しい manifest を読めない(`mainLid` 必須・`window` が未知)。F4 に上げるときは新しいアプリと一緒に。古いアプリにも新 bundle を読ませたいなら `pack-assets` が `mainLid` を書く案があるが、書く数は誤解を招くので勧めない。
3. **窓リスト(`fle_av_poll`)に依存する**。決定 0141 以降の bundle なら良い。窓リストが無い bundle は規則 2 だけになる。C 側で面の表にクラス名を足す案(version 3)は互換の手間に見合わないと考え、採らなかった。
4. 最終 bundle(vclock1 取り込み後)で `main-check.mjs` と実エンジン e2e を 1 回走らせて、『主窓は lid N、規則 class』のログと三者の SHA 一致を見てほしい(手順は MERGE-NOTES)。
5. 測定ツール(`perf1/perf2/final0129`)は走らせていない。

## 12. 限界(測れなかったこと)

- 過去の bundle での動作(§0 の (2)(c))。実機 Windows の主窓。複数の同クラス窓が同時にある状況。vclock1 の最終 bundle。フォント seed を入れた bundle。
- ブラウザは Chrome 156 headless 1 台だけ。実エンジンの実行は 4 回(+ e2e 1 回)。
- **性能(AGENTS.md の P1〜P3、`docs/milestones/performance-targets.md`)は、この変更の対象ではない**。観測だけ書く: (実測) 最初の idle まで **37.1〜37.7 s**(隔離 headless Chrome 156 の 1 台、他の作業と共有で負荷のある機械)。P1『起動が 10 秒以内』は**未達**で、この変更とは無関係。120 fps の UI と Song Mode の再生は**測っていない**。この変更のコスト: 主窓の判定 1 回は、node の合成メモリ(窓リスト 28 窓、面 3 枚)で **11.3 µs**(20,000 回の平均、`real/bench-main-window.mjs`。ブラウザ内の値ではない)。ブラウザのビルドが受けるイベントは first pixel と stop の 2 種類だけで頻度は低い(1 回の実行で kind 1 が 1 件、stop で 3 窓分)。

## 13. 出典

- `app/src/engine/main-window.ts`、`surface-events.ts`、`app/test/unit/main-window*.test.ts`、`app/test/helpers/synthetic-engine.ts`。
- 窓リストの実測: `lift/out-git/app3-results/logs/real-engine-lid18-summary.txt`、`logs/e2e/*-engine-log.txt`(`handle 0x7f990034 lid 13 class "TFruityLoopsMainForm" ... owner 0`)、今回の `logs/real/run2〜4/result.json`・`engine-log.txt`。
- `lift/harness/fruity_surface.c`(`owner_lid`/`form_key` に代入が無い。first-pixel イベントの規則)、`fruity_idle.c`(kind 3 は `fle_g3_snapshot` だけ)、`lift/runtime/fle_appview.c`、`lift/runtime/fle_win.c`(`next_lid++`)、`docs/investigations/detach-window.md` R5、`docs/policies/progress-integrity.md`。
- 決定 0126、0136、0141、0145、0147(PM の調査による。この木に無い)。
