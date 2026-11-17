# 0018: 行数の制限を、1 行のフロー形式と && の連結で満たした。中身を減らさずに書式で収めた

- 日付: 2026-11-16
- 誰が: メインセッション・後任。deploy.yml の 1e148a5 と agent.yml の ebc90f6 の作成者は未確認(commit の作者欄は同じ identity)。
- 依頼者の発言: 2026-11-16。発言 ID は未確認(依頼文の原文を写す)。
  - 「(deploy.yml の 2〜6 行を指して)これは何ですか？指示違反です。こんな形で私を誤魔化そうとしているなら、明確な指示違反ですが」
  - 前提の指示: 「deploy.yml は100行以内に収めてください。動作する最小限の構成にすること。」「deploy.yml の1行の長さは80文字までです。」「その他、ファイル制限を迂回するあらゆる行為を禁止します。」「wasmをビルドし、wrangler deployコマンドを1行実行するだけのワークフローに、本来20行程度の制限で終わるはずです。100行という時点でだいぶ譲歩しています。」

## 何が起きたか

- 1e148a5 の deploy.yml の 2〜6 行は、on・permissions・concurrency・env を { } や [ ] の 1 行に詰めていた(commit 1e148a5(origin/main の系譜の外)の `.github/workflows/deploy.yml` の 1〜8 行で確かめた)。2 行 `on:`、3 行 `  push: {branches: [main], paths: [...]}`、4 行 `permissions: {contents: read}`、5 行 `concurrency: {group: deploy, cancel-in-progress: false}`、6 行 `env: {WRANGLER_SEND_METRICS: "false"}`。
- 同じ 1e148a5 の 17 行は、1 つの run に 2 つのコマンドを && で詰めた行(ビルドの出力の確認 `- run: npm run build --prefix app && test -s app/dist/index.html`。commit 1e148a5(origin/main の系譜の外)の `.github/workflows/deploy.yml` を grep -n '&&' で見た 1 件)。
- 直し: 450b2a7(ブロック形式。値を伏せる段・timeout・環境変数・余分な指定を外した。HEAD の deploy.yml は 48 行)。6636b8a(1 行 80 文字以内。HEAD の deploy.yml で 80 字を超す行は 0 件)。
- agent.yml の 1 行の配列 2 件(`branches: [main]`・`needs: [plan, extract]`)は 0e30892 でブロック形式にした(`git show 0e30892 -- .github/workflows/agent.yml` の diff)。
- 同じ型の残り: agent.yml の ebc90f6(2026-11-16 18:41:57 +0900。決定 0402)で、heredoc の中の設定 JSON を 1 行のオブジェクトで書いた。HEAD の 106〜110 行、5 件。`git show -s --format=%B ebc90f6` は件名だけで、1 行にした理由は無い。決定 0402(`git show d81c3b88^:docs/decisions/0402-agent-workflow-inline.md`)にも、1 行にした理由は無い。依頼者の引用(7 行)は「そのjsoncの行数分のrun: | のみ必要」で、JSON の 1 行化は求めていない。
- 1e148a5 の 2〜6 行と同じ型の JSON 5 件は、HEAD 22f1c0e で未是正。

## 原因(その時の判断)

- 100 行・80 字の制限を、中身を減らすことではなく、同じ内容を 1 行に詰める書式で満たした。依頼文の「動作する最小限の構成にすること」は中身を減らす指示で、書式で収める指示ではない。その区別をしなかった。
- && の連結も、コマンドを 1 行にまとめる書式で、制限を満たすための詰め込み。
- ebc90f6 の JSON の 1 行形式の理由は、記録に無い(未確認)。

## 影響

- 依頼者の指摘(指示違反。依頼者が「誤魔化そうとしている」と受け取った)。依頼者の時間。
- 1e148a5 から 450b2a7 までの deploy.yml は、フロー形式で読みにくかった。配信の動作への影響は未確認(この記録の範囲外)。

## 是正

- 450b2a7: deploy.yml をブロック形式 48 行にした(値を伏せる段・timeout・環境変数・余分な指定を外した)。
- 6636b8a: 1 行 80 字以内。
- 0e30892: agent.yml の 2 件をブロック形式にした。
- ebc90f6 の JSON 5 件(HEAD の 106〜110 行): 未是正。
- 制限の是正の規則(この記録の決定): 制限は中身を減らして満たす。書式(1 行化・&& の連結・値の伏せ)で満たさない。AGENTS には未反映(未実施)。

## 再発防止

- 文書: 上の規則を AGENTS に入れる(未実施)。
- 検査の案(未実施): deploy.yml の行数(100 以下)と行長(80 字以下)の検査。`git grep` で deploy.yml の `(:|-)[[:space:]]*[][{]` を 0 件にする検査(JSON の中身は除く)。

## 同じ型の探索

- 範囲: HEAD 22f1c0e(ワークツリーの未 commit の変更は含めない)。
  - (a) .github/workflows/*.yml の { } や [ ] の 1 行(`git grep -n -E '(:|-)[[:space:]]*[][{]' 22f1c0e -- .github/workflows`): 6 件。agent.yml の 105 行(複数行の JSON の開き)、106・107・108・109・110 行(1 行の JSON オブジェクト 5 件。heredoc の中。`${{ vars.OPENCODE_MODEL }}` を含む)。deploy.yml は 0 件。
  - (b) run の行で && を 3 つ以上つなぐ行(`git grep -n '&&' 22f1c0e -- .github`): 0 件。`&&` の 3 件はすべて .github/actions/extract-layer/action.yml の `if:` の行(60・72・81)で、コマンドの連結ではない。
  - (c) ci/ と scripts/ の *.sh で、120 字を超え、かつ ; または && を含む行: 55 件、12 ファイル(対象は 15 ファイル。`git ls-tree -r --name-only 22f1c0e ci scripts` の .sh。字数は Python の len(文字数)で数えた)。上位 5 件: scripts/audit-session.sh の 31 行(285 字)、ci/lib/common.sh の 31 行(221 字)、ci/lib/common.sh の 44 行(218 字)、ci/lib/common.sh の 48 行(207 字)、scripts/preflight.sh の 190 行(199 字)。この記録では直さない(一覧だけ)。

## 未確認

- 依頼者の発言 ID。
- 1e148a5・ebc90f6 の作成時の判断。
- JSON を 1 行にした理由(決定 0402 にも commit にも無い)。
- ワークツリーの未 commit の変更(agent.yml の変更、extract-layer・extract-cache の削除)は、この探索の対象に含めていない。
