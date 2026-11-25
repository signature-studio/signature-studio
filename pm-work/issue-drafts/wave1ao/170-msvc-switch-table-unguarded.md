---
draft: "170"
title: "MSVC の switch 表の飛び先(ガード `cmp`・`ja` が無い形)を、lift が認識する: 『indirect call to … is not a known function entry』で止まらない"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "022"
blocked_by: ["157"]
labels: ["plugin-builtin", "resources"]
size: L
priority: High
project: signature-studio/3
---

## 目的

内蔵の音源 Sytrus(`Sytrus_x64.dll`)の関数の途中に在る MSVC の switch 表の飛び先を、lift が認識しない。このため、Sytrus を使う demo を開くと、「indirect call to 0xc9cf273 is not a known function entry」で engine が止まる。この Issue は、この形の表を lift が認識できるようにし、止まりを無くす。

目標:
1. `0x44f273`(Sytrus の基底からの RVA)を含む、Sytrus の認識できない `jmp reg` の全件の形を、`pm-work/findings/switch-tables.md` に形ごとに書く。
2. ガードの無い形の表を、表の範囲から N を決めて認識する。誤認識(飛び先が関数の範囲の外、または命令の途中)を 0 件にする。
3. Sytrus・FLEngine・OTT・Balance で、認識できない表が前より増えない。

## 背景

