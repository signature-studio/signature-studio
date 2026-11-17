# 0019: 依頼者が求めた R2 へのキャッシュを、依頼者の発言を確かめずに捨てる決定をした(0402)。同じ型の探索で、捨てる・置かない 24 件のうち発言 ID が付くのは 4 件

- 日付: 2026-11-16
- 誰が: メインセッション(決定 0402 の作成と、「抽出のキャッシュをやめ、extract-layer を削除する」の指示。PM 経由の情報)。0402 の実装は後任の worker(commit ebc90f6)。
- 依頼者の発言:
  - msg_1a2ccc2c3001RSHoExRGznB5pH(JST 2026-11-04 04:04:51。ワークツリーの外の handover の 01a-user-messages-full.md の 1216〜1232 行、seq 44954): 「キャッシュは、GitHub Actions Cacheではなくキャッシュ用R2に保持します。そのため、actions/cacheではなくruns-on/cacheを使用します。」(1226 行)
  - msg_1e28f3a20001GYppSYhLxztoMh(JST 2026-11-16 13:13:19。原文は evidence/R6-9.md の 92 行の引用): 「ワークフロー側のエージェントはR2からアーティファクトを読み取り、実際のインストーラーから抽出を行い、抽出物はR2にキャッシュする、前任者と同じことをやっているのですよね？」
  - 依頼者(2026-11-16。ID 無し。0402 の 7 行の引用): 「(4) ci/extract-cache.sh は不要です。なんなら .github/actions/extract-layer/action.yml ごと不要かもしれません。なぜ runs-on/cache を採用しないのか、理解に苦しみます。」
  - 依頼者(2026-11-16。main 経由の報告。ID は未確認。原本は道具で取れなかった。`opencode api` は worker の権限で拒否された): 「deploy.yml に、アーティファクトのダウンロード、R2 へのキャッシュがありませんが」

## 何が起きたか

- 0402(commit ebc90f6、2026-11-16 18:41:57 +0900)は「抽出のキャッシュをやめる」と決めた(0402 の 2・23 行)。根拠は時間。evidence/R6-8.md の 76 行「R2 の復元 9 秒は、キャッシュ前の取得 + 抽出(9〜12 秒)と同程度。時間は減っていない。」。0402 に msg_ の ID は 0 件(`git show d81c3b88^:docs/decisions/0402-agent-workflow-inline.md | grep -c msg_`)。
- 0402 の 23 行は「.github/actions/extract-layer(取り除く済み)」「ci/extract-cache.sh(取り除く済み)」と書いたが、22f1c0e の木には両方あった(作成と削除の履歴、`git ls-tree 22f1c0e .github/actions ci/extract-cache.sh`)。agent.yml の 54・82 行は extract-layer を使っていた(`git grep -n 'extract-layer' 22f1c0e -- .github`)。
- 0401(ebc90f6 より前)は runs-on/cache について、依頼者の「なぜ runs-on/cache を採用しないのか理解に苦しみます」と msg_1a2ccc… を引き、「0400 の公開 action の限定と食い違う」と書いて、採否を「未決(main)」にした(0401 の 26 行)。その後の 0402 は、未決のまま、キャッシュをやめる側に決めた。
- 0404(取り消し。commit 8b51a44 に含む)は、「0402 は、時間が減らないことを理由に、依頼者の発言を確かめずに機能を捨てた(誤り)」と書いた(0404 の 13 行)。
- 8b51a44 は agent.yml の 63・117 行に `runs-on/cache@v5`(RUNS_ON_S3_BUCKET_CACHE: fl-web-cache、endpoint は R2)を置き、extract-layer と ci/extract-cache.sh を削除した。deploy.yml には cache・fl-web-assets・artifact・R2 の語が 0 件(`git grep -c -i -E 'cache|fl-web-assets|artifact|R2' 8b51a44 -- .github/workflows/deploy.yml` の結果は空)。依頼者が指摘した deploy.yml の件は、この commit では未是正。

## 原因(その時の判断)

- 0402 は、依頼者の発言を確かめずに、時間の比較だけで「キャッシュをやめる」を決めた。msg_1a2ccc…(2026-11-04)を、決定の前に検索も確認もしなかった。AGENTS の先頭の規則(捨てる前に依頼者の過去の発言と、置き換える前の機能を調べる)を実行しなかった。
- 0402 の 23 行の「取り除く済み」を、ファイルの有無で確かめなかった(22f1c0e で両方あった)。
- 0401 は同じ ID を引き、食い違いを見つけながら採否を main に回した。見つけた時点で、依頼者の要求との照合を決定の文に書かなかった。

