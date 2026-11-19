# 0003: 実エンジンの起動(P1)を、この runner で n=3 で測る(測定のみ。変更しない)

## 報告の先頭の規則
- 報告の 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。実行していなければ「実エンジン: 実行なし」と書く。
- 代役・再生・推測を数字に使わない。数字は、この実行で得た実測だけを書く。各数字に、出典のコマンドと、出力の行(JSON の行)を付ける。
- 測定の値には「runner の値。この機械とは別枠」と書く。runner の種類(meta.txt の runner・nproc・loadavg)を添える。

## 目的と根拠
docs/ops/fidelity-and-performance.md の P1(ページを開いてから、エンジンが最初の idle に達するまで。10 秒以内。初回とキャッシュ後を分ける)を、この runner で測る。依頼者の指摘は、実エンジンが動くことの確かめに関わる。前の記録(R4-5c の起動の測定)は、この機械の headless Chrome の値であり、この runner の値ではない。

## 読む物(実在を確認済み)
- docs/ops/fidelity-and-performance.md の §3(P1 の条件と測り方)
- lift/crates/fe-build/src/main.rs の 557 行付近(子コマンド engine・extract・bundle・upload・endpoint)、484 行と 498 行(bundle の出力先 lift/out-app/bundle/raw/fl-Oz)
- tools/boot-check/boot-bundle.mjs(引数は、bundle ディレクトリの絶対パス。出力は JSON の 1 行。timelineIdle が最初の idle の ms)
- ci/versions.env(EMSDK_VERSION・NODE_VERSION・RUST_TOOLCHAIN。値は grep で読む。read ツールでは読めない)
- .github/workflows/agent.yml の run ジョブ(Node・rust・fe-build・emsdk・抽出物の展開の手順。この run の前提)

## やること
1. 前提を確かめる。`test -f lift/out-app/host/binaries/win64/FLEngine_x64.dll` の結果と、`find lift/out-app/host -type f | wc -l` の件数を書く。無ければ、ここで止め、「実エンジン: 実行なし(抽出物が無い)」と報告する。
2. 版を確かめる。`grep '^EMSDK_VERSION=' ci/versions.env` と `emcc --version` の 1 行目を書く。
3. ビルドする(この runner の既定の手順。fe-build の endpoint は使わない。配信ではない)。`npm ci --prefix app`、`cargo build --release --locked --manifest-path lift/Cargo.toml`、`lift/target/release/fe-build engine`、`lift/target/release/fe-build bundle`。各コマンドの終了コードと、最後の 3 行の出力を書く。
4. bundle の場所を確かめる。`ls lift/out-app/bundle/raw/fl-Oz/` の結果を書く(manifest.json が在るか)。
5. 測る(n=3)。各回で `node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz"` を 1 回ずつ実行する(新しいプロファイル。各回の JSON の 1 行を、そのまま報告に貼る)。1 回の時間の上限は道具の既定に従う。
6. 判定する。各回について、timelineIdle(ms)・state・engineKind・error・standIn を読む。standIn が null でない、または error が null でない回は「代役か、失敗」と書き、P1 の数字には入れない。timelineIdle が 10000 以下の回の数を書く。
7. キャッシュ後の値を分ける。同じプロファイルで 2 回目を測る手順が道具に無ければ、「未確認(キャッシュ後の測り方が道具に無い)」と書く。
8. 画面を 1 枚保存する。道具が画面を保存しなければ、「未確認(画面の保存の道具が無い)」と書く。保存する場合は、ci/agents/out/0003-engine-p1-measure/ の中に置く。

## 出力
- ci/agents/out/0003-engine-p1-measure/findings.md に、手順 1 から 8 の結果を日本語で書く。各数字に出典の行を付ける。
- ci/agents/out/0003-engine-p1-measure/boot-runs.jsonl に、boot-bundle.mjs の出力の JSON の行を、そのまま保存する(n=3)。
- 最終の応答の 1 行目は「実エンジン: …」で始める(規則の欄)。

## 禁止と注意
- 変更は作らない。コードも設定も、ci/agents/out/0003-engine-p1-measure/ の外も変えない。commit も push もしない。
- 代役・モック・再生・事前に作った画像や録画を、この測定に使わない。
- wrangler と cf は実行しない。Secrets と Variables は読まない。R2 の鍵はこの実行には無い。
- ホストの絶対パス、アカウント ID、鍵、URL のホスト名を書かない。パスは $RUNNER_TEMP のような変数名、または repo 内の相対パスで書く。
- この実行の checkout は全履歴。git の結果は、コマンドの出力で示す。
- sleep、timeout、nohup、末尾の & は使わない。
