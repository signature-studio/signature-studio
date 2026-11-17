# 0400: deploy-engine.yml と deploy-worker.yml を 1 本の deploy.yml(Worker の配信だけ。100 行・1 行 80 文字・run の直書き)に統合し、旧 2 本を廃止する
分類: 整備(重複する 2 本の統合)。Worker の配信の形(upload → 100% → 確かめ → 戻し)は既存の決定 0379・0389 の形を保つ。
撤回条件: (1) 依頼者が別の構成(bundle の作成・R2 への載せ・root の差し替えを deploy.yml に戻す、など)を指示したとき。(2) deploy.yml の run が、依頼者の指示の 100 行・80 文字・スクリプト禁止のどれかに当たると判明したとき(その時点で構成を見直す)。
日付: 2026-11-16   ROADMAP: R6-5・R6-7 の行に注(文の追記)。R6-19・R6-20 は doing・blocked の状態を変えない   一覧: なし(整備)   コミット: aba137b(deploy.yml の追加と旧 2 本の削除)
番号の理由: `docs/decisions/` の最後は 0399(2026-11-16 の ls)。0400 が最初の空き。

## 0 検索した範囲と件数(設計の前の確認)

| 範囲 | 検索・確認 | 件数と ID |
|---|---|---|
| `.github/workflows/`(入力) | ファイル数 | 3(agent.yml・deploy-engine.yml・deploy-worker.yml)。deploy-engine.yml は 1014 行・48 段(bundle ジョブ)、deploy-worker.yml は 432 行・15 段(deploy ジョブ) |
| `ci/`(入力) | 全ファイル | 23 件(`find ci -type f`)。読んだ: extract-cache.sh・wrangler_config.py・verified-bundles.tsv・versions.env |
| `77ab7773^:app/scripts/build-engine-bundle.sh` | 先頭の使い方 | 456 行の 9 段(gen・resplit・wasm・hybn0・weblink・wasmopt・hybrid・seed・bundle) |
| `app/wrangler.jsonc` | 全行 | コメントあり(`//`)。`main`・`assets`・`r2_buckets`(2)・`vars`(空の placeholder 2)・`workers_dev`・`preview_urls` |
| `scripts/` `tests/` `.github/` `ci/` `app/scripts/` `app/src/` | `grep -rln "deploy-engine\|deploy-worker"` | 旧 2 本を名指すファイルは、コメントを除いて 0 件(ci/extract-cache.sh・ci/stage-keys.sh は注釈の言及のみ) |
| `docs/` | `grep -rln "deploy-engine\|deploy-worker"` | 17 ファイル(決定 0379・0381・0386・0389・0391・0395・0398、ROADMAP、docs/requests/0005・0008、docs/failures/0007・0012・0014、docs/ops/lessons.md ほか)。バッククォートの実在しないパスで `scripts/check-consistency.sh` が指摘したのは 13 件 |
| 依頼者の指示(原文。main 経由) | 2026-11-16 の 2 通 | 「deploy-engine.yml は廃止し、deploy-worker.yml に統合し、新たに deploy.yml を作成してください。deploy.yml は100行以内に収めてください。動作する最小限の構成にすること。」(1 通目)。「wasmをビルドし、wrangler deployコマンドを1行実行するだけのワークフローに、本来20行程度の制限で終わるはずです。100行という時点でだいぶ譲歩しています。」「当然R2へのアップロードも維持します。」(2 通目。main 経由で届いた原文) |
| 依頼者の発言の過去の記録(`~/work/handover/setup/S16-intent/catalog.md`) | 未確認 | この作業では読んでいない(時間の制約。未確認) |

