# #1419 Browser の面の console エラー(記録)

実エンジン: この runner で、受け入れのコマンドを段ごとに通した。`npm run build`・`cargo build --release --locked`・`fe-build engine`・`fe-build bundle` は終了コード 0(`error-offline.html` は lazy 44,729 B で bundle に入った)。boot-bundle は 2 回とも終了コード 0。流れ A〜D は 4 行とも通った。`npm test --prefix app` は 75 passed | 1 skipped・1465 passed | 10 skipped。smoke(DPR 1)は `rows 1, accepted true`(終了コード 0)、`jq -e -s -f tools/e2e/judge.jq` は終了コード 0。browser-face(DPR 1 と DPR 2)は `rows 2, accepted true`(終了コード 0)。`guard/scan.sh` は 0 件。修正の前の版(HEAD の `app/src/ui/webview2-surface.ts`)も同じ集合で測った(下の 4)。

runner の値(この機械とは別枠): nproc 4。loadavg は browser-face の後の時点で 1.17・2.02・1.98(1 分・5 分・15 分の順)。

## 1. 特定

- 対象: `iframe[data-fl-webview2]` の 1 件。`ensure()` で作られ、クラウドのタブの `Navigate` で文書が入る。タブを 3 回押す(6 回の押下)と、`Navigate` は 3 回、面の読み込みは 3 回(DPR 1 と DPR 2 で同じ)。再投入のループは無い。
- 実物の頁: `System/Internet/error-offline.html`(44,729 B。bundle の lazy 資源。sha256 0b8daf3c…)。`<script>` は無く、`on*` 属性も `javascript:` も無い。インラインの `style` 属性が 1 件(svg の `enable-background`)と、`@font-face` の `data:` フォントが 1 件。
- 修正の前の console エラー(browser-face の 3 回の読み込み。`details` の実物): DPR 1・DPR 2 とも 6 件。内訳は `Applying inline style violates the following Content Security Policy directive 'style-src 'self''` が 3 件と、`Loading the font 'data:font/truetype…' violates …` が 3 件。どちらも頁の CSP で拒まれる。`Blocked script execution` は 0 件。pageErrors と failedRequests は 0。

## 2. 原因(実測)

- 実物の頁の 2 つの記述(インラインの style 属性と `data:` フォント)が、面の文書に継がれる CSP(`style-src 'self'`・`font-src 'self'`)で拒まれ、読み込みのたびに error を出す。srcdoc の文書は親の CSP を継ぐ。
- 前回の推定「実物の頁に script がある」は、この頁については成り立たない。実物の頁に `<script>` は無い。
- #1408 の R11 の `Blocked script execution` の洪水(DPR 1 で 21,758 件、DPR 2 で 8,385 件)は、この集合では再現しない(修正の前も 0 件)。R11 の経路(言語の切り替えで面に入る文書)は未確認。

## 3. 直し方(`app/src/ui/webview2-surface.ts`)

- `withoutCspBlocked`: 文書から、CSP で拒まれる style 属性と `data:` フォントを取り除く。この 2 つは修正の前も効いていなかったので、見た目は変わらない(7 を見る)。
- `withoutScripts`: 文書から `<script>` と `on*` 属性を取り除く。実物の頁には無い。防御として残す(script を持つ文書が面に入った時、`Blocked script execution` を出さないため)。単体テストの例に、その形(`<Script language>` と `onSubmit`)を足した。
- `navigate()` で、`rewriteRefs` の前に両方を適用する。
- `sandbox` は `allow-same-origin` のまま。`allow-scripts` は足していない。`allow-scripts` と `allow-same-origin` を同時に付けていない。
- `ensure()` の `srcdoc = ''` と、`navigate` の世代の扱いは変えていない(再投入の原因が無いため)。

## 4. 件数の変更の前後(browser-face の 3 回の読み込み。同じ集合・同じ runner・同じ run)

| 版 | DPR 1(consoleErrors・pageErrors・failedRequests) | DPR 2 |
| --- | --- | --- |
| HEAD(修正の前。両方の関数が無い。`vite build` だけで dist を作った。型検査は、テストが新しい名前を import するため飛ばした) | 6・0・0 | 6・0・0 |
| この作業(両方の関数が有る) | 0・0・0 | 0・0・0 |

