---
draft: "160"
title: "demo 2 件の素材を、起動の後の取得で配信し、ゲストが見つけられない素材の内訳(インストーラに在る物が 0 件)を確かめる"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["129"]
labels: ["resources", "audio"]
size: M
priority: High
project: signature-studio/3
---

## 目的

段階 1(#1373)で、DEDZII - NIGHT TERROR と Mehran Abbasi - With You が要求する素材のうち、インストーラに在る物(`inventory`。DLL を除く)の完全なパスが、`pm-work/findings/demo-assets-inventory.tsv` に書かれ、`ci/resources-prefixes.txt` の行として、次の run の取り出しに入った。この Issue は、その素材を起動の後の取得(lazy)で、ゲストに配信する。

目標:
1. DEDZII と Mehran の両方で、`missingInInventory`(絶対パスの要求のうち、インストーラに在り、見つからなかった物。DLL を除く)が 0。`%変数%\…` の生の綴りの要求は、FL 自身の探索なので数えない(`rawVariableProbes` に出す。下の「測定の定義」)。
2. 見つからなかった残りの内訳を出す: `notInInstaller` の件数と先頭 10 件(JSON と報告に出す)。
3. 流れ A〜D が通り、`elapsedMs` と `preBootBytes` が基準を超えない。DEDZII と Mehran が開くことは、この Issue の目標に入れない。開けない理由は、別の Issue で扱う(#1420 が Mehran の Elastique の停止。DEDZII の File > Open の停止は #1421(Urgent)。下の「前回の試行で分かったこと」)。

## 背景

- 段階 1 の出力: `pm-work/findings/demo-assets-inventory.tsv`(列: `demo`・`path`・`bytes`・`sha256`。`path` は一覧の guest の綴りで、インストール先からの相対)。各 `path` は、`ci/resources-prefixes.txt` の完全なパスの行として在る。実物は、次の run の取り出しで `lift/out-app/host-extra/resources/` に、相対パスと同じ木の形で在る。
- 前回の run の実測(段階 1 の前): DEDZII の一覧に在る素材は 27 件・合計 4,830,812 B。Dance の 12 件と Misc の 9 件は、seed 表の `@lazy-file:` の行で配信済み(段階 1 の差分に含まれる)。残りの 27 件が、この Issue の対象。
- 配信の仕組み: 起動の後の取得(`@lazy-file:`)。seed 表(`lift/harness/seeds/vfs-seed-idle1.tsv`)の行と、`lift/harness/web/pack.args` の `--lazy-file` の組が要る。ゲストの仮想のファイルシステムに載るのは、この 2 つの行が在る物だけ(`pm-work/workflow-design.md` の「接頭辞で取った物は」)。
- lazy の上限は `app/src/bundle/manifest.ts` の `maxItems`(128)。段階 1 の時点の件数に 27 を足すと、上限を超える可能性がある。実測で数え、超えるなら上げる(既定の動作として。オプトインを作らない)。
- `app/scripts/pack-assets.mjs` は、URL の `#`・`%`・`?` を `%XX` にする(段階 1 で入った。`#` より後ろが fragment として切られて読めなかった Mehran の `.ogg` の修正)。新しい素材のファイル名に `#`・`%`・`?` が含まれるなら、この修正で読めることを確かめる。

## 前回の試行で分かったこと(2 回目の試行)

run 45518902053(00:35〜02:11 UTC。`failed`)の結果。数値は runner の値。

- 配信: seed 表に `@lazy-file:` の行を 27 行、`pack.args` に `--lazy-file` の組を 27 組(54 行)足した。配信は 27 件・4,830,812 B。lazy の件数は manifest で 123(`maxItems` 128 の内。上限は変えていない)。
- 読まれた 8 件(196,280 B)は、絶対パスの綴りで要求され、以後は `missing-path` に出ない。
- 未解決の 19 件(DEDZII の `inventory`。4,634,532 B 分は配信済みだが読まれない)。19 件の要求は、どれも `%FLStudioFactoryData%\Data\Patches\…` の綴り(変数の根)で、`GetFileAttributesW` が「パスが無い」(3)を返す。内訳は Impulses 2 件と Packs 17 件。seed 表と `pack.args` は絶対パスの綴りだけを持つので、一致しない。
- ゲストは `ExpandEnvironmentStringsW` を呼ばず、環境は起動の時に空。VFS は `%…%` を展開しない。Blocks.flp(テンプレート)の同じ測定は、`inventory` 0・変数の要求 0。
- Mehran の `inventory` は 0。DEDZII と Mehran の両方で「Problems loading the project」(TMissingPluginsForm。プラグインの欠落)の窓が出る。この Issue の受け入れの外。
- 流れ A〜D は通った(`elapsedMs` 9,083〜9,230、`preBootBytes` 311,137,416)。`npm test --prefix app`: 1,404 passed・10 skipped。
- `notInInstallerFirst10`(両方共通): 空の綴り、Dance の `Basic 808` 系の `.ZGR` 4 件、`Presets\Plugin database\Installed\.Plugins.ini`、`FLEX.NFO`、`*.*`、`Emphasizer.NFO`、`Windows\Fonts\OpenSans-Regular.ttf`。
- 決定(PM。下の 3 回目の節で改めた): 変数の根は、ゲストの環境ブロックで展開する(#1412)。この Issue は、#1412 が閉じた後に起動する(blocked by)。19 件の lazy 行は外さない。展開された綴りは、インストール先の絶対パスの行と一致する見込み。一致しなければ、その綴りを報告に書く。

## 前回の試行で分かったこと(3 回目の試行。#1412 の run 45541813190 の結果)

数値は runner の値(nproc 4)。#1412 の試行の詳細は、#1412 の「前回の試行で分かったこと」の節。

- #1412 の試行で、FL は起動の時に自分の変数(`FLStudioFactoryData` など)を `SetEnvironmentVariableW` で設定し、`ExpandEnvironmentStringsW` が展開できることが分かった。展開後の綴りは、`C:\Program Files\Image-Line\FL Studio 21\Data\Patches\…`(推定。長さと展開の結果から)。
- 生の `%FLStudioFactoryData%\Data\Patches\…` の 27 件は、`GetFileAttributesW` に「パスが無い」で届く。Windows の `GetFileAttributesW` も `%…%` を展開しないので、Windows でも同じ結果になる。FL 自身の探索の一部で、展開の欠けではない。この要求は数えない(`rawVariableProbes`)。
- 展開後の絶対パスの 27 件(`C:\Program Files\Image-Line\FL Studio 21\Data\Patches\…`)は、「パスが無い」で届く。27 件は、インストーラの一覧(`inputs/fl-installer-files.tsv`)に在り、展開後の木(`lift/out-app/host-extra/resources/`)に実物がある。
- 27 件の行は、`lift/harness/seeds/vfs-seed-idle1.tsv` に無い(`Minimal Kick 36`・`Gill SFX 2`・`Smigen Boom SFX`・`HouseGen Clap 02` を grep して 0 件)。main には、この Issue の lazy の行と `--lazy-file` の組が入っていない。
- したがって、この Issue の作業は、27 件の絶対パスの行を seed 表と `pack.args` に足し、配信することで、`missingInInventory` を 0 にする。前回の差分(run 45518902053)に同じ行が在れば、それを使う。
- 配信が効けば、絶対パスの要求は成功し、`missingInInventory` は 0 になる見込み。
- #1412 の試行の流れ A〜D は通った(`elapsedMs` 11514〜11901、`preBootBytes` 311145137)。boot は engine-idle・standIn が false・error が null(`elapsedMs` 12177)。

## 前回の試行で分かったこと(4 回目。run 45566707815)

数値は runner の値(nproc 4。loadavg 0.26・0.48・0.70)。前回の差分(`lift/harness/seeds/vfs-seed-idle1.tsv`・`lift/harness/web/pack.args`・`tools/p3-audio-probe/measure.mjs`)は、`.prior-work.md` の取り込みで入る。

- 配信は動いた。seed 表に `@lazy-file:` の行を 27 行、`pack.args` に `--lazy-file` の組を 27 組足した。配信は 27 件・合計 4,830,812 B。manifest の lazy は 126 件(`maxItems` 128 の内。上限は変えていない)。一覧の 27 件は、大きさと sha256 が一覧の値と一致した。
- 起動: `boot-bundle` で engine-idle・standIn false・error null・elapsedMs 10,223。
- DEDZII(停止時点): `missingInInventory` 1。その 1 件は `Plugins\Fruity\Generators\Sytrus\Data\Default.dat`(絶対パスの `GetFileAttributesW`。インストーラの一覧に在り、1,832 B・sha256 `adc5842c…`)。この実物は、作業ツリーに無かった(`ci/resources-prefixes.txt` の取り出しは別ジョブ)。その後、#1414 で Sytrus の Data が lazy で配られるようになった(この再開の run で届く見込み)。
- DEDZII の `rawVariableProbes` 0・`dll` 21・`notInInstaller` 45・`missingDistinct` 67・`inventory` 1。`notInInstallerFirst10` は、空の綴り・Dance の `Basic 808 *.ZGR` 4 件・`Presets\Plugin database\Installed\.Plugins.ini`・`FLEX.NFO`・`*.*`・`Emphasizer.NFO`・`C:\Windows\Fonts\OpenSans-Regular.ttf`。
- DEDZII は、開く途中で `fle: CreateFileW: empty path is not defined` の行を出して止まる(exit 134。タイトルは「Song info」)。27 行を外した対照でも同じ停止。素材の配信とは無関係。PM の E2E の確認では、File > Open で約 5 秒後に終了コード 134 で止まった。これは #1421(Urgent)で扱う。
- Mehran(停止時点): `missingInInventory` 0・`dll` 15・`missingDistinct` 33・`notInInstaller` 18・`inventory` 0。`fle: indirect call to 0xb2844c0 is not a known function entry` で止まる(exit 134。Elastique の像の内側)。対照でも同じ。この停止は、#1420 で扱う。
- 停止の前に、27 件の `Patches` が要求されたかは、ログでは分からない(読み出しを残さない)。missing-path に出ていないことだけ確認した。
- 流れ A〜D(最終の bundle で単独に実行): 4 行とも error null・engine-idle・standIn false・trapped false・hostErrors 0・opened true。`elapsedMs` 8,780・8,832・8,928・8,838。`preBootBytes` 312,199,811。`npm test --prefix app`: 1,410 件が通る(71 files passed・1 skipped)。
- 残りの課題: (1) DEDZII の `Default.dat` は、#1414 の配信で届く見込み。届いたら `missingInInventory` が 0 になる。(2) Sytrus の Data の 109 件を足すと、lazy の件数が上限(128)を超えうる。#1414 が件数をどう扱ったかを、この run で確かめる。超えるなら `maxItems` を上げる(既定の動作として)。(3) DEDZII の File > Open の停止と、Mehran の Elastique の停止は、この Issue の受け入れに入れない(上の目標 3)。

## 読む物

- `pm-work/findings/demo-assets-inventory.tsv`(段階 1 の出力)
- `ci/resources-prefixes.txt`(段階 1 で足した完全なパスの行)
- `lift/harness/seeds/vfs-seed-idle1.tsv`(`@lazy-file:` の行の形。`Data\Patches\Packs\Legacy\Drums\Dance\DNC_*.wav` の行が手本)
- `lift/harness/web/pack.args`(`--lazy-file` の組の形)
- `app/src/bundle/manifest.ts`(`maxItems`)、`app/scripts/pack-assets.mjs`
- `tools/p3-audio-probe/measure.mjs`(段階 1 で入った内訳)、`tools/smoke/welcome-flow.mjs`
- `pm-work/workflow-design.md`(「接頭辞で取った物は」の段落)

## 測定の定義(3 回目の試行で改めた)

- `tools/p3-audio-probe/measure.mjs` の JSON に次を出す(tools/ の変更は許す)。`inventory`・`dll`・`notInInstaller`・`notInInstallerFirst10`・`rootStripped` は従来どおり出す。
- `missingInInventory`(数): `missing-path` の行のうち、(a) 綴りが絶対パス(`C:\` などで始まる)で、API が `GetFileAttributesW`・`CreateFileW`・`FindFirstFileW` の物。接頭辞を外した綴りが、インストーラの一覧に在り、`.dll` ではない物の件数。同じ綴り(小文字)は 1 件。
- `rawVariableProbes`(`{count, first10}`): `%変数%\…` の生の綴りの要求。`count` は件数(同じ綴りは 1 件)、`first10` は先頭 10 件(綴りのまま)。数えるだけで、判定には使わない。

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。
1. 段階 1 の出力を確かめる: `demo-assets-inventory.tsv` の各行の `path` が `ci/resources-prefixes.txt` に在り、取り出された実物(`lift/out-app/host-extra/resources/` の下)の大きさと sha256 が `bytes`・`sha256` と一致する。一致しない行が在れば、報告に書いて止める(一覧の値が正)。
2. 各行を配信する。(a) seed 表に `@lazy-file:` の行を足す。1 列目は guest(インストール先の絶対パスの下の綴り。前の行と同じ書き方で、接頭辞は同じ seed 表の既存の行からコピーする)、2 列目は `@lazy-file:resources/<path の \ を / にした物>`、3 列目は sha256(一覧の値)。行の形は `DNC_*.wav` の行と同じ。(b) `lift/harness/web/pack.args` に、同じ数の `--lazy-file` の組を足す。組の形は既存の行と同じ(`lift/out-app/host/resources/<path>=seed/resources/<path>`)。`lift/crates/fe-build/tests/plan/bundle.txt` は main の版では生成の形(`@lines`)なので、組は `pack.args` にだけ足す。空の代替(空ファイル・0 埋め)は置かない。
3. lazy の件数を数える。`app/src/bundle/manifest.ts` の `maxItems`(128)を超えるなら、上限を上げる。件数と、上げたかどうかを報告に書く。
4. 測定の出力に `missingInInventory` と `rawVariableProbes` を出す(上の「測定の定義」)。配信の後の `missingInInventory` が 0 になることを、受け入れの出力で確かめる。0 にならない素材が在れば、その綴りと API と理由(seed 行の綴りの違い・大文字小文字・`#`/`%`/`?` の扱い)を報告に書く。
5. 受け入れを通す。通らなければ、作業ツリーを commit して終える。
6. commit する。push はしない(ジョブの最後の判定が通ったときだけ push される)。`app/` を変えたら、push すると `deploy.yml` が動くので、報告に書く。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-1403 && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/DEDZII - NIGHT TERROR.flp" .tmp-1403/dedzii.flp && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/Mehran Abbasi - With You.flp" .tmp-1403/mehran.flp && for f in dedzii mehran; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-1403/$f.flp" > "$RUNNER_TEMP/open-$f.json"; done && node -e 'const fs=require("fs");const L=process.argv.slice(1).map((f)=>JSON.parse(fs.readFileSync(f,"utf8").trim()));const ok=(r)=>r.missingInInventory===0&&typeof r.rawVariableProbes==="object"&&r.rawVariableProbes!==null&&typeof r.rawVariableProbes.count==="number"&&typeof r.notInInstaller==="number"&&typeof r.dll==="number"&&Array.isArray(r.notInInstallerFirst10)&&r.notInInstallerFirst10.length<=10;process.exit(L.length===2&&L.every(ok)?0:1)' "$RUNNER_TEMP/open-dedzii.json" "$RUNNER_TEMP/open-mehran.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- 起動 1 回で、engine-idle・standIn が false・error が null(boot)。
- DEDZII と Mehran の `measure.mjs` の行で、`missingInInventory` が 0(数値)。`error`・`state` は判定に入れない(開けなかった理由は、JSON と報告に出す。上の目標 3)。DEDZII の `missingInInventory` が 0 にならない時は、残った要求の一覧(綴り・API)を報告に書く。`rawVariableProbes` は `count` が数値の object。`notInInstaller` と `dll` が数値、`notInInstallerFirst10` が配列で 10 件以下。`inventory` は JSON に出すが、判定には使わない。
- 流れ A〜D(`welcome-flow.mjs`)の 4 行: 各行で `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。各行の `elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。
- 単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 配信した件数と合計の大きさ、追加した seed 行と `pack.args` の組の数、lazy の件数と `maxItems` を変えたか、DEDZII と Mehran の `missingInInventory`・`rawVariableProbes`(件数と先頭 10 件)・`inventory`(参考)・`dll`・`notInInstaller`・`notInInstallerFirst10`、流れ A〜D の 4 行、`elapsedMs`・`preBootBytes`、`app/` を変えたこと(push すると `deploy.yml` が動く)。

## 制約

- 禁止されているのは、AI のベンダー名と、Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。遅延は既定の動作にする(オプトインを作らない)。
- 空の代替(空ファイル・0 埋め)を置かない。大きさと SHA-256 は、一覧(インストーラの実物から取った値)と照合したものだけを置く。
- インストーラに無い素材を、どこからも取らない(配信の対象外)。
- 名前の規則(訂正): 禁止されているのは、AI のベンダー名(`guard/scan.sh` が検査する)と、Cloudflare の識別子だけ。FL の製品名・`Image-Line` のインストール先・`%FLStudioFactoryData%` などの変数名は、ファイル・Issue の本文に書いてよい(`lift/crates/fe-build/src/plugins.rs` などが既に書いている)。
- Python は製品に使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- エンジンのビルドは 1 回約 32〜46 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。`sleep`・`timeout` をコマンドの先頭に書かない。

## 他のタスクとの関係

- blocked_by: #1373(段階 1。素材の収集と、`ci/resources-prefixes.txt` の行)。閉じている。
- #1412(ゲストの環境変数)とは独立。前回は blocked by にしていたが、外した。生の綴りは FL 自身の探索で、環境変数の展開では解けない(上の 3 回目の節)。
- #1414(Sytrus の Data の配信)は閉じている。その配信が、DEDZII の `Default.dat` を届ける(前回の試行の節)。
- #1420(Mehran の Elastique の間接呼び出し)と、#1421(DEDZII の File > Open の停止)は、この Issue の受け入れに入れない。
- 後続: #1401(音が出る demo を決める)。#1401 は、この Issue の後に行う。#1335(P3 の測定)は #1401 を待つ。
- #1387(Riff machine)・#1400(Sytrus)・#1395(言語)も、seed 表と `pack.args` を変える。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
