---
draft: "154"
title: "FL を英語以外の言語で起動できる: ILSvg_x64.dll を real module にし、言語のファイル(.moe・.svg)を seed にする"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["125","142"]
labels: ["resources","verify"]
size: L
priority: High
project: signature-studio/3
---

## 目的

依頼者の指摘(2026-11-24。原文): 「言語を切り替えると再起動を求められますが(これ自体は正常な挙動)、再起動しても設定が反映されていません。」

#{{142}} で、言語の選択(レジストリの値)は、ページの再読込をまたいで保持されるようになった。しかし、英語以外の言語で起動すると、engine が止まる。この Issue の目標は 2 つ。
1. 英語以外の言語(配信の 7 言語: de・es・fr・ja・ko・vi・zh)で起動して、engine が止まらず `engine-idle` に届くこと。
2. 選んだ言語で、画面の言語の欄が英語の時と異なる画素になること(言語が画面に反映されること)。

## 背景(#{{142}} の実測。run 45364912379)

- 言語の選択の保存は動く。保存先は `HKCU\Software\<ベンダー>\Shared\Language` の値 `Program language`(REG_SZ)。再読込の後の起動で、`RegQueryValueExW "Program Language" -> found type=1 size=6 value="es?"` が読まれた。
- 英語以外の言語で起動すると、engine が止まる。`es`・`vi`・`fr`・`ja` の 4 回とも同じ止まり方: `missing-path: LoadLibraryW 126 "ILSvg_x64.dll"` の後に `fle: indirect call to 0 is not a known function entry`。
- 原因(実測): この bundle は `ILSvg_x64.dll` を lift していない。インストーラの一覧(`inputs/fl-installer-files.tsv`)には `Shared\ILSvg_x64.dll`(1,504,256 B。SHA-256 は一覧の値)と、各言語の `System\Languages\<code>.moe`・`.svg` がある(en は `.svg` のみ)。
- 言語のファイルの seed: リポジトリの seed 表(`lift/harness/seeds/vfs-seed-idle1.tsv`)には、`System\Languages\` の 7 言語の `.moe`・`.svg` の行が在る(#{{105}}。ホスト側は `resources/System/Languages/` の行)。#{{142}} の実行の記録では、bundle の seed に入ったのは英語の `en.svg` だけだった。なぜ入らなかったか(`resources` の復元と、seed の作り)は、この Issue で確かめる。
- #{{142}} の安全策(暫定): エンジンが異常に終わった回は、保存された木を捨てる(`app/src/engine/worker.ts` の `closeRegState`)。この Issue が英語以外の言語の起動を直せば、安全策が要るかを報告に書く。
- #{{142}} の 2 回目の試行(run 45409012582)。英語以外の 7 言語のうち、`ILSvg_x64.dll` の停止は es・fr、`CompareStringW` の停止は de・ko・zh・ja・vi。止まる理由は言語と回で変わる。

## 前回の試行で分かったこと(run 45504133372)

- 判定は差し戻し(失敗の段: 判定)。受け入れは未達。7 言語のうち engine-idle になったのは ja の 1 言語だけ。作業は work.diff に保存された。作業者の commit は 2 本(`d8ca7a60`・`90db6f74`。作業者の環境。push していない)。次の試行は、その差分を main の上に取り込んで続ける。runner の値。nproc 4、loadavg 1.0〜2.5。
- 受理(実測): 英語の起動(`boot-bundle.mjs`)は `engine-idle`・`standIn` false・`error` null・`elapsedMs` 8,158。ja(`measure.mjs --language ja`)は `engine-idle`・`languageKept` true(352 画素)。流れ A〜D は 4 本とも `engine-idle`・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true、B と D は `welcomeClosed` true。単体は `npm test --prefix app` 1,401 件、`cargo test -p fe-build` 20 件、`fe-lift --test locale` 39 件が通る。
- 差し戻しの停止(6 言語。`exited`・`languageKept` false): de・es・fr・ko・zh は `fle_addr_hi32`(`guest address 0x3280983d0 (8-byte load) has bits above 32 set`。ILSvg の内部で、64 ビットの値が番地として読まれる。どの関数かは未特定)。vi は `GetFileAttributesW: wildcard in a path that does not allow one: component "OpenSans-Th?ng thu?ng.ttf"`。
- 前回の停止を 2 つ直した(前進):
  - 間接呼び出し `0xc683c80`(RVA 0x103C80)は、`.pdata` に無い葉の関数で、`.rdata` の関数ポインタ表から参照されていた。decision 0074 の規則では入口にならなかった。`fe-wasmgen --reloc-seeds`(`.pdata` に無い再配置の値を入口にする)で直した。ILSvg の直接の関数は 3,221 → 3,465。
  - 間接呼び出し `0xc670041`(RVA 0xF0041)は、MSVC の memcpy のサイズ分岐の表(`lea r9,[ImageBase]` を基準にした 32 ビットの相対値の表)の飛び先。`--rva-tables`(表の一覧は `lift/plugins/ilsvg/rva-tables.txt`)で、表の飛び先を入口にした。直接の関数は 3,573。
- 残り: `jmp reg` の 73 件のうち、一覧にあるのは 40 件だけ。残り 33 件は表の形を認識できず、未確認(やること 10)。
- fe-wasmgen の欠陥を直した: `movmskps`・`movmskpd`・`pmovmskb` の 64 ビット宛先が i64 に拡張されず、ILSvg の 21 関数が wasm の型検査で落ちていた。
- 数値(runner の値。ILSvg 無しの同じ機械は 165,630,234 B。#{{142}} の記録): engine.wasm 166,375,864 B(+745,630 B)。ILSvg の wasm(lazy)は 7,695,230 B。lazy は 92 件(上限 128 の内。`maxItems` は変えていない)。seed pack は 372 件(前は 386 件)。`left_out` 85 件に、7 言語の `.moe`・`.svg` の 14 件が入る(lazy)。`en.svg` は pack に入る。
- seed の言語の行は 16 行(7 言語×2、`en.svg`、フォルダの行)。#{{142}} の記録の「`en.svg` だけ」は、この木では再現しなかった。
- ILSvg の実物: この作業環境にはインストーラが無い。作業者は `lift/out-plugins/ilsvg_x64.dll` を複写し、SHA-256 `92f76fef…` で照合した(commit されない)。CI の extract 段が同じ物を作るかは未確認。
- `plugin.tsv` の `args` は空にできず、`--pdata --reloc-seeds --rva-tables lift/plugins/ilsvg/rva-tables.txt` になった(ビルド道具の引数。製品の旗ではない)。`ci/extract-dlls.tsv` の ILSvg の行は作業者が外した(作業者の commit の中。`ci/` が変わったので、push されると `deploy.yml` が動く)。
- 「前提」の節の「作業者は `ci/` を変えない」は、d29eaf6a の前の文。前提が満たされた今は、やること 1 の手順(同じ commit で `ci/extract-dlls.tsv` の行を消す)に従う。
- best-fit の表(やること 6): 出所は `lift/harness/README.md` の「bestfit1252」節(作業者の commit の中)。取得日 2026-11-24、取得物 37,057 B、SHA-256 `72ea23c9…`。WCTABLE 698 行から MBTABLE で戻る 256 行を除いた 442 行。テスト `bestfit_1252_table_is_the_wctable_rows_that_do_not_map_back` が照合する。`Intl.Collator(en-US)` と Windows の sort 表の一致は未確認。
- vi の名前の `?` は、表示の欠落か、1252 に無い文字の変換の結果か未確認(やること 9)。

## その前の試行で分かったこと(run 45458518648)

- 判定は差し戻し(失敗の段: 判定)。作業は保存された。次の再試行で、その差分を取り込んでから始める。作業者の commit は `ea95d873`(作業者の環境。push していない)。runner の値。nproc 4、loadavg 1.4〜2.1。
- 英語の起動(`boot-bundle.mjs`): `engine-idle`・`standIn` false・`elapsedMs` 8555。受理。
- ja(`measure.mjs --language ja`): `engine-idle`・`standIn` false・`languageKept` true(352 画素)・`restartPrompted` true・`registryKept` true・`error` null・`elapsedMs` 7467・`preBootBytes` 304,836,976。受理(この 1 言語だけ)。
- de・es・fr・ko・zh: 再読込の後に `missing-path: LoadLibraryW 126 "ILSvg_x64.dll"` で止まる(`stopReason` が `missing-ilsvg`)。作業者の機械にはインストーラが無く、ILSvg の DLL を作れなかったため。
- vi: `WideCharToMultiByte` の U+01B0 が範囲 D の外で止まる(2 回とも)。やること 6 で直す。
- 流れ A〜D: 4 回とも `engine-idle`・`error` null・`trapped` false・`hostErrors` 0・`welcomeClosed` true・`opened` true(4276〜4391 ms)。受理。起動の遅れが 1〜2 回出たが、流れが再読込で回復した。
- 保存された差分の内容: (1) ILSvg の検査の、`Shared\` の 2 段の guest の許可(Rust と JS の両方)。(2) seed 表の 14 行を `@lazy-file:` にし、`pack.args` に 14 組。`en.svg` は seed のまま。(3) `CompareStringW` の、モデルが決めない組の停止だけを、ページの `Intl.Collator(en-US)` の答えにした(経路: `fle_cmp_page`(弱い参照)→ `lift/harness/fruity_idle.c` → `fle_js_collate`)。既存の決定済みの答えは変えていない。(4) `tools/persist/measure.mjs` の `state`・`standIn`・`languageKept`(画素 30 以上で true)。
- 大きさ: `engine.wasm` は 165,630,692 B(ILSvg 無しの同じ機械のビルドは 165,630,234 B。増分 +458 B)。lazy の件数は 89(ILSvg を足すと 92。上限 128 の内)。seed pack は 386 件から 372 件に減った(言語の 14 件)。
- 言語のファイルが seed に入らなかった原因: 前回の木の seed 表は、言語の 14 行を入れていた(pack-seed は 15 件すべてを入れた)。#{{142}} の記録(`en.svg` だけ)は再現しなかった。原因は未確認(やること 3)。
- 安全策 `closeRegState`: 前回の作業は残した。7 言語のうち、再読込で起動するのは ja だけで、残りは止まった。外すのは、7 言語が実 DLL で `engine-idle` になった後。
- 未確認: `Intl.Collator` と Windows の sort 表の一致。`languageKept` の 30 画素の閾値(ja の 1 件だけで確かめた)。zh の初回の起動が、一度 40 秒を超えて止まった(再現していない)。
- 受け入れ(7 言語)は、どの回も通っていない。前回の run の plugin の段は、`lift/out-app/host/lift/out-plugins/ilsvg_x64.dll` が無いため `No such file or directory` で止まった(下の「実物の置き場と前提」)。

## 読む物

- `inputs/fl-installer-files.tsv`(`Shared\ILSvg_x64.dll`・`System\Languages\` の行)
- `ci/resources-prefixes.txt`(`System\Languages\` の接頭辞)、`.github/workflows/agent.yml` の resources の段(言語の取り出しは、`resources` のラベルの作業だけで行われる)
- `ci/extract-dlls.tsv`(ILSvg の行と、その注釈)
- `lift/harness/seeds/vfs-seed-idle1.tsv`(`System\Languages\` の行)
- `lift/plugins/`(#{{125}} の形。見本は `lift/plugins/fruity-balance/plugin.tsv`)
- `lift/crates/fe-build/src/plugins.rs`(`host_file()`: plugin の段が読む場所。`validate`)、`app/scripts/lib/plugin-rows.mjs`
- `app/src/bundle/manifest.ts`(lazy の配列の `maxItems`)
- `lift/runtime/fle_locale.c`(`fle_imp_GetACP`・`fle_wide_to_mb_1252`・`fle_imp_WideCharToMultiByte`・範囲 D)、`lift/crates/fe-lift/tests/locale.rs`
- `tools/persist/measure.mjs`(#{{142}} の道具)、`tools/smoke/welcome-flow.mjs`

## やること

0. 作業ツリーの確認。`.prior-work.md` があれば最初に読む。保存された前回の差分(上の「前回の試行で分かったこと」の (1)〜(4))が取り込まれているかを確かめる。二重になっていないことを `grep -c` で確かめる。衝突の印が在れば解消する。
1. `ILSvg_x64.dll` を real module にする。`lift/plugins/ilsvg/plugin.tsv` を 1 つ足す(見本は `lift/plugins/fruity-balance/plugin.tsv`)。列は 8 個: `key`・`guest`・`role`・`sha256`・`size`・`hook`・`args`・`omit`(`omit` は `-`)。`guest` は `Shared\ILSvg_x64.dll`、`role` は `本体`、`sha256` と `size` は `inputs/fl-installer-files.tsv` の値(`size` は 1504256)、`hook` は `none`、`args` は空。`key` は `<種別>.<フォルダ>.<DLL の名>` の小文字(`shared.ilsvg.ilsvg_x64.dll`)。共有の表(`modules.tsv`・`pack.args` など)は手で編集しない。`guest` の 2 段(`Shared\`)は、main の検査では拒否される(`app/scripts/lib/plugin-rows.mjs` の `validate` と `lift/crates/fe-build/src/plugins.rs` の `validate` は、3 段未満の guest を拒否する。2026-11-25 に、仮の plugin.tsv で確かめた)。main に 2 段の許可は無い。許可は作業者が `app/`・`lift/` で足す(保存された差分を取り込むときも、両方に許可が在るかを確かめる)。2 段のときの seed の場所は、両方の計算で `lift/out-idle1/inputs/install//ILSvg_x64.dll`(フォルダの部分が空になる)になるので、作業者が直す。
   - 前提の確認: `test -f lift/out-app/host/lift/out-plugins/ilsvg_x64.dll`(下の「実物の置き場と前提」)。前提が無いときは、この行の plugin.tsv の commit は受け入れを通らない。その場合は、2〜7 の作業を進めて commit し、前提が無いことを報告に書いて終える。
   - 前提が在るときは、plugin.tsv を足す同じ commit で、`ci/extract-dlls.tsv` の ILSvg の行(その注釈 1 行を含む)を消す。