前回の試行の値も同じ(6・6 → 0・0)。

## 5. 受け入れの値(この作業の最後の版。16:03 頃の run)

- boot(`tools/boot-check/boot-bundle.mjs`): 1 回目 `elapsedMs` 8,160(`coldMs` 9,145。判定に使わない)。2 回目 `elapsedMs` 8,110(上限 12,500 の内)。`error` null・`state` engine-idle・`standIn` false。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`): A 7,700 ms・B 7,637 ms・C 7,628 ms・D 7,594 ms。4 行とも `error` null・engine-idle・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true。B と D は `welcomeClosed` true。`preBootBytes` は 4 行とも 313,279,997。
- `npm test --prefix app`: Test Files 75 passed | 1 skipped (76)。Tests 1465 passed | 10 skipped (1475)。
- smoke(`--scenarios smoke --displays 4k-dpr1`): 10 件のうち 9 件が ok true で、4 カウンタ 0・engine-idle・stops 0。`plugin-manager-opens`(印 #1499)は ok false だが、印の規則で通る(`rowPasses`)。`file-open-demo` は ok true・engine-idle(印 #1421 は外さない)。
- browser-face(`--scenarios browser-face --displays 4k-dpr1,4k-dpr2`): 2 行とも `servedHeadersMatch` true・`error` null。シナリオは ok true・`consoleErrors` 0・`pageErrors` 0・`failedRequests` 0・`stuckToasts` 0・engine-idle。`presses` 6・`facesAtOpen` 1・`facesAtEnd` 1・`faceLoads` 3・`navigateLines` 3・`fireLines` 16。`navigateUris` は 3 件とも `file://…\System\Internet\error-offline.html`。
- `bash guard/scan.sh`: 0 件。

## 6. 実物の頁が bundle に入ったか

- 入った。`lift/out-app/bundle/raw/fl-Oz/manifest.json` の `lazy` に `resources/System/Internet/error-offline.html`(44,729 B・sha256 0b8daf3c…)が在る。`resources` ラベルで `lift/out-app/host-extra/resources/System/Internet/error-offline.html` が置かれ、`fe-build` の `OFFLINE_PAGE_SRC` がそれを読む。

## 7. 画面の前後(見た目と操作)

- 前回の試行で、同じ集合の `browser-face-tabs` の最後の画面(`--shots`)を、修正の前後で画素ごとに比べた(Chromium の canvas で数えた)。この run では画素の比較をしていない。
- DPR 1: 差分 9,785 画素、bbox x 3509〜3813・y 208〜1005。DPR 2: 差分 2,320 画素、bbox x 437〜1873・y 45〜1034。
- 差分の bbox は、どちらも面(左の列。「You're offline」の箱が在る所)の外。右の表示パネル(ログ・時刻)と FL の窓の時刻・表示の変化による。面の領域の画素は、前後で一致した。
- 推定の域を出ない点: 差分が FL の表示の時刻・ログに由来することは、画面を見て推定した。
- 見た目が変わらない理由(推定): 取り除いた style 属性と `data:` フォントは、修正の前も CSP で拒まれ、効いていなかった。

## 8. 作業ツリーの注意

- `lift/out-app/host/resources` は、`agent.yml` の「取った物を lift/out-app/host-extra/resources と lift/out-app/host/resources に置く」の段で置かれる。この run では手で複製する必要は無かった。

## 9. 未確認

- #1408 の R11 の件数(DPR 1 で 21,758、DPR 2 で 8,385。`Blocked script execution`)の経路。この集合と bundle の面に入る文書は `System\Internet\error-offline.html` だけで、R11 の行が main に入った後に確かめる。
- 言語の切り替え(再読込)の中での面の件数。
- deploy の E2E(`.github/workflows/e2e.yml`)の matrix は smoke だけを回す。browser-face はまだ門に入っていない(Issue の範囲外)。

## 10. 取り込み(衝突の解消)

- main(4a607c54)の `--report-only`・`runPasses`・`beforeLoad` と、この作業の `--scenarios <集合>`(browser-face)を両方残した。`legacyScenario(s, set = 'smoke')` が集合名を受け、`beforeLoad` を保つ。`run.mjs` の用法は `smoke | browser-face | --area` と `--report-only` の両方を載せる。
