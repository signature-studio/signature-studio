---
draft: "169"
title: "ゲストの環境変数を、Windows と同じ振る舞いにする: FL が `%FLStudioFactoryData%` などを展開できるようにする"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["resources", "audio"]
size: M
priority: High
project: signature-studio/3
---

## 目的

FL は、素材(サンプル)のパスを `%FLStudioFactoryData%\Data\Patches\…` の形で持ち、Windows では環境変数を展開して開く。この VM では環境変数が展開されず、DEDZII - NIGHT TERROR の素材の欠けの原因になっていた。前回の試行で、FL は起動の時に自分の変数(`FLStudioFactoryData`・`FLStudioUserData` など)を `SetEnvironmentVariableW` で設定し、`ExpandEnvironmentStringsW` がそれを展開できることが分かった(「前回の試行で分かったこと」)。この Issue は、ゲストの環境ブロックを Windows と同じ振る舞いにし、FL が設定する変数と Windows の標準の変数を、同じブロックで Set・Get・Expand できるようにする。

素材の欠けそのもの(生の `%…%` の綴りと、絶対パスの配信)は、この Issue の受け入れに入れない。配信は #1403 の範囲。

目標:
1. ゲストの環境ブロックが 1 つで、`SetEnvironmentVariableW`・`GetEnvironmentVariableW`・`ExpandEnvironmentStringsW`・`GetEnvironmentStrings`(A と W)、と、UCRT の環境変数の関数が、同じブロックを読み、書く。
2. 起動の時に、Windows のセッションの標準の変数が、ブロックに在る。FL が設定する変数(`FLStudioFactoryData`・`FLStudioUserData`・`ILSharedData` など)は、FL の `SetEnvironmentVariableW` で入り、`GetEnvironmentVariableW`・`ExpandEnvironmentStringsW` で読める。
3. DEDZII を File > Open で開くと、測定の出力の `envLog` に、`FLStudioFactoryData`・`FLStudioUserData`・`ILSharedData` の `SetEnvironmentVariableW` の行と、`ExpandEnvironmentStringsW` の行が出る。

## 背景

