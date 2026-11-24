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

## 前回の試行で分かったこと

run 45316190489(#{{123}} の 2 回目の試行。判定の段で落ちた。作業は保存された: `work.diff`。コミット 04fc7321・6d29ce64・23986780 を含む。次の試行は、その差分が載った作業ツリーから始まる)。

1. 受け入れの鎖は通った(実測。runner の値。nproc 4)。`build.sh ott`、`npm run build`、`cargo build`、`fe-build engine`(45 分 37 秒)、`fe-build bundle`、`boot-bundle.mjs`(`engine-idle`・error null・standIn false・elapsedMs 10,508)、`measure.mjs` が 0 で終わった。`measure.mjs` の 1 行: `{"plugin":"ott","instantiated":true,"blockFrames":0,"nonSilent":false,"error":null}`(wall 0.38 s、RSS 175 MB)。判定の段は、受け入れが 0 で終わった後で `guard/scan.sh` に進んだ。
2. 落ちた理由: `guard/scan.sh` が `python` を 1 件見つけた。`lift/harness/vst3a/build_vst3a_native.sh:35`(`if [ ! -e "$SRCD/countf_arrays.c" ]; then python3 "$HERE/count_fn.py" ...`)。この行は、COUNT=1 の経路(既定の COUNT=0 では実行されない)。`count_fn.py` は repo にも git の履歴にも無い。製品の経路の規則(Python を入れない)に反するので、直す。
3. 直すこと(次の試行の最初): その 1 行を、Python を使わない明示の失敗に変える。例: `if [ ! -e "$SRCD/countf_arrays.c" ]; then echo "COUNT=1 の数え上げの生成(count_fn)は、この版に無い。$SRCD/countf_arrays.c を先に置く" >&2; exit 1; fi`。OTT の経路には影響しない。直したら `bash guard/scan.sh lift/harness/vst3a/build_vst3a_native.sh` が 0 件。
4. 作った物(差分に入っている。元の物が無いので、推定で書いた): `lift/harness/vst/`(`vst_host.h`・`vst_hm.def`・`mkrows.sh`・`imports-vst.txt`・`gen_vst.sh`・node の `mk_stubs.mjs`・`merge_tables.mjs`・`seeds_scan.mjs`・`vfs-seed-vst.tsv`)、`lift/harness/web/mt1/plugin-worker/`(`build.sh`・`main_vst_ext.c`・`thread_stub.c`・`seeds/ott.txt` 1,994 件)、`tools/plugin-run/measure.mjs`、`app/src/plugins/ott-host.ts`(wasm の import の追加 7 件)、`lift/harness/vst3a/vst3a_imp.c`(答えの追加。重複定義 5 件の削除)、`build_vst3a_wasm.sh`・`build_vst3a_native.sh`・`gen_vst3a.sh`(ImageBase は `fe-pe pe-info`、stub 表は `mk_stubs.mjs`。patch の 3 段は外した)。
5. OTT の lift(実測): rebase(ImageBase 0x180000000 → 0x2000000)。関数 10,233(`.pdata` だけでは 6,676。reloc・switch 表・lea の seed を足した)。未対応の行 2,135(V・P で始まる命令が約 1,080。VEX 系と SSE 整数系。#{{124}} の対象)。import 380(名前付き 379・序数 1)。名前付きの停止 119、序数の停止 1。答えの行は harness 28・runtime 24。
6. 未確認: Serum 2 と Vital の build(`build.sh` は受けるが、この段では動かしていない)。`app/test/unit/plugins-ott.test.ts`(hash が #{{124}} の処理の後で決まるので、この段では走らせていない)。`vst3a_imp.c` の追加の答えを、Serum 2 の native harness で再試験していない。
7. `tools/` を変えたので、判定は `spec/guard.test.sh`・`spec/workflow.test.sh`・`spec/driver.test.sh` を走らせる。作業者も、commit の前に走らせて通す。

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

1. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば読む。`git status` と `git diff --stat` で、前回の差分(上の 4)が載っているかを確かめる。載っていなければ、上の 4 の物を、同じ内容で作る。
2. 上の「前回の試行で分かったこと」の 2・3 を直す(`build_vst3a_native.sh` の count_fn の行を、Python の無い形に)。
3. 受け入れを通す。engine のビルドを含み、約 35〜46 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は 5400000 にする。
4. `tools/` を変えたので、`spec/guard.test.sh`・`spec/workflow.test.sh`・`spec/driver.test.sh` を通す。
5. commit する(受け入れが通らなくても、作業の最後に作業ツリーを commit する)。push はしない。

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
