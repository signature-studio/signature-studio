---
draft: "102"
title: "プラグインのコードを engine.wasm から分け、Fruity Balance を 1 プラグイン 1 wasm で起動後に取得する(最初の分割)"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["101"]
labels: ["delivery", "plugin-builtin", "resources"]
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

## 前回の試行で分かったこと

### 1 回目の試行(run 45124597952。約 4 時間 20 分。受け入れ (4) で落ち、判定は push しなかった。作業は `work.diff` として artifact `agent-1342` に残った。ローカルの commit `8080495b` は push されていない)

次の run は、この `work.diff` を作業ツリーに取り込んでから始める(取り込みの結果は `.prior-work.md`)。artifact は次の作業者が読めないので、要点を本文に書く。

- **変更の前(基準。runner の値。nproc 4、loadavg 1.4〜1.9)**: `engine.wasm` は 169,015,160 B(この Issue の 168,996,523 B とは一致しない)。起動は engine-idle、所要 10,993〜11,093 ms(measure の `elapsedMs`)、boot-check 12,224 ms。`reads` は DSP・Blend2D・QFC・FreeType・ILTools の 5 件だけで、Balance の要求は起動中に起きていない。
- **分割の方式(b)**: 既存の hybrid の境界(`c.memory`・`c.fle_imp_*`・R[] の共有メモリ)を使い、`wasm-merge` の代わりに JS で別の instance に結ぶ。fe-wasmgen の one-module 版で Balance を direct に下ろすと、12,500 関数のうち 12,282 が ok(残り 218 は C 経路のまま)。
- **選ばなかった (a)**: side module(emscripten の dylink)。165 MB の engine 全体を MAIN_MODULE で作り直すことになり、全体の codegen が変わる(P2・P3 への影響は未測定)。C の R[] と dispatch 表は別 instance に複製できないので、C 単独のリンクも不可。
- **変更後の測定(3 回。runner の値)**:

| 受け入れ | 判定 | 出力 |
|---|---|---|
| (1) engine-idle・standIn false・error null | 受理 | boot-check の `elapsedMs` 11468。measure の 3 回とも同じ |
| (2) engine.wasm が 168,996,523 B 未満 | 受理 | `engine wasm` 165,079,668 B(3 回とも) |
| (3) Balance の wasm が lazy。取得の開始が起動前の取得の完了以後 | 受理 | lazy `fruity_balance_x64.6800000.wasm` 10,452,743 B。取得の開始 4010・4154・4102 ms、起動前の完了 2436・2510・2416 ms |
| (4) reads に Balance の instantiate(requested=true) | **未達** | 3 回とも Balance の行は `[]`。起動中に要求が無い(基準と同じ) |

- **大きさ**: engine.wasm 169,015,160 → 165,079,668 B(−3,935,492 B)。Balance の wasm は 10,452,743 B。合計は 175,532,411 B で、基準より +6,517,251 B(+3.9%)。direct に下ろした Balance は、C 経路で engine に入っていた分より大きい(推定)。単一 wasm の上限(約 1GB)に対し、engine は約 15%、Balance は約 1%。
- **起動時間(measure の `elapsedMs`)**: 基準 10,993・11,093・11,010 ms、変更後 10,747・10,785・10,667 ms。P1(10 秒)は両方で未達。
- **実装の要点**(作業ツリーに在る): 分けた module は merge せず別のファイルに書く(`--separate`・`--separate-dir`)。n0 の exports に `__wrap_<名前>` も出す(`--wrap` の対象 `fle_imp_FormatMessageW`・`fle_imp_LockResource` が素の名前で export されず、hybrid の検査で落ちたため)。JS 側は素の名前が無いとき `__wrap_` 版を使う。`tests/plan/engine.txt`・`bundle.txt` を現在の入力に合わせた(`bundle.txt` は #{{019}} 以来古かった)。`fe-build engine` は hybrid 段で 1 回落ちた。直した後は、残りの段を `tests/plan/engine.txt` の同じコマンドで手で走らせた。
- **未確認・リスク**: Worker の数と、各 Worker が持つ物(未測定)。MXCSR・x87 の状態は direct モジュールごとに持つので、Balance 側の写しが一つ増える。Balance のフックポイント(`fruity-delphi-effect-hook-points.txt` の 3 つ)は direct の経路に無いので適用していない。ゲストのスレッド(別の pthread Worker)から Balance を呼ぶと、その Worker に供給が無く、glue の欠落 import で止まる(fail-closed。未確認)。Balance 越しの longjmp は JS の stub を通るため通らない(未確認)。

### 残りのやること

(a) 作業ツリーを確かめる。`git status`・`git diff HEAD` で、次のパスの変更が載っていること: `lift/crates/fe-wasmgen/src/hybrid.rs`・`main.rs`、`lift/harness/hyb/`、`lift/harness/two/main_two.c`、`lift/harness/web/pack.args`、`app/src/engine/xmod.ts`(新規)・`fruity.ts`、`lift/crates/fe-build/tests/plan/*.txt`、`tools/delivery-split/measure.mjs`。載っていなければ、`work.diff` の内容から作り直す(違いは報告に書く)。
(b) 下の「(4) の場面」で、Balance を要求させる場面を測定に入れる。
(c) その場面で instantiate の経路が動くこと(`image-read: … requested=1` が出て、Balance の呼び出しが落ちないこと)を確かめる。動かなければ直す。
(d) Worker の数と、各 Worker が持つ物を測り、報告に書く。
(e) 作業ツリーを commit する(受け入れが通らなくても)。

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

1 回目の試行で、1 から 4 の前半(測定・方式・fe-build・fe-wasmgen・harness・JS)は済んだ。方式は (b) のまま、方針は変えていない。残りは「残りのやること」。

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

### (4) の場面(Balance を要求させる)

製品の既定の起動は変えない。測定(`tools/delivery-split/measure.mjs`)に、Balance を要求させる場面を入れる。場面は次の順に、作業者が実測で選ぶ。ラベル `resources` を付けた(`lift/out-app/host-extra/resources/` に、テンプレートとデモのプロジェクトが在る)。

1. `lift/out-app/host-extra/resources/` の `.flp`(テンプレートとデモ)から、プラグイン名 `Fruity Balance`(ASCII か UTF-16 のバイト列)を含む物を探し、そのプロジェクトを FL の UI(スタート画面の一覧・File メニューの Open)で開く。`tools/feat-c6-popup-menus/` などが、メニューの項目を選ぶ方法を持つ。
2. ミキサーの FX スロットのメニュー(#{{010}}・#{{106}} で、メニューが出る)から Balance を足す。
3. 上の 2 つが使えなければ、`lift/harness/two/main_two.c` の試験の引数で、Balance を LoadLibrary する guest の手順を実行し、`fle_xmod_load` の経路を通す。

どの場面でも、`reads` に Balance の wasm の instantiate が `requested: true` で出ることが、(4) の判定そのもの。Welcome の窓を閉じる操作(「No thanks」・Escape)は、今 engine が trap する既知の不具合があり、別の Issue で直す予定。閉じずに済む方法を先に探す。閉じないと測れない場合は、その旨を報告に書く。

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
