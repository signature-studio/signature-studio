# 決定 0220(cache1): 『wasm が毎回ダウンロードされる』の測定と、ページ自身の永続コピー(OPFS)+WebCrypto 検証

**状態: 実装済み(`cache1.diff`)。HEAD 88c7809 に 3 方向マージ済みの木で typecheck・単体・e2e・Worker テストを実行。測定はすべてこの機械(Linux・16 コア・SwiftShader・load 10〜35)で、依頼者の機械(Windows 10 + Firefox 159)ではない。依頼者の機械での値は 0 件。**

## 0. 三点

**何を実測したか**: (1) 本番と同じ Worker のコードの応答ヘッダー(`wrangler dev --local`)と、本番 R2 のメタデータ(ci1 が S3 HeadObject で読んだ表)。(2) Chrome 156 と Firefox 158(headless、geckodriver)で、HTTP キャッシュが何 MB の応答まで 2 回目に使われるか(通常プロファイル/オフ・ザ・レコード/プライベート、`--disk-cache-size` あり・なし)。(3) 実エンジンのページを、キャッシュ有効・プロファイル保持・ブラウザ再起動で『冷たい→温かい』に読み、PerformanceResourceTiming とサーバーのアクセスログで由来を確認し、`download_start→download_done` の中身(読み出し+ブラウザの展開、JS の SHA-256、コピー)を分解。(4) その結果に基づく変更(OPFS の永続コピー+WebCrypto)を base(HEAD)と交互に各 5 サイクル測定。

**結論**: 通常のウィンドウでは Chrome も Firefox も **HTTP キャッシュは効いている**(温かい読み込みでサーバーに来るのは manifest の 304 だけ、4 アセットとも transferSize 0・deliveryType cache)。それでも画面の『ダウンロード』の区間(`download_start→download_done`)が温かくても縮まないのは、この区間が**ネットワークではなく CPU 時間**(ブラウザの brotli 展開 207 MB+ページの JS の SHA-256+コピー)だから(Firefox: HEAD の木で 冷 4.6 s/温 4.8 s)。依頼者の 4.4〜5.1 s はこの値と同じ桁で、『毎回ダウンロードしている』証拠ではない。**毎回本当にダウンロードされる条件**は、シークレット/プライベートウィンドウ(wasm 11.3 MB と DLL pack 14.4 MB だけが毎回来て、seed と glue は HTTP キャッシュから出る。実ページで確認)、DevTools の『キャッシュを無効化』、強制再読み込み、HTTP キャッシュが小さい Chrome(1 エントリの上限は容量の 1/8)。本変更(展開後のバイトを OPFS にハッシュ名で保存し、WebCrypto で検証して使う)で、この機械の download 区間は Firefox 4.6〜4.8 s → 2.2〜2.4 s、Chrome 2.2〜2.3 s → 0.6 s(温)/1.6 s(冷)。決定 0130 の『6 MB 超は HTTP キャッシュされない』は、**オフ・ザ・レコードの文脈で測った値**で、通常のプロファイルには当てはまらない(訂正)。

**未確認**: 本番(Cloudflare Access の背後)で実際に何が起きているか(応答ヘッダー、Content-Encoding、キャッシュの命中)は見られない。依頼者の Windows Firefox 159 での download 区間の内訳と、本変更後の値。Firefox のプライベートでは OPFS が使えず本変更は効かない(下記)。メモリの増加(wasm の 130 MB の一時バッファ)の実機での影響。

## 1. 依頼者の質問への答え

> 『cloudflare workers 環境でも毎回 wasm がダウンロードされているように見えるが、なぜキャッシュされないんだ?ETag なんか使えば、更新時のみ再ダウンロードすることもできるし、帯域幅の節約やオーバーヘッドの削減につながるのに』

### 分かったこと

