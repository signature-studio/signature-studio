# 0379: Worker fl-studio-web の配信を GitHub Actions(deploy-worker)で行う。版の bindings は今 100% の版から読み、versions upload と versions deploy で出す
分類: (b API 拡充)。新しい処理は「版の作り方」だけ(今 100% の版の bindings と vars を読み、同じ値で新しい版を作る)。(a)(c) の対象外。
撤回条件: 版の応答の bindings の形が読めず、修正を重ねても直らない(push 8 回の上限で失敗)ときは、この方式を捨てて依頼に切り替える。配信後に公開の入口(カスタムドメイン)が外れ、戻せないときも見直す。
日付: 2026-11-16   ROADMAP: 依頼 ID R6-5(ROADMAP に R6-5 の行は無い。ROADMAP は変えない)   一覧: なし   コミット: この文書と `.github/workflows/deploy-worker.yml` と `ci/lib/wrangler_config.py` を含む commit(push で deploy-worker が動く)

## 決定
- 決定 1: 配信は `.github/workflows/deploy-worker.yml` で行う(`on: push`、`main`、`app/**` と自分自身の変更で動く)。根拠: 0001 の Secrets(`CLOUDFLARE_API_TOKEN` は Workers Scripts: Edit だけ)で配信できる経路がこれだけ。
- 決定 2: Worker の bindings と vars は、今 100% の版から実行時に読み、同じ値で新しい版を作る。リポジトリに本番の値を置かない(公開リポジトリ。0001)。根拠: `app/wrangler.jsonc` の値は placeholder で、空の vars のまま配信すると Worker は全リクエストに 403 を返す(`app/wrangler.jsonc`、`ci/lib/wrangler_config.py`)。
- 決定 3: 版は `wrangler versions upload` で作り、100% への切り替えは `wrangler versions deploy <版>@100%` で行う。`wrangler deploy` は使わない。根拠: 0217。`wrangler deploy` はカスタムドメインの更新で、Workers Scripts: Edit だけのトークンではゾーンを引けず失敗した。
- 決定 4: 書き込みより前に検査を全部行う。読み取り、bindings の型と名前(この workflow が作らない binding が 1 つでもあれば止める。新しい版で失われるため)、値の有無、配信設定の検査(route は無し、workers_dev と preview_urls は false)、カスタムドメインが 1 件以上。どれか失敗なら何も書かない。
- 決定 5: 切り替え後、最新の deployment が新しい版 100% で、カスタムドメインの数が前と同じことを確かめる。違えば直前の版(100% だった版)へ戻し、失敗にする。
- 決定 6: build と test は、配信の前の検査として同じ workflow に置く。ビルドやテストだけの workflow は作らない(AGENTS.md §4)。

## 実測(この機械。コマンドと出力)
- `npm ci --prefix app --ignore-scripts --no-audit --no-fund`: `added 74 packages in 3s`、rc 0。
- `npm run build --prefix app`: rc 0。`app/dist/index.html` あり。
- `npm test --prefix app`(vitest): rc 0。`Test Files 65 passed | 1 skipped (66)`、`Tests 1290 passed | 7 skipped (1297)`。
- `npm run test:worker`: 手元では未実行(`wrangler dev --local` を起動するため。wrangler は手元で実行しない)。CI の段で実行する。
- `cf.py`(読み取りと確かめ)を偽の API に対して 14 件テスト: 正常、辞書形式の bindings、想定外の binding、欠落、値が空、アカウント 2 件、版 2 件、403、bindings 無し、ドメイン 0 件、確かめの 4 件。すべて期待どおりの rc。値(偽のアカウント ID・アクセス設定・バケット名)がログに出ないことも確かめた。
- 切り替えの段を偽の npx と偽の cf.py で 5 通り: 成功、切り替えの失敗 → 戻し、確かめの失敗 → 戻し、戻しの確かめの失敗、upload の失敗。すべて期待どおり。
- `scripts/scan-forbidden.sh .github/workflows/deploy-worker.yml ci/lib/wrangler_config.py`: `0 findings in 2 file(s)`、rc 0。

## 読み(公開ドキュメントの応答の形)
- Get Worker Script Version: `result.resources.bindings`。ドキュメントの例は `{}` で、型は展開されていない。ワークフローは配列と辞書の両方を読む。型は plain_text(値は `text`)と r2_bucket(`bucket_name`)を想定する。
- List Worker Deployments: `result.deployments[0]` が最新で配信中。各項目の `versions[{version_id, percentage}]`。
- List Worker Domains: `GET /accounts/{id}/workers/domains?service=`。権限は Workers Scripts の Read か Write。

## 未確認
- 版の応答の bindings の実際の形(配列か辞書か、値の欄の名前)。想定と違えば workflow が止まる。実行で確かめる。
- Workers Scripts: Edit だけで versions upload と versions deploy が通ること(アセットの upload を含む)。0217 の方式は使い捨ての Worker でしか実測していない。
- カスタムドメインを持つ本番の Worker で、ルートを含まない設定の versions upload がドメインを変えないこと。workflow が数で確かめる。
- 公開ホストの応答。Access の配下で、匿名の headless Chrome では開けない。確かめられるのは Actions の出力まで。
- `ci/lib/wrangler_config.py` だけを変えた commit では、この workflow は動かない(paths は依頼どおり)。

## 出典
- `docs/archive/decisions/0217-ci-deploy.md` の冒頭の 3 と §5(wrangler deploy の失敗、versions の方式、順序)。
- `docs/requests/0001-cloudflare-actions-secrets.md`(Secrets の名前と権限。Variable を使わない理由)。
- `archive/ci-old/ci/stage-deploy.sh` の worker 段と `archive/ci-old/ci/lib/wrangler_config.py`(後者は `ci/lib/wrangler_config.py` に同一のまま置いた)。
- `evidence/R6-1b.md` §2(2026-11-07 作成の 100% の版。この版から更新が無い)。
