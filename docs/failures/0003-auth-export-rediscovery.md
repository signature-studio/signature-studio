# 0003: 依頼者が前に指定した認証情報の方法を探さず、調査を後任に頼んだ

- 日付: 2026-11-16(01:30〜01:55Z)
- 誰が: メインセッション
- 依頼者の発言: 「なぜ『opencode auth export opencode コマンドを叩いて、その内容をSecretsに登録してください』の1行がいつまで経っても聞こえてこないのですか」(ID は未取得)。続けて、「OPENCODE_AUTH_CONTENT の件は既に解決済み。解決済みの課題に対して重複した問題を提起し、解決に向けて動こうともしない」(ID は未取得)。

## 何が起きたか
- Issue 4 の取り下げの後、私は「BYOK なしで動かす方法を調べる」と後任に指示した(msg_1e1f60ac900141z4ABtXjuCBoU の周辺)。
- 依頼者に指摘された後、`opencode auth export --help` を初めて実行した。そこで見つけた環境変数 `OPENCODE_AUTH_CONTENT` を、「実装は未確認。実測せよ」として、後任に調査を頼んだ(msg_1e1fb742d001paqfnVoVYGXO4D)。
- 依頼者の 2026-11-08 の指示(msg_1b854db83001HV82xjrIrfn2e8: opencode auth export opencode し、実行環境で opencode auth import しろ)は、私が作った ~/work/handover/setup/S16-intent/catalog.md の 3・14・15 行に要約があった。読んでいなかった。

## 原因(その時の判断)
- 自分の道具の `--help` も、自分が作った依頼者の発言の記録も読まず、公開ソースの検索で答えを探した。「記録に答えがあるはず」と考えなかった。
- 「grep で見つからない」を「未確認」と書けば足りると考え、確認の作業を後任に回した。

## 影響
- 依頼者が、解決済みの内容を再び説明する手間。後任への重複した調査の指示。

## 是正
- 後任に、方法の訂正を送った(msg_1e1fe0210001Tp2DLvImh2D0GZ): Secret `OPENCODE_AUTH_JSON`、ランナーで `opencode auth import` に標準入力で渡す。Issue 5 で依頼者の操作は 2 つだけ。

## 再発防止
- AGENTS.md の先頭の節(依頼者に頼む前に、自分の道具の `--help`・ソース・docs を先に読む。決める前に過去の発言を調べる)。
- docs/ops/intent.md の節(依頼者が方法を指定した過去の指示は、前任の記録に出典がある)。
- opencode.jsonc の worker の拒否(`opencode auth` の取り出しと取り込み)。

## 同じ型の探索
- 依頼者が過去に方法を指定した他の指示の見落とし: S16 の catalog.md の一覧(約 18 件)を、現在の設計と突き合わせていない(未確認)。

## 未確認
- `opencode auth import` が、インストール版 2.2.6 で標準入力を読むこと(ヘルプでは確認済み。実行ログでは `Imported 1 credential` を確認)。
