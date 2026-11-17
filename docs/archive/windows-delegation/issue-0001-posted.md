# Issue 0001 の起票記録

`docs/windows-delegation/issue-0001-api-replay.md`(プレースホルダ `<ISSUE>` 入りの下書き。このファイルは変更していない)を GitHub Issue として起票した記録。

| 項目 | 値 |
|---|---|
| 題名 | `[win-vm] A2b: 実機 Windows で Windows API の応答を測る（tools/win-replay）` |
| リポジトリ | `signature-studio/signature-studio-develop` |
| Issue 番号 | 1 |
| URL | https://github.com/signature-studio/signature-studio-develop/issues/1 |
| 起票日時 | 2026-10-28T20:34:08+09:00(`date -Iseconds`。起票コマンドの直後に取得。GitHub の `createdAt` は 2026-10-28T11:34:03Z) |
| 起票に使ったアカウント | `oc-pm-for-signature-studio[bot]`(`gh auth status` の Logged in の行。GitHub 上の author 表示は `app/oc-pm-for-signature-studio`) |
| ラベル | **なし**(リポジトリに `windows-vm` / `measurement` ラベルが無く、作っていない) |
| 状態 | OPEN |

## 本文

- 下書きの先頭 1〜7 行目(HTML コメント: 題名案・ラベル案・PM への注意と、直後の空行)を除いた、8 行目以降をそのまま使った。内容は変えていない。
- 起票時の本文は 25,371 バイトで、`<ISSUE>` が 11 個あった。
- 起票後に `<ISSUE>` を `1` に置換した本文(25,305 バイト)で `gh issue edit` により更新した。
- 下書き全体(HTML コメントを含む)は 25,937 バイト。

## 確認結果(`gh issue view 1 --json body` で取り直した本文)

- `<ISSUE>` は 0 個。
- コードブロック内は `$ISSUE = '1'`(29 行目)に置換されていた。
- 取得した本文は 25,305 バイトで、置換後のローカル本文と `cmp` で完全に一致した。
- 起票時の本文との差は 66 バイトで、期待値の 11 個 × 6 バイト(`<ISSUE>` 7 バイト → `1` 1 バイト)と一致した。
- `<版>`、`<保存した名前>`、`<masked>` は置換されずに残っている。
- 取得した本文に HTML コメント(`<!--`)は無い。

## 起票時点のリモートの状態(読み取りのみ)

- リモートの main の SHA は `e071fc0563575eddb1a8ffbe45a39198c92deeb3`(ローカルの `origin/main` と同じ)。
- ローカルの `main` は `86439bc` で、リモートに無いコミットが 4 件ある。
- 本文が参照する `tools/win-replay/` 以下(`replay.c`、`cases.tsv`、`compare.py`、`gen_cases.py`、`replay-win10.manifest`、`README.md`、`SMOKE-TEST.md`)と `docs/windows-delegation/README.md` は、リモートの main に無い(`gh api` で 404)。
- VM のエージェントは `git clone` で取得するため、これらを push するまでは Step 2 の `Test-Path` が失敗する。
