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

#{{125}}(プラグインの追加を、1 プラグイン 1 ディレクトリの追加だけにする)の生成の段は、第 2 波の 160 本の像を、ゲストのアドレス空間に重ならずに置く。今の配置には入らない。この Issue では、像の置き場所と実行時の表の上限を直し、起動の時間とメモリ(物理のメモリ)を実測で確かめる。

依頼者の優先(この順で迷ったら上を取る): P3(Song Mode で音が途切れない)> P2(UI が 120 fps で動く)> P1(10 秒以内に起動する。目標は #{{035}})> サイズ。

## 背景(実測)

1. #{{125}} の 1 回目の試行の報告(「(7) 配置」の節)で、ゲストのアドレス空間の見積もりが誤っていたと分かった。
   - 静的な像の末尾は oggio(base 0xc000000)の 0xc1cf000 である。Balance は 0xc1d0000(SizeOfImage 0x3a1000)に置かれた。
   - GLOBAL_BASE(0x10000000。`lift/harness/web/link.rsp` の `-sGLOBAL_BASE=268435456`)までの空きは 0x3E30000、約 65.2 MB(62.2 MiB)である。今の配置(像を GLOBAL_BASE より下に置く案 A)では、第 2 波の 160 本は入らない。
   - 訂正: 160 本の大きさは、ファイルの大きさの和 608,521,016 B ではなく、PE の `SizeOfImage` の和 748,158,976 B(64 KiB 単位)で決まる。前回の試行で、インストーラの木(`lift/out-app/host`。182 DLL)から SHA-256 で一致させた 160 本の値を読んで求めた。Issue の初版の「約 609 MB」は、ファイルの大きさの和だった。
2. 今の wasm の設定(`lift/harness/web/link.rsp`): `-sINITIAL_MEMORY=671088640`(640 MiB)、`-sMAXIMUM_MEMORY=2147483648`(2 GiB)。
3. 実行時の表の上限: `RM_MAX`(16。`lift/runtime/fle_realmod.c`)、`SHARED_MODULE_REALMODS`(16。`lift/runtime/fle_cursor.c`。RM_MAX と同じ値の複製)、`MAIN_REAL_MAX`(32。`lift/harness/two/main_two.c`)。今の `lift/harness/b2d2/modules.tsv` は 10 行で、160 本を足すと 170 行になる。
4. 依頼者からの指摘(2026-11-24): 評価用プロジェクト(demo の曲)を開こうとすると落ちる(#{{121}}・#{{128}}。どちらも閉じた)。#{{125}} の試行でも、Balance を読んだ後、scene の終わりで `ExitProcess`(終了コード 0)で止まった。像の配置がこの症状に関わるかは未確認。

## 前回の試行で分かったこと

(run 45452762437。failed。作業は work.diff として保存された。commit は、この main に無い。次の試行は、その差分を main の上に取り込んで続ける。)

- 測定(各 3 回。`GLOBAL_BASE` だけを変えた。読まれる像は 3 本(Balance・Filter・Delay 2)で、160 本のバイトは置いていない):

  | 配置(像の領域) | `engineMemoryBytes`(確保量) | 起動 `elapsedMs` | `bootLast60sAt70s` |
  |---|---|---|---|
  | 現行(約 62 MiB) | 1,071,906,816 B | 11,220.8 / 停止(180 秒超) / 11,166.6 | 0 / 0 / 0 |
  | 100 MiB | 1,111,556,096 B | 10,912〜11,313 | 0 / 0 / 0 |
  | 300 MiB | 約 1.32 GB | 11,116〜11,283 | 0 / 0 / 0 |
  | 600 MiB | 1,585,512,448 B | 11,267〜11,478 | 0 / 0 / 0 |
  | 900 MiB | 1,942,028,288 B | 11,182〜11,370 | 0 / 0 / 0 |

- 160 本が入るには、像の領域が 748 MB 以上必要。900 MiB の領域(943,718,400 B)は入るが、確保量は 1.81 GiB になる。確保量は物理のメモリを表さない(触れられていないページは RSS に入らない)。このため判定を RSS に直す(下の PM の決定 1)。
- 起動の停止: 現行の配置で 7 回中 2 回、180 秒を超えて `state running` のままだった。他の 4 配置の 16 回では止まらなかった。#{{149}}(起動の間欠の停止。完了 22:28Z)で直ったとされる。この試行の後の出来事で、この Issue では未確認。
- scene の終わり: 12 回すべてで、ゲストの `ExitProcess(0)` の後に runtime が終了コード 134 で止まり、`exited` になった。配置による差は無い。原因は未確認。
- ヒープ: `engineMemoryBytes` は `GLOBAL_BASE` より上に約 0.75〜0.80 GB を足したもの。`INITIAL_MEMORY` は、現行で `GLOBAL_BASE` + 384 MiB。
- 製品の配置(`link.rsp`)は変えていない。差分に入っているのは、表の上限 192、160 本の道具 `tools/plugin-layout/wave2-plugins.mjs` と、その試験、報告の草稿 `pm-work/findings/guest-layout.md`。
- ファイル名の重複: 160 本のうち、ファイル名が同じ物が在る(`engine_x64.dll` が 18 本。`sytrus_x64.dll`・`patcher_x64.dll` など)。`lift/crates/fe-build/src/plugins.rs` の `validate()`(210 行付近)が、ファイル名の一意を要求して拒否する。
- 再リンク: 同じオブジェクトから、リンクのフラグだけを変えて作り直せる。600 MiB の `flengine6.wasm` と `flengine6.js` を、バイト単位で再現した。`link` 段は約 7 秒。wasm-opt・hybrid・glue・bundle を含む再リンクは約 28 分。`fe-build engine` は 33〜35 分。
- `tests/realmod.rs` は MinGW(`x86_64-w64-mingw32-gcc`)が要る。この runner には無いので、動かせなかった。

## PM の決定(依頼者の判断。この通りに行う)

1. **メモリの判定を、RSS に直す。** 確保量(`engineMemoryBytes`。`WebAssembly.Memory` の `buffer.byteLength`)は、物理のメモリを表さないので、判定に使わない(参考値として JSON に出す)。判定に使うのは、ブラウザの全プロセス(起動した Chrome と、その子孫のプロセス)の `/proc/<pid>/status` の `VmRSS` の合計である。`tools/delivery-split/measure.mjs` に、この値を JSON の項目 `rssMb`(MB。1 MB = 1,048,576 B)として足す。engine-idle から場面の終わりまで 1 秒ごとに取り、最大値を出す。**1.5 GB の基準は外す。**
2. **配置は、160 本が入る最小の領域にする。** 像の領域は、0xc1d0000 から始め、160 本の `SizeOfImage` の和(748,158,976 B)に 5% の余裕を足した大きさ(約 750 MiB)にする。`GLOBAL_BASE` は、この領域の末尾に置く。`INITIAL_MEMORY` は、`GLOBAL_BASE` の上のヒープが収まるように上げる。`MAXIMUM_MEMORY` は 2 GiB のままにする。収まらないなら、報告に書いて止まる。`app/src/perf/stats.ts` の `WASM_MEMORY_LIMIT_BYTES` は、`MAXIMUM_MEMORY` と同じ値にする。160 本の base の割り当ては、実物の 160 本で `plugins.rs` が計算する。
3. **実行時の表の上限は 192。** `RM_MAX`・`SHARED_MODULE_REALMODS`・`MAIN_REAL_MAX` を 192 にする(前回の差分に在る)。生成の段は、行数が上限を超えたら止まる(既存)。
4. **ファイル名の重複を許す。** `key` は `<種別>.<フォルダ>.<DLL 名>` で一意にする。`validate()` の検査は、`key` と `guest` の一意だけにし、ファイル名の一意の検査を外す。ゲストの `LoadLibrary` は、パスの接尾辞の表(`fle_realmod_find_path`。`lift/runtime/fle_realmod.c`)で引く。素の名前の呼び出し(`fle_realmod_find(name)`: `lift/runtime/fle_load.c` の 427 行、`lift/runtime/fle_modules.c` の 226・232 行)が、重複する名前(`engine_x64.dll`・`sytrus_x64.dll`・`patcher_x64.dll` など)で誤るかを、`inputs/plugin-classes.tsv` と runtime を読んで確かめる。足りなければ、報告に書いて止まる(直すかどうかは PM が決める)。
5. **受け入れ**は、下の「受け入れ」と「判定の基準」による。

## 読む物

- `lift/harness/web/link.rsp`(`GLOBAL_BASE`・`INITIAL_MEMORY`・`MAXIMUM_MEMORY`)
- `lift/runtime/fle_realmod.c`(`RM_MAX`・`fle_realmod_find_path`・`fle_realmod_find`)、`lift/runtime/fle_realmod.h`、`lift/runtime/fle_cursor.c`(`SHARED_MODULE_REALMODS` とハンドルの式)、`lift/runtime/fle_loadimage.h`(ハンドルの領域の注釈)
- `lift/runtime/fle_load.c`・`lift/runtime/fle_modules.c`(realmod の引き当て。素の名前の呼び出し)
- `lift/harness/two/main_two.c`(`MAIN_REAL_MAX` と、像のファイルの読み)
- `lift/crates/fe-build/src/plugins.rs`(生成の段の base の割り当て。`validate()`)
- `lift/harness/b2d2/modules.tsv`(今の base の行)
- `inputs/plugin-classes.tsv`(160 本の一覧)
- `tools/plugin-layout/wave2-plugins.mjs`(前回の差分に在る。160 本の道具)
- `app/src/perf/stats.ts`(`WASM_MEMORY_LIMIT_BYTES`)・`app/src/main.ts`(`engineMemoryBytes`)
- `tools/delivery-split/measure.mjs`(起動の時間は `elapsedMs`。`reads`)・`tools/p3-audio-probe/measure.mjs`(`--boot`。`bootLast60sAt70s`)
- `tools/boot-check/boot-bundle.mjs`・`tools/smoke/welcome-flow.mjs`

## やること

1. **配置を反映する**(PM の決定 2〜4): `lift/harness/web/link.rsp` の `GLOBAL_BASE`・`INITIAL_MEMORY`・`MAXIMUM_MEMORY`、`app/src/perf/stats.ts`、`lift/crates/fe-build/src/plugins.rs` の base の割り当てと `key`・`validate()`、表の上限 192(`fle_realmod.c`・`fle_cursor.c`・`main_two.c`)。生成の段が、行数と像の領域の両方を検査する(既存)。`#ifdef` は足さない。
2. **試験**: `cargo test -p fe-build` で、160 本の base が全て領域の中に入り、互いに重ならないことを確かめる(前回の差分の試験を使う。領域は採った配置の値にする)。
3. **測定の道具**: `tools/delivery-split/measure.mjs` に `rssMb` を足す(PM の決定 1)。現行の配置の bundle を作る道具 `tools/plugin-layout/make-base.sh` を作る(引数なし。リポジトリの直下で走る。現行の `link.rsp` の値で、直前の `fe-build engine` のオブジェクトから再リンクし、`lift/out-base/bundle/raw/fl-Oz` に、`tools/delivery-split/measure.mjs` が読める形(`raw/`・`br/`・3 つ上の `host/` の木)で置く。成功で 0、失敗で 1)。
4. **報告**: `pm-work/findings/guest-layout.md` に、表(現行と採る配置の `rssMb` の 3 回・`elapsedMs` の 3 回・`engineMemoryBytes`(参考)・`bootLast60sAt70s` の 3 回)、採った配置と理由、base の割り当ての件数、名前の重複の扱い、素の名前の呼び出しの確認の結果、scene の終わりの状態(`ExitProcess` の有無)を書く。

## 受け入れ

```accept
npm run build --prefix app && cargo test --locked --manifest-path lift/Cargo.toml -p fe-build && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && sh tools/plugin-layout/make-base.sh && node tools/delivery-split/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/split.jsonl" && node tools/delivery-split/measure.mjs "$PWD/lift/out-base/bundle/raw/fl-Oz" > "$RUNNER_TEMP/split-base.jsonl" && node -e 'const fs=require("fs");const rd=(f)=>fs.readFileSync(f,"utf8").trim().split("\n").map((l)=>JSON.parse(l));const med=(L)=>L.map((r)=>r.rssMb).sort((a,b)=>a-b)[1];const ok=(L)=>L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&typeof r.rssMb==="number"&&r.elapsedMs<=12500&&Array.isArray(r.reads)&&r.reads.some((x)=>/balance/i.test(x.name)&&/\.wasm$/i.test(x.name)&&x.requested===true));const A=rd(process.argv[1]);const B=rd(process.argv[2]);process.exit(ok(A)&&ok(B)&&med(A)<=med(B)+150?0:1)' "$RUNNER_TEMP/split.jsonl" "$RUNNER_TEMP/split-base.jsonl" && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --boot > "$RUNNER_TEMP/p3-1.json" && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --boot > "$RUNNER_TEMP/p3-2.json" && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --boot > "$RUNNER_TEMP/p3-3.json" && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/p3-"+i+".json","utf8")));process.exit(L.every((r)=>r.error===null&&r.bootLast60sAt70s===0&&r.state==="engine-idle")?0:1)' && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && test -f pm-work/findings/guest-layout.md
```

## 判定の基準

- ビルドと起動確認: `tools/boot-check/boot-bundle.mjs` の `state` が engine-idle、`standIn` が false、`error` が null。
- 採る配置の `tools/delivery-split/measure.mjs` の 3 回: 全て engine-idle・standIn が false・error が null・`rssMb` が数値、`elapsedMs` が 12,500 以下、Balance の wasm が `reads` に `requested: true` で在る。
- 現行の配置(`lift/out-base/`)の同じ道具の 3 回: 上と同じ条件。
- メモリ: 採る配置の `rssMb` の中央値(3 回の真ん中の値)が、現行の配置の `rssMb` の中央値 + 150 MB(1 MB = 1,048,576 B)以下。
- `tools/p3-audio-probe/measure.mjs --boot` の 3 回(採る配置): `error` が null、`bootLast60sAt70s` が 0。
- `cargo test -p fe-build` が通る(160 本の base の割り当ての試験を含む)。
- `tools/smoke/welcome-flow.mjs` の流れ A〜D: 全て `error` が null・engine-idle・standIn が false・`preBootBytes` が 314,368,711 以下。流れ B と D は `welcomeClosed` が true。
- `pm-work/findings/guest-layout.md` が在る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 現行と採る配置の表(`rssMb`・`elapsedMs`・`engineMemoryBytes`(参考)・`bootLast60sAt70s` の 3 回)、採った配置と理由、表の上限の値、試験の結果、scene の終わりの状態(ExitProcess の有無)、名前の重複の扱いと、素の名前の呼び出しの確認の結果。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。測定のための一時の分岐も作らない。既定の動作を、測定のためだけに変えない。
- Python を使わない(製品にも、測定にも)。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- `lift/`・`app/` の変更は main の `deploy.yml` を動かすので、報告に書く。

## 作業の進め方

- ビルドは約 35 分(`fe-build engine`)。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000 にする。背景で起動して待たない。
- 受け入れの全体(ビルド・現行の bundle の作成・測定)は約 90 分かかる。判定は commit の後に同じ job で、もう一度全部走る。job の上限は 5 時間 50 分。
- 作業の開始時に `date -u` で時刻を記録する。3 時間 30 分を過ぎたら、新しいビルドを始めない。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- 1 回の試行に収まらない場合は、差分を commit して終える。次の試行は、その上から続ける。

## 他のタスクとの関係

- blocked_by: #{{125}}(閉じた。生成の段と、1 プラグイン 1 ディレクトリの仕組みは main に在る)。
- #{{126}}(Fruity Filter を、ディレクトリの追加だけで足す)と #{{127}}(内蔵のプラグインを 1 本ずつ確かめる道具)は、この Issue の後。
- #{{121}}・#{{128}}(demo の曲を開くと落ちる)と症状が重なるが、原因は未確認。この Issue の測定で、scene の終わりの状態を報告に書く。
