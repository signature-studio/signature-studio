---
draft: "102"
title: "プラグインのコードを engine.wasm から分け、Fruity Balance を 1 プラグイン 1 wasm で起動後に取得する(最初の分割)"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["101"]
labels: ["delivery", "plugin-builtin"]
size: XL
priority: High
project: signature-studio/3
---

## 目的

内蔵のプラグインのコードを、engine.wasm から分ける。1 プラグイン 1 wasm にして、起動後に取得し、guest の要求の時に instantiate する。最初の 1 件は Fruity Balance。

今のまま内蔵のプラグインを足し続けると、単一の wasm が、ブラウザが読み込める上限(約 1GB。依頼者の指示)を超える見込みで、起動(P1 の 10 秒)にも効く。第 2 波(内蔵の約 57 件の束ね)の前に、この分割の仕組みが要る。

## 背景

- 今の構造: 10 個のモジュール(FLEngine など)は、`fe-build` の `gen-all`(`lift/crates/fe-build/src/main.rs` の `plan` の gen 段。引数は `lift/harness/b2d2/gen.args`)で、1 つの wasm(`lift/out-app/web/engine.wasm`。168,996,523 B)に連結される。harness(`lift/harness/two/main_two.c`)は、引数の全ての DLL を起動の時に 1 つのメモリに写す。
- 内蔵のプラグインの登録: `lift/harness/b2d2/modules.tsv`(8〜10 行。Balance は 8 行目。base は `0x6800000`。hook 点は `lift/harness/hooks/fruity-delphi-effect-hook-points.txt`)。
- #{{101}} の結果を使う。起動の経路は、像の読みを guest の要求の時まで遅らせる形になる。
- 前の作業の結果(#{{015}}。runner の値): Balance の DLL は起動後の取得に移したが、Balance のコードは engine.wasm に入ったまま(168,996,523 B。前後で同じ)。取得を移しても、wasm の大きさは変わらない。
- 方針(依頼者の指示): 1 プラグイン 1 wasm。Worker の上限に当たる場合を除く。難しいと判断した場合は、方針を変えてよい。変える時は、実測の根拠を報告に書く。

## 読む物

- `lift/crates/fe-build/src/main.rs`(231〜260 行: `plan` の gen 段。459〜475 行: `pack_step_for`・`bundle`)
- `lift/harness/b2d2/gen.args`、`lift/harness/b2d2/modules.tsv`(8 行: Balance)、`lift/harness/web/pack.args`
- `lift/harness/two/main_two.c`、`lift/harness/image.c`
- `lift/runtime/fle_realmod.c`・`lift/runtime/fle_realmod.h`、`lift/runtime/fle_load.c`
- `lift/harness/hooks/fruity-delphi-effect-hook-points.txt`(hook 点の形。Balance・Filter・Delay 2 の RVA)
- `app/scripts/pack-assets.mjs`(bundle の作り)
- `app/src/engine/fruity.ts`(DLL の mount と instantiate)、`app/src/engine/client.ts`、`app/src/engine/worker.ts`、`app/src/engine/lazy-file.ts`
- `app/src/main.ts`(677 行 `startLazy`)
- `tools/delivery-split/measure.mjs`(#{{101}} で `reads` が入った版)、`tools/boot-check/boot-bundle.mjs`
- #{{101}} の出力(像の読みの形)

## やること

1. 変更の前に、測る: `engine.wasm` の大きさ(168,996,523 B)と、起動の経路の読み(`tools/delivery-split/measure.mjs` の 3 回の出力)を報告に書く。
2. 分割の方式を決める。方式は作業者が決める。例: (a) wasm の動的リンク(side module)の形で、Balance を別の wasm にし、engine の runtime の関数を共有メモリと関数の表で呼ぶ。(b) モジュールごとに別の wasm を作り、engine との境界を、共有メモリと関数の表の呼び出しにする。選ばなかった方式の理由を報告に書く。
3. `fe-build` を変える: gen-all の 10 モジュールの連結から、Balance を外し、別の wasm を作る段を足す。bundle の段(`pack_step_for`・`app/scripts/pack-assets.mjs`)で、その wasm を起動後の取得(manifest の `lazy`)に入れる。#{{101}} の遅延の仕組みを使う。名前には `balance` と `.wasm` を含める。
4. harness と runtime を変える: Balance を、guest の要求(`LoadLibrary` など)の時に、その wasm を instantiate する。instantiate の時刻と要求の有無を、engine のログに 1 行で書く(#{{101}} の `image-read:` と同じ形。常に出す)。
5. `tools/delivery-split/measure.mjs` の `reads` に、Balance の wasm の instantiate を入れる(`requested` が true なら guest の要求)。
6. 方針を変えた場合は、変えた理由・代わりの案・実測の根拠を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/delivery-split/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/split.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const B=(a)=>a.filter((f)=>/balance/i.test(f.name)&&/\.wasm$/i.test(f.name));const eng=(a)=>a.find((f)=>f.name==="engine wasm");const ok=L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&eng(r.preStart).bytes<168996523&&B(r.lazy).length===1&&B(r.lazy)[0].bytes>0&&r.fetches.some((f)=>/balance/i.test(f.name)&&/\.wasm$/i.test(f.name)&&f.startMs>=r.preStartDoneMs)&&Array.isArray(r.reads)&&r.reads.some((x)=>/balance/i.test(x.name)&&/\.wasm$/i.test(x.name)&&x.requested===true));process.exit(ok?0:1)' "$RUNNER_TEMP/split.jsonl"
```

判定の基準: 3 回の起動で、(1) engine-idle・standIn が false・error が null、(2) engine wasm(`preStart` の `engine wasm`)が 168,996,523 B より小さい、(3) Balance の wasm(名前に `balance` と `.wasm`)が起動後の取得(`lazy`)に在り、取得の開始が起動前の取得の完了以後である、(4) `reads` に Balance の wasm の instantiate が、guest の要求(`requested` が true)で在る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 分割の方式と、選ばなかった方式の理由、変えたパスと増減したバイト数(engine.wasm の前後。新しい wasm の大きさ)、起動時間(elapsedMs の 3 回の値)、Worker の数と、各 Worker が持つ物、起動前と起動後の合計の大きさ、単一の wasm の上限(約 1GB)との比較。
- 方針を変えた場合は、変えた理由と、代わりの案(例: 複数のモジュールを束ねた分割の単位)を、実測の根拠つきで書く。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。分割は既定の動作にする。オプトインを作らない。
- 単一の wasm は 1GB 以下にする(依頼者の指示)。超える物は分ける。
- Worker の数の上限は、事前には決めない。当たったら報告に書き、方針を変える。
- 製品の既定の起動の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 依頼者が言っていない制限を、この作業の判断で増やさない。

## 他のタスクとの関係

- blocked_by: #{{101}}(像の読みを、guest の要求の時まで遅らせる)。
- #{{103}}(内蔵のプラグインの DLL の分類)の結果は、第 2 波の束ね方に使う。この Issue の型は、第 2 波の内蔵の DLL に当てる。
- #{{019}}(Fruity Filter と Fruity Delay 2)と、#{{027}}〜#{{029}}(外部プラグイン)は、この Issue の型を使う。
- 同じ表(`lift/harness/b2d2/modules.tsv`・`lift/harness/web/pack.args`)と、`lift/crates/fe-build/src/main.rs` を、他の作業が変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
