# 0152: 代役の manifest を既定のパス `/assets/manifest.json` で配信しない(409)・実エンジン無しの 404 に明示の本文・ページはサーバーの説明をそのまま出す・計測ビーコン `POST /__perf` と性能パネル。仮の決定

状態: **仮の決定**。依頼は PM(`docs/policies/progress-integrity.md` §4 の未実施 2 行と、`docs/milestones/performance-targets.md` §6)。実装・テスト・文書は作業者(`web1`)。作業は HEAD **2f94e25**(ブランチ main)の `git archive` から始め、差分 `web1.diff` は最新の HEAD **0991f92** の `git archive` の木に対する `diff -ruN`(`app` 全体と、`docs/policies/progress-integrity.md`・`docs/milestones/performance-targets.md`・`.gitignore` の 3 ファイル。`patch -p1` の形。2f94e25 から 0991f92 までに差分の対象と重なる変更は `performance-targets.md` の 1 ファイルだけで、その 1 行を 0991f92 の版に当て直した。`patch -p1 --dry-run` は 0991f92 の木で fuzz・offset・reject なし)。**`lift/runtime/`・`lift/crates/`・`lift/harness` は 0 行、面の表の ABI は変えていない。デプロイ・R2 へのアップロード・`wrangler login` はしていない**(Worker はローカルの `wrangler dev --local` だけ)。

区分の凡例は決定 0121〜0150 と同じ。**実測**＝このセッションのコマンドの出力を見た事実(2026-11-03 JST)。**読み**＝コード・文書を読んだ解釈で、走らせていない。**出典なしの選択**＝根拠が無く作業者が選んだ値(`S-web1-…`、§6 に撤回条件つき)。**未確認**＝確かめていない。

## 0. 先に書くこと(進捗の誠実性、AGENTS.md / `docs/policies/progress-integrity.md` に従う)

**(1) 実エンジンが実行された事実(実測)**: 配信側の拒否(A)のテストは配信コードとテストの検査で、**実エンジンは動かしていない**。ビーコン(B)は、PM が配信している実エンジンの bundle(`lift/out-websrv/assets/fl-vc`、manifest の `engine.kind` = `emscripten-fruity-v1`、bundle 名 `fl-engine-vclock1-snapshot`、wasm 320,731,834 B)を**読み取りだけ**して(自分の `/tmp/opencode/web1/real-assets/` に `fl-vc` へのシンボリックリンクと manifest のコピーを置き、それを `FL_ASSETS_DIR` にした。PM のサーバー〔5173〕と `lift/out-websrv` には書いていない)、自分の `vite preview`(別ポート)から、**パラメータ無しの既定の入口 `/`** を隔離した headless Chrome 156(TCP の非ループバック 0、UDP 送信 0)で開いて確認した(`app/test/e2e/perf-real-engine.mjs`)。結果: **最終ビルドの 1 回**(03:46 JST)で、パラメータ無しの入口は `window.__app.engineKind` = `emscripten-fruity-v1`、STAND-IN の帯なしで、**最初の idle まで 37.5 s**(`timeline.idle`。ダウンロード完了 2.8 s、コンパイル完了 3.0 s、`main_start` 3.05 s)。サーバーのログ(`FL_PERF_LOG` を指定せず、`FL_ASSETS_DIR` の親ディレクトリの既定のパス `/tmp/opencode/web1/perf-log.jsonl`)に 3 行: `first-idle`(37.5 s 時点。`engine.state` = `engine-idle`)、`periodic`(その 30.0 s 後)、`exit`(ページが End session を押した 0.16 s 後、`reason: stopped`)。1 つ前の版でも 1 回走らせた(41.7 s、3 行。この版は最初のビーコンの `engine.state` が `running` だったので、`beacons.firstIdle()` を状態の更新の後に移した)。これは vclock1 の**途中成果物**の bundle で、最終 bundle ではない。

