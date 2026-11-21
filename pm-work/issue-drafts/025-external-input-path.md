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

Serum 2・OTT・Vital のインストーラを、作業の段が使えるようにする。3 件は R2 の artifact に既に在る。取得は FL Studio のインストーラと同じ経路で行う。作業の段は R2 の鍵を持たず、抽出の木から読む。

## 背景

- 入力: `ci/artifacts.lock` の 2〜4 行(外部プラグインの 3 件。大きさ・SHA-256・ETag が書いてある)。FL Studio のインストーラは 1 行目で、別の物。
- 現状: 作業の段には、この artifact の鍵が無い。R2 の鍵は、FL Studio のインストーラを取る extract の段だけが持つ。extract の段の取得は `app/scripts/extract-fl.mjs` と `.github/workflows/agent.yml` の extract の段の仕組みに従う。
- 方針(PM の決定): 3 件を、extract の段が R2 から取り、FL のインストーラと同じ仕組みで抽出の木に置く。作業の段は、復元された抽出の木から読む。作業の段に鍵は渡さない。
- 取り出したインストーラから、VST3 のバンドル(プラグインの本体)を得る経路は、#{{027}}〜#{{029}} で確かめる。

## 読む物

- `ci/artifacts.lock`(外部の 3 件の行。大きさ・SHA-256・ETag)
- `.github/workflows/agent.yml`(extract の段の取得と鍵。作り直し中の版。段の名前は作業者が確かめる)
- `app/scripts/extract-fl.mjs`(抽出の仕組み)

## やること

1. extract の段で、外部の 3 件を、FL のインストーラと同じ取得の仕組み(R2 の鍵を使う段)で取り、`lift/out-app/external/` に置く。抽出の木のキャッシュに入るので、作業の段から読める。キャッシュの鍵は、この変更を含むように変える(extract の段と作業の段の両方の鍵を揃える)。
2. 道具 `tools/external-inputs/check.mjs` を作る。引数は置き場所。3 件の大きさと SHA-256 を `ci/artifacts.lock` と照合し、結果を JSON の 1 行で出す。
3. 結果を報告に書く: 3 件の大きさと SHA-256 の検査の結果、extract の段の所要の増分、鍵を使った段の名前(鍵の値は書かない)。

## 受け入れ

```accept
node tools/external-inputs/check.mjs "$PWD/lift/out-app/external"
```

判定の基準: 道具が、3 件の大きさと SHA-256 を `ci/artifacts.lock` と照合して、終了コード 0 で終わること。道具の出力の JSON の 1 行を報告に貼る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 鍵の値、ホストの名前、アカウント ID は書かない。
- 変えたパス(`.github/` と `tools/` を含む)を報告に書く。

## 制約

- 作業の段に、依頼者の個人の資格を置かない。鍵を使うのは extract の段だけ。
- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- ベンダー名と Cloudflare の識別子を書かない。インストーラの名前に含まれる会社名を、そのまま書かない。
- 値を隠す命令(マスク)を置かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 抽出の段の鍵を変える場合は、変えたことを報告に書く(全件の抽出し直しになる)。

## 他のタスクとの関係

- #{{027}}・#{{028}}・#{{029}} が、この Issue の入力を使う。
- #{{020}}(内蔵の DLL の表)と、同じ extract の段を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直す。
