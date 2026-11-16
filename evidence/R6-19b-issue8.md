※ 依頼者向けの本文の内部用語の件数の表。直したのは Issue 8 だけ

## 数え方
- 対象: 依頼者が読む部分。コードブロック(三連バッククォート)と「自分で試した記録」の節は除く。「完了の確認コマンド」の本文の行、「依頼なしで続ける作業」「確認の記録」は含む。
- 語(英字は単語の一致。件数は出現回数。括弧内は行番号): workflow(ワークフローを含む)、commit、main、worker(英小文字)、run、段(普段・段階は除く)、probe、binding、standalone、env、evidence、rc、dry-run、ジョブ。
- 用語集で許可されている「コミット」は数えていない。ファイル名・コマンド名の引用(`deploy-worker.yml`、`gh run list`、`origin/main`、`evidence/…`)も数えている。行を見て、引用か用語かを分けた。
- 道具: python3 の正規表現で行ごとに数えた。Issue 8 の完了条件は次の grep で確かめた: `gh issue view 8 --json body --jq .body | grep -c -E 'workflow|commit|probe|binding|main の|gh run rerun'`。

## 1 Issue 8(今回の対象)
| 時点 | 全体 | 記録の節を除く | 記録の節 |
|---|---|---|---|
| 直す前(起票時の本文) | 8 | 4 | 4 |
| 直した後(`gh issue view 8 --json body --jq .body`) | 4 | 0 | 4 |

直した後の記録の節の 4 件は、`.github/workflows/deploy-worker.yml`、`docs/decisions/0379-deploy-worker-workflow.md`、`binding`、`probe` の引用(記録の節の中)。

## 2 docs/requests/(9 ファイル。依頼者が読む部分の件数)
| ファイル | 対応する Issue | 件数 | 内訳(行番号) |
|---|---|---|---|
| 0001-cloudflare-actions-secrets.md | #1(CLOSED) | 19 | workflow 13(3,5,35,37,73,84)、commit 1(84)、worker 1(75)、run 1(73)、evidence 3(3,37,73) |
| 0002-owner-machine-measurement.md | #3(OPEN) | 0 | — |
| 0003-history-routes-value.md | #2(CLOSED。取り下げ) | 0 | — |
| 0004-actions-model-credentials.md | なし(取り下げ) | 0 | — |
| 0005-actions-console-login.md | #5(CLOSED) | 16 | workflow 6(18,19,39,40,43,44)、commit 1(44)、run 5(18,38,43,44)、段 1(44)、evidence 2(44)、ジョブ 1(18) |
| 0006-actions-model-variable.md | #6(CLOSED) | 12 | workflow 1(20)、commit 1(46)、run 6(19,21,42,46)、段 1(46)、evidence 3(19,46) |
| 0007-actions-provider-variable.md | #7(OPEN) | 6 | workflow 3(15,19,20)、run 1(38)、evidence 1(15)、ジョブ 1(34) |
| 0008-public-hostname-secret.md | #8(OPEN) | 2 | worker 1(16)、main 1(46)。いずれもファイル名・コマンドの引用 |
| TEMPLATE.md | なし | 0 | — |

## 3 起票済みの Issue(番号 4 は存在しない。`gh issue view 4` は見つからないと返す)
| Issue | 状態 | 件数 | 内訳(行番号) |
|---|---|---|---|
| #1 Cloudflare 資格情報の登録 | CLOSED | 2 | workflow 2(31)。件名の末尾に、用語集の禁止語が残る |
| #2 履歴にあるホスト名の取り扱い | CLOSED | 9 | commit 5(8,17,29)、main 1(9)、evidence 3(8,9,16) |
| #3 依頼者機での計測 | OPEN | 0 | — |
| #5 console のログイン情報の登録 | CLOSED | 13 | workflow 6(18,19,39,40,43,44)、commit 1(44)、run 5(18,38,43,44)、ジョブ 1(18) |
| #6 作業用エージェントのモデルの Variable | CLOSED | 9 | workflow 2(18,19)、commit 1(45)、run 5(20,41,45)、evidence 1(45) |
| #7 提供元の指定の Variable | OPEN | 6 | workflow 3(15,19,20)、run 1(38)、evidence 1(15)、ジョブ 1(34) |
| #8 公開ホスト名の Secret | OPEN | 2 | worker 1(16)、main 1(46)。いずれもファイル名・コマンドの引用 |

## 4 依頼者への影響(Issue ごと)
- #1 CLOSED: 依頼者の操作は求めていない(閉じている)。内部用語が 2 件残る。未修正。
- #2 CLOSED: 依頼者の操作は求めていない(閉じている)。内部用語が 9 件残る。未修正。
- #3 OPEN: 依頼者機での計測を求めている。この表の語は 0 件(「段」の 1 件は「普段」の一致で、除外済み)。
- #5 CLOSED: 依頼者の操作は求めていない(閉じている)。内部用語が 13 件残る。未修正。
- #6 CLOSED: 依頼者の操作は求めていない(閉じている)。内部用語が 9 件残る。未修正。
- #7 OPEN: 依頼者の Variable 登録を求めている。別の作業者の commit 1e70e2a(0007 の直し)の後も、内部用語が 6 件残る。件数は直す前と同じで、行番号だけが動いた(38・34)。この作業では変えていない。
- #8 OPEN: 依頼者の Secret 登録を求めている。直した後は 2 件(ファイル名と、コマンドの引用)。この表の語で内部用語は残っていない(引用を除く)。