1. **ETag・304・immutable はすでに正しく働いている**。本番 R2 のオブジェクト(ci1 の HeadObject、`tables/r2-head-object-from-ci1.tsv`)は wasm / glue / seed / dll の 4 つが `Cache-Control: public, max-age=31536000, immutable` と `Content-Encoding: br`、manifest は `no-cache`。Worker は R2 の値に `, no-transform` を足して返し(`app/worker/index.ts`)、強い ETag・`Vary: Accept-Encoding`・`Accept-Ranges: bytes`・`If-None-Match`→304(本文 0 B)・Range→206 を返す(ローカルの本番コード、`tables/worker-headers.md`)。原因は R2 の Cache-Control の不足ではない。
2. **通常ウィンドウのブラウザは実際にキャッシュする**。実エンジンのページ(HEAD の木)を同じプロファイルで 2 回読むと、2 回目にサーバーが見るのは manifest の 304 だけで、4 アセットは transferSize 0・deliveryType cache(Chrome 156 も Firefox 158 も)。1 エントリの上限は Chrome がキャッシュ容量の 1/8(Chromium `net/disk_cache/simple/simple_backend_impl.cc` の `kMaxFileRatio = 8`、実測: 80 MiB で 10 MiB まで命中・11 MiB から外れ)、Firefox が 50 MB(`browser.cache.disk.max_entry_size`、`StaticPrefList.yaml` を 2026-11-04 に取得、実測: 48 MiB 命中・64 MiB 外れ)。Chrome の既定容量は Windows で 200〜320 MiB(`cache_util.cc`: 基準 80 MiB、空き 2 GB 以上で 200 MiB、20 GB 以上で最大 320 MiB)=1 エントリ 25〜40 MiB で、wasm.br 11.3 MB・dll.br 14.4 MB は入る。この機械の Chrome(Linux、空き 693 GB)は 128 MB の応答も入った。
3. **見かけの『ダウンロード』の主因は CPU**。`download_start→download_done` は、ブラウザが brotli を展開してページに渡すバイト列の読み出し+ページの JS 製 SHA-256(`src/bundle/sha256.ts`)+コピーを含む。分解(`tables/cpu-split.md`、HTTP キャッシュ命中時の 4 アセット合計、中央値): Chrome は読み出し 1.03 s+JS の SHA-256 1.16 s+コピー 0.13 s、同じバイトの WebCrypto の digest は 0.22 s。実ページの Σhash(JS の増分ハッシュ、`?hash=stream`)は Chrome 1.13 s、Firefox 3.44 s。Firefox の読み出しは 0.78 s、WebCrypto の digest は 0.25 s。
4. **毎回ダウンロードされる条件を実ページで確認**(`tables/private-modes.md`、同じセッションで 2 回読み): Chrome のオフ・ザ・レコードの文脈(シークレット相当)も Firefox のプライベートブラウズも、2 回目に **wasm=network 11,339,514 B・dll=network 14,408,194 B、seed と glue=http-cache 0 B**。理由は一次資料: Chrome のメモリ内キャッシュは最大 50 MB・1 エントリ 1/8=6.25 MB(`mem_backend_impl.cc` の `kMaxMemoryCacheSize = 10 MiB × 5`、`MaxFileSize() = max_size_ / 8`; 実測 6 MiB 命中・7 MiB 外れ)、Firefox のメモリキャッシュは `browser.cache.memory.max_entry_size` = 5 MiB(実測 4 MiB 命中・5 MiB 外れ)。**決定 0130 の『6 MB 超は HTTP キャッシュされない』(`docs/decisions/0130-wasm-size-and-time.md` の 2 回目の起動の行と結論の 5)は、CDP の `Target.createBrowserContext`(オフ・ザ・レコード)で測った値で、通常のプロファイルの Chrome には当てはまらない。0130 の元の文は直さず、ここで訂正する。**
5. Worker の応答自体は変更不要(下記 7)。

### 分からないこと

- 本番(Access の背後)の応答ヘッダー(特に `Content-Encoding` が br のままか、`Cache-Control` が immutable のままか)と、依頼者の画面で何が起きているか。**ローカルの `wrangler dev` だけの癖**として、`Accept-Encoding` に gzip を含む要求に対し wasm / js / json は『保存された br を展開して gzip で再圧縮』して返す(wasm 21.4 MB、Content-Length なし。octet-stream の flpack は br のまま)。本番のエッジでも同じことが起きるかは不明。
- 依頼者がシークレット/プライベートで見ていたか、DevTools の『キャッシュを無効化』が入っていたか。

### 未決のこと(3 行)

1. DevTools の Network を開き、『キャッシュを無効化(Disable cache)』を**外し**、シークレット/プライベートでないウィンドウで通常再読み込み(Ctrl+R。Ctrl+Shift+R ではない)。
2. wasm の行の Size(Chrome)/Transferred(Firefox)が『(disk cache)』/『cached』なら HTTP キャッシュは効いている(Time が数秒でも展開と SHA-256 の CPU 時間)。『11.3 MB』や『21 MB』と出るなら毎回転送している。
3. その行の応答ヘッダーの `Cache-Control`(immutable が付くか)と `Content-Encoding`(br か gzip か)と Size を教えてください。本変更後は、パフォーマンスパネルの『Start-up assets』の行(由来=network / HTTP cache / own stored copy と各 ms)を見せてもらえれば、次のセッションで原因が分かります。

## 2. 測定の方法と環境

- 機械: Linux 6.x、16 コア、SwiftShader(GPU なし)、Chrome 156.0.8078.47、Firefox 158.0(snap)、geckodriver 0.37.1、Node 24.19。load は各測定に併記(他の担当の測定と同居のため 10〜35)。**壁時計の絶対値は load に左右される。比較は同じサイクル内で交互に走らせた変種どうし**。
- 実エンジンのページは `lift/out-websrv/assets` を指す**自分の preview サーバー(別ポート 4962/4963/4964、アクセスログ付き)**。既定の入口(5173)には触っていない。base = HEAD 0235dff のビルド、new = 本変更。
- 道具(`tools/` と `app/test/e2e/`): `cache-probe-server.mjs`(大きさ別の応答、本番と同じヘッダー、アクセスログ)、`cache-probe-chrome.mjs` / `cache-probe-firefox.mjs`、`cache-real-run.mjs`(実ページの冷温サイクル、変種の交互、PerformanceResourceTiming、ログ)、`cache-cpu-split.mjs`、`cache-compile-probe.mjs`、`cache-brotli-probe.mjs`、`cache-private-run.mjs`、`cache-headers-worker.mjs`、`asset-cache.mjs`(e2e)、`preview-logged.mjs`、`tools/summarize-matrix.mjs`。

## 3. 事実の表

### 3.1 Worker の応答(ローカル、本番コード、R2 に本番と同じメタデータを置いた)— `tables/worker-headers.md`

