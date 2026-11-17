# 0024: 私の push(d0d91b30)が、別の worker の未 push・未検証の commit e27ad00f(R5-6c)を origin に送り、deploy の run 43980468976 を起動させた

- 日付: 2026-11-17(JST 08:34 の push。UTC 23:34 の run 作成)
- 誰が: worker(R6-33 の作業者。push d0d91b30 の作成者)。e27ad00f の作成者は別の worker(R5-6c。commit の作者欄は signature-studio agent)
- 依頼者の発言: なし(依頼は main 経由の指示。2026-11-16)。main の決定は 2026-11-17 のメッセージ(main の session からの agent-message)

## 何が起きたか

- R5-6c の worker が、同じワークツリー(共有の .git)で commit e27ad00f を作った(reflog: HEAD@{2026-11-17 07:13:38 +0900})。内容は lift/harness/b2d2/modules.tsv・lift/harness/seeds/soundgoodizer-8800000-seeds.txt(+18 行)・docs/decisions/0409(`git show --stat e27ad00f`)。未 push で、R5-6c の作業は未了。
- 私は R6-33 の commit d0d91b30 を e27ad00f の上に作った(reflog: HEAD@{08:34:13 +0900} の直前は e27ad00f)。
- 同じ command の中で、commit → `git fetch` → `git log --oneline origin/main..HEAD` → `git push origin main` を続けて実行した。`git log` の出力に e27ad00f が出ていたが、読まずに push した。push の出力: `db00513d..d0d91b30  main -> main`。
- origin/main は e27ad00f を含む d0d91b30 になった。push の paths の一致で deploy の run 43980468976(headSha d0d91b30、UTC 23:34:38 作成)が始まった。
- 気づいた後、run 43980468976 を `gh run cancel` で取り消した(UTC 23:35:55。conclusion cancelled)。取り消した時点は cargo と emcc の段階で、ログの件数: upload 0・copy 0・wrangler 0。R2 には何も載っていない。
- main の決定(e27ad00f は origin に残す。取り消した run は再実行して最後まで見る)に従い、`gh run rerun 43980468976` を実行した。UTC 00:42:09 に completed success。ログの件数: upload 5(fl-Oz/ の 5 件。engine は `engine.a695401862ed.wasm`、pack-assets の engine wasm 行の sha256 は a695401862ed…)、入口 `s3://fl-web-assets/manifest.json` への upload 0・copy 0、`verified: 0`、wrangler の step success。
- 結果として、未検証の engine a695401862ed(R5-6c の修正版を含む)が R2 の fl-Oz/ に載った。入口(root)は変わっていない(一覧が空。決定 0410)。

## 原因(その時の判断)

- push の前の確認を、結果を読まずに push の command に続けた。範囲に e27ad00f があることを見ていた。AGENTS.md の運用(push の前に `git log HEAD..origin/main` を見る。失態 0023 の再発防止)を、「範囲が空か」の確認として実行し、「範囲に何が入るか」を読まなかった。
- 同じワークツリー(共有の .git)で、別の worker が未 push の commit を作っていたことを想定しなかった。私の直前の確認(fa751e1a の時点)では範囲は空だった。その後に入った commit 1 件を、push の前に確かめなかった。

## 影響

- origin/main に未検証の commit e27ad00f が入った(main の決定により残す)。
- R2 の fl-Oz/ に、未検証の engine a695401862ed と fl-Oz/manifest.json が載った(rerun の upload 行)。入口(root)は変わっていない。
- 一覧 lift/harness/web/verified.txt は空のまま(決定 0410)。
- 完了の run 43963630080(headSha 11f40f0e。success)のログに、アカウント ID を含む endpoint が 7 行出た(旧の run は 10 行)。main の決定により、ログは削除しない(evidence/R6-33.md に事実を書く)。

## 是正

- run 43980468976: 取り消し(23:35:55 UTC)→ rerun(00:42:09 UTC、success)。
- d0d91b30 で入れた ::add-mask:: は、main の決定で外した(commit 16fe90e8。99 行・最長 80 字)。16fe90e8 の run は 43989906851(結果は evidence/R6-33.md)。
- e27ad00f は origin に残す(main の決定)。

## 再発防止

- 文書(main の決定の文言): 共有のワークツリーで push する前に、`git log origin/main..HEAD` で自分の commit だけか確かめ、他の worker の未検証の commit が入るときは push しない(その worker が検証してから push する)。確かめる command と push は別の command で行い、出力を読んでから push する。
- 検査(未実施。案のみ): pre-push で、push の範囲に deploy の paths の変更があり、その commit の作者が現在の作業者と違うなら止める。

## 同じ型の探索

- 範囲: 私がこのセッションで行った push 3 回(`git log --format='%h %an %s' origin/main..HEAD` と `git diff --stat origin/main..HEAD` で確かめた)。
  - 11f40f0e(範囲 fa751e1a..11f40f0e。commit 2 件。fb7a3ef8 と 11f40f0e。両方とも自分の commit)。
  - db00513d(範囲 11f40f0e..db00513d。commit 1 件。自分の docs)。
  - d0d91b30(範囲 db00513d..d0d91b30。commit 2 件。e27ad00f が別の worker の commit)。← 本件。
  - 16fe90e8(範囲 d0d91b30..16fe90e8。commit 1 件。自分の 1 行)。
- 件数: 4 回中 1 回で、別の worker の commit が範囲に入った(本件)。

## 未確認

- 取り消し後の rerun の前に、R2 に何も載っていなかったことは、ログの件数(upload 0)で確かめた。R2 の一覧は読んでいない(AGENTS の規則で R2 に接続しない)。
- engine a695401862ed の P1〜P3 と Export は未測定。
- run 43963630080 のログにあるアカウント ID の件数は、main の決定により削除しない。
