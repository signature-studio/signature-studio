# 0395: 段のキャッシュを内容の鍵にし、wasm を 7 群の object で持つ。bundle は入力の全体の鍵で 9 段を飛ばす。段 gen のキャッシュ(0391)は撤回する
分類: c 通信・変換の削減(AGENTS.md §6)。同じ入力の段の出力を R2 から戻し、作り直しと載せ直しを省く。段 gen と weblink は毎回作る(§3 決定 5)。
撤回条件: (1) 段または群の復元の秒(同じ run の restore step)が、同じ段の作り直しの秒(同じ run の stages.log)の 1/3 を超える実行が 2 回続いたら、その段・群を外す。(2) 復元した出力から作った wasm の sha256 が、作り直したときの sha256 と違ったら、その段の復元を外す。(3) 出力を変える入力が鍵に無いと分かったら、表(§2)に足す。(4) R2 の鍵を復元・保存の step の外へ渡す変更は差し戻す(0386 決定 4)。
日付: 2026-11-16   ROADMAP: R6-12(R6-13 の群の部分を含む)   一覧: evidence/R6-9.md の C8・C9・C10・C11・C12   コミット: この文書・ci/・.github/・app/scripts/ の commit(後続)

## 0 検索した範囲と件数(設計の前の確認)
- 依頼者の過去の発言: `~/work/handover/setup/S16-intent/catalog.md`(84 行)で「キャッシュ」「cache」は 0 件(grep -c)。`~/work/handover/evidence/01a-user-messages-full.md`(263 件)で同じ語を含む発言は 3 件: msg_1a2ccc2c3001RSHoExRGznB5pH(2026-11-04 04:04:51。「キャッシュは前述のステップごとに行われ、変更がなければステップをスキップします」「Dockerfile と Docker Build のように」)、msg_1a36588b5001JDXRUWPAcZ0qbI(2026-11-04 06:51:41。「wasm が毎回ダウンロードされている… なぜキャッシュされないんだ？」)、もう 1 件は R2 の鍵の名の一覧で依頼の文ではない(0391 の §0)。2026-11-16 の依頼者の発言(「なぜその重要な部分を消し、エージェントにわざわざ不便な環境を強いているのですか?」。main の依頼文に引用。発言 ID は未確認: worker は opencode api の権限が無い)。
- 旧設計: `archive/ci-old/ci/objkeys.sh`(24 行。全行)、`plan.sh`(50 行。全行)、`needs.sh`(38 行。全行)、`archive/ci-old/deploy.yml` の 163〜200・575〜600 行(sed)。旧の 7 群は objkeys.sh の `grp` の 7 行(fle・m1〜m5・rt)。
- 今の部品: 旧 deploy 系の workflow(deploy-engine。決定 0400 で deploy.yml に統合)(旧 533 行。全行)、`77ab7773^:app/scripts/build-engine-bundle.sh`(453 行。全行)、`77ab7773^:lift/harness/two/build_nmod_wasm.sh`(81 行。全行)、`lift/harness/web/fruity/link-web.sh`(全行)、ci/stage-keys.sh(削除済み)・ci/stage-cache.sh(削除済み)・.github/actions/stage-cache/action.yml(削除済み)(0391 の版)。
- 決定文書: `docs/decisions/` で「キャッシュ」を含むもの 0351・0386・0387・0391。
- 実測の入力: `~/.cache/signature-studio/work-r53k`(R5-3 のワークツリーの WORK。実在する全段の出力)の `du -cb`(大きさ)と `w-Oz/obj/log/*.log.time`(群ごとの秒の和)。

