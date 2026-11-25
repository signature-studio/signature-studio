---
draft: "164"
title: "E2E の基盤: GitHub Actions 上で、4K(3840×2160)・120 Hz のブラウザを Playwright で動かし、本番と同じ配信に対して、ユーザーの操作と権限を試し、停止・例外・消えない表示を検出する"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: []
labels: ["verify", "render"]
size: XL
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の指示(原文): 「あなたが E2E を実施する必要がある、という点を理解できていますか？ Playwright でも何でも良いので、GitHub Actions 上で 4K 120Hz のブラウザを動かして E2E を実行する必要があります。」

この Issue は段階 1 で、E2E の基盤を作る。
1. GitHub Actions の作業者の run の中で、4K(3840×2160)・120 Hz を模したブラウザ(Playwright の Chromium)を動かす。
2. 本番と同じ配信(Worker のコード・ヘッダー・bundle・assets)に対して、実際のマウス・キーの操作を行う。
3. 停止・例外・消えない表示・失敗した要求・意図しない権限の確認を検出する。
4. 最小のシナリオ 3 本が、4 つの表示で通る。

シナリオ集(依頼者が報告した操作の全件)は、#{{165}} で、この基盤の上に足す。

## 背景

- 今の検査は、自前の CDP の道具(`tools/smoke/welcome-flow.mjs`・`tools/clipboard/measure.mjs` など)で、起動と個々の操作を測っている。ユーザーの操作の網羅、権限、表示が消えるか、4K、120 Hz は、どれも検査していない。そのため、依頼者が実機で見つけた不具合(例: ノブの右クリックで出る権限の表示が消えない。#{{163}})を、今の検査は捕まえられなかった。
- 本番のサイトは、アクセス制御の裏にあり、作業者の run からは入れない。そこで、本番と同じ配信を、作業者の run の中で立てる。本番の認証の設定の値は、リポジトリに書かない。
- 作業者の run は GitHub Actions の Linux の runner。GPU は無い。4K・120 Hz の画面は、仮想ディスプレイとブラウザのフレームの制御で模す。模せない時は、模せなかったことを数で出す(`achievedFps`)。隠さない。
- 道具は Node.js と Playwright で書く。Python は使わない。
- 既存の部品: `tools/boot-check/boot-bundle.mjs`・`tools/boot-check/cdp.mjs`・`tools/boot-check/stage-assets.mjs`(起動と CDP)、`window.__app.deskDumpBase64(peek)`(FL の窓の表と画素。窓の座標の取得に使う)、`app/src/perf/beacon.ts`(実機のビーコンの項目)。

## 読む物

- `app/worker/index.ts`(Worker の応答・`baseHeaders`・`/assets/*` の配信・`run_worker_first`)・`app/worker/access.ts`(アクセス制御の検証と、テスト用の設定の入口 `ACCESS_CERTS_URL`)・`app/shared/headers.ts`(`ISOLATION_HEADERS`・`PRIVACY_HEADERS`・`CSP`・`cacheControlFor`・`mimeFor`)・`app/wrangler.jsonc`(変数の名前。値は書かない)
- `.github/workflows/deploy.yml`(本番の配信。bundle と assets を置く手順)
- `tools/boot-check/boot-bundle.mjs`・`tools/boot-check/cdp.mjs`・`tools/boot-check/stage-assets.mjs`
- `tools/smoke/welcome-flow.mjs`(起動と Welcome の閉じ方)・`tools/feat-c6-popup-menus/measure.mjs`(窓の表と、メニューの文字列の選び方)・`tools/clipboard/measure.mjs`(権限の設定の仕方)
- `app/src/main.ts`(`window.__app` の API の全体)・`app/src/perf/beacon.ts`(既存の計測の項目)

## やること

1. 道具の場所: `tools/e2e/`(新規)。`package.json`(Playwright の版を固定)と `package-lock.json`(commit する)・`run.sh`(入口)・`lib/`(操作の部品と検出)・`scenarios/`(シナリオ)・`serve/`(配信)。
   - 取得: `(cd tools/e2e && npm ci && npx playwright install chromium)`。依存の取得(`--with-deps`)は root が要るので、`sudo -n true` で使えるかを確かめ、使えなければ、ブラウザの取得だけにする。取得は、一時の失敗に強くする(再試行)。
   - 取得が runner で動かない時は、エラーの全文を報告に書く(この Issue はそこで止まる)。

2. 画面(4 つの表示。`run.sh --displays` の名前で選ぶ)。物理の画面は、どれも 3840×2160 の画素。
   - `4k-dpr1`: ページの大きさ 3840×2160 CSS px・DPR 1。
   - `4k-dpr2`: ページの大きさ 1920×1080 CSS px・DPR 2(4K の画面の拡大 200%)。
   - `4k-random`: 1.01〜3.00 の乱数の DPR を 2 つ選び、それぞれ 1 行にする(ページの大きさは 3840 ÷ DPR、2160 ÷ DPR の CSS px)。乱数の値は JSON の `dpr` に出す。
   - 仮想ディスプレイは 3840×2160 にする(`xvfb-run -s '-screen 0 3840x2160x24'` の headed の Chromium。または、headless の新しい形式で `--window-size=3840,2160`)。ページの大きさと DPR は `Emulation.setDeviceMetricsOverride` で指定する。

3. 120 Hz。候補は次の 2 つ。実測で動く方法を選び、理由を報告に書く。
   - (i) headless の shell を、`--enable-begin-frame-control` で起動し、CDP の `HeadlessExperimental.beginFrame` を、8.333 ms 間隔で実時間に合わせて送る。
   - (ii) `--disable-frame-rate-limit`・`--disable-gpu-vsync` で rAF の上限を外し、実際に作れたフレームの数を測る。
   - 測る値: `achievedFps`(シナリオの実行中の、rAF の 1 秒あたりの回数の平均。5 秒以上の標本)。120 Hz に届かなければ、届いた値をそのまま出す。`refreshHzTarget` は 120 と出す。FL が見るリフレッシュレート(`EnumDisplaySettings`)を、この実測の間隔に結ぶのは #1402 の仕事で、この Issue では測るだけにする。

4. 本番と同じ配信。
   - (a) 本番の Worker(`app/worker/index.ts`)を `wrangler dev --local` で立てる。`/assets/*` の中身は、ローカルの R2 に、同じキーで入れる。アクセス制御の検証は、テストの道具の中で、テスト用のチーム名・AUD・署名の鍵を作り、鍵の配布(`ACCESS_CERTS_URL` が指す、ローカルの JWKS)と、テスト用の JWT の署名で通す。製品の設定の値は書き換えない。wrangler の変数の指定は、テストの道具の起動の引数に限る。
   - (b) (a) が立たない時は、同じ Worker のコード(`app/worker/index.ts` の応答)を、ローカルの HTTP サーバーから呼んで返す。
   - どちらを選んだかを報告に書く(JSON の `delivery` に `a` か `b` を出す)。
   - `servedHeadersMatch`: 配信の応答のヘッダーが、`app/shared/headers.ts` の `ISOLATION_HEADERS`・`PRIVACY_HEADERS`・`CSP` の値と全て一致する。`/assets/*` の `Content-Encoding` と `Cache-Control` が、`cacheControlFor` と `app/worker/index.ts` の規則と一致する。

5. 操作の部品(`tools/e2e/lib/`)。実際のマウス・キーのイベントを送る(CDP の `Input.dispatchMouseEvent`・`Input.dispatchKeyEvent`、または Playwright の `page.mouse`・`page.keyboard`)。ページの関数を直接呼ばない。
   - `click`・`rightClick`・`dblclick`・`key`(Ctrl+V など)・`drag`・`wheel`。
   - `menuPath(['File','Open'])`: メニューの文字列の階層で項目を選ぶ(`tools/feat-c6-popup-menus/` の手順を再利用)。
   - `grantPermissions`・`denyPermissions`・`resetPermissions`・`setClipboard`・`readClipboard`(CDP の `Browser.*`)。
   - 座標は固定にしない。FL の画面の座標は、`window.__app.deskDumpBase64(true)` の窓の表と画素から取る。

6. 検出(各シナリオで全て数える)。
   - ページの `pageerror`・console の `error`・`requestfailed`・HTTP の 4xx と 5xx。
   - エンジンの状態: `running` のまま 20 秒・`exited`・`error`。停止の理由は、エンジンの終了の報告(`engine.exit.stop` の内容)で読む。
   - 消えない表示: `role="status"`・`role="alert"`・`[aria-live]` の要素が、可視のまま 10 秒を超えて残る(`stuckToasts`)。
   - 意図しない権限の確認: 操作の前後で `navigator.permissions.query` の状態が変わった(`prompt` のまま、または `denied` のままのはずの操作で、変わった)。

7. 最小のシナリオ(`tools/e2e/scenarios/smoke/`)。各シナリオは、`name`・`ok`・`expectFail`(印。無ければ null)と、検出の数を返す。
   - (a) `start-welcome-default`: 起動して、確認ダイアログを「No thanks」で閉じ、Welcome の Default を開く。エンジンが engine-idle になり、プロジェクトが開く。
   - (b) `knob-right-click-permissions`: Channel Rack の Volume のノブを右クリックする。権限の 3 通り(`prompt`・`denied`・`granted`)で試す。FL の文脈メニューが出る。右クリックの後、`navigator.clipboard.readText`・`read` の呼び出しは 0 回。表示(`role="status"` の要素)は可視にならない。この 1 本は `expectFail: '#{{163}}'`(表示が消えない不具合の検出。#{{163}} が直るまで失敗する。直ったら印を外す)。
   - (c) `file-open-demo`: File > Open の経路で、インストーラの評価用の demo(曲)を 1 つ開く。エンジンが止まらず、プロジェクトが開く。

8. 出力。stdout に、表示ごとに 1 行の JSON(表示の数 4 行)。形は次の通り。
   `{"display":{"width":3840,"height":2160,"dpr":1,"refreshHzTarget":120,"achievedFps":0},"delivery":"a","scenarios":[{"name":"...","ok":true,"expectFail":null,"pageErrors":0,"consoleErrors":0,"stuckToasts":0,"failedRequests":0,"engineState":"engine-idle"}],"servedHeadersMatch":true}`
   - `display.width`・`height` は物理の画素(3840・2160)。`dpr` は表示の DPR。`achievedFps` は数(0 でも数値)。
   - 終了コード: 全ての行が受け入れの判定を満たした時だけ 0。JSON は、失敗した時も出す(診断のため)。
   - 途中の画像・ログは `.tmp-e2e/` に置き、終わったら消す(commit しない)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && (cd tools/e2e && npm ci && npx playwright install chromium) && bash tools/e2e/run.sh --scenarios smoke --displays 4k-dpr1,4k-dpr2,4k-random > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const okS=(s)=>s.ok===true&&s.pageErrors===0&&s.consoleErrors===0&&s.stuckToasts===0&&s.failedRequests===0;process.exit(L.length===4&&L.every((r)=>r.servedHeadersMatch===true&&r.display.width===3840&&r.display.height===2160&&r.display.refreshHzTarget===120&&typeof r.display.achievedFps==="number"&&r.scenarios.length===3&&r.scenarios.every((s)=>s.expectFail!==null||okS(s)))?0:1)' "$RUNNER_TEMP/e2e.jsonl"
```

判定の基準:
- `run.sh` の終了コードが 0。JSON の行が 4 件(`4k-dpr1`・`4k-dpr2`・`4k-random` の 2 行)。
- 各行で、`display` の幅と高さが 3840 と 2160、`refreshHzTarget` が 120、`achievedFps` が数。`servedHeadersMatch` が真。シナリオが 3 本。
- 印(`expectFail`)の無いシナリオは全て `ok` が真で、`pageErrors`・`consoleErrors`・`stuckToasts`・`failedRequests` が 0。印の付いたシナリオ(b)は、結果を報告に書く(失敗は、検出が働いたことの証拠になる)。
- 取得(`npm ci`・`npx playwright install chromium`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 配信の方式(`a` か `b`)、120 Hz の方式(i か ii)、`achievedFps` の値を、そのまま書く。
- 落ちた時は、JSON の行と、失敗のエラーの全文(取得の失敗は全文)。既存の Issue の不具合に当たった時は、その番号を書く。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品(`app/`・`lift/`)は、この Issue では変えない(読むだけ)。必要な窓の表・計測が既存の API に無い時は、報告に書いて止める。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。配信のテストの設定は、テストの道具の中で完結させる。
- `.github/` は変えない。
- ベンダー名、Cloudflare の識別子(アカウント・バケット・ホスト名・アクセス制御のチーム名と AUD の値)を、ファイルに書かない。
- Python を使わない。OCR を使わない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は、条件(要素・イベント・フレーム)で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- 順序: (1) 取得と配信を先に動かす(Playwright・Chromium・本番と同じ配信で、起動の 1 回まで)。(2) 操作の部品と検出。(3) 3 本のシナリオ。(4) 4 つの表示。最初の実行は、開始から 2 時間 30 分までに終える。
- engine の作り直しは約 35 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: なし。
- #{{165}}(シナリオ集)は、この Issue が通った後に、この基盤の上にシナリオを足す。
- #{{163}}(クリップボードの問い合わせ)は並行で入る。(b) の印は、#{{163}} が直ったら外す。
- #1377(親)の実機のビーコンとは、別の道具(ビーコンを読まない。ブラウザを動かして検査する)。
