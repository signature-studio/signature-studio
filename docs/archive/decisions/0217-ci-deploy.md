# 0217: Cloudflare Workers + R2 へのデプロイと、段階キャッシュ付きの GitHub Actions ワークフロー(ブランチ `ci/cloudflare-deploy`)。R2 の 3 バケット・インストーラー 4 本のアップロード・Access アプリ・Worker のデプロイをこの機械から実行し、段階スクリプトを act で動かした。標準の GitHub ランナーに固定、Variables 0 個・Secrets 5 個。GitHub の標準ランナーで push のドライランを 8 回実行(6 回成功。本番デプロイは未実行)。仮の決定

状態: **仮の決定(すべての選択)**。依頼は依頼者(Cloudflare Workers + R2 へのデプロイと、それを継続的に行う GitHub Actions)、PM 経由の決定 0217。実装・測定・文書は作業者 `ci1`(2026-11-04 JST 04:05〜)。基準は `origin/main` 7c0e9cc(のち d17b038、さらに 62ac3e9 に rebase。ship2 は取り込み済み)。この決定は**デプロイ・CI の基盤であって、FL Studio の実エンジンの進捗ではない**(G 段階表の到達に数えない)。実エンジンの実行は、(i) lift の生成 C と wasm のビルドの再現(クリーンな木から段階スクリプトで作った物)に限る。ブラウザでの動作は測っていない。

区分の凡例は決定 0152 と同じ。**実測**＝このセッションのコマンドの出力を見た事実。**読み**＝コード・文書を読んだ解釈。**出典なしの選択**＝根拠が無く作業者が選んだ値(`S-ci1-…`、§9)。**未確認**＝確かめていない。環境は『この機械』= 16 コアの Linux VM(他担当の負荷が常にあり load 30〜55)。GitHub のランナーではない。

## 冒頭の三点

1. **何を実測したか**: インフラ作業(R2 3 バケットの作成、インストーラー 4 本のマルチパートのアップロードと読み戻し、Access アプリの作成、Worker のデプロイ・再デプロイ・改名)を、この機械の wrangler(OAuth)と cf CLI で実行した。`runs-on/cache` v5.0.7 を R2 の S3 API に対して Actions の外と act の中で実行した。最小権限を一時トークンで実測した(Worker の deploy は Workers Scripts: Edit だけ/バケット限定の R2 トークンは REST API の `wrangler r2 object put` を 403 で拒否〔R2 の文書にも明記〕/複数の指定バケットに限定した 1 つのトークンで 2 バケットに書け、他は AccessDenied)。`git archive` のクリーンな木から全段階をスクリプトで実行し(F2xH と F2xH2〔混成なし〕を計 5 回)、生成 C の決定性、wasm の大きさ・時間・メモリ、同じ木の 2 回のビルドの最終 wasm の一致、ship2 の木でのサイズの一致を測った。標準の GitHub-hosted ランナー(private: 2 vCPU / 8 GB / 14 GB)で足りるかを、実測のピーク(emcc 1.43 GB、wasm-opt 3〜5 GB、ディスク約 7 GB)と GitHub の文書で検討した。ワークフローを nektos/act 0.2.96 のホスト実行モードで plan・vst3・app・build ジョブとして実 R2 に対して動かした。依頼者が Secrets 5 個を設定した後、PM の指示でトピックブランチだけの一時的な push トリガー(必ずドライラン)を足し、**GitHub の標準ランナーで 8 回実行した**(冷たい build 2 回、温かい 2 回、wasm の入力を変えた 1 回、rebase 後のツリー 1 回、失敗 2 回)。この機械で同じ木を作った最終 wasm と GitHub のランナーの物はバイト単位で一致した。
2. **結論**: 段階キャッシュ付きのワークフローを `ci/cloudflare-deploy` に作った(`main` への push で本番デプロイ〔docs だけの変更と他のブランチの push は動かさない〕。手動実行は `main` での `dry_run=false` 以外ドライラン)。**初版は第三者のランナー(RunsOn)と大きなランナー、Variables、3 組の S3 トークンを前提にしていたが、依頼に無い私の判断で誤りだった。標準の `ubuntu-24.04` に固定し、Variables は 0 個、Secrets は 5 個にした。**キーは入力だけから作り、bundle が hit なら build ジョブを起動しない。R2 での `runs-on/cache` は動く(空のバケットで黙って GitHub のキャッシュになる落とし穴はラッパーで防ぐ)。クリーンな木からの全段階の実行で、生成 C は入力を替えても 2 回目でも全一致、同じ木を別のディレクトリで 2 回作った F2xH2 の最終 wasm(`flengine6.opt.wasm`)と glue は SHA-256 まで完全に一致。act での往復は 初回(冷たい)の全段階の実行・保存・検証と、2 回目の全 hit(ビルド不要の判定)、一部だけ消した再実行(bundle・link・dsp のオブジェクト)。Worker は `fl-studio-web` に改名し(旧 `fl-studio-web-private` は削除)、認証なしの 8〜12 要求は前・中・後のすべてで Access の 302(依頼者の Access 経由の表示は未確認)。
3. **未確認**: **GitHub 上の本番デプロイ(`dry_run=false`)**: 配信バケットへの書き込み(`R2_CACHE_ASSETS_*` で `fl-web-assets` に書けるか)、`<CREDENTIAL-NAME>`(Workers Scripts: Edit)での `wrangler deploy`、デプロイ後の Access 到達確認は、GitHub 上で一度も走っていない。ドライランで通ったのは、成果物バケットの読み取り(`R2_ARTIFACT_*`)、キャッシュバケットの読み書き(`R2_CACHE_ASSETS_*`)、`wrangler deploy --dry-run` まで。依頼者の Access 経由の表示。ship2 の混成(`hyb.sh`)の再現と、混成なしの F2xH2 の SHA-256 の ship2 との一致(ship2 に SHA-256 の記録が無く、サイズ 130,225,915 B の一致 の一致まで)。FLEngine の既定構成の生成 C `cd9107ce…` と全体テスト 178/2,274/0 は再実行していない。

## 0. 先頭の答え

| 問い | 答え | 根拠 |
|---|---|---|
| インストーラーを R2 に置けたか | **はい**(4 本、計 2,469,325,400 B)。読み戻した SHA-256 が手元の 4 本すべてと一致、ETag も手元計算(md5 / 64 MiB パートの md5 の md5)と一致 | §1、`ci/artifacts.lock`、`tables/r2-roundtrip.sha256` |
| `runs-on/cache` は R2 で動くか | **動く**(save 単発・マルチパート、restore、lookup-only、composite action 経由の save→verify を実測)。ただし**落とし穴**(空のバケットで黙って GitHub のキャッシュ。RunsOn という第三者のランナーでだけ起きる 2 つ〔資格情報を捨てる、`RUNS_ON_AWS_REGION` が先に効く〕は標準ランナーでは起きない)と、**原因不明の restore 停止 1 回** | §2、`tables/runs-on-cache-r2.md` |
| 段階キャッシュ(変更が無ければ飛ばす)は動くか | キー計算・lookup・「何を復元し何を走らせるか」の判定は実 R2 に対して動いた(act)。**GitHub の標準ランナーで確認済み**(冷たい→温かい→一部変更、§4c)。act ではフルビルドの往復と、消した物だけの再ビルドを確認 | §3、§5 |
| クリーンな木から段階を順に実行して生成 C・wasm が再現するか | 生成 C の SHA-256 7 本は、入力を替えても 2 回目でも全一致。wasm は 別ディレクトリ・別の時刻の 2 回のビルドで最終 wasm が SHA-256 まで一致(§5b)。混成なしの F2xH2 は ship2 の木でサイズが ship2 の記録(130,225,915 B)と一致 | §5 |
| 配信(Workers + R2 + Access)は動いたか | この機械から 1 回(+ PM 指示の再デプロイ 1 回)デプロイした。**認証なしの要求は 12 件すべて Access の 302**。依頼者の Access 経由の表示は**未確認** | §6 |
| GitHub Actions 上で動いたか | **動いた**(標準ランナー、push トリガーのドライラン 8 回〔うち成功 6〕)。冷たいビルド 1 回の build ジョブは **37〜46 分で成功**(見積りの 70〜100 分より速い)、温かい実行は **約 1 分 20 秒**。2 回の失敗は私のコードの誤り(swap ファイルが空きを食った/未使用ループ変数)で修正済み。GitHub 上の**本番デプロイ(`dry_run=false`)は未実行** | §4c、`tables/github-runs.md` |

## 1. R2 の 3 バケットとインストーラー(実測)

`wrangler r2 bucket create`(この機械の OAuth)で作成。作成前の `bucket list` に `fl-web-*` は無く、依頼者の他の 5 バケット(`cfcpa-config`、`cloudflareos-workshop-blueprint-content`、`nksf-parser-fixtures`、`scriba-session-archive`、`scriba-tfstate`)には触れていない。3 つとも `r2.dev` 無効・カスタムドメイン無し(`wrangler r2 bucket dev-url get` / `domain list` で確認)。

