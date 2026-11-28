---
draft: "254"
title: "`cargo test -p fe-build` が main で 2 件落ちる: `the_second_wave_is_160_dlls_and_fits_the_image_area`(`File exists`)と `the_second_wave_with_its_data_fits_the_limits`(`fruity-delay-bank` の guest path が一意でない)"
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

main の `cargo test --locked --manifest-path lift/Cargo.toml -p fe-build` が、`lift/crates/fe-build/src/plugins.rs` の 2 件で落ちる。この 2 件は、試験の側の誤りで、製品の挙動が原因ではない見込み。直すのは試験だけ。試験の検査の中身は弱めない。

`cargo test -p fe-build` を受け入れに持つ Issue は、この 2 件が直るまで通らない。今の open の Issue で `cargo test -p fe-build` を受け入れに持つのは #1539 だけで、#1539 はこの Issue が閉じるまで起動しない(blocked_by に足した)。急ぐ。

## 背景

- 観測(#1539 の作業者の報告。runner の値。この機械とは別枠)。基底の main(#1539 の変更を外した木)でも同じ 2 件が落ちた。`--test-threads=1` でも落ちる。結果は `test result: FAILED. 32 passed; 2 failed`。
  - `plugins::tests::the_second_wave_is_160_dlls_and_fits_the_image_area`: `plugins.rs:1240` で `File exists`(`fs::create_dir` の失敗)。
  - `plugins::tests::the_second_wave_with_its_data_fits_the_limits`: `plugins.rs:1436` で `plugin fruity-delay-bank (effects.fruity-delay-bank.fruity_delay_bank_x64.dll): the guest path is not unique`。
- 原因の見立(この機械で、cargo は使わずに読みと、試験の計算を JavaScript で同じ手順に写して確かめた。実測はその模擬の値。cargo での確認は、この Issue の作業で行う):
  1. 2 件の試験は、第 2 波の束(`tools/plugin-layout/wave2-plugins.mjs` が作る 130 の folder・159 の DLL)を、`lift/plugins/` に無い物として書かれた(#1478 の 8b4821e8・#1479 の 65d6e5fb)。
  2. その後、次の 4 本の commit で、第 2 波の 12 の folder が `lift/plugins/` に入った: 521bbcc3(#1440。fruity-delay・fruity-delay-bank・fruity-fast-dist)、aee8b188(#1441。fruity-fast-lp・fruity-flanger・fruity-flangus)、4aade125(#1445。fruity-notebook-2・fruity-panomatic・fruity-parametric-eq)、d5ddf948(#1447。fruity-phaser・fruity-reeverb・fruity-reeverb-2)。試験は、その folder が在る場合を扱っていない。
  3. `the_second_wave_is_160_dlls_and_fits_the_image_area` の `write_wave_folders` は、コピーの `lift/plugins/` に在る folder にも `fs::create_dir` を呼ぶ。表の 130 の folder のうち 12 は、同じ名前でコピーに在る(模擬で確かめた。`effects-fruity-delay` など 12 件)。そのため `File exists` で止まる。
  4. `the_second_wave_with_its_data_fits_the_limits` は、`lift/plugins/` の DLL の行に、表の 159 行を `dlls.extend(...)` で足す。同じ guest path の行が 2 つになり、`validate` が落ちる。模擬の出力は、報告の文と同じになった。報告される行が `fruity-delay-bank` なのは、key の順で表の行(`effects.effects-fruity-delay-bank...`)が先に来て、同じ guest path の 2 つ目(folder の行)で `validate` が止まり、その中で key の順に最初に重なるのがこれだから。
  5. 同じ試験ファイルの `second_wave_dlls`(65d6e5fb で足された)は、コピーでも `lift/plugins/` でも、guest path で一度だけ数える。模擬では、この数え方で、164 の DLL・大きさの和 748,683,264 B(748,158,976 + Reverb の 524,288)・全ての DLL に SizeOfImage の行あり・像の配置が上限の内、になった(推定。cargo では未確認)。
- 直した後の数の見込み(模擬の数え上げ。推定。cargo では未確認): seed 表の行 1,629(上限 `VS_MAX_ENTRIES` 2,048)、lazy の seed 行 777(上限 `LAZY_MAX_STREAMS` と `VL_REG_MAX` の小さい方 1,024)、pack.args の lazy の項目 1,273(上限 `manifest.ts` の `lazy` の `maxItems` 1,536)。

## 読む物

- `lift/crates/fe-build/src/plugins.rs`: `mod tests` の中の `the_second_wave_is_160_dlls_and_fits_the_image_area`・`write_wave_folders`・`gen_form`・`second_wave_dlls`・`first_wave_folder`・`the_second_wave_with_its_data_fits_the_limits`・`wave2_data_tsv`・`wave2_dlls`。行番号は動くので `grep -n` で探す。
- `git show 65d6e5fb --stat` と `git show 8b4821e8 -- lift/crates/fe-build/src/plugins.rs`(試験の形の変遷)。
- `tools/plugin-layout/wave2-plugins.mjs`(表の作り方。`node tools/plugin-layout/wave2-plugins.mjs <出力の folder>` で作れる。`lift/plugins` を出力にしてはいけない)。
- `lift/plugins/*/plugin.tsv`(第 2 波の 12 の folder は、この木に在る)・`inputs/plugin-image-sizes.tsv`。

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。`git status` で作業ツリーを確かめる。
1. 赤を確かめる。直す前に、`cargo test --locked --manifest-path lift/Cargo.toml -p fe-build the_second_wave` を実行し、2 件が落ちる行(`FAILED`・`panicked` の行)を報告に貼る。落ちなければ、その出力を報告して止まる。
2. 試験を直す。
   - `write_wave_folders`: コピーに同じ名前の folder が既に在れば、その folder を残して、無い folder だけを書く(`fs::create_dir` の前に `Path::new(...).exists()` で飛ばす)。コピーの folder の中身は `lift/plugins/` の物のままにする。
   - `the_second_wave_with_its_data_fits_the_limits`: `read_plugins_from(plugins_dir())` の結果に `wave2_dlls(...)` を足す 3 行(`extend`・`sort_by`・`validate`)を、`second_wave_dlls(plugins_dir(), &wave2_dlls(...)).unwrap()` の 1 行にする(第 1 の試験と同じ。guest path で一度だけ数える)。`let mut` が要らなくなれば外す。
   - 試験の assert(数・値・上限)は消さない・緩めない・`#[ignore]` を付けない。
   - 上の見込みと違う所(例: 試験の値が、模擬の数と合わない)が出たら、どの値が何と違ったかを報告に書く。表(`tools/plugin-layout/wave2-plugins.mjs` の出力・`inputs/`)が誤りだと確かめた場合だけ、その表を直し、理由を報告に書く。
   - 直した後に、seed・lazy・pack の上限の assert が落ちたら、上限を上げずに、数と上限の出所(`lift/harness/vfs_seed.h` の `VS_MAX_ENTRIES`・`lift/harness/vfs_lazy.h` の `VL_REG_MAX`・`lift/runtime/fle_vfs.c` の `LAZY_MAX_STREAMS`・`app/src/bundle/manifest.ts` の `lazy` の `maxItems`)を報告して止まる。
3. 通す。受け入れのコマンドで確かめる。`cargo test --locked --manifest-path lift/Cargo.toml -p fe-build` の全体の `test result` の行を報告に貼る。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && cargo test --locked --manifest-path lift/Cargo.toml -p fe-build && bash guard/scan.sh
```

判定の基準:
- `cargo test --locked --manifest-path lift/Cargo.toml -p fe-build` が通る(`test result: ok`)。特に `the_second_wave_is_160_dlls_and_fits_the_image_area` と `the_second_wave_with_its_data_fits_the_limits` が通る。
- 起動確認(`boot.json`)が engine-idle・standIn が false・error が null・elapsedMs が 20,000 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `bash guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 1 行目は、何を実行し、どの出力を観測したかで始める(例: 「実エンジン: cargo test -p fe-build の the_second_wave の 2 件を実行し、…」)。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 直す前の赤の出力の行(落ちた試験の名前と、落ちた行)、(b) 直した後の `test result` の行、(c) 試験の変更の要約(どの行を変えたか。消した行と足した行の数)、(d) 試験の数の値(160 の名前の DLL 数 164 等)、(e) seed・lazy・pack の実測の数と、上限との比、(f) 起動確認の `elapsedMs`、流れ A〜D の 4 行、(g) `bash guard/scan.sh` の結果。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- `lift/plugins/`・`ci/`・`inputs/` は変えない(上の例外を除く)。製品の関数(`plugins.rs` の本体)は変えない。変えるのは `mod tests` の中だけ。
- 試験を弱めない: assert を消さない・緩めない・`#[ignore]` を付けない・上限を上げない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- `git add` は、変えたファイルを 1 つずつ指定する(`git add -A`・`git add .` は使わない)。

## 作業の進め方

- 作業の最初に `date -u` を記録する。作業を始めてから 3 時間 15 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(判定に約 2 時間掛かる)。
- ビルドは 1 回にして、同じ bundle で繰り返し確かめて直す。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- #1539(`ildsp_x64.dll` の行)は、`cargo test -p fe-build` を受け入れに持つ。この Issue が閉じるまで起動しない(blocked_by に足した)。#1539 の `failed` は、PM が外す。
- 第 2 波の束(#1434・#1435・#1436 など)は、`cargo test -p fe-build` を受け入れに持たないので、この 2 件の赤では止まらない。
- `lift/crates/fe-build/src/plugins.rs` は、#1537(溢れている上限)も変え得る。両方が同じファイルを変えるので、後から入る方が取り込みで衝突を直す。

