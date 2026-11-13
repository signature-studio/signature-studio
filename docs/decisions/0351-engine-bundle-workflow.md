# 0351: 配信用ワークフローの設計。bundle は手元から取らず、9 段を GitHub の標準ランナーで回す
分類: 整備(lift の処理ではないので (a)(b)(c) の対象外)
撤回条件: 冷たいビルドが 6 時間(ジョブの上限)に当たる、または 5653 インストーラの取得・照合が 2 回続けて失敗する、または 9 段のどれかが GitHub 上で再現せず直せない。そのときは、手元の bundle を Actions に渡す経路(依頼)に切り替える。
日付: 2026-11-13   ROADMAP: R8-3   一覧: なし(整備)   コミット: 36de581..(本文書の commit)
番号: 0346〜0350 は ROADMAP の R1-4・R2-5・R2-8・R3-1・R7-2 が予約(`ls docs/decisions` は README のみ。旧決定は 0001〜0345)。0351 が最初の空き。

## 決定
決定: (B) 9 段を GitHub の標準ランナーで回す。根拠: (A) の手元の bundle(272,254,720 B)は git にも GitHub にも無く、Actions へ渡すには R2 への書き込みが要る。その書き込みは wrangler か S3 キーが要り、どちらも私は使わない(依頼になる)。(B) の入力は FL インストーラ 1 本(1,106,033,456 B)で、R2 の `fl-web-artifacts` から読める。

手順(`.github/workflows/engine-bundle.yml`。R8-4 で書く):
1. `on: push`(`branches: [main]`)のみ。`permissions: contents: read`。`workflow_dispatch`・入力・URL パラメータ・環境変数・`#ifdef`・ビルド変数は足さない。
2. アカウント ID: Secrets `CLOUDFLARE_API_TOKEN` で `/accounts` を読み(`cloudflare-check.yml` の段 1 と同じ)、得た直後に `::add-mask::`。段 3 の R2 の endpoint に使う。Variable `CLOUDFLARE_ACCOUNT_ID` は読まない(値が公開ログに出る。0001 の注記)。
3. 入力: Secrets `R2_ARTIFACT_ACCESS_KEY_ID` / `R2_ARTIFACT_SECRET_ACCESS_KEY` で `installers/flstudio_win64_26.1.7.5653.exe` を取り(S3 API。バケット `fl-web-artifacts`)、`ci/artifacts.lock` の大きさと SHA-256 を照合する。不一致なら止まる。
4. 抽出: `python3 ci/lib/extract_fl.py <インストーラ> <HOST_ROOT> --dlls ci/extract-dlls.tsv --seed lift/harness/seeds/vfs-seed-idle1.tsv`(DLL 8 本と seed。不一致は exit 2)。
5. 道具: `ci/setup-toolchain.sh emsdk` と `rust`(`ci/versions.env`: emcc 6.0.12、rust 1.99.0)、Python 3.14、Node 24。上流 Blend2D は `lift/harness/b2d1/build_upstream.sh`(固定コミットと tarball の SHA-256 で照合)。
6. 9 段: `WORK=$RUNNER_TEMP/work HOST_ROOT=<段 4 の出力> B2D_UPSTREAM_LIB=… B2D_UPSTREAM_SRC=… bash app/scripts/build-engine-bundle.sh`(段の既定の順のまま)。
7. 成果物: GitHub の artifact に上げない(`upload-artifact` を使わない)。ログには段の rc・時間・ファイルの SHA-256 だけを出す。手元の E13 bundle は 9 ファイル。GitHub の実行は 11 ファイル(+`SHA256SUMS`・`upload-commands.txt`。実行 43434252094 の段 7、出典は `evidence/R8-4.md`。`app/scripts/pack-assets.mjs` の 127・143 行が 2 つを書く)。旧: 9 ファイル
8. 成功の判定: `gh run list --workflow engine-bundle.yml --limit 3` が `success`、かつ段 6 の 9 段の rc が 0。

決定: `paths` の絞り込みは R8-4 で足さない。根拠: 指示は `on: push` のみで、絞り込みは条件を足すことになる(worker-rules の範囲)。

境界: R8-4 は bundle を作って sha を出すまで。R2 への書き込み(R6-3)、Worker の版(R6-3・R6-4)、既定の入口の差し替え(R6-4 の条件を満たすまで不可)は含めない。キャッシュは入れない(冷たい 1 回を先に成功させる。6 時間の上限の内)。Secrets は `CLOUDFLARE_API_TOKEN`(段 2 のみ)と `R2_ARTIFACT_*`(段 3 のみ)。`R2_CACHE_ASSETS_*` は R6-3 で使う。値は書かない。

