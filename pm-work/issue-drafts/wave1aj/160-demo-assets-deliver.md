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
1. DEDZII と Mehran の両方で、`inventory`(要求が見つからなかった素材のうち、インストーラに在る物。DLL を除く)が 0。
2. 見つからなかった残りの内訳を出す: `notInInstaller` の件数と先頭 10 件(JSON と報告に出す)。
3. DEDZII と Mehran が開く(`error` が null・`engine-idle`)。流れ A〜D が通り、`elapsedMs` と `preBootBytes` が基準を超えない。

## 背景

- 段階 1 の出力: `pm-work/findings/demo-assets-inventory.tsv`(列: `demo`・`path`・`bytes`・`sha256`。`path` は一覧の guest の綴りで、インストール先からの相対)。各 `path` は、`ci/resources-prefixes.txt` の完全なパスの行として在る。実物は、次の run の取り出しで `lift/out-app/host-extra/resources/` に、相対パスと同じ木の形で在る。
- 前回の run の実測(段階 1 の前): DEDZII の一覧に在る素材は 27 件・合計 4,830,812 B。Dance の 12 件と Misc の 9 件は、seed 表の `@lazy-file:` の行で配信済み(段階 1 の差分に含まれる)。残りの 27 件が、この Issue の対象。
- 配信の仕組み: 起動の後の取得(`@lazy-file:`)。seed 表(`lift/harness/seeds/vfs-seed-idle1.tsv`)の行と、`lift/harness/web/pack.args` の `--lazy-file` の組が要る。ゲストの仮想のファイルシステムに載るのは、この 2 つの行が在る物だけ(`pm-work/workflow-design.md` の「接頭辞で取った物は」)。
- lazy の上限は `app/src/bundle/manifest.ts` の `maxItems`(128)。段階 1 の時点の件数に 27 を足すと、上限を超える可能性がある。実測で数え、超えるなら上げる(既定の動作として。オプトインを作らない)。
- `app/scripts/pack-assets.mjs` は、URL の `#`・`%`・`?` を `%XX` にする(段階 1 で入った。`#` より後ろが fragment として切られて読めなかった Mehran の `.ogg` の修正)。新しい素材のファイル名に `#`・`%`・`?` が含まれるなら、この修正で読めることを確かめる。

## 読む物

- `pm-work/findings/demo-assets-inventory.tsv`(段階 1 の出力)
- `ci/resources-prefixes.txt`(段階 1 で足した完全なパスの行)
- `lift/harness/seeds/vfs-seed-idle1.tsv`(`@lazy-file:` の行の形。`Data\Patches\Packs\Legacy\Drums\Dance\DNC_*.wav` の行が手本)
- `lift/harness/web/pack.args`(`--lazy-file` の組の形)
- `app/src/bundle/manifest.ts`(`maxItems`)、`app/scripts/pack-assets.mjs`
- `tools/p3-audio-probe/measure.mjs`(段階 1 で入った内訳)、`tools/smoke/welcome-flow.mjs`
- `pm-work/workflow-design.md`(「接頭辞で取った物は」の段落)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。
1. 段階 1 の出力を確かめる: `demo-assets-inventory.tsv` の各行の `path` が `ci/resources-prefixes.txt` に在り、取り出された実物(`lift/out-app/host-extra/resources/` の下)の大きさと sha256 が `bytes`・`sha256` と一致する。一致しない行が在れば、報告に書いて止める(一覧の値が正)。
2. 各行を配信する。(a) seed 表に `@lazy-file:` の行を足す。1 列目は guest(インストール先の絶対パスの下の綴り。前の行と同じ書き方で、接頭辞は同じ seed 表の既存の行からコピーする)、2 列目は `@lazy-file:resources/<path の \ を / にした物>`、3 列目は sha256(一覧の値)。行の形は `DNC_*.wav` の行と同じ。(b) `lift/harness/web/pack.args` に、同じ数の `--lazy-file` の組を足す。組の形は既存の行と同じ(`lift/out-app/host/resources/<path>=seed/resources/<path>`)。`lift/crates/fe-build/tests/plan/bundle.txt` は main の版では生成の形(`@lines`)なので、組は `pack.args` にだけ足す。空の代替(空ファイル・0 埋め)は置かない。
3. lazy の件数を数える。`app/src/bundle/manifest.ts` の `maxItems`(128)を超えるなら、上限を上げる。件数と、上げたかどうかを報告に書く。
4. 配信の後の `inventory` が 0 になることを、受け入れの出力で確かめる。0 にならない素材が在れば、その綴りと理由(seed 行の綴りの違い・大文字小文字・`#`/`%`/`?` の扱い)を報告に書く。
5. 受け入れを通す。通らなければ、作業ツリーを commit して終える。
6. commit する。push はしない(ジョブの最後の判定が通ったときだけ push される)。`app/` を変えたら、push すると `deploy.yml` が動くので、報告に書く。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-160 && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/DEDZII - NIGHT TERROR.flp" .tmp-160/dedzii.flp && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/Mehran Abbasi - With You.flp" .tmp-160/mehran.flp && for f in dedzii mehran; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-160/$f.flp" > "$RUNNER_TEMP/open-$f.json"; done && node -e 'const fs=require("fs");const L=process.argv.slice(1).map((f)=>JSON.parse(fs.readFileSync(f,"utf8").trim()));const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.inventory===0&&typeof r.notInInstaller==="number"&&typeof r.dll==="number"&&Array.isArray(r.notInInstallerFirst10)&&r.notInInstallerFirst10.length<=10;process.exit(L.length===2&&L.every(ok)?0:1)' "$RUNNER_TEMP/open-dedzii.json" "$RUNNER_TEMP/open-mehran.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- 起動 1 回で、engine-idle・standIn が false・error が null(boot)。
- DEDZII と Mehran の `measure.mjs` の行で、`error` が null・`state` が engine-idle・`inventory` が 0(数値)。`notInInstaller` と `dll` が数値、`notInInstallerFirst10` が配列で 10 件以下。
- 流れ A〜D(`welcome-flow.mjs`)の 4 行: 各行で `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。各行の `elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。
- 単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 配信した件数と合計の大きさ、追加した seed 行と `pack.args` の組の数、lazy の件数と `maxItems` を変えたか、DEDZII と Mehran の `inventory`・`dll`・`notInInstaller`・`notInInstallerFirst10`、流れ A〜D の 4 行、`elapsedMs`・`preBootBytes`、`app/` を変えたこと(push すると `deploy.yml` が動く)。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。遅延は既定の動作にする(オプトインを作らない)。
- 空の代替(空ファイル・0 埋め)を置かない。大きさと SHA-256 は、一覧(インストーラの実物から取った値)と照合したものだけを置く。
- インストーラに無い素材を、どこからも取らない(配信の対象外)。
- ベンダー名・Cloudflare の識別子・インストール先の既定の値(絶対パス・工場データの変数の綴り)を、本文とファイルに書かない。
- Python は製品に使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- エンジンのビルドは 1 回約 32〜46 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。`sleep`・`timeout` をコマンドの先頭に書かない。

## 他のタスクとの関係

- blocked_by: #1373(段階 1。素材の収集と、`ci/resources-prefixes.txt` の行)。閉じた後に起動する。
- 後続: #1401(音が出る demo を決める)。#1401 は、この Issue の後に行う。#1335(P3 の測定)は #1401 を待つ。
- #1387(Riff machine)・#1400(Sytrus)・#1395(言語)も、seed 表と `pack.args` を変える。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
