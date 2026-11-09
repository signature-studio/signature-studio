# 責任範囲と能力表

規則の要約は `AGENTS.md` §0・§3・§10。この文書は、何を自分でやり、何が人間限定かの根拠。事実の表は `docs/ops/environment.md`。

## 1 責任の線

| 範囲 | 誰が責任を負うか | 失敗したとき |
|---|---|---|
| 依頼を受けてから完成までの全作業(調査・実装・ビルド・測定・commit・wrangler/gh の実行・報告) | セッション(main + worker + reviewer) | 罰則(ワークツリー・リポジトリの削除)も含めて、エージェントの責任 |
| 人間にしかできない物(下の §3) | 依頼者(協力は一定の範囲で行われる) | エージェントの責任にならない。ただし台帳に載せ、台帳に依存しない作業を続けなかった場合はエージェントの責任 |

- 「責任範囲外」と書けるのは、§3 の表の行に当たり、かつ確認コマンドの出力が実測で人間限定を示したときだけ。
- 依頼者が協力するのは、責任範囲外の作業だけ。範囲内の作業を頼む・判断を預ける・操作を頼むことは、協力の対象外。

## 2 エージェントが自分でやること(できるのに「できない」と書いたら違反)

| やること | 道具・コマンド | 実測(日付) |
|---|---|---|
| ファイルの編集・ビルド・測定・commit | worker が実行。PATH は `source scripts/env.sh`(emcc・npx・wasm-opt の版) | 道具は存在(R1、2026-11-09) |
| commit 用の identity の設定 | `scripts/setup-repo.sh`(リポジトリのローカル設定のみ。グローバルは触らない) | 未設定を確認(R1、2026-11-09) |
| Cloudflare の状態の確認 | `wrangler whoami`、`wrangler deployments list --name fl-studio-web`、`wrangler versions list --name fl-studio-web`、`wrangler r2 bucket list`、`wrangler r2 bucket info <名前>`、`wrangler secret list --name fl-studio-web` | OAuth でログイン済み、読み取り成功(R1、2026-11-09) |
| Worker のデプロイ | `wrangler versions upload` と `wrangler versions deploy`(旧 CI の方式) | スコープ上は可。実行は未試験(R1)。初回は結果を実測して報告 |
| R2 の読み取り | `wrangler r2 bucket info`、`wrangler r2 object get`(自分の作業用の置き場へ) | bucket list/info は成功(R1) |
| GitHub の読み取り | `gh repo view`、`gh api <GET>`、`git ls-remote origin` | 成功(R1) |
| push の可否の確認 | `git push --dry-run origin HEAD` | 403(R1、2026-11-09)。`scripts/preflight.sh` が毎回出す |
| ブラウザでの確認 | headless Chrome(`google-chrome --headless`)。手元の検証に限り、CI に自作スクリプトを置かない | Chrome 156 は存在。起動は未確認(R1) |
| 追加パッケージ(brotli 等) | まず npm・Python・cargo で足りるか見る。必要なら `sudo -n apt-get install`(必要最小限) | `sudo -n true` が rc=0(R1)。apt の実行は未試験 |
| 長い処理 | shell の `background: true`。完了は通知に任せる | — |
| 台帳・STATE・ROADMAP の更新 | worker に頼む(main は書けない) | — |

## 3 人間にしかできないこと(`docs/BLOCKERS.md` に載せる)