## 1 段と群の表(測定の前に書いた見込み。出典は evidence/R6-12.md §1)
| 段・群 | 出力(WORK の下) | 大きさ(B) | 復元(÷32 MB/s) | 作り直し(秒の実測) | 採る |
|---|---|---|---|---|---|
| gen | onat | 1,083,999,330 | 34 s | 22〜30 | 採らない(撤回) |
| resplit | wsrc | 509,237,848 | 16 s | 4 | 採らない |
| wasm 群 fle | w-Oz/obj/fle | 90,378,926 | 2.8 s | 388(23 unit の和) | 採る |
| wasm 群 m1 | obj/mod/m1-*.o | 55,057,671 | 1.7 s | 235 | 採る |
| wasm 群 m2・m3・m4 | obj/mod/m2〜m4-*.o | 1.6 M・8.5 M・4.3 M | 0.05・0.27・0.13 s | 7・42・23 | 採る |
| wasm 群 m5(モジュール 5〜9) | obj/mod/m[5-9]-*.o | 39,045,754(m5〜m7) | 1.2 s | 154(m5〜m7) | 採る |
| wasm 群 rt | obj/rt・h-fruity・mod/common.o・b2d | 20,157,066 | 0.6 s | 14(rt 11・h 1・common 1・b2d の adapter は含まない) | 採る |
| hybn0 | hyb/n0-*・exports.rsp・c-imports | 20,649,408 | 0.6 s | 164(R3-1b)・208(R1) | 採る |
| weblink | web/flengine6.wasm・js・hyb/link.map | 193,933,899 | 6.1 s | 2(R3-1b)・14(R1) | 採らない(毎回作る) |
| wasmopt | web/flengine6.c.opt.wasm | 131,176,555 | 4.1 s | 138(R3-1b)・335(R1)・336〜521(R6-10) | 採る |
| hybrid | web/flengine6.opt.wasm・js | 143,863,306 | 4.5 s | 182(R3-1b)・231(R1) | 採る |
| seed | in/seedpack.bin・dll | 86,992,045 | 2.7 s | 0(R3-1b。同秒) | 採らない |
| bundle | bundle/fl-engine-ci-<8> | 267,822,470 | 8.4 s | 284(R1・R3-1b) | 採る(入力の全体の鍵) |

- 群の秒は `.time` の和で、unit は並列に走る(段の壁時計 433 s < 和 848 s)。群の割合を見るための数で、段の壁時計の代わりにしない。
- 段の割合(R3-1b §4 の stage-command 和 1,241 s): wasm 445(36%)、hybn0 164(13%)、hybrid 182(15%)、wasmopt 138(11%)、bundle 284(23%)、gen 22(2%)。採る 5 段で 98%。wasm は最大の 1 段だが過半ではない。順は変えない(理由: 最大の段を群に分けて先に作る。残りは同じ形)。
- R6-10 の stages.log の全行は evidence に残っていない。R6-10 の値は wasm 466・759、wasmopt 336・521、hyb-n0-dsp 96・124、hyb-gen-dsp 97・125、gen 30(evidence/R6-10.md §3)。他の段は R3-1b・R1 の同じ段の値(別の run)。

