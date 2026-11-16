# 0379: Worker fl-studio-web の配信を GitHub Actions(deploy-worker)で行う。版の bindings は今 100% の版から読み、versions upload と versions deploy で出す
分類: (b API 拡充)。新しい処理は「版の作り方」だけ(今 100% の版の bindings と vars を読み、同じ値で新しい版を作る)。除去(a)は 1 件(追記を参照)。(c) の対象外。
撤回条件: 版の応答の bindings の形が読めず、修正を重ねても直らない(push 8 回の上限で失敗)ときは、この方式を捨てて依頼に切り替える。配信後に公開の入口(カスタムドメイン)が外れ、戻せないことが分かったときも見直す。
日付: 2026-11-16   ROADMAP: 依頼 ID R6-5(ROADMAP に R6-5 の行は無い。ROADMAP は変えない)   一覧: なし   コミット: この文書と 旧 deploy 系の workflow(deploy-worker。決定 0400 で deploy.yml に統合) と `ci/lib/wrangler_config.py` を含む commit(push で deploy-worker が動く)

## 決定
- 決定 1: 配信は 旧 deploy 系の workflow(deploy-worker。決定 0400 で deploy.yml に統合) で行う(`on: push`、`main`、`app/**` と自分自身の変更で動く)。根拠: 0001 の Secrets(`CLOUDFLARE_API_TOKEN` は Workers Scripts: Edit だけ)で配信できる経路がこれだけ。
- 決定 2: Worker の bindings と vars は、今 100% の版から実行時に読み、同じ値で新しい版を作る。リポジトリに本番の値を置かない(公開リポジトリ。0001)。根拠: `app/wrangler.jsonc` の値は placeholder で、空の vars のまま配信すると Worker は全リクエストに 403 を返す(`app/wrangler.jsonc`、`ci/lib/wrangler_config.py`)。
- 決定 3: 版は `wrangler versions upload` で作り、100% への切り替えは `wrangler versions deploy <版>@100%` で行う。`wrangler deploy` は使わない。根拠: 0217。`wrangler deploy` はカスタムドメインの更新で、Workers Scripts: Edit だけのトークンではゾーンを引けず失敗した。
- 決定 4: 書き込みより前に検査を全部行う。読み取り、bindings の型と名前(この workflow が作らない binding が 1 つでもあれば止める。新しい版で失われるため)、値の有無、配信設定の検査(route は無し、workers_dev と preview_urls は false)。どれか失敗なら何も書かない。
- 決定 5: 切り替え後、最新の deployment が新しい版 100% であることを確かめる。違えば直前の版(100% だった版)へ戻し、失敗にする。
- 決定 6: build と test は、配信の前の検査として同じ workflow に置く。ビルドやテストだけの workflow は作らない(AGENTS.md §4)。
- 追記(分類 (a) の除去。main の決定): カスタムドメインの件数の検査を削除した(`GET /accounts/{id}/workers/domains`、件数の比較、0 件なら止める分岐)。根拠: versions upload と versions deploy はカスタムドメインを変えないので、検査に効果がない。実行 43784397668(commit d22d557)で、この検査が HTTP 403(エラーの番号 10000)で失敗した。失敗の段は読み取りだけで、upload と deploy は skipped、Cloudflare への書き込みは無かった。

## 実測(この機械。コマンドと出力)
- `npm ci --prefix app --ignore-scripts --no-audit --no-fund`: `added 74 packages in 3s`、rc 0。
- `npm run build --prefix app`: rc 0。`app/dist/index.html` あり。
- `npm test --prefix app`(vitest): rc 0。`Test Files 65 passed | 1 skipped (66)`、`Tests 1290 passed | 7 skipped (1297)`。
- `npm run test:worker`: 手元では未実行(`wrangler dev --local` を起動するため。wrangler は手元で実行しない)。CI の段で実行する。
- `cf.py`(読み取りと確かめ)を偽の API に対して 13 件テスト: 正常、辞書形式の bindings、想定外の binding、欠落、値が空、アカウント 2 件、版 2 件、403、bindings 無し、確かめの 4 件。すべて期待どおりの rc。値(偽のアカウント ID・アクセス設定・バケット名)がログに出ないことも確かめた。
- 切り替えの段を偽の npx と偽の cf.py で 5 通り: 成功、切り替えの失敗 → 戻し、確かめの失敗 → 戻し、戻しの確かめの失敗、upload の失敗。すべて期待どおり。
- 実行 43784397668(d22d557): 段の結論は build・unit・worker が success、「今 100% の版を読む」が failure(HTTP 403、エラーの番号 10000)。読み取った版は `2cb938be-e32d-44e4-8268-1824660956fe`(作成 UTC 2026-11-07T21:40:44.504223Z)。

## 読み(公開ドキュメントの応答の形)
- Get Worker Script Version: `result.resources.bindings`。実行 43784397668 で、`resources` の下に `bindings` があることを確かめた。型は plain_text(値は `text`)と r2_bucket(`bucket_name`)として読めた。
- List Worker Deployments: `result.deployments[0]` が最新で配信中。各項目の `versions[{version_id, percentage}]`。

## 未確認
- Workers Scripts: Edit だけで versions upload と versions deploy が通ること(アセットの upload を含む)。0217 の方式は使い捨ての Worker でしか実測していない。
- カスタムドメインを持つ本番の Worker が、versions upload と versions deploy の後も同じドメインに対応すること。この workflow は確かめない(追記の通り)。依頼者が Access の経由で確かめるまで未確認。
- 公開ホストの応答。Access の配下で、匿名の headless Chrome では開けない。確かめられるのは Actions の出力まで。
- `ci/lib/wrangler_config.py` だけを変えた commit では、この workflow は動かない(paths は依頼どおり)。

## 出典
- `docs/archive/decisions/0217-ci-deploy.md` の冒頭の 3 と §5(wrangler deploy の失敗、versions の方式、順序)。
- `docs/requests/0001-cloudflare-actions-secrets.md`(Secrets の名前と権限。Variable を使わない理由)。
- `archive/ci-old/ci/stage-deploy.sh` の worker 段と `archive/ci-old/ci/lib/wrangler_config.py`(後者は `ci/lib/wrangler_config.py` に同一のまま置いた)。
- `evidence/R6-1b.md` §2(2026-11-07 作成の 100% の版。この版から更新が無い)。

追記(0400 で置き換え): 配信は deploy.yml に統合した。旧 deploy-worker.yml は削除(決定 0400)。
