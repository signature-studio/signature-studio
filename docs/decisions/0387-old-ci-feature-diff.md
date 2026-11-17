# 0387: 旧 deploy.yml の機能 47 行を一覧にし、戻す 26 件・戻さない 2 件・残っている 19 件と決めた
分類: 整備(lift の処理ではないので (a)(b)(c) の対象外。戻す項目の分類は ROADMAP の行(R6-10〜R6-20)に、戻さない 2 件の分類は本文に書く)
撤回条件: (1) 依頼者が CI の dry run(手動の起動を含む)を必要と言ったとき、A2 を戻す(ROADMAP の行を足す)。(2) 外部インストゥルメントの取り込みを依頼者が求め、bundle が VST3 を読む段を作るとき、D4 を戻す(ROADMAP R5-8 の設計で)。(3) 一覧に機能の漏れが見つかったら、evidence/R6-9.md の表に足す(この決定は変えない)。(4) 依頼者が公開の入口を Access の背後に置かないと言ったとき、G9 を外す。
日付: 2026-11-16   ROADMAP: R6-10〜R6-20(新規 11 行。状態 todo)   一覧: なし(整備)   コミット: この文書・docs/ROADMAP.md・evidence/R6-9.md を含む commit
番号の理由: 0386 は抽出キャッシュ(R6-8)の決定で、この文書より前に別の作業が書く。0387 が最初の空き。

## 決定
- 決定 1: 旧 `archive/ci-old/deploy.yml` の機能を 47 行に分けて一覧にする(`evidence/R6-9.md` §1)。戻す 26、戻さない 2、残っている 19。根拠: 依頼者の発言 msg_1e298741b0011ilEMoBvRoXd5n(2026-11-16 13:23:24。「前任者の成果物を捨てているところが他にないか探さないのですか？」)。
- 決定 2: A2(手動の起動と、main 以外の dry run)は戻さない。除去の分類は (a)。根拠: 依頼者の発言 msg_1e15243580013dGyuyAqcD7Pzz(2026-11-16 07:27:31。「dry-runといった全く不必要な機能を入れる」)と msg_1e1d137bb001YLTwfA0valOgeO(09:45:46。「OpenCodeも動かさず、WorkersのデプロイもしないCIは全く不必要です」)。2026-11-04 の msg_1a2ccc2c3001RSHoExRGznB5pH(トピックブランチの dry run は dispatch で行える)と msg_1a3964d54001FVQ76lEhAA3IYQ(dispatch テスト)は、この 2 件の最新の発言に置き換わる。旧の位置: `archive/ci-old/deploy.yml` の 19 行(workflow_dispatch)と 1049-1054 行(dry run)。
- 決定 3: D4(VST3 の取り込み。Serum 2・OTT・Vital の 3 インストーラから VST3 を取り、保存する段)は戻さない。除去の分類は (a)。根拠: 旧設計の記述 `archive/ci-old/deploy.yml` の 323 行(nothing in the engine bundle reads them yet)と、docs/decisions/0351-engine-bundle-workflow.md の 43 行(現在の 9 段の入力に無い)。依頼者の発言で VST3 の取り出しを求める物は 0 件(範囲: evidence/R6-9.md §0)。Serum に触れた発言 msg_1a891fb3d0017x2dSsfegheTZ9(2026-11-05)と msg_1b3164acc001s8lnNln85ZiIGe(2026-11-07。「今後…Serum 2 などの外部インストゥルメント…を追加していく上で」)は、撤回条件 (2) の根拠で、取り出しの指示ではない。
- 決定 4: 戻す 26 件のうち、キャッシュの 12 件(C1・C2・C4・C6・C7・C8・C9・C10・C11・C12・C13・C14)は分類 (c)(通信・変換の削減)。根拠: 依頼者の発言 msg_1a2ccc2c3001RSHoExRGznB5pH(2026-11-04 04:04:51。「キャッシュは前述のステップごとに行われ、変更がなければステップをスキップします」「キャッシュは、GitHub Actions Cache ではなくキャッシュ用R2に保持」)。抽出のキャッシュ(C3)は R6-8 が実装済み(決定 0386 の作業)。
- 決定 5: 戻す 26 件の残り 14 件(A5・A6・A7・B1・B2・B3・C15・E7・E10・F2・F5・G6・G9・H2)は分類 整備。根拠: 既定(依頼者の発言 msg_1e298741b0011ilEMoBvRoXd5n)。G9(公開の入口の Access の確認)は依頼者の発言 msg_1982c56e0001L9vl9HVSX0PeIB(2026-11-02 02:33:26。「CF Accessによって私のみアクセス可能なWorkers…一般公開されることはありません」)が根拠。G6(cloudflare/wrangler-action)は msg_1a2ccc2c3001RSHoExRGznB5pH(「cloudflare/wrangler-action によるビルド・デプロイ」)が根拠。
- 決定 6: 戻す物の行を ROADMAP に R6-10〜R6-20 の 11 行で足す(状態 todo)。抽出キャッシュ(R6-8)の行は足さない。別の作業が進行中(決定 0386)。
- 決定 7(前提の訂正): 前任の計画 `~/work/handover/setup/R2-import-plan.md` は、旧 deploy.yml・段階キャッシュ・stage-deploy の類を「書き直し」と書き(同ファイルの 293・295・296・297・376 行)、「捨てる」と書いたのは 377 行の ci/agents/ だけ。いまの設計は「書き直し」を経ずに、段階キャッシュ・検査・表・配信の関所を落とした。この計画は依頼者の指示ではなく、R2 担当の subagent の [推奨](同ファイルの 3・4 行)。根拠: evidence/R6-9.md §4。

## 検索した範囲と件数(`docs/ops/intent.md` の「設計の前の確認」1〜3)
- 旧設計の機能: 47 行(evidence/R6-9.md §1)。旧の段は 101(名前つき 97・無名の uses 4)。全段を機能に割り当て、未割り当て 0・余り 0(evidence/R6-9.md §5)。
- 依頼者の過去の発言: 前任の記録 catalog.md の 29 件を全部読んだ(CI の機能に関わる行は 0 件)。前任のセッションの 263 件は語で検索し、引いたのは 4 件(msg_1a2ccc2c3001RSHoExRGznB5pH、msg_1a3964d54001FVQ76lEhAA3IYQ、msg_1a42d9ad6001R8L5w0FaTfZSvq、msg_1982c56e0001L9vl9HVSX0PeIB)。今回のセッションの user 発言 76 件は語で検索して 14 件を読み、引いたのは 7 件(evidence/R6-9.md §3)。
- 0 件の範囲: VST3 の取り出しを求める発言(0 件。範囲は上の 3 つ)。180 日の保存期限を求める発言(0 件)。
- Issue: `gh issue list --state all` の 5 件(#4 は一覧に無い)。コメントは #1 の 1 件(依頼者の連絡の記録)のみ。

## 注意
- 前任のセッションの発言 ID(msg_1a…)は、この表の原文の ID。依頼者の今回の発言の ID(msg_1e…)は `opencode api session.message.list` で確かめた。
- 「2 つだけ」(CI は OpenCode の実行と Workers のデプロイの 2 つ)は docs の PM の言い換え。依頼者の原文は msg_1e1d137bb001YLTwfA0valOgeO の「OpenCodeも動かさず、WorkersのデプロイもしないCIは全く不必要です」。
- 戻さない A2 の判断は、依頼者の 2026-11-04 の許可(dispatch の dry run)と、2026-11-16 の評価が食い違う物を、新しい方で決めた。食い違いは evidence/R6-9.md §3 に書いた。
