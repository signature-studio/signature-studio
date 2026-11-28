---
draft: "263"
title: "製品: 起動の時に OPFS の hkcu.log の getSize が失敗すると、engine が error のまま戻らず、同じ page の後続の本が連鎖して落ちる(e2e-full の最新の run で 33 本)"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "133"
blocked_by: []
labels: ["verify"]
size: M
priority: High
project: signature-studio/3
---

## 目的

起動の時に、レジストリの保存先(OPFS の `/fl/state/hkcu.log`)を開いたあとの `getSize()` が `InvalidStateError: Failed to execute 'getSize' on 'FileSystemSyncAccessHandle'` で失敗すると、engine は起動の途中で `error` の状態になり、その page では以後ずっと動かない。E2E の shard は 1 つの page を多くの本で共有するので、この失敗が 1 回起きると、後続の本が連鎖して落ちる。e2e-full の最新の run では、この連鎖で 33 本が落ちた(menus・settings・file)。

この Issue の目標は次の 3 つ。

1. 保存先の失敗で engine の起動が止まらないこと(既定の動作。オプションにしない)。保存先が使えない時の既存の経路(「保存しない」の警告と、保存先を使わない起動)に合わせる。
2. 失敗の原因を特定すること。仮説は「読み直しの直後に、前の page の worker が同じ file を開いたままで、取った handle が無効になる」(推定)。確かめられた原因を直す。例外を捕まえて黙らせるだけの変更にはしない。原因が仮説と違えば、報告に書く。
3. 単体試験と E2E で、この失敗が起動を止めないこと、後続の本が連鎖しないことを確かめる。

## 背景

### 最新の run の結果(実測。`tools/e2e` の出力)

- 理由の記録(`engineStopReason`)に `getSize` の文言が残った本は 9 本。`menus/view-11-16-choose`・`menus/view-11-17-choose`・`resilience/reload-20`(20 回の再読込の中で 1 回だけ error)・`settings/theme-ash`(表示 dpr1・dpr2)・`settings/theme-dedzii`(dpr1・dpr2)・`settings/theme-dedzii-light`(dpr2)・`settings/theme-dark-orange`(dpr2)。どれも、その本の中で page を読み直した(記録の注: `the page was reloaded in the scenario`)。
- 同じ page の後続の本は、全て `this engine build has no desktop compositor` で落ちた(33 本。`desk-dump-error`)。engine が error のままなので、desk の画を取れない。
- この run の E2E は、非 file の領域では、engine が error でも page を読み直さなかった。読み直しの規則は後で変わっている。この Issue では変えない。

### 製品の箇所(コードで確認)

- `app/src/engine/regstore.ts` の `openRegState`(約 58〜72 行)。`storage.getDirectory`・`getFileHandle`・`acquire`(`createSyncAccessHandle` を最大 50 回、100 ms 間隔で再試行)の失敗は、`try` で捕まえ、警告を出して `null` を返す(「保存しない」)。
- しかし、その後の `h.getSize()`(71 行)は `try` の外にあり、失敗は呼び出し元へ伝わる。呼び出し元は `app/src/engine/worker.ts` の `init`(111 行で `openRegState` を待つ)で、`init` の例外は `handle` の `catch`(73 行)が `{ t: 'error', phase: 'init' }` の通知にする。engine は、その後の描画・入力の経路を持たない状態で止まる。
- `app/src/engine/` は、この run の版(eea3c21)から HEAD まで変わっていない。よって、この問題は HEAD にもある(実測)。

### 仮説(推定。確認できていない)

- 読み直しの直後は、前の page の worker が同じ `hkcu.log` の handle を持ったままのことがある。`acquire` の再試行で、新しい handle は取れるが、その handle の `getSize` が無効(`InvalidStateError`)になる。
- 確認できない: この失敗の正確な条件。

## 読む物

- `app/src/engine/regstore.ts`(`acquire`・`openRegState`・`REG_STATE_*`)。
- `app/src/engine/worker.ts`(`openRegState` の呼び出し元。起動の例外が `error` の通知になる所)。
- `app/test/unit/reg-journal.test.ts`(既存の単体試験の書き方)。
- `tools/e2e/scenarios/resilience/reload-20.mjs` と `tools/e2e/lib/resilience.mjs`(再読込の繰り返し)。
- `tools/e2e/lib/common.mjs`(`startProject`・`resetProject`)。

## やること

1. 再現: `resilience/reload-20` を 1 本だけ、`--displays 4k-dpr1` で走らせる。`getSize` の文言が出るかを確かめる。出なければ、最新の run で文言が出た `settings/theme-ash` を走らせる。
2. 原因の特定: `getSize` が失敗する条件を、ログと試験で示す。前の page の worker が同じ file を持っていたかどうかを、確かめた方法と共に書く。
3. 直す: 起動が、保存先の `getSize` の失敗で止まらないようにする。保存先が使えない時の既存の経路(警告と `null`)に合わせる。捕まえる範囲は、`openRegState` の中の保存先の操作だけにする。
4. 単体試験を足す(`app/test/unit/` に、`getSize` が例外を投げる `SyncAccess` を使う試験)。例外で終わらず、保存先を使わない状態を返すこと。既存の試験は全て通すこと。
5. 受け入れを通す。通らない時は、落ちた試験と、出力の 1 行を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && npm test --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --only menus/view-11-16-choose,menus/view-11-17-choose,settings/theme-ash,settings/theme-dedzii,settings/theme-dark-orange,resilience/reload-20 --displays 4k-dpr1,4k-dpr2 --budget 40m > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((x)=>JSON.parse(x));process.exit(L.length>=1&&L.every((r)=>r.error===null&&r.scenarios.length>=1&&r.scenarios.every((s)=>s.ok===true&&s.skipped!==true&&s.error===null&&s.engineStops===0&&s.engineState==="engine-idle"))?0:1)' "$RUNNER_TEMP/e2e.jsonl"
```

判定の基準:
- 受け入れの式が終了コード 0 で終わる(単体試験が全て通り、起動確認が engine-idle、6 本の E2E が全て合格し、engine の停止と error が無い)。
- 報告に: (a) 再現の結果(`getSize` の文言が出た本と回数。出なかった場合はその事実)、(b) 原因の特定と、確かめた方法、(c) 直した箇所(ファイルと行)、(d) 追加した単体試験の名前、(e) 6 本の E2E の出力の行。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。
- 段ごとに、やること 1〜5 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品(`app/`・`lift/`)は直してよい。`tools/e2e/` は、この Issue の受け入れの本を動かすために必要な分だけ直す。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。
- sleep・nohup・末尾の `&`・先頭の `timeout` は使わない。待つ時は条件で待つ。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit・PR に、AI が書いたことを示す行を付けない。commit の名義は、環境の設定のまま。
- GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドと E2E は、この job の中で行う。長いコマンドは、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。`git add` は、自分が変えたパスだけを指定する。

## 他のタスクとの関係

- 箱が閉じない群(#{{264}})・plugins の一覧の読み(#{{265}})とは、原因が違う。
- 製品の停止の型(VirtualQuery の画像の表・間接呼び出し・CompareStringW・PathCleanupSpec)とは別。
