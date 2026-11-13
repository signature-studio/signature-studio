実エンジン: 実行なし(設計のみ。R2・Worker には書き込んでいない)
書き換えの対象: R2 root `manifest.json` を E13 の manifest と同じバイト列にする(1 件)。Worker の再デプロイは要らない: 未確認(現コードの読みでは要らないと読める。デプロイ済みの版 2cb938be のコードは未確認。§1)
退避: R2 の `backup/` 以下の別のキー(時刻入り)。確認できなければ書き換えない
戻し: 段 3 の後の失敗は自動(§2 段 5)。公開ホストでの確認(R4-5)が失敗なら、退避から戻す(§4)
後任にできない確認: 公開ホストを開いて実エンジンが動くこと。R4-5 に回す(§3)

# R6-4 設計: 既定の入口の差し替え(root manifest.json 1 件)

## 0 決定

- 分岐の分類: 段 1 の「sha が E13 の値なら何もせず終了 0」は、通信・変換の削減(再 push で二重に書き換えない)。新しい処理の分類はこの 1 行。

- 決定 A(書き換えの中身): root `manifest.json` 1 件だけ。根拠: `evidence/R6-2d.md` の段 3e で、R2 の E13 の 5 件が手元の raw と全 64 桁一致。手元の E13 は engine-idle を 15 回(`evidence/R0-2.md` 5 回、`evidence/R0-3.md` 10 訪問)。
- 決定 B(退避の名前): `backup/manifest-root-<UTC 時刻>.json`。根拠: 依頼の「fl-web-assets の root の外」を優先した。依頼の例 `manifest.backup-<日付>.json` は root に置かれ、文言と食い違うため。時刻を入れるのは、前回の退避を上書きしないため。
- 決定 C(前提が未確認の間は書かない): §1 の G1 が通るまで `.github/workflows/default-entry.yml` を commit しない。根拠: 手順 4(iv)。`app/worker` が最初に入ったのは `14268c2`(2026-11-10)、版 2cb938be は 2026-11-07 作成(`evidence/R6-1b.md` §2)。
- 決定 D(一回限り): ワークフローは書き換えを 1 回行い、成功後の次の commit で `default-entry.yml` を削除する。根拠: `on: push` のため、同じファイルを再 push すると書き換えが再実行される。再実行は、root が既に E13 と同じなら何もしない(段 1)。
- 決定 E(方式の変更): R6-4 の既定の入口の差し替えは、ROADMAP の `wrangler versions deploy`(100%)ではなく、R2 の root `manifest.json` を E13 の manifest と同じバイト列に書き換える 1 件で行う。根拠: Worker は R2 の root を要求ごとに読む(§1)。版 2cb938be の module は現在の `app/worker` のビルドと SHA-256 一致(`evidence/R6-4-g1.md`)。R2 の E13 の 5 件は手元と全桁一致(`evidence/R6-2d.md`)。分類: 通信・変換の削減(Worker の新しい版の作成・配備を避け、データの 1 件の差し替えにする。新しい処理・分岐は足さない)。ROADMAP の R6-4 の行は変えない(文言の差はこの決定に書く)。

## 1 Worker の配信の読み(`app/worker/index.ts`・`app/shared/headers.ts`・`app/shared/manifest-guard.ts`)

- 読むキー: `/assets/manifest.json` は R2 のキー `manifest.json` を読む(`index.ts:70` の head、`index.ts:101` の get)。既定の判定の対象は `isDefaultManifestKey`(`index.ts:71`)。
- 判定: `judgeDefaultManifest`(`index.ts:47-63`)が、`Content-Encoding` が無ければそのまま `judgeManifestBytes`(`index.ts:60`)に渡す。許可リストは `['emscripten-fruity-v1']`(`manifest-guard.ts:6`)。E13 の `engine.kind` は一致する。実行: 手元の node v24.19.0 で `judgeManifestBytes`(E13 の manifest のバイト列)を `--experimental-strip-types` で呼ぶ → `{"ok":true,"kind":"emscripten-fruity-v1"}`(manifest の判定。実エンジンの実行ではない)。
- 応答の作り: 要求ごとに R2 を読む。Worker のコードに Cache API・`cacheTtl`・`cacheEverything` は無い(`grep -rn 'caches\.\|cacheTtl\|cacheEverything' app/worker` が 0 件)。`Cache-Control` は `head.httpMetadata.cacheControl ?? cacheControlFor(pathname)`(`index.ts:86`)。`cacheControlFor` は manifest を `no-cache` にする(`headers.ts:50`)。ETag は `head.httpEtag`(`index.ts:84`)。書き換えで ETag が変わるので、`If-None-Match` の再確認で新しい内容が返る(`index.ts:89`)。
- 結論: 書き換えに Worker の再デプロイは要らない、と読める。Worker のコードは毎要求で R2 を読むので、Worker 側で書き換えの反映を遅らせる処理は読み取れない(`index.ts:70-107`)。Cloudflare の前段(キャッシュ規則の有無)での扱いは未確認(Cloudflare の docs 検索は接続が切れて使えなかった)。前段が応答を保持する時間は未確認。
- G1(未確認): デプロイ済みの版 2cb938be が、この読みと同じコードか。`app/worker` は `14268c2`(2026-11-10 04:48 JST)で初めて入った。版の作成は 2026-11-07。前任のワークツリー(読み取りのみ。ディレクトリ名は書かない)の同じ 4 ファイルは現在と同一(`cmp`)だが、そのツリーが版と同じかは分からない。cloudflare-api の MCP は再認証を要求し、API の仕様を検索できなかった(実行の出力)。確かめ方の案: `cloudflare-check.yml` に読み取りの段を足し、版の script を読んで現在の `app/worker` のビルドと比べる。R6-4 の worker が Cloudflare API の仕様で決める(未確認)。
- 手元の boot-check(`tools/boot-check/boot.mjs`)は Worker を通らない(`grep` で Worker・wrangler の参照が 0 件)。Worker が wasm に `Content-Encoding: br` を付けて返す経路は、手元の確認に含まれていない。

