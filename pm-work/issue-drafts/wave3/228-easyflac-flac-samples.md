---
draft: "228"
title: "共有の DLL 3: EasyFLAC_x64.dll を実モジュールにして、FLAC の音声の読み書きを揃える"
type: "Task"
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["227"]
labels: ["resources", "plugin-builtin", "verify"]
size: "S"
priority: "Medium"
project: "signature-studio/3"
---

## 目的

依頼者の指示に沿う範囲: FL が読みに行く共有の DLL のうち、日常の機能に要る物を、実モジュールにする。この束は `Shared\EasyFLAC_x64.dll`(FLAC の音声)の 1 本。

## 背景

- 分類(#{{225}}。`pm-work/findings/shared-dll-inventory.md`)では、日常の機能に要る(a)。FLAC のサンプルの読み込み(demo の曲の中に `.flac` のサンプルが在る: `JL Hat 184 Closed Hat 02.flac`)に要る見込み。
- 計測では、この DLL の要求は出なかった(未計測)。FLAC の書き出し(`File > #9 > #3`。窓の題名 `Rendering to ?untitled.flac`)は、保存のファイルを 1 つ増やした。読み込みの要求は、この計測の操作には無い。
- 一覧の綴り: `Shared\EasyFLAC_x64.dll`(394,240 B・SHA-256 cd2d670f29381613d496be517d7e90bfcb5c3aae9ee3bf43ccffdf84612bfd69・依存は kernel32)。
- `ci/extract-dlls.tsv` に行は無い。
- 要求の操作(どの読み込みで FLAC の DLL を求めるか)は、この下書きの時点で観測していない。束の作業の最初に、FLAC のサンプルを含む曲(demo の中から選ぶ)を開いて、`tools/shared-dll/measure.mjs` の計測の出力で、要求の操作を確かめて報告する。観測できなければ、その旨を書いて止まる(推測で値を決めない)。

## 入れ方

- `lift/plugins/easyflac/plugin.tsv` を 1 つ足す。見出しは 8 列(`key`・`guest`・`role`・`sha256`・`size`・`hook`・`args`・`omit`)、区切りは TAB。
- key: `easyflac_x64.dll`(bare 名 `EasyFLAC_x64.dll` を小文字にした物。要求が bare 名で来るかは、上の観測で確かめる)。guest は `Shared\EasyFLAC_x64.dll`、role は `補助`。sha256・size は上の値。
- role を `補助` にする理由: `Shared\` の DLL は Plugin database の項目ではない。入口(export の `CreatePlugInstance`)を持たない DLL は補助(`tools/plugin-classes/plugin-classes.mjs` の定義)。`本体` にすると、`plugin_db_names` がその folder 名 `Shared` を Plugin database の項目名にし、誤った項目を作る。補助の行は Plugin database に項目を作らない。この DLL が入口を持たないことは、束の作業で export を確かめて報告する。
- `hook`・`omit`・`args`: 手で推測しない。束の作業で `fe-pe`・`fe-lift` の結果から決めて、報告に書く。
- `ci/extract-dlls.tsv`: 行を足す必要があるかを確かめる(作業の木に取り出されていないなら、足す)。足した場合は、`plugin.tsv` が main に入ったときに外す。注意(確認済み): 抽出の表は `ci/extract-dlls.tsv` の行と `lift/plugins/*/plugin.tsv` の行の和で(`pm-work/workflow-design.md` の抽出の段)、plugin.tsv の行でも抽出される。表の行を足すなら、plugin.tsv と同じ commit で外す。

## 前提

- guest が `Shared\...` の 2 要素のため、既存の検査(`lift/crates/fe-build/src/plugins.rs` の `validate`・`plugin_db_names`、`app/scripts/lib/plugin-rows.mjs` の `validate`)は通らない(`validate` は guest を 3 要素以上と求める。`plugin_db_names` は guest の後ろから 2 番目の要素(`Shared`)を Plugin database の名にするため、そのままでは誤った項目を作る)。
- `plugin_db_names` は、本体の行を 1 つ要求する(本体の行が 0 の folder は、`want one` で落ちる)。補助だけの束を通すには、この要求を直す必要がある。
- この 2 点は #{{187}} の作業で直る見込み。#{{187}} の 4 回目の節は、guest の要素数と、role と名の扱いを、両方の検査と同じ commit で直す、と書く。この下書きの時点で #{{187}} は open で、main には無い。main の版で確かめる。直っていなければ、報告して止まる。
- blocked_by: #{{227}}(WavPack と OGG)が main に入ってから起動する。両方が `ci/extract-dlls.tsv` を触りうるため、順に流す。

## 受け入れ

```accept
test -f lift/plugins/easyflac/plugin.tsv && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/shared-dll/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --only demo,file-9 > "$RUNNER_TEMP/m.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l)).filter((r)=>!r.sessionEnd);process.exit(!L.some((r)=>(r.missing||[]).some((m)=>/LoadLibrary/.test(m.api)&&/easyflac_x64\.dll/i.test(m.path)))&&L.filter((r)=>r.step==="open-demo"||r.step==="file-9-3").every((r)=>r.state==="engine-idle")?0:1)' "$RUNNER_TEMP/m.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500;process.exit(L.length===4&&L.every(ok)?0:1)' "$RUNNER_TEMP/flow.jsonl" && test -z "$(git diff --name-only origin/main...HEAD -- . ':!lift/plugins' ':!ci/extract-dlls.tsv')"
```

判定の基準:
- 起動の確認(`boot.json`): engine-idle、`standIn` が false、`elapsedMs` が 12,500 以下。
- FLAC のサンプルを含む曲の読み込み(`open-demo`)と、FLAC の書き出し(`file-9-3`)の操作で、`EasyFLAC_x64.dll` の `LoadLibraryW` の 126 が無く、engine が engine-idle のまま。(この受け入れは、要求の操作が観測されていれば、その操作を入れて強める。)
- 注意(確認済み): `open-demo` は Asher Postman の曲(`tools/shared-dll/measure.mjs` の DEMO)で、FLAC のサンプル(`JL Hat 184 Closed Hat 02.flac`)は DEDZII の資産(`pm-work/findings/demo-assets-inventory.tsv` の `DEDZII` の行)に在る。そのため、この受け入れは FLAC の要求の操作を通らないことがある。背景の最初の観測で要求の操作を確かめ、計測の群に足してから、この受け入れを強める。
- 流れ A〜D(4 行。各行 `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true)。
- 差分: `lift/plugins/` と `ci/extract-dlls.tsv` の外に変更が無い。

## 読む物

- `inputs/shared-dll-requests.tsv`・`pm-work/findings/shared-dll-inventory.md`(この束の入力)
- `pm-work/findings/shared-dll-measure.jsonl`(計測の出力。`open-demo` と `file-9-3` の行)
- `lift/runtime/fle_load.c`(`load_by_name`・`not_found`)・`lift/runtime/fle_realmod.c`
- `lift/plugins/sytrus/plugin.tsv`(見出しと行の書式)

## 作業の進め方

- 作業の開始時刻を `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 最初に、上の「前提」を main の版で確かめる。直っていなければ、作業を進めず、その旨を報告に書いて終える。
- engine の作り直しは約 35〜40 分かかる。ビルドはフォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ(背景で起動して待たない)。
- 受け入れのコマンドの出力を、そのまま報告に貼る。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。既定の動作として直す。
- Python を使わない。OCR を使わない。
- AI のベンダー名と Cloudflare の識別子を、ファイルと報告に書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit の名義を変えない。commit・タグ・PR に Claude の行を付けない。必要の無いブランチを作らない。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。