| バケット | 用途 | 入れた物 |
|---|---|---|
| `fl-web-artifacts` | アーティファクト用(インストーラーのみ) | `installers/` に 4 本。二次成果物は入れない |
| `fl-web-cache` | キャッシュ用(`runs-on/cache`) | 段階の出力。`cache/` の 60 日で失効のライフサイクル規則を設定(S-ci1-1) |
| `fl-web-assets` | 配信用(Worker の `ASSETS_BUCKET`) | SHIP2 の bundle(`fl-engine-ship2-F2xH2/`)と既定の `manifest.json` |

インストーラーのアップロード: 小さい 2 本(OTT 8.4 MB、Vital 41.8 MB)は `wrangler r2 object put`。FL Studio(1,106,033,456 B)と Serum 2(1,313,084,056 B)は wrangler が『300 MiB まで』で拒否した(エラー文を確認)ので、**ローカルの `wrangler dev`(R2 バインディング `remote: true`、127.0.0.1 のみ、使い捨ての認証トークン)の Worker で R2 のマルチパート API(64 MiB パート)を叩いた**。各 約 1 分 40 秒。読み戻し(同じ Worker の GET)の SHA-256 が手元の 4 本すべてと一致。ETag は md5 / md5 of part md5s-N が手元の計算と一致。記録: `ci/artifacts.lock`。(この一時 Worker は deploy していない。終了済み。)

一時 API トークン: 最小権限の測定と HeadObject のために `ci1-tmp-…` の名前で 6 本を作り(期限付き)、作業の最後にすべて `cf accounts tokens delete` で失効させた。**実測**: `cf accounts tokens list` にはアカウントの元からのトークン `GitHubActions` と `R2 Account Token` の 2 本だけが残る(私は触っていない)。テストで使った `fl-web-cache` の 20 個のオブジェクトも削除した(空)。資格情報の値はどの文書・ログ・コミットにも書いていない(`grep` で `cfat_`・`cfut_`・`Bearer`・`<CREDENTIAL-NAME>=` を成果物に対して探して 0 件)。

## 2. `runs-on/cache` を R2 で使う(一次資料と実測。`tables/runs-on-cache-r2.md`)

一次資料: https://github.com/runs-on/cache の README と、v5.0.7(`88d90644…`)の `src/custom/backend.ts`。必要な設定は `RUNS_ON_S3_BUCKET_CACHE`、`RUNS_ON_S3_BUCKET_ENDPOINT=https://<account>.r2.cloudflarestorage.com`、`RUNS_ON_S3_FORCE_PATH_STYLE=true`、リージョン `auto`、`<CREDENTIAL-NAME>`/`<CREDENTIAL-NAME>`(R2 の S3 トークン)。R2 の制約(オブジェクト 5 TiB、パート最大 10,000、同一キーの書き込み 1 回/秒)とは、キャッシュ(32 MiB パート、キーが内容ごとに別)で衝突しない(読み)。

実測(R2 の 1 バケット限定の一時トークンで、アクションの `dist/` を Actions の外から、のち act で composite action ごと): save 単発・5 パートのマルチパート、restore(内容の SHA-256 一致)、lookup-only、ミス、save→lookup の検証、すべて動いた。AWS SDK の checksum ヘッダは R2 に受け入れられた。

**落とし穴(いずれも ワークフローで対処)**: (a) バケットが空だと**黙って GitHub の Actions Cache に切り替わる**。(b) RunsOn という第三者のランナーサービスのランナー(`RUNS_ON_RUNNER_NAME` が非空)では AWS 資格情報を捨てる(標準の GitHub ランナーでは起きない。ラッパーは念のため空にしてある)。(c) リージョンは `RUNS_ON_AWS_REGION` が `AWS_REGION` より先(RunsOn のランナーだけがこれを設定する。ラッパーは `auto` に固定してある)。(d) 結合版(`runs-on/cache`)の post-step は『ジョブが成功したときだけ』保存し、保存の失敗は warning のまま成功扱い。→ `restore` / `save` の単独アクションを段階ごとに使い、save の直後に lookup で存在を確かめる(`ci/actions/r2-cache`)。
**原因不明の事象**: 約 10 分間、同じ 16 MiB 区間の署名付き URL の Range GET が途中で止まった(curl の HTTP/2、action の node の両方。action は warning で『ミス』扱い)。その後は同じ URL が 0.2 秒で返り、restore も成功した。この機械の負荷(load 45)か R2 側かは不明。→ restore の失敗を数えて summary に出す。

## 3. 段階キャッシュの設計(Dockerfile の層の規則。`ci/README.md` に表)

* **キーは入力だけから作る**(`ci/plan.sh`): ファイルのハッシュ(`git ls-files -s`)、ツールのバージョン(`ci/versions.env`: emscripten 6.0.12 と emsdk の rev、rust 1.99.0、python 3.14、node 24)、インストーラーの SHA-256(`ci/artifacts.lock`)、ビルド設定(`app/scripts/build-engine-bundle.sh` の SHIP_PRESET を展開した後の値。`ci/lib/effective_env.sh` が preset の部分だけを部分シェルで評価する。ship2 の `F2xH2` の写像も読める)、`build-engine-bundle.sh` の**その段階の関数の本文**(他の段階の変更で無効にならない)。前段のキーを次段のキーに入れる(層)。
* **何を復元し何を走らせるか**(`ci/needs.sh`): bundle のキーが hit なら build ジョブは起動しない。link と seed が hit なら bundle の段階だけ。miss の段階だけ、その入力(前段の出力)を復元または実行する。256 通りのヒットの組合せで不変条件を確かめる(`ci/test-needs.sh`)。`build-engine-bundle.sh` は**どの段階でも**先に `binaries/win64/FLEngine_x64.dll` と SHA-256 を確かめるので、build ジョブは常に抽出(74 MB)を復元する。
* **wasm のオブジェクトはモジュールごと**(FLEngine、dsp、Blend2D、FreeType、QFC、ILTools、runtime の 7 つ)、**生成 C の内容のハッシュ**で(`ci/objkeys.sh`)。`build_nmod_wasm.sh` は既存のオブジェクトを飛ばす(読み、かつクリーンな木の実行で確認)ので、復元したオブジェクトの単位は再コンパイルされない。
* **各段階は成功した直後に保存**し、`lookup-only` で存在を確かめる(保存の失敗は黙って成功扱いになるため)。後段が失敗しても前段の層は残る。
* **インストーラーは、抽出の段階が miss のときだけダウンロード**する(読み取り専用トークンのアーティファクト用バケットから、ETag・サイズ・SHA-256 を `ci/artifacts.lock` と照合)。
* 同時実行は `concurrency: deploy`(待たせる)、`permissions` は最上位が空・ジョブごとに `contents: read`、timeout はジョブごと、アクションはすべてコミット SHA で固定、`main` への push で動く(docs だけの変更と他のブランチの push は動かさない。依頼者の決定)。本番デプロイは `main` への push、または `main` での dispatch の `dry_run=false`、dispatch の既定は `dry_run=true`。
* `cache table`(summary): 段階・キー・操作・結果(hit/miss/restored/saved + verified/RESTORE FAILED)と、**lookup が見つけたのに restore が失敗した数**を先頭に出す(PM 指示)。

## 4. act(ホスト実行)で動かした範囲(GitHub 上の最初の実行より前。GitHub 上の実行は §4c)

`gh workflow run deploy.yml --ref ci/cloudflare-deploy` は **HTTP 403(Resource not accessible by integration)**。GitHub App の権限に actions の書き込みが無い。Variables・Secrets・runners も同じ 403。よってこの時点では **GitHub 上の実行は 0 回**だった(のち、PM の指示でトピックブランチだけの `push` トリガーのドライランを GitHub 上で実行した: §4c)。その前に `nektos/act` 0.2.96 を**ホスト実行モード**(`-P ubuntu-24.04=-self-hosted`、この機械で直接。コンテナ無し)で使い、本物の composite action と `runs-on/cache` を本物の R2 に対して動かした:

| ジョブ | 結果(実測) |
|---|---|
| `plan` | 静的検査、設定の検査、キー 10 個、R2 への lookup 10 件(全部クリーンな miss)、`needs.sh`。成功 |
| `vst3` | アーティファクト用バケットから読み取り専用トークンで 3 本をダウンロード(Serum 2 は 1.3 GB)→ETag・サイズ・SHA-256 検証→VST3 3 本を抽出→キャッシュへ save→lookup で検証。20 秒+6 秒。成功 |
| `app` | npm ci、型検査、vite、vitest 1,290 passed + 7 skipped、Worker テスト。**1 回目は 3 件失敗**(私が `app/wrangler.jsonc` のバケット名を `fl-web-assets` に変えたため。テストは placeholder 名 `fl-studio-web-assets-changeme` を固定で使う=変更を戻した)、2 回目は私の一時 uploader の `wrangler dev` とのポート衝突(終了させた)、3 回目 成功 |
| `build` | 冷たい実行: extract→felift→b2d→lift→objects 7 群→link→seed→bundle が全部『実行→save→verify』(各 save は 0.4〜9 秒)。wasm objects 22 分 19 秒(CI_JOBS=2)、link 11 分 25 秒。2 回目の plan は全 hit(`need_build=false`)。bundle・link・dsp のオブジェクトを R2 から消した再実行: dsp 群のオブジェクトだけが miss になり、dsp の 12 個だけを再コンパイル(3 分 54 秒)→ save → verify。link は 8 分 19 秒(wasm-opt 込み)、bundle(brotli・manifest・SHA256SUMS)は 7 分 16 秒で、build ジョブ全体が成功。出来た wasm の SHA-256 は冷たい実行と同じ(1efa6a97…)で、各 .br のバイト数も同じだった(再コンパイルしても出力が変わらない) |

