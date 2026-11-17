# 0405: 旧ビルドスクリプトを削除する(deploy.yml・fe-lift・fe-wasmgen への置き換えの後の残り)

分類: 整備(a の除去。置き換えた旧経路の削除)。状態: 仮の決定。
日付: 2026-11-16   ROADMAP: R6-32(doing。done にしない)   一覧: なし
証跡: `evidence/R6-32.md`(削除の一覧、参照の検索の出力、検査の出力、deploy の run)
撤回条件: deploy の run(paths の ci/**・lift/**・app/** で起動)が失敗し、旧経路が要るとき。旧版は削除の直前の commit(0ee94698)に残り、そこから戻せる。

## 0 範囲と件数(先頭。決定の前に確かめた物)

### 0.1 依頼者の過去の発言
- 検索範囲: 前任の記録 `~/work/handover/setup/S16-intent/catalog.md` の全 29 件(No 1〜29。2026-11-10 までの発言)。旧スクリプトの削除・残置に直接触れる指示は 0 件。近い型は No 11(`msg_1b7bb7b92001i7T2kIeDM1AT7O`、2026-11-08 05:37:59。「100行近く記述しているのも間違っている」)。
- `opencode api session.message.list` は、この作業者の権限では拒否された(configuration policy)。したがって 2026-11-16 の依頼者の発言は、PM の依頼文に引用された原文を使う。発言の ID は確かめていない(未確認)。
- 依頼文の原文(2026-11-16): 「ワークフロー内で実行するだけの処理にシェルスクリプトは不要です。」「過度な異常系の考慮は全く不要です。削除してください。」
- 決定 0401 が「スクリプト禁止」の出典として引く `msg_1b7bb7b92001i7T2kIeDM1AT7O` を、この決定の出典にも並べる。

### 0.2 参照の検索(git grep。追跡ファイル。docs・evidence・archive・patches を除く)
| 候補 | 生きている物の行(削除後) | 実行・読み込み(削除後に残る物) | 記録の行(docs・evidence・archive・patches) | 決定 |
|---|---|---|---|---|
| app/scripts/build-engine-bundle.sh | 27(削除前 34) | app/package.json(npm script)。ci/lib/common.sh の run_bb・script_part・script_preamble(呼ぶ・読む。common.sh は残す) | 464 | 削除 |
| app/package.json の bundle:engine | 0(削除前 1) | なし | 3 | 行を削除 |
| lift/harness/b2d2/gen-direct.sh | 5 | なし(呼び元の build-engine-bundle.sh が削除対象) | 114 | 削除 |
| lift/harness/b2d2/realmod_tables.awk | 2 | なし(gen-direct.sh が削除対象) | 35 | 削除 |
| lift/harness/two/gen_two.sh | 32 | build_two_native.sh が modules.txt の作成者として名指す(実行はしない) | 156 | 残す(§3-1) |
| lift/harness/two/build_nmod_wasm.sh | 18 | なし(呼び元の build-engine-bundle.sh が削除対象。link-web.sh の使い方の文に名指し) | 121 | 削除 |
| lift/harness/two/merge_tables_n.py | 17 | gen_two.sh:150(残す) | 49 | 残す(§3-2) |
| lift/harness/hyb/chk_lowered.py | 2 | なし(build-engine-bundle.sh が削除対象) | 24 | 削除 |
| lift/harness/hyb/wasm_c_imports.py | 1 | なし(コメントのみ) | 20 | 削除 |
| lift/harness/hyb/strip_glue_stubs.py | 0 | なし | 21 | 削除 |
| lift/harness/two/split_bytes.py | 13 | lift/crates/fe-lift/tests/split_units.rs:76(cargo テスト) | 34 | 残す(§3-3) |
| lift/harness/two/lowregs.py | 31 | split_units.rs:79、lift/crates/fe-lift/tests/size1_lowregs.rs:106(cargo テスト) | 118 | 残す(§3-4) |
| lift/harness/hyb/strip_units.py | 7 | lift/crates/fe-lift/tests/split_units.rs:87(cargo テスト) | 27 | 残す(§3-5) |
| lift/crates/fe-lift/tests/realmod_s1/tools/split_mod.py | 13 | lift/crates/fe-lift/tests/realmod.rs:140(cargo テスト)、gen_two.sh(残す) | 33 | 残す(§3-6) |
| lift/harness/web/fruity/link-web.sh | 6 | lift/harness/parity/make-test-tree.sh:21・22(読んで書き換える) | 97 | 残す(§3-7) |
| lift/harness/b2d1/build_upstream.sh | 6 | なし(lift/harness/parity/native-build.sh:5 が入力の作り方として名指し) | 41 | 残す(§3-8) |
| ci/setup-toolchain.sh | 0 | なし | 27 | 削除 |
| ci/lib/common.sh | 2 | lift/harness-docs-tools/alpha1/mk-wasm.sh:12(`. ci/lib/common.sh`) | 32 | 残す(§3-9) |
| ci/lib/wrangler_config.py | 0 | なし | 42 | 削除 |
| ci/verified-bundles.tsv | 0 | なし | 30 | 削除 |

「生きている物の行」は `git grep -h -F`、「実行・読み込み」は各行の読み方(呼ぶ・読む・名指す)で分けた。全行の一覧は `evidence/R6-32.md` §1。

### 0.3 件数
- 削除: 10 ファイル、832 行(削除前の `wc -l` の合計)。
- 編集: app/package.json の scripts の 1 行(bundle:engine)。docs の 19 行(check-consistency が指摘した行だけ。パスを「(削除済み)」の表記にした)。
- 残す: 9 件(§3)。

## 1 決定
1. 削除する(10 ファイル。§2)。
2. app/package.json の bundle:engine を削除する。`npm run build` と `npm test` は変えない(deploy.yml は build だけを呼ぶ)。
3. 残す(9 件。§3)。残した理由は、生きているテストか道具がその物を実行・読み込むこと、または名指しの入力の作り方が他に無いこと。
4. docs は check-consistency が指摘した 19 行だけを直す。決定と証跡の本文は変えない。
5. 置き換えの確かめは deploy の run。engine の鍵 e5fecd8d93d5 が基準と一致した run 43901558294(success)を、既存の記録(evidence/R6-30.md、R6-31.md)で確かめた。この commit の後の run は evidence/R6-32.md に記す。
分類: 整備(a の除去)。根拠: 旧経路の 9 工程は deploy.yml と fe-lift gen-all・fe-wasmgen hybrid に置き換え済み(R6-28、R6-29、R6-30、R6-31)。

## 2 削除の一覧と置き換え先
| 削除(10 件) | 行数 | 旧の機能 | 置き換え先(今の場所) |
|---|---|---|---|
| app/scripts/build-engine-bundle.sh | 456 | 9 工程(gen・resplit・wasm・hybn0・weblink・wasmopt・hybrid・seed・bundle) | deploy.yml(29〜100 行)。gen は `fe-lift gen-all`(lift/harness/b2d2/gen.args)、hybn0 と hybrid は `fe-wasmgen hybrid-n0` と `fe-wasmgen`(lift/harness/hyb/n0.cmd・hybrid.cmd)、seed は pack_seed.py、bundle は pack-assets.mjs(どちらも変えていない) |
| lift/harness/b2d2/gen-direct.sh | 47 | 生成段階(gen_two の rebase・gen・split・merge の呼び出し) | fe-lift gen-all(lift/crates/fe-lift/src/gen_all.rs。R6-28) |
| lift/harness/b2d2/realmod_tables.awk | 23 | realmod 表の C の出力 | gen_all.rs の realmod()(同じ出力) |
| lift/harness/two/build_nmod_wasm.sh | 81 | wasm の rt・harness・units・o1・link(emcc) | deploy.yml の emcc 行(59〜84 行。lift/harness/web/cflags-*.args) |
| lift/harness/hyb/chk_lowered.py | 19 | 一覧の関数が fe-wasmgen で出たかの検査 | lift/crates/fe-wasmgen/src/hybrid.rs の検査(R6-29) |
| lift/harness/hyb/wasm_c_imports.py | 69 | wasm の import 名の列挙(C 部の import が 0 件、f_ の import が 0 件の検査) | hybrid.rs の import の読み(R6-29) |
| lift/harness/hyb/strip_glue_stubs.py | 11 | glue の未定義スタブの除去 | deploy.yml の `sed -E -f lift/harness/web/glue.sed`(85 行) |
| ci/setup-toolchain.sh | 27 | emsdk 6.0.12・rust 1.99.0 の導入 | deploy.yml の mymindstorm/setup-emsdk@v16(6.0.12)。rust は runner の cargo(版の固定なし。§5-4) |
| ci/lib/wrangler_config.py | 97 | Worker の設定の書き出し | app/wrangler.jsonc(決定 0403) |
| ci/verified-bundles.tsv | 2 | 検証済みの wasm の一覧(root の差し替え用) | 置かない(決定 0400) |

編集(1 行): app/package.json の scripts の bundle:engine。

### 2-1 消える検査(依頼者の指示により不要)
build-engine-bundle.sh の中の、次の検査は削除と同時に消える。依頼文(2026-11-16)の「過度な異常系の考慮は全く不要です。削除してください。」に従う。
- hyb_guard(リンクマップの番地の検査 4 種)。移していない。
- memguard(MemAvailable の下限)、capped(cgroup の上限。systemd-run)。
- 工程の飛ばし判定(出力の有無)、件数の検査、die の 58 箇所。
- chk_lowered と wasm_c_imports の検査は、fe-wasmgen の hybrid に同じ規則で移した(R6-29)。
- 停止時の案内(pack-assets の --stop-note 2 件・--description・--brotli-quality)は、この決定で消えるものではない。`lift/harness/web/pack.args` を 7f6a4cba で作った時に欠けていた。戻した(経緯は `docs/failures/0022-pack-args-stop-note-dropped.md`)。

## 3 残した 9 件と理由
| 残す | 行数 | 理由 | 参照している物 |
|---|---|---|---|
| §3-1 lift/harness/two/gen_two.sh | 154 | lift/harness/two/build_two_native.sh(生きている native の作成)は $V/modules.txt を要求し、その作成者はこの file だけ。gen-all は modules.txt を書かない(gen_all.rs の出力一覧に無い) | build_two_native.sh の要求の文(2 行)。実行は 0 件 |
| §3-2 lift/harness/two/merge_tables_n.py | 93 | gen_two.sh が実行する(表の統合) | gen_two.sh:150 |
| §3-3 lift/harness/two/split_bytes.py | 43 | R6-27 の受け入れテスト(cargo test split_units)が基準として実行する | split_units.rs:76 |
| §3-4 lift/harness/two/lowregs.py | 31 | split_units.rs と size1_lowregs.rs が実行する | split_units.rs:79、size1_lowregs.rs:106 |
| §3-5 lift/harness/hyb/strip_units.py | 30 | split_units.rs が基準として実行する | split_units.rs:87 |
| §3-6 lift/crates/fe-lift/tests/realmod_s1/tools/split_mod.py | 31 | realmod.rs が実行する。gen_two.sh(残す)も実行する | realmod.rs:140、gen_two.sh:130・135 |
| §3-7 lift/harness/web/fruity/link-web.sh | 30 | parity の make-test-tree.sh が読んで書き換える(rep)。削除できない | make-test-tree.sh:21・22 |
| §3-8 lift/harness/b2d1/build_upstream.sh | 29 | parity の native-build.sh の入力(UP_NATIVE_LIB・UP_SRC)の作り方として名指し。native の Blend2D を作る手順は deploy.yml に無い(wasm だけ) | native-build.sh:5 |
| §3-9 ci/lib/common.sh | 69 | lift/harness-docs-tools/alpha1/mk-wasm.sh が読む | mk-wasm.sh:12 |

残した 9 件は、PM が決める。gen_two.sh と build_upstream.sh は、残す理由が名指しの入力だけなので、削除に変えてよい(その場合、build_two_native.sh と native-build.sh の使い方が使えなくなる)。

## 4 残る物の扱い(動作に影響しない)
- ci/lib/common.sh の run_bb(68 行)は、削除した build-engine-bundle.sh を呼ぶ。script_part(28 行)と script_preamble(31 行)は同じ物を読む。common.sh は残す(§3-9)。mk-wasm.sh は、この run_bb 経由で削除物を呼ぶ。mk-wasm.sh は追跡されない出力(lift/out-gpu1 等)に依存し、リポジトリの中では動かない(読み。実行は未確認)。
- 使い方の文だけが古くなる物(実行はしない)。件数と場所は evidence/R6-32.md §1。主なもの: lift/harness/parity/native-build.sh(WORK は削除した build-engine-bundle.sh の gen・resplit の出力)、lift/harness/parity/make-test-tree.sh(usage)、lift/harness/web/fruity/link-web.sh(OBJ は削除した build_nmod_wasm.sh の出力)、lift/harness/two/build_two_native.sh(modules.txt の作り方)。
- lift/crates/fe-lift/src/gen_all.rs:281 の出力文字列「generated by lift/harness/b2d2/gen-direct.sh」は、生成する realmod.c の先頭に入る。直していない。直すと出力が変わり、R6-28 の sha256 の照合の基準が変わるため。

## 5 残る問題(未確認)
1. ci/lib/common.sh と mk-wasm.sh の残置(§4)。
2. 使い方の文の更新は、この決定では行わない(文書の指摘が無い限り触らない。件数は evidence/R6-32.md)。
3. verified-bundles(検証済みの一覧)と root の差し替えは、決定 0400 で置かないと決めた。この決定でも変わらない。
4. rust の版: 旧は setup-toolchain.sh が rustc 1.99.0 を固定していた。deploy.yml は runner の cargo を使い、版を固定しない。engine の鍵が基準 e5fecd8d93d5 と一致した run 43901558294 で、この差の影響は出ていない。runner の rustc の版は未確認(evidence/R6-31.md §9)。
5. 旧の upload-commands.txt(R6-31 §8)は、既に使っていない(deploy.yml は aws s3 sync で載せる)。

## 6 出典
- 削除前の行数: `wc -l`(evidence/R6-32.md §2)。
- 参照の検索: git grep(evidence/R6-32.md §1)。
- 依頼文の原文: PM の依頼文(2026-11-16)。発言の ID は未確認。決定 0401 が引く msg_1b7bb7b92001i7T2kIeDM1AT7O。
- engine の鍵の基準: run 43901558294(success)、evidence/R6-30.md §2、evidence/R6-31.md §3。
