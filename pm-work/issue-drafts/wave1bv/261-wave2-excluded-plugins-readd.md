---
draft: "261"
title: "第 2 波で停止の型により束から外した plugin を、(B) の後に入れ直す: 追跡の表と受け入れ"
type: "Task"
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "021"
blocked_by: ["260"]
labels: ["plugin-builtin"]
size: "M"
priority: "High"
project: "signature-studio/3"
---

## 目的

第 2 波の束(内蔵エフェクト 1・4・5・9・10・11・13 の Issue)で、engine の停止の型により、束から外した plugin が 10 本ある。この Issue は、その 10 本を追う表であり、#{{260}}(プラグインの再生・描画の停止)が閉じた後に、10 本を入れ直して、その受け入れを行う。

束から外した 10 本の、停止の型の記録と、入れ直しの手順を、この Issue の本文に全部書く。束の Issue の本文の編集は PM が行う。

## 背景

- 束から外した plugin と、停止の型(PM の表。runner の値。この機械とは別枠)。`dir` は `lift/plugins/<dir>/` の名前、下書きは行の出所(`pm-work/issue-drafts/wave2/` の該当の番号のファイル。その中の `### <名前>(lift/plugins/<dir>/plugin.tsv)` の見出しの下の表が、その plugin の行):

| 束 | 名前 | dir | 行の出所(下書き) | 停止の型(記録) |
| --- | --- | --- | --- | --- |
| #1434 | Control Surface | `control-surface` | `041-effects-01.md` | 画素の形(`indirect call to`。束の報告の値 `0xd5ffccd1d5ffccd1`) |
| #1434 | Distructor | `distructor` | `041-effects-01.md` | 同 |
| #1437 | Frequency Splitter | `frequency-splitter` | `044-effects-04.md` | 像の中の番地(`indirect call to 0xf7e6c88`) |
| #1437 | Fruity Bass Boost | `fruity-bass-boost` | `044-effects-04.md` | `indirect call to`(値の記録は無い) |
| #1438 | Fruity Big Clock | `fruity-big-clock` | `045-effects-05.md` | 場面 d で失敗(`indirect call to 0` の型の一覧にも在る。値の記録は無い) |
| #1442 | Fruity HTML NoteBook | `fruity-html-notebook` | `049-effects-09.md` | 文字の形(`indirect call to`。値 `0x5400464f5400464f` ほか) |
| #1443 | Fruity LSD | `fruity-lsd` | `050-effects-10.md` | 画素の形(`indirect call to`。値 `0x53ff444b51ff4d54` ほか) |
| #1444 | Fruity NoteBook | `fruity-notebook` | `051-effects-11.md` | 画素の形(`indirect call to`。値 `0x36ff2a3136ff2a31`) |
| #1446 | Fruity Parametric EQ 2 | `fruity-parametric-eq-2` | `053-effects-13.md` | 像の中の番地(`indirect call to 0x11933b40`) |
| #1446 | Fruity Peak Controller | `fruity-peak-controller` | `053-effects-13.md` | 未対応の命令 |

- この表に入れない物: #1435(Edison・Effector・Emphasis)と #1436(Emphasizer・FL Studio Mobile Rack FX・Frequency Shifter)。この 2 件の束は、束ごと #{{260}} の完了を待つ(その 2 件の blocked_by)。Effector・Emphasis・FL Studio Mobile Rack FX は、既に `lift/plugins/` に行が在る。
- 行の値: 上の下書きの表の行を、そのまま書く(見出し 8 列: `key`・`guest`・`role`・`sha256`・`size`・`hook`・`args`・`omit`。区切りは TAB)。直してよいのは、行の `hook`・`args`・`omit` だけ。その理由を報告に書く。
- 像の領域: 10 本を入れると、`lift/crates/fe-build/src/plugins.rs` の像の領域(`GLOBAL_BASE`)に入るかは未確認。入らなければ、`fe-build engine` が `the image area is full` で止まる。その出力を報告に貼って止まる。
- この Issue の起動の前提: `startup-cap-bytes`(`pm-work/findings/wave2-round2.md`)を、この Issue の受け入れが読む。#{{259}}(起動の前に取る量)が閉じていないと、流れ A〜D の `preBootBytes` が上限を超えて落ちうる。その場合は、その旨を報告に書く。

## 入れ方

1. 10 本の `lift/plugins/<dir>/plugin.tsv` を、上の下書きの行どおりに置く。
2. `node tools/place-plugin-dlls.mjs` が DLL を木に置く。`missing` が出たら、行の guest と sha256 を `inputs/fl-installer-files.tsv` と照らす。
3. 1 回の bundle(`fe-build engine` と `fe-build bundle`)で 10 本を入れ、`tools/plugin-run` で 1 本ずつ確かめる(道具の出力は 1 本 1 行)。

