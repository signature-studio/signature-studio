---
draft: "268"
title: "opengl32 を WebGL2 へ橋渡しする: FLEX の engine から始める"
type: "Task"
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "022"
blocked_by: []
labels: ["plugin-builtin"]
size: "XL"
priority: "High"
project: "signature-studio/3"
---

## 目的

内蔵のインストゥルメント FLEX(#1544)の補助の DLL(`engine_x64.dll`・`engineV2_x64.dll`)は、`opengl32.dll` を import する。runtime(`lift/runtime/`)には opengl32 の関数の実装が無い(検索で 0 件)。そのため、FLEX は既定の曲を開いた直後に止まる(#1544 の作業の観測。下の「背景」)。

依頼者の判断: OpenGL を使う VFX 系の内蔵プラグイン(11 件。`inputs/vfx-deferred.tsv`。#1332 で後から入れる)は後回しにする。ただし、**FLEX は既定の曲(welcome-default)が使うので、FLEX が既定の曲の読み込みに必要であり、OpenGL の橋渡しは後回しにできない**。

この Issue の目標は、opengl32 の関数を、ブラウザの WebGL2 へ橋渡しする仕組みを、FLEX の engine から始めて入れること。最終の目標は、#1544 の受け入れが通ること(FLEX が `plugin-run` で instantiated と processed になり、既定の曲を開いても止まらない)。

## 背景

- 依頼者の方針: FL のコードを lift して走らせる。描画は、engine の描画の呼び出しとして canvas に出す。ページが画を描かない(ページの側で FLEX の画を描く経路を作らない)。
- 補助の DLL と import(`inputs/plugin-classes.tsv` の行 135・136、`inputs/vfx-deferred.tsv` の行 7・8):
  - `Plugins\Fruity\Generators\FLEX\engine_x64.dll`(5,673,984 B)。import: opengl32.dll・d3d9.dll・dwmapi.dll・windowscodecs.dll・dbghelp.dll ほか。
  - `Plugins\Fruity\Generators\FLEX\engineV2_x64.dll`(8,119,808 B)。import: opengl32.dll・d3d9.dll・dwmapi.dll・windowscodecs.dll・dbghelp.dll ほか。
  - sha256 は `inputs/plugin-classes.tsv` の値を使う。
- 本体 `FLEX_x64.dll` は、#1544 の作業で単独で読んだ。`GetFileAttributesW` で `…\Shared\Engine_x64.dll` の探索が失敗した後、NULL の関数ポインタを呼んで止まった。補助の不足が原因だと推定する(未確認)。
- 既存の描画の経路: `lift/runtime/fle_desk.c`・`fle_desk.h`(窓の絵の合成。`fle_desk_scene_mode` で、窓ごとのテクスチャを描く経路へ切り替わる)、`app/src/engine/desk-gl.ts`(WebGL で窓のテクスチャを描く既存の経路)。
- 固定機能の描画(`glBegin`・`glEnd`・`glVertex*` など)は WebGL2 に無い。頂点を溜めて描く形の橋渡しが要る。
- 像の領域: `lift/crates/fe-build/src/plugins.rs` の `GLOBAL_BASE` の範囲は、ほぼ一杯である(#1544 の作業の観測)。本体 1 本は入った。補助 2 本(計 約 14 MB)は、入らない見込み(推定)。入らないときは、入らない量を報告して止まる。`GLOBAL_BASE` の値は、この Issue では変えない。

## 段階(1 回で終わらない前提。段ごとに commit する)

### ① 実現の見込みの確認: `pm-work/findings/opengl-bridge.md`

- 補助 2 本が import する opengl32.dll の関数の全一覧を出す。PE の読みは、`lift/crates/fe-pe` の道具か、`tools/` の既存の道具で行う。どの道具を使うかは作業者が決める。
- 関数ごとに表の 1 行を書く: 関数名・使う DLL・WebGL2 で対応できるか(「対応」「要の変換」「無し」のいずれか)・橋渡しの方法。
- `d3d9.dll`(`Direct3DCreate9` など)・`dwmapi.dll`・`windowscodecs.dll` の import も、同じ形で表にする。関数の数が多いときは、FLEX の engine が実際に呼ぶ物を優先し、その旨を書く。
- 表は Markdown の表にし、1 行 1 関数にする。見出しと区切りを含めて、行頭が `| ` の行が 22 行以上あること(受け入れで数える)。
- 依頼者の方針に合う設計を 1 つ選び、理由を同じファイルに書く。候補の例(作業者が選ぶ): (a) runtime の中で opengl32 の呼び出しを受け、描画の命令を既存の窓の経路(`fle_desk`)に載せる。(b) 描画の命令を既存の WebGL の経路(`desk-gl.ts`)の描画へ渡す。(c) runtime の中で描いて、窓の表面に書く。どれを選んでも、ページが画を独自に描かない。
- 不可能・過大と分かったら、理由と代替を `findings` と報告に書いて止まる。代替の例: FLEX の engine が opengl32 を使うのは UI の描画だけで、音声の処理(DSP)だけ動かす経路が在るか。

### ② 最小の橋渡し(初期化に要る物)

- ①の表で初期化に要ると分かった物を、runtime に入れる。候補: `ChoosePixelFormat`・`SetPixelFormat`・`DescribePixelFormat`・`wglCreateContext`・`wglMakeCurrent`・`wglDeleteContext`・`SwapBuffers`・`glGetString`・`glGetError`・`glViewport`・`glClear`・`glClearColor`。
- 実装の場所は `lift/runtime/` の新しいファイル(名前は作業者が決める)。既存の import の登録の仕組み(`lift/runtime/fle_imports.c` など)を読んで、それに載せる。
- 単体試験: `lift/crates/fe-lift/tests/opengl_bridge.rs`。`lift/crates/fe-lift/tests/a4_report.rs` と同じ形で、`clang` が `lift/runtime/` の C を直接コンパイルして呼ぶ。試験の中身は、初期化の呼び出しの順(画素の形式の選択、コンテキストの作成、現在のコンテキストの設定、描画、入れ替え)がエラー無しに返ること。**先に red を書き、red の出力を報告に貼る**。

### ③ FLEX の plugin.tsv(補助 2 本を含める)

- `lift/plugins/flex/plugin.tsv` を作る。行は 3 本。
  - 本体: key `generators.flex.flex_x64.dll`、role `本体`、hook `rva:0x1b4b0,0x37930,0x37950`、args `--pdata`、omit `-`(#1544 の本文の値と同じ。sha256 は `inputs/plugin-classes.tsv` の行 137 の値)。
  - 補助: key `generators.flex.engine_x64.dll` と `generators.flex.enginev2_x64.dll`、role `補助`、hook `none`、args `--pdata`、omit `-`。guest は `Plugins\Fruity\Generators\FLEX\engine_x64.dll` と `…\engineV2_x64.dll`、sha256 と size は `inputs/plugin-classes.tsv` の行 135・136 の値。形は `lift/plugins/effector/plugin.tsv` の補助の行と同じ。
- `ci/extract-dlls.tsv` に、補助 2 本の行を足す(Effector の補助の行(24 行目)と同じ形。宛先は `plugins/Plugins/Fruity/Generators/FLEX/`)。
- `lift/crates/fe-build/src/plugins.rs` の `first_wave_folder()` の `FIRST_WAVE` に `r"Plugins\Fruity\Generators\FLEX"` を足し(配列の大きさを 1 増やす)、その試験の件数と `wave_total` を、FLEX の 3 本の分だけ直す。
- 像の領域に入るかを、`fe-build engine` が像の領域の超過で止まらないことで確かめる。止まったら、入らない量を報告して止まる。
- 確認(受け入れには入れない): `node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "FLEX" --scene c` で、FLEX が `instantiated` true・`processed` true・`error` null。既定の曲を開いても止まらない。`plugin-run` で FLEX を名前で解決するには、#{{267}} の直しが main に要る。main に無いときは、「確認できない」と報告に書く。

## やること

1. 段ごとに、変えたファイルを `git add` で 1 つずつ指定して commit する(①・②・③)。
2. `fe-build engine` は、③ の後に 1 回だけ回す。途中では回さない。
3. 受け入れ(下)を通す。

## 受け入れ

```accept
test -f lift/crates/fe-lift/tests/opengl_bridge.rs && test "$(grep -c '^| ' pm-work/findings/opengl-bridge.md)" -ge 22 && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && cargo test --locked --manifest-path lift/Cargo.toml -p fe-lift --test opengl_bridge && rm -rf lift/out-app/b2d-src lift/out-app/gen && node tools/place-plugin-dlls.mjs && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=20000&&r.preBootBytes<=314368711&&(!/^[BD]$/.test(r.flow)||r.welcomeClosed===true);process.exit(L.length===4&&L.map((r)=>r.flow).join("")==="ABCD"&&L.every(ok)?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- `pm-work/findings/opengl-bridge.md` の表の行(行頭が `| `)が 22 行以上ある(見出しと区切りを含む)。
- `lift/crates/fe-lift/tests/opengl_bridge.rs` が通る。先に red だったことを、報告に出力の行で書く。
- 起動確認(`boot.json`): `state` が engine-idle、`standIn` が false、`error` が null、`elapsedMs` が 20,000 以下。
- 流れ A〜D(4 行): `error` が null、`state` が engine-idle、`standIn` が false、`trapped` が false、`hostErrors` が 0、`opened` が true、B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- 報告に書く: ①の表の要点(対応・要の変換・無しの数)、選んだ設計と理由、②・③ の状態(済み・未了・止まり)、③ の FLEX の `plugin-run` の 1 行(あれば)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかで書く。出力の行を付ける。確認できなかった物は「確認できない」と書く。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- `GLOBAL_BASE`(像の領域)の値を変えない。
- 依頼者の方針に反する経路を作らない: FLEX の描画を JS で作り替えること、ページが画を独自に描くこと。
- Python・OCR を使わない。
- AI のベンダー名と Cloudflare の識別子を、ファイル・コミット・報告に書かない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit の名義を変えない。commit に AI の署名の行を付けない。必要の無いブランチを作らない。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の開始時刻を `date -u` で記録する。3 時間 15 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(この Issue は段階で進めるので、commit 済みの段を報告に書く)。
- 長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- 作業の最後に、受け入れが通らなくても commit する。

## 他のタスクとの関係

- #1544(FLEX)は、この Issue を blocked_by に持つ(起票の後に設定する)。#1544 の本体の行は、この Issue の ③ で補助 2 本と一緒になる。
- #{{267}}(`plugin-run` がインストゥルメントを名前で解決する)は、この Issue と独立。③ の確認の前に main に入っていると、確認が通る。
- #1463〜#1477 の 15 件は、#1544 を blocked_by から外された。FLEX の失敗で止まらないようにするため。
