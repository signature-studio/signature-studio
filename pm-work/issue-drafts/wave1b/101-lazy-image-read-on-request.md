---
draft: "101"
title: "起動時に全 DLL を写す設計を、guest の LoadLibrary の時まで遅らせる(遅延の読み込みの完成)"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["105"]
labels: ["delivery", "startup"]
size: L
priority: High
project: signature-studio/3
---

## 目的

プラグインの像(DLL)を、起動の経路から外す。今は、起動の時に引数の全ての DLL を写し、seed の複製も起動の時に読む。guest が `LoadLibrary` などでそのモジュールを要求した時に、初めて写して読む形にする。

## 背景

- 今の起動の経路(`lift/harness/two/main_two.c`): `main` は、引数の全ての DLL を起動の時に `image_load` で写す(120〜124 行。失敗で `return 2`)。その後に、`fle_realmod_table` の件数と、写した件数の検査がある(152 行付近。違えば `return 2`)。seed の複製も、起動の時に `vs_read_host`(`lift/harness/vfs_seed.h`。258〜262 行付近。`fopen`・`fread` で読み、SHA-256 を検査する)で読まれる。呼び出しは `main_two.c` の 171 行付近の `vfs_seed_from_env`。
- `lift/harness/image.c` の `image_load` は `open` と `pread` で読む。ファイルが小さすぎると `file too small` で止まる。空や 0 埋めの代替を置いても、seed 表の SHA-256 の検査で止まる。代替では要求を満たせない。
- 前の作業の結果(#{{015}}。runner の値。この機械とは別枠): Fruity Balance の DLL の像と seed の複製を pack から外し、起動後の取得にした。起動前の取得の合計は 279.6 MB から 272.2 MB(272,181,853 B)に減った。elapsedMs は 6.4〜6.5 秒で、差は揺れの範囲に入る。
- 前の作業で分かったこと: engine は起動の途中で Balance を要求して待つ。そのため、取得を起動後に移しても、起動は前倒しにならなかった。Balance のコードは engine.wasm に入ったままで(168,996,523 B。前後で同じ)、これは #{{102}} で扱う。
- 前の作業の形(残す物): `app/scripts/pack-seed.mjs` に `--lazy HOST`、`app/scripts/pack-assets.mjs` に `--lazy-file SRC=PATH`。`lift/harness/web/pack.args` に `--lazy-dll fruity_balance_x64.6800000.dll` と `--lazy-file`、`lift/harness/web/seed.args` に `--lazy`(manifest の `lazy`)。読みは `app/src/engine/lazy-file.ts` の既存の仕組みで、engine は `Atomics.wait` で待つ。取得は `app/src/main.ts` の `startLazy`(677 行)が、`client.start()` の後に始める。
- 参考の既存の仕組み: `lift/harness/vfs_lazy.h`(install tree の遅延ファイル。決定 0314)。

## 読む物

- `lift/harness/two/main_two.c`(104〜175 行: 像の写しと、realmod の検査。171 行付近: seed の読みの呼び出し)
- `lift/harness/image.c`(56 行〜: `image_load`)
- `lift/harness/vfs_seed.h`(`vs_read_host` と、その呼び出し元)、`lift/harness/vfs_lazy.h`(参考)
- `lift/runtime/fle_realmod.h`(1〜10 行の注釈)、`lift/runtime/fle_realmod.c`、`lift/runtime/fle_load.c`(`LoadLibrary`・`GetModuleHandle` の解決)
- `lift/harness/web/pack.args`・`lift/harness/web/seed.args`(#{{015}} の `--lazy-dll`・`--lazy-file`・`--lazy`)
- `app/src/main.ts`(677 行 `startLazy`、759 行の呼び出し)、`app/src/engine/lazy-file.ts`
- `tools/delivery-split/measure.mjs`(起動後の取得の測定。この Issue で項目を足す)、`tools/boot-check/boot-bundle.mjs`

## やること

1. 変更の前に、起動の経路を確かめる。`tools/delivery-split/measure.mjs` の 3 回の出力(起動前の取得の一覧と合計)と、起動の途中で Balance を要求する箇所(`fle_load.c` の `LoadLibrary` の経路。engine のログ)を報告に書く。
2. `main_two.c` の、引数の DLL の写し(`image_load`)と、seed の複製の読み(`vs_read_host`)を、guest の要求の時まで遅らせる。要求の時に写して、SHA-256 と大きさを検査する。検査が失敗した時の振る舞いを決め、報告に書く。黙って空の像を返さない。
3. 遅延の読みごとに、engine のログに 1 行を書く(stderr。環境変数・`#ifdef` は作らず、常に出す)。行の形は `image-read: <名前> requested=<0 か 1> <時刻 ms>`。時刻は起動の開始からの ms(`elapsedMs` と同じ基準。取り方は作業者が決める)。
4. `tools/delivery-split/measure.mjs` に、JSON の項目 `reads` を足す。`reads` は `[{"name","atMs","requested"}]` の配列で、engine のログの `image-read:` 行から作る。`requested` は guest の要求の時が true。`name` は像の名前(例: `fruity_balance_x64.6800000.dll`)。
5. 既定の動作として遅らせる。オプトインを作らない。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/delivery-split/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/feat-lazy.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const sum=(a)=>a.reduce((s,f)=>s+f.bytes,0);const fb=(a)=>a.some((f)=>/fruity.?balance/i.test(f.name));const ok=L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&!fb(r.preStart)&&sum(r.preStart)<=272181853&&Array.isArray(r.reads)&&r.reads.every((x)=>!/fruity.?balance/i.test(x.name)||x.requested===true||x.atMs>=r.elapsedMs));process.exit(ok?0:1)' "$RUNNER_TEMP/feat-lazy.jsonl"
```

判定の基準: 3 回の起動で、(1) engine-idle・standIn が false・error が null、(2) Fruity Balance が起動前の取得に無く、起動前の取得の合計が 272,181,853 B 以下、(3) `reads` の Balance の像の読みは、engine-idle の後に起きたか、guest の要求(`requested` が true)で起きたこと。(3) の確かめは、`reads` の欄が配列で在ること(欄が無いと落ちる)が前提。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 起動の途中の要求の有無(変更の前の読み)、遅らせた読みの一覧(名前・`requested`・時刻)、検査の失敗時の振る舞い、変えたパス(lift/・app/・tools/ を含む)、起動前の取得の合計(変更の前後)、elapsedMs の 3 回の値。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。遅延は既定の動作にする。
- 空の代替(空ファイル・0 埋め)で要求を満たさない。SHA-256 と大きさを検査した実物だけを写す。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 依頼者が言っていない制限(例: 他のプラグインの扱い)を、この作業の判断で増やさない。

## 他のタスクとの関係

- #{{015}}(Fruity Balance を起動後の取得に移す)の続き。この Issue は、その結果の「起動の前倒しにならなかった」を直す。
- #{{102}}(プラグインのコードを engine.wasm から分ける)は、この Issue の遅延の読みを使う。
- #{{014}}(区分の仕組み)と #{{035}}(起動を 10 秒以内にする)が、同じ起動の経路(`app/src/main.ts`・`app/src/engine/client.ts`)を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