| アセット | Cache-Control(送出) | ETag | Content-Encoding | Vary | Content-Length | Accept-Ranges | Age | 条件付き(同じ ETag) |
|---|---|---|---|---|---|---|---|---|
| engine.…wasm | `public, max-age=31536000, immutable, no-transform` | 強い(md5) | br(wrangler dev では gzip、上記) | Accept-Encoding | 11,339,214(HEAD) | bytes | なし | 304、本文 0 B |
| engine-glue.…js | 同上 | 強い | br(同上) | 同上 | 57,558 | bytes | なし | 304 |
| seed.…flpack | 同上 | 強い | br | 同上 | 3,326,299 | bytes | なし | 304 |
| dll.…flpack | 同上 | 強い | br | 同上 | 14,407,894 | bytes | なし | 304 |
| manifest.json | `no-cache, no-transform` | 強い | (wrangler dev では gzip) | 同上 | 3,282 | bytes | なし | 304 |

Age は Worker が付けない(Cloudflare の CDN が付ける物)。ETag は本番 R2 の HeadObject と同じ値(`6938b265…` ほか=R2 が返す MD5)。

### 3.2 本番 R2(ci1 の HeadObject 6 本、読み取りのみ)— `tables/r2-head-object-from-ci1.tsv`

wasm / glue / seed / dll: `CacheControl = public, max-age=31536000, immutable`、`ContentEncoding = br`。manifest 2 本: `no-cache`、ContentEncoding なし。ContentLength は保存された br のバイト数。

### 3.3 HTTP キャッシュの上限(`tables/chrome-http-cache.md`、`tables/firefox-http-cache.md`)

| 場面 | 2 回目に使われる最大の応答 | 命中・外れの実測 | 一次資料 |
|---|---|---|---|
| Chrome 156、通常プロファイル(この機械) | 少なくとも 128 MB | 128 MB 命中 | `cache_util.cc`(Linux は基準 320 MiB、上限 1,280 MiB)、`simple_backend_impl.cc`(1 エントリ=容量/8) |
| Chrome、`--disk-cache-size=80 MiB` | 10 MiB | 10 MiB 命中・11 MiB 外れ | 同上(80/8) |
| Chrome、Windows の既定(**推測**: 空き 2 GB 以上) | 25〜40 MiB | 未測定(Linux の機械) | `cache_util.cc`: Windows は `percent_relative_size = 100`(80 MiB 基準、200〜320 MiB) |
| Chrome、オフ・ザ・レコード(CDP の `createBrowserContext`/シークレット相当) | 6 MiB | 6 MiB 命中・7 MiB 外れ | `mem_backend_impl.cc`: 最大 50 MB、`MaxFileSize() = max_size_/8` |
| Firefox 158、通常 | 50 MB | 32 MiB 命中、48 MiB は 3 回目から命中、64 MiB 外れ | `StaticPrefList.yaml`: `browser.cache.disk.max_entry_size` = 50 MB(『容量の 1/8 より大きい物はキャッシュしない』) |
| Firefox 158、プライベート | 5 MiB 未満 | 4 MiB 命中・5 MiB 外れ | `browser.cache.memory.max_entry_size` = 5 MiB |

取得した一次資料の写し: `logs/chromium-cache_util.cc`、`logs/chromium-simple_backend_impl.cc`、`logs/chromium-mem_backend_impl.cc`、`logs/firefox-StaticPrefList-excerpt.txt`(2026-11-04 に main ブランチから)。

### 3.4 ページ側の fetch(`app/src`)

`loader.ts` の `fetchChecked` は `fetch(url, { signal })` を既定のキャッシュ規則で 1 回ずつ呼ぶ(`cache:` は付けない)。同じ URL を 2 回取る箇所はない(診断の `worker-diag.ts` の `no-store` を除く)。

### 3.5 tailnet(`tailscale serve` → `vite preview`)

`Cache-Control: public, max-age=31536000, immutable`・強い ETag・`Content-Encoding: br`・`Vary`・Range 206・条件付き 304 がすべて保たれる(この機械から HEAD/条件付き GET)。Cloudflare 本番は Access の背後で見られない。

## 4. なぜ『download』が縮まないか(`tables/cpu-split.md`、`tables/startup-matrix.md`)

HEAD の木の実ページ(5 サイクルの中央値): Firefox 冷 4,575 ms(4,132–4,660)/温 4,806 ms(3,929–5,977)、Chrome 冷 2,154/温 2,306 ms。温かい読み込みでサーバーが見るのは manifest の 304 だけ。依頼者の Firefox 159 のビーコン(`perf-log.jsonl`、tailnet、13 セッション)は 4.4〜5.1 s。**同じ桁で、キャッシュが効いていても出る値**。内訳(上記 3 項)から、Firefox の JS 製 SHA-256 が約 3.4 s を占める。

## 5. 実装

**場所**: `app/src/bundle/assetstore.ts`(新規)、`assets-report.ts`(新規)、`loader.ts`(追加のみ+3 行の置換)、`main.ts`(配線)、`perf/beacon.ts`・`perf/panel.ts`・`ui/phases.ts`・`shared/perf.ts`・`scripts/perf-log-summary.mjs`(表示と検査)。`lift/`・Worker・R2・デプロイは変更なし。

