# 0014: 依頼者の Variable の値が公開ログに出た後、同じ目的の登録を別の Issue で頼み、依頼者に「重複」として閉じられた

- 日付: 2026-11-16
- 誰が: メインセッション(build)と PM(別のセッション)。依頼 Issue 6 と 7 の文書は docs/requests/0006・0007 にある。
- 依頼者の発言:
  - msg_1e2274299001Wt4h6i62ve8mbb(2026-11-16 02:19:44Z):「#5 : 登録しました。それと、なぜモデル指定をvariableで行う前提になっていないのですか？」
  - msg_1e28d917b001aBDYAym44NKP6p(2026-11-16 04:11:31Z):「#6 : 登録しました。」
  - msg_1e372bb3d001wZf2JpIusu3g8B(2026-11-16 08:21:49Z):「#7, #8に回答しました」
  - 出典の読み込み: `opencode api session.message.list`(主セッション・`ses_e47377663ffenaSj97QVpPzRtp` の type=user)。

## 何が起きたか

- Issue 6(Variable `OPENCODE_MODEL` の登録。`docs/requests/0006-actions-model-variable.md`)は、2026-11-16 04:11:21Z に依頼者が完了として閉じた(`gh issue view 6`)。
- run 43804191981(commit 1aff9bb。success)のログの行 271 に、Variable の値(モデルの ID を含む)が env の表示で 1 行出た(evidence/R9-3.md の 2・3 節。値は書かない)。
- その後、同じ目的の別の Variable `OPENCODE_PROVIDER` の登録を依頼者に頼む Issue 7 を起票した(2026-11-16 04:59:44Z。`docs/requests/0007-actions-provider-variable.md`)。
- 依頼者は Issue 7 を 08:20:58Z に「重複(DUPLICATE)」で閉じ、コメント 1 件(08:20:49Z)を付けた。コメントの本文は記号のみ(「🖕」)。Issue 8(Secret `PUBLIC_HOSTNAME` の登録。別の作業)は、08:20:14Z に「実施しない(NOT_PLANNED)」で閉じた(出典: `gh issue view 7` と `gh issue view 8`)。
- 依頼者の 02:19:44Z の発言は、Variable でのモデル指定を求める文だった。決定 0399 は、その Variable を読まない形に替えた(依頼者の求めと食い違う。未解決。決定 0399 の撤回条件 (1))。

## 原因(その時の判断)

- Issue 6 を依頼する時点で、登録する値が公開ログに出るかを確かめなかった。GitHub が Variables を env の表示で伏せないことは、run 43804191981 の後に分かった(決定 0388 の事実の欄)。
- Issue 7 を起票する前に、同じ目的の登録が既に依頼されていて、その値が別の経路で置き換えられるかを調べなかった。取り込んだ認証の一覧から選ぶ案は、Issue 7 の後に出た(決定 0399)。
- 依頼者に別の登録を頼む前に、自分の道具(`opencode models --standalone`)で、登録を要らない形を確かめなかった。

## 影響

- 依頼者の時間: Issue 7 の手順(依頼者のマシンでのコマンドと、GitHub の画面での登録)を依頼した。依頼者は重複として閉じた。時間の量は未確認。
- 公開物: run 43804191981 など、値が出た run 6 件が公開ログに残っている(evidence/R9-3.md の 6 節。削除は未実施。成功の後に削除する計画のため)。
- CI: run 43830920574(attempt 1・2)は、Variable が空のまま、設定の段で止まった(evidence/R9-3.md の 10 節)。

## 是正

- 2026-11-16: 決定 0399(Variable を読まない。取り込んだ一覧から選ぶ)。Issue 7 は依頼者が閉じた。依頼者への追加の依頼は出していない。
- 是正の検証: run 43834208450・43834458288 は、どちらも候補が 2 件で、設定の段で止まった(evidence/R9-3.md の 11 節)。その後、PM の決定(決定 0399 の追記。2 件以上は辞書順で先頭の 1 件)で、run 43835670107(commit c08104f)が success。worker が `opencode run` で rc 0 の出力を得た(evidence/R9-1.md の 5 節)。ログ検査の件数は evidence/R9-1.md の 5 節の 3。
- 値が出た run 6 件は、成功の後に `gh run delete` で削除した(rc 0。evidence/R9-3.md の 13 節)。

## 再発防止(強制 > 検査 > 文書)

