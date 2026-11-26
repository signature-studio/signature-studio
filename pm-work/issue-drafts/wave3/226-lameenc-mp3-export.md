---
draft: "226"
title: "共有の DLL 1: LAMEenc_x64.dll を実モジュールにして、MP3 の書き出しの止まりを直す"
type: "Task"
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["resources", "plugin-builtin", "verify"]
size: "S"
priority: "High"
project: "signature-studio/3"
---

## 目的

依頼者の指示に沿う範囲: FL が読みに行く共有の DLL のうち、日常の機能に要る物を、実モジュールにして、読み込みの失敗(126)で engine を止めない。この束は `Shared\LAMEenc_x64.dll`(MP3 の書き出し)の 1 本。

## 背景

- 計測(#{{225}}。`pm-work/findings/shared-dll-measure.jsonl`)で、File > Export の MP3 の項目(`File > #9 > #1`)で描画の窓を開き、Start を押すと、`LoadLibraryW 126 "LAMEEnc_x64.dll"` が出て、直後に engine が `fle: indirect call to 0 is not a known function entry` で止まった(runner の値。この機械とは別枠)。項目 14(同じ MP3 の窓。`File > #9 > #14`)でも、同じ `LAMEEnc_x64.dll` の 126 と同じ止まりが出た。
- `LAMEEnc_x64.dll` は、ビルドに無い。`fle_load.c` の `load_by_name` は、名前が表に無ければ、仮想ボリュームを探し、見つからなければ `not_found`(NULL と 126)を返す。その NULL を使って止まる。
- 一覧の綴り: `Shared\LAMEenc_x64.dll`(465,720 B・SHA-256 0703adcd734071c74e938d9e6dd5e075594823ff376ca5d96df7dd8a1e880843)。依存は kernel32・user32・advapi32。
- 計測の一覧の行(`inputs/shared-dll-requests.tsv`): `LAMEenc_x64.dll	Shared\LAMEenc_x64.dll	file-9/file-9-1	10	true`(10 件の要求。止まり)。
- `ci/extract-dlls.tsv` に行は無い。作業の木に取り出されていないので、この束の最初に行を足す(下の「入れ方」)。
- 範囲に入れない物: `Shared\` の他の DLL(別の束)。Reverb(#{{187}})と ILSvg(#{{154}})は、それぞれの Issue が扱う。

## 入れ方

- `lift/plugins/lameenc/plugin.tsv` を 1 つ足す。見出しは 8 列(`key`・`guest`・`role`・`sha256`・`size`・`hook`・`args`・`omit`)、区切りは TAB。
- 行: key は `lameenc_x64.dll`(bare 名 `LAMEEnc_x64.dll` を小文字にした物。#{{187}} の決め方と同じ。`fle_load.c` の `load_by_name` は bare 名を小文字にし、`fle_realmod_find` が完全一致で比べる)。guest は `Shared\LAMEenc_x64.dll`。role は `補助`。sha256・size は上の値。
- role を `補助` にする理由: `Shared\` の DLL は Plugin database の項目ではない。入口(export の `CreatePlugInstance`)を持たない DLL は補助(`tools/plugin-classes/plugin-classes.mjs` の定義)。`本体` にすると、`plugin_db_names` がその folder 名 `Shared` を Plugin database の項目名にし、誤った項目を作る。補助の行は Plugin database に項目を作らない。この DLL が入口を持たないことは、束の作業で export を確かめて報告する。
- `hook`・`omit`・`args`: 手で推測しない。この下書きの時点では値を決めていない。束の作業で、`fe-pe`・`fe-lift` の結果から決めて、報告に書く。
- `ci/extract-dlls.tsv`: この DLL の行は無い。`plugin.tsv` を足すときは、抽出の表に行を足す必要があるかを確かめる(`fe-build` が作業の木の DLL を読むか)。行を足した場合は、同じ DLL の行を二重にしない。注意(確認済み): 抽出の表は `ci/extract-dlls.tsv` の行と `lift/plugins/*/plugin.tsv` の行の和で(`pm-work/workflow-design.md` の抽出の段。`app/scripts/extract-fl.mjs` が `app/scripts/lib/plugin-rows.mjs` で plugin.tsv を読む)、plugin.tsv の行でも抽出される。表の行を足すなら、plugin.tsv と同じ commit で外す。

## 前提

- guest が `Shared\...` の 2 要素のため、既存の検査(`lift/crates/fe-build/src/plugins.rs` の `validate`・`plugin_db_names`、`app/scripts/lib/plugin-rows.mjs` の `validate`)は通らない(`validate` は guest を 3 要素以上と求める。`plugin_db_names` は、guest の後ろから 2 番目の要素(`Shared`)を Plugin database の名にするため、そのままでは誤った項目を作る)。
- `plugin_db_names` は、本体の行を 1 つ要求する(本体の行が 0 の folder は、`want one` で落ちる)。補助だけの束を通すには、この要求を直す必要がある。
- この 2 点は #{{187}} の作業で直る見込み。#{{187}} の 4 回目の節は、guest の要素数と、role と名の扱いを、両方の検査と同じ commit で直す、と書く。この下書きの時点で #{{187}} は open で、main には無い。main の版で確かめる。直っていなければ、この束は報告して止まる。

## 受け入れ

```accept
test -f lift/plugins/lameenc/plugin.tsv && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/shared-dll/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --only file-9 > "$RUNNER_TEMP/m.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l)).filter((r)=>!r.sessionEnd);const hit=L.some((r)=>(r.missing||[]).some((m)=>/LoadLibrary/.test(m.api)&&/lameenc_x64\.dll/i.test(m.path)));const r1=L.find((r)=>r.step==="file-9-1");process.exit(!hit&&r1&&r1.state==="engine-idle"?0:1)' "$RUNNER_TEMP/m.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500;process.exit(L.length===4&&L.every(ok)?0:1)' "$RUNNER_TEMP/flow.jsonl" && test -z "$(git diff --name-only origin/main...HEAD -- . ':!lift/plugins' ':!ci/extract-dlls.tsv')"
```

判定の基準:
- 起動の確認(`boot.json`): engine-idle、`standIn` が false、`elapsedMs` が 12,500 以下。
- MP3 の書き出し(`file-9-1`)の操作で、`LoadLibraryW` の `LAMEEnc_x64.dll` の 126 が無く、engine が engine-idle のまま。
- 流れ A〜D(4 行。各行 `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true)。
- 差分: `lift/plugins/` と、`ci/extract-dlls.tsv`(この束が表の行を足した場合だけ)の外に変更が無い。

## 読む物

- `inputs/shared-dll-requests.tsv`・`pm-work/findings/shared-dll-inventory.md`(この束の入力)
- `pm-work/findings/shared-dll-measure.jsonl`(計測の出力。`file-9-1` と `file-9-14` の行)
- `lift/runtime/fle_load.c`(`load_by_name`・`not_found`)・`lift/runtime/fle_realmod.c`
- `lift/plugins/sytrus/plugin.tsv`(見出しと行の書式)
- `tools/shared-dll/measure.mjs`(計測の道具。`--only file-9` で MP3 の項目を含む群を動かす)

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
