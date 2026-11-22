---
draft: "103"
title: "内蔵のプラグインの DLL を分類し、第 2 波の入力(inputs/plugin-classes.tsv)を commit する"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: []
labels: ["inventory", "plugin-builtin"]
size: M
priority: Medium
project: signature-studio/3
---

## 目的

内蔵のプラグインの DLL を、実物で分類して表にする。第 2 波(内蔵の DLL を束ねた Issue の起票)の入力になる。どの DLL がプラグイン本体で、どれが補助の DLL か、hook 点が要るか、OpenGL を使う VFX 系かを決める。

## 背景

- 一覧: `inputs/fl-installer-files.tsv`(インストーラ全件。列は guest・bytes・sha256・kind・imports)。この中で、`Plugins\Fruity\Effects\` と `Plugins\Fruity\Generators\` の下の `kind` が `dll` の行は、Effects が 112 件、Generators が 70 件(計 182 件)。依頼者の申告は、Effects 約 90 件・Generators 約 51 件(±2 ほど違いうる)。
- 実物: `lift/out-app/host-extra/plugins/` に、この Issue の作業の段が、R2 のキャッシュから置く(`plugin-builtin` のラベルの作業だけ)。木は、インストール先からの相対のパス(`/` 区切り)で、`Plugins/Fruity/Effects/...` と `Plugins/Fruity/Generators/...`。木の直下に `MANIFEST.tsv`(guest・bytes・sha256 の表)が在る。
- 後回しの一覧: `inputs/vfx-deferred.tsv`(OpenGL を import する DLL の 11 件。#{{023}} の結果)。
- 既存の 3 件: Fruity Balance・Fruity Filter・Fruity Delay 2 は、`lift/harness/hooks/fruity-delphi-effect-hook-points.txt` の 3 つの RVA(0x17bc0 などの 3 点)を使う。同じファイルの注釈によると、Delphi の別のプラグイン(Frequency Shifter・Parametric EQ 2・Soundgoodizer)は、同じ 3 点を持たず、それぞれ別の hook 点のファイルが要る。
- PE の読み: `tools/fl-installer-files.mjs` が、`dll` の import(小文字の DLL 名の一覧)を読んでいる。その読みを再利用してよい。
- 既存の登録: `lift/harness/b2d2/modules.tsv`(8〜10 行)。

## 読む物

- `inputs/fl-installer-files.tsv`、`inputs/vfx-deferred.tsv`
- `lift/out-app/host-extra/plugins/MANIFEST.tsv` と、木の中の DLL(作業の段が置く)
- `tools/fl-installer-files.mjs`(PE の読み)
- `lift/harness/hooks/fruity-delphi-effect-hook-points.txt`(注釈と、既存の 3 件の RVA)
- `lift/harness/b2d2/modules.tsv`(8〜10 行: 既存の 3 件の登録)

## やること

1. 道具 `tools/plugin-classes/`(node)を作る。Effects と Generators の `dll` の全件(182 件)について、大きさ・SHA-256(`MANIFEST.tsv` の値と一致すること)・import・export の名前を読む。PE の読みは `tools/fl-installer-files.mjs` と同じ方法にする。
2. 表 `inputs/plugin-classes.tsv` を作る(TAB 区切り。1 行目が見出し。1 行が 1 件の `dll`)。見出しと列の意味:
   - `パス`: インストール先からの相対のパス(`inputs/fl-installer-files.tsv` の guest と同じ書き方。バックスラッシュ区切り)。
   - `大きさ`・`SHA-256`: 展開後の大きさと、SHA-256。
   - `種別`: `Effects` か `Generators`(パスの接頭辞で決める)。
   - `本体か補助か`: `本体` か `補助` のどちらか一方だけ。本体 = プラグインの入口を持つ DLL(例: export に `CreatePlugInstance`)。補助 = 本体が使う DLL で、入口を持たない(例: 同じフォルダの `engine_x64.dll` など)。判定の根拠は `備考` に書く。
   - `export 名`: export の名前(`;` 区切り)。
   - `import した DLL`: import した DLL の名前(小文字。`;` 区切り)。
   - `hook の要否`: `既存` か `要`。既存 = 既存の hook 点ファイル(上の 3 つの RVA)と同じ 3 点を、初期化で使う Delphi の土台を持つ。要 = 同じ 3 点を持たない Delphi の DLL(別の hook 点が要る)。Delphi の土台を持たない DLL は `不要` と書く。
   - `VFX`: import に `opengl32.dll` があれば `yes`、無ければ `no`。
   - `備考`: 判定の根拠と、特記事項。インストール先の既定の絶対パス(会社名を含む)は書かない。
3. `inputs/vfx-deferred.tsv` の 11 件と、`VFX` が `yes` の行が一致するかを確かめ、差を報告に書く。
4. 件数と申告の差を報告に書く(Effects と Generators それぞれの、本体・補助の件数、hook の要否の件数、VFX の件数)。
5. `inputs/plugin-classes.tsv` を commit する(この 1 つのファイルと、道具 `tools/plugin-classes/` だけ)。

## 受け入れ

```accept
awk -F'\t' 'FNR==NR { if (FNR>1 && $4=="dll" && (index($1,"Plugins\\Fruity\\Effects\\")==1 || index($1,"Plugins\\Fruity\\Generators\\")==1)) { want[$1]=1; nw++ } next } FNR>1 && $1!="" { rows++; if ($5 !~ /^(本体|補助)$/) bad++; if (($1 in want) && !($1 in seen)) { seen[$1]=1; got++ } } END { exit !(nw==182 && rows==182 && got==182 && bad==0) }' inputs/fl-installer-files.tsv inputs/plugin-classes.tsv
```

判定の基準: `inputs/plugin-classes.tsv` が在り、`inputs/fl-installer-files.tsv` の Effects・Generators の `dll` の 182 件が、各 1 行ずつ全て在り(重複は数えない)、表の全ての行の `本体か補助か` が `本体` か `補助` である(行数は 182 行)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 件数は、道具の出力から書く。推定と実測を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: Effects・Generators それぞれの本体・補助の件数、`hook の要否` が `要` の件数と、その DLL の名前の一覧、`VFX` が `yes` の件数と一覧(`inputs/vfx-deferred.tsv` との一致)、DLL の数(112・70)と申告(約 90・約 51)との差の説明、変えたパス(commit したファイルだけ)。
- インストール先の既定の絶対パス(会社名を含む)を、報告に書かない。

## 制約

- 製品(`app/`・`lift/`)・ワークフロー(`.github/`)を変えない。既存の `inputs/` の中身を変えない(新しい `inputs/plugin-classes.tsv` を足すだけ)。
- インストーラを取り直さない。R2 の鍵・アカウント ID を探さず、書かない。
- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit の名義は環境の git の設定のまま。Claude の行を付けない。ブランチを作らない。

## 他のタスクとの関係

- #{{023}}(OpenGL を使う VFX 系の判定と、後回しの一覧)と、#{{020}}(抽出の表の揃え)と、表の内容を照合する。
- #{{101}}・#{{102}}(起動の読みの遅延、プラグインの分割)と並行して進めてよい。この Issue の表は、第 2 波の束ね方の入力になる。
- 補助の DLL を、本体と同じ束に入れるかは、この表の結果を見て第 2 波の起票の時に決める。
