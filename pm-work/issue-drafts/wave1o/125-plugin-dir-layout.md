---
draft: "125"
title: "プラグインの追加を、共有のファイルの編集ではなく、1 プラグイン 1 ディレクトリ(lift/plugins/<名前>/)の追加だけにする"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["delivery", "plugin-builtin"]
size: XL
priority: High
project: signature-studio/3
---

## 目的

第 2 波(内蔵プラグインの約 45 束)を並行に動かすための基盤。1 プラグインを足す作業を、共有のファイル(表・引数・一覧)の編集から外し、`lift/plugins/<名前>/plugin.tsv` を 1 つ足すだけにする。共有の物は、プラグインの一覧(ディレクトリの走査の結果)から、ビルドが生成する。

この Issue で、Fruity Balance(#1342 の分割)を、この形に移す。動作は今と同じにする。

## 背景(状況。実測)

- 1 件(Fruity Balance)を足した変更は、commit `212eaf59`(#1342 の途中)。`git show --stat 212eaf59` で 16 ファイル。その commit の中で、1 プラグインごとに同じ形で編集された共有のファイルは次の 7 つ:
  - `lift/harness/hyb/hyb-args.tsv`(1 行)
  - `lift/harness/hyb/hybrid.cmd`(`--separate balance` と `--separate-dir` の 4 行)
  - `lift/harness/hyb/lowered-all.txt`(12,282 行)
  - `lift/harness/hyb/lists/lowered-balance.txt`(12,282 行)
  - `lift/harness/web/pack.args`(2 行。`--args` の 1 行の中の語を含む)
  - `lift/crates/fe-build/tests/plan/engine.txt`(7 行。`stage compile (... total 170)` の件数を含む)
  - `lift/crates/fe-build/tests/plan/bundle.txt`(2 行)
- 1 度だけ必要な共有の変更(プラグインごとには要らない): `app/src/engine/fruity.ts`・`app/src/engine/xmod.ts`(新規)・`lift/crates/fe-wasmgen/src/hybrid.rs`・`lift/crates/fe-wasmgen/src/main.rs`・`lift/harness/two/harness-inputs.txt`・`lift/harness/two/main_two.c`・`lift/harness/two/xmod_glue.c`(新規)・`lift/harness/web/fruity/fle_idle_lib.js`・`tools/delivery-split/measure.mjs`。
- 45 件を並行すると、同じ行(`pack.args` の `--args` の行、`hybrid.cmd` の `--separate` の行、`engine.txt` の件数、`lowered-all.txt`)を同時に編集し、衝突でほぼ全てが失敗する(#1341 と #1342 の試行の先例)。
- `lift/harness/hyb/lowered-all.txt` は、プラグインの関数の一覧を全部持つ。Balance と同じ 1.2 万行の物が 132 本の本体に在ると、約 162 万行(推定。1 本ごとの実数は、この Issue で測る)。リポジトリが膨れる。
- 第 2 波の対象は 160 本の DLL(本体 132 本)。`inputs/plugin-classes.tsv` の VFX 系の 7 フォルダ(19 本)と、第 1 波の 3 本を除く。生成器(`pm-work/tools/gen-wave-plugins.mjs --dry-run`)の値は、フォルダ 131(Effects 85・Generators 46)、束 45。DLL のファイルの大きさの和は 608,521,016 B(本体 576,925,696 B)。

### 第 1 波の残り(Filter と Delay 2)の現状

- `lift/harness/b2d2/modules.tsv` に Fruity Filter(0x7000000)と Fruity Delay 2(0x7800000)の行が在る。gen-all で C に下ろされ、`engine.wasm` の中に入る。
- `lift/harness/hyb/hyb-args.tsv` に、この 2 つの行は無い。分けた wasm になっていない。`lift/harness/web/pack.args` の `--lazy-dll` で、DLL のファイルだけを起動後に取得している(#1319 の結果)。つまり、#1319 で届いたのは DLL のファイルで、コードは engine.wasm に残る。

## 1 回目の試行で分かったこと(run 45331544653。`failed`。約 2 時間 27 分)

- 作業ツリーの状態: 前回の作業(前回の作業者の commit の内容。main には無い)は、取り込みの段が `git apply --3way` で未 commit のまま載せ、`.prior-work.md` に書く。今の main への当たりは、`lift/crates/fe-build/src/main.rs` の 1 箇所だけが衝突する(#1362 が `HARNESS_DIR` の直後に定数を足したため)。他の 21 ファイルは衝突せずに当たる。衝突の印を解消してから受け入れを行う。
- 抽出: 前回の作業は Balance を静的な表から消し、`lift/plugins/fruity-balance/plugin.tsv` を 1 行足した。抽出は main の `app/scripts/extract-fl.mjs` が plugin.tsv の行を読む(main の 617e8339)。
- 受け入れ(runner の値。nproc 4。loadavg 1.44〜1.69)。通った: npm build・fe-build(7 件)・fe-wasmgen・guard.test・(1) engine-idle・standIn false・error null(3 回)・(3) Balance の wasm は lazy(10,628,659 B)で、取得の開始 2,087・2,107・2,240 ms は起動前の取得の完了 1,916・1,934・1,988 ms の後・(4) reads に Balance の instantiate(requested=true)(3 回)・elapsedMs 7,210・7,419・7,632・共有の表の balance 0 件・流れ A(error null・engine-idle・standIn false・trapped false・hostErrors 0・opened true)。落ちた: (2) engine.wasm が 165,084,314 B(3 回とも同じ)。改定前の閾値 165,079,668 B を 4,646 B 超えた。改定後の閾値 165,100,887 B では通る(余り 16,573 B)。起動の確認(boot-check)は elapsedMs 8,126(参考)。
- (2) の内訳: 閾値 165,079,668 B は下書き 102 の値(Balance の hook 点を適用しない時の値)。#1342 の完了時は 165,084,503 B(今回は −189 B)。hook 点の 3 関数を C の経路に残したことによる増分は、785 + 171 + 483 = 1,439 B(関数番号 34626・34685・34686)。残りの約 3,207 B は**未特定**。
- 生成物(`lift/out-app/plugins/`。コミットしない): modules.tsv 10 行 2,301 B・hyb-args.tsv 9 行 752 B・pack.args 11,507 B・extract-dlls.tsv 28,730 B・vfs-seed-idle1.tsv 98,294 B・hybrid.cmd 514 B・seed.args 492 B・plugins.tsv 188 B。lowered-all.txt は 19,045 件から hook 点 3 件を除いて 19,042 件(186,089 B)。Balance の一覧は ok 12,282 件(#1342 と同じ)で、hook 点を除いて 12,279 件。Balance の base は 0xc1d0000(下の (7))。
- 共有の表の変更(行数): `modules.tsv` −1・`hyb-args.tsv` −1・`hybrid.cmd` +2 −6・`n0.cmd` +2 −2・`gen.args` +2 −2・`pack.args` +1 −7・`seed.args` +1 −3・`vfs-seed-idle1.tsv` −1・`ci/extract-dlls.tsv` −1・`lists/lowered-balance.txt` −12,282(削除)・`lowered-all.txt` −19,090(削除。生成物にした)・`.gitattributes` −1・追加 `plugin.tsv` +2。ほかに plugins.rs +641、main.rs +59 −21、`tests/plan/*.txt` の期待値(`rt` の 90 → 91 は、もとの期待値が古かったため)、`tests/plugins/` のダミー 2 本、`measure.mjs` +4 −1。
- (7) の訂正は、下の設計 (7) を参照(旧記述の 0x7B8F200 は誤り。Balance は 0xc1d0000)。
- (9)(10) の実測(静的。実行時の数は未確認): `fle_realmod_find(name)` は `fle_load.c:427`・`fle_modules.c:226` と `:232`・`fle_realmod.c:100`(定義は `fle_realmod.c:105`)。`fle_realmod_find_path` は `fle_load.c` と `fle_realmod.c` に各 1。realmod の名前は DLL の名のまま(素の名前の検索を変えないため)。分けた wasm の import は module `c` だけ(関数 606・memory 1・global 1)で、別の分けた wasm への直接の呼び出しは 0 件。
- (11) 段の時間(前回の `fe-build engine`。23 分 26 秒。秒): plugins 0.0・lists 10.8・gen 31.6・compile 434.2(172 段、並列)・hybrid-n0 226.8・link 4.5・wasm-opt 341.4・hybrid 334.9(9 本)・blend2d 0.8 / 20.8 / 0.1・合計 約 1,406。1 本あたりは未測定(推定: hybrid は平均 約 37 秒。lists の 10.8 秒を 160 本に直列で掛けると 約 29 分)。
- 設計からの変更: (a) realmod の名前は key ではなく DLL の名。(b) プラグインの行は静的な行の後ろ。Balance は m10(以前は m8)、Filter が m8、Delay 2 が m9。単位の名の集合は同じ。(c) 静的な一覧 8 本はコミットのまま残し、生成したのは Balance の一覧だけ。(d) 単位の一覧は走査せず、一致の検査だけ((13) は未実施)。(e) args が空なら `--pdata`。(f) modules が `RM_MAX` などを超えないかを生成の段で検査(10 ≤ 16)。(g) `fe-build tables`(抽出用の副命令)を足したが、抽出には使わない(残してよい)。
- 未確認: 160 本の生成、案 B、wasm メモリの実測、段ごとの 1 本あたりの時間、素の名前の実行時の数、ダミー 2 本での計画全体の比較、動的な単位の一覧。

## 前回の試行で分かったこと(2 回目。run 45353228077。`failed`)

- 作業は保存された(差分は取り込みの段で作業ツリーに載る)。作業者の commit は `2cf9a6ef`(作業者のツリーの commit。main には無い)。判定は、受け入れの `boot-bundle` の段で落ちた(下の (a))。
- 取り込み(1 回目の差分): 衝突は `lift/crates/fe-build/src/main.rs` の `pack_step_for` の 1 箇所だけだった。main の #1362 の `OFFLINE_PAGE` 処理と、生成された `pack.args` を読む形の両方を残した。オフラインページの `--lazy-file` は、生成の段が `pack.args` に足す(#1362 と同じ動作)。
- 作業者が足した物: `lift/crates/fe-wasmgen/src/hybrid.rs` に、分けた wasm の関数 import が `c` 以外なら止める検査(項目 (10))。計画の期待値は、固定のダミー 2 本(`lift/crates/fe-build/tests/plugins/`)から作る形に直した(実物の計画は、段の名前と順序だけを確かめる。13 段)。`fe-build` の試験 8 件・`fe-wasmgen`・`guard.test`(62 件)が通った。
- 作業者の実行の結果(runner の値。nproc 4、loadavg 1.3〜1.9):
  - 通った: 生成の段(`plugins: 1 DLLs`)。engine-idle・standIn false・error null(測定の前段で 3 回)。Balance の wasm は起動後の取得(開始 4,033・4,092・4,028 ms。起動前の取得の完了の後)。hook 点の VA は一覧に 0 件。C の出力に `hook_points 3`。共有の表から Balance の行が消えた(`grep` が空)。流れ A(`elapsedMs` 10,959)。`scan.sh` 0 件。
  - 落ちた: (2) engine.wasm が **165,622,185 B**(1 回目の 165,084,314 B より +537,871 B。閾値 165,100,887 B を超えた)。
  - (b) 未確認: (4) reads の Balance の instantiate。測定の scene が、Welcome の窓が (0,0) のまま本体が描かれず、Default の行が効かず、NewStuff.flp を開けなかった(4 回。ホイール操作を外した一時のコピーでも同じ)。流れ A は同じ bundle で、窓が (481,241) に出て通る。
- (a) 判定の段の結果: 受け入れの `boot-bundle` が、`state running`・`no first idle, error or exit within 180000 ms`(`elapsedMs` 180,550)で落ちた(下の (a))。作業者の実行の `boot-check` は engine-idle・12,450 ms で通っていた(P1 の 10 秒は、この実行では未達)。
- 生成物(作業者の数): `lift/out-app/plugins/` の大きさは、1 回目の記録と 1 バイトまで一致した(`modules.tsv` 2,301 B・`pack.args` 11,507 B・`lowered-all.txt` 186,089 B・`vfs-seed-idle1.tsv` 98,294 B ほか)。よって engine の増分は生成の入力の差ではない。推定(未確認): コンパイルされる側の差。作業者の確認では、`emcc` 7.0.0、clang `24.0.0git`(emsdk の main の取り込み。`ci/versions.env` の `EMSDK_REV` が効いていないように見える)。依頼者の代理(PM)の決定: 閾値 (2) を下の 166,000,000 B に直す。
- 共有の表の変更(作業者の数。+追加 −削除): `.gitattributes` −1・`ci/extract-dlls.tsv` −1・`lift/harness/b2d2/modules.tsv` −1・`lift/harness/hyb/hyb-args.tsv` −1・`lift/harness/hyb/hybrid.cmd` +2 −6・`lift/harness/hyb/n0.cmd` +2 −2・`lift/harness/b2d2/gen.args` +2 −2・`lift/harness/web/pack.args` +1 −7・`lift/harness/web/seed.args` +1 −3・`lift/harness/seeds/vfs-seed-idle1.tsv` −1・`lift/harness/hyb/lists/lowered-balance.txt` −12,282(削除)・`lift/harness/hyb/lowered-all.txt` −19,090(削除。生成物)・`lift/crates/fe-build/tests/plan/engine.txt` +16 −5・`bundle.txt` +2 −2・`extract.txt` +1 −1・追加 `lift/plugins/fruity-balance/plugin.tsv`。
- 段の時間(作業者の run。秒): plugins 0.1・lists 14.2(DLL 1 本)・blend2d 1.1 / 35.8 / 0.1・gen 37.1・compile 713.5(172 件、並列)・hybrid-n0 286.2(9 モジュール)・link 6.7・wasm-opt 466.3・hybrid 411.4(9 モジュール)。合計 1,972.5。1 モジュールあたりは hybrid-n0 約 31.8、hybrid 約 45.7(推定。個別の実測は未確認)。160 本を直列にすると、hybrid 系だけで 2 時間を超える(推定)。
- 設計の項目の結果(作業者): (6)(7) 案 A。Balance の base 0xc1d0000・SizeOfImage 0x3a1000・終端 0xc571000。GLOBAL_BASE までの空き 61,403,136 B。(8) 上限は変更なし(生成の行数 10 ≤ 16)。(9) 素の名前の呼び出しは製品で 3 箇所(`fle_load.c:427`・`fle_modules.c:226`・`fle_modules.c:232`)。パスの表から名前で引くのは `fle_realmod.c:100` の 1 箇所。実行時の回数は未確認。(10) 分けた wasm の関数 import は `c` だけ(別の分けた wasm への直接の呼び出しは 0)。(13) 単位の一覧は静的のまま(`gen` 後の走査は未実施)。
- 次の試行でやること(PM の決定と、作業者の疑い):
  1. 閾値を直す: 受け入れ (2) は engine wasm が **166,000,000 B 以下**(粗い上限)。理由: 閾値は、他の Issue の変更で engine の大きさが動くので、増分を測らず、上限で見る。この Issue が engine に本体を足す誤りを、この上限で検出する。
  2. (a) の原因を確かめる: 載った作業ツリーを確かめる → ビルド → `boot-check` を 3 回、`measure.mjs` の scene を 2 回、フォアグラウンドで実行する。止まる・黒いなら、止まった時の engine のログの末尾と `[js]` の行を `.tmp-` に取り、報告に書く。
  3. (b) の疑い(未確認): (i) モジュールの番号の変更(Balance が m10(以前は m8)、Filter が m8、Delay 2 が m9。プラグインの行を静的な行の後ろに置いたため)。`lift/harness/two/main_two.c`・`xmod_glue.c`・`lift/runtime/fle_realmod.c`・`fle_modules.c` が、番号を固定で参照していないか。(ii) `pack.args` の `--lazy-file`(オフラインのページ。#1362)を、生成の段が足す形に替えた点。(iii) seed の表(`vfs-seed-idle1.tsv` の Balance の行の消失・生成した表との差)。
  4. 原因の特定: 生成物を 1 つずつ前の固定の表に戻す実験(モジュールの番号・`pack.args`・seed の表)で、止まりが消える物を探し、直す。どれが原因かを報告に書く。(4) の reads の確認は、この後で行う。
  5. 範囲: 生成の段・試験・必要なら固定の表。`app/`・`lift/` を変えると main の `deploy.yml` が動く(報告に書く)。

## 1 プラグインを足す手順(今の表)と、この設計での置き場所

| 置く場所 | 1 プラグインごとの行 | 固有か共有か | この設計 |
|---|---|---|---|
| `ci/extract-dlls.tsv` | 1 行(インストーラの名前・抽出先・SHA-256) | 固有 | `plugin.tsv` から生成 |
| `lift/harness/b2d2/modules.tsv` | 1 行(realmod の名前・DLL・base・lift の引数・パスの接尾辞) | 固有 | 生成 |
| `lift/harness/seeds/vfs-seed-idle1.tsv` | 1 行(ゲストのパス・ホストのファイル・SHA-256) | 固有 | 生成 |
| `lift/harness/web/seed.args` | 1 行 | 固有 | 生成 |
| `lift/harness/web/pack.args` | `--args` の語・`--lazy-dll`・`--lazy-file` の行 | 固有(`--args` の行は共有で衝突する) | 生成 |
| `lift/harness/hyb/hyb-args.tsv` | 1 行 | 固有 | 生成 |
| `lift/harness/hyb/hybrid.cmd` | `--separate` の名前の列 | 共有(1 行に全部入る) | 生成 |
| `lift/harness/hyb/lists/lowered-<名前>.txt` | 約 1.2 万行(Balance の場合) | 固有 | 生成(ビルドが求める。下の (4)) |
| `lift/harness/hyb/lowered-all.txt` | 一覧の和 | 共有 | 生成。コミットしない |
| `lift/crates/fe-build/tests/plan/engine.txt`・`bundle.txt` | 件数と引数の行 | 共有 | プラグインの数に依らない試験(下の (12)) |
| `lift/harness/hooks/*.txt` | hook 点の RVA(Delphi 系の本体だけ) | 固有 | `plugin.tsv` の `hook` 列 |

## 設計

**(1) 固有の物: `lift/plugins/<名前>/plugin.tsv` を 1 つ。** `<名前>` は、プラグインのフォルダの名前を小文字にし、空白をハイフンにしたもの(例: `fruity-balance`)。1 フォルダの中の本体と補助の DLL は、1 つの `plugin.tsv` に行として並べる(補助も、それぞれ独立した wasm になる)。見出し付きの TAB 区切り。列は `key`・`guest`・`role`・`sha256`・`size`・`hook`・`args`。

- `key`: DLL の一意の名。小文字。空白と `\`・`/`・`:` を含まない(realmod の名前の条件。`lift/runtime/fle_realmod.c` の `table_check`)。`<種別>.<フォルダ>.<DLL の名>` の形(例: `effects.fruity-balance.fruity_balance_x64.dll`)。DLL の名だけでは一意にならない(下の (9))。
- `guest`: `Plugins\Fruity\Effects\Fruity Balance\Fruity Balance_x64.dll`(`C:\Program Files\<ベンダー>\FL Studio 21\` の後)。
- `role`: `本体` か `補助`。
- `sha256`・`size`: `inputs/plugin-classes.tsv` の値。
- `hook`: `none`、または `existing`(`lift/harness/hooks/fruity-delphi-effect-hook-points.txt` の 3 点)、または `rva:0x...,0x...,0x...`(`inputs/plugin-hooks.tsv` の 3 点)。
- `args`: 空、または lift の引数(既定 `--pdata`)。

**(2) 走査と検査。** `fe-build` に新しい段 `plugins` を足す。`lift/plugins/*/plugin.tsv` を名前順に読み、次を検査する: `key` の一意、`guest` の一意、`sha256` は 64 桁、`size` は正の整数、`role` の値、`hook` の形(`none` なら 3 点は要らない。`rva:` は 3 点)。違反なら、何も生成せずに止まる。

**(3) 生成(全て `lift/out-app/plugins/` の下。コミットしない)。** 共有の表は、今の行(FLEngine・dsp・Blend2D など、プラグインでない物)と、プラグインの行を合わせて書く。
- 抽出の表(`ci/extract-dlls.tsv` の行 + 各プラグインの行。`fe-build extract` が読む)。
- seed の表(`vfs-seed-idle1.tsv` の行 + 各プラグインの行)と、`seed.args` の行。
- `modules.tsv`(gen-all)。`args` は `--hook-points` を含む(`hook` が `none` なら含まない)。base は下の (6)。
- `hyb-args.tsv`。`hybrid.cmd` の `--separate` の列は、全プラグインの名前をコンマで 1 語にする。
- 一覧(下の (4))と、その和の `lowered-all.txt`(下の (5) で hook 点を除く)。
- `pack.args` の `--args`(実モジュールの一覧)・`--lazy-dll`(DLL)・`--lazy-file`(分けた wasm `lift/out-app/xmod/<名前>.wasm`)。
- `gen` の出力の単位(下の (13))。

`gen.args` の `--omit-list` は、この生成物の path にする。

**(4) 一覧(direct にできる関数の一覧)は保存せず、ビルドで求める。** 新しい段 `lists`(`gen` より前。DLL ごとに並列にする)。各 DLL を `fe-wasmgen <dll> <out> <n0 の引数> --report <tsv>` で、一覧なしで走らせ、status が `ok` の VA を `lift/out-app/plugins/lists/lowered-<key>.txt` に書く。既存の `hybrid-n0`・`hybrid` の段の `check_listed` が、一覧の全てが `ok` で出ることを確かめる(落ちたら止まる)。Balance の件数は、#1342 の記録では、12,500 関数のうち 12,282 が ok。この Issue で同じ件数を再現する。

**(5) hook 点は direct の一覧から外す。** 根拠(実測): `lift/crates/fe-wasmgen/src` に hook の処理は無い(`hook` の語は dispatch の表だけ)。hook の guard は `fe-lift lift --hook-points` の C の出力にだけ在る。Balance の 3 点(VA = base + RVA。0x6817bc0・0x6831d70・0x6831d90)は、`lift/harness/hyb/lists/lowered-balance.txt` の 582・1117・1118 行に在り、direct の wasm に下りている(guard が効かない)。既存の ILTools・QFC・FLEngine の hook 点は、共通の一覧(`lowered-all.txt`)に 0 件(確認済み)。この設計では、どのプラグインの hook 点も、一覧から外し、C の経路に残す。受け入れ: 生成した一覧に hook 点の VA が 0 件、C の出力に `hook_points 3`(fe-lift の出力の行)。Balance の効果は、この変更の後に、#1371 の道具で確かめる(未確認)。

**(6) base(像の番地)は生成時に割り当てる。** プラグインを `key` の順に並べ、既存の固定の番地の後ろから、各像を 64 KiB に揃えて置く。像の大きさは PE の SizeOfImage(ヘッダから読む)を 64 KiB に揃えた値。固定の番地は `plugin.tsv` に書かない(並行の Issue が番地で衝突するため)。番地が変わっても、一覧・hook 点・名前は生成物なので、影響は生成物の中だけ。

**(7) 像の配置(案 A を採用。前回の試行で訂正した)。** 静的な像の末尾は oggio(base 0xc000000、SizeOfImage 0x1cf000)の 0xc1cf000。Balance は 0xc1d0000 に置く(SizeOfImage 0x3a1000)。GLOBAL_BASE(0x10000000)までの空きは 0x3E30000(約 62.2 MiB。65,208,320 B)。
- 案 A(採用。像を GLOBAL_BASE より下に置く。動作は変えない): Balance(3.7 MB)は入る。同程度の大きさで約 17 本が上限で、第 2 波の 160 本(608.5 MB)は入らない。この Issue では案 A で Balance を置く。
- 案 B(GLOBAL_BASE と INITIAL_MEMORY を上げ、像の領域をデータの先頭の上に置く。`lift/harness/web/link.rsp` を変える): 160 本が入る配置は、別の Issue(#{{135}})で決める。この Issue では行わない。
- 旧記述の「既存の像の末尾は 0x7B8F200、空き 132 MB」は誤り(Delay 2 の像の末尾は 0x7BA5000 で、oggio はさらに後ろにある)。

**(8) 実行時の表の上限を上げる。** `lift/runtime/fle_realmod.c` の `RM_MAX`(16)・`lift/runtime/fle_cursor.c` の `SHARED_MODULE_REALMODS`(16。RM_MAX と同じ値の複製)・`lift/harness/two/main_two.c` の `MAIN_REAL_MAX`(32。実モジュールのファイル数の上限)。上限は、生成した `modules.tsv` の行数(既存 + プラグイン)以上にし、生成の段で検査する。`#ifdef` は足さない(`guard/flags.sh` が見る)。`fle_cursor.c` のハンドルの式は、像の番地ではないが、行数に合うかを確かめる。

**(9) 名前の一意。** realmod の名前は `key`。guest の `LoadLibrary` は、パス(`\` を含む)なら `fle_realmod_find_path`(接尾辞の表。`lift/runtime/fle_load.c` の 334 行付近)で、素の名前なら `fle_realmod_find(name)`(`fle_load.c` の 427 行付近、`fle_modules.c` の 232 行付近)で引く。DLL のファイル名が重なる物が在る: `engine_x64.dll` が補助に 18 件、`sytrus_x64.dll`・`patcher_x64.dll`・`fruity wrapper_x64.dll` が Effects と Generators の両方に在る。素の名前で引く呼び出しが、どの DLL のどの呼び出しか(補助を同じフォルダから名前で引く経路)は未確認。この Issue で、素の名前の呼び出しを数え、報告に一覧する。扱い(同じフォルダの名前を優先する等)は、数えた後に決める。

**(10) 分けた wasm の検査。** `app/src/engine/xmod.ts` は、分けた wasm の import を `c` だけ許し、ほかは読み込みで止まる。`lift/crates/fe-wasmgen/src/hybrid.rs` の検査は、`c` の import だけを見る。生成の段で、分けた wasm の `env` の import(他の分けた wasm の `f_` を含む)が 1 つでも在れば、ビルドを止める。別の分けた wasm の直接の関数への呼び出しの数を、生成の段で数え、報告に書く(0 を期待する。0 でなければ、扱いを決める)。

**(11) 段の順序と時間(実測で決める)。** 今の `hybrid` 段は、モジュールを 1 つずつ、`fe-wasmgen` と `wasm-opt -Oz` を直列に回す(`lift/crates/fe-wasmgen/src/hybrid.rs` の `for m in &modules`)。`hybrid-n0` も同じ。160 本を直列にすると、この段だけで 5 時間の上限に近づく。並列にする案を作り、1 本あたりの `fe-wasmgen`・`wasm-opt` の時間を測って、報告に書く(実測と推定を分ける)。

**(12) 試験の期待値。** `lift/crates/fe-build/tests/plan/engine.txt`・`bundle.txt` は、プラグインの数と中身に依らない形にする。試験は、`lift/crates/fe-build/tests/` の下に置く固定の 2 本のダミーのプラグイン(`plugin.tsv` と、生成器の入力になる最小の表)で計画を作り、期待値と比べる。実物の計画は、段の名前と順序だけを確かめる。

**(13) 単位の一覧。** `lift/harness/two/plain-units.txt`・`ai-units.txt` は、`gen` の単位の静的な一覧(`lift/out-app/gen/src/m1-00.c` などを書いている)。プラグインを足すと、`mK-NN.c` の単位が増える。`gen` の後に `lift/out-app/gen/src/` を走査して単位を決める。

**(14) 生成物にする物。** `.gitattributes` の `lift/harness/hyb/lowered-all.txt merge=union` の行を消し、コミットされた `lift/harness/hyb/lowered-all.txt` と `lift/harness/hyb/lists/lowered-*.txt` を消す(生成物になる)。

**(15) Balance の移行(この Issue の commit)。** `lift/plugins/fruity-balance/plugin.tsv`(`effects.fruity-balance.fruity_balance_x64.dll` の 1 行。`hook` は `existing`)を足す。共有の表から Balance の行を消す: `modules.tsv`・`hyb-args.tsv`・`hybrid.cmd` の `--separate` の Balance・`pack.args` の `--args` と `--lazy-dll` の Balance・`lowered-all.txt`・`lists/lowered-balance.txt`・`ci/extract-dlls.tsv` の Balance の行・`vfs-seed-idle1.tsv` の Balance の行・`seed.args` の Balance の行。Filter と Delay 2 の行は、この Issue では消さない(#1370 で移す)。生成する base は、既存の固定の番地の後ろ(下の (6))なので、Filter(0x7000000)・Delay 2(0x7800000)と重ならない。前回の試行では Balance の base は 0xc1d0000 だった(下の (7))。

## 受け入れ

```accept
npm run build --prefix app && cargo test --locked --manifest-path lift/Cargo.toml -p fe-build -p fe-wasmgen && bash spec/guard.test.sh && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/delivery-split/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/split.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const B=(a)=>a.filter((f)=>/balance/i.test(f.name)&&/\.wasm$/i.test(f.name));const eng=(a)=>a.find((f)=>f.name==="engine wasm");const ok=L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&eng(r.preStart).bytes<=166000000&&B(r.lazy).length===1&&B(r.lazy)[0].bytes>0&&r.fetches.some((f)=>/balance/i.test(f.name)&&/\.wasm$/i.test(f.name)&&f.startMs>=r.preStartDoneMs)&&Array.isArray(r.reads)&&r.reads.some((x)=>/balance/i.test(x.name)&&/\.wasm$/i.test(x.name)&&x.requested===true)&&r.elapsedMs<=12500);process.exit(ok?0:1)' "$RUNNER_TEMP/split.jsonl" && ! grep -qi balance lift/harness/b2d2/modules.tsv lift/harness/hyb/hyb-args.tsv lift/harness/hyb/hybrid.cmd lift/harness/web/pack.args ci/extract-dlls.tsv lift/harness/seeds/vfs-seed-idle1.tsv lift/harness/web/seed.args && test -f lift/plugins/fruity-balance/plugin.tsv && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const a=L.find((r)=>r.flow==="A");process.exit(a&&a.error===null&&a.state==="engine-idle"&&a.standIn===false&&a.trapped===false&&a.hostErrors===0&&a.opened===true?0:1)' "$RUNNER_TEMP/flow.jsonl"
```

判定の基準:
- 生成の段が、`lift/plugins/*/plugin.tsv` だけから、共有の表と一覧を作る。`lift/out-app/plugins/` の生成物は、コミットしない。
- Balance の 3 回の起動(`tools/delivery-split/measure.mjs`): (1) engine-idle・standIn が false・error が null、(2) engine wasm が 166,000,000 B 以下(粗い上限。閾値は、他の Issue の変更で engine の大きさが動くので、増分は測らず上限で見る。この Issue が engine に本体を足す誤りを検出するための物)、(3) Balance の wasm が起動後の取得に在り、取得の開始が起動前の取得の完了以後、(4) `reads` に Balance の wasm の instantiate が `requested: true` で在る。
- 生成した一覧に、Balance の hook 点の VA(0x6817bc0・0x6831d70・0x6831d90)が 0 件。C の出力に `hook_points 3`。
- 生成した一覧の件数(Balance)を報告に書く(#1342 の 12,282 との比較)。
- 共有の表から Balance の行が消えている(受け入れの `grep`)。
- 3 回の起動の `elapsedMs` が、どれも 12500 以下。
- 流れ A(`tools/smoke/welcome-flow.mjs`)が通る。
- 報告に書く: (4)(7)(9)(11) の実測の値と、選んだ案と理由。

## 範囲外の注

- **抽出(agent.yml・deploy.yml)**: `app/scripts/extract-fl.mjs` は、`--plugins` が無ければ `lift/plugins` を既定に読む(main の 617e8339)。この Issue の作業者は `.github/` を変えない(判定の凍結比較が通らない)。`fe-build` の `tables` 副命令は、抽出には使わない(残してよい)。
- **Balance を読んだ後の終了**: `tools/delivery-split/measure.mjs` の scene の終わりで、engine が `ExitProcess`(終了コード 0。`state exited after the scene`)で止まる。原因は未確認。#1342 の記録では SIGABRT(134)だった。#1365(demo を開くと止まる)・#1372(Exoplanet)と同じ現象の可能性が高い(依頼者の『評価用プロジェクトを開こうとすると落ちる』)。この Issue では直さない。ただし、疑う commit・関数・呼び出しの並びを報告に書く。
- **measure.mjs の読み先**: `host-extra/resources` が無いとき(CI の runner。resources の作業だけが復元する)は、host の同じ `NewStuff.flp`(インストーラ由来。SHA-256 が一致)を読む。前回の作業に入っている(+4 −1)。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。**4 時間 30 分を過ぎたら**、受け入れが通らなくても、作業ツリーを commit して、報告を書いて終える。
- ビルドは約 35 分(直近は約 33 分。1 回目は 23 分 26 秒)。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大の 5400000 にする。背景で起動して待たない。
- 前回の作業は作業ツリーに載っている(上の「前回の試行で分かったこと」)。`git status` と `.prior-work.md` で状態を確かめてから、続きから行う。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 共有の表の変更の一覧(ファイルごとの行数)、生成物の一覧の件数と大きさ、(7) の配置の案と実測のメモリ、(9) の素の名前の呼び出しの一覧、(11) の段の時間(1 本あたり)。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。生成の段は、既定の動作として常に走る(オプトインを作らない)。
- 依頼者の指示にない制限(例: プラグインの数の上限)を、この作業の判断で増やさない。
- ベンダー名と Cloudflare の識別子を書かない。Python は製品に使わない(調査と検証にだけ使う)。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 他のタスクとの関係

- blocked_by: なし。
- #1341(像の読みを guest の要求の時まで遅らせる)と #1342(Balance の分割。この Issue の移行元)の結果を使う。
- #{{135}}(内蔵のプラグイン 160 本の像の配置。上の (7) の案 B)は、この Issue の後に入る。この Issue の受け入れは案 A で通る。
- #1370(Fruity Filter を、ディレクトリの追加だけで足す)と #1371(内蔵のプラグインを 1 本ずつ確かめる道具)は、この Issue の後。
- 第 2 波(下書き番号 041〜085 の束)は、この Issue と #1370 と #1371 が閉じた後に起票する(`pm-work/issue-drafts/WAVES.md`)。
- `lift/` の変更は、main の `deploy.yml` を動かす(報告に書く)。
