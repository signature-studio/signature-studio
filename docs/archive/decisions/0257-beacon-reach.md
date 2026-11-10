# beacon1(決定 0257 仮): 依頼者の機械の beacon が本番の R2 に来ているか — 読み取りだけの調査

状態: 第 1 報(20:30 JST)+ beacon2 の追加(20:50、`BEACON2.md` と `beacon2.diff`)。読み取りのみ。書き込み・削除・lifecycle の変更・Worker への POST はしていない。トークン・IP・メールアドレスは表示も保存もしていない(表のメールは `<email>` に伏せた)。生の beacon は 0 件で、repo にも置いていない。

## 1. 結果(実測。本番の R2 そのものを読んだ)
- **`fl-web-cache` の `perf/` に beacon は 0 件。** バケット全体は 402 オブジェクト(2026-11-05 20:18 JST)、**全部 `cache/`(GitHub Actions のキャッシュ)**で、最上位の prefix は `cache/` だけ(`delimiter=/` の一覧)。`perf/`、`perf`、`Perf` の prefix 一覧はどれも空。最終更新の最新は 2026-11-05T11:14Z(`cache/`)。→ `tables/perf-prefix-listing.txt`
- **依頼者の機械由来の beacon: 0 件。最後の beacon の日時と UA: 該当なし**(beacon が 1 件も無い)。この機械の開発用ブラウザ・CI のブラウザ由来も 0 件(本番へは誰も beacon を送っていない、または保存されていない)。
- したがって『最初の依頼者の機械の実測』は**まだ無い**。依頼者の機械の値は 0 件のまま。

## 2. PM の前提の訂正: 『ci1 の確認で 204 保存済み』は本番の R2 ではない
- ci1 の報告 4(`docs/investigations/ci1/verify-afd46e67/REPORT-4-to-PM.md` の表)の『beacon 204 保存済み、same-origin』は、**『手元の再構築で』取った値**(`vite preview` が自分の `perf-log.jsonl` に追記した。開発サーバーの経路)。本番の Worker `fl-studio-web` へ POST した結果ではない。本番の R2 に beacon が無いこととも矛盾しない。

## 3. 本番の Worker は beacon を R2 に置く構えになっている(読み取りで確認)
- `fl-studio-web` の、Cloudflare に残っている全 10 バージョン(最古 2026-11-04T23:03Z、現在 cd4923f5 の 77699bbe)は、すべて `PERF_BUCKET = fl-web-cache`(`ASSETS_BUCKET = fl-web-assets`)。→ `tables/worker-versions-bindings.txt`
- デプロイ済みのスクリプト本体(GET `workers/scripts/fl-studio-web/content/v2`、214,605 B)に `PERF_BUCKET`・`storeBeacon`・`X-Perf-Stored`・`/__perf`・`perf/` がある(文字列の有無だけを確認。本体は表示も保存もしていない)。
- lifecycle は `perf/` を 180 日で失効(`expire-60d` は `cache/` だけ)。**失効で消えた物は無い**(最古のバージョンが 2026-11-04 のため)。

## 4. なぜ 0 件か(確定していない。読み取りだけでは分けられない)
| 候補 | 内容 | この調査で分かること |
|---|---|---|
| H1 | 誰も本番の `/__perf` に POST していない(依頼者が本番を開いていない、開いたがエンジンが first idle 前にタブを閉じた、など) | 排除できない |
| H2 | POST は出たが Access が通さなかった(セッション切れで 302 → ログイン、`fetch` が別オリジンへの redirect で失敗 = status なし)。ページは**黙って捨てる**(status なしでは止まりもしない) | 排除できない。Access のログは権限不足(401)で読めない |
| H3 | 最初の POST に 4xx / 5xx が返り、そのページは以後送らない(`main.ts` の `perfState.disabled`) | 同上 |
| H4 | Worker は 204 を返したが `X-Perf-Stored: no`(R2 への put の失敗) | 過去は不明(Worker の `observability` は false でログが無い)。put 失敗の証拠も無い |
| H5 | 依頼者が見ているのは本番ではない別の URL(その場合 beacon は R2 でなく、その開発サーバーの `perf-log.jsonl`) | 排除できない |
- 読み取りの限界: Cloudflare Access のログ、Workers の分析(GraphQL)は、既存の wrangler ログインの権限では 401 で読めない。`wrangler tail` は現在の接続しか見られない(第 8 節『ライブの tail』を参照)。
- 分けるための手段(いずれも私は実行していない): ① 偽の beacon を本番に POST して `X-Perf-Stored` を見る(書き込みなので禁止)。② Worker の observability を有効にする(設定変更なので禁止)。③ Access の Audit Logs Read の権限のあるトークン(依頼者の作業が要る)。④ 依頼者のブラウザの Network 画面(依頼者には何も頼まない方針)。

