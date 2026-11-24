---
draft: "124"
title: "VST3 ホストの最小を作る(1b): OTT が 1 ブロックの音声を処理し、非無音・非 NaN を測る(VEX 系の命令の経路を確かめる)"
type: Task
milestone: "M6 外部プラグイン"
parent: "024"
blocked_by: ["123"]
labels: ["plugin-external"]
size: XL
priority: High
project: signature-studio/3
---

## 目的

課題 C-5 の基盤(2 段目)。#{{123}} で READY になった OTT の wasm に、1 ブロックの音声を流し、出力が非無音で NaN・Inf を含まないことを、道具 `tools/plugin-run/measure.mjs` で測る。fe-lift が「最小集合の外」と出した VEX 系の 87 行が、実行の経路に入るかを確かめ、入るなら扱う。

## 前提

#{{123}} が閉じている(受理された)こと。READY で止まった wasm と、`measure.mjs`(`instantiated` まで)が在る。この Issue では、その先の process の段と、音声の検査を足す。

## 前回までの試行で分かったこと

- #{{123}} の「前回までの試行で分かったこと」の 1〜8 を前提にする(この Issue は、その続き)。
- AVX: `fe-lift lift` の出力に、最小集合の外の VEX 系が 87 行(Vpslld・Vpsrad・Vpunpckldq・Vroundsd など)。`lift/crates/fe-lift/src/vex.rs` は、VMOVUPS・VMOVAPS・VMOVNTPS・VZEROUPPER と、FS/GS を使う LEA・MOV だけを扱い、他の VEX と EVEX は `Err`(未対応)。
- `lift/runtime/fle_cpuid.c` は、CPUID で AVX を返さない(ECX に AVX も SSE3 も無い。決定 0009・0032・0075)。そのため、コンパイラが出した AVX の分岐は通らない見込み。確かめるのは、実行の trace。
- 同じ VEX 形は、`lift/crates/fe-wasmgen/src/lower.rs` の 1567 行で `unreachable` になる(決定 0164。機械は SSE2 の経路を使う)。この経路に入ったら、`trapped_vex` の数が増える。

## やること

1. process の段を入れる。`IAudioProcessor::process` を、ProcessData(入力と出力の AudioBusBuffers、IParameterChanges、IEventList)で呼ぶ。作り方は `lift/harness/vst3a/main_vst3a.c` の process の段に合わせる。入力は共有メモリの slot(`protocol.ts` の inL・inR)から読み、出力は slot(outL・outR)へ書く。`main_vst_ext.c` の mt1_ext_wait と mt1_ext_done の中で行う。
2. 1 ブロック(4096 フレーム、44,100 Hz)を流す。入力は `app/src/plugins/test-signal.ts` の OttTestSignal(決定 0164 の信号)。出力を読み、`nonSilent`(0 でない値が 1 つ以上)を取る。NaN・Inf が 1 つでもあれば、`measure.mjs` は `error` に `non-finite output: <数>` と書く。
3. VEX 系の扱い: 2 のブロックの trace(`trapped_vex` の数、実行された命令)で、VEX の経路に入るかを確かめる。入るなら、(a) その経路を SSE2 の経路にする(決定 0164 の方式)か、(b) `vex.rs` に足す(公開の仕様。x86-64 の命令の仕様 SDM の Vol. 2 の擬似コード)。どちらにしたかと理由を報告に書く。入らないなら、入らないことを trace の数字で書く。
4. 参考値(報告に書くだけ。合否には使わない): 40 ブロック × 4096 フレームの out_hash を、`app/test/unit/plugins-ott.test.ts` の期待値 `8104f50e938ccf36` と比べる。一致すれば、決定 0164 の harness と同じ出力。
5. `measure.mjs` の 1 ブロックの JSON の行を、報告にそのまま貼る。

## 受け入れ

```accept
bash lift/harness/web/mt1/plugin-worker/build.sh ott "$PWD/lift/out-app/external" "$PWD/lift/out-app/plugins/ott" && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/plugin-run/measure.mjs "$PWD/lift/out-app/plugins/ott" ott > "$RUNNER_TEMP/ott.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());process.exit(r.error===null&&r.instantiated===true&&r.blockFrames>0&&r.nonSilent===true?0:1)' "$RUNNER_TEMP/ott.json"
```

判定の基準: `ott.json` の 1 行の生の値で、`error` が null、`instantiated` が true、`blockFrames` が 0 より大きい、`nonSilent` が true。加えて、`boot.json` が engine-idle で、`error` が null、`standIn` が false。

## 作業の進め方

- ビルド(engine と wasm)は約 35 分。フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ(この上限まで)。`sleep`・`timeout` をコマンドの先頭に書かない。背景のビルドを `tail --pid`・`wait` などで待たない。
- 1 回の試行で終わらないことが多い。最初に `date -u` で開始を記録し、4 時間 30 分を過ぎたら、受け入れが通らなくても作業ツリーを commit し、報告を書いて終える。
- 段ごとに、実行した出力の要点を報告に残す。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: process の段の作り方、VEX 系の trace の数字(`trapped_vex` と、実行された VEX の命令の数)、扱った方式(a か b か、その理由)、measure の JSON の行(そのまま貼る)、変えたパス。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- Python を製品と本番の経路に入れない。
- ベンダー名と Cloudflare の識別子を書かない。インストーラの名前に含まれる会社名を、そのまま書かない。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。sleep・timeout コマンド・nohup・末尾の `&` は使わない。

## 他のタスクとの関係

- 前提は #{{123}}(基盤 1a)。
- この Issue が閉じると、#{{027}}(Serum 2)・#{{028}}(OTT の配信と既定の経路)・#{{029}}(Vital)が起動する(それぞれ、プラグイン固有の対応を足す)。
- 同じ基盤のファイルを変えるため、他の Issue と衝突し得る。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
