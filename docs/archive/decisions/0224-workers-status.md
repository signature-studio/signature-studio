# 0224: Workers(`https://<CF-VALUE-REMOVED>/`)で状態を確認できるようにする — ビーコンの R2 保存、`/__perf/summary`、最新アプリの手動配備と手順書(担当 workers1)

状態: **仮の決定(すべての選択)**。依頼は依頼者(原文: 「ローカルのサーバーは停止して大丈夫です。あなたもしくはあなたの subagent が検証する場合は一時的に立ち上げることもできます。私は今後、Workers で状態を確認します。」)、PM 経由の決定 0224。実装・測定・文書は作業者 `workers1`(2026-11-04 JST 08:38〜)。基準は `origin/main` 050fa43(`git archive HEAD`)。この決定は**配信・計測の基盤であって、FL Studio の実エンジンの進捗ではない**(G 段階表の到達に数えない)。実エンジンが実行されたのは §5 の互換確認(HEAD のページ + SHIP2 の wasm を Chrome 156 headless で first idle まで)だけ。

区分の凡例は決定 0152 と同じ。**実測**＝このセッションのコマンドの出力を見た事実。**読み**＝コード・文書を読んだ解釈。**出典なしの選択**＝根拠が無く作業者が選んだ値(`S-workers1-…`、§9)。**未確認**＝確かめていない。環境の『この機械』= 16 コアの Linux VM(他担当の負荷で load 17〜43)。依頼者の機械ではない。壁時計の値は参考値。

## 冒頭の三点

1. **何を実測したか**: (a) 書いたコード(Worker のビーコン保存、`/__perf/summary`、集計ライブラリの共通化、ci のスクリプト)を、型検査・vitest・Worker テスト(`wrangler dev --local` のローカル R2)・e2e(Chrome 156、ページの実ビーコン → Worker → R2 のオブジェクト)・変異検査で測った。(b) HEAD のページ + SHIP2 の wasm の組を実エンジンで first idle まで動かした(6.18 s、load 17.7)。(c) 本番に対して、ライフサイクル規則の追加 1 回、Worker の配備 1 回(version `b1823d09-dbb8-40d6-a922-0e99b708d556`)、一時トークンの作成と失効 7 本を行い、配備の前後に認証なしの 16 件が全部 Access の 302 であることを確かめた。(d) R2 の読み取りの手順(バケット限定の読み取りトークン → `perf/` だけの一時資格情報)を一時トークンで試した。
2. **結論**: Workers の Worker は、ページのビーコンを **1 ビーコン = 1 オブジェクト**(`fl-web-cache` の `perf/<UTC 日付>/<sid>-<seq>.json`、中身は `{t, remote:"workers", perf}` の JSONL 1 行)で保存し(IP・Access のメール・Cookie・トークンは保存しない)、`GET /__perf/summary`(Access の背後)で集計を日本語の見出し付きのテキストで返す。保存期間は `perf/` を 180 日で失効。最新の main(050fa43)+ この差分のアプリを Worker `fl-studio-web` に配備した。依頼者が一度ページを開けば、以後は依頼者もエージェントも Workers 上の実測を読める。**本番の Worker が実際に R2 へ書くことは、ローカルの R2 では確認したが本番では未確認**(Access の背後)。
3. **未確認**: 本番の Worker から R2 への書き込み(依頼者のブラウザの最初のビーコン後に作業者が `perf/` を読んで確認する)、依頼者の Access 経由の表示、`ci/deploy-manual.sh bundle --execute`(本番の配信バケットは変更していない)、GitHub 上のワークフローとの結合(`workers1-ci.diff` はブランチ上で `ci/test-wrangler-config.sh` と `deploy --dry-run` を通しただけ)、`tools/perf-sync.sh` の『オブジェクトがある場合』の経路(本番に `perf/` のオブジェクトがまだ無い)。

## 0. 先頭の答え