## 実測(日付・n・取り方)
| 項目 | 値 | 取り方 |
|---|---|---|
| bundle 9 ファイルの合計 | 272,254,720 B | `du -cb artifacts/bundles/fl-engine-ci-0363b050/*`(2026-11-13、n=9) |
| うち wasm | 143,087,706 B | `ls -la`(同) |
| FL インストーラ 5653 | 1,106,033,456 B、SHA-256 の頭 `04393e79` | `stat`。`ci/artifacts.lock` の行と一致 |
| `FLEngine_x64.dll` | 22,227,016 B | `stat binaries/win64/FLEngine_x64.dll` |
| 冷たいビルド(標準ランナー) | build ジョブ 37 分 33 秒・45 分 39 秒(n=2) | 0217 §4c の表、実行 5 と 2(Actions API の step 時刻) |
| 温かい実行(全部 hit) | 約 1 分 20 秒(n=2)。旧: n=1、実行 3 | 0217 §4c の実行 3 と 8(全 hit) |
| 同じ木の最終 wasm | GitHub と手元が SHA-256 まで一致 | 0217 §4c(`cf384a1c…`、130,237,776 B) |
| 別のインストーラ 3 本の取得と抽出(VST3 用。このビルドの入力ではない) | 1 分 13 秒、3 本・1.36 GB | 0217 §4c の実行 1(GitHub 上) |
| ジョブの上限 | 6 時間 | https://docs.github.com/en/actions/reference/limits(2026-11-13 に読んだ) |
| 公開リポジトリの標準ランナー | ubuntu-latest: 4 vCPU・16 GB・14 GB(SSD) | https://docs.github.com/en/actions/reference/runners/github-hosted-runners(2026-11-13 に読んだ。public の表の Linux x64 `ubuntu-latest` の行が 4 / 16 GB / 14 GB。旧の出典は 0217 §4b の 14 GB で、これは private の表の値(0217 の 74 行目)。public の SSD の値は、この表で確かめた) |
| 旧 CI の 17〜22 分 | [文書](16 コア・JOBS=4 の値) | `docs/core/pipeline.md` 27 行。GitHub の実測 37〜46 分とは環境が違う |

## 読み(解釈。実行していない)
- 0217 の 37〜46 分は private の 2 vCPU・8 GB で測った値。公開の今は 4 vCPU・16 GB と読む(文書)。速くなるかは未確認。
- 公開リポジトリのため、artifact・ログに bundle や DLL の中身を出さない。ログに出すのは sha256 と時間だけ。
- `pack-assets.mjs` が出す `upload-commands.txt` は `wrangler r2 object put` の形。R6-3 はこれを使わず、S3 API の形を別に作る(0217: バケット限定の R2 トークンは wrangler の REST 経路で 403)。
- 入力は 5653 の 1 本で足りる想定。ただし `ci/extract-dlls.tsv` の 8 行がすべて 5653 から取れることは未確認(0217 の記録は DLL 6 本の抽出。残り 2 本は後述の未確認)。Serum・OTT・Vital はこのビルドの 9 段に入っていない(`app/scripts/build-engine-bundle.sh` の入力に無い)。
- E13 の生成 C・wasm との一致は要求しない(R0-5 の範囲。違いは事実として記録する)。

## 未確認
- 5653 が `fl-web-artifacts` に今も在るか(R8-2 は `aws s3 ls` の成功のみ。内容は記録していない)。R8-4 の初回で確かめる。
- `oggio_x64.dll`・`elastique_x64.dll` が 5653 に入っているか(0217 の抽出は DLL 6 本と記録。この機械では未実行)。
- 手元の bundle の元の木(manifest の `HEAD d0447902`)は、このクローンに無い(`git cat-file -t d0447902` が失敗)。
- 冷たいビルドの時間(4 vCPU の値)。
- 0217 の 4 本の合計 2,469,325,400 B と、今日の和 2,469,324,400 B(`stat` の和)の 1,000 B の差。原因は未調査。
- 公開リポジトリの artifact の閲覧範囲(文書は読んでいない。手順 7 は保守的な方針)。

## 出典
docs/decisions/README.md、docs/ROADMAP.md(R6-3・R6-4・R8-3・R8-4、0346〜0350 の予約)、app/scripts/build-engine-bundle.sh(冒頭と 9 段)、ci/artifacts.lock、ci/extract-dlls.tsv、ci/versions.env、ci/lib/extract_fl.py、docs/archive/decisions/0217-ci-deploy.md(§4c・§5)、docs/requests/0001-cloudflare-actions-secrets.md、evidence/R8-2.md、evidence/R0-1.md、docs/core/inputs.md、docs/core/pipeline.md。

## 追記(R8-4a、2026-11-13)
- 決定: `paths` を足す(本文の「足さない」を改める)。根拠: 証跡の commit ごとに 40 分前後のビルドが走るのを防ぐ。起動条件の絞り込みだけで、入力・旗・分岐は増えない。分類: 整備(0351 の手順。lift の処理ではない)。
- 一覧と根拠: `app/scripts/build-engine-bundle.sh`(本体)、`app/scripts/pack-assets.mjs`→`app/src/bundle/{pack,manifest,schema,sha256}.ts`、`app/scripts/lib/wasm-exports.mjs`、`app/package.json`(`.ts` を Node が読む際の `type`。**未確認**: 外しても動くかは未実行)、`lift/.cargo`・`lift/Cargo.{toml,lock}`・`lift/crates`(cargo)、`lift/runtime`・`lift/data`(prune-SM.tsv)、`lift/harness`(gen-direct.sh・two・hyb・web・seeds)、`ci/artifacts.lock`・`ci/extract-dlls.tsv`・`ci/versions.env`(`ci/lib/common.sh` が読む)・`ci/setup-toolchain.sh`・`ci/lib`、`tools/extract_nsis2.py`・`tools/nsis_script_dump.py`(`lift/harness/seeds/gen_install_tree.py` が読む)。
- 上流 Blend2D は本文に無い patch(`lift/harness/b2d1/b2d3-compop.patch`)を当てる。根拠: 出荷の入力(`docs/core/inputs.md` §4、`ci/lib/common.sh` の `B2D_PATCH`)と揃えるため。
