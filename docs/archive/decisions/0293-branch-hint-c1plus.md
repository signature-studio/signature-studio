# 0293 branch hint(C1+): MXCSR 窓検査の if と、それを包む RC の if に `metadata.code.branch_hint` を付ける(fe-wasmgen。mt3w。仮)

状態: **仮**。作業者 mt3w、2026-11-06 JST。基準: PM の `e11-wip`(799b0417 = E10 + 0285 + 0285b)。差分: `impl/0293-branch-hint.diff`(`patch --batch -p1` が **e11-wip に offset 0・fuzz なし**、**main 9fdc492f(E10)に lower.rs が offset −39 行〔18 hunk〕・fuzz/reject なし**で当たる。実測、§4)。区分: **実測**=出力を見た値/**読み**=コード・資料を読んだ推論/**未確認**。

## 0. 冒頭の三点(正直さ)

1. **実エンジンが実行されたか**: 実行した。fe-wasmgen(hint 入りと、e11-wip のままの 2 本)を、実エンジンの 8 module(FLEngine と 7 DLL。E10 の CI 作業ディレクトリ `lift/out-opt2/x16` の DLL と、E11 の hybn0 の引数そのまま〔`--hybrid-dispatch` の仮アドレス含む〕)に走らせて n0 wasm を作って比べ、さらに CI と同じ `wasm-opt -Oz`(binaryen 133)→ merge1 → merge2(E10 の C 部)を通した(§5)。さらに **E10 の CI 作業ディレクトリ x16 の実アドレスで hybrid 段階を作り直し、hint を除いた成果物が x16 の hint 無しの成果物と段階ごとに sha256 一致〔18/18〕**を確かめた(§4 の 3b)。代役・再生・推測の画面や音は使っていない。
2. **実行していない物**: ブラウザ/Chrome での速さ(alpha1 の領分。alpha1 の node の測定は −14.0%〔窓検査の if だけ〕、−38.6%〔窓検査の if + 外側の if〕と PM から受け取った。私は再測定していない)、実エンジンの冷間 CI、node の実エンジン走行(progress/slices/snap の行)。V8(node 24.19 = V8 13.6)では『受理する』『テストが同じ結果になる』だけを見た(速さは見ていない)。
3. **未確認**: Chrome 156 の hint の扱いの細部(PM が alpha1 の測定として受け取った『既定で読む』以外)。wasm-opt が 530 件(28,100 → 27,570)を落とした理由(§5)。

## 1. 変更(ファイル別。`-` の行は §9 に全部)

| ファイル | 変更 |
|---|---|
| `lift/crates/fe-wasmgen/src/wasm.rs`(+56 −8) | `Code.hints: Vec<(u32, u8)>`(`if` の opcode の `b` 内 offset と、1=then 腕が likely/0=else 腕が likely)。`Code::if_hint(then_likely)` と `if_ty_hint(ty, then_likely)`(hint を記録して普通の `if` を書く。`b` の中身は従来と同一)。`Module::locals_header`(関数 body 先頭の locals ベクタ。code section と hint の offset の計算が同じ関数を使う)。`Module::encode` が、hint を持つ関数ごとに最終 offset(= locals ベクタの長さ + `b` 内 offset)と関数番号(imports を数える)を書き、`metadata.code.branch_hint` custom section を **code section の直前**に出す。hint が 1 つも無い module には section を書かない。テスト用 switch `wasm::HINTS_OFF`(thread-local、既定 false。`lower::MX_FORCE_SLOW`・`locshare::LOCSHARE_OFF` と同じ型のテスト用。CLI・環境変数からは届かない) |
| `lift/crates/fe-wasmgen/src/lower.rs`(+21 −19) | 18 箇所の `if_()` / `if_ty()` を `if_hint(…)` / `if_ty_hint(…, true)` に(§3)。`mx_force` の doc に向きの規則を 3 行 |
| `lift/crates/fe-wasmgen/tests/branch_hint.rs`(新、+599) | §4 |
| `lift/crates/fe-wasmgen/tests/{c_only_forms,mx_hybrid,mx_rc_wire,opaque_stub,tls_ctx,xabi_passthrough}.rs`(各 +4 −1) | `wasm-validate --enable-all` → `wasm-validate $WABT_FEATURES`(§7: wabt 1.0.36 が正しい hint section を誤って拒否する) |

main.rs・lib.rs は変えていない。旗・環境変数・オプション・class/アドレスによる分岐は無い。常に出す(hint を読まない engine は無視する)。

## 2. 契約(hint の意味と section の形)

- 意味(**読み**: branch-hinting 提案):hint は助言で、命令の意味(どちらの腕が実行されるか)を変えない。`if` の hint 1 = 条件が真(then 腕)が起きやすい、0 = 偽(else 腕)が起きやすい。
- section: 名前 `metadata.code.branch_hint`、custom section(id 0)、**code section の直前**(実測: wat2wasm が出す section の位置と、wasm-opt・wasm-merge・V8 の扱いで確認〔info 1〕)。中身: `uleb(関数の数)`、各関数に `uleb(関数番号)`(imports を数えた番号)、`uleb(件数)`、各 hint に `uleb(offset)`、`uleb(1)`(データ長)、`1 byte`(0/1)。関数番号と offset は厳密に増える。
- **offset は関数 body の先頭(body size の LEB の次の 1 byte 目 = locals ベクタ)から数える**。`Module::encode` で `locals_header(&locals).len() + b 内 offset`。これを忘れると hint は別の命令を指す。V8 13.6 は壊れた offset を黙って受理する(info 1 で実測)ので、正しさは我々の側のテストで保証する(§4)。

## 3. 付ける所(emitter 18 箇所)と向き

付ける規則: **生成側が『どちらの腕が cold か』を知っている if だけ**。cold = MXCSR 窓検査が外れたときの C 経路の関数(`c.fle_hyb_mx`)を呼ぶ側。窓検査が成り立つ側(素の SIMD 演算)と、窓検査を含む側が likely。

| 種類 | 行(lower.rs。差分を当てた後の行番号) | 形 | hint |
|---|---|---|---|
| 窓検査の if(16 箇所) | 3349・3374・3415 `mx_form`、3562 `mx_round`、3604 `mx_cvttps2dq`、3633 `mx_cvtps2dq`、3735・3763・3785 `mx_packed`、3868 `mx_cmp`、4001 `mx_cvtt`(`if_ty`)、4060 `mx_cvtdq2ps`、4095 `mx_cvtsi2sd`、4141 `mx_cvtsi2ss`、4203 `mx_packed2` | `mx_force()` の直後。then = 素の演算、else = C 経路の関数 | 1 |
| 同(ucomis) | 1642 `lower_insn` | `i32.eqz` の後。then = C 経路の呼び(演算が普通でないとき)、else なし | **0** |
| 外側の if(2 箇所) | 3340 `mx_form`(`rc_wrap`: スカラー add/sub/mul/div)、3781 `mx_packed`(`rcl == 0`) | 窓検査が then 腕、RC の helper(`mx_rc_scalar`・`rcgen`)が else 腕 | 1 |

付けていない所(意図して):RC の helper の内側の if(`if ok {helper の値} else {mx_slow}`、`rcl-1 < 3`):窓検査が外れて RC ≠ 0 の経路の中で、どちらが多いかを生成側は知らない。typed_indirect(E11、0285)の `if done else c_stub`:site ごとに target が適合するか決まる(φ は動的な割合で 68.9%、適合しない site は常に else)ので、静的に『hit が likely』と言えない。alpha1 の C2(call の無い腕を cold と決め打ち、29,474 箇所)が逆向きの hint で悪化した(PM 受領の測定)のと同じ理由で、**この種は付けない**。

向きの機械検査は §4 の `verify`(全件)。

**alpha1 の件数(5,380)との関係**(alpha1 本人の返事、PM の指示で直接聞いた): alpha1 の 5,380 = 窓検査の if 2,716 + 外側の if 2,664 は、**E9 のテストビルド(`tw-e15`)の module 全体に、生成側ではなく Python で後付けで注入した数**(`a17-hint-patch3.py`: 26 命令の列の直後の if を『窓検査の if』とし、1 演算 1 段。64 関数。f_b34aca0 は 646 件)。私の 28,100 は生成側が付ける全 mx 形(加減乗除の群は外側 1 + quick 1 + 精密 1 の 3 段)の 8 module 全体(E10/E11 の形)で、範囲も数え方も違う(同じ 2 関数 f_b34aca0・f_b34eaf0 は私の側で各 1,392 件〔全部 1〕= 464 群 × 3)。したがって **28,100 件版の速さは alpha1 の node の 2 関数〔E9 のテストビルド〕では測れない**。Chrome の A/B のために、同じ手順の完全な bundle 2 つ(hint 入りと plain。§4 の 3b、`tables/bundle93.txt`)を alpha1 に渡した。

**向きの根拠(fle を含む)**: 根拠は『窓検査が外れる = `c.fle_hyb_mx` を呼ぶ = cold』が全 28,100 件で成り立つ、という**機械検査だけ**(`verify`: likely 腕に cold の呼びが無く、cold 腕に呼びが有る)。実際に窓検査がどれだけの割合で通るかは、module ごとに**測っていない**(UI のコード〔fle の 13,075 件、全体の 47%〕で窓検査が通る頻度は、仕込みの走行では測れない。alpha1 の node の測定は elastique の 2 関数だけ)。したがって『hint が速くなる』は elastique の 2 関数でだけ測られていて、fle・oggio・qfc・dsp・b2d・ilt の分は測られていない。窓検査が普段ほとんど外れる関数があれば、その関数では hint は逆効果になりうる(§6)。

## 4. 検証(PM が再現できる形)

1. **cargo test(fe-wasmgen 全体、`--no-fail-fast`)**: 33 results・**76 passed・0 failed**・2 ignored(`tables/cargo-test-fe-wasmgen.log`。e11-wip の分が 71 passed、新しい branch_hint が 5 passed + 1 ignored)。なお e11-wip の `typed_indirect.rs` は 4 テストで、0285-impl.diff の `two_targets_in_one_slot_are_both_found_through_the_bucket` は e11-wip に入っていない(私の側の記録との差。理由は未確認)。
2. **`tests/branch_hint.rs` の 5 テスト + ignored 1**:
   - `every_window_check_has_a_hint_that_points_at_its_if`: 6 形(cvtsi2ss・addsd・cvttsd2si・ucomisd・addps・sqrtsd)の各関数で、hint の値の列(例: addsd は外側・quick・精密の `[1,1,1]`)、種類(leaf・chain・outer の数)、**全 hint の offset が `if`(0x04)の opcode**、section が code section の直前、関数番号・offset が厳密増加、section が末尾まで消費、を確かめる。`verify` で向きも確かめる。
   - `offsets_count_the_locals_vector_of_a_larger_function`: 6 形を 1 関数に並べた関数(locals の run が 2 つ以上)で offset が locals ベクタを数えていること。
   - `the_module_without_the_section_is_the_module_without_hints`: **`HINTS_OFF` で作った module(hint 無し)と、hint 入りの module から section を取り除いた物が 1 byte も違わない**。
   - `the_direction_check_fails_for_a_turned_hint`: 全 hint を 1 つずつ反転した module が必ず `verify` で違反になる(mutation。検査が働くことの確認)。
   - `node_accepts_the_hinted_module`: node(V8)が `--experimental-wasm-branch-hinting` の有無どちらでも validate 通過・customSections 1 個。
   - `real_modules`(`#[ignore]`、`FE_HINT_MODULES=a.wasm,b.wasm` で実 module の hint を検査):`cargo test --test branch_hint real_modules -- --ignored --nocapture`。
   - **`verify` の規則**(wasm の命令列を読む。block/loop/if/else/end の入れ子と memarg〔multi-memory の memory index つき〕・SIMD の immediate を読み、`c.fle_hyb_mx` の呼びを数える): 各 hint 付き if で、(a) likely 腕に、中の hint 付き if の cold 腕に無い呼びが無い、(b) cold 腕に呼びが 1 つ以上ある、(c) 中の hint 付き if が両方の腕に無い。分類: leaf=中に hint 付き if が無い、chain=cold 腕の中だけに有る(quick test)、outer=likely 腕の中に有る(RC の外側)。
3. **実エンジンの 8 module での確認(tables/)**: `modules-c1plus.tsv`(hint 数・向き・種類・section の大きさ・増分・**hint section を除いた sha256 が e11-wip の fe-wasmgen の出力の sha256 と一致〔8/8〕**)、`direction-check-n0-modules.txt`(8 module で違反 0)、`survival-wasm-opt-merge.tsv`・`direction-check-after-oz-and-merge.txt`(§5)。
3b. **実アドレスの hybrid 段階(main 9fdc492f = E10 + 0293 の fe-wasmgen)**: E10 の CI 作業ディレクトリ `lift/out-opt2/x16`(hint 無しの E10 の成果物が全段階で残っている)の link map から実アドレス(fle_teb 0x109202a0、fle_mxcsr 0x10209958、fle_hyb_buf 0x109222b0、fle_progress_count 0x1030cc48、TLS 0x40)を `stage_hybrid` と同じ正規表現で読み、同じ DLL・seed・list で 8 module を作り直し、`wasm-opt -Oz`、merge1、merge2(x16 の C 部)を通した(`tools/e10real93.sh`)。**各段階の成果物から hint section を取り除いた物の sha256 が、x16 の同じ段階の成果物の sha256 と全て一致**: fe-wasmgen の出力 8 個、`wasm-opt -Oz` 後の 8 個、direct.wasm、**flengine6.opt.wasm(`4d0019aaa3ee…`、142,776,280 B)**(`tables/e10-real-hybrid-stage-hint-vs-x16.txt`)。つまり CI の経路で、hint 入りの成果物と hint 無しの成果物は section の分だけ違う。(この確認の途中で、x16 の `logs/hyb-addresses.txt` が別の link のアドレスを持っていて、最初に作った物は実アドレスが違っていた。link map から読み直して作り直した物だけを使っている。)
4. **cargo test を『V8 が hint を読む』node で**: 上と同じ cargo test を、node に `--experimental-wasm-branch-hinting` を付ける wrapper(`tools/nodewrap/node`)を PATH の先頭にして走らせた(wrapper の起動記録で 661 回の node 起動)。33 results・**76 passed・0 failed**・2 ignored(`tables/cargo-test-fe-wasmgen-v8-hinting.log`)。host CPU との突き合わせを含む mx_hybrid・mx_rc_wire・sse_forms ほかが、V8 が hint を読む状態で同じ結果。
5. **ci/test-hybrid.sh**: 42 ok、all checks hold(`tables/test-hybrid.log`)。
6. **`harness/test.sh full`**: **results=219・passed=2413・failed=0**・ignored=2(`FLE_TEST_NOCACHE=1 harness/test.sh full --jobs 2`、414 s、`tables/testfull-0293.log`)。基準 e11-wip の 218/2,408/0 に対し、+1 result(branch_hint)・+5 passed(同)。入力は `FLE_INPUT_ROOT` で main の checkout から `test-env.sh --link` で繋ぎ(0 件欠け)、`tools/authenticode_check.py` を木に入れた。**1 回目と 2 回目は木の不備で失敗した**(1 回目: 入力が繋がっておらず fe-lift の 10 テストが失敗、15 件 SKIPPED;2 回目: `tools/` を木に入れ忘れて `authenticode_check` の 14 テストが失敗〔単独では 14 passed〕。2 回目の記録は `tables/testfull-failed-run-tools-missing-14-authenticode.log`)。
7. **a4**: **942/942、runnable_started=895 completed=895 in_progress=0、functions_distinct=5310、function_entries=1009737、started_but_not_entered=0**(期待値と一致、`tables/a4-0293.log`。a4 は native の C 経路で fe-wasmgen を使わない。runtime・fe-lift は変更なし)。`EXPECT_FLE_SHA=skip` で走らせた(a4-multi の既定の期待 sha はこの構成では基準でも合わない。0284c の記載どおり)。
8. **差分の当たり方(実測、`tables/patch-dry-run-e11.log`・`patch-dry-run-e10.log`)**: e11-wip 799b0417: 9 file とも clean(offset 0)。main 9fdc492f: lower.rs の 18 hunk が `offset −39 lines` で当たる(0285 が lower.rs に足した 39 行の分)。fuzz・reject なし。wasm.rs・テストは e10/e11 で同一。

## 5. 保存確認(CI の手順を通して残るか。C1+ で再測定)

実エンジンの n0 wasm 8 個(`tables/survival-wasm-opt-merge.tsv`):

| 段階 | hint 数 | 備考 |
|---|---|---|
| fe-wasmgen の出力(n0) | **28,100**(1: 27,148、0: 952) | 8 module 合計。hint 付きの関数 657 |
| `wasm-opt -Oz`(binaryen 133、CI と同じ引数)後の 8 個 | **27,570**(98.1%。1: 23,389、0: 4,181) | fle 12,934 / elastique 8,485 / oggio 2,022 / qfc 1,890 / dsp 1,482 / b2d 430 / ilt 327 |
| merge1(8 個 → direct.wasm) | 27,570 | 関数 646 |
| merge2(E10 の C 部へ → merged.wasm) | 27,570 | section 122,609 B。merged.wasm = 142,811,908 B(E11 の n0 + E10 の C 部) |

- 全段階で: 関数番号・offset は厳密増加、section は末尾まで消費、全件が `if` の opcode を指す(br_if は 0、それ以外は 0)、section は code section の直前。
- **向きの機械検査**(`verify`)は n0・wasm-opt -Oz 後の 8 個・direct.wasm・**merged.wasm の全 27,570 件で違反 0**。wasm-opt は条件を反転するとき hint を反転して書き直す(elastique の 1 が 8,653 → 5,685、0 が 142 → 2,800。dsp も 1,462 → 978 / 83 → 504)が、その結果も向きが合っている。
- 落ちた 530 件(0.9% の fle 141、dsp 63、elastique 310 ほか)の理由は**未確認**。wasm-opt が if を畳む/関数を消す/統合する、と推定しているだけ(hint を持つ関数は 657 → 646 に減った)。
- 実アドレスの hybrid 段階(E10 + 0293、§4 の 3b)でも同じ: wasm-opt 後 27,570、merge1・merge2 後 27,570、hint section 122,876 B(結合後 module 142,899,156 B)、全件が `if` を指す。表は E11 の n0〔仮アドレス〕で作った物と同じ数。
- V8 13.6.233.17(node 24.19): merged.wasm(142,811,908 B)を `--experimental-wasm-branch-hinting` の有無どちらでも `WebAssembly.validate` が true、customSections 1 個(122,579 B)。
- (補足、info 1 の訂正)前回の『35,226 件 → 34,386 件』は fe-wasmgen を使わない prototype(`c.*` 呼びを含む if に後付け)での値で、その中の『34,6xx』は根拠の無い数だった。この節の値が C1+ の実測。

## 6. 誤った hint は速度にしか効かない(PM 依頼 (2))

- **読み**: branch-hinting 提案では hint は助言で、意味を変えない。V8 13.6 は offset・関数番号が壊れた hint も黙って受理して正しく動いた(info 1 の実測)。
- **実測**: **全 hint を反転した fe-wasmgen**(`wasm.rs` の `hint()` を `!then_likely` に変えた使い捨ての木。納品しない)で作った module に対し、V8 が hint を読む node(上の wrapper)で fe-wasmgen の cargo test 全体を走らせた: 33 results・**73 passed・3 failed**・2 ignored(`tables/cargo-test-fe-wasmgen-all-hints-turned-v8-hinting.log`)。失敗した 3 つは branch_hint.rs の『向きと値を確かめるテスト』そのもの(`every_window_check_has_a_hint_that_points_at_its_if`・`offsets_count_the_locals_vector_of_a_larger_function`〔`verify` が反転を見つけた〕、`the_direction_check_fails_for_a_turned_hint`〔反転を更に反転すると正しい向きに戻るので『気づかれない』になる〕)。**host CPU との突き合わせを含む mx_hybrid・mx_rc_wire・sse_forms・c_only_forms ほか、他の 73 テストは、向きが全部逆の hint の module でも合格**。つまり hint の向きは結果の値を変えない(変わるのは V8 の機械語の配置だけで、遅くなりうる)。
- したがって hint を付ける誤りの影響は『遅くなる(cold 腕を hot と思う)』に限られる。向きの正しさは `verify`(全 27,570 件、違反 0)で担保した。

## 7. wabt の警告と CI(PM 依頼 (1))

- wabt 1.0.36 の `wasm-objdump`・`wasm-validate --enable-all` は、wasm-opt/wasm-merge/fe-wasmgen の hint section(正しい)に『invalid function index N, only M bytes left in section』を出し、`wasm-validate --enable-all` は **失敗(rc 1)**する。wabt の最新 main の `ReadCodeMetadataSection` は関数番号を `ReadIndex` で読んでおり、1.0.36 の `ReadCount`(値 ≤ section の残りバイト数、を要求)が誤り(**読み**: 最新の main のソースを読んだ。1.0.36 のソースは未確認だが、エラー文は `ReadCount` の形と一致)。V8 は受理し、自作の検査〔`verify`・Python の構造検査〕は正しいと判定する。
- **CI は wabt を使わない**(実測): `.github`・`ci/`・`app/scripts` を wabt・wasm-validate・wasm-objdump・wat2wasm・wasm2wat で grep → 0 件。CI のテストは `ci/test-hybrid.sh`(42 ok)だけで、cargo test は走らない。wabt を呼ぶのは手元の cargo test(入っていれば。`have("wasm-validate")`)と開発用の `lift/harness/draw1/extract-fn.sh` だけ。
- 手元の cargo test のうち `--enable-all` を使う 6 テストは、hint 入りの module で落ちるので `--enable-threads --enable-tail-call --enable-exceptions --enable-extended-const --enable-multi-memory`(`WABT_FEATURES`)に置き換えた(code-metadata を有効にしない)。`--enable-all` を使わない他のテストは、wabt が section を無視するので落ちない。
- 将来 wabt を CI に入れるなら 1.0.36 では使えない(最新版が必要)。

## 8. サイズ(実測)

- n0 の増分 = hint section の大きさ(各 module の他の byte は同一): 合計 **+125,151 B**(11,019,843 → 11,144,994 B、+1.14%)。fle +56,658(+0.90%)、elastique +41,188(+1.78%)、oggio +9,024(+2.01%)、qfc +8,176(+1.07%)、dsp +6,853(+1.23%)、b2d +1,859(+1.24%)、ilt +1,393(+0.62%)、ft 0。
- CI の段階を通した後の増分(結合後 module): **flengine6.opt.wasm 142,899,156 B(hint 入り)対 142,776,280 B(hint 無し、x16)= +122,876 B(+0.086%)= hint section の大きさそのもの**(§4 の 3b。実アドレスの hybrid 段階、E10 の C 部)。brotli 後は bundle の encodedSize で比べる(`tables/bundle93.txt`、品質 9 の同じ手順で作った 2 つの bundle)。
- **first idle への影響(PM 依頼の式)**:
  - V8 13.6 の module decode(`new WebAssembly.Module`、実アドレスの結合後 module: hint 入り 142,899,156 B 対 hint 無し 142,776,280 B、7 回の min と median、`--experimental-wasm-branch-hinting` 有り無し、他の job が走っている最中の測定): hint 無しの min 91〜102 ms、hint 入りの min 99〜112 ms。**min の差は +1.2〜+10 ms で雑音と区別できない**(`tables/v8-module-decode-time.jsonl`)。hint section の構文解析の上限として t_parse ≤ 10 ms(27,570 件で割ると ≤ 0.36 µs/件)。P1(起動 10 秒)の 0.1% 以下。
  - 全関数を eager に validate + Liftoff compile した場合(`--no-wasm-lazy-compilation --no-wasm-lazy-validation`、node の同じ module。`new WebAssembly.Module` の 1 回目〔2 回目以降は V8 の cache に当たって約 100 ms〕): hint 無し 0.97〜1.38 s、hint 入り 1.01〜1.52 s(4 回ずつ、範囲が重なる = 差は雑音の内。`tables/v8-eager-compile-time.jsonl` の max_ms)。**hint 入りの module 全体を、hint を読む V8 が eager に検証して受理した**(1,044 ms)。
  - 式: ΔT(first idle) ≈ t_parse + 0(Liftoff は hint を読まない〔**読み**〕)+ ΔT_bg。ΔT_bg = r × T_TF,hint / n_th(r = TurboFan の機械語の増分比。alpha1 が f_b34aca0 で測った 411,884 → 502,352 B = +22.0% を compile 時間の増分の代理として使う〔**compile 時間そのものは未測定**〕、T_TF,hint = first idle までに tier-up する hint 付き関数の TurboFan compile 時間の合計〔**未測定**〕、n_th = バックグラウンドの compile thread 数)。tier-up が first idle より後なら ΔT_bg = 0(hint の効果は first idle ではなく定常の速さに出る)。
  - 結論: 測れた分(構文解析)は ≤ 10 ms。TurboFan の分は T_TF,hint が分からないと数にできない(alpha1 の Chrome A/B の first idle の列で見る)。

## 9. `-` の行(e11-wip 比。全部)

| # | 場所 | 行 | 理由 |
|---|---|---|---|
| 1 | wasm.rs `Code` | doc 1 行(`/// Instruction bytes of one function body (without the locals header).`)を、`hints` の説明を足した 2 行に | 説明 |
| 2 | wasm.rs `Code::new` | `Code { b: Vec::new() }` を `Code { b: Vec::new(), hints: Vec::new() }` に | 構造体の初期化 |
| 2b | wasm.rs `Module::encode` | code section の関数 body の locals ベクタを書く 6 行(`let mut f = Vec::new();` `uleb(&mut f, b.locals.len() as u64);` `for (n, t) in &b.locals {` `uleb(&mut f, *n as u64);` `f.push(t.byte());` `}`)を `let mut f = Self::locals_header(&b.locals);` の 1 行に | hint の offset と同じ計算を 1 箇所に(書くバイト列は同一) |
| 3 | lower.rs | `self.c.if_();` / `self.c.if_ty(…);` の 18 行 → `if_hint(…)` / `if_ty_hint(…, true)` | §3 |
| 4 | lower.rs `mx_force` | doc 1 行(`/// after a window condition … forces the slow path`)を、向きの規則を足した 3 行に | 説明 |
| 5〜10 | tests 6 file | `.arg("--enable-all")` → `.args(WABT_FEATURES)` 各 1 行 | §7 |

## 10. 方針との適合(必須の記載)

- **lift の忠実性**: 追加する処理は (c)『通信/変換の削減』でも (a)『余計な処理の除去』でも (b)『外部 API の契約の実装』でもない。**PM の指示(0293)による、性能のための注釈**で、生成された wasm の命令列・意味を一切変えない(hint section を除いた sha が従来と同一、8/8)。FL の挙動を補う・代行する処理、class/窓の種類/アドレスによる分岐は無い。付ける根拠は『この if は MXCSR 窓検査で、C 経路の関数を呼ぶ側が外れ側』という、生成側が持つ事実だけ(§3)。決定文書に明記する。
- **no-options**: 旗・環境変数・実行時の切り替えは無い。常に出す。テスト用の `HINTS_OFF` は `MX_FORCE_SLOW`・`LOCSHARE_OFF` と同じ型のテスト用 switch(thread-local。CLI・環境変数から届かない)。
- **進捗の誠実性**: 実エンジンが実行された範囲(§0 の 1)と、していない範囲(ブラウザの速さ)を分けて書いた。数は実測だけ。

## 11. 候補(未実装)

PM の判断(C2 は付けない)に従い、次は**付けていない**。参考(**手元の集計**: E10 の n0 8 module で、`c.*` の呼びが if の腕に直接ある数。prototype `tools/prototype/hintcount.py` による。C1+ の hint とは別の数え方):mx(`c.fle_hyb_mx`)31,976、ext(C 経路に残された関数の呼び)2,824、dispatch(`fle_hyb_dispatch`)1,533、import 11。mx 以外は、C 経路の関数を呼ぶ腕が cold とは限らない(ext は C 経路に残された関数で普通に呼ばれる)ので、alpha1 の C2 と同じ種類の誤りになる。typed_indirect の `if done` も §3 のとおり。mx のうち C1+ に入っていない物は、RC の helper の内側の if と、`mx_slow` を直接呼ぶ関数(`mx_c_only`・div/sqrt の packed など if を持たない形)。

## 12. 再現手順(tools/)

- `dry93.sh TAG FEWBIN`: E11 の hybn0 の引数で 8 module の n0 wasm を作る(`FEWBIN` に e11-wip のままの fe-wasmgen と hint 入りの fe-wasmgen を渡して比べた)。
- `surv93.sh TAG`: CI と同じ `wasm-opt -Oz` と merge1・merge2。
- `hintstat.py`: section の検査、hint 数、section を除いた sha256。
- `a4run93.sh`: a4。`nodewrap/node`: node に `--experimental-wasm-branch-hinting` を付ける wrapper(PATH の先頭に置いて cargo test を走らせる)。
- prototype(info 1 の保存確認): `hintinject.py`・`hintcount.py`・`hintcheck.py`。


## PM の判断と取り込み(2026-11-06 14:35、docs のみ。製品の差分は未投入)

取り込み: mt3 の最終(14:2x)の文書・表・道具と差分 `0293-branch-hint.diff`(9 file、+700/−33)を `docs/investigations/mt3w/0293/` に置いた。PM の dry-run(`git archive` の複製): E11 の e11-wip 799b0417 に offset 0・fuzz なし、E10 の main 9fdc492f に `lower.rs` が offset −39 行・fuzz/reject なし。実行したのは fe-wasmgen を実エンジンの 8 module に当てた生成、wasm-opt -Oz と merge、V8 の検証、node でのテストで、**Chrome での速さの測定はこの文書には無い**(alpha1 の 0290 に注入版の Chrome 1 組)。依頼者の機械は 0 件。

判断:
1. **hint(C1+、28,100 件、+122,876 B)は E11 の push 条件ではなく、E12 以降の候補**。実 bundle(`~/work/mt3w-bundle93/{h0293,plain}`)の Chrome A/B(自己時間と first idle の +100 ms 以内)が出るまで push しない。誤った hint は速度にだけ効く(全反転で 73 テストが通る)ので正しさの危険は小さいが、速度の根拠が node の 2 関数と Chrome の注入版 1 組(A/other −19.7%、重なりあり)だけで、UI 側(fle の 13,075 件)の向きの根拠は機械検査のみ。
2. 付ける範囲は C1+(窓検査の if + 外側の if)。C2(call のある腕を cold と決め打つ型)は alpha1 の node で UI 側が +18% 悪化したので採らない。
3. 未確認: Chrome での実 bundle の速さと first idle、fle の 13,075 件の効果、wasm-opt で落ちる 530 件(1.9%)の理由。