| 問い | 答え | 根拠 |
|---|---|---|
| ビーコンは Workers で失われなくなったか | **ローカルでは保存される**(11 件の Worker テスト + e2e 33/33)。本番の保存は**未確認**(依頼者の最初のビーコン待ち) | §1、§6 |
| 保存するもの/しないもの | 時刻・`workers`・ビーコン本体のみ。IP・`Cf-Access-Authenticated-User-Email`・Cookie・`Cf-Access-Jwt-Assertion`・User-Agent ヘッダーは保存しない(それらを付けて送ったテストで、保存物に含まれないことを確認) | §1、テスト `keeps neither the IP address…` |
| 失敗したらページは壊れるか | 壊れない。204 + `X-Perf-Stored: no`、失敗は Worker のログへ(キーと原因のみ) | §1 |
| 費用 | 1 ページ読み込みで最大 22 回の PUT(Class A)≒ **$0.0001**。無料枠内(月 100 万 Class A) | §1.4 |
| 保持期間 | `perf/` を 180 日で失効(本番に追加。既存の 2 規則は残った) | §1.5 |
| 依頼者が見る手段 | `https://<CF-VALUE-REMOVED>/__perf/summary`(ブラウザ、Access の認証済み) | §2 |
| エージェントが読む手段 | この機械の `cf` / `wrangler` のログインで読める(`tools/perf-sync.sh`)。依頼者が望めば専用の読み取り資格情報(§2.3、手順あり) | §2 |
| 最新の main のアプリを Workers に配備したか | **はい**(050fa43 + 本差分 + ci1 の Worker 名 1 行)。version `b1823d09…`(2026-11-04 09:07 JST) | §4 |
| 配備後に認証なしの要求は全部 Access で止まるか | 16 件すべて 302(配備の前も後も) | §4 |
| SHIP2 のバンドルは最新のページと互換か | 互換(実エンジンが first idle に到達、6.18 s) | §5 |
| Worker に余計なロジックは有るか | **見つからなかった**(FL 固有の分岐 0)。候補 2 点(§7) | §7 |

## 1. ビーコンの保存(Worker)

### 1.1 設計(読みと実測)

- 保存先: R2 バケット **`fl-web-cache` の `perf/` 接頭辞**(S-workers1-1)。新しいバケットは作っていない。配信バケット `fl-web-assets` には入れない(`/assets/*` の配信経路に混ざらない)。`cache/` の 60 日失効規則とは別に `perf/` の規則を足した(§1.5)。Worker の R2 束縛は `PERF_BUCKET`(`app/wrangler.jsonc`)。
- キー: `perf/<UTC 日付>/<sid>-<seq>.json`(S-workers1-2)。`sid` はページが読み込みごとに作る 12 桁の 16 進、`seq` はビーコンの通し番号。**ページの文字列がキーに入らない**: `sid` が 12 桁の小文字 16 進でなければ `nosid-<8 桁乱数>`、`seq` が 0〜999,999,999 の整数でなければ 0(`perfObjectKey`、単体テスト 21 件)。日付は Worker が受け取った時刻の UTC。
- 中身: ローカルのログと同じ JSONL の 1 行 `{"t": ISO 時刻, "remote": "workers", "perf": <ビーコン>}`(`perfLine`)。`cat` した複数オブジェクトがそのままログになる。`remote` は固定の語 `workers`(S-workers1-5: 従来の `loopback|lan|tailnet|other` は Workers 経由では Cloudflare のアドレスの分類になり意味を失い、アドレスは個人情報なので保存しない)。
- サイズ: 従来どおり 1 ビーコン 16 KB 以内(検査は変更なし: 405/403/415/413/400)。保存されるオブジェクトは時刻と `workers` の分だけ少し大きい(16 KB ちょうどの本文で保存できることをテスト)。実測の大きさ: 本番に出たビーコンはまだ無いので、これまでのローカルのログ 184 件の 1 行は中央値 5,003 B、p95 7,478 B、最大 8,262 B。
- 失敗時: `put` の例外は捕まえ、`console.error`(キーと原因のみ。ビーコンの中身は出さない)を残して **204 + `X-Perf-Stored: no`**。束縛が無い Worker も 204/no(ログは出さない: 設定であって故障ではない)。ページは `X-Perf-Stored` を読んで『保存された/されなかった』を表示する(ページは変更していない)。
- 『検査を通ったものだけ保存』: 415/413/400/403/405 のどれでも保存されない(テストで前後のオブジェクト数を比べる)。
- 置いた場所: `app/worker/perf-store.ts`(`storeBeacon`)、`app/shared/perf.ts`(`perfObjectKey`、`PERF_PREFIX`)、`app/worker/index.ts`(呼び出しと `X-Perf-Stored`)。

### 1.2 テスト(実測)

