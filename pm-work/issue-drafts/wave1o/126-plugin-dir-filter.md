---
draft: "126"
title: "2 つ目のプラグイン(Fruity Filter)を、lift/plugins/fruity-filter/ の追加だけで足せることを確かめる(共有のファイルの変更は 0 行)"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["125"]
labels: ["delivery", "plugin-builtin"]
size: M
priority: High
project: signature-studio/3
---

## 目的

#{{125}} で、Fruity Balance は、`lift/plugins/fruity-balance/plugin.tsv` だけで足せる形になった。この Issue は、2 つ目のプラグイン(Fruity Filter)を同じ形に移し、その追加が、共有のファイルの変更 0 行で済むことを、実物で確かめる。

前回の試行では、Filter の生成と配信までは動いた。しかし、Fruity プラグインの読み込みの後に engine が止まり、Balance と Filter を別々の起動で要求する形になった。この試行では、その停止の原因(imm32.dll の読み込みの失敗)を確かめ、原因なら直す。

## 背景(状況。実測)

- Fruity Filter(`Plugins\Fruity\Effects\Fruity Filter\Fruity Filter_x64.dll`。3,780,608 B。SHA-256 `f4801444224236132b61983b7b81d0d3f89395085a1dc5430653a5edb7273d6c`。ハッシュは `lift/harness/hooks/fruity-delphi-effect-hook-points.txt` の見出しの値。`inputs/plugin-classes.tsv` で確かめる)は、今どうなっているか:
  - `lift/harness/b2d2/modules.tsv` に行が在る(base 0x7000000。gen-all で C に下ろされ、engine.wasm に入る)。
  - `lift/harness/hyb/hyb-args.tsv` に行が無い。分けた wasm になっていない。
  - `lift/harness/web/pack.args` の `--lazy-dll`(`fruity_filter_x64.7000000.dll`)と、`--lazy-file`(seed の行)で、DLL のファイルだけを起動後に取得している(#{{019}} の結果)。
  - `ci/extract-dlls.tsv`・`lift/harness/seeds/vfs-seed-idle1.tsv`・`lift/harness/web/seed.args` に、Filter の行が在る。
- hook 点: `existing`(3 点。RVA 0x17bc0・0x31d70・0x31d90。Balance と同じ)。生成の段が base + RVA で VA を求め、一覧から外す(#{{125}} の (5))。
- Fruity Delay 2 は、この Issue では移さない(共有の表に残る。#{{125}} の (15))。
- `lift/harness/web/fruity/fle_idle_lib.js`・`lift/harness/two/main_two.c`・`app/src/engine/xmod.ts` は、#{{125}} で一般化された物で、この Issue では変えない。

## 前回の試行で分かったこと(run 45505986680)

- 判定は差し戻し。Balance の場面は通った。Filter の場面は `exited`(`ExitProcess`。終了コード 0)で、判定 (3) が不通。作業は 3 つの commit(`13372a12` 基盤・`8695b23e` imm32・`4cdd0985` 追加。作業者の環境。push していない。work.diff に保存された)。runner の値。nproc 4、測定の後の loadavg 3.08 2.06 2.06。
- 基盤(共有の行の削除。共有は計 +1 −10 行): `ci/extract-dlls.tsv` +0−1、`lift/harness/b2d2/modules.tsv` +0−1、`lift/harness/seeds/vfs-seed-idle1.tsv` +0−1、`lift/harness/web/pack.args` +1−5、`lift/harness/web/seed.args` +0−2。`tools/delivery-split/measure.mjs` は +59 −13(場面の交互化)。
- 追加: `lift/plugins/fruity-filter/plugin.tsv` の 1 ファイル。前回の index の plugin.tsv は 7 列で、パーサ(8 列)に合わなかったため、`omit` 列 `-` を足した。`git grep -i "fruity.filter" -- ci lift/harness/b2d2 lift/harness/hyb lift/harness/web lift/harness/seeds` は 0 件。
- imm32(決定 0015 の見直し): `imm32.dll` を仮想モジュール(N = 43)にし、FLEngine と Fruity の DLL が遅延 import する 7 関数(ImmSetOpenStatus・ImmSetConversionStatus・ImmGetConversionStatus・ImmAssociateContextEx・ImmReleaseContext・ImmGetContext・ImmIsIME)を、IME の無い Windows と同じ答えにした(`ImmGetContext` は NULL、BOOL は 0)。変更は `fle_modules.c`・`fle_load.c`・`fle_win.c`・`fle_win.h`・`fe-lift/src/imports_table.rs`。
- 結果(3 回の起動。場面は Balance・Filter・Balance。`measure.mjs`):
  - 1 回目 Balance(NewStuff.flp): `elapsedMs` 11,406.2。`fruity_balance_x64.c1d0000.wasm` が `requested=true`。場面の後は `engine-idle`。
  - 2 回目 Filter(Exoplanet.flp): `elapsedMs` 11,004.1。`fruity_filter_x64.c580000.wasm` が `requested=true`。場面の後は `exited`。
  - 3 回目 Balance(NewStuff.flp): `elapsedMs` 11,180.0。`requested=true`。場面の後は `engine-idle`。
  - 起動確認(`boot-bundle.mjs`): `engine-idle`・`standIn` false・`error` null(`timelineIdle` 12,176.7 ms。判定の対象外)。
  - 前回(run 45452762437)の `LoadLibraryW 126 "imm32.dll"` の後の `ExitProcess` は、Balance の場面では出なくなった。ただし、成功した読み込みはログに出ないため、直接の確認ではない。
- 生成: `plugins: 2 DLLs; bases 0xc1d0000 0xc580000`(Balance は変わらず、Filter は固定の 0x7000000 から 0xc580000 へ)。Filter の wasm は 11,315,994 B(DLL 3,780,608 B)。`emitted ok 12283, left out 3 (omit 0), listed 12280`。hook 点の VA(0xc597bc0・0xc5b1d70・0xc5b1d90)は一覧に 0 件。
- `engine.wasm`: 165,623,930 B(変更の前)→ 158,407,386 B(−7,216,544 B、−4.36%)。前回の試行の 158,399,383 B より +8,003 B(imm32 の行)。hybrid: 8 direct modules、2 separate modules。
- Filter の場面の停止の順序: `image-read: fruity_filter_x64.c580000.wasm requested=1` → DllMain(attach。`reason=1`)→ `FindFirstFileW` が `Shared\FruityPlug_x64.en-US`・`.en`・`.ENU`・`.EN` の 4 件を探す(いずれも無い)→ `ExitProcess`。Balance も同じ 4 件を探すが、その後は止まらない。スレッド通知(`reason` 2・3)は無関係(Balance は受けても止まらない)。
- 静的解析(objdump・Python。一時ファイルは消した): Filter の AddressOfEntryPoint は VA 0x6cdf20(イメージのベース 0x400000 なので RVA 0x2cdf20)。DllMain は `setjmp`(VA 0x40e120。両方の DLL で同じ番地)の結果が 0 なら本体(VA 0x4189c0)を呼び、戻ると `Halt`(VA 0x40e270)を呼ぶ。`Halt` は、グローバル `[0x721cb0]` が 0 なら `ExitProcess`、0 でなければ DllMain へ戻る。`ExitProcess` の呼び出しは、Filter の DLL で `Halt` 経由の 1 箇所だけ。`[0x721cb0]` は、Filter の DLL でも Balance の DLL でも書かれていない。
- 見立て(PM。実測で確かめる): これは #{{128}} で直した Balance の問題と同じ型。Delphi の DllMain が `setjmp` を呼ぶ関数は、直接 wasm に lift せず、C で lift して、着地点(`fle_landing_push`。`lift/runtime/fle_dispatch.h`)を登録する必要がある。Balance は `plugin.tsv` の `omit` 列(RVA 0x2c13a0)で外している。Filter の `plugin.tsv` には `omit` が無いので、同じ型の関数が直接 wasm に入り、着地が無いまま `Halt` → `ExitProcess` に落ちる、と見られる。
- 判定の確認: `spec/guard.test.sh`(67 件)・`spec/workflow.test.sh`(443 件)が通る。`lift/crates/fe-lift/tests/`(load.rs・wall_fr6.rs・wall_fr5.rs・msctf.rs)に、imm32 を「無い」とする前提の assert が残っている(やること 9)。

## その前の試行で分かったこと

(run 45452762437。failed。作業は work.diff として保存された。commit は、この main に無い。次の試行は、その差分を main の上に取り込んで続ける。)

- 作業の形: 基盤の commit(共有の行の削除。5 ファイル。+1 −10)、Filter の `plugin.tsv`(1 ファイル。+2)、測定の道具(`tools/delivery-split/measure.mjs`)。
- 生成(実測): `plugins: 2 DLLs; bases 0xc1d0000 0xc580000`。Balance の base は変わらず、Filter は固定の 0x7000000 から 0xc580000 になった。Filter の wasm は 11,418,352 B(DLL 3,780,608 B)。一覧は `listed 12280, emitted ok 12280`。hook 点の VA(0xc597bc0・0xc5b1d70・0xc5b1d90)は、一覧にも lowered-all にも 0 件。
- engine.wasm: 165,623,930 B(変更の前の 450420bc)→ 158,399,383 B(−7,224,547 B、−4.36%)。hybrid: 8 direct modules、2 separate modules。
- 場面(run ごとに交互。1 回目 Balance、2 回目 Filter、3 回目 Balance):
  - 1 回目(Balance。`NewStuff.flp`、313,356 B): 起動が `state running` のまま 180 秒止まり、場面に入れなかった。
  - 2 回目(Filter。`Exoplanet.flp`、950,527 B。Filter の名を 20 回含む): `engine-idle`(elapsedMs 11137.4)。Filter の wasm が `requested=true`。場面の後の state は `exited`。
  - 3 回目(Balance): `engine-idle`(elapsedMs 10828.9)。Balance の wasm が `requested=true`。場面の後の state は `exited`。
- 停止の形: Fruity プラグインの要求の後で、`LoadLibraryW 126 "imm32.dll"` が出て、ゲストが `ExitProcess`(終了コード 0)を呼ぶ。runtime は `stopped here, not exited` を出し、終了コード 134 で止まる。観測は 3 件とも同じ: Asher(Balance だけを要求)、Exoplanet(Filter だけを要求)、NewStuff(Balance だけを要求)。NewStuff は、変更の前の 450420bc でも同じ。`lift/runtime/fle_load.c` の決定 0015 は、この環境に imm32 が無いと決めている。
- 同じ起動で Balance と Filter を両方要求させる試みは、上の停止のため、この決定の下では成功しなかった。
- 起動の停止(変更の前から在る): 450420bc の診断で 20 回中 2 回(10%)。変更の後の 20 回の診断では 0 回。#{{149}}(起動の間欠の停止。完了 22:28Z)で直ったとされる。この試行の後の出来事で、この Issue では未確認。
- 場面の材料: この runner には `lift/out-app/host-extra/resources/` が無い。installer の木 `lift/out-app/host/resources/` を使った(seed の行と SHA-256 が一致する)。Filter の名を含む .flp は、ASCII では見つからず、UTF-16LE で 5 件。Exoplanet.flp は、Balance と Filter の両方の名を含む。
- 受け入れは通らなかった(差し戻し)。停止の原因(imm32)は、この Issue の範囲で確かめる(下の「やること」5)。

## 読む物

- `lift/runtime/fle_load.c`(決定 0015 の注。`LoadLibraryW` の答え)・`lift/runtime/fle_modules.c`(仮想のモジュールの表)
- `lift/plugins/fruity-balance/plugin.tsv`(#{{125}} の形)・`lift/plugins/`(この Issue で足す Filter の行の形)
- `tools/delivery-split/measure.mjs`(場面の交互。`scene`)

## やること

1. **基盤の commit(追加の前)**: Fruity Filter の共有の行を消す(`modules.tsv`・`pack.args` の `--args` と `--lazy-dll` と `--lazy-file`・`ci/extract-dlls.tsv`・`vfs-seed-idle1.tsv`・`seed.args` の Filter の行)。`git grep -n -i "fruity.filter" -- ci lift/harness/b2d2 lift/harness/hyb lift/harness/web lift/harness/seeds` で、消し残しが無いことを確かめる(`lift/harness/hooks/` の見出しの文は対象外)。この commit の共有の変更は、この commit に全て含める。報告に、ファイルごとの行数を書く。
2. **追加の commit**: `lift/plugins/fruity-filter/plugin.tsv` だけを足す。`key` は `effects.fruity-filter.fruity_filter_x64.dll`、`guest` は `Plugins\Fruity\Effects\Fruity Filter\Fruity Filter_x64.dll`、`role` は `本体`、`hook` は `existing`、`args` は `--pdata`。`sha256` と `size` は上の値。この commit の変更は、`lift/plugins/fruity-filter/` の 1 ファイルだけであること(`git show --stat` で確かめ、報告に書く)。
3. **生成**: 生成の段が、Filter を分けた wasm(`lift/out-app/xmod/fruity_filter_x64.<生成した base>.wasm`)として作ること。Filter の一覧から、hook 点の VA(base + RVA)が 0 件であること。
4. **測定の場面**: `tools/delivery-split/measure.mjs` は、run ごとに場面を交互に使う(1 回目 Balance、2 回目 Filter、3 回目 Balance)。Balance の場面は NewStuff.flp(#{{102}} の場面)。Filter の場面は、`host-extra/resources`(無ければ `host/resources`)の .flp のうち、Fruity Filter の名を最も多く含む物(ASCII と UTF-16LE の合計。同数なら、パス順の先)。前回の試行では Exoplanet.flp(20 回)が選ばれた。
5. **imm32.dll の確認(依頼者の決定)**: 前回の試行の停止(Fruity プラグインの要求の後の `LoadLibraryW 126 "imm32.dll"` と `ExitProcess`)が、Fruity プラグインの読み込みの後の停止の原因かを確かめる。確かめ方: engine のログで、126 の後の呼び出しと `ExitProcess` の経路を追う。
   - 原因なら、この Issue で直す。Windows に在る imm32.dll を runtime が提供する。IME が無効の Windows と同じ答えにする(`ImmGetContext`・`ImmReleaseContext`・`ImmAssociateContext`・`ImmGetCompositionStringW` など。FL が引く関数に限る。NULL か 0 を返す)。決定 0015(`lift/runtime/fle_load.c` の注)を見直し、理由を本文と報告に書く。これは第 2 波(全プラグイン)の共通の障害なので、ほかの Issue に送らない。
   - 原因でなければ、その根拠を報告に書いて止まる(直し方は PM が決める)。
6. **受け入れ**: 3 回の起動で、Balance の場面と Filter の場面が、それぞれ `requested=true` で、その後も engine が `engine-idle` のまま(`ExitProcess` で止まらない)こと。両方を 1 回の起動で要求できる場面が見つかれば、報告に書く(受け入れは場面ごとに判定する)。
7. **共有のファイルの変更は 0 行**(Filter の追加の commit)。
8. **Filter の停止(PM の指示。#{{128}} と同じ型)**: 上の「前回の試行で分かったこと」の見立てを、実測で確かめ、直す。
   - ① `lift/plugins/fruity-filter/plugin.tsv` の `omit` 列に、DllMain の入口の RVA(0x2cdf20。VA 0x6cdf20 から 0x400000 を引いた値。**要確認**)を足して試す。`omit` の値は RVA(`lift/crates/fe-build/src/plugins.rs` の `rvas_or_none`。16 進のカンマ区切り)。
   - ② 第 2 波(約 45 の内蔵プラグイン。同じ Delphi の形)に備え、`omit` を手で書かず、`setjmp`(VA 0x40e120)を呼ぶ関数から機械的に決める。既存の仕組みを確かめて、使う物を選ぶ(候補: `lift/crates/fe-decode/src/setjmp.rs` の `landings`。`lift/crates/fe-lift/src/lib.rs` の 1186 行付近で既に使われている。または `fe-wasmgen` の報告)。
   - ③ 手で書いた `omit`(Balance の 0x2c13a0)と、自動の結果が一致することを、`cargo test -p fe-build` で確かめる(Balance の自動の結果は 0x2c13a0 を含む)。自動で決まるなら、`omit` 列は「自動の結果に足す手の指定」の意味にする。
9. **fe-lift の試験の更新(PM の指示)**: `lift/crates/fe-lift/tests/`(load.rs・wall_fr6.rs・wall_fr5.rs・msctf.rs)の、imm32 を「無い」とする前提の assert を、今回の変更に合わせて直す。`cargo test -p fe-lift` の通過件数を、変更の前後で報告に書く(新しい失敗は 0 件)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/delivery-split/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/split.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const B=(a,re)=>(a||[]).filter((f)=>re.test(f.name)&&/\.wasm$/i.test(f.name));const req=(r,re)=>(r.reads||[]).some((x)=>re.test(x.name)&&/\.wasm$/i.test(x.name)&&x.requested===true);const ok=(r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&r.elapsedMs<=12500&&!!r.scene&&r.scene.error===null&&r.scene.state==="engine-idle"&&B(r.lazy,/balance/i).length===1&&B(r.lazy,/filter/i).length===1&&req(r,r.scene.name==="Fruity Filter"?/filter/i:/balance/i);process.exit(L.length===3&&L.every(ok)&&new Set(L.map((r)=>r.scene.name)).size===2?0:1)' "$RUNNER_TEMP/split.jsonl" && ! grep -qi "fruity.filter" lift/harness/b2d2/modules.tsv lift/harness/web/pack.args ci/extract-dlls.tsv lift/harness/seeds/vfs-seed-idle1.tsv lift/harness/web/seed.args && test -f lift/plugins/fruity-filter/plugin.tsv
```

判定の基準:
- 3 回の起動(場面は run ごとに Balance・Filter・Balance):
  - (1) engine-idle・standIn が false・error が null。
  - (2) Balance と Filter の wasm が起動後の取得(`lazy`)に在る。
  - (3) 場面の wasm の instantiate が `reads` に `requested: true` で在り、場面の後も `scene.state` が engine-idle(`ExitProcess` で止まらない)。
  - (4) `elapsedMs` が 12500 以下。
  - 3 回のうち、Balance の場面と Filter の場面が両方在る。
- 追加の commit(手順 2)の共有のファイルの変更は 0 行。基盤の commit(手順 1)の共有の変更は、報告に行数を書く。
- 生成の一覧に Filter の hook 点の VA が 0 件。
- imm32 の確認(手順 5)の結果が報告に在る(原因か否か。直したなら、決定 0015 の変更の内容)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 基盤の commit と追加の commit の SHA と、それぞれの変更のファイル一覧、Filter の生成した base と wasm の大きさ、engine.wasm の前後の大きさ、3 回の `elapsedMs`、場面ごとの使った .flp の名前と `reads` の `requested`、scene の終わりの state、imm32 の確認の結果。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。Python は製品に使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。
- 名前の扱い(依頼者の訂正): ファイルに書いてはならないのは、AI のベンダー名と、Cloudflare の識別子だけ(`guard/scan.sh` が検査する)。FL の製品名(FL Studio など)・`Image-Line` のインストール先の名前・`%FLStudioFactoryData%` などの変数名は、書いてよい。

## 作業の進め方

- ビルド(`fe-build engine`)は約 35 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000 にする。背景で起動して待たない。
- 受け入れの全体は約 60 分。判定は commit の後に同じ job で、もう一度全部走る。job の上限は 5 時間 50 分。
- 作業の開始時に `date -u` で時刻を記録する。3 時間 30 分を過ぎたら、新しいビルドを始めない。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- 1 回の試行に収まらない場合は、差分を commit して終える。次の試行は、その上から続ける。

## 他のタスクとの関係

- blocked_by: #{{125}}(閉じた。プラグインのディレクトリの仕組みは main に在る)。
- #{{019}}(Filter と Delay 2 の起動後の取得)の結果を、この Issue が置き換える(Filter の部分)。
- #{{127}}(1 本ずつ確かめる道具)は、この Issue と並ばない(#{{127}} は、この Issue の後)。
