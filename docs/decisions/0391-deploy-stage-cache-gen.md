# 0391: 段 gen(生成 C)の出力を R2 の段のキャッシュに保存し、同じ鍵なら戻して gen を飛ばす(段のキャッシュの 1 段目)。鍵は旧 plan.sh の方法、入力は今の段の入力
分類: c 通信・変換の削減(AGENTS.md §6)。gen の出力を、同じ入力の 2 回目以降は作り直さない。stage_gen の「出力が在るなら飛ばす」分岐も c(出力が在る生成を行わない)。鍵・復元・保存の部品は整備(lift の処理ではない)。
撤回条件: (1) 戻した gen の出力から作った wasm の sha256 が、同じ入力を作り直したときの sha256 と違ったら、段の復元を外す(evidence/R6-10.md の 2 回の実行で確かめる)。(2) 段 gen の復元の時間が、作り直し(gen 22〜25 秒 + fe-pe・fe-lift の cargo)より長い実行が 2 回続いたら、この決定の効果を見直す。(3) gen が読むファイルで §2 の表に無い物が見つかったら、表に足す(本文は変えない)。(4) R2 の鍵を .github/actions/stage-cache の restore・save の step の外へ渡す変更は、差し戻す(決定 0386 の決定 4 と同じ)。
日付: 2026-11-16   ROADMAP: R6-10   一覧: evidence/R6-9.md の C1・C2(gen の 1 段だけ)   コミット: 本文書の commit(後続)

## 0 検索した範囲と件数(AGENTS.md「設計の前の確認」1〜3)
- 旧設計: `archive/ci-old/ci/plan.sh`(50 行。全行)、`needs.sh`(38 行。全行)、`objkeys.sh`(24 行。全行)、`actions/r2-cache/action.yml`(141 行。全行)、`test-needs.sh`(32 行。全行)、`archive/ci-old/deploy.yml` の 163〜169・304〜321・548〜550・790〜793・831〜834 行(sed で読んだ範囲)。
- 今の部品: 旧 deploy 系の workflow(deploy-engine。決定 0400 で deploy.yml に統合)(492 行。全行)、.github/actions/extract-layer/action.yml(削除済み)(90 行。全行)、ci/extract-cache.sh(削除済み)(180 行。全行)、`app/scripts/build-engine-bundle.sh`(452 行。全行)、`lift/harness/b2d2/gen-direct.sh`(49 行。全行)、`lift/harness/two/gen_two.sh`(全行)、`ci/lib/common.sh`・`ci/versions.env`・`ci/setup-toolchain.sh`。
- 依頼者の過去の発言: `~/work/handover/setup/S16-intent/catalog.md`(84 行)で「キャッシュ」「cache」「抽出」「段」「ステップ」を含む行は 0 件(`grep -c`)。`~/work/handover/evidence/01a-user-messages-full.md`(発言 263 件。`grep -c '^## seq'`)で「キャッシュ」「cache」を含む行は 6 行、発言は 3 件。その 3 件: msg_1a2ccc2c3001RSHoExRGznB5pH(seq 44954。1216〜1227 行。「キャッシュは前述のステップごとに行われ、変更がなければステップをスキップします」。C1・C2 の根拠)、msg_1a36588b5001JDXRUWPAcZ0qbI(seq 47194。1267 行。「なぜキャッシュされないんだ？」)、msg_1a3964d54001FVQ76lEhAA3IYQ(seq 48017。1343〜1344 行は R2 の鍵の名の一覧で、依頼の文ではない)。今回のセッションの発言は evidence/R6-9.md §0 の範囲のまま(再検索は未確認)。
- `docs/decisions/` で「キャッシュ」を含む決定: 0351・0386・0387 の 3 件(`grep -l`)。
- 段の秒(作り直しの時間): `evidence/R3-1b.md` の段ごとの表(gen 22、wasm 445、hybn0 164、weblink 2、wasmopt 138、hybrid 182)と、`evidence/R1-engine-bundle.md` の段 6 の内訳(gen 25、wasm 449、hybn0 208、weblink 14、wasmopt 335、hybrid 231)。resplit・seed・bundle は stage-command の行が無い。

