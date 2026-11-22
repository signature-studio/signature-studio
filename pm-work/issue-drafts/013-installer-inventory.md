---
draft: "013"
title: "インストーラ全件の一覧を作り、作業者が読めるように commit する"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["inventory", "delivery"]
size: M
priority: High
project: signature-studio/3
---

## 目的

FL Studio のインストーラの全ファイルの一覧(インストール先からの相対パス、大きさ、SHA-256、種別、DLL の import)を、リポジトリに commit する。言語・テーマ・テンプレート(C-3)と、内蔵のエフェクト・インストゥルメント(C-4)の件数を、実物で確かめるための入力になる。

## 背景

- 一覧は、ワークフローの extract の段が、インストーラから `tools/fl-installer-files.mjs` で作った物。この作業の段が、作業ツリーの `inputs/fl-installer-files.tsv` に置いている(置いただけで、commit はしていない)。あなたは、そのファイルを検査して、commit する。
- ワークフローの extract の段は、PM が作った。ファイルは、作業ツリーに在る。ワークフロー(`.github/`)は、変えない(ワーカー用 App が push できない)。
- 作業者には R2 の鍵が無い。インストーラを取り直したり、表を作り直したりしない。
- 表の形式(TAB 区切り):
  - 1 行目: `#` で始まる注釈(方針)。
  - 2 行目: 見出し `guest`・`bytes`・`sha256`・`kind`・`imports`。
  - 3 行目以降: 行。`guest` はインストール先からの相対パス(バックスラッシュ区切り。例: `Plugins\Fruity\Effects\...`)。`kind` は `dll`・`exe`・`other`。`bytes` と `sha256` は、`dll`・`exe` の行と、`Plugins\`・`Data\` の下の行だけに入る(他は空)。`imports` は `dll`・`exe` の PE が import する DLL の名前(小文字、`;` 区切り)。
  - 行は `guest` の小文字の順。インストール先の既定の値(会社名を含む絶対パス)は、表に無い。
- 申告の数(依頼者): Effects の DLL 約 90、Generators の DLL 約 51(±2)。

## やること

1. 表を検査する: 注釈と見出しを除いた行数、`Plugins\Fruity\Effects\` の下の `dll` の件数、`Plugins\Fruity\Generators\` の下の `dll` の件数、`dll` の行の SHA-256 が 64 桁の 16 進かどうか。
2. 件数と申告の差を報告に書く(Effects・Generators の実数と、約 90・約 51 との差)。
3. VFX 系(`imports` に `opengl32.dll` を含む `dll` の行)の件数と名前の一覧を、報告に出す。
4. `inputs/fl-installer-files.tsv` を `git add` して、commit する(この 1 つの commit だけ。内容は変えない。並べ替えや整形はしない)。
5. 受け入れコマンドを実行し、その出力を報告に貼る。

## 受け入れ

```accept
f=inputs/fl-installer-files.tsv; d=$(awk -F'\t' '$1 != "guest" && $1 !~ /^#/' "$f" 2>/dev/null); test -f "$f" && test "$(printf '%s\n' "$d" | grep -c .)" -ge 100 && test "$(printf '%s\n' "$d" | awk -F'\t' 'index($1, "Plugins\\Fruity\\Effects\\") == 1 && $4 == "dll"' | wc -l)" -ge 85 && test "$(printf '%s\n' "$d" | awk -F'\t' 'index($1, "Plugins\\Fruity\\Generators\\") == 1 && $4 == "dll"' | wc -l)" -ge 45 && test "$(printf '%s\n' "$d" | awk -F'\t' '$4 == "dll" && (length($3) != 64 || $3 !~ /^[0-9a-f]+$/)' | wc -l)" -eq 0 && git log -1 --format=%s -- "$f" | grep -q .
```

判定の基準: 注釈と見出しを除いて 100 行以上。Effects の `dll` が 85 以上、Generators の `dll` が 45 以上。`dll` の行の SHA-256 は、すべて 64 桁の 16 進。ファイルが commit されている。

実数が閾値を下回ったときは、実数を報告に書いて終える。閾値は、この本文の受け入れで決まっていて、作業者は変えられない。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 件数などは、実行の出力から書く。推定と実測を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: 件数(全体・Effects の `dll`・Generators の `dll`)、申告との差、SHA-256 の検査の結果、VFX 系の一覧、変えたパス(commit したファイルだけ)。
- インストール先の既定の値を、報告に書かない。

## 制約

- R2 の鍵・アカウント ID・インストーラを、探さず、書かない。
- 製品(`app/`・`lift/`)・ワークフロー(`.github/`)・ハーネス(`tools/`・`guard/`・`spec/`)を変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit の名義は環境の git の設定のまま。Claude の行を付けない。ブランチを作らない。