**方式**:
1. マニフェスト(`no-cache`+ETag、約 3 KB)を毎回取る。そこに名指された各アセットの SHA-256 をキーに、OPFS の `/fl/cache/assets/<sha256>.<wasm|flpack|js>`(展開後のバイト列、1 アセット 1 ファイル)を探す。**あれば**: ネットワークに出さず、ファイルを読み、**WebCrypto の `crypto.subtle.digest` で SHA-256 を照合**してから使う(wasm はコンパイルを同時に走らせ、照合に失敗したらモジュールを捨てる=検証前に実行しない原則を維持)。サイズが違う・ハッシュが違う・読めないファイルは『無かった』ことにして削除し、従来どおりネットワークから取る。**なければ**: ネットワークから取り、読みながら (a) `WebAssembly.compileStreaming` の流れ、(b) 1 本のバッファ、(c) 保存先(4 MiB ごとのコピー)へ配る。全バイトが揃ったら**サイズと SHA-256 を WebCrypto で一括照合**し、通った後にだけ保存をコミットし、呼び出し側へ返す(コミットはバックグラウンド)。
2. 保存は `createWritable()`(`close()` まで見えない)なので、存在するファイルは照合済みの完全な物。保存が失敗(容量不足・シークレット・OPFS なし・他タブが書き込み中)しても黙って従来の経路に落ち、理由を固定語彙のコードで記録する(ビーコンと画面)。
3. 掃除: 全アセットの保存が落ち着いた後、現行マニフェストが名指さない `<64hex>.<ext>` を消す。他のページが使っているファイルは**共有 Web Lock `fl-asset:<sha>`**(各ページが生存中保持)で守る。排他ロックを `ifAvailable` で取れた物だけを消す。Web Locks が無い環境では何も消さない。自分の物でない名前のファイルは触らない。
4. `navigator.storage.persist()` は呼ばない(Firefox は人に尋ねる)。`persisted()` と `estimate()` を報告する。Files ウィンドウの既存のボタンで依頼者が保護できる。
5. `?cache=0` = 従来の経路そのもの(保存なし、到着中に JS で SHA-256)。`?hash=stream|subtle` はハッシュだけを選ぶ。ブラウザに OPFS が無い・非セキュアなら保存なし。

**ビーコンとパネル(仕事 3)**: ビーコン版 3 の加算として任意の `assets` ブロック(`shared/perf.ts` の `assetsBlock` が検査): 4 アセットそれぞれの `kind`(wasm/dll/seed/glue)、`sha`(先頭 8 桁)、`bytes`、`wire`(encodedBodySize)、`transfer`、`origin`(`network` / `http-cache` / `storage-cache`)、`store`(off/hit/writing/stored/skipped/failed)、`fail`(固定コード unsupported/quota/security/locked/io/other/verify/busy)、`ms`(headers/read/hash/copy/parse/store/total/compile)、`storage`(usedMiB/quotaMiB/persisted)、`clean`(removed/kept/busy/failed)。名前・パス・URL・エラー文は載せない。パネルに『Start-up assets』の要約行と 1 アセット 1 行、Download 段の詳細に『; from: own stored copy ×4』。`perf-log-summary.mjs` に『由来別(cold / HTTP cache / own stored copy / mixed)の表』を追加(ログには個人の識別が無いので 2 回目以降は由来で見分ける)。

**lift の忠実性の方針(`docs/policies/lift-fidelity.md` の点検表)**: この変更はページがアセットを取得・検証する部分だけで、FL の挙動の代行・補正ではない(窓・画素・クラス名・位置の分岐なし)。(b) 外部 API の契約の実装でも (a) 余計なロジックの除去でもなく、**『変換と通信の削減』**に当たる(JS での 207 MB の SHA-256 をネイティブに、毎回の brotli 展開 207 MB と転送 29 MB を除去)。観測(ビーコン・パネル)は固定語彙・低頻度で、`?beacon=0` で送らない既存の規則に従う。

## 6. 測定: 冷たい起動と温かい起動(Chrome 156 / Firefox 158、各 5 サイクル、変種は交互)— `tables/startup-matrix.md`

(下の表は `tools/summarize-matrix.mjs` の出力。first idle・download 窓・compile_done は中央値(最小–最大)、単位 ms。『load』は各読み込み開始時の 1 分 load の中央値。**壁時計は load に左右されるので変種どうしの比較に使い、絶対値は依頼者の機械の値ではない**。)

| browser | variant | load | n | first idle ms, median (min-max) | download_start→done ms, median (min-max) | compile_done ms, median | Σ read ms | Σ hash ms | Σ copy ms | origins | load avg (median) |
|---|---|---|---:|---|---|---:|---:|---:|---:|---|---:|
| chrome 156.0.8078.47 | base | cold | 5 | 9501 (7166-13419) | 2154 (1949-2640) | 2390 | - | - | - | - | 16.9 |
| chrome 156.0.8078.47 | base | warm1 | 5 | 8685 (6876-14462) | 2306 (1708-3747) | 2504 | - | - | - | - | 17.3 |
| chrome 156.0.8078.47 | newstream | cold | 5 | 7441 (6922-23374) | 2098 (1820-4943) | 2104 | 5052 | 1134 | 110 | network | 18.8 |
| chrome 156.0.8078.47 | newstream | warm1 | 5 | 5702 (5201-12287) | 703 (566-2447) | 811 | 628 | 291 | 0 | storage-cache | 20.1 |
| chrome 156.0.8078.47 | new | cold | 5 | 8202 (5552-13815) | 1565 (1005-2162) | 1756 | 2748 | 232 | 125 | network | 17.2 |
| chrome 156.0.8078.47 | new | warm1 | 5 | 6776 (5505-13765) | 621 (591-754) | 788 | 547 | 309 | 0 | storage-cache | 17.0 |
| firefox 158.0 | base | cold | 5 | 12214 (10479-12992) | 4575 (4132-4660) | 6013 | - | - | - | - | 27.8 |
| firefox 158.0 | base | warm1 | 5 | 12353 (10618-13650) | 4806 (3929-5977) | 5795 | - | - | - | - | 27.0 |
| firefox 158.0 | newstream | cold | 5 | 11355 (9978-15619) | 4511 (4292-6952) | 5618 | 13877 | 3439 | 207 | network | 26.3 |
| firefox 158.0 | newstream | warm1 | 5 | 8433 (8073-9622) | 2152 (1886-2522) | 2522 | 993 | 306 | 0 | storage-cache | 26.4 |
| firefox 158.0 | new | cold | 5 | 9426 (6209-11130) | 2191 (1502-2594) | 2753 | 4532 | 570 | 184 | network | 26.4 |
| firefox 158.0 | new | warm1 | 5 | 9455 (7975-10776) | 2352 (1894-3441) | 2613 | 1051 | 433 | 0 | storage-cache | 25.4 |