- 事実(#1400 の 2 回目の試行。run 45518902053): Sytrus の wasm を起動の後に読み込んだ後、synth と olbaid の demo が `indirect call to 0xc9cf273 is not a known function entry` で止まる。ゲストの VA `0xc9cf273` は、Sytrus の基底 `0xc580000`(wasm の名前 `sytrus_x64.c580000` の番号)に対する RVA `0x44f273`。関数 `0x44f130` の途中で、関数の先頭ではない。
- 飛びの形: `mov eax,[rcx+rax*4+0x4520d0]; add rax,rcx; jmp rax`。`mov eT,[B+I*4+T2]; add rT,B; jmp rT` の形で、直前に `cmp`・`ja` のガードが無い(または、ガードが別の基本ブロックに在る)。
- 認識の仕組み: `lift/crates/fe-decode/src/jumptab.rs` の先頭の注釈(decision 0089・0094・0095・0109)。`recognize_msvc` は `[cmp idx,N; ja D;]` の形から N を決める。`recognize_msvc_scheduled` は、ガードが離れている形を、その入り口(`derive_scheduled_guard`)から求める。N が決まらない形は表を採らず、`Unresolved` のまま残り、`jumptab_rejected` に記録される。
- 停止の経路: 残った間接の飛びは、実行時に既知の関数の入口だけへ飛ぶ。入口でない飛び先では `is not a known function entry` で止まる。
- 参考(報告。main には無い): 別の `jmp reg` の表は、73 件のうち 40 件だけ列挙できた(ILSvg。#1395 の作業)。33 件は表の形を認識できない。
- 既存の試験: `lift/crates/fe-decode/tests/msvc_switch.rs`・`lift/crates/fe-decode/tests/jumptab.rs`・`lift/crates/fe-lift/tests/msvc_switch.rs`・`lift/crates/fe-lift/tests/ldst_switch.rs`。実物の DLL を読む試験は、`FLENGINE_PE`(無ければ `binaries/win64/FLEngine_x64.dll`)を使う(`lift/crates/fe-decode/tests/adjustor.rs` が手本)。
- Sytrus の DLL は、取り出し後に `lift/out-plugins/sytrus_x64.dll`(6,269,440 B。sha256 `e1395e97…`)。

## 読む物

- `lift/crates/fe-decode/src/jumptab.rs`(先頭の注釈と、`recognize_msvc`・`recognize_msvc_scheduled`・`derive_scheduled_guard`・`resolve_msvc`・`verify_msvc`)
- `lift/crates/fe-decode/src/indirect.rs`・`lift/crates/fe-decode/src/lib.rs`(`jumptab_rejected` と `kept unresolved` の記録。625 行付近)
- `lift/crates/fe-lift/src/jumptab.rs`・`lift/crates/fe-lift/src/indirect.rs`
- 試験: `lift/crates/fe-decode/tests/msvc_switch.rs`・`lift/crates/fe-decode/tests/jumptab.rs`・`lift/crates/fe-lift/tests/msvc_switch.rs`
- `inputs/plugin-classes.tsv`・`inputs/plugin-hooks.tsv`(Sytrus の行)
- `objdump -d`(関数 `0x44f130` から `0x44f273` までの命令列と、表の位置)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. `0x44f273` の形を特定する。`objdump -d` で、関数の先頭(`0x44f130`)から `0x44f273` までの命令を読み、表の位置(`0x4520d0` の RVA)と、表の値の範囲を確かめる。前の基本ブロックに `cmp`・`ja` が在るか、`cmp` が `sub` などに畳まれているか、を記録する。
2. 基準値を作る(直す前に)。Sytrus・FLEngine・OTT・Balance の、認識できなかった `jmp reg` の全件を、形ごとに分けて `pm-work/findings/switch-tables.md` の表にする(件数・形・例の VA)。取り方は、`fe-lift` の出力の `jumptab_rejected` と `kept unresolved`(`lift/crates/fe-decode/src/lib.rs`)。この表の件数が、手順 5 の基準になる。
3. 認識を広げる。ガードが無い形でも、表の範囲から N を決める。範囲の候補: (a) 表の中の値(飛び先)が、同じ関数の `.pdata` の範囲に入る間。(b) 隣の表の始まり。(c) セクションの終わり。N を決めたら、既存の `resolve_msvc` と同じ飛び先の検査(関数の範囲・命令の先頭・`.text` の中)を必ず掛ける。N を大きく取り過ぎると、飛び先でない値を関数の入口として扱う危険があるので、誤認識を 0 件にする。
4. 試験を先に書く(red を確かめてから実装)。Sytrus の `0x44f273` の表を認識する試験。実物の DLL で、認識の増分と誤認識 0 件を確かめる試験。
5. 道具 `tools/switch-tables/check.mjs` を、この Issue で作る。`pm-work/findings/switch-tables.md` の基準値と、`fe-lift` の出力を比べ、判定の基準(下)を検査する。
6. 受け入れを通す。通らなければ、作業ツリーを commit して終える。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && cargo test --locked --manifest-path lift/Cargo.toml -p fe-decode -p fe-lift && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/switch-tables/check.mjs && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- `cargo test -p fe-decode -p fe-lift` が通る(新しい試験を含む)。
- `tools/switch-tables/check.mjs` が通る。Sytrus の認識できない `jmp reg` が 0 件(`0x44f273` を含む)。FLEngine・OTT・Balance の件数が、`pm-work/findings/switch-tables.md` の基準値より増えない。誤認識(飛び先が関数の範囲の外、または命令の途中)が 0 件。
- 起動 1 回で、engine-idle・standIn が false・error が null。
- 流れ A〜D(`welcome-flow.mjs`)の 4 行: 各行で `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。`elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。
- 単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: `0x44f273` の形(命令の列)、基準値の表(形ごとの件数)、広げた認識の件数と、誤認識の件数、`check.mjs` の出力の行、流れ A〜D の 4 行、`elapsedMs`・`preBootBytes`、`lift/`・`app/` を変えたこと(push すると `deploy.yml` が動く)。

## 制約

- 製品に、新しい環境変数(ビルド・実行を切り替える旗)・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 認識を広げるのは既定の動作にする。オプトインを作らない。
- 誤認識を隠すための握りつぶし(飛び先の検査を外す・停止を飲み込む)をしない。
- 名前の規則(訂正): 禁止されているのは、AI のベンダー名(`guard/scan.sh` が検査する)と、Cloudflare の識別子だけ。FL の製品名・`Image-Line` のインストール先・`%FLStudioFactoryData%` などの変数名は、ファイル・Issue の本文に書いてよい。Cloudflare の資源の名前・アカウント ID・ホスト名は書かない。
- Python と OCR を使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- エンジンのビルドは約 35 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。`sleep`・`timeout` をコマンドの先頭に書かない。

## 他のタスクとの関係

- blocked_by: #1400(Sytrus の wasm を、起動の後に読み込めるようにする。段階 1)。閉じた後に起動する。
- 後続: #1387(Riff machine の窓)は、この Issue と #{{171}} が閉じた後に起動する。
- #{{171}}(Sytrus の Data)とは独立。どちらも demo が最後まで開くために要る。
