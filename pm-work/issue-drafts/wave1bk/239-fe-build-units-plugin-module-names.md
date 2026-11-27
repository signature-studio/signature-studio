---
draft: "239"
title: "`fe-build`: Edison の build が、`the units of the compile lists are not written by gen: [\"m10-01.c\", \"m10-02.c\", \"m10-03.c\"]` で止まる"
type: "Task"
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "021"
blocked_by: []
labels: ["plugin-builtin"]
size: "M"
priority: "High"
project: "signature-studio/3"
---

## 目的

`fe-build engine` は、gen の出力の単位を、静的な一覧(`lift/harness/two/plain-units.txt`・`lift/harness/two/ai-units.txt`)と照らして検査する(`lift/crates/fe-build/src/plugins.rs` の `check_units`)。静的な一覧に在る単位が gen の出力に無ければ、build は止まる。

今の `plain-units.txt` は、プラグインの module の番号(`m<K>`)を固定で書いている。プラグインを足すと、module の番号は key の順で決まり直すので、番号と単位が合わなくなる。内蔵のエフェクトの Edison(本体と補助の 2 つの DLL)を足した build で、この検査が止まった。

この Issue の目標は 2 つ。
1. 静的な一覧に、プラグインの module の単位(今は `m9-00.c`〜`m10-03.c` の 8 行)を書かない。プラグインの単位は、gen の出力から求める今の経路(`check_units` が見つけ、`compile-extra` がコンパイルする)で拾う。静的な一覧の検査(一覧の単位が gen の出力に無いと止まる)は残す。
2. 2 つの DLL を持つ plugin(本体と補助)で、units の検査が通ることを、`fe-build` の単体試験で確かめる。

この Issue は #1435(Edison・Effector・Emphasis)の前提。

## 背景

- 実際の出力(Actions の runner の出力): 内蔵のエフェクトの束(Edison・Effector・Emphasis)を入れた build で、`gen-all: 18 modules, 19 tables merged` と `stage gen` の後の `stage units` で、`fe-build: the units of the compile lists are not written by gen: ["m10-01.c", "m10-02.c", "m10-03.c"]` が出て止まった。
- 仕組み(コードで確かめる):
  - module の番号: `lift/crates/fe-lift/src/gen_all.rs` の 97 行付近で、k 番目(0 始まり)の module の単位の接頭辞を `m{k+1}` にする(`fle` の単位は `fle-NN.c`)。
  - module の並び: `lift/crates/fe-build/src/plugins.rs` の `read_rows_from`(392 行付近)は、プラグインの DLL の行を key の順に並べる。DLL 1 本ごとに 1 つの module を持つ(本体と補助は別の番号)。
  - 今の main の並び: m9 = `effects.fruity-balance`、m10 = `effects.fruity-filter`、m11 = `generators.sytrus`。`plain-units.txt` の 43〜50 行の `m9-00.c`〜`m10-03.c` は、この 2 つの単位の名残。
  - Edison を足すと、key の順で `effects.edison.edison_x64.dll`(m9)と、その補助 `effects.edison.testdll_x64.dll`(m10)が前に入り、fruity-balance は m14、fruity-filter は m15 になる。静的な `m10-01.c`〜`m10-03.c` は gen が書かない(エラーの一覧が示す。`m10-00.c` は書かれている)ので、検査が止まった。
  - 検査: `check_units`(888 行付近)は、静的な一覧の単位が gen の出力に無いと止める。gen の出力にあって一覧に無い `m<K>-<NN>.c` は、`is_plugin_unit`(909 行付近)でプラグインの単位と見なし、`find_units`(`lift/crates/fe-build/src/main.rs` の 541 行付近)が `compile-extra` と link に渡す。
  - 静的な一覧は、`main.rs` の 376 行付近と 400 行付近で読まれ、`tools/engine-cost/build-variant.mjs` の 163 行付近でも読まれる。
- `plugin.tsv` の 2 行(本体 1・補助 1)は、`lift/plugins/` の書式に従う。Edison の `plugin.tsv` は main に無い。この Issue では `lift/plugins/` に新しい plugin を足さない。試験は、偽の `plugin.tsv`(2 行)で行う。Edison の実物は、#1435 の再試行で確かめる。

