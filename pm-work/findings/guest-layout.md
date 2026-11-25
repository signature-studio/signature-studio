実エンジン: `lift/target/release/fe-build engine`(exit 0、約 25 分)→ `fe-build bundle`(exit 0)→ `tools/boot-check/boot-bundle.mjs`(state engine-idle、standIn false、error null、elapsedMs 9230)→ `sh tools/plugin-layout/make-base.sh`(exit 0、現行の配置の bundle)→ `tools/delivery-split/measure.mjs` を新旧それぞれ 3 回(全て engine-idle、error null、場面の終わりも engine-idle)→ `tools/p3-audio-probe/measure.mjs --boot` を 3 回(error null、bootLast60sAt70s 0、3 回とも)→ `tools/smoke/welcome-flow.mjs` の流れ A〜D(error null、engine-idle、standIn false、trapped false、hostErrors 0、opened true、welcomeClosed は B・D で true)。headless Chrome 157 で観測した。

# #1379 ゲストのアドレス空間の配置(第 2 波の 160 本が入る配置)

判定: **受理**(受け入れのコマンドの各段が通った。下の「未確認・PM の判断」の項目は残る)。

- runner の値。この機械とは別枠。nproc 4。loadavg は welcome-flow の終わりで 2.69 / 1.71 / 1.68、boot-check で 1.73 / 1.38 / 1.79。

## 1. 結論

- 採った配置: `GLOBAL_BASE` = 0x3AFD0000(= 0xC1D0000 + 750 MiB)、`INITIAL_MEMORY` = 0x52FD0000(= `GLOBAL_BASE` + 384 MiB。現行と同じ余裕)、`MAXIMUM_MEMORY` = 2 GiB(変えず)。`app/src/perf/stats.ts` の `WASM_MEMORY_LIMIT_BYTES` は既に 2048 × 1024 × 1024 で、`MAXIMUM_MEMORY` と同じなので変えていない。
- 160 本(と Balance)の像は、`SizeOfImage` を 64 KiB に切り上げた和が 752,025,600 B(Balance 3,866,624 B を含む。160 本だけなら 748,158,976 B)。0xC1D0000 から並べると、末尾は 0x38F00000(955,252,736 B)で、`GLOBAL_BASE` まで 34,406,400 B(32.8 MiB)の余りがある。
- RSS(判定の値)は、新旧で差が無い(中央値の差 +8.3 MB。基準は +150 MB 以内)。
- ただし、ヒープの余りは減った(下の 4)。

## 2. 測定の表(runner の値)

`rssMb` は、ブラウザ(Chrome の本体と、その下の全プロセス)の VmRSS の合計の最大(engine-idle から場面の終わりまで 1 秒ごと。1 MB = 1,048,576 B)。`engineMemoryBytes` は参考(確保量。物理のメモリではない)。

| 配置 | `rssMb`(3 回) | 中央値 | `elapsedMs`(3 回) | `engineMemoryBytes`(参考) | `bootLast60sAt70s`(3 回) |
|---|---|---|---|---|---|
| 現行(GLOBAL_BASE 0x10000000、INITIAL 671088640) | 3734.7 / 3674.6 / 3690.3 | **3690.3** | 8870.8 / 8811.7 / 8703 | 1,074,462,720 | 未測定(判定の基準は採る配置だけ) |
| 採る(GLOBAL_BASE 0x3AFD0000、INITIAL 1392312320) | 3698.6 / 3693.5 / 3726.3 | **3698.6** | 8836.6 / 8826.5 / 8772 | 1,778,909,184 | 0 / 0 / 0 |

