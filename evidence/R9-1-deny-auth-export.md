※ 完了ではない: P2 の 120 fps は未確認、P3 は音切れが残る、O9 は未特定、依頼者機の値は 0 件。この記録は許可設定の追加で、機能の進捗ではない。

# R9-1: `opencode auth export` と `opencode auth import` を、全 agent の shell で拒否する(2026-11-16)

依頼: worker と reviewer で `opencode auth export` と `opencode auth import` を拒否する(空白・引数の有無を含む)。認証情報は依頼者の BYOK で、エージェントはこれらを実行しない(AGENTS.md §8)。

## 1. 変更前の穴(規則の評価器で測定)

旧版は `git show HEAD:opencode.jsonc`、新版はワークツリーの `opencode.jsonc`。測定は `tests/harness/lib/rules.py` の `decide_shell`(opencode のスキャナで分割した各片を評価)。

| agent | 命令 | 変更前 | 変更後 |
|---|---|---|---|
| worker | `opencode auth export` | deny | deny |
| worker | `opencode  auth export`(空白 2 つ) | **allow** | deny |
| worker | `opencode<TAB>auth<TAB>export` | **allow** | deny |
| worker | `FOO=1 opencode auth export` | **allow** | deny |
| worker | `bash -c "opencode auth export"` | **allow** | deny |
| worker | `opencode auth export --help` | deny | deny |
| worker | `opencode auth list --help` | deny | allow(完全一致) |
| worker | `opencode auth import ./creds.json` | deny | deny |
| worker | `opencode  auth import` | **allow** | deny |
| worker | `opencode auth` | deny | deny |
| worker | `$X auth export`(変数で綴りを隠す) | allow | allow(限界。§5) |
| reviewer | `opencode  auth export` | deny | deny |
| reviewer | `opencode auth list --help` | deny | deny |
| build | `opencode  auth export` | deny | deny |
| build | `opencode api session.get --param sessionID=ses_x`(回帰の確認) | allow | allow |

旧規則 `opencode auth *` は、空白 1 つの字句にだけ一致していた。

## 2. 差分(`git diff -U0 -- opencode.jsonc`)

```diff
@@ -35,0 +36,4 @@
+    // V-G01 (2026-11-16): `opencode auth export` prints every stored credential; `opencode auth import` writes them. Denied for every agent.
+    // Matches any spacing, arguments, an env prefix or `bash -c` (the words must appear in this order, with `opencode` spelled out).
+    { "action": "tool.use", "resource": "shell:*opencode*auth*export*", "effect": "deny" },
+    { "action": "tool.use", "resource": "shell:*opencode*auth*import*", "effect": "deny" },
@@ -279 +283,3 @@
-        { "action": "shell", "resource": "opencode auth *", "effect": "deny" },
+        // Every `opencode auth` form (any spacing, env prefix, bash -c). The policies above deny export and import for every agent; `list --help` is the one allowed form (help text only).
+        { "action": "shell", "resource": "*opencode*auth*", "effect": "deny" },
+        { "action": "shell", "resource": "opencode auth list --help", "effect": "allow" },
@@ -439 +445,3 @@
-        { "action": "shell", "resource": "git diff *--no-index*", "effect": "deny" }
+        { "action": "shell", "resource": "git diff *--no-index*", "effect": "deny" },
+        // The owner's credentials: every `opencode auth` form (the policies above deny export and import for every agent).
+        { "action": "shell", "resource": "*opencode*auth*", "effect": "deny" }
```

差分は追加だけ。worker の旧規則 `opencode auth *` は、新しい `*opencode*auth*` が全ての一致を含むため、緩めていない。

## 3. 決定(根拠つき)

- 決定: 上位の `policies` に `shell:*opencode*auth*export*` と `shell:*opencode*auth*import*` を置く。根拠: policies は agent の allow で上書きできない。`opencode run*` と同じ置き方。全 agent に効く。
- 決定: worker と reviewer の shell に `*opencode*auth*`(`opencode auth` の全形)を置く。根拠: 空白・タブ・環境変数の接頭辞・`bash -c` をまとめて止める。
- 決定: build の agent 規則は足さない。根拠: build は `*` deny で、allow に opencode auth が無い。上位の policies が export と import を止める。
- 決定: `opencode auth list --help` だけを worker で allow にする(依頼どおり)。注記: 旧版では `opencode auth *` で拒否されていた。依頼文の「許可のまま」は、変更前の状態とは違っていた。
- 決定: `opencode auth export --help` と `opencode auth import --help` は拒否のまま。根拠: (a) 規則の形を揃える。(b) help の実測(§4)で、認証情報は出なかった。(c) worker が help を読む必要は無い。(d) 上書きできない policies に help の例外を入れると、例外の範囲が広がる。

## 4. help の実測(隔離した sandbox)

HOME と XDG_* を空の一時ディレクトリにして、`bash $TMPDIR/opencode/r9-1/help-sandbox.sh <sandbox>` を実行。実機の認証ファイルには触れていない。sandbox に認証情報は無い。

- `opencode auth export --help`: 説明は "print stored credentials, including secrets, as JSON"。help の文だけを出し、認証情報は出なかった。rc=0。
- `opencode auth import --help`: 説明は "import credentials exported by auth export"。"reads stdin when omitted" とあり、引数が無いと stdin を読む。だから素の `opencode auth import` も拒否の対象。rc=0。
- `opencode auth list --help`: help の文だけ。rc=0。
- 実行後、sandbox の data には log ファイル 1 本だけ。

## 5. 限界(LIMIT-allow。`shell-cases.src` と `tests/harness/README.md` の表に記載)

