# tools/e2e: E2E の基盤(Issue #1407・#1485)

本番と同じ配信(Worker・ヘッダー・bundle・assets)に対して、Playwright の Chromium で実際のマウス・キーの操作を行い、停止・例外・消えない表示・失敗した要求・意図しない権限の確認を数える。製品(`app/`・`lift/`)は変えない。

存在する機能の一覧(`features/`)を軸に、領域ごとのシナリオで、期待する結果を確かめる(#1485)。`tools/crash-hunt/`(停止だけを見る走査)は別に残してある。

## 取得と実行

```
(cd tools/e2e && npm ci && npx playwright install chromium)
bash tools/e2e/run.sh --scenarios smoke --displays 4k-dpr1,4k-dpr2,4k-random > e2e.jsonl
bash tools/e2e/run.sh --area page --shard 1/2 --displays 4k-dpr1 --budget 40m > e2e.jsonl
node tools/e2e/run.mjs --list
```

前提: `app/node_modules` と `app/dist`(`npm run build --prefix app`)、bundle(`lift/out-app/bundle/{raw,br}/fl-Oz`、`fe-build bundle`)。

- stdout: 表示の行ごとに 1 行の JSON。stderr: 経過。終了コードは、全ての行が受け入れを満たしたときだけ 0(`--report-only` のときは、下の「判定の基準」の規則)。
- 各シナリオの結果: `displays`(宣言の表示)・`ms`(開始から終了までのミリ秒)・`dprOk`(ページの DPR が行の表示と同じか。`4k-random` の行は null)・`expectStop`(停止の宣言)・`failLog`(判定で落ちたシナリオだけ。エンジンのログの、そのシナリオの開始から後の末尾 200 行。画像は無い)。
- 試験: `node --test tools/e2e/test/*.test.mjs`(Playwright を使わない。純粋な部分の試験)。

## 引数(`run.mjs`)

| 引数 | 意味 |
|---|---|
| `--scenarios smoke` | 旧い形の集合(`scenarios/smoke/index.mjs`)。各シナリオは、それぞれ新しいページで動く(従来どおり)。 |
| `--scenarios browser-face` | 旧い形の集合(`scenarios/browser-face/index.mjs`、#1419)。Browser の面のタブを切り替え、console エラーを数える。各シナリオは、それぞれ新しいページで動く。 |
| `--scenarios regression` | 依頼者の報告 1 件 1 シナリオの回帰(`scenarios/regression/index.mjs`、#1408)。旧い形で、各シナリオは新しいページで動く。 |
| `--area <領域>` | `scenarios/<領域>/` の全て。13 領域: `page` `input` `file` `menus` `windows` `audio` `settings` `plugins` `topbar` `resilience` `visual` `perf` `delivery`。 |
| `--list` | 起動しない。領域ごとに 1 行の JSON: `{"area","scenarios","covers","features"}`(`--area` を付けると、その 1 行)。 |
| `--shard i/n` | シナリオの ID を昇順に並べ、`index % n === i - 1` のものを動かす(決定的)。 |
| `--repeat n` | 選んだ各シナリオを n 回動かす(行の各シナリオに `repeat`)。 |
| `--only ID[,ID]` | 領域の ID(`page/canvas-fills-viewport`)か、旧い名前(`file-open-demo`)。 |
| `--budget 40m` | 単位 `s`・`m`・`h`。予算は表示の行ごとに 1 つ(行ごとに時計を作る)。行の予算が過ぎたら、新しいシナリオを始めない。残りが、シナリオの `needsMs`(例 `resilience/wait-idle-30min` は 30 分)より少なければ、始めない。始めなかったものは `skipped: true`(その行は受け入れない)。 |
| `--displays` | 行(表示)の選択。`4k-dpr1`(3840 x 2160 CSS px、DPR 1)、`4k-dpr2`(1920 x 1080、DPR 2)、`4k-random`(DPR を 1.01〜3.00 の乱数で 2 つ)、`dpr:<数>`(例 `dpr:1.25`。CSS px は 3840 ÷ DPR、2160 ÷ DPR)。シナリオは、宣言の `displays` にその表示が在る物だけ、その行で動く。`--displays` はシナリオを広げない。 |
| `--report-only` | 結果を報告するだけで、シナリオの ok・カウンター・停止・DPR では落とさない。全ての行が出て(シナリオの行の数が揃い)、行の `error` が null なら終了コード 0。行や行の欠け・`error` は、従来どおり 1。ログに `report-only: exit 0`。行の構造(`servedHeadersMatch` など)は、呼び出す側が見る(受け入れの `node -e`)。 |
| `--clock raf\|begin` | 既定は `raf`(下の「120 Hz」)。 |
| `--shots DIR` / `--engine-logs DIR` | 試験用。画面の PNG と、エンジンのログ。 |

領域のシナリオは、表示の指定がなければ、各シナリオの `displays`(既定は `4k-dpr1`・`4k-random`)で動く。`--displays` を付けると、行は指定した表示だけになり、各行では、その表示を宣言したシナリオだけが動く(宣言に無い表示では動かない)。指定した表示を宣言するシナリオが 1 本も無ければ、行が無いので落ちる。

**起動は 1 シャードで 1 回。** 領域(`--area`)の実行では、行ごとにページを 1 つ作って起動し、シナリオの間は `File > New`(Ctrl+N。`lib/common.mjs` の `resetProject`)で空の曲に戻してから、次のシナリオを動かす。旧い形(`smoke`)は、従来どおりシナリオごとに新しいページ。

## 領域とシナリオの書き方

シナリオは `tools/e2e/scenarios/<領域>/<名前>.mjs` に 1 本ずつ置く。`run.mjs` が自分で見つける(登録の表は無い)。

```js
// <領域>/<名前>: 何を、どう確かめるかを 1〜2 行で書く。
import { defineScenario } from '../../lib/scenario.mjs';
import { openMenu, closeNew, newWindows, openLids } from '../../lib/common.mjs';

export default defineScenario({
  id: 'input/menu-open-click',          // <領域>/<名前>。領域は 13 のどれか
  covers: ['menus/menu/file'],          // このシナリオが確かめる機能の ID(features/ の ID)
  displays: ['4k-dpr1', '4k-random'],   // 既定。省けば同じ
  expectFail: null,                     // 既知の欠陥なら '#<Issue の番号>' か 'new'
  async run({ guest, page, origin, log }) {
    const base = await openLids(guest);
    await openMenu(guest, 'File');
    const { pops } = await newWindows(guest, base);
    await closeNew(guest, base);
    return { ok: pops.length > 0, engineState: (await guest.state())?.state ?? null, notes: [`drop-downs: ${pops.length}`] };
  },
});
```

- `run(ctx)` は `{ ok, notes, results }` を返す。`ctx` は `guest`(操作と読み取り。`lib/guest.mjs`)・`page`・`cdp`・`browserCdp`・`browserContextId`・`origin`・`repo`・`log`・`index`(このページの中の順番)。
- 操作は実際のマウス・キーだけ(`guest.click`・`guest.rightClick`・`guest.key`・`guest.ctrl`・`guest.drag` など)。エンジンの関数は呼ばない。読み取りは試験用の道具(窓の表 `deskDumpBase64`、画素、ゲストのメモリの文字列 `guestText`、音声の数 `audio.detail()`)。
- 文字は OCR で読まない。名前は、窓の表の題名、ゲストのメモリの文字列、または項目の番号(`#n`)で書く。
- `covers` は機能の ID の配列(`<領域>/<種類>/<名前>`)。ID の形と領域だけ検査する。別の領域の機能を覆ってもよい。
- `defineScenario` は、ID の形・領域・`covers` の形・`expectFail` の形を検査し、違えば読み込みで落ちる。

領域ごとの見本(各 3 本。どれも通る):

| 領域 | 見本 |
|---|---|
| page | `canvas-fills-viewport`・`first-frame-drawn`・`persist-reload-starts-again` |
| input | `menu-open-click`・`right-click-mixer`・`key-escape-closes-popup` |
| file | `new-project-empty`・`menu-first-item-chosen`・`open-box-shows` |
| menus | `file-popup-opens`・`edit-popup-opens`・`down-moves-highlight` |
| windows | `f8-opens-plugin-list`・`main-form-present`・`right-click-mixer-menu` |
| audio | `output-starts-on-click`・`quanta-while-open`・`no-underrun-while-idle`(#1491 で通るようにした。下の「音声の領域」) |
| settings | `f10-opens-midi-settings`・`pages-listed`・`sections-under-midi` |
| plugins | `balance-lazy-entry`・`sytrus-lazy-entry`・`manifest-lists-plugins` |
| topbar | `menu-bar-visible`・`toolbar-panels-present`・`control-bar-present` |
| resilience | `idle-20s-no-stop`・`reset-five-times`・`escape-storm` |
| visual | `no-black-regions`・`menu-changes-picture`・`picture-size-matches-guest` |
| perf | `rendering-frames-counted`・`menu-open-latency`・`engine-idle-after-menu` |
| delivery | `manifest-served`・`lazy-module-served`・`document-security-headers` |

### 共通の部品(`lib/common.mjs`)

- `menuPath(guest, ['File', 'Open'])` / `['File', '#9']` / `['File', '#9', '#2']`: メニューの項目を選ぶ(最後の Enter が項目の動作を実行する)。名前は `features/generated/menus.tsv` のもの、無ければ `#n`。戻り値は項目の ID(`menus/item/file-9`)。
- `waitWindow(guest, cls, ms)`・`waitTable(guest, pred, ms)`: 窓の表で条件が出るまで待つ。
- `pixelHashRegion(guest, rect)`: 画素の一部の SHA-256。画素が変わったかの判定に使う。
- `noBlackRegions(guest)`: 全画面の 64 x 64 の区画(タイル)のうち、全画素が 0 のものの数。
- `guestText(guest, markers, radius)`: ゲストのメモリの文字列(ASCII と UTF-16LE)。
- `audioProbe(guest, ms)`: 音声の状態の前後の値と、出力の標本の要約(`tools/p3-audio-probe/waveform.mjs` を使う)。
- `persist(page, guest)`: 再読み込みの後に、起動し直して曲が開くかを確かめる。
- `resetProject(guest)`(File > New)・`startProject(guest)`(利用データの箱・Welcome・Problems の箱を閉じ、既定の曲を開く)。
- `closeNew(guest, base)`(Escape で新しい窓を閉じる): Escape の後は、窓が消えるまで窓の表で待つ(`waitFewer`、最大 5 秒)。固定の待ちでは、閉じ終わる前の二度目の Escape が主窓に届き、Channel rack が隠れる(#1510)。

### プラグインの領域(#1493)

- `scenarios/plugins/_generate.mjs`(引数なし。リポジトリから実行する)が、`features/generated/plugins.tsv`(内蔵 142 件と外部 3 件)から、1 プラグイン 1 本ずつ 4 段のシナリオ(`builtin-<種類>-<名前>.mjs`・`external-<名前>.mjs`)と、窓の位置のシナリオ(`position-<…>.mjs`)を作る。`features/plugins.tsv`(機能の表。窓・パラメータ・音声・止まらないの 4 行ずつ、ほかにツールとプリセットの行)も同じ道具が書く。生成物は commit する。`_` で始まるファイルは、`run.mjs` が読まない。
- 生成のやり方: `node tools/e2e/scenarios/plugins/_generate.mjs`(リポジトリの根から。引数なし)。生成物(`builtin-*`・`external-*`・`position-*`・`features/plugins.tsv`)は commit する。生成物は手で編集しない。`_generate.mjs` は、取り込まれても何も書かない(関数は試験の `tools/e2e/test/plugin-marks.test.mjs` が使う)。
- 印(`_generate.mjs` の `markOf`、#1508): 内蔵は、名前を含む開いている束の Issue の番号(題名が「内蔵エフェクト N」「内蔵インストゥルメント N」の行)。束が無い物は `#1535`(Plugin database に行が描かれない件。挿入の段で落ちる)。外部 3 本(Serum 2・OTT・Vital)は `#1324`(開いた Feature)。閉じた番号(`CLOSED_ISSUES`: #1315・#1319・#1336・#1337・#1338・#1400・#1484)は印にしない。単独の名前の行(例: #1342)は束ではないので印にしない。`#1535` が直ったら、その作業が `INSERT_MARK` の印を外す。
- 前提の確認(#1508): 件数は一致する。Installed の enum-dir の件数は 3 で、`lift/plugins/` の内蔵 3 本(fruity-balance・fruity-filter・sytrus)と同数。名前はエンジンのログに出ないので、一覧に出る名前は確かめられない。Installed の行は描かれず(画素で読む)、項目は選べない。
- 挿入(`lib/plugins.mjs` の `insertFromDatabase`、#1508): Mixer を `ensureMixer`(F9)で出し、挿入 1 を遅い二度押しで選ぶ → 右の欄の Slot 1 → picker(More plugins…・Plugin database・表示の切替)→ Plugin database の項目で、Browser が Plugin database(Installed)を出す。挿入欄の位置は Mixer の大きさに対する割合で求める(DPI で拡大されるため)。Installed の行が描かれているかは、Installed を選んで Right を押した後の行の領域の画素で読む(`installedRowsShown`)。行が描かれなければ、項目は選べないので、挿入はせず、結果(`results.named`・`notes`)に書く。この build では行は描かれない。行が描かれた場合も、項目の行の位置は未測定なので、`insertFromDatabase` は ok にしない。
- 一覧の観測: `databaseLines(guest, from)` は、シナリオの開始時のエンジンのログの長さ `from` を必ず渡す(既定は無い)。件数は `installedCount`(enum-dir の行の数)で、`results.count` に出す。エンジンは一覧を、ページの最初に 1 回だけ書くので、同じページの後のシナリオでは件数は無い(`results.count` は null)。名前は、ログに出ないので、描かれた行(画素)でしか読めない(`results.named`)。
### 見た目(visual)のゴールデン(#1496)

- 画素の期待値は `golden/visual/<kind>.tsv`(`キー<TAB>ハッシュ<TAB>許容誤差`)。kind は `window`・`theme`・`language`・`dpr`・`start`。ハッシュは矩形の RGB バイト列の SHA-256(`lib/picture.mjs` の `regionHash`)。
- 同じ場所の `<kind>.cells.tsv` は、矩形を 16 x 16 の区画に分けた、区画ごとの平均の R・G・B(16 進)。ハッシュが違うときは、区画の判定で差を見る: 平均のどれかのチャンネルが 8(255 中)を超えて動いた区画の割合が、行の許容誤差以内なら `within`、超えたら `differs`(`lib/visual.mjs` の `judgeGolden`)。許容誤差の既定は 0.02(256 区画のうち 5 区画まで)。
- キーの無い行は、その実行の画素を初回の値として書く(`recorded`)。キーには DPR を含める(`@dpr1.25` など)。
- 更新: 値を変える理由を報告に書いた時だけ。該当の行(`<kind>.tsv` と `<kind>.cells.tsv` の両方)を消し、同じ DPR の実行で書き直す。変わった時は、報告に区画の割合を書く。
- 判定の式(`lib/visual.mjs`): 縁は、窓の矩形の 4 辺それぞれで、外側と内側の対比が 20 以上、縁の線の幅(外側にも内側にも似ない画素)が 2 以下、柔らかい帯(10〜90 %)が 2 以下。黒い領域は全体の 64 x 64 の区画で、全画素が 0 の数が 0。文字は上部バー(TNewMenu)の色の数が 8 以上、輝度の幅が 40 以上。カーソルは `image-set` で、宣言された倍率が DPR に一致(0.01 以内)。
- DPR のシナリオは、CDP の Emulation で DPR を設定し(物理の大きさを保つ)、測って、最後に戻す。乱数の 2 点は、シード 1496 で引いた固定の値(キーに値が入る)。
- 表示: visual の全件は `4k-dpr1`(DPR 1。乱数の表示は使わない。キーが毎回変わるため)。
- 黒い領域: 数えるのは Browser 窓(`TEdgeBrowser`)の外だけ。Browser 窓の範囲の黒い区画は `browserBlack` として別に出す(判定には入れない。原因は未確認)。
- エンジンが止まった場合: 次の visual の場面の開始で、ページを読み直して曲を開き直す(`lib/visual.mjs` の `recover`。DPR は行の基準に戻す)。
- 印 `new` の場面(窓の縁・テキスト欄のカーソル・DPR 2.06 の設定窓)は、判定の式に合わない値、または測れない値が出た場面。値は run の出力に残る。
## 音声の領域(#1491)

- `tools/e2e/lib/audio.mjs`: 操作と読み取りの共通部品と、判定(`CHECKS`)。各シナリオ(`scenarios/audio/`)は、判定の名前と引数を `runCheck` に渡す。
- 読み方: 装置(`deviceState` と量子)は、エンジンの起動から再生中であり、無音でも量子を取る。**トランスポートが動いているかは、上部バーの時刻表示の画素(`TIME_REGION`)が動くこと、または非無音の量子が増えること**で見る(`isPlaying`)。量子の数だけでは再生を判定できない。
- 途切れ(underrun)は、再生の窓の前に 1.5 秒の warm-up を置き、その後の窓で数える(`underrunQuanta` の差)。
- 位置(`PLACE`)は、3840 x 2160 のゲストの画素(4k-dpr1)。期待値ではなく、押す場所。
- 機能の表 `features/audio.tsv`(43 件)のうち、印の無いシナリオが覆うものは 38 件(88%)。覆えていないのは 5 件: 拍子(`audio/transport/time-signature`。拍子の部品を画面で見つけていない。トップバーのテンポ・メトロノーム・カウントインの右クリックのメニューに無い)、ルーティング(`audio/route/insert-output`。インサートの ▲ は、キックの音を変えない。キックはマスターへ直接届く)、録音(`record-toggle`・`record-play`・`count-in-clicks`。下の「録音」)。90% の 39 件には 1 件足りない。
- 追加の 4 本(#1491): `export-wav-nonsilent`・`export-wav-length`(File > Export > Wave file の Start → 保存されたファイル `window.__app.saved` を `parseWav` で読む。長さは 1 小節 = 4 拍・130 BPM = 1.846 s、±0.03 s。130 BPM は新規の曲の既定のテンポ)、`pan-master-wheel`(マスターのつまみ `PLACE.masterKnob` を 4 目盛り回すと、左右の釣り合い `balanceOf` が 0.1 超動き、戻すと 0.05 未満に戻る)、`solo-insert-1-silences`(インサート 1 の LED を Ctrl+クリック(`modClick`)するとキックの音が消え、もう一度で戻る)。
- キックの拍(`kickSteps`): 既定の曲のチャンネルラックのキックの 1・5・9・13 拍を押すと、再生で音が出る(`nonSilentQuanta` が 400 を超える)。メトロノームは切る(`setMetronome`。ボタンの画素の明るさで判定。前の場面が点けたままのことがあるため)。
- 録音(製品の欠陥。印 `new`): 録音ボタンは一度押すと待機になる。二度目の押しで戻らないことがある。待機の間は Space で再生が始まらない(音も出ない)。このため、録音の場面は同じシャードの最後に走るよう、ID を `zz-` で始める(`zz-record-arm-then-play`・`zz-record-button-toggle`・`zz-record-button-toggle-wide`・`zz-count-in-clicks-before-record`。ID の並びは `run.mjs` のシャードの順)。
- 印 `new` の 4 本: 上の録音の 4 本(`zz-record-arm-then-play`: 待機で Space が再生を始めない。`zz-count-in-clicks-before-record`: 待機で音が出ない。`zz-record-button-toggle`: 一度目は変わり二度目で戻るが、通らないことがある。`zz-record-button-toggle-wide`: 一度目で画素が変わらない)。
- 印を外した 3 本(#1491): `cycles-3`・`repeat-12288`・`master-fader-down-200`。録音の場面の後に走ったために音が無かったもので、録音の場面を最後に回したら通った(実行の行は報告に書く)。
## 印(expectFail)

- 印の無いシナリオ: `ok` が真で、`pageErrors`・`consoleErrors`・`stuckToasts`・`failedRequests` が 0。
- 印 `#<Issue の番号>`(既に Issue がある既知の欠陥)・`new`(このシナリオが見つけた欠陥で、まだ Issue が無い): `ok` は問わない。`pageErrors`・`consoleErrors`・`failedRequests` が 0。`stuckToasts` は問わない(JSON に出す)。エンジンが止まった(`engineStops` > 0、または `engineStuckRunning`)なら失敗する。通ったら標準エラーに警告を出す(`judge.jq`)。
- 停止の宣言 `expectStop`: `defineScenario`(旧い集合の `index.mjs` の `{ name, … }` も同じ)に `expectStop: '#<Issue の番号>'` または `'new'` を書くと、そのシナリオの停止だけを許す。`expectFail` が無ければ読み込みで落ちる(停止は既知の欠陥の中でだけ許す)。カウンター(`pageErrors`・`consoleErrors`・`failedRequests`)は、停止を宣言しても 0 でなければならない。
- 予算の残り: `defineScenario` の `needsMs`(開始に必要な残りの予算。既定 0)。残りが足りなければ、そのシナリオは `skipped: true` になる。`ctx.budget.remainingMs()` で、行の残りを読める。
- ワークレットの読み込みは、以前は HTTP 403 だった(Worker の Access の検査が、ワークレットの要求にトークンのヘッダーを付けられていなかった)。試験の中継(`lib/delivery.mjs` の `startAccessProxy`)が、トークンの無い要求にだけ付けて直した。

## 機能の一覧(`inventory.mjs`)と網羅率(`coverage.mjs`)

### 一覧

`node tools/e2e/inventory.mjs --out tools/e2e/features/generated`(本番と同じ配信で FL を 1 回起動して読む。リポジトリの一覧も作る。約 10 分)。出力は `features/generated/<領域>.tsv` と `summary.json`(`total`・`perArea`・`unnamed`・`fl`・`perFile`)。

列: `id`(`<領域>/<種類>/<名前の slug>`。全体で一意)・`kind`・`name`・`source`(見つけた経路)・`how`(位置の求め方)・`nameSource`。

- `nameSource = memory`: 名前は、製品が持つデータから読んだ(窓の表の題名、設定の頁の題名、リポジトリの入力の一覧のファイル名、マニフェスト)。
- `nameSource = index`: 名前は番号か位置(`File > #9`・`TQuickSheetSelector #23`)。OCR は使わない。

| ファイル | 中身 | 件数(この runner・この実行) |
|---|---|---|
| `menus.tsv` | 上部の 8 つの見出し(窓の表のバーの位置。見出しの名前は `MENU_LABELS`)と、各プルダウンの項目(Down キーで画素が繰り返すまでの数) | 157(見出し 8、項目 149。実行ごとに 155〜159 の間で揺れる) |
| `windows.tsv` | F5〜F12・Alt+F8 で開く窓(題名あり)、右クリックのプルダウン 3(Channel rack・Mixer・Playlist)、ブラウザの埋め込みの Web 表示(`TEdgeBrowser`、子窓) | 8 |
| `topbar.tsv` | 主窓の下の上部バーの部品(`TWPControl`・`TQuickControlBar`・`TQuickEditToolBar`・`TQuickEditToolbarPanel`・`TNewMenu`) | 22 |
| `settings.tsv` | F10 の設定窓の頁(選択欄の走査で読んだ 10 頁)と、頁ごとの節・一覧・その他の子窓。言語とテーマ(`resources.tsv`)を含めた数 | 97(FL 77 と、言語・テーマ 20) |
| `plugins.tsv` | 本体のエフェクト・ジェネレータ、外部 3(OTT・Serum 2・Vital)、プラグインの データベース・マネージャ 2 | 147 |
| `file.tsv` | テンプレート・デモの曲(`inputs/fl-installer-files.tsv`)、File メニューの項目 | 91 |
| `resources.tsv` | 言語 7(`System\Languages\*.moe`)、テーマ 13(`Artwork\Themes\` の `.flstheme` の 1 件 = 1 テーマ) | 20 |
| `page.tsv` | 本体のモジュールの機能 8(`app/src/`) | 8 |
| `delivery.tsv` | bundle の manifest.json の lazy の全て | 235 |

合計 765(`summary.json` の `total`)。領域ごとは `perArea`(menus 157・windows 8・topbar 22・settings 97・plugins 147・file 91・page 8・delivery 235。resources の 20 は settings に入る)。`nameSource = memory` は 528 件、`index` は 237 件(`unnamed`: メニュー 157、ファイル 10、上部バー 21、設定 35、窓 4、プラグイン 2、page 8)。

**取れなかった・まだ読んでいないもの**(この実行で。runner の値。この機械とは別枠。nproc 4):

- メニューの名前(157 件は全て `nameSource index`): プルダウンの窓(`TQuickPopupMenuWindow`)は、窓の表に子も題名も無く、矩形だけが出る。ゲストのメモリの `&` 付きの文字列(`&New` の周りを `guestText` で `radius` 600 に読む)は、UI の全体の文字列をアルファベット順に並べた一つの表(`&Insert`・`&Invert`・…・`&Open`・`&Options`)で、プルダウンの項目の順とは対応しない。項目の矩形からこの表へ結ぶ規則(項目のポインタの経路)は確かめられなかった。名前は付けず、`#n` で書いた。
- メニューの階層: 上位のプルダウンの項目だけ。サブメニューは歩いていない。項目数は実行ごとに数件揺れる(Down で画素が繰り返すまでの数え方。原因は未確認)。
- 設定窓の頁: 選択欄の上の 10 頁(MIDI・Audio・General・File・Theme・Project・Info・Debug・Account・About)の題名は、窓の表の題名から読んだ。仕様の 16 頁とは差がある。選択欄の隙間(Theme と Project の間、Info と Debug の間)は、クリックしても頁が変わらなかった。残りの頁は見つかっていない。
- 窓: ブラウザのタブ(8 の枠)は、窓の表では無名の `TVectorPanel` の 1 つで、枠ごとには読んでいない。ミキサーのインサートは窓ではなく描画(窓の表には無名の `TVectorPanel` のみ)。ピアノロールは、チャンネルの左端・名前・ステップの位置を単クリック・二重クリックしても開かなかった(窓の表に子窓が出ない)。ブラウザの右クリックは、試した位置ではプルダウンが出ない。右クリックは、設定窓の走査(F10 と Escape)の後では Channel rack と Playlist が窓の表から消えたため、設定より前に読む(`inventory.mjs` の `main`)。
- 上部バーの部品の題名: 21 件は空で、`TWPControl #16` のような番号で書いた。

### 網羅率

`node tools/e2e/coverage.mjs --features tools/e2e/features --run <run の JSON Lines> [--run ...] --out coverage.json`

- 機能: `features/<領域>.tsv`(手で書く。今は見出し行だけ)と `features/generated/*.tsv`。ID が重複すれば落ちる。
- 覆われた: `covers` に持ち、`ok` が真で、`skipped` でなく、`expectFail` が null の結果が 1 つ以上ある機能(印の結果は数えない)。
- 出力: `{ total, covered, unknownCovers, perArea: { <領域>: { features, covered, ratio, scenarios, min } } }`。`scenarios` はその領域の、通ったシナリオの数。
- 判定: `jq -e -f tools/e2e/judge-coverage.jq coverage.json`。領域ごとの `features/<領域>.min`(`scenarios<TAB>最小のシナリオ数`・`coverage<TAB>最小の網羅率`)を下回ると失敗。全てのシャードの結果を合わせて判定する(1 シャードでは判定しない)。`judge.jq` は変えていない。

`.min` の値: page 80・input 120・file 100・menus 400・windows 250・audio 80・settings 150・plugins 200・topbar 60・resilience 40・visual 120・perf 30・delivery 40(網羅率は全て 0.9)。今は見本の 39 本しか無いので、判定は落ちる(見本の結果だけで 2 / 762 の機能が覆われる)。領域の Issue が、シナリオと機能を足して埋める。

`covers` は一覧の ID だけを指す(`unknownCovers` は 0 が正しい)。`input`・`audio` の見本の一部は一覧の機能(`menus`・`page`)を指す。`perf`・`resilience`・`visual`・`delivery/document-security-headers` の見本は `covers: []`。その機能は、領域の Issue が `features/<領域>.tsv` に書き、見本の `covers` に入れる。

## 判定の基準(`judge.jq`・`rowPasses`)

- `servedHeadersMatch` が真。`error` が無い。
- 全てのシナリオ: `skipped` が無い。`dprOk` が偽でない(ページの DPR が行の表示の DPR と同じ。`4k-random` の行は確かめない)。
- 印の無いシナリオ: `ok`・`pageErrors` 0・`consoleErrors` 0・`stuckToasts` 0・`failedRequests` 0。
- 印の有るシナリオ: `pageErrors`・`consoleErrors`・`failedRequests` が 0。エンジンの停止が無い(`engineStops` 0・`engineStuckRunning` が偽)。ただし `expectStop` が在るシナリオは停止を問わない。
- 判定で落ちたシナリオ(`skipped` を除く)には `failLog`(エンジンのログの末尾 200 行)が付く。
- `--report-only`(`runPasses`): 判定の代わりに、行の数とシナリオの行の数が揃い、行の `error` が null であること。シナリオの結果では落とさない。領域の受け入れは、これを使う(個々の ok と印は、`e2e-full` が見る)。

## 構成

- `lib/delivery.mjs`: 配信。R2 の代わりに、ローカルの R2(`wrangler dev --local` の永続化先)へ bundle を入れ、`app/worker/index.ts` を `wrangler dev --local` で立てる。アクセス制御の鍵・チーム名・AUD は、この実行のために作る(`lib/access.mjs`)。製品の設定の値は書き換えない。ページの `origin` は、前段の試験の中継(トークンの無い要求にだけ `Cf-Access-Jwt-Assertion` を付ける)の番地で、配信の検査(`probeServed`)は Worker を直接呼ぶ(`upstream`)。
- `lib/served.mjs`: `servedHeadersMatch`。配信の応答のヘッダーを `app/shared/headers.ts` と比べる。`Content-Encoding` は、Worker を直接呼んだ応答で比べる。
- `lib/browser.mjs`: 表示(DPR・CSS の大きさ)、Chromium の起動、120 Hz の時計、rAF の計測。
- `lib/guest.mjs`: 操作(クリック・右クリック・ダブルクリック・ホイール・ドラッグ・キー・Ctrl+キー)と読み取り(窓の表、エンジンの状態)。座標はゲストの画素から CSS px へ写す。
- `lib/detect.mjs`: 検出(`pageerror`・`console.error`・失敗した要求と 4xx/5xx・消えない表示・エンジンの停止と `running` の 90 秒・クリップボードの呼び出しと権限の変化)。
- `lib/flow.mjs`: 起動の手順(利用データの確認、Welcome を閉じる、「Problems loading」の箱、Ctrl+O で開く)。
- `lib/scenario.mjs`(登録・検査)、`lib/plan.mjs`(引数・表示・シャード・時間・判定。Playwright を使わない)、`lib/common.mjs`(共通の部品)、`lib/dump.mjs`(窓の表の読み)、`lib/picture.mjs`(画素)、`lib/inventory.mjs`(一覧の行)、`lib/topbar.mjs`(上部バーの部品の操作と、メモリの表示の値。`scenarios/topbar/`)。
- `inventory.mjs`・`coverage.mjs`・`judge-coverage.jq`: 機能の一覧と網羅率。
- `features/`: 手で書く見出し行と `.min`、`generated/`(一覧の生成物。コミットする)。
- `scenarios/smoke/`: (a) `start-welcome-default`、(b) `knob-right-click-permissions`(Issue #1482: 印は外した。右クリックで読まず、通知が出ない)、(c) `file-open-demo`(File > Open で DEDZII を開く。修正の前は約 5 秒後に engine が exit 134 で止まった(#1421)。修正後は開き、印は外してある)、(d) `plugin-manager-opens`(印 `#1499`: Options の項目を探し、Plugin Manager の窓が出て engine が 10 秒 engine-idle のままかを見る。Plugin Manager は別の実行ファイル(`System\Tools\Plugin Manager\PluginManager.exe`)で、#1499 までは窓が出ない。印の付いたシナリオは、ok を問わず、カウンタが 0 であることだけを見る)、(e) `plugin-database-lists-builtins`(Plugin database の `Installed\` の一覧の件数が、`lift/plugins/` の、本体(主の DLL)の行を持つフォルダの数以上。補助だけのフォルダ(`reverb`)は数えない。件数はエンジンの listing(`enum-dir`)から読む)、(f) `page-chrome`(Issue #1481: 最初の idle で、キャンバスが窓いっぱいに出ること、キャンバス以外に見える要素が無いこと、本文に `data:image` の文字列が無いこと)。
  - (b) の DPR: 宣言は `4k-dpr1`・`4k-dpr2` だけ(ノブの位置を測った DPR)。他の DPR(`4k-random` の 2.73 など)では、メニューが出ないことを判定せず、注記 `position-unverified` を出す。クリップボードの読み・通知・権限の変化の 0 は全ての DPR で見る。
  - `--displays` を明示すると宣言は効かない(`run.mjs` の `rowsOf` は明示時に全ての行で走らせる。直すのは #1506)。その時も上の扱い(安全策)で判定する。
  - Issue #1482 の 3 件: (g) `context-menu-none`(各領域で右クリック 20 回。ブラウザのメニューが抑止されること)、(h) `clipboard-load-permission`(権限 prompt で読み込み、最初のクリック後に readText の要求が 1 回以上あること)、(i) `clipboard-no-toast`(権限 denied で Ctrl+V・Ctrl+C をし、通知の要素が出ないこと)。
- `scenarios/browser-face/`(Issue #1419): (a) `browser-face-tabs`。Browser の面のタブを切り替えて 10 秒待ち、console エラーを数える。集合の件数は 1(`run.mjs` の行の判定は、集合の件数で数える)。
- `lib/proxy.mjs`: 配信への転送。ブラウザの要求を経由させ、アクセスのトークンを、トークンの無い要求(オーディオのワークレットの取得を含む)に付ける。本番では配信の前の認証の端が、ページの全ての要求に付けるのと同じ。
- `scenarios/regression/` (#1408): 依頼者の報告 1 件 1 シナリオ(R01〜R18)。各ファイルの先頭の定数に、名前・期待・印を書く。`index.mjs` の `ALLOWED_MARKS` が、印に使える番号(開いている Issue)の一覧。`common.mjs` は、窓の表(子の窓と、絶対の位置・窓の style)、既定のプロジェクトの起動、メニュー、設定の頁(タブの枠)、画素の比較、オーディオの数字を読む。
  - 実行: `bash tools/e2e/run.sh --scenarios regression --displays 4k-dpr1,4k-dpr2`。一部だけ試すには `--only 名前,名前`(開発用)。
  - 印の付いたシナリオが全て通ったときは、stderr に `warning: expectFail scenario passed: <name> <issue>. remove the mark` を出す(通ったのに失敗扱いの時は出さない: 消えない表示・ページの例外・停止があれば、通ったとはみなさない)。
  - 出力の行の `expectFail` に、印の一覧(`[{"name","issue"}]`)を出す。1 つのシナリオに印が 2 つあるとき(R11)は、そのシナリオの `expectFail` を配列にし、一覧に 2 件出す。
  - 印の番号は `index.mjs` の `ALLOWED_MARKS`(`common.mjs` の `OPEN_ISSUES`)に在るものだけ。閉じた Issue の印は付けない。
  - 測れない操作(窓の表に出ない文字・頁のタイル)は、そのシナリオを落とし、報告に書く。推測の座標では判定しない。

## 120 Hz の方法と実測

- 方法 (ii) を採った: Chromium を `--disable-frame-rate-limit --disable-gpu-vsync` で起動し、ページの rAF を数える。`achievedFps` は各シナリオの rAF の 1 秒あたりの平均(全シナリオの合計)。
- 方法 (i) も試した(headless shell の BeginFrame)。3840 x 2160 では 1 秒に 1 フレーム程度で、入力も止まったため採らなかった。`--clock begin` は残してある。
- 測定の値は runner の値(この機械とは別枠)。

## 注・既知の限界

- Escape で Welcome を閉じると、Welcome の選んでいる項目のプロジェクトが開く(表示によって違う)。
- 開く箱(Ctrl+O)は、Escape では閉じない。headless のブラウザは選択画面をすぐ閉じ、ページは「人の次の操作を待つ」状態で箱を残す(`app/src/ui/filedlg.ts` の `QUICK_CANCEL_MS`)。箱を閉じるには、`lib/flow.mjs` の `openProjectFile` で答える(`file/open-box-shows`)。
- 「消えない表示」は、`#status`・`#audio-status`・`#taskbar` を除く `role=status`・`role=alert`・`aria-live` の要素で数える。
- `wrangler dev --local` は br の本体を gzip に変換して返す(観測)。そのため `Content-Encoding` は Worker を直接呼んだ応答で確かめる。
- 一時ファイルは `.tmp-e2e-*` に置き、終わると消す。Chromium の本体は Playwright の既定の場所(リポジトリの外)に入る。
