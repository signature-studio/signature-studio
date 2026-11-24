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

## 背景(状況。実測)

- Fruity Filter(`Plugins\Fruity\Effects\Fruity Filter\Fruity Filter_x64.dll`。3,780,608 B。SHA-256 `f4801444224236132b61983b7b81d0d3f89395085a1dc5430653a5edb7273d6c`。ハッシュは `lift/harness/hooks/fruity-delphi-effect-hook-points.txt` の見出しの値。`inputs/plugin-classes.tsv` で確かめる)は、今どうなっているか:
  - `lift/harness/b2d2/modules.tsv` に行が在る(base 0x7000000。gen-all で C に下ろされ、engine.wasm に入る)。
  - `lift/harness/hyb/hyb-args.tsv` に行が無い。分けた wasm になっていない。
  - `lift/harness/web/pack.args` の `--lazy-dll`(`fruity_filter_x64.7000000.dll`)と、`--lazy-file`(seed の行)で、DLL のファイルだけを起動後に取得している(#{{019}} の結果)。
  - `ci/extract-dlls.tsv`・`lift/harness/seeds/vfs-seed-idle1.tsv`・`lift/harness/web/seed.args` に、Filter の行が在る。
- hook 点: `existing`(3 点。RVA 0x17bc0・0x31d70・0x31d90。Balance と同じ)。生成の段が base + RVA で VA を求め、一覧から外す(#{{125}} の (5))。
- Fruity Delay 2 は、この Issue では移さない(共有の表に残る。#{{125}} の (15))。
- `lift/harness/web/fruity/fle_idle_lib.js`・`lift/harness/two/main_two.c`・`app/src/engine/xmod.ts` は、#{{125}} で一般化された物で、この Issue では変えない。

## やること

1. 基盤の commit(追加の前): Fruity Filter の共有の行を消す(`modules.tsv`・`pack.args` の `--args` と `--lazy-dll` と `--lazy-file`・`ci/extract-dlls.tsv`・`vfs-seed-idle1.tsv`・`seed.args` の Filter の行)。`git grep -n -i "fruity.filter" -- ci lift/harness/b2d2 lift/harness/hyb lift/harness/web lift/harness/seeds` で、消し残しが無いことを確かめる(`lift/harness/hooks/` の見出しの文は対象外)。この commit の共有の変更は、この commit に全て含める。報告に、ファイルごとの行数を書く。
2. 追加の commit: `lift/plugins/fruity-filter/plugin.tsv` だけを足す。`key` は `effects.fruity-filter.fruity_filter_x64.dll`、`guest` は `Plugins\Fruity\Effects\Fruity Filter\Fruity Filter_x64.dll`、`role` は `本体`、`hook` は `existing`、`args` は `--pdata`。`sha256` と `size` は上の値。この commit の変更は、`lift/plugins/fruity-filter/` の 1 ファイルだけであること(`git show --stat` で確かめ、報告に書く)。
3. 生成の段が、Filter を分けた wasm(`lift/out-app/xmod/fruity_filter_x64.<生成した base>.wasm`)として作ること。Filter の一覧から、hook 点の VA(base + RVA)が 0 件であること。
4. 測定: `tools/delivery-split/measure.mjs` に、Filter を要求させる場面を足す。場面は次の順に選ぶ: (a) `lift/out-app/host-extra/resources/` の `.flp`(テンプレートとデモ)のうち、`Fruity Filter` の名を含む物(ASCII か UTF-16 のバイト列)を探し、その物を #{{125}} の NewStuff.flp と同じ方法で開く、(b) 無ければ、報告に「Filter を名前で要求する場面が無い」と書き、この Issue を差し戻す(#{{127}} の道具は、この Issue より後なので使えない)。
5. 測定の 3 回で、Balance と Filter の両方の instantiate が `requested: true` で出ること(`reads`)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/delivery-split/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/split.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const B=(a,re)=>a.filter((f)=>re.test(f.name)&&/\.wasm$/i.test(f.name));const ok=L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&r.elapsedMs<=12500&&B(r.lazy,/filter/i).length===1&&B(r.lazy,/balance/i).length===1&&["balance","filter"].every((k)=>r.reads.some((x)=>new RegExp(k,"i").test(x.name)&&/\.wasm$/i.test(x.name)&&x.requested===true)));process.exit(ok?0:1)' "$RUNNER_TEMP/split.jsonl" && ! grep -qi "fruity.filter" lift/harness/b2d2/modules.tsv lift/harness/web/pack.args ci/extract-dlls.tsv lift/harness/seeds/vfs-seed-idle1.tsv lift/harness/web/seed.args && test -f lift/plugins/fruity-filter/plugin.tsv
```

判定の基準:
- 3 回の起動で、(1) engine-idle・standIn が false・error が null、(2) Balance と Filter の wasm が起動後の取得に在る、(3) `reads` に Balance と Filter の wasm の instantiate が `requested: true` で在る、(4) `elapsedMs` が 12500 以下。
- 追加の commit(手順 2)の共有のファイルの変更は 0 行。基盤の commit(手順 1)の共有の変更は、報告に行数を書く。
- 生成の一覧に Filter の hook 点の VA が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 基盤の commit と追加の commit の SHA と、それぞれの変更のファイル一覧、Filter の生成した base と wasm の大きさ、engine.wasm の前後の大きさ、3 回の `elapsedMs`、Filter の場面(使った .flp の名前)。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。Python は製品に使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 他のタスクとの関係

- blocked_by: #{{125}}(プラグインのディレクトリの仕組み)。
- #{{019}}(Filter と Delay 2 の起動後の取得)の結果を、この Issue が置き換える(Filter の部分)。
- #{{127}}(1 本ずつ確かめる道具)は、この Issue と並ばない(#{{127}} も #{{125}} の後)。