**同じサイクルの中での差(new − base、ms。変種は 1 サイクルの中で順に走るので load がほぼ同じ)**— `tables/paired.md`:

| browser | kind | variant vs base | cycles | Δ first idle ms per cycle | median Δ first idle | negative in | Δ download window ms per cycle | median Δ download window |
|---|---|---|---:|---|---:|---:|---|---:|
| chrome 156.0.8078.47 | cold | new | 5 | -1299, -1614, -1673, -2548, 397 | -1614 | 4/5 | -817, -1031, -754, -685, 8 | -754 |
| chrome 156.0.8078.47 | warm1 | new | 5 | -1909, -647, 6188, -5203, -8957 | -1909 | 4/5 | -1600, -1117, -1112, -2494, -3126 | -1600 |
| chrome 156.0.8078.47 | cold | newstream | 5 | -2060, -244, 15685, -3302, -6394 | -2060 | 4/5 | -284, -215, 2994, -312, -331 | -284 |
| chrome 156.0.8078.47 | warm1 | newstream | 5 | -3050, -1674, 4711, -5954, -8760 | -3050 | 4/5 | -1700, -1142, 718, -2545, -3041 | -1700 |
| firefox 158.0 | cold | new | 5 | -4270, -1862, -1821, -3724, -2788 | -2788 | 5/5 | -2630, -2066, -2825, -2385, -2299 | -2385 |
| firefox 158.0 | warm1 | new | 5 | -2643, -3278, -1577, -2497, -5105 | -2643 | 5/5 | -2913, -3009, -488, -1679, -3898 | -2913 |
| firefox 158.0 | cold | newstream | 5 | -501, -1927, 3566, -1304, 396 | -501 | 3/5 | 160, -205, 2323, -64, 178 | 160 |
| firefox 158.0 | warm1 | newstream | 5 | -2185, -5071, -2731, -3192, -5577 | -3192 | 5/5 | -2622, -3556, -1407, -2144, -3853 | -2622 |

読み方:
- **download 窓**(`download_start→download_done`)は base の温かい経路(HTTP キャッシュ)で冷と同じ(Firefox 4.8 s/4.6 s、Chrome 2.3 s/2.2 s)。new では Firefox 2.2〜2.4 s、Chrome 冷 1.6 s/温 0.6 s。
- **compile_done**: Firefox は wasm のコンパイルが温かい経路の下限(`compile` 1.1〜1.3 s、`tables/compile-cache.md`: 3 回の読み込みで短くならない=**コンパイル済みコードのキャッシュは Chrome にも Firefox にも観測できない**)。Chrome の `WebAssembly.compile` は遅延コンパイルで 0.14〜0.2 s。
- **first idle**: Firefox(load 26〜28)で base 12.2/12.4 s → new 9.4/9.5 s。Chrome(load 17〜19)で base 9.5/8.7 s → new 8.2/6.8 s。範囲が広い(load の揺れ)ので、差は『読み』として扱う。first idle の下限はエンジンの起動(約 4〜6 s)で、本変更では変わらない。
- 冷たい経路のハッシュ(`newstream` と `new` の冷): WebCrypto 一括は JS 増分より速い(Chrome download 窓 2.10→1.57 s、Σhash 1.13→0.23 s。Firefox 4.51→2.19 s、Σhash 3.44→0.57 s)。代償は wasm の読み取り中に 130 MB の一時バッファを持つこと。
- 依頼者の P1(10 秒)への効き: Firefox の download 窓が約 2.4 s 短くなる(この機械の測定。依頼者の機械は 8 スレッドの Windows で、同じ桁の download 窓 4.4〜5.1 s を記録しているので、**同程度の短縮が見込めるが、依頼者の機械では未測定**)。first idle の約 10.4 s のうち download 窓が約 47% だったので、**読み**で 10.4 s → 8 s 前後。

### 6.1 最終ビルド(HEAD 88c7809 にマージした木)の smoke と、メモリ — `tables/final-smoke.md`

マージ後の木のビルド(ポート 4964)を HEAD 0235dff のビルド(4962)と交互に、3 サイクル(load 18〜21)。**マージ前の木で測った上の表と同じ傾向**(Chrome 冷 2.04→1.30 s・温 1.94→0.60 s、Firefox 冷 4.31→1.36 s・温 3.79→1.62 s、download 窓の中央値)で、first idle の中央値は Chrome 冷 7.3→6.7 s・温 7.7→6.3 s、Firefox 冷 10.8→6.6 s・温 10.7→7.1 s(3/3 サイクルすべて短縮)。