**(2) 代役・再生・推測を含む範囲**: ビーコンの e2e(`perf-beacon`)と配信側の拒否の e2e(`default-manifest`)は、**代役 `surface-table-v1`**(`/assets/animate/…`、`/assets/stopscreen/…`、明示のパス)を使う。これは画像の再生で、FL Studio のエンジンではない。その記録は `bundle.standIn: true` と `engineKind: surface-table-v1` を持ち、ページには STAND-IN の帯(決定 0150)と、性能パネルの見出し「STAND-IN: …FL Studio の数字ではない」が出る。**これらの数字を G 段階表の到達や性能の達成として読んではならない。** 性能の数字(P1〜P3)は、この決定では**どれも達成されていない**(下の実エンジンの値は目標に届かない)。

## 1. 規則(A): 既定のマニフェストのパスは実エンジンのバンドル専用

`shared/manifest-guard.ts`(新規)が、`vite dev`・`vite preview`(`tools/assets-middleware.ts`)と Cloudflare Worker(`worker/index.ts`、R2 のキー)の**両方**から使われる。

| 要求(`/assets/` の後ろを一度だけ percent-decode し、正規化したキーが `manifest.json`) | 応答 |
|---|---|
| ファイルの JSON の `engine.kind` = `emscripten-fruity-v1` | 従来どおり(200/206/304/416/HEAD。**判定したバイト列をそのまま返す**。読み直さない) |
| `engine.kind` = `surface-table-v1` | **409**、text/plain、`Cache-Control: no-store`。本文: 「このパスは実エンジンのバンドル専用です。/assets/manifest.json の engine.kind は "surface-table-v1" です(代役: 保存した画像の再生であり、FL Studio のエンジンは動きません)。代役(surface-table-v1)は /assets/<名前>/manifest.json に置き、?bundle=/assets/<名前>/manifest.json で指定してください。理由: docs/policies/progress-integrity.md」 |
| JSON だが `engine.kind` が読めない、または未知の kind | **409**(許可リスト方式。`S-web1-allow-list`) |
| JSON として読めない・UTF-8 でない・2 MiB 超(検査できない)・br を保存した R2 の manifest(Worker は検査できない) | **500**、「/assets/manifest.json が壊れています(…)。…配信しません」(**中身は本文に出さない**。V8 の構文エラーが引用する断片も出さず、位置だけ) |
| manifest が無い(FL_ASSETS_DIR が空でも同じ) | **404**、「実エンジンのバンドルが未配置です。/assets/manifest.json が見つかりません。…代役で代わりにしてはいけません。…」 |
| それ以外の `/assets/…`(`/assets/<名前>/manifest.json` を含む) | 従来どおり。404 の本文も従来のまま |

判定の入口(`logicalKey`): 復号後のキーを、NFKC・`\` を `/`・`.` と空セグメント(`//`)の除去・`..` の解決・**小文字化**・最後のセグメントの末尾の `.` と空白の除去で正規化する。末尾が `/` のキーは `…/` のまま(ディレクトリ)。ファイルが `manifest.json.br`・`.gz` の変種で配信される場合(ブラウザが `Accept-Encoding: br` を送るとき)も、**復号した中身**を判定する(平文が実エンジン用で `.br` に代役が隠れていても、そのクライアントには 409)。`manifest.json.br` を**明示して**取る要求は保存バイトのままで、manifest として使えないので対象外(`S-web1-explicit-variants`)。メソッド・Range・If-None-Match・HEAD の前に判定するので、304 や 206 で迂回できない。

**ページ(`src/bundle/loader.ts`)**: manifest の HTTP エラーが text/plain の本文を持つとき、メッセージを「HTTP 409 Conflict\n<本文>」にする(2000 字まで。HTML のエラーページは写さない。大きな資産〔wasm など〕の失敗は従来の短いメッセージ)。エラー欄がそれをそのまま出す。**代役へは切り替えない**: e2e でパラメータ無しのページを開くと、state=`error`、`engineKind` null、manifest null、帯なし、canvas 0、`/assets/` への要求は `manifest.json` の 1 件だけ(wasm も stand-in の bundle も取らない)。

## 2. 計測ビーコン(B)

