# survey-c2-worker-split: C-2(画面の描画が音声処理を圧迫しない: Worker 分離の現状)を、この runner で測る(測定のみ。製品は変えない)

## 報告の先頭の規則
- 報告の 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。実行していなければ「実エンジン: 実行なし(理由)」と書く。
- 測定の値には「runner の値。この機械とは別枠」と書き、runner の種類(nproc と loadavg の値)を添える。
- 代役・再生・事前に作った録音や画像を使わない。数字は、この実行で得た実測だけを書く。各数字に、出典の出力の行(JSON の行)を付ける。推定は「推定」と書き、実測と分ける。
- 「達成」「直った」「音が途切れない」と書かない。判定は「受理」「差し戻し」「未確認」のいずれかにする。耳の確認はこの作業では行わない。

## 規則の要約(この task で守ること)
- 製品(app/・lift/・ci/・tools/・patches/・.github/)を変えない。変更は queue/findings/survey-c2-worker-split.md だけ。
- 製品の作業(ビルド・実行)はこの runner の中だけで行う。読むだけの手順は、その旨を書く。
- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(この task では何も足さない)。
- Python は使わない。ベンダー名、Cloudflare のアカウント ID・ホスト名・ETag(32 桁の 16 進)、絶対パスを書かない。パスは repo の中の相対パスで書く。
- commit は環境の git の設定のまま行う(git config・--author・GIT_AUTHOR_*・git -c を使わない)。commit に Claude の行を付けない。ブランチを作らない。push はしない(判定の段が push する)。
- .git/config、opencode.jsonc、.opencode を読まない。git の内部命令(update-index・hash-object・update-ref・commit-tree・write-tree・bundle・for-each-ref を含む)を使わない。force は使わない。
- Secrets と Variables は読まない。wrangler と cf の CLI は使わない。R2 の鍵はこの実行には無い。
- sleep、timeout、nohup、disown、末尾の & は使わない。
- 一時ファイルは repo の中の .tmp-* に置き、終わったら消す。リポジトリの外(/tmp を含む)に書かない。

## 目的と根拠
台帳 L-033 の (2): 「画面の描画処理が音声処理を圧迫している。Worker 分離は完了していないのか」。queue/plan.md の C-2 を参照。優先は P3(Song Mode で音が途切れない)。コードの読みでは、エンジン(wasm)・デスクトップの合成と提示・音声の書き込みが、同じ fl-engine の Worker で動く。音声の出力だけが AudioWorklet にある。この task は、スレッドの配置を読みと実測で確かめ、描画の所要が音声の書き込みと同じスレッドで走ることを数字にする。製品は変えない。

## 読む物(実在を確認済み)
- app/src/engine/client.ts(75 行: fl-engine の Worker を作る。86 行: 描画の canvas を OffscreenCanvas にして渡す)
- app/src/engine/worker.ts(36〜39 行: 描画の canvas と提示。220〜232 行: エンジンの idle での描画。277 行: エンジンの run。338〜357 行: 描画の rAF の loop。381 行: deskTick)
- app/src/engine/desk.ts(3 行: 描画の提示の注釈。80 行: compose の呼び出し)
- app/src/audio/page-audio.ts(159 行: AudioWorkletNode の作成)
- lift/runtime/fle_audio_transport.c(20 行: fle_js_init、54 行: fle_js_publish、108 行: fle_js_wait の実時間の待ち)
- lift/runtime/fle_thread.h(1〜6 行: ゲストのスレッドは、1 つのホストのスレッドで協調して動く)
- app/src/engine/chunk-io-worker.ts(先頭の注釈: 別の Worker は I/O の専用)
- app/src/engine/protocol.ts(66 行付近: 描画の統計 desk)と app/src/main.ts(558 行付近: 統計を window.__app.deskStats に入れる)
- tools/boot-check/boot-bundle.mjs と tools/boot-check/cdp.mjs(起動と CDP の使い方)

## 先頭の注意(入力)
- P3 の所定のプロジェクトは runner に無い(依頼者の個人のファイル。tools/verify-kit/tools/scn-p3-mixmove-300.json が開くもの)。したがって、Song Mode の音の途切れ(AudioWorklet の underrun)は、この task では測れない。測れない項目は「未確認」と書く。別のプロジェクトで代用して P3 の数字とみなさない。
- 実測には engine の bundle が要る。ビルドの段が失敗したら、ここで止め、「実エンジン: 実行なし(ビルドの段: <段の名前>、終了コード <n>)」と報告する。

