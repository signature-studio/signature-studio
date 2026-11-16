# 0389: deploy-worker の版の upload と 100% の切り替えを cloudflare/wrangler-action で行い、配信の後に公開の入口の Access の確かめ(probe)を戻す。公開ホストはカスタムドメインの一覧から得て、得られないときだけ Secret PUBLIC_HOSTNAME を使う
分類: 整備(R6-19・R6-20。(a)(b)(c) の対象外。ROADMAP の行の分類どおり)
撤回条件: (1) wrangler-action の versions upload・versions deploy が、この Worker の配信の資格情報で通らないと run で確かめたとき、0379 の決定 3 の方式(npx wrangler を直接)に戻す。(2) カスタムドメインの一覧の値と、公開の入口が違うと分かったとき、ホストの決め方を見直す。(3) 依頼者が公開の入口を Access の背後に置かないと言ったとき、probe を外す(0387 の撤回条件 (4) と同じ)。
日付: 2026-11-16   ROADMAP: R6-19・R6-20(状態は doing。done にしない)   一覧: なし(整備)   コミット: この文書・`.github/workflows/deploy-worker.yml`(依頼が出た場合は docs/requests/0008-public-hostname-secret.md も)を含む commit
番号の理由: 0388 は別の作業(agent の provider 変数)が未 commit で先に使っている。0389 が最初の空き(2026-11-16 の ls)。

## 検索した範囲と件数(`docs/ops/intent.md` の「設計の前の確認」)

| 範囲 | 読んだこと | 件数 |
|---|---|---|
| 旧 `archive/ci-old/deploy.yml`(全 1,128 行) | wrangler-action の段(1050 行の SHA。`grep -c wrangler-action` = 1)、probe の段(1091〜1094 行)、`PUBLIC_HOSTNAME` の行(33 行。`grep -c` = 1) | 3 行 |
| 旧 `archive/ci-old/ci/stage-deploy.sh`(全 93 行) | worker 段(47〜61 行)、probe 段(62〜91 行) | 12 件の要求の判定 |
| 現在 `.github/workflows/deploy-worker.yml`(変更前 342 行)と `ci/lib/wrangler_config.py`(全行) | 段の構成、gen 段の route の検査、版の読み取り | — |
| 依頼者の発言(前任の記録) `~/work/handover/evidence/01a-user-messages-full.md`(2,633 行、263 件) | `Access` は 1 行(928 行 = seq 29271 = msg_1982c56e0001L9vl9HVSX0PeIB)。`wrangler` は 2 行(1227 行 = seq 44954 = msg_1a2ccc2c3001RSHoExRGznB5pH の cloudflare/wrangler-action、1232 行 = 同じ発言の wrangler CLI の許可)。`probe\|公開` は 3 行(658・669 行は公開リポジトリの話で、配信の確かめとは無関係) | 263 件の seq |
| 依頼者の発言(catalog)`~/work/handover/setup/S16-intent/catalog.md`(84 行) | Access・公開・wrangler・ホスト・probe・CI の 6 語で 2 行(31・32 行。認証と指示の判定の行。配信の確かめには関係しない) | 配信・Access に関わる行は 0 件 |
| 依頼者の今回の発言 | この作業では再検索していない(evidence/R6-9.md §0 の 76 件の読みを引く) | — |
| Cloudflare の公開 API のドキュメント(webfetch) | Get Worker Script Version の応答(`result.resources` は bindings・script・script_runtime。routes と hostname は無い)。List Worker Domains(権限は Workers Scripts Write か Read。query の `service` で絞れる) | 2 件 |

## 決定

