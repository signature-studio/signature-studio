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

- stdout: 表示の行ごとに 1 行の JSON。stderr: 経過。終了コードは、全ての行が受け入れを満たしたときだけ 0。
- 試験: `node --test tools/e2e/test/*.test.mjs`(Playwright を使わない。純粋な部分の試験)。

## 引数(`run.mjs`)

| 引数 | 意味 |
|---|---|
| `--scenarios smoke` | 旧い形の集合(`scenarios/smoke/index.mjs`)。各シナリオは、それぞれ新しいページで動く(従来どおり)。 |
| `--area <領域>` | `scenarios/<領域>/` の全て。13 領域: `page` `input` `file` `menus` `windows` `audio` `settings` `plugins` `topbar` `resilience` `visual` `perf` `delivery`。 |
| `--list` | 起動しない。領域ごとに 1 行の JSON: `{"area","scenarios","covers","features"}`(`--area` を付けると、その 1 行)。 |
| `--shard i/n` | シナリオの ID を昇順に並べ、`index % n === i - 1` のものを動かす(決定的)。 |
| `--repeat n` | 選んだ各シナリオを n 回動かす(行の各シナリオに `repeat`)。 |
| `--only ID[,ID]` | 領域の ID(`page/canvas-fills-viewport`)か、旧い名前(`file-open-demo`)。 |
| `--budget 40m` | 単位 `s`・`m`・`h`。時間を過ぎたら、新しいシナリオを始めない。始めなかったものは `skipped: true`(その行は受け入れない)。 |
| `--displays` | `4k-dpr1`(3840 x 2160 CSS px、DPR 1)、`4k-dpr2`(1920 x 1080、DPR 2)、`4k-random`(DPR を 1.01〜3.00 の乱数で 2 つ)、`dpr:<数>`(例 `dpr:1.25`。CSS px は 3840 ÷ DPR、2160 ÷ DPR)。 |
| `--clock raf\|begin` | 既定は `raf`(下の「120 Hz」)。 |
| `--shots DIR` / `--engine-logs DIR` | 試験用。画面の PNG と、エンジンのログ。 |

領域のシナリオは、表示の指定がなければ、各シナリオの `displays`(既定は `4k-dpr1`・`4k-random`)で動く。`--displays` を付けると、選んだシナリオは全てその表示で動く。

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
| audio | `output-starts-on-click`(通る)・`quanta-while-open`・`no-underrun-while-idle`(印 `new`、後述) |
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

### プラグインの領域(#1493)

- `scenarios/plugins/_generate.mjs`(引数なし。リポジトリから実行する)が、`features/generated/plugins.tsv`(内蔵 142 件と外部 3 件)から、1 プラグイン 1 本ずつ 4 段のシナリオ(`builtin-<種類>-<名前>.mjs`・`external-<名前>.mjs`)と、窓の位置のシナリオ(`position-<…>.mjs`)を作る。`features/plugins.tsv`(機能の表。窓・パラメータ・音声・止まらないの 4 行ずつ、ほかにツールとプリセットの行)も同じ道具が書く。生成物は commit する。`_` で始まるファイルは、`run.mjs` が読まない。
- 印: 印の番号は `pm-work/created-issues.tsv` の題名で引く(内蔵は「内蔵エフェクト N」「内蔵インストゥルメント N」の行、外部は「Serum 2 を…」の行)。表に無い物は `'new'`。Fruity Balance・Fruity Filter・Fruity Delay 2・Sytrus は、読み込みの前に Plugin database の一覧が空なので(#1484)、その番号を付ける。
- 挿入の経路(`lib/plugins.mjs`): Mixer の挿入 1 を遅い二度押しで選ぶ → 右の欄の Slot 1 → picker(More plugins…・Plugin database・表示の切替)→ Plugin database の項目で、Browser が Plugin database(Installed)を出す。挿入欄の位置は Mixer の大きさに対する割合で求める(DPI で拡大されるため)。

## 印(expectFail)

- 印の無いシナリオ: `ok` が真で、`pageErrors`・`consoleErrors`・`stuckToasts`・`failedRequests` が 0。
- 印 `#<Issue の番号>`(既に Issue がある既知の欠陥)・`new`(このシナリオが見つけた欠陥で、まだ Issue が無い): `ok` は問わない。`pageErrors`・`consoleErrors`・`failedRequests` が 0。`stuckToasts` は問わない(JSON に出す)。通ったら標準エラーに警告を出す(`judge.jq`)。
- `new` の印を付けた見本: `audio/quanta-while-open`・`audio/no-underrun-while-idle`。音声のワークレットは読み込まれる(`nodeReady` が真。`audio/output-starts-on-click` は通る)が、音声の標本(クォンタ)が測った間に増えない(`quanta +0`)。試験の環境の音声出力か製品かは未確認。印は、原因を見て Issue を起こし、外す。
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
- 印の無いシナリオ: `ok`・`pageErrors` 0・`consoleErrors` 0・`stuckToasts` 0・`failedRequests` 0。`skipped` が無い。
- 印の有るシナリオ: `pageErrors`・`consoleErrors`・`failedRequests` が 0。

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
- `scenarios/smoke/`: (a) `start-welcome-default`、(b) `knob-right-click-permissions`(Issue #1482: 印は外した。右クリックで読まず、通知が出ない)、(c) `file-open-demo`(File > Open で DEDZII を開く。修正の前は約 5 秒後に engine が exit 134 で止まった(#1421)。修正後は開き、印は外してある)、(d) `plugin-manager-opens`(印 `#1499`: Options の項目を探し、Plugin Manager の窓が出て engine が 10 秒 engine-idle のままかを見る。Plugin Manager は別の実行ファイル(`System\Tools\Plugin Manager\PluginManager.exe`)で、#1499 までは窓が出ない。印の付いたシナリオは、ok を問わず、カウンタが 0 であることだけを見る)、(e) `plugin-database-lists-builtins`(Plugin database の `Installed\` の一覧の件数が、`lift/plugins/` の内蔵プラグインの数以上。件数はエンジンの listing(`enum-dir`)から読む)、(f) `page-chrome`(Issue #1481: 最初の idle で、キャンバスが窓いっぱいに出ること、キャンバス以外に見える要素が無いこと、本文に `data:image` の文字列が無いこと)。
  - Issue #1482 の 3 件: (g) `context-menu-none`(各領域で右クリック 20 回。ブラウザのメニューが抑止されること)、(h) `clipboard-load-permission`(権限 prompt で読み込み、最初のクリック後に readText の要求が 1 回以上あること)、(i) `clipboard-no-toast`(権限 denied で Ctrl+V・Ctrl+C をし、通知の要素が出ないこと)。

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
