---
draft: "025"
title: "外部プラグインのインストーラ(3 件)を、SHA-256 つきで作業の段に取る経路を決めて作る"
type: task
milestone: "M6 外部プラグイン"
parent: "024"
blocked_by: []
labels: ["task", "plugin-external", "needs-owner", "priority-p1"]
size: 4
needs_owner: true
---

## 目的

台帳 L-033 の (5) の前提: Serum 2・OTT・Vital の入力(インストーラ)を、作業の段が取れること。取る経路は、作業者の資格(L-011)と、抽出の段の既存の仕組みに沿って決める。

## 規模

規模: 大(想定 4 時間。経路の決定と、取得の確かめ)

## 背景

- 入力: `ci/artifacts.lock` の 2〜4 行(外部プラグインの 3 件。大きさと SHA-256 と ETag が書いてある。FL のインストーラは 1 行目)。
- 現状(旧の測定。pm-work/findings/survey-c5-external-plugins.md の 1 節): インストーラは runner に無い。R2 の artifact の鍵は、extract の段だけが持つ(FL のインストーラを取る段)。作業の段は持たない。
- 依頼者の決まり(台帳 L-011): 作業者に、依頼者の個人の資格を置かない。L-012: 依頼者の名義で GitHub に書き込まない。
- 経路の候補(案。依頼者が決める。QUESTIONS 2): (a) extract の段で、外部の 3 件も取り、抽出の木に置く(FL と同じ仕組み。鍵は extract の段にだけ在る)。(b) 作業の段に、読み取り専用の鍵を渡す(依頼者の判断が要る)。
- 取り出したインストーラから、VST3 のバンドル(プラグインの本体)を得る経路は未確認。

## 読む物

- `ci/artifacts.lock`(外部の 3 件の行。大きさ・SHA-256・ETag)
- `ci/external-inputs.sha256`(存在の確認。1 行目の物は artifacts.lock に無い。未確認)
- `.github/workflows/agent.yml`(extract の段の取得と鍵。作り直し中の版)
- `app/scripts/extract-fl.mjs`(抽出の仕組み)

## やること

1. 依頼者の判断(QUESTIONS 2)が出ているかを、報告の先頭に書く。出ていない場合は、この Issue は起動しない(needs-owner)。
2. 決まった経路で、外部の 3 件を取得し、`ci/artifacts.lock` の大きさと SHA-256 で検査する。
3. 取得した 3 件を、作業の段から読める場所に置く。置く場所は、抽出の木の中(`$RUNNER_TEMP`)とし、リポジトリには入れない。
4. 結果を報告に書く。

## 受け入れ

```accept
node tools/external-inputs/check.mjs "$RUNNER_TEMP/external" && test -s "$RUNNER_TEMP/external/serum2.exe" && test -s "$RUNNER_TEMP/external/ott.exe" && test -s "$RUNNER_TEMP/external/vital.exe"
```

判定の基準: 道具 `tools/external-inputs/check.mjs` が、3 件の大きさと SHA-256 を `ci/artifacts.lock` と照合して終了コード 0 で終わること。道具は、作業の段の中で作る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: 決まった経路と理由、3 件の大きさと SHA-256 の検査の結果、鍵を使った段の名前(鍵の値は書かない)。
- 鍵の値、ホストの名前、アカウント ID は書かない(L-006)。

## 制約

- 作業者に依頼者の個人の資格を置かない(L-011)。鍵を作業の段に渡すかは、依頼者の判断。
- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(L-002)。
- ベンダー名と Cloudflare の識別子を書かない(L-005・L-006)。
- 値を隠す命令(マスク)を置かない(L-031)。
- sleep・timeout・nohup・末尾の `&` は使わない。

## 他のタスクとの関係

- 027・028・029 が、この Issue の入力を使う。
- 依頼者の判断(QUESTIONS 2)が必要なので、needs-owner のまま起動しない。
