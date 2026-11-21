---
draft: "024"
title: "E 外部プラグイン(Serum 2・OTT・Vital を、既定の経路で読み込み、音声を処理する)"
type: epic
milestone: "M6 外部プラグイン"
parent: null
blocked_by: []
labels: ["epic", "plugin-external", "priority-p1"]
size: "子の合計"
needs_owner: false
---

## 目的

台帳 L-033 の (5): Serum 2・OTT など、多数の外部プラグインを存在させる。台帳 L-002: オプトインを作らず、既定で全部有効にする。台帳 L-035: プラグインは起動後にバックグラウンドで取る。

## 背景

- 入力: `ci/artifacts.lock` の 2〜4 行(外部プラグインの 3 件のインストーラ。R2 の artifact。大きさと SHA-256 は同ファイル)。FL Studio のインストーラ(1 行目)とは別。
- 作業ジョブに、この artifact の鍵は無い。FL のインストーラを取る extract の段だけが鍵を持つ(旧の測定。pm-work/findings/survey-c5-external-plugins.md の 1 節)。
- 製品の画面からは、プラグインの仕組みを呼んでいない。`app/src/plugins/` はテストからだけ使われる(survey-c5 の 1 節)。
- VST3 の link(`lift/harness/two/wrp1_vst3.c`)は、リンクされた時だけ働く。注釈は「opt-in」(台帳 L-002 に反する)。
- OTT の harness の source と build は、リポジトリに無い(`lift/harness/vst`・`lift/harness/web/mt1` が無い。survey-c5 の 2 節)。
- Vital の harness は、リポジトリに無い。Vital の入力(インストーラ)の中身は未確認。

## 完了の条件

- 子 025: 外部プラグインの入力を、作業の段が SHA-256 つきで取れる経路が決まり、作られている(依頼者の判断が要る。QUESTIONS 2)。
- 子 026: VST3 の link が既定で有効になっている(オプトインが無い)。
- 子 027・028・029: Serum 2・OTT・Vital のそれぞれが、既定の経路で読み込まれ、1 ブロックの音声を処理する(実測)。

## Sub-Issue の一覧

- 025 外部プラグインの入力の取得経路(needs_owner。needs-owner)
- 026 VST3 の link を既定で有効にする(blocked_by なし)
- 027 Serum 2 を読み込み、1 ブロックの音声を処理する(blocked_by 002, 025, 026, 015)
- 028 OTT を読み込み、1 ブロックの音声を処理する(blocked_by 002, 025, 026, 015)
- 029 Vital を読み込み、1 ブロックの音声を処理する(blocked_by 002, 025, 026, 015)

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(L-002)。オプトインを作らない。
- 作業者に依頼者の個人の資格を置かない(L-011)。外部の artifact の鍵を作業の段に渡す場合は、依頼者の判断を待つ(QUESTIONS 2)。
- ベンダー名と Cloudflare の識別子を、ファイルにも報告にも書かない(L-005・L-006)。インストーラの名前に含まれる会社名を、そのまま書かない。
- Python を製品に使わない(L-008)。