| 層 | 結果 |
|---|---|
| 単体(`test/unit/worker-perf-store.test.ts`、メモリ上の偽 R2) | 40 件 passed: キーの組み立て 21、保存(中身・プライバシー・束縛なし・失敗・Error でない拒否)5、集計(窓・上限 600・同時 6 以下・カーソル・欠けたオブジェクト・rows・machine・json・読めないオブジェクト・パラメータ検査)10、結合順序・`--dir` の CLI・`machineOf` 4 |
| Worker(`npm run test:worker`、`wrangler dev --local` のローカル R2) | 50/50(これまで 43。旧『保存しない』4 件を、保存と集計の 11 件に置き換え)。ローカル R2 の中身は miniflare の sqlite とブロブファイルを**直接**読んで確かめた(Worker の API を通さない独立の読み) |
| e2e(Worker 対象、perf-beacon 群) | 33/33: ページの実ビーコン(スタンドインのバンドル)が Worker に届き、ローカル R2 に新しいオブジェクトが 1 つ、キー形式・content type・1 行の JSONL・`remote: "workers"`・IP 無し・`beaconProblems` が空、`/__perf/summary` にその記録が STAND-IN と印を付けて出る |
| 変異検査(新 20 件、unit + worker 層) | **20/20 検出**(1 回目は 19/20。見逃した `perf-store-without-a-binding-crashes` は、束縛が無いまま進んでも `try/catch` が false を返すので結果が同じになる等価変異で、『束縛が無いときは失敗として記録しない』をテストに足して再実行し検出)。検出の理由はテスト名で確認(`logs/mutation-0224.log`、`mutation-0224-rerun.log`)。注意: worker 層は load 40〜70 の間の実行で、runner は非ゼロ終了を『検出』と数えるため、1 件は無関係の既存の Worker テストの失敗、1 件は失敗テスト 0 件の終了(harness の失敗)が混ざった。unit 層でも検出した変異はそれで確かな検出。worker 層だけが頼りの 3 件(`perf-worker-always-says-stored`、`perf-worker-stores-before-checking-the-body`、`perf-summary-route-answers-post`)は、失敗したテストの名前が狙いどおり |
| 型検査 | rc=0 |
| vitest 全体 | 1,545 passed + 7 skipped(これまで 1,505 + 7) |
| e2e 全体(dev + preview + worker) | **1,262/1,262**(dev 426 + preview 426 + worker 410。これまで 1,256。増えた 6 件は Worker 対象の保存の確認〔perf-beacon 群 4、perf-endpoint 群 2〕。`logs/e2e-full.log`。load 40〜70 の間の実行) |

### 1.3 『プライバシー』の点検

保存物に入る可能性があるのは、**ページが自分で作ったビーコン**(ブラウザ名・画面・論理コア数・GPU 名・各種計測)だけ。これは決定 0152/0192 で依頼者に説明済みの内容で、パネルに『送る項目/送らない物』が書いてある。リクエストのヘッダーは 1 つも保存しない。**注意**: パネルの文面(`src/perf/panel.ts:244`)は『このページを配信しているサーバーのログ(perf-log.jsonl)に送られます』のままで、Workers では R2 のオブジェクトである。文面が実態と食い違うが、ページ(担当外)は変えていない。

### 1.4 費用の見積り

