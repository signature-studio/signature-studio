---
draft: "251"
title: "プラグインを足すと、既存のプラグインが `indirect call to 0` で止まる: 溢れている上限を特定して直す"
type: "Task"
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "021"
blocked_by: []
labels: ["plugin-builtin"]
size: "L"
priority: "Urgent"
project: "signature-studio/3"
---

## 目的

内蔵のプラグインを 1 本ずつ足すと、その 1 本は動く。しかし、足す本数を増やすと、既に動いていた `Fruity Balance` と `Fruity Filter` が止まる。どちらも、3 本の plugin.tsv(下の「再現」)を足す前は `processed` が true。足すと、2 本とも落ちる。

この Issue は、どの上限(表・配列・番号)が溢れて、既存の物を壊しているかを特定し、直す。プラグインを減らすことや、足す行を外すことでは直さない。

#1434・#1435・#1436・#1437・#1438・#1439・#1442・#1443・#1444・#1446・#1448 の 11 件は、この Issue を前提にする。これらは、この Issue が閉じるまで起動しない。

## 背景

- 観測 1(#1436 の作業。runner の値。この機械とは別枠): Emphasizer・FL Studio Mobile Rack FX・Frequency Shifter の 3 本を足した木で、既存の `Fruity Balance`・`Fruity Filter` が落ちた。3 本を外した木(ベースライン)では、2 本とも `instantiated` と `processed` が true。3 本を足すと、流れ A〜D も `trapped` になった。FL Studio Mobile Rack FX だけを足した木の停止は `indirect call to 0x59ff676259ff6762 is not a known function entry`(関数の入口でない、ごみの値)。その木では、起動の前に読む量(preBootBytes)も上限を超えた。この値は、この Issue の受け入れの基準にしない(報告に値を書く)。
- 観測 2(#1435 の作業。runner の値): Edison・Effector・Emphasis の 3 本を足した木で、対照の `Fruity Filter` が `processed` false(`state exited after the play`)。Effector と Emphasis は `indirect call to 0 is not a known function entry`。Edison の停止は別の件(`GetClassLongPtrW` の未対応。#{{252}} が扱う)。
- 停止の型: `indirect call to 0 is not a known function entry`。switch 表の飛び先の型(#1413)を直した後も、この型が出る。直接の原因は未確認。観測では、`R[0]` が `0x63736d65`(C++ 例外の経路の疑い。推定)。
- 上限の候補(作業者が読んで確かめる。確認していない):
  - `lift/runtime/fle_vmap.c` の `IH_MAX_MODS`(今は 512)。#1516 で、この上限が 8 だったため、第 2 波の DLL を 3 本足すと起動の前に止まった。この前例がある。
  - `lift/runtime/fle_realmod.c`・`lift/runtime/fle_vmap.c`・`lift/harness/two/main_two.c` の固定長の配列(realmod の数、インラインフックの登録の数、image の領域、dispatch の表、名前表)。
  - module の番号。#1525 の前例では、固定の単位名が、足した plugin の番号とずれた。

## 再現

次の 3 本の `plugin.tsv` を、そのまま `lift/plugins/` に書く(#1435 と #1436 の作業の物)。DLL は `node tools/place-plugin-dlls.mjs` が置く。この 3 本の観測(#1435・#1436)には、`ildsp_x64.dll` の読み込みの失敗が無い。`ildsp_x64.dll` を読むのは Emphasizer と Frequency Shifter で、#{{253}} が扱う。

`lift/plugins/effector/plugin.tsv`:

```tsv
# Effector (Effects, Issue #1435). 本体 Effector_x64.dll; 補助 engine_x64.dll (the engine the 本体 names: no Delphi hook points). hook: the three Delphi hook points of the RVAs (inputs/plugin-hooks.tsv). omit: - (the setjmp entries come from fe-decode, #1370).
key	guest	role	sha256	size	hook	args	omit
effects.effector.effector_x64.dll	Plugins\Fruity\Effects\Effector\Effector_x64.dll	本体	8d437239007f9532dc9eeddac960d8555004cb14e5c75b700f4774601f3a18bc	3726336	rva:0x17980,0x31a10,0x31a30	--pdata	-
effects.effector.engine_x64.dll	Plugins\Fruity\Effects\Effector\engine_x64.dll	補助	6b0c7c25e4eeb91a91b9e93b6113e909031d390711e7045faace68565357172c	498176	none	--pdata	-
```

`lift/plugins/emphasis/plugin.tsv`:

```tsv
# Emphasis (Effects, Issue #1435). 本体 Emphasis_x64.dll; no 補助. hook: the three Delphi hook points of the RVAs (inputs/plugin-hooks.tsv). omit: - (the setjmp entries come from fe-decode, #1370).
key	guest	role	sha256	size	hook	args	omit
effects.emphasis.emphasis_x64.dll	Plugins\Fruity\Effects\Emphasis\Emphasis_x64.dll	本体	dc48d5710b5c2cf75bfd109d108311ac773322ca5799c53fc3a0910db60d2063	4142080	rva:0x191e0,0x33860,0x33880	--pdata	-
```

`lift/plugins/fl-studio-mobile-rack-fx/plugin.tsv`:

```tsv
# FL Studio Mobile Rack FX (Effects, 1 DLL). hook: none (inputs/plugin-hooks.tsv has no row for it). omit: - (the setjmp entries are found by fe-decode, run_list).
key	guest	role	sha256	size	hook	args	omit
effects.fl-studio-mobile-rack-fx.fl_studio_mobile_rack_fx_x64.dll	Plugins\Fruity\Effects\FL Studio Mobile Rack FX\FL Studio Mobile Rack FX_x64.dll	本体	8c43108b377cbcee5dc0abfc2c59b09903f108b1291459a12c64bfa8bcbe737e	8228864	none	--pdata	-
```

再現の手順:
1. 上の 3 本を書き、受け入れの手順のとおりに `place-plugin-dlls`・`fe-build engine`・`fe-build bundle` を行う。
2. `node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "Fruity Balance" "Fruity Filter"` を実行する。3 本を外した木では 2 行とも `processed` が true(#1436 の観測)。3 本を足した木では、`processed` が false になる(同じ観測)。

## やること

1. 再現する。3 本を足した木で、`Fruity Balance`・`Fruity Filter` の JSON 行と、engine の末尾の行を、報告に貼る。再現しなければ、その出力を報告して止まる。
2. 読みで候補を絞る。次の grep で、固定長の表と上限を読む。
   `grep -rnE 'MAX|\[[0-9]+\]' lift/runtime/fle_realmod.c lift/runtime/fle_vmap.c lift/harness/two/main_two.c | head -60`
   溢れている表が特定できたら、表の名前・上限・足した数を、報告に書く。
3. 足す行の数の実測(直すためではなく、どの数から既存が落ちるかを確かめるため)。3 本から行を減らした木(0・1・2・3 本)で、既存の 2 本が落ちる境目を見る。ビルド 1 回は数十分かかるので、読みで絞った候補に必要な回数だけ行う。行えなかった数は「未確認」と書く。直した後の木には、3 本を入れたままにする。
4. 直す。溢れている表・配列の大きさを、第 2 波の全ての DLL が入る大きさにする。上限を超えたときは、既存の物を黙って壊さず、表の名前と上限を言って止まる(例: `fle: <表の名前> is full (<上限>)`)。上限は定数のままにし、オプションにしない。
5. 試験を先に赤にする。`lift/crates/fe-lift/tests/plugin_table_capacity.rs`(binary 名 `plugin_table_capacity`)に、直した上限の手前と先の構成を確かめる試験を書く。今の木では、上限を超える構成が既存の物を壊すので赤になる。試験の作り方は作業者が選ぶ。
6. 直した後も 3 本を入れたままで、受け入れを通す。既存の 2 本が `processed` true であることを確かめる。

## 受け入れ

```accept
test -f lift/plugins/effector/plugin.tsv && test -f lift/plugins/emphasis/plugin.tsv && test -f lift/plugins/fl-studio-mobile-rack-fx/plugin.tsv && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && node tools/place-plugin-dlls.mjs && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "Fruity Balance" "Fruity Filter" > "$RUNNER_TEMP/plugin-run.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===2&&L.every((r)=>r.instantiated===true&&r.error===null&&r.processed===true)?0:1)' "$RUNNER_TEMP/plugin-run.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && node tools/fe-lift-test.mjs --test plugin_table_capacity && bash guard/scan.sh
```

判定の基準:
- 3 本の `lift/plugins/*/plugin.tsv` が在り、この Issue の commit に含まれる。
- 起動確認(`boot.json`)が engine-idle・standIn が false・error が null・elapsedMs が 20,000 以下。
- `Fruity Balance`・`Fruity Filter` の 2 行が、`instantiated` が true・`error` が null・`processed` が true。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。`node tools/fe-lift-test.mjs --test plugin_table_capacity` が終了コード 0。
- `bash guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 再現の出力(`Fruity Balance`・`Fruity Filter` の JSON 行と、engine の末尾の行)、(b) 足す数 0・1・2・3 の結果(観測できなかった数は「未確認」)、(c) 溢れていた表・配列の名前と上限、直し方とその理由、(d) 直した後の `plugin-run` の 2 行、(e) 起動確認の `elapsedMs`、流れ A〜D の 4 行、preBootBytes の値、(f) `plugin_table_capacity` の結果、(g) `bash guard/scan.sh` の結果。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。上限は定数のままにし、オプションにしない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・変数名は書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- `git add` は、変えたファイルを 1 つずつ指定する(`git add -A`・`git add .` は使わない)。
- 3 本の `plugin.tsv` は、この Issue の commit に含める(判定は commit した木で行う)。
- `lift/` に触れるので、判定が通って push されると、main の `deploy.yml` が動く。報告に書く。

## 作業の進め方

- 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
- ビルド(`fe-build engine`)は、1 回に数十分(観測: 約 17 分から 38 分)。この Issue は、再現と、直した後の確認で 2 回を標準とする。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- 作業を始めてから 3 時間 15 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(判定に約 2 時間掛かる)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- #1434・#1435・#1436・#1437・#1438・#1439・#1442・#1443・#1444・#1446・#1448 は、この Issue を前提にする。
- 同じ束の #{{252}}(runtime に無い import の実装)と #{{253}}(共有の DLL の行)と、同じファイルを変えることがあり得る。衝突は、後から入る方が直す。
