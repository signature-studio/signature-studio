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
Variable は GitHub が伏せない。旧の Variable `OPENCODE_MODEL`(モデルの名前を含む値)は、実行 43804191981 のログに 1 行、そのまま出た(公開ログに出る。根拠は、下の「自分で試した記録」に書く)。提供元の名前だけにすれば、モデル ID の文字列(ベンダー名を含む)は Variable に入らず、モデルの選択は自動処理の中で行う。Variables の登録は、リポジトリの管理者の操作で、エージェントは Variables を一覧・読み書きしない(依頼者の権限)。
提供元の名前 1 語は、登録の後も、公開ログに表示される。ベンダー名・認証情報は含まない。

## 影響範囲
- 変わる物: Variable `OPENCODE_PROVIDER` が 1 件増える。値は提供元の名前 1 語。登録の後、自動処理(agent.yml)の次の実行が、この値の提供元からモデルを 1 件選ぶ。
- 変わらない物: Secrets、Cloudflare の設定、自動処理の他の手順。
- 旧の Variable `OPENCODE_MODEL` は、参照されなくなる。削除は任意(削除しなくても動作に影響しない)。
- 取り消し方: Variable `OPENCODE_PROVIDER` を削除する。削除すると、次の実行は「提供元の指定(Variable OPENCODE_PROVIDER)が空」と出して止まる(Issue 0007 を参照)。

## 自分で試した記録
エージェントは、1 の出力の形だけを確かめた。出力の中身は、この記録にもリポジトリにも書かない。Variables を一覧・読み書きするコマンドは実行していない。
```
(上の 1 行のコマンド)| grep -Ec '^[^/#[:space:]]+$'   → 1(作業したマシンでの結果。出力は 1 行)
opencode models --help の抜粋: List all available models
opencode models --help の抜粋: opencode models [flags]
```
上の help の抜粋は、利用できるモデルの一覧を出す道具(opencode models)の説明。この依頼の登録では使わない。
根拠の記録: evidence/R9-3.md(旧の Variable の値が、実行のログに 1 行出た件。値の中身は書かない)。

## 完了の確認コマンド
- 依頼者からの、登録が済んだ旨の連絡。
- 登録の後、次の実行の作業(作業用エージェント)の結論が success になること:
```
gh run list --workflow agent.yml --limit 1 --json databaseId --jq '.[0].databaseId' | xargs -I{} gh run view {} --json jobs --jq '[.jobs[] | select(.name | startswith("worker")) | .conclusion] | join(",")'   → success
```
- 同じ実行のログで、ベンダー名の語の件数が 0 であること。語ごとに、次のコマンドで数える。提供元の名前の件数は別に数えて記録する。
```
gh run view <id> --log | grep -ci <語>
```

## 依頼なしで続ける作業
- 登録の無い間も、`tests/harness/t11_agent_model.sh` で、設定の組み立てと検査(空・候補の件数・値の形・5 つのエージェントの設定の一致)を確かめ続ける。
- 登録の後、次の作業の変更を GitHub に送った(push した)後の実行で、取り込みの後に選択と検査が通ることを確かめ、結果を下の「確認の記録」に追記する。

## 確認の記録
(登録の後に追記: 日付・完了の確認コマンドの出力 1 行)

## 起票
- 起票: https://github.com/signature-studio/signature-studio/issues/7