## 1 決定
- 決定 1(保存する段): この決定で保存・復元する段は gen だけ。根拠: 依存が最も少ない。gen の入力は抽出の鍵と lift の入力だけ(§2)。weblink は wasm の .o 群・hybn0 の exports.rsp・link map を読む(`build-engine-bundle.sh` の stage_weblink 156〜167 行)。bundle は 9 段の全部の出力を読む(stage_bundle 411〜419 行)。時間の大半は wasm・hybn0・wasmopt・hybrid・bundle(上の段の秒)で、この決定では保存しない(R6-11〜R6-13 の行)。gen の作り直しは 22〜25 秒なので、効果は小さい。実測は evidence/R6-10.md。
- 決定 2(鍵): 鍵 = `fl-gen-` + SHA-256 の先頭 40 桁。SHA-256 の入力は、キャッシュの版(`stage-cache v1`)・段の名前・上流の鍵(抽出の層の鍵)・道具の版(rust 1.99.0・python 3.14)・gen が読む git の blob id の一覧(§2 の表)・段の文(`script_preamble`(40〜74 行の定数)と `script_part`(capped・stage_gen・tag_dir)のテキスト)。方法は旧 `archive/ci-old/ci/plan.sh` の k()(`fl-<段>-<hash>`)と同じ。実装は ci/stage-keys.sh(削除済み)。
- 決定 3(飛ばす分岐): `stage_gen` は、`$ONAT/gen/flengine.c` と `$ONAT/v-*/src/common.c` が在れば、cargo も gen-direct.sh も行わず「飛ばす」と表示する(`build-engine-bundle.sh` の 93 行。分類 c)。根拠: 戻した出力が在るのに fe-pe・fe-lift の cargo を走らせると、戻す利点が消える。後の段が fe-wasmgen を要るときは、`hyb_few`(194〜199 行)が従来どおり作る。gen の出力を読む後の段(resplit・wasm)は、`$ONAT` の中身だけを読む(本文の決定 4 で同じ中身を戻す)。
- 決定 4(戻す・保存する): R2 `fl-web-cache` の `stage-cache/<鍵>/` に 3 件(out.tar・out.tar.sha256・SHA256SUMS。SHA256SUMS を最後に書く)。復元は、tar の SHA-256・tar の中身(絶対パス・`..`・リンクの無し)・件数・各ファイルの SHA-256 を、保存時の記録と照らす。404 なら作り直す(hit=false)。在るのに戻せなければ、段を止めて再実行を求める(`::error::`)。根拠: 依頼の指示(hit のはずが復元に失敗したら止めて再実行を求める)と、旧 deploy.yml 548〜550 行の扱い。抽出の層(決定 0386)は失敗時に取り直すが、この段は、在るのに戻せない原因の多くが R2 の一時的な失敗で再実行により直るものとして、止めて再実行を求める。実装は ci/stage-cache.sh(削除済み) と .github/actions/stage-cache/action.yml(削除済み)。
- 決定 5(R2 の鍵): `R2_CACHE_ASSETS_*` は stage-cache の restore・save の step の env にだけ渡す。build の step には渡さない。根拠: 決定 0386 の決定 4 と同じ。
- 決定 6(範囲外): R6-11(道具・Blend2D・lifter)、R6-12(resplit 以降の段の保存)、R6-13(wasm の .o)、R6-14(app)は、この決定に含めない。根拠: ROADMAP の各行。

## 2 gen の鍵の入力の表(決定 2 の入力。「段が読む根拠」は、そのファイルの行)
| 入力 | 段が読む根拠(ファイル:行) | 鍵での扱い |
|---|---|---|
| 抽出の層の鍵(インストーラと抽出の入力) | `build-engine-bundle.sh` 58・61〜62 行(HOST の FLEngine DLL と SHA-256)、`gen-direct.sh` 14〜33 行(HL/out-* の入力)、`ci/extract-dlls.tsv`(out-* の配置) | 上流の鍵(`ci/extract-cache.sh key`) |
| `lift/harness`(gen-direct.sh・two/gen_two.sh・hooks・seeds・表) | `build-engine-bundle.sh` 98 行(FLE_HOOKS・FLE_SEEDS)、`gen-direct.sh` 14〜33 行、`gen_two.sh` 45・130・135・150 行 | `git ls-files -s`。web/ と hyb/ も含む(過剰。鍵が変わるだけで、古い出力を戻すことは無い) |
| `lift/crates`(fe-pe・fe-lift・realmod_s1/tools) | `build-engine-bundle.sh` 95 行(`cargo build --release -p fe-pe -p fe-lift`)、`gen_two.sh` 45 行(TOOLS) | `git ls-files -s` |
| `lift/Cargo.toml`・`lift/Cargo.lock`・`lift/.cargo` | `build-engine-bundle.sh` 95 行(`cargo --offline`) | `git ls-files -s` |
| `lift/data`(prune-SM.tsv) | `build-engine-bundle.sh` 45 行(PRUNE_LIST) | `git ls-files -s` |
| rust 1.99.0・python 3.14 | `ci/versions.env` 6・7 行、`build-engine-bundle.sh` 95 行、`gen_two.sh` 130・135・150 行(python3) | 道具の版(鍵の文に入る) |
| スクリプトの定数 | `build-engine-bundle.sh` 1〜74 行(preamble。例: FRUITY_SECONDS 40 行、EXTRA_IMPORTS 44 行) | `script_preamble` のテキスト |
| stage_gen・capped・tag_dir | `build-engine-bundle.sh` 75〜104 行 | `script_part` のテキスト |
| 外すもの: `lift/runtime` | `lift/crates/fe-lift/src/insn_int.rs` の `runtime` の読み取り(395 行)は `mod tests`(198 行〜)の中。fe-wasmgen の `flagout.rs` 287 行は hybrid の段。fe-lift の本体は読む物を引数で受ける(`main.rs`)。`lift/crates` の他の `runtime/` は C の #include の文字列(lib.rs 1236 行)か、コメント | 鍵の入力から外す。ci/test-stage-keys.sh(削除済み) が、runtime の変更で鍵が変わらないことを確かめる |
| 外すもの: emcc・emsdk | `gen_two.sh`・`gen-direct.sh` は emcc を呼ばない(`grep -c emcc` が 0) | 外す |

## 3 実測と未確認
- 実測の表と run の ID は `evidence/R6-10.md`(この決定を書いた時点では未実行)。
- 未確認: gen の出力の大きさ(初回の実行で測る)。R2 の転送の時間。
- 公開ホストは未確認(この決定の範囲では確かめない)。

## 追記(2026-11-16)
段 gen のキャッシュは、決定 0395 で撤回した(0395 §3 決定 1。根拠は evidence/R6-10.md §3.3 と main の決定)。本文は変えない。stage-cache の action と部品は残し、段 gen の使用だけを外した。
