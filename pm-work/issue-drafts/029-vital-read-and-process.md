---
draft: "029"
title: "Vital: VST3 ホストの上に、固有の対応を足し、1 ブロックの音声を処理する"
type: Task
milestone: "M6 外部プラグイン"
parent: "024"
blocked_by: ["040", "025", "026", "015", "124"]
labels: ["plugin-external"]
size: M
priority: Medium
project: signature-studio/3
---

## 目的

課題 C-5: Vital を存在させる(Serum 2・OTT と合わせた 3 件のうちの 1 件)。#{{123}}・#{{124}} の VST3 ホストの上に、Vital の固有の対応を足し、既定の経路(オプトイン無し)で、1 ブロックの音声を処理する。1 プラグイン 1 wasm で、起動後に届ける。

## 背景

- 入力: `lift/out-app/external/` の Vital のインストーラ(`ci/artifacts.lock` の 4 行目。41,808,960 B。Inno Setup 形式)。中身は 6 件(Vital.exe・Vital.vst3・Vital.dll・Vital.clap・vc_redist・アイコン)。VST3 本体は 15,571,240 B(`inputs/external-plugins.tsv` の Vital の行。SHA-256 の先頭 a622a2c9…)。
- 前回の試行(run 45253143948。報告 agent-1338)の結果: 差し戻し(受理ではない)。
  - 受け入れの第 3 引数 `$RUNNER_TEMP/external/vital.exe` は存在しない。作業の段が `$RUNNER_TEMP/external/*` を `lift/out-app/external/` へ移すので、その dir は空になる。受け入れは、`lift/out-app/external/` の入力を `build.sh` が取り出す形に直した。
  - ビルドと起動確認は通った: engine-idle、error null、standIn false、elapsedMs 11,345(判定の段の実行)と 12,376(作業者の実行)。runner の値。
  - 入力の照合、VST3 本体の取り出し、PE の読みまでは動いた。その後、wasm が無いところで止まった。
  - PE の形: x86-64(PE32+)。export は GetPluginFactory・InitDll・ExitDll を含む 5 件。import は 26 DLL(kernel32・user32・msvcp140・dbghelp・wininet・ws2_32・vcruntime140 など)。
  - 固有の論点(未確認): `IPPCODE`・`IPPDATA` という名前のセクションがあり、性能ライブラリの命令の切り替えの部分と見られる。fe-lift が扱える命令かどうかは未確認。`msvcp140`(C++ の標準ライブラリ)と `dbghelp` の import の扱いが要る。
- 基盤: #{{123}}(VST3 ホストの header と生成の道具、OTT の wasm)と #{{124}}(1 ブロックの処理)。

## 前回の試行で分かったこと(3 回目。run 45441361371)