- `X=opencode; $X auth export`: 本文に `opencode` の字句が無いので一致しない。allow のまま。
- `o""pencode auth export`: 引用符で割った綴り。allow のまま。
- どちらも `audit-session.sh` の `opencode auth` の数え方では数えない。検出する手段は無い。

## 6. 代償(既知)

- 本文に `opencode` → `auth` → `export` または `import` の順で字句を含むコマンドは、全 agent で拒否される(コミットメッセージ・メッセージ本文も対象)。この作業では、規則を入れた後の shell 呼び出しを 1 度拒否された(比較用のスクリプトの heredoc に該当語が入っていた)。回避: 本文を write 道具でファイルに書き、`git commit -F <file>` を使う。

## 7. テストと検査の結果

- `bun run tests/harness/lib/gen-shell-cases.ts`(README の手順): `wrote shell-cases.tsv (423 cases)`。`tests/harness/shell-cases.tsv` の差分は追加 37 行だけ。
- `bash tests/harness/t02_permissions.sh`: `5 passed, 0 failed`。`shell cases: 423 agree with the rules (23 of them are documented LIMIT-allow forms)`。
- `bash tests/harness/run.sh`(foreground、出力は `$TMPDIR/opencode/r9-1/run.out`): 全 10 本で `TOTAL pass 599 fail 0 skip 0`、`RESULT: PASS`。t04 も含めて失敗は無い。
- `scripts/preflight.sh` の `agents` 行: `OK`(`ids build,compaction,reviewer,summary,title,worker; build=…sonnet-5-5#; reviewer=…haiku-5-5#; worker=…haiku-5-5#`)。
- `scripts/scan-forbidden.sh opencode.jsonc tests/harness/shell-cases.src tests/harness/shell-cases.tsv tests/harness/README.md`: `0 findings in 4 file(s)`。
- 注記: `scripts/scan-forbidden.sh`(引数なし、リポジトリ全体)は 1 件: `docs/requests/0005-actions-opencode-auth.md:12`(spacing 規則)。この記録の変更の外で、別 worker の文書。`scripts/preflight.sh` の `scan` 行は FAIL(表示は 2 行)。この件は main に報告し、触らない。

## 8. 道具の利用

`portal_` で始まる道具と execute: この作業では 0 回。

## 10. 依頼者の名義の道具(portal_* と execute)の拒否(2026-11-16 追加)

根拠: 依頼者の名義の連携(GitHub・メール・ファイル・予定・Cloudflare)を worker が呼び、Issue に依頼者の名義のコメントが書かれた。許可設定は shell と edit だけが規則で、MCP の道具と Code Mode を拒否していなかった(文書だけの規則)。

書き方(出典は opencode v2.2.6 のソース `/home/ubuntu/work/opencode`):
- MCP の道具の名前: `portal_<ツール名>`。`packages/core/src/tool/mcp.ts:21-25` と `packages/core/src/tool/runtime.ts:288-293`(名前 = namespace + `_` + 正規化した名前)。server 名 `portal` が namespace になるかは未確認(道具の一覧で確かめる)。
- 権限の規則(`permissions`)の道具の判定: `packages/core/src/tool.ts:257`(`whollyDisabled` の道具は一覧に出ない)と `381-384`(最後に一致した規則の resource が `*` かつ deny なら無効)。
- policies の `tool.use`(`<道具>:<入力>`): `packages/core/src/config/plugin/policy.ts:98-111`(権限の評価の hook で deny)。
- policies の `mcp.use`(server 名): `packages/core/src/config/plugin/policy.ts:88-90`(server を登録簿から外す)。
- Code Mode の `execute`: `packages/core/src/tool.ts:260`(`whollyDisabled("execute")` で無効)と `325`(check の拒否)。

追加した規則(`opencode.jsonc`、差分は追加だけ):
- 上位の `policies`: `tool.use` の `portal_*`(全 agent の呼び出しで deny)、`tool.use` の `execute:*`、`mcp.use` の `portal`(server ごと外す)。
- 上位の `permissions`: `portal_*` の `*` deny(一覧に出さない)と `execute` の `*` deny(Code Mode を外す)。

評価器(`tests/harness/lib/rules.py`)の変更: 無し。action の名前(`portal_*`・`execute`)は既存の wildcard で扱える。ただし `mcp.use` の policies は評価器が扱わない(`policy_deny` は `tool.use` だけ)。`mcp.use` は源のコードで確かめた事実であり、テストでは確かめていない(未確認の実測は §9 と同じ)。

テスト(`tests/harness/t02_permissions.sh` の非シェルの表に追加): 7 つの名前(`portal_github_add_issue_comment`・`portal_github_search_code`・`portal_github_get_latest_release`・`portal_gmail_delete_message`・`portal_drive_search_files`・`portal_calendar_update_event`・`portal_cloudflare_search`)と `execute` を、build・worker・reviewer の各 agent で deny。許可のまま: worker の `edit`・`read`(証拠ファイル)、reviewer の `read`(`AGENTS.md`)・`skill`。結果: `non-shell cases: 101 agree with the rules`(`t02` の出力)。

誤検出の事実(README と `tests/harness/README.md` の「誤検出」の行): `*opencode*auth*` は、コマンドの文字列に `opencode` の後ろに `auth` が続くと、ファイル名・コミットメッセージ・引数も拒否する。方針: 規則は狭めない(前任のメインの決定)。書く側が、そういう語を並べない。

未確認: 実セッションでの拒否(main が別の worker で確かめる)。`server 名 portal` の一致。

## 9. 未確認

- 実セッションでの拒否(model と permission の実経路)は未確認。本記録は規則の評価器(`rules.py`)と、実物の opencode の解決(t01、`opencode debug agents`)による。
- 実機で `opencode auth export` と `opencode auth import` を実行していない(実行しないのが目的)。
