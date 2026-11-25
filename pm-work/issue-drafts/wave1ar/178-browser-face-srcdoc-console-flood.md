---
draft: "178"
title: "Browser の面の iframe(`about:srcdoc`)が、『Blocked script execution』の console エラーを、2 万件以上出し続ける: 原因を直す"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: []
labels: ["verify", "render"]
size: S
priority: Urgent
project: signature-studio/3
---

## 目的

Browser の面(WebView2 の iframe)が、サンドボックスの中で script を動かそうとして、`Blocked script execution in 'about:srcdoc'` の console エラーを、大量に出し続けている。#1408 の E2E で、DPR 1 では 21,758 件、DPR 2 では 8,385 件が出た。利用者の機械でも同じ物が出ている(本番)。console の洪水は、描画(P2)と音(P3)の負荷になり、ブラウザの作業も遅くする。

この Issue の目標は次の 2 つ。
1. Browser の面を開いて操作しても、`console.error` が 0 件(DPR 1 と DPR 2 の両方)。
2. 面の見た目と操作(オフラインのページの表示・Sounds タブの前後関係)を変えない。

## 背景

- 事実 1(#1408 の E2E の報告): R11(言語の切り替え)の実行中に、`console.error` が DPR 1 で 21,758 件、DPR 2 で 8,385 件出た。全て `Blocked script execution in 'about:srcdoc' because the document's frame is sandboxed and the 'allow-scripts' permission is not set.` の形。
- 事実 2(コード): `app/src/ui/webview2-surface.ts` の面は、iframe に `sandbox="allow-same-origin"` を付ける(197 行付近)。`navigate()` が `srcdoc` に文書を入れ(264 行付近)、`ensure()` が `srcdoc = ''` を入れる(205 行付近)。文書の `<script>` は、サンドボックスで実行できないため、読み込みのたびに、この error を出すと推定する(推定。実測で確かめる)。
- 事実 3(設計の前提): 面の冒頭の注釈(`webview2-surface.ts` の 1〜5 行)は、面は自分の script を動かさない、と書いている。つまり、面の文書に `<script>` は要らない。
- 禁止: `allow-scripts` と `allow-same-origin` を同時に付けない。両方が揃うと、サンドボックスが無効になり、面の中の script が親のページに届く(安全の境界の喪失)。
- 関連: #1362(面の作成。閉じている)・#1376(Sounds タブの面と、FL の窓の前後関係)。

## 読む物

- `app/src/ui/webview2-surface.ts`(面の作成・`navigate`・`install`・`onLoad`・`place`)
- `grep -rn 'srcdoc\|sandbox\|allow-scripts' app/src` の全件
- `lift/runtime/fle_webview2.c`(FL の WebView2 の model。Navigate の呼び出しの回数)
- `tools/browser-panel/measure.mjs`(面の iframe を数える道具)
- `tools/e2e/lib/detect.mjs`(`consoleErrors` の数え方)・`tools/e2e/run.mjs`(行の判定)・`tools/e2e/judge.jq`(deploy の門)
- `pm-work/findings/`(#1362 の記録)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. **特定**: Browser の面の、どの iframe が console エラーを出すかを特定する(`data-fl-webview2` の iframe の数と、`srcdoc` の中身)。何件出るか、1 回の Navigate で何件か、Navigate が何回呼ばれるかを、ログで数える。原因(文書の `<script>` を毎回実行しようとする/面を作り直す/同じ文書を何度も入れる、のどれか)を、実測で特定し、報告に書く。
2. **直す**(既定の動作として。オプトインを作らない):
   - 面の文書が script を動かす必要が無いなら、面に入れる文書から `<script>` を取り除く(`splitStyles` の段)。
   - 面の作り直し・同じ文書の再投入が原因なら、その原因を直す(`navigate` の世代の扱い、`ensure` の `srcdoc = ''`)。
   - `sandbox` に `allow-scripts` を足さない。
3. **受け入れの道具**: Browser の面を開き、タブを切り替えて 10 秒待ち、`consoleErrors` を数える E2E のシナリオを 1 本足す。`tools/e2e/scenarios/browser-face/`(新しい集合)に 1 ファイルを置き、`index.mjs` で集合を作る。`tools/e2e/run.mjs` の行の判定は「シナリオが 3 件」を前提にしているので、件数を集合の件数に変える。`tools/e2e/judge.jq`(deploy の門)は smoke の 3 件を見るので、smoke の集合は変えない。
4. 報告に、特定した iframe と、console エラーの件数の変更の前後(DPR 1 と DPR 2)を書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&Math.max(...L.map((r)=>r.elapsedMs))<=12500&&Math.max(...L.map((r)=>r.preBootBytes))<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && bash tools/e2e/run.sh --scenarios smoke --displays 4k-dpr1 > "$RUNNER_TEMP/e2e-smoke.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L[0].servedHeadersMatch===true&&L[0].scenarios.length===3&&L[0].scenarios.every((s)=>s.consoleErrors===0&&s.pageErrors===0&&s.failedRequests===0)?0:1)' "$RUNNER_TEMP/e2e-smoke.jsonl" && bash tools/e2e/run.sh --scenarios browser-face --displays 4k-dpr1,4k-dpr2 > "$RUNNER_TEMP/e2e-face.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===2&&L.every((r)=>r.servedHeadersMatch===true&&r.scenarios.length>=1&&r.scenarios.every((s)=>s.ok===true&&s.consoleErrors===0&&s.pageErrors===0&&s.failedRequests===0&&s.stuckToasts===0))?0:1)' "$RUNNER_TEMP/e2e-face.jsonl"
```

判定の基準(全て満たす):
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 12,500 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- smoke の 3 件(DPR 1)で、`consoleErrors`・`pageErrors`・`failedRequests` が全て 0。
- browser-face の集合(DPR 1 と DPR 2)で、全シナリオが `ok` かつ `consoleErrors` が 0。
- `bash guard/scan.sh` が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 特定した iframe・原因(実測と推定を分ける)、変えた箇所、console エラーの件数の変更の前後(DPR 1 と DPR 2)、流れ A〜D の 4 行、smoke と browser-face の JSON の要点。

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
- #1408(E2E のシナリオ集)の R11 は、この Issue が直るまで、`consoleErrors` の判定を印で緩める。この Issue が直ったら、その印を外す。
- #1376(Sounds タブの面の前後関係)と、`app/src/ui/webview2-surface.ts` で衝突しうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
