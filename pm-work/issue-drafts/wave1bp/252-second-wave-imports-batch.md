---
draft: "252"
title: "第 2 波の DLL が使う import のうち runtime に無い物を、全件、表にして実装する: 実装しない物は名前つきの停止にする"
type: "Task"
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "021"
blocked_by: []
labels: ["plugin-builtin"]
size: "L"
priority: "High"
project: "signature-studio/3"
---

## 目的

内蔵のプラグインの DLL は、runtime に定義の無い Windows の関数(import)を使う。今は、止まった DLL ごとに 1 件ずつ見つけて足しているので、止まる DLL が次々と出る。1 件ずつ足すと終わらない。

前例: #1386 は、python312 の import(361 件)を全件の表(`pm-work/findings/python312-imports.md`)にして、実装する物と止める物を分けた。この Issue は、同じ作り方を第 2 波の全 DLL に広げる。全 DLL の import を集め、runtime に無い物を全件、表にし、分類して、実装する。実装しない物は、`unsupported instruction` で止まらず、名前つきの停止にする。

#1434・#1435・#1436・#1437・#1438・#1439・#1442・#1443・#1444・#1446・#1448 の 11 件は、この Issue を前提にする。

## 背景

- 観測(runner の値。この機械とは別枠):
  - `GetClassLongPtrW`: #1435(Edison)・#1442(Fruity HTML NoteBook)・#1446(Fruity Peak Controller)が、`fle: unsupported instruction at …` で止まった。生成の段の報告にも `import GetClassLongPtrW is not modelled by the C runtime` が出る。
  - `TryEnterCriticalSection`: #1439(Fruity Convolver)が、`fle: unsupported instruction at …` で止まった。生成の段の報告は `import TryEnterCriticalSection is not modelled by the C runtime`。`kernel32.dll` の `EnterCriticalSection` は在るが、この名前は無い。
  - 字形: `DrawTextW`(#1434 Control Surface)と `GetTextMetricsW`(#1444 Fruity NoteBook)は、`not modelled (decision 0111)` で止まった。字形の実物(フォント)が要る。
- 生成の段の文 `is not modelled by the C runtime` は、`lift/crates/fe-wasmgen/src/lower.rs` の `call_indirect` と `TailImport` の経路で出る。
- `GetClassLongPtrW` は、`lift/runtime/`・`lift/harness/`・`ci/`・`inputs/` のどこにも無い。`lift/crates/fe-lift/tests/class.rs` と `cursor.rs` の試験にだけ在る。
- import の表: `lift/crates/fe-lift/src/imports_table.rs`。行の形は `("kernel32.dll", "EnterCriticalSection", "fle_imp_EnterCriticalSection", "jmp_imp:EnterCriticalSection")`。C の実装は `lift/runtime/fle_sys.c` の `fle_imp_*`(宣言は `lift/runtime/fle_sys.h`)と、`lift/runtime/fle_imports.c`。
- import の名前: `inputs/fl-installer-files.tsv` の `imports` 列は DLL の名前だけで、関数の名前は無い。関数の名前は、DLL の import 表から読む(`lift/crates/fe-pe`)。
- 名前の一覧の見本: `lift/harness/hyb/c-imports-engine.txt`・`lift/harness/seeds/ship6-imports.txt`。

## 読む物

- `inputs/fl-installer-files.tsv`(`Plugins\Fruity\` の下の DLL と、`Plugins\` の下の内蔵のプラグインの行。kind が dll の物)
- `lift/crates/fe-lift/src/imports_table.rs`(冒頭の注釈と表)
- `lift/runtime/fle_imports.c`・`lift/runtime/fle_imports.h`・`lift/runtime/fle_sys.c`・`lift/runtime/fle_sys.h`(`fle_imp_*` の形)
- `lift/crates/fe-wasmgen/src/lib.rs`(`c_imp` と `import_sites` の作り方)・`lift/crates/fe-wasmgen/src/lower.rs`(`is not modelled by the C runtime` の出所)
- `lift/crates/fe-pe/`(import 表の読み)
- `lift/crates/fe-lift/tests/class.rs`・`cursor.rs`(既存の import の試験の形)

## やること

1. 全 DLL の import を集める。対象は、`inputs/fl-installer-files.tsv` の `Plugins\Fruity\` の下の DLL と、`Plugins\` の下の内蔵のプラグインの DLL。DLL の実物は、作業の木の `lift/out-app/host-extra/plugins/`(内蔵のプラグインの取り出し)に在る。DLL ごとに、import の `dll!名前` を出す。
2. runtime に無い物を全件、表にする。表は `pm-work/findings/second-wave-imports.md`。列は、DLL・`dll!名前`・runtime の有無・分類・実装の場所(または停止の名前)。全件を載せる。分類は次の 3 つ。
   - (a) 実装する。仕様どおりに実装する。OS に依る物は、Windows と同じ失敗の値を返す。窓や外の世界を要求しない。
   - (b) 字形など、フォントの実物が要る物。名前つきの停止のままにする。
   - (c) 使われていない物。理由を書く。
3. (a) を実装する。`lift/runtime/` の `fle_imp_*` を足し、`lift/crates/fe-lift/src/imports_table.rs` の表に行を足す。時間内に終わらない物は、表に「未実装」と書いて、名前つきの停止にする(報告に数を書く)。
4. (b)・(c)、および未実装の物は、`unsupported instruction` で止まらず、名前つきの停止(`fle: import <DLL>!<名前> is not modeled`)にする。どこで止めるかは、`lift/crates/fe-wasmgen/src/lower.rs` を読んで選ぶ。
5. 試験を先に赤にする。実装した import ごとに、戻り値と副作用を確かめる試験を `lift/crates/fe-lift/tests/second_wave_imports.rs`(binary 名 `second_wave_imports`)に書く。既存の `class.rs` の形に倣う。今の木では、この import は未実装なので赤になる。
6. 受け入れを通す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && node tools/place-plugin-dlls.mjs && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && test -f pm-work/findings/second-wave-imports.md && node tools/fe-lift-test.mjs --test second_wave_imports && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 20,000 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `pm-work/findings/second-wave-imports.md` が在る。
- `node tools/fe-lift-test.mjs --test second_wave_imports` が終了コード 0。
- `bash guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 全 DLL の import の総数と、runtime に無い数、分類ごとの数、(b) 実装した import の名前、(c) 名前つきの停止の一覧と、未実装の数、(d) `second_wave_imports` の `test result` の行、(e) 起動確認の `elapsedMs`、流れ A〜D の 4 行、(f) `bash guard/scan.sh` の結果、(g) 表の場所。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・変数名は書いてよい。
- 窓・ファイル・ネットワークを要求する実装を足さない。仮想の答えを返す。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- `git add` は、変えたファイルを 1 つずつ指定する(`git add -A`・`git add .` は使わない)。
- `lift/` に触れるので、判定が通って push されると、main の `deploy.yml` が動く。報告に書く。

## 作業の進め方

- 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
- ビルド(`fe-build engine`)は、1 回に数十分(観測: 約 17 分から 38 分)。ビルドは 1 回にして、同じ bundle で確かめる。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- 作業を始めてから 3 時間 15 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(判定に約 2 時間掛かる)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- #1434・#1435・#1436・#1437・#1438・#1439・#1442・#1443・#1444・#1446・#1448 は、この Issue を前提にする。
- 同じ束の #{{251}}(プラグインを足すと既存が止まる)と #{{253}}(共有の DLL の行)と、同じファイルを変えることがあり得る。衝突は、後から入る方が直す。
