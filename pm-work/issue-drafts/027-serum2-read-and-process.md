---
draft: "027"
title: "Serum 2: VST3 ホストの上に、固有の対応を足し、1 ブロックの音声を処理する(wasm の大きさを先に見積もる)"
type: Task
milestone: "M6 外部プラグイン"
parent: "024"
blocked_by: ["040", "025", "026", "015", "124"]
labels: ["plugin-external"]
size: L
priority: Medium
project: signature-studio/3
---

## 目的

課題 C-5: Serum 2 を存在させる。#{{123}}・#{{124}} の VST3 ホスト(OTT で 1 ブロックの音声を処理できるもの)の上に、Serum 2 の固有の対応を足し、既定の経路(オプトイン無し)で、1 ブロックの音声を処理する。1 プラグイン 1 wasm で、起動後に届ける。

## 背景

- 入力: `lift/out-app/external/` の Serum 2 のインストーラ(`ci/artifacts.lock` の 2 行目。1,313,084,056 B)。VST3 本体は 19,236,864 B(`inputs/external-plugins.tsv` の Serum 2 の行。SHA-256 の先頭 501e7bb3…)。
- 前回の試行(run 45253143948。報告 agent-1336)の結果: 差し戻し(受理ではない)。
  - 受け入れの第 3 引数 `$RUNNER_TEMP/external/serum2.exe` は存在しない。作業の段は `$RUNNER_TEMP/external/*` を `lift/out-app/external/` へ移すので、その dir は空になる(`.github/workflows/agent.yml` の 569〜575 行)。受け入れは、`lift/out-app/external/` の入力を `build.sh` が取り出す形に直した。
  - ビルドと起動確認は通った: engine-idle、error null、standIn false。elapsedMs は 11,503(判定の段の実行)と 12,602(作業者の実行)。runner の値。P1(10 秒)を超える。この Issue の範囲外。
  - Serum 2 の wasm は作れず、読み込みも 1 ブロックの処理も観測していない。
  - 止まった理由: 基盤の VST3 ホストの header が無い(`vst_host.h`)。生成の Python(`mk_stubs.py`・`patch_*.py`)が無い。1 プラグイン 1 wasm の段が `fe-build` に無い。道具 `tools/plugin-run/measure.mjs` が無い。
  - 型は在る: 起動後の取得(`pack-assets.mjs --lazy-file`。#{{015}})と、1 プラグイン 1 wasm の分割(`hybrid --separate`。#{{102}})。
- 既存の先例: `lift/harness/vst3a/main_vst3a.c`(Serum 2 の harness。決定 0165)。`lift/harness/two/wrp1_vst3.c`(#{{026}} で既定の build に入った。TLS と IAT の hook。ImageBase 0x8000000 の module に作用する)。`lift/harness/seeds/wrp1-vst3-imports.txt`(Serum 2 の import の表。153 行)。
- 固有の論点(前回の試行で見えたこと。未確認の物は未確認と書く):
  - import は 13 DLL。`d3d11`・`d2d1`・`dwrite`・`dwmapi` など、描画の import が含まれる。
  - `wrp1_vst3.c` の注釈: Serum 2 の ordinal の import 17 件は `fle_unsupported` のまま(呼ばれたら名前のある停止)。
  - wasm の大きさは未測定。1 つの wasm の上限は 1 GB(#{{033}})。

## 前回の試行で分かったこと(2 回目)

- 2 回目の試行(run 45379401811。作業 #1336)は差し戻し。受け入れの `measure.mjs serum2` の 1 行: `{"plugin":"serum2","instantiated":false,"blockFrames":0,"nonSilent":false,"error":"start: plugin worker: Error: wasm abort"}`。読み込みと 1 ブロックの処理は、まだ確認できていない。
- 通った段: `build.sh serum2`・`npm run build`・`cargo build`・`fe-build engine`・`fe-build bundle`。manifest の lazy に `serum2.wasm`(134,961,671 B)が入った。boot は、1 回目は `state running` のまま 180 秒で打ち切られ(原因は未確認)、2 回目は engine-idle・error null・standIn false・`elapsedMs` 12,241(runner の値。P1 の 10 秒を超える)。
- wasm の大きさ: 見積もりは約 109 MB(Serum 2 の `.text` 10,593,334 B から、既存の engine の比で換算。範囲 50〜200 MB)。実測は `vst-ext.wasm` 134,961,671 B(1 GB の 13%)。分割は不要と判断した。lift は関数 39,715・扱えない箇所 13,440・生成 C 207,266,111 B。
- 止まった所: `LoadLibraryW` と DllMain は通る。次に `InitDll` の中で、`d2d1.dll` の序数 1(`D2D1CreateFactory`)の import thunk(`0x2a1b150`。IAT `0xfe3048`)に入り、`fle: unsupported instruction` で止まる。序数の import は全部で 17 件あり、今わかっているのは 1 件だけ(残り 16 件は未確認)。
- 試験で分かったこと(製品には入れていない): (a) `InitDll` を外すと先へ進むが、ホストの動きと違うので戻した(VST3 SDK の Windows 用のローダーは、export があれば `InitDll` を呼ぶ)。(b) `InitDll` を外すと、設定ファイル `Serum2Prefs.json` が無いことによる NULL 付近の読み出しで止まる。(c) 設定ファイルに `{}` を置くと、パス文字列をポインタとして読む所で止まる。設定ファイルの正しい中身は分からない。推測で置かない(下の 8)。
- 次の試行の作業ツリーに載る差分(判定は通らなかったので、commit ではなく差分として載る): `lift/harness/web/mt1/plugin-worker/main_vst_ext.c`(TLS の callback を DllMain の前に呼ぶ。イベント入力を持つ plugin に、最初のブロックで note-on)、`seeds/serum2.txt`(新規。`seeds_scan.mjs` の 2,994 件と、間接呼び出しで飛んだ先 `0x256ff20` の 1 件)、`pack.args`・`bundle.txt`(`--lazy-file` の 1 行)、`pm-work/findings/serum2-1336.md`(新規)。
- 既存の失敗: `lift/crates/fe-build/tests/plan/engine.txt` の試験(`lift/runtime/fle_webview2.c` が記録に無い)は、この Issue の変更とは無関係。#{{125}} で直る。この Issue では直さない(下の 11)。

## 読む物

- `lift/harness/vst3a/main_vst3a.c`、`lift/harness/two/wrp1_vst3.c`(1〜12 行)、`lift/harness/seeds/wrp1-vst3-imports.txt`
- #{{123}} と #{{124}} の成果物(基盤。`lift/harness/web/mt1/plugin-worker/build.sh`・`main_vst_ext.c`・`tools/plugin-run/measure.mjs`)
- #{{028}} の成果物(起動後の取得の型)

## やること

1. 最初に、wasm の大きさを見積もる(1 GB の上限)。Serum 2 の VST3 本体(19 MB の PE)から lift した C の大きさ(`fe-lift`)と、emcc の出力の大きさを見積もり、報告の最初に数字で書く。1 GB を超えるなら、分割の方式(1 つの wasm を複数に分ける、または、一部の関数を別の wasm にする)を決めてから作る。決めた方式と理由を報告に書く。
2. #{{123}} の harness を Serum 2 に当てる。`main_vst3a.c` の Serum 2 固有の部分(別の controller、MIDI の note、TLS callback)を、plugin-worker の口に合わせて使う。
3. import の stub: `wrp1-vst3-imports.txt` の行を使う。答えの無い import は名前のある停止にする。ordinal の 17 件は、呼ばれたら止まる形のままにし、呼ばれたかどうかを報告に書く。ただし序数 1(`D2D1CreateFactory`)は、下の 7 で答える。
4. 1 プラグイン 1 wasm: `lift/harness/web/mt1/plugin-worker/build.sh serum2 <dir> <出力>` で作る(#{{123}} の契約)。出力は `lift/out-app/plugins/serum2/`。
5. 既定の経路: 起動後の取得で届ける。`lift/harness/web/pack.args` に `--lazy-file lift/out-app/plugins/serum2/vst-ext.wasm=plugins/serum2.wasm` を足し、`lift/crates/fe-build/tests/plan/bundle.txt`(fe-build の計画の記録)を新しい引数に合わせる(#{{015}} の先例)。起動前に取る量は増やさない。
6. 道具 `tools/plugin-run/measure.mjs serum2` で、1 ブロックの音声を処理する(#{{124}} の契約)。
7. 序数 1(`D2D1CreateFactory`)は、Windows が Direct2D を使えない環境で返す、忠実な失敗で答える。`HRESULT` の失敗値(`D2DERR_*` か `E_NOINTERFACE`)を返し、出力のポインタは NULL にする。理由: Serum 2 の `InitDll` は GUI のための初期化で、ヘッドレスのホストでは Direct2D が使えない。使えない環境の答えは、Windows に在る忠実な挙動である。答えるのは序数 1 の 1 件だけ。残り 16 件の序数の import は、止まる形のままにする(呼ばれたら、1 件ずつ、Windows の忠実な答えを決める)。
8. `Serum2Prefs.json` の中身を推測して置かない。Windows の挙動を runtime に実装する: `SHGetKnownFolderPath`(FOLDERID_RoamingAppData など)・`SHGetFolderPathW`・`CreateDirectoryW`・`CreateFileW`(新規作成・書き込み・読み戻し)が、仮想のボリュームで Windows と同じように成功するかを、`lift/runtime/fle_vfs.c`・`fle_shell.c`・`fle_api2_shell.c` で確かめ、足りない所を直す。設定フォルダは `<ベンダー>\Serum 2`。設定が無ければ、プラグインが既定の値で作る、という経路が通ること。
9. **deploy の連鎖の一致**: `pack.args` に足す `--lazy-file lift/out-app/plugins/serum2/vst-ext.wasm=…` は、`fe-build bundle` の計画の中で作られること。main の #1337 の修正で、計画は `build.sh ott`(OTT の wasm)を最初に呼び、`fe-build external` がインストーラを取る形になった。この Issue は、`lift/crates/fe-build/src/main.rs` の OTT の段・`lift/crates/fe-build/tests/plan/bundle.txt`・`lift/crates/fe-build/tests/plan/external.txt` を読み、同じ形で `build.sh serum2 lift/out-app/external lift/out-app/plugins/serum2` を計画に足す。インストーラの取得は、`ci/artifacts.lock`・`inputs/external-plugins.tsv` の物(`external.txt` に在る)。`spec/workflow.test.sh` の (v) の検査が、これを見る(足さないと落ちる)。deploy は、外部プラグインのインストーラ(約 1.4 GB)を毎回取る。`deploy.yml` は、この Issue では直さない。#{{029}} が先に main に入った場合は、その計画の形の上に足す。`bundle.txt` の衝突は、両方を残す。
10. `tools/plugin-run/measure.mjs`(と `app/src/plugins/`)で、start が失敗したとき、Worker が捨てている harness の `vst:` の行を `error` に付け、失敗の理由が 1 行の JSON で分かるようにする。`app/` の変更を許す。新しい環境変数・`#ifdef` は足さない。
11. `lift/crates/fe-build/tests/plan/engine.txt` の試験の失敗は、#{{125}} が直すので、この Issue では直さない。

## 受け入れ

```accept
bash lift/harness/web/mt1/plugin-worker/build.sh serum2 "$PWD/lift/out-app/external" "$PWD/lift/out-app/plugins/serum2" && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node -e 'const m=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(Array.isArray(m.lazy)&&m.lazy.some((f)=>f.name==="serum2.wasm")?0:1)' "$PWD/lift/out-app/bundle/raw/fl-Oz/manifest.json" && node tools/plugin-run/measure.mjs "$PWD/lift/out-app/plugins/serum2" serum2 > "$RUNNER_TEMP/serum2.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());process.exit(r.error===null&&r.instantiated===true&&r.blockFrames>0&&r.nonSilent===true?0:1)' "$RUNNER_TEMP/serum2.json"
```

判定の基準: `serum2.json` の 1 行の生の値で、`error` が null、`instantiated` が true、`blockFrames` が 0 より大きい、`nonSilent` が true。加えて、`boot.json` が engine-idle・error null・standIn false で、`manifest.json` の lazy に `serum2.wasm` が在る。

## 作業の進め方

- ビルド(engine と wasm)は約 35 分。フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ。`sleep`・`timeout` をコマンドの先頭に書かない。背景のビルドを `tail --pid`・`wait` などで待たない。
- 1 回の試行で終わらないことが多い。最初に `date -u` で開始を記録し、4 時間 30 分を過ぎたら、受け入れが通らなくても作業ツリーを commit し、報告を書いて終える。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: wasm の大きさの見積もり(数字)、分割の方式(要るなら)、ordinal の import が呼ばれたか、起動後の取得での到着時刻、measure の JSON の行(そのまま貼る)、変えたパス(lift/・app/ を含む)。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- Python を製品と本番の経路に入れない。
- ベンダー名と Cloudflare の識別子を書かない。インストーラの名前に含まれる会社名を、そのまま書かない。
- 外部のインストーラを実行しない。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。sleep・timeout・nohup・末尾の `&` は使わない。

## 他のタスクとの関係

- 前提: #{{123}}(基盤 1a)、#{{124}}(基盤 1b)。#{{040}}(音声への圧迫の解消)、#{{025}}(入力)、#{{026}}(既定の link)、#{{015}}(1 プラグイン 1 wasm の型)。
- #{{028}}・#{{029}} と同じ基盤と、同じ `lift/harness/web/pack.args` を使う。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。

## 前回の試行の記録(下書きの履歴)

- run 45253143948(2026-11-23 19:52Z)で落ちた。`failed` を外し、`blocked_by` に #{{124}} を足して、基盤の完了まで起動しない形にした(PM の決定)。