**メモリ(Chrome のみ、プロセス全体の常駐メモリの最大、1 秒おきの標本、3 サイクルの中央値)**: 冷たい読み込み base 2,103 MiB → 本変更 2,219 MiB(+116 MiB、wasm 取得中の 130 MB の一時バッファに合う)、温かい読み込み 1,881 → 1,916 MiB(+35 MiB)。Firefox は未測定。

### 6.2 コンパイル済みコード・保存形式・プライベートの実測の置き場

- `tables/compile-cache.md`: 3 回の読み込みで `compileStreaming(fetch)` も `compile(buffer)` も速くならない(Chrome 387→567→746 ms と 136→184→200 ms〔遅延コンパイルで読み出しが主〕、Firefox 1126→964→1063 ms と 1591→1015→1144 ms)=コンパイル済みコードのキャッシュは観測できない(『効かない』ではなく『この方法では観測できない』)。
- `tables/store-format-brotli.md`: §8。
- `tables/private-modes.md`: §1 の 4。

## 7. Worker 側(仕事 5): 現状の維持と提案

- **維持を確認した物**: ETag(強い、R2 の MD5)・`If-None-Match`→304・Range/If-Range・`Cache-Control` の R2 優先・immutable。Worker テスト 43 件(`test/worker`)は変更なしで通る(下記)。
- **提案(決めない。PM の判断)**:
  1. **変更不要**。HTTP キャッシュが効かない原因は Worker にない(シークレット/プライベートの上限、DevTools の設定、強制再読み込み)。
  2. Worker の Cache API は**入れない**: 効果を測れない(ブラウザの HTTP キャッシュが主、利用者は 1 人、R2 の転送は無料、Access の背後で本番の効果を測れない)。
  3. 任意の小改善: `serveAsset` は `head()` の後に `get()` を呼ぶので 1 リクエストあたり R2 の読み取りが 2 回(304 は 1 回)。`get()` に `onlyIf: { etagDoesNotMatch }` を使えば 1 回にできる。R2 の Class B 操作の単価が小さく(利用者 1 人)、挙動(412 の扱い)が変わるので**入れない**。
  4. 本番でのみ起きうる再エンコードの確認(上記『分からないこと』)は、依頼者の DevTools(3 行)で。

## 8. 保存形式の選択(OPFS に展開後か brotli のままか)— `tables/store-format-brotli.md`

**展開後のバイトを保存する**。理由(実測): (1) **Chrome 156 は `DecompressionStream('brotli')` を持たない**(`Unsupported compression format: 'brotli'`)。brotli のまま保存すると Chrome で展開できず、JS/wasm の brotli 展開器を足す=依頼に無い依存になる。(2) Firefox 158 は持つが、保存した brotli(29 MB)を展開する時間は 4 アセットで 1.22 s、展開後(207 MB)を読むのは 0.63 s で、展開後の方が速い。(3) 展開後の flpack1 / flseedp1 はメンバーの offset/size の表をファイル内に持つので、memfs1(決定 0221)が必要なメンバーだけをファイルから直接読める。代償はディスク 207 MB(197 MiB)(brotli なら 29 MiB)。

## 9. memfs1(決定 0221)との擦り合わせ

保存形式は展開後の 1 ファイル(上記)。`LoadedPack.stored: { path, size, sha256 } | null` を足した(保存コピーを検証して使ったときだけ non-null。冷たい経路はコミット前なのでバイト列を使う)。Worker 側の `OpfsSource` / `MemorySource` / `openPackSource` は memfs1 が `hostfiles.ts` に書く。プロトコルの assets の形(`Uint8Array`)は変えていない。`FileSystemSyncAccessHandle` は排他なので、別タブが開いている場合は MemorySource に落ちる設計を依頼した。

## 10. テスト

測定は最新 HEAD 88c7809 に 3 方向マージした木(`lift/out-cache1/src2`)。

| テスト | 結果 | 備考 |
|---|---|---|
| `npm run typecheck`(4 つの tsconfig) | rc=0 | |
| vitest 単体 | **1,505 passed + 7 skipped**(68 ファイル、うち 1 ファイル skipped) | 変更前の同じ HEAD の木: 1,463 passed + 7 skipped(65 ファイル)。増えたのは本変更の 42 件(`assetstore.test.ts` 21、`assets-report.test.ts` 16、`assets-summary.test.ts` 5)。`loader.test.ts` ほか既存は無変更で通る |
| e2e 全体(`node test/e2e/run.mjs`、dev/preview/worker、スタンドインのバンドル。実エンジンのバンドルは指定せず) | **1,256 / 1,256** | `logs/e2e-full.log`。既定で保存コピーが ON のページで通る(保存の経路を通る) |
| Worker テスト(`node test/worker/run.mjs`) | **43 / 43** | `logs/worker-tests.log`。Worker は無変更 |
| 本変更の e2e(`test/e2e/asset-cache.mjs`、Chrome + Firefox の実 OPFS、スタンドインの小さなバンドルで保存の仕組みをテスト。エンジンのテストではない) | **29 / 29**(2 回、別の実行でも 29/29) | 冷→温(サーバーに資産の要求 0)、1 バイト変えたコピーは捨てて取り直し、`?cache=0`、`?hash=stream`、別の版の掃除、2 タブの Web Lock(Chrome)。途中で 2 件落ちた(テストの仮定の誤り 2 件: 破損したコピーの再取得は HTTP キャッシュが答える/掃除の完了を待たずに報告を読んだ)を直して、各 2 回通した |
| 変異検査(本変更の 11 件、unit 層) | **11 / 11 検出** | `logs/mutation-0220.log`(10/11 検出。1 件目 `store-write-uses-a-view-not-a-copy` は書き込みが即時に走るフェイクでは検出できず MISSED → フェイクの書き込みに遅延を足してテストを直し、`logs/mutation-0220-b.log` で DETECTED、コピーと木は同一・単体は緑)。変異: 保存ブロックを view で渡す/保存コピーのハッシュ未検査/網経路のハッシュ不一致の無視/保存コピーのサイズ未検査/他ページのロックの無視/現行版の削除/容量未検査/従来経路のメンバー検査の省略/`fail` に任意の文字列/保存由来を network と報告/割り当て失敗の未処理。既存の変異検査の全体は再実行していない |
| 実エンジンのページ(5173 以外の自分の preview) | 全 84 回の読み込み(§6 の表 60 回 + 最終ビルドの smoke 24 回)が first idle に到達、保存の失敗 0 | §6。ビーコンの `assets` ブロックは 57 件すべて `beaconProblems` を通る(`tools/check-beacons.mjs`) |

