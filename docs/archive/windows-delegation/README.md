# Windows 実機への測定の委任（Issue → Windows VM のエージェント → `worklog/`）

状態: **新設の手順書（まだ一度も実行していない）。** Windows VM 上のエージェントが指示書どおりに動くかは**未確認**。

## なぜ必要か

完了条件 A2b（決定 0046 §3）は「実行で踏まれる import 応答が全て仕様確認済み」を求める。現状は 171 種のうち 78 種（P/X）が仮・創作・非公式で、実機の Windows で測らない限り数えられない（`docs/investigations/a2b-audit/SUMMARY.md`）。この開発環境（Linux）に Windows 実機は無いが、**Hyper-V 上の Windows VM で同じ OpenCode のエージェントを動かせる**。そこで、測定を GitHub Issue の形で VM のエージェントに依頼する。

## 全体の流れ

1. **PM** が指示書の下書きを `docs/windows-delegation/issue-NNNN-<名前>.md` に置く（Markdown。GitHub Issue の本文としてそのまま投稿できる形）。測定に使う道具は `tools/<名前>/` に置く。
2. **PM** が内容を確認し、GitHub に Issue として投稿する。本文中の `<ISSUE>` を、付いた番号に置換する（コードブロック内の `$ISSUE = '<ISSUE>'` を含む）。**下書きの作成者（このリポジトリの作業者）は投稿しない。**
3. **PM** が Windows VM のエージェントに Issue の URL を渡して、実行を頼む。
4. **VM のエージェント** が、リポジトリを clone し、ブランチ `worklog-issue-<n>` を作り、道具をビルドして走らせ、結果を `worklog/<n>/` にコミットして push し、Issue にコメントする。
5. **PM** が結果を検証する（下の表）。受け入れる場合は、測定記録として `docs/investigations/` に解釈の文書（実測／読み／未確認の凡例つき）を書き、`worklog/<n>/` を main に取り込む（マージの方法は PM が決める）。
6. 解釈（A2b を満たすか）は PM が決定 0046 の基準で行う。外部エージェントはしない。

## 取り決め

- 外部エージェントが変更してよいのは `worklog/<n>/` だけ。`main` に push しない。
- 外部エージェントは**測って記録する**だけ。結果を解釈・採否判断しない。
- **FL Studio のバイナリ・インストーラ・抽出物を Windows 側に渡さない・持ち込ませない**（ライセンス）。道具は Windows の標準 API だけを呼ぶ独立したプログラムで、FL Studio のコードを含まない。トレースの引数に FL Studio 由来の文字列（パス、キー名、フォント名、API 名）が入るのは調査データであって、これは可。
- 資格情報（トークン、SSH 鍵、パスワード、ブラウザの保存情報）に触れさせない。push には VM に既にある認証を使うだけ。
- レジストリとシステム設定を変えさせない。ネットワークへは測定結果以外を送らせない。
- 道具（`replay.c` など）を外部エージェントに書き換えさせない。測定の同一性は `artifacts-sha256.txt` で確かめる。
- 外部エージェントの完了報告は鵜呑みにしない。報告された内容が実際のコミットと一致するかを PM が確認する。

## PM の検証表（受け取ったとき）

| 確認 | 方法の例 |
|---|---|
| 変更範囲 | `git diff --stat main...worklog-issue-<n>` が `worklog/<n>/` だけ。exe・dll・巨大なファイル・FL Studio の物が無い |
| 道具が無改変 | `artifacts-sha256.txt` の `tools/win-replay/replay.c` `cases.tsv` `compare.py` の SHA-256 が、検証時点の main のファイルと一致 |
| 完走 | `observed*.tsv` の行数が `cases.tsv` の件数と一致。`# tag` `# RtlGetVersion` `# mask-private 1` のヘッダが期待どおり。`# RtlGetVersion` が `env.md` と合う |
| 再現できる | Linux 側で `python tools/win-replay/compare.py --cases tools/win-replay/cases.tsv --observed worklog/<n>/observed.tsv --observed-manifest worklog/<n>/observed-manifest.tsv` を実行し、提出された `compare-report.md` と同じ数になる |
| 異常 | `crash:`・`hang-or-killed`・`nomodule`・`noexport` の行を一覧する。全行が skip／全行が同じ値、などの異常が無い。A と A'（繰り返し）の差が時刻・ポインタ系の行だけ |
| 禁止事項 | `notes.md` のコマンドに、レジストリや設定を変えるもの（`reg add`、`Set-ItemProperty`、`setx`、`Set-WinSystemLocale` など）、FL Studio の取得、資格情報の参照が無い |
| 個人情報 | `worklog/<n>/` をユーザー名・ホスト名・メール・IP・ドメインで検索して、ヒットが無い |
| 報告との一致 | Issue のコメントの数（件数、一致／不一致）が、コミットされたファイルの数と一致 |