**ページ(`src/perf/`、`src/main.ts`)**。送る内容(`POST /__perf`、同じオリジンだけ。CSP は `connect-src 'self'` のまま。`credentials: 'omit'`):

| 項目 | 内容 |
|---|---|
| `browser` | `hardwareConcurrency`、`deviceMemoryGB`、`userAgent`(300 字まで)、`screen` の幅高さ、`devicePixelRatio`、`crossOriginIsolated`、`isSecureContext`、`webglRenderer`(`UNMASKED_RENDERER_WEBGL`、取れなければ null)、`jsHeap`(`performance.memory`、あれば。ページの JS ヒープで、エンジンの wasm メモリではない) |
| `bundle` | 名前・版・`engineKind`・`standIn`(`surface-table-v1` なら true) |
| `timeline`・`startup` | `window.__app.timeline` の各段(ナビゲーション開始からの ms)、`firstIdleSeconds` = `timeline.idle / 1000`(idle が無ければ null)、目標 10 |
| `ui` | 窓の面が更新された回数(合計)と直近 10 秒の更新間隔の中央値・p95・1 秒あたりの更新回数。同じ統計をエンジンの idle にも(`engineIdles`)。目標 120 /s。**表示方式 A(OffscreenCanvas を Worker が描く)ではページが更新を数えられないので `updates: null`**(0 と書かない)。実エンジンの既定は B |
| `input` | 入力→面の更新の遅延(直近 20 件の p50/p95/max と 20 件の値)。定義は決定 0141 の `latency.ts` の `toAnyFrame`: ページが入力を処理してから、**エンジンが入力を取った後の最初の、いずれかの窓の描画**まで。送信数・処理数・捨て/併合数・待ち数も。目標 p95 ≤ 17 ms |
| `audio` | `underruns: null`(**今の bundle には AudioWorklet が無く、測っていない**) |
| `engine`・`error` | 表示方式、状態、終了(`reason`・`code`・`presents`)、失敗(`what`・`where`。400/200 字まで) |

統計の定義(`src/perf/stats.ts`): 中央値と p95 は**ニアレストランク**(昇順で添字 `ceil(p/100·n) − 1`。偶数個の中央値は下側の中央)。1 秒あたりの回数 = 窓内の更新数 ÷ 窓の長さ(`min(10 s, now − エンジン開始)`)。手計算の期待値で単体テストした(例: 間隔 100 200 300 400 7000 500 100 400 → 昇順 100 100 200 300 400 400 500 7000、中央値 = 添字 3 = 300、p95 = 添字 7 = 7000、9 回 ÷ 10 s = 0.9 /s)。

**時機**(`BeaconScheduler`): 実エンジンの**最初の idle で 1 回**、その後 **30 秒ごと(最大 20 回)**、エンジンが終わったとき(`exit` か `error` の早い方)に **1 回**(以後の定期は止める)。代役は idle が無いので終わりの 1 回だけ。サーバーが HTTP 4xx/5xx を返したら、そのページは以後送らず、ページのログに 1 行出す(コンソールを毎回汚さない)。

**サーバー(`vite dev`・`vite preview`、`tools/perf-middleware.ts`、`shared/perf.ts`)**: `POST /__perf` を JSONL に追記する。パスは `FL_PERF_LOG`、無ければ **`FL_ASSETS_DIR` の親ディレクトリの `perf-log.jsonl`**、どちらも無ければ 503(保存しないと言う)。受理: POST、同じオリジン、`Content-Type: application/json`、本文 16 KB 以内、JSON のオブジェクト 1 つ。拒否: 405(`Allow: POST`)・403(別オリジン)・415・413(`Content-Length` が大きければ本文を読まずに答えて接続を閉じる)・400。1 行 = `{"t": ISO 時刻, "remote": "loopback|lan|tailnet|other", "perf": <ビーコン>}`。**IP は書かない**(ソケットの相手の種類だけ: 127/8・::1 = loopback、100.64/10・fd7a:115c:a1e0::/48 = tailnet、10/8・172.16/12・192.168/16・169.254/16・fc00::/7・fe80::/10 = lan、他 = other。`tailscale serve` の背後では相手がローカルなので `loopback` と出る〔読み〕)。応答は 204 と `X-Perf-Stored: yes`(拒否・未保存は `no`)。ログが 64 MiB を超えたら 507。**Worker**: 同じ検査をして**捨てる**(204 + `X-Perf-Stored: no`。保存しない)。別オリジンの判定は、`Sec-Fetch-Site`(ブラウザが付け、スクリプトは設定できない)があればそれで、無ければ `Origin` の host と `Host` の比較(`S-web1-sec-fetch-site`)。