- 決定 1: 版の upload と 100% の切り替え(versions deploy)は cloudflare/wrangler-action で行う。版は `953926a2e2182532811c01a25e53647d93bf07c0 # v4.1.3`(旧 deploy.yml:1050 と同じ版・SHA)。command は `versions upload` と `versions deploy <版>@100%` だけ。`wrangler deploy` は使わない(0379 の決定 3)。分類: 整備。根拠: 依頼者の発言 msg_1a2ccc2c3001RSHoExRGznB5pH(01a:1227「cloudflare/wrangler-actionによるビルド・デプロイ」)。
- 決定 2: wrangler の版はアプリのロック(app/package-lock.json、wrangler 4.156.0)のまま。wranglerVersion は省く。根拠: wrangler-action の `installWrangler` は、入力が無いとき `npx --no-install wrangler --version` で既存の版を使い、入れ直さない(SHA 953926a2 の src/wranglerAction.ts)。分類: 整備。
- 決定 3: 版の ID は wrangler の出力ファイル(`WRANGLER_OUTPUT_FILE_PATH` の ndjson の version-upload 行)から読む。action の deployment-url 出力は使わない。根拠: wrangler 4.156.0 の getOutputFilePath は、`WRANGLER_OUTPUT_FILE_PATH` を `WRANGLER_OUTPUT_FILE_DIRECTORY` より先に使う(app/node_modules/wrangler/wrangler-dist/cli.js:65168-65182)。分類: 整備。
- 決定 4: 切り替えの段の後に check 段を置く(`always()` かつ版の段が成功のとき)。check は、切り替えの結果が success で、cf.py check が新しい版を 100% と見たときだけ成功する。外れたら直前の版へ戻し、失敗にする。確かめ(cf.py check)と戻し(`npx wrangler versions deploy <直前の版>@100%`)は、今の形のまま。切り替えの段に continue-on-error は使わない。根拠: ROADMAP R6-20「直前の版への戻しと、100% の確かめは今の形を残す」。分類: 整備。
- 決定 5: 公開ホストの決め方(分類: 整備。根拠: ROADMAP R6-19)。(1) read 段で、カスタムドメインの一覧 `GET /accounts/{id}/workers/domains?service=fl-studio-web` を読む。件数が 1 で形が正しいときだけ、その hostname を使う。値は `::add-mask::` で伏せ、GITHUB_OUTPUT の `public_host` で渡す(env には出さない。env に出すと gen 段の route の検査が落ちる)。(2) 得られないときだけ、Secret `PUBLIC_HOSTNAME` を使う(旧 deploy.yml:33 と `ci/lib/wrangler_config.py` の入力名と同じ。0001 の 5 件には無い)。(3) どちらも無い・形が違うときは、Cloudflare に書く前(upload の前)に止める。
- 決定 6: 一覧を Secret より先に試す理由: 公開ドキュメント(前掲)は、一覧が Workers Scripts Write で読めると書く。一方、0379 の追記は、同じ権限の Workers Scripts: Edit のトークンで一覧が HTTP 403(エラーの番号 10000)だったと書く(run 43784397668)。2 つは食い違うので、run で確かめる(「実測」)。依頼者の操作(Secret の登録)は、一覧で得られないと確かめた後にだけ頼む。
- 決定 7: probe(配信の後)は、旧 stage-deploy.sh の probe と同じ 12 件(GET/POST/HEAD/Range。`/`・`/static/probe.js`・`/assets/manifest.json`・`/assets/probe/engine.0000000000.wasm`・`/__perf`・`/__perf/summary` など)と、同じ判定(302 で `*.cloudflareaccess.com`、401、403 は OK。他は失敗)。アセットの任意の URL は無し(bundle の manifest はこの workflow が読まない)。ログには転送先の URL を出さない(Access のログイン URL の path に公開ホストが入るため)。根拠: 旧 `archive/ci-old/ci/stage-deploy.sh:62-91`、依頼者の発言 msg_1982c56e0001L9vl9HVSX0PeIB(928 行「CF Accessによって私のみアクセス可能なWorkers…一般公開されることはありません」)。分類: 整備。
- 決定 8: 順序は旧と同じ(配信 → 確かめ → probe)。probe が失敗しても配信は戻さない(旧と同じ。戻しは版の確かめの段だけ)。この点は未確認の論点として残す(「未確認」)。
- 決定 9: 公開ホストは `wrangler.deploy.json` の route に入れない。gen 段の inline の検査は route を拒否する。カスタムドメインは Cloudflare 上に既にあり、versions の操作は触らない(0379 の追記)。分類: 整備。

## 実測(この機械。Cloudflare には触れていない)

