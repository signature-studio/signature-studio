---
draft: "152"
title: "ゲストの wasm の関数名の表を、全ての関数に付ける: 名前の無い 12,770 件を無くす(プロファイルの未分類を 5% 以下にする)"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: []
labels: ["perf", "verify"]
size: S
priority: Medium
project: signature-studio/3
---

## 目的

エンジンの重さを計測する道具(`tools/engine-cost/`。#1388 の段階 0)で、プロファイルの時間の一部が「未分類」になり、区分(A〜F)の割合が決まらない。原因は、ゲストの wasm の関数名の表(`lift/out-app/gen/wasm-func-names.tsv`)に、名前の無い行が 12,770 件あることである。この Issue で、全ての関数に名前を付け、未分類を 5% 以下にする。製品の動きは変えない。

## 背景

計測の結果(`pm-work/findings/engine-cost-attribution.md`・`.json`。#1388 の段階 0。製品の `-Oz` の engine):

- 分類率(`classifiedShare`)は、場面 (i) で 0.918〜0.928、場面 (ii) で 0.821〜0.864、場面 (iii) で 0.966〜0.969。全 run で 0.95 以上になっていない(18 run のうち 12 件が届かない)。
- 未分類の大半は、名前の無い wasm 関数の時間(`unnamedWasmMs`)。場面 (i) で 189〜272 ms、場面 (ii) で 1,072〜1,208 ms、場面 (iii) で 361〜424 ms。未分類の 98〜99.7% を占める。
- 名前の無い行は、表の `kind` の欄が `-` で、名前の欄が空の行。表は `index`・`kind`・`va`・`name` の 4 列。kind は `import`・`f`(lift 由来の `f_<VA>`)・`ia`(`ia_<VA>`)・`c`(C 関数の記号)・`-`(名無し)のいずれか。
- 出荷する `lift/out-app/web/engine.wasm` には name セクションが無い(`lift/crates/fe-wasmgen/src/hybrid.rs` の `without_names` が外している)。だから wasm の中からは名前を引けない。名前を持つのは表(TSV)だけ。
- 表は `lift/crates/fe-wasmgen/src/hybrid.rs` の `names_table` が作る。`lift/crates/fe-build/src/main.rs` の hybrid の段が、`hyb/wasm-func-names.tsv` を `gen/wasm-func-names.tsv` にコピーする。変種(O2・O3)の表は `tools/engine-cost/build-variant.mjs` がコピーする。
- 区分の規則は `tools/engine-cost/measure.mjs` の `classify`。名前の無い行は未分類に入る。

## 読む物

- `lift/crates/fe-wasmgen/src/hybrid.rs`(`names_table`・`function_names`・`without_names`・名前の型を決める `classify` 相当の関数)
- `lift/crates/fe-build/src/main.rs`(hybrid の段。`hyb/` から `gen/` へのコピー)
- `lift/out-app/hyb/wasm-func-names.tsv` と `lift/out-app/gen/wasm-func-names.tsv`(ビルドで出る。名前の無い行の番号の並びを見る)
- `tools/engine-cost/measure.mjs`(表の読みと分類・`classifiedShare`)・`tools/engine-cost/run.sh`・`tools/engine-cost/build-variant.mjs`
- `pm-work/findings/engine-cost-attribution.md`・`.json`(基準の値)
- `tools/smoke/welcome-flow.mjs`(流れ A〜D)・`tools/boot-check/boot-bundle.mjs`(起動の確認)

## やること

1. 原因の特定。名前の無い 12,770 行が、どの段で名前を失うかを、段ごとの表で確かめる(hybrid が表を作る時点で name セクションに無いのか、結合(merge)・blend2d・分けた wasm(`--separate`)の段で消えるのか)。原因を報告に書く。
2. 直す。全ての関数に名前を付ける。
   - 名前は表(TSV)にだけ出す。出荷する wasm の大きさを増やさない。name セクションを出荷の wasm に入れない。
   - 名前が元々無い関数は、`unnamed_<index>` の名前にし、kind は `c`(C 関数)にする(区分は D に入る)。ただし、`f_<VA>` の形で付けてよいのは、その関数が本当に lift 由来のコードと分かる時だけ(間違った区分にしない)。
   - 製品の動き(engine の振る舞い)は変えない。名前は表の値だけ。
3. 計測の道具に、短い確認の入口を足す。`tools/engine-cost/run.sh --quick` を足す。場面 (i)・(ii)・(iii) を、DPR 1.00・n=1 で、1 回ずつ、窓を 30 秒にして測る。`--quick` は集計の段(`aggregate`)を行わない(`pm-work/findings/engine-cost-attribution.*` を書き換えない)。run.sh は既存の結果を飛ばすので、測る前に `lift/out-app/engine-cost/runs` を消す。
4. 受け入れの確認(下の受け入れのコマンドを、このまま実行する)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > .tmp-152-boot.json && node -e 'const r=JSON.parse(require("fs").readFileSync(".tmp-152-boot.json","utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' && test "$(awk -F'\t' 'NR>1 && $2=="-"' lift/out-app/gen/wasm-func-names.tsv | wc -l)" -eq 0 && rm -rf lift/out-app/engine-cost/runs && bash tools/engine-cost/run.sh --quick && node -e 'const fs=require("fs");const d="lift/out-app/engine-cost/runs/Oz";const R=fs.readdirSync(d).filter((f)=>f.endsWith(".json")).map((f)=>JSON.parse(fs.readFileSync(d+"/"+f,"utf8")));const sz=fs.statSync("lift/out-app/web/engine.wasm").size;process.exit(R.length===3&&R.every((r)=>r.error===null&&r.classifiedShare>=0.95)&&sz<=165619829+16384?0:1)' && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > .tmp-152-flow.jsonl && node -e 'const L=require("fs").readFileSync(".tmp-152-flow.jsonl","utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)'
```

判定の基準:
- 名前の無い行(表の kind が `-`)が 0 件。
- `run.sh --quick` の 3 run(場面 i・ii・iii)の全てで、`error` が null、`classifiedShare` が 0.95 以上。
- `lift/out-app/web/engine.wasm` の大きさが、基準 165,619,829 B(`pm-work/findings/engine-cost-attribution.json` の `optLevels` の Oz の `wasmBytes`)+ 16,384 B 以下。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`)の 4 行が全て、`error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。流れ B と D は `welcomeClosed` が true。各行の `elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 B 以下。

## 作業の進め方

- engine のビルドは約 35 分。フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000。上限 5400000)を渡して、直接実行する。背景で起動して、終わるのを待たない。
- ビルドが 403 で落ちた(blend2d の取得)ら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業は開始から 5 時間以内に終える。開始時刻を `date -u` で最初に記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit して、報告を書いて終える。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する(commit しないと判定が落ちる)。

## 報告

- 原因の特定(どの段で名前が失われていたか、件数の内訳)、直した方式と理由。
- 受け入れのコマンドの出力を貼る(`--quick` の 3 run の `classifiedShare`、`engine.wasm` の大きさ、流れ A〜D の 4 行)。
- 「達成」と書かず、「受理」か「差し戻し」かを、出力の行で書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- Python を使わない(調査と検証にも、名前の表の処理にも使わない)。OCR を使わない。
- ベンダー名・Cloudflare の識別子・内部の ID・URL を、ファイルと報告に書かない。
- sleep・timeout コマンド・nohup・末尾の `&` を使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。ビルドと測定は、作業ジョブの中で行う。
- `lift/` の変更は、push で main の `deploy.yml` を動かす。報告に書く。

## 他のタスクとの関係

- blocked_by: なし。
- `#1388`(段階 0)は閉じた。その道具と記録は `tools/engine-cost/` と `pm-work/findings/engine-cost-attribution.*` にある。
