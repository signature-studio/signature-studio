---
draft: "W3-003"
title: "共有の DLL 3: EasyFLAC_x64.dll を実モジュールにして、FLAC の音声の読み書きを揃える"
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

依頼者の指示に沿う範囲: FL が読みに行く共有の DLL のうち、日常の機能に要る物を、実モジュールにする。この束は `Shared\EasyFLAC_x64.dll`(FLAC の音声)の 1 本。

## 背景

- 分類(#1511。`pm-work/findings/shared-dll-inventory.md`)では、日常の機能に要る(a)。FLAC のサンプルの読み込み(demo の曲の中に `.flac` のサンプルが在る: `JL Hat 184 Closed Hat 02.flac`)に要る見込み。
- 計測では、この DLL の要求は出なかった(未計測)。FLAC の書き出し(`File > #9 > #3`。窓の題名 `Rendering to ?untitled.flac`)は、保存のファイルを 1 つ増やした。読み込みの要求は、この計測の操作には無い。
- 一覧の綴り: `Shared\EasyFLAC_x64.dll`(394,240 B・SHA-256 cd2d670f29381613d496be517d7e90bfcb5c3aae9ee3bf43ccffdf84612bfd69・依存は kernel32)。
- `ci/extract-dlls.tsv` に行は無い。
- 要求の操作(どの読み込みで FLAC の DLL を求めるか)は、この下書きの時点で観測していない。束の作業の最初に、FLAC のサンプルを含む曲(demo の中から選ぶ)を開いて、`tools/shared-dll/measure.mjs` の計測の出力で、要求の操作を確かめて報告する。観測できなければ、その旨を書いて止まる(推測で値を決めない)。

## 入れ方

- `lift/plugins/easyflac/plugin.tsv` を 1 つ足す。見出しは 8 列(`key`・`guest`・`role`・`sha256`・`size`・`hook`・`args`・`omit`)、区切りは TAB。
- key: `easyflac_x64.dll`(bare 名 `EasyFLAC_x64.dll` を小文字にした物。要求が bare 名で来るかは、上の観測で確かめる)。guest は `Shared\EasyFLAC_x64.dll`、role は `本体`。sha256・size は上の値。
- `hook`・`omit`・`args`: 手で推測しない。束の作業で `fe-pe`・`fe-lift` の結果から決めて、報告に書く。
- `ci/extract-dlls.tsv`: 行を足す必要があるかを確かめる(作業の木に取り出されていないなら、足す)。足した場合は、`plugin.tsv` が main に入ったときに外す。

## 前提

- guest が `Shared\...` の 2 要素のため、既存の検査(`lift/crates/fe-build/src/plugins.rs` の `validate`・`plugin_db_names`、`app/scripts/lib/plugin-rows.mjs` の `validate`)は通らない(`validate` は guest を 3 要素以上と求める。`plugin_db_names` は guest の後ろから 2 番目の要素(`Shared`)を Plugin database の名にするため、そのままでは誤った項目を作る)。#1429 の作業で直る見込み。main の版で確かめる。

## 受け入れ

```accept
test -f lift/plugins/easyflac/plugin.tsv && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/shared-dll/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --only demo,file-9 > "$RUNNER_TEMP/m.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l)).filter((r)=>!r.sessionEnd);process.exit(!L.some((r)=>(r.missing||[]).some((m)=>/LoadLibrary/.test(m.api)&&/easyflac_x64\.dll/i.test(m.path)))&&L.filter((r)=>r.step==="open-demo"||r.step==="file-9-3").every((r)=>r.state==="engine-idle")?0:1)' "$RUNNER_TEMP/m.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500;process.exit(L.length===4&&L.every(ok)?0:1)' "$RUNNER_TEMP/flow.jsonl" && test -z "$(git diff --name-only origin/main...HEAD -- . ':!lift/plugins' ':!ci/extract-dlls.tsv')"
```

判定の基準:
- 起動の確認(`boot.json`): engine-idle、`standIn` が false、`elapsedMs` が 12,500 以下。
- FLAC のサンプルを含む曲の読み込み(`open-demo`)と、FLAC の書き出し(`file-9-3`)の操作で、`EasyFLAC_x64.dll` の `LoadLibraryW` の 126 が無く、engine が engine-idle のまま。(この受け入れは、要求の操作が観測されていれば、その操作を入れて強める。)
- 流れ A〜D(4 行。各行 `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true)。
- 差分: `lift/plugins/` と `ci/extract-dlls.tsv` の外に変更が無い。

## 読む物

- `inputs/shared-dll-requests.tsv`・`pm-work/findings/shared-dll-inventory.md`(この束の入力)
- `pm-work/findings/shared-dll-measure.jsonl`(計測の出力。`open-demo` と `file-9-3` の行)
- `lift/runtime/fle_load.c`(`load_by_name`・`not_found`)・`lift/runtime/fle_realmod.c`
- `lift/plugins/sytrus/plugin.tsv`(見出しと行の書式)
