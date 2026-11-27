---
draft: "231"
title: "fe-wasmgen: `movmskps`・`movmskpd`・`pmovmskb` の 64 ビット宛先で、wasm の型の検査が落ちる(`i64x2.bitmask` の結果の型)"
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

`lift/crates/fe-wasmgen/src/lower.rs` の `sse_x2`(4229 行付近)は、xmm から汎用レジスタへ書く命令(`movmskps`・`movmskpd`・`pmovmskb`・`pextrw`)の結果を、64 ビットの宛先(`rax` などの `r64`)へ書くとき、型を合わせていない。その結果、生成された wasm が検査に落ち、`fe-build engine` が hybrid の direct の wasm の最適化(wasm-opt)で止まる。直して、同じ型の落ちが他の命令に無いことを確かめる。

## 背景

- 実測(判定の実行の runner の値。この機械とは別枠): `fe-build engine` が次の行で止まる。`[wasm-validator error in function f_c3baa30] local.set's value type must be correct, on (local.set $1 (i64x2.bitmask (local.get $11)))`・`Fatal: error validating input`・`hybrid: wasm-opt of the direct module effects.frequency-splitter.engine_x64.dll failed`。
- 原因: `sse_x2` の、xmm から汎用レジスタへ書く部分(4252〜4260 行付近。`M::Movmskps` から `M::Pextrw` までの match の後、`return self.set_reg(dst)`)。この match の値は i32(`i32x4.bitmask`・`i64x2.bitmask`・`i8x16.bitmask`・`i16x8.extract_lane_u` の結果)。`set_reg`(294 行付近)は、32 ビットの宛先の拡張は自分でするが、64 ビットの宛先は、呼ぶ側が `i64.extend_i32_u` をしてから呼ぶ前提。この経路は、その拡張をしていない。
- 直しの差分は、リポジトリの `pm-work/findings/fe-wasmgen-movmsk-1395.patch` にある(前回の試行が作った。4 行。`git apply --check` が通ることを確かめてある)。
- 試験に穴がある: `lift/crates/fe-wasmgen/tests/sse_x2_forms.rs` の `cases()` の `gp` の 5 件(`movmskps`・`movmskpd`・`pmovmskb`・`pextrw 5`・`pextrw 0`)は、すべて 32 ビットの宛先(`eax`)で、64 ビットの宛先は試されていない。
- 同じ関数の周辺を読んで確かめた結果(直す必要は無いと読んだ物も、試験で確かめる)。
  - `cvttsd2si`(2814 行付近)と `mx_cvtt`(3975 行付近): 宛先の幅(`wide`)で i64 と i32 を分けて作っている。
  - `movd`・`movq` の xmm から汎用レジスタへの経路(4679 行付近): `movq` は i64 の値、`movd` は i32 の値を作り、`set_reg` へ渡す。
  - `pextrb`・`pextrd`・`pextrq` は `fe-wasmgen` の `src` に経路が無い(このリポジトリの検索で 0 件)。

## 読む物

- `lift/crates/fe-wasmgen/src/lower.rs`(`set_reg` 294 行付近、`cvttsd2si` 2814 行付近、`mx_cvtt` 3975 行付近、`sse_x2` 4229 行付近、`movd`・`movq` の経路 4640〜4690 行付近)
- `pm-work/findings/fe-wasmgen-movmsk-1395.patch`(直しの差分)
- `lift/crates/fe-wasmgen/tests/sse_x2_forms.rs`(`gp` の枠と、`cases()`。宛先は 32 ビットだけ)
- `lift/crates/fe-wasmgen/tests/simd_int_forms.rs`(同じ命令の別の試験)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。`git status` で作業ツリーを確かめる。
1. 試験を先に足して、赤を確かめる(`lift/crates/fe-wasmgen/tests/sse_x2_forms.rs` の `cases()`)。`gp` の枠で、64 ビットの宛先の形を、32 ビットの既存の形の隣に足す: `48 0f 50 c0`(movmskps rax, xmm0)・`66 48 0f 50 c0`(movmskpd rax, xmm0)・`66 48 0f d7 c0`(pmovmskb rax, xmm0)・`66 48 0f c5 c0 05`(pextrw rax, xmm0, 5)。期待値は既存の枠の関数のまま(結果は 0 で拡張された u64)。符号化は、iced-x86 で decode して、意図した命令になることを確かめてから使う。直す前に、この試験が赤であることを確かめる(wasm の検査か、node の読み込みで落ちる)。
2. 差分を当てる: `git apply pm-work/findings/fe-wasmgen-movmsk-1395.patch`。差分は、命令の match の後に、宛先が 64 ビットの時だけ `i64.extend_i32_u` を入れる。この差分で、この経路の 64 ビットの宛先の結果がすべて i64 になることを、読んで確かめる。
3. 同じ型の落ちが他の経路に無いかを、上の「同じ関数の周辺を読んで確かめた結果」の 3 項目について、`lower.rs` を読んで確かめる。i32 の値が 64 ビットの宛先へ渡る経路を見つけたら、同じ形(宛先の幅で拡張する)で直し、その命令の 64 ビットの試験を足す。
4. 直した後、試験が通ることを確かめる(受け入れのコマンドで)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && cargo test --locked --manifest-path lift/Cargo.toml -p fe-wasmgen && node tools/fe-lift-test.mjs && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 12,500 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `cargo test -p fe-wasmgen` が通る。64 ビットの宛先の試験を含む。
- `node tools/fe-lift-test.mjs` の新しい失敗が 0 件(host 依存の失敗は、新しい失敗に数えない)。
- `bash guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 直す前の 64 ビットの試験の赤の出力の行、(b) 差分を当てた後の、この経路の 64 ビットの結果の型の読み、(c) 周辺の 3 項目の読みの結論(同じ型の落ちが見つかったか、見つかった時の直し)、(d) `fe-build engine` が hybrid の段を越えたか(越えた時の出力の行。越えなければ、止まった行)、(e) 流れ A〜D の 4 行、(f) `cargo test -p fe-wasmgen` と `node tools/fe-lift-test.mjs` の最後の行。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは 1 回にして、同じ bundle で繰り返し確かめて直す。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- 内蔵のエフェクトの束 #1437(Frequency Splitter・Fruity 7 Band EQ・Fruity Bass Boost)は、この Issue を前提にする。#1437 は、この Issue が閉じるまで起動しない。
