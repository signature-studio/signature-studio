# 0004: P2(UI の fps と入力の遅延)を、この runner で測る(測定のみ。変更しない)

## 報告の先頭の規則
- 報告の 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。実行していなければ「実エンジン: 実行なし(理由)」と書く。
- 測定の値は「runner の値。この機械とは別枠」と書く。runner の種類(meta.txt の runner・nproc・loadavg)を添える。
- 代役・再生・事前に作った画像や録画を使わない。数字は、この実行で得た実測だけを書く。各数字に、出典の出力の行(JSON の行)を付ける。
- 「達成」「直った」と書かない。判定は「受理」「差し戻し」「未確認」のいずれかにする。

## 先頭の注意(入力が runner に無い)
- P2 の入力は、docs/ops/fidelity-and-performance.md の P2 が指す所定のプロジェクト(依頼者の個人のファイル)である。このファイルはリポジトリにも runner にも無い(未確認: runner の取得手段は無い)。
- したがって、この下書きのままでは P2 の条件を満たさない。runner で実行する前に、main が入力の扱いを決めること。決められるまで、この task は「実エンジン: 実行なし(入力の所定プロジェクトが runner に無い)」と報告して止まる。
- 120 fps の確かめについて: docs/decisions と evidence の記録では、この機械の rAF は 60 Hz である(R4-2c)。120 fps は確かめられないことがある。確かめられないなら、その旨を書く。

## 目的と根拠
P2(所定のプロジェクトを開き、UI を 120 fps で動かし、UI 操作への遅延がほとんど無い。目標は入力から画面の p95 が 17 ms 以下)を、runner の headless Chrome で測る。前の測定(R4-2 と R4-2c)は、この機械の値であり、runner の値ではない。

## 読む物(実在を確認済み)
- docs/ops/fidelity-and-performance.md の §3(P2 の条件と測り方)
- tools/verify-kit/serve-once-lat-bundle.mjs(bundle を 1 回配信する道具)
- tools/verify-kit/tools/sdriveg2-lat.mjs(入力の遅延 T と、presentsPerS を出す駆動の道具。先頭のコメントに使い方)
- tools/verify-kit/tools/scn-lat-ff.json(P2 の台本)
- lift/crates/fe-build/src/main.rs(子コマンド engine と bundle。bundle の出力先は lift/out-app/bundle/raw/fl-Oz)

## やること
1. 入力を確かめる。所定のプロジェクトが runner にあるかを、`test -f` の結果で書く(無ければ、上の先頭の注意に従い止まる)。
2. 版と抽出物を確かめる。`grep '^EMSDK_VERSION=' ci/versions.env` の結果、`find lift/out-app/host -type f | wc -l` の件数を書く。
3. ビルドする(0003 と同じ手順。`npm ci --prefix app`、`cargo build --release --locked --manifest-path lift/Cargo.toml`、`lift/target/release/fe-build engine`、`lift/target/release/fe-build bundle`)。終了コードを書く。
4. 配信して台本を走らせる。tools/verify-kit/serve-once-lat-bundle.mjs と tools/verify-kit/tools/sdriveg2-lat.mjs の先頭のコメントの使い方に従い、scn-lat-ff.json を n=3 で走らせる。各回の出力の JSON の行をそのまま保存する。
5. 数字を読む。各回の presentsPerS(フレーム間隔の代わりの値)と、再生中の File クリックと Esc の T(p50・p90・p99)を、出力の行つきで書く。
6. 判定する。p95 が 17 ms 以下か、120 fps が出るかを、出力の行で書く。出ないときは「差し戻し」または「未確認」と書き、理由(rAF の 60 Hz など)を書く。

## 出力
- ci/agents/out/0004-p2-ui-fps-measure/findings.md に、手順 1 から 6 の結果を日本語で書く。
- ci/agents/out/0004-p2-ui-fps-measure/lat-runs.jsonl に、各回の JSON の行をそのまま保存する(n=3)。
- 最終の応答の 1 行目は「実エンジン: …」で始める。

## 禁止と注意
- 変更は作らない。ci/agents/out/0004-p2-ui-fps-measure/ の外のファイルを変えない。commit も push もしない。
- .git/config、opencode.jsonc、.opencode を読まない。git の内部命令(update-index・hash-object・update-ref・commit-tree・write-tree・bundle・for-each-ref・refs を含む命令)を使わない。force は使わない。
- Secrets と Variables は読まない。wrangler と cf は使わない。R2 の鍵はこの実行には無い。
- ホストの絶対パス、アカウント ID、鍵、URL のホスト名を書かない。パスは変数名か repo 内の相対パスで書く。
- sleep、timeout、nohup、末尾の & は使わない。
