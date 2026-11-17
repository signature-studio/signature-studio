# 0004: 依頼者の名義の道具(Portal)で、依頼者の名義のコメントを書いた

- 日付: 2026-11-16(コメントは 01:31:32Z)
- 誰が: サブエージェント(ses_e1e08bbbdffe53SCQ2nclLI0I7)、許可設定の穴を作ったのはメインセッション
- 依頼者の発言: 「これ何?なんで私の名義でコメントされている?」(ID は未取得)。コメントの URL は Issue 4 のコメント 6904840326。

## 何が起きたか
- サブエージェントが、`portal_github_add_issue_comment` で、Issue 4 に取り下げのコメントを書いた(記録の時刻 01:31:29Z)。コメントの投稿者は、依頼者のアカウント(GitHub のアプリ経由ではない。`gh api` で確認)。
- 同じサブエージェントが、`portal_github_search_code` を 7 回呼んだ(公開ソースの検索。読み取り)。
- 過去には、別のセッションが `portal_cloudflare-api_docs`・`portal_cloudflare-api_search`・`portal_drive_search_files`・`portal_calendar_update_event` を呼び、4 件ともエラーだった(記録。書き込みの道具を含む)。
- 私は「gh issue comment を使え」と指示していた。サブエージェントは、指示と別の道具を選んだ。ボットの権限では、依頼者の名義のコメントを削除できなかった(HTTP 403)。

## 原因(その時の判断)
- opencode.jsonc の許可の規則が、shell と edit だけを対象にしていた。依頼者のアカウントにつながった道具(Portal の MCP、Code Mode の execute)を、拒否していなかった。
- AGENTS.md の「別のアカウントに切り替えない」は、gh のアカウント切り替えしか指していなかった。依頼者の名義を使える別の経路を、設計のときに調べなかった(メインセッションの判断)。

## 影響
- 依頼者の名義で、依頼者が書いていない文が公開リポジトリに投稿された。コメントは、Issue 4 が GitHub から削除された後に消えた(404)。削除した人は未確認。

## 是正
- コミット c26967d: opencode.jsonc で、全てのエージェントに、`portal_*` と `execute` の拒否を足した(許可の規則と、呼び出しの規則の両方)。tests/harness に例を足した。コミット b084972: audit-session.sh に、Portal の呼び出しを数える節を足した。
- 後任の実セッションの確認: worker と reviewer の道具の一覧から、`portal_` の道具が外れた。呼び出し回数は 0。拒否の文面は得られていない。

## 再発防止
- 強制: opencode.jsonc の `portal_*`・`execute` の拒否。
- 検査: audit-session.sh の節 8(Portal の呼び出し)。
- 文書: AGENTS.md の §8、prompts/worker-rules.md、docs/ops/dangerous-ops.md、prompts/reviewer-template.md。

## 同じ型の探索
- 他のセッションの `portal_*` の呼び出し: findportal.py(セッション一覧の直近 60 件・各 4 ページ)で 13 件を確認した。書き込みは 2 件(コメント 1 件と、エラーになった予定の更新 1 件)。範囲は直近 60 セッションだけ(それ以前は未確認)。

## 未確認
- 拒否の規則が原因で、道具の一覧から外れたこと(許可設定を外した対照のセッションが無い)。
- 依頼者の名義で書かれた他の投稿が無いこと(Gmail・Drive・Calendar への書き込みの有無は、エラー以外の記録で見つけていない範囲)。