## 2 ワークフロー `default-entry.yml`(書かない。設計)

- 起動: `on: push` の `paths` は `.github/workflows/default-entry.yml` だけ。`workflow_dispatch`・`schedule`・入力・環境変数の旗を置かない。`permissions: contents: read`。`concurrency` は 1 本(二重実行の防止)。`timeout-minutes: 30`。
- Secrets: `CLOUDFLARE_API_TOKEN`(段 0 のアカウント ID の取得だけ)、`R2_CACHE_ASSETS_ACCESS_KEY_ID`・`R2_CACHE_ASSETS_SECRET_ACCESS_KEY`(`fl-web-assets` の読み書き。名前は `docs/requests/0001-cloudflare-actions-secrets.md` の表)。
- 出力: 段の結果、sha256、大きさ、`engine.kind`、bundle 名だけ。アカウント ID は `::add-mask::`。`upload-artifact` を使わない。

| 段 | 内容 | 失敗したとき |
|---|---|---|
| 0 | アカウント ID を実行時に 1 つ得る(`cloudflare-check.yml` の段 1 と同じ) | 段 1 へ進まない |
| 1 | root の `manifest.json` を get-object して `$RUNNER_TEMP` に置く。sha256・大きさ・ContentEncoding・engine.kind・bundle 名をログに出す。**sha が E13 の値なら何もせず終了 0**。退避先 `backup/manifest-root-<UTC>.json` の head が 404 であることを確かめ、put-object する(ContentType・ContentEncoding・CacheControl は root の head の値を、有るものだけ)。head と get で、大きさと sha が root と一致することを確かめる | exit 1(書き換えない) |
| 2 | E13 の `fl-engine-ci-0363b050/manifest.json` を R2 から get する。(i) sha256 が `57817472d5a0ea2d07c0859d45ace925086800b98e952107eabc61322e9e12dd`、(ii) `engine.kind` が `emscripten-fruity-v1`、かつ `manifest-guard.ts` を node 24 で読んで `judgeManifestBytes` が ok、(iii) `engine.wasm.url` が `/assets/fl-engine-ci-0363b050/` で始まる、(iv) wasm の head が ContentEncoding `br`・大きさ 15,697,877。get して `zlib.brotliDecompressSync` で展開し、大きさ 143,087,706・sha256 `58ad4a6aa720375aa412e6d87d58852688ddfa08ea27571953592d14ee74ccbf`(`evidence/R6-2d.md`)、(v) manifest の `/assets/` の URL 4 件(wasm・glue・dll・seed)の head が在る | exit 1(書き換えない) |
| 3 | 段 2 で R2 から読んだ E13 のバイト列を、そのまま root に put-object する(ContentType `application/json`、CacheControl `no-cache`、ContentEncoding は付けない)。head と get で、大きさ 3,098 と sha が E13 の値であることを確かめる。ローカルのファイルから書かない | 段 5 で戻す |
| 4 | runner で、root の sha・engine.kind・bundle 名を出す。root のバイト列を `judgeManifestBytes`(node 24)に通し、ok を確かめる | 段 5 で戻す |
| 5 | 段 1〜4 の結果の表を出す。段 3 以降に失敗していたら、退避のバイト列を root に put-object し、head と get で退避の sha に一致することを確かめて表に書く | 戻しも失敗: 退避のキー名と sha を表に出して exit 1。次の commit で §4 の手順 |

- 段 4 に「公開 URL を開く」は無い。公開ホストの名前が無いため(§3)。

## 3 後任にできない確認の扱い(決定)

- (i) 書き換えの根拠: E13 は手元で実行確認済み(`evidence/R0-2.md`・`evidence/R0-3.md`)。R2 の中身は手元と同じ(`evidence/R6-2d.md`)。
- (ii) 書き換えの前後で確かめる物: root の engine.kind・bundle 名・wasm の大きさ(段 1 と段 4 のログ)。
- (iii) 公開ホストの確認は依頼者機の計測(R4-5)で行う。結果が `engine-idle` でなければ、退避から戻す(§4)。R4-5 の依頼は GitHub の issue 3 に出した(`docs/requests/0002-owner-machine-measurement.md`)。
- (iv) 前提が崩れたら書かない: G1 が未確認の間は commit しない(決定 C)。
- 残るギャップ(未確認): 公開ホストの応答、Worker 経由の wasm の配信(§1 の最後の行)、版 2cb938be の中身。

## 4 戻し手順

- 自動: 段 5。
- 後から(R4-5 が失敗したとき): `default-entry-restore.yml` を、その時に一回限りで commit する。段 1 の head・get の確認と、段 5 の put・確認の組。退避のキー名と sha は `evidence/R6-4.md` に書く(名前だけ。ホストは書かない)。
- 戻した後は、root の sha と engine.kind を `evidence/R6-4.md` に書く。

## 5 commit の順(この依頼では実行しない)

1. G1 を通す(今は未確認)。
2. `default-entry.yml` と `evidence/R6-4.md` を commit → `git push origin main` → Actions の結果を表に書く。
3. 次の commit で `default-entry.yml` を削除する(一回限り)。
4. R4-5 の依頼を出す(未実施)。
