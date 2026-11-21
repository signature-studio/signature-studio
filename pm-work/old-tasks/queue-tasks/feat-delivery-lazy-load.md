# feat-delivery-lazy-load: 起動前に取る物と、起動後にバックグラウンドで取る物を分ける(L-035。P1 とサイズ)。最初の 1 件は Fruity Balance

mode: agent
規模: 大

## 目的
依頼者の提案(台帳 L-035): 起動に必要な wasm が揃った段階で FL Studio は起動し、プラグインが使われていなくても、プラグインとアセットをバックグラウンドで取る(lazy-load)。1 プラグイン 1 wasm で試行する。実現が難しいと判断したら、方針を変えてよい(変えた理由を報告に書く)。queue/plan.md の「配信の方針(L-035)」と C-4 を、実装で始める。

## 方針(台帳 L-035 の文言。依頼者の指示)
- 起動前に取る物: FLEngine(エンジンの wasm と DLL の PE)、言語パック、テーマ、プロジェクトテンプレートなど。P1 の 10 秒に入る。
- 起動後にバックグラウンドで取る物: プラグイン(内蔵・外部)とアセット。FL の起動処理が始まった時点から、順次取る。プラグインやアセットへのアクセスを起点にしない。
- 1 プラグイン 1 wasm で試行する。Worker 数の上限は、実際に当たったら方針を変える(事前に検討しない)。
- 最初の 1 件は Fruity Balance。3 件の Fruity のうち、survey-c4 の実測で最も小さい(3,718,656 B。Fruity Filter は 3,780,608 B、Delay 2 は 3,731,968 B)。