- 構文: deploy-worker.yml を `yaml.safe_load` で読み、15 段を得た(出力: 段の id と uses・run の一覧)。read 段の cf.py を取り出して `py_compile` が通った。
- 一覧の分岐(偽の API。`python3 /tmp/opencode/cft/run_cases.py`。リポジトリ外): 1 件・形が正しい → `public_host` を出力(ログは「件数 1(値は伏せた)」)、403 → 出力なし(「HTTP 403、エラーの番号 10000」)、2 件 → 出力なし、形が違う 1 件 → 出力なし、他の service のみ → 件数 0、空 → 件数 0。6 件すべて rc 0 で、期待どおり。
- host 段の分岐(偽の環境変数。bash で実行): 一覧のみ → rc 0、Secret のみ → rc 0、両方無し → rc 1(「Cloudflare へは書いていない」)、形が違う → rc 1、両方 → 一覧を使う(rc 0)。
- probe 段(偽の curl): 401 → 12 件すべて ok、rc 0。200 → OPEN、rc 1。
- wrangler-action の引数: `@actions/exec` の argStringToArray は二重引用符を解く(actions/toolkit packages/exec/src/toolrunner.ts)。`--message "ci <12 文字>"` は 1 つの引数になる。

## 実行(run)

- run 43807505570(push。commit 50e21bc。結論 failure。`gh run view 43807505570 --json conclusion,jobs`)。build・test(vitest)・test:worker・版の読み取り・配信設定の検査は success。「公開ホスト名を決める」の段は failure。一覧は `HTTP 403、エラーの番号 10000`(`gh run view --job <id> --log` の該当行)で、Secret PUBLIC_HOSTNAME も無いので、「Cloudflare へは書いていない」と出して止まった。upload・切り替え・probe は skipped。Cloudflare への書き込みは無い。
- 一覧の 403 は、前回の実行 43784397668 と合わせて 2 回目(同じ権限の資格情報)。公開ドキュメントの権限表と食い違うが、この資格情報では読めないという実測を採る。よって決定 5 の (2) の Secret が要る。依頼は Issue 0008(#8)。
- ログ検査(run 43807505570 の job のログ 662 行。件数): アカウント ID の形 0、メールの形 0、ホスト名の形 0、キーの形 0、セッション ID の形 0。
- evidence: `evidence/R6-19.md`、`evidence/R6-20.md`。

## 未確認

- Secret PUBLIC_HOSTNAME の登録の後の run(upload・切り替え・確かめ・probe の段の結果)。依頼は Issue 0008(#8)。それまで、wrangler-action の versions upload・versions deploy が、この Worker の配信の資格情報で通るかは未確認(0001 の未確認と同じ)。
- probe の 12 件の結果(Access の 302 か 401/403 になるか)。未確認。
- probe が失敗したときに配信を戻すか(旧と同じく戻さない。提案として報告する)。
- action のログに出るアカウント ID・ホスト名の件数(upload と切り替えの段のログで、同じ検査をする)。
- (解決: 一覧の読み取りは、この資格情報では 403。2 回の実行で確かめた。)

## 出典

- `archive/ci-old/deploy.yml`:33(PUBLIC_HOSTNAME)、1049〜1054(wrangler-action の dry run の段。SHA の出典)、1091〜1094(probe の段)。
- `archive/ci-old/ci/stage-deploy.sh`:9-11(worker と probe の説明)、47-61(worker 段)、62-91(probe 段)。
- `ci/lib/wrangler_config.py`:6-8(PUBLIC_HOSTNAME の入力名)、74-88(PUBLIC_HOSTNAME がある時だけ route を付ける)。
- `docs/decisions/0379-deploy-worker-workflow.md`(決定 2・3・4・5 と追記)、`docs/decisions/0387-old-ci-feature-diff.md`(撤回条件 (4))、`evidence/R6-9.md`(G6・G9)、`docs/requests/0001-cloudflare-actions-secrets.md`(Secret の 5 件)。
- 依頼者の発言: msg_1a2ccc2c3001RSHoExRGznB5pH(01a:1216〜1232)、msg_1982c56e0001L9vl9HVSX0PeIB(01a:923〜928)、msg_1e298741b0011ilEMoBvRoXd5n(2026-11-16 13:23:24。既定の指示)。
- wrangler-action の SHA 953926a2e2182532811c01a25e53647d93bf07c0 のソース(action.yml、src/index.ts、src/wranglerAction.ts、src/commandOutputParsing.ts、src/packageManagers.ts。webfetch、2026-11-16)。
- Cloudflare API: Get Worker Script Version(`/accounts/{account_id}/workers/scripts/{script_name}/versions/{version_id}`)、List Worker Domains(`/accounts/{account_id}/workers/domains`)。developers.cloudflare.com の API リファレンス(webfetch、2026-11-16)。
- wrangler 4.156.0 のソース: `app/node_modules/wrangler/wrangler-dist/cli.js`(65168-65182、25357-25362)。
