# 0001: 依頼者に AWS の認証情報を作らせる Issue を出させた(BYOK の取り違え)

- 日付: 2026-11-16(Issue の起票は 01:17Z)
- 誰が: メインセッション(指示)、後任(起票)
- 依頼者の発言: 「なぜさっきまでBYOKについて散々叱責してきたのに、その1時間後に同じ失敗を繰り返しているのか」(msg_1e1f47b75001Ml9NjyQbbQeyBG、01:24Z)。続けて、過去の発言を貼った(msg_1e1f8328e001r5ffK7lwKiOkEz、01:28Z。元の発言は msg_1e1024fb7001O26DOtWu58mBXZ、2026-11-15 20:59Z: 依頼者が使うのは console の BYOK だけ。ローカルマシンの認証情報を使う判断が誤り)。

## 何が起きたか
- 後任への指示に、「モデルの認証情報の Secrets は私には登録できない。Issue で名前と手順を依頼する」と書いた。後任は Issue 4(2026-11-16T01:17:25Z)を起票した。
- 本文は、依頼者に AWS の IAM ユーザーを作らせ、Bedrock の API キーかアクセスキーのどちらかを選んで登録させ(選択)、地域名と ARN の空欄を埋めさせ(空欄)、完了の確認に Secrets の一覧を使い(許可していない操作)、内部用語を含んでいた。出典は GitHub の Issue 4 の本文(後に GitHub から削除された。削除した人は未確認)と、docs/requests/0004 の旧本文(コミット履歴)。
- 依頼者は 2026-11-08 に、「opencode の認証情報だけを export し、実行環境で import しろ」と指示していた(msg_1b854db83001HV82xjrIrfn2e8、前任のセッション。要約は ~/work/handover/setup/S16-intent/catalog.md の 3・14・15 行)。

## 原因(その時の判断)
- 設計の前に、依頼者の過去の発言も、依頼者のモデルの認証の方法も調べなかった。目の前の指示(「モデルを動かすには認証情報が要る」)と、自分の知識だけで、AWS の経路を選んだ。
- AGENTS.md §10 の「トークンの発行・Secrets の登録は Issue で頼む」を、依頼の許可として読んだ。何を頼んでよいかの検査が無かった。

## 影響
- 依頼者が、同じ失敗を再び指摘する手間と時間。起票から削除までの約 1 時間、誤った依頼が公開されていた。

## 是正
- Issue 4 を取り下げ、docs/requests/0004 を取り下げの短い文書に置き換えた(コミット 2754249)。
- Issue 5 を起票した(2026-11-16T01:53:55Z)。依頼者の操作は 2 つだけ(export と Secret の登録)。依頼者が登録した。Actions の実行 43791988553・43792496714・43793599375 で、取り込み 1 件の成功と提供元の件数をログで確認した。
- 決定 0382(決定 6)、決定 0383。

## 再発防止
- 文書: AGENTS.md の先頭の節(決める前に過去の発言を調べる)、AGENTS.md の依頼の節(モデルの認証は console の BYOK だけ)、docs/ops/requests.md、docs/ops/authority.md、prompts/worker-rules.md。
- 検査: scan-forbidden.sh の byok の規則(docs/requests/ の中の AWS の認証情報の語を検出)。tests/harness の t04。
- 強制: opencode.jsonc で、worker の `opencode auth` の実行を拒否(認証情報の取り出しと取り込み)。

## 同じ型の探索
- 依頼者に別の認証情報を作らせる Issue が他に無いか: gh issue list --state all(Issue 1・2・3・5・6 を確認。Issue 1 は Cloudflare の登録で、依頼者が許可した範囲)。件数 0。
- 探した範囲は GitHub の Issue と docs/requests/ だけ。ワークフローが読む Secrets・Variables の一覧は、許可が無いため読んでいない(未確認)。

## 未確認
- Issue 4 を GitHub から削除した人と時刻。