- 前回: run 45441361371(19:20 → 20:37 UTC)。**判定(judge)は通った**(受け入れの終了コード 0)。落ちたのは main への push だけ(衝突 3 件)。作業の commit は `667fbda5`(衝突の解消と (v) の検査)と `310a94f5`(所見の記録)。push はしていない。差分は作業ツリーに載る。
- 衝突した 3 件: `lift/crates/fe-build/src/main.rs`・`lift/crates/fe-build/tests/plan/bundle.txt`・`spec/workflow.test.sh`。main 側には、#{{125}}(外部プラグインの段の修正: インストーラが無ければ、外部プラグインを bundle から省く)・`4d73666a`・Project の Status の変更(`01f90600`)が先に入っていた。
- 解消(`667fbda5`): `main.rs` は main 側(`external_present`・`pack_args`・そのテスト)を土台にし、`ott_steps` を `plugin_steps`(ott と vital の 2 段)に置き換えた。`bundle.txt` は 4 段、`bundle-no-external.txt` は 2 段。`spec/workflow.test.sh` の (v) は、`stage bundle: 3` の前提を直し、vital の作る段が pack-assets より前に在ることを検査に足し、固定の manifest に `plugins/vital.wasm` を足した。
- 検査: `cargo test --locked -p fe-build` 10 件通過。`spec/workflow.test.sh` 440 件 ok。`spec/guard.test.sh` 67 件 ok。`guard/scan.sh`・`guard/flags.sh` の指摘 0。
- 測定(runner の値。nproc 4、loadavg は受け入れの後 0.24 0.83 1.68): `{"plugin":"vital","instantiated":true,"blockFrames":4096,"nonSilent":true,"error":null}`。`out_hash=7d2d71f375347b15`(2 回目と同じ)。`maxabs` 0.357191・`rms` 0.143032。音の正しさは未確認(参照の出力が無い)。
- 起動: `boot.json` は engine-idle・error null・standIn false。`elapsedMs` は **12,608**(P1 の 10 秒を超える。過去の試行は 10,371〜12,376 ms)。Vital の wasm は起動後の取得なので、起動前の量は増えていない。超過の原因は未確認。
- lift(`--pdata`。runner の値): 関数 33,567(44 単位)・生成した命令 2,090,660 件・扱えた 1,885,965 件・扱えない 204,695 件(232 種)。扱えない命令の内訳は `.text` の 181,337 件と、`IPPCODE`(VA 0x28a6000〜)の 23,358 件。`IPPCODE` は 130,738 件のうち扱えた 107,380 件。`IPPDATA` は命令 0 件(データだけ)。扱えない命令の大半は AVX・AVX-512(`Vaddps`・`Vmovups`(EVEX) など)。
- import(`mk_stubs`): 501 件(序数 16 件を含む)。TABLE・既存の答えに無い 286 件のうち、runtime の既存の定義が 26 件、`vst_crt.c` の答えが 81 件(関数の定義 48 件・マクロ 33 件)、名前のある停止が 179 件(msvcp140 58・kernel32 20・winmm 15 など)。findings の数字は、この数え直しで置き換えた。
- wasm: `vst-ext.wasm` 115,538,297 B。manifest の lazy に `vital.wasm`(br 7,376,525 B)。`ott.wasm` は 27,803,120 B。
- 入力: VST3 本体 15,571,240 B・SHA-256 `a622a2c9…` が `inputs/external-plugins.tsv` と一致する。
- 未確認: P1(起動 10 秒以内)の超過の原因・音の正しさ・wasm の到着時刻(起動後の取得の時間)・OTT と Serum 2 の wasm(この受け入れの範囲では動かしていない)。

## 前回の試行で分かったこと(2 回目)

