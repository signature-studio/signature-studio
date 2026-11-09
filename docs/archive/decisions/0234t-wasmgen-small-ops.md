# mt3 (決定 0254 仮): fe-wasmgen の『小さい』3 項目 — Jecxz/Jrcxz、Cpuid、x87 制御語(Fnstcw/Fldcw)

基準: HEAD 83637721(lift/crates は 2c1029ed と同一)。成果: `mt3-wasmgen-small.diff`(`patch -p1 --dry-run` が `git archive HEAD` の木に通る。746 行: ir.rs / lib.rs / lower.rs と試験 `tests/small_ops_hybrid.rs`)。**opt1 の Cvt* / Rsqrtss / Elastique の腕には触れていない**(lower.rs の差分は Fnstcw/Fldcw の腕の差し替え 1 か所、Jcc 終端の関数化、Cnd の追加、Cpuid の腕の追加)。

## 1. 変更
- **(a) Jecxz / Jrcxz**: `ir.rs` で near の `jrcxz`/`jecxz` を新しい終端 `Term::Jcz { wide, taken, fall }` にする(これまで『loop/jrcxz/far conditional branch』で関数ごと C 経路)。`lower.rs` の Jcc の 2 つの腕(`dispatch_pc` つきと通常)を 1 つの `jcond(b, Cnd, taken, fall)` に畳み、条件を `Cnd::Cc(cc)`(従来のフラグ)と `Cnd::Z { wide, neg }`((r|e)cx == 0)にした。`jecxz`(67 プレフィックス)は ecx の下位 32 ビットだけを見る。`loop`/`loope`/`loopne`/`jcxz`(16 ビット)は従来どおり C 経路。フラグは読まない(`cond()` は呼ばない)。
- **(b) Cpuid**: 答えは leaf だけで決まる定数なので、直 wasm の中で計算する(helper 呼び出し無し): eax = (leaf == 0)、ebx = ecx = 0、edx = (1 ≤ leaf ≤ 0x7fffffff) ? FLE_CPUID_LEAF1_EDX(FPU・CX8・CMOV・MMX・FXSR・SSE・SSE2) : 0。C 側の `fle_cpuid.c` との一致は**試験で固定**(下)。
- **(c) Fnstcw / Fldcw**: 制御語は C 経路の状態(`fle_x87.c`)なので、直 wasm は C の 2 関数 `c.fle_x87_store_cw(addr)` / `c.fle_x87_load_cw(addr)` を実効アドレスで呼ぶ(`Ldmxcsr` の `mx_ld` と同じ形)。import は**その命令を持つモジュールだけ**(`lib.rs`)。`exports.rsp` は import から自動で作られるので C 側の export は増やさない。`Fstcw`(9B つき)は iced が `wait` + `fnstcw` に分けるので対象外(`wait` が未対応)。

## 2. 試験(`tests/small_ops_hybrid.rs`、9 件。第 9 件は §6。`cargo test -p fe-wasmgen --no-fail-fast` を HEAD e6433b23 + 本差分の木で実行: test result 21 本すべて ok、33 件通過、失敗 0、無視 1(表 `tables/wasmgen-small-cargo-test-on-HEAD-e6433b23.txt`))
- (a) rcx / ecx の 12 値(0、1、0x7fffffff、0x80000000、0xffffffff、0x1_0000_0000、0x1_0000_0001、0xffffffff00000000、u64::MAX ほか)で `jrcxz` は rcx、`jecxz` は下位 32 ビットだけを見る; `jecxz` をヘッダにした後ろ向きの辺を持つループ(ecx を数え下げ、結果 = ecx の下位 32 ビット)。
- (b) 18 個の leaf(0〜7、0xb、0xd、0x14、0x15、0x7fffffff、0x80000000/1/8、0xc0000000、0xffffffff)× 上位 32 ビットの垃圾 × ecx 2 通りで、**native の `fle_cpuid.c` を clang でコンパイルして走らせた結果と、直 wasm の eax/ebx/ecx/edx が一致**。
- (c) **65,536 個すべての制御語**を `fldcw [rcx+8]; fnstcw [rcx+0x10]` に通し、保存値 = `(v & 0x1F3F) | 0x40`、状態、呼び出しの順序とアドレスを確認; `fnstcw [rcx+rdx*2+4]` の実効アドレス; 絶対アドレス; 命令の無いモジュールが helper を import しない; JS の stub が `fle_x87ctl.c` の式と同じであることを C の文面で固定。
- **変異の確認**(試験が感度を持つこと): cpuid の edx から SSE2 のビットを落とす → (b) が失敗; `jecxz` を rcx の比較に変える → (a) が失敗。fldcw の実効アドレスから `off` を落とす変異は通る(`mem_addr` が変位を基準に畳むので `off` は常に 0 = 等価な変異)。

## 3. census(モジュール全体、`--census`、HEAD 対 変更後。`/tmp` に出力、表は `tables/wasmgen-small-census.txt`)
| | functions_ok(old → new) | insns_ok(old → new) | この項目の blocker |
|---|---|---|---|
| FLEngine(fle) | 58,555 → 58,571(+16) | 2,276,795 → 2,277,519 | Jecxz 10 関数、x87 制御語 4、Cpuid 4(全部消えた) |
| dsp_ippv2 | 14,640 → 14,870(+230) | 3,414,181 → 3,566,265(+152,084) | x87 制御語(167 roots、後ろ向き 2,204 関数)、Cpuid(73 roots、7,109 関数)が消え、次の blocker は `Xgetbv`(4 roots、1,780 関数) |
- gpu1 の表に出ている関数: **`f_6a1e50`(fle、Jecxz、470 ms)= 直 wasm 3,122 B で ok**; **`f_310fb40`(dsp、x87 制御語、761 ms)= 156,457 B で ok**; **`f_3ac7790`(dsp、Cpuid、210 ms)= 2,020 B で ok**; `f_32e9c00`(dsp、x87 制御語、122 ms)も解除。
- 注意: 『直にできる』になっただけで、**実際に直になるかは hybrid の一覧(`hyb-U-*.txt`)にこれらが入ること**による(一覧は PM / opt1 の選定。サイズ制限やクロージャの条件もある)。