## 影響

- 依頼者が求めた R2 のキャッシュ(runs-on/cache)が、0402(ebc90f6)から 8b51a44 までの間、実装されていなかった。取得の 1.1 GB が毎回 R2 から取られた(evidence/R6-8.md の 76 行の記述による。run の測定は未確認)。
- 依頼者の時間(同じ指摘の繰り返し)。
- deploy.yml の R2 キャッシュとアーティファクトの取得は、8b51a44 の時点でも無い。

## 是正

- 0404(決定の取り消し。msg_1a2ccc… と msg_1e28… を引く。commit 8b51a44 に含む)。
- 8b51a44(agent.yml の runs-on/cache@v5 の 2 段。extract-layer と ci/extract-cache.sh の削除)。別の worker の作業。run での動作の確認は未確認。
- deploy.yml の R2 キャッシュとアーティファクトの取得: 未是正。依頼者の指摘の件として main の判断に返す。

## 再発防止

- 文書: 決定の文に「依頼者の発言 ID」の行を置く。ID が無いときは「発言の裏づけ無し」と書く(未実施)。
- 検査の案(未実施): 決定文書に『やめる・削除・置かない・取り除く』の行があるのに、同じ決定に msg_ が無いときに scripts/check-consistency.sh で止める。
- 「取り除く済み」の語を決定に書くとき、`git ls-tree` と `git grep` で存在と参照を確かめる(未実施)。

## 同じ型の探索

- 範囲: 決定 0388〜0404(17 件。`git ls-tree --name-only HEAD docs/decisions/` の一覧。HEAD は dd4d1ab で、8b51a44 を含む)と docs/ROADMAP.md(95 行)。
- 対象: 「削除・置かない・外す・撤回・廃止・読まない・飛ばす」の語を含み、かつ bundle・R2・root・抽出・キャッシュ・cache・probe・公開ホスト・Variable・vars の語を含む行(`grep -n -E` と Python で抽出)。
- 数え方: 件を書いた行(同じ行)に msg_ があるかで判定した。同じ決定の別の行の ID は数えない。ただし件の行が ID を引いて撤回・逆方向を述べる場合は、ID 有として数えた。

