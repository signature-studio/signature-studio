# AGENTS.md

このプロジェクトでは、FL Studio(Windows 版)のエンジンを x86-64 から C / wasm へ変換(lift)し、ブラウザで動かします。

## 目標
- P1: 10 秒以内に起動する。
- P2: UI が 120 fps で動く。
- P3: Song Mode で音が途切れない。
- 目標どうしがぶつかるときは、P3、P2、P1、サイズの順に優先します。
- 完成までに必ず達成する課題は、`queue/plan.md`(台帳 L-033)。

## 役割
- 作業者(Haiku 5.5): GitHub Actions の runner で動きます。1 つのタスクを 1 つのジョブで、新しいセッションで行います。
- 監査・受付(Sonnet 5.5): 1 つの固定のセッションです。依頼者の指示を受け、全体を見て、止まっているところを調べて直します。タスクの分解や計画の見直しも、ここで行います。
- 駆動: `.github/workflows/agent.yml` の fill(`driver/fill.sh`)です。待ち行列から、閾値(`queue/threshold`)まで作業ジョブを起動します。待ちが 100 件を下回ると、生成器(`driver/generate.sh`)がタスクを足します。

ワークフローとハーネス(駆動、判定の道具、テスト、設定)は、エージェントが自分で改善します。改善はテストを通してから入れます。

## 作業者の手順
待ち行列は Issues です。題名がタスクの名前、ラベル(ready・running・failed)が状態です。タスクの本文は `bash driver/task.sh <名前>` が出します(`queue/tasks/<名前>.md`、`ci/tasks/<名前>.md`、または `queue/kinds/` の型)。受け入れコマンドは、本文の中の `accept: ` で始まる行に書きます。この行が無いタスクは、待ち行列に入りません。タスクを足すときは、`queue/tasks/<名前>.md` を main に入れ、同じ題名の Issue をラベル ready で作ります。
1. タスクを読みます。目的と受け入れコマンドが書いてあります。
2. 受け入れコマンドを実行し、その出力をそのまま報告に貼ります。
3. 変更を commit します。push はしません。ジョブの最後の段が `guard/judge.sh` で判定し、通ったときだけ push します。
4. 通らないときは、その出力を貼って終えます。

## 守ること(依頼者の指示。台帳 `ledger/instructions.tsv` にある)
- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない(オプションは作らず、既定で全部有効にする)。
- Python は調査と検証にだけ使い、製品と本番の処理には使わない。
- ベンダー名と、Cloudflare の識別子をファイルに書かない。
- commit の名義を変えない(環境の git の設定のまま)。commit・タグ・PR に Claude の行を付けない。
- 必要の無いブランチを作らない。
- 他のプロジェクトのセッション、portal の道具、依頼者の設定と資格情報に触れない。
- リポジトリの外に、ファイル・ユニット・設定を作らない。一時ファイルはリポジトリの中の `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。このマシンでは行わない。
- GitHub Actions の実行は、依頼者が承認した(台帳 L-027)。作業ジョブの起動は駆動(`driver/fill.sh`)だけが行い、`gh workflow run` を手で打たない。`schedule` は足していない。`app/`・`lift/`・`ci/` に触れる push は main の `deploy.yml` を動かすので、報告に書く。

## 監査・受付の決まり
- 依頼者の発言のうち、「質問:」で始まる物だけが質問です。質問には、道具を使わずに文章で答えます。
- それ以外の発言は、指示か是正の要求です。台帳(`ledger/instructions.tsv`)に書き足し、指された動作を行います。理由や経緯は書きません。
- 状態は、起きるたびに添付されます。状態を確かめるだけのコマンドは打ちません。添付が欠けているときと、異常があるときだけ、道具で調べます。

## GitHub に登録済みの値
- Secrets: `OPENCODE_AUTH_JSON`(opencode の認証)、`R2_CACHE_ASSETS_ACCESS_KEY_ID`・`R2_CACHE_ASSETS_SECRET_ACCESS_KEY`(ビルドのキャッシュ用の R2)、`R2_ARTIFACT_ACCESS_KEY_ID`・`R2_ARTIFACT_SECRET_ACCESS_KEY`(インストーラなどを置く R2)、`CLOUDFLARE_API_TOKEN`(デプロイ用)
- Variables: `OPENCODE_MODEL`(作業者のモデル)、`CLOUDFLARE_ACCOUNT_ID`

## 置き場所
- `app/` `lift/` `tools/` `patches/` `ci/`: 製品
- `driver/` `queue/`: 駆動(補充・生成器・タスクの本文・待ち行列の状態)と、閾値・生成器の入力と型・手で書いたタスク
- `guard/`: 判定の道具(`judge.sh` が全体の判定。`scan.sh`・`flags.sh`・`deploy-lint.sh` を使う)
- `spec/`: テスト(`spec/guard.test.sh`・`spec/driver.test.sh`)。`ledger/`: 依頼者の指示の台帳
