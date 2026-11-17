※ R4-5c の 4 回目の検査の記録。判定: 受理(差し戻し 0 件)。実スレッド版の採否は main の判断で、この記録は判定しない。

## 0. 出典と範囲

- 検査の判定と要点は、main が worker に伝えた内容(reviewer の報告の要約。reviewer の全文は worker は受け取っていない)。ここには、その要約に含まれる確認項目と、要約が未確認としたものの確かめ結果だけを書く。
- 残りの文面の修正(rt1 の木の書き方、P3 の仮説の語)は、この検査の後に行った。決定 0406 と `evidence/R4-5c.md` に反映済み。

## 1. 受理の根拠として確かめた物(検査の要点)

| 項目 | 確かめた事実 | 出典 |
|---|---|---|
| N1: main.ts の行 | rt(03238820)と rt1(1f9c22b3)は 569 行、co(3bbbdb25)と origin/main は 567 行。3bbbdb25 と origin/main の app/src/main.ts は同一 | `git show 03238820:app/src/main.ts`、`git show 3bbbdb25:app/src/main.ts`、`git diff --stat 3bbbdb25 origin/main -- app/src/main.ts`(空) |
| N2: app/ の差 | rt の鎖(3bbbdb25..03238820)で app/ の 8 ファイル、+149/−61。`03238820` の commit は app/ に触れていない(0 行) | `git diff --stat 3bbbdb25 03238820 -- app`、`git show --stat --format= 03238820 -- app` |
| 再計算 | T の per-run p50・p90・p99 の 80 行(20 run × 4)を、独立の Python 実装と Node の出力で比べ、一致 | `evidence/R4-5c/p2/t-recompute-check.txt`(最後の行 `rows compared 80, equal 80, different 0`) |
| 撤回条件の引用 | 0406 冒頭の引用が 0393 の 3 行目と同文 | `sed -n 3p docs/decisions/0393-realthread-on-head.md` と 0406 の引用 |
| 再起動後の表 | 10 run の underrun・書き込み率・presents/s が TSV と再計算の出力で一致 | `evidence/R4-5c-p3-runs.tsv`(`post` の 10 行)と `evidence/R4-5c-recompute-post.txt` |

## 2. 検査の要約にあった未確認の 5 件と、その後の確認

検査の要約は、次の 5 件を reviewer が確かめていないとした。それぞれ、この記録の作成時に確かめた。

1. rt の `#if` 系の行 53 件の再計数。
   - 結果: 53 件(追加行のうち `#if` 1・`#ifdef` 39・`#ifndef` 13)。
   - 出典: `git diff 3bbbdb25 03238820 -- lift app` の追加行に `^\+\s*#\s*(if|ifdef|ifndef)\b` を当てた件数。
2. `fle_sys_cs_owner` の呼び出し 0 件。
   - 結果: 呼び出しは 0 件。`03238820` の木で定義(`lift/runtime/fle_sys.c` 376 行)が 1 件あるだけ。`3bbbdb25` には無い。
   - 出典: `git grep -n fle_sys_cs_owner 03238820 -- ':!patches'`。
3. 束の更新時刻(P2 の chrome-1 と chrome-2)。
   - 結果: chrome-1 は 2026-11-16 22:25:52〜23:04:59、chrome-2 は 2026-11-17 00:02:44〜00:43:00(機械の時刻)。束 1 = chrome-1、束 2 = chrome-2 と読める。
   - 出典: `stat -c '%y' tools/verify-kit/out/r42c-cop2r*-chrome-*`(r4-5-co の out/)。
4. `d557c422` が記録時点の tip だったか。
   - 結果: `ddd7d631` の親は `d557c422`。`ddd7d631` の push の表示は `d557c422..ddd7d631`。よって、push の時点で `origin/main` の tip は `d557c422` だった。
   - 出典: `git show -s --format='%h parent=%p' ddd7d631`、push 時の表示。
5. `tcheck.py` の所在。
   - 結果: `ddd7d631` で追加され、`963b78fe` で削除された。ワークツリーには無い。履歴の `ddd7d631` にある内容は 17 行。
   - 出典: `git log --all -- evidence/R4-5c/p2/tcheck.py`、`git show ddd7d631:evidence/R4-5c/p2/tcheck.py`。

## 3. 残った事項(受理の後に変えていない物)

- `fle_sys_cs_owner` を除くかどうかは決めていない(呼び出し 0 件。除くには bundle の作り直しと再測定が要る)。
- rt と co は app/ の 8 ファイルも違う。その差が P3 に影響した可能性は未確認。
- 未確認の一覧(9 件)は `evidence/R4-5c.md` §6 と決定 0406 §10 に同じ内容で載っている。
