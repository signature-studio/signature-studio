# 0415: 本番の経路の Python を Node に移す(抽出と seed pack)。製品ソースの生成元の Python は削除する。製品の入力データの生成元は検証の道具として残す。scripts/ の検査は Node に移す。製品の Python は検査で止める(依頼者の恒久指示 2026-11-17)
分類: (a 除去)。移植は同じ入力で同じ出力(sha256 と cmp で確かめた)。新しい処理・旗・環境変数・`#ifdef` は 0 件。
撤回条件: (1) 依頼者が、検証の道具を含めて Python を禁じる範囲を広げたとき。(2) deploy の run で engine・seed pack・DLL pack の鍵が移植の前と違うとき(原因が移植なら戻す)。(3) 削除した生成元が、再生成を要すると判明したとき(その場合は Node の版を作る)。(4) 依頼者から、範囲の外と範囲の内の検証の道具について別の指示が来たとき(下の §範囲と件数。その場合は Node への移植か削除)。
日付: 2026-11-17   ROADMAP: R6-37、R6-37b   一覧: 変更なし   コミット: 2a512da9..27f3bcb4(差し替え)、9590e599(原本の削除)、0079e4d5(生成元の削除と規則)、01bc7124(規則の穴・許可・入力データの来歴・範囲の数え直し)、この決定の commit(scripts の移植・閉包の全件・区分・os-fonts・main の判定)

注: 決定番号 0414 は R6-36a が使用(`0414-setup-emsdk-origin-and-names.md`)。0412 は origin に 2 本、0413 は R6-38。この文書は 0415。