- この節の事実 1〜5 は、この Issue を書いた時点のもの。前回の試行の結果は、「前回の試行で分かったこと」の節。
- 事実 1(#1403 の 2 回目の試行。run 45518902053): DEDZII の `measure.mjs` で `inventory` が 19。19 件の要求は、すべて `%FLStudioFactoryData%\Data\Patches\…` の綴りで、`GetFileAttributesW` が「パスが無い」(3)を返す。Mehran は 0。Blocks.flp(テンプレート)の測定では、変数の要求は 0 だった。
- 事実 2(同じ試行のエンジンの記録): ゲストは `ExpandEnvironmentStringsW` を呼んでいない。VFS は `%…%` を展開しない。`lift/runtime/fle_env.c` の `ExpandEnvironmentStringsW` は、見つからない名前を書かれたまま残す(MSDN の振る舞い)。
- 事実 3(コード): 環境ブロックは `lift/runtime/fle_env.c` の静的な配列(`env_v`・`env_n`)で、起動の時の初期化は無い(空で始まる)。実装は、`GetEnvironmentVariableW`(125 行付近)・`SetEnvironmentVariableW`(153 行付近)・`ExpandEnvironmentStringsW`(202 行付近)・`GetEnvironmentStringsW`(314 行付近)・`FreeEnvironmentStringsW`(347 行付近)・`GetEnvironmentStrings`(403 行付近)・`FreeEnvironmentStringsA`(440 行付近)。`GetEnvironmentVariableA`・`SetEnvironmentVariableA`・`ExpandEnvironmentStringsA` は `lift/runtime` に無い。
- 事実 4(インストーラ): `lift/harness/seeds/reg-seed-nsis.tsv` の注記では、インストーラのスクリプトが書くレジストリの値は `Software\Image-Line\Shared\Paths` の下だけで、環境変数の鍵は無い。FL の変数がどこから来るかは、実測で決める。
- 事実 5(仮想の値): `lift/runtime/fle_env.h` の `FLE_VIRTUAL_*`(Windows のディレクトリ `C:\Windows`・`C:\Windows\System32`、ユーザーのプロファイル `C:\Users\user`、ユーザー名 `user`、コンピュータ名 `DESKTOP-0000001`、DLL と EXE のパス)。`SHGetFolderPathW`(`fle_env.c` の 500 行付近)の答えは、これらの値から作られている。初期のブロックの値は、これと一致させる。
- 関係: #1386 の作業は、UCRT の環境変数の関数(`_wgetenv`・`_putenv`・`_wputenv_s`・`getenv`・`_dupenv_s`)を `lift/runtime/fle_crt.c` に足す。この Issue を書いた時点で、それは main に無い。UCRT の関数も、この Issue と同じブロックを読む形にする。#1386 が main に入る前は、Win32 の API だけを直し、UCRT の関数の扱いを報告に書く。

## 前回の試行で分かったこと(run 45566655916。failed)

数値は runner の値(nproc 4、loadavg 1.05 1.17 1.76)。

- 判定は差し戻し。DEDZII(File > Open)の段で止まった。`error` は `timeout 180000 ms: the title of dedzii (the title is "Song info")`、`end.state` は exited。受け入れの他の段は通った(下の行)。作業は work.diff に保存された。次の試行は、`.prior-work.md` の取り込みで差分を載せてから続ける。
- 止まり方: 選んだファイルの後に `[err] filedlg: THE SELECTION WAS MADE BY THE PERSON AT THE PAGE (1 item)`、続けて `[err] fle: CreateFileW: empty path is not defined`。最初の呼び出しは `GetFileAttributesW 3 ""`。`fle_filedlg.c` は、選んだパス(`C:\Users\user\Downloads\dedzii.flp`)を空にしていない。空のパスは FL の後続の処理で作られる。どこで作られるかは未確認。
- HEAD の版(環境の変更なし)でも、同じ DEDZII が同じ位置で止まる。元のファイル名でも同じ。環境の変更が原因ではない。PM の決定で、開く段の判定を受け入れから外した(別の Issue #1421 が扱う)。
- 観測(起動の時の記録。停止までの全ログ):
  ```
  env: SetEnvironmentVariableW "FLStudioUserData" set 44
  env: SetEnvironmentVariableW "FLStudioFactoryData" set 40
  env: SetEnvironmentVariableW "ILSharedData" set 35
  env: ExpandEnvironmentStringsW "%FLStudioUserData%\Presets\Plugin database\" expanded 69
  env: ExpandEnvironmentStringsW "%FLStudioFactoryData%\Data\Demo projects\" expanded 60
  ```
- ほかの Set は `FLStudioDownloadsData`(44)・`FLStudioUserCommonConfig`(53)・`FLStudioData`(45)など。`GetEnvironmentStringsW` は 3 回(ブロック 645 バイト)。`GetEnvironmentVariableW` は 6 回で、すべて absent(`LANG` ×4・`CLAP_PATH`・`OneDrive`)。`ExpandEnvironmentStringsW` の記録は上限の 200 に達した。
- 初期のブロック: `env_std` の 22 件(`SystemRoot`・`SystemDrive`・`windir`・`USERNAME`・`USERPROFILE`・`APPDATA`・`LOCALAPPDATA`・`ProgramData`・`ProgramFiles`・`ProgramFiles(x86)`・`CommonProgramFiles`・`TEMP`・`TMP`・`COMPUTERNAME`・`OS`・`PATH`・`PATHEXT`・`NUMBER_OF_PROCESSORS`・`PROCESSOR_ARCHITECTURE`・`HOMEDRIVE`・`HOMEPATH`・`USERDOMAIN`)。値は `FLE_VIRTUAL_*` と `SHGetFolderPathW` の答えから作る。`OS`(`Windows_NT`)と `PATHEXT` だけは直書き。
- 試験: `env.rs` は 13 件が通る。新しい 5 件(標準の変数・大文字小文字・一つのブロック・観測の 200 件・SHGetFolderPath との一致)は、HEAD の版で赤になることを確かめた。`k32_dsp.rs`・`wall_b.rs` は、「空のブロック」の前提を標準の並びに直した。両方は `fle_clip_host_read/write` の未定義(#1389)でリンクできないため、リンク条件を一時に緩めて 14 件・13 件が通るのを確かめ、元に戻した。`wall_r2.rs`・`wall_fr3.rs` の失敗は、この変更の外(`fle_clip` のリンク・`archive/harness-attic/a4-multi.sh` の無さ)。
- 受け入れの他の段: boot は engine-idle・standIn false・error null(`elapsedMs` 12202)。流れ A〜D は 4 行とも条件を満たす(`elapsedMs` 11529・11642・11460・11466、`preBootBytes` 312195974)。`npm test --prefix app` は 1410 件が通り、10 件はスキップ。
- `GetTempPathW` は TMP(`C:\Users\user\AppData\Local\Temp\`)を返すようになった。VFS にそのフォルダは無い。`GetTempFileNameW` は一時ファイルを作らない。作成の失敗の仕方は従来と同じと見られる(エンジン上では未確認)。
- `measure.mjs` の `envLog` は、`projectReport`(開く段の成功の後)でしか作られない。前回は開く段で止まったため、`envLog` は null だった。直しは「やること」4。
- 前回の差分は、`tools/p3-audio-probe/measure.mjs` で main の `imageReads`(#1400)と衝突した。両方を残す形で解いてある(差分の中)。
- UCRT の環境関数(`_wgetenv`・`_putenv`)は、main に `fle_crt.c` が無いため未実装。#1386 のマージ後に、同じ `env_find`・`env_store` を使う。

## 2 回前の試行で分かったこと(run 45541813190。03:42〜07:03 UTC。`failed`)

数値は runner の値(nproc 4)。

- 観測: FL は起動の時に、次の変数を `SetEnvironmentVariableW` で設定する(ログは長さだけを書く)。`FLStudioUserData`(44)・`FLStudioFactoryData`(40)・`FLStudioDownloadsData`(44)・`FLStudioUserCommonConfig`(53)・`FLStudioFactoryCommonConfig`(40)・`FLStudioDownloadsCommonConfig`(44)・`FLStudioUserConfig`(56)・`FLStudioFactoryConfig`(40)・`FLStudioData`(45)・`ILSharedData`(35)。Get だけで読む FL の変数は、観測の範囲に無い。
- 値(長さと展開の結果から推定。末尾の `\` は無い): `FLStudioFactoryData` = `C:\Program Files\Image-Line\FL Studio 21`、`FLStudioUserData` = `C:\Users\user\Documents\Image-Line\FL Studio`。
- 展開: `ExpandEnvironmentStringsW` は、`%FLStudioUserData%\…` と `%FLStudioFactoryData%\Data\Demo projects\` を展開する。観測の上限(API ごとに 20 件)が起動の呼び出しで尽きたため、サンプルの展開は記録に残らなかった。上限は 200 に上げる(手順 1)。
- 生の `%…%` の綴り: `%FLStudioFactoryData%\Data\Patches\…` の 27 件が、`GetFileAttributesW` に「パスが無い」(3)で届く(`variableMissing` に出る)。記録の範囲では、この綴りの前に展開の呼び出しは見えない(上限のため確かではない)。Windows の `GetFileAttributesW` も `%…%` を展開しないので、Windows でも同じ結果になる。FL が絶対パスの探索の後で生の綴りも試す、という FL 自身の探索と読める。この Issue では直さない。
- 絶対パスの要求: 同じ 27 件のサンプルは、展開後の `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\…` の綴りでも要求され、「パスが無い」で届く。インストーラの一覧(`inputs/fl-installer-files.tsv`)に在り、展開後の木に実物がある。`lift/harness/seeds/vfs-seed-idle1.tsv` に行が無く、配信されていない。これは #1403 の範囲。
- 試して採らなかった案: `HKCU\Environment` の読みと、その種の行。FL が上書きするので要らない。前回の作業では削除した。
- 試験: `cargo test -p fe-lift --test env` の 13 件が通る(変更前は 9 件が赤)。`vfs`(23)・`reg`(15)・`window`(37)も通る。`fle_env.c` の文字列を見る試験(`wall_fr3.rs`・`wall_r2.rs`)は、`fle_env.h` の位置に合わせた。
- 既存の失敗(この変更とは無関係): `k32_dsp`・`wall_b`・`wall_e1`・`wall_fr11` などは、`fle_clip_host_read`/`write` が未定義でリンクできない(#1389 以降)。
- 受け入れの他の値: boot は engine-idle・standIn が false・error が null(`elapsedMs` 12177)。流れ A〜D は 4 行とも基準内(`elapsedMs` 11514〜11901、`preBootBytes` 311145137)。`npm test --prefix app`: 1406 passed・10 skipped。
- 変更の影響: `GetTempPathW` は TMP を返すようになった(従来は `C:\Windows\`)。その場所に実ファイルを作る処理の挙動は未確認。
- UCRT(#1386): main に `fle_crt.c` は無い。前回の差分の `fle_env.c` の内部の関数(`env_store`)が、Set 側の共有の入口。#1386 が入るときは、その入口を使う。

## 読む物

- `lift/runtime/fle_env.c`・`lift/runtime/fle_env.h`
- `lift/runtime/fle_reg.c`・`lift/harness/reg_seed.h`・`lift/harness/seeds/reg-seed-nsis.tsv`・`lift/harness/seeds/reg-seed-app.tsv`
- `lift/crates/fe-lift/tests/env.rs`(環境の API の試験の手本)
- `tools/p3-audio-probe/measure.mjs`(`openProject`・`missingBreakdown`・`rootKind`・`envLog`)、`tools/delivery-split/measure.mjs`
- `tools/smoke/welcome-flow.mjs`
- `inputs/fl-installer-files.tsv`(インストーラの一覧。接頭辞を外した相対パス)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 観測は前回の試行で済んだ(上の節)。前回の差分の記録(`env: <API> "<名前>" <結果> <n>` の行)は、常に動く軽い記録として残す。その上限(`lift/runtime/fle_env.c` の `FLE_ENV_LOG_MAX`。前回は 20)を **200** にする。環境変数と `#ifdef` は足さない。
2. 実装。環境ブロックを 1 つにする。Set・Get・Expand・GetEnvironmentStrings・UCRT の関数が、同じブロックを読み、書く。Windows と同じ振る舞い(名前は大文字小文字を区別しない・見つからない名前は書かれた通りに残す・Set の NULL は削除・名前に `=` を含むと拒否)は、既存の `fle_env.c` の振る舞いを保つ。
   - 初期のブロック: Windows のセッションの標準の変数を入れる。`SystemRoot`・`SystemDrive`・`windir`・`USERNAME`・`USERPROFILE`・`APPDATA`・`LOCALAPPDATA`・`ProgramData`・`ProgramFiles`・`ProgramFiles(x86)`・`CommonProgramFiles`・`TEMP`・`TMP`・`COMPUTERNAME`・`OS`(`Windows_NT`)・`PATH`・`PATHEXT`・`NUMBER_OF_PROCESSORS`・`PROCESSOR_ARCHITECTURE`・`HOMEDRIVE`・`HOMEPATH`・`USERDOMAIN`。値は `fle_env.h` の `FLE_VIRTUAL_*` と `SHGetFolderPathW` の答えから作り、別の定数の表を持たない。
   - FL の変数: FL が起動の時に自分で `SetEnvironmentVariableW` で設定する(前回の試行で確認)。そのため、レジストリや種のファイルには入れない。前回の差分に `HKCU\Environment` の読みや種の行が在れば、外す。試験で、FL が設定するのと同じ名前と値を Set し、Get と Expand で読めることを確かめる。
   - 生の `%…%` の綴りを `GetFileAttributesW` に渡す要求は、この Issue では直さない(FL 自身の探索。素材の配信は #1403)。
3. 試験(`lift/crates/fe-lift/tests/env.rs`)は、前回の 13 件を保ち、FL が設定する変数の Set→Get→Expand の試験を足す。新しい試験は、実装の前に red を確かめる。`cargo test` で通ることを確かめる。
4. 測定。DEDZII を File > Open で開き、`tools/p3-audio-probe/measure.mjs` の出力の `envLog` に、FL の 3 つの `SetEnvironmentVariableW` の行と、`ExpandEnvironmentStringsW` の行が出ることを見る。開く段で止まっても(前回の試行で止まった。別の Issue #1421 が扱う)、`envLog` は JSON に出す。`envLog` は今、`projectReport`(開く段が成功した後)でしか作られないので、`openProject` が失敗したときも、起動から今までの engine のログの `env:` の行を `envLog` に入れる。止まった時の `error` は判定に使わない。道具は開く段の失敗では終了コード 0 を返す(#1400 の規則)。`inventory`・`rootStripped` も JSON に出るが、判定には使わない(#1403 の範囲)。
5. 受け入れを通す。通らなければ、作業ツリーを commit して終える。

前回の差分は、`.prior-work.md` の取り込みで作業ツリーに入る。足すのは、上の 4 の `envLog` の直し(開く段が失敗したときも出す)だけ。それ以外の新しい変更は足さない。この Issue は、ほぼ完了している。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && cargo test --locked --manifest-path lift/Cargo.toml -p fe-lift --test env && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-env && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/DEDZII - NIGHT TERROR.flp" .tmp-env/dedzii.flp && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-env/dedzii.flp" > "$RUNNER_TEMP/open-dedzii.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());const L=Array.isArray(r.envLog)?r.envLog:[];const has=(re)=>L.some((l)=>re.test(l));process.exit(has(/^(\[err\] )?env: SetEnvironmentVariableW "FLStudioFactoryData" set \d+$/)&&has(/^(\[err\] )?env: SetEnvironmentVariableW "FLStudioUserData" set \d+$/)&&has(/^(\[err\] )?env: SetEnvironmentVariableW "ILSharedData" set \d+$/)&&has(/^(\[err\] )?env: ExpandEnvironmentStringsW ".*" expanded \d+$/)?0:1)' "$RUNNER_TEMP/open-dedzii.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- 起動 1 回で、engine-idle・standIn が false・error が null(boot)。
- DEDZII の測定(File > Open)で、`envLog` に、`SetEnvironmentVariableW` の `FLStudioFactoryData`・`FLStudioUserData`・`ILSharedData` の各 1 行(`set <n>`)と、`ExpandEnvironmentStringsW` の 1 行(`expanded <n>`)が在る。起動の時の記録で足りる(開く段より前の行)。`error`・`state`・`inventory`・`rootStripped` は判定に使わない。開く段の停止(`error`)は、別の Issue #1421 が扱う。
- 流れ A〜D(`welcome-flow.mjs`)の 4 行: 各行で `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。`elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。
- 環境の試験(`cargo test -p fe-lift --test env`)と、単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 観測の結果(どの API を、どの名前で、どの順で呼んだか。FL の変数の Set・Get・Expand)、初期のブロックの値、`envLog` の FL の行(3 つの Set と Expand)、`error`(開く段の停止の理由。判定には使わない)、`inventory`・`rootStripped`(判定には使わない。参考)、流れ A〜D の 4 行、`elapsedMs`・`preBootBytes`、`lift/`・`app/` を変えたこと(push すると `deploy.yml` が動く)。

## 制約

- 禁止されているのは、AI のベンダー名と、Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- 製品に、新しい環境変数(ビルド・実行を切り替える旗)・`#ifdef`・URL パラメータ・ビルド変数を足さない。ゲストの環境変数は Windows と同じ振る舞いの製品の機能で、オプトインを作らない。
- 標準の変数の値は、既存の `FLE_VIRTUAL_*` と `SHGetFolderPathW` の答えから作る(別の値の表を持たない)。FL の変数は種に入れない(FL が自分で設定する)。
- 名前の規則(訂正): 禁止されているのは、AI のベンダー名(`guard/scan.sh` が検査する)と、Cloudflare の識別子だけ。FL の製品名・`Image-Line` のインストール先・`%FLStudioFactoryData%` などの変数名は、ファイル・Issue の本文に書いてよい(`lift/crates/fe-build/src/plugins.rs` などが既に書いている)。Cloudflare の資源の名前・アカウント ID・ホスト名は書かない。
- Python と OCR を使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- エンジンのビルドは約 35 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。`sleep`・`timeout` をコマンドの先頭に書かない。
- 起動の間欠の停止や CDP の timeout で流れ A〜D が落ちたら、報告に書き、commit して終える。

## 他のタスクとの関係

- 後続: #1403(demo 2 件の素材の配信。段階 2)は、この Issue とは独立に起動する(前回は blocked by にしていたが、外した)。欠けた素材の配信は #1403 の範囲で、この Issue の受け入れには入れない。
- #1386(UCRT の環境変数の関数。`lift/runtime/fle_crt.c`)と、同じ環境ブロックを使う。
- UCRT の環境関数(`_wgetenv`・`_putenv`)は、#1386 のマージ後に、同じ `env_find`・`env_store` を使う。#1386 が main に入るまでは、UCRT の関数に手を付けず、報告の「未確認・残り」に書く。
- #1400(Sytrus)・#1387(Riff machine)とは独立。
