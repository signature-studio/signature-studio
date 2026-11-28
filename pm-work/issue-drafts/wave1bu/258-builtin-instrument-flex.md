---
draft: "258"
title: "内蔵インストゥルメント 16: FLEX を、1 プラグイン 1 wasm で入れる"
type: "Task"
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "022"
blocked_by: []
labels: ["plugin-builtin"]
size: "L"
priority: "High"
project: "signature-studio/3"
---

## 目的

依頼者の指示: 内蔵のプラグインを、起動の後にバックグラウンドで届ける(1 プラグイン 1 wasm)。内蔵のインストゥルメント FLEX を、1 プラグイン 1 wasm で入れる。

FLEX は、既定の曲(welcome-default)が使う内蔵のインストゥルメントである。FLEX が無いと、既定の曲を開いた時に「Problems loading the project」の箱が出る。WavPack(#{{227}})の読み込みが通ると、この箱が出るので、FLEX が無いと WavPack を本番に出せない。

## 背景

- 箱の正体(#{{227}} の作業者の調査): 不足は `Plugins\Fruity\Generators\FLEX\FLEX_x64.dll` と `Plugins\Fruity\Effects\Emphasizer\Emphasizer_x64.dll` の 2 つで、どちらも存在確認が 3(パスが無い)。Emphasizer は #{{043}} が受け持つ。この Issue は FLEX だけを受け持つ。
- FLEX は、第 2 波のどの束にも無い。理由: `inputs/plugin-classes.tsv` で、FLEX フォルダの DLL 2 本(補助)の VFX 列が `yes`(OpenGL の import)なので、`tools/plugin-layout/wave2-plugins.mjs` がフォルダごと外している。
- FLEX フォルダの DLL は 3 本(`inputs/fl-installer-files.tsv` と `inputs/plugin-classes.tsv` の行)。

| 役割 | guest | 大きさ | SHA-256 | VFX 列 |
|---|---|---|---|---|
| 本体 | `Plugins\Fruity\Generators\FLEX\FLEX_x64.dll` | 5,194,752 B | b5d2bc32c9113e17f9a53bb1dc880d9f5c3907ea9441d37e15999b5713bf7dfc | no |
| 補助 | `Plugins\Fruity\Generators\FLEX\engine_x64.dll` | 5,673,984 B | a45f66f386732aec35a0bf3d0fc09985d949662c255bdd2100a335e1c3db572a | yes |
| 補助 | `Plugins\Fruity\Generators\FLEX\engineV2_x64.dll` | 8,119,808 B | e4755e1430e2870c9947a221f53bf565ee646ce9e93030c507479b77995c0c86 | yes |

- 範囲: **本体 `FLEX_x64.dll` の 1 本だけ**を `lift/plugins/flex/plugin.tsv` に入れる。補助の 2 本は、この Issue の範囲に入れない。理由は 2 つ。
  1. `inputs/vfx-deferred.tsv` に、2 本とも「opengl32.dll を import」として在る。OpenGL を使う VFX 系は #{{039}} で後から入れる、と決めてある。
  2. 像の領域(`lift/crates/fe-build/src/plugins.rs` の `GLOBAL_BASE`)が、ほぼ一杯である。`inputs/plugin-image-sizes.tsv` の値から計算すると、第 2 波の 159 本と `lift/plugins/` の既存の行を合わせた 166 本の後の残りは約 17.4 MB(推定)。本体 1 本の SizeOfImage は 5,283,840 B(表の値。64 KiB 単位で 5,308,416 B)で、残りは約 12.1 MB になる。補助 2 本の SizeOfImage は表に無い(VFX の DLL は抽出していない)が、大きさから見て合わせて約 14 MB と推定するので、入らない見込み(推定)。
- 補助 2 本が無いと、本体 `FLEX_x64.dll` が instantiate できない可能性がある(推定。未確認)。この Issue の作業で確かめる。instantiate が補助の不足で止まったら、止まりの出力を報告に貼って止まる(下の「止まり方」)。補助を入れるかどうかは、報告を見て依頼者が決める。
- Plugin database の行は、本体の行から build が作る(`plugin_db_rows`)。行を手で足さない。
- `ci/extract-dlls.tsv` の 143 行(`FLEX_x64.dll` → `plugins/Plugins/Fruity/Generators/FLEX/FLEX_x64.dll`)は残す。Effector と同じ形(既存の行と、`lift/plugins/` の行が並ぶ)。
- 親: #{{022}}(内蔵インストゥルメント)の子。
- 範囲に入れない物: FLEX フォルダ直下の非 DLL の 8 件(`cache.json`・`data.json`・`missing.png`・`packbg.jpg`・`packtile.jpg`・`Plugin.nfo`・`styletags.txt`・`typetags.txt`。計 291,941 B)。`Data\Patches\Packs\FLEX\` の 16 件(計 94,228,947 B)。`Data\System\` の FLEX の 11 件(計 395,513 B)。`data.tsv` の書式は `Plugins\Fruity\<種別>\<フォルダ>\Data\` の下だけを認める(`app/scripts/lib/plugin-rows.mjs` の `validateData`)ので、これらは宣言できない。FLEX が既定の曲でこれらを読むかは、実行で確かめる(下の「止まり方」)。

## 入れ方

- `lift/plugins/flex/plugin.tsv` を 1 つ足す(見出しは 8 列: `key`・`guest`・`role`・`sha256`・`size`・`hook`・`args`・`omit`。区切りは TAB)。行は 1 つ(本体)。
- 見本: `lift/plugins/fruity-filter/plugin.tsv`(本体 1 本)、`lift/plugins/effector/plugin.tsv`(本体と補助の形)。
- 行の値(下の表を、そのまま書く):

```tsv
key	guest	role	sha256	size	hook	args	omit
generators.flex.flex_x64.dll	Plugins\Fruity\Generators\FLEX\FLEX_x64.dll	本体	b5d2bc32c9113e17f9a53bb1dc880d9f5c3907ea9441d37e15999b5713bf7dfc	5194752	rva:0x1b4b0,0x37930,0x37950	--pdata	-
```

- `hook`: `inputs/plugin-hooks.tsv` の FLEX_x64.dll の行(hook 点 3 つの RVA。確信は高)。手で推測しない。
- `args`: `--pdata`。`omit`: `-`(setjmp を呼ぶ関数の入口は、#{{126}} で入った自動の検出が決める。手で書かない)。
- 受け入れの前に `node tools/place-plugin-dlls.mjs` が、本体の DLL を `lift/out-app/host-extra/plugins/` から木に置く。`missing` が出たら、行の `guest`・`sha256` を `inputs/fl-installer-files.tsv` と照らす。

## 道具の前提(tools/plugin-run)

- `tools/plugin-run/run.mjs` の `effectRows()` は、key が `effects.` の本体だけを読む(#{{127}} は効果プラグインを対象にした)。FLEX は `generators.` なので、今のままでは `no effect plug-in named "FLEX"` で止まる。
- 直すのは、この 1 箇所だけ: `effectRows()` の判定を、key が `effects.` か `generators.` で始まる本体(role `本体`)を読む形にする。関数の名前(`effectRows`)は変えない(spec が使う)。ファイル先頭の説明(`//` の行)と、関数の注釈を、インストゥルメントも含む内容に直す。
- `spec/plugin-run.test.mjs` に、`FLEX` が `resolveNames` で解決されることの試験を 1 件足す(`fruity-balance` と同じ形)。
- 場面は scene (c) だけを使う(受け入れで `--scene c` を付ける)。scene (d) は、ミキサーのインサートのプラグインを作り替える場面で、インストゥルメントの要求は作れない。
- 1 ブロックの処理(`processed`)は、道具の既存の判定(`quantaWhilePlaying` が増える・エンジンの行に失敗が無い・idle に戻る)をそのまま使う。判定を変えない。

## fe-build の像の領域(lift/crates/fe-build/src/plugins.rs)

FLEX の行が入ると、既存の試験 `the_second_wave_is_160_dlls_and_fits_the_image_area` と `the_second_wave_with_its_data_fits_the_limits` が、次の理由で落ちる: FLEX のフォルダは表(159 本)に無く、先頭の束(`FIRST_WAVE`)にも無いので、`second_wave_dlls` が「the table is out of date」で止まる。ildsp(#{{253}})の行を入れたときと同じ形で、次の 2 箇所だけを直す。

1. `first_wave_folder()` の `FIRST_WAVE` に `r"Plugins\Fruity\Generators\FLEX"` を足す(配列の大きさを 8 から 9 に、doc comment に FLEX の Issue 番号を書く)。
2. その試験の件数: `read_rows_from(&state)` の期待値 166 → 167、`dlls.len()` の期待値 166 → 167。`wave_total` の期待値に、FLEX_x64.dll の 5,308,416 B(5,283,840 B を 64 KiB に切り上げた値)を足す。

この試験は、#{{254}} の直しが main に入るまで、`File exists` で赤い。この Issue の受け入れには含めない。作業の報告に、この試験の結果(赤か緑か、その出力の行)を書く。

## 注意

- 本体 1 本は 5.2 MB。lift が重い。`indirect call to … is not a known function entry` が出たら、`args` に `--pdata --reloc-seeds --rva-tables <file>` を足す(#{{232}} の仕組み。`lift/plugins/lameenc/plugin.tsv` と `rva-tables.txt` の作り方に倣う)。
- `fe-build` が `GLOBAL_BASE` の超過(`the image area is full`)で止まったら、像の領域に入らないのは補助を含めた行の数、と報告する。補助を外すのは、この Issue の範囲(既に外してある)。

## 受け入れ

```accept
test -f lift/plugins/flex/plugin.tsv && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && node tools/place-plugin-dlls.mjs && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "FLEX" --scene c > "$RUNNER_TEMP/plugin-run.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L.every((r)=>r.instantiated===true&&r.processed===true&&r.error===null&&r.engineStop===null&&r.elapsedMs<=20000)?0:1)' "$RUNNER_TEMP/plugin-run.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=20000&&r.preBootBytes<=314368711&&(!/^[BD]$/.test(r.flow)||r.welcomeClosed===true);process.exit(L.length===4&&L.map((r)=>r.flow).join("")==="ABCD"&&L.every(ok)?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && test -z "$(git diff --name-only origin/main...HEAD -- . ':!lift/plugins' ':!tools/plugin-run' ':!spec/plugin-run.test.mjs' ':!lift/crates/fe-build/src/plugins.rs')"
```

判定の基準:
- 起動の確認(`boot.json`): `state` が engine-idle、`standIn` が false、`error` が null、`elapsedMs` が 20,000 以下(DPR 1)。
- FLEX の 1 行(道具の出力は 1 行): `instantiated` が true、`processed` が true、`error` が null、`engineStop` が null、`elapsedMs` が 20,000 以下。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`)は 4 行。各行で `error` が null、`state` が engine-idle、`standIn` が false、`trapped` が false、`hostErrors` が 0、`opened` が true、`elapsedMs` が 20,000 以下、`preBootBytes` が 314,368,711 以下。B と D は `welcomeClosed` が true。
- 差分: `lift/plugins/` の外に変更が無い。ただし、`tools/plugin-run/run.mjs`・`spec/plugin-run.test.mjs`・`lift/crates/fe-build/src/plugins.rs` の 3 箇所は、上の節で許している。
- 箱の文言(流れ D の見出し `title`)は、この Issue の判定に入れない。Emphasizer(#{{043}})が残る間は、箱が出るので。箱の文言に FLEX の行が無くなったかは、読めたときだけ報告に書く(`tools/e2e/lib/common.mjs` の `guestText`)。

## 止まり方

| 止まり | 扱い |
|---|---|
| 読み込みの後の `ExitProcess`(終了コード 0) | `omit` の問題。`plugin.tsv` の `omit` は `-` のまま、自動の検出の結果を報告する。直せないなら止まる |
| instantiate が、補助 `engine_x64.dll`・`engineV2_x64.dll` の不足で止まる | 補助を入れない範囲(上の「範囲」)。止まりの出力(`tools/plugin-run` の JSON の `engineStop` の欄。その中の `lastFle` を含む)を報告に貼って止まる。補助の扱いは依頼者が決める |
| 既定の曲が FLEX の `data.json`・`cache.json` などを要求して止まる(`missing-path`・`CreateFileW: empty path`) | `data.tsv` では宣言できない(上の「範囲」)。要求されたファイルの名前を報告に書いて止まる |
| `indirect call to … is not a known function entry` | 上の「注意」の `--reloc-seeds --rva-tables` で直す。直らなければ止まる |
| `GLOBAL_BASE` の超過 | 補助を含めていないはずなので、止まる。表の値の計算(上の「範囲」)と、実際の出力を報告に書く |

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: (a) 追加した `plugin.tsv` の行の数(本体 1)、(b) `plugin-run` の 1 行の JSON、`elapsedMs` の値、(c) 流れ A〜D の 4 行、(d) 箱の文言が読めたか、(e) 補助 2 本を入れなかったことの確認(`fe-build` の像の領域の出力)、(f) `the_second_wave_is_160_dlls_and_fits_the_image_area` と `the_second_wave_with_its_data_fits_the_limits` の結果、(g) 止まりがあれば、その型と出力の末尾。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 製品の既定の動作を、測定のためだけに変えない。
- 共有のファイルを変えない。許すのは次の 4 つだけ: `lift/plugins/flex/plugin.tsv`(新規)、`tools/plugin-run/run.mjs`(`effectRows()` の 1 箇所と説明)、`spec/plugin-run.test.mjs`(試験 1 件)、`lift/crates/fe-build/src/plugins.rs`(上の 2 箇所だけ)。それ以外の変更は報告に書いて止まる。
- AI のベンダー名と Cloudflare の識別子を書かない。FL の製品名・インストール先の名前・変数名は書いてよい。
- Python・OCR を使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit の名義を変えない。commit・タグ・PR に、AI の署名の行を付けない。必要の無いブランチを作らない。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の開始時刻を `date -u` で記録する。`.prior-work.md` が在れば最初に読む。`git status` で作業ツリーを確かめる。
- ビルド(`fe-build engine`)は 1 回にする。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- 3 時間 15 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。`git add` は 1 つずつ行う。

## 他のタスクとの関係

- 段: 独立(依存は無い)。`lift/crates/fe-build/src/plugins.rs` と `tools/plugin-run/run.mjs` を、他の束も触るかもしれないので、commit の前に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直す。
- #{{227}}(WavPack)は、この Issue を blocked_by に持つ(起票の後に、依頼者側で設定する)。
- #{{043}}(Emphasizer)は、箱の残りを受け持つ。この Issue の受け入れには入れない。