- 2 回目の試行(run 45379401811。作業 #{{029}})で、判定(取り直した main の `guard/judge.sh` と、この受け入れの全体)は通った。`measure.mjs` の 1 行: `{"plugin":"vital","instantiated":true,"blockFrames":4096,"nonSilent":true,"error":null}`。出力の行: `RESULT out_hash=7d2d71f375347b15 nonfinite=0 samples=8192 nonzero=8192 maxabs=0.357191 rms=0.143032 blocks=1`。元の VST3 本体の SHA-256 は `inputs/external-plugins.tsv` と一致する(`check.mjs` の 3 件も一致)。
- 落ちたのは main への push だけ。衝突したのは `lift/harness/web/pack.args` と `lift/crates/fe-build/tests/plan/bundle.txt` の 2 件。#{{028}}(OTT)が、同じ位置の隣に OTT の `--lazy-file` の行を足していたため。
- 衝突したときの直し方: (1) main の版を取る。(2) この Issue の変更(`pack.args` の `--lazy-file lift/out-app/plugins/vital/vst-ext.wasm=plugins/vital.wasm`・`bundle.txt` の同じ引数)だけを取り直す。(3) **OTT の行(#{{028}})は消さず、両方を残す**。(4) `bundle.txt` が、#{{125}} で固定のダミーの計画(`lift/plugins`・`tests/plugins/`)になっていたら、この Issue の外部プラグインの引数は計画の期待値に影響しない形になるので、`bundle.txt` の期待値を `fe-build` の出力から再生成する。
- deploy の連鎖の一致: `pack.args` に足す `--lazy-file` の wasm は、`fe-build bundle` の計画の中で作られること。計画は、#{{028}} の修正(#{{028}})で、`build.sh ott` を最初に呼び、`fe-build external` がインストーラを取る形になった。衝突を直すときは、`lift/crates/fe-build/src/main.rs` の OTT の段・`lift/crates/fe-build/tests/plan/bundle.txt`・`external.txt` を読み、同じ形で `build.sh vital lift/out-app/external lift/out-app/plugins/vital` を計画に足す。`spec/workflow.test.sh` の (v) の検査が、これを見る(足さないと落ちる)。この Issue が先に main に入った場合、#{{027}} は、その計画の形の上に足す。`bundle.txt` の衝突は、両方を残す。
- 前回の変更(判定は通り、push だけ落ちた commit 1 件。次の試行で commit として取り込まれる): `lift/harness/web/mt1/plugin-worker/vst_crt.c`(新規。CRT の初期化・onexit・数学・書式・RTTI・時刻・`ReplaceFileW` などに答える。答えた import は 81 件)、`seeds/vital.txt`(新規。間接呼び出しの飛び先の種)、`build.sh`、`main_vst_ext.c`(イベント入力を持つ plugin に、ブロック 0 で C4 のノートオン)、`lift/harness/vst/mk_stubs.mjs`(C++ のマングル名を C の識別子に変える)、`pack.args`・`bundle.txt`、`pm-work/findings/vital-read-1338.md`(新規)。
- 実測(runner の値。この機械とは別枠。nproc 4): lift の関数 27,077・扱えない命令 204,668(種類 229。大半は IPPCODE の AVX・AVX-512)。`vst-ext.wasm` 115,536,635 B(manifest の br 7,330,260 B)。manifest の lazy に `vital.wasm` が入った。`boot.json` は engine-idle・error null・standIn false、`elapsedMs` 10,566(P1 の 10 秒を超える。原因は未確認)。
- import: 非 TABLE の 286 件のうち、runtime の既存の定義が 26、`vst_crt.c` の答えが 81、名前のある停止のままが 179(msvcp140 58、kernel32 20、winmm 15 など。全件は `pm-work/findings/vital-read-1338.md`)。
- 注意: 起動確認(`boot-bundle.mjs`)は、#{{149}} の間欠の停止(`state running` のまま 180 秒)で落ちうる。その時は、この Issue の変更を直さず、報告に書く。

## 読む物

- #{{123}} と #{{124}} の成果物(`lift/harness/web/mt1/plugin-worker/build.sh`・`main_vst_ext.c`・`tools/plugin-run/measure.mjs`)
- `ci/artifacts.lock`(4 行目)、`inputs/external-plugins.tsv`(Vital の行)、`tools/external-inputs/inno.mjs`(Inno Setup の読み)
- #{{028}} の成果物(起動後の取得の型)

## やること

1. 入力の照合と取り出しを確かめる: `tools/external-inputs/check.mjs lift/out-app/external`、次に `extract.mjs` で VST3 本体を取り出し、SHA-256 が `inputs/external-plugins.tsv` と一致することを確かめる。
2. Vital の VST3 本体を fe-lift で C にする(`--pdata`)。`IPPCODE`・`IPPDATA` の命令が扱えるかを確かめ、扱えない命令の一覧を報告に書く。
3. import の stub(26 DLL)。`msvcp140`・`dbghelp`・`wininet`・`ws2_32` などの応答は、runtime の既存の応答を使う。答えの無い import は名前のある停止にする。
4. 1 プラグイン 1 wasm: `lift/harness/web/mt1/plugin-worker/build.sh vital <dir> <出力>` で作る(#{{123}} の契約)。出力は `lift/out-app/plugins/vital/`。
5. 既定の経路: 起動後の取得で届ける。`lift/harness/web/pack.args` に `--lazy-file lift/out-app/plugins/vital/vst-ext.wasm=plugins/vital.wasm` を足し、`lift/crates/fe-build/tests/plan/bundle.txt` を新しい引数に合わせる(#{{028}} と同じ型)。起動前に取る量は増やさない。
6. `tools/plugin-run/measure.mjs vital`(#{{124}} の契約)で、1 ブロックの音声を処理する。

## 受け入れ

```accept
bash lift/harness/web/mt1/plugin-worker/build.sh vital "$PWD/lift/out-app/external" "$PWD/lift/out-app/plugins/vital" && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node -e 'const m=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(Array.isArray(m.lazy)&&m.lazy.some((f)=>f.name==="vital.wasm")?0:1)' "$PWD/lift/out-app/bundle/raw/fl-Oz/manifest.json" && node tools/plugin-run/measure.mjs "$PWD/lift/out-app/plugins/vital" vital > "$RUNNER_TEMP/vital.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());process.exit(r.error===null&&r.instantiated===true&&r.blockFrames>0&&r.nonSilent===true?0:1)' "$RUNNER_TEMP/vital.json"
```

判定の基準: `vital.json` の 1 行の生の値で、`error` が null、`instantiated` が true、`blockFrames` が 0 より大きい、`nonSilent` が true。加えて、`boot.json` が engine-idle・error null・standIn false で、`manifest.json` の lazy に `vital.wasm` が在る。

## 作業の進め方

- ビルド(engine と wasm)は約 35 分。フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ。`sleep`・`timeout` をコマンドの先頭に書かない。背景のビルドを `tail --pid`・`wait` などで待たない。
- 1 回の試行で終わらないことが多い。最初に `date -u` で開始を記録し、4 時間 30 分を過ぎたら、受け入れが通らなくても作業ツリーを commit し、報告を書いて終える。
- push が衝突したら(前回の試行で、3 件が衝突した): main の版を取り、この Issue の変更(`build.sh vital`・`pack.args` の `--lazy-file … vital …`・`vst_crt.c`・`mk_stubs.mjs`・`main_vst_ext.c`・`seeds/vital.txt`)だけを取り直す。**OTT の行と、外部プラグインの計画**(`fe-build bundle` が `build.sh <名前>` を呼ぶ形。`tests/plan/bundle.txt`・`external.txt`・`bundle-no-external.txt`)を消さず、Vital の分を足す。他は変えない。衝突を直した後は、`cargo test --locked -p fe-build` と `spec/workflow.test.sh` を通す。受け入れのコマンドは、ジョブ末尾の判定で、もう一度走る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: `IPPCODE`・`IPPDATA` の lift の結果(扱えた命令と扱えなかった命令の数)、答えた import の数と、名前のある停止にした import の一覧、wasm の大きさ、到着時刻、measure の JSON の行(そのまま貼る)、変えたパス。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- Python を製品と本番の経路に入れない。
- ベンダー名と Cloudflare の識別子を書かない。インストーラの名前に含まれる会社名を、そのまま書かない。
- 外部のインストーラを実行しない。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。sleep・timeout・nohup・末尾の `&` は使わない。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 他のタスクとの関係

- 前提: #{{123}}(基盤 1a)、#{{124}}(基盤 1b)。#{{040}}(音声への圧迫の解消)、#{{025}}(入力)、#{{026}}(既定の link)、#{{015}}(1 プラグイン 1 wasm の型)。
- #{{027}}・#{{028}} と同じ `lift/harness/web/pack.args` を使う。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。

## 前回の試行の記録(下書きの履歴)

- run 45253143948(2026-11-23 19:52Z)で落ちた。`failed` を外し、`blocked_by` に #{{124}} を足して、基盤の完了まで起動しない形にした(PM の決定)。
