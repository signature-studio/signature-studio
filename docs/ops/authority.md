# 責任範囲と能力表

規則の要約は `AGENTS.md` §0・§3・§10。この文書は、何を自分でやり、何を Issue で頼むかの根拠。事実の表は `docs/ops/environment.md`。

## 1 責任の線

| 範囲 | 誰が責任を負うか | 失敗したとき |
|---|---|---|
| 依頼を受けてから完成までの全作業(調査・実装・ビルド・測定・commit・push・報告) | セッション(main + worker + reviewer) | 罰則(ワークツリー・リポジトリの削除)も含めて、エージェントの責任 |
| 依頼者にしかできない物(下の §3) | 依頼者(P3 の範囲で協力する) | エージェントの責任にならない。ただし Issue を出さなかった場合、または依頼に依存しない作業を続けなかった場合はエージェントの責任 |

- 「Issue で頼む」と書けるのは、§3 の表の行に当たり、かつ自分で試した記録(コマンドと出力)が、自分では完了できないことを示すときだけ。
- 依頼者が協力するのは、責任範囲外の作業だけ。範囲内の作業を頼む・判断を預ける・操作を頼むことは、協力の対象外。

## 2 エージェントが自分でやること(できるのに「できない」と書いたら違反)

| やること | 道具・コマンド | 実測(日付) |
|---|---|---|
| ファイルの編集・ビルド・測定・commit | worker が実行。PATH は `source scripts/env.sh`(emcc・npx・wasm-opt の版) | 道具は存在(R1、2026-11-09) |
| commit 用の identity の設定 | `scripts/setup-repo.sh`(リポジトリのローカル設定のみ。グローバルは触らない) | 未設定を確認(R1、2026-11-09) |
| Cloudflare の確認・変更 | **実行しない**(`wrangler`・`cf` は読み取りも含め後任の権限外。依頼者の決定 P4)。確認・変更が要るなら Issue(`docs/ops/requests.md`) | — |
| 配信(デプロイ) | GitHub Actions(R8)と、依頼者が Issue に応じて登録する Secrets で行う(P4) | Actions の実行結果(Issue の確認コマンドで確かめる) |
| GitHub の読み取り | `gh repo view`、`gh api <GET>`、`git ls-remote origin` | 成功(R1) |
| push | `git push origin main`(通常の push のみ。scan 通過後。P1)。`git ls-remote origin main` で届いたことを確かめる | 2026-11-10 08:27 JST のリポジトリのディレクトリでの dry-run は rc=0(`To https://github.com/signature-studio/signature-studio.git`、`29966d6..a70e4ef  HEAD -> main`)。[文書・旧] 同日の前の rc=1(PUBLIC の拒否)は前日の誤り(拒否した hook は修正済み) |
| push の可否の確認 | リポジトリのディレクトリで `gh auth status`(トークンの行は除く)と `git push --dry-run origin HEAD` | 同上。`.permissions` は根拠にしない(P1) |
| ブラウザでの確認 | headless Chrome(`google-chrome --headless`)。手元の検証に限り、CI に自作スクリプトを置かない | Chrome 156 は存在。headless の起動は未確認(R1) |
| 追加パッケージ(brotli 等) | まず npm・Python・cargo で足りるか見る。必要なら `sudo -n apt-get install`(必要最小限) | `sudo -n true` が rc=0(R1)。apt の実行は未テスト |
| 長い処理 | shell の `background: true`。完了は通知に任せる | — |
| STATE・ROADMAP の更新 | worker に頼む(main は書けない) | — |

## 3 Issue で頼む物(`docs/ops/requests.md` の形式で出す)

