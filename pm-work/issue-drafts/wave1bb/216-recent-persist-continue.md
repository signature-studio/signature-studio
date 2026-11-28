---
draft: "216"
title: "FL の Recent(最近開いたファイル)を、ページの再読込をまたいで保持する(続き): 閉じる時の FL の後始末を時間の枠つきで走らせ、書かれた Recent を journal に載せる"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["165"]
labels: ["resources", "verify"]
size: XL
priority: High
project: signature-studio/3
---

## 目的

依頼者の報告(原文): 「ページを再読込すると Recent が消えるなど、一部のデータは保持されていないように見えます。」

#1409 は、FL が Recent をどこへ・いつ書くかを実測で調べ、診断で閉じた(保持は作っていない)。この Issue はその続きで、次の 3 つを目標にする。
1. FL の Welcome の窓に、Recent の窓(`TRecentFilesModule`)が出る条件を特定する(陽性の対照を作り直して、出る条件を実測で確かめる)。
2. ページを離れる時(`pagehide`)に、FL の後始末(`instance->vtable[0]`)を、時間の枠つきで走らせ、設定と Recent を FL に書かせる。枠を超えたら打ち切り、ページを離れる動作を遅らせない。
3. 書かれた Recent を、既存の journal(ページの再読込をまたぐ保持の仕組み)に載せ、再読込の後に FL が Recent を読めるようにする。

