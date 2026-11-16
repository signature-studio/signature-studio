# Issue 0005: Actions でエージェントを動かすための console のログイン情報の登録
## 依頼 ID・日付
0005 / 2026-11-16

## 対象
このリポジトリ(signature-studio)の GitHub の Secrets(Actions 用)に、Secret `OPENCODE_AUTH_JSON` を 1 件登録する。中身は、依頼者のマシンの opencode のログイン情報(opencode の console に登録した BYOK)だけ。
依頼者にしてほしい操作は次の 2 つだけ。この順番で行う。

1. 依頼者のマシンで、`opencode auth export opencode` を実行する。対象の `opencode` を省いて実行しない(省くと、すべての提供元のログイン情報が出力されるため)。出力された JSON を、全部コピーする。
2. GitHub で、このリポジトリの Settings を開き、Secrets and variables の Actions で New repository secret を開く。Name に `OPENCODE_AUTH_JSON` と入力し、Secret の欄に 1 でコピーした JSON を貼って保存する。

値は、この依頼・コミット・チャット・ログのどれにも書かない。

## 合理的理由
Actions のランナーには、依頼者の opencode のログインが無い。依頼者の console に登録した BYOK を Actions から使うには、依頼者の opencode のログイン情報を、依頼者が GitHub の Secret に登録する必要がある。エージェントは、このログイン情報を読まず、Secret を登録もできない(依頼者の権限)。登録が無いと、`ci/tasks/*.md` を push して動く作業用エージェントは 1 回も動かない。

## 影響範囲
- 変わる物: Secret が 1 件増える(`OPENCODE_AUTH_JSON`)。ワークフローは、この Secret を、run ジョブの 1 つの手順(`opencode auth import` に標準入力で渡す)でだけ読む。
- 変わらない物: Cloudflare の Secrets、他の Secrets・Variables、他のワークフローには渡さない。
- 料金: `ci/tasks/*.md` を push するたびに、依頼者の console の BYOK の呼び出しの料金が発生する。1 push の上限は 60 件、1 件の上限は 350 分。
- 取り消し方: GitHub の Secret `OPENCODE_AUTH_JSON` を削除する。必要なら、opencode の console で、このログイン情報を無効にする。

## 自分で試した記録
エージェントは `opencode auth export` と `opencode auth import` を実行しない(`AGENTS.md` §8)。代わりに、次を確かめた(2026-11-16)。
- `opencode auth export --help` の表示を試したが、エージェントの実行の許可が無く、表示されなかった(取り込みの `--help` は試していない)。
- opencode のソース(ローカルのクローン。版 ef45f1fbe0、2026-11-08 のコミット)の説明文。`commands.ts` の定義:
  - export: 「print stored credentials, including secrets, as JSON」。対象は「Integration ID or name (exports every integration when omitted)」。
  - import: 「import credentials exported by auth export」。ファイルは「JSON file to import (reads stdin when omitted)」。
- `handlers/auth/export.ts`: 標準出力に JSON の配列を出す。`handlers/auth/import.ts`: 標準入力の JSON を読み、取り込みの件数を標準エラー出力に `Imported N credential(s)` と出す(値は出さない)。
- 未確認: インストールした版(2.2.6)で、同じ仕様かどうか。対象 `opencode` が依頼者の機械の一覧に ID として有るかどうか。

## 完了の確認
1. 依頼者から、Secret `OPENCODE_AUTH_JSON` を登録した旨の連絡がある。
2. 登録の後、エージェントが小さなタスク 1 件を push して、次のコマンドの結論が `success` になる。
```
gh run list --workflow agent.yml --limit 1 --json conclusion --jq '.[0].conclusion'
```

## 依頼なしで続ける作業
- 登録が無い間は、`ci/tasks/*.md` を置かない(run を動かさない)。
- ワークフローの組み立ての検査(`ci/agents/check-agents.sh` など)を、登録が無くても続けて確かめる。
- Workers の配信(`deploy-engine` のワークフロー)の作業を続ける。

## 確認の記録
(登録の後に追記: 日付・`gh run list --workflow agent.yml --limit 1` の結論の 1 行)
