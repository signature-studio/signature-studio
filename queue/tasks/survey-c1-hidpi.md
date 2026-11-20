# survey-c1-hidpi: C-1(HiDPI で画面がぼやける)の現状を、この runner で測る(測定のみ。製品は変えない)

## 報告の先頭の規則
- 報告の 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。実行していなければ「実エンジン: 実行なし(理由)」と書く。
- 測定の値には「runner の値。この機械とは別枠」と書き、runner の種類(nproc と loadavg の値)を添える。
- 代役・再生・事前に作った画像を使わない。数字は、この実行で得た実測だけを書く。各数字に、出典の出力の行(JSON の行)を付ける。推定は「推定」と書き、実測と分ける。
- 「達成」「直った」と書かない。判定は「受理」「差し戻し」「未確認」のいずれかにする。

## 規則の要約(この task で守ること)
- 製品(app/・lift/・ci/・tools/・patches/・.github/)を変えない。変更は queue/findings/survey-c1-hidpi.md だけ。
- 製品の作業(ビルド・実行)はこの runner の中だけで行う。読むだけの手順は、その旨を書く。
- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(この task では何も足さない)。
- Python は使わない。ベンダー名、Cloudflare のアカウント ID・ホスト名・ETag(32 桁の 16 進)、絶対パスを書かない。パスは repo の中の相対パスで書く。
- commit は環境の git の設定のまま行う(git config・--author・GIT_AUTHOR_*・git -c を使わない)。commit に Claude の行を付けない。ブランチを作らない。push はしない(判定の段が push する)。
- .git/config、opencode.jsonc、.opencode を読まない。git の内部命令(update-index・hash-object・update-ref・commit-tree・write-tree・bundle・for-each-ref を含む)を使わない。force は使わない。
- Secrets と Variables は読まない。wrangler と cf の CLI は使わない。R2 の鍵はこの実行には無い。
- sleep、timeout、nohup、disown、末尾の & は使わない。
- 一時ファイルは repo の中の .tmp-* に置き、終わったら消す。リポジトリの外(/tmp を含む)に書かない。

## 目的と根拠
台帳 L-033 の (1): 「HiDPI に対応しておらず画面がぼやけている」。queue/plan.md の C-1 を参照。コードの読みでは、デスクトップの canvas の描画バッファは guest 画面の画素数で、CSS の大きさと 1:1 である(app/src/ui/desktop.ts:24〜25)。拡大率 2 の画面では、ブラウザがこの canvas を 2 倍に引き伸ばすと推定される。この task は、その現状を runner の headless Chrome で数字にする。製品は変えない。

## 読む物(実在を確認済み)
- app/src/ui/desktop.ts(21〜27 行: deskCanvas の canvas.width と canvas.height。45〜52 行: CSS の大きさ)
- app/src/ui/screen-fit.ts(3〜4 行: 「1 guest pixel = 1 CSS pixel。ブラウザが canvas を devicePixelRatio で拡大する」の注釈。57 行: drawScale)
- app/src/main.ts(260〜276 行: measureRoom と applyDisplay。303〜304 行: ResizeObserver と dppx の変化)
- lift/runtime/fle_desk.h(先頭: 画面の大きさ。1920 x 1080 が既定)
- tools/boot-check/boot-bundle.mjs(bundle を 1 回起動し、JSON を 1 行出す。引数は bundle ディレクトリの絶対パス)と tools/boot-check/cdp.mjs(launchChrome の extraArgs)
- queue/kinds/lower.md の accept(ビルドの段の正本)

## 先頭の注意(入力)
- 入力は、このリポジトリのビルド物(engine と bundle)だけ。依頼者の個人のファイルは使わない。
- 実測には engine の bundle が要る。ビルドの段が失敗したら、ここで止め、「実エンジン: 実行なし(ビルドの段: <段の名前>、終了コード <n>)」と報告する。

## やること
1. 版と抽出物を確かめる。`grep '^EMSDK_VERSION=' ci/versions.env` の結果と、`find lift/out-app/host -type f | wc -l` の件数を書く。
2. ビルドする(queue/kinds/lower.md の accept と同じ段): `npm ci --prefix app`、`npm run build --prefix app`、`cargo build --release --locked --manifest-path lift/Cargo.toml`、`lift/target/release/fe-build engine`、`lift/target/release/fe-build bundle`。各段の終了コードを書く。
3. 読みを記録する(実行しない): `grep -n 'canvas.width\|canvas.height\|devicePixelRatio' app/src/ui/desktop.ts app/src/main.ts app/src/ui/screen-fit.ts app/src/engine/desk.ts app/src/engine/desk-gl.ts` の結果を findings に貼る。canvas の大きさに devicePixelRatio を掛けている行が無いかを、行の番号つきで書く。
4. 実測(headless Chrome、拡大率 2): tools/boot-check/boot-bundle.mjs の main を、同じ場所の一時ファイル tools/boot-check/.tmp-survey-c1.mjs に写す(import の相対パスを保つため、同じディレクトリに置く。.tmp-* なので git の無視の対象。終わったら消す)。写す時に、launchChrome の extraArgs へ `--force-device-scale-factor=2` を足す。配信・起動・待ちは boot-bundle.mjs と同じにし、state が engine-idle になるまで待つ。engine-idle の後に、次の式を評価し、その値を JSON の 1 行で出す。
   `(() => { const c = document.querySelector('canvas[data-desk="1"]'); if (!c) return null; const r = c.getBoundingClientRect(); return { dpr: window.devicePixelRatio, canvasW: c.width, canvasH: c.height, cssW: r.width, cssH: r.height, ratio: c.width / (r.width * window.devicePixelRatio) }; })()`
   3 回走らせ、各回の JSON の行を findings に貼る。`--force-device-scale-factor` で dpr が 2 にならないなら、CDP の Emulation.setDeviceMetricsOverride(deviceScaleFactor: 2) に替え、その旨を書く。
5. 判定を書く。ratio が 1 に近ければ「描画バッファは CSS の 1 px に 1 画素」(拡大されてぼやける)と、実測の値で書く。2 に近ければ、拡大率を掛けているので、手順 3 の読みと合わない旨を書く。3 回の ratio を並べる。
6. 後始末: `rm -f tools/boot-check/.tmp-survey-c1.mjs` を行い、残った一時ファイルが無いことを `git status --short` で確かめる。

## 出力
- queue/findings/survey-c1-hidpi.md に、次の見出しで日本語で書く。
  - `## 実測`: 手順 1・2・4・5 の結果(出力の行つき)
  - `## コードの読み`: 手順 3 の結果と、読む物の要点(パスと行番号つき)
  - `## 未確認`: 実測できなかった項目と、その理由
  - `## 次の手順の案`: C-1 の実装の案。canvas の大きさを CSS の大きさに拡大率を掛けた画素数にする場合の影響(画素数が拡大率の 2 乗で増えること、描画の所要が増えること。どちらも推定と明記)と、C-2 の後に着手する理由。
- commit する(push はしない)。
- 最終の応答の 1 行目は「実エンジン: …」で始める。

accept: test -s queue/findings/survey-c1-hidpi.md && grep -q '^## 実測' queue/findings/survey-c1-hidpi.md && grep -q '^## 未確認' queue/findings/survey-c1-hidpi.md && grep -q '^## 次の手順の案' queue/findings/survey-c1-hidpi.md
