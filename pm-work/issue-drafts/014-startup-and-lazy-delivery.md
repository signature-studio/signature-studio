---
draft: "014"
title: "起動前に取る物と、起動後にバックグラウンドで取る物を分ける(lazy-load の仕組み)"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["delivery"]
size: L
priority: High
project: signature-studio/3
---

## 目的

依頼者の提案: FL Studio の起動に必要な wasm が揃った段階で FL は起動し、プラグインやアセットは、使われていなくても、起動後にバックグラウンドで取る。FL の起動処理が始まった時点から順次取り、プラグインやアセットへのアクセスを起点にしない。1 プラグイン 1 wasm で試行する。実現が難しいと判断した場合は、方針を変えてよい(理由を報告に書く)。

## 規模

規模: 大(想定 8 時間。ビルドを約 5 回回す規模)

## 背景

- 今の起動前の取得は 4 つで、すべて起動の前にまとめて取る(`app/src/main.ts` 714〜717 行): `loadWasm`(engine wasm)、`loadPack`(seed pack)、`loadPack`(DLL pack)、`fetchAsset`(engine glue)。起動後の区分は無い。`client.init` は 724 行。
- 取得の部品(`app/src/bundle/loader.ts`): `fetchAsset`(185 行)、`loadWasm`(189 行。197 行で `WebAssembly.compileStreaming`)、`loadPack`(219 行)。検証は SHA-256 と大きさ(`verifyingStream` の 138 行付近)。
- engine の instantiate は `app/src/engine/fruity.ts` 133 行(`WebAssembly.instantiate`)。DLL pack は `fruity.ts` 125 行付近で mount される。
- 取得の先例(製品の経路には繋がっていない): `app/src/engine/chunk-io.ts`(遅延の install tree の chunk を、別の Worker が取り、engine は Atomics.wait で待つ)。`app/src/engine/chunk-io-worker.ts` は、製品から起動されていない。
- 旧の測定の値(runner の値。pm-work/findings/survey-c2-worker-split.md・survey-c3-fl-resources.md): engine wasm 167,414,828 B(br 17,717,323 B)、seed pack 20,780,949 B、DLL pack 88,675,800 B(11 ファイル。FLEngine と 10 モジュール)。Fruity の 3 つは、DLL pack と seed pack の両方に入っている。
- DLL pack の中身は、`lift/harness/web/pack.args` の `--args` の行(13〜14 行)で engine に渡される。モジュールの表は `lift/harness/b2d2/modules.tsv`(10 行)。
- 旧の計画の論点(案。依頼者の指示ではない): (1) 起動後の取得の順序、(2) 取得が終わる前にプラグインが使われた時の振る舞い、(3) 取得した物の検証(SHA-256 と大きさ。manifest の検査と同じ作り)とキャッシュ、(4) 取得とコンパイル・インスタンス化を音声の Worker の外で行う、(5) 起動前の予算(P1 の 10 秒に入るか)。

## 完了の条件

- 起動前に取る物(FLEngine・言語・テーマ・テンプレートなど)と、起動後に取る物(プラグインとアセット)の区分が、manifest か bundle の作りで分かれている。
- 起動後の取得が、FL の起動処理の開始から始まり、engine-idle を遅らせない(測定の道具の出力で確かめる)。
- 取得した物の SHA-256 と大きさを検査する。失敗した場合の振る舞いを決め、報告に書く。

## 読む物

- `app/src/main.ts`(260〜276 行: 表示。714〜724 行: 起動時の取得と client.init)
- `app/src/bundle/loader.ts`(138 行: `verifyingStream`。185 行: `fetchAsset`。189・197 行: `loadWasm`。219 行: `loadPack`)
- `app/src/bundle/manifest.ts`(73〜75 行: `seedPack` と `dllPack`)、`app/src/bundle/assetstore.ts`
- `app/src/engine/fruity.ts`(120〜140 行: DLL pack の mount と instantiate)、`app/src/engine/hostfiles.ts`(`mountPack`)
- `app/src/engine/chunk-io.ts`(1〜10 行の注釈)
- `app/scripts/pack-assets.mjs`(bundle の作り)、`app/scripts/pack-seed.mjs`(seed pack の作り)
- `lift/crates/fe-build/src/main.rs`(bundle の段。`pack_step_for`)
- `lift/harness/web/pack.args`(13〜14 行: `--args` の行)、`lift/harness/b2d2/modules.tsv`(10 行)
- `lift/runtime/fle_realmod.h`(1〜3 行の注釈: real module)、`lift/runtime/fle_load.c`(LoadLibrary の解決)
- `app/worker/index.ts`(2 行目: /assets/ の配信)、`app/tools/assets-middleware.ts`(ローカルの配信)
- `tools/boot-check/boot-bundle.mjs`、`tools/boot-check/cdp.mjs`
- `pm-work/plan.md`(「配信の方針」の節。論点の出典)

## やること

1. 変更の前に、今の起動前の取得を測る(resource timing の名前・大きさ・開始時刻)。出力を報告に貼る。
2. 区分の実装。起動前に取る物と、起動後に取る物を、manifest か bundle の作りで分ける。設計は作業者が決める。
3. 起動後の取得: FL の起動処理が始まった時点から、順次取る。順序の決め方と、取得が終わる前に使われた時の振る舞いを、設計として書く。
4. 測定の道具 `tools/delivery-split/measure.mjs` を作る。boot-bundle と同じ起動で、engine-idle まで待つ。stdout に JSON の 3 行(1 行が 1 回の起動): `{"state","standIn","error","elapsedMs","preStart":[{"name","bytes"}],"lazy":[{"name","bytes"}],"preStartDoneMs","fetches":[{"name","startMs","endMs"}]}`。`fetches` は `performance.getEntriesByType('resource')` から取る。
5. 方針を変えた場合は、変えた理由と、代わりの案を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/delivery-split/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/delivery.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const sum=(a)=>a.reduce((s,f)=>s+f.bytes,0);const ok=L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&Array.isArray(r.preStart)&&sum(r.preStart)>0&&Array.isArray(r.lazy)&&typeof r.preStartDoneMs==="number");process.exit(ok?0:1)' "$RUNNER_TEMP/delivery.jsonl"
```

判定の基準: 道具の 3 行の生の値を、上の node -e が読む。起動前の取得の合計が 0 より大きいこと、3 回とも engine-idle に達したこと。`elapsedMs` の 3 回の値(最小・中央値)は報告に書く。P1 の 10 秒との比較は、判定に入れず、報告に書く(子 035)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 設計の選択(区分の作り、取得の順序、使われた時の振る舞い、検証)、方針を変えた場合はその理由、変えたパス(app/・lift/ を含む)、起動前と起動後の合計の大きさ(変更の前後)、elapsedMs の 3 回の値。
- 起動後の取得の開始が engine-idle より前にある場合は、その事実を書く。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 製品の既定の起動の動作(起動前に取る物の一覧)を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- 作業者に依頼者の個人の資格を置かない。
- Worker 数の上限は、事前に考えない。当たったら報告に書き、方針を変える。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- 015(Fruity Balance を起動後の取得に移す)、035(起動を 10 秒以内にする)、027〜029(外部プラグイン)が、この仕組みを使う。
- 002(描画の分離)と同じ起動の経路(`app/src/engine/client.ts`)を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
