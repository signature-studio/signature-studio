---
draft: "212"
title: "E2E の領域 delivery: manifest.json の lazy の全ての取得と sha256、ヘッダー・圧縮・オフライン・キャッシュ・ビーコンの形の、期待する結果のシナリオを足す"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: ["199"]
labels: ["verify", "render"]
size: XL
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の指示(2026-11-26。原文): 「そうではなく、存在する機能について、考えうる限り全ての E2E テストを追加してください。」

この領域(delivery)は、本番と同じ配信の機能を、期待する結果つきのシナリオで確かめる。配信される全ての物が取得でき、中身が一致し、ヘッダーと圧縮と、オフライン・キャッシュ・ビーコンの形が正しいことを確かめる。最小のシナリオ数は 40 本(`tools/e2e/features/delivery.min` と同じ)。目標は次の 2 つ。
1. 配信の物(lazy の全て・ヘッダー・圧縮・ビーコン)を、期待する結果を持つシナリオで覆うこと。
2. 配信の物の一覧(`delivery.tsv`)の全てに、1 本以上のシナリオがあること。

## 背景

- #{{199}}(E0)が作る物: `defineScenario`・`run.mjs --area`・`--list`・`--shard`・`--budget`・配信の一覧 `tools/e2e/features/generated/delivery.tsv`(bundle の `manifest.json` の lazy の全て)・見本 3 本(`tools/e2e/scenarios/delivery/`)。E0 が閉じてから始める。
- 配信は、`tools/e2e/lib/delivery.mjs`(本番と同じ Worker を `wrangler dev --local` で立て、ローカルの R2 に bundle を入れる)と、`tools/e2e/lib/served.mjs`(配信の応答のヘッダーを `app/shared/headers.ts` と比べる)で行う。
- 本番の Worker の応答の規則は `app/worker/index.ts`(`baseHeaders`・`/assets/*`・`run_worker_first`)。ヘッダーの値は `app/shared/headers.ts`。
- ビーコン(実機の計測の送信)の形の検証は、`beaconProblems`(場所は作業で確かめる)。

## 読む物

- `tools/e2e/`(E0 の後の姿。lib/delivery.mjs・lib/served.mjs・lib/access.mjs・lib/flow.mjs の `openProjectFile`)
- `app/worker/index.ts`・`app/worker/access.ts`・`app/shared/headers.ts`・`app/wrangler.jsonc`(変数の名前。値は書かない)
- `.github/workflows/deploy.yml`(本番の配信の手順。読むだけ)
- `app/src/perf/beacon.ts` と、ビーコンの形の検証の関数(`beaconProblems`)
- `pm-work/findings/hidpi-fl-scaling.md`・`tools/delivery-split/`・`tools/e2e/features/generated/delivery.tsv`(E0 が作る一覧)
- `pm-work/findings/e2e-program.md`

## やること

1. **機能の表** `tools/e2e/features/delivery.tsv`(手で書く。見出し行は E0 が作る)。機能ごとに 1 行。列は `id`・`kind`・`name`・`expected`・`how`。
2. **シナリオ**(`tools/e2e/scenarios/delivery/`。1 本 1 ファイル)。次の全てを、機能ごとに 1 本以上。
   - **lazy の全て**: `manifest.json` の lazy の物の全てを、取得する(ページの要求で)。期待: 全てが取得でき(HTTP 200)、中身の sha256 が `manifest.json` の値と一致する。一致しない物は、一覧に出し、印 `expectFail: 'new'`。
   - **ヘッダー**: 配信の応答で、COOP・COEP・CSP・`Content-Encoding`・`Cache-Control` を確かめる(`lib/served.mjs` と `app/shared/headers.ts` の値が一致する)。`/assets/*` の物ごとに、`cacheControlFor` の規則と一致する。
   - **圧縮の整合**: `br` の物の中身が、展開すると `raw` の物と一致する(sha256)。`Content-Encoding` と中身の形が合う。
   - **ネットワークが遅い時**: 要求の応答を遅らせる(ブラウザの網の制限。CDP の `Network.emulateNetworkConditions`)。期待: 起動が完了するか、遅いことが分かる表示が出る(消えない表示は 0。ページは止まらない)。
   - **ネットワークが切れた時**: 配信を切ってから、起動の途中の要求が失敗する。期待: エラーの表示(`error-offline` の画面)が出る。停止 0(ページの例外は、表示の範囲に限る)。
   - **キャッシュの無効化**: 版が変わった時(`manifest.json` の版の違い)に、古いキャッシュを使わない(`RAW_CACHE`・`BR_CACHE` の規則)。
   - **ビーコンの形**: 実機の計測の送信(`beaconProblems` の結果が空)。送信の要求の形(個人情報が無い。項目の名前が既知の物だけ)。
   - **起動の前の取得**(`preBootBytes`): 起動の前に取る量が、基準の値の範囲に在る(報告の run の値を基準にする。絶対値の定数を書かない)。
   - **アクセス制御の検証**(テスト用の設定。`lib/access.mjs`): 検証が通った時、配信の物が見える。検証が無い時、入れない(要求が 401/403 で止まる)。
