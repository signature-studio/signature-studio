# int_ext・int_more の host 比較の失敗の分類(#1395 の確認)

作業の記録(2026-11-27、UTC 06:43 開始)。対象: `lift/crates/fe-lift/tests/int_ext.rs`・`int_more.rs` の `*_matches_host`。

## 0. 方法と限り

- 読むだけ。試験・lift の生成部・runtime・生成された C・CI の記録を読んだ。
- **試験は実行していない**(依頼の規則で、この機械では lift の build と cargo test を行わない)。左右の値は、記録に無い物は確かめられない。
- 「実測」は、読んだ行と、記録にある数字。「推定」は、読みからの推論。

## 1. 試験が比べる物

- 比べる: rax・rcx・rdx・rbx・rsi・rdi の 6 本(`int_ext.rs:14`、比較は `int_ext.rs:221-233`)と、バッファの 64 バイト。
- 比べない: フラグ(`int_ext.rs:230-231`、`int_more.rs:243` の注釈と比較)。rsp・rbp・r8〜r15 も比べない。
- mem のベクタ(`ins_mem`)は、比較の前に rdi を両側で 0 にする(`int_ext.rs:226-229`、`int_more.rs:239-242`)。host の rdi はバッファの番地(`int_ext.rs:137-139`)、C の rdi は静的配列の番地(`int_ext.rs:172-174`)で、0 にしないと必ず違う。
- mem でないベクタは rdi に番地を入れない(`host_run` の `int_ext.rs:137` と `c_run` の `int_ext.rs:172` は `v.mem` の時だけ)。

## 2. 判定

| 試験 | (a) CPU で未定義の出力 | (b) rdi のポインタ | (c) lift の欠陥 | 読んだ行 |
|---|---|---|---|---|
| `int_ext movsx_matches_host` | 該当しない | 該当しない(現コード) | 読んだ範囲では見つからない。確認できない(実行していない) | `int_ext.rs:56-75`・`221-234`・`236-239`。lift: `src/int_ext.rs:30-41`・`124-133`・`147-175`。runtime: `runtime/fle_runtime.h:56-57`・`64-73`。生成 C: `lift/out-int-ext/test/movsx-3542719-3.c` の c0〜c15 |
| `int_ext idiv_imul_matches_host` | 該当しない | 該当しない(現コード) | 読んだ範囲では見つからない。確認できない(実行していない) | `int_ext.rs:97-123`・`247-254`。lift: `src/int_ext.rs:43-90`。runtime: `runtime/fle_int_ext.c:6-51`。生成 C: `lift/out-int-ext/test/idiv_imul-3542719-1.c` の c0〜c20 |
| `int_ext sign_extension_matches_host`(対照) | — | — | — | mem のベクタが無い(`int_ext.rs:77-94`)。作業者の記録では通った |
| `int_more xadd・cmpxchg・bt・div_matches_host` | 該当しない | 該当しない(現コード) | 未確認。落ちる件の名前は手元の記録に無い | `int_more.rs:249-267`・`234-247` |

### (a) CPU で未定義の出力

- movsx は宛先だけを書き、その値は SDM で定義されている。
- idiv の rax・rdx は、#DE でなければ SDM で定義されている(未定義はフラグの 6 個)。
- 1 オペランドの imul の rax・rdx も定義されている(未定義はフラグ)。
- 未定義なのはフラグだけで、フラグは比べていない。よって、CPU の違いで比較の結果が変わる経路は、これらの試験には無い。**(a) とは判定しない。**
- 一覧(`tools/fe-lift-host-dependent.txt`)に足す物は無い。

### (b) rdi のポインタ

- 現コードでは、rdi にポインタが入るのは mem のベクタだけで、そのベクタは比較の前に両側で 0 にされる(`int_ext.rs:226-229`、`int_more.rs:239-242`)。
- mem でないベクタには番地が入らず、rdi の値は両側とも入力どおり(`int_ext.rs:137`・`172` を参照)。
- **よって、この版のコードでは rdi の差は比較に残らない。** 前々回の報告の「rdi だけが落ちた」は、この版の試験からは説明できない(3 節と 4 節)。
- **(b) とは判定しない**(試験側の誤りは、現コードには見つからない)。

### (c) lift の欠陥

