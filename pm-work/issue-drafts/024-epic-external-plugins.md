---
draft: "024"
title: "E 外部プラグイン(Serum 2・OTT・Vital を、既定の経路で読み込み、音声を処理する)"
type: epic
milestone: "M6 外部プラグイン"
parent: null
blocked_by: []
labels: ["epic", "plugin-external", "priority-p1"]
size: "子の合計"
---

## 目的

課題 C-5: Serum 2・OTT など、多数の外部プラグインを存在させる。対象は Serum 2・OTT・Vital の 3 件(依頼者の指示)。既定で全部有効にし、オプトインは作らない。プラグインは起動後にバックグラウンドで取る。

## 背景

- 入力: `ci/artifacts.lock` の 2〜4 行(外部プラグインの 3 件のインストーラ。R2 の artifact に既に在る。大きさと SHA-256 は同ファイル)。FL Studio のインストーラ(1 行目)とは別。
- 取得の経路は、FL Studio のインストーラと同じ: extract の段が R2 から取り、抽出の木に置く。作業の段は R2 の鍵を持たず、抽出の木から読む(子 025)。
- 製品の画面からは、プラグインの仕組みを呼んでいない。`app/src/plugins/` はテストからだけ使われる。
- VST3 の link(`lift/harness/two/wrp1_vst3.c`)は、リンクされた時だけ働く。注釈は「opt-in」で、今は製品の build に入っていない(子 026)。
- OTT の harness の source と build は、リポジトリに無い(`lift/harness/vst`・`lift/harness/web/mt1` が無い)。harness は作り直す(子 028)。
- Vital の harness は、リポジトリに無い。Vital のインストーラの中身は未確認(子 029)。

## 完了の条件

- 子 025: 外部プラグインの 3 件を、extract の段が取り、作業の段から読める。大きさと SHA-256 が `ci/artifacts.lock` と一致する。
- 子 026: VST3 の link が既定で有効になっている(オプトインが無い)。
- 子 027・028・029: Serum 2・OTT・Vital のそれぞれが、既定の経路で読み込まれ、1 ブロックの音声を処理する(実測)。

## Sub-Issue の一覧

- 025 外部プラグインの入力を、extract の段で取り、作業の段から読めるようにする(blocked_by なし)
- 026 VST3 の link を既定で有効にする(blocked_by なし)
- 027 Serum 2 を読み込み、1 ブロックの音声を処理する(blocked_by 040, 025, 026, 015)
- 028 OTT を読み込み、1 ブロックの音声を処理する(blocked_by 040, 025, 026, 015)
- 029 Vital を読み込み、1 ブロックの音声を処理する(blocked_by 040, 025, 026, 015)

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 作業の段に、依頼者の個人の資格を置かない。外部の artifact の鍵は extract の段だけが持つ。
- ベンダー名と Cloudflare の識別子を、ファイルにも報告にも書かない。インストーラの名前に含まれる会社名を、そのまま書かない。
- Python を製品に使わない。