| No | 出典(HEAD) | 件(捨てる・置かない・撤回) | 依頼者の発言 ID | 判定 |
|---|---|---|---|---|
| 1 | 0388 行 8 | 旧 Variable OPENCODE_MODEL をワークフローとスクリプトから参照しない | 無し(根拠は run 43804191981 のログ) | 発言の裏づけ無し |
| 2 | 0399 行 1・20 | Variable を読まない(0388 の Variable 設計を廃止) | msg_1e2274299001Wt4h6i62ve8mbb(行 20。Variable での指定を求める発言。逆方向) | ID 有(逆) |
| 3 | 0399 行 24 | ワークフローの vars. 参照を削除 | 無し(根拠は公開ログの実測) | 発言の裏づけ無し |
| 4 | 0402 行 2・7・11 | import-login・runner-config・check-agents を取り除き、確かめの段を外す | 無し(行 7 の引用に ID 無し) | 発言の裏づけ無し |
| 5 | 0402 行 2・23 | 抽出のキャッシュをやめる(extract-cache・stage-cache・stage-keys・extract-layer・stage-cache を取り除く) | 無し(行 7 の引用に ID 無し。根拠は時間) | 発言の裏づけ無し |
| 6 | 0395 行 51 | 段 gen のキャッシュ(0391)を撤回 | 無し。同じ決定の行 7 は、キャッシュを求める msg_1a2ccc… と msg_1a36588b… を引く(逆方向) | 発言の裏づけ無し |
| 7 | 0400 行 32 | 抽出の層(R2 の抽出キャッシュを含む)を置かない | 無し(0400 は msg_ 0 件。行 17 の原文は main 経由) | 発言の裏づけ無し |
| 8 | 0400 行 33 | 9 段の bundle と段のキャッシュ(stage-cache)を置かない | 無し | 発言の裏づけ無し |
| 9 | 0400 行 34 | R2 fl-web-assets への 5 件の載せを置かない | 無し | 発言の裏づけ無し |
| 10 | 0400 行 35 | root の manifest.json の差し替えを置かない | 無し | 発言の裏づけ無し |
| 11 | 0400 行 36 | 結果の表を置かない | 無し | 発言の裏づけ無し |
| 12 | 0400 行 30 | deploy-worker の公開ホスト名を決める段と probe を置かない | 無し | 発言の裏づけ無し |
| 13 | 0400 行 31 | ci/lib/wrangler_config.py の検査を置かない | 無し | 発言の裏づけ無し |
| 14 | 0400 行 46 | 旧 2 本(deploy-engine.yml・deploy-worker.yml)を削除 | 無し(行 17 の原文は main 経由) | 発言の裏づけ無し |
| 15 | 0401 行 9・11 | S1 の R2 抽出キャッシュ・S6 の段キャッシュを外す | 行 11 が msg_1e28f3a20001… を『旧の残す根拠』として引き、『抽出キャッシュを外すため撤回』と書く(逆方向)。行 9 は『不要かもしれません』(本日)の引用で ID 無し | ID 有(逆) |
| 16 | 0401 行 35 | 捨てる物(build-engine-bundle.sh・Python のつなぎ・stage-keys.sh の関数本文のハッシュ) | msg_1b7bb7b92001i7T2kIeDM1AT7O(依頼文「スクリプト禁止」を引く。01a に 1 件あり) | ID 有(支持) |
| 17 | 0403 行 24〜26 | deploy.yml の 20〜57 行(設定を作る段)・確かめ・戻し・versions upload、wrangler_config.py の参照を削除 | 無し(行 25 は『依頼者の指示(過度な異常系は不要)』。ID 無し) | 発言の裏づけ無し |
| 18 | 0389 行 25 | 公開ホストが無いときに Cloudflare に書く前に止める(決定 5 の (3)) | 無し | 発言の裏づけ無し |
| 19 | 0398 行 1・3 | 公開ホスト名が無いときは probe を飛ばす(配信は止めない) | 無し(同じ決定の行 44 は Access の検査の趣旨で msg_1982c56e0001… を引く。飛ばす根拠ではない) | 発言の裏づけ無し |
| 20 | 0398 行 36 | 依頼者は PUBLIC_HOSTNAME を登録しない(前提) | 無し(関連の msg_1e372bb3d001… は 0399 行 18 にあり、0398 行 36 には無い) | 発言の裏づけ無し |
| 21 | ROADMAP 行 77(R6-10) | GitHub Actions の cache は使わない。extract-layer・extract-cache.sh の削除済みの注 | msg_1a2ccc2c3001…(同じ行。キャッシュを R2 に置くことを求める。逆方向) | ID 有(逆) |
| 22 | ROADMAP 行 89(R6-23) | エンジン bundle のシェル・Python・Makefile のつなぎを消す | 無し(行 89 に ID 無し。根拠は『依頼者の指示(2026-11-16 の 2 通)』) | 発言の裏づけ無し |
| 23 | ROADMAP 行 61(R6-3) | 既定の入口(root)は差し替えない | 無し | 発言の裏づけ無し |
| 24 | ROADMAP 行 74(R8-4) | 本番の既定の入口は差し替えない(R6-4 まで) | 無し | 発言の裏づけ無し |

- 件数: 24 件。発言 ID が付くのは 4 件(逆方向 3 件: No 2・15・21。支持 1 件: No 16)。付いていないのは 20 件(発言の裏づけ無し)。
- ROADMAP の行 86(R6-19)は probe を残す側で、msg_1982c56e0001… を引く。捨てる行ではないので数えない。0398 の『probe を飛ばす』とは食い違う(未確認。main に返す)。

## 未確認

- msg_1e28f3a20001… の原本。01a にはこの ID が無く(`grep -c` は 0 件)、evidence/R6-9.md の 92 行の引用だけを確かめた。
- main 経由の「deploy.yml に、アーティファクトのダウンロード、R2 へのキャッシュがありませんが」の発言 ID と原文。
- 8b51a44 の runs-on/cache の動作(run の結果)。
- 0402 と 0404 が引く「時間」の測定の再現(evidence/R6-8.md は読んだ。run の再測定はしていない)。