- 文書: 依頼者に Variable・Secret の登録を頼む前に、(a) 登録する値が公開ログに出るか、(b) 同じ目的の登録が既に依頼されていないか、(c) 取り込み済みの情報から決められないか、の 3 点を確かめ、出典(コマンドの出力)を依頼文に書く。この形は未実装(docs/ops/requests.md への追記は未実施)。
- 提案(未実装): `docs/requests/` の既存の依頼との同じ目的の比較を、起票の前に行う検査。

## 同じ型の探索

- 範囲: `gh issue list --state all --limit 100 --json number,state,stateReason,title`(件数 7: #1・#2・#3・#5・#6・#7・#8。#4 は無い。決定 0380 の注のとおり取り下げ)。`docs/requests/`(0001〜0008 の 8 件と TEMPLATE)。
- 登録(Secret・Variable)を依頼者に頼んだ Issue: #1(Cloudflare の Secrets と Variable。依頼者が 2026-11-11 に登録。msg_1cc18fd9b00136Egi9BMa3P0RO)、#5(login の Secret。COMPLETED)、#6(Variable OPENCODE_MODEL。COMPLETED)、#7(Variable OPENCODE_PROVIDER。DUPLICATE)、#8(Secret PUBLIC_HOSTNAME。NOT_PLANNED)。合計 5 件。
- 同じ目的で 2 回目に頼んだ物: #6 と #7(モデル指定の Variable)。この型の件数は 1 組(2 件)。
- 0004 と 0005(モデルの認証情報)は、0004 が依頼者に AWS の認証情報を作らせる誤った依頼で、取り下げられた(docs/requests/0004 の状態の欄)。0005 はその代わりの依頼(COMPLETED)。1 回目が誤りなので、この型には入れない。
- 関連の可能性(同じ目的かは未確認): #2(公開リポジトリの履歴にあるホスト名の取り扱い。NOT_PLANNED)と #8(公開ホスト名の Secret。NOT_PLANNED)。
- 公開ログに値が出た型: 0011(Variable の値が公開ログに出た件。R8-2 の Cloudflare のアカウント ID と合わせて 2 回目)。この型は 0011 で扱う。

## 未確認

- 依頼者の「重複」の意味。推定は決定 0399 の「読み」の欄(同じ目的の登録を別に頼むな、の意味と読んだ。確かめられない)。
- Issue 7 のコメントの投稿者。依頼者として扱ったが、確認は未確認(名前は書かない)。
- 値が出た run 6 件が公開されていた時間(evidence/R9-3.md の 6 節と同じく未確認)。

## 追記(2026-11-16): deploy-worker の 6 回の停止(依頼者の Secret への依存)

- 事実(`gh run view <id> --json conclusion,headSha,createdAt,jobs`。ログ本文は確かめていない): deploy-worker の run 6 件が failure。作成時刻と commit は 43807505570(50e21bc、05:00:53Z)、43812589594(fb42bc4、05:48:12Z)、43819916606(9e2e354、06:53:43Z)、43821895100(f80f9f4、07:10:16Z)、43826158277(a566fff、07:46:26Z)、43831017455(84ebbd8、08:25:00Z)。どの run も、段「公開ホスト名を決める(一覧から得た物。無ければ Secret PUBLIC_HOSTNAME。…)」で failure。
- 段の名前は、公開ホスト名が一覧にも Secret にも無いときに止める、と書いている。停止した版(commit f073cd0 の、統合の前の deploy-worker の workflow)では、その段が `secrets.PUBLIC_HOSTNAME` を参照していた(行 271)。その後、deploy-worker.yml と deploy-engine.yml は 1 本の `.github/workflows/deploy.yml` に統合された(commit aba137b。この統合は別の作業。統合後の版に Secret の参照は無い)。
- 依頼者は Issue 8(Secret PUBLIC_HOSTNAME の登録)を 08:20:14Z に「実施しない(NOT_PLANNED)」で閉じた。依頼者は、この Secret を登録しない(deploy-worker.yml の 6 行目の注)。
- 読み(PM の指摘。main の指示): 配信を依頼者の Secret に依存させた設計が、6 回の停止の原因。同じ型の依頼(依頼者の登録に依存する)。確かめた範囲: 依頼者の登録の依頼は、Issue 1・5・6・7・8 の 5 件(`gh issue list --state all`)。この型の依存の数は、deploy-worker の 1 件(公開ホスト名)。deploy-engine の依存は未確認。
- 是正: この記録では、配信の版を変えていない(統合後の deploy.yml は別の作業の範囲)。6 回の停止は未是正。
