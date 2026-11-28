---
draft: "068"
title: "内蔵エフェクト 28: Vintage Chorus・Vintage Phaser・Vocodex を、1 プラグイン 1 wasm で入れる"
type: "Task"
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "021"
blocked_by: ["126","127","192","041","042","043"]
labels: ["plugin-builtin"]
size: "M"
priority: "High"
project: "signature-studio/3"
---

## 目的

依頼者の指示: 内蔵のプラグインを、起動の後にバックグラウンドで届ける(1 プラグイン 1 wasm)。内蔵のエフェクトのうち、次の 3 フォルダ(Vintage Chorus・Vintage Phaser・Vocodex)を、1 プラグイン 1 wasm で入れる。

## 背景

- 対象は内蔵のエフェクトの 3 フォルダ。単位はプラグインのフォルダ(`Plugins\Fruity\<種別>\<フォルダ>\`)で、本体の DLL と、その補助の DLL(本体が使う。入口を持たない)は同じフォルダに在る。
- この束の DLL は、本体 4・補助 2(計 6)。下の表(各フォルダの `plugin.tsv` の全文)が、この束の入力そのもの。大きさ・SHA-256・export・import・hook の値は、分類の表(`inputs/plugin-classes.tsv`)と hook 点の表(`inputs/plugin-hooks.tsv`)から写した。
- 範囲: Data(`Plugins\...\Data\` の下のファイル)は、この束の `lift/plugins/<dir>/data.tsv` で宣言する(下の「Data の宣言(この束)」)。宣言の書式と配信の仕組みは #{{192}} の本文のとおり。
- 範囲に入れない物: OpenGL を使う VFX 系の DLL(`inputs/vfx-deferred.tsv`)。
- 親: #{{021}}(内蔵 Effects の残り)の子。
- 依存: #{{126}}(第 2 波の 2 本目の型)・#{{127}}(1 本ずつの道具)・#{{192}}(Data の宣言の仕組み)・段 1 の先頭 3 束(#{{041}}・#{{042}}・#{{043}})。段 1 の先頭 3 束が閉じるまで、この束は起動しない

## 1 プラグインの入れ方(第 1 波の経験)

- フォルダごとに `lift/plugins/<dir>/plugin.tsv` を 1 つ足す(`<dir>` は各見出しの名前)。見出しは 8 列(`key`・`guest`・`role`・`sha256`・`size`・`hook`・`args`・`omit`)、区切りは TAB。下の表を、そのまま書く。
- 受け入れの前に `node tools/place-plugin-dlls.mjs` が、この束の DLL(と Data)を、`lift/out-app/host-extra/plugins/`(`plugin-builtin` の作業の木に在る)から木に置く。`missing` が出たら、`plugin.tsv`/`data.tsv` の行の `guest`・`sha256` が `host-extra/plugins/MANIFEST.tsv` と合っているかを確かめる。
- 共有のファイルは 1 行も変えない: `ci/`・`inputs/`・`lift/harness/`・`lift/runtime/`・`lift/crates/`・`app/`・`tools/` の既存の物(表・一覧・試験の期待値を含む)。生成の段が `lift/plugins/` から表を作る。だから、この束の commit は `lift/plugins/` の中だけになる。
- `hook`: 下の表のとおり。`rva:` の 3 つは hook 点の表の RVA(本体の 要・既存)。`none` は、不要の DLL と、補助の DLL。
- `omit`: 表のとおり `-` と書く。setjmp を呼ぶ関数の入口は、#{{126}} で入った自動の検出が決める。手で推測した値は書かない。読み込みの後に `ExitProcess`(終了コード 0)で止まったら、その停止を報告して止まる(第 1 波の Filter の型)。
- `args`: 表のとおり `--pdata`。

## Data の宣言(この束)

- 宣言の場所: 下の各フォルダに、`lift/plugins/<dir>/data.tsv` を 1 つ足す。書式(4 列: `guest`・`installer`・`sha256`・`size`。TAB 区切り)と、取り出し・配信の仕組みは #{{192}} の本文のとおり。
- Vocodex(`lift/plugins/vocodex/data.tsv`): 28 ファイル・388,284 B。直下の `Plugins\Fruity\Effects\Vocodex\Data\` の下の行のうち、この束の DLL でない物。
- この束の Data: 1 フォルダ・28 ファイル・計 388,284 B。
- 行の値は、`inputs/fl-installer-files.tsv` の該当行(guest・大きさ・SHA-256)から写す。一覧に無い行は書かない。宣言する行の数は、上の件数と同じにする。
- 道具の出力: `tools/plugin-run/run.mjs`(#{{127}})の各行に `missingInInventory` を載せる。#{{127}} の本文の項目に無ければ、`tools/p3-audio-probe/measure.mjs` の `missingInInventory` と同じ数え方(インストーラの一覧に在る絶対パスの要求で、見つからず、.dll でない物。名前も同じ)で、道具に足す。
- 宣言に含めない物: 「本文に書いた表」の「Data の外(入れ子)」の行。
- 共有のファイル(`lift/harness/seeds/vfs-seed-idle1.tsv`・`lift/harness/web/pack.args`)には 1 行も足さない。

## 停止が出たときの型

| 停止 | 扱い |
|---|---|
| 読み込みの後の `ExitProcess`(終了コード 0)。setjmp の着地が無い | `omit` の問題。直すのは `plugin.tsv` の `omit` の値だけ(自動の検出の結果を確かめる)。直せないなら報告して止まる |
| `missing-path`(DLL や言語のファイルが無い。例: `ILSvg_x64.dll`) | 仮想のモジュール(runtime)は共有の物。直さず、報告して止まる(#{{154}} の範囲) |
| `indirect call to … is not a known function entry`(switch 表の飛び先) | lift の共有の物(#{{170}} で直した型)。再発は報告して止まる |
| `GetFileVersionInfoSizeW` などの、ディレクトリ部の無いファイル名 | #{{172}} が直す。未完なら報告して止まる |
| `%FLStudioFactoryData%` などの環境変数の展開。`fle_stop` の行 | #{{169}} が直す。未完なら報告して止まる |
| `CreateFileW: empty path`・Data の欠落 | `data.tsv` の宣言の不足。上の Data の節の行を足す。節に無い物を要求したら、報告して止まる |

直してよいのは、`lift/plugins/<dir>/plugin.tsv` の中の値(`hook`・`args`・`omit`)と、Data の節どおりの `lift/plugins/<dir>/data.tsv` の行だけ。それ以外の直しは、報告に書いて止まる。

## 受け入れ

```accept
test -f lift/plugins/vintage-chorus/plugin.tsv && test -f lift/plugins/vintage-phaser/plugin.tsv && test -f lift/plugins/vocodex/plugin.tsv && test -f lift/plugins/vocodex/data.tsv && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").split("\n").filter((l)=>l.trim()!==""&&!l.startsWith("#")&&!l.startsWith("guest\t"));process.exit(L.length===28?0:1)' lift/plugins/vocodex/data.tsv && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && node tools/place-plugin-dlls.mjs && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" 'vintage-chorus' 'vintage-phaser' 'vocodex' > "$RUNNER_TEMP/plugin-run.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===4&&L.every((r)=>r.instantiated===true&&r.error===null&&r.elapsedMs<=12500&&r.processed===true&&r.missingInInventory===0)?0:1)' "$RUNNER_TEMP/plugin-run.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711&&(!/^[BD]$/.test(r.flow)||r.welcomeClosed===true);process.exit(L.length===4&&L.map((r)=>r.flow).join("")==="ABCD"&&L.every(ok)?0:1)' "$RUNNER_TEMP/flow.jsonl" && test -z "$(git diff --name-only origin/main...HEAD -- . ':!lift/plugins')"
```

判定の基準:
- 起動の確認(`tools/boot-check/boot-bundle.mjs`): `state` が engine-idle、`standIn` が false、`error` が null、`elapsedMs` が 12,500 以下(DPR 1)。
- 1 本ずつの道具(`tools/plugin-run/run.mjs`。#{{127}})の出力は、この束の本体ごとに 1 行(行数は 4 行)。どの行も、`instantiated` が true、`error` が null、`elapsedMs` が 12,500 以下、かつ `processed` が true。
- Data(`lift/plugins/<dir>/data.tsv`): 宣言した行数が、上の Data の節の件数と同じ。道具の出力の各行の `missingInInventory` が 0(Data を要求して見つけられない件数。`tools/p3-audio-probe/measure.mjs` の `missingInInventory` と同じ数え方)。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`)は 4 行。各行で `error` が null、`state` が engine-idle、`standIn` が false、`trapped` が false、`hostErrors` が 0、`opened` が true、`elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。B と D は `welcomeClosed` が true。
- 差分: `lift/plugins/` の外に変更が無い(受け入れの最後の判定)。

## 読む物

- `lift/plugins/sytrus/plugin.tsv`(見出しと行の書式。8 列の見本)
- `lift/crates/fe-build/src/plugins.rs`(行の検査: key の一意、guest の一意、hook の形、omit の形)
- `inputs/plugin-classes.tsv`・`inputs/plugin-hooks.tsv`(この束の行の出所)
- `tools/plugin-run/run.mjs`(#{{127}}。main に無ければ、止まって報告する)
- #{{192}}(Data の宣言の仕組み)の本文: `data.tsv` の書式(列と件数の数え方)と、取り出し・配信の仕組み。

## 本文に書いた表(入力)

### Vintage Chorus(`lift/plugins/vintage-chorus/plugin.tsv`)

- 種別: エフェクト。本体 1・補助 1。Data: なし
- 本体: `Plugins\Fruity\Effects\Vintage Chorus\Vintage Chorus_x64.dll`
  - 大きさ 3941888 B・SHA-256 c8703b432460b609e4d05e961014be9dd0e46501b8b844ae4b7e16b8e9b0c2eb
  - export: CreatePlugInstance; __dbk_fcall_wrapper; dbkFCallWrapperAddr
  - import: winmm.dll; comctl32.dll; shell32.dll; user32.dll; version.dll; oleaut32.dll; advapi32.dll; imagehlp.dll; kernel32.dll; ole32.dll; msacm32.dll; api-ms-win-crt-string-l1-1-0.dll; gdi32.dll
  - hook: rva:0x17c10,0x31e70,0x31e90(hook 点の表の 3 つの RVA)
- 補助: `Plugins\Fruity\Effects\Vintage Chorus\engine_x64.dll`
  - 大きさ 204800 B・備考: CreatePlugInstance を export しない(入口が無い)。Delphi 系の印が無い(不要)。

```tsv
key	guest	role	sha256	size	hook	args	omit
effects.vintage-chorus.vintage_chorus_x64.dll	Plugins\Fruity\Effects\Vintage Chorus\Vintage Chorus_x64.dll	本体	c8703b432460b609e4d05e961014be9dd0e46501b8b844ae4b7e16b8e9b0c2eb	3941888	rva:0x17c10,0x31e70,0x31e90	--pdata	-
effects.vintage-chorus.engine_x64.dll	Plugins\Fruity\Effects\Vintage Chorus\engine_x64.dll	補助	1569c57746448dfbfc2b75aab3aa490ea79a24b0120f6b4165d3a17698d9677e	204800	none	--pdata	-
```

### Vintage Phaser(`lift/plugins/vintage-phaser/plugin.tsv`)

- 種別: エフェクト。本体 1・補助 1。Data: なし
- 本体: `Plugins\Fruity\Effects\Vintage Phaser\Vintage Phaser_x64.dll`
  - 大きさ 3973632 B・SHA-256 a14e30ac46a436a363d5208b85c6b8f81d3ea259cb44cd304b29224cb9660b88
  - export: CreatePlugInstance; __dbk_fcall_wrapper; dbkFCallWrapperAddr
  - import: winmm.dll; comctl32.dll; shell32.dll; user32.dll; version.dll; oleaut32.dll; advapi32.dll; imagehlp.dll; kernel32.dll; ole32.dll; msacm32.dll; api-ms-win-crt-string-l1-1-0.dll; gdi32.dll
  - hook: rva:0x17c10,0x31e90,0x31eb0(hook 点の表の 3 つの RVA)
- 補助: `Plugins\Fruity\Effects\Vintage Phaser\engine_x64.dll`
  - 大きさ 313344 B・備考: CreatePlugInstance を export しない(入口が無い)。Delphi 系の印が無い(不要)。

```tsv
key	guest	role	sha256	size	hook	args	omit
effects.vintage-phaser.vintage_phaser_x64.dll	Plugins\Fruity\Effects\Vintage Phaser\Vintage Phaser_x64.dll	本体	a14e30ac46a436a363d5208b85c6b8f81d3ea259cb44cd304b29224cb9660b88	3973632	rva:0x17c10,0x31e90,0x31eb0	--pdata	-
effects.vintage-phaser.engine_x64.dll	Plugins\Fruity\Effects\Vintage Phaser\engine_x64.dll	補助	360e150175ed8597b3aa0eca4d840bae3aac41baff1c1e578fe177947c4d7215	313344	none	--pdata	-
```

### Vocodex(`lift/plugins/vocodex/plugin.tsv`)

- 種別: エフェクト。本体 2・補助 0。Data: 28 ファイル(388,284 B。`lift/plugins/vocodex/data.tsv` で宣言する)
- Data の外(入れ子): `Plugins\Fruity\Effects\Vocodex\Synthesizer\Data\` の 26 ファイル(55,452 B)。直下の `Plugins\Fruity\Effects\Vocodex\Data\` ではないので、宣言に含めない(#{{192}} の定義)。この Data が要る停止が出たら、報告に書く。
- 本体: `Plugins\Fruity\Effects\Vocodex\Synthesizer\Sytrus_x64.dll`
  - 大きさ 6308352 B・SHA-256 e9c163265d11bb066cd87592638e51e2e367a587aa2249abd4993397c25bedd0
  - export: CreatePlugInstance; __dbk_fcall_wrapper; dbkFCallWrapperAddr
  - import: winmm.dll; winspool.drv; comdlg32.dll; comctl32.dll; shell32.dll; user32.dll; version.dll; oleaut32.dll; advapi32.dll; imagehlp.dll; kernel32.dll; ole32.dll; msacm32.dll; api-ms-win-crt-string-l1-1-0.dll; gdi32.dll
  - hook: rva:0x1b960,0x38bb0,0x38bd0(hook 点の表の 3 つの RVA)
- 本体: `Plugins\Fruity\Effects\Vocodex\Vocodex_x64.dll`
  - 大きさ 5957632 B・SHA-256 68e3560956c8af8d1b50b926d8819a69a40aef28c0e1efd2b0dfa5941b680d17
  - export: CreatePlugInstance; __dbk_fcall_wrapper; dbkFCallWrapperAddr
  - import: winmm.dll; winspool.drv; comdlg32.dll; comctl32.dll; shell32.dll; user32.dll; version.dll; oleaut32.dll; advapi32.dll; imagehlp.dll; kernel32.dll; ole32.dll; msacm32.dll; api-ms-win-crt-string-l1-1-0.dll; gdi32.dll
  - hook: rva:0x1b7b0,0x389e0,0x38a00(hook 点の表の 3 つの RVA)

```tsv
key	guest	role	sha256	size	hook	args	omit
effects.vocodex.sytrus_x64.dll	Plugins\Fruity\Effects\Vocodex\Synthesizer\Sytrus_x64.dll	本体	e9c163265d11bb066cd87592638e51e2e367a587aa2249abd4993397c25bedd0	6308352	rva:0x1b960,0x38bb0,0x38bd0	--pdata	-
effects.vocodex.vocodex_x64.dll	Plugins\Fruity\Effects\Vocodex\Vocodex_x64.dll	本体	68e3560956c8af8d1b50b926d8819a69a40aef28c0e1efd2b0dfa5941b680d17	5957632	rva:0x1b7b0,0x389e0,0x38a00	--pdata	-
```

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 追加した `plugin.tsv` の行の数(本体と補助)、道具の JSON の行、`elapsedMs` の値、停止があれば出力の末尾と、その停止の型(上の表)、Data の扱い(宣言した `data.tsv` の行数(フォルダごと)と、道具の出力の `missingInInventory`)。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 製品の既定の動作を、測定のためだけに変えない。
- 共有のファイルを変えない(上の「1 プラグインの入れ方」)。直してよいのは `lift/plugins/<dir>/plugin.tsv` の中の値と、Data の節どおりの `data.tsv` の行だけ。
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

- 段: 段 2(同じ段の束は、並行に動く)。
- blocked_by: #{{126}}(第 2 波の 2 本目の型)・#{{127}}(1 本ずつの道具)・#{{192}}(Data の宣言の仕組み)・段 1 の先頭 3 束(#{{041}}・#{{042}}・#{{043}})。段 1 の先頭 3 束が閉じるまで、この束は起動しない
- 同じ段の他の束とは、共有のファイルを編集しないので衝突しない(各束は別の `lift/plugins/<dir>/` だけに書く)。
- 第 1 波の Balance(#{{125}})・Filter(#{{126}})・Sytrus(#{{157}})は、この束の型の見本。
