# 0005: P3(Song Mode の再生で音が途切れない)を、この runner で測る(測定のみ。変更しない)

## 報告の先頭の規則
- 報告の 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。実行していなければ「実エンジン: 実行なし(理由)」と書く。
- 測定の値は「runner の値。この機械とは別枠」と書く。runner の種類(meta.txt の runner・nproc・loadavg)を添える。
- 代役・再生・事前に作った録音や画像を使わない。数字は、この実行で得た実測だけを書く。各数字に、出典の出力の行を付ける。
- 「達成」「直った」「音が途切れない」と書かない。判定は「受理」「差し戻し」「未確認」のいずれかにする。耳の確認はこの作業では行わない(0 件と書く)。

## 先頭の注意(入力と道具)
- P3 の入力は、docs/ops/fidelity-and-performance.md の P3 が指す所定のプロジェクトである(依頼者の個人のファイル。runner に無い。未確認: runner で取得する手段は無い)。台本 tools/verify-kit/tools/scn-p3-mixmove-300.json が、そのプロジェクトを開くかどうかを、台本の中身で確かめる(開くなら、実行しないで「実エンジン: 実行なし(入力の所定プロジェクトが runner に無い)」と報告して止まる)。
- 所定のプロジェクトを開かない台本だけを使う場合は、その台本が P3 の条件を満たさないことを報告に書く。
- 道具の所在(この下書きを書いた時点で確認済み): tools/verify-kit/serve-once-p3-bundle.mjs(bundle の配信)、tools/verify-kit/tools/r635-run.sh(1 回の列。audio-p3.json を出力に集める)、tools/verify-kit/tools/r635-state.sh、tools/verify-kit/tools/p3-recompute-dirs.py(underrun の数を数える検証用。Python は調査・検証に限って使う。製品の経路には入れない)。

## 目的と根拠
docs/ops/fidelity-and-performance.md の P3(所定のプロジェクトを Song Mode で再生して、音が途切れない。AudioWorklet のリングの枯渇 0。再生時間と消費フレーム数の比)を、runner の headless Chrome で測る。前の測定(R3-1d、R4-3c、R6-35)は、この機械の値であり、runner の値ではない。

## 読む物(実在を確認済み)
- docs/ops/fidelity-and-performance.md の §3(P3 の条件と測り方)
- tools/verify-kit/tools/scn-p3-mixmove-300.json(5 分の台本。条件 2: マウス移動と Mixer 表示)
- tools/verify-kit/serve-once-p3-bundle.mjs、tools/verify-kit/tools/r635-run.sh(使い方は各ファイルの先頭のコメント)
- lift/crates/fe-build/src/main.rs(子コマンド engine と bundle。bundle の出力先は lift/out-app/bundle/raw/fl-Oz)

## やること
1. 入力と台本を確かめる(先頭の注意)。台本が所定のプロジェクトを開くかを、台本の該当行で書く。
2. 版と抽出物を確かめる。`grep '^EMSDK_VERSION=' ci/versions.env` の結果、`find lift/out-app/host -type f | wc -l` の件数、`emcc --version` の 1 行目を書く。
3. ビルドする(0003 と同じ手順。npm ci --prefix app、cargo build --release --locked --manifest-path lift/Cargo.toml、lift/target/release/fe-build engine、lift/target/release/fe-build bundle)。終了コードを書く。
4. 1 回の列を走らせる。tools/verify-kit/tools/r635-run.sh の使い方に従い、同じ台本で n=3 を走らせる。各回の audio-p3.json と report.json を、出力の場所の名前で残す。
5. underrun の数を数える。tools/verify-kit/tools/p3-recompute-dirs.py を、3 回の出力に対して使う(検証用)。各回の underrun の合計と、書き込み率の最小を、出力の行つきで書く。
6. 判定する。P3 は underrun 0 が条件。0 でない回がある場合は、その回の数と時刻の範囲を書く。耳の確認はしないので、「音が途切れない」とは書かない。

## 出力
- ci/agents/out/0005-p3-song-mode-underrun/findings.md に、手順 1 から 6 の結果を日本語で書く。
- ci/agents/out/0005-p3-song-mode-underrun/p3-runs.txt に、各回の集計の出力行をそのまま保存する(n=3)。
- 最終の応答の 1 行目は「実エンジン: …」で始める。

## 禁止と注意
- 変更は作らない。ci/agents/out/0005-p3-song-mode-underrun/ の外のファイルを変えない。commit も push もしない。
- .git/config、opencode.jsonc、.opencode を読まない。git の内部命令(update-index・hash-object・update-ref・commit-tree・write-tree・bundle・for-each-ref・refs を含む命令)を使わない。force は使わない。
- Secrets と Variables は読まない。wrangler と cf は使わない。R2 の鍵はこの実行には無い。
- ホストの絶対パス、アカウント ID、鍵、URL のホスト名を書かない。パスは変数名か repo 内の相対パスで書く。
- sleep、timeout、nohup、末尾の & は使わない。
