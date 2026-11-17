reviewer の再検査の報告(2026-11-16、対象 b754613)。判定: 差し戻し(不一致 2 件)。要点の保存(原文ではない)

1. 不一致 1: evidence/R3-6b-review.md の先頭に、原文のホームの絶対パスを省いた旨の注が無い。注は項目 1 の括弧内の「原文の表記は、この保存で省いた」だけ。
2. 不一致 2: evidence/R3-6b.md の 150 行付近「禁止語の確認(決定): 残す」。reviewer は、前回の検査が main に残した判断を worker が決定として書いた、と読んだ。main の訂正: この判断は main(依頼を出した側)が 2026-11-16 に「残す」と決めて worker に指示した物で、worker が自分で決めた物ではない。
3. 出典不足: evidence/R3-6b.md の 83 行付近と 140 行付近の「12 回すべて」(User-Agent の件数・index の行数と出現数)。reviewer が確認できたのは A-1・B-1・B-5 の 3 フォルダだけで、残りの 9 フォルダの実数が文書に無い。150 行付近の「scan-forbidden.sh(3 ファイル、0 件)が通る」と、83 行付近の「grep -c と grep -o … | wc -l で確認」は、出力が文書に無い。
4. docs/decisions/0378-focusout-recovery-removal-experiment.md の 78 行「reviewer の検査は未実施」は、今は誤り(初回の検査は evidence/R3-6b-review.md に保存済み)。
5. 数え直しの要望: 12 フォルダの User-Agent の件数と index の出現数を、フォルダごとに数えて表で示す。
