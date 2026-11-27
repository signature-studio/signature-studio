---
draft: "237"
title: "realmod の名前が 59 字を超えると、engine が起動の前に止まる: 名前の作業域を広げ、key の長さを生成の段で検査する"
type: "Task"
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "021"
blocked_by: []
labels: ["plugin-builtin"]
size: "S"
priority: "High"
project: "signature-studio/3"
---

## 目的

`lift/harness/two/main_two.c` は、engine が読む実モジュールのファイル(`<stem>.<基底>.dll`)を、realmod の表の名前と照合するとき、名前を `char want[64]` に写す。stem が 59 字を超えると、この照合が失敗し、起動は止まる。内蔵のエフェクトの束の DLL には、この上限を超える名前がある(#1442・#1444 の起動確認が止まった)。この Issue は、名前の作業域を広げ、上限を超える key を生成の段(`fe-build`)で止める。

この Issue の目標は 2 つ。
1. 名前が 100 字の realmod を含む起動が、名前の照合で止まらないこと(表の名前と照合でき、起動の時の検査が通ること)。
2. key が 255 字を超える DLL は、`fe-build` の生成の段で、名前と上限を言って止まること(起動の失敗より早く分かる)。

## 背景

- 実測(判定の実行の runner の値。この機械とは別枠): #1442(Fruity Formula Controller・Free Filter・HTML NoteBook)と #1444(Multiband Compressor・Mute 2 ほか)の起動確認が `state exited before the first idle`(約 4 秒)で止まる。
- 原因(作業者の読み。`lift/harness/two/main_two.c` で確かめる): 196 行付近の `char want[64]` に対し、`rm_table_name`(73〜90 行)が `stem + 5 > n` で -1 を返す。key `effects.fruity-formula-controller.fruity_formula_controller_x64.dll` の stem は 63 字、`effects.fruity-multiband-compressor.fruity_multiband_compressor_x64.dll` の stem は 67 字。上限は 59 字。
- 照合の段(195〜219 行)は、渡された全ての実モジュールのファイルについて回る。1 件でも長い名前があると、起動の全体が止まる。止めるときの文(198〜201 行)は「the real module file ... has no base suffix」で、長さが原因だとは分からない。
- 名前の入力の上限は `lift/runtime/fle_load.c` の `LOAD_NAME_MAX`(256)。255 字の key はこれに収まる。
- `lift/runtime/fle_realmod.c` の表の名前は `const char *` で、固定長の配列を持たないと読める(作業者が確かめる)。`lift/runtime/fle_vfs.c` の `module_path` の `nm[128]` は、表の名前ではなく、DLL 自身の export の名前を入れる(作業者が確かめる)。

## 読む物

- `lift/harness/two/main_two.c`(60〜90 行の `rm_table_name`、189〜220 行の照合)
- `lift/crates/fe-build/src/plugins.rs`(`Dll::stem`、`valid_key` と `validate`(327〜365 行)、`mod tests`(939 行〜。`balance()` の使い方と、`validate` の既存の試験(1000 行付近))
- `lift/runtime/fle_realmod.c`(`table_check`、40 行付近)・`lift/runtime/fle_realmod.h`
- `lift/runtime/fle_load.c`(`LOAD_NAME_MAX`)・`lift/runtime/fle_vfs.c`(`module_path`、2049 行付近)
- `lift/crates/fe-lift/tests/wall_fr3.rs`(`preprocess`: main_two.c を `clang -E` で読む試験の書き方)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。`git status` で作業ツリーを確かめる。
1. 試験を先に足して、赤を確かめる。
   - (a) `lift/crates/fe-build/src/plugins.rs` の `mod tests`: 100 字の key の DLL は `validate` が通す。255 字の key も通す。256 字の key は `validate` が落とす。今の木では 256 字も通るので、この試験は赤になる。
   - (b) 新しい試験 `lift/crates/fe-lift/tests/realmod_names.rs`(binary の名前は `realmod_names`)。`lift/harness/two/main_two.c` の名前の照合が、100 字と 255 字の名前の realmod を通すことを確かめる。今の木では `has no base suffix` で止まるので、この試験は赤になる。試験の作り方は作業者が選ぶ。ただし、試験が見る `rm_table_name` は main_two.c の物と同じでなければならない(写すなら、試験の中で main_two.c の本文と比べる)。
2. 直す。
   - `lift/harness/two/main_two.c` の 196 行付近の `char want[64]` を `char want[256]` にする。`rm_table_name` の上限(`stem + 5 > n`)は、この大きさに従う(`n` は `sizeof want` で渡る)。
   - 名前が長すぎて止まる時の文を、基底の接尾辞が無い時の文と分ける。原因が文から分かること(長さの上限と、実際の長さ)。
   - `lift/crates/fe-build/src/plugins.rs` の `validate`: key が 255 字を超える行は、名前と上限を言う Err で止める(例: `plugin <名前> (<key>): the key is 255 characters at most (the real module name is held in a 256 byte buffer of main_two.c)`)。255 は、`want` の大きさ 256 から NUL を除いた字数。
   - `lift/runtime/fle_realmod.c`・`fle_load.c`・`fle_vfs.c` を読み、realmod の名前を入れる固定長の配列で、255 字より小さいものがあれば 256 以上にする。無ければ、報告に「無い」と書く。
3. 試験を通す。受け入れのコマンドで確かめる。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && cargo test --locked --manifest-path lift/Cargo.toml -p fe-build && node tools/fe-lift-test.mjs --test realmod_names && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 20,000 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `cargo test --locked --manifest-path lift/Cargo.toml -p fe-build` が通る。手順 1 の (a) の試験を含む。
- `node tools/fe-lift-test.mjs --test realmod_names` が終了コード 0(新しい失敗が無い)。
- `bash guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 直す前の赤の出力の行(`validate` の試験の行、`realmod_names` の行)、(b) 直した後の `cargo test -p fe-build` の `test result` の行と、`node tools/fe-lift-test.mjs --test realmod_names` の最後の行、(c) `want` の大きさと、key の上限(255 字)、(d) 起動確認の `elapsedMs`、(e) 流れ A〜D の 4 行、(f) `fle_realmod.c`・`fle_load.c`・`fle_vfs.c` の固定長の配列の読みの結果、(g) `bash guard/scan.sh` の結果。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。上限は定数のままにし、オプションにしない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは 1 回にして、同じ bundle で繰り返し確かめて直す。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- 内蔵のエフェクトの残りの束(#1434〜#1448)は、この Issue を前提にする。それらは、この Issue が閉じるまで起動しない。
- `lift/crates/fe-build/src/plugins.rs` は、#1518(関数の入口の seed の複写)も変える。両方が同じファイルを変えるので、後から入る方は取り込みで衝突し得る。衝突は、後から入る方が直す。