## 決定
1. 決定: 依頼者の恒久指示を AGENTS.md §6 に 1 行で書き、§11 の表に検査を置く(scripts/scan-forbidden.sh の python 規則。テストは tests/harness/t12_python.sh)。根拠: 依頼者の原文(2026-11-17)。
2. 決定: 本番の経路が呼ぶ 2 本を Node に移す。ci/lib/extract_fl.py → app/scripts/extract-fl.mjs(依存: app/scripts/lib/nsis.mjs・nsis-script.mjs・lzma1.mjs)。lift/harness/web/final0129/pack_seed.py → app/scripts/pack-seed.mjs。根拠: 棚卸し A(移植の前は 2 件、evidence/R6-37.md §1.1)。依頼者は node を許可している(main の指示)。
3. 決定: 抽出を 7z で置き換えない。根拠: 7z 26.00 は NSIS のインストーラを開けない(evidence/R6-37.md §2)。
4. 決定: Rust にしない。根拠: 抽出は agent.yml の extract の行でも動く必要があり、そこは Rust の toolchain を入れていない。LZMA の crate はこの機械のレジストリに無い。LZMA1 は app/scripts/lib/lzma1.mjs(自前、依存を増やさない。app/package.json の依存は fflate だけ)。
5. 決定: 原本の Python 2 本は、deploy の run 44034012920(success)の後に削除した(9590e599。run 44044050468 も success)。根拠: 移植を戻せるように、run の確認まで残した。
6. 決定: 許可リストは名指しにする(scripts/forbidden-allowlist.txt の python 節)。dir 単位は、deploy の入力を含まない検査・測定・テストの道具だけのディレクトリ(lift/crates/fe-lift/tests・lift/crates/fe-wasmgen/tests・lift/runtime/test・lift/harness-docs-tools、lift/harness の alpha1・api2・census・fp1・idle・noreturn-audit・parity・testfast・vst3a)。file 単位は、deploy の入力を含むディレクトリ(lift/harness の b2d2・hyb・seeds・two・web)と、wasm の split_c.py(wasm の dir には deploy の入力 trace_fruity.c があるので dir 単位にしない)。行単位は、名指しの .sh の python 行(lift/harness/test.sh・two/build_two_native.sh・two/chain.sh の 2 行・two/gen_two.sh・web/threads/compare_scen.sh・wasm/build.sh)。各項目の前に理由の行を付けた。根拠: 依頼者の「既存のコードベース全て」と「調査・検証を除く」の読み(下の §読み)。確かめ方: evidence/R6-37.md §8.3(dir 許可の下に deploy の入力が 0 件)。scripts/ には python が無い(決定 11)。
7. 決定: 検査・測定の道具(tools/verify-kit・tools/win-replay・lift/harness の測定とビルド・テストの生成器・記録の道具)は残す。根拠: 依頼者の指示は「調査・検証を除く」。範囲の内の 99 件は名指しで、範囲の外の 149 件は下の §範囲と件数の通り。
8. 決定(R6-37b): 製品ソース(lift/runtime)を生成していた Python 9 件は、生成元を削除し、生成物を正本とする(分類 整備)。根拠: (1) 再生成は今の作業に要らない。ビルドは生成物を読むだけで、どの CI もテストも生成元を実行しない。(2) 3 件の再生成の入力(保存した Learn のページ・UnicodeData 16.0.0・host の CPU の測定)は repo に無い。(3) 6 件は生成元が git に無い。削除した 3 件: tools/gen_errmsg_hresult.py・lift/runtime/test/gen_crash1_case.py・lift/crates/fe-lift/tests/rcp/gen_rcp_table.py。生成元の本文は削除の前の commit で読める。製品ソース(.h・.inc)の先頭の注釈は変えていない(製品のバイトを変えないため)。注釈が指す生成元は、削除されたことを前提に読む。9 件の名前と、git ls-files が 0 行であることの確かめ方は ROADMAP R6-37b の完了条件。
9. 決定: 生成元の再生成が要る日が来たら、Node の版を作る(再生成の入力を repo に入れる判断を伴う。その時の決定で扱う)。
10. 決定(deploy が読む入力データの来歴)。deploy と fe-build が読む入力を、args・cmd・rsp・sed・txt・tsv の参照で推移的に閉じた集合(evidence/R6-37.md §8.1。閉包は 83 件: コード 21 件、非コードのデータ 62 件)として求め、データを生成元と結び付けた。判定の基準は 3 つ。(a) 再生成の入力が repo にあるか。(b) deploy が Python を実行するか。(c) 生成物が正本か(commit されていて deploy が読むか)。閉包の中の全件は、区分 G1〜G5 のどれかに入る(区分単位の決定。件ごとの根拠は区分の根拠と、証跡 §8.2 の件名)。どの件も (b) は「実行しない」(deploy.yml・agent.yml・fe-build の起動物は evidence/R6-37.md §6)。
   - G1: 生成元が木にある Python 道具(来歴が特定できた)。決定: 残す(検証と来歴の道具)。根拠: (c) 生成物が正本で commit されており、deploy はそれを読むだけ。(a) 再生成の入力(インストーラの中身・ゲストの trace・upstream の一覧)は repo に無い。削除すると、正本の来歴を再現できなくなる。Node に移さない理由: 道具の出力は正本と比べる検査の基準であり、移すと検査の基準が移植先の実装に変わる(別の決定が要る)。件:
     - lift/harness/seeds/authenticode-claims.tsv ← tools/authenticode_check.py(先頭の注釈。cargo test authenticode_check.rs が道具を実行する)。
     - lift/harness/seeds/reg-seed-nsis.tsv ← tools/nsis_script_dump.py --registry(先頭の注釈。wall_fr3.rs・wall_fr5.rs が読む)。
     - lift/harness/seeds/vfs-seed-idle1.tsv ← lift/harness/seeds/gen_vfs_seed_installed.py(先頭の注釈が名指す。この道具は gen_install_tree.py を import し、gen_install_tree.py は tools/extract_nsis2.py と tools/nsis_script_dump.py を import する)。
     - lift/harness/seeds/iltools-6000000-seeds400.txt ← lift/harness/seeds/refine_seeds.py(docs/archive/decisions/0104-iltools-realmodule.md: 14,456 個から 400 個。400 行と一致)。
     - lift/harness/hyb/lists/lowered-{b2d,dsp,elastique,fle,ft,ilt,oggio,qfc}.txt(8 件)← seeds/hyb-U-{同じ 8 モジュール}.txt と VA の集合が一致(件数 76・158・165・2758・176・132・46・364)。
     - lift/harness/hyb/lowered-all.txt ← 上の 8 件の和集合(3,875 件と一致)。deploy は b2d2/gen.args:12 で読む。
     - seeds/hyb-U-*.txt ← lift/harness/hyb/filter-hybrid-list.py(実体の名前。注釈は tools/ と書く)が、LOCK 付きの命令を持つ関数を除いた(mt4 stage A)。除去の件数は、各ファイルの先頭の注釈による: b2d 9・dsp 2・elastique(件数の注記は無い)・fle 32・ft 3・ilt 10・oggio 1・qfc 9(計 66)。除去の一覧 seeds/mt4-atomic-functions.tsv ← lift/harness/hyb/list-atomic-functions.py(注釈が名指す)。hyb-U-*.txt の除去前の一覧(decision 0239 の operation scripts が実行した関数)は、その生成物が repo に無く、未確認。
   - G2: 注釈が名指す生成元が木に無い(記録のみ)。決定: 残す(正本のデータ。生成元は repo に無い。削除もしない)。根拠: 生成元を失っても、deploy の入力(commit された正本)は変わらない。今の作業で再生成は要らない(撤回条件 3)。件:
     - lift/data/prune-SM.tsv ← 注釈が tools/prune/judge.py を名指す。tools/prune/ は木に無い(`git ls-files tools/prune` の出力は 0 行)。deploy は lift/harness/b2d2/modules.tsv の --prune-list で読む。
     - lift/harness/seeds/dec1-seeds-oggio-c000000.txt ← decision 0206 が tools/mk_leaf_seeds.py を名指す(木に無い)。作った道具の同一性は未確認。
     - lift/harness/seeds/elastique-seeds-all3.txt ← decision 0200 が tools/elq-jumptables.py を名指す(木に無い)。このリストを作った道具は未確認。
     - lift/harness/seeds/os-fonts/NotoSansCJKjp-subset-msgothic.ttc ← os-fonts/README.md が tools/make-msgothic.py(fonttools 4.68.0・brotli 1.2.1 の venv で python3 が動かす)を名指す。木に無い(`git ls-files | grep -i msgothic` の出力は .ttc の 1 行だけ)。subset の作り方は README に書かれており、再生成は repo からはできない。
   - G3: 来歴が特定できない(生成元の記録が repo と docs に無い)。決定: 残す(正本のデータ)。根拠: 生成元の特定は未確認のまま。削除も移植もしない(移植先が無い)。件:
     - lift/harness/seeds/ft-seeds-4b00000-c2.txt(decision 0123 は「新規」と書く。作った道具は未確認)。
     - lift/harness/seeds/dsp-ippv2-2000000-seeds-idle1.txt(作った道具は未確認)。
     - lift/harness/seeds/flengine-seeds-thunks.txt(8 行。作った道具は未確認)。
     - lift/harness/hyb/c-imports-engine.txt(611 行の import 名。作った道具は未確認)。
     - lift/harness/web/verified.txt(engine の sha の記録。書いた者は未確認)。
   - G4: 生成元が Python でない(残す)。根拠: 生成元が Node・Rust・shell で、本件の規則の対象外。件:
     - lift/harness/seeds/ship6-imports.txt(注釈: `cat ship5-imports.txt dwmti-imports.txt > ship6-imports.txt`)。
     - lift/harness/hooks/ の閉包の中の 6 件(blend2d-no-jit-sites.txt 10 行・dsp-hle-entries.txt 7 行・flengine-hook-points.txt 13 行・fruity-delphi-effect-hook-points.txt 14 行・iltools-hook-points.txt 21 行・qfc-hook-points.txt 16 行。いずれも注釈が fe-lift の --hook-points・--hle-entries・--site-overrides(Rust)を名指す)。
     - 閉包の外の 7 件目: lift/harness/hooks/wrapper-hook-points.txt(9 行)。閉包に入らない理由: deploy が読むどのファイルも名指さない(`git grep -n wrapper-hook-points` の出力は docs/archive の 0183 と 0309 だけ)。Fruity Wrapper の DLL は、lift/harness/b2d2/modules.tsv の deploy の module の一覧に無い。残す(調査の記録。deploy の入力ではない)。
   - G5: 手で書かれた入力(生成元が無い。選択の記録)。決定: 残す。根拠: 正本の入力そのもの。件:
     - lift/harness/seeds/reg-seed-app.tsv(decision 0116。注釈: installer の出典ではない「出典の無い選択」)。
     - lift/harness/fruity-rows.txt(harness の選択。注釈が書く)。
     - lift/harness/b2d1/blend2d-context-swap.txt(decision 0160)、lift/harness/b2d1/blend2d-sources.txt、blend2d-4700000-seeds-wrapper.txt、b2d3-compop.patch(手で書いた一覧と patch)。
     - lift/harness/b2d2/gen.args・fle.args・modules.tsv、lift/harness/hyb/n0.args・h.args・n0.cmd・hybrid.cmd・hyb-args.tsv・wasm-features.txt、lift/harness/two/plain-units.txt・ai-units.txt・harness-inputs.txt、lift/harness/web/*.args・*.rsp・*.cmd・*.sed・root.txt(設定と一覧)。
     - lift/harness/seeds/os-fonts/ の 3 件の .ttf・.ttc 以外: LiberationSans-Regular.ttf・DejaVuSans.ttf・DejaVuSansMono.ttf は、Debian/Ubuntu の fonts-liberation 1:2.1.5-3build1・fonts-dejavu-core 2.37-8build1 の同一のバイト(os-fonts/README.md。SHA-256 も書かれている)。来歴は特定できた。生成元の Python は無い。G5 に入れる。
     - lift/harness/seeds/os-fonts/ の README.md と COPYRIGHT-*.txt 3 件(閉包の外。deploy は読まない)。決定: 残す。根拠: README.md は 4 件の出典の記録(来歴)。COPYRIGHT は、再配布する font の著作権・ライセンスの文(DejaVu・Liberation(OFL 1.1)・Noto)で、font と一緒に残す義務がある。
     - ci/extract-dlls.tsv の注釈(2026-11-16 と 2026-11-17 の行)は、削除済みの ci/lib/extract_fl.py を名指す。呼び出しではない。注釈の行を 1 行足し、削除済みと書いた(app/scripts/extract-fl.mjs が読み、# の行は読み飛ばす。app/scripts/extract-fl.mjs:27)。
11. 決定(scripts/ の検査の移植。3 回目・4 回目・5 回目の差し戻しの K2): scripts/check-consistency.sh・audit-session.sh・preflight.sh の python を Node に移した(heredoc の node。新しいファイルは作らない。文書に載っていない scripts/ の検出(g 規則)を避けるため)。根拠: 依頼者の「一切の Python の使用を禁止(調査・検証を除く)」と、scripts/ は製品の範囲(AGENTS §11)。検査の道具は検証と読めるが、移植は小さく、既存の検査が挙動を守る(t06 105・t07 78・t10 67 が通る)。移植の確かめ: check-consistency の出力は、移植の前後で実リポジトリと、誤りを仕込んだ sandbox の全行で一致した(証跡 §5.5 に全文)。audit-session の since の変換は、Python の値(1794877500000)と一致し、存在しない日付は拒む。preflight の JSONC の読込は、audit と同じ規則。

## 範囲と件数(今の木で数えた。コマンドと出力は evidence/R6-37.md §1)
- 追跡ファイルの .py は 248 件(`git ls-files '*.py'`)。検査の範囲内(.github・ci・app・scripts・lift)は 99 件、範囲の外は 149 件。
- 範囲の内の 99 件は、全件が許可リストに名指しされている(未許可 0 件)。dir 単位 72 件: cargo テストの生成器とオラクル 9 件(lift/crates/fe-lift/tests 8、fe-wasmgen/tests 1)、runtime テストの期待値の生成器 4 件(lift/runtime/test。fe-build は lift/runtime 直下の fle_*.c だけを読む)、記録の表を作った道具 14 件(lift/harness-docs-tools)、harness の測定とビルドの道具 45 件(alpha1 2・census 2・fp1 1・idle 1・noreturn-audit 1・parity 14・testfast 1・vst3a 23)。file 単位 27 件: b2d2 2・hyb 3・seeds 4・two 16・wasm 1・web 1。
- 基底名が docs・evidence・archive・patches・.md の外で参照されない .py は 149 件。参照ありは 99 件。2 × 2 の表:

| | 参照あり | 参照なし | 計 |
|---|---|---|---|
| 範囲の内 | 78 | 21 | 99 |
| 範囲の外 | 21 | 128 | 149 |
| 計 | 99 | 149 | 248 |

- 参照なし 149 件の内訳:
  - 範囲の内 21 件: lift/harness/parity 8、lift/harness/vst3a 11、lift/harness-docs-tools/vst2 2。許可された dir の中の計測の道具。どの build も test も基底名で呼ばない。残す理由: 計測の記録の再現。
  - 範囲の外 128 件: archive/ 73(下の 2 件の vst を含む)、tools/ 40、docs/archive/ 9、evidence/ 6。
- archive/ の扱い(テストが読む・実行する物を名指す。確かめは `git grep -n "archive/" -- lift/crates tests lift/harness lift/runtime ci app scripts .github`):
  - archive/harness-attic/a4-multi.sh(.sh)は、lift/crates/fe-lift/tests/wall_fr3.rs:575(実行)と 616(読込)、wall_fr4.rs:996(実行)と 1035(読込)が使う。このスクリプトの python3 は、harness/wasm/split_c.py(392 行)と、gen_two.sh(409 行)経由の python。テストは REG_SEED・AUTH_CLAIMS の不正な値で早期に拒否するので、この python への実行に届くかは未確認。
  - archive/harness-attic/vis1/vis1_apis.inc と link-flags.txt は、wall_fr21.rs:414-415 が読む(.py は無い)。archive/harness-attic/web/opfs1 は、wall_opfs1_map.rs:15 が読む(.py は無い)。
  - archive/harness-attic/vst の 2 件(count_instrument.py・mk_common.py)は、noreturn_audit.rs:69 が、ディレクトリを一時の木へ丸ごとコピーする(実行はしない。コピーだけ)。
  - 上記以外の archive の .py は、どの test も build も読まない(基底名の参照 0)。残す理由: 退役した旧い道具の記録で、docs/archive が名指す。
- 範囲の外の残す理由: tools/verify-kit 37 件(検証の道具)、tools/ 直下 3 件(count_rip_indirect.py・find_nsis_by_size.py・provisional-census.py。調査の道具)、docs/archive/investigations 9 件(調査の道具。文書と一緒に残す)、evidence 6 件(計測の記録の道具)。
- 参照ありの範囲の外 21 件の例: archive/harness-attic/pick1/html2txt.py は lift/harness-docs-tools/alpha1/mk-results.sh が名指す。archive の 4 件(html2txt.py・mk-act-matrix.py・web/pw1/summarize.py・web/srm1/summarize.py)は、基底名の一致だけを見た。同名の別ファイルの可能性があり、参照の有無は未確認。
- 依頼者への確認: 範囲の外と範囲の内の検証の道具を残すことを、依頼者に確かめた記録は無い。handover の S16-intent/catalog.md(更新日 2026-11-10)に Python の語は 0 件(grep -c の出力)。
- main の判定(5 回目の検査の後。main が書いた文の転記。判定は main): 範囲外 149 件と範囲内 99 件は、依頼者の例外(調査・検証)に当たる道具・記録として残す。根拠: 製品・deploy・agent・scripts の実行・fe-build の起動物に python の呼び出しが 0 件。範囲内 99 件は cargo テストと harness の検証の道具(オラクル)。依頼者から別の指示が来たときは、Node への移植か削除。これで解釈の幅は、main の判定として閉じる。

## 読み(解釈。実行していない)
- 本番の経路に残っていた Python は、2 本の道具(抽出と seed pack)と agent.yml の同じ抽出だけだった。その他のステップは、cargo・fe-build・emcc・wasm-opt・node(pack-assets.mjs)・aws・wrangler で動く。scripts/ の 3 本は、本番の経路ではないが製品の範囲だったので Node に移した(決定 11)。
- 依頼者の指示「調査・検証を除き」の読み: 検証の道具(オラクル・測定・テストの入力の生成器・harness のビルド・来歴の道具)は残す。本番の経路と製品のソースは Node か Rust。製品のソースを生成する Python は、R6-37b で削除した。

## 実測(コマンドと出力は evidence/R6-37.md)
- 移植の前後の鍵(deploy の run のログ): engine `b440e13a623a`、seed `d6d345ed0b46`、DLL pack `bce3049ad4d1`。移植の前の run 44024013509 と、移植の後の run 44034012920 で同じ。
- seed pack: Node の版の出力は、Python 版の基準と cmp で一致(20,780,949 B、sha256 `d6d345ed0b4690b057dfc9ee9e3b281ce02ac9a6656abfa94d7ee3e7343ac13c`、manifest も同じ)。
- 抽出: Node の版の manifest と木は、Python 版のオラクルと cmp・diff -r で一致(238 ファイル)。

## 未確認
- agent.yml の node の抽出のステップ(ci/tasks の push でしか動かない。未実行)。
- 決定 10 の G2・G3 の生成元(tools/prune/judge.py・tools/mk_leaf_seeds.py・tools/elq-jumptables.py・tools/make-msgothic.py は木に無い。G3 は記録も無い)。
- archive/harness-attic/a4-multi.sh の python3 への実行に届くか(テストが早期に拒否するため未確認)。

## 出典
- 依頼者の恒久指示(2026-11-17。main の指示の原文)。AGENTS.md §6・§11(本決定の行)。
- 決定 0400(deploy の 100 行・80 字・スクリプト禁止)。決定 0401(fe-build の方針)。
- 失態 0026(`docs/failures/0026-python-in-production-path.md`)と 0029(`docs/failures/0029-r6-37-reviewer-rejected-three-times.md`)と 0032(`docs/failures/0032-r6-37-fourth-rejection-stale-tree-and-unread-claims.md`)。
- docs/archive/decisions/0104-iltools-realmodule.md(refine_seeds.py の出力 400 件)。
- lift/harness/seeds/os-fonts/README.md(4 件の出典と SHA-256、msgothic の作り方)。
