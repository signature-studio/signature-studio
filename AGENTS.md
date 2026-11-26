# AGENTS.md

このプロジェクトでは、FL Studio(Windows 版)のエンジンを x86-64 から C / wasm へ変換(lift)し、ブラウザで動かします。

## 目標
- P1: 10 秒以内に起動する。
- P2: UI が 120 fps で動く。
- P3: Song Mode で音が途切れない。
- 目標どうしがぶつかるときは、P3、P2、P1、サイズの順に優先します。
- 完成までに必ず達成する課題は、`pm-work/plan.md`(台帳 L-033)。

## 役割
- 作業者(Haiku 5.5): GitHub Actions の runner で動きます。Issue 1 件を、1 つのジョブで、新しいセッションで行います。
- 監査・受付(Sonnet 5.5): 1 つの固定のセッションです。依頼者の指示を受け、全体を見て、止まっているところを調べて直します。Issue の分解や計画の見直しも、ここで行います。
- 駆動: `.github/workflows/agent.yml` です。Issues とコメントのイベントで起動し、ブロッカーの無い Issue Type が Task の Issue を、即座に起動します。ブロッカーが閉じたときは、そのブロッカーが塞いでいた Issue を起動します。`workflow_dispatch` と `schedule` は置きません。設計は `pm-work/workflow-design.md` です。

ワークフローとハーネス(駆動、判定の道具、テスト、設定)は、エージェントが自分で改善します。改善はテストを通してから入れます。

## 作業者の手順
タスクは GitHub の Issue です。題名がタスクの名前で、本文にタスクの全ての情報を書きます。受け入れコマンドは、本文の ` ```accept ` のコードブロックに、1 つだけ、中身は 1 行のコマンドで書きます。このブロックが無いと、作業は失敗して、Issue にその旨がコメントされます。
作業者に渡されるのは、この `AGENTS.md` と Issue の本文だけです。本文に無い情報を前提にしません。
作業ツリーに `.prior-work.md` が在れば、1 の前に最初に読みます(前回の作業の取り込みの結果と、次にやること)。
1. タスクを読みます。目的と受け入れコマンドが書いてあります。
2. 受け入れコマンドを実行し、その出力をそのまま報告に貼ります。
3. 変更を commit します。push はしません。ジョブの最後の段が `guard/judge.sh` で判定し、通ったときだけ push します。
4. 通らないときは、その出力を貼って終えます。

- ターンを終えるのは、受け入れが通って commit した時か、続けられない理由を報告に書く時だけ。バックグラウンドのコマンドの完了を待つために、ターンを終えない(ヘッドレスの実行では、ターンを終えるとそこで作業が終わり、完了の通知は届かない)。
- 受け入れが通らなくても、作業の最後に、作業ツリーを commit する(未 commit の変更が残ると、判定は「commit していない変更がある」で落ちる。commit 済みで受け入れが落ちた場合も、差分が保存され、次の試行が続きから行う)。
- 長く掛かるコマンド(ビルド・起動確認・測定。数分〜1 時間)は、フォアグラウンドで実行し、シェルツールの `timeout` 引数(ミリ秒。例 3600000。上限は下の規則の 5400000)に十分な値を渡す。コマンドの先頭に `timeout`・`sleep` を書かない(拒否される。ツールの引数の `timeout` は別物)。
- ビルド・測定などの長いコマンドは、背景で起動して、プロセスの終了を待つ(`tail --pid`・`wait`・`while kill -0` など)ことをしない。フォアグラウンドで、直接実行する。シェルツールの `timeout` 引数は、最大 5400000(90 分)にする(0 や、それ以上にしない)。90 分で戻らないコマンドは、固まったとみなし、ログの末尾と、`ps` で、状態を確かめる。
- この作業は、開始から 5 時間以内に終える(ジョブの上限は 5 時間 50 分。時間切れでは、報告が書けない)。開始の時刻を、最初に `date -u` で記録し、**4 時間 30 分を過ぎたら**、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて、ターンを終える。

起動と再試行:
- Issue に `blocked by` の依存が無ければ、作成や更新の時点で起動します。依存の Issue が閉じたときに、塞がれていた Issue が起動します。
- 落ちた Issue には `failed` が付きます。再試行は、Issue に `/retry` とコメントするか、`failed` のラベルを外します。
- 起動しない条件(Issue Type が Task でない・`pm-hold`・`running`・`needs-owner`・sub-issue を持つ・open の依存がある)は、`pm-work/workflow-design.md` の「5」にあります。

## 守ること(依頼者の指示。台帳 `pm-work/ledger/instructions.tsv` にある)
- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない(オプションは作らず、既定で全部有効にする)。
- Python は調査と検証にだけ使い、製品と本番の処理には使わない。
- ベンダー名と、Cloudflare の識別子をファイルに書かない。
- commit の名義を変えない(環境の git の設定のまま)。commit・タグ・PR に Claude の行を付けない。
- 必要の無いブランチを作らない。
- 他のプロジェクトのセッション、portal の道具、依頼者の設定と資格情報に触れない。
- リポジトリの外に、ファイル・ユニット・設定を作らない。一時ファイルはリポジトリの中の `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。このマシンでは行わない。
- GitHub Actions の実行は、依頼者が承認した(台帳 L-027)。作業ジョブの起動は Issues のイベント(`.github/workflows/agent.yml`)だけが行い、`gh workflow run` を手で打たない。`schedule` と `workflow_dispatch` は置かない。`app/`・`lift/`・`ci/` に触れる push は main の `deploy.yml` を動かすので、報告に書く。

