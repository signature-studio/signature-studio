# 第 2 波の 1 束の型(#1369・#1370・#1400 の経験から)

下書きは `pm-work/tools/gen-wave-plugins.mjs` が作る(試験は `gen-wave-plugins.test.mjs`)。対象は 130 フォルダ・159 DLL(本体 131)。Sytrus は #1400 で入ったので、下書き 45 束は 44 束になった(Effects 29・Generators 15。S 24・M 20・L 0。最大 6 DLL)。

## (a) 1 束が作る物

- `lift/plugins/<dir>/plugin.tsv` だけ(見出し 8 列。行は本文に全文)。`dir` はフォルダ名の slug。Effects と Generators で同名の 2 フォルダ(Fruity Wrapper・Patcher)は `<種別>-<名前>`。
- hook: 本体の 要・既存 は `inputs/plugin-hooks.tsv` の 3 RVA(121 行が `rva:`)。`inputs/plugin-hooks.tsv` は編集しない(132 本体の行が在る)。残りは `none`(38 行)。
- omit: `-`。setjmp の入口は #1370 の自動の検出が決める(手で書かない)。
- `ci/plugin-builtin-prefixes.txt` は編集しない(2 接頭辞が全フォルダを覆う)。`ci/extract-dlls.tsv` の #1321 の行(167 件)は残す(二重に取り出すが害は無かった。#1400)。
- Data: 551 ファイル・22 フォルダ(直下の `<フォルダ>\Data\` の下の、DLL でない行)。Data を持つ 19 束は、`lift/plugins/<dir>/data.tsv` で宣言する(#1478。下の (d) の 1、(f))。

## (b) 共有のファイルの編集: 0 行

受け入れの最後の判定が `git diff --name-only origin/main...HEAD -- . ':!lift/plugins'` の空を見る。束どうしは別のディレクトリに書くので衝突しない。Data の宣言は `lift/plugins/<dir>/data.tsv`(#1478 の形)で、共有の seed 表・`pack.args` には足さない。

要対応: `lift/crates/fe-build/src/plugins.rs` の試験 `the_second_wave_is_160_dlls_and_fits_the_image_area` は `lift/plugins/` を読み、159 行を足す。束の行が main に入ると key が重複して落ちる。受け入れは `cargo test` を走らせないので束は通るが、main の試験は落ちる。段 1 の前に、試験を直す(`lift/plugins/` の既存の行を数えない形)。`tools/plugin-layout/wave2-plugins.mjs` の dir の付け方(種別を前に付ける)は、この型と違う。試験の表としてだけ使う。

## (c) よく出た停止と直し方の型

| 停止 | 例 | 直し方 | 束で直すか |
|---|---|---|---|
| 読み込みの後の `ExitProcess`(終了 0) | Balance(#1372。omit 0x2c13a0)、Filter(#1370。0x2cdf20)、Sytrus(0x48cb70) | setjmp の入口を omit に | はい(plugin.tsv) |
| 不明な DLL の読み込み(`LoadLibraryW` の失敗) | imm32.dll(#1370。仮想モジュール)、ILSvg_x64.dll(#1395) | runtime の仮想モジュール(共有) | いいえ(報告) |
| `indirect call to … is not a known function entry` | switch 表(#1413 済)・null の呼び出し(#1416 済) | lift(共有) | いいえ(報告) |
| `GetFileVersionInfoSizeW` などの、ディレクトリ部の無い名前 | #1415(未完) | runtime(共有) | いいえ(報告) |
| 環境変数の展開・`fle_stop` の行 | %FLStudioFactoryData%(#1412。未完) | runtime(共有) | いいえ(報告) |
| `CreateFileW: empty path`(Data の欠落) | Sytrus の Data(#1414。seed 表に 109 行を足した) | 共有の seed 表 | いいえ(下の (d) の 1) |

## (d) PM の判断が要る点

1. **Data(551 ファイル)**: 解決: #1478 が `lift/plugins/<dir>/data.tsv` で宣言する形を作る(共有の seed 表には書かない)。Data を持つ 19 束は、本文に Data の節と受け入れを足し、#1478 を blocked_by に足した(下書き 192)。入れ子の Data の扱いは PM の判断待ち(下の (f) の 4)。
2. **段**: 段 1 の 15 束は、#1370・#1371 が閉じると同時に走る。段 2 は段 1 の先頭 3 束、段 3 は段 2 の先頭 3 束が閉じるまで待つ。
3. **インストゥルメント(Generators)**: #1371 の道具は効果だけ。Generators の受け入れは `processed` を見ず、`instantiated` と `error` を見る。道具がインストゥルメントを要求できるかは未確認(段 2 の前に確かめる)。
4. **列挙の対応**: `requested` は #1371 の出力の `instantiated` に当てた。`engine-idle` は boot-bundle と welcome-flow の `state`。`stops` は、どの道具の出力にも無いので判定に入れていない。
5. **禁止の数値**: 6 件の SHA-256 の 16 進の中に、禁止の数値の並びが含まれる。禁止の検査は散文だけにし、表の SHA 列は除く。

## (e) 所要の見積もり(推定。実測ではない)

第 1 波の実測から: `fe-build engine` は 23 から 35 分(9 モジュール)。1 モジュールの追加は約 1〜1.5 分(hybrid-n0・hybrid・lists・単位のコンパイル)。6 DLL の束で +10 分弱。道具・起動の確認・流れ A〜D は約 10 分。1 回の試行は約 60 分の見込みで、4 時間 30 分の枠に 2〜3 回入る。

## (f) Data の件数の照合(一覧 616 件と、下書き 551 件)

1. 一覧(`inputs/fl-installer-files.tsv`)の `Plugins\Fruity\` の `\Data\` の行は 725 件。Sytrus(109 件。#1400・#1414)を除くと 616 件。下書きの 551 件は、フォルダ直下の `<フォルダ>\Data\` の下の行のうち DLL でない物(生成器の `data()`。dry-run が内訳を出す)。この数え方は #1478 の定義(`Plugins\Fruity\<種別>\<フォルダ>\Data\`)と合うので、生成器は直さなかった。
2. 差の 65 件: VFX の後回し 38 件(Fruity Slicer 2。Slicex の Data と同じ 38 件・16,856 B)、入れ子の Data 26 件(`Vocodex\Synthesizer\Data\`。Default.dat 1 件と Presets\*.fst 25 件・55,452 B)、本体・補助の DLL 1 件(`Edison\Data\Scripts\Examples\testdll_x64.dll`。plugin.tsv の補助)。
3. 拡張子と Presets は差の原因ではない: 直下の Data(551 件)にも .fnv・.bmp・.wv・.txt・.groovepat などの資料と、`Data\Presets\` の 4 件があり、同じ数え方で入る。差の 26 件の入れ子だけが .fst の Presets を含む。
4. 未決(PM): 入れ子の `Vocodex\Synthesizer\Data\`(26 件・55,452 B)は、本体 `Synthesizer\Sytrus_x64.dll` の隣の Data。この作業の本文では宣言に含めず、報告に書かせる。含めるなら、Vocodex の Data は 54 件・計 443,736 B になり、#1478 の本文の 551 件・約 9.4 MB と、#1478 の件数の見積もり(577 件へ)を直す必要がある。
5. 同じ内容の重複: Slicex(生成器の束)と Fruity Slicer 2(VFX の後回し)は、相対パスと SHA-256 が同じ 38 件を持つ。後回しを解くとき、同じ SHA-256 が 2 つの宣言に入るので、#1478 の検査でその扱いを確かめる。

## (g) Vocodex の入れ子の Data(PM の決定)
Vocodex の `Synthesizer\Data\`(26 件・55,452 B)は、入れ子の本体 `Sytrus_x64.dll` の Data で、同じプラグインの中で要求される。#1461 の `lift/plugins/vocodex/data.tsv` に含める(54 行・443,736 B)。#1461 の本文は直した。下書き 068 は再生成すると戻るので、再生成する時は、この節に合わせて直す。

## (h) 木に無い DLL(#1370 の連鎖の対策)

- 束の受け入れは、`fe-build engine` の前に、作業の木へ自分の束の DLL を置く(`node tools/place-plugin-dlls.mjs`。`plugin-builtin` の作業は `lift/out-app/host-extra/plugins/` から sha256 で照合して複写する)。束の行は同じ job で足すので、job 開始時の抽出の木には無い。第 2 波の 041 の本文の受け入れ(`pm-work/issue-drafts/wave2/041-effects-01.md`)は、この手順を持たない(直していない。推定では、作業者の受け入れは plugins の段で止まる。実測ではない)。
- 判定(`guard/judge.sh`)は、受け入れの前に同じ道具を呼び、作業の木にある他の束の DLL も置く。判定は main を取り込まない(取り込みは push の段の pull だけで、その後に受け入れは走らない)ので、置くのは作業ツリーの行の分。道具が 1 で終わっても判定は続き、受け入れの結果で決まる。
- 限界: `host-extra/plugins` の外の DLL(`Shared\` など)は置けない(skipped)。その行の DLL は、抽出の木の作り直しでしか入らない。
- 第 2 波の 44 件(#1434〜#1477。下書き 041〜084)の accept に `node tools/place-plugin-dlls.mjs && ` を `fe-build engine` の直前に 1 回入れ、本文の「1 プラグインの入れ方」に、その手順の 1 行を足した。生成器・テンプレート・下書き・試験(`spec/gen-wave-plugins.test.mjs`)も同じ。
- 作業者の受け入れは、この手順が accept の式に在るので、木に無い束の DLL を `fe-build engine` の前に自分で置く(判定の前の呼び出しとは別に、受け入れの式でも置く)。`plugin-builtin` の取り出しが無い作業では置けないが、0 で続く(`missing`・`mismatch` が無ければ)。
