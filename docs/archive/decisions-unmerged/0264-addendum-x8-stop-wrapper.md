# 0264 の続き(§21 以降、mt3): E7b(x8)が起動直後に止まる原因 = 0002(rsp-delta)の wrapper が、k≠0 の継続入口の復元した callee-saved レジスタを消す。修正 `0002b`

(この文書は 0264-f4776f60-size-limit.md の末尾に §21〜§26 として足せる番号で書いてある。決定 0270 は Elastique の 3 関数の flags 直 wasm 化に空けた。)

基準: HEAD d6926b4d 以降のコード(8c6b295e と同じ)+ E7b の 0001/0002/0004/0005。**実エンジンでの確認は opt1 の x13(= x9 の木 + 新 0002b)待ち**(この文書の測定は静的解析と node の合成試験だけ)。

## 21. 冒頭の 3 点
1. **測ったこと(事実)**: `f_<va>` の wrapper(C 経路から直 wasm への入口。`lib.rs`)は、呼び出しの前に callee-saved レジスタ(rbx・rbp・rsi・rdi・r12〜r15)を R[] から読み、関数の戻り値を R[] に書いた**後に、読んでおいた入口の値を R[] へ書き戻す**。普通の関数は callee-saved を保存するので、これは『関数の中の stub 呼び出しが R[] に書く一時値から C 側の呼び出し元を守る』だけの働きをする。しかし **k≠0 の継続入口**(別の関数が `jmp` で入る共有エピローグ。`add rsp,N; pop rbx; pop rsi; …; ret`)は、親のフレームから callee-saved を pop して復元するのが仕事で、wrapper はその結果を書いた直後に入口の値(親が使っていた一時値)で**上書きして消す**。HEAD では k≠0 の関数は C 経路に残っていたので表に出ず、0002 が初めて直 wasm にした。node の合成試験 `tests/rsp_boundary.rs`(C 経路の親が `jmp E`、E = `mov rcx,[rsp+8]; add rax,rcx; add rsp,0x20; pop rbx; ret`)で、**修正前は E の後の rbx が入口の値 0x1111、期待は pop した 0x2222**(E2 の `pop rdi; pop rsi; pop r12; pop r13; pop rbp` も同じ)。E7b の hyb-U(0005 まで)で k≠0 の関数はちょうど 7 本(fe-wasmgen の solve の k を全 U で出した): b2d 0x4770ed2(k=184)・0x47748e5(k=344)、dsp 0x275c35d(k=248)、Elastique 0xb034096・0xb0345a2・0xb04a921・0xb04addc(k=120)。18 run の実行: dsp 0x275c35d は 621 回(start 5.4 万・par-boot 31.6 万命令)、b2d 0x4770ed2 は 53 万回、0x47748e5 は 43 万回、Elastique の 4 本は start・par-boot で 0。
2. **結論**: 修正(`0002b`)は wrapper の『入口の値で戻す』対象から、その関数自身が結果(`res`)として変更する callee-saved レジスタを除く 1 行。普通の関数は callee-saved を `res` に持たない(pres が保存と見る)ので挙動は変わらない。試験は修正前に落ち、修正後に通る。`cargo test -p fe-wasmgen`(0001〜0005 + 0002b): 27 結果・51 通過・失敗 0。x8 の起動中の abort は、この壊れた callee-saved を受け取った C 経路(dsp の CPU 判定初期化付近)で起きた、というのが私の仮説(opt1 の JS スタックの f_2001040)。
3. **未確認**: 実エンジンで abort が消えるか(opt1 の x13 = x9 の木 + 新 0002b で確かめる。PM の木の検証: 生成 C の sha 不変、ci 試験 rc=0、test.sh full 210 結果・2,377 通過・失敗 0・無視 1、a4 942/942・895/895)。abort の直接の機構(どの検査が `abort()` を呼んだか)は見ていない。0261 の報告で『rsp-delta は健全』と書いた一部が誤りだった(C ↔ 直 wasm の境界で『k≠0 の継続入口が callee-saved を復元する』場合の試験が無く、試験のコメントに『rbx は検査しない』と書いて飛ばしていた)。

## 22. 差分
`e7/0002b-wrapper-keeps-the-registers-a-continuation-pops.diff`(PM が採った差し替え版は 15,806 B・b724cb7e。初版は 201 行、最終は 281 行。`lib.rs` の wrapper の 1 行 + コメント、`tests/rsp_boundary.rs` 新規。`-` は 1 行: `let cs = …` の旧い定義)。現 HEAD に `patch -p1 --dry-run` が通る(lib.rs に offset −13、fuzz なし)。0002 の直後に当てる。

## 23. 修正の追加(0002b の 2 つ目の変更)
最初の修正(wrapper が `res` に入る callee-saved を『入口の値で戻す』対象から除く)だけでは、**モジュールの中の C 経路の関数から `jmp` で入られない継続入口**(`called_from_c` に入らない=dispatch か、モジュールに現れない jump で入られる物)は `res` に callee-saved が入らず、pop した値が R[] に出ない。そこで **k≠0 の関数は `want`(結果として返すレジスタ)に ALL_REGS を入れる**(`rdelta[k] != Some(0)`、`lib.rs` の `out` の計算に 1 項)。k≠0 の関数は常に別のフレームの継続入口で、誰が入っても C の版は pop した値を R[] に残すから。普通の関数(k = 0)は変わらない。