act は本物の GitHub ランナーではない(標準ランナーの実機、`sudo`、`actions/setup-python` の 3.14、`/opt/emsdk` の復元と保存、ランナーのメモリ・ディスクは act では確認できない)。これらは §4c の GitHub 上の実行で確認した。

## 4b. 標準ランナー(private リポジトリ)で足りるか(依頼者の指摘への対応。数字と出典)

**誤りの訂正**: 初版は RunsOn(第三者のランナーサービス)と 16 vCPU の大きいランナーを前提にし、ディスク『100 GB』にも根拠が無かった。依頼者が指定したのは『キャッシュに `runs-on/cache` を使う』ことだけで、ランナーの指定は無い。ランナーは標準の GitHub-hosted `ubuntu-24.04` に固定し、RunsOn・larger runner の記述と変数 `CI_RUNS_ON` を全部消した。

**標準ランナーの仕様**(https://docs.github.com/en/actions/reference/runners/github-hosted-runners、2026-11-04 に読んだ表『Standard GitHub-hosted runners for private repositories』): `ubuntu-24.04`/`ubuntu-latest` = Linux **2 vCPU、8 GB RAM、14 GB SSD**(x64)。public リポジトリは 4 vCPU・16 GB。1 ジョブの実行時間の上限 **6 時間**(https://docs.github.com/en/actions/reference/limits の『All GitHub-hosted runners / Job execution time』)。PM の見積り『7 GB』は現行の文書では 8 GB。

| 項目 | 実測(この機械) | 標準ランナーとの比較 |
|---|---|---|
| `emcc -Oz` 1 単位 | 最大 RSS **1.43 GB**(130 単位、`/usr/bin/time`)。2 並列で約 3 GB | 8 GB に収まる |
| link(emcc) | 0.63 GB(`link.time`) | 収まる |
| `wasm-opt -Oz`(F2xH、174 MB の入力) | `BINARYEN_CORES=1` で `/usr/bin/time -v`: **最大 RSS 5.19 GB(5,191,208 kB)**、CPU 時間 1,287.8 s(21.5 分)+ system 13.9 s、壁時計 30 分 59 秒(この機械の負荷で CPU の 70% しか得られなかった値)。出力の SHA-256 は既定のスレッド数で作った物と同一(`ec7efc8e72ca…`)。既定のスレッド数では ps の標本で 3.9〜4.8 GB(最大値ではない) | 8 GB に収まる。余裕が小さいので、ジョブの最初に swap ファイル(RAM+swap が 12 GB 未満なら追加)を作る |
| 作業ディレクトリ | `ci-work` 1.7〜1.9 GB(F2xH2 の完走後 1,674 MB、`onat/gen` の削除後)。ピーク(最適化前の wasm 174 MB と最適化後の出力が併存)は約 2.1 GB | |
| emscripten | `/opt/emsdk` 1.9 GB(`du`) | |
| FL インストーラー | 1,106,033,456 B(抽出後に削除) | |
| cargo / rustup / npm | cargo レジストリ 313 MB(この機械の値、他の用途の分を含む)、rust minimal ツールチェーンは約 0.2 GB、npm キャッシュ(act で保存した値は 1.1 GB だがこの機械全体の `~/.npm` のため過大) | |
| **ディスクのピーク(合計の見積り)** | **約 7 GB** | 14 GB に収まる。足りなければ `/mnt` に作業ディレクトリを置き、最後に未使用の事前インストール物を削除(`ci/preflight.sh`) |
| 冷たいビルド 1 回の時間 | 段階ごとの実測(CI_JOBS=2〜4、load 30〜55): 抽出 3 s、lift 44〜86 s、上流 Blend2D 57〜102 s、wasm objects 990〜1,222 s(合計 CPU 3,635 s)、link 454〜562 s、bundle(brotli q11)506〜831 s | 2 vCPU: コンパイル 3,635 CPU 秒 ÷ 2 ≒ 30 分、link + wasm-opt 約 12〜25 分、brotli 約 9〜14 分、ツールチェーンの用意・キャッシュの保存で約 10 分 → **約 70〜100 分**(見積り。2 vCPU の実測は 0)。6 時間の上限の 1/4 強 |

並列度は `ci/lib/common.sh` の `jobs_default`(CPU 数と RAM/3 GB の小さい方)が決める。2 vCPU / 8 GB では 2。`wasm-opt` は 1 スレッドで CPU 21.5 分かかるので、2 vCPU の壁時計はスレッドの並列化の効き次第で約 11〜22 分(並列化の効きは未測定)。

ジョブの最初の `ci/preflight.sh` が `nproc`・`free`・`df` を表示し、RAM+swap が 10 GB 未満なら swap ファイルを作り(`/mnt` が別のファイルシステムで空きがあればそこへ)、作業領域の空きが 10 GB 未満なら(`/mnt` が別のファイルシステムなら)`/mnt` を使い、それでも足りなければ未使用の事前インストール物(.NET、Android、GHC、CodeQL、Swift)を消し、それでも足りなければ原因の分かる文面で失敗する。第三者のアクションは使わない。**標準ランナーで OOM・ディスク不足・6 時間超過が実際に起きた場合の larger runner などは『依頼者が選ぶ選択肢』として Issue に一言書いただけで、導入していない。** GitHub 上のビルドは私には走らせられない(dispatch が 403)ので、『初回の冷たいビルドがテストになる』と Issue に書いた。

費用(GitHub Actions): 標準 Linux 2 コアは $0.006/分、ジョブごとに分単位で切り上げ(https://docs.github.com/en/billing/reference/actions-runner-pricing)。private リポジトリは契約の『含まれる計算使用量』(Free $12、Pro・Team $18、Enterprise $300 / 月)から引かれる(https://docs.github.com/en/billing/concepts/product-billing/github-actions)。冷たいビルド 1 回(build 約 70〜100 分 + plan・app・vst3・deploy 約 15 分)は約 $0.51〜0.69、全部 hit の実行は約 10 分で約 $0.06(見積り)。

## 4c. GitHub の標準ランナーでの実行(トピックブランチの push トリガーによるドライラン。実測)

**経緯**: bot の権限では `workflow_dispatch` が 403(Actions の権限が無い。`gh run cancel` も 403)。PM の指示で、**トピックブランチ `ci/cloudflare-deploy` だけに一時的な `push` トリガーを足し**(ブランチの push は本番デプロイの条件を満たさないのでドライラン。§4c-3)、そこで実行した。一時トリガーは別コミットにして、PR の前に外した(PR の差分に入っていない)。依頼者が Secrets 5 個を設定済み(`gh secret list` で 5 個の名前を確認)。数字は Actions API の各 step の `started_at`/`completed_at`(GitHub のランナーの実測。この機械の測定ではない)。表は `tables/github-runs.md`。

**1. 実行の一覧**

| # | 実行 ID | コミット | 入力 | 結果 |
|---|---|---|---|---|
| 1 | 41733915505 | ab7f500 | 初回(キャッシュ空) | plan 35 s・VST3 1m13s(3 本、1.36 GB)・app 1m35s は成功。**build は preflight で 11 秒で失敗**(swap ファイルが空きを食った) |
| 2 | 41734573818 | 811031f | 冷たい build(VST3・app は 1 の hit) | **成功**。build ジョブ **45m39s**、deploy ドライラン 19 s。実行全体 約 47 分(46m55s) |
| 3 | 41741173698 | 2afafc7 | 同じ入力 | **成功、全体 約 1m20s**(build と VST3 は skipped) |
| 4 | 41741429508 | c155326 | 冷たくする塩(`peak1`)付き | plan が 8 秒で失敗(shellcheck SC2034: 計測用の関数の未使用ループ変数) |
| 5 | 41741564421 | 21c36c0 | 4 の修正(冷たい) | **成功**。build ジョブ **37m33s**(VST3 36 s、app 1m37s)。実行全体 約 39 分 |
| 6 | 41741830118 | eebd533 | wasm の入力の 1 行を変更(同じ塩。runtime を変えた場合の想定) | **成功**。build ジョブ **29m32s**。実行全体 30m29s |
| 7 | 41750829393 | f156fd5 | rebase 後のツリー(main 29b0b4b + この PR)。`lift/crates` などが変わっていたので felift から作り直し、emsdk・extract・b2d は hit | **成功**。build ジョブ **25m16s**(objects 10m23s、link 7m48s、bundle 4m43s)。実行全体 26m20s。wasm `dd3140ed…`(130,251,270 B) |
| 8 | 41755120853 | 716896d | 上の push 仕様のワークフロー(ブランチ上、一時の `branches` 追加つき。同じ入力なので全 hit) | **成功、全体 約 1m20s**。plan 27 s(静的検査に `ci/test-real-deploy.py` を含む)/ build・VST3 skipped / app 18 s / deploy ドライラン 23 s / 要約ジョブ 4 s。アップロード・`wrangler deploy`・Access 到達確認・拒否の step は skipped |

**2. 実ランナーで初めて分かったこと**

* ランナー: `ubuntu-24.04` 標準 = **2 vCPU、RAM 7,932 MB、swap 3,071 MB(元からある)、`/` は 72 GB のうち 59 GB が使用済みで空き 13,030 MB、`/mnt` は別のディスクではなく `/` と同じファイルシステム**。文書の『14 GB SSD』は、事前インストール物を除いた空きの目安として合っていた。
* **1 回目の失敗(私の誤り)**: `ci/preflight.sh` が 4 GB の swap ファイルを `/mnt` に作ったが、`/mnt` は `/` と同じなので空きが 13 GB → 8.9 GB になり『空き 12 GB 未満』で自分で止めた(11 秒)。手元では `/mnt` の中身を確かめていなかった。修正: swap は `/mnt` が別のファイルシステムのときだけそこへ、閾値を 10 GB に(実測のピーク約 7 GB)、`df` を全部表示。以後 swap ファイルは作られていない(RAM + swap = 11 GB ≥ 10 GB)。
* **2 回目の失敗**: 追加した計測用の関数の未使用ループ変数を、plan ジョブの静的検査(`shellcheck -S warning`)が止めた(8 秒)。直した。
* `aws` と `7z` は元から入っていた(`7-Zip` の step が 1 秒未満で、`apt-get` は走っていない)。`actions/setup-python` の 3.14、`setup-node` 24、`rustup` の 1.99.0(10 秒)、emsdk の取得(25〜31 秒)は問題なく動いた。
* **`runs-on/cache` は標準ランナーから R2 の S3 API で動いた**: lookup・save・save 後の検証・restore、すべて(plan 10 件の lookup、各段階の save→verify)。`RUNS_ON_S3_BUCKET_CACHE` などはワークフローの `env:` から渡り、空バケット変数の落とし穴はラッパーのガード(空なら失敗)で塞いである。リージョンの問題(RunsOn のランナーだけの `RUNS_ON_AWS_REGION` の優先)は起きなかった。
* Secrets はログで `***` に置換される(ログ全体を検索して、`cfat_`・`cfut_` 形式の文字列は 0 件。32 桁の 16 進数は Cloudflare のアカウント ID〔`env:` に平文で書いてある値〕だけ)。
* **デプロイジョブ(ドライラン)**: バンドルと `app/dist` の復元、アップロード計画(6 オブジェクト 28,733,518 B、`engine.kind` が `emscripten-fruity-v1` であることの検査)、`wrangler deploy --dry-run`(バインディングを一覧、Total Upload 21.77 KiB)が成功。19〜28 秒。配信バケットへは何も書いていない。
* **決定性(GitHub のランナーとこの機械)**: 生成 C 7 本の SHA-256 が GitHub のランナーとこの機械で**全一致**。同じ木(コミット 811031f の `git archive`)をこの機械で段階スクリプト(`CI_JOBS=2`)で作ると、**最終 wasm `cf384a1c0695a19a563cd736e674ad7862278b6a9c3a2919fa32860adf8dc750`(130,237,776 B)、glue(`19be80e762cb…`)、seed pack、DLL pack、それぞれの brotli が、ランナーの物とバイト単位で一致**した(`manifest.json` だけが時刻・prefix のため違う)。

**3. 本番デプロイの条件(式の落とし穴とテスト)**: 式 `inputs.dry_run == false` は、`workflow_dispatch` 以外のイベントでは `inputs` が空で、**`null == false` が真になる**(式の型変換)。そのままだと、push で本番デプロイの step が走る条件を満たしてしまう。そこで本番デプロイの 3 つの step(配信バケットへのアップロード、`wrangler deploy`、Access の到達確認)の条件は、イベントと ref を名指しする: `(push && ref == main) || (dispatch && dry_run == false && ref == main)`。『main 以外の ref で `dry_run=false` は拒否』の step は `dispatch && dry_run == false && ref != main`。`ci/test-real-deploy.py` が、これらの式をワークフローから読み出し、GitHub の式の意味(`&&`/`||` は被演算子を返す、型が違う `==` は数値で比べる)で、イベント・ref・入力の 10 通りを評価する(main への push → 本番、別ブランチの push → ドライラン、pull_request・schedule → ドライラン、main の dispatch は `dry_run` による、別ブランチの dispatch は `dry_run=false` なら拒否)。plan ジョブの静的検査で毎回走る。実行 1〜7 は、ブランチの push が本番に進まないことをこの設計で確かめた: 3 つの step と拒否の step は全実行で skipped だった(API で確認)。

**4. 段階ごとの時間(標準ランナー、冷たい build ジョブ)**: 実行 2 と 5 は同じ入力でキャッシュが空(実行 5 は `cache_salt` で冷たくした)。ランナーの個体差で 20% ほど違う。

| 段階 | 実行 2 | 実行 5 |
|---|---|---|
| emsdk(取得 + 保存 462 MB) | 31 s + 28 s | 25 s + 24 s |
| FL インストーラーの抽出 | 20 s | 13 s |
| felift / Blend2D / lift | 17 s / 79 s / 40 s | 14 s / 64 s / 38 s |
| wasm objects 130 個(`CI_JOBS=2`) | 18m02s | 14m13s |
| link(emcc 26 s + wasm-opt) | 15m10s | 12m54s |
| bundle(brotli q11) | 7m31s | 6m15s |
| **build ジョブ全体** | **45m39s** | **37m33s** |

**5. メモリとディスク(実行 5。`ci/preflight.sh end` の出力)**: ジョブ全体の cgroup(`hosted-compute-agent.service`)の `memory.peak` は **7.43 GB**(page cache を含むので RSS のピークではない)、`memory.swap.peak` は **0**(system.slice 全体でも 28 KB)、メモリの圧力(PSI)は終了時 0.00、カーネルの OOM の記録は無し。link の `emcc` は最大 RSS 0.63 GB(`link.time`)。wasm-opt 単体の RSS は**ランナー上では測っていない**。ディスクは `/` の使用が 59 GB → 63 GB(空き 13 GB → 8.9 GB。emsdk と cargo などを含む)、`ci-work` は 1.25〜1.67 GB。つまり標準ランナーの 8 GB・13 GB で足り、swap ファイルも事前インストール物の削除も使っていない。

**6. 温かい実行(同じ入力)**: 実行 3 は plan 31 s、app 12 s(キャッシュ hit で `note` だけ)、build と VST3 は skipped、deploy ドライラン 28 s で、**全体 約 1 分 20 秒、成功**。ビルドが飛ばされた。

**7. 『wasm の入力を変えた』場合(実行 6)**: `docs/investigations/a4-six-module-wasm/tools/wb.sh` に 1 行のコメントを足した(wasm 段階の入力 `H_WASM` の hash が変わる。この commit も一時 commit で、PR には入れない)。**`emsdk`(462 MB)・extract・Blend2D・lift は hit で復元**(emsdk の restore は 23 秒、ほかは 1〜4 秒)、**7 群のオブジェクトは全部 miss で 130 個を作り直し**(12m13s)、link 9m52s、bundle 6m01s、**build ジョブ全体 29m32s**。最終 wasm の SHA-256 は実行 2・5 と同じ `cf384a1c…`(コメント 1 行では出力が変わらないことの確認でもある)。ジョブ全体の cgroup の `memory.peak` は 7.31 GB、`memory.swap.peak` は 0。これで実ランナーで、冷たい build・温かい実行(全 hit)・一部変更(キャッシュの復元 + 作り直し)の 3 つの経路が確かめられた。

**8. 何が GitHub 上で確かめられていないか**: 本番デプロイ(`dry_run=false`)の step(配信バケットへのアップロード、`wrangler deploy`、Access の到達確認)。`main` への push か、`main` での dispatch の `dry_run=false` でだけ走る(ブランチの実行は必ずドライラン)。この PR のマージで初めて走る。ドライランが通ったのは、アップロード計画と `wrangler deploy --dry-run` まで。

**9. rebase 後のツリーでのドライラン(実行 7)**: main が 29b0b4b(workers1 の `PERF_BUCKET` 束縛を含む)に進んだので、ブランチを rebase し、`workers1-ci.diff` を別コミットで適用して、もう一度ドライランした。app ジョブ(Worker のテストを含む)は成功(1m29s)。deploy ジョブのドライランで `ci/lib/wrangler_config.py` が `PERF_BUCKET` を `fl-web-cache` に束縛した設定(`env.PERF_BUCKET (fl-web-cache)`)を書き、`wrangler deploy --dry-run` が通った。build ジョブは 25m16s。**この実行で、main の現在のツリー(29b0b4b)+ この PR の bundle(`fl-engine-ci-F2xH2-d73bb04d`、wasm `dd3140ed…`)がキャッシュに入った**。ただし**キャッシュのキーは入力から決まる**ので、これはこのツリーのときだけ有効: 実行 7 の後に main は 825fa28(memfs1。`lift/harness` を変更)に進んだため、その main + この PR のマージで走る最初の実行は bundle を作り直す(25〜46 分)。

**10. push 仕様のワークフローのドライラン(実行 8)**: 依頼者の決定(`main` への push で動く)に合わせたワークフロー(`on: push: branches: [main]`、`paths-ignore`、本番デプロイの条件、要約ジョブ)を、トピックブランチの一時的な `branches` 追加(別 commit、PR には入れない)で 1 回だけドライランした。全ジョブ成功。ブランチの push なので `real deploy: false`、本番の 3 step は skipped。

## 4d. 最初の本番デプロイ(実行 41755825673、main 15540a0、2026-11-04 11:24 JST)の失敗と修正

* **事実**: build ジョブは成功(41m16s)。deploy ジョブが `wrangler deploy` で失敗した。Worker は `Uploaded fl-studio-web` で上がり(版 `ba41b917-1e08-46b9-96db-b43027074914`、100%、02:24:38Z)、続くトリガー(カスタムドメイン)の更新で `The zone "<CF-VALUE-REMOVED>" does not exist on your account. [code: 10084]`(`Trigger configuration ... was only partially updated`)。依頼者が作った API トークン(Workers Scripts: Edit だけ)では、ドメインのゾーンを引けず、ドメインを付け替えられない。この push の前に、bundle のアップロード(6 オブジェクト、既定の `manifest.json` が最後)は成功していた。Access の到達確認の step は skipped になった。
* **その時点の本番(読み取りで確認)**: Worker は新しい版(app は main 15540a0)、既定 manifest は新しい bundle(`fl-engine-ci-F2xH2-88235937`、wasm sha256 `8cabc3a3…8a3`)。この wasm と glue は、同じ木をこの機械で作った物とバイト単位で一致。認証なしの 16 件(GET/POST/HEAD/Range、アプリ・/assets/・/__perf・manifest の 4 アセット)はすべて Access の 302(失敗で skipped になった step と同じ検査を手元で実行)。つまり本番は整合しており、赤かったのはドメインの更新の 1 点だけ。同じ push が続くたびに同じ失敗が繰り返される状態だった。
* **修正**: `wrangler deploy` をやめ、トリガーに触れない `wrangler versions upload` + `wrangler versions deploy <id>@100%` にした(`ci/stage-deploy.sh worker`。`versions upload` は『トリガーの変更は `wrangler triggers deploy` で』と出し、route つきの設定でも触れない)。カスタムドメインは既に付いているので触らない。
* **テスト(権限を足さずに)**: Workers Scripts: Edit だけの一時トークンで、本番とは別名の使い捨て Worker(ルートもドメインも無い)を使った: (a) 新規の Worker には `versions upload` できない(先に `deploy` が要る=本番の Worker は既にある)。(b) route つき(存在しないホスト名)の設定でも `versions upload` は成功し、トリガーは変わらない。(c) `versions deploy <id>@100% --yes` と `ci/stage-deploy.sh worker` は成功。終わって使い捨ての Worker を削除し(`This Worker does not exist` を確認)、トークンを失効した(`ci1-tmp` 0 本)。本番の Worker・ドメイン・バケットには触れていない。**ドメインが付いた本番の Worker に対する `versions upload`/`versions deploy` は未テスト**(次の push が最初)。
* **順序**: 『bundle のファイル(既定 manifest 以外)→ Worker の版 → 既定 manifest(最後)』にした。理由: (i) Worker の版が失敗しても既定 manifest は古いままで、本番は変わらず整合する(今回のような『Worker だけ新しい』一部更新にならない)。(ii) 以前の順(manifest まで上げてから Worker)では、『古いページ + 新しい manifest』の窓がアップロードの終わりから `wrangler deploy` の終わりまで(この失敗では約 8 秒)あった。新しい順の窓は、Worker の版の deploy から manifest の PUT(1 リクエスト)までの『新しいページ + 古い manifest』で、数秒以内。**結論: ページが新旧どちらの bundle とも動く期間は保証できない(schema が合わなければ止まる)ので、窓を最短にし、失敗時に整合が残る新しい順を採った。**

## 5. クリーンな木からの段階実行(実測。この機械、`CI_JOBS=4`、load 30〜55 の間。壁時計は参考値)

方法: `git archive` で作った木 + 依頼者の `~/ダウンロード` のインストーラー(`CI_INSTALLER_DIR`)。HOST_ROOT は CI が抽出した木(PM のワークツリーではない)。`SHIP_PRESET=F2xH`(rebase 前の main 7c0e9cc + 本ブランチ)。`NO_CGROUP=1`。

| 段階 | 実測 | 備考 |
|---|---|---|
| extract(FL インストーラー) | 2 s | DLL 6 + ゲストのファイル 218 を抽出、全 SHA-256 が `extract-dlls.tsv`・`vfs-seed-idle1.tsv` と一致。マニフェストの SHA-256 `e837a19f…` |
| felift | 9 s | cargo のキャッシュ有り(初回のネットワーク取得の時間は含まない) |
| lift(gen + resplit) | 78 s + 8 s | |
| 上流 Blend2D(wasm) | 57 s | `build_upstream.sh`(commit `a7f94767…` と tarball の SHA-256 を固定。ネットワークから取得) |
| wasm objects(44 単位 + runtime) | 990 s | 単位の emcc の最大 RSS 1.43 GB、130 個の合計 CPU 3,635 s |
| link(weblink + wasm-opt -Oz) | 562 s | wasm-opt の RSS を 4.18 GB まで観測(1 点の標本) |
| seed | 1 s | |
| bundle(brotli q11 ほか) | 506 s | |
| **合計** | 約 36 分 | |

出力: wasm raw **138,866,442 B**(ネイティブ 70,572,928 B の 1.968 倍)、brotli 11,032,150 B。`engine.ec7efc8e72ca.wasm`。`dll.50206405ba5f.flpack`(sha256 `50206405…`)と `seed.0f4b547e6656.flpack`(`0f4b547e…`)は ship2 の bundle の同名の物と**バイト同一**(= CI の抽出と PM の木の入力から作った seed pack が同じ)。wasm と glue に作業ディレクトリのパス(`/home/ubuntu`、`ciwork` ほか)は含まれない(`grep -c` 0)。

**生成 C の決定性・入力の同一性**(F2xH、gen + resplit のみ、各 約 80 秒): CI の入力での 2 回の実行と、HOST_ROOT だけを PM のワークツリーにした実行で、`flengine.c`、`m1.c`〜`m5.c`、`common.c` の SHA-256 7 本が**全一致**(`tables/generated-c-sha256.txt`)。ship1 の F1xH の参照(HEAD d425abd、`docs/investigations/ship1/logs/generated-c-sha256-A.txt`)とは `flengine.c`(`95e68e97…`)・`m2.c`・`m4.c`・`m5.c` が一致、`m1.c` は prune S と S+M の違い、`m3.c` と `common.c` の差は参照の HEAD からの差と読む(**未検証**)。FLEngine の既定構成(a4)の生成 C `cd9107ce…` は**再実行していない**(本ブランチは `lift/crates`・`lift/runtime` を変えていないので変わらないという読み)。

**キャッシュ項目の大きさ**(`tables/cache-sizes.tsv`。tar | zstd -3、アクションと同じ): extract 22.7 MB、felift 0.8 MB、b2d 1.5 MB、lift 64.9 MB、wasm objects 計 33.2 MB(fle 18.1、dsp 7.7、Blend2D 0.4、QFC 1.4、FreeType 0.9、ILTools 2.6、runtime 2.1)、link 17.8 MB、seed 22.6 MB、bundle 27.5 MB。**1 組で約 191 MB**。emsdk(`/opt/emsdk`)は 1.9 GB → 507 MB。

### 5b. ship2 の F2xH2(混成なし)との照合と、wasm の決定性

ship2 の記録(`docs/decisions/0208-ship2.md`、`lift/out-git/ship2-results/`): 混成なしの F2xH2(`NAME=A SHIP_PRESET=F2xH2 … gen resplit wasm weblink wasmopt`)の wasm は **130,225,915 B**、brotli **10,946,486 B**。**SHA-256 は記録が無く、ファイルも残っていない**。混成(`hyb.sh`)は `build-engine-bundle.sh` の外の手順で、ワークフローには入れていない。したがって『ship2 の wasm と SHA-256 が一致するか』は**確認できない**。代わりに次を測った。

| 木 | 構成 | wasm raw | brotli q11 | SHA-256 の頭 |
|---|---|---:|---:|---|
| 現在の `main`(d17b038)+ 本ブランチ、CI の段階スクリプト、CI_JOBS=2(`clean3`) | F2xH2 | **130,226,177 B**(1.8453 倍) | 10,950,321 B | `ad49eb28653a` |
| 7c0e9cc + `ship2.diff`(ship2 が作った時の木に相当)、CI の段階スクリプト、CI_JOBS=3(`clean4`) | F2xH2 | **130,225,915 B**(1.8453 倍) | (bundle は作っていない) | `ab7e6f9087be` |
| ship2 の記録(`A`) | F2xH2 | 130,225,915 B | 10,946,486 B | 記録なし |

F2xH2(`clean3`)の段階の時間(CI_JOBS=2、load 30〜55): extract 3 s、felift 9 s、lift 44 s、上流 Blend2D 102 s、wasm objects 1,222 s、link 531 s、seed 1 s、bundle 831 s(brotli q11 が長い)、合計約 54 分。`dll.50206405ba5f.flpack` と `seed.0f4b547e6656.flpack`(raw も brotli も)は ship2 の bundle の同名の物と**バイト同一**。glue と wasm は別物(ship2 の bundle は混成の weblink で作られているため。`ci/compare-bundle.sh`、`tables/compare-bundle-F2xH2-vs-ship2.txt`)。

**ship2 の木(7c0e9cc + `ship2.diff`)を CI の段階スクリプトで作ると、混成なしの F2xH2 の wasm は 130,225,915 B で、ship2 の記録とバイト単位で一致した**(`tables/clean4-ship2-tree.txt`)。SHA-256 は ship2 に記録が無いので一致とは言えない(サイズの一致までが事実)。現在の `main` の木では +262 B になる(130,226,177 B)のは、ship2 の木より後に入った runtime の変更(`lift/runtime/fle_hle.c`・`fle_realmod.c` など)のためと読む(同じ 2 つのオブジェクトだけが act の実行と違った事実と整合する。ただし 130,226,177 B の木から該当コミットを抜いた確認はしていない)。

**wasm の決定性**(同じ木・別のディレクトリ・別の並列数): 同じ木(`clean3` と `clean5`、F2xH2)を別ディレクトリ・別の時刻に作った 2 回のビルドの最終 wasm `flengine6.opt.wasm`(SHA-256 `ad49eb28653ab90a…`、130,226,177 B)と glue(`6ac98b8015b35fc2…`)は**完全に一致**した(`cmp`。`tables/wasm-determinism.txt`)。 別の2つのビルド(run1〔7c0e9cc 基準〕と act〔0a3ab9a 基準〕、作業ディレクトリ・並列数・時刻が違う)の C→wasm オブジェクト 130 個を `cmp` で比べると **128 個がバイト同一**で、違った 2 個(`rt/fle_hle.o`、`rt/fle_realmod.o`)は、この 2 つの基準の間で `lift/runtime` のソースが変わったファイルそのもの(`git diff --stat 7c0e9cc 0a3ab9a -- lift/runtime`: `fle_hle.c` +184 行、`fle_realmod.c` +6 行)。`tables/object-determinism.txt`。

## 6. 配信: Access → Worker の順で作り、認証なしの要求が通らないことを確かめた(実測)

調べた依頼者の私専用 Worker の方式(`cf` の一覧のみ。他の Worker のコード・設定は読んでいない): ゾーン `cabbagelettuce.com` の `<短い名前>.cabbagelettuce.com`(Worker のカスタムドメインの AAAA 記録)+ `self_hosted` の Access アプリ + 再利用ポリシー。`*.workers.dev` の物は Cloudflare 自動作成の『Preview URLs』ポリシー。同じ方式にそろえた。

順序(PM 指示): (1) `cf zero-trust access applications create` で `fl-studio-web`(`<CF-VALUE-REMOVED>`、セッション 24h、再利用ポリシー `df4639a8-…` を `policies: [{id, precedence: 1}]` で**参照**)。AUD は作成の応答から取得。(2) **その後** `wrangler deploy` が `routes: [{pattern: "<CF-VALUE-REMOVED>", custom_domain: true}]` で Worker に名前を付けた(設定は `ci/lib/wrangler_config.py` が `app/wrangler.jsonc` + 変数から生成。リポジトリにホスト名の直書き無し。Access の設定が無いのにホスト名が与えられたら拒否する)。(3) 認証なしの GET/POST の `/`、`/assets/manifest.json`、`/__perf`、`/static/x.js`、`/assets/fl-engine-ship2-F2xH2/manifest.json`、`/favicon.ico` の 12 件は**すべて 302 で `https://<CF-VALUE-REMOVED>/cdn-cgi/access/login/<CF-VALUE-REMOVED>` へ**(`logs/access-check.txt`、再デプロイ後も 8 件 `logs/probe-second.txt`)。PM も外から 3 件を確認した。(4) Worker は fail-closed のまま(`ACCESS_REQUIRED=true`、`ACCESS_TEAM_DOMAIN`・`ACCESS_AUD` が空なら全要求 403。生成設定で両方入れた)。`workers_dev`・`preview_urls` は false のまま。R2 の 3 バケットに公開アクセス無し。

デプロイした Worker: 最初は `fl-studio-web-private`(既存の Worker と衝突しない名前)。初回 version `06691b38…`(2026-11-04 04:55 頃、`main` 7c0e9cc の app)、PM 指示の再デプロイ version `8c455cf0…`(`main` 0a3ab9a の app = dedupe 既定 ON・Files・ビーコンの変更を含む)。配信バケットの bundle は SHIP2 の `fl-engine-ship2-F2xH2`(`manifest.json` の SHA-256 `1e7c6ccc…`、md5 `3573980654f17af141871ea9f1abac95` = PM が既定の入口にしている物と同じ)。R2 から読み戻した 6 オブジェクトの SHA-256 は bundle の `SHA256SUMS` と全一致(`tables/assets-bucket-readback.sha256`)。

**本番バケット `fl-web-assets` の HeadObject(S3 API、読み取り専用の一時トークン、測定後に失効を確認、`tables/r2-head-object.tsv`)**: wasm・glue・seed pack・DLL pack の 4 つは `Cache-Control: public, max-age=31536000, immutable` と `Content-Encoding: br`、`manifest.json`(ルートと `fl-engine-ship2-F2xH2/` の 2 つ)は `Cache-Control: no-cache` で Content-Encoding なし。ContentType は wasm が `application/wasm`、glue が `text/javascript`、2 つの pack が `application/octet-stream`、manifest が `application/json`。ContentLength は保存された(br 後の)バイト数(wasm 11,339,214 B)。Worker は R2 の CacheControl を `cacheControlFor` より優先するので、ブラウザへは `public, max-age=31536000, immutable, no-transform` が出るはず(コードからの推定。Access の内側の実応答ヘッダーは見ていない)。

**Worker 名の変更(依頼者の指摘、PM 指示)**: 名前の `private` を外して `fl-studio-web`(Access のアプリ名と同じ)にした(`app/wrangler.jsonc` の `name`。ワークフローで名前を変える変数は無い)。手順と確認: (1) 新名の Worker をルート無しで deploy(workers.dev 無効のまま URL が付かない。version `efda5c06…`)。この時点で旧 Worker がホスト名を持っていて、認証なしの GET/POST 8 件は全部 Access の 302(`logs/probe-rename-before.txt`)。(2) 新名の Worker をルート付きで deploy(version `497d0938…`)。カスタムドメインは旧 Worker から新 Worker へ 1 回の操作で付け替わり(API の `workers/domains` の一覧で `<CF-VALUE-REMOVED> -> fl-studio-web` を確認。付け替えの間にホスト名が無保護になった瞬間は観測していない=Access はホスト名に付いているので、Worker が無い間も未認証の要求は Worker に届かない〔読み〕)、認証なしの 8 件は全部 302(`logs/probe-rename-after.txt`)。(3) **旧 `fl-studio-web-private` だけ**を `wrangler delete --name fl-studio-web-private --force` で削除(他の Worker・バケット・Access アプリには触れていない。削除後も 8 件すべて 302、`logs/probe-rename-after-delete.txt`)。Worker テスト 43/43 passed(名前の変更後)。`docs/decisions/0135-frontend-app.md` の旧名の記述は、当時の placeholder の記録なので直していない。

**依頼者が Access 経由で開いて動くかは未確認**(依頼者にしかできない)。**最小権限の実測**: (a) Worker の deploy(カスタムドメイン込み)は『Account · Workers Scripts · Edit』だけのトークンで通った。(b) `wrangler r2 object put --remote` は**バケット限定の R2 トークンだと 403**(REST API は account 全体の『Workers R2 Storage Write』を要求)。→ 配信バケットへの書き込みは S3 API + そのバケットに限定したトークン。Content-Encoding・Cache-Control・Content-Type が保たれることを `head-object` で確認。assets 用トークンは artifacts バケットを読めない(AccessDenied)。

## 7. GitHub の Secrets(Issue #3、日本語)

GitHub の Secrets は私のトークンでは作れない(403)ので、Issue #3(https://github.com/signature-studio/signature-studio-develop/issues/3)に Secrets **5 個**の名前と権限を書いた。依頼者が 5 個を設定し(PM が `gh secret list` で 5 個の名前を確認)、Issue #3 はクローズ済み。Variables は 0 個。値はどこにも書いていない。

## 8. 出典なしの選択(S-ci1-)

| 番号 | 選択 | 理由・代替 |
|---|---|---|
| S-ci1-1 | キャッシュ用バケットの `cache/` を 60 日で失効 | 古い層が溜まるのを避ける。代替: 30 日(再ビルドが増える)、無期限(容量が増える。費用は無料枠内なので効くのは整理だけ) |
| S-ci1-2 | ホスト名 `<CF-VALUE-REMOVED>` | PM が第一候補と指定。依頼者の既存の私専用 Worker の名前の付け方(`os.`、`pushnot.`、`gha-graph.` …)にそろえた |
| S-ci1-3 | 配信バケットへのアップロードは S3 API(aws CLI)+ バケット限定トークン | 実測: wrangler の REST 経路はバケット限定トークンを 403 で拒否する。代替: account 全体の R2 Write トークン(依頼者の他の 5 バケットも書ける)。PM 文面の『wrangler-action で r2 object put 相当』からは外れる=wrangler-action は Worker の deploy のみ |
| S-ci1-4 | S3 トークンは 2 組: 成果物バケット(インストーラー)は読み取りのみ、キャッシュ+配信の 2 バケットは読み書き(その 2 つに限定) | PM 指示。1 つのトークンを複数の指定バケットに限定できることを Cloudflare の文書(https://developers.cloudflare.com/r2/api/tokens/ の 5 番)と実測で確認。代替: 3 組(さらに分離、設定が増える)、1 組(インストーラーも書き込める) |
| S-ci1-5 | キーは `fl-<段階>-<40 hex>` の固定長 | `runs-on/cache` の lookup が前方一致なので、あるキーが別のキーの前置にならないように |
| S-ci1-6 | wasm のオブジェクトを 7 群(fle、m1〜m5、runtime)に分け、群のキー = その群の C の内容 + wasm 入力 | 依頼者の挙げた単位(FLEngine、dsp、Blend2D、FreeType、QFC、ILTools、runtime)に合わせた。単位ごと(44 個)にしなかった理由は R2 の操作数と YAML の長さ。代替: 単位ごと |
| S-ci1-7 | 並列コンパイル数 = min(CPU, RAM/3 GB)(自動。標準ランナーでは 2) | `emcc -Oz` の単位の最大 RSS 1.43 GB(130 個の実測)に余裕を取った。-O0 の巨大単位(最大 9 GB、決定 0130 の記録)は Oz 構成には無い。上書きは環境変数 `CI_JOBS`(GitHub の Variables ではない) |
| S-ci1-8 | ランナーは標準の GitHub-hosted `ubuntu-24.04` に固定(第三者のランナーも larger runner も使わない)。`ci/preflight.sh` の閾値: RAM+swap 10 GB、作業領域の空き 10 GB(初版は 12 GB だったが、実ランナーの初回で、swap ファイルが作業領域の空きを食って失敗した〔§4c〕。実測のピーク約 7 GB に対する余裕) | **誤りの訂正**: 初版は RunsOn と 16 vCPU の大きいランナーを前提にしていた(依頼にも根拠にも無い私の決定)。閾値の根拠は §4b の実測(wasm-opt 最大 5.2 GB + OS、ディスクのピーク約 7 GB)。代替: 環境変数 `CI_NEED_MEM_MB`・`CI_NEED_DISK_MB` |
| S-ci1-9 | 本番デプロイは `main` への push、または `main` での dispatch の `dry_run=false`。dispatch の既定は `dry_run=true` | 依頼者の決定(push で動かす)。ほかのブランチの push は動かさず、ブランチの dispatch で `dry_run=false` は拒否する。代替: Environment の承認者(private でのプラン条件は未確認) |
| S-ci1-10 | ツールのバージョンの固定値(emsdk rev、rust 1.99.0、python 3.14、node 24) | この機械で参照のバンドルを作った環境の値 |
| S-ci1-11 | 本番デプロイの条件を `(github.event_name == 'push' && github.ref == 'refs/heads/main') || (github.event_name == 'workflow_dispatch' && inputs.dry_run == false && github.ref == 'refs/heads/main')` にした(3 か所の step + 要約)。式は `ci/test-real-deploy.py` で全組み合わせをテストする | 実測で得た知識: dispatch 以外のイベントでは `inputs` が空で `inputs.dry_run == false` が真になる(実行 1〜7 で `push` を足すときに気づいた)。イベントと ref を名指しする。代替: `inputs.dry_run != true`(これも空で真になる)ので不可 |
| S-ci1-12 | `on: push` は `branches: [main]` と `paths-ignore: [docs/**, **/*.md]` | 依頼者の決定(push で動かす。docs だけの変更では動かさない)。`ci/**`・`app/**`・`lift/**`・`.github/workflows/deploy.yml` などは動かす(`paths-ignore` なので、無視の表に無い物はすべて動かす) |
| S-ci1-13 | ブランチの検証用の一時的な `push` トリガー(ブランチ名を `branches` に足す commit)は別 commit にして PR の前に外す | PM の指示。bot は dispatch できない(403)。条件がイベントと ref を名指しするので、そのブランチの push は必ずドライラン |
| S-ci1-11 | `lift` のキャッシュは gen の生成 C(`onat/gen`)を持たず、resplit 後の `wsrc` と DLL・realmod 表・モジュール表を持つ(64.9 MB) | 後段が読む物だけ(クリーンな木の実行で確認)。代替: 全部(約 4 倍) |
| S-ci1-12 | link のキャッシュは最適化前の wasm を持たず、メモを書いたファイルで置き換える | `stage_bundle` は最適化前のファイルの存在(`-s`)しか見ない。300 MB を保存する価値が無い |
| S-ci1-13 | GitHub の Variables は 0 個。名前・設定はワークフローの `env:` に置く(Worker 名は `app/wrangler.jsonc` の定数、`ACCESS_POLICY_ID` は置かない) | PM 指示(依頼者の指摘)。いずれも秘密ではなく、リポジトリで管理する物。選び直せるのは `workflow_dispatch` の入力 `ship_preset` と `cache_salt` だけ |

## 9. 未確認

* GitHub 上の**本番デプロイ**(`dry_run=false`: 配信バケットへの書き込み、`wrangler deploy`、Access の到達確認 step)は一度も実行していない(`main` への push か `main` での dispatch の `dry_run=false` だけ。この PR のマージで初めて走る)。ドライランで通ったのは、計画・`wrangler deploy --dry-run` まで。
* 標準ランナーのジョブ間・時刻のばらつき(同じ入力の冷たい build が 37.5 分と 45.7 分)。`wasm-opt` 単体の最大 RSS はランナー上では測っていない(`memory.peak` は page cache を含む)。標準ランナーの空き 13 GB は今日のイメージの値で、イメージの更新で変わりうる(preflight が見て、足りなければ事前インストール物を消す。この削除の分岐は実ランナーでは一度も走っていない)。
* 実 R2 でのキャッシュの往復のうち、act のフルビルドで確かめた範囲を超える部分(下の表)。依頼者の Access 経由の表示。依頼者が設定する Secrets の権限が足りるか(私の一時トークンは同じ権限の組で試した=同一とは限らない)。
* ship2 の混成(`hyb.sh`)はワークフローに無い(混成込みの SHIP2 の wasm は再現していない)。混成なしの F2xH2 の wasm は、ship2 の木では記録のサイズと一致した(§5b)。
* FLEngine の既定構成の生成 C `cd9107ce…`・全体テスト 178/2,274/0 は再実行していない(`lift/crates`・`lift/runtime` を変えていない)。
* 非決定性: 最終 wasm の非決定性は見つからなかった(§5b)。manifest.json は作成時刻と HEAD を含むので毎回違う(設計どおり)

## 10. PM の判断点

1. `SHIP_PRESET` の既定は `F2xH2`(ワークフローの `env:`。ship2 は main に入っている。選び直せるのは `workflow_dispatch` の入力だけ)。`push` トリガーは依頼者の決定で有効(`main` だけ、docs だけの変更は除く)。事実は §10b。
2. S-ci1-3(R2 への書き込みを wrangler-action でなく S3 API にした)の承認。
3. 本ブランチの PR(私は開いていない)。
4. 配信中の Worker を、ワークフローの最初の実デプロイで置き換えてよいか(今の物は私がこの機械から出した)。

## 11. lift の忠実性の方針(AGENTS.md、2026-11-04 追加。`docs/policies/lift-fidelity.md`)との関係

* この決定が足した物は、CI のスクリプト・ワークフロー・Worker の名前 1 行(`app/wrangler.jsonc`)・`build-engine-bundle.sh` の `capped()` の `NO_CGROUP=1`(ビルドの実行方法の切り替えで、既定では何も変わらない)だけ。**ランタイム・ページ・Worker のコードに、FL の窓・クラス・位置による分岐や、FL の挙動の代行・補正・特別扱いは 1 行も足していない**(`git diff origin/main..ci/cloudflare-deploy` の `-` 行は 7 行で、すべて読んだ: 置き場のワークフロー 6 行と Worker の名前 1 行)。
* 方針の分類(外部 API の契約の実装/余計なものの除去/通信・変換の削減)のどれにも当たらない『ビルド・配信の基盤』であり、製品の出力を変えないことを実測で確かめた: 同じ木の 2 回のビルド、および一部のキャッシュを消した再ビルドで、最終 wasm の SHA-256 は同一(§5b、§4 の `build` 行)。
* 余計なロジックの点検: この作業では `app/worker/index.ts` の `serveAsset` を読んだ範囲(R2 のメタデータをそのまま HTTP ヘッダーにする。FL 固有の分岐は見なかった)に限る。Worker・ページ・ランタイム全体の点検は **していない**(未確認)。

### 10b. `main` への push で自動デプロイする(依頼者の決定)ことの事実

ワークフローは `main` への push で動き、本番へデプロイする(`on: push: branches: [main]`、`paths-ignore: [docs/**, **/*.md]`)。docs だけの変更と、他のブランチの push は動かさない。手動実行(`workflow_dispatch`)は、`main` で `dry_run=false` を指定したとき以外はドライランで、他のブランチで `dry_run=false` は拒否する(§4c-3、`ci/test-real-deploy.py`)。Access の保護は変わらない(デプロイの最後に、認証なしの要求が Access に止められるかを確かめ、止まらなければジョブが失敗する)。

**費用の単位**: private リポジトリの標準 Linux 2 コアは $0.006/分、ジョブごとに分単位で切り上げ(https://docs.github.com/en/billing/reference/actions-runner-pricing)。含まれる計算使用量は Free $12 / Pro・Team $18 / Enterprise $300(月)(約 2,000 分 / 3,000 分 / 50,000 分に相当)。これは試算であり、契約のプランは確認していない。

| 変更の種類 | 実行される物 | 時間(標準ランナー、実測) | 1 回の費用(試算) |
|---|---|---|---|
| `app/**` だけ(bundle は hit) | plan + app + deploy + 要約 | **約 1 分 20 秒**(実行 3 と 8: すべて hit) | 約 $0.02(3〜4 分) |
| `lift/runtime`・`lift/harness` など wasm の入力 | 7 群のオブジェクトの全部 + link + bundle(emsdk・extract・lift・Blend2D は hit) | build ジョブ **29.5 分**(実行 6: objects 12m13s + link 9m52s + bundle 6m01s。emsdk の restore は 23 秒) | 約 $0.20 |
| `lift/crates`(fe-lift)・生成器の入力 | felift → lift → 全オブジェクト → link → bundle | 冷たい build ジョブ **25〜46 分**(実行 2・5・7) | 約 $0.2〜0.31 |
| インストーラー(`artifacts.lock`)・`SHIP_PRESET`・`cache_salt` | 全部 | 同上 + VST3 約 1〜2 分 | 約 $0.3 |

* **キーは入力だけから決まる**ので、ブランチによらず同じ入力なら hit する(R2 の 1 つのキャッシュを全ブランチが共有)。同じ入力の冷たい build が 45.7 分と 37.5 分になる(約 20% のばらつき)ことが実行 2 と 5 で見えた。
* **wasm の入力の変更は、オブジェクトの 7 群の全部を作り直す**(群のキーに `lift/runtime`・`lift/harness` の内容の hash が入るため。群ごとに分けた利点が出るのは、生成 C の一部だけが変わる `lift/crates` 側の変更のとき)。runtime をいじる今のような時期は、wasm の入力を変える push ごとに約 25〜40 分・$0.2〜0.3。1 日 10 回なら月 $60〜90、1 日 3 回なら月 $18〜27(含まれる使用量は上のとおり)。
* **『app だけの変更』でも安いとは限らない**: キーは**ツリー全体**の入力から決まるので、前回の実行より後に `lift/**` が main に入っていれば(その push の実行が走らなかった、または失敗した場合)、app だけの push でも bundle を作り直す。
* **同時実行**: `concurrency: deploy`(`cancel-in-progress: false`)。実行中の 1 本 + 待ちの 1 本だけが残り、間の push の実行は(待ちが新しいものに置き換わって)取り消される(GitHub の文書の挙動。実測はしていない)。連続して main に push しても、デプロイは直列に走り、最後の push のツリーが最終的に配信される。取り消された実行の push は、次の実行が含む。
* **失敗の通知**: GitHub の標準では、メールか Web の通知を有効にしている場合に、**その実行を起こしたユーザー**(push した人。bot が push すれば bot のアカウント)へ通知が行く。失敗だけの通知も選べる(https://docs.github.com/en/actions/concepts/workflows-and-actions/notifications-for-workflow-runs)。それとは別に、最後のジョブ `result of the run` が、どのジョブがどう終わったかの表と『本番は変わった/変わっていない/一部更新の可能性』を実行の要約に出し、失敗したジョブ名を注釈(`::error::`)に出す。実行 8(ブランチのドライラン)で、この要約のジョブが成功することを確認した。
* **途中で失敗したとき**: bundle のアップロードは既定の `manifest.json` を最後に上げるので、アップロードの途中で失敗しても、既定の入口は前の bundle を指したままで、新しい bundle のオブジェクトが増えるだけ。`wrangler deploy` が失敗すれば Worker は前のまま。配信バケットのアップロードが済んだ後の `wrangler deploy` の失敗は、新しい bundle と古い Worker が併存する(長さと影響は測っていない)。

**ページと bundle の整合**: この設計では**不整合は起きない**。実行ごとに、その実行のコミットのツリー全体の入力から bundle のキーを作り、同じコミットの app と一緒に配信し、既定の manifest を最後に上げる。bundle が app より古い組み合わせは作れない(bundle が hit なら、その bundle はその入力から作った物)。ページと bundle は一緒に出す前提で、これは必要な前提でもある(PM の指摘: memfs1 の新形式の bundle を古いページで開くと、schema メッセージで止まる)。manifest に app のコミットを持たせることは今回は行わない(`app/shared/manifest-guard.ts` が見るのは engine.kind だけ)。小さな窓は、配信バケットへのアップロードと `wrangler deploy` の間の数秒に、新しい bundle の既定の manifest と古い Worker/ページが併存すること(長さと影響は測っていない)。

**この push 配備が一度も走っていないこと**: `main` への push による本番デプロイ(配信バケットへのアップロード、`wrangler deploy`、Access の到達確認)は、GitHub 上で一度も走っていない。ブランチのドライランでは、これらの step は skipped で、アップロード計画と `wrangler deploy --dry-run` までが通った。この PR のマージが最初の実行になる。
## PM の判断と取り込み(2026-11-04)

- **依頼者の指摘への対応は妥当と判断した**: ランナーは標準の GitHub-hosted `ubuntu-24.04` に固定(第三者ランナーの RunsOn と大きいランナーは依頼に無く、担当=エージェントが決めたのは誤り。撤回)。Variables は 0 個、Secrets は 5 個(`<CREDENTIAL-NAME>`、`R2_ARTIFACT_*` 読み取りのみ、`R2_CACHE_ASSETS_*` キャッシュ+配信バケットの読み書き)。Worker 名は `fl-studio-web`(旧 `-private` の名は公開版が予定されているように読めるため)。Issue #3 は日本語。標準ランナー(private: 2 vCPU / 8 GB / 14 GB SSD、6 時間)で足りる根拠は担当の実測と GitHub の文書(`wasm-opt` のピーク RSS 5.19 GB、ディスクのピーク約 7 GB)だが、**GitHub 上での実行は 0 回**(dispatch が 403)なので、初回の冷たいビルド(見積り 70〜100 分)がテストになる。
- **ブランチの `app/scripts/build-engine-bundle.sh` の変更(+7 行、`NO_CGROUP=1` で systemd-run を使わない)**: AGENTS.md が systemd-run を禁止しているため。ship3 の担当が同じ変更を足す予定だったので、PR の方を正とし、ship3 では取り込まない(重複させない)。
- **lift の忠実性の方針との関係**: ランタイム・ページ・Worker のコードへの追加は無い(Worker 名 1 行を除く)。配信の Worker `serveAsset` の点検(余計なロジックの有無)は読んだ範囲のみで、Worker 全体は未点検。

(上の PM の節は main の版のまま。次の追記は ci1 による PR 作成時のもの。)

## ci1 の追記(PR 作成時。GitHub 上のドライランの結果と、依頼者の決定を受けて)

- 上の『GitHub 上での実行は 0 回』『初回の冷たいビルド(見積り 70〜100 分)がテスト』は、その後の実行で置き換わった: 標準ランナーで **冷たい build ジョブ 37.5〜45.7 分で成功**、温かい実行は約 1 分 20 秒(§4c)。ブランチの検証は、トピックブランチだけの一時的な `push` トリガー(必ずドライラン)で行った。そのトリガーは PR に入れていない。
- 依頼者の決定で、ワークフローは `main` への push で動き、本番へデプロイする(docs だけの変更と他のブランチの push は動かさない)。dispatch は `main` での `dry_run=false` 以外ドライラン。費用の事実は §10b。
- 残る未確認は GitHub 上の本番デプロイの step(§9)。

## PM の判断(2026-11-04、PR #4 のマージ時)

- 依頼者の決定に従い、**main への push が本番デプロイを起こす**(docs/**・*.md だけの変更は除く)。PM が先に決めた『dispatch のみ』は撤回した(依頼された形は push での継続デプロイで、止めておく決定点を PM が作ったのが誤り。docs/policies/owner-contact.md)。
- **CI に実ブラウザの操作・自作の playwright スクリプトは置かない**(依頼者が拒否)。CI の検査は、ワークフローの式のテスト(`ci/test-real-deploy.py`)、静的検査、Worker のテスト、アップロード計画の検査、`wrangler deploy --dry-run`、デプロイ後の Access の到達確認。bundle の中身の動作確認は CI の外(手元)で PM が行う。
- 初回の本番デプロイは PR のマージで起きる(実行 41755825673)。ブランチ上の本番 step はドライランで skipped だったため、**本番 step が GitHub 上で動くのはこの実行が最初**。

## PM の判断(2026-11-04、最初の本番デプロイの失敗と修正)

最初の本番デプロイ(実行 41755825673)は、bundle のアップロードまで成功し、`wrangler deploy` がカスタムドメインの更新(ゾーンを引けない、code 10084)で失敗した。依頼者の API トークンは Workers Scripts: Edit だけで、権限の追加は求めない。修正(`fix-versions.diff`)を採る: トリガーに触れない `wrangler versions upload` + `versions deploy`、順序は『bundle の中身 → Worker の版 → 既定 manifest(最後)→ Access 到達確認』。本番(カスタムドメインつき)への `versions upload/deploy` は、この修正の push が最初の実地の検証になる。
