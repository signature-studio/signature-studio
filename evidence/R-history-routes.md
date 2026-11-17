実エンジン: 実行なし(公開ホストを開いていない。curl・headless Chrome・wrangler は未実行)
代役・再生・推測: なし
未確認: 履歴と現在のツリーにある値が実在のホストかどうか(値を読んでいない。語の一致は実在の判定の根拠にならない)

# R-history-routes: git 履歴にあるホスト名の範囲(値は読まず・書かない)

日付: 2026-11-13(HEAD f52a5fa の上で実施。履歴の書き換えはしていない)
依頼: Issue `docs/requests/0003-history-routes-value.md`(番号の理由は §5)
方法: 件数・SHA・日付・件名・ファイル名だけを出した。`git log -p`・`git diff`・`git show <commit> -- <file>` は実行していない。`git show <commit>:<file>` は grep と wc へのパイプでだけ使い、出力は件数のみ。

## 1 範囲(SHA・日付・件名・ファイル名。値なし)

| 検索 | 件数 | commit | ファイル |
|---|---|---|---|
| `-G` 値の形(`routes` の値。検索式は依頼どおり) in `app/wrangler.jsonc ci/ docs/` | 2 | 14268c2 2026-11-10 「lift: import source from the previous tree」 / a70e4ef 2026-11-10 「docs: track docs/archive (research excerpts are public)」 | app/wrangler.jsonc / docs/archive/decisions/0135-frontend-app.md・0217-ci-deploy.md |
| `-S 'custom_domain'` in `app/wrangler.jsonc` | 1 | 14268c2 | app/wrangler.jsonc |
| `app/wrangler.jsonc` を変えた commit の総数 | 1 | 14268c2 のみ | — |

- R6-2a の「git 履歴に 1 件」と、件数の上では一致する(値の形を持つ行は `app/wrangler.jsonc` の 1 件 = 14268c2)。ただし値の形を持つ行は、a70e4ef の `docs/archive/decisions/0135-frontend-app.md` にも 1 件ある(`0217-ci-deploy.md` は 0 件。`-G` で当たったのは `routes` の語を含む行で、値の形ではない)。

## 2 値の有無(件数のみ。語の一致 `example|your-domain` で「プレースホルダの語を含む」を判定)

| 対象 | 値の形の行(件数) | うちプレースホルダの語を含まない | 備考 |
|---|---|---|---|
| 14268c2:app/wrangler.jsonc | 1 | 0 | `custom_domain` の行は 1 件で、コメント行(非コメントは 0) |
| a70e4ef:docs/archive/decisions/0135-frontend-app.md | 1 | 0 | — |
| a70e4ef:docs/archive/decisions/0217-ci-deploy.md | 0 | 0 | — |
| HEAD:app/wrangler.jsonc | 1 | 0 | — |

- 判定: 履歴の値の形は、語の一致によれば全部プレースホルダの語を含む。ただし語の一致は実在の判定の根拠にならない。**実在かどうかは判別できない(値を読まないため)**。

## 3 広い形(現在のツリーと履歴。検索式は R2・Access・Workers の 3 種の接尾辞。接尾辞はここに書かない)

- HEAD(main)のツリー: 当たったファイル 15 件、区別できる値の形 5 種。語の一致(`changeme|CF-VALUE|example|your-domain`)を含む形は 0 種、含まない形は 5 種。**判別できない(値を読まないため)**。
- 当たったファイル(名前のみ): `.github/workflows/cloudflare-check.yml`、`.github/workflows/engine-bundle.yml`、`app/scripts/perf-log-summary.mjs`、`app/test/helpers/wrangler.mjs`、`app/test/unit/access.test.ts`、`app/wrangler.jsonc`、`archive/ci-old/ci/actions/r2-cache/action.yml`、`archive/ci-old/ci/lib/upload_plan.py`、`archive/ci-old/ci/lib/wrangler_config.py`、`archive/ci-old/ci/stage-deploy.sh`、`archive/ci-old/ci/stage-extract.sh`、`archive/ci-old/ci/test-wrangler-config.sh`、`docs/archive/decisions/0217-ci-deploy.md`、`docs/archive/investigations/ci1/tables/runs-on-cache-r2.md`、`evidence/R8-2.md`
- 同じ広い検索で当たった履歴の commit(`-G`): b4a88a8 2026-11-13(ci: cloudflare-check に読み取りの確認を足す…)・e78f667 2026-11-13(ci: 配信用 bundle のワークフロー…)・978e443 2026-11-12(R8-2 record: …)・f76237a 2026-11-12(Add a read-only workflow that checks the … secrets and variable)・a68f38d 2026-11-10(evidence, patches and archive: import from the previous tree)・a70e4ef・14268c2。上の 6 件(a68f38d を除く)は HEAD の祖先(`git merge-base --is-ancestor`)。

## 4 R6-2a との食い違い(記録のみ。どちらが正しいかは判定しない)

- R6-2a は「追跡ファイル全体に、伏せていない形は 0 件」と書く。その検索式は「R2 の識別子・Worker のホストの形」で、今回の広い式とは違う。
- 今回の広い式では、現在のツリーに 15 ファイル・5 種の形がある。R6-2a の「0 件」は、今回の式では成り立たない。R6-2a の式が狭かったためか、今回の式が広すぎるためかは未確認。

## 5 Issue の番号

- `docs/ROADMAP.md` の R4-5 は `docs/requests/0002-owner-machine-measurement.md` を予定している(予約)。`ls docs/requests` は `0001-cloudflare-actions-secrets.md` と `TEMPLATE.md` のみ。
- 予約されているため、この Issue は `0003-history-routes-value.md`。

## 6 やっていない物

- 履歴の書き換え(filter-repo・rebase・force push・amend)。
- 値の表示・読み取り(§2・§3 は件数と名前のみ)。
- 公開ホストへの接続、wrangler・cf の実行、ブラウザ、cargo、ビルド。
- `docs/ROADMAP.md`・`docs/STATE.md` の変更。
