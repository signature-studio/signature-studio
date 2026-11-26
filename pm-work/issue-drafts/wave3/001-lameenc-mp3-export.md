---
draft: "W3-001"
title: "共有の DLL 1: LAMEenc_x64.dll を実モジュールにして、MP3 の書き出しの止まりを直す"
type: "Task"
milestone: "PM が決める"
parent: "1511"
blocked_by: []
labels: ["shared-dll"]
size: "S"
priority: "PM が決める"
project: "signature-studio/3"
---

## 目的

依頼者の指示に沿う範囲: FL が読みに行く共有の DLL のうち、日常の機能に要る物を、実モジュールにして、読み込みの失敗(126)で engine を止めない。この束は `Shared\LAMEenc_x64.dll`(MP3 の書き出し)の 1 本。

## 背景

- 計測(#1511。`pm-work/findings/shared-dll-measure.jsonl`)で、File > Export の MP3 の項目(`File > #9 > #1`)で描画の窓を開き、Start を押すと、`LoadLibraryW 126 "LAMEEnc_x64.dll"` が出て、直後に engine が `fle: indirect call to 0 is not a known function entry` で止まった(runner の値。この機械とは別枠)。項目 14(同じ MP3 の窓。`File > #9 > #14`)でも、同じ `LAMEEnc_x64.dll` の 126 と同じ止まりが出た。
- `LAMEEnc_x64.dll` は、ビルドに無い。`fle_load.c` の `load_by_name` は、名前が表に無ければ、仮想ボリュームを探し、見つからなければ `not_found`(NULL と 126)を返す。その NULL を使って止まる。
- 一覧の綴り: `Shared\LAMEenc_x64.dll`(465,720 B・SHA-256 0703adcd734071c74e938d9e6dd5e075594823ff376ca5d96df7dd8a1e880843)。依存は kernel32・user32・advapi32。
- 計測の一覧の行(`inputs/shared-dll-requests.tsv`): `LAMEenc_x64.dll	Shared\LAMEenc_x64.dll	file-9/file-9-1	10	true`(10 件の要求。止まり)。
- `ci/extract-dlls.tsv` に行は無い。作業の木に取り出されていないので、この束の最初に行を足す(下の「入れ方」)。
- 範囲に入れない物: `Shared\` の他の DLL(別の束)。Reverb(#1429)と ILSvg(#1395)は、それぞれの Issue が扱う。

## 入れ方

- `lift/plugins/lameenc/plugin.tsv` を 1 つ足す。見出しは 8 列(`key`・`guest`・`role`・`sha256`・`size`・`hook`・`args`・`omit`)、区切りは TAB。
- 行: key は `lameenc_x64.dll`(bare 名 `LAMEEnc_x64.dll` を小文字にした物。#1429 の前例)。guest は `Shared\LAMEenc_x64.dll`。role は `本体`。sha256・size は上の値。
- `hook`・`omit`・`args`: 手で推測しない。この下書きの時点では値を決めていない。束の作業で、`fe-pe`・`fe-lift` の結果から決めて、報告に書く。
- `ci/extract-dlls.tsv`: この DLL の行は無い。`plugin.tsv` を足すときは、抽出の表に行を足す必要があるかを確かめる(`fe-build` が作業の木の DLL を読むか)。行を足した場合は、同じ DLL の行を二重にしない。

## 前提

- guest が `Shared\...` の 2 要素のため、既存の検査(`lift/crates/fe-build/src/plugins.rs` の `validate`・`plugin_db_names`、`app/scripts/lib/plugin-rows.mjs` の `validate`)は通らない(`validate` は guest を 3 要素以上と求める。`plugin_db_names` は、guest の後ろから 2 番目の要素(`Shared`)を Plugin database の名にするため、そのままでは誤った項目を作る)。#1429 の作業で直る見込み。main の版で確かめる。直っていなければ、この束は報告して止まる。

## 受け入れ

```accept
test -f lift/plugins/lameenc/plugin.tsv && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/shared-dll/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --only file-9 > "$RUNNER_TEMP/m.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l)).filter((r)=>!r.sessionEnd);const hit=L.some((r)=>(r.missing||[]).some((m)=>/LoadLibrary/.test(m.api)&&/lameenc_x64\.dll/i.test(m.path)));const r1=L.find((r)=>r.step==="file-9-1");process.exit(!hit&&r1&&r1.state==="engine-idle"?0:1)' "$RUNNER_TEMP/m.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500;process.exit(L.length===4&&L.every(ok)?0:1)' "$RUNNER_TEMP/flow.jsonl" && test -z "$(git diff --name-only origin/main...HEAD -- . ':!lift/plugins')"
```

判定の基準:
- 起動の確認(`boot.json`): engine-idle、`standIn` が false、`elapsedMs` が 12,500 以下。
- MP3 の書き出し(`file-9-1`)の操作で、`LoadLibraryW` の `LAMEEnc_x64.dll` の 126 が無く、engine が engine-idle のまま。
- 流れ A〜D(4 行。各行 `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true)。
- 差分: `lift/plugins/` の外に、plugin.tsv の行を外す(抽出の表の行を足した場合を除く)以外の変更が無い。

## 読む物

- `inputs/shared-dll-requests.tsv`・`pm-work/findings/shared-dll-inventory.md`(この束の入力)
- `pm-work/findings/shared-dll-measure.jsonl`(計測の出力。`file-9-1` と `file-9-14` の行)
- `lift/runtime/fle_load.c`(`load_by_name`・`not_found`)・`lift/runtime/fle_realmod.c`
- `lift/plugins/sytrus/plugin.tsv`(見出しと行の書式)
- `tools/shared-dll/measure.mjs`(計測の道具。`--only file-9` で MP3 の項目を含む群を動かす)