**ページの「Performance」パネル**(`<details id="perf-panel">`、**既定は閉じている**): 同じ値を目標と並べる。P1(最初の idle までの秒 vs 10)、P2(更新回数/秒 vs 120、更新間隔の p95 vs 8.3 ms、入力→更新の p95 vs 17 ms)、P3(アンダーラン。今は「測っていない」)、ビーコン(何件送り、何件受理、何件保存されたか)。**赤** = 測った値が目標に届かない(idle 待ちが 10 秒を超えたときも赤)、**緑** = 測った値が目標を満たし、かつ実エンジン、**灰** = 測っていない(ダッシュ)、**琥珀** = 代役の数字(赤にも緑にもしない)。数字は測った値だけで、目標は別の列。

## 3. 実測

- `npm run typecheck`: 通過。
- vitest: **742 passed**(基準 HEAD 2f94e25 で 477。+265)。新規: `default-manifest-guard`(判定表・迂回・変種・壊れ方)、`perf-stats`(手計算)、`perf-server`(分類・拒否・追記・IP が無いこと)、`perf-beacon`(ペイロード・予定・POST・ブラウザ情報)、`perf-panel`(赤/緑/灰/琥珀)、`loader` の説明本文。
- Worker 結合(`npm run test:worker`、`wrangler dev --local`): **43 passed**(基準 29)。R2 が**空から始まる**もう 1 つの Worker で、代役・壊れた JSON・gzip/br 保存・`Manifest.json` などのキーを `wrangler r2 object put/delete --local` で入れ替えながらテストする。`POST /__perf` の 204/415/413/400/403/405 も。
- e2e(エンジン無し、dev・preview・Worker の 3 配信先、隔離 Chrome、TCP の非ループバック 0、UDP 送信 0): **691/691**(基準 522/522)。新規場面: `default-manifest`(404/409/200 とページの表示、迂回 9 経路、代役が明示のパスで配信されること)、`perf-beacon`(ビーコンの全項目、ログ 1 行、IP が無い、パネル)、`perf-endpoint`。
- 変異検査(判定・規則を 1 つずつ壊す→落ちる→戻す。`test/mutation/run.mjs`、木のコピー上): ****65/65**(最終の木で。A の規則 27 件〔shared/manifest-guard.ts 11・assets-middleware 8・Worker 5・loader 3〕+ B のビーコン 38 件〔統計・サーバー・Worker・ページ・予定・パネル・配線〕の合計 65 件を、unit/worker/e2e の層で 1 件ずつ)。**最初の実行で 3 件が生き残った**(`perf-p95-is-max`: p95 と最大値は 19 個以下の間隔では同じなので、20 個以上のテストが無かった。`perf-beacon-periodic-continues-after-the-end`: 終了後に残った 1 回分のタイマーをテストが見ていなかった。`perf-worker-body-limit-not-enforced`: 分割転送の過大な本文を途中で打ち切る処理をテストが見ていなかった)→ 3 件ともテストを足して、検出されることを再実行で確認した。途中の別の実行(A だけの 27 件、木が少し古い版)も 27/27。ログ: `logs/mutation-*.log`**。
- 迂回の経路ごとの結果(e2e の実ブラウザ側のサーバー、代役を既定のパスに置いた状態): プレーン・クエリ付き・`//`・`/./`・`/%2e/`・`/x/../`・`/x/%2e%2e/`・`%6d` 始まり・`manifest%2ejson` は dev と preview で**すべて 409**、Worker は `//` が 400(`bad path`)、他は 409。単体では更に 大文字小文字(`Manifest.json`)・末尾の `.`・全角・NFKC・symlink・`.br`/`.gz` の変種・HEAD・Range・If-None-Match も、200 にならないことを確認。