- **訂正**: 依頼文の『30 秒ごとの定期ビーコンで 1 セッション 1 時間 約 120 回』は実装と合わない。ビーコンは『最初の idle で 1 回、その後 30 秒ごとに**最大 20 回**、終了時に 1 回』(`src/perf/beacon.ts` の `BEACON_MAX_PERIODIC = 20`)で、**1 ページ読み込みで最大 22 回**(最初の 10 分で打ち切り)。
- R2 の料金(https://developers.cloudflare.com/r2/pricing/ 、2026-11-04 に読んだ。Standard): Class A(PutObject・ListObjects ほか)$4.50/100 万回、Class B(GetObject・HeadObject ほか)$0.36/100 万回、無料枠は月 100 万 Class A・1,000 万 Class B・10 GB-month(認証・認可で失敗した要求は課金されない、とも書いてある)。アカウントは `R2 Paid` と `Workers Paid`(`cf accounts subscriptions get`、実測)。
- 1 ページ読み込み: 最大 22 PUT = 22 × $4.50 / 1,000,000 = **$0.000099**。サイズは最大 22 × 8.3 KB ≒ 182 KB(最大 8,262 B のビーコンで)。1 日 1 ページ読み込みを 180 日(保持の上限)続けても約 33 MB・約 4,000 PUT で、無料枠(10 GB、100 万 Class A)の 1% 未満。
- `/__perf/summary` 1 回: LIST が `days` 日分(既定 2 回〔1,000 件ごと〕)と GET が最大 600 回 = Class A 2 回 + Class B 600 回 = 約 $0.0002。
- 実際の請求額は依頼者のダッシュボードで確認できる(未確認)。

### 1.5 保持期間(本番に追加した規則)

`wrangler r2 bucket lifecycle add fl-web-cache perf-expire perf/ --expire-days 180`(`ci/deploy-manual.sh lifecycle`、`Workers R2 Storage Write` の一時トークン 3 時間、使用後に失効)。**実測**: 前後の `lifecycle list` で、`Default Multipart Abort Rule`(全体、7 日)と `expire-60d`(`cache/`、60 日)が残り、`perf-expire`(`perf/`、180 日)が増えた(`logs/prod/lifecycle-run.txt`)。180 日は依頼文の指定(S ではない)。

## 2. 集計の確認手段

### 2.1 (i) `GET /__perf/summary`(依頼者がブラウザで見る)

- Access の背後(Worker の `checkAccess` の後、他のすべてのルートと同じ。認証なし/不正トークンは 403、テスト済み)。FL の挙動にも窓にも関係しない。
- 返すもの: 先頭に日本語の見出し(保存の説明、範囲、範囲内のオブジェクト件数と読んだ件数、パラメータの一覧)、続けて既存の集計(`shared/perf-summary.mjs` の `format`: P1/P2/P3 の行、エンジンの停止メッセージの表〔決定 0209〕、Files の使用〔0216〕、起動アセットの由来〔0220〕、機械/ブラウザ/画面周波数ごとの表)。記録ごとの 1 行表示は最後の 30 件(`?rows=all` で全部)。
- 見え方(ローカルの Worker に 1 件送ったテストの先頭 3 行。実測):
  ```
  計測の集計(Workers が R2 の perf/ に保存したビーコン。保存するのはこのWorkerが受け取った記録だけで、IP アドレス・メール・Cookie は保存していません)
  範囲: UTC 2026-11-03 〜 2026-11-03(1 日)。範囲内のオブジェクト 1 件、読んだ 1 件
  ?days=N(1〜180)  ?machine=all|tailnet|loopback|lan|other|workers  ?rows=N|all  ?format=json
  ```
- パラメータ: `?days=N`(1〜180、既定 2。UTC の日数)、`?machine=all|tailnet|loopback|lan|other|workers`、`?rows=N|all`、`?format=json`(`{window, summary}`)。不正な値は 400。HEAD は 200(本文なし)、他のメソッドは 405。
- 上限(S-workers1-4): 1 回に読むのは最大 600 オブジェクト(範囲内がそれより多ければ**アップロードの新しい順**に 600 件。古い分は数えていないと見出しに書く)、同時に 6 本。根拠: Workers の同時接続 6 本の制限と、1 回の呼び出しのメモリ 128 MB(600 × 最大 16 KB の本文 + 解析結果)。Workers Paid なので CPU 30 s・サブリクエスト 10,000 の範囲(https://developers.cloudflare.com/workers/platform/limits/ 、2026-11-04 に読んだ。Free だと CPU 10 ms・サブリクエスト 50 で足りないが、このアカウントは Paid)。
- 注意: 同じ集計ライブラリが、エンジンに関する数値を『実測した値』として出す。代役(`surface-table-v1`)の記録は STAND-IN と印が付き、判定の行には数えない(従来どおり)。

### 2.2 (ii) エージェントが読む手段

1. **この機械の既存のログインで読める**(依頼者の作業は不要): `cf r2 objects list --bucket-name fl-web-cache --prefix perf/`(実測: 空の `[]` が返った)と `wrangler r2 object get fl-web-cache/<key> --remote --file <path>`(実測: `fl-web-assets/manifest.json` を同じ方法で読めた)。`tools/perf-sync.sh <ディレクトリ> [日数]` がこの 2 つで `perf/` を取り、`node app/scripts/perf-log-summary.mjs --dir <ディレクトリ>` で `/__perf/summary` と同じ集計を出す。ただしこのログインは依頼者のアカウント全体の広いスコープ(OAuth)であり、`perf/` に限定されていない。
2. **`--dir` の追加**: `scripts/perf-log-summary.mjs` に `--dir <ディレクトリ>`(以下の `*.json` を全部読み、記録の時刻順に並べて 1 つのログとして集計)を足した。`aws s3 sync` した物にもそのまま使える。

### 2.3 (iii) 専用の読み取り資格情報(任意。依頼者が望めば)

エージェントのログインを使わず、読み取り専用の資格情報で `perf/` だけを読ませたい場合の手順。**値は作業者に求めない・文書にも書かない**。方法は 3 つ:

**A. バケット限定の読み取りトークン(依頼者がダッシュボードで作る)**

Cloudflare ダッシュボード → 左メニュー『R2 オブジェクト ストレージ』(R2 object storage)→ 『概要』(Overview)→ 『アカウントの詳細』(Account Details)の『API トークン』の『管理』(Manage)→ 『アカウント API トークンを作成』(Create Account API token)→ トークン名(任意、例 `r2-read-fl-web-cache`)→ 権限『オブジェクト読み取りのみ』(Object Read only)→ 『特定のバケットのみに適用』(Apply to specific buckets only)で **`fl-web-cache` だけ**を選ぶ → TTL は任意(例 30 日)→ 作成。画面に出る **Access Key ID** と **Secret Access Key**(一度しか表示されない)を控える。(メニュー名の日本語表記は読みの推測。英語表記は https://developers.cloudflare.com/r2/api/tokens/ のとおり。)

- 実測(作業者が同じ権限のトークンを API で作ってテスト): このトークンは S3 資格情報として `fl-web-cache` を読める(`perf/` 0 件、`cache/` 32 件)、他のバケット(`fl-web-assets`)は AccessDenied。**R2 の資格情報はバケット単位で、プレフィックスでは絞れない**: このトークンは `cache/`(CI の段階キャッシュ)も読める(中身は我々のビルド成果物の層で、個人情報ではない)。
- 渡し方(選択肢のみ): (1) 依頼者が、エージェントを起動する端末の環境変数 `<CREDENTIAL-NAME>`・`<CREDENTIAL-NAME>` に設定する(リポジトリの外。GitHub の Secrets ではない: Secrets は GitHub Actions からしか読めず、エージェントには届かない)。(2) 端末のホームの設定ファイル(リポジトリ外)に書く。いずれも、チャットや文書に値を貼らない。
- 使い方: `AWS_DEFAULT_REGION=auto aws s3 sync s3://fl-web-cache/perf/ <dir> --endpoint-url <CF-VALUE-REMOVED>` → `node app/scripts/perf-log-summary.mjs --dir <dir>`。

**B. 一時資格情報(`perf/` だけ・最長 7 日。A の親トークンから作る)**

R2 の『一時資格情報』(https://developers.cloudflare.com/r2/api/s3/temporary-credentials/)は、親トークンから**プレフィックスと有効期限を絞った**資格情報(Access Key ID・Secret・Session Token の 3 つ)を作る。`cf r2 temporary-credentials create --bucket fl-web-cache --parent-access-key-id <A の Access Key ID> --permission object-read-only --prefixes perf/ --ttl-seconds 604800`(最長 7 日)。3 つの値を環境変数 `<CREDENTIAL-NAME>`・`<CREDENTIAL-NAME>`・`AWS_SESSION_TOKEN` で渡す。

- 実測(作業者が一時トークンでテスト): この資格情報は `perf/` を読め、`cache/` と全体の list は AccessDenied、書き込み(PutObject)も AccessDenied。親トークンを失効すると派生物も即失効する(文書の記述)。作成の直後は親トークンの反映待ちで、1〜2 回の再試行が要った(`logs/prod/r2-read-token-check3.txt`)。

**C. 何も作らない**: §2.2 の 1(この機械の既存ログイン)。依頼者の作業ゼロ。広いスコープのログインを使うことになる点だけが違い。

## 3. 配備した物と、その中身

- 配備した木: main HEAD 050fa43 の `git archive` + `workers1.diff`(アプリ)+ ブランチ `ci/cloudflare-deploy` の `ci/`・`.github/`・`app/wrangler.jsonc` の `name` 1 行(`fl-studio-web`)。main の `app/wrangler.jsonc` は今も `fl-studio-web-private` で、名前の変更は ci1 のブランチの PR にだけある。この名前のまま配備すると、別の Worker が新規に作られてカスタムドメインがそちらへ移る(読み)ので、`ci/deploy-manual.sh` は『その名前の Worker の deployments が無ければ拒否』にした(`--new-worker` で明示した場合だけ通る)。
- 含まれる main の変更(依頼文の挙げた物): 音の既定 ON、OPFS の窓、画面サイズ(決定 0212)、アセットの OPFS 永続キャッシュ(0220)、ビーコン版 3 の追加、停止メッセージ(0209)。バンドルは SHIP2(`fl-engine-ship2-F2xH2`、`fl-web-assets` に既にある物。変更していない)。

## 4. 本番の配備(実測。`tables/prod-ops.tsv`、`logs/prod/`)

| 項目 | 内容 |
|---|---|
| 配備前の記録 | `wrangler deployments list --name fl-studio-web`: 現行 `497d0938-6d13-4e1e-863c-c7c08d1c4eb9`(2026-11-03T21:43:47Z)、その前 `efda5c06-…`。ほか versions list、`lifecycle list`、トークン一覧(5 本)を保存 |
| 配備 | `ci/deploy-manual.sh deploy`: ビルド → 設定(Worker `fl-studio-web`、束縛 `ASSETS_BUCKET=fl-web-assets`・`PERF_BUCKET=fl-web-cache`、ルート `<CF-VALUE-REMOVED>`)→ dry-run → 一時トークン(Workers Scripts Write、3 時間)→ deploy → 失効。version **`b1823d09-dbb8-40d6-a922-0e99b708d556`**、2026-11-04T00:07:18Z(09:07 JST)。Worker Startup Time 4 ms、アップロード 58.15 KiB |
| 警告 | wrangler が『PERF_BUCKET の自動プロビジョンの権限が無い』と警告(Workers Scripts Write だけのトークンでは R2 の存在確認ができないため、と読む)。配備は成功した。**本番の書き込みが動くかは未確認**(§0) |
| 認証なしの要求 | 配備の**前**に 16 件、**後**に 16 件(GET `/`、`/static/probe.js`、`/assets/manifest.json`、`/assets/` の架空のアセット、`/__perf`、`/__perf/summary`; POST `/`、`/assets/manifest.json`、`/__perf`、`/__perf/summary`; HEAD `/assets/manifest.json`; Range 付き GET 1 件; manifest が挙げる 4 アセット各 1 件の Range 付き GET)。**全部 302(Access のログイン)**。`/__perf/summary` は配備前から Access の 302(Worker にまだ無い経路でも Access が先に止める) |
| トークン | 配備用 1(自動失効)、ライフサイクル用 1(手動失効。最初のスクリプトは `cf … delete` に `--force` が無く失効に失敗した)、読み取りの手順確認用 5。全部失効し、一覧は元の 5 本 |

## 5. SHIP2 のバンドルと最新のページの互換(実エンジン)

`fl-web-assets` の既定 manifest(`manifest.json`)の SHA-256 は `1e7c6cccb993aad2…`(本番の R2 を `wrangler r2 object get --remote` で読んで確認。ローカルの SHIP2 の manifest と同一)。HEAD のページ(`dist`)と SHIP2 の wasm(`engine.f4a57e84a5e1.wasm`、sha256 `f4a57e84…`)を、一時 `vite preview`(127.0.0.1:5973、終了済み)で Chrome 156.0.8078.47 headless に読ませた(`tools/compat-first-idle.mjs`、冷たいプロファイル): `state=engine-idle`、**first idle 6,183 ms**、first_pixel 6,208 ms、download 1,205 ms、エンジンのエラー無し、RSS ピーク 2,172 MiB。アセットの由来は 4 つとも network、保存は stored。**load 17.7 の間の壁時計で、依頼者の機械ではない。P1(10 秒以内)の判定は依頼者の機械の値で行う**(判定ではない)。互換は壊れていない。

## 6. 検証のまとめ

`tables/tests.tsv`。

## 7. Worker 全体の方針点検(lift の忠実性。`tables/policy-audit.md`)

`app/worker/index.ts`・`access.ts` と、Worker が import する `shared/{headers,range,manifest-guard,perf}.ts` を点検表で全行読んだ。**FL のクラス名・窓の種類・位置・文言による分岐は 0 行、FL の挙動の代行・補正も 0 行**。除去の差分は無い。候補:

1. 観測(ビーコン)の既定 ON — 点検表は『観測は製品の経路から切り離す』だが、ビーコンは依頼者の明示の要求(決定 0152/0192: 実機の P1/P2 を質問なしで集める)で既定 ON。別パス `/__perf` で、ページが `?beacon=0` で止められ、FL の挙動に触れない。本決定の保存と集計も観測だけで、新しい処理の三分類(API の契約・余計な処理の除去・変換/通信の削減)のどれにも当たらない。足した理由は §0: 依頼者の『Workers で状態を確認する』(性能要件は測定値だけを書く)に応える基盤。**PM の判断点**。
2. `shared/headers.ts cacheControlFor` — 名前から不変かどうかを当てる推測規則。R2 に Cache-Control が保存されていればそちらが使われ(本番は全オブジェクト保存済み)、この規則は使われない。保存が無いときは `no-cache` に倒す方が安全。変更していない(テストが依存)。

## 8. 規則の自己申告

- 08:55 に `( … &)` 形式で e2e を起動(ツールは拒否しなかった。気づいて kill し、`background: true` で再実行)。
- 変異検査の runner を誤って `node -e "import(run.mjs)"` で実行し、2 分のツール期限で中断(コピーの木だけを触る設計でワークツリーに損害無し)。
- 最初のライフサイクル用トークンの失効にスクリプトの不備(`--force` 無し)で失敗し、約 2 分後に手動で失効した。
- **トークン値の露出 1 件**: R2 読み取りの手順確認スクリプトの不備(bash の `${3:+VAR=$3}` 展開)で、15 分有効の一時資格情報(`perf/` の読み取り専用)のセッショントークンの先頭約 100 文字がエラーメッセージとしてツール出力に出た。ログの 1 ファイルからは削除済み(`<scrubbed>`)。その資格情報の親トークンは同じ実行の終了時に失効済みで、現時点で有効な物ではない。以後の実行では env 渡しに直し、出力に値は出ていない。
- 開始時に `git fetch -q origin` を 1 回実行(リモート追跡参照の更新のみ。ワークツリー・ブランチは変更していない)。

## 9. 出典なしの選択(S-workers1-)

| 番号 | 選択 | 理由・代替 |
|---|---|---|
| S-workers1-1 | 保存先は既存の `fl-web-cache` の `perf/` | 指示の候補。代替: 配信バケット(`/assets/*` の経路に混ざる=不可)、新しいバケット(指示で禁止)、別の接頭辞名 |
| S-workers1-2 | キー `perf/<UTC 日付>/<sid>-<seq>.json`、`nosid-<8 桁>` | 指示の形。日付ごとの LIST で期間を絞れる。sid が無い/不正な物はページの文字列をキーに入れないため。代替: 時刻+乱数(sid ごとの束ねが無くなる)、sid 単位の 1 オブジェクトに追記(R2 は追記できない) |
| S-workers1-3 | 同じ `sid-seq` の再送は上書き(先勝ちにしない) | 同じ内容の再送が想定。代替: `onlyIf` で先勝ち(PUT が失敗扱いになる経路が増える) |
| S-workers1-4 | 要約の既定は 2 日・最大 600 件・同時 6 本・記録の 1 行表示は 30 件 | Workers の同時接続 6 本・メモリ 128 MB・ブラウザで読める長さ。代替: 日数の既定を 7 に、件数の上限を上げる(Paid の CPU には余裕があるがメモリが先に効く) |
| S-workers1-5 | `remote` は固定語 `workers`、`machineOf` に `workers` / `workers, headless` を追加 | 従来の分類は Workers 経由では意味を失う。代替: `cf-ipcountry` などの属性を保存(個人情報に近づく=不採用) |
| S-workers1-6 | 保存の失敗は 204 + `X-Perf-Stored: no`(ページは壊さない) | 指示(c)。代替: 5xx(ページは beacon の失敗を表示する) |
| S-workers1-7 | `PERF_BUCKET` は Worker では任意(`PERF_BUCKET?`)、ci の設定生成では必須で配信バケットと同名は拒否 | 束縛が無い開発環境でも動く/本番の設定で付け忘れを防ぐ。代替: 常に必須 |
| S-workers1-8 | 一時トークンの権限は配備=Workers Scripts Write のみ、ライフサイクル/バンドル=Workers R2 Storage Write、期限 3 時間 | 指示の 3 時間。最小の権限組(実測: 配備は前者だけで通った) |
| S-workers1-9 | 認証なしのプローブは 12 件+manifest の各アセットの Range 付き GET | 指示の『12 件』を超えて、manifest が挙げる 4 アセットを足した(16 件)。代替: 12 件 |
| S-workers1-10 | エージェントの読み取りは、既存の `cf`/`wrangler` ログインを既定にし、専用の資格情報は任意とした | 依頼者の作業が要らない。代替: 専用トークンを既定にする(依頼者の作業が増える。広いスコープを避けられる) |
| S-workers1-11 | 配備の木は main + 本差分 + ci1 のブランチの Worker 名 1 行 | main の名前のままでは別の Worker が作られる。代替: ci1 のブランチの PR を待つ(依頼者はその間、古いアプリを見る) |
| S-workers1-12 | 範囲内が上限を超えたら『アップロードの新しい順』に読む | 直近の実測が大事。代替: 日付の新しい順/sid ごとの最終 seq のみ(1 ページの読み込みが 1 行になり、記録ごとの表示が消える) |

## 10. PM の判断点

1. **PR の順序**: `workers1.diff`(main)を先に取り込む。`workers1-ci.diff` はその後(ブランチの `wrangler_config.py` が `PERF_BUCKET` の束縛を要求するため、逆だと app ジョブが落ちる)。main の `app/wrangler.jsonc` の `name` は ci1 のブランチの PR で変わる。
2. ビーコンの既定 ON と、Worker での保存・集計という観測の基盤が、『観測は製品の経路から切り離す』に反しないか(§7)。
3. ページのパネル文面(`perf-log.jsonl` と書いてある)を Workers の実態に合わせるか(別件。ページの変更)。
4. `cacheControlFor` の推測規則を `no-cache` への倒しに変えるか(§7)。
5. 本番の書き込みの確認のために、依頼者が一度 `https://<CF-VALUE-REMOVED>/` を開くこと(開いたら私が `perf/` を読んで確認する)。

## 付録 A: 手動配備の手順書

`docs/deploy-manual.md`(同じ作業者の成果物)。スクリプトは `ci/deploy-manual.sh`(`workers1-ci.diff`)。

## 付録 B: 成果物

`workers1.diff`(HEAD に当たる)、`workers1-ci.diff`(ブランチ `ci/cloudflare-deploy` の `origin` 811031f に当たる)、`MERGE-NOTES.md`、`docs/`(この文書、`deploy-manual.md`、README の 1 行)、`tables/`、`tools/`(`compat-first-idle.mjs`、`perf-sync.sh`、`r2-read-token-check.sh`、`make-diff.sh`)、`logs/`。

## PM の判断と取り込み(2026-11-04)

取り込み先: HEAD 67d60b8(crash1 〔決定 0209〕の取り込み後)。`workers1.diff` の当たり: 3e4f540 には fuzz・offset なしで当たったが、crash1 が `app/scripts/perf-log-summary.mjs` と `.d.mts` に足した soft stops の表と型(約 20 行)と衝突した。ライブラリの移動(`app/scripts/perf-log-summary.mjs` の本体 → `app/shared/perf-summary.mjs`)は `workers1.diff` のまま行い、crash1 の追加は移動先 `app/shared/perf-summary.mjs` と `.d.mts` に同じ hunk を当てて移した(`git diff 3e4f540 67d60b8` の該当 2 ファイルを移動先に当てた。offset 2〜5 行)。crash1 の `.d.mts` に `SummarySoft` の宣言が 2 回入っていたので 1 回にした(型の内容は同じ)。`-` 402 行はこの移動(本体の削除 348 行ほか)で、読んだ。

PM が測った値(取り込み後のワークツリー): typecheck 4/4 rc=0、vitest 1,551 passed + 7 skipped(1,545 + crash1 の soft-stops 6)、Worker テスト 50/50、e2e 1,262/1,262(rc=0。crash1 取り込み後の 1,256 から +6 は workers1 の追加分)。

判断(workers1 の判断点 1〜6 への答え):
1. PR の順序: 本文のとおり main が先。`workers1-ci.diff` は ci1 のブランチで当てる(ci1 に伝えた)。
2. main の `app/wrangler.jsonc` の `name`: `fl-studio-web` に直した。ci1 のブランチの同じ行と**同じ文字列**にした(PR の合流で衝突させないため)。
3. ビーコン既定 ON と保存・集計: 許容する。理由: 観測の基盤であり(進捗の誠実性の規則の実測を、依頼者のブラウザから取るため)、保存する項目は IP・Access のメール・Cookie・トークンを含まず、FL 固有の分岐は 0 行(方針点検 `tables/policy-audit.md`)。lift の忠実性の方針の『新しい処理の三分類』には当たらない観測の基盤であることを本節に記録する。
4. パネルの文面: `src/perf/panel.ts` の『サーバーのログ(perf-log.jsonl)に送られます』を、手元の vite ならログ、Cloudflare Workers なら R2 の保存先 `perf/` と書き分けた。閉じたパネルの題の行(`src/main.ts`)の『このサーバーのログに送られます』は『このサーバーに送られます』にし、e2e の同じ文言の検査をそれに合わせた(検査の強さは同じ: 文言と `?beacon=0` の両方を見る)。
5. `cacheControlFor`(R2 に Cache-Control が保存されていないときの推測規則): 本番は R2 に保存済みなので使われない。推測規則は残すが、**R2 側に Cache-Control が無い物を見つけたら推測に頼らず R2 の保存を直す**(`ci/deploy-manual.sh bundle` の役目)。
6. 依頼者が `https://<CF-VALUE-REMOVED>/` を開いた時点で `perf/` に 1 件目が入る。確認は私(または agent)が `tools/perf-sync.sh` で読む。依頼者に頼む作業は無い(依頼者は『今後は Workers で状態を確認する』と述べている)。**本番の Worker が R2 に書くことは、依頼者のブラウザからの 1 件目が届くまで未確認**のまま。

注意(運用): 本番の Worker は 2026-11-04 09:07 JST に手動で 1 回配備された(version b1823d09…)。それ以後の main のページの変更(crash1 の beacon の soft stops、この取り込みの文面)は、次の配備まで本番に出ない。
