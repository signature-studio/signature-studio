# tools/e2e: E2E の基盤(Issue #1407)

本番と同じ配信(Worker・ヘッダー・bundle・assets)に対して、Playwright の Chromium で実際のマウス・キーの操作を行い、停止・例外・消えない表示・失敗した要求・意図しない権限の確認を数える。製品(`app/`・`lift/`)は変えない。

## 取得と実行

```
(cd tools/e2e && npm ci && npx playwright install chromium)
bash tools/e2e/run.sh --scenarios smoke --displays 4k-dpr1,4k-dpr2,4k-random > e2e.jsonl
```

前提: `app/node_modules` と `app/dist`(`npm run build --prefix app`)、bundle(`lift/out-app/bundle/{raw,br}/fl-Oz`、`fe-build bundle`)。

- stdout: 表示の行ごとに 1 行の JSON。stderr: 経過。終了コードは、全ての行が受け入れを満たしたときだけ 0。
- `--displays`: `4k-dpr1`(3840 x 2160 CSS px・DPR 1)、`4k-dpr2`(1920 x 1080・DPR 2)、`4k-random`(DPR を 1.01〜3.00 の乱数で 2 つ。行は 2 つ)。
- `--clock raf|begin`: 既定は `raf`(下の「120 Hz」)。`begin` は BeginFrame の時計(検討の記録用に残した)。
- `--only NAME[,NAME]`: 試験用。セットの中の名前のシナリオだけを動かす(例: `--only file-open-demo`)。行のシナリオ数は、動かした数になる。
- `--shots DIR`: 試験用。各シナリオの最後の画面を PNG で書く。
- `--engine-logs DIR`: 試験用。各シナリオのエンジンのログ(ページが保持する全行)を DIR に書く。
- 各シナリオの `engineStop`: エンジンが止まった時(`engineStops` が 1 以上、または状態が error・exited)だけ、停止の事実を入れる。`exit`(reason・code)・`message`・`stop`(最後の `fle:` の行・`== stop:`・abort kind・メモリ)・`returnAddress`(戻り番地)・`registers`(R[0..15])・`report`(停止の報告の行)。止まらなければ `null`。読み方は `lib/stop.mjs`(製品の `app/src/perf/exit-message.ts` を使う)。

## 判定と印(expectFail)

- 印の無いシナリオ: `ok` が真で、`pageErrors`・`consoleErrors`・`stuckToasts`・`failedRequests` が 0。
- 印(`expectFail: '#<Issue の番号>'`)の付いたシナリオ(既に在る欠陥): `ok` は問わない。`pageErrors`・`consoleErrors`・`failedRequests` が 0。`stuckToasts` は問わない(JSON に出す)。通ったら標準エラーに警告を出す。
- 門(deploy.yml)は `judge.jq` で判定する。`run.mjs` の終了コードは同じ規則(`rowPasses`)。
- シナリオの JSON: `dumpRetries`(窓の表の取得が「ask again」で待った回数。上限は 60 秒)、`engineRunningMaxMs`(`running` の最長。止まりの判定は 90 秒を超えたときだけ)。

## 構成

- `lib/delivery.mjs`: 配信。R2 の代わりに、ローカルの R2(`wrangler dev --local` の永続化先)へ bundle を入れ、`app/worker/index.ts` を `wrangler dev --local` で立てる。アクセス制御の鍵・チーム名・AUD は、この実行のために作る(`lib/access.mjs`)。製品の設定の値は書き換えない。
- `lib/served.mjs`: `servedHeadersMatch`。配信の応答のヘッダーを `app/shared/headers.ts` と比べる。`Content-Encoding` は、Worker を直接呼んだ応答で比べる(下の注)。
- `lib/browser.mjs`: 表示(DPR・CSS の大きさ)、Chromium の起動、120 Hz の時計、rAF の計測。
- `lib/guest.mjs`: 操作(クリック・右クリック・ダブルクリック・ホイール・ドラッグ・キー・Ctrl+キー)と読み取り(窓の表、エンジンの状態)。座標はゲストの画素から CSS px へ写す。
- `lib/detect.mjs`: 検出(`pageerror`・`console.error`・失敗した要求と 4xx/5xx・消えない表示・エンジンの停止と `running` の 90 秒(この runner は GPU が無く、4K の起動が 24 秒かかる。起動の止まりは boot-bundle.mjs と tools/boot-hang が見る)・クリップボードの呼び出しと権限の変化)。
- `lib/flow.mjs`: 起動の手順(利用データの確認、Welcome を閉じる、「Problems loading」の箱、Ctrl+O で開く)。
- `scenarios/smoke/`: (a) `start-welcome-default`、(b) `knob-right-click-permissions`(印 `#1406`)、(c) `file-open-demo`(File > Open で DEDZII を開く。修正の前は約 5 秒後に engine が exit 134 で止まった(#1421)。修正後は開き、印は外してある)、(d) `plugin-manager-opens`(印 `#1499`: Options の項目を探し、Plugin Manager の窓が出て engine が 10 秒 engine-idle のままかを見る。Plugin Manager は別の実行ファイル(`System\Tools\Plugin Manager\PluginManager.exe`)で、#1499 までは窓が出ない。印の付いたシナリオは、ok を問わず、カウンタが 0 であることだけを見る)、(e) `plugin-database-lists-builtins`(Plugin database の `Installed\` の一覧の件数が、`lift/plugins/` の内蔵プラグインの数以上。件数はエンジンの listing(`enum-dir`)から読む)、(f) `page-chrome`(Issue #1481: 最初の idle で、キャンバスが窓いっぱいに出ること、キャンバス以外に見える要素が無いこと、本文に `data:image` の文字列が無いこと)。

## 120 Hz の方法と実測

- 方法 (ii) を採った: Chromium を `--disable-frame-rate-limit --disable-gpu-vsync` で起動し、ページの rAF を数える。`achievedFps` は各シナリオの rAF の 1 秒あたりの平均(全シナリオの合計)。
- 方法 (i) も試した(headless shell の `--enable-begin-frame-control` と `--run-all-compositor-stages-before-draw`、`HeadlessExperimental.beginFrame` を 8.333 ms 間隔)。3840 x 2160 では 1 秒に 1 フレーム程度(1460 フレームを 1492 秒)で、入力の応答も止まり、起動のシナリオが 25 分止まった。このため (ii) にした。`--clock begin` は残してある。

## 注・既知の限界

- Escape で Welcome を閉じると、Welcome の選んでいる項目のプロジェクトが開く。Default の行を名前で選ぶ手段が画素には無いため、この経路を使う。表示によっては開くプロジェクトが違う(5 チャンネルの 808 のプロジェクトを観測)。その場合、(b) のノブの位置が合わない。
- Ctrl+O は File > Open の割り当てで、メニューの項目を文字列で選ぶ `menuPath` は作っていない(メニューの文字は画素からしか読めないため)。
- `wrangler dev --local` は br の本体を gzip に変換して返す(観測)。そのため `Content-Encoding` は Worker を直接呼んだ応答で確かめ、配信の応答では `wireContentEncoding` として出すだけにしている。
- 「消えない表示」は、`#status`・`#audio-status`・`#taskbar` を除く `role=status`・`role=alert`・`aria-live` の要素で数える(この 3 つは常に画面にある)。
- 一時ファイルは `.tmp-e2e-*` に置き、終わると消す。Chromium の本体は Playwright の既定の場所(リポジトリの外)に入る。