ビーコンの大きさ: `assets` ブロックは約 1.1 KB(first-idle ビーコン 5.1 KB → 6.2 KB、上限 16 KB)。

## 11. 出典のない選択(S-番号)

| 番号 | 選択 | 理由と代替 |
|---|---|---|
| S-cache1-1 | 保存先 `/fl/cache/assets/<sha256>.<ext>` | PM の指示で OPFS に統一。代替: Cache Storage(PM が却下)、IndexedDB(1 値 126 MB が上限、決定 0161)。Files ウィンドウの根に見える(207 MB(197 MiB))=依頼者が消せる。`/fl/state`・`/fl/license` の保護リストには入れていない(消しても再取得するだけ) |
| S-cache1-2 | 展開後を保存 | §8。代替: brotli のまま(Chrome で展開できない) |
| S-cache1-3 | 書き込みブロック 4 MiB(コピー) | 1 回の write が大きいほどメッセージが減り、小さいほどメモリが減る。view でなくコピーなのは Worker への transfer で detach されるため(実機で見つけた不具合)。代替: 1 MiB / 16 MiB(未測定) |
| S-cache1-4 | 書き込み前の容量余裕 1.25 倍 | `estimate()` の誤差と並行書き込みの余裕。代替: 余裕なし(失敗してから quota を記録) |
| S-cache1-5 | `persist()` を自動で呼ばない | Firefox は人に尋ねる。代替: 初回に呼ぶ(依頼者がボタンで押せる) |
| S-cache1-6 | 既定のハッシュ = WebCrypto 一括 | §6。代替: JS 増分(`?hash=stream`)。メモリ +130 MiB の一時バッファが代償 |
| S-cache1-7 | 新経路ではパックのメンバーごとの JS ハッシュを省く | パック全体の SHA-256 が manifest と一致していて、索引もメンバーも覆われる。メンバーの再ハッシュは約 76 MB の JS ハッシュの二重払い。従来経路(`?cache=0`)は従来どおりメンバーも検査。代替: 維持(Firefox で約 1 s 余計) |
| S-cache1-8 | 掃除は全アセットの保存が落ち着いた後(最長 120 s 待つ) | 書き込み中のファイルを消さない。代替: 起動直後 |
| S-cache1-9 | Web Locks が無い環境では掃除しない | 他ページの使用を知り得ない。代替: 常に消す(別タブで壊す危険) |
| S-cache1-10 | 保存のコミットを待たずに起動を進める | 起動(P1)を遅らせない。代替: 待つ(保存を確実にするが冷たい起動が遅れる)。ブラウザを 1 秒以内に閉じると保存は失われる(次回は冷たい) |
| S-cache1-11 | 由来の分類は `deliveryType === 'cache'`(無ければ transferSize 0) | ブラウザ自身の言葉を優先。代替: transferSize だけ(再検証の 304 〔数百 B〕を network と誤る) |
| S-cache1-12 | 失敗コードの語彙 | 固定リストのみ。エラー文は送らない(`files` ブロックと同じ規則) |
| S-cache1-13 | 既定 ON(PM の方針) | `?cache=0` で従来経路。判断点として明記 |

## 12. 限界と自己申告

