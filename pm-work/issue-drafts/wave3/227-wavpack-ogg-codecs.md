---
draft: "227"
title: "共有の DLL 2: WavpackDLL_x64.dll と oggio_x64.dll を実モジュールにして、既定の曲の読み込みの 126 と OGG の書き出しを揃える"
type: "Task"
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["226"]
labels: ["resources", "plugin-builtin", "verify"]
size: "M"
priority: "Medium"
project: "signature-studio/3"
---

## 目的

依頼者の指示に沿う範囲: FL が読みに行く共有の DLL のうち、日常の機能に要る物を、実モジュールにする。この束は、音声の形式の 2 本(WavPack と OGG)。

## 背景

- 計測(#{{225}}。`pm-work/findings/shared-dll-measure.jsonl`)で、既定の曲を開く(`welcome-default`)の度に、`LoadLibraryW 126 "WavpackDLL_x64.dll"` が出た(計測の 12 回。runner の値)。同じ操作で `ffmpeg\x64\ilvideo_x64.dll` も 126 になる(この束には入れない。分類の表を参照)。engine は止まらない。
- `oggio_x64.dll` は、OGG の書き出し(`File > #9 > #2`。窓の題名 `Rendering to ?untitled.ogg`)で、存在の確認(`GetFileAttributesW` 2)だけが出た。LoadLibrary の要求は観測されていない。書き出しの窓は開き、保存のファイルは 1 つ増えた(中身は確かめていない)。
- 一覧の綴り: `Shared\wavpackdll_x64.dll`(271,672 B・SHA-256 3bd37cc93bb9e7bda5da35ed1784324b3d7f71c818642ed189ec306c51431bad・依存は kernel32)。`Shared\oggio_x64.dll`(1,872,384 B・SHA-256 b26c544f5d3987331bbc946d4e5ec3084004a5fff5739cafb9f602c66e6466f9・依存は kernel32)。
- `ci/extract-dlls.tsv` の行: `oggio_x64.dll	lift/out-sampledec/oggio_x64.dll`(抽出済み。sample decoder の段が使う)。`WavpackDLL_x64.dll` の行は無い。
- 注意: `lift/out-sampledec/` の物が、どの経路で読まれるか(実行時の仮想ボリューム、または sample decoder の段)は、この下書きでは確かめていない。束の作業で確かめる。

## 入れ方

- `lift/plugins/wavpack/plugin.tsv` と `lift/plugins/oggio/plugin.tsv` を、それぞれ 1 つ足す。見出しは 8 列(`key`・`guest`・`role`・`sha256`・`size`・`hook`・`args`・`omit`)、区切りは TAB。
- key: `wavpackdll_x64.dll`(bare 名 `WavpackDLL_x64.dll` を小文字にした物)と、`oggio_x64.dll`(bare 名の小文字。OGG の要求が bare 名で来るかは未確認。来ないなら、key は同じ名前のまま)。guest は `Shared\...`、role は `補助`。sha256・size は上の値。
- role を `補助` にする理由: `Shared\` の DLL は Plugin database の項目ではない。入口(export の `CreatePlugInstance`)を持たない DLL は補助(`tools/plugin-classes/plugin-classes.mjs` の定義)。`本体` にすると、`plugin_db_names` がその folder 名 `Shared` を Plugin database の項目名にし、誤った項目を作る。補助の行は Plugin database に項目を作らない。この 2 本が入口を持たないことは、束の作業で export を確かめて報告する。
- `hook`・`omit`・`args`: 手で推測しない。束の作業で `fe-pe`・`fe-lift` の結果から決めて、報告に書く。
- `ci/extract-dlls.tsv`: `oggio_x64.dll` の行は、`lift/plugins/oggio/plugin.tsv` が main に入ったら外す(同じ DLL の行を二重にしない)。`WavpackDLL_x64.dll` の行は、作業の木に取り出す必要があるかを確かめて足すかを決める。注意(確認済み): 抽出の表は `ci/extract-dlls.tsv` の行と `lift/plugins/*/plugin.tsv` の行の和で(`pm-work/workflow-design.md` の抽出の段)、plugin.tsv の行でも抽出される。表の行を足すなら、plugin.tsv と同じ commit で外す。

## 前提

- guest が `Shared\...` の 2 要素のため、既存の検査(`lift/crates/fe-build/src/plugins.rs` の `validate`・`plugin_db_names`、`app/scripts/lib/plugin-rows.mjs` の `validate`)は通らない(`validate` は guest を 3 要素以上と求める。`plugin_db_names` は guest の後ろから 2 番目の要素(`Shared`)を Plugin database の名にするため、そのままでは誤った項目を作る)。
- `plugin_db_names` は、本体の行を 1 つ要求する(本体の行が 0 の folder は、`want one` で落ちる)。補助だけの束を通すには、この要求を直す必要がある。
- この 2 点は #{{187}} の作業で直る見込み。#{{187}} の 4 回目の節は、guest の要素数と、role と名の扱いを、両方の検査と同じ commit で直す、と書く。この下書きの時点で #{{187}} は open で、main には無い。main の版で確かめる。直っていなければ、報告して止まる。
- blocked_by: #{{226}}(LAMEenc)が main に入ってから起動する。両方が `ci/extract-dlls.tsv` を触りうるため、順に流す。

## 受け入れ

```accept
test -f lift/plugins/wavpack/plugin.tsv && test -f lift/plugins/oggio/plugin.tsv && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/shared-dll/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --only start,file-9 > "$RUNNER_TEMP/m.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l)).filter((r)=>!r.sessionEnd);const hit=L.some((r)=>(r.missing||[]).some((m)=>/LoadLibrary/.test(m.api)&&/wavpackdll_x64\.dll/i.test(m.path)));const idle=L.filter((r)=>r.step==="welcome-default"||r.step==="file-9-2").every((r)=>r.state==="engine-idle");process.exit(!hit&&idle?0:1)' "$RUNNER_TEMP/m.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500;process.exit(L.length===4&&L.every(ok)?0:1)' "$RUNNER_TEMP/flow.jsonl" && test -z "$(git diff --name-only origin/main...HEAD -- . ':!lift/plugins' ':!ci/extract-dlls.tsv')"
```

判定の基準:
- 起動の確認(`boot.json`): engine-idle、`standIn` が false、`elapsedMs` が 12,500 以下。
- 既定の曲を開く(`welcome-default`)で、`WavpackDLL_x64.dll` の `LoadLibraryW` の 126 が無い。OGG の書き出し(`file-9-2`)の操作が engine-idle のまま。
- 流れ A〜D(4 行。各行 `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true)。
- 差分: `lift/plugins/` と `ci/extract-dlls.tsv`(oggio の行を外す場合)の外に変更が無い。

## 読む物

- `inputs/shared-dll-requests.tsv`・`pm-work/findings/shared-dll-inventory.md`(この束の入力)
- `pm-work/findings/shared-dll-measure.jsonl`(計測の出力。`welcome-default` と `file-9-2` の行)
- `lift/runtime/fle_load.c`(`load_by_name`・`not_found`)・`lift/runtime/fle_realmod.c`
- `ci/extract-dlls.tsv`(oggio の行と、その注釈)
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
