# Issue 0007: Actions の提供元の指定を Variable OPENCODE_PROVIDER に登録
## 依頼 ID・日付
0007 / 2026-11-16

## 対象
このリポジトリ(signature-studio)の GitHub の Variables(Actions 用)に、Variable `OPENCODE_PROVIDER` を 1 件登録する。値は、依頼者のマシンのリポジトリのフォルダで次の 1 行のコマンドを実行して出た、提供元の名前 1 語(モデルの名前は含まない。認証情報ではない)。
依頼者にしてほしい操作: 次の 1 つだけ。登録の手順は 2 段階で、依頼者の 1 つの操作として行う。
1. 依頼者のマシンで、このリポジトリのフォルダにおいて、次の 1 行のコマンドを実行する。出力は提供元の名前 1 語。
```
f=.opencode/opencode.jsonc; [ -f "$f" ] || f=opencode.jsonc; grep -A4 '^ *"worker": *{' "$f" | grep -m1 -o '"model" *: *"[^"]*"' | sed -E 's/.*"([^"]*)"$/\1/; s/#.*//; s#/.*##'
```
2. GitHub で、このリポジトリの Settings を開き、Secrets and variables の Actions で Variables のタブを開く。New repository variable を押し、Name に `OPENCODE_PROVIDER`、Value に 1 の出力をそのまま貼って保存する。

## 合理的理由
Variable は GitHub が伏せない。旧の Variable `OPENCODE_MODEL`(モデルの名前を含む値)は、run 43804191981 のログに 1 行、そのまま出た(依頼者の公開ログに出る。evidence/R9-3.md)。提供元の名前だけにすれば、モデル ID の文字列(ベンダー名を含む)は Variable に入らず、モデルの選択はワークフローの中で行う。Variables の登録は、リポジトリの管理者の操作で、エージェントは Variables を一覧・読み書きしない(依頼者の権限)。
注意: 提供元の名前 1 語は、登録の後も、その段のログに env の表示として出る(GitHub の仕様)。ベンダー名は出ない。提供元の名前を公開ログに出すかどうかは、この依頼の範囲の外で、main が決める(決定 0388 の未確認に記載)。

## 影響範囲
- 変わる物: Variable `OPENCODE_PROVIDER` が 1 件増える。値は提供元の名前 1 語。登録の後、agent.yml の次の run が、この値の提供元からモデルを 1 件選ぶ。
- 変わらない物: Secrets、Cloudflare の設定、ワークフローの他の手順。
- 旧の Variable `OPENCODE_MODEL` は、参照されなくなる。削除は任意(削除しなくても動作に影響しない)。
- 取り消し方: Variable `OPENCODE_PROVIDER` を削除する。削除すると、次の run は「提供元の指定(Variable OPENCODE_PROVIDER)が空」と出して止まる(Issue 0007 を参照)。

## 自分で試した記録
エージェントは、1 の出力の形だけを確かめた。出力の中身は、この記録にもリポジトリにも書かない。Variables を一覧・読み書きするコマンドは実行していない。
```
(上の 1 行のコマンド)| grep -Ec '^[^/#[:space:]]+$'   → 1(この機械。出力は 1 行)
opencode --help の抜粋: --standalone            Run with a private server instead of the background service
```

## 完了の確認コマンド
- 依頼者からの、登録が済んだ旨の連絡(main が受け取る)。
- 登録の後、次の run の worker ジョブの結論が success になること:
```
gh run list --workflow agent.yml --limit 1 --json databaseId --jq '.[0].databaseId' | xargs -I{} gh run view {} --json jobs --jq '[.jobs[] | select(.name | startswith("worker")) | .conclusion] | join(",")'   → success
```
- 同じ run のログで、ベンダー名の語の件数が 0(`gh run view <id> --log | grep -ci <語>` を、語ごとに数える)。提供元の名前の件数は別に数えて記録する。

## 依頼なしで続ける作業
- 登録の無い間も、`tests/harness/t11_agent_model.sh` で、設定の組み立てと検査(空・候補の件数・値の形・5 つの agent の一致)を確かめ続ける。
- 登録の後、次のタスクを含む push の run で、取り込みの後に選択と検査が通ることを確かめ、結果を下の「確認の記録」に追記する。

## 確認の記録
(登録の後に追記: 日付・完了の確認コマンドの出力 1 行)

## 起票
- `gh issue create` の出力: (起票の直後に追記)