- **範囲**: 測定は 1 台・Linux・Chrome 156 と Firefox 158 の headless・GPU なし・load 10〜35(他の担当と同居)。サイクル数は各 5 で、first idle の中央値は範囲が広い(Chrome 5.5〜14 s)。download 窓と Σ の ms は安定している。依頼者の機械(Windows 10 + Firefox 159、8 スレッド)は 0 件。**依頼者の機械での短縮は推測**(download 窓 4.4〜5.1 s に対し、この機械の Firefox で 4.6〜4.8 s → 2.2〜2.4 s)。
- **本番(Cloudflare Access の背後)は見ていない**。Worker のヘッダーはローカルの `wrangler dev` の値と、ci1 の本番 R2 の HeadObject の値で、エッジが付け替える物(`Age`、再エンコード)は未確認。
- **Firefox のプライベートブラウズでは OPFS が使えない**(`getDirectory()` が SecurityError)ので、保存コピーは効かず、従来の経路に落ちる(失敗コード `security` を記録)。wasm 11.3 MB と DLL pack 14.4 MB は HTTP キャッシュにも入らない(5 MiB 未満だけ)ため、プライベートでは毎回転送される。**本変更では直らない**(代替: プライベートでは配信形式を分割して 5 MiB 未満にする案があるが、依頼に無く未測定)。Chrome のシークレット相当では OPFS は使え、2 回目は保存コピーから出た(実測)。
- 冷たい経路(1 回目、保存コピーなし)の 29 MB の転送は変えていない(帯域幅の節約は 2 回目以降)。冷たい経路では wasm の取得中に 130 MB の一時バッファが増える(`?cache=0` か `?hash=stream` でも保存を使うなら同じ)。メモリの実測は Chrome の RSS ピークのみ(§6 の追記)で、Firefox は未測定。
- **ブラウザを閉じる前に保存のコミットが終わらないと、次回は冷たい**(冷たい読み込みで書き込み約 2〜5 s。コミットは起動をブロックしない)。
- 保存は使ったバイトの完全性(SHA-256)を守るが、OPFS の中のファイルを他のコードが書き換える攻撃は防がない(同一オリジンの任意コードは元から何でもできる)。照合は毎回行う。
- 計測用の道具(preview のアクセスログ、プローブサーバー)は `app/test/e2e/` に置いた。本番の経路には入っていない。
- **自己申告**: (1) 存在しない `bash` ツールを 1 回呼んだ(shell として実行され、副作用なし)。(2) `sleep` を 2 回打って拒否された(実行されていない)。(3) 作業の途中で、`git diff --no-index` の全ファイル名一覧(数百行)を端末に出してしまった(`head` で切るべきだった。副作用なし)。(4) 中間報告 2 の『デッドな原因の候補』は『有力な原因の候補』の誤記。(5) 作業の開始時に基準とした HEAD(0235dff)が途中で進んだため、3 方向マージ(衝突 2 件、両方とも同じ位置への追記で両方を残した)をして、マージ後の木でテストをやり直した。測定(§6 の表)はマージ前の木(HEAD 0235dff + 本変更)のビルドで、最終ビルドでは smoke(§6 の追記)だけを取り直した。(6) 掃除を忘れて `lift/out-cache1/work` が一時 1.3 GB になった(終了時に整理)。

---

## PM の決定と取り込み(2026-11-04)

- **取り込み**: `cache1.diff`(32 ファイル、`app/` のみ。`-` 行 15 本はすべて 1 行の置換で PM も全部読んだ)を HEAD 88c7809 に当てた。clean(offset・fuzz なし)。PM 実測: 型検査 rc=0、vitest 1,505 passed + 7 skipped(基準 1,463 + 7)、Worker 43/43、e2e 1,256/1,256。`lift/` に触れていないので生成 C・全体テスト・a4 は再実行していない。
- **依頼者の質問への答え(PM の要約)**: 「毎回 wasm がダウンロードされ、なぜキャッシュされないのか。ETag で更新時のみ再ダウンロードできるはず」→ (1) ETag・`If-None-Match`→304・`immutable` は Worker と R2 のメタデータの両方で入っており、**通常のウィンドウでは Chrome も Firefox も HTTP キャッシュは効いている**(温かい読み込みでサーバーに来るのは manifest の 304 だけ)。(2) ただし画面の『ダウンロード』区間(約 4.4〜5.1 秒)の中身は、ネットワークではなく CPU(ブラウザの展開、JS 製 SHA-256、コピー)で、キャッシュが効いても縮まない。(3) 毎回転送される条件は、Chrome のシークレット(メモリ内キャッシュ、1 エントリ 6.25 MB まで)と Firefox のプライベート(5 MiB まで)で、wasm 11.3 MB と DLL パック 14.4 MB が入らない。依頼者がどちらかで見ていたかは未確認。(4) 決定 0130 の『6 MB 超は HTTP キャッシュされない』はオフ・ザ・レコードの文脈で測った値で、通常のウィンドウには当てはまらない(訂正)。
- **変更の採用**: 展開後バイトを OPFS(`/fl/cache/assets/<sha256>.<ext>`)に保存し、WebCrypto で SHA-256 を照合して使う。既定 ON、`?cache=0` で従来経路。理由: HTTP キャッシュの規則・サイズ上限・DevTools の設定に左右されず、展開と JS のハッシュの CPU 時間を消す(Firefox で first idle が同サイクル内で約 −2.6〜−2.8 秒、Chrome の温かい download 区間 2.3 → 0.6 秒。この機械の値で、依頼者の機械は推測)。方針(`docs/policies/lift-fidelity.md` 3)の『変換と通信の削減』に当たる。余計なロジックの分岐は足していない。
- **判断点への回答**: 既定 ON(ディスク約 207 MB、1 版分。Files 窓の根に `fl/cache/assets/…` が見える)は承認。`persist()` を自動で呼ばない点は承認(依頼者が必要なら Files 窓から)。wasm 取得中の一時バッファ 130 MB は許容。新経路でパックのメンバーごとの JS ハッシュを省く点は、パック全体の SHA-256 が manifest と一致する限り承認。
- **未確認**: 本番(Cloudflare の Access の背後)で実際のヘッダーがどうなるか、依頼者のブラウザでの値、Windows の Chrome の既定キャッシュ容量(一次資料からの読み)、Firefox のメモリ、Firefox のプライベートでは OPFS が使えず従来経路になる(改善なし)。依頼者には DevTools で wasm の行の Size 列と『Disable cache』、シークレット/プライベートでないか、を確認していただく(3 行は本文 §1)。
- **memfs1(決定 0221)との接続**: 展開後 1 ファイルを OPFS に置き、memfs1 が `OpfsSource` を書いて JS ヒープの 76 MB を消す。両方が `app/src/bundle/` と `app/src/engine/fruity.ts` を触るので、後から入る方が載せ替える。