## 背景(survey の実測と読み。出典は queue/findings/)
- 今の起動前の取得は 4 つで、すべて起動の前にまとめて取る(app/src/main.ts:714〜717: loadWasm・loadPack(seed pack)・loadPack(DLL pack)・fetchAsset(glue))。実測(survey-c3・c4 の build。runner の値): engine wasm 167,414,828 B(br 17,717,323 B)、seed pack 20,780,949 B、DLL pack 88,675,800 B(11 ファイル)。
- DLL pack の 11 ファイルは FLEngine と 10 モジュール。engine へは起動引数で並ぶ(lift/harness/web/pack.args の --args の行。13 行の --args、14 行の値)。Fruity の 3 つは、DLL pack と seed pack の両方に入っている(survey-c3 の 6 節。3 DLL の SHA-256 は ci/extract-dlls.tsv の 15・16・18 行と一致)。
- 取得の経路: app/src/bundle/loader.ts(fetchAsset 185 行、loadWasm 189 行、loadPack 219 行。verifyingStream 138 行で SHA-256 と大きさを検査)。DLL pack の mount: app/src/engine/fruity.ts:125(hostFiles.mountPack)、instantiate: fruity.ts:133。
- モジュールの PE と wasm の関係: lift/runtime/fle_realmod.h:1〜3 は、ビルドが同じプログラムにリンクし、リベース・lift した第 2 の PE を real module と呼ぶ。lowered の一覧は lift/harness/hyb/lists/ の 8 件(b2d・dsp・elastique・fle・ft・ilt・oggio・qfc)で、Fruity の一覧は無い(survey-c4)。Fruity が、どちらの扱いかは未確認。
- 先例: app/src/engine/chunk-io.ts(1〜10 行。遅延の install tree の chunk を別の I/O Worker が取り、エンジンは Atomics.wait で待つ。製品の経路には繋がっていない)。app/src/plugins/ott-host.ts(OTT の 1 インスタンス 1 Worker。テストからだけ使われる)。app/src/plugins/plugin-client.ts。
- 配信: app/worker/index.ts の 2 行目(/assets/* は R2 から、単一の Range・ETag・保存された Content-Encoding で返す)。app/tools/assets-middleware.ts(ローカルの配信)。bundle の作り: lift/crates/fe-build/src/main.rs の pack_step_for(459〜463 行)。

## 論点(設計は作業者が決める。依頼者の指示ではなく案。監査が見直す)
1. 起動後の取得の順序(何を先に取るか)。
2. 取得が終わる前に、そのプラグインが使われた時の振る舞い(待つ、その 1 つを先に取る、など)。
3. 取得した wasm と DLL の検証(SHA-256 と大きさ。loader.ts の検査と同じ作り)とキャッシュ。
4. 取得とコンパイル・インスタンス化を、音声を作る fl-engine の Worker の外で行うか(survey-c2 の結論: 今の取得もコンパイルもページのメインスレッドで行われている。インスタンス化は fl-engine)。
5. Fruity Balance を起動引数から外すと、engine が起動時に何を読むか(引数の読み、VFS の見つからない時の扱い)。これは lift/runtime の読みで確かめる。

## 制約(守る。台帳の依頼者の指示)
- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。オプトインを作らない(既定で有効。L-002)。
- ベンダー名・Cloudflare のアカウント ID・ホスト名・絶対パスを書かない。wrangler.jsonc を書き換える仕組みを作らない。マスクの命令を置かない。Python を使わない。
- commit は環境の git の名義のまま(git config・git -c・--author・GIT_* を使わない)。commit に Claude の行を付けない。ブランチを作らない。
- app/・lift/ を変える push は main の deploy.yml を動かす。報告に、変えたパスを書く。
- 他のタスク(feat-c2-render-worker・feat-c1-hidpi・feat-c3-resources-seed)が、app/src/main.ts・app/src/engine/・lift/runtime などを変えうる。作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、下の受け入れをもう一度通してから commit する。
- Worker 数の上限(1 ページの Worker 数)は、事前に考えない。当たったら報告に書き、方針を変える。
- 製品の既定の起動の動作(起動前に取る物の一覧)を、測定のためだけに変えない。

## 読む物(実在を確認済み)
- app/src/main.ts(260〜276 行: 表示。714〜724 行: 起動時の取得と client.init)
- app/src/bundle/loader.ts(138 行 verifyingStream、185 行 fetchAsset、189 行 loadWasm、219 行 loadPack)、app/src/bundle/manifest.ts(73〜75 行: seedPack と dllPack)、app/src/bundle/assetstore.ts
- app/src/engine/fruity.ts(120〜140 行: DLL pack の mount と instantiate)、app/src/engine/hostfiles.ts(mountPack)
- app/scripts/pack-assets.mjs(先頭 1〜10 行: bundle の作り。--dll-dir の扱い)、app/scripts/pack-seed.mjs(1〜8 行)
- lift/harness/web/pack.args(13 行の --args、14 行の値)、lift/crates/fe-build/src/main.rs(459〜474 行: pack_step_for と bundle)
- lift/runtime/fle_realmod.h(1〜3 行)、lift/runtime/fle_realmod.c(173 行)、lift/runtime/fle_modules.c(先頭の注釈: 仮想モジュールの表)、lift/runtime/fle_load.c(LoadLibrary の解決)
- queue/findings/survey-c4-builtin-plugins.md、survey-c5-external-plugins.md(コードの読み)
- tools/boot-check/boot-bundle.mjs、tools/boot-check/cdp.mjs

## やること
1. 変更の前に、今の起動前の取得を測る(ネットワークの記録。resource timing の名前・大きさ・開始時刻)。出力を報告に貼る。
2. 区分の実装: 起動前に取る物と、起動後にバックグラウンドで取る物を、manifest か bundle の作りで分ける。Fruity Balance を後者にする。1 プラグイン 1 wasm の試行は、この区分の中で行う(設計は作業者)。
3. 起動後の取得: FL の起動処理が始まった時点から順次(上の方針)。
4. 測定の道具を作る: tools/feat-delivery-lazy/measure.mjs(JSON の行を 3 行、stdout に出す。1 行が 1 回の起動)。boot-bundle.mjs と同じ起動で、engine-idle になるまで待つ。各行: {"state","standIn","error","elapsedMs","preStart":[{"name","bytes"}],"lazy":[{"name","bytes"}],"preStartDoneMs","fetches":[{"name","startMs","endMs"}]}。preStart と lazy は、起動の取得の一覧(ファイル名と大きさ)。fetches は performance.getEntriesByType('resource') の、ファイルごとの開始と終了(ms。navigation の開始からの値)。preStartDoneMs は、起動前の取得の最後の endMs。elapsedMs は、navigate から engine-idle の確認まで(測定の値。判定には入らない)。

## 受け入れ
下の accept の行が 0 で終わること。内容は次のとおり。
- 通常のビルド(app・cargo・fe-build engine・fe-build bundle)と、boot.json の起動確認(state が engine-idle、standIn が false、error が null)。
- 測定の道具の 3 行を node -e が読む。3 行とも state が engine-idle、standIn が false、error が null。preStart に Fruity Balance が無く、lazy に Fruity Balance が在る(名前は /fruity.?balance/i で判定)。lazy の合計が 0 より大きい。Fruity Balance の取得の開始(startMs)が、起動前の取得の完了(preStartDoneMs)以上である。
- 注意: 台帳 L-035 は「engine-idle の後」とあるが、方針は「FL の起動処理が始まった時点から」なので、受け入れは「起動前の取得の完了の後」で判定する。engine-idle の時刻との関係は報告に書く。
- elapsedMs の 3 回の値(と最小・中央値)を報告に書く。P1 の 10 秒(10000 ms)との比較は、判定に入れず報告に書く。survey-c1 の実測は 10205〜10584 ms(poll を含む)。
- 起動前の合計の大きさ(preStart の bytes の合計)と、起動後の合計の大きさ(lazy の合計)を、報告に書く。変更の前の値と比べる。

accept: npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-delivery-lazy/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/feat-lazy.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const sum=(a)=>a.reduce((s,f)=>s+f.bytes,0);const fb=(a)=>a.some((f)=>/fruity.?balance/i.test(f.name));const ok=L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&!fb(r.preStart)&&fb(r.lazy)&&sum(r.lazy)>0&&r.fetches.some((f)=>/fruity.?balance/i.test(f.name)&&f.startMs>=r.preStartDoneMs));process.exit(ok?0:1)' "$RUNNER_TEMP/feat-lazy.jsonl"

## 報告の規則
- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、runner の種類(nproc と loadavg)を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 設計の選択(区分の作り、1 プラグイン 1 wasm の試行の形、Fruity Balance の扱い)、方針を変えた場合はその理由、変えたパス(app/・lift/ を含む)、起動前と起動後の合計の大きさ、elapsedMs の 3 回の値。
- 取得の開始が engine-idle より前にある場合は、その事実を書く(受け入れの注意を参照)。

## 禁止と注意
- 測定の値を、accept の判定のために書き換えない(道具の出力の生の値を、accept の node -e が読む)。
- 製品の既定の動作を、測定のためだけに変えない。
- sleep・timeout・nohup・disown・末尾の & は使わない(待ちは JS のタイマー)。
- 一時ファイルは .tmp-* に置き、終わったら消す。リポジトリの外(/tmp を含む)に書かない。
- ログ・Artifacts・run を削除しない。
