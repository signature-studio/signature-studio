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

FL は、素材(サンプル)のパスを `%FLStudioFactoryData%\Data\Patches\…` の形で持ち、Windows では環境変数を展開して開く。この VM では展開されないまま `GetFileAttributesW` に届くので、DEDZII - NIGHT TERROR が要求する素材のうち 19 件が見つからない(#1403 の 2 回目の試行の結果)。この Issue は、ゲストの環境変数を Windows と同じ振る舞いにし、FL が変数を展開できるようにする。

目標:
1. ゲストの環境ブロックが 1 つで、`SetEnvironmentVariableW`・`GetEnvironmentVariableW`・`ExpandEnvironmentStringsW`・`GetEnvironmentStrings`(A と W)、と、UCRT の環境変数の関数が、同じブロックを読み、書く。
2. 起動の時に、Windows のセッションの標準の変数と、FL が使う変数(`FLStudioFactoryData`・`FLStudioUserData` など。実測で決める)が、ブロックに在る。
3. DEDZII を File > Open で開くと、`GetFileAttributesW` に `%` を含む綴りの要求が届かず、`inventory` が 0 になる。

## 背景

- 事実 1(#1403 の 2 回目の試行。run 45518902053): DEDZII の `measure.mjs` で `inventory` が 19。19 件の要求は、すべて `%FLStudioFactoryData%\Data\Patches\…` の綴りで、`GetFileAttributesW` が「パスが無い」(3)を返す。Mehran は 0。Blocks.flp(テンプレート)の測定では、変数の要求は 0 だった。
- 事実 2(同じ試行のエンジンの記録): ゲストは `ExpandEnvironmentStringsW` を呼んでいない。VFS は `%…%` を展開しない。`lift/runtime/fle_env.c` の `ExpandEnvironmentStringsW` は、見つからない名前を書かれたまま残す(MSDN の振る舞い)。
- 事実 3(コード): 環境ブロックは `lift/runtime/fle_env.c` の静的な配列(`env_v`・`env_n`)で、起動の時の初期化は無い(空で始まる)。実装は、`GetEnvironmentVariableW`(125 行付近)・`SetEnvironmentVariableW`(153 行付近)・`ExpandEnvironmentStringsW`(202 行付近)・`GetEnvironmentStringsW`(314 行付近)・`FreeEnvironmentStringsW`(347 行付近)・`GetEnvironmentStrings`(403 行付近)・`FreeEnvironmentStringsA`(440 行付近)。`GetEnvironmentVariableA`・`SetEnvironmentVariableA`・`ExpandEnvironmentStringsA` は `lift/runtime` に無い。
- 事実 4(インストーラ): `lift/harness/seeds/reg-seed-nsis.tsv` の注記では、インストーラのスクリプトが書くレジストリの値は `Software\Image-Line\Shared\Paths` の下だけで、環境変数の鍵は無い。FL の変数がどこから来るかは、実測で決める。
- 事実 5(仮想の値): `lift/runtime/fle_env.h` の `FLE_VIRTUAL_*`(Windows のディレクトリ `C:\Windows`・`C:\Windows\System32`、ユーザーのプロファイル `C:\Users\user`、ユーザー名 `user`、コンピュータ名 `DESKTOP-0000001`、DLL と EXE のパス)。`SHGetFolderPathW`(`fle_env.c` の 500 行付近)の答えは、これらの値から作られている。初期のブロックの値は、これと一致させる。
- 関係: #1386 の作業は、UCRT の環境変数の関数(`_wgetenv`・`_putenv`・`_wputenv_s`・`getenv`・`_dupenv_s`)を `lift/runtime/fle_crt.c` に足す。この Issue を書いた時点で、それは main に無い。UCRT の関数も、この Issue と同じブロックを読む形にする。#1386 が main に入る前は、Win32 の API だけを直し、UCRT の関数の扱いを報告に書く。

## 読む物

- `lift/runtime/fle_env.c`・`lift/runtime/fle_env.h`
- `lift/runtime/fle_reg.c`・`lift/harness/reg_seed.h`・`lift/harness/seeds/reg-seed-nsis.tsv`・`lift/harness/seeds/reg-seed-app.tsv`
- `lift/crates/fe-lift/tests/env.rs`(環境の API の試験の手本)
- `tools/p3-audio-probe/measure.mjs`(`openProject`・`missingBreakdown`・`rootKind`)、`tools/delivery-split/measure.mjs`
- `tools/smoke/welcome-flow.mjs`
- `inputs/fl-installer-files.tsv`(インストーラの一覧。接頭辞を外した相対パス)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 観測(先に、実測で)。ゲストが環境変数の API を、どの名前・どの順で呼ぶかを、エンジンのログに出す。常に動く軽い記録(API の名前・名前・結果の長さ。最初の 20 件まで)。環境変数と `#ifdef` は足さない。起動から Welcome を閉じ、DEDZII を File > Open で開くまでの間に、`FLStudioFactoryData`・`FLStudioUserData` などの名前が、`SetEnvironmentVariableW` で設定されるのか、`GetEnvironmentVariableW`・`ExpandEnvironmentStringsW` で読まれるだけなのかを特定する。結果を報告に書く。
2. 実装。環境ブロックを 1 つにする。Set・Get・Expand・GetEnvironmentStrings・UCRT の関数が、同じブロックを読み、書く。Windows と同じ振る舞い(名前は大文字小文字を区別しない・見つからない名前は書かれた通りに残す・Set の NULL は削除・名前に `=` を含むと拒否)は、既存の `fle_env.c` の振る舞いを保つ。
   - 初期のブロック: Windows のセッションの標準の変数を入れる。`SystemRoot`・`SystemDrive`・`windir`・`USERNAME`・`USERPROFILE`・`APPDATA`・`LOCALAPPDATA`・`ProgramData`・`ProgramFiles`・`ProgramFiles(x86)`・`CommonProgramFiles`・`TEMP`・`TMP`・`COMPUTERNAME`・`OS`(`Windows_NT`)・`PATH`・`PATHEXT`・`NUMBER_OF_PROCESSORS`・`PROCESSOR_ARCHITECTURE`・`HOMEDRIVE`・`HOMEPATH`・`USERDOMAIN`。値は `fle_env.h` の `FLE_VIRTUAL_*` と `SHGetFolderPathW` の答えから作り、別の定数の表を持たない。
   - FL の変数: 手順 1 の結果、FL が読むだけなら、インストーラが書いたはずの値を、データ(レジストリの種。または新しい種のファイル)に入れ、起動の時に読む。値を runtime のコードの定数にしない。インストーラの実際の値が記録に無ければ、`FLStudioFactoryData` = `C:\Program Files\Image-Line\FL Studio 21\`、`FLStudioUserData` = `C:\Users\user\Documents\Image-Line\FL Studio\` を、この環境の選択として入れ、報告に「選択」と書く。
   - FL が自分で `SetEnvironmentVariableW` で設定するなら、その結果が Get と Expand で見えることを、試験で確かめる。
3. 試験を先に red で書く。環境ブロックの試験を `lift/crates/fe-lift/tests/env.rs` に足す。Set→Get→Expand が同じブロックを見る。初期の値。大文字小文字。見つからない名前。A 版と W 版の一致。`cargo test` で red を確かめてから実装する。
4. 測定。DEDZII を File > Open で開き、`tools/p3-audio-probe/measure.mjs` の出力で `inventory` が 0、`rootStripped.variable` が 0 であることを見る。`%` を含む要求の件数が出ない場合は、`measure.mjs` に 1 項目足す(tools/ の変更は許す)。
5. 受け入れを通す。通らなければ、作業ツリーを commit して終える。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && cargo test --locked --manifest-path lift/Cargo.toml -p fe-lift --test env && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-env && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/DEDZII - NIGHT TERROR.flp" .tmp-env/dedzii.flp && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-env/dedzii.flp" > "$RUNNER_TEMP/open-dedzii.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());process.exit(r.error===null&&r.state==="engine-idle"&&r.inventory===0&&r.rootStripped.variable===0?0:1)' "$RUNNER_TEMP/open-dedzii.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- 起動 1 回で、engine-idle・standIn が false・error が null(boot)。
- DEDZII の測定で、`error` が null・`state` が engine-idle・`inventory` が 0・`rootStripped.variable` が 0。
- 流れ A〜D(`welcome-flow.mjs`)の 4 行: 各行で `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。`elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。
- 環境の試験(`cargo test -p fe-lift --test env`)と、単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 観測の結果(どの API を、どの名前で、どの順で呼んだか。FL の変数が Set されるのか、Get だけか)、初期のブロックの値、種のデータの行、`inventory`・`rootStripped`、流れ A〜D の 4 行、`elapsedMs`・`preBootBytes`、`lift/`・`app/` を変えたこと(push すると `deploy.yml` が動く)。

## 制約

- 製品に、新しい環境変数(ビルド・実行を切り替える旗)・`#ifdef`・URL パラメータ・ビルド変数を足さない。ゲストの環境変数は Windows と同じ振る舞いの製品の機能で、オプトインを作らない。
- 値を runtime のコードの定数にせず、データ(種のファイル)にする。既存の `FLE_VIRTUAL_*` の定数は、そのまま使ってよい。
- 名前の規則(訂正): 禁止されているのは、AI のベンダー名(`guard/scan.sh` が検査する)と、Cloudflare の識別子だけ。FL の製品名・`Image-Line` のインストール先・`%FLStudioFactoryData%` などの変数名は、ファイル・Issue の本文に書いてよい(`lift/crates/fe-build/src/plugins.rs` などが既に書いている)。Cloudflare の資源の名前・アカウント ID・ホスト名は書かない。
- Python と OCR を使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- エンジンのビルドは約 35 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。`sleep`・`timeout` をコマンドの先頭に書かない。
- 起動の間欠の停止や CDP の timeout で流れ A〜D が落ちたら、報告に書き、commit して終える。

## 他のタスクとの関係

- 後続: #1403(demo 2 件の素材の配信。段階 2)は、この Issue が閉じた後に起動する(blocked by)。#1403 の 19 件は、変数の綴りが展開されれば、配信済みの絶対パスの行と一致する。
- #1386(UCRT の環境変数の関数。`lift/runtime/fle_crt.c`)と、同じ環境ブロックを使う。
- #1400(Sytrus)・#1387(Riff machine)とは独立。
