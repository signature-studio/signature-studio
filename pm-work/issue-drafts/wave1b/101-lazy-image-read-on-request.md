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

プラグインの像(DLL)を、起動の経路から外す。今は、起動の時に引数の全ての DLL を写す。guest が `LoadLibrary` などでそのモジュールを要求した時に、初めて写して読む形にする。この Issue の目的は、起動前の取得を減らすことと、像を読む時を遅らせること。起動の時間の目標は、別の Issue が扱う。

## 背景

- 今の起動の経路(`lift/harness/two/main_two.c`): `main` は、引数の全ての DLL を起動の時に `image_load` で写す(120〜123 行。失敗で `return 2`)。その後に、`fle_realmod_table` の件数と、写した件数の検査がある(152 行付近。違えば `return 2`)。seed の表は、起動の時に `vfs_seed_from_env`(171 行付近)が読む。表のうち `@lazy:` の索引行だけを、起動の時に `vs_read_host`(`lift/harness/vfs_seed.h`。249 行。`vs_run` の 505 行付近から呼ばれる)で読む。Balance などの seed の複製は `@lazy-file:` の行で、起動の時は名前とサイズだけを置く(`vs_put_lazy_file`。464 行付近)。中身は guest が読んだ時に `vl_read` が読み、SHA-256 を初回に検査する(#1345 の形)。起動の時に seed の複製を読むことは無い(コードの読みで確かめた。実測はしていない)。
- 前の作業の結果(#{{015}}。runner の値。この機械とは別枠): Fruity Balance の DLL の像と seed の複製を pack から外し、起動後の取得にした。起動前の取得の合計は 279.6 MB から 272.2 MB(272,181,853 B)に減った。elapsedMs は 6.4〜6.5 秒で、差は揺れの範囲に入る。その後、#1345 で言語・テーマ・テンプレートを起動前の seed pack に入れたため、今の main の起動前の取得の合計は 314,326,602 B になった。
- 前の作業で分かったこと: #{{015}} の時点では、engine は起動の途中で Balance を要求して待つと読んでいた。前回の試行の測定では、この要求は再現しなかった(下の「前回の試行で分かったこと」)。Balance のコードは engine.wasm に入ったままで(#{{015}} の時点で 168,996,523 B。今の main で 169,013,713 B)、これは #{{102}} で扱う。
- 前の作業の形(残す物): `lift/harness/web/pack.args` の `--lazy-dll`(Balance・Filter・Delay 2 の 3 件。pack に入れず、起動後に取る)と `--lazy-file`(Fruity の 3 件の seed の複製と、デモ曲)、`lift/harness/web/seed.args` の `--lazy`(Fruity の 3 件の seed の複製。pack に入れない)。読みは `app/src/engine/lazy-file.ts` の既存の仕組みで、engine は `Atomics.wait` で待つ。取得は `app/src/main.ts` の `startLazy`(680 行)が、`client.start()`(761 行)の後に始める。
- 参考の既存の仕組み: `lift/harness/vfs_lazy.h`(install tree の遅延ファイル。決定 0314)。

## 前回の試行で分かったこと

前回の試行は run 45100795756(約 2.5 時間。`failed`)。作業は完了したが、受け入れ (2) の古い基準で落ち、commit されずに終わった。作業は artifact `agent-1341`(`work.diff`・`report.md`)に保存されている。次の run は、その差分を作業ツリーに載せる(commit はしない。`.prior-work.md` が指示する)。

- 変えたパス(6 件): `lift/harness/two/main_two.c`・`lift/runtime/fle_realmod.c`・`lift/runtime/fle_realmod.h`・`app/src/engine/worker.ts`・`app/src/main.ts`・`tools/delivery-split/measure.mjs`。
- 起動の経路: 起動の時は FLEngine だけを写す。引数の DLL は、ファイル名(`<stem>.<base hex>.dll`)を小文字の `<stem>.dll` にして `fle_realmod_table` の項目へ照合するだけにした(件数の一致は起動の時に検査する)。写すのは、guest の `LoadLibrary` の時に呼ばれるフック `fle_realmod_map_image` の中だけ。写した後、ImageBase が表の base と一致することを検査する。`fruity_add_image` は、新しい `#ifdef` を足さないため弱い参照で呼ぶ。
- 行と項目: 読みのたびに stderr へ `image-read: <ファイル名> requested=1 <ms>` を出す(この経路の唯一の呼び元が guest の要求なので、`requested` は常に 1)。`app/src/engine/worker.ts`・`app/src/main.ts` がページの時刻の印にして `window.__app.imageReads` に積み、`tools/delivery-split/measure.mjs` が `reads`(`[{name, atMs, requested}]`、常に配列)にする。
- 遅らせた読み(最終の build の 3 回。atMs は runner の値): `dsp_ippv2_x64.2000000.dll`(3065〜3108 ms)・`Blend2D.4700000.dll`(3103〜3158)・`quickfontcache_x64.1d00000.dll`(3113〜3169)・`freetype_x64.4b00000.dll`(3143〜3209)・`ILTools_x64.6000000.dll`(3312〜3380)。5 件とも engine-idle より前で、`requested=1`。Balance・Filter・Delay 2・Oggio・Elastique は、3 回とも engine-idle まで読まれなかった。
- 起動の途中の要求: 変更の前の 3 回(`lazyWaitFirstMs` は 3 回とも null)と、変更の後の 6 回(2 つの build)で、engine-idle までの Balance の読みは 0 件。#{{015}} の「起動の途中で Balance を要求して待つ」は、この測定では再現しなかった。
- elapsedMs(runner。nproc 4。この機械とは別枠): 変更の前 10212.2・10149.2・10271.1 ms。変更の後(1 回目の build)9888.8・10021.1・10136.2 ms。変更の後(最終)10120.4・10121.9・10323.8 ms。差は揺れの範囲で、短縮は確認できていない。
- 起動前の取得の合計(復号後): 変更の前 **314,326,602 B**(engine wasm 169,013,713・seed pack 66,702,132・DLL pack 77,444,112・engine glue 1,166,645)。変更の後 **314,328,003 B**(engine wasm の +1,401 B だけ)。
- 検査と失敗時: 大きさと SHA-256 は、ページ側が検査する(`app/src/bundle/loader.ts` の `verifyingStream`。バイトを engine へ渡す前に照合し、失敗すると読みは EIO)。C 側では SHA-256 を計算していない(期待値を C へ渡す入力が無いため。入力を足すことになるので足していない)。C 側では、PE の読み切り(`image_load`)と、ImageBase = 表の base を検査する。失敗すると `fle_stop` で run を名前付きで止める(`real module "<名前>": ...` の理由を付ける)。空の像は作らない。この失敗の経路は実行していない(未確認)。
- 単体の確認: `npm run typecheck` と vitest(68 ファイル、1332 件)が通った。`main_two.c` を emcc(FRUITY あり)と clang(FRUITY なし)で単体にコンパイルした。表の名前の照合を、実際の 10 件の名前で単体テストした(13 件)。`guard/scan.sh` の指摘は 0 件。
- 判定が落ちた理由: 判定は「commit していない変更がある」で落ちた(判定の中では受け入れが走らない)。作業者の受け入れの実行では、(2) が不合格だった(起動前の合計 314,328,003 B。基準の 272,181,853 B を超えた)。(1)(3) は通った。272,181,853 B は #{{015}} の時点の値で、#1345 の後の起動前の取得の合計は、この Issue の変更の前から 314,326,602 B。この Issue の変更で増えたのは 1,401 B だけ。そこで、受け入れ (2) の基準を今の main に合わせて直した(下の「受け入れ」)。#1345 の決定(言語・テーマ・テンプレートは起動前)は変えない。
- 次の試行でやること: 差分を確かめ(`git status`・`git diff HEAD`)、受け入れを通して commit する。C 側で SHA-256 を検査するか(ページ側の検査で足りるとするか)は、作業者が決め、報告に書く。新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数は足さない。

## 読む物

- `lift/harness/two/main_two.c`(104〜175 行: 像の写しと、realmod の検査。171 行付近: seed の表の読み)
- `lift/harness/image.c`(56 行〜: `image_load`)
- `lift/harness/vfs_seed.h`(`vs_run` の 505 行付近: `@lazy:` の行を `vs_read_host` で読む。464 行付近の `vs_put_lazy_file`: `@lazy-file:` の行)、`lift/harness/vfs_lazy.h`(参考。`vl_read`)
- `lift/runtime/fle_realmod.h`(1〜10 行の注釈)、`lift/runtime/fle_realmod.c`、`lift/runtime/fle_load.c`(`LoadLibrary`・`GetModuleHandle` の解決)
- `lift/harness/web/pack.args`・`lift/harness/web/seed.args`(`--lazy-dll`・`--lazy-file`・`--lazy`)
- `app/src/main.ts`(680 行 `startLazy`、762 行の呼び出し)、`app/src/engine/lazy-file.ts`
- `tools/delivery-split/measure.mjs`(起動後の取得の測定。この Issue で `reads` を足す)、`tools/boot-check/boot-bundle.mjs`

## やること

1. 変更の前の起動の経路は、前回の試行で確かめた(上の「前回の試行で分かったこと」)。報告には、その要点を引く。変えた後の 3 回の出力(起動前の取得の一覧と合計、`reads`)は、受け入れの出力として報告に貼る。
2. `main_two.c` の、引数の DLL の写し(`image_load`)を、guest の要求の時まで遅らせる。要求の時に写して、SHA-256 と大きさを検査する。検査が失敗した時の振る舞いを決め、報告に書く。黙って空の像を返さない。seed の複製は、#1345 で既に読みを遅らせてあるので、この Issue では触らない。
3. 遅延の読みごとに、engine のログに 1 行を書く(stderr。環境変数・`#ifdef` は作らず、常に出す)。行の形は `image-read: <名前> requested=<0 か 1> <時刻 ms>`。時刻は起動の開始からの ms(`elapsedMs` と同じ基準。取り方は作業者が決める)。
4. `tools/delivery-split/measure.mjs` に、JSON の項目 `reads` を足す。`reads` は `[{"name","atMs","requested"}]` の配列で、engine のログの `image-read:` 行から作る。`requested` は guest の要求の時が true。`name` は像の名前(例: `fruity_balance_x64.6800000.dll`)。
5. 既定の動作として遅らせる。オプトインを作らない。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/delivery-split/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/feat-lazy.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const sum=(a)=>a.reduce((s,f)=>s+f.bytes,0);const fl=[/fruity.?balance/i,/fruity.?filter/i,/fruity.?delay.?2/i];const lazyOk=(r)=>fl.every((re)=>r.lazy.some((x)=>re.test(x.name)));const ok=L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&lazyOk(r)&&sum(r.preStart)<=314330698&&Array.isArray(r.reads)&&r.reads.every((x)=>!/fruity.?balance/i.test(x.name)||x.requested===true||x.atMs>=r.elapsedMs));process.exit(ok?0:1)' "$RUNNER_TEMP/feat-lazy.jsonl"
```

判定の基準: 3 回の起動で、(1) engine-idle・standIn が false・error が null、(2) Balance・Filter・Delay 2 の像が起動前の取得に無く(起動前の取得の内訳は名前で出ないので、`lazy` の一覧に 3 件とも在ることで確かめる)、かつ起動前の合計(復号後)が 314,330,698 B 以下(この Issue の変更の前の main の値 314,326,602 B に 4,096 B を足した値。判定の基準の数字で、製品の値ではない)、(3) `reads` の Balance の像の読みは、engine-idle の後に起きたか、guest の要求(`requested` が true)で起きたこと。(3) の確かめは、`reads` の欄が配列で在ること(欄が無いと落ちる)が前提。

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

## 作業の進め方

- 前回の試行の差分は、作業ツリーに載っている(`.prior-work.md` が指示する)。取り除かず、残りを行う。
- engine の作り直し(`fe-build engine`)は約 30 分、受け入れの 1 回は約 45 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000)を渡して待つ。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。未 commit の変更が残ると、判定は「commit していない変更がある」で落ちる。commit 済みで受け入れが落ちた場合は、差分が保存され、次の試行が続きから行う。

## 他のタスクとの関係

- #{{015}}(Fruity Balance を起動後の取得に移す)の続き。#{{015}} の結果では、起動は前倒しにならなかった(前回の試行の測定でも、elapsedMs は変わらなかった)。この Issue は、残っている起動の時の DLL の写しを遅らせ、起動前の取得を減らす。
- #1345(言語・テーマ・テンプレートを起動前の seed pack に入れた)の後の main が基準。この Issue は、その決定を変えない。
- #{{102}}(プラグインのコードを engine.wasm から分ける)は、この Issue の遅延の読みを使う。
- #{{014}}(区分の仕組み)と #{{035}}(起動を 10 秒以内にする)が、同じ起動の経路(`app/src/main.ts`・`app/src/engine/client.ts`)を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
