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

課題 C-3 と (4) の前提: FL Studio のインストーラの全ファイルの一覧を作る。言語・テーマ・テンプレート(C-3)と、内蔵のエフェクト・インストゥルメントの件数(C-4。約 141 は依頼者の申告)を、実物で確かめるための入力になる。

## 背景

- 作業ジョブは R2 の鍵を持たない。インストーラを取れるのは、R2 の鍵を持つ extract の段だけ(鍵は既存の仕組みで extract の段にだけある)。このため、一覧は extract の段で作り、作業ジョブがコミットする。
- 依頼者の要件(この Issue の目的): 一覧は、インストール先の全ファイル。作業者は、その一覧を読める。
- 以前の試み(main の履歴の commit `c6418a78` が、インストーラを読む道具と extract の段の追加を入れた。その道具は、旧の仕組みの削除で消えた)。しかし、その一覧は main に無い(`inputs/` も無い)。この Issue は、一覧が実際に出ることを確かめる。
- インストーラの入力: `ci/artifacts.lock` の 1 行目(FL Studio の exe。大きさ 1,106,033,456 B。SHA-256 は同ファイル)。
- 読みの道具: `app/scripts/lib/nsis.mjs`(`Installer`・`streamFile`)、`app/scripts/lib/nsis-script.mjs`(`Script`・`installTree`。インストール先の下の EXTRACTFILE の一覧)。`app/scripts/extract-fl.mjs` が、同じ読みで抽出している。
- 旧の一覧の仕様(この Issue の要件。作業者は、手段を決めてよい):
  - 列: `guest`(インストール先からの相対パス。接頭辞を外す)・`bytes`・`sha256`・`kind`(`dll`・`exe`・`other`)・`imports`(dll と exe の PE が import する DLL の名前。小文字、`;` 区切り)。
  - 展開するのは、`dll`・`exe`、`Plugins\` と `Data\` の下のファイル。他の行は `bytes`・`sha256`・`imports` を空にする(全件の展開の所要は未計測)。
  - 同じ guest パスが 2 回出たら、後の物を使う。行は guest パスの小文字の順。
  - インストール先の既定の値(会社名を含む絶対パス)は、表にもファイルにも書かない。読みには中立の接頭辞を使う。
- 一覧の置き場所(案): リポジトリの `inputs/fl-installer-files.tsv`(見出し行を 1 行残し、残りは C ロケールの順)。
- 旧の数字(pm-work/findings/survey-c4-builtin-plugins.md。runner の値): 抽出の木にある DLL は 14 件。インストーラの全件の数は未確認。

## 読む物

- `app/scripts/lib/nsis.mjs`(`Installer`・`streamFile`)
- `app/scripts/lib/nsis-script.mjs`(`Script`・`installTree`)
- `app/scripts/lib/lzma1.mjs`(展開)
- `app/scripts/extract-fl.mjs`(同じ読みで抽出する道具。インストール木の作り方)
- `ci/artifacts.lock`(インストーラの入力。大きさと SHA-256)
- `ci/extract-dlls.tsv`(抽出する DLL の表。SHA-256 つき)
- `.github/workflows/agent.yml`(extract の段と作業の段。作り直し中の版。段の名前は作業者が確かめる)
- `pm-work/findings/survey-c3-fl-resources.md`、`pm-work/findings/survey-c4-builtin-plugins.md`(旧の数字)

## やること

1. extract の段で、インストーラを取り(既存の取得の経路)、一覧を作る道具を動かす。道具の場所は作業者が決め、報告に書く。
2. 一覧の表を `$RUNNER_TEMP` に作り、キャッシュまたは成果物として作業の段に渡す。作業の段は、一覧を `inputs/fl-installer-files.tsv` にして commit する。手段は作業者が決める。
3. 一覧の件数、`Plugins\Fruity\Effects\` の DLL の件数、`Plugins\Fruity\Generators\` の DLL の件数、dll 行の SHA-256 の検査、を報告に書く。
4. 段の所要時間(extract の段の全体。取得・展開・一覧)を報告に書く。全件の展開の所要は、この Issue で初めて測る。
5. 一覧が取れなかった場合は、どの段で失敗したかを書き、「確認できない」と書く。

## 受け入れ

```accept
f=inputs/fl-installer-files.tsv; test -f "$f" && test "$(wc -l < "$f")" -ge 101 && test "$(awk -F'\t' 'index($1, "Plugins\\Fruity\\Effects\\") == 1 && $4 == "dll"' "$f" | wc -l)" -ge 85 && test "$(awk -F'\t' 'index($1, "Plugins\\Fruity\\Generators\\") == 1 && $4 == "dll"' "$f" | wc -l)" -ge 45 && test "$(awk -F'\t' '$4 == "dll" && (length($3) != 64 || $3 !~ /^[0-9a-f]+$/)' "$f" | wc -l)" -eq 0 && git log -1 --format=%s -- "$f" | grep -q .
```

判定の基準: 閾値(Effects の DLL 85 以上、Generators の DLL 45 以上)は、依頼者の申告(約 90・約 51)から余裕を取った案。実際の件数が違う場合は、報告に実数を書く。実数が閾値を下回る場合は、閾値を実数に合わせて直してよい(理由と実数を報告に書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: 一覧の件数(全体・Effects の DLL・Generators の DLL)、SHA-256 の検査の結果、extract の段の所要、一覧の作り方の選択と理由、変えたパス。
- インストール先の既定の値を、報告に書かない。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- 作業者に R2 の鍵を渡さない。鍵は、既存の extract の段でだけ使う。
- ベンダー名と Cloudflare の識別子を書かない。
- 製品のビルドの入力(`app/`・`lift/`)を変える場合は、main の deploy の仕組みが動くので、報告に書く。
- commit の名義は環境の git の設定のまま。Claude の行を付けない。
- sleep・timeout・nohup・末尾の `&` は使わない。

## 他のタスクとの関係

- #{{017}}(言語・テーマ・テンプレートの全件)と #{{020}}(抽出の対象の拡張)と #{{023}}(VFX 系の判定)が、この一覧を使う。
- `.github/workflows/agent.yml` を、別の担当が作り直している。作り直しと衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直す。