3. **判定**: 各シナリオは `ok` と、測った値(`results`: 取得の数・一致の数・不一致の名前・ヘッダーの一致・圧縮の一致・ビーコンの問題の数)を返す。`notes` に、判定の根拠を 1 行で書く。
4. **表示**: 配信の物は DPR によらない。既定は `['4k-dpr1']`(起動を伴う物だけ、表示を足す)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --area delivery --list > "$RUNNER_TEMP/list.jsonl" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n")[0]);process.exit(r.area==="delivery"&&r.scenarios>=40?0:1)' "$RUNNER_TEMP/list.jsonl" && node tools/e2e/run.mjs --area delivery --shard 1/8 --displays 4k-dpr1 --budget 40m > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true&&r.scenarios.length>0&&r.scenarios.every((s)=>s.skipped!==true))?0:1)' "$RUNNER_TEMP/e2e.jsonl"
```

判定の基準:
- 受け入れの式が通る(流れ A〜D・`npm test --prefix app`・`--list` の delivery の scenarios が 40 以上・1 シャード(1/8)の実行が終了コード 0。`skipped` が無い)。
- `manifest.json` の lazy の全ての物(`generated/delivery.tsv` の行)が、取得できて、sha256 が一致する(件数と、不一致の名前の一覧を報告に書く)。
- `servedHeadersMatch` が真で、`/assets/*` の全ての種類(`raw`・`br`)の物で、ヘッダーと圧縮が合う。
- ビーコンの形の検証の問題数が 0。
- 網羅率: 全シャードを合わせて、この領域の機能の 90% 以上が、`ok` かつ `expectFail` 無しのシナリオで覆われる(`judge-coverage.jq`。PM が判定する。報告に覆った数を書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に、(a) 追加したシナリオの ID と、各 `covers`、(b) lazy の件数・取得の数・sha256 の一致の数・不一致の名前、(c) ヘッダーの一致の表(名前・期待・実際)、(d) ネットワークの条件の結果、(e) ビーコンの問題の一覧、(f) 印 `expectFail: 'new'` の一覧、(g) 通らなかった物と理由。
- 配信の値(ホスト名・アカウント・バケット)は、報告にも書かない。ローカルの配信の名前(`localhost` の形の物)だけ書く。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。製品の欠陥は、シナリオに `expectFail: 'new'` を付けて記録する。直すのは PM が起票する別の Issue。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。`.github/` は変えない。
- 領域の作業で作るファイルは、`tools/e2e/scenarios/delivery/`・`tools/e2e/features/delivery.tsv`・`tools/e2e/lib/delivery-tests.mjs`(必要な時だけ)に限る。`lib/delivery.mjs`・`lib/served.mjs`・`lib/access.mjs` は、最小の差分にとどめ、変えた理由を報告に書く。
- Python を使わない。OCR を使わない。
- ベンダー名・Cloudflare の識別子(アカウント・バケット・ホスト名・アクセス制御のチーム名と AUD の値)を、ファイルに書かない。R2 の値・アカウント ID も書かない。
- 期待値に、絶対値(バイト・ミリ秒)を書かない。比と、ハッシュの一致で書く。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 先に、lazy の全件の取得と sha256(配信の一覧)を、次にヘッダーと圧縮、最後にオフライン・キャッシュ・ビーコンを足す。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{199}}(E0)。E0 が閉じるまで起動しない。
- 他の領域(#{{200}}〜#{{211}})と、`coverage.json` の母数を共有するが、ファイルは共有しない。`lib/delivery.mjs` を使うのは、この領域だけでよい。
- 配信の変更(`app/worker/`・`app/shared/headers.ts`)の Issue が後で来た時は、その Issue に対応するシナリオを 1 本以上足す。