## 4. 効果の見積り(gpu1 のミキサー内の自己時間の表から。直接測っていない)
- ミキサーの自己時間に占める割合: (a) 467 ms(3.3%)、(b) 210 ms(1.5%)、(c) 1,001 ms(7.1%)= 1,678 ms(約 11.8%、ミキサーの合計 約 14.2 s)。
- 直 wasm にしたときの短縮は、ハイブリッド全体の利得(非ハイブリッド 2.1 → 1.3 CPU 秒/秒、1.6 倍)を当てると 1 − 1/1.6 = 37.5%: 1,678 ms × 0.375 = **約 630 ms = ミキサーの 4.4%**。**ミキサーの速度は約 1.046 倍**にとどまる(1.3 倍には遠い)。per-function の倍率は測っていない(この 1.6 は全体の平均)。大きい項目は Elastique(57%)と C 経路の関数が引き起こす費用(60%)。

## 5. 取り込み前の確認の結果(2026-11-05 20:10): **fle のハイブリッドの出力が変わる(2 関数、+44 B)。84/55/87 行と a4 は走らせていない**
- 道具: `tools/wasmgen-same.sh`(変更前 = HEAD の fe-wasmgen、変更後 = 本差分。現行の一覧 `hyb-U-<module>.txt`、8 モジュールの実物の DLL、`--hybrid-only`)。表: `tables/wasmgen-small-same.txt`。
- **7 モジュール(dsp、b2d、qfc、ft、ilt、oggio、elastique)は直 wasm がバイト単位で同一**(sha256 先頭 16 桁が一致)。報告表の差は理由の文字列と、一覧に無い関数の署名だけ(『ok』の行の差は 0)。
- **fle だけが違う**: 6,009,732 → 6,009,776 B(+44 B)。違う関数は 2 つ(0x6a45e0 の 980 → 994 B、0x11eaec0 の 7,565 → 7,595 B)。どちらも**JECXZ を含む C 経路の関数の呼び出し側**で、呼び出しの後の rax・rcx・rdx(と 1 か所の呼び出しの前の xmm0)の受け渡しが増えた。原因・逆アセンブルの確認・変更前の出力が『この 2 か所では結果が正しいと読める』理由は表の下半分。
- したがって**『製品の wasm と 84/55/87 行・a4 は変わらない』とは言えない**。取り込むなら、fle の直 wasm が変わったビルドで 84/55/87 行と a4 を走らせる必要がある(私は走らせていない: 木の違う 2 本の全面ビルド + node の走行が要り、この機械・負荷 17〜20 の残り時間に収まらない)。実行時の根拠は、差分試験(下の試験を含む 9 件)と、2 関数の wasm の差を手で読んだことだけ。

## 6. 見つかったこと: JECXZ を含む関数の署名が切れていた(変更前の fe-wasmgen の欠陥。本差分が JECXZ の分は直す)
- 変更前、`ir.rs` は JECXZ/JRCXZ/LOOP を『後続の無い終端』(`Term::Unsup`)にしていた。終端に後続の辺が無いので、署名の解析(`pres::analyze`、`liveness`)に JECXZ より先のブロックが入らない(と読んだ。試験の結果と整合する)ため、**JECXZ より後で読む・変える・返すレジスタが署名に入らなかった**(fle の JECXZ 関数 10 個は `sig_out = 0x0000`、つまり『何も返さず、全レジスタを保存する』)。呼び出し側は、その関数が実際には rax を返していても、自分の rax を持ち越す。
- 試験 `a_caller_reads_back_the_result_of_a_c_path_function_that_holds_a_jecxz`(`tests/small_ops_hybrid.rs`): 呼び出し側(直 wasm)が C 経路の関数(`mov eax,ecx; jecxz +2; inc eax; ret`)を呼んで rax を使う。**変更前の fe-wasmgen では失敗**(rcx = 0 のとき eax = 2596069105 = 呼び出し側の古い rax + 1。正しくは 1)、**変更後は通る**。(変更前の木 `h8.pristine` に試験を一時的に置いて確認した。)
- 製品の出力では、この欠陥は 2 か所に現れていたが、どちらも結果に効かなかったと読んだ(逆アセンブルで、読み戻される rax・rcx・rdx が使う前に上書きされることを確認。f_6a22d0 の後の rax は『戻り先のループで使うか』を確かめていない)。84/55/87 行・a4・Export の一致は、この 2 か所が無害だったこととも矛盾しない。
- **同じ種類の切れ(別の `Term::Unsup`)は、本差分では直らない**: 現行の一覧の直 wasm が呼ぶ C 経路の関数のうち、ブロックが切れているもの(理由が『undecodable instruction or falls out』ほか)は dsp に 40 個、elastique に 2 個、fle・qfc・b2d・ft・ilt・oggio は 0 個(`tools/wasmgen-unsup-callees.sh`、`tables/wasmgen-small-same.txt`)。これらは全部 2 命令・16 バイトの関数で、`call <別の関数>` の後に nop が続く形(IPP の CPU 振り分けの入口に見える。逆アセンブルで 2 個だけ確認)。署名は `sig_out = 0x0000`。**この形が呼び出し側に害を及ぼすかは解析していない**(Export が native と一致していること、a4 が通ることが間接の根拠)。指摘だけに留める。