## 読む物

- `lift/crates/fe-build/src/plugins.rs`: `read_rows_from`(392 行付近)、`check_units`(888 行付近)、`is_plugin_unit`(909 行付近)、`mod tests`(938 行〜。`parse_plugin` の使い方、`the_plugins_units_are_the_mk_nn_names_beyond_the_fixed_lists`(1086 行付近)、`BALANCE` の `include_str!` の形)
- `lift/crates/fe-build/src/main.rs`: `plan_with`(356 行〜)、`find_units`(541 行付近)
- `lift/crates/fe-lift/src/gen_all.rs`(97 行付近)
- `lift/harness/two/plain-units.txt`・`lift/harness/two/ai-units.txt`
- `tools/engine-cost/build-variant.mjs`(163 行付近)
- `lift/plugins/fruity-balance/plugin.tsv`(書式の見本)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。`git status` で作業ツリーを確かめる。
1. 試験を先に足して、赤を確かめる。
   - `lift/crates/fe-build/src/plugins.rs` の `mod tests` に、2 つの DLL を持つ偽の plugin(`parse_plugin` で読む。本体 1 行・補助 1 行)を使う試験を足す。試験は、静的な一覧(`plain-units.txt` を `include_str!` で読む。`BALANCE` と同じ形)と、その plugin の module の番号(key の順で決まる K と K+1)の単位が gen の出力に在るとき、units の検査が通ることを確かめる。今の木では、静的な一覧の `m10-01.c`〜`m10-03.c` が gen の出力に無いので、この試験が赤になる。
   - `check_units` は今、`lift/out-app/gen/src` を固定で読むので、試験で使えるように、gen の出力の一覧と静的な一覧を受ける関数に分けてよい(分け方は作業者が決める)。
   - 直す前に、この試験が赤になることを、出力の行で確かめる。
2. 直す。
   - `lift/harness/two/plain-units.txt` から、プラグインの module の行(`m9-00.c`〜`m10-03.c` の 8 行)を消す。それらの単位は、`check_units` の動的な経路で `compile-extra` が拾う。`ai-units.txt` にプラグインの行が無いことを確かめる。
   - `tools/engine-cost/build-variant.mjs`(163 行付近)は同じ一覧を読み、`compile-extra` の経路を持たない。一覧から行を消すと、この道具はプラグインの単位を扱わなくなる。道具を直すか、報告に影響を書くかを作業者が決める。
   - 検査そのもの(静的な一覧の単位が gen の出力に無いと止まる)は弱めない。
   - 製品の挙動は、`plugins.rs`(試験の分け方を含む)と `plain-units.txt` の 2 点だけを変える。
3. 試験を通す。受け入れのコマンドで確かめる。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && cargo test --locked --manifest-path lift/Cargo.toml -p fe-build && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 20,000 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `cargo test --locked --manifest-path lift/Cargo.toml -p fe-build` が通る。手順 1 の試験を含む。
- `bash guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 直す前の赤の出力の行(試験の名前と、落ちた行)、(b) 直した後の `cargo test -p fe-build` の `test result` の行、(c) 静的な一覧から消した行の数と内容、`units:` の行(プラグインの単位として拾った名前)、(d) `tools/engine-cost/build-variant.mjs` の扱い、(e) 起動確認の `elapsedMs`、(f) 流れ A〜D の 4 行、(g) `bash guard/scan.sh` の結果。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- `lift/plugins/` に新しい plugin(Edison など)を足さない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは 1 回にして、同じ bundle で繰り返し確かめて直す。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- この Issue は #1435(Edison・Effector・Emphasis)の前提。#1435 は、この Issue が閉じるまで起動しない。
- `lift/crates/fe-build/src/plugins.rs` は、#1523(realmod の名前の上限。`validate` と `mod tests`)も変える。両方が同じファイルを変えるので、後から入る方は取り込みで衝突し得る。衝突は、後から入る方が直す。