## 5. 読み取りに使った手段(PM 指示の『gpu1 と同じ手段』との違い)
- この機械に `cf` CLI は無い(`command not found`)。0224 の `cf r2 objects list` の代わりに、**同じ wrangler ログインの OAuth トークン**を wrangler の設定ファイルから読み、`Authorization` ヘッダにだけ使って Cloudflare の REST を **GET** した(`tools/r2api.py`・`r2list.py`・`worker-has.py`。トークンは表示も保存もしない)。一覧は `GET /accounts/<id>/r2/buckets/fl-web-cache/objects`。オブジェクトの本体は読む必要が無かった(0 件)。`wrangler r2 object get` は使わなかった。
- ほかに使った読み取り: `wrangler r2 bucket info / lifecycle list`、`wrangler deployments list`、`wrangler versions list / view`、`wrangler whoami`、Access・GraphQL の GET/POST(読み取りの問い合わせ。どちらも 401 で終わった)。`wrangler` 自身が OAuth トークンの更新で自分の設定ファイルを書き換えた(wrangler の通常の動作)。
- 『書き込み禁止』に近い操作が 1 つある: GraphQL の問い合わせは HTTP の POST だが、読み取りの問い合わせで、401 で拒否された(何も起きていない)。

## 6. beacon の項目と意味 → `tables/beacon-schema.md`
- ページ側の組み立て(`app/src/perf/beacon.ts`、`browser-info.ts`、`app/shared/perf.ts`、`app/worker/perf-store.ts`)を読んだ。**R2 の beacon に IP アドレス、Access の識別情報(Cf-Access-Authenticated-User-Email)、cookie は入らない設計**(`perfLine` は `{t, remote:"workers", perf}` だけを書く)。含まれるのは UA(300 字)、GL の描画装置名、画面、論理 CPU 数、メモリ、時間などの測定値。実物の beacon が無いので、『入っていない』は**コードを読んだ結論**で、実物では確かめていない。
- beacon は first idle・30 秒ごと(最大 20 回)・終了時に送る。version 3。

## 7. 分類の道具 → `tools/beacon-classify.py`
- UA・OS・headless・GL の描画装置名で、`headless-or-software`(この機械の開発用 / CI のブラウザ)、`real-gpu-windows`(+ `owner-like` = Windows・Firefox・GTX/GeForce/NVIDIA の描画装置名。依頼者の機械の記述に合うだけで、識別ではない)、`real-gpu-other`、`unknown` に分け、クラスごとに first idle、`desk.mode`、`crossOriginIsolated`、`desk.drawMs`/提示、`engineRun`、画面更新/秒、入力 p50/p95、audio の underrun と lead、エラーを集計する(生の beacon は写さない)。
- 本番の beacon が 0 件なので、**この機械の ci1 の局所実行の beacon(本番ではない)に当てた例**だけを置いた: `tables/classify-example-local-dev.txt`(Chrome 156 headless は SwiftShader、Firefox 158 headless は llvmpipe)。依頼者の機械の値ではない。beacon が現れたら `python3 tools/beacon-classify.py <perf/ をコピーした場所>` でそのまま集計できる(R2 から取るなら `tools/r2list.py` の一覧 + `wrangler r2 object get fl-web-cache/<key> --remote --file`)。

