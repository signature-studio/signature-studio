# AI の利用先を切り替える

AI の利用先は 2 つあります。請求の状況に合わせて、依頼者がどちらかを選びます。
- **Cloudflare の AI ゲートウェイ経由**(初期の設定。`opencode.jsonc` にその値が入っている)
- **Amazon Bedrock**

リポジトリのフォルダで、次のように実行します。
- Cloudflare に戻す: `scripts/set-model-profile.sh gateway [セッションID]`
- Bedrock に切り替える: `scripts/set-model-profile.sh bedrock [セッションID]`

- `gateway` は、この機械だけの上書きの設定(Git に入らないファイル)を消します。`bedrock` は、決まった値をその上書きの設定に書きます。どちらも、続けて `opencode reload` を実行して、動いているサーバーに設定を読ませます。
- Bedrock の設定は、variant(`#high` など)を付けない固定値です(付けると HTTP 400 で作業用のサブエージェントが起動しません)。メインと build は Sonnet 5.5、作業用・検証用・表題・要約は Haiku 5.5 です。全 Haiku の構成は、もうありません。
- Bedrock のモデル名には、依頼者が禁止した語が含まれるため、リポジトリのファイルには書きません。切り替えるときにスクリプトが組み立てて、Git に入らない上書きの設定に書きます。
- `scripts/launch.sh`(メインセッションの作成)と `scripts/audit-session.sh`(固定外の検査)は、その上書きの設定があれば、そのモデルを読みます。
- 動いているセッションのモデルは、第 2 引数のセッション ID を付けたときだけ切り替わります(`opencode api session.switchModel`)。付けないと、そのセッションは次の操作で止まります(安全のための動きです)。モデルを選び直すか、ID を付けて実行し直してください。

## GitHub Actions(作業用エージェント)のモデル

Actions の作業用エージェントのモデルは、このリポジトリの Variable `OPENCODE_PROVIDER`(提供元の名前 1 語)で決まります(決定 0388)。モデル ID は、その提供元の一覧から `runner-config.sh` が選びます。
- 値は提供元の名前 1 語だけです。モデル ID は書きません。選ばれるのは、その提供元の作業用の型(末尾 `claude-haiku-5-5`)の 1 件だけです。
- 切り替えは、GitHub のこのリポジトリの Variables で `OPENCODE_PROVIDER` の値を書き換えます。次の run から切り替わります。
- 値を空にする、または候補が 1 件でないと、run は「Issue 0007 を参照」と出して止まります。既定の値はありません。
- 提供元の名前は、その段の env の表示としてログに 1 行出ます(決定 0388 の追記)。値が空の run では、空の値の行になります。
- `scripts/set-model-profile.sh` の切り替えは、この機械のローカルのセッションだけに効きます。Actions の値とは別に決まります。
