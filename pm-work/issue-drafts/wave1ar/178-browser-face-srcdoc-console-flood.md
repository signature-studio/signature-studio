---
draft: "178"
title: "Browser の面の iframe(`about:srcdoc`)が、『Blocked script execution』の console エラーを、2 万件以上出し続ける: 原因を直す"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: []
labels: ["resources", "render", "verify"]
size: S
priority: Urgent
project: signature-studio/3
---

## 目的

Browser の面(WebView2 の iframe)が、サンドボックスの中で script を動かそうとして、`Blocked script execution in 'about:srcdoc'` の console エラーを、大量に出し続けている。#{{165}} の E2E で、DPR 1 では 21,758 件、DPR 2 では 8,385 件が出た。利用者の機械でも同じ物が出ている(本番)。console の洪水は、描画(P2)と音(P3)の負荷になり、ブラウザの作業も遅くする。

この Issue の目標は次の 2 つ。
1. Browser の面を開いて操作しても、`console.error` が 0 件(DPR 1 と DPR 2 の両方)。
2. 面の見た目と操作(オフラインのページの表示・Sounds タブの前後関係)を変えない。

## 前回の試行で分かったこと(run 45593572502)

数値は runner の値(nproc 4、loadavg 1.72 から 1.81)。この機械とは別枠。前回の差分(commit `d9924332`、未 push)は `.prior-work.md` の取り込みで作業ツリーに入る。
- 直した点(`app/src/ui/webview2-surface.ts`): `withoutScripts(html)` を足した。面に入れる文書から `<script>` 要素と、タグの `on*=` 属性を取り除く。`navigate()` で `rewriteRefs` の前に適用する。`sandbox` は `allow-same-origin` のまま(`allow-scripts` は足していない)。単体テストを 1 件足した(`app/test/unit/webview2-face.test.ts`)。
- 特定(実測): 対象は `iframe[data-fl-webview2]` の 1 件(`ensure()` で作られ、クラウドのタブの `Navigate` で文書が入る)。`Navigate` は、タブを押した回数と 1 対 1(3 回。DPR 1 と DPR 2 で同じ)。面の読み込みも 3 回。再投入のループは見つからなかった。
- 代用の頁での実測(`<script>` 2 個と `onclick` 1 個を含む自作の頁を、bundle に一時的に入れた): 読み込みごとに `Blocked script execution in 'about:srcdoc'` が 3 件。3 回の読み込みで、DPR 1 は 9 件、DPR 2 は 9 件。修正後は、DPR 1 と DPR 2 とも 0 件(読み込み 3 回)。`script` と属性のどちらが何件かは分けていない。
- 代用の頁の見た目(「You're offline」・本文・リンク)は、修正の前後で同じ位置・配色だった(画面で確認)。
- DPR 2 の集合の座標: 前回は、タブの座標を CSS 座標のまま押していたので、DPR 2 では `Navigate` が 0 回だった。`guestPerCss()` で換算して直した(`dismissProblems` と同じ方法)。
- 実物の頁: 前回の bundle には `error-offline.html` が入らなかった(`resources` ラベルが無く、`host-extra` が無い)。そのため面は空の文書で、console エラーは 0 件(DPR 1 と DPR 2)。この条件では、browser-face の 0 件は頁が無いため自明に成り立つ。実物の頁の中身は未確認。
- #{{165}} の R11 の 21,758 件(DPR 1)・8,385 件(DPR 2)は、この集合では再現しなかった。#1362 の記録は「スクリプト無し」としていて、#{{165}} の数と合わない。
- 推定: 実物の頁に `script` または `on*` 属性があり、読み込みごとに Blocked が出る。R11 の件数は、言語切り替えによる再投入の回数に関係するかもしれない(未確認)。
- 起動の計測(engine と bundle を作り直した直後): boot の 1 回目は `elapsedMs` 17,714 ms(上限 12,500 を超えた。`error` は null、state は engine-idle、standIn は false)。2 回目は 11,494 ms(上限内)。流れ A は 11,660 ms、B は 11,888 ms、C は 11,643 ms、D は 11,310 ms。`preBootBytes` は 312,222,067。1 回目は、bundle を作った直後の冷えた状態と見られる(推定)。
- boot-bundle は、この受け入れでは 2 回続けて走らせ、2 回目を判定に使う。1 回目は、冷えた状態の時間を見るだけで、判定には使わない。
- smoke(DPR 1)の `file-open-demo`: DEDZII の読み込みで engine が終了コード 134 で止まる(`fle: indirect call to 0xb2844c0 is not a known function entry`)。変更前の HEAD でも同じ停止を 3 回再現した。この変更とは無関係。#1421 の範囲で、main の bc740423 で印が付いた。
- browser-face(チェーンの外で、同じ bundle で単独に実行): 4k-dpr1・4k-dpr2 とも `ok`、consoleErrors 0、pageErrors 0、failedRequests 0、stuckToasts 0、engine-idle、faces 1、faceLoads 3、navigateLines 3、fireLines 16。
- `npm test --prefix app`: Test Files 73 passed・1 skipped(74)、Tests 1433 passed・10 skipped(1443)。
- ビルド: engine と bundle の作り直しは約 50 分。
- 未確認: 実物の頁の中身。R11 の件数の内訳。起動の 17.7 s と 11.5 s の差(冷えた状態と推定)。受け入れの全体(smoke の停止は #1421 の印で通るかどうか)は、次の試行で初めて確かめる。
- この Issue に `resources` ラベルを付けた。この試行の未確認の 1 つ目を解くため(bundle に実物の頁 `error-offline.html` が入る)。

## 背景

- 事実 1(#{{165}} の E2E の報告): R11(言語の切り替え)の実行中に、`console.error` が DPR 1 で 21,758 件、DPR 2 で 8,385 件出た。全て `Blocked script execution in 'about:srcdoc' because the document's frame is sandboxed and the 'allow-scripts' permission is not set.` の形。
- 事実 2(コード): `app/src/ui/webview2-surface.ts` の面は、iframe に `sandbox="allow-same-origin"` を付ける(197 行付近)。`navigate()` が `srcdoc` に文書を入れ(264 行付近)、`ensure()` が `srcdoc = ''` を入れる(205 行付近)。文書の `<script>` は、サンドボックスで実行できないため、読み込みのたびに、この error を出すと推定する(推定。実測で確かめる)。
- 事実 3(設計の前提): 面の冒頭の注釈(`webview2-surface.ts` の 1〜5 行)は、面は自分の script を動かさない、と書いている。つまり、面の文書に `<script>` は要らない。
- 禁止: `allow-scripts` と `allow-same-origin` を同時に付けない。両方が揃うと、サンドボックスが無効になり、面の中の script が親のページに届く(安全の境界の喪失)。
- 関連: #1362(面の作成。閉じている)・#1376(Sounds タブの面と、FL の窓の前後関係)。

## 読む物

- `app/src/ui/webview2-surface.ts`(面の作成・`navigate`・`install`・`onLoad`・`place`)
- `grep -rn 'srcdoc\|sandbox\|allow-scripts' app/src` の全件
- `lift/runtime/fle_webview2.c`(FL の WebView2 の model。Navigate の呼び出しの回数)
- `tools/browser-panel/measure.mjs`(面の iframe を数える道具)
- `tools/e2e/lib/detect.mjs`(`consoleErrors` の数え方)・`tools/e2e/run.mjs`(行の判定。main の `rowPasses`)・`tools/e2e/judge.jq`(deploy の門)
- `tools/boot-check/boot-bundle.mjs`(起動の確認)
- `pm-work/findings/`(#1362 の記録)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。前回の差分(上の「直した点」)が作業ツリーに在れば、それを土台にする。
1. **特定**: Browser の面の、どの iframe が console エラーを出すかを特定する(`data-fl-webview2` の iframe の数と、`srcdoc` の中身)。何件出るか、1 回の Navigate で何件か、Navigate が何回呼ばれるかを、ログで数える。原因(文書の `<script>` を毎回実行しようとする/面を作り直す/同じ文書を何度も入れる、のどれか)を、実測で特定し、報告に書く。
2. **直す**(既定の動作として。オプトインを作らない):
   - 面の文書が script を動かす必要が無いなら、面に入れる文書から `<script>` を取り除く(`splitStyles` の段)。
   - 面の作り直し・同じ文書の再投入が原因なら、その原因を直す(`navigate` の世代の扱い、`ensure` の `srcdoc = ''`)。
   - `sandbox` に `allow-scripts` を足さない。
3. **受け入れの道具**: Browser の面を開き、タブを切り替えて 10 秒待ち、`consoleErrors` を数える E2E のシナリオを 1 本足す。`tools/e2e/scenarios/browser-face/`(新しい集合)に 1 ファイルを置き、`index.mjs` で集合を作る。`tools/e2e/run.mjs` の行の判定は、集合の件数で数える。`tools/e2e/judge.jq`(deploy の門)は smoke の 3 件を見るので、smoke の集合は変えない。
4. **実物の頁**: `resources` ラベルで bundle に実物の頁 `error-offline.html` が入ったかを、報告に書く。入っていなければ、「頁なし」と書き、browser-face の 0 件は頁が無いため自明に成り立つと明記する。
5. 報告に、特定した iframe と、console エラーの件数の変更の前後(DPR 1 と DPR 2)を書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot0.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"?0:1)' "$RUNNER_TEMP/boot0.json" && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&Math.max(...L.map((r)=>r.elapsedMs))<=12500&&Math.max(...L.map((r)=>r.preBootBytes))<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && bash tools/e2e/run.sh --scenarios smoke --displays 4k-dpr1 > "$RUNNER_TEMP/e2e-smoke.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L[0].servedHeadersMatch===true&&L[0].scenarios.length===3&&L[0].scenarios.every((s)=>s.consoleErrors===0&&s.pageErrors===0&&s.failedRequests===0)?0:1)' "$RUNNER_TEMP/e2e-smoke.jsonl" && bash tools/e2e/run.sh --scenarios browser-face --displays 4k-dpr1,4k-dpr2 > "$RUNNER_TEMP/e2e-face.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===2&&L.every((r)=>r.servedHeadersMatch===true&&r.scenarios.length>=1&&r.scenarios.every((s)=>s.ok===true&&s.consoleErrors===0&&s.pageErrors===0&&s.failedRequests===0&&s.stuckToasts===0))?0:1)' "$RUNNER_TEMP/e2e-face.jsonl"
```

判定の基準(全て満たす):
- 起動確認: 1 回目(`boot0.json`)は、`error` が null・state が engine-idle だけを見る(判定には使わない)。2 回目(`boot.json`)が engine-idle・standIn が false・elapsedMs が 12,500 以下。boot-bundle は、この受け入れでは 2 回続けて走らせる。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- smoke の 3 件(DPR 1)で、`consoleErrors`・`pageErrors`・`failedRequests` が全て 0。`file-open-demo` は印 #1421 の付いた既知の欠陥で、`ok` は問わない(`run.sh` の終了コードは main の `rowPasses` の規則で決まる)。
- browser-face の集合(DPR 1 と DPR 2)で、全シナリオが `ok` かつ `consoleErrors` が 0。
- `bash guard/scan.sh` が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 特定した iframe・原因(実測と推定を分ける)、変えた箇所、console エラーの件数の変更の前後(DPR 1 と DPR 2)、実物の頁が bundle に入ったかどうか、流れ A〜D の 4 行、smoke と browser-face の JSON の要点、boot の 2 回の `elapsedMs`。

## 制約

- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- `allow-scripts` と `allow-same-origin` を同時に付けない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は、条件(要素・イベント)で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。

## 他のタスクとの関係

- blocked_by は無い。
- #{{165}}(E2E のシナリオ集)の R11 は、`rowPasses` が印の有無に関わらず `consoleErrors` 0 を求めるので、この Issue が直るまで R11 の行は落ちうる。#{{165}} の R11 の印(`#1419`)は、この Issue が直ったら外す。この Issue の受け入れに R11 の `consoleErrors` を足すのは、#{{165}} が main に入った後(本文に書く。今回の受け入れにはまだ足さない)。
- #1421(smoke の file-open-demo の停止)は、main の bc740423 で印が付いた。この Issue の受け入れの smoke は、その印を前提にする。#1421 が直ったら、その印を外す(#1421 の範囲)。
- #1376(Sounds タブの面の前後関係)と、`app/src/ui/webview2-surface.ts` で衝突しうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
