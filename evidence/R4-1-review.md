# R4-1: reviewer の検査の報告(コピー)

下は reviewer(セッション `ses_e239d9eecffeo7IuEthGTHFinI`、2026-11-15)の最終報告の全文のコピー。コピーした worker はこの検査を再実行していない。検査対象は `evidence/R4-1.md` の commit `38c92ad` 時点と、`docs/ROADMAP.md` の commit `03a64ae` 時点の R4-1 の行。この前に、同じ記録について reviewer が 3 回差し戻している(`ses_e23b81daeffeaRDn5y7rOf7xqZ`: 415 の出典・perf-check の違い・本番の読み戻しの置き場。`ses_e23ad4328ffegfZeG6Duos4tUg`: 受理済みの記述・pagehide。`ses_e23a7263bffevbSZR0SCqzd8zz`: listen の呼び出しの行)。それらの報告のコピーは保存していない(未保存)。直しは `evidence/R4-1.md` の補い・補い 2・先頭注にある。

## reviewer の報告(全文)

**reviewer 報告(38c92ad・03a64ae。読み取りのみ)**

総合: **差し戻し**(項目 3(e) の 1 句のみ。他の項目は受理)

1. **受理**。`git show --stat 38c92ad` は `evidence/R4-1.md` の 1 ファイル 1+/1-。`git diff -U0 38c92ad~1 38c92ad` は 1 行の置換で、旧文は新文の先頭に全文が残っている。`git show --stat 03a64ae` は `docs/ROADMAP.md` の 1 ファイル 1+/1-。`git diff -U0` の hunk は `@@ -44 +44 @@`(R4-1 の行だけ)。
2. **受理**。`expire_grep`(`host\.listen`、`app/src/perf/failure-beacon.ts`)は 209(`pagehide`)・213(`error`)・217(`unhandledrejection`)の 3 件で、訂正文と一致。`expire_grep`(`reviewer の最終検査は未了`、`evidence/R4-1b.md`)は 10 行目。「9 行目は現在 10 行目」と一致。
3. (a)〜(d) **受理**。
   - (a) `evidence/R4-1.md` の表 24〜26 行: n=3、保存行 1、204、yes。
   - (b) `evidence/R4-1b.md` 6 行(実行なし)、7 行(191・2026-11-07)、82 行(読み戻した 1 件の日付 2026-11-07)。POST の記述は一致。ただし同 8 行目は公開ホストへの POST を「未確認」と書いている。
   - (c) `evidence/R4-1b.md` 67 行: 191。
   - (d) `ls docs/requests` で 0002 が存在。同ファイル 100 行に Issue 番号 3 の行。`expire_grep`(`^\| R4-5 \|`)で `todo`。
   - (e) **差し戻し**。括弧書きの「訂正後の文面は reviewer 未読」は 03a64ae 時点では事実。ただし本検査がその文面を読んだため、受理後はこの句が事実と合わなくなる。`done` を残すなら、句を「この検査(2026-11-15)で読んだ」に直す commit が要る。
4. **受理**。`expire_grep`(`^\| R4-1 (\|[^|]*){9}$`、R4-2 も同じ)は、両行とも 9 列で一致。括弧書きに `|` は無い。
5. **受理**。禁止語の検索で R4-1 行に一致したのは「達成」1 語。「P1〜P3 の達成・…の意味ではない」の否定文脈。ログイン名・秘密の検索語は `evidence/R4-1.md` で 0 件。禁止語 2 種は `evidence/R4-1.md` と `docs/ROADMAP.md` の両方で 0 件。
6. **差し戻し**(項目 3(e) のみ)。ROADMAP を `todo` に戻すかは main の判断。戻さない場合は、句を直した commit のあとに再確認が要る。
   - 判定の外の注記: R4-1 の完了条件「読み戻した 1 件」は、2026-11-07 保存の既存 1 件の読み戻しで、今回の POST ではない。括弧書きはこれを明記しており、事実と一致。

## reviewer の指摘の扱い

1. 項目 3(e) の句は、`docs/ROADMAP.md` の R4-1 の行で、この報告を根拠に直した(main の決定: `done` のまま)。
2. `evidence/R4-1b.md` の 8 行目の公開ホストへの POST の「未確認」は、POST の実行の有無(していない)ではなく、公開ホストの状態の未確認を指す。括弧書きの「POST していない」と食い違いではない。