## 2 鍵の入力(根拠の行は build-engine-bundle.sh の現在の行)
| 段 | 鍵に入る内容 | 段が読む根拠 |
|---|---|---|
| wasm 群 fle | wsrc/src/fle-NN.c のバイト | build_nmod_wasm.sh の units(OUT=$WSRC、`src/fle-[0-9][0-9].c`)、build-engine-bundle.sh 128〜129 行 |
| wasm 群 m1〜m5 | wsrc/v-TAG/src/mK-NN.c のバイト | build_nmod_wasm.sh の units(`m[0-9]*-[0-9][0-9].c`) |
| wasm 群 rt | common.c・realmod_table_gen.c のバイト | build_nmod_wasm.sh(`$V/src/common.c`・REALMOD_C)、build-engine-bundle.sh 129 行 |
| wasm 全群 | git: lift/runtime・lift/harness | build_nmod_wasm.sh の CFLAGS の -I$RT・-I$LIFT/harness、b2d_swap.cpp(b2d_adapter 138〜143 行) |
| wasm 全群 | 段の文: preamble・capped・tag_dir・wasm_env・b2d_adapter・stage_wasm・hyb_strip・hyb_lists | 1〜74・124〜154・310〜327・183〜192 行 |
| hybn0 | ONAT/v-TAG/dll/*.dll | hyb_modargs 202 行(`$(tag_dir)/dll`) |
| hybn0 | git: lift/harness・lift/crates・lift/Cargo.*・lift/.cargo | hyb_few 195〜197 行(fe-wasmgen のビルド)、339〜340 行(lists・chk_lowered) |
| wasmopt | WEB/flengine6.wasm のバイト・wasm-opt の版 | 169・172・174 行 |
| hybrid | c.opt.wasm・hyb/link.map・link.map.sha256・glue(orig、無ければ js)・dll・wasm-merge と wasm-opt の版 | 347・348・354〜357・366・370・387〜388・351・382〜383 行 |
| bundle | 入力の全体のハッシュ(deploy-engine.yml の 64〜68 行の一覧。app/scripts/build-engine-bundle.sh・pack-assets.mjs・lift・ci の全部) | 412〜445 行の stage_bundle は一覧に含まれる |
- 鍵の形式: 段 `fl-<段>-<40 hex>`、群 `fl-wasm-<40 hex>`(群ごとに別の値)。鍵の版 `stage-cache v2`。道具の版は ci/versions.env(emsdk・rust・python・node)。
- 入力は段ごとに前の段の出力が在るときに読む(ci/stage-keys.sh の注記)。鍵の計算は段の順に行う(§3 決定 4)。

## 3 決定
1. 段 gen のキャッシュ(0391)は撤回する。根拠: evidence/R6-10.md §3.3(復元 34 秒、作り直し 30 秒、保存 129 秒。1.08 GB)と main の決定(2026-11-16)。stage-cache の action と部品は残し、段 gen の使用だけを外す。0391 には追記だけ。
2. wasm は 7 群の object で持つ(fle・m1・m2・m3・m4・m5=m5〜m9・rt)。根拠: 旧 `archive/ci-old/ci/objkeys.sh` の 7 群(main の指示)。build_nmod_wasm.sh は既存の object を作らない(compile_one の `[ -e ]`)ので、復元した群は作り直されない。1 件の unit が変わると、その群の鍵だけが変わる(ci/test-stage-keys.sh の 5 件で確かめる)。
3. 鍵は内容の鍵: 前の段の出力(群なら単位のバイト)の SHA-256 + git の blob id + 段の文 + 道具の版。同じバイトなら同じ鍵。1 バイト違えば違う鍵(ci/test-stage-keys.sh)。
4. 順序: 鍵は前の段の出力が在るときに計算する。gen・resplit → wasm の 7 群(鍵・復元・作る・保存)→ hybn0 → weblink → wasmopt → hybrid → seed・bundle。bundle の鍵は入力の全体のハッシュ(prefix の全体)で、最初に復元する。
5. gen と weblink と seed は毎回作る。根拠: §1 の基準 1/3 を超える(weblink は 6 s の復元に対し作り直し 2〜14 s、seed は作り直しが 0 s)。
6. bundle は入力の全体の鍵で、hit なら 9 段と R2 への載せの前の段(emsdk・rust・cargo・上流 Blend2D)を飛ばす。bundle の manifest の description は build の HEAD を含む(build-engine-bundle.sh の 436 行)。復元した bundle の description は、その prefix を最初に作った commit の HEAD を指す。R2 への載せは既存の挙動と同じ(同じ prefix の manifest があれば載せ直さない)。未確認: 依頼者の画面での表示。
7. 復元した内容は保存時の SHA-256 と照合する。鍵が在るのに戻せなければ止めて再実行を求める(0391 の決定 4 と同じ)。
8. R2 の鍵は、復元と保存の step の env にだけ渡す(0386 決定 4)。ビルドの段には渡さない。
9. 段の飛ばし分岐は 2 つだけ足す: hybn0(`exports.rsp` が在れば飛ばす)と bundle(`upload-commands.txt` が在れば飛ばす)。wasm は既存の per-object の飛ばし(build_nmod_wasm.sh)を使う。新しい環境変数・workflow_dispatch の入力・#ifdef は足さない。

## 4 実行の結果
§4 は run の後に書く(evidence/R6-12.md §3)。

## 未確認
- 上流 Blend2D の出力のバイトは鍵に入らない(入力: lift/harness/b2d1 と emsdk の版)。
- 公開ホストは未確認(この作業では確かめない)。
- 群の objects が gen の再実行で同じバイトになるかは、run の C(wasm の鍵が 1 回目と同じか)で確かめる。
