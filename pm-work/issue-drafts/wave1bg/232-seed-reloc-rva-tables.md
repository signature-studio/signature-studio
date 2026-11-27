---
draft: "232"
title: "関数の入口の seed を足す仕組みを入れる: `--reloc-seeds`・`--rva-tables`(fe-lift・fe-wasmgen)と、plugin の seed の複写(fe-build)"
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

DLL の関数の入口(番地)は、`.pdata` の関数の表から決まる。`.pdata` の無い葉の関数や、MSVC の switch の表の飛び先は入口にならないので、その先への間接呼び出しは「既知の関数の入口ではない」で止まる。この Issue は、入口を足す 3 つの仕組みを入れる。(1) 再配置の値から入口を取る `--reloc-seeds`、(2) 表の行から入口を取る `--rva-tables`(fe-lift と fe-wasmgen の CLI。ビルドの道具の引数)、(3) plugin の DLL を、seed の置き場へ複写する `seed_copies`(fe-build)。#1512(LAMEenc)の止まりを直す前提になる。

## 背景

- 実測(判定の実行の runner の値。この機械とは別枠): #1512(LAMEenc)の判定の前の run で、MP3 の書き出しが `fle: indirect call to 0xcf65a4d is not a known function entry` で止まる。RVA は `0x5a4d`。`.pdata` の無い葉の関数で、DIR64 の再配置にも、`lea` にも、データ中の値にも現れない。実行時に計算される値である。
- 手がかり: `0x31be0` 付近の `[rbx+rcx*8+0xc]` の RVA に ImageBase を足して `call r8` する形。MSVC の switch の表の形(RVA の表)。
- 前回の試行(#1395 の ILSvg)が、同じ型の止まりを直す仕組みを作った。その差分は、リポジトリの `pm-work/findings/fe-seeds-1395.patch` にある。`git apply --check` が main に通ることを確かめてある。#1395 の本体の変更は含まない。差分の中身は次のとおり。
  - `lift/crates/fe-decode/src/relocseeds.rs`: `rva_tables`(`RVA COUNT` の行の表のファイルを読む。RVA は 16 進で、`0x` は付けても省いてもよい。`#` はコメント)と、`rva_table_seeds`(表の各エントリについて、`ImageBase + エントリ` が `.text` の中なら入口にする。昇順・重複なし)。
  - `lift/crates/fe-lift/src/main.rs`: `--reloc-seeds`(再配置の slot が持つ、`.pdata` の無いコード番地を入口にする。`reloc_seeds(...).not_function`)と、`--rva-tables FILE`。
  - `lift/crates/fe-wasmgen/src/main.rs`: 同じ 2 つの引数。
  - `lift/crates/fe-build/src/plugins.rs`: `seed_copies`。作業の木が plugin の行より前に作られた時、`lift/out-app/host/lift/out-plugins/<key>` の実物を、`seed_file()` の場所へ、sha256 を照合して複写する。PM の読みでは、この欠けが #1512 の試行の `pack-seed` の ENOENT の原因。
- main に、ILSvg の `plugin.tsv` と `rva-tables.txt` は無い。前例として使えない。plugin の args 列から fe-lift・fe-wasmgen へ引数が渡る経路は、コードを読んで確かめる(下の「やること」3)。

## 読む物

- `pm-work/findings/fe-seeds-1395.patch`(差分。上の 4 ファイル)
- `lift/crates/fe-build/src/plugins.rs`: `parse_plugin` の args 列、`seed_file`・`host_file`、`module_row`(fe-lift の引数)、`hyb_row`(hybrid の fe-wasmgen の引数)、lists の段の fe-wasmgen の引数、`mod tests` の既存の試験(`module_row` の試験の書き方)
- `lift/crates/fe-decode/tests/extra_seed.rs`・`lift/crates/fe-decode/tests/pdata_seeds.rs`(既存の seed の試験の書き方)
- `lift/crates/fe-lift/src/main.rs`(`lift_main` の引数の解析)・`lift/crates/fe-wasmgen/src/main.rs`(`compile_cli` の引数の解析)
- `tools/fe-lift-test.mjs`(fe-lift の試験の走らせ方)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。`git status` で作業ツリーを確かめる。
1. 試験を先に足して、赤を確かめる。差分を当てる前の main の木に、次の試験を書く(関数が無いので、コンパイルが落ちる。それを赤と数える)。
   - `fe-decode`(`relocseeds.rs` の `mod tests` か、`tests/` の新しい試験): `rva_tables` の構文(`0x` つき・なし・`#` のコメント・空行は通る。不正な行(RVA が 16 進でない・件数が無い)はエラー)。`rva_table_seeds`(表の各エントリが `.text` の中なら入口、外なら捨てる。昇順・重複なし)。
   - `fe-build`(`plugins.rs` の `mod tests`): `seed_copies` の試験。seed の置き場に在れば触らない。無ければ key の実物から複写する。sha256 が違えばエラー。
   - `fe-lift`・`fe-wasmgen` の CLI の引数の解析の試験があれば、`--reloc-seeds`・`--rva-tables` を足す。
2. 差分を当てる: `git apply pm-work/findings/fe-seeds-1395.patch`。
3. plugin の args の経路の試験を、`plugins.rs` の `mod tests` に足す。行の args 列に `--pdata --reloc-seeds --rva-tables lift/plugins/<名前>/rva-tables.txt` を持つ DLL の行が、`module_row`(fe-lift の引数)・`hyb_row`(hybrid の fe-wasmgen の引数)・lists の段の fe-wasmgen の引数に、そのまま入ることを確かめる。相対のパスの基準は、fe-build の実行の位置(リポジトリの根)。足りなければ直す。
4. 試験を通す(受け入れのコマンドで)。`lift/plugins/` の行は足さない(#1512 が足す)。`tools/e2e/` に触れない。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && cargo test --locked --manifest-path lift/Cargo.toml -p fe-decode -p fe-build -p fe-wasmgen && node tools/fe-lift-test.mjs && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 20,000 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `cargo test -p fe-decode -p fe-build -p fe-wasmgen` が通る。手順 1 と 3 の試験を含む。
- `node tools/fe-lift-test.mjs` の新しい失敗が 0 件(host 依存の失敗は、新しい失敗に数えない)。
- `bash guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 差分を当てる前の試験の赤の出力の行(コンパイルの失敗の行、または落ちた試験の行)、(b) `cargo test -p fe-decode -p fe-build -p fe-wasmgen` の `test result` の行、(c) args の経路の試験の結果(足りなかった所と、直した所)、(d) 流れ A〜D の 4 行、(e) `node tools/fe-lift-test.mjs` の最後の行、(f) `bash guard/scan.sh` の結果。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。`--reloc-seeds`・`--rva-tables` は、ビルドの道具(fe-lift と fe-wasmgen)の引数で、製品の旗ではない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは 1 回にして、同じ bundle で繰り返し確かめて直す。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- #1512(LAMEenc)は、この Issue を前提にする。#1512 は、この Issue が閉じるまで起動しない(#1512 に blocked_by を足す)。