**実エンジンの値(この決定のためではなく、ビーコンが運んだ実測)**: 最終ビルドの 1 回で(`periodic` の行): P1 = 37.5 s(目標 10 s)、窓の面の更新 0.6 /s(目標 120 /s。直近 10 秒に 6 回、間隔の中央値 2.0 ms〔1 フレームの中で複数の窓を続けて描くため〕、p95 3404 ms)、エンジンの idle は 10 秒に 45 回(間隔の中央値 4.4 ms の塊。塊と塊の間は数秒)、入力→更新(8 件のポインタ移動)は p50 4830 ms・p95 6235 ms(目標 p95 17 ms)、アンダーランは未測定。**どれも目標に届かない**。測定環境は『このサーバー』(Linux VM、16 コア、headless Chrome 156、SwiftShader の GPU 名、画面 800x600、ロードアベレージ 3〜6)で、性能目標の文書 §2 のとおり**代理の測定環境であって、基準ではない**。依頼者の PC の値はまだ無い。

## 4. 実エンジンのページから届いた実例

`/tmp/opencode/web1/perf-log.jsonl` の 1 行目(`first-idle`、1,774 B。`remote` は `loopback`、IP は無い。UA の `156.0.0.0` は Chrome の版):

```
{"t":"2026-11-02T18:46:36.709Z","remote":"loopback","perf":{"v":1,"seq":0,"reason":"first-idle","pageMs":37544.77,"visibility":"visible","browser":{"hardwareConcurrency":16,"deviceMemoryGB":32,"userAgent":"Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) HeadlessChrome/156.0.0.0 Safari/537.36","screen":{"width":800,"height":600},"devicePixelRatio":1,"crossOriginIsolated":true,"isSecureContext":true,"webglRenderer":"ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0x0000C0DE)), SwiftShader driver)","jsHeap":{"usedBytes":12720799,"totalBytes":14738083,"limitBytes":4395630592}},"bundle":{"name":"fl-engine-vclock1-snapshot","version":"1c0eb51a8b08","engineKind":"emscripten-fruity-v1","standIn":false},"engine":{"present":"B","updatesObservable":true,"state":"engine-idle","exit":null},"error":null,"timeline":{"page_script":49.8,"manifest_loaded":62.4,"download_start":76.2,"download_done":2824.1,"compile_done":2979.5,"glue_loaded":3008.5,"worker_ready":3009.7,"engine_start":3009.8,"wasm_instantiated":3028.8,"memfs_ready":3047.9,"main_start":3049.7,"idle":37517.9,"first_pixel":37531.9},"startup":{"firstIdleSeconds":37.51794995117187,"targetSeconds":10},"ui":{"updates":{"total":3,"recent10s":{"n":3,"intervalMedianMs":1.44,"intervalP95Ms":12.04,"perSecond":0.3}},"engineIdles":{"total":1,"recent10s":{"n":1,"intervalMedianMs":null,"intervalP95Ms":null,"perSecond":0.1}},"targetUpdatesPerSecond":120},"input":{"sent":0,"processed":0,"dropped":0,"pending":0,"n":0,"p50Ms":null,"p95Ms":null,"maxMs":null,"lastMs":[],"targetP95Ms":17},"audio":{"underruns":null}}}
```

(数値の小数は読みやすさのため丸めた。ログの全文は `logs/perf-log.jsonl`、`logs/perf-real-engine-result.json`。ページのスクリーンショットは `png/real-engine-perf-page.png`: 実エンジンの画面〔ウェルカムのウィザード、左の Browser 欄が黒い既知の欠け〕と、開いた Performance パネル〔赤い値〕。)

`node app/scripts/perf-log-summary.mjs perf-log.jsonl` の出力:

```
2026-11-02T18:46:36.709Z loopback #0 first-idle emscripten-fruity-v1 first idle 37.5 s | updates 0.3 /s (p95 gap 12 ms) | input p95 - ms (n=0) | underruns not measured | 16 cores 32 GB 800x600@1 | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0x0000C0DE)), SwiftShader driver) | Mozilla/ …
2026-11-02T18:47:06.708Z loopback #1 periodic   emscripten-fruity-v1 first idle 37.5 s | updates 0.6 /s (p95 gap 3404 ms) | input p95 6235 ms (n=8) | underruns not measured | 16 cores 32 GB 800x600@1 | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0x0000C0DE)), SwiftShader driver) | Moz …
2026-11-02T18:47:06.866Z loopback #2 exit       emscripten-fruity-v1 first idle 37.5 s | updates 0.6 /s (p95 gap 3404 ms) | input p95 6235 ms (n=8) | underruns not measured | 16 cores 32 GB 800x600@1 | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0x0000C0DE)), SwiftShader driver) | Moz …
3 records (3 real engine, 0 stand-in)
P1 first idle (target <= 10 s), real engine: 37.5, 37.5, 37.5 s
P2 updates per second (target >= 120), real engine: 0.3, 0.6, 0.6; input p95 (target <= 17 ms): 6235, 6235 ms
P3 audio under-runs: not measured
```

## 5. 取り込みで変わること

- PM が配信している `lift/out-websrv` の `vite preview` は、この差分を取り込んで**再ビルド・再起動するまで変わらない**。再起動後も、既定の `/assets/manifest.json` が実エンジン(`emscripten-fruity-v1`)なので配信は変わらない。変わるのは、**そこに代役を置くと 409 になること**と、**`FL_ASSETS_DIR` の親ディレクトリに `perf-log.jsonl` ができること**(`lift/out*/` は `.gitignore` 済み。差分は `perf-log*.jsonl` も無視に加える)。
- 依頼者がページを開くと、最初の idle(約 40 秒後)に 1 件、その後 30 秒ごとに最大 20 件、閉じる/終了で 1 件、ログに入る。ログには User-Agent・画面の大きさ・GPU 名・コア数が入る(IP は入らない)。
- `FL_PERF_LOG` を指定しないと、配信中のサーバーでは `lift/out-websrv/perf-log.jsonl` になる。

## 6. 出典なしの選択(撤回条件つき)

- **S-web1-allow-list**: 既定のパスでは、`engine.kind` が未知・読めない manifest も 409 にした(指示は「`surface-table-v1` なら 409」)。理由: 将来の代役が新しい名前で素通りしない。**副作用**: 既存のテスト fixture の `manifest.json = {}` を `engine.kind: emscripten-fruity-v1` の manifest に変えた(`MANIFEST_REAL`)。撤回: 新しい実エンジンの kind を足すときは `REAL_ENGINE_KINDS` に足す(意図した変更になる)。
- **S-web1-explicit-variants**: `/assets/manifest.json.br`・`.gz` を明示した要求は保存バイトのまま(判定しない)。ブラウザはそれを manifest として読めない。撤回: 判定が要るなら復号して判定する。
- **S-web1-sec-fetch-site**: 別オリジンの判定で `Sec-Fetch-Site` があればそれを優先し、`Origin` と `Host` の比較は無いときだけにした。理由: `tailscale serve` などの逆プロキシが Host を書き換えても自分のページのビーコンを拒否しない。**未確認**: 実際の `tailscale serve` の背後(ts.net の名前)での動作。撤回: 403 が出るなら、パネルの「beacon」行に `HTTP 403` と出るので、`Origin` の比較を外す。
- **S-web1-one-end**: 終了のビーコンは `exit` と `error` の早い方 1 件(両方の情報を 1 件が持つ)。指示の「30 秒ごと(最大 20 回)」とは別の、終わりの 1 件を足した。
- **S-web1-no-send-after-refusal**: サーバーが 4xx/5xx を返したら以後送らない。
- **S-web1-limits**: ログの上限 64 MiB(507)、manifest の検査上限 2 MiB、エラー本文の表示上限 2000 字、UA 300 字、パネルの更新 1 秒ごと(開いているときだけ)。
- **S-web1-rate-window**: 1 秒あたりの回数の窓は `min(10 s, エンジン開始からの時間)`。10 秒に満たない間は短い窓で割る。