- 読んだ範囲で確かめた点:
  - movsx: 宛先の幅ごとの書き方(32 ビットは 0 拡張、16 ビットは上位を残す、8 ビットの高バイトは `R[n] >> 8`)が SDM どおり(`src/int_ext.rs:31-41`・`125-133`・`111-123`)。
  - idiv: 被除数は AX・DX:AX・EDX:EAX・RDX:RAX、商は AL・AX・EAX・RAX、余りは AH・DX・EDX・RDX(`src/int_ext.rs:45-68`)。runtime の `fle_idiv` は符号付きの 2w ビットの被除数を正しく作り、商の範囲を越えると止める(`runtime/fle_int_ext.c:12-36`)。
  - 1 オペランド imul: 8・16・32・64 ビットの積の置き場所(`src/int_ext.rs:71-90`)。runtime の `fle_imul1` は CF = OF を SDM どおりに作る(`fle_int_ext.c:38-51`)。
  - 生成 C: 同じ試験の入力を lift した C(この機械の `out-int-ext/test` に残る 2026-11-25 の生成物。lift の該当部分は 2026-11-20 の f4cffe71 以後変わっていない)を、ベクタごとに読んだ。movsx の 16 本、idiv・imul の 21 本(#DE の 1 本を除く)。入力と出力の式は、各ベクタの SDM の結果と一致した。
  - #DE の除外(`int_ext.rs:247-249`): 21 本の商は、どれも範囲内(手で確かめた)。
- **読んだ範囲では (c) の欠陥は見つからない。** ただし、実行していないので、「見つからない」は確認ではない。

## 3. 記録の中身(実測。読んだ記録)

- 2026-11-25 の全体の run(`pm-work/findings/view-11-1-null-call-2026-11-25.md`、「受け入れの結果」の節): 新しい失敗は `int_more2 bsf_bsr_match_host` の 1 件だけ。**int_ext の 2 件は出ていない**(その run では通った)。
- PM の基準(`pm-work/findings/fe-lift-test-baseline.md` §5): 残りの 59 件に int_ext は無い。
- 2026-11-27 の run 45922656650(作業 #1395)。その作業者の記録(成果物 `agent-1395` の `report.md`):
  - `movsx_matches_host`・`idiv_imul_matches_host` が落ちた、と書いてある。
  - 失敗は「`cst[5]`(rdi)の比較」と書いてある。失敗の左右の値は記録に無い。
  - 「`mem=false` のベクタで IN-IF が出る」という出力を、作業者が読んでいる。現コードの `if v.mem` の中の出力は、mem のベクタでしか出ない。二つの試験が並行に出力しているので、この読みは混ざっている可能性がある。
  - 「main(`c8d20c79`)でも同じ 2 件が落ちる」と書いてある。その run の出力は手元に無い(この機械では確かめられない)。
- run 45922656650 の CI のログ(`gh run view --log`)と `judge.txt` には、試験の出力(assertion の本文)が無い。

## 4. 推定(実測ではない)

1. 最も有力: 作業者の run で使われた試験の binary が、現在の試験のコードと違う版だった(rdi の 0 化が無い版)。rdi の差が「ポインタ」として出るのは、その版の場合だけ。記録の食い違い(3 節の IN-IF)も、この説明と合わない。作業者は、target をキャッシュから復元したと書いている。**確かめ方**: 失敗する run で、target を作り直し(キャッシュ無し)、2 件を単独(`--test-threads=1`)で走らせ、assertion の `left`・`right` を出す。
2. 次点: 並行に走る試験の出力を読み違えた(作業者の記録の IN-IF)。
3. 低い: host の CPU の違い。上の (a) の理由で、レジスタの結果には入らない。

## 5. 一覧と spec

- (a) と判定した物は無い。よって `tools/fe-lift-host-dependent.txt` と `spec/fe-lift-test.test.mjs` は変えない。
- `node --test spec/fe-lift-test.test.mjs` は通る(変えていない)。

## 6. Issue にすべき物(PM が起票する)

- 題名案: `fe-lift int_ext: movsx_matches_host・idiv_imul_matches_host の失敗の左右の値を出し、原因を分ける(試験の binary の版・並行の出力・lift のどれか)`
  - 内容: 2 件を、キャッシュ無しの target で、単独に走らせる。assertion の `left`・`right` と、失敗した index を記録する。
  - (c) の欠陥が見つかった時だけ、lift の欠陥の Issue を別に起票する。現時点では、(b)・(c) の Issue の根拠は無い。

## 7. 確認できなかった物

- 試験の実行による確認(この機械の規則)。失敗の左右の値は、この機械では見ていない。
- 前々回の報告の int_more・int_ext の「6 件」の名前と値。その報告の本文は、手元の記録(リポジトリ・CI の成果物)に無い。
- 失敗した runner の CPU の種類と、clang の版。
- int_more の各試験の lift の欠陥の有無(読んだのは試験の比較の仕組みと、mem の扱いだけ)。
