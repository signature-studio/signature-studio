# 0403: 配信の設定をツリーの app/wrangler.jsonc に書き、deploy.yml は wrangler deploy を 1 回実行するだけにする(設定を実行時に作るステップ・確かめ・戻し・versions upload を外す)
分類: 整備(依頼者の指示)。例外: scripts/forbidden-allowlist.txt の 1 行(app/wrangler.jsonc の account_id の行だけ)。
撤回条件: (1) 依頼者が配信の設定を別の場所に置く、または版の確かめ・戻しを求めたとき。(2) 本番の配信が依頼者の期待と違う版を出したとき(その時点で deploy.yml を見直す)。
日付: 2026-11-16   ROADMAP: 変えない   一覧: なし   コミット: この文書・`.github/workflows/deploy.yml`・`app/wrangler.jsonc`・`app/test/helpers/wrangler.mjs`・`scripts/forbidden-allowlist.txt` を含む commit
番号の理由: `docs/decisions/` の最後は 0402(別の作業が作成)。0403 が最初の空き。

## 0 依頼者の原文(main 経由。2026-11-16)

「これは何ですか?指示違反です。デプロイ用のwrangler.jsonなどというものは存在せず、全てのデプロイに必要な記述はツリーに含めることができます。」

読み(決定): 依頼者は、配信の時に設定を作るステップ(今 100% の版の bindings と vars を読んで wrangler.deploy.json を書くステップ)を指示違反と判断した。配信に必要な記述はすべてツリーの app/wrangler.jsonc に置く。依頼者は、この 1 ファイルについて、アカウントの識別子(account_id)・バケット名・Access の設定・カスタムドメインを含めることを許可した。AGENTS.md §8 の識別子の規則は、このファイルについて依頼者の指示で例外。

## 1 検索した範囲と件数(設計の前の確認)

- `.github/workflows/`: 2 本(agent.yml・deploy.yml)。deploy.yml は、前の版で 100 行(20〜57 行が設定を作るステップ、58 行以降が確かめと戻し)。
- `app/wrangler.jsonc`: 変更前は placeholder 2 件(`fl-studio-web-assets-changeme`・`fl-studio-web-perf-changeme`)と空の vars 2 件、コメントに ci/lib/wrangler_config.py(削除済み)の参照 1 件。
- `app/test/`: `changeme` を前提にした箇所 2 件(`app/test/helpers/wrangler.mjs` の BUCKET・PERF_BUCKET の定数)。配信の設定を読む箇所 1 件(`app/test/worker/run.mjs` の「as shipped」の場合。空の team・aud を前提)。
- 値の取得: 値は今の本番の版にしか無い。この作業では、一時の読み取り用 workflow(run 43841035737。結論 success。ログから値を読んだ後に run を削除、workflow の file も削除)で取った。値は commit・決定文書・報告に書いていない。

## 2 置き換える物

| 旧 | 新 |
|---|---|
| deploy.yml の 20〜57 行(今の版の bindings を読み、wrangler.deploy.json を書くステップ) | 削除(旧設計の出典: docs/archive/decisions/0217-ci-deploy.md の「設定は ci/lib/wrangler_config.py が app/wrangler.jsonc + 変数から生成」の形)。設定は app/wrangler.jsonc に写した(account_id・r2_buckets の bucket_name・vars の ACCESS_TEAM_DOMAIN・ACCESS_AUD)。カスタムドメインは 0 件(取得した一覧が空)なので routes は書いていない |
| deploy.yml の「確かめ」と「戻し」、versions upload・版の ID | 削除。依頼者の指示(過度な異常系は不要) |
| ci/lib/wrangler_config.py(削除済み)の参照(app/wrangler.jsonc のコメント) | 削除 |
| placeholder(`changeme`)2 件、空の ACCESS 値 2 件 | 実際の値に直した |

deploy.yml の新しい形: checkout・setup-node(24)・npm ci・build・npm test・npm run test:worker・cloudflare/wrangler-action(`command: deploy`、`workingDirectory: app`、apiToken は secrets.CLOUDFLARE_API_TOKEN。accountId は書かない。account_id は app/wrangler.jsonc が持つ)。

## 3 例外の範囲(依頼者の指示)

- 識別子の規則の例外は、app/wrangler.jsonc の 1 ファイルだけ。scripts/forbidden-allowlist.txt には `cfhex|app/wrangler.jsonc|"account_id": "` の 1 行を足した(その行だけを許す)。
- app/test/helpers/wrangler.mjs は、バケット名を app/wrangler.jsonc から実行時に読む(テストのファイルに識別子を書かない)。
- app/test/helpers/wrangler.mjs の unconfigured の場合は、ACCESS の 2 つの変数を空にして渡す(空の team・aud で 403 を確かめる従来のテストを保つ)。

## 4 検査

- `npm run build --prefix app`: rc 0。
- `npm run test:worker --prefix app`: Tests 49 passed。
- `npm test --prefix app`: Tests 1308 passed、9 skipped。
- `scripts/scan-forbidden.sh` の対象: 0 件。
- 本番の配信の確認は、deploy の run の結果で行う(evidence/R6-21.md は 1 回目の版の記録。この決定の run は別に記録する)。

## 5 未確認

- ACCESS の値の形の検査(旧 wrangler_config.py の 64 桁・形)は無い。値は今の版と同じ。
- 公開の入口が Access の背後にあるか(0398 と同じ。確かめていない)。
- カスタムドメインが 0 件だった理由(今の版が workers/domains に載っていない)は未確認。
- 本番の root(既定の入口)は、この決定で読まず、書いていない。
