# Issue 0006: Actions の作業用エージェントのモデルを指定する Variable の登録
状態: 置き換え済み(決定 0388)。Variable `OPENCODE_MODEL` は参照しない。Issue 0007 で、提供元の名前だけの Variable `OPENCODE_PROVIDER` に替える。
## 依頼 ID・日付
0006 / 2026-11-16

## 対象
このリポジトリ(signature-studio)の GitHub の Variables(Actions 用)に、Variable `OPENCODE_MODEL` を 1 件登録する。値は、依頼者のマシンで次の 1 行のコマンドを実行して出た文字列(モデルの名前だけ。認証情報ではない)。
依頼者にしてほしい操作: 次の 1 つだけ。登録の手順は 2 段階で、依頼者の 1 つの操作として行う。
1. 依頼者のマシンで、このリポジトリのフォルダにおいて、次の 1 行のコマンドを実行する。出力は 1 行の文字列。
```
f=.opencode/opencode.jsonc; [ -f "$f" ] || f=opencode.jsonc; grep -A4 '^ *"worker": *{' "$f" | grep -m1 -o '"model" *: *"[^"]*"' | sed -E 's/.*"([^"]*)"$/\1/; s/#.*//'
```
2. GitHub で、このリポジトリの Settings を開き、Secrets and variables の Actions で Variables のタブを開く。New repository variable を押し、Name に `OPENCODE_MODEL`、Value に 1 の出力をそのまま貼って保存する。

## 合理的理由
Actions の作業用エージェントのモデルは、依頼者が切り替えられる必要がある(依頼者の指摘。発言 `msg_1e2274299001Wt4h6i62ve8mbb`)。コミットを書き換えずに切り替えられるのは、GitHub の Variable だけ。Variable の登録は、リポジトリの管理者の操作で、エージェントは Variables を一覧・読み書きしない(依頼者の権限)。

## 影響範囲
- 変わる物: Variable `OPENCODE_MODEL` が 1 件増える。値は、モデルの名前だけ。(この記録の時点の想定。誤り: GitHub は Variables を伏せず、値は実行のログに出た。evidence/R9-3.md。)
- 変わらない物: Secrets、Cloudflare の設定、自動処理の他の手順。
- 取り消し方: Variable `OPENCODE_MODEL` を削除する。削除すると、次の実行は取り込みの後で「Issue 0006 を参照」と出して止まる。値を書き換えると、次の実行から切り替わる。

## 自分で試した記録
エージェントは、1 の出力の形だけを確かめた。出力の中身は、この記録にもリポジトリにも書かない。Variables を一覧・読み書きするコマンドは実行していない。
```
(上の 1 行のコマンド)| grep -Eq '^[^/#[:space:]]+/[^/#[:space:]]+$' && echo OK || echo NG
OK
(同じ 1 行のコマンドを、opencode.jsonc だけがあるフォルダで実行。.opencode/ は無い)| grep -Eq '^[^/#[:space:]]+/[^/#[:space:]]+$' && echo OK || echo NG
OK
grep -c 'vars.OPENCODE_MODEL' .github/workflows/agent.yml
2
```

## 完了の確認コマンド
```
gh run list --workflow agent.yml --limit 1 --json databaseId --jq '.[0].databaseId' | xargs -I{} gh run view {} --json jobs --jq '[.jobs[] | select(.name | startswith("worker")) | .conclusion] | join(",")'   → success
```
依頼者からの、登録が済んだ旨の連絡も、確認の一つとする。

## 依頼なしで続ける作業
- 登録の無い間も、`tests/harness/t11_agent_model.sh` で、設定の組み立てと検査(Variable が空・値の形・5 つの agent の一致)を確かめ続ける。
- 登録の後、タスクを含む push の実行で、取り込みの後に検査が通ることを確かめ、結果を下の「確認の記録」に追記する。

## 確認の記録
(登録の後に追記: 日付・完了の確認コマンドの出力 1 行)
- 2026-11-16: 完了の確認コマンドの出力: `success`(実行 43804191981、コミット 1aff9bb。「設定を確かめる」手順も success。詳細は evidence/R9-1.md)。注: 同じ実行のログに、Variable の値が 1 行出ている(GitHub は Variables を自動で伏せないため。evidence/R9-1.md の 4 節。修正の試行は効かず、戻した。判断は未決 → 決定 0388 で、Variable `OPENCODE_PROVIDER`(Issue 0007)に替えた)

## 起票
- `gh issue create` の出力: https://github.com/signature-studio/signature-studio/issues/6