## やること
1. 版と抽出物を確かめる。`grep '^EMSDK_VERSION=' ci/versions.env` の結果と、`find lift/out-app/host -type f | wc -l` の件数を書く。
2. ビルドする(queue/kinds/lower.md の accept と同じ段): `npm ci --prefix app`、`npm run build --prefix app`、`cargo build --release --locked --manifest-path lift/Cargo.toml`、`lift/target/release/fe-build engine`、`lift/target/release/fe-build bundle`。各段の終了コードを書く。
3. 読みを記録する(実行しない)。次の 3 つを、行番号つきで書く。
   - `grep -n 'compose()\|putImageData' app/src/engine/desk.ts app/src/engine/desk-gl.ts` の結果。描画の合成と提示が、どの関数から呼ばれるか。
   - `grep -rn 'fle_js_publish\|fle_tp_publish' lift/runtime/*.c` の結果。音声の書き込みを呼ぶ側が、エンジンの run と同じスレッドか。
   - `grep -n 'new Worker\|new AudioWorkletNode' app/src -r` の結果。Worker と AudioWorklet の数。
4. 実測(headless Chrome、拡大率 1)。tools/boot-check/boot-bundle.mjs の main を、同じディレクトリの一時ファイル tools/boot-check/.tmp-survey-c2.mjs に写す(import の相対パスを保つため。.tmp-* なので git の無視の対象。終わったら消す)。配信と起動は boot-bundle.mjs と同じにし、state が engine-idle になるまで待つ。engine-idle の後、10 秒間、1 秒ごとに次の式を評価する。
   `(() => { const a = window.__app; return a ? { state: a.state, deskStats: a.deskStats } : null; })()`
   deskStats の composeMs・putMs・presents の、1 秒ごとの増え方を書く。
5. 実測(スレッドの一覧)。手順 4 と同じ実行で、CDP の Target.getTargets の結果を type ごとに数え、JSON の 1 行で出す。worker は URL の末尾(fl-engine や chunk-io-worker など、repo の中の名前だけ)を添える。
6. 実測(音声の数字)。window.__app の音声の項目(app/src/main.ts の audio や underrun の項目。名前を読んで使う)が取れれば、underrun の数を書く。取れなければ「未確認(取れない理由)」と書く。
7. 手順 4〜6 を 3 回走らせる。各回の JSON の行を findings に貼る。
8. 配信の方針(L-035)の読み(読みのみ)。起動後のバックグラウンドの取得と、wasm のコンパイル・インスタンス化が、音声を作る Worker(fl-engine)の外で行えるかを、次の順に読み、行番号つきで書く。(a) 起動時の取得を、どのスレッドが行うか: app/src/main.ts の 714〜717 行(loadWasm・loadPack・fetchAsset をページから呼ぶ)と、app/src/bundle/loader.ts の fetchAsset(185 行)・loadWasm(189 行)・loadPack(219 行)。(b) 遅延の install tree の先例: app/src/engine/chunk-io.ts(1〜10 行)と app/src/engine/chunk-io-worker.ts(1 行。I/O の Worker が取り、エンジンは Atomics.wait で待つ)。(c) 音声の書き込み(lift/runtime/fle_audio_transport.c の 54 行: fle_js_publish)が、どのスレッドで走るか。(a) と (b) の取得とコンパイルが、そのスレッドと同じかどうかを、表で書く(スレッド・何を行うか・音声と同じか)。(d) 結論: 取得・コンパイル・インスタンス化を、音声のスレッドの外に置けるか(推定と明記。実測は、手順 4〜7 の結果と合わせて書く)。
9. 後始末: `rm -f tools/boot-check/.tmp-survey-c2.mjs` を行い、残った一時ファイルが無いことを `git status --short` で確かめる。

## 出力
- queue/findings/survey-c2-worker-split.md に、次の見出しで日本語で書く。
  - `## 実測`: 手順 1・2・4〜7 の結果(出力の行つき)
  - `## コードの読み`: 手順 3 の結果と、読む物の要点(パスと行番号つき)。スレッドの配置を、表(スレッド・Worklet・何を走らせるか)で書く。
  - `## 未確認`: P3 の所定のプロジェクトが runner に無いこと、測れなかった項目と理由
  - `## 次の手順の案`: C-2 の実装の案。描画の合成と提示を、エンジンの Worker から分ける方法(エンジンの画面の画素を共有メモリ経由で渡すなど)と、その時の P3 の測り方。推定と明記する。
- commit する(push はしない)。
- 最終の応答の 1 行目は「実エンジン: …」で始める。

accept: test -s queue/findings/survey-c2-worker-split.md && grep -q '^## 実測' queue/findings/survey-c2-worker-split.md && grep -q '^## 未確認' queue/findings/survey-c2-worker-split.md && grep -q '^## 次の手順の案' queue/findings/survey-c2-worker-split.md