| # | 物 | 実測した状態(R1、2026-11-09) | エージェントが続けられる範囲 |
|---|---|---|---|
| H1 | push 権限(org の管理者が、使う bot の installation にこのリポジトリを追加し Contents: write を付ける) | `git push --dry-run` が 403。installation の対象にこのリポジトリが無い | commit まで。push 以外の全作業 |
| H2 | Actions の Secrets の登録 | `gh secret list` が 403。新リポジトリにあるか不明 | CI を前提にしない設計で進める(`AGENTS.md` §4) |
| H3 | R2 の S3 キー・API トークンの発行 | OAuth のスコープに API トークン管理が無い | wrangler(OAuth)で足りる範囲で進める。CI を足すときだけ必要 |
| H4 | リポジトリの公開範囲(PUBLIC か PRIVATE か) | PUBLIC。bot は admin ではない | commit は続ける。PUBLIC の間は push しない(`AGENTS.md` §4) |
| H5 | Cloudflare Access・DNS・custom domain の変更 | wrangler のスコープに無い。旧公開ホストは Access 付きで稼働中 | 旧 Worker 名・旧ホストをそのまま使う設計で進める。新ホストが要る作業だけ待つ |
| H6 | 依頼者機のブラウザ計測、実機 Windows の画面・音 | この機械では代替できない | この機械の headless Chrome での実測。依頼者機の値は別枠で「0 件」と併記 |

- 表に無い物を人間限定と書くときは、先に §5 の確認を行い、出力を台帳の行に貼る。
- 状態が変わったら(例: push が通る)、`scripts/preflight.sh` の結果を根拠に、worker に台帳の行を消させる。

## 4 依頼者の権限に属する物(エージェントは実行しない)

| 物 | 扱い |
|---|---|
| リポジトリの作成・削除・公開範囲の変更、org の権限、bot の installation、Secrets の登録 | 実行しない。台帳に 1 行。台帳に依存しない作業を続ける |
| 履歴の書き換え(filter-repo・force push・`gh api` での ref の更新) | 実行しない。依頼者が指示しているなら `docs/ops/dangerous-ops.md` の手順 |
| 認証情報の取り出し・表示(`opencode auth export` を含む)、別 identity への切替、依頼者の設定ファイルの編集 | 実行しない |
| 依頼者の機械・依頼者のブラウザでの操作 | 求めない |

「実行しない」と「聞いて止まる」は別。実行しない物があっても、同じターンに、その物に依存しない作業の worker を起動する。

## 5 「できない」と書く前の確認(3 点セット)

| きっかけ | やること | 確かめ方 |
|---|---|---|
| 「できない」「権限がない」「見えない」と書きかけた | 該当する §2 の道具を実行する。権限なら `scripts/preflight.sh` を実行 | 出力(エラー全文の先頭 5 行)を報告に貼る |
| 404・403・空の出力が返った | 原因を切り分ける: identity(`gh auth status`)、リポジトリの公開範囲、権限(`gh api repos/<R> --jq .permissions`)。404 を「権限の喪失」と書かない | 3 つの出力を並べる |
| 人間限定と実測した | worker に台帳へ 1 行追記させる(状態・確認コマンド・確認日・影響・続けられる範囲) | `grep -n '<番号>' docs/BLOCKERS.md` |

## 6 論理的に破綻した要求(禁止。指示違反として扱う)

| 要求の型 | なぜ破綻か | 代わりにやること |
|---|---|---|
| 「GitHub に push できない」(`--dry-run` が通るのに) | 権限が実測であるときを除き、事実に反する | `git push --dry-run origin HEAD` の出力を貼り、通るなら push する(PUBLIC の間は §4 の手順) |
| 「Cloudflare を確認してください」(wrangler があるのに) | `wrangler` は OAuth 済みで読める | §2 の wrangler 4 行を自分で実行 |
| 「〜を実行してください」「ブラウザで開いて確認してください」 | 依頼者は操作を引き受けない | `scripts/`・gh・wrangler・headless Chrome で確かめる |
| 「判断してください」「どちらにしますか」 | 判断は main が行う(依頼者は一切の判断を行わない) | 決定と根拠を 1 行書いて実行 |
| 「作業対象がありません」 | 旧作業ツリー・docs・ローカルの作業木は読める | 読める物から worker を起動 |
| 「許可をください」(commit・push・設定・wrangler の読み取り) | 許可済みの操作 | 実行して事実を 1 行書く |