## 7. 未確認・限界

- **依頼者のブラウザからの実測は未取得**(取り込みと再起動の後)。GPU 名は headless Chrome では SwiftShader と出る(本物の GPU 名は依頼者のブラウザで初めて分かる)。`deviceMemory` は Chrome が丸めた値。
- 更新の数え方は「いずれかの窓の描画 1 回」で、エンジンは 1 フレームに複数の窓/矩形を続けて描くので、**更新間隔の中央値は 1〜3 ms のように小さく**出る(実測 1.4・2.0・2.9 ms)。1 フレームの間隔は `engineIdles` の統計とあわせて読む。静止した画面では更新が少なく、120 /s の目標に対しては常に赤になる(P2 は動いている画面の話で、止まった画面の更新数ではない)。
- 入力→更新の遅延は「エンジンが入力を取った後の最初の描画」までで、その描画が入力の結果を映していることの確認ではない。実測では約 3.9〜7.0 s(2 回の実行で各 8 件のポインタ移動)。
- AudioWorklet のアンダーランは**測っていない**(`null`)。G7 が接続されたときに差し込む。
- Worker は `wrangler dev --local` でだけテストした。Cloudflare 上の `DecompressionStream`(gzip/deflate)・`Sec-Fetch-Site` の扱い・Access 経由の `POST` は未確認。
- 同じ文字列の別名(別の綴りの symlink・ハードリンク)は、`manifest.json` という名前で配信されるときだけ内容で判定する(別名のパスは対象外)。Windows の NTFS の代替データストリームなどは対象外(Linux の配信を前提)。
- ビーコンは `credentials: 'omit'`(Cookie を付けない)で送る。Cloudflare Access の背後では Access の Cookie が付かないため、ビーコンは Access に止められて届かない可能性がある(読み。Worker はどのみち保存しないので実害はない)。保存するのは `vite dev`/`vite preview` の配信(アクセス制御の無い tailnet 内の配信)だけ。
- タブが背景のとき、ブラウザがタイマーを間引くので、30 秒ごとが遅れることがある。ページを閉じたときの送信(`pagehide`)はしていない。

## 8. 区分

実測: §3 の件数、§4 の実エンジンのページの値。読み: `tailscale serve` の背後で `remote` が `loopback` になること、`DecompressionStream` の振る舞い。出典なしの選択: §6。未確認: §7。

## 追記(2026-11-05、PM): ビーコンの credentials を same-origin に直した

『no cookie(credentials: omit)』の記述は、Cloudflare Access の背後では誤りだった。gpu1 の測定: R2 `fl-web-cache` の `perf/` は 0 オブジェクト(265 件すべて `cache/`)、本番 Worker の `wrangler tail`(読み取り)で、依頼者の Firefox 159 が 2026-11-04T22:05:48Z に本番ページを読み込んだ後の 7 分以上、`POST /__perf` は 0 件。`omit` では Access の `CF_Authorization` Cookie が付かず、Access が 302 を返し、Worker にビーコンが届かない(読み)。`credentials: 'same-origin'` にした(`app/src/perf/beacon.ts`、テストと mutation `perf-beacon-omits-the-access-cookie` を更新)。同一オリジンで付くのは Access 自身の Cookie だけで、アプリは Cookie を設定しない。Worker は IP・Access の ID・Cookie を保存しない(既存の Worker テストは不変)。PM が測った値(この木): typecheck rc=0、vitest 1,377 passed + 7 skipped、Worker 50/50。確認は未了: デプロイ後に依頼者のセッションから `POST /__perf` が届くこと(`wrangler tail` の読み取りと R2 `perf/` の一覧)。
