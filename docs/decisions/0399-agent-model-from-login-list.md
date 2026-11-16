# 0399: 作業用エージェントのモデルは、取り込んだ認証の提供元の一覧から選ぶ。Variable は読まない(0388 を置き換える)
分類: (b) 契約の実装(GitHub は Variables の値を env の表示で伏せず、公開ログに出る。取り込んだ一覧から選ぶ)。(a) 除去(Variable の参照の削除)は下の決定の 2 行目に分けて書く
撤回条件: (1) 依頼者が Variable での指定を改めて求めたとき(発言 msg_1e2274299001Wt4h6i62ve8mbb の要求)。(2) 候補が 2 件のまま run が止まり続けるとき。候補の数は 2026-11-16 の実測で 2 件(下の実測)。見直しは PM が行い、依頼者には選ばせない
日付: 2026-11-16   ROADMAP: R9-1   一覧: —   コミット: 本決定の commit(後続の commit で記録)

## 検索した範囲と件数(設計の前の確認)
- 依頼者の発言: 3 つの範囲の type=user。範囲外(他のセッションの依頼者の発言)は未確認。
  - 前任の全文(リポジトリの外。`~/work/handover/evidence/01a-user-messages-full.md`): 263 件(`grep -c '^## seq'`)。2026-10-19 02:57 〜 2026-11-08 01:28 UTC。Variable・Secret・登録・重複・二重・もう一度・モデル指定の語を含む発言は 8 件(11-04 の 4 件と 11-08 の 4 件。Secrets の依頼と、モデルの API 鍵の登録の話。モデル指定・重複・二重・もう一度は 0 件)。
  - `ses_e47377663ffenaSj97QVpPzRtp`: 84 件(`opencode api session.message.list --param type=user --param limit=200`。cursor の前後は 0 件)。2026-11-08 01:31 〜 11-16 08:23 UTC。同じ語を含む発言は 10 件(登録・Secret・secret)、Variable・変数・重複・二重・もう一度・モデル指定を含むのは 4 件: msg_1cc18fd9b00136Egi9BMa3P0RO(11-11 19:32Z)、msg_1cc23470f001z3v00DJph1WpFw(11-11 19:43Z)、msg_1e1fd2d30001RwkJqrqjPuXtZn(11-16 01:33Z。重複は別件の話)、msg_1e2274299001Wt4h6i62ve8mbb(11-16 02:19Z)。
  - 主セッション `ses_e2b1511aeffe5dF7dYQK544FTz`: 2 件(cursor の前後は 0 件)。msg_1e372bb3d001wZf2JpIusu3g8B(11-16 08:21Z「#7, #8に回答しました」)を含む。
- Issue(`gh issue view <番号> --json state,stateReason,closedAt,createdAt,comments`。`gh issue list --state all` は 7 件: #1・#2・#3・#5・#6・#7・#8。#4 は無い。決定 0388 の注のとおり取り下げ):
  - #5: CLOSED・COMPLETED・04:11:19Z・コメント 0。
  - #6: 起票 02:44:17Z・CLOSED・COMPLETED・04:11:21Z・コメント 0。
  - #7: 起票 04:59:44Z・CLOSED・DUPLICATE・08:20:58Z・コメント 1(08:20:49Z)。コメントの本文は記号のみ「🖕」。投稿者は依頼者として扱う(確認は未確認。名は書かない)。
  - #8: CLOSED・NOT_PLANNED・08:20:14Z・コメント 0。

## 依頼者の回答の読み(決定の根拠)
- 回答: msg_1e372bb3d001wZf2JpIusu3g8B「#7, #8に回答しました」。#7 は「重複」、#8 は「実施しない」で閉じられた。
- 読み(推定。確かめられない): #7(Variable OPENCODE_PROVIDER の登録)を重複と閉じたのは、同じ目的(モデルの指定)の登録を、依頼者に別に頼むな、の意味と読む。根拠: #6 の登録の回答(msg_1e28d917b001aBDYAym44NKP6p、04:11:31Z)の後に #7 が起票された(04:59:44Z)。確かめられない点: 「重複」が「#6 の Variable を使え」の意味か、指定の依頼そのものを取り下げる意味か。コメントは記号だけで、理由は無い。
- 依頼者の発言 msg_1e2274299001Wt4h6i62ve8mbb(02:19:44Z):「#5 : 登録しました。それと、なぜモデル指定をvariableで行う前提になっていないのですか？」。Variable でのモデル指定を求める文。本決定は、その Variable を読まない形に替える。根拠: Variable は GitHub が伏せず、値(モデルの ID。ベンダー名を含む)が env の表示で公開ログに 1 行出た(実測: run 43804191981 の行 271。evidence/R9-3.md の 2・3 節)。AGENTS.md §8 はログへの禁止文字列の表示を禁じる。この替え方は、依頼者の求めと食い違う。食い違いは未解決で、撤回条件 (1) に入れた。

## 決定
- 決定: 作業用エージェントのモデルは、取り込んだ認証の一覧(`opencode models --standalone`)から、提供元の名が console- で始まり、モデル ID が haiku-5-5 で終わる行の 1 件を選ぶ。1 件でなければ run は件数だけを出して止まる(提供元の名前・モデル ID は出さない)。根拠: 上の読み。Variable を読まないので、公開ログに値が出ない。
- 決定(a 除去): ワークフローの Variable 参照(`vars.`)を削除する。`OPENCODE_PROVIDER`・`OPENCODE_MODEL` は、`.github`・`ci`・`scripts`・`tests` で 0 件(実測は下)。根拠: 公開ログに値が出る参照を残さない。
- 決定: 候補が 2 件のとき、どちらの提供元を使うかは、このスクリプトで決めない。根拠: 依頼者の発言 msg_1c2f7aab7001uxIZsXBc9C3Vhj(2026-11-10 00:59Z)は、請求の状況に合わせて 2 つの提供元を切り替えていると書く。既定の 1 件を決めると、その切り替えを無効にする。
- 決定: 検査(`check-agents.sh`)は、設定のモデルの末尾が haiku-5-5・`#` 無し・5 つの agent の一致・worker の拒否の規則 8 件を確かめる。提供元は固定しない。
- 決定: 0388 を置き換える。0388 には置き換えの 1 行だけを足す。

## 実測(コマンド・条件・日付・n)
- 2026-11-16、この機械で `opencode models --standalone`(stderr 0 行): 行数 384。`console-` で始まる行 94(提供元 7)。`console-` で始まり末尾が `haiku-5-5` の行 2(提供元 2。各 1)。`haiku-5-5` を含む行 9(うち console- 以外 7)。`#` を含む行 0。件数だけ。行の中身は書かない(n=1)。
- 同日、`bash ci/agents/runner-config.sh <一時パス>`: 出力は「候補が 2 件」の停止の行、rc 1。設定は書かれない(n=1)。
- 同日、`bash tests/harness/t11_agent_model.sh`: 37 passed、0 failed(n=1)。
- 参照の件数: `grep -rc -e OPENCODE_PROVIDER -e OPENCODE_MODEL .github ci scripts tests` の合計 0(n=1)。
- 前提の訂正: evidence/R9-3.md の 10 節と決定 0388 の追記は「console- の行 5 件、作業用の型を含む行 1 件」を前提にしていた。今回の一覧は console- の行 94、末尾 haiku-5-5 の console- の行 2。食い違いの原因は未確認(取得の時刻・版・提供元の変化の可能性)。

## 読み(解釈。実行していない)
- 2 件は、依頼者が請求の状況で切り替える 2 つの提供元に対応する(上の msg_1c2f7aab… の読み)。この読みが正しければ、候補の 1 件化はスクリプトでは決められない。選ぶ権限は PM(決定)か依頼者にある。
- 0388 の撤回条件 (3)(候補が 1 件でない状態が続く)は、この実測で成立している。

## 未確認
- 解決済み(実測): CI の一覧も 2 件だった(run 43834208450・43834458288。候補が 2 件で停止。evidence/R9-3.md の 11 節)。worker の段と、取り込み後の背景のサービスの起動は、停止のため未到達(未確認)。
- 依頼者の Variable での指定の求めとの食い違い(撤回条件 (1))。
- 値が出た run 6 件(evidence/R9-3.md の 6 節)は、成功していないため削除していない。
- 範囲外の依頼者の発言(他のセッション)。

## 出典
- 依頼者の発言: msg_1e2274299001Wt4h6i62ve8mbb、msg_1e28d917b001aBDYAym44NKP6p、msg_1e372bb3d001wZf2JpIusu3g8B、msg_1c2f7aab7001uxIZsXBc9C3Vhj。
- Issue 5・6・7・8(`gh issue view`)。`docs/requests/0006-actions-model-variable.md`、`docs/requests/0007-actions-provider-variable.md`。
- 決定 0388(置き換え)、0384、0380。失態の記録 0011(Variable の値が公開ログに出た件)。
- evidence/R9-3.md(2・3・6・10・11 節)。