| # | 頼む物 | 頼む理由・対象・影響範囲の書き方 | 依頼なしで続けられる作業 |
|---|---|---|---|
| 1 | Secrets・トークンの発行と登録(Cloudflare の API トークン、R2 の S3 キー。GitHub Actions で使う) | 理由: トークンの発行と Secrets の登録は依頼者の権限(P3)。対象: Secret の名前(値は書かない)と用途。影響: Secret の追加のみ。取り消し: Secret の削除 | commit・ビルド・測定。配信は Actions の結果を待たず、この機械の headless Chrome の実測で進める |
| 2 | Access・DNS・custom domain の変更 | 理由: 後任は Cloudflare を実行しない(P4)。対象: `<ホスト名>` のプレースホルダ。影響: 公開の経路 | 旧 Worker 名・旧ホストのまま、ローカルの実測で進める。新ホストが要る作業だけ止まる |
| 3 | 実機・依頼者機の計測(依頼者機のブラウザ、実機 Windows の画面・音・120 fps) | 理由: headless では代替できない(P5)。手順: 読み取りのみ。返す値の形式。影響: なし | headless の実測で進める。依頼者機の値は「依頼中」と書く(Issue を出した後) |
| 4 | org の権限(`.github/workflows/` を push するのに要る権限が足りないとき) | 理由: org の権限は依頼者の権限。対象: 足りない権限の名前。影響: 権限の付与のみ | workflow 以外の commit と push。`.github/workflows/` の変更は、権限が付いた後に push する |

- push 権限と公開範囲の行は、この表に無い(解消・意図的。P1・P2)。
- Issue の必須項目(対象・理由・影響範囲・自分で試した記録・確認コマンド・依頼なしで続ける作業)は `docs/ops/requests.md`。表に無い物を頼むときは、先に `docs/ops/environment.md` の確認コマンドを実行し、結果を Issue に貼る。
- 状態が変わったら(例: 権限が付いた)、確認コマンドの出力を根拠に、Issue の末尾に確認の記録を書く。

## 4 依頼者の権限に属する物(エージェントは実行しない)

| 物 | 扱い |
|---|---|
| リポジトリの作成・削除・公開範囲の変更、org の権限、bot の installation、Secrets の登録 | 実行しない(依頼者の権限)。Issue で頼む(§3)。依頼に依存しない作業を続ける |
| 履歴の書き換え(filter-repo・force push・`gh api` での ref の更新) | 実行しない。依頼者が指示しているなら `docs/ops/dangerous-ops.md` の手順 |
| 認証情報の取り出し・表示(`opencode auth export` を含む)、別 identity への切替、依頼者の設定ファイルの編集 | 実行しない |
| 依頼者の機械・依頼者のブラウザでの操作 | 求めない(計測だけ §3 の 3 で Issue を出す) |

「実行しない」と「聞いて止まる」は別。実行しない物があっても、同じターンに、その物に依存しない作業の worker を起動する。

## 5 「できない」と書く前の確認(3 点セット)

| きっかけ | やること | 確かめ方 |
|---|---|---|
| 「できない」「権限がない」「見えない」と書きかけた | 該当する §2 の道具を実行する。権限なら `scripts/preflight.sh` を実行 | 出力(エラー全文の先頭 5 行)を報告に貼る |
| 404・403・空の出力・フックの拒否が返った | 原因を切り分ける: identity(`gh auth status`)、作業ディレクトリ(リポジトリの中か)、リポジトリの公開範囲、フックの出力。404 を「権限の喪失」と書かない。`.permissions` は根拠にしない | 3 つの出力を並べる |
| Issue が要ると実測した | Issue を出す(`docs/requests/TEMPLATE.md`。必須項目を満たしてから) | `ls docs/requests` に Issue がある |

## 6 論理的に破綻した要求(禁止。指示違反として扱う)

| 要求の型 | なぜ破綻か | 代わりにやること |
|---|---|---|
| 「GitHub に push できない」(`--dry-run` が通るのに) | 権限が実測であるときを除き、事実に反する | リポジトリのディレクトリで `gh auth status` と `git push --dry-run origin HEAD` の出力を貼る。通るなら `git push origin main` |
| 「Cloudflare を確認してください」 | 後任は wrangler を実行しない(P4)。依頼者に確認の操作を頼むのは、依頼者の操作の要求 | 確認が要るなら Issue を出す(`docs/ops/requests.md`) |
| 「〜を実行してください」「ブラウザで開いて確認してください」 | 依頼者は操作を引き受けない | `scripts/`・gh・git・headless Chrome で確かめる。実機の画面・音は Issue(§3 の 3) |
| 「判断してください」「どちらにしますか」 | 判断は main が行う(依頼者は一切の判断を行わない) | 決定と根拠を 1 行書いて実行 |
| 「作業対象がありません」 | 旧ワークツリー・docs・ローカルのワークツリーは読める | 読める物から worker を起動 |
| 「許可をください」(commit・push・設定の変更) | 許可済みの操作 | 実行して事実を 1 行書く |
