※ この記録は、`evidence/R5-3l-review.md` の reviewer の再検査が未確認とした git の出力の保存で、reviewer の検査ではない。完了ではない: P2 の 120 fps は未確認、P3 は音切れが残る、O9 は未特定、依頼者機の値は 0 件

# R5-3 の再検査で未確認だった git の出力の保存

保存先: `evidence/R5-3l-review2/`(6 ファイル)。すべて main の `HEAD` を基に、読み取りの git コマンドだけで作った。

## コマンドと結果

| # | コマンド | rc | 保存先 | 出力の要点 |
|---|---|---|---|---|
| 1 | `git show --stat --format=%h%x20%s d2d90a9` | 0 | `d2d90a9-stat.txt`(15 行) | 12 ファイル、110 行追加、8 行削除。ファイルは `lift/` の中だけ |
| 2 | `git log --oneline -- lift/harness/seeds/vfs-seed-idle1.tsv` | 0 | `seed-log.txt`(1 行) | 1 件: `14268c2 lift: import source from the previous tree` |
| 3 | `git show --stat --format=%h%x20%s 14268c2 -- lift/harness/seeds/vfs-seed-idle1.tsv` | 0 | `14268c2-seed-stat.txt`(4 行) | 当該ファイル 1 件、244 行追加 |
| 4 | `git diff --numstat a03c903 HEAD -- <wall_fr の 7 ファイル>` | 0 | `wall-fr-numstat.txt`(7 行) | 合計 15 行追加、8 行削除(7 ファイル) |
| 5 | `git diff -U0 a03c903 HEAD -- <同じ 7 ファイル>` | 0 | `wall-fr-diff-u0.txt`(66 行) | 下の「差分の数え方」を参照 |
| 6 | `git diff -U0 a03c903 HEAD -- lift/crates/fe-lift/tests/oleaut_ole.rs` | 0 | `oleaut-ole-diff-u0.txt`(76 行) | 71 行追加、0 行削除(`--numstat`) |

wall_fr の 7 ファイル: `wall_fr4.rs`、`wall_fr5.rs`、`wall_fr13.rs`、`wall_fr15.rs`、`wall_fr16.rs`、`wall_fr19.rs`、`wall_fr20.rs`。

## 差分の数え方(`wall-fr-diff-u0.txt` に対して)

- `+` の行(`^\+[^+]`): 15 行。このうち `rows_0372` を含む行(`^\+[^+].*rows_0372`): 15 行。内訳は `mod rows_0372;` が 7 行、数える式または添字の式が 8 行。
- `-` の行(`^-[^-]`): 8 行。
- `wall-fr-numstat.txt` の合計(15 追加・8 削除)と一致。

## main の読みとの照合

- `d2d90a9` の stat: 一致。12 ファイル、110 行追加、8 行削除。`lift/` の外のファイルは 0 件(`git show --name-only --format= d2d90a9` を `grep -v '^lift/'` に通して 0 行)。
- `vfs-seed-idle1.tsv` の履歴: 一致。`git log` の出力は 1 件(`14268c2`)。
- wall_fr の差分: おおむね一致。`mod rows_0372;` の追加と、行数を数える式への `rows_0372::ROWS_0372.len()` の 1 項の追加。違いが 1 点ある: `wall_fr20.rs` には、数える式のほかに、添字の式(`table[table.len() - 1 - rows_0267::ROWS_0267.len() - rows_0372::ROWS_0372.len()]`)にも同じ 1 項の追加がある(2 つ目のハンク)。これも項の追加だけで、他の変更はない。
- 数値の定数: 変わっていない。`wall_fr4.rs`・`wall_fr5.rs` の `LATER_ROWS` の数値の列(`9 + 15 + 73 + 2 + 33 + 4`、`8 + 15 + 73 + 2 + 33 + 4`)は、追加前と同じで、末尾の項 `rows_0372::ROWS_0372.len()` が増えただけ。`wall-fr-diff-u0.txt` の `-` の行と `+` の行を並べて目視で確かめた(開いたファイル: `evidence/R5-3l-review2/wall-fr-diff-u0.txt`、目視)。差は `rows_0372` の項だけ。

## 未確認

- この記録は差分と履歴の文字列の確認までで、テスト(`cargo test`)は実行していない。`wall_fr` の期待値が正しいかは、この記録からは判断しない。
- `oleaut_ole.rs` の 71 行の中身は、保存した `oleaut-ole-diff-u0.txt` を読んでいない(行数だけ確認)。
