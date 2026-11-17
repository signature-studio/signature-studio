# 0412: エンジンのビルド手順(deploy.yml の 51〜95 行の shell)を Cargo のクレート fe-build の 1 つのプログラムに写す(本プロジェクトの build に CMake は採らない)
分類: (a) 除去(deploy.yml の shell とつなぎ(mkdir・cp・mv・xargs・$(cat)・sed・glob)を消し、同じ処理を fe-build に写す)。新しく書いた処理の分類: sed の 2 規則の移植は (a) と同じ処理の写し。並列コンパイル(run_parallel)は build の時間だけに効き、製品の出力に影響しない。
分類(--plan): CLI から外した(no-options の方針。決定 0243)。plan を作る関数は残り、cargo test が tests/plan/*.txt と比べる(R6-36c §H)。
分類(存在ガード): CheckSources・rm の NotFound の無視・lift/Cargo.toml の確認・emcc の PATH 探索は、入力が無いときに止めるだけで、製品の出力に影響しない(build の入力の確かめ)。
撤回条件: (1) 最終の engine.wasm の sha256 が lift/harness/web/verified.txt の b440e13a623a(64 桁)と一致しないとき(最初に違う中間物を探し、本決定を見直す)。(2) 依頼者が fe-build の起動する emcc・em++・emar・wasm-opt・git・patch・aws・node の使用を禁じたとき。(3) 依頼者が deploy の run の中の、cargo・fe-build・npm の 1 行の呼び出しを禁じたとき。
日付: 2026-11-17   ROADMAP: R6-36b(doing)・R6-36c(doing)   一覧: なし   証跡: `evidence/R6-36b.md`(§8 は engine の sha と deploy の run。§9 は deploy.yml の整理)、`evidence/R6-36c.md`(上流 Blend2D の CMake をやめた記録)、`evidence/R6-36b/deploy-draft.yml`(初期の下書き。現在の deploy.yml とは違う)
コード: fe-build は origin/main に入っている。branch r36b の 9a78d633 は main の 206ad1d1 として cherry-pick された(同じ内容)。以降の fe-build の変更は main の eae8615c・3da1bc18・3de663ec・0e4a2c86 など。206ad1d1 の commit メッセージの「main には入れない(検証前)」は cherry-pick 前の文面で、事実と違う(履歴は書き換えない)。
番号の理由: 2026-11-17 の 2a512da9 時点で docs/decisions の最後は 0416(0412 は R6-36b の番号)。現在(d1dda509 の時点)の最後は 0421 まで(git ls-tree で確かめた)。
改訂の経緯: 初版は CMake + Ninja だった。依頼者の発言(main 経由の原文「流石にCMakeなんて醜いものは採用しないですよね？」。発言 ID は未確認)により、本プロジェクトの build に CMake は採らない。ただし初版は上流 Blend2D の CMake の呼び出し(emcmake・cmake --build)を残していた。これは 0417(R6-36c)で外した。恒久指示(main 経由の原文「調査・検証を除き、一切のPythonの使用を禁止します。この指示は、既存のコードベース全てに適用されます。」。発言 ID は未確認)により、新しい経路に Python を入れない。

## 0 範囲と件数(設計の前の確認)

| 範囲 | 確認 | 件数と結果 |
|---|---|---|
| 依頼者の発言(01a。2026-10-19〜11-08) | `grep -c '^## seq'` | 263(件数は一致) |
| 同、ビルド・ツールチェイン語 | 発言単位に awk で集計 | 5 件: seq 33918 msg_19d1f19a5001zxBVjoHjBWcuAe、seq 34564 msg_19d8b7e90001ARNs0DXVNMmDEc、seq 44747 msg_1a2ba25fa001KucAOXwpOPz7tj、seq 44954 msg_1a2ccc2c3001RSHoExRGznB5pH、seq 47689 msg_1a37ef303001g81KcbrsJRey36 |
| Rust 採用の決定 | 同 | seq 74 msg_1502eb3cd001LU4sxYbGsre74H、seq 131 msg_15034d596001HdCrYgl5mxPtfQ |
| 2026-11-16〜17 の発言(main 経由の原文) | 01a は 11-08 までのため対象外 | CMake 却下、Python 全面禁止、deploy.yml のシェル禁止、100 行、ファイル制限の迂回禁止。発言 ID は未確認 |
| 置き換える前の機能 | `sed -n 51,95p` の 45 行(旧 deploy.yml) | 上流 Blend2D の clone・patch・CMake(51〜62 行)、engine(63〜82 行)、pack・R2 の載せ・root(83〜95 行) |

## 1 比べた案と決定

| 案 | 内容 | 結果 |
|---|---|---|
| A | cc クレートの build.rs が C を .o にし、cargo が link する | 却下(実測: cc は既定で -O2・-ffunction-sections・--target などを足す。最終リンクは cargo ではできず、emcc の Command になる。build.rs の cwd はパッケージ直下) |
| B | fe-build(bin)が emcc・em++・emar・wasm-opt・fe-lift・fe-wasmgen を Command で起動する。cwd はリポジトリ直下 | 採る |
| C | CMake + Ninja を本プロジェクトの build に使う | 却下(依頼者の発言) |

決定: B。根拠: 引数・cwd・入力ファイルが今の deploy と同じになり、出力の同一性を保てる。

## 2 対応表(要点。全 45 行は evidence/R6-36b.md §2)

| deploy.yml の行(旧) | 今の処理 | fe-build の行き先 |
|---|---|---|
| 52〜53 | CFLAGS・CXXFLAGS の env | 上流の CMake を外した後は使わない(0417 で em++ の 24 フラグに置き換え) |
| 55・56 | printf >> GITHUB_PATH・cargo build | printf は fe-build の PATH 設定に移した(engine が emcc の bin を先頭に置く)。cargo build は残す |
| 57〜62 | git clone・checkout・patch・emcmake cmake・cmake --build | clone・checkout・patch は blend2d_clone()。CMake の 2 行は 0417 で em++ 159 件と emar・emranlib に置き換え |
| 63〜82 | mkdir・xargs・cp・emcc・mv・sed | Mkdir・Run・CopyDlls・compile_unit・Glue。object は -o で置く |
| 83〜95 | pack-assets・aws s3 sync・sha256・grep・root の cp | fe-build bundle(pack)と upload(R2 の載せ)。aws・node は Command のまま |
| 43〜47(旧 extract) | mkdir・aws s3 cp・node | fe-build extract(R6-36d。deploy.yml の 1 行) |

## 3 出力の同一性

- 揃えた物: 入力(lists・args・rsp・cmd)は同じファイル。cwd はリポジトリ直下。argv は今の並び。link の object の順は h → mod → fle → rt(各群はバイト順)。Blend2D の一覧は ninja の順。
- 確認済み: engine.wasm の sha256 が verified.txt の b440e13a623a と一致(R6-36b §8-4。R6-36c §C で CMake を外した後も一致)。よって、-o・順・PATH の変更は最終の出力を変えなかった。
- 未確認: CI の LANG(C.UTF-8 と見込む。R6-31 §3 で同じ結果)。

## 4 検査(実測。出力は evidence/)

- bundle: seed 20780949 B(d6d345ed0b46)、dll 88675800 B(bce3049ad4d1)。br の 4 ファイルは前回と cmp 一致(R6-36b §8-5)。
- upload: 偽の aws で、呼び出しの順と分岐(verified・root.txt)をテスト(R6-36b §8-7)。
- CI の deploy の run 44048858711(7449d3e0): 全ステップ success。R2 への upload 6 件、失敗 0 件(R6-36b §8-10)。
- deploy の run: 9ad85333 の run 44063480368(R6-36c §E)と、dbe64977 の run 44069533342(R6-36b §9-5)。いずれも success。

## 5 残る問題

1. 上流 Blend2D の CMake: R6-36c(0417)で外した。fe-build に cmake の呼び出しは無い。
2. Python: deploy の経路の extract と seed は、R6-37(決定 0415)で Node(app/scripts/extract-fl.mjs・pack-seed.mjs)に移った。deploy.yml に python3 は無い。
3. 0400 の公開 action の制限(checkout・setup-node・wrangler-action のみ)は、HEAD で既に破られている(runs-on/cache・setup-emsdk)。本決定では変えない。
4. 0401 の fe-build(publish 1 つ)は履歴に無い。本決定の fe-build は名前を借りるだけで、中身は別。
5. extract(fe-build extract)は CI で未実行の経路。run 44063480368 と 44069533342 では cache hit で skipped。cache の鍵(ci/** と app/scripts の .mjs)が変わるか cache が無くなった run で初めて実行される。失敗時の切り分けは evidence/R6-36c.md §G。
