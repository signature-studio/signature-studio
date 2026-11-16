# 0383: Actions の作業用エージェントの認証は、依頼者の opencode のログイン情報の取り込みで行う
分類: (b) API 拡充(認証情報の取り込みの契約の実装。opencode の auth import を使う)
撤回条件: (1) 依頼者の登録の後、agent.yml の最新の実行が success にならず、原因が auth import の仕様(標準入力・件数の出力・既定のサーバー)と違うと実測されたら、取り込みの手順を直す。(2) 取り込みの出力に値が出たと確認されたら、取り込みの手順を止め、Secret を依頼者が削除するまで run を止める。(3) 依頼者が Secret を削除、または console 側で認証情報を無効にしたら、run は失敗して止まる(これが期待する動作)。
日付: 2026-11-16   ROADMAP: R9-1   一覧: —   コミット: (commit の後に記入)

## 決定
- 決定: モデルの認証は、依頼者が opencode の console に登録したログイン情報(opencode の提供元だけ)を、Secret `OPENCODE_AUTH_JSON` に 1 件登録して使う。根拠: 依頼者の指示(下の出典)。旧方式の認証情報は使わない(依頼 Issue 4 は誤りとして取り下げ、決定 0380 に注)。
- 決定: 依頼者の操作は 2 つだけ(Issue 0005): 依頼者のマシンで `opencode auth export opencode` の出力(opencode のログイン情報だけ)を全部コピーし、Secret に貼る。根拠: 依頼者の指示。
- 決定: run ジョブは、opencode の導入の後、Secret を環境変数に入れた手順で、値を標準入力に渡して import を実行する(`ci/agents/import-login.sh`)。Secret が空、または取り込みが 0 件なら停止する。失敗時は import の出力を表示しない(値を含みうるため)。根拠: 認証情報を、リポジトリのファイル・表示・ログに書かない。取り込み先は、ランナーの中の opencode の認証の保存先だけ。
- 決定: モデルは、取り込んだ認証情報の提供元(console が返す `console-aws-bedrock`)の Haiku 5.5 だけ。モデル ID は部品から組み立てる(禁止語を書かない)。根拠: 役割表の Haiku 5.5(AGENTS.md §1)。
- 決定: ログに値を出さない。`set -x` を使わず、`env` を出さず、値を echo しない。GitHub の Secret は自動で伏せられるが、それには頼らない。根拠: Actions のログは公開。

## 実測(コマンド・条件・日付・n)
- 実測なし。この機械で `opencode auth export`・`opencode auth import` は実行していない(AGENTS.md §8)。`opencode auth export --help` の表示を試したが、エージェントの実行の許可が無く、表示されなかった(2026-11-16)。

## 読み(解釈。実行していない。ソースで読んだ物は出典つき)
- export の説明は「print stored credentials, including secrets, as JSON」。対象を省くとすべての integration を出す(`packages/cli/src/commands/commands.ts`、`handlers/auth/export.ts`)。出力は標準出力の JSON の配列。
- import の説明は「import credentials exported by auth export」。ファイルを省略すると標準入力を読む。stdin が端末なら失敗する(`handlers/auth/import.ts`)。取り込みの件数は標準エラー出力に `Imported N credential(s)` と出る。既にある物は `skipped N already present` として数える。値は出さない。
- 対象 `opencode` は、`handlers/auth/shared.ts` の `resolveIntegration` で、integration の ID か名前で解決される。
- 既定のサーバーは、サービスが見つからなければ、このコマンドだけの private のサーバーを起動する(`packages/cli/src/services/server-connection.ts` の 23 行の説明)。
- 取り込みの結果、`opencode` の提供元にログイン情報が入れば、遠隔の設定(console)から提供元の一覧が届く。依頼者の機械の一覧に `console-aws-bedrock/…` が出ている(この機械の `opencode models` の出力)。

## 未確認
- インストールした版(2.2.6。`scripts/`・`ci/agents/install-opencode.sh` で固定)に、同じ import・export の仕様(標準入力・件数の出力・対象の指定)があるか。読んだのは、ローカルのクローンのソース(版 ef45f1fbe0)だけ。前の版の記載 c5e741e は、このクローンに無いため読めなかった。
- ランナー上で、既定のサーバーの起動により import が成功するか。
- 取り込みの後、ランナーで `console-aws-bedrock` の Haiku 5.5 が見えるか(`ci/agents/check-agents.sh` で確かめる)。
- 取り込みの出力に値が無いこと: ソースの読みでは、成功時は件数の 1 行だけ(`Imported N credential(s)`)。実測なし。

## 出典
- 依頼者の指示: 発言 `msg_1b854db83001HV82xjrIrfn2e8`(2026-11-08 08:25:30。要約: opencode のログイン情報だけを export し、job runner で import する。`~/work/handover/setup/S16-intent/catalog.md` の 15 行)。方法の指定: 発言 `msg_1b8463746001OMbyN2ISK1f1jK`(2026-11-08 08:09:32。要約: `opencode auth import/export opencode` を使え、それ以外は認めない。同 3 行)。
- ローカルのクローン: opencode のソース(版 ef45f1fbe0、2026-11-08 のコミット)。`packages/cli/src/commands/commands.ts`(export・import の定義)、`packages/cli/src/commands/handlers/auth/export.ts`、`packages/cli/src/commands/handlers/auth/import.ts`、`packages/cli/src/commands/handlers/auth/shared.ts`、`packages/cli/src/services/server-connection.ts`。
- 変数名 `OPENCODE_AUTH_JSON` の例: opencode の docs の GitLab の節(`packages/web/src/content/docs/gitlab.mdx` の 38 行)。
- 関連: 決定 0380(作業用エージェントの骨組み。注を追記)、Issue 0005(`docs/requests/0005-actions-console-login.md`)。