## やること

1. 10 本の行を入れ、bundle を作る(ビルドは 1 回。作業の中で 2 回目のビルドをしない。直した後の確認は、判定(受け入れ)に任せる)。
2. `plugin-run` の 10 行を、報告に貼る。停止した物は、その型(`engineStop` の stop・message・lastFle)を報告に書く。
3. 停止の型が #{{260}} と同じ型(画素・文字・0・像の番地)なら、#{{260}} の報告の原因を参照する。型が違えば、その型を報告に書く。
4. 「未対応の命令」(Fruity Peak Controller): #{{260}} の範囲に入らない。直すのに命令の実装が要るなら、直さず、報告に書いて止まる。
5. 停止した plugin が 1 本でもあれば、この Issue の受け入れは通らない。報告に、その型と、残りの 9 本の結果を書く。

## 受け入れ

```accept
test -f lift/plugins/control-surface/plugin.tsv && test -f lift/plugins/distructor/plugin.tsv && test -f lift/plugins/frequency-splitter/plugin.tsv && test -f lift/plugins/fruity-bass-boost/plugin.tsv && test -f lift/plugins/fruity-big-clock/plugin.tsv && test -f lift/plugins/fruity-html-notebook/plugin.tsv && test -f lift/plugins/fruity-lsd/plugin.tsv && test -f lift/plugins/fruity-notebook/plugin.tsv && test -f lift/plugins/fruity-parametric-eq-2/plugin.tsv && test -f lift/plugins/fruity-peak-controller/plugin.tsv && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && node tools/place-plugin-dlls.mjs && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "Control Surface" "Distructor" "Frequency Splitter" "Fruity Bass Boost" "Fruity Big Clock" "Fruity HTML NoteBook" "Fruity LSD" "Fruity NoteBook" "Fruity Parametric EQ 2" "Fruity Peak Controller" > "$RUNNER_TEMP/plugin-run.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length>=10&&L.every((r)=>r.instantiated===true&&r.processed===true&&r.error===null&&r.engineStop===null)?0:1)' "$RUNNER_TEMP/plugin-run.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const cap=Number(fs.readFileSync("pm-work/findings/wave2-round2.md","utf8").match(/^startup-cap-bytes: (\d+)$/m)[1]);const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=20000&&r.preBootBytes<=cap&&(!/^[BD]$/.test(r.flow)||r.welcomeClosed===true);process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(L.find((r)=>r.flow===f)))?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && bash guard/scan.sh
```

判定の基準:
- 10 本の `plugin.tsv` が在る(上の `test -f` の 10 件)。
- 起動確認(`boot.json`): engine-idle・standIn が false・error が null・elapsedMs が 20,000 以下。
- 道具(`plugin-run`、場面は既定の auto)の 10 行: instantiated と processed が true、error と engineStop が null。
- 流れ A〜D の 4 行: 条件が通り、preBootBytes が `pm-work/findings/wave2-round2.md` の `startup-cap-bytes` 以下。
- `npm test --prefix app` が通る。`bash guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 10 本の表(名前・processed・error・engineStop の型・elapsedMs)、(b) 直した行(`hook`・`args`・`omit`)があれば、その行と理由、(c) 像の領域の結果(入ったか。`fe-build engine` の出力の行)、(d) 起動確認の elapsedMs、流れ A〜D の 4 行、preBootBytes、(e) `bash guard/scan.sh` の結果。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告の末尾に、判定が通って push されると `deploy.yml` が動く(`lift/` に触れるため)と書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- Python・OCR を使わない。
- sleep・timeout のコマンド・nohup・末尾の `&` を使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- `git add` は、変えたファイルを 1 つずつ指定する(`git add -A`・`git add .` は使わない)。
- commit の名義は、環境の git の設定のまま。commit・タグ・PR に、AI の署名の行を付けない。必要の無いブランチを作らない。
- 差分は `lift/plugins/` の 10 本の `plugin.tsv` だけ。それ以外を変えるなら、報告に書いて止まる。
- この機械で製品のビルドや測定をしない。製品の作業は Actions の作業ジョブで行う。

## 作業の進め方

- 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。
- ビルド(`fe-build engine`)は 1 回にする。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- 作業を始めてから 3 時間 15 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(判定に約 2 時間 30 分掛かる)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: #{{260}}(プラグインの再生・描画の停止)。
- #{{259}}(起動の前に取る量)は、この Issue と並行してよい(blocked_by は付けない)。ただし、#{{259}} が閉じていないと、流れ A〜D の preBootBytes が上限を超えて落ちうる。
- 束の Issue の本文は、PM が直す(この表の番号と、束から外した理由を反映する)。
