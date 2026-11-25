---
draft: "171"
title: "Sytrus の Data(109 件)を、起動の後の取得で配信し、`Default.dat` の空のパスで止まらない"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "022"
blocked_by: ["157"]
labels: ["plugin-builtin", "resources"]
size: M
priority: High
project: signature-studio/3
---

## 目的

内蔵の音源 Sytrus は、読み込みの時に、自分の `Data` フォルダの中身(`Data\Default.dat` など)を読む。このフォルダが VFS に無いため、Sytrus の `CreateFileW` が空のパスを受け、`fle: CreateFileW: empty path is not defined` で止まる。`Data` の 109 件を全部 seed に入れると、boot の段で「more than 512 seed files」で止まる。この Issue は、両方を直し、Sytrus の `Data` を起動の後の取得で配信する。

目標:
1. 空のパスの `CreateFileW` の原因を特定し、直す。Sytrus が `Data` の場所を組み立てる経路が、VFS の中の正しいパスを得るようにする。
2. Sytrus の `Data` の 109 件(合計 1,229,971 B)を、起動の後の取得(lazy)で配信する。seed の上限(boot の 512 件・lazy の 128 件)を超えない形にする。
3. demo(synth・olbaid)の測定で、空のパスの停止が出ない。

## 背景

- 事実(#1400 の 2 回目の試行。run 45518902053): `Data\Default.dat`(1,832 B)が無いと、空のパスの `CreateFileW` で止まる。`Default.dat` の 1 件だけを seed に足すと、この停止は消え、次の停止(switch 表。#{{170}} が扱う)が出る。`Data` の 109 件を全部 seed に入れると、boot の段で止まる。
- 一覧: `inputs/fl-installer-files.tsv` の `Plugins\Fruity\Generators\Sytrus\Data\` の下に 109 件・1,229,971 B。ファイル名に空白を含むものが 71 件ある。`Default.dat` の sha256 は `adc5842c…`。
- 取り出し: `ci/plugin-builtin-prefixes.txt` の `Plugins\Fruity\Generators\` で、Sytrus の `Data` は既に取り出されている(前回の試行で、`host-extra` に在ることを確かめた)。`ci/` を変える必要は、確かめた上で無い時は足さない。
- 上限: boot の seed は、`lift/harness/vfs_lazy.h` の `VL_REG_MAX`(512。文言「more than %u seed files」は `lift/harness/vfs_seed.h`)。lazy の件数は、`app/src/bundle/manifest.ts` の `maxItems`(128)。`pack-seed` の `--lazy` も 128 件までと、前回の試行で確かめた。
- 空白: seed の `--lazy` の行は空白で区切って読まれる(`RunWords`。前回の試行の観測)。空白を含む host のパスは、`_` にした名前にする(guest 側の名前は元のまま)。
- 型: 既存の `@lazy-file:` の行(`lift/harness/seeds/vfs-seed-idle1.tsv`)と、`lift/harness/web/pack.args` の `--lazy-file` の組。#{{160}} と同じ表を編集するので、衝突したら main を取り込み、直す。

## 読む物

- `lift/harness/seeds/vfs-seed-idle1.tsv`(`@lazy-file:` の行の形と、先頭の注釈 #1345)
- `lift/harness/vfs_seed.h`(`more than %u seed files`)・`lift/harness/vfs_lazy.h`(`VL_REG_MAX`)
- `app/scripts/pack-seed.mjs`(`--lazy`・`--extra`)・`app/scripts/pack-assets.mjs`・`app/src/bundle/manifest.ts`(`maxItems`)
- `lift/harness/web/pack.args`
- `lift/runtime/fle_env.c`(`SHGetFolderPathW` の答え)・`lift/runtime/fle_env.h`(`FLE_VIRTUAL_*`)
- `inputs/fl-installer-files.tsv`
- `tools/p3-audio-probe/measure.mjs`(`missingBreakdown`・`installerRows`)・`tools/smoke/welcome-flow.mjs`

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 空のパスの原因を特定する。Sytrus が `Data` の場所を組み立てる経路を、ログ(`CreateFileW` の前の呼び出し)で確かめる。`GetModuleFileNameW` の答え・`SHGetFolderPathW` の答え・`Plugins` の下のパスなど。原因を、答えの値で説明する。「Data の場所を VFS が持たない」なら手順 2 で直す。「組み立ての値が空」なら、その API の答えを仕様どおりにする(この Issue の中で直す)。
2. `Data` を配信する。109 件を、ファイル単位の `@lazy-file:` の行(`vfs-seed-idle1.tsv`)と、`pack.args` の `--lazy-file` の組で足す。1 列目(guest)は既存の行と同じ書き方。2 列目は、取り出した実物の host の綴り。3 列目は sha256(一覧の値)。大きさと sha256 は、一覧と照合したものだけを置く。空の代替は置かない。
3. 上限を収める。lazy は、既存の件数に 109 を足すと `maxItems`(128)を超える。超えるなら、既定の値として `maxItems` を上げる(オプトインを作らない)。boot の seed の件数が `VL_REG_MAX`(512)を超えるなら、同じく既定の値を上げるか、配信の仕方を変える。どうしたかを、件数と一緒に報告に書く。
4. 空白を含む名前は、host の綴りを `_` にする(guest の綴りは元のまま。前回の試行の観測)。
5. 受け入れを通す。通らなければ、作業ツリーを commit して終える。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-sytrus-data && cp "lift/out-app/host/resources/Data/Demo projects/Product demos/Synth_dfr - Overcome - Sytrus.flp" .tmp-sytrus-data/synth.flp && cp "lift/out-app/host/resources/Data/Demo projects/Product demos/Olbaid - Sytrus Essentials.flp" .tmp-sytrus-data/olbaid.flp && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-sytrus-data/synth.flp" > "$RUNNER_TEMP/open-synth.json" && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-sytrus-data/olbaid.flp" > "$RUNNER_TEMP/open-olbaid.json" && node -e 'const fs=require("fs");const L=["synth","olbaid"].map((f)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/open-"+f+".json","utf8").trim()));process.exit(L.every((r)=>!JSON.stringify(r).includes("empty path")&&!JSON.stringify(r.inventoryList??[]).includes("Sytrus"))?0:1)' && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- 起動 1 回で、engine-idle・standIn が false・error が null(boot)。
- synth と olbaid の測定の出力に、空のパスの停止(`empty path`)が無い。`inventoryList` に Sytrus の行が無い(一覧に在る Sytrus の `Data` の要求が、missing にならない)。
- 流れ A〜D(`welcome-flow.mjs`)の 4 行: 各行で `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。`elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。
- 単体テスト(`npm test --prefix app`)が通る。

注: demo 2 件が最後まで開くことは、この Issue の基準に入れない(switch 表の #{{170}} と組で解ける)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 空のパスの原因(答えの値と、呼び出しの順)、配信した件数と合計の大きさ、追加した `@lazy-file:` の行と `--lazy-file` の組の数、lazy の件数と `maxItems`・`VL_REG_MAX` を変えたか、synth と olbaid の測定の出力(空のパスの有無と、Sytrus の要求の有無)、流れ A〜D の 4 行、`elapsedMs`・`preBootBytes`、`lift/`・`app/` を変えたこと(push すると `deploy.yml` が動く)。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。上限の変更は既定の値として行い、オプトインを作らない。
- 空の代替(空ファイル・0 埋め)を置かない。大きさと SHA-256 は、一覧(インストーラの実物から取った値)と照合したものだけを置く。
- インストーラに無い素材を、どこからも取らない。
- 名前の規則(訂正): 禁止されているのは、AI のベンダー名(`guard/scan.sh` が検査する)と、Cloudflare の識別子だけ。FL の製品名・`Image-Line` のインストール先・`%FLStudioFactoryData%` などの変数名は、ファイル・Issue の本文に書いてよい。Cloudflare の資源の名前・アカウント ID・ホスト名は書かない。
- Python と OCR を使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- エンジンのビルドは約 35 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。`sleep`・`timeout` をコマンドの先頭に書かない。

## 他のタスクとの関係

- blocked_by: #1400(Sytrus の wasm を、起動の後に読み込めるようにする。段階 1)。閉じた後に起動する。
- #{{170}}(switch 表の飛び先)とは独立。どちらも demo が最後まで開くために要る。
- #{{160}}(#1403。demo 2 件の素材)と、同じ表(`vfs-seed-idle1.tsv`・`pack.args`)を編集する。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- #1387(Riff machine の窓)は、この Issue と #{{170}} が閉じた後に起動する。