- 判定(受け入れの式): 採る配置の中央値 3698.6 ≤ 現行の中央値 3690.3 + 150 → 満たす(差 +8.3 MB)。
- 全 6 回の run: state engine-idle、standIn false、error null、`reads` に Balance の wasm が `requested: true` で在る。
- boot-check(採る配置、1 回): elapsedMs 9230、engine-idle、standIn false、error null。
- welcome-flow(採る配置): 流れ A〜D の elapsedMs は 8760 / 8788 / 8846 / 8772、`preBootBytes` は全て 311,124,239(上限 314,368,711 以内)。
- 注: この bundle の engine は、`lift/plugins/` の 1 本(Balance)と既存の像を載せる。第 2 波の 160 本はまだ engine に入っていない(#1370 以降で足す)。したがって、この測定は「配置の変更だけ」の影響を見ている。160 本の像のバイトは読まれていない。

## 3. 採った配置と理由

- PM の決定 2 に従った。像の領域は 0xC1D0000 から始め、160 本の `SizeOfImage` の和(748,158,976 B)に 5% の余裕を足して 750 MiB。`GLOBAL_BASE` は領域の末尾(0x3AFD0000 = 989,659,136)。
- `INITIAL_MEMORY` は、`GLOBAL_BASE` の上のヒープに現行と同じ 384 MiB の余裕を取った(0x52FD0000 = 1,392,312,320)。
- 検査: `fe-build` の `assign_bases` は、像の末尾が `GLOBAL_BASE` を超えると止まる(既存)。`plugins::tests::the_second_wave_is_160_dlls_and_fits_the_image_area` で、161 本(Balance と 160 本)が 0xC1D0000 から 64 KiB 単位で、互いに重ならず、`GLOBAL_BASE` の下に入ることを確かめた。SizeOfImage は `inputs/plugin-image-sizes.tsv`(SHA-256 で一致させた 171 本。`tools/plugin-layout/image-sizes.mjs` で作った)から読む。VFX の 11 本は抽出していないので、表に無い(計画の 160 本には要らない)。

## 4. ヒープの余りが減った(PM の判断が要る)

- 実測: 採る配置の `engineMemoryBytes` − `GLOBAL_BASE` = 789,250,048 B(752.5 MiB)。現行は 806,027,264 B(768.7 MiB)。ヒープの使用量はほぼ同じ。
- しかし、`MAXIMUM_MEMORY`(2 GiB = 2,147,483,648 B)までの余りは、現行 1,073,020,928 B(1.0 GiB)から、採る配置 368,574,464 B(0.34 GiB)に減った。ヒープの上限は、2 GiB − `GLOBAL_BASE`(0.92 GiB)= 約 1.16 GB になる。
- 2 GiB は PM の決定どおり変えていない。余りの扱い(上限を上げるか、像の置き方を変えるか)は PM が決める。

## 5. 表の上限(192)

- `lift/runtime/fle_realmod.c`: `RM_MAX` 16 → 192(エラーの文言も 192)。
- `lift/runtime/fle_cursor.c`: `SHARED_MODULE_REALMODS` 16 → 192。ハンドルの式は index 191 でも 1 MiB の領域の内(`fle_loadimage.h`)。
- `lift/harness/two/main_two.c`: `MAIN_REAL_MAX` 32 → 192。
- 生成の段は、`LIMITS`(この 3 つの値)の最小を超えると止まる(既存)。

## 6. 名前の重複(実装と確認)

- 160 本の中の重複(ファイル名の小文字で): `engine_x64.dll` 18 本、`fruity wrapper_x64.dll` 2 本、`patcher_x64.dll` 2 本、`sytrus_x64.dll` 2 本(160 本のうち 140 種類)。
- 対応(PM の決定 4):
  - `key` は `<種別>.<フォルダ>.<DLL 名>` で一意(既存の形)。`validate()` は、key と guest(インストーラの経路)の一意と、key が `.dll` で終わることを検査する。ファイル名の一意の検査は外した。
  - `Dll::name()` は key。これがテーブルの名前(realmod の名前)、抽出先の `lift/out-plugins/<key>`、seed の名前 `…/Plugins/Fruity/<種別>/<key>`、回転した名前 `<key の幹>.<base hex>.dll` になる。どれも key で一意。
  - ゲストの LoadLibrary(経路つき)は `fle_realmod_find_path` が接尾辞で引き、名前(key)で `fle_realmod_find` する。key は一意なので、正しい DLL に決まる。
  - 抽出の照合(`app/scripts/extract-fl.mjs`)は、名前の接尾辞と SHA-256 の両方で候補を絞るので、ファイル名が同じでも取り違えない(`installer_name` は基本名のまま)。
- Balance の変化: 名前 `fruity_balance_x64.dll` → `effects.fruity-balance.fruity_balance_x64.dll`。抽出先と seed の名前も key になった。`lift/plugins/fruity-balance/plugin.tsv` の key は元から同じ。
- 素の名前の呼び出しの確認(PM の指示の「確かめる」):
  - `lift/runtime/fle_load.c:427`(LoadLibrary の素の名前)と `fle_modules.c:226・232`(GetModuleHandle の素の名前)は、`fle_realmod_find(name)` で表の名前と完全一致を取る。表の名前は key になったので、素の DLL 名(例 `engine_x64.dll`)は一致せず -1 になる。仮想の DLL の表にも無いので、「見つからない」(LoadLibrary は 126)になる。**誤った DLL には決まらない。**
  - 変更前は、同じ名前の最初の表の行に決まった(`engine_x64.dll` の 18 本のうちの 1 本)。これは黙って誤る。
  - **未確認(PM の判断)**: `inputs/plugin-classes.tsv` の備考に、47 本(160 本の中)が「名前で参照する同じ組の DLL」として `engine_x64.dll`(18 参照)などを挙げる。参照が素の名前か、フォルダを含むパスかは、表からは決まらない。素の名前なら、この変更の後は見つからない(126)。直す案は、呼び出し元の DLL のフォルダで素の名前を解く、など。直すかどうかは PM が決める。
  - インポート表(列「import した DLL」)には、プラグインの DLL の名前は無い。

## 7. 試験の結果

- `cargo test --locked -p fe-build`: 21 passed; 0 failed(`plugins::tests::the_second_wave_is_160_dlls_and_fits_the_image_area` と `the_global_base_is_the_one_of_the_link` を含む)。
- `cargo test --locked -p fe-lift --test wall_dec1`: 7 passed(`rm_max_is_8` は 192 を pin)。
- `lift/crates/fe-lift/tests/realmod.rs` は **変えていない**(下の 8 を参照)。その `a_table_that_breaks_the_rules_stops_by_name` の `seventeen-modules` の場合(17 本は「16 を超える」で止まる、と期待)は、RM_MAX 192 では止まらないので、この試験は古くなっている。このマシンでは `x86_64-w64-mingw32-gcc` が無く、動かせなかった。

## 8. 未確認・PM の判断が要る点

1. **`realmod.rs` の試験が古い**: `lift/crates/fe-lift/tests/realmod.rs` は、`python3` を 4 箇所(139・169・271・394 行)で呼ぶ。`guard/scan.sh` の python 規則(L-008)は、`lift/` の変更ファイルの全行を見るので、このファイルを変えると判定が落ちる(前回の試行と同じ)。そこで、このファイルは HEAD のままにした。`seventeen-modules` の 17 → 193 の更新は、規則の扱いが決まってから入れる。(a) 検証だけの試験に限って python を許す、(b) その試験を書き換える、のどちらか。
2. **抽出の木の鍵**: `agent.yml`(2 箇所)と `deploy.yml` の抽出の木の鍵は、`lift/plugins/*/plugin.tsv` を読むが、`lift/crates/fe-build/src/plugins.rs`(ホストのパスを決める)は読まない。このままだと、古い木が復元される。そこで `lift/plugins/fruity-balance/plugin.tsv` に説明のコメント行を足した(鍵の入力なので、鍵が変わり、木を取り直す)。根本の対処は、鍵に `lift/crates/fe-build/src/plugins.rs` を足すこと。ただし `spec/workflow.test.sh` が既存の段の内容を変更前と一致させるので、ワークフローは変えなかった(PM の判断)。
3. **ヒープの余り**(4 の件)。
4. **素の名前の参照**(6 の件)。
5. **`lift/harness/web/fruity/link-web.sh`** は、古いヒープの値(INITIAL 671088640)のままである。fe-build の link は `link.rsp` を使うので、engine には効かない。このスクリプトが使われているかは確かめていない。

## 9. 場面の終わり(ExitProcess の有無)

- 6 回の場面(採る配置 3 回・現行 3 回)すべて、終わりの state は engine-idle、`exit` は null、`error` は null。
- 場面の終わりの log(末尾 25 行と、末尾 60 行の抜粋)に ExitProcess は無い(末尾は `missing-path` の行)。
- 前回の試行の 12 回すべての「ExitProcess(0) の後に終了コード 134」は、この環境では再現しなかった。原因は未確認(前回と条件が同じか、確かめていない)。#1365・#1372 との関係も未確認。
- 流れ D の終わりは、「Problems loading the project」の箱が開いたまま(既存の動作。welcome は閉じている)。

## 10. 起動の停止

- この実行では、起動の停止は無かった(boot-check 1 回、measure 6 回、p3 3 回、welcome-flow 4 回。すべて engine-idle、8.7〜9.2 秒)。前回の現行配置の 7 回中 2 回の停止とは比べられる数が少ない。

## 11. 変えた物

- `lift/harness/web/link.rsp`: `-sGLOBAL_BASE=989659136`、`-sINITIAL_MEMORY=1392312320`。
- `lift/crates/fe-build/src/plugins.rs`: `GLOBAL_BASE` = 0x3AFD0000、`name()` = key、`seed_file()` の名前 = key、`validate()`(ファイル名の一意を外し、key が `.dll` で終わることを加えた)、`read_rows_from` の分割、試験(Balance の期待値の更新、wave2 の 160 本、配置の入る試験。merge の衝突も解いた)。
- `lift/runtime/fle_realmod.c`、`fle_realmod.h`(注釈)、`fle_cursor.c`、`lift/harness/two/main_two.c`: 表の上限 192。
- `lift/crates/fe-lift/tests/wall_dec1.rs`: `rm_max_is_8` が 192 を pin。
- `lift/plugins/fruity-balance/plugin.tsv`: 説明のコメント行(8 の 2)。
- `tools/delivery-split/measure.mjs`: `rssMb`(ブラウザの VmRSS の最大)と `engineMemoryBytes`(参考)を足した。
- `tools/plugin-layout/wave2-plugins.mjs`: 出力の plugin.tsv を 8 列にした(`omit` は `-`)。
- `tools/plugin-layout/image-sizes.mjs`(新)、`inputs/plugin-image-sizes.tsv`(新。SizeOfImage の表)。
- `tools/plugin-layout/make-base.sh`(新): 現行の配置の bundle を、同じ目的物から再リンクして `lift/out-base/` に作る(`lift/out-base/` は git で無視される)。ホストの木と host-extra は `lift/out-app/` へのシンボリックリンク。
- `pm-work/findings/guest-layout.md`: この報告。

`lift/` と `app/`(`app/` は変えていない)を変えたので、main への push は `deploy.yml` を動かす(報告に書く)。

## 12. 次にやること(PM)

- 8 の 1(`realmod.rs` の試験の扱い)と 2(抽出の木の鍵)を決める。
- 4(ヒープの余り)。`MAXIMUM_MEMORY` を上げるか、像の置き方を変えるかを決める。
- 6 の未確認(素の名前の参照が、パスか素の名前か)を、`engine_x64.dll` を持つ DLL で確かめる。
