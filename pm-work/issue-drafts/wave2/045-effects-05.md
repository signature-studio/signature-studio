---
draft: "045"
title: "内蔵エフェクト 5: Fruity Big Clock・Fruity Blood Overdrive・Fruity Center を、1 プラグイン 1 wasm で入れる"
type: "Task"
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "021"
blocked_by: ["126","127"]
labels: ["plugin-builtin"]
size: "S"
priority: "High"
project: "signature-studio/3"
---

## 目的

依頼者の指示: 内蔵のプラグインを、起動の後にバックグラウンドで届ける(1 プラグイン 1 wasm)。内蔵のエフェクトのうち、次の 3 フォルダ(Fruity Big Clock・Fruity Blood Overdrive・Fruity Center)を、1 プラグイン 1 wasm で入れる。

## 背景

- 対象は内蔵のエフェクトの 3 フォルダ。単位はプラグインのフォルダ(`Plugins\Fruity\<種別>\<フォルダ>\`)で、本体の DLL と、その補助の DLL(本体が使う。入口を持たない)は同じフォルダに在る。
- この束の DLL は、本体 3・補助 0(計 3)。下の表(各フォルダの `plugin.tsv` の全文)が、この束の入力そのもの。大きさ・SHA-256・export・import・hook の値は、分類の表(`inputs/plugin-classes.tsv`)と hook 点の表(`inputs/plugin-hooks.tsv`)から写した。
- 範囲に入れない物: Data(`Plugins\...\Data\` の下のファイル)。生成の段は Data を seed に載せる仕組みを持たず、seed の表は共有のファイルで、並行の束が衝突する。Data が要る停止が出たら、報告に書いて止まる。
- 範囲に入れない物: OpenGL を使う VFX 系の DLL(`inputs/vfx-deferred.tsv`)。
- 親: #{{021}}(内蔵 Effects の残り)の子。
- 依存: #{{126}}(第 2 波の 2 本目の型)・#{{127}}(1 本ずつの道具)。段 1 の束は、この 2 件が閉じたら起動する

## 1 プラグインの入れ方(第 1 波の経験)

- フォルダごとに `lift/plugins/<dir>/plugin.tsv` を 1 つ足す(`<dir>` は各見出しの名前)。見出しは 8 列(`key`・`guest`・`role`・`sha256`・`size`・`hook`・`args`・`omit`)、区切りは TAB。下の表を、そのまま書く。
- 共有のファイルは 1 行も変えない: `ci/`・`inputs/`・`lift/harness/`・`lift/runtime/`・`lift/crates/`・`app/`・`tools/` の既存の物(表・一覧・試験の期待値を含む)。生成の段が `lift/plugins/` から表を作る。だから、この束の commit は `lift/plugins/` の中だけになる。
- `hook`: 下の表のとおり。`rva:` の 3 つは hook 点の表の RVA(本体の 要・既存)。`none` は、不要の DLL と、補助の DLL。
- `omit`: 表のとおり `-` と書く。setjmp を呼ぶ関数の入口は、#{{126}} で入った自動の検出が決める。手で推測した値は書かない。読み込みの後に `ExitProcess`(終了コード 0)で止まったら、その停止を報告して止まる(第 1 波の Filter の型)。
- `args`: 表のとおり `--pdata`。

## 停止が出たときの型

| 停止 | 扱い |
|---|---|
| 読み込みの後の `ExitProcess`(終了コード 0)。setjmp の着地が無い | `omit` の問題。直すのは `plugin.tsv` の `omit` の値だけ(自動の検出の結果を確かめる)。直せないなら報告して止まる |
| `missing-path`(DLL や言語のファイルが無い。例: `ILSvg_x64.dll`) | 仮想のモジュール(runtime)は共有の物。直さず、報告して止まる(#{{154}} の範囲) |
| `indirect call to … is not a known function entry`(switch 表の飛び先) | lift の共有の物(#{{170}} で直した型)。再発は報告して止まる |
| `GetFileVersionInfoSizeW` などの、ディレクトリ部の無いファイル名 | #{{172}} が直す。未完なら報告して止まる |
| `%FLStudioFactoryData%` などの環境変数の展開。`fle_stop` の行 | #{{169}} が直す。未完なら報告して止まる |
| `CreateFileW: empty path`・Data の欠落 | Data の配信(範囲に入れない)。報告して止まる |

直してよいのは、`lift/plugins/<dir>/plugin.tsv` の中の値(`hook`・`args`・`omit`)だけ。それ以外の直しは、報告に書いて止まる。

## 受け入れ

```accept
test -f lift/plugins/fruity-big-clock/plugin.tsv && test -f lift/plugins/fruity-blood-overdrive/plugin.tsv && test -f lift/plugins/fruity-center/plugin.tsv && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" 'fruity-big-clock' 'fruity-blood-overdrive' 'fruity-center' > "$RUNNER_TEMP/plugin-run.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===3&&L.every((r)=>r.instantiated===true&&r.error===null&&r.elapsedMs<=12500&&r.processed===true)?0:1)' "$RUNNER_TEMP/plugin-run.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711&&(!/^[BD]$/.test(r.flow)||r.welcomeClosed===true);process.exit(L.length===4&&L.map((r)=>r.flow).join("")==="ABCD"&&L.every(ok)?0:1)' "$RUNNER_TEMP/flow.jsonl" && test -z "$(git diff --name-only origin/main...HEAD -- . ':!lift/plugins')"
```

判定の基準:
- 起動の確認(`tools/boot-check/boot-bundle.mjs`): `state` が engine-idle、`standIn` が false、`error` が null、`elapsedMs` が 12,500 以下(DPR 1)。
- 1 本ずつの道具(`tools/plugin-run/run.mjs`。#{{127}})の出力は、この束の本体ごとに 1 行(行数は 3 行)。どの行も、`instantiated` が true、`error` が null、`elapsedMs` が 12,500 以下、かつ `processed` が true。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`)は 4 行。各行で `error` が null、`state` が engine-idle、`standIn` が false、`trapped` が false、`hostErrors` が 0、`opened` が true、`elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。B と D は `welcomeClosed` が true。
- 差分: `lift/plugins/` の外に変更が無い(受け入れの最後の判定)。

## 読む物

- `lift/plugins/sytrus/plugin.tsv`(見出しと行の書式。8 列の見本)
- `lift/crates/fe-build/src/plugins.rs`(行の検査: key の一意、guest の一意、hook の形、omit の形)
- `inputs/plugin-classes.tsv`・`inputs/plugin-hooks.tsv`(この束の行の出所)
- `tools/plugin-run/run.mjs`(#{{127}}。main に無ければ、止まって報告する)

## 本文に書いた表(入力)

### Fruity Big Clock(`lift/plugins/fruity-big-clock/plugin.tsv`)

- 種別: エフェクト。本体 1・補助 0。Data: なし
- 本体: `Plugins\Fruity\Effects\Fruity Big Clock\Fruity Big Clock_x64.dll`
  - 大きさ 3598336 B・SHA-256 1e4bbcea2b62680bf318ee831aa200baba53c8662d6b8009d2b8fa24711d8284
  - export: CreatePlugInstance; __dbk_fcall_wrapper; dbkFCallWrapperAddr
  - import: winmm.dll; comctl32.dll; shell32.dll; user32.dll; version.dll; oleaut32.dll; advapi32.dll; kernel32.dll; ole32.dll; msacm32.dll; api-ms-win-crt-string-l1-1-0.dll; gdi32.dll
  - hook: rva:0x17ba0,0x31d60,0x31d80(hook 点の表の 3 つの RVA)

```tsv
key	guest	role	sha256	size	hook	args	omit
effects.fruity-big-clock.fruity_big_clock_x64.dll	Plugins\Fruity\Effects\Fruity Big Clock\Fruity Big Clock_x64.dll	本体	1e4bbcea2b62680bf318ee831aa200baba53c8662d6b8009d2b8fa24711d8284	3598336	rva:0x17ba0,0x31d60,0x31d80	--pdata	-
```

### Fruity Blood Overdrive(`lift/plugins/fruity-blood-overdrive/plugin.tsv`)

- 種別: エフェクト。本体 1・補助 0。Data: なし
- 本体: `Plugins\Fruity\Effects\Fruity Blood Overdrive\Fruity Blood Overdrive_x64.dll`
  - 大きさ 3727360 B・SHA-256 f2394c1fa3bbb4373279af7e8ab3415ca0558a89412be9760be9d3b6c7cc5c83
  - export: CreatePlugInstance; __dbk_fcall_wrapper; dbkFCallWrapperAddr
  - import: winmm.dll; comctl32.dll; shell32.dll; user32.dll; version.dll; oleaut32.dll; advapi32.dll; kernel32.dll; ole32.dll; msacm32.dll; api-ms-win-crt-string-l1-1-0.dll; gdi32.dll
  - hook: rva:0x17bc0,0x31ec0,0x31ee0(hook 点の表の 3 つの RVA)

```tsv
key	guest	role	sha256	size	hook	args	omit
effects.fruity-blood-overdrive.fruity_blood_overdrive_x64.dll	Plugins\Fruity\Effects\Fruity Blood Overdrive\Fruity Blood Overdrive_x64.dll	本体	f2394c1fa3bbb4373279af7e8ab3415ca0558a89412be9760be9d3b6c7cc5c83	3727360	rva:0x17bc0,0x31ec0,0x31ee0	--pdata	-
```

### Fruity Center(`lift/plugins/fruity-center/plugin.tsv`)

- 種別: エフェクト。本体 1・補助 0。Data: なし
- 本体: `Plugins\Fruity\Effects\Fruity Center\Fruity Center_x64.dll`
  - 大きさ 3588608 B・SHA-256 ffaf5d072f35388651d1f2f9316e02174fd30fda437a9b8e0917a20f6ab8d1af
  - export: CreatePlugInstance; __dbk_fcall_wrapper; dbkFCallWrapperAddr
  - import: winmm.dll; comctl32.dll; shell32.dll; user32.dll; version.dll; oleaut32.dll; advapi32.dll; kernel32.dll; ole32.dll; msacm32.dll; api-ms-win-crt-string-l1-1-0.dll; gdi32.dll
  - hook: rva:0x17ba0,0x31d50,0x31d70(hook 点の表の 3 つの RVA)

```tsv
key	guest	role	sha256	size	hook	args	omit
effects.fruity-center.fruity_center_x64.dll	Plugins\Fruity\Effects\Fruity Center\Fruity Center_x64.dll	本体	ffaf5d072f35388651d1f2f9316e02174fd30fda437a9b8e0917a20f6ab8d1af	3588608	rva:0x17ba0,0x31d50,0x31d70	--pdata	-
```

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 追加した `plugin.tsv` の行の数(本体と補助)、道具の JSON の行、`elapsedMs` の値、停止があれば出力の末尾と、その停止の型(上の表)、Data の扱い。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 製品の既定の動作を、測定のためだけに変えない。
- 共有のファイルを変えない(上の「1 プラグインの入れ方」)。直してよいのは `lift/plugins/<dir>/plugin.tsv` の中の値だけ。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- Python・OCR を使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の最初に `date -u` で時刻を記録する。`.prior-work.md` が在れば、最初に読む。
- ビルド(`fe-build engine`)は約 35 分から 50 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- 4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit して報告を書いて終える。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する(未 commit の変更が残ると、判定は失敗する)。

## 他のタスクとの関係

- 段: 段 1(同じ段の束は、並行に動く)。
- blocked_by: #{{126}}(第 2 波の 2 本目の型)・#{{127}}(1 本ずつの道具)。段 1 の束は、この 2 件が閉じたら起動する
- 同じ段の他の束とは、共有のファイルを編集しないので衝突しない(各束は別の `lift/plugins/<dir>/` だけに書く)。
- 第 1 波の Balance(#{{125}})・Filter(#{{126}})・Sytrus(#{{157}})は、この束の型の見本。