## 監査・受付の決まり
- 依頼者の発言のうち、「質問:」で始まる物だけが質問です。質問には、道具を使わずに文章で答えます。
- それ以外の発言は、指示か是正の要求です。台帳(`pm-work/ledger/instructions.tsv`)に書き足し、指された動作を行います。理由や経緯は書きません。
- 状態は、起きるたびに添付されます。状態を確かめるだけのコマンドは打ちません。添付が欠けているときと、異常があるときだけ、道具で調べます。

## GitHub に登録済みの値
- Secrets: `OPENCODE_AUTH_JSON`(opencode の認証)、`R2_CACHE_ASSETS_ACCESS_KEY_ID`・`R2_CACHE_ASSETS_SECRET_ACCESS_KEY`(ビルドのキャッシュ用の R2)、`R2_ARTIFACT_ACCESS_KEY_ID`・`R2_ARTIFACT_SECRET_ACCESS_KEY`(インストーラなどを置く R2)、`CLOUDFLARE_API_TOKEN`(デプロイ用)、`GH_APP_CLIENT_ID`・`GH_APP_PEM`(Issue のラベル・コメント・push に使う App の鍵)
- Variables: `OPENCODE_MODEL`(作業者のモデル)、`CLOUDFLARE_ACCOUNT_ID`、`GH_APP_INSTALLATION_ID`(App の installation)

## 置き場所
- `app/` `lift/` `patches/` `ci/`: 製品
- `tools/`: 製品の検査の道具と、駆動の道具(`select-targets.mjs`: 起動する Issue を選ぶ。`task-accept.mjs`: 本文から受け入れコマンドを取り出す)。判定ではハーネス扱い
- `guard/`: 判定の道具(`judge.sh` が全体の判定。`scan.sh`・`flags.sh`・`deploy-lint.sh` を使う)
- `spec/`: テスト(`spec/guard.test.sh`・`spec/workflow.test.sh`)
- `.github/workflows/`: `agent.yml`(作業フロー)、`e2e.yml`(製品のビルドと E2E)、`deploy.yml`(E2E が通った後のデプロイ)、`e2e-full.yml`(全シナリオ・非ゲート)
- `pm-work/`: PM 資料。計画(`pm-work/plan.md`)、設計(`pm-work/workflow-design.md`)、依頼者の指示の台帳(`pm-work/ledger/instructions.tsv`)
