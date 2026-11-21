---
draft: "023"
title: "OpenGL を使う VFX 系の DLL を一覧から判定し、後回しの一覧を作る"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: ["013"]
labels: ["plugin-builtin", "inventory"]
size: S
priority: High
project: signature-studio/3
---

## 目的

依頼者の指示: OpenGL を使う VFX 系は後回しにしてよい。後回しにする物の一覧を、インストーラの import の情報で作り、理由(OpenGL の DLL を import する)を書く。後回しの物は、#{{020}} の表から外す。

## 背景

- 判定の材料: インストーラの一覧(#{{013}})の `imports` 列(dll と exe の PE が import する DLL の名前。小文字、`;` 区切り)。判定の規則: import に `opengl32.dll` を含む DLL を、VFX 系(後回し)とする。
- 旧の文字列の判定(pm-work/findings/survey-c4-builtin-plugins.md の手順 4。目安にすぎない): `grep -a -c -i opengl32` で、FLEngine に 1 行、Fruity の 3 件に 0 行。PE の import 表の解析ではない。
- 動的に読む DLL は、import 表に出ない(推定)。判定の限界は報告に書く。

## 読む物

- `inputs/fl-installer-files.tsv`(#{{013}} の成果物。列: guest・bytes・sha256・kind・imports)
- `pm-work/findings/survey-c4-builtin-plugins.md`(手順 4 の旧の判定)
- `lift/crates/fe-pe/src/main.rs`(`pe-info` の import の読み。67〜105 行)

## やること

1. #{{013}} の一覧から、`Plugins\Fruity\Effects\`・`Plugins\Fruity\Generators\` の DLL のうち、`opengl32.dll` を import する物を取り出す。取り出した件数と名前を報告に書く。
2. `inputs/vfx-deferred.tsv` を作る。列: `guest`(タブ)`reason`(タブ)`imports`。見出し行を 1 行残す。`reason` は `opengl32.dll を import`。
3. 判定の限界(動的に読む DLL、import 表の無い DLL)を、報告に書く。
4. 後回しの物が、#{{020}} の表から外れていることを確かめ、結果を報告に書く。

## 受け入れ

```accept
node -e 'const fs=require("fs");const inv=fs.readFileSync("inputs/fl-installer-files.tsv","utf8").trim().split("\n").slice(1).map((l)=>l.split("\t"));const want=inv.filter((r)=>r[3]==="dll"&&/(^|;)opengl32\.dll(;|$)/.test(r[4]||"")&&/^plugins\\fruity\\/i.test(r[0])).map((r)=>r[0]);const got=fs.readFileSync("inputs/vfx-deferred.tsv","utf8").trim().split("\n").slice(1).map((l)=>l.split("\t")[0]);process.exit(want.length>0&&want.every((g)=>got.includes(g))&&got.length===want.length?0:1)'
```

判定の基準: 一覧で opengl32.dll を import する Fruity の DLL が 1 件以上あり、`inputs/vfx-deferred.tsv` の件数と一致すること。一覧が無い場合は「未確認」と書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。この Issue は実行を伴わない(一覧の読み)なら「実エンジン: 実行なし(一覧の読みのみ)」と書く。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: 後回しの件数と名前の一覧、判定の規則、限界。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- ベンダー名と Cloudflare の識別子を書かない。
- Python を製品に使わない。判定は既存の道具か JavaScript で行う。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- blocked_by: #{{013}}。
- #{{020}} が、この Issue の一覧を使う。
- 後回しの範囲は、この Issue の判定で決まる。判定の結果(件数と名前)は、この Issue の報告に出す。後回しの物の実装は #{{039}} で、全群の後に行う。