## データの扱い

- `worklog/<n>/` は**一次記録**。編集しない（誤りが分かったときは、別の文書に訂正を書く）。
- 実機の値を見ても、すぐ runtime の応答を直さない。まず `docs/investigations/` に、どの値が仕様の確認になるか（機種・VM に依存しないか）を整理する。
- 実機の値は、その VM の OS ビルド・ロケール・DPI・モニタ構成の値であって、他の Windows でも同じとは限らない（`env.md` に記録された条件つきで読む）。

## 限界（最初に知っておくこと）

- 実機の測定は A2b の**十分条件ではない**。コンピュータ名・ユーザー名・画面解像度・モニタ構成・インストール済みフォントなどは、測っても仕様の確認にならない（環境の定義を決める話）。道具は、それらを「参考（reference）」として分けて記録する。
- 測れるのは、トレースにある呼び出しの形と、その周辺を面で標本にしたもの（probe）だけ。ゲストが戻り値でどの枝に進んだかは、この方法では分からない。
- VM は Hyper-V 上の仮想環境で、物理ハードウェアの画面・GPU・フォントとは違いうる。1 つの OS ビルド・1 つのロケールの値である。
- Windows 上のエージェント（LLM）は手順を読み違えることがある。結果を機械的に検証できる形（行数、ハッシュ、compare の再実行）にしてあるのはそのため。
- リポジトリ `signature-studio/signature-studio-develop` は **private**（2026-10-28 に `gh repo view signature-studio/signature-studio-develop --json isPrivate,visibility` で `isPrivate: true`、`visibility: PRIVATE` を確認）。private であっても、Issue とコミットに個人情報を入れさせない作りにしてある。公開範囲を広げる場合は、先に既存の Issue ・コミットと `worklog/<n>/` の内容を PM が確認すること。

## 次の Issue を作るとき

1. 道具を `tools/<名前>/` に置く（README に、呼ぶ API／コマンドの一覧、副作用、限界を明記。wine や Linux で煙テストできるなら実施し、その記録は「実機の値ではない」と書く）。
2. `issue-NNNN-<名前>.md` に、目的（背景 3〜5 行）／前提／手順（PowerShell の具体的なコマンド）／成功条件／禁止事項／提出物（Issue コメント＋`worklog/<n>/` にコミットするもの）／`<ISSUE>` の扱い、を書く。PowerShell のコードブロックは、PowerShell 7 の構文解析にかけて誤りが無いことを確認する。
3. 提出物には必ず、環境の記録、作業メモ、アーティファクトの SHA-256 を含める。

## Issue 一覧

| 下書き | 内容 | 状態 |
|---|---|---|
| `issue-0001-api-replay.md` | 実機 Windows で Windows API 171 種のうち 153 種の応答を測る（`tools/win-replay/`） | **起票済み**: [Issue #1](https://github.com/signature-studio/signature-studio-develop/issues/1)（2026-10-28 起票、OPEN、ラベルなし、コメント 0 件）。VM のエージェントの結果は未受領 |

- 起票の記録は `issue-0001-posted.md`。実際の Issue の本文は、このファイルの先頭の HTML コメントを除き、`<ISSUE>` を `1` に置換したもの。**このファイル（`issue-0001-api-replay.md`）は `<ISSUE>` を含む下書きのまま変えていない。**
- 確認（2026-10-28）: `gh issue view 1 --repo signature-studio/signature-studio-develop --json state,title,url,comments,createdAt` で OPEN、コメント 0 件、作成 2026-10-28T11:34:03Z。`git ls-remote https://github.com/signature-studio/signature-studio-develop.git main` は `9aa3615d525f7e93b6c4e3e0feaf76f018bdbdde`（確認時点のローカルの HEAD と同じ）。`gh api repos/signature-studio/signature-studio-develop/contents/tools/win-replay` と `…/contents/docs/windows-delegation` の応答に、`replay.c`・`cases.tsv`・`compare.py` など（前者 11 件）と `README.md`・`issue-0001-api-replay.md`・`issue-0001-posted.md`（後者 3 件）がある。したがって、`issue-0001-posted.md` の「起票時点のリモートの状態」（`tools/win-replay/` がリモートの main に無い）は、その後の push で解消している。`git ls-remote --heads` のブランチは `main` だけで、`worklog-issue-1` は無い。