## 8. ライブの tail
- 現在の接続だけを見るために `wrangler tail fl-studio-web --format json` を、`tools/tail-filter.py` を通して集計した(時刻・メソッド・パス・status・ブラウザの種類・OS・beacon に触れるログ行だけを残し、IP・cookie・Access の識別情報・ヘッダは捨てる)。結果: 20:14〜20:41(27 分、PM の許可は 21:10 まで。待つ手段が無いので 20:41 で止めた)に **Worker に届いたリクエストは 0 件**(`tables/tail-live.txt`。Access が手前で止めた物は見えない)。止めた後、サーバー側の tail 一覧が空になったことを確認。最後にもう一度 `perf/` を一覧して 0 件(全 404 オブジェクトは全部 `cache/`)。

## 9. beacon2(失敗の痕跡が本番に残るか) → `BEACON2.md`、`beacon2.diff`
- 今の beacon は first idle・定期(first idle の後)・showError・終了でしか出ず、『固まる』『first idle 前に離れる』『boot の外の uncaught error』『Access 切れ・最初の 4xx で止まる』は何も残さない。最小の失敗 report(1.5 KB、種類ごとに 1 回、旗なし)を足す差分とテストを置いた。詳細・限界は `BEACON2.md`。

## PM の判断と取り込み(2026-11-05 20:55)

取り込み: mt3 の締め(20:45)の README を本決定に、`BEACON2.md`・`beacon2.diff`・表・道具を `docs/investigations/beacon1/` に置いた。コードは `beacon2.diff`(4 ファイル: `app/src/main.ts`、`app/src/perf/failure-beacon.ts`〔新規〕、`app/test/e2e/run.mjs`、`app/test/unit/failure-beacon.test.ts`〔新規〕)を HEAD 2ca825ff に `patch -p1 --batch`(offset・fuzz・reject なし)で当てた。`-` 行は `main.ts` の 5 行だけで、`showError` の署名に `kind` を足し、呼び出し 4 か所に種類を渡す変更。旗・URL パラメータ・設定は足していない(`docs/policies/no-options.md`)。

PM が測った値(この機械): typecheck rc=0、vitest 1,288 passed + 7 skipped(基準 1,269 + 19)、`test:worker` rc=0、`test:e2e` 190/190(エンジン無し。default-manifest の失敗経路の検査を含む)。

判断:
1. **採る(beacon2)。** 失敗の痕跡(固まる、first idle 前に離れる、boot の外の例外、能力検出の 11 個の真偽)が本番に残るようにするだけで、成功時の beacon は変えない。1.5 KB の report を種類ごとに 1 回、送れなくても黙る。メッセージは URL・メール・IP・利用者パス・ファイル名を落として 200 字。
2. **依頼者の機械の値は 0 件のまま。** 本番の R2 の `perf/` は 0 件(全 404 オブジェクトは `cache/`)、Worker への到達も 27 分の tail で 0 件。H1〜H5 は読み取りだけでは分けられない。ci1 の「beacon 204 保存済み」は局所の `vite preview` の値で、本番の保存ではない(PM の前提の訂正を受け入れる)。依頼者に頼まない方針なので、beacon2 が本番に出て、依頼者が開いたときに初めて原因が分かれる。
3. 限界(mt3 の申告どおり): 実 Firefox 158(この機械、Linux、headless)で能力検出が全部 true を返すことは確かめたが、依頼者の Firefox 159 / Windows / GTX 980 では未確認。ページの JS が動かない場合、Access のセッション切れ(302 で fetch が失敗)、最初の 4xx で `perfState.disabled`、別の URL は、`app/src` からは拾えない。60 秒の no-first-idle は遅い正常起動でも出うる。大きい beacon と小さい failure report の二重報告がある。
4. 本番への反映は app だけの変更なので、短い CI(Worker のバージョン更新)で出る。