受け入れは、E2E のシナリオ `recent-persists`(#1408 が `tools/e2e/scenarios/regression/` に作る)の印を外し、2 表示で ok になること。

## 前提

- #1408(E2E のシナリオ集)が main に入っていること。この Issue は #1408 が閉じた後に起動する(依存の設定)。`recent-persists` は #1408 の作ったシナリオ。
- #1409 の診断(`pm-work/findings/recent-1409.md`)が前提。その記録の「次の Issue にすべき点」の 1 と 2 を、この Issue が扱う。

## 背景

実測(#1409 の記録。runner の値):
- FL は Recent を、レジストリ `HKCU\Software\Image-Line\FL Studio 26\MRU` の値 `"1"`〜`"49"`(型 1・文字列)から読む。起動の中で `RegOpenKeyExW` は 50 回。`Settings\Browser\Recent files.scr` は起動の時に探され、中の 1 行を読む。
- 曲のファイルと MRU の値と `Settings\Browser\Recent files.scr` を、起動の前に仮想ボリュームへ置いても、Welcome の窓に `TRecentFilesModule` は出ない。出る条件は未確認。
- 閉じる要求(`pagehide` が出す SC_CLOSE)は、FL のメインの窓の `WM_CLOSE` まで届き、`PostQuitMessage` が呼ばれる。その後の FL64 の後始末 `instance->vtable[0](instance)` は、今の engine では呼ばれない(`lift/harness/fruity.c` の「stage 1: recorded only」)。Recent の書き手は、この後始末の中に在る可能性がある(推定。未確認)。
- この後始末を呼ぶ probe(#1409 の 5 回目。取り消した)は、呼び出しから戻らず、engine が 30 秒の `engine-idle` のまま止まった。陽性の対照のファイルも作れなくなった。待ちの場所は未確認。
- 書き側は、開く・保存・Save As・新規・閉じる要求のどれでも、Recent・MRU・`Settings\Browser\Recent files.scr` への書きを観測していない。

既存の仕組み:
- 仮想ボリュームはメモリの上だけにあり、ページの再読込で消える(`lift/runtime/fle_vfs.h`)。
- レジストリの journal(`/fl/state/hkcu.log`。キーは値の名前)。起動の前に `fle_reg_restore_saved` が戻す(`app/src/engine/reg-journal.ts`・`app/src/engine/regstore.ts`)。
- ページを離れる時の要求: `app/src/main.ts` の `pagehide`(`client.exitRequest()`)。その先は `app/src/engine/client.ts`・`app/src/engine/worker.ts`・`app/src/engine/fruity.ts`。

## 読む物

- `pm-work/findings/recent-1409.md`(前回の診断の全部)
- `lift/harness/fruity.c`(`CreateFruityInstance`・stage 1 の記録・`instance->vtable[0]` の呼び出しの場所。約 1089 行の付近)
- `lift/runtime/fle_reg.c`・`lift/runtime/fle_vfs.c`・`lift/runtime/fle_win.c`・`lift/runtime/fle_thread.c`(窓の trace・スレッドの trace)
- `app/src/engine/reg-journal.ts`・`app/src/engine/regstore.ts`・`app/src/engine/worker.ts`・`app/src/engine/fruity.ts`・`app/src/engine/client.ts`・`app/src/main.ts`(`pagehide`)
- `tools/persist/measure.mjs`(files 段。陽性の対照と、閉じる要求の列)・`tools/smoke/welcome-flow.mjs`・`tools/boot-check/boot-bundle.mjs`
- `tools/e2e/scenarios/regression/` の `recent-persists`(#1408 が作る版)・`tools/e2e/lib/detect.mjs`

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。`git status` で、前回の差分が作業ツリーに在るかを確かめる。在る物は取り除かず、残りを行う。
1. **Recent の窓の出る条件を特定する**: 陽性の対照(曲のファイルと MRU の値と `.scr` を、起動の前に置いた状態)で、Welcome の窓を作る経路を、窓の表と VCL の生成の trace で見る。`TRecentFilesModule` が出る条件(値の形・Welcome の窓の作り方・設定の既定の値のどれか)を、実測で特定する。推定で作らない。
2. **閉じる時の後始末を走らせる**: `pagehide` の時に `instance->vtable[0](instance)` を、時間の枠つき(枠は秒で決め、報告に書く。枠を超えたら打ち切る)で呼ぶ経路を作る。その中で FL が設定と Recent を書いたかを、レジストリと仮想ボリュームの書きの trace で確かめる。前回の probe が戻らなかったので、待ちの場所をスレッドの trace(`fle_thread_set_trace`)で先に確かめてから、枠と打ち切りの形を決める。打ち切った時は JSON に出す。
3. **書いた物を journal に載せる**: レジストリの値(MRU など)は、レジストリの journal(`/fl/state/hkcu.log`)に載せる。ファイル(`Settings\` の下)は、#1385 の保存の記録(op 5・6・7)の形に載せる。場所(パス)や値の名前は、FL が書いた物をそのまま写す。定数・表・分岐として runtime・app・テストに入れない。
4. **再読込の後の判定**: 再読込の後に、Recent の窓(または項目の文字)が出ることを、窓の表と文字の走査で確かめる。曲の名前の一致では判定しない(保存したプロジェクトのファイルは、再読込の後にゲストのメモリへ戻るため、名前が在っても Recent は無いことがある)。
5. `recent-persists` の印を外す(#1408 の本文の印の表で、この Issue の番号が付いている)。
6. 受け入れを通す。通らなければ、作業ツリーを commit して、報告を書いて終える。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && (node tools/e2e/run.mjs --scenarios regression --only recent-persists --displays 4k-dpr1,4k-dpr2 > "$RUNNER_TEMP/e2e.jsonl") && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const good=(r)=>{const s=r.scenarios.find((x)=>x.name==="recent-persists");return r.servedHeadersMatch===true&&r.error===null&&!!s&&s.expectFail===null&&s.ok===true&&s.pageErrors===0&&s.consoleErrors===0&&s.failedRequests===0&&s.stuckToasts===0;};process.exit(L.length===2&&L.every(good)?0:1)' "$RUNNER_TEMP/e2e.jsonl" && bash guard/scan.sh
```

## 判定の基準

- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 12,500 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `recent-persists` が、2 表示(4k-dpr1・4k-dpr2)の行で、印なし(`expectFail` が null)・`ok` が真・`pageErrors`・`consoleErrors`・`failedRequests`・`stuckToasts` が 0。
- `bash guard/scan.sh` が 0 件。
- 報告に、陽性の対照の結果(`TRecentFilesModule` が出る条件の特定の有無)、閉じる時の後始末の枠(秒)と打ち切りの記録、journal に載せた場所(キー・パス)、`recent-persists` の 2 行の JSON を書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 段ごと(上の「やること」の 1〜4)に、完了・途中・未着手を書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。保存は既定の動作にする(オプトインを作らない)。テストのための分岐を製品に入れない。
- ファイルの名前・場所・値の名前を、runtime・app・seed・テストに、定数・表・分岐として入れない。
- ページを離れる時の後始末は、時間の枠を超えたら打ち切る。枠を超えてページを離れる動作を遅らせない。
- 既存の journal の形(#1383・#1385)を変えない。`app/src/files/select.ts`(開く・保存の窓の橋)の動作を変えない。
- `tools/e2e/judge.jq` と `tools/e2e/run.mjs` の `rowPasses` は変えない。
- Python と OCR を使わない。禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業(ビルド・測定)は Actions で行う。この機械では行わない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 1 回で終わらない前提で、上の段 1〜4 ごとに commit する。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: #1408(`recent-persists` を作る。main に在る必要がある)。
- #1409(診断。閉じている)の続き。#1383・#1385(journal の形)を使う。
