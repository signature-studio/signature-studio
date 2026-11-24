---
draft: "123"
title: "VST3 ホストの最小を作る(1a): OTT の IComponent を作り、setupProcessing・setActive まで通す"
type: Task
milestone: "M6 外部プラグイン"
parent: "024"
blocked_by: []
labels: ["plugin-external"]
size: XL
priority: High
project: signature-studio/3
---

## 目的

課題 C-5 の基盤(1 段目)。OTT(外部の 3 件のうち、インストーラが一番小さい)を使い、VST3 のホストの COM の口を、製品の build の一部として作る。GetPluginFactory → createInstance(IComponent) → initialize → setupProcessing → setActive まで通す。この段では音声は処理しない(処理は #{{124}})。

出来上がりの形は 1 プラグイン 1 wasm。`app/src/plugins/plugin-worker.ts` が起動する wasm(`vst-ext.wasm`)を OTT について作り、道具 `tools/plugin-run/measure.mjs` で、その Worker が READY(setupProcessing と setActive が済んだ状態)になるまでを測る。

## 前回までの試行で分かったこと

run 45253143948(2026-11-23 19:52Z。#1337 の作業。#1336・#1338 も同じ壁で落ちた)の結果。

1. 受け入れの入力 `$RUNNER_TEMP/external/ott.exe` は存在しない。作業の段が `$RUNNER_TEMP/external/*` を `lift/out-app/external/` へ `mv` するので、その dir は空になる(`.github/workflows/agent.yml` の 569〜575 行)。入力は `lift/out-app/external/` の中の、`ci/artifacts.lock` の 3 行目の名前のファイル(8,397,928 B。SHA-256 の先頭 d4e1d3f6…)。照合は通った。
2. インストーラの中の VST3 本体(`OTT.vst3`)は 3,244,032 B(SHA-256 の先頭 8965c711…。`inputs/external-plugins.tsv` の OTT の行と一致)。PE32+、x86-64。export は GetPluginFactory・InitDll・ExitDll。import は 8 DLL(kernel32・user32・gdi32・gdiplus・ole32・shell32・winmm・imm32)で、大半は GUI の import。
3. 素の `fe-lift lift` は 1.1 秒で終わり、6,676 関数・532,220 行・44.6 MB を出した。VEX 系の命令 87 行が「最小集合の外」(`lift/crates/fe-lift/src/vex.rs` は VMOVUPS などの一部だけを持ち、他は `Err`)。扱いは #{{124}} で決める。
4. 基盤の header が無い。`lift/harness/vst/` は git の履歴に一度も無い。`archive/gen4-2026-11-18` のタグは、この clone にも remote にも無い。`lift/harness/vst3a/main_vst3a.c` が include する `vst_host.h`(と `vst_hm.def`)が無く、コンパイルは `fatal error: 'vst_host.h' file not found` で止まる。
5. 生成の道具が無い: `lift/harness/vst/gen_vst.sh`・`mkrows.sh`・`imports-vst.txt`(`gen_vst3a.sh` が呼ぶ)と、Python の `mk_stubs.py`・`patch_ordinals.py`・`patch_shd.py`・`patch_bmi.py`。
6. 製品の wasm の元も無い: `lift/harness/web/mt1/plugin-worker/build.sh` と `main_vst_ext.c`(`app/src/plugins/ott-host.ts` と `app/test/unit/plugins-ott.test.ts` の注釈が指す)。seed の表(`vfs-seed-vst.tsv`)も無い。
7. 測定の道具 `tools/plugin-run/measure.mjs` は無い(受け入れは `MODULE_NOT_FOUND` で終わった)。
8. 受け入れの鎖は、`npm run build`・`cargo build`・`fe-build engine`・`fe-build bundle`・`boot-bundle.mjs` まで通る(engine-idle、error null、standIn false。elapsedMs は 8,100〜12,600。runner の値)。ビルドは約 35 分。

## 既に在る物(再利用する。作り直さない)

- ホストの COM の実装: `lift/harness/vst3a/vst_host3a.c`(`vst_host.c` の copy。IHostApplication・IComponentHandler・IParameterChanges・IEventList と、`vst_hm_*` の応答)、`vst_stream3a.c`(IBStream)、`vst_msg3a.c`(IMessage・IAttributeList)、`vst_env3a.c`(ファイルと LoadLibrary の応答)、`vst3a_imp.c`(Windows API の応答)、`vst_exc3a.c`。
- VST3 の呼び出し順: `lift/harness/vst3a/main_vst3a.c`(984 行)。LoadLibraryW → DllMain → GetPluginFactory → createInstance(IComponent) → initialize → IAudioProcessor → setupProcessing → setActive → setProcessing → process。結果は `RESULT out_hash=… nonfinite=… nonzero=… rms=…` の行で出る。`main_vst3a.c` は Serum 2 用の harness(決定 0165)なので、Serum 2 固有の部分(別の controller、MIDI の note、TLS callback)は、そのまま持ち込まず、OTT に要る部分だけを使う。
- 製品の受け口(実装済み。今はテストだけが使う): `app/src/plugins/plugin-worker.ts`(init で DllMain から setActive まで。READY で待つ)、`ott-host.ts`(wasm の import を満たす)、`protocol.ts`(共有メモリの口)、`plugin-client.ts`(`PluginInstance.start`)、`node-worker.ts`(node の Worker)、`test-signal.ts`(OTT の入力信号。参照値の out_hash 8104f50e938ccf36 は 40 ブロック × 4096 フレーム)。
- 単体テスト `app/test/unit/plugins-ott.test.ts`(MT1_OTT_DIR に `vst-ext.wasm`・`ott.vst3`・`seed.tsv` が在れば動く。その dir の形を、build の出力にも使う)。
- COM の口の先例: `lift/runtime/fle_webview2.c`(#{{115}}。vtable の pseudo-VA と、`fle_dispatch_miss` → `fle_win_try_thunk` の流れ)。
- PE の rebase: `lift/crates/fe-pe`(`rebase`。`fe-lift gen-all` が使う)。
- インストーラの照合と取り出し: `tools/external-inputs/check.mjs`(DIR)、`tools/external-inputs/extract.mjs`(DIR OUT --tsv)。
- emcc の build: `lift/harness/vst3a/build_vst3a_wasm.sh`(オブジェクトの一覧・`wraps.txt`・リンクの引数)と `gen_vst3a.sh`(lift → patch → gen_vst)。Python の段だけを置き換える。
- seed の固定点: `lift/harness/vst3a/tools/seed_fixpoint.sh`・`repro.sh`(harness を動かして関数の入口を集める方式)。

## 読む物

- `lift/harness/vst3a/main_vst3a.c`(467 行の main と、533〜950 行の呼び出し順)、`vst_host3a.c`(先頭と、`vst_hm_*` の行)、`vst_msg3a.c`、`vst_stream3a.c`、`vst_env3a.c`、`vst3a.h`
- `lift/harness/vst3a/build_vst3a_wasm.sh`、`gen_vst3a.sh`、`run_vst3a_node.cjs`、`lift/harness/vst3a/tools/seed_fixpoint.sh`・`repro.sh`
- `app/src/plugins/plugin-worker.ts`、`ott-host.ts`、`protocol.ts`、`plugin-client.ts`、`node-worker.ts`、`test-signal.ts`、`app/test/unit/plugins-ott.test.ts`
- `lift/runtime/fle_cpuid.c`(AVX を返さない)、`lift/crates/fe-lift/src/vex.rs`
- `inputs/external-plugins.tsv`(OTT の行)、`ci/artifacts.lock`(3 行目)、`tools/external-inputs/check.mjs`・`extract.mjs`
- `.github/workflows/agent.yml` の 552〜575 行(外部のインストーラの置き場)

## やること

1. 基盤の header を作る。`lift/harness/vst/vst_host.h` を、`lift/harness/vst3a/` の C の 5 ファイルが使う名前(`vst_host`・`vst_iid`・`vst_iid_make`・`VST_HM_*`・`VST_HM_VA`・`K_RESULT_OK` など。`grep` で全部を数える)から書く。`vst_hm.def`・`mkrows.sh`・`imports-vst.txt`・`gen_vst.sh` も、`gen_vst3a.sh` と `main_vst3a.c` が要る分だけ作る。元の物は無いので、推定で書いた物は報告に「推定」と書く。
2. Python を使わない生成の段を node か Rust で作る。`mk_stubs.py` の役(import の stub の表)と、`patch_ordinals.py`・`patch_shd.py`・`patch_bmi.py` の役(lift した C の後処理)を、同じ入出力の形で置き換える。`build_vst3a_wasm.sh` の 29 行の `python3 -c`(PE の ImageBase を読む)も同じく置き換える。製品と本番の経路に Python を入れない(依頼者の規則)。
3. OTT の PE を ImageBase 0x2000000 に rebase し(`lift/crates/fe-pe/src/rebase.rs` の `rebase`。決定 0177 の先例)、`fe-lift lift --pdata` で C にする。seed は 2 の固定点で集める。import は 8 DLL の stub にし、既存の応答(`fle_imports.c`・`vst3a_imp.c` の行)で答える。答えの無い import は、名前のある停止(`vst3a_unanswered` の形)にする。
4. 製品の受け口の wasm を作る: `lift/harness/web/mt1/plugin-worker/build.sh`(新規)と `main_vst_ext.c`(新規)。`main_vst3a.c` の呼び出し順を、plugin-worker の口に合わせる(init で DllMain から setActive まで。その後は mt1_ext_wait で待つ。process は #{{124}})。wasm が import する名前は、`ott-host.ts` が満たす形にする。
   - build.sh の契約: `bash lift/harness/web/mt1/plugin-worker/build.sh <ott|serum2|vital> <インストーラの dir> <出力の dir>`。段は、(a) `check.mjs` で照合、(b) `extract.mjs` で VST3 本体を取り出し、(c) rebase、(d) lift、(e) emcc で wasm、(f) 出力は `<出力>/vst-ext.wasm`・`<出力>/<名前>.vst3`・`<出力>/seed.tsv`(app/test の MT1_OTT_DIR と同じ形)。どの段が失敗しても非 0 で終わり、標準エラーに段の名前を出す。
5. 道具 `tools/plugin-run/measure.mjs`(新規。node)を作る。契約は下の「測定の道具」。
6. 受け入れの式(下)を通す。通らなければ、作業ツリーを commit して終える。

## 測定の道具 `tools/plugin-run/measure.mjs`

この Issue で作る。#{{124}}・#{{027}}・#{{028}}・#{{029}} も使う。

- 呼び方: `node tools/plugin-run/measure.mjs <プラグインの dir> <ott|serum2|vital>`。dir は build.sh の出力の dir。
- 中では `app/src/plugins/plugin-client.ts` の `PluginInstance.start(nodePluginWorker, cfg)` を使う。cfg は、`module` が `vst-ext.wasm` を `WebAssembly.compile` したもの、`files` が `/<名前>.vst3` と `/seed.tsv`、`pluginPath` が `/<名前>.vst3`、`frames` 4096、`sampleRate` 44100、`maxBlocks` 4096。
- 標準出力は JSON の 1 行: `{"plugin":"<名前>","instantiated":真偽,"blockFrames":数,"nonSilent":真偽,"error":文字列か null}`。
- `instantiated`: start が READY で返った時 true。この段では `blockFrames` は 0、`nonSilent` は false。
- 終了コード: `error` が null なら 0、それ以外は 1。使い方の誤りは 64。
- 失敗時: `error` に、止まった段を書く(build の出力が無い、compile の失敗、start の失敗など)。

## 受け入れ

```accept
bash lift/harness/web/mt1/plugin-worker/build.sh ott "$PWD/lift/out-app/external" "$PWD/lift/out-app/plugins/ott" && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/plugin-run/measure.mjs "$PWD/lift/out-app/plugins/ott" ott > "$RUNNER_TEMP/ott.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());process.exit(r.error===null&&r.instantiated===true?0:1)' "$RUNNER_TEMP/ott.json"
```

判定の基準: `ott.json` の 1 行の生の値で、`error` が null、`instantiated` が true。加えて、`boot.json` が engine-idle で、`error` が null、`standIn` が false。

## 作業の進め方

- ビルド(engine と wasm)は約 35 分。フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ(この上限まで)。`sleep`・`timeout` をコマンドの先頭に書かない。背景のビルドを `tail --pid`・`wait` などで待たない。
- この Issue は 1 回の試行で終わらないことが多い。最初に `date -u` で開始を記録し、4 時間 30 分を過ぎたら、受け入れが通らなくても作業ツリーを commit し、報告を書いて終える。
- 段ごとに、実行した出力の要点を報告に残す(どの段で止まったかが次の試行の材料になる)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: 作った基盤のファイル(パス)、生成の道具(node か Rust)、OTT の lift の結果(関数数・未対応の行の数・答えた import の数)、measure の JSON の行(そのまま貼る)、変えたパス。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない(VST3 の link は既定で入っている。#{{026}})。
- Python を製品と本番の経路に入れない。生成の道具も node か Rust にする。
- ベンダー名と Cloudflare の識別子を書かない。インストーラの名前に含まれる会社名を、そのまま書かない(台帳の行と SHA-256 で参照する)。
- 外部のインストーラを実行しない。取り出しは、ファイルを読むだけにする。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。sleep・timeout コマンド・nohup・末尾の `&` は使わない。

## 他のタスクとの関係

- この Issue は、#{{124}}(process と AVX)と、#{{027}}(Serum 2)・#{{028}}(OTT の配信と既定の経路)・#{{029}}(Vital)の基盤。
- 同じ基盤のファイルを変えるため、他の Issue と衝突し得る。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
