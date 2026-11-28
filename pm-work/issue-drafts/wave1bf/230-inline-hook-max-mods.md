---
draft: "230"
title: "インラインフックの登録の上限を直す: `IH_MAX_MODS` が 8 で、第 2 波の DLL を 3 本足すと、engine が起動の前に止まる"
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

`lift/runtime/fle_vmap.c` の、インラインフック(関数の先頭 5 バイトを `jmp rel32` で書き換えて使う hook 点)の登録は、hook 点を持つモジュールを 8 個までしか受け付けない。9 個目で `fle_stop` が呼ばれ、engine は起動の前に止まる。内蔵のエフェクトの残りの DLL の束(#1434〜#1448)が入ると、この上限を超えるので、上限を直す。

この Issue の目標は 2 つ。
1. hook 点を持つモジュールを 300 個登録しても止まらず、各モジュールの hook 点について `fle_hook_target` が、その点の答え(0 か、書かれた `jmp rel32` の行き先)を返すこと。
2. 上限を超えた時は、止めた文に、件数と上限が入ること(原因が一目で分かるように)。

## 背景

- 実測(判定の実行の runner の値。この機械とは別枠): `fe-build engine` と `fe-build bundle` は通る。`tools/boot-check/boot-bundle.mjs` が `state: exited`・`error: "state exited before the first idle"`(約 3 秒)で終わる。engine のログに `[err] fle: inline hook: more modules with hook points than the runtime holds` が出る。
- 原因は `lift/runtime/fle_vmap.c` の 401 行付近の `#define IH_MAX_MODS 8u`(モジュール数の上限)と、402 行付近の `#define IH_MAX_PAGES 64u`(VirtualProtect で記録する hook 点のページの上限)。`fle_hook_register`(423 行付近)は、hook 点を持つ各 DLL の生成された C の constructor(`fle_hook_init_<基底>`。`lift/crates/fe-lift/src/hook.rs` が書く)から、起動の時に 1 モジュールずつ呼ばれる。上限に達すると、そこで止まる。
- 今の登録は上限の 8 に近い(内蔵の FLEngine・Fruity Balance・Filter・Sytrus などが hook 点を持つ)。後続の束は、約 130 本の DLL を足す。全部が hook 点を持つとは限らないが、上限は桁を上げないと足りない。
- 静的な配列の大きさ: `ih_mod` は 1 件 32 バイト前後、`ih_page` は 1 件 16 バイト。上限を 512 と 4096 にすると、配列は合わせて約 80 KB になる。起動の時間に効かないことを、起動確認の `elapsedMs` で確かめる。
- 探索の時間: `fle_hook_target`(440 行付近)は関数の入口ごとに呼ばれるが、表を探さない。番地の 5 バイトを読んで比べるだけ。線形の走査は `ih_image_at`(475 行付近。VirtualProtect の経路だけで、登録の数だけ回る)と `ih_point_at`(1 モジュールの hook 点の数だけ回る)。

## 読む物

- `lift/runtime/fle_vmap.c`(395〜535 行付近: 登録 `fle_hook_register`・`fle_hook_target`・`ih_image_at`・`ih_protect`)・`lift/runtime/fle_vmap.h`(hook 点の注釈の説明)
- `lift/crates/fe-lift/src/hook.rs`(生成される constructor の形。`fle_hook_register(基底, SizeOfImage, 表, 件数)`)
- `lift/crates/fe-lift/tests/hook_points.rs`(hook の試験。1 の節は生成の試験、2 の節は native の driver。`driver()` は runtime の `fle_*.c` と `tests/support/` の 2 本を clang で link して走らせる)
- `tools/fe-lift-test.mjs`(`--test <binary>` で、1 つの試験の binary だけを走らせる。binary の名前は `hook_points`)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。`git status` で作業ツリーを確かめる。
1. 試験を先に足して、赤を確かめる(`lift/crates/fe-lift/tests/hook_points.rs`)。300 モジュールを登録する試験を書く。C の driver は、`driver()` と同じ clang の呼び方で、別の main を持つ物を作る。300 個のイメージを、互いに重ならない番地に map し(`lift/runtime/fle_sys.h` の `FLE_MAX_APP_ADDR` より下)、各イメージに hook 点を 1 つ置く(関数の入口の 5 バイトに `mov eax, imm32` などを書く)。各イメージを `fle_hook_register` で登録する。そのあと次を確かめる: 各モジュールの hook 点に `jmp rel32` を書くと `fle_hook_target(va, orig5)` が、その行き先を返す。書かない時は 0 を返す。最後のモジュールの hook 点で VirtualProtect が成功する。driver は止まらず、終了コード 0 で、最後に `ok` を出す。今の上限 8 では、9 個目の登録で止まるので、この試験は赤になる。赤を確かめてから直す。
   - 上限の文の試験も足す: 上限を超えるまで登録を続け、止まった時の stderr に `more modules with hook points` と、件数と上限が入ることを確かめる。上限の数は driver が知らなくてよい。driver が登録できた件数を stdout に出し、stderr の文の件数と、止まる前の件数が一致することを確かめる。
2. 上限を直す(`lift/runtime/fle_vmap.c`): `IH_MAX_MODS` を 512 以上、`IH_MAX_PAGES` を 4096 以上にする。上限は定数のままにし、オプションにしない。
3. 上限に達した時の文: `fle_hook_register` の `fle_stop` の文に、止めた登録のイメージの基底と、登録の件数と上限の値を入れる。`fle_vmap.c` の中で、既に `snprintf` で `why` を組む形を使っているので、その形に倣う。
4. 探索の時間: 上の「探索の時間」の読みを、`fle_vmap.c` の現物で確かめる。`fle_hook_target` には、表を探す処理を足さない。`ih_image_at` の線形の走査が 512 件でも、VirtualProtect の 1 回の呼び出しに十分に速いと読んで確かめられれば、そのままにする。二分探索は、確かめた上で必要な時だけ入れる。
5. 直した後、`lift/crates/fe-lift/tests/hook_points.rs` の試験(1 と、上限の文の試験)が通ることを、`node tools/fe-lift-test.mjs --test hook_points` で確かめる。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && node tools/fe-lift-test.mjs --test hook_points && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 12,500 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `node tools/fe-lift-test.mjs --test hook_points` が終了コード 0(新しい失敗が無い)。300 モジュールの試験と、上限の文の試験を含む。
- `bash guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 直す前に、300 モジュールの試験が赤だったことの出力の行、(b) 直した後の `node tools/fe-lift-test.mjs --test hook_points` の最後の行、(c) `IH_MAX_MODS`・`IH_MAX_PAGES` の値と、静的な配列の大きさ、(d) 起動確認の `elapsedMs`、(e) 探索の時間の読みの結論(二分探索を入れたかどうか、理由)、(f) 流れ A〜D の 4 行。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。上限はオプションにしない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは 1 回にして、同じ bundle で繰り返し確かめて直す。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- 内蔵のエフェクトの残りの束(#1434〜#1448)は、この Issue を前提にする。それらは、この Issue が閉じるまで起動しない。
