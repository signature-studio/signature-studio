---
draft: "229"
title: "分離 wasm のプラグインを含む曲を再生すると、協調スレッドが `SuspendError: trying to suspend JS frames` で失敗する(5 回中 3 回)"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["127"]
labels: ["perf", "audio", "verify"]
size: XL
priority: High
project: signature-studio/3
---

## 目的

内蔵プラグインを含む曲を再生しても、協調スレッドが失敗せず、音声の処理が続くこと。P3(Song Mode で音が途切れない)に直結する。協調スレッドが落ちると、その音声の処理が止まり、途切れの原因になる。

## 背景

実測(#1371 の道具の前回の試行の報告。runner の値。この機械とは別枠):

- 道具は `tools/plugin-run/run.mjs`(#1371 で作られる)。プラグインを含む曲を開き、Space で再生する。Fruity Balance は `Chan Gibbons x Ceebeaats - Time.flp`、Fruity Filter は `Asher Postman - Future House.flp` を使う(いずれも `lift/out-app/host-extra/resources/` の曲。ビルドの生成物)。
- Space を押した後、同じ bundle で 5 回中 3 回、エンジンのログに次の行が出た:

```
[err] [js] fle thread 23 failed: SuspendError: trying to suspend JS frames | at wasm://wasm/2623a8ae:wasm-function[158170]:0x8e89f8e | at wasm://wasm/2623a8ae:wasm-function[158172]:0x8e8a43c (guest thread 23)
```

- Space を押さない対照(1 回)では、この行は出なかった。
- `audio.detail()` の `nonSilentQuanta` は 0(曲のサンプルが欠けて無音。別の件)。`quantaWhilePlaying` は増える。

疑い(推定。確かめていない):

- 分離 wasm(`#1342` の `--separate`。プラグインごとの wasm。`lift/crates/fe-wasmgen/src/hybrid.rs`)の関数は、engine から `env.f_<va>` の import を経由して呼ばれる。その import は、`app/src/engine/xmod.ts` の JS の stub(`XmodLoader.stub`)で、読み込み済みの分離 wasm の関数へ転送される。
- 協調スレッドの切り替えは JSPI(`WebAssembly.Suspending` と `WebAssembly.promising`)で行う。suspend しうる import(Sleep・待ちなど)の呼び出しの途中に JS のフレーム(この stub)が挟まると、JSPI は suspend できず `SuspendError` になる。
- engine 本体は単一の wasm で、suspend する import の間に JS のフレームが挟まらないので、この失敗は出ない。

既存の候補(main には無い。効果は確かめていない): `pm-work/findings/perf-windows-browser/patches/07-xmod-wasm-relay-jspi.patch`(記録は `pm-work/findings/perf-windows-browser/README.md` の表の 14)。xmod の stub を、Wasm の関数表で転送する中継に置き換え、JS のフレームを suspend しうる呼び出しの間から外す。非同期待機中の SuspendError の回帰試験を含む。実 FLP での検証は、その記録の時点で済んでいない。

## 読む物

- `app/src/engine/xmod.ts`(`XmodLoader.stub`・`installXmod`)・`app/src/engine/worker.ts`・`app/src/engine/features.ts`(JSPI の確認)
- `lift/crates/fe-wasmgen/src/hybrid.rs`(`--separate`・`wasm-func-names-<モジュール>.tsv`・`__wrap_<名前>` の export)・`lift/crates/fe-build/src/main.rs`(`hyb/wasm-func-names.tsv` を `gen/` へ写す段)
- `lift/runtime/fle_thread.c`(協調スレッド。`fle_thread_spawn`・`fle_thread_sleep`・待ち)・`lift/harness/web/link.rsp`(`--wrap`)
- `pm-work/findings/perf-windows-browser/patches/07-xmod-wasm-relay-jspi.patch`・`pm-work/findings/perf-windows-browser/README.md`(表の 14)
- `tools/plugin-run/run.mjs`(#1371 の道具)・`tools/ab/run.mjs`・`tools/ab/gates.mjs`・`tools/ab/base.mjs`・`tools/p3-audio-probe/measure-live.mjs`
- `tools/delivery-split/measure.mjs`(場面の作り方の参考。変えない)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. **再現**。道具(`tools/plugin-run/run.mjs`)で、Fruity Balance と Fruity Filter を各 5 回(同じ bundle)。Space の後の `playFailures` の件数を表にする。`playFailures` は、Space の後のログのうち `fle thread` と `failed` を両方含む行の配列(無ければ `[]`)。道具に、Space の後の再生と `playFailures` が無ければ、この Issue で足す(`tools/` の中だけ。製品の既定の動作は変えない)。
2. **切り分け**。失敗の行の `wasm-function[158170]`・`wasm-function[158172]` を関数名に直す(engine の名前は `lift/out-app/gen/wasm-func-names.tsv`。分離 wasm の名前は `lift/out-app/hyb/` の下の `wasm-func-names-<モジュール>.tsv`。確かめる)。どの import(Sleep・待ちなど)が suspend を起こすか、呼び出しの経路(分離 wasm → xmod の JS の stub → engine)を特定する。
3. **直す**。候補:
   - (a) xmod の stub を、JS の関数ではなく Wasm の中継(関数表で転送する)にする。ブランチの 07 の方式。JSPI を保つ。
   - (b) 分離 wasm から呼ばれる、suspend しうる import を、suspend しない形にする(その場で協調スレッドを切り替えず、次の切り替え点まで待つ)。
   - (c) プラグインの処理(`Process`・`Render`)を、suspend が起きない経路に限る。
   選んだ方式と理由を報告に書く。
4. **音声への影響**。変更後も、P3 の A/B(`tools/ab/run.mjs p3`)で途切れが基準を超えない(判定の基準を見る)。
5. 受け入れを通す。通らなければ、作業ツリーを commit して、報告を書いて終える。

## 受け入れ

```accept
node tools/ab/base.mjs "$(git merge-base HEAD origin/main)" .tmp-base && (cd .tmp-base && npm ci --prefix app && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle) && node tools/ab/base.mjs --verify .tmp-base && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "Fruity Balance" "Fruity Filter" > "$RUNNER_TEMP/pr-1.jsonl" && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "Fruity Balance" "Fruity Filter" > "$RUNNER_TEMP/pr-2.jsonl" && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "Fruity Balance" "Fruity Filter" > "$RUNNER_TEMP/pr-3.jsonl" && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "Fruity Balance" "Fruity Filter" > "$RUNNER_TEMP/pr-4.jsonl" && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "Fruity Balance" "Fruity Filter" > "$RUNNER_TEMP/pr-5.jsonl" && node -e 'const L=process.argv.slice(1).flatMap((f)=>require("fs").readFileSync(f,"utf8").trim().split("\n").map((l)=>JSON.parse(l)));const n=(p)=>L.filter((r)=>r.plugin===p).length;const ok=(r)=>r.error===null&&r.instantiated===true&&r.processed===true&&Array.isArray(r.playFailures)&&r.playFailures.length===0;process.exit(L.length===10&&n("Fruity Balance")===5&&n("Fruity Filter")===5&&L.every(ok)?0:1)' "$RUNNER_TEMP/pr-1.jsonl" "$RUNNER_TEMP/pr-2.jsonl" "$RUNNER_TEMP/pr-3.jsonl" "$RUNNER_TEMP/pr-4.jsonl" "$RUNNER_TEMP/pr-5.jsonl" && node tools/ab/run.mjs p3 "$PWD/.tmp-base/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-base/app" "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/app" 3 > "$RUNNER_TEMP/ab-p3.json" && node -e 'const s=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(s.pass===true?0:1)' "$RUNNER_TEMP/ab-p3.json" && bash guard/scan.sh
```

## 判定の基準

- 起動確認(`boot.json`)が engine-idle・`standIn` が false・`elapsedMs` が 12,500 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- 道具の 10 行(Fruity Balance 5 行・Fruity Filter 5 行)が、`error` が null・`instantiated` が true・`processed` が true・`playFailures` が空の配列。
- P3 の A/B(`tools/ab/run.mjs p3`。変更前と変更後を交互に 3 組)が `pass` が true。判定の帯は `tools/ab/gates.mjs` のとおり: 変更後の中央値の `underrunDelta` と `last60sUnderruns` が、基準の run の最大値 + 4 以下(中央値どうしは比べない。絶対の 0 は求めない)。`pcmRepeat8192` が null か 0.01 未満。`nonSilentDelta` が 0 より大きい。
- 報告に書く: Balance と Filter の各 5 回の `playFailures` の件数、失敗の行の関数名(`wasm-func-names` による)、suspend を起こす import と呼び出しの経路、選んだ方式(a)〜(c) と理由、`tools/ab/run.mjs` の出力の `medians`・`baseMax`・`pass`。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。道具の試験のための分岐を製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。
- `app/`・`lift/`・`ci/` の変更は、push で main の `deploy.yml` を動かす。報告に書く。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- engine のビルドは約 35 分。受け入れのコマンドは、基準の版(`.tmp-base`)の engine のビルドを含むので、ビルドは 2 回(約 70 分)かかる。変更後の bundle は、作業の中で作り直す回数を減らし、同じ bundle で再現と切り分けを繰り返す。
- フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して、終わるのを待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: #{{127}}(道具 `tools/plugin-run/run.mjs`)。その Issue が閉じるまで起動しない。
- 親: #1302(描画と音声の分離)。分離 wasm の仕組みは #1342。P3 の判定の帯は #1426 と同じ。
- `app/src/engine/xmod.ts` は、第 2 波の内蔵プラグインの束も触る。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