2. 言語のファイル(7 言語 de・es・fr・ja・ko・vi・zh の `.moe`・`.svg`)の lazy 化は、保存された差分で済んでいる(seed 表の 14 行が `@lazy-file:`、`pack.args` に 14 組)。lazy の件数は 89(ILSvg を足すと 92)。上限 128 の内なので、`maxItems` は変えない。言語のファイルは、インストーラの一覧(`System\Languages\`)から生成し、コードに言語の名前を定数として書かない。
3. seed に言語のファイルが入らなかった原因(#{{142}} の記録の `en.svg` だけ)は、前回の木では再現しなかった。生成した seed の表と、pack-seed の出力で、7 言語の `.moe`・`.svg` が入ることを確かめ、報告に書く。入らない物が在れば直す。
4. `CompareStringW`(`lcid=0x400`。`LOCALE_USER_DEFAULT`): 保存された差分の答え(モデルが決めない組の停止だけを `Intl.Collator(en-US)` にした)を確かめ、残す。`Intl.Collator` と Windows の sort 表の一致は未確認なので、報告に書く。停止の行(`fle: CompareStringW: lcid=… string1=… string2=…`)が 7 言語の起動に出ないことを確かめる。`CompareStringEx`・`CompareStringOrdinal`・`LCMapStringW`・`lstrcmpiW` は、停止の行に出た物から順に(今の 7 言語では出ていない)。
5. 測定の道具(`tools/persist/measure.mjs`)の `state`・`standIn`・`languageKept` は、保存された差分で入っている。`languageKept` は、画面の言語の欄の画素を、英語の時と比べて出す。
6. vi の `WideCharToMultiByte`(U+01B0 が範囲 D の外で止まる)。PM の決定: Windows の既定は best-fit(`WC_NO_BEST_FIT_CHARS`(0x400)が無い時)。`?` にはしない。
   - 実行時のコードページ(`GetACP` が返す値。`lift/runtime/fle_locale.c` の `fle_emulated_locale.acp`。既定 1252)の best-fit の表を、Unicode コンソーシアムが公開しているコードページの best-fit の表(1252 の分)から作る。取得は一度だけ(再試行つきの `curl`。取得した物の SHA-256 を記録する)。出所・取得の日付・SHA-256 は `lift/harness/README.md`(無ければ作る)に書く(`ci/versions.env` には書かない)。取得元の URL も README に書く(本文とコードには書かない)。
   - 表は、生成したデータのファイル(`lift/runtime/` の C の表。`fle_c1_latin1` と同じ形)として commit する。ビルドの中で取りに行かない。
   - 1252 の経路(`fle_wide_to_mb_1252`): `dwFlags` に 0x400 が無いときは、表の best-fit の答えを返す(例: U+01B0 → `u`)。0x400 が在るときは、今の完全な逆変換だけを返す(既存の挙動)。表に無い文字は既定の文字(`?`)にする。範囲 D の外で止まらない(この経路だけ。`GetStringTypeW`・`LCMapStringW` の範囲 D は変えない)。
   - `lift/crates/fe-lift/tests/locale.rs` が範囲 D の外の停止を期待していたら直す。`cargo test -p fe-lift --test locale`(36 件)と `cargo test -p fe-build` が通ること。
7. 報告に、`engine.wasm` の大きさの増分、`ILSvg_x64.dll` の wasm の大きさ、lazy の件数、選んだ案と理由、best-fit の表の出所と SHA-256(README の行)、`Intl.Collator` の未確認の点、`closeRegState` の扱いを書く。
8. **停止の記録の診断(PM の決定。#{{146}} と同じ)**: `fle_addr_fault`(`lift/runtime/fle_host.c`。64 ビットの値を番地として読んだ停止。文言の見出しは `[fle_addr_hi32]`)が起きたとき、停止の記録として**常に**、呼び出し元の戻り番地・`R[0..15]`・最も近い名前つきの関数を出す。目的は、次の実機の停止で原因が分かること。環境変数・`#ifdef`・URL パラメータを足さない。#{{146}} の作業者も同じ診断を入れる。両方の差分が衝突したら、両方を残す。
9. **vi の停止(PM の決定)**: ANSI のコードページは 1252 のまま(`GetACP` の答えを変えない)。停止の名前の正体を、**UTF-16 の 16 進**でログに出して特定する(`?` は表示の欠落のため)。`inputs/fl-installer-files.tsv` に、その名前の行が在るかを確かめる。在れば、その名前のまま(UTF-16)で `CreateFileW`・`FindFirstFileW` が答えられるかを確かめる。確かめる先は、seed の名前の符号化と、`lift/harness/web/pack.args` の URL の `%XX`。
10. **switch 表(別の Issue)**: ILSvg の残りの `jmp reg` 33 件の表(MSVC の、ガードの無い switch 表)は、この Issue では扱わない。switch 表の認識の拡張として、別の新しい Issue で扱う(PM が起票する。番号は未定)。`lift/plugins/ilsvg/rva-tables.txt` の 40 件の手作業の列挙は、それまでの暫定。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && for c in de es fr ja ko vi zh; do node tools/persist/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --language "$c"; done > "$RUNNER_TEMP/persist.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l)).filter((x)=>"language" in x);const want=["de","es","fr","ja","ko","vi","zh"];const ok=(r)=>r.error===null&&r.restartPrompted===true&&r.state==="engine-idle"&&r.standIn===false&&r.languageKept===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===7&&want.every((c)=>L.some((r)=>r.language===c))&&L.every(ok)?0:1)' "$RUNNER_TEMP/persist.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- `tools/persist/measure.mjs` の出力は 7 行(言語ごとに 1 行。`language` は de・es・fr・ja・ko・vi・zh の全て。各 1 回。DPR 1)。受け入れの command が、`--language` で 7 回実行して 1 つのファイルに集める。
- 各行で、`error` が null、`restartPrompted` が true(言語を変えると、再起動の案内が出る)、`state` が `engine-idle`(再読込の後に止まらない)、`languageKept` が true(画面の言語の欄の画素が、英語の時と異なる。前回の画素の比べ方と同じ)、`standIn` が false。
- `elapsedMs` が 12,500 以下。`preBootBytes` が 314,368,711 以下(言語のファイルは起動の後の取得。起動の前に取る量を増やさない)。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`)が通る。単体テスト(`npm test --prefix app`)が通る。
- 報告に書く: `engine.wasm` の大きさの増分、`ILSvg_x64.dll` の wasm の大きさ、lazy の件数、seed に言語のファイルが入らなかった原因(生成した seed の表の確認の行)、選んだ案と理由。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に、安全策(`closeRegState`)を残すか外すかを書く。
- 報告に、best-fit の表の出所(README の節)と SHA-256、`Intl.Collator` の未確認の点を書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 言語の名前や値を、runtime・app・テストに、定数・表・分岐として入れない。言語のファイルの一覧は、インストーラの一覧から生成する。測定の道具の入力(7 言語の一覧)は例外。
- ベンダー名と Cloudflare の識別子を書かない。取得元の URL を本文とコードに書かない(README に書く)。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。
- 名前の扱い(依頼者の訂正): ファイルに書いてはならないのは、AI のベンダー名と、Cloudflare の識別子だけ(`guard/scan.sh` が検査する)。FL の製品名(FL Studio など)・`Image-Line` のインストール先の名前・`%FLStudioFactoryData%` などの変数名は、書いてよい。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大の 5400000 にする。背景で起動して待たない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。
- この Issue は 1 回の試行で終わらない可能性が高い(ILSvg の前提、7 言語の確認、best-fit の表)。

## 他のタスクとの関係

- blocked_by: #{{125}}(プラグインの 1 ディレクトリの形)と #{{142}}(言語の選択の保存)。両方が閉じた後に起動する。
- #{{142}} の安全策(異常終了の回は保存を捨てる)は、この Issue の結果で要否を決める。
- #{{122}}(lazy の上限を 128 にした)の件数に影響する。
- #{{160}}(段階 2 の素材の配信)も、seed 表と `pack.args` を変える。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- switch 表の認識の拡張(ILSvg の残りの `jmp reg` 33 件)は、別の新しい Issue で同じ形を扱う(PM が起票する)。この Issue の `rva-tables.txt` は、それまでの暫定。

## 実物の置き場と前提(PM の対応)

- `Shared\ILSvg_x64.dll` の実物は、`ci/extract-dlls.tsv` の ILSvg の行(main の d29eaf6a で、抽出先を直した)で、作業ツリーの `lift/out-app/host/lift/out-plugins/ilsvg_x64.dll` に取り出される。lift はされていない。(0610ae87 の時点では `lift/out-app/host/Shared/ILSvg_x64.dll` に取り出されていた。)
- ただし、plugin の段(`lift/crates/fe-build/src/plugins.rs` の `host_file()`)が読むのは `lift/out-app/host/lift/out-plugins/ilsvg_x64.dll` で、`Shared/` の下の物は読まない。前回の run は、この段で `No such file or directory` で止まった。
- 前提(d29eaf6a で満たした。作業者は `ci/` を変えない): `lift/out-app/host/lift/out-plugins/ilsvg_x64.dll` が取り出されること(`ci/extract-dlls.tsv` の ILSvg の行の抽出先は `lift/out-plugins/ilsvg_x64.dll`。`fruity_filter_x64.dll` などの行と同じ形)。前提が満たされていないと、plugin.tsv を足した作業は、その run の中では受け入れを通らない(作業ジョブは、ベースの commit から取り出した木を使う)。
- 前提が満たされた後は、plugin.tsv を足す同じ commit で、同じ DLL の `ci/extract-dlls.tsv` の行を消す(二重に取り出さないため。`plugin-rows.mjs`・`plugins.rs` は `ci/extract-dlls.tsv` を読まないので、重複は検査されない)。
- `plugin.tsv` の行が main に入った後の run からは、取り出しの表は `ci/extract-dlls.tsv` と `plugin.tsv` の行の和になり、plugin の行の DLL は `lift/out-plugins/` に取り出される。
