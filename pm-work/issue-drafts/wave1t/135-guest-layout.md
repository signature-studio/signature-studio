---
draft: "135"
title: "ゲストのアドレス空間の配置: 内蔵プラグイン 160 本の像(約 609 MB)が入るように、GLOBAL_BASE と実行時の表の上限を決め、起動時間とメモリを実測する"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["125"]
labels: ["delivery", "plugin-builtin"]
size: L
priority: High
project: signature-studio/3
---

## 目的

#{{125}}(プラグインの追加を、1 プラグイン 1 ディレクトリの追加だけにする)の生成の段は、第 2 波の 160 本の像を、ゲストのアドレス空間に重ならずに置く。今の配置には入らない。この Issue では、像の置き場所と、実行時の表の上限を決め、起動の時間とメモリを実測で確かめる。

依頼者の優先(この順で迷ったら上を取る): P3(Song Mode で音が途切れない)> P2(UI が 120 fps で動く)> P1(10 秒以内に起動する。目標は #{{035}})> サイズ。

## 背景(実測)

1. #{{125}} の 1 回目の試行の報告(「(7) 配置」の節)で、ゲストのアドレス空間の見積もりが誤っていたと分かった。
   - 静的な像の末尾は oggio(base 0xc000000)の 0xc1cf000 である。Balance は 0xc1d0000(SizeOfImage 0x3a1000)に置かれた。
   - GLOBAL_BASE(0x10000000。`lift/harness/web/link.rsp` の `-sGLOBAL_BASE=268435456`)までの空きは 0x3E30000、約 65.2 MB(62.2 MiB)である。
   - 像を GLOBAL_BASE より下に置く案(案 A。今の配置)では、3.7 MB 級の像で約 17 本が上限(推定)。第 2 波の 160 本は、DLL のファイルの大きさの和が 608,521,016 B(本体 132 本で 576,925,696 B)で、入らない。
2. 今の wasm の設定(`lift/harness/web/link.rsp`): `-sINITIAL_MEMORY=671088640`(640 MiB)、`-sMAXIMUM_MEMORY=2147483648`(2 GiB)。
3. 実行時の表の上限: `RM_MAX`(16。`lift/runtime/fle_realmod.c`)、`SHARED_MODULE_REALMODS`(16。`lift/runtime/fle_cursor.c`。RM_MAX と同じ値の複製)、`MAIN_REAL_MAX`(32。`lift/harness/two/main_two.c`)。今の `lift/harness/b2d2/modules.tsv` は 10 行で、160 本を足すと約 170 行になる。どれも増やす必要がある。
4. 案 B(GLOBAL_BASE を上げ、像の領域を広げる)は、まだ評価していない。wasm のメモリの実測が無い。起動の時間と、メモリの確保(commit)が増える可能性がある。
5. 依頼者からの指摘(2026-11-24): 評価用プロジェクト(demo の曲)を開こうとすると落ちる(#{{121}}・#{{128}})。#{{125}} の試行でも、Balance を読んだ後、`tools/delivery-split/measure.mjs` の scene の終わりで `ExitProcess`(終了コード 0)で止まった。像の配置がこの症状に関わるかは未確認。

## 読む物

- `lift/harness/web/link.rsp`(`GLOBAL_BASE`・`INITIAL_MEMORY`・`MAXIMUM_MEMORY`)
- `lift/runtime/fle_realmod.c`(`RM_MAX`・`table_check`)、`lift/runtime/fle_realmod.h`、`lift/runtime/fle_cursor.c`(`SHARED_MODULE_REALMODS` とハンドルの式)、`lift/runtime/fle_loadimage.h`(ハンドルの領域の注釈)
- `lift/runtime/fle_modules.c`・`lift/runtime/fle_load.c`(realmod の引き当て)
- `lift/harness/two/main_two.c`(`MAIN_REAL_MAX` と、像のファイルの読み)
- `lift/crates/fe-build/src/plugins.rs`(#{{125}} が足す。生成の段の base の割り当て)
- `lift/harness/b2d2/modules.tsv`(今の base の行)
- `app/src/engine/fruity.ts`・`app/src/engine/worker.ts`(`memBytes`)・`app/src/main.ts`(`engineMemoryBytes`)・`app/src/perf/stats.ts`(`WASM_MEMORY_LIMIT_BYTES`。wasm の最大の値の定数。`-sMAXIMUM_MEMORY` を変えたら、この定数も同じ値にする)
- `tools/delivery-split/measure.mjs`(起動の時間は `elapsedMs`。scene の Balance の読みは `reads`)
- `tools/p3-audio-probe/measure.mjs`(`--boot`。`bootLast60sAt70s`)
- `tools/boot-check/boot-bundle.mjs`・`tools/smoke/welcome-flow.mjs`

## やること

1. **実測で案を選ぶ(像の置き場所)。**
   - ダミーの像を作る。使われない大きな領域を、生成して置く(実物のプラグインの代わり)。ダミーは測定のためだけに使い、製品の既定の動作として残さない。測定の後に取り除く。
   - 像の領域の大きさを 100 MB・300 MB・600 MB・900 MB に変え、GLOBAL_BASE と INITIAL_MEMORY を上げる。大きさごとに、次を n=3 で測る(headless Chrome。runner の値)。
     - 起動の時間: `tools/delivery-split/measure.mjs` の `elapsedMs`(3 回)。`tools/p3-audio-probe/measure.mjs --boot` の `elapsedMs` は、試験の終わりまでの時間で、起動の時間ではないので使わない。
     - wasm のメモリの確保量: `window.__app.engineMemoryBytes`(`WebAssembly.Memory` の `buffer.byteLength`)。ブラウザのプロセス群の RSS も取れれば取る(取り方を報告に書く。取れなければ「確認できない」と書く)。
     - 音の途切れ: `tools/p3-audio-probe/measure.mjs <bundle> --boot` の `bootLast60sAt70s`(3 回)。
   - 測定のための一時の変更は commit しない。
2. **選ぶ基準。** 像を 600 MB 級に置いた配置で、起動の時間が 3 回とも 12,500 ms 以下、`bootLast60sAt70s` が 3 回とも 0、wasm のメモリの確保が 1.5 GB 以下。この基準を満たす配置を選ぶ。満たせない場合は、像を要る時に置く案(プラグインを guest が要求した時に `memory.grow` し、像をその番地へ置く。ヒープの後ろに像の領域を置く)を、同じ測り方で比べ、選ぶ。
3. **選んだ配置を反映する。**
   - `lift/harness/web/link.rsp` の `GLOBAL_BASE`・`INITIAL_MEMORY`(必要なら `MAXIMUM_MEMORY`。変えたら `app/src/perf/stats.ts` の `WASM_MEMORY_LIMIT_BYTES` も同じ値にする)。
   - `lift/crates/fe-build/src/plugins.rs` の base の割り当て。
   - 実行時の表の上限: `RM_MAX`(`fle_realmod.c`)、`SHARED_MODULE_REALMODS`(`fle_cursor.c`。RM_MAX と同じ値)、`MAIN_REAL_MAX`(`main_two.c`)。上限は、生成する `modules.tsv` の行数以上にし、生成の段で検査する(超えたら止まる)。`fle_cursor.c` のハンドルの式が、この行数と合うかを確かめる。
   - 試験: `tools/` に、`inputs/plugin-classes.tsv` から、実物の 160 本の本体の `plugin.tsv` を作る道具を足す(生成したファイルは `lift/plugins/` に入れない。試験の中で作る)。実際の DLL は使わない。その 160 本の base の割り当てが、選んだ配置の像の領域に全て入り、互いに重ならないことを、`cargo test -p fe-build` の試験で確かめる。起動の確認は、Balance・Filter・Delay 2 の 3 本のままにする。
   - `#ifdef` を足さない。
4. **報告。** `pm-work/findings/guest-layout.md` に、表(案ごと。像の領域の大きさ・起動の時間の 3 回・engineMemoryBytes・RSS(取れた場合)・bootLast60sAt70s の 3 回)と、選んだ配置、選んだ理由、基準との比較を書く。

## 受け入れ

```accept
npm run build --prefix app && cargo test --locked --manifest-path lift/Cargo.toml -p fe-build && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/delivery-split/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/split.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&r.elapsedMs<=12500&&Array.isArray(r.reads)&&r.reads.some((x)=>/balance/i.test(x.name)&&/\.wasm$/i.test(x.name)&&x.requested===true));process.exit(ok?0:1)' "$RUNNER_TEMP/split.jsonl" && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --boot > "$RUNNER_TEMP/p3-1.json" && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --boot > "$RUNNER_TEMP/p3-2.json" && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --boot > "$RUNNER_TEMP/p3-3.json" && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/p3-"+i+".json","utf8")));process.exit(L.every((r)=>r.error===null&&r.bootLast60sAt70s===0&&r.state==="engine-idle")?0:1)' && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const a=L.find((r)=>r.flow==="A");process.exit(a&&a.error===null&&a.state==="engine-idle"&&a.standIn===false&&a.trapped===false&&a.hostErrors===0&&a.opened===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && test -f pm-work/findings/guest-layout.md
```

判定の基準:
- ビルドと起動確認: `tools/boot-check/boot-bundle.mjs` の `state` が engine-idle、`standIn` が false、`error` が null。
- `tools/delivery-split/measure.mjs` の 3 回: 全て engine-idle・standIn が false・error が null、`elapsedMs` が 12,500 以下、Balance の wasm の instantiate が `reads` に `requested: true` で在る。
- `tools/p3-audio-probe/measure.mjs --boot` の 3 回: `error` が null、`bootLast60sAt70s` が 0。
- `cargo test -p fe-build` が通る(160 本の base の割り当ての試験を含む)。
- `tools/smoke/welcome-flow.mjs` の流れ A が通る。
- `pm-work/findings/guest-layout.md` が在る。

## 作業の進め方

- ビルドは約 35 分(`fe-build engine` は約 23 分)。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000 にする。背景で起動して待たない。
- 像の大きさの 4 通りを、1 回のビルドで測れない場合は、ビルドを分ける。
- 作業の開始時に `date -u` で時刻を記録し、4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える。
- 1 回の試行に収まらない場合は、実測(1・2)だけで commit して終える(次の段は PM が起票する)。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。測定のための一時の分岐も作らない。既定の動作を、測定のためだけに変えない。
- Python を使わない(製品にも、測定にも)。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- `lift/`・`app/` の変更は main の `deploy.yml` を動かすので、報告に書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 像の領域の大きさごとの表(起動の時間 3 回・engineMemoryBytes・RSS・bootLast60sAt70s 3 回)、選んだ配置と理由、表の上限(RM_MAX・SHARED_MODULE_REALMODS・MAIN_REAL_MAX)の変更の値、試験の結果、scene の終わりの状態(ExitProcess の有無)。

## 他のタスクとの関係

- blocked_by: #{{125}}(生成の段と、base の割り当て `lift/crates/fe-build/src/plugins.rs` を入れる)。#{{125}} が閉じるまで、この Issue は動かない。
- #{{126}}(Fruity Filter を、ディレクトリの追加だけで足す)と #{{127}}(内蔵のプラグインを 1 本ずつ確かめる道具)は、この Issue の後。
- #{{121}}・#{{128}}(demo の曲を開くと落ちる)と症状が重なるが、原因は未確認。この Issue の測定で、scene の終わりの状態を報告に書く。