制約(依頼者の指示。main 経由。決定の文に含める): deploy.yml は 100 行以内・各行 80 文字以内。シェルスクリプトを実行しない(`bash ci/…sh`・`sh`・`source`・`./…`、app/scripts の .sh、ci/lib の .py の実行を含む)。`run` の中に直接書いたコマンドと、`npm run` で既存の package.json のスクリプト(build・test・test:worker)を呼ぶことだけ可。再利用ワークフロー(workflow_call)は新設しない。composite action・ローカル action・新しい ci の .sh/.py/.mjs・Makefile は使わない。公開 action は actions/checkout・actions/setup-node・cloudflare/wrangler-action(deploy-worker.yml と同じ SHA `953926a2e2182532811c01a25e53647d93bf07c0`)だけ。

## 1 入れた物と、置かなかった物(事実)
出典: 依頼者の指示(main 経由。発言 ID は未確認)と、旧の段の出どころ archive/ci-old/deploy.yml(bundle の段・R2 の載せ・root の段)。

| 旧の段 | 行き先 | 件数・根拠 |
|---|---|---|
| deploy-worker.yml: build・test・test:worker・依存関係 | deploy.yml の run の直書き | 4 段(npm ci・npm run build・npm test・npm run test:worker)。`run` は既存の package.json のスクリプト名だけ |
| deploy-worker.yml: 現在 100% の版を読む・設定を作る | deploy.yml の「現在の版」段(curl・jq・インラインの node) | 1 段。読む物: アカウント 1 件、100% の版 1 件、版の resources.bindings(plain_text・r2_bucket・assets 以外は失敗)。値は ::add-mask:: で伏せ、接頭辞(8 字以上)も伏せる。配信設定は app/wrangler.jsonc(コメント除去)に r2 の bucket_name と vars(今の版の plain_text)を写して app/wrangler.deploy.json に書く |
| deploy-worker.yml: versions upload・版の ID・versions deploy(100%)・確かめ・戻し | cloudflare/wrangler-action(upload・deploy・戻し)、jq(版の ID と確かめ) | 5 段(upload・版の ID・100%・確かめ・戻し)。戻しは確かめが失敗したときと、upload の後に失敗したときに直前の版を 100% にする |
| deploy-worker.yml: 公開ホスト名を決める・probe(配信の後の Access の確かめ) | 置かない | 決定 0398 で置き換え済み(Secret PUBLIC_HOSTNAME は依頼者が登録しない。Access の probe は 0398 の決定 1・3 で飛ばす分岐)。deploy.yml にも無い。依頼者の指示にも無い |
| deploy-worker.yml: `77ab7773^:ci/lib/wrangler_config.py` の検査(ACCESS_TEAM_DOMAIN の形・ACCESS_AUD の 64 桁・PERF と ASSETS の名が違うこと) | 置かない(一部だけ写す) | deploy.yml は値を今の版から写すだけで、形の検査は持たない。空・無し・route の有無・workers_dev・preview_urls の検査は持つ。形の検査の欠落は未確認の危険として残る(下の 4) |
| deploy-engine.yml: bundle の鍵・アカウント ID の取得・抽出の層(installer の取得・照合・抽出・R2 の抽出キャッシュ)・emsdk・rust・cargo fetch | 置かない | 依頼者の指示の「スクリプト禁止」と「100 行」に当たる(抽出の層は composite の extract-layer を使う形でしか書けず、迂回に当たる) |
| deploy-engine.yml: 上流 Blend2D の wasm ビルド・9 段の bundle(build-engine-bundle.sh)・段のキャッシュ(stage-cache)・bundle の検査 | 置かない | build-engine-bundle.sh は 456 行の shell で、run の直書きでは 100 行に収まらない(スクリプト禁止に当たる)。段のキャッシュは composite の stage-cache を使う形でしか書けない |
| deploy-engine.yml: R2 `fl-web-assets` への 5 件の載せ・1 件ずつの読み直し・既存の prefix の確かめ | 置かない | bundle が無いと載せる物が無いため、bundle の段と同じ run に置く必要がある(依頼者の指示)。その bundle の段が入らないので載せも入らない。ディレクトリごとの `aws s3 sync` の 2〜3 行は、この時点で載せる物を作れないので入れない |
| deploy-engine.yml: root の manifest.json の差し替え(一覧 `77ab7773^:ci/verified-bundles.tsv` の sha256 に在るときだけ・退避つき・戻し) | 置かない | 差し替えの元の bundle(prefix)が作れないため、入れても動かせない。本番の既定の入口は、この commit では変えていない(deploy.yml は root を読まない・書かない) |
| deploy-engine.yml: 結果の表(失敗しても出す) | 置かない | 件数 1 段。ステップ一覧は GitHub の run が出すので、run の表は evidence/R6-21.md で代える |

