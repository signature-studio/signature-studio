---
draft: "176"
title: "DEDZII の曲を File > Open で開くと、約 5 秒後に engine が終了コード 134 で止まる(本番と同じ配信の E2E で再現): 停止の理由を特定して直す"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: []
labels: ["verify"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者が報告した「評価用プロジェクトを開くとクラッシュする」を、本番と同じ配信の E2E で再現し、停止の理由を特定して直す。

この Issue の目標は次の 3 つ。
1. `DEDZII - NIGHT TERROR.flp` を File > Open で開いたとき、engine が止まらず、プロジェクトが開くこと。
2. 止まる場合でも、E2E のシナリオの JSON(`engineStop`)に、停止の理由(最後の `fle:` の行・`== stop:`・`abort kind`・R[0..15]・戻り番地)が出ること。
3. 1 回の試行で直せない場合は、停止の理由を `pm-work/findings/` に記録し、その理由が E2E の JSON に出ていること。

## 背景

- 事実 1(E2E の再現。deploy の run 45559400107・版 `908248db`。runner の値): E2E の `file-open-demo` が、4 表示(DPR 1・2・1.51・2.86)の全てで失敗した。手順は次のとおり。Welcome を閉じて Default を開き、File > Open(Ctrl+O)で、`Data\Demo projects\Demo songs\DEDZII - NIGHT TERROR.flp` をファイル管理の窓へアップロードし、選んで Open を押す。すると約 5 秒後に題名が `Song info` になり、engine が終了コード 134(SIGABRT)で止まる。JSON の `engineStopReason` は終了コードだけで、停止の理由は出ていなかった。
- 事実 2: 依頼者の報告と同じ経路の再現(本番と同じ配信・同じ操作)。
- 記録(`pm-work/findings/p3-blocks-1335.md` の 61〜63 行と 162 行): ワーカーの `openProject`(`tools/p3-audio-probe/measure.mjs`。VFS へ直接置く経路)で DEDZII を開いたとき、`Song info`(TInfoForm)の窓が出た直後に、ログ `fle: LockResource: no PE image is mapped at 0x400000 (no MZ signature)` で止まった。Exoplanet も同じ。この記録は古い版のもので、今の版では確かめられていない。停止の理由の候補の 1 つとして扱う。
- PM の確認(記録の場所は未確認): ワーカーの `openProject` の経路では DEDZII が engine-idle まで行ったと報告されている(#{{129}}・#{{160}} の報告)。この経路と E2E の経路の違いが、原因の手がかりになりうる。
- 関連: 素材は `%FLStudioFactoryData%\…` の綴りで要求される(#{{169}} の環境変数の経路。#{{160}} の素材の配信)。Delphi の例外(0xEEDFADE)は #1410 で直した経路。Mehran の曲でも、同じ 134 の停止が間接呼び出しの飛び先の判定で起きたと #1403 の報告にある(記録は main に無い)。DEDZII の停止の理由が、それと同じかは分からない。

## 読む物

- `tools/e2e/`: `run.mjs`(`--only`・`engineStop`)、`lib/flow.mjs`(`openProjectFile`: ファイル管理の窓への経路)、`lib/detect.mjs`(`engineExitFacts`)、`scenarios/smoke/file-open-demo.mjs`。
- `tools/p3-audio-probe/measure.mjs`(VFS へ直接置く経路。上の記録の道具)。
- `pm-work/findings/p3-blocks-1335.md`(DEDZII の記録)、`pm-work/findings/real-user-exits-2026-11-24.md`(99 行。停止の理由を記録に入れる案)。
- `lift/harness/fruity.c` の `report()`(`== stop:`・`runtime message`・`abort kind`・R[0..15]・戻り番地の出力)、`lift/harness/wasm/trace_fruity.c` の `abort_kind()`。
- `app/src/perf/exit-message.ts`(`engineStopFacts`・`engineExitMessage`)、`app/src/main.ts` の `exit()`。
- `lift/runtime/fle_env.h` の `fle_stop`(文言は `fle: <api>: <why>`)。
- `lift/harness/seeds/vfs-seed-idle1.tsv`・`pm-work/findings/demo-assets-inventory.tsv`(DEDZII の素材の行)。

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 再現する。`cd tools/e2e && npm ci && npx playwright install chromium` の後、bundle を作ってから、`node tools/e2e/run.mjs --scenarios smoke --only file-open-demo --displays 4k-dpr1` を実行し、出力の `engineStop` を見る。`--only` と `engineStop` は `tools/e2e/` に在る(無ければ足す)。
2. 特定する。停止の理由が何か: `LockResource` の PE 無し・`CreateFileW` の空のパス・`indirect call to 0`・`ILSvg`・Delphi の例外(0xEEDFADE)・メモリ・その他。停止の直前の `fle:` の行・戻り番地・R[0..15] から、呼び出し元の FL の手続きまで辿る。
   経路の違いも確かめる: ワーカーの `openProject`(VFS へ直接)と E2E の経路(ファイル管理の窓へのアップロード・選んで Open)の違い。ファイル名・パス・大きさ・素材の配信(起動の後の取得。`@lazy-file:`)・アップロードされたファイルの扱い。
3. 直す。製品の既定の動作として直す(旗・環境変数・`#ifdef` を足さない)。直し方と、その理由を報告に書く。直せない時は、停止の理由と、そこまでの確かめた事実を `pm-work/findings/dedzii-<短い名前>.md` に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/fe-lift-test.mjs && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --scenarios smoke --only file-open-demo --displays 4k-dpr1,4k-random > "$RUNNER_TEMP/e2e-open.jsonl"; node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=(r)=>r.scenarios.find((s)=>s.name==="file-open-demo");const fixed=L.length===3&&L.every((r)=>{const s=S(r);return !!s&&s.ok===true&&s.engineState==="engine-idle"&&s.engineStops===0;});const named=fs.readdirSync("pm-work/findings").some((f)=>/^dedzii-/.test(f));const reported=L.length===3&&L.every((r)=>{const s=S(r);return !!s&&!!s.engineStop&&Array.isArray(s.engineStop.report)&&s.engineStop.report.length>0;});process.exit(fixed||(reported&&named)?0:1)' "$RUNNER_TEMP/e2e-open.jsonl"
```

判定の基準(次のどちらか):
- 直した場合: 起動確認(`boot.json`)が engine-idle・`standIn` が false・`elapsedMs` が 12,500 以下。流れ A〜D の 4 行の判定(`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true)。`cargo test -p fe-lift` と `npm test --prefix app` が通る。E2E の `file-open-demo` の 3 行(4k-dpr1 と 4k-random の 2 表示)が、全て `ok` が真・`engineState` が engine-idle・`engineStops` が 0。
- 直せなかった場合: 上の検査は通り、E2E の `file-open-demo` の 3 行の全てに、`engineStop.report` が空でない停止の報告が在り、`pm-work/findings/` に `dedzii-` で始まる記録が在る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 停止の理由(`engineStop` の `stop`・`report` の行をそのまま)、経路の違いで確かめたこと、直した方式と理由(変えたパス。`lift/` を変えると main の deploy が動く)、流れ A〜D の 4 行、E2E の `file-open-demo` の 3 行の `ok`・`engineState`・`engineStop`。
- 実測と推定を分ける。推定だけの原因は、「推定」と書く。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数・旗を足さない。
- Python と OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- 資料は、題名と年で書く。URL を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない(コマンドの先頭に書かない)。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by は無い。
- `#{{160}}`(demo の素材の配信)と `#{{169}}`(ゲストの環境変数)は、DEDZII の素材の読みに関わる。先に main に入っていれば、その後の E2E の結果で判断する。`#{{160}}` の受け入れを待たない。
- `#{{129}}`(demo の素材)と `vfs-seed-idle1.tsv` を共有しうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- `#{{166}}`(Delphi の例外)と、`lift/runtime/fle_env.h` の停止の経路を共有しうる。
