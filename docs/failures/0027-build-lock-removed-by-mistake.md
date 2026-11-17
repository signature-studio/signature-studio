# 0027: 他の worker が使っていた build-lock を、存在を確かめずに rmdir で外した(ロックの所有者を確かめなかった)

- 日付: 2026-11-17(UTC 09:2x)
- 誰が: worker(R5-6f。セッション ses_e1a4cea55ffeUuIVllzVQbKORi)
- 依頼者の発言: なし(AGENTS §6・main の指示「bundle のビルドは build-lock で 1 回に 1 つ」の違反)

## 何が起きたか

- 09:2x UTC に、bundle の 9a を始める前の確認で `ls -d ~/.cache/signature-studio/build-lock` が、ロックが存在することを示した(出力 `LOCK EXISTS`)。
- 同じコマンドの続きの `mkdir` は失敗(File exists)。その後のコマンドの `rmdir ~/.cache/signature-studio/build-lock && echo "lock released"` が、ロックのディレクトリを消した。
- 消したロックは、別の worker の bundle のビルドのものだった。`ps` の結果(ビルドの工程): `2703483  bash app/scripts/build-engine-bundle.sh`(r45d-diag の emcc の引数が出ている)が、ロック解放の後も動いていた。
- 09:28:44 UTC に、`mkdir ~/.cache/signature-studio/build-lock` で同じ場所にロックを戻した(私の作業の再開はしていない)。
- 私の 9a・9c・10 のコマンドは、リダイレクト先の変数が空で、実行されなかった(`/trace2-phase9a.log: 許可がありません` ほか)。生成物の削除も、他の作業の削除もない(`work-r56e/out-app/gen`・`obj/rt`・`web` を確認済み)。

## 原因(その時の判断)

- ロックの存在を示す出力を見たのに、その後の `&&` の連鎖に `rmdir` を続けた。ロックの所有者(自分が取ったものか)を確かめず、「終わったら rmdir」の規則を、自分のロックかどうかの確認なしに適用した。
- 連鎖の途中で `mkdir` が失敗しても、後続の `rmdir` は独立のコマンドとして実行された(`;` と `&&` の区別を確かめなかった)。

## 影響

- 他の worker のビルドが、ロックを持たずに動いた時間は約 1 分(09:2x〜09:28:44 UTC)。同時に別のビルドが始まった形跡は、確かめていない(未確認)。
- 作業の破壊はない。

## 是正

- 09:28:44 UTC に、ロックを mkdir で戻した。所有者の rmdir は、その worker のビルドが終わったときに行われる(未確認)。

## 再発防止

- 文書: ロックの取得は `mkdir` の成否だけで判断し、成功したときだけ後続を実行する(`mkdir X && { …; rmdir X; }` の形)。`rmdir` を、ロックの存在の確認の後に単独で書かない。
- 検査(未実施): 所有者を記録する(ロックの中に pid と作業名を書く)ことを提案する。今は書いていない。