件数: 旧 2 本の計 63 段(deploy-engine 48 段 + deploy-worker 15 段)。deploy.yml に残る段は 12 段(checkout・setup-node・npm ci・build・test・test:worker・現在の版・新しい版・100%・版の ID・確かめ・戻し)。置かなかった物は、bundle・R2・root・抽出・ステップの表・probe である(上の表)。

## 2 決定

1. 起動は `.github/workflows/deploy.yml`(`on: push`、`main`、`app/**` と自分自身の変更)。`permissions: contents: read`、`concurrency: deploy`(取り消さない)、`WRANGLER_SEND_METRICS=false`、`timeout-minutes: 40`、`ubuntu-24.04`。分類: 整備。根拠: 旧 deploy-worker.yml の同じ設定。
2. 配信の形(版の upload → 100% → 確かめ → 失敗なら直前の版へ戻す)は旧 deploy-worker.yml と同じ。分類: 整備。根拠: 決定 0379・0389。
3. 公開ホスト名・probe は入れない。分類: 整備。根拠: 決定 0398。
4. 配信設定は `77ab7773^:ci/lib/wrangler_config.py` を使わず、run の直書きの node で作る(スクリプト禁止のため)。今 100% の版の bindings と vars を写す(値は伏せて出す)。分類: 整備。根拠: 依頼者の指示(スクリプト禁止)。
5. 旧 2 本(deploy-engine.yml・deploy-worker.yml)は削除する。分類: 依頼者の指示。根拠: 依頼者の指示の原文(上)。
6. `ci/` の使わなくなった物(stage-cache.sh・stage-keys.sh・test-stage-cache.sh・test-stage-keys.sh・lib/wrangler_config.py・verified-bundles.tsv・.github/actions/stage-cache)は、この commit では消さない(提案)。根拠: ci/test-stage-*.sh が参照する。agent.yml は extract-layer と extract-cache.sh を使うので残す。

## 3 検査

- `wc -l .github/workflows/deploy.yml` = 100、`wc -L` = 79(commit aba137b の内容。HEAD で `git show HEAD:.github/workflows/deploy.yml | wc -l`)。
- `python3 -c 'import yaml; yaml.safe_load(…)'` が通り、steps が 12。
- `scripts/scan-forbidden.sh .github/workflows/deploy.yml` = 0 件。
- 実行: run 43836818321(push。headSha 80cd617 の時点。deploy.yml は aba137b で入った)。結論 success。ステップの結論は evidence/R6-21.md。
- `scripts/check-consistency.sh`: 13 件(deploy 系の旧パスへの言及)。直した後の件数は evidence/R6-21.md。

## 4 未確認

- 本番の root(既定の入口)が今どの bundle を指しているか。この commit は R2 を読まない。未確認。
- 配信の後、公開の入口が Access の背後にあるか(0398 と同じ。確かめていない)。
- 今の版の ACCESS の値の形(0379 の検査を外したため)。写した値は今の版と同じ。形は確かめていない。
- 戻し(rollback)の後の 100% の確かめは持たない(戻しの後の check は無い)。

## 5 追記(置き換え先の参照)

- 決定 0379・0381・0389 には 1 行の追記(置き換え先: 0400)。
- 0391・0395・0398 の「今の部品: deploy-engine.yml」は履歴として残す。参照の追記は行わない(本文の決定は変えない)。
