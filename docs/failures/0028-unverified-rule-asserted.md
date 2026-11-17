# 0028: 出典を確かめていない規則(「AGENTS の規則で ~ の形は許される」)を断定して worker に指示し、worker がそれを規則として記録に書いた

- 日付: 2026-11-17
- 誰が: メインセッション(指示の断定。ses_e2b1511aeffe5dF7dYQK544FTz)・worker(R6-39。断定を evidence/R6-39.md に規則として書いた。ses_e174d7901ffeScXRG7gOaXMzuT)
- 依頼者の発言: なし(検査者(reviewer)の指摘。基準は commit 171d5744)

## 何が起きたか

- R6-39 の 4 回目の検査の指示で、メインが「0389 の ~/work/handover の 2 件は、AGENTS 規則で ~ の形はホームの絶対パスの書き方として許されている(依頼者の発言の記録の所在)ので直さない」と書いた。
- worker は、その規則の出典を確かめずに、evidence/R6-39.md の 165 行に「AGENTS の規則で、ホームの絶対パスは `~` の形で書くことになっているため」と書いた(commit 171d5744。`git blame -L 165,165 171d5744 -- evidence/R6-39.md` で確かめた)。
- 5 回目の検査(reviewer)が、AGENTS.md・prompts・docs/ops に該当の規則が無いことを指摘した。メインは、その規則が確かめていない断定だったと訂正した。
- この記録の作成時に確かめた結果: `grep -rn '絶対パス' AGENTS.md prompts docs/ops` は 0 件、`grep -rn 'ホーム' AGENTS.md prompts docs/ops` は 0 件。
- 起動時の worker 指示の本文には「ホームの絶対パス(~ で書く)を書かない」と同じ趣旨の一文があった。その元の場所は未確認(リポジトリの prompts/ には無い)。

## 原因(その時の判断)

- メインは、規則の有無を AGENTS.md・prompts・docs/ops の行で確かめずに、規則として断定して指示した。
- worker は、指示の規則を出典で確かめず、記録の文に規則として書いた(確かめていない物を事実として書いた)。
- 正しくは、規則の出典を grep で確かめ、無ければ「規則の出典は未確認」と書く。または、確かめた事実(検査の仕組み)だけを書く。

## 影響

- evidence/R6-39.md の 1 行の記述が誤りだった。差し戻しが 1 回増えた(5 回目の検査)。
- 依頼者の成果物・公開物への影響は無い(確かめた範囲で)。

## 是正

- evidence/R6-39.md の 165 行を、確かめた事実に書き換えた(検査の仕組みの行番号と、規則は未確認と書く)。この記録と同じ commit。
- 検査の指摘とメインの訂正は、メインのメッセージ(5 回目)。

## 再発防止

- 文書(この記録): 規則を指示・記録に書く前に、その規則の行を AGENTS.md・prompts・docs/ops・scripts のどれかで grep し、出典の行を書く。無ければ「規則の出典は未確認」と書く。
- 検査(未実施): 「AGENTS の規則で」「規則で」を含む行に、出典の行番号が無いものを止める検査は無い。提案のみ。

## 同じ型の探索

- 範囲: docs/・evidence/・prompts/・AGENTS.md。
- コマンド: `grep -rn -e 'AGENTS の規則で' -e 'AGENTS 規則で' docs evidence prompts AGENTS.md`(件数はファイル数 2)。
- 見つけた件: (1) evidence/R6-39.md の 165 行(この記録で是正)。(2) docs/failures/0024-push-carried-other-worker-commit.md の 51 行「AGENTS の規則で R2 に接続しない」。出典を `grep -rn '接続' AGENTS.md prompts docs/ops | grep R2` で確かめ、0 件。(2) は未是正(別の worker の記録。出典が無いことを記録する追記は未実施)。
- 広い語「規則で」を含む行は 128 件あった(`grep -rn '規則で' docs evidence prompts AGENTS.md | wc -l`)。各行の出典は確かめていない(未確認)。

## 未確認

- 起動時の worker 指示の本文にある「ホームの絶対パス(~ で書く)」の一文の元の場所。
- 「規則で」を含む 128 件のうち、出典の無い行の数。
- 0024 の 51 行の規則が、どの文書にも無いことの、AGENTS.md 以外(グローバルの指示の場所)での有無。
