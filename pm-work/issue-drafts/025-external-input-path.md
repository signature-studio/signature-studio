---
draft: "025"
title: "外部プラグインのインストーラ(3 件)を、FL のインストーラと同じ経路で作業の段から読めるようにする"
type: Task
milestone: "M6 外部プラグイン"
parent: "024"
blocked_by: []
labels: ["plugin-external"]
size: M
priority: High
project: signature-studio/3
---

## 目的

外部プラグイン 3 件(Serum 2・OTT・Vital)のインストーラから、VST3 の本体(`.vst3` の中の DLL)を取り出す方法を確かめ、取り出した物の一覧を commit する。

## 背景

- 3 件のインストーラは、ワークフローの extract の段が R2 から取り、`tools/external-inputs/check.mjs` で大きさと SHA-256 を照合して、R2 のキャッシュに置いた。この作業の段が、それを `lift/out-app/external/` に置き、照合し直している(照合が通らなければ、この作業は始まらない)。ファイル名は `ci/artifacts.lock` の `installers/` の下の名前のまま。
- ワークフローの extract の段は、PM が作った。ワークフロー(`.github/`)は、変えない(ワーカー用 App が push できない)。
- 作業者には R2 の鍵が無い。インストーラを取り直さない。
- インストーラの形式は未確認。NSIS・Inno Setup・その他のどれかを、まず調べる。
- 取り出した本体は、リポジトリに commit しない(大きさと著作物のため。commit するのは一覧だけ)。取り出し先は `lift/out-app/` の下(git の無視の対象)か、`.tmp-*`。
- 実行環境は Linux。`7z`(p7zip)などが在るかを、確かめて使う。インストーラは実行しない(読むだけ)。
- 既存の道具: `app/scripts/lib/nsis.mjs`・`app/scripts/lib/nsis-script.mjs`(NSIS の読み。読むだけで、`app/` は変えない)。

## やること

1. 3 件それぞれの形式を調べ、VST3 の本体をどの方法で取り出せるかを、実際の出力で確かめる。
2. 取り出しの道具を `tools/external-inputs/` の下に作る(node。インストーラを実行しない)。既存の NSIS の読みを使えるなら使う。
3. 3 件から、`.vst3` の中の DLL を取り出し、`lift/out-app/` の下の作業の場所(git の無視の対象)に置く。
4. `inputs/external-plugins.tsv` を作って commit する。TAB 区切り。1 行目は見出し `plugin`・`path`・`bytes`・`sha256`・`imports`。2 行目以降は、取り出した DLL ごとに 1 行。`plugin` は `Serum 2`・`OTT`・`Vital` のどれか(この名前のまま)。`path` は、取り出した DLL のインストーラの中での相対パス(インストール先の接頭辞は書かない)。`bytes` と `sha256` は、取り出した DLL の大きさと SHA-256。`imports` は、PE が import する DLL の名前(小文字、`;` 区切り)。
5. 受け入れコマンドを実行し、その出力を報告に貼る。

## 受け入れ

```accept
f=inputs/external-plugins.tsv; test -f "$f" && test "$(awk -F'\t' '$1 == "Serum 2"' "$f" | wc -l)" -ge 1 && test "$(awk -F'\t' '$1 == "OTT"' "$f" | wc -l)" -ge 1 && test "$(awk -F'\t' '$1 == "Vital"' "$f" | wc -l)" -ge 1 && test "$(awk -F'\t' 'NR > 1 && (length($4) != 64 || $4 !~ /^[0-9a-f]+$/)' "$f" | wc -l)" -eq 0 && node tools/external-inputs/check.mjs lift/out-app/external > /dev/null
```

判定の基準: `inputs/external-plugins.tsv` があり、`Serum 2`・`OTT`・`Vital` の行がそれぞれ 1 つ以上、見出しを除く全ての行の `sha256` が 64 桁の 16 進、`node tools/external-inputs/check.mjs lift/out-app/external` が終了コード 0 で終わる。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: 3 件のインストーラの形式、取り出しの方法、取り出した件数、照合の結果(`check.mjs` の出力の JSON の 1 行)。
- 鍵の値、ホストの名前、アカウント ID は書かない。
- 変えたパス(commit したファイルだけ)を書く。

## 制約

- R2 の鍵・アカウント ID・ホストの名前を、探さず、書かない。
- インストーラを実行しない。`lift/out-app/external/` の 3 件を変えたり消したりしない。
- 取り出した本体を commit しない(`.dll`・`.vst3` は git の無視の対象に置く。`inputs/` には一覧だけ)。
- 製品(`app/`・`lift/` の製品の部分)・`ci/`・ワークフロー(`.github/`)を変えない。
- ベンダー名と Cloudflare の識別子を書かない。インストーラの名前に含まれる会社名を、そのまま書かない。
- 値を隠す命令(マスク)を置かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit の名義は環境の git の設定のまま。Claude の行を付けない。ブランチを作らない。