## 24. 監査(PM の 01:12 の依頼 1): wrapper の『入口の値で戻す』『入口で読む』対象
- **入口で読む**: `sg.params()`(= `inr`)のレジスタだけを R[] から読む。callee-saved に限らない。継続入口が親から引き継ぐレジスタ(r8・r11・rbx…)は `inr` に入る。穴なし。
- **戻りで書く**: `res`(= `out`)のレジスタを R[] へ、R[4] は入口 + k。`out = (変更したレジスタ ∩ want) ∪ …`、`want` は直 wasm の caller の需要 + rax(export)+ **C 経路の関数から呼ばれる/jmp される関数は全レジスタ**(`called_from_c`、`call_targets` は call と tail jmp の両方を含む)+(今回)k≠0 の関数は全レジスタ。**戻りで callee-saved を入口の値に戻す対象は、`res` に入らない callee-saved だけ**(今回の修正)。rax・rdx・r8〜r11 などの volatile は『戻りで入口の値に戻す』処理が無い(`res` に入った分だけ書く)ので、同種の穴はない。
- **XMM(xmm6〜15 が callee-saved)**: wrapper は触らない。`XPlan` が、関数が書いた xmm(`own`)を**どの `ret` でも X[] へ書く**ので、継続入口が `movaps xmm6,[r11-0x10]` で復元した値は X[] に出る(b2d 0x4770ed2 の出口がこの形)。入口の値で戻す処理は無い。穴なし。
- **フラグ**: 直 wasm は C の `FL` を持たない。継続入口が親のフラグを読む場合は『reads a flag that is live into the function』で lowering しない(無言の誤りは無い)。
- **直 wasm どうしの tail jmp**(C を経由しない): `Term::Tail(g)` は g の結果をそのまま返し、親の `out` に g が変えるレジスタが入る(親の `pres` は g の `pres` を畳む)。親が push した callee-saved を g が pop して復元する形は 0261 の試験(P → E: rbx が親の書いた slot から戻る)が通る。0001(locshare)はフラグの局所だけに関わり、レジスタの受け渡しには関係しない。
- **インデックスの出ない経路**: 間接 call/jmp 用のアダプタ(`ia_<va>`)は `!c_mode`(非 hybrid)だけで、製品の hybrid では作られない。C → 直 wasm の入口は `f_<va>` の wrapper だけ。

## 25. 試験(PM の依頼 2)
`tests/rsp_boundary.rs` を 1 本から 4 本にした(いずれも node、修正前に落ち、修正後に通る。k≠0 の 7 本の形に近い合成): (1) 共有エピローグ `add rsp,0x20; pop rbx; ret` と `add rsp,0x30; pop rdi; pop rsi; pop r12; pop r13; pop rbp; ret`(C 経路の親の `jmp` から)、(2) E3: 6 つ pop(r15・r14・r13・r12・rdi・rbx)+ rsi を `lea` で書くが pop しない + rbp は触らない + R[4] が +0x68(= b2d 0x4770ed2 のような `pop` と上書きの混在)、(3) E4: 出口の pop 量が違う継続は C 経路に残る、(4) 親がモジュールに無い継続(dispatch で入る物)が pop した rbx を返す(2 つ目の修正の試験)。`cargo test -p fe-wasmgen`(0001〜0005 + 0002b): **27 結果・51 通過・失敗 0**。実関数 7 本(`tables/k-nonzero-in-U.tsv`)を host x86 と比較できない理由: どれも別のフレームの継続入口で、親のフレーム・レジスタ・DLL の静的データ(node の試験環境は DLL を読み込まない)が要るため、関数単体を実行できない。同じ理由で fe-fruity の x87_hybrid/small_ops_hybrid の流儀(1 関数を合成で走らせる)の対象にならない。wrapper の意味(レジスタの受け渡し)だけを合成で調べた。

## 26. 訂正文(PM の依頼 3): 0261 / 0264 の docs の『rsp-delta は健全』(本文側の訂正は `docs-0261-0264-retraction.diff`)
- **0261 §2 の最後の項**『k は solve の中で他の署名と同時に決まり、呼び出し側は callee の署名で RSP を足すだけで、レジスタを書かない』は **RSP の値についてだけ正しい**。wrapper(C → 直 wasm の入口)は callee-saved を『入口の値で戻す』処理を持ち、k≠0 の継続入口が pop した callee-saved を消す。この節の『HEAD の ok が不健全だった』という主張(oggio の 6 関数)は変わらないが、『0002 は健全』と読める記述(0261 §2・§3 と、0264 §16 の最後の段落)は**撤回する**。
- **誤りの原因**: 0261 の試験 `tests/rsp_delta.rs` は『C 経路から呼ばれた k≠0 の関数が戻った後の R[4]』を見たが、**同じ呼び出しで callee-saved レジスタが pop した値になるか**を見ていなかった(試験のコメントに『rbx はここでは検査しない』と書いて飛ばした)。census と cargo の試験は直 wasm どうし・R[4]・rax だけを見た。k≠0 の関数は HEAD では C 経路にあり、wrapper のこの処理を通ったことが無かった(= 0002 が初めて通した)。
- **E7b で表に出た経路**: x8(0003 入り)の停止は、起動で動く dsp 0x275c35d(621 回)と Blend2D の 0x4770ed2・0x47748e5 が、C 経路の親から jmp されて戻るたびに callee-saved を壊したため、と読む(実エンジンでの確認は x13)。
