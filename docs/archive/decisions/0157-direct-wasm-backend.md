# 0157: x86-64 機械語から wasm を直接生成する生成部(`fe-wasmgen`)の試作。fe-pe・fe-decode の前段を流用して生成部だけを置き換え、freetype で 1,639 関数を lower、差分テストの不一致 0、サイズは C 経路(-Oz)の約 0.5 倍、整数の速度は native の約 1.0〜1.2 倍。SSE の MXCSR・x87・AVX・setjmp は未対応で、混成(扱えない関数を C 経路に残す)の試作で FreeType の公開 API を通した。すべて仮の決定

状態: **仮の決定(すべての選択)**。**これは新しい生成部の試作の測定で、進捗の到達ではない。** 本番のビルドは変えていない(`lift/runtime/` と `fe-lift` の C 生成は無変更)。依頼は PM(番号 0157 は PM の割り当て、仮)。調査・実装・測定・文書は作業者(wasmgen1)が行った(2026-11-03 03:52〜。差分 `wasmgen1.diff` は `git archive` した `lift/` に対するもので、新規 crate `lift/crates/fe-wasmgen/`、`lift/Cargo.toml` の workspace member 1 行と `[profile.dev.package.fe-wasmgen]` 3 行、`lift/Cargo.lock` の新 crate の項だけ。`patch -p1 --dry-run` で当たる)。

区分の凡例: **実測**＝このセッションのコマンドの出力を見た事実。**読み**＝コードや出力を読んで得た解釈で、確かめていない。**未確認**＝確かめていない。**推測**＝根拠が弱い見積り。**出典なしの選択**＝名前つき(`S-wg1-…`)で撤回条件を書く。数字はすべて実測で、速度は **CPU 秒**(`process.threadCpuUsage` またはプロセス CPU)で測り、負荷平均(共有機で 7〜25)を併記する。wall-clock の速度は載せない(負荷が 8 を超える時間が長かったため)。

## 先に結論

| 項目 | 結果 |
|---|---|
| lower できた量(実測) | freetype(本番と同じ seeds `ft-seeds-4b00000-c2.txt` + pdata): **1,747 関数のうち 1,639 関数(93.8%)、命令 112,274 のうち 100,611(89.6%)**を wasm にし、`wasm-validate` を通した。残りの 108 関数は、`_setjmp`(XMM 入口 live)とその呼び元、デコード不能 1、RSP が入口値のまま戻らない 65(他関数の枠の途中に入る継続入口)。`--promote-shared` を付けると 65 → 3 で、**1,641 / 1,776 関数、命令 100,604 / 105,182(95.7%)**。混成モードでは **1,664 関数(95.2%)**、混成 + promote-shared で **1,733 / 1,776 関数、命令 104,414 / 105,182(99.3%)** |
| 5 モジュール全関数(実測、`--census`) | 純粋(呼び先もすべて lower できる)の関数数: FLEngine 21,963 / 59,305(**37.0%**)、dsp 1,527 / 40,259(3.8%)、Blend2D 788 / 3,093(25.5%)、QFC 3,117 / 6,712(46.4%)、ILTools 5,872 / 12,930(45.4%)。命令数では FLEngine 14.9%、dsp 1.2%。全関数に対する率は悲観的な指標で、本命は次の行 |
| **実行される関数集合**(size1 の 12,298 関数、実測) | 自分の命令だけで対応できる関数: **11,124 / 12,298(90.5%)**(FLEngine 91.4%、QFC 89.3%、ILTools 93.6%、Blend2D 62.6%、dsp 72.5%、freetype 99.7%)。**呼び先をすべて含めて新バックエンドだけで閉じる関数は 5,450(44.3%)**。入口回数で重みづけすると 77.4% / 47.5%、`emitted_native_insns` で重みづけすると 52.1% / 13.2%。混成にすれば「自分の命令だけ」の率がそのまま lower される率になる(後述) |
| 差分テスト(実測) | 新(wasm)対 C 経路(生成 C を emcc -O2)。freetype の 1,639 関数 × 300 反復 = **482,464 反復で、不一致 0、新だけが落ちる 0**。比較は rax・callee-saved・rsp・scratch・スタック引数・.data・低位 64 KB・import 呼びの列。C 経路の配置依存(別配置の 2 本目の C と食い違う)728 反復と、前反復の残骸 2 反復は除いた(分類は下)。check Elim の同じテストは、値の不一致 0・新だけが落ちる 16,829 反復(63:32 の検査が、負の基底 + 変位で C 経路より厳しい)。**差分テストが見つけた新バックエンドの実バグは 2 件**(rep scas/cmps の範囲検査、間接呼びアダプタが rax を渡さない)で、無作為分割の混成テストが見つけた境界の実バグも 2 件(ラッパが RSP を戻さない、スタブが C の callee-saved を壊す)。いずれも修正済み。hang 38 関数(4 秒の監視で未完)は完了した反復だけを数えた |
| FreeType 公開 API(実測) | 純粋な新バックエンドは `FT_Init_FreeType` → `FT_Done_FreeType` が C 経路と一致(戻り値・import 76 回・ヒープ 22,256 B)。**混成**(新 1,664 + C 83 関数、1 つの wasm メモリ。無作為に 30%・50%・80% だけを新にした 3 分割でも結果は同じで、境界の往復は各方向 8 千〜2 万回)では `FT_New_Memory_Face`・`FT_Set_Pixel_Sizes`・`FT_Get_Char_Index`・`FT_Load_Glyph`・`FT_Render_Glyph`・`FT_Done_Face` まで通り、**65 グリフで戻り値一致、import 301 回(引数 3 つ)一致、ヒープ(75,296 B、ビットマップを含む)は 5 バイトだけ違う**(rasterizer の jmp_buf の r12・r13・r15 の保存値。テスト用アダプタが 0 を渡す部分で、longjmp が起きなければ読まれない) |
| サイズ(実測、freetype の 1,639 関数の本体、native の命令バイト 372,686 B) | native → C 経路 -Oz(wasm-opt 後)1,939,745 B(**5.20×**、19.3 B/insn)→ **新 wasm-opt -Oz 1,015,979 B(2.73×、10.1 B/insn)**。C 経路 -O2 は 4,374,076 B(11.74×)。新の check 別(wasm-opt 後): Exact 2.73×、Base 2.48×、Elim 2.35×、なし 2.03×。**S1 の 2.0× に最も近いのは check をすべて外した 2.03× で、届かない**。実行される関数 10,808 の本体は C -Oz の 0.60 倍(emit のまま)。**ただし xz -9 後は新 288,312 B 対 C -Oz 290,760 B でほぼ同じ**(配信サイズには効かない) |
| 速度(実測、CPU 秒、native 比) | 整数(K3 xorshift)**1.0×**、K5 挿入ソート **1.11×**(check なしで 0.64×)、K6 fib **1.06〜1.21×**。スカラー double(K2 biquad)**2.05×**・K4(sqrt/div/cvt)**1.49×**(MXCSR を持たない既定)、MXCSR の窓つきで K2 **5.8〜6.7×**・K4 **3.9〜4.3×**。packed float(K1)**1.70×**(MXCSR なし)。C 経路 -O2 は K1 約 150×、K2 約 17×、K3 約 10×、K4 約 12×、K5 約 25×、K6 約 13×。→ PM の go/no-go(整数 ≤2×、スカラー ≤4×、packed ≤6×)は MXCSR を持たない既定で満たし、**窓を入れた K2(5.8〜6.7×)・K4(3.9〜4.3×)は 4× 付近か超える**(窓は素朴な形。最適化は未実施) |
| 生成時間(実測) | freetype: 新 **CPU 2.1 s・48 MB**(生成そのものは 1.6 s。fe-lift の C 生成 0.25 s のあとに emcc -Oz 32.8 s・661 MB、-O2 102 s・1.4 GB)。FLEngine 全体は 10.5 s CPU・884 MB、dsp 11.8 s・1.9 GB。混成モードは FLEngine で 84 s・1.0 GB(C 側の関数も署名導出に含めるため。最適化していない) |
| V8 のコンパイル CPU(実測、freetype、eager) | TurboFan: C -Oz 1.17 s、**新 1.54 s**、C -O2 3.18 s。Liftoff: 0.06 / 0.05 / 0.12 s。→ **バイトは半分でもコンパイル CPU は C -Oz より増える**。起動(P1)が速くなるかは**未確認** |
| 混成の速度(実測、freetype の loadglyph + render を 16 グリフ × 60 回、CPU 時間、負荷 8〜16) | C 経路単独(-O2)307 ms → 混成(JS 経由の橋)96 ms(**3.2×**)。`wasm-merge` で 1 モジュールにすると 72 ms(**4.3×**)。3 ラウンドの最小どうし |
| 本番のビルドへの影響 | なし。既存ファイルの編集は `lift/Cargo.toml`(member 1 行・profile 3 行)と、cargo が書き換えた `lift/Cargo.lock` だけ。`bash lift/harness/test.sh full` は **150 results、2,094 passed、0 failed**(最終パッチ。SKIPPED 1 件は `fe-lift/wall_fr9t` の既存のもの)。同じ HEAD(996990a)の素の書き出しは 145 results・2,090 passed・0 failed なので、差は新 crate の 5 results(bin・encode・lib・synthetic と doc-test)・4 passed だけ。基準として PM から聞いた 142 results・2,083 passed は古い HEAD のもの。実行: `FLE_INPUT_ROOT=… bash harness/test-env.sh --link` のあと `bash harness/test.sh full --jobs 2 --threads 2`(wall 239 s。`CARGO_TARGET_DIR` は `lift/out-wasmgen1/exp-target`) |

**肯定的に見せる点は限られる。** (1) 「不一致 0」は、freetype 1 モジュール・ランダム入力・上の除外つき。FLEngine(SEH・x87・スレッド・巨大関数)の機械語では**何も動かしていない**(census で lower 率を数えただけ)。(2) FLEngine で純粋に閉じるのは関数の 37%・命令の 15%。長い尾があり、上位 12 の原因を実装しても 53% で止まる。混成を前提にしないと本番には使えない。(3) SSE は最小限(K1・K2・K4 の形)で、MXCSR を持たない。窓つきは K2 で 6〜7× になる。(4) V8 のコンパイル CPU は C -Oz より増える。(5) 混成の橋は JS 経由の試作で、`wasm-merge` は memory が 2 本(同じ Memory を渡して別名にした)になる形で、本番の形ではない。(6) longjmp は両経路とも実行していない。(7) wwt の数字は PM から受け取ったもので、**私は wwt を実行していない**。

## 設計(何を作ったか)

### 方針と範囲

`fe-pe`(PE の読み込み)、`fe-decode`(関数の発見・解決された間接分岐・ジャンプテーブル)、`iced-x86`(デコード)は既存のものをそのまま使い、**生成部だけ**を新 crate `fe-wasmgen`(約 6,600 行 Rust、`wasm.rs ir.rs cfg.rs fx.rs flags.rs pres.rs lower.rs lib.rs main.rs`)にした。wasm のバイナリは自前のエンコーダ(`wasm.rs`)で書き、`wasm-opt`・`emcc` を生成に使わない(サイズ表の wasm-opt は測定用)。ホストは wasm32 の `env.mem` を import し、**ゲストのアドレス = wasm のアドレス**(identity map)。レジスタは wasm の **i64 ローカル**で、グローバルの `R[]` はない。仮定した ABI はなく、**関数ごとの署名は関数間の活性解析で導出**する。

### 署名・保存レジスタ・退避の省略

- **署名**(`lib.rs: solve/liveness`)。各関数の `inr`(引数にするレジスタ)、`ins`(厳密な読み)、`out`(戻り値にするレジスタ = 変更レジスタ ∩ 呼び元が必要とするもの)を不動点で求める。間接呼びに渡す rcx・rdx・r8・r9 は『弱い読み』(`inr` にだけ入る)。MSVC の LTCG は内部関数に独自の規約(rbx・rbp・rsi・r10・r11 に引数)を使うが、署名は解析で出るので壊れない。
- **保存レジスタの証明**(`pres.rs`)。前向きの状態(`Hold`/`Rel`/`Other`)とスタックスロットの追跡で、callee-saved レジスタが戻りで入口値に戻ることを証明する。証明できた `push`/`pop`・`mov [rsp+d], reg` は**メモリ書き込みごと省略**する(入口値が別に使われる、尾部のスロットが live、のときは省略しない)。省略はサイズを約 6% 減らす(1,639 関数中 1,075 に省略あり)。仮定: 保存スロットを別のポインタが上書きしない(**未確認**)。
- **RSP**。`ret` や末尾ジャンプで RSP が入口値でない関数(他関数の枠の途中に入る継続入口)は生成しない(freetype で 65 関数)。`ret imm≠0` も生成しない。`--promote-shared` で共有の尾部を関数に昇格すると 65 → 3 に減る。

### フラグ

フラグごとの到達定義(`flags.rs`)。読み手(jcc・setcc・cmovcc)が 1 つの定義だけを見るなら、定義が**ローカルに保存したオペランドから直接条件を作る**(`cmp a,b; jl` → `i32.lt_s`)。複数の定義が届くなら定義側が条件を具体化したローカルを読む。読まれないフラグは何も生成しない。種類: Add・Sub・Logic・Inc/Dec・Neg・Adc/Sbb・即値カウントの Shl/Shr/Sar・BitTest・rep 文字列比較。Cmp+Jcc の既定では、C 経路の遅延フラグ(`fle_op_*`・`fle_cc`・大域 `FL`)に相当するものはない。**Ucomis*/Comis* の旗は未対応**で、実行される関数の対応できない原因の 2 位・3 位(下)。

### メモリ(検査 4 方式の実装)

identity map のため、`rsp` 基準以外のアクセスだけが上位 32 ビット(63:32)の検査を要する(x86-64 のレジスタは 64 ビットで、wasm32 は下位 32 ビットで切り詰めるため)。モード: **Exact**(既定。C 経路の `fle_addr` と同じ意味: 実効アドレス全体の 63:32 が 0)、**Base**(基底レジスタだけ検査)、**Elim**(基底レジスタを 1 度検査したら、書き換えるまで再検査しない。インデックスつきは全体を検査)、**なし**。wasm32 の範囲外アクセスは V8 のガードページが処理する(検査は無料)。C 経路の `FLE_CHECKED_MEM`(領域外アドレスの検査)に相当する検査は**ない**(低位アドレスの NULL 参照は wasm の低位メモリを読む)。

### 制御フロー

Ramsey の構造化(`cfg.rs`: `block`/`loop`/`if`/`br`/`br_table`)。**既約でない関数**(freetype で 6)は `br_table` のディスパッチループにする。ジャンプテーブルはスロットが 8 バイト刻みで連続なら `br_table`、そうでなければ比較の列。

### 呼び出し

- **直接呼び**: wasm の `call`。引数は署名の `inr`、戻りは `out`(multivalue)。
- **間接呼び**: 表(`call_indirect`)の添字 = 目標アドレス − 最初のコードアドレス。要素は**アダプタ**(型 `(rsp, rcx, rdx, r8, r9, rax) → rax`)。アダプタを持つのは、入口で読むレジスタが引数レジスタ(と rax)だけで、callee-saved を保存する関数。`rax` を第 6 引数にしたのは、`sete al` のように rax の下位だけを書く関数が呼び元の rax に合流できるようにするため(差分テストの実バグ 2 件目の修正)。freetype の既定で 424 関数がアダプタを持ち、1,215 関数は持たない(内部規約の関数。callee-saved を『データとして読む』 setjmp 経路などで署名が引数レジスタを超える)。`--adapt-all` は、その他のレジスタに 0 を渡すテスト用の選択。
- **import**: IAT の呼びは wasm の import `imp_<DLL!Name>(rsp, rcx, rdx, r8, r9) → rax`(freetype で 74 個)。スタック引数は先頭の `rsp` から読める。

### SSE / MXCSR / x87(最小限)

XMM は v128 のローカル(制限: 関数の入口で live な XMM と、call をまたぐ XMM は未対応)。スカラー double/float の四則・sqrt・変換(`cvttsd2si` の整数不定値は host 関数)、packed float の加減乗(K1)など。**MXCSR は既定では持たない**(丸め・例外旗は常に既定として扱う)。`--mx-window` は fplow1 の窓の規則(オペランドが float 2^-62..2^62・double 2^-510..2^510 の正規数で MXCSR が既定なら host の演算、そうでなければ従来のコード=host 関数 `mx_slow_f64`)を**素朴な形**で入れたもの。x87・AVX・整数 SIMD の大半(Pshufd・Paddd・Punpck…)・Movd/Movq・Cvtsi2ss は**未対応**。

### 混成(扱えない関数を C 経路に残す)

PM の指示(2)に応えて試作した。生成部の `--hybrid-r ADDR` で有効にする。

- **関数の三分類**: (a) 新で lower する(自分の命令がすべて対応でき、呼び先に opaque がない)、(b) ext = 解析はできるが lower しない(未対応命令、または opaque を呼ぶ)→ C 経路、(c) opaque = 何も分からない(デコード不能・RSP が入口値で戻らない)→ C 経路。署名の導出は(a)(b)の両方に対して行う(ext の署名は命令の読み書きから出る)。
- **境界**: C 経路の呼び規約は『`R[4] -= 8; [R[4]] = 戻り先; f_X(); R[4] += 8`、引数・戻りは大域 `R[16]`』。**新 → C(スタブ)**: 呼び元が戻りアドレスを積み、`R[4]` と ext の `inr` のレジスタを `R[]` に書き、void の import `cf_<va>`(C の `f_<va>`)を呼び、`out` のレジスタを `R[]` から読み戻す。**C → 新(ラッパ)**: 新モジュールが各 lower 済み関数 `f_Y` の void ラッパ `w_Y` を export し、C 側の `f_Y` import(`fle_dispatch_table` の要素も同じ)につなぐ。ラッパは `R[4]` と `inr` を読み、呼び、`out` を `R[]` に書く。**さらに入口の rsp と callee-saved 8 本(rbx・rbp・rsi・rdi・r12–r15)を保存して最後に `R[]` へ書き戻す**(新の関数内のスタブ呼びが `R[]` に一時的な値を書き、それが C の呼び元の `R[]` を壊すため。無作為分割のテストで見つけた実バグ)。**C 側から呼ばれる関数は変更した全レジスタを返す**(C 側の読み手が分からないため)。間接呼びの表には ext 用のアダプタ(`R[]` への往復)を入れる。`R[]` のアドレスは生成時に焼き込む(C をリンクして `R` の値を読み、新を生成する 2 段)。
- **callee-saved・スタック・フラグ**: callee-saved は両側で保存される(新は証明つき、C は push/pop)ので境界で受け渡さない。スタックは共有のゲストスタック。フラグは呼びをまたいで live にしない(live な関数は ext)。
- **テストと結果**: 本文『FreeType 公開 API』と『混成の速度』。実行される関数集合に対する混成の対応率は『実行される関数集合に対する対応率』。

### setjmp / longjmp / SEH(設計のみ。未実装)

MSVC の `_setjmp` は機械語としては『レジスタ(buf の順に Frame=rdx・rbp・rsi・rdi・r12–r15・戻り先・MXCSR・FPCSR・xmm6–15。ダンプで読んだ並び)を buf に書く関数』で、`longjmp` は buf から復元して**保存した戻り先へ jmp する**(枠の途中に入る)。新バックエンドは (i) `_setjmp` の呼び元を **wasm の例外処理(`try`/`catch`)**で包み、longjmp を `throw` に置き換える、(ii) 呼び元の callee-saved は呼び元自身のローカルが残るので、buf への保存は不要、という形を考えている(**未実装・未確認**)。いまの試作では `_setjmp` と longjmp(opaque)は C 経路に残し、**呼び元は新で lower した**ので、buf の callee-saved の保存値がテスト用アダプタの 0 になり得る(5 バイトの差)。SEH(`try/except`)は C 経路の `FLE_LANDING_FORCE` + SJLJ=wasm の仕組みに任せるのが妥当と読む(**未確認**)。

## 結果

### 5 モジュールの census(全関数、実測)

`fe-wasmgen --census` で、本番と同じ seeds(`lift/harness/seeds/*`)+ pdata の全関数を対象にした。『純粋』= 関数と呼び先がすべて lower できる。『混成』= 自分の命令が対応でき、opaque を呼ばない関数(残りは C 経路)。生成時間は `User time`(共有機、負荷 7〜25)。

| module | functions_total | pure_ok | pure_ok_pct | insns_total | pure_ok_insns_pct | unbalanced | gen_cpu_s | max_rss_mb | hybrid_lowered | hybrid_lowered_pct | hybrid_gen_cpu_s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| flengine | 59305 | 21963 | 37.0 | 2528838 | 14.9 | 0 | 10.47 | 884 | 54910 | 92.6 | 83.96 |
| dsp | 40259 | 1527 | 3.8 | 6845864 | 1.2 | 3756 | 11.75 | 1946 | 3919 | 9.7 | 74.15 |
| blend2d | 3093 | 788 | 25.5 | 1186705 | 5.3 | 176 | 3.47 | 233 | 1508 | 48.8 | 5.83 |
| qfc | 6712 | 3117 | 46.4 | 209289 | 24.9 | 0 | 1.55 | 85 | 6371 | 94.9 | 6.55 |
| freetype | 1747 | 1639 | 93.8 | 112274 | 89.6 | 65 | 2.07 | 48 | 1664 | 95.2 | 1.80 |
| iltools | 12930 | 5872 | 45.4 | 404806 | 23.4 | 14 | 2.87 | 160 | 12087 | 93.5 | 12.57 |

- dsp の純粋 3.8%・混成 9.7% は、関数の大半が AVX(Vmovups 11.5 万など)・Movd・Cvtsi2ss・Shufps を含み、さらに **3,756 関数が『RSP が入口値で戻らない』**(`mov rsp, rbp` 型の枠の復元などを私の RSP 追跡が扱えない。読み)ためで、実行される 335 関数は別の分布(次節)。
- 混成の生成時間が長い(FLEngine で 84 s、純粋の 10.5 s の約 8 倍)のは、C 経路に残す関数も署名の導出に含めるため(**読み**。最適化していない)。
- 本番の seeds にはフックの指定(`blend2d-no-jit-sites.txt`、`iltools-hook-points.txt`)もあるが、census では使っていない(**未確認**: これらの指定が lower 率を変えるか)。

### 実行される関数集合に対する対応率(PM の指示(1)、実測)

size1 の `lift/out-size1/tables/executed-functions.tsv`(on-seh FRUITY 構成の native -O0 を cmp-short.g3 の 16 スナップで走らせて関数の入口を数えたもの。12,298 関数 = 生成 C の 11.0%、-Oz 本体の 9.1%。プロジェクトを開く等は含まない)の各 VA を census の関数表に突き合わせた(12,290 が見つかり、見つからない 8 は seeds にない入口)。

| module | executed | direct_ok | pure_ok | direct_ok_pct | pure_ok_pct | direct_ok_insn_pct | pure_ok_insn_pct | direct_ok_entries_pct | pure_ok_entries_pct |
|---|---|---|---|---|---|---|---|---|---|
| flengine | 8772 | 8021 | 3658 | 91.4 | 41.7 | 59.9 | 12.2 | 81.3 | 48.9 |
| dsp | 335 | 243 | 68 | 72.5 | 20.3 | 28.4 | 5.2 | 42.0 | 3.2 |
| blend2d | 329 | 206 | 84 | 62.6 | 25.5 | 7.2 | 2.2 | 44.9 | 28.3 |
| qfc | 1119 | 999 | 605 | 89.3 | 54.1 | 79.1 | 19.9 | 73.8 | 52.8 |
| freetype | 376 | 375 | 370 | 99.7 | 98.4 | 99.9 | 95.4 | 100.0 | 100.0 |
| iltools | 1367 | 1280 | 665 | 93.6 | 48.6 | 83.8 | 18.1 | 83.5 | 64.0 |
| TOTAL | 12298 | 11124 | 5450 | 90.5 | 44.3 | 52.1 | 13.2 | 77.4 | 47.5 |

- `direct_ok` = 自分の命令だけで対応できる(混成で必要な最小条件)。`pure_ok` = 呼び先すべてを含めて新バックエンドだけで閉じる。`insn` は size1 の `emitted_native_insns` で、`entries` は入口の回数(83,886,080 入口の dump 時点)で重みづけた率。
- **混成モードで実際に lower される率**(opaque を呼ぶ関数が C 経路に落ちる分だけ `direct_ok` より低い)。

| module | executed | lowered | lowered_pct | insns_lowered_pct | entries_lowered_pct | not_found | stays_on_c_path | of_which_opaque_or_unbalanced | of_which_unsupported_insn_or_flag | of_which_calls_opaque |
|---|---|---|---|---|---|---|---|---|---|---|
| flengine | 8772 | 7777 | 88.7 | 55.2 | 80.9 | 6 | 989 | 2 | 744 | 243 |
| dsp | 335 | 206 | 61.5 | 16.4 | 34.7 | 0 | 129 | 28 | 77 | 24 |
| blend2d | 329 | 198 | 60.2 | 6.9 | 42.5 | 0 | 131 | 11 | 112 | 8 |
| qfc | 1119 | 984 | 87.9 | 77.6 | 68.2 | 1 | 134 | 2 | 118 | 14 |
| freetype | 376 | 375 | 99.7 | 99.9 | 100.0 | 0 | 1 | 0 | 1 | 0 |
| iltools | 1367 | 1268 | 92.8 | 82.9 | 83.4 | 1 | 98 | 2 | 85 | 11 |
| TOTAL | 12298 | 10808 | 87.9 | 48.4 | 75.9 |  |  |  |  |  |

関数数では 87.9%、入口回数では 75.9% が新バックエンドで動くが、**命令数では 48.4%**(大きな関数ほど未対応の命令を含む)。FLEngine は関数 88.7%・入口 80.9%・命令 55.2%。dsp(AVX が多い)は関数 61.5%・入口 34.7%・命令 16.4%、Blend2D は関数 60.2%・入口 42.5%・命令 6.9%。

### 対応できない原因(上位 20、実行される関数のうち、その形を直接持つ関数の数)

『唯一の原因』は、その関数の対応できない理由がその 1 つだけの関数の数(それを実装すればその関数が増える)。

| reason | executed_functions_with_reason | sole_reason | entries_in_those_functions |
|---|---|---|---|
| an XMM register is live into the function | 227 | 227 | 3743012 |
| flag read from a definition without recipe: Ucomiss Opaque flag N | 206 | 206 | 681720 |
| flag read from a definition without recipe: Ucomisd Opaque flag N | 181 | 181 | 1191232 |
| mnemonic Cvtsi2ss | 119 | 115 | 110552 |
| mnemonic Xadd | 108 | 95 | 2777887 |
| mnemonic Movd | 44 | 3 | 2481226 |
| flag read from a definition without recipe: Comisd Opaque flag N | 37 | 37 | 625438 |
| an XMM register is live across a call | 37 | 37 | 74073 |
| mnemonic Cmpxchg | 35 | 32 | 464947 |
| mnemonic Xchg | 30 | 29 | 136813 |
| mnemonic Shufps | 25 | 7 | 833983 |
| mnemonic Cpuid | 24 | 13 | 76513 |
| mnemonic Movq | 23 | 1 | 508918 |
| mnemonic Punpcklbw | 23 | 0 | 1039203 |
| bit test on memory with a register bit offset | 20 | 16 | 3052 |
| mnemonic Pshufd | 18 | 0 | 1362106 |
| mnemonic Packuswb | 17 | 0 | 1014336 |
| Imul 1-byte | 14 | 10 | 30 |
| mnemonic Bsf | 14 | 8 | 116427 |
| mnemonic Pshuflw | 13 | 0 | 818573 |

読み: 上位は**実装が小さいもの**が多い。(1) XMM 引数(Win64 の浮動小数点引数 xmm0–3)を署名に入れる(v128 の追加パラメータ)。(2) `Ucomis*`/`Comis*` の旗(ZF・PF・CF)から `ja/jb/je/jp` 等の条件を作る。(3) `Cvtsi2ss`。(4) `Xadd`/`Cmpxchg`/`Xchg`(ロックつきならば wasm の `atomic.rmw` が必要で、メモリを shared にする)。(5) `Cpuid`(host 関数)、`Bsf/Bsr`(`ctz/clz`)。x87(`Fld/Fmul/Fstp…`)は実行集合の上位には出ない(FLEngine の census 全体では多い)。dsp は AVX が支配的で、実行集合でも Movntdq・Shufps が出る。

### 上位の原因を実装すると率がどう上がるか(推定。census の関数単位の再集計で、実装はしていない)

貪欲に、実行される関数をもっとも増やす原因を 1 つずつ『実装したことにして』再集計した(top-k = 上位 k 個の原因を実装した後の率、%)。純粋は呼び先を含む閉包で再計算した。

| module | mode | top0 | top1 | top2 | top3 | top5 | top8 | top12 | first_5_reasons_implemented |
|---|---|---|---|---|---|---|---|---|---|
| flengine | pure (whole call tree) | 41.7 | 48.4 | 49.7 | 50.6 | 51.7 | 52.7 | 53.0 | Xchg; Xadd; Cmpxchg; XMM live into the function; flag:Ucomiss |
| flengine | hybrid (function alone) | 91.4 | 93.5 | 95.2 | 96.8 | 98.5 | 99.1 | 99.4 | flag:Ucomiss; XMM live into the function; flag:Ucomisd; Cvtsi2ss; Xadd |
| dsp | pure (whole call tree) | 20.3 | 27.8 | 29.9 | 31.9 | 34.9 | 40.0 | 40.9 | Xadd; Shufps; flag:Comisd; terminator: undecodable instruction or falls out at N; Cpuid |
| dsp | hybrid (function alone) | 72.5 | 76.1 | 78.5 | 80.3 | 83.3 | 87.2 | 89.9 | flag:Comisd; terminator: undecodable instruction or falls out at N; XMM live into the function; Cpuid; Xadd |
| blend2d | pure (whole call tree) | 25.5 | 31.0 | 48.3 | 51.1 | 58.1 | 67.5 | 69.0 | Xchg; Xadd; XMM live into the function; Bsr; flag:Comisd |
| blend2d | hybrid (function alone) | 62.6 | 69.3 | 76.0 | 82.7 | 87.5 | 91.5 | 94.5 | Xadd; XMM live into the function; flag:Comisd; Bsf; flag:Ucomisd |
| qfc | pure (whole call tree) | 54.1 | 62.4 | 65.2 | 68.5 | 71.5 | 73.4 | 74.4 | Xchg; XMM live into the function; flag:Ucomisd; Xadd; Cmpxchg |
| qfc | hybrid (function alone) | 89.3 | 92.7 | 95.1 | 96.2 | 97.6 | 98.5 | 99.2 | XMM live into the function; flag:Ucomisd; Xadd; Cmpxchg; flag:Ucomiss |
| freetype | pure (whole call tree) | 98.4 | 99.2 | 99.2 | 99.2 | 99.2 | 99.2 | 99.2 | XMM live into the function |
| freetype | hybrid (function alone) | 99.7 | 100.0 | 100.0 | 100.0 | 100.0 | 100.0 | 100.0 | XMM live into the function |
| iltools | pure (whole call tree) | 48.6 | 56.5 | 58.6 | 59.4 | 61.1 | 62.5 | 63.8 | Xchg; Xadd; XMM live into the function; Cmpxchg; flag:Cmpxchg |
| iltools | hybrid (function alone) | 93.6 | 94.7 | 95.6 | 96.5 | 97.6 | 98.7 | 99.2 | Xadd; XMM live into the function; flag:Ucomiss; Cmpxchg; XMM live across a call |

読み: **混成なら上位 5 個の実装で FLEngine 98.5%・QFC 97.6%・ILTools 97.6%**。純粋は呼び先の尾が長く、上位 12 個を入れても FLEngine 53%・ILTools 64%・QFC 74% で止まる(残りは Movd/Movq・x87・SSE の source-register 形・SEH など多数の小さな原因)。したがって**混成を前提にするのが現実的**で、そのときの優先は上の(1)(2)(3)(4)。

### サイズ(実測)

freetype の 1,639 関数(本体のみ、native の命令バイト 372,686 B、100,611 命令)。C 経路は同じ seeds で作り直した(`lift/out-wasmgen1/c2/`)。『wasm-opt 後』は `wasm-opt -Oz -g`(名前節を残して関数ごとのバイト数を取る)で、**wasm-opt が併合した関数が欠ける**ため該当行は少し低い(欠け数を併記)。

| 構成 | バイト | native 比 | B/insn | 欠け |
|---|---|---|---|---|
| C 経路 -Oz(リンク後) | 2,005,261 | 5.38× | 19.9 | 0 |
| C 経路 -Oz + wasm-opt -Oz | 1,939,745 | 5.20× | 19.3 | 5 |
| C 経路 -O2(リンク後) | 5,095,743 | 13.67× | 50.6 | 0 |
| C 経路 -O2 + wasm-opt -Oz | 4,374,076 | 11.74× | 43.5 | 7 |
| **新(emit のまま、Exact)** | 1,305,138 | 3.50× | 13.0 | 0 |
| **新 + wasm-opt -Oz(Exact)** | **1,015,979** | **2.73×** | **10.1** | 22 |
| 新 + wasm-opt、Base | 925,820 | 2.48× | 9.2 | 22 |
| 新 + wasm-opt、Elim | 876,328 | 2.35× | 8.7 | 22 |
| 新 + wasm-opt、check なし | 757,821 | 2.03× | 7.5 | 22 |
| 新 + wasm-opt、退避の省略なし | 1,082,290 | 2.90× | 10.8 | 20 |
| 新 + wasm-opt、check なし + 省略なし | 825,467 | 2.21× | 8.2 | 20 |

- モジュール全体(名前節つき、wasm-opt -Oz、`--export-funcs`): 新 1,086,843 B、C -Oz 2,248,043 B(`--compress-relocations` + wasm-opt -Oz 後。リンク直後の `--compress-relocations` 版は 2,364,292 B)、元の DLL 607,032 B。圧縮後(freetype 全体、名前節つきの wasm-opt 後): xz -9 で新 288,312 B、C -Oz 290,760 B、C -O2 520,944 B、元の DLL 264,920 B。gzip -9 で新 375,943、C -Oz 410,192、C -O2 811,555、DLL 313,099。**C -Oz とは圧縮後にほぼ同じ**(繰り返しの多い C 経路が圧縮で縮むため。配信サイズ=圧縮後に新バックエンドの利点は小さいと読む)。brotli は未インストールで未測定。
- **実行される関数集合での実測(混成で lower される 10,808 関数、emit のまま・wasm-opt なし)**: size1 の『-Oz の wasm 関数本体バイト』と、同じ関数の新の本体バイトを比べた。新は C -Oz の **0.60 倍**(7,001,545 B 対 11,635,060 B。freetype 0.51、Blend2D 0.51、dsp 0.56、QFC 0.60、FLEngine 0.61、ILTools 0.61)。native の命令バイトに対しては 4.28 倍(emit のまま。wasm-opt で約 2 割減ると読む)。lower される関数は実行集合の C -Oz 本体 28.8 MB の 40.4%(FLEngine 49.4%、QFC 73.6%、ILTools 79.1%、freetype 99.8%、dsp 11.1%、Blend2D 4.4%)なので、**実行集合 12,298 関数を混成にした場合の本体は 約 24.2 MB(新 7.0 + C 17.2)で、C 経路 28.8 MB の約 0.84 倍**(**計算**)。

| module | lowered_executed | c_oz_bytes_of_those | new_emit_bytes_of_those | ratio_new_over_c_oz | measured_share_pct |
|---|---|---|---|---|---|
| flengine | 7777 | 8304448 | 5080309 | 0.61 | 49.4 |
| dsp | 206 | 170758 | 95506 | 0.56 | 11.1 |
| blend2d | 198 | 302533 | 153591 | 0.51 | 4.4 |
| qfc | 984 | 1143498 | 685287 | 0.60 | 73.6 |
| freetype | 375 | 641119 | 327864 | 0.51 | 99.8 |
| iltools | 1268 | 1072704 | 658988 | 0.61 | 79.1 |
| TOTAL | 10808 | 11635060 | 7001545 | 0.60 | 40.4 |

- **S1(2.0× / 1.2×)のサイズ側**: 既定(Exact)は 2.73× で届かない。2.0× に最も近いのは『check なし』の 2.03×(それでも届かない)。Elim で 2.35×。→ **S1 のサイズは安全側の構成では届かない**。速度側(1.2×)は整数で届き(K3 0.98×、K5 1.11×、K6 1.06〜1.21×)、SSE では届かない(K1 1.70×、K2 2.05×、K4 1.49×)。
- **fe-lift の共有本体の重複(`--promote-shared`、実測)**: 共有の尾部を関数に昇格する 29 個で、閉包の命令数が 112,274 → 105,182(−6.3%)、unbalanced が 65 → 3、対応する命令の割合が 89.6% → 95.7%。モジュールのバイト数は +98 B(変わらない)。つまり**重複は新バックエンドのサイズにほぼ効かない**が、継続入口を関数にできるので対応率を上げる(本番では promote-shared を既定にするのが妥当と読む)。

### 速度(実測、CPU 秒の最小値、K1〜K6)

カーネルは `lift/out-wasmgen1/kbench/`(K1–K3 は `docs/investigations/vst3-hosting/tools/microbench/` 由来、K4 sqrt/div/cvt、K5 挿入ソート 256 要素、K6 fib)。native は同じカーネルを gcc で作った実行ファイル。結果は `kbench/kbench-final.txt`(3 ラウンド)から最小値を取った。

| 構成 | K1 gain+acc packed (ns/sample) | K2 biquad scalar double (ns/sample) | K3 xorshift int (ns/sample) | K4 sqrt/div/cvt scalar (ns/sample) | K5 insertion sort 256 (µs) | K6 fib(32) (ms) |
|---|---|---|---|---|---|---|
| native(gcc -O2、最小) | 0.057 | 2 | 1.25 | 2.49 | 7.21 | 6.6 |
| 新・check Exact(MXCSR なし) | 0.097 (1.70x) | 4.1 (2.05x) | 1.22 (0.98x) | 3.71 (1.49x) | 8 (1.11x) | 8 (1.21x) |
| 新・Base | 0.097 (1.70x) | 4.15 (2.08x) | 1.27 (1.02x) | 3.91 (1.57x) | 8 (1.11x) | 7 (1.06x) |
| 新・Elim | 0.097 (1.70x) | 4.2 (2.10x) | 1.27 (1.02x) | 4.12 (1.66x) | 8.62 (1.20x) | 7 (1.06x) |
| 新・check なし | 0.097 (1.70x) | 4.15 (2.08x) | 1.27 (1.02x) | 3.86 (1.55x) | 4.65 (0.64x) | 7 (1.06x) |
| 新・Exact + MXCSR の窓(ゼロを含むデータ) | 0.098 (1.72x) | 13.3 (6.67x) | 1.27 (1.02x) | 10.6 (4.27x) | 9.33 (1.29x) | 7 (1.06x) |
| 新・Exact + MXCSR の窓(ゼロなし) | 0.097 (1.70x) | 11.5 (5.76x) | 1.27 (1.02x) | 9.77 (3.93x) | — | — |
| C 経路 -O2 | 8.54 (149.89x) | 33.2 (16.60x) | 12.5 (9.99x) | 29.3 (11.78x) | 182 (25.24x) | 87 (13.18x) |
| C 経路 -Oz | 12 (209.74x) | 68.8 (34.44x) | 38.3 (30.76x) | 55.7 (22.38x) | 277 (38.47x) | 187 (28.33x) |

括弧は native の最小に対する比。K5・K6 は測定スクリプトが整数に丸めて出す(K5 は µs、K6 は ms)ので比は粗い。負荷平均 12〜13(開始時 12.3、終了時 12.9)、3 ラウンドの最小。native の 3 ラウンドの範囲: K1 0.057〜0.081、K2 2.00〜2.12、K3 1.25〜1.33、K4 2.49〜2.59、K5 7.2〜8.7、K6 6.6〜7.6。

- 出力の一致: K1〜K4 を同じ入力に 1 回ずつ呼んだ出力の md5 は、C 経路 -O2・-Oz と新の全構成(Exact・Base・Elim・なし・窓)で**一致**(`kbench/check.cjs`。窓つきはゼロを含むデータで slow-path 113 回、含まないデータで 2 回)。K5・K6 は整列済み・fib=6765 の検査のみ。繰り返し実行中の `sink` は呼び回数が構成ごとに違うので比べていない。
- PM の go/no-go(整数 ≤2×、スカラー ≤4×、packed ≤6×): 既定(MXCSR なし)はすべて満たす。**窓つきの K2(スカラー double)は 5.8〜6.7×、K4 は 3.9〜4.3× で、4× を超えるか境界**。窓の判定は素朴(演算ごとに指数部の検査 + 大域 `mx_ok`)で、ブロック単位の検査・ループ外への巻き上げは未実施。
- **K2 の忠実度(MXCSR なし対 窓)**: 上の一致は MXCSR が既定(0x1f80)でデータが正規数の場合の実測。**読み**: MXCSR なしの既定は、wasm の IEEE 演算をそのまま使うので、エンジンの実際の MXCSR(0x9fe0 など = FTZ・DAZ が立つ、決定 0143)の**非正規数の扱い・丸めモード・例外旗を無視する**。つまり非正規数が入力か結果に出るコードでは C 経路と違い得る(未実測)。窓つきは、オペランドが float 2^-62..2^62・double 2^-510..2^510 の正規数で MXCSR が既定のときだけ host の演算を使い、それ以外は従来のコード(host 関数)に回るので、窓の条件下では FTZ・DAZ の影響を受けない。ただし**窓つきも PE(不正確)旗は更新しない**(PE は sticky でマスクされているという 0143 の前提に依存)。コストは K2 で 2.05× → 5.8〜6.7×(演算ごとの指数部の検査 + `mx_ok`)。
- **関数単位の速度(freetype の 150 関数、40 命令以上、同じ状態で繰り返し呼んだ CPU 時間。C -O2 / 新)**: **測定は無効**(参考値のみ): `t_<va>` アダプタ(16 引数・16 戻り値の BigInt)の JS → wasm 呼びが 1 µs 前後かかり、100〜200 ns の関数では呼びの費用が支配する(C 経路の `f_<va>()` は引数なし)。60 関数のうち 48 関数で数値が出て、C -O2 / 新 の比は中央値 0.7、時間で重みづけて 2.1、最大 12.0 — 関数の実行時間ではなく呼びの形を測った値なので、速度の根拠には使わない(`tables/bench-functions-INVALID-call-overhead.tsv`)。代わりに上の K 系と、下の混成の FreeType(loadglyph + render)の測定を使う。wasm 側にループを持つテスト用 export を作って測り直すのは**未実施**
- 速度の読み: C 経路は R[] のメモリ往復と遅延フラグが毎命令にあるため、新との差は **7〜150×**。一方 native との差は整数で 1.0〜1.2×。**『C 経路の遅さ』が主因で、新が native 並みになる**と読める。ただし K 系は小さなカーネルで、**実エンジンでの速度は未測定**。

### 差分テストの内訳(実測)

`tools/difftest.cjs`: 関数ごとに新しいインスタンスを作り、ランダムなレジスタ・scratch・スタック・.data で C 経路(`f_<va>`)と新(アダプタ)を呼び、rax・callee-saved・rsp・scratch・スタック引数・.data・低位 64 KB・import 呼びの列を比べる(両方が落ちるのは一致)。比較が食い違った反復は (a) C 経路の配置を変えた 2 本目の C(`--c2`)と食い違うなら C 側の配置依存(C のデータもゲストメモリに載っており、野良書込で `R[]`・dispatch 表が壊れる)、(b) まっさらなメモリで再実行して分類が変わるなら前反復の残骸、で除外する。

| 項目 | 値 |
|---|---|
| 対象 | freetype の lower 済み 1,639 関数 × 300 反復(ビルド c3b: テスト用アダプタつき) |
| 反復数 | 482,464(hang の 38 関数は完了分のみ) |
| 両方が落ちる(一致) | 241,210(50.0%) |
| **不一致** | **0** |
| **新だけが落ちる** | **0**(残骸で除外 2: `0x4b06fd0` の `unreachable`、`0x4b33150` の範囲外) |
| C だけが落ちる | 2,505(10 関数。すべて C のテスト環境の `fprintf` stub = 未モデルの IAT import で C が abort し、新は import 呼びをそのまま lower する) |
| C 経路の配置依存で除外 | 728 反復(39 関数。多い順に `0x4b2d770` 86、`0x4b57cd0` 79、`0x4b2d810` 79、`0x4b2d940` 74、`0x4b2f8d0` 59、`0x4b2f0d0` 59、`0x4b2d9d0` 59、`0x4b56380` 52、`0x4b20870` 36(adler32)、`0x4b01fb0` 36) |
| hang(4 秒の監視で未完) | 38 関数。1 回前のビルド(36 関数)で C だけ・新だけを別々に走らせた結果: C 単独で hang 35、新単独で hang 22(うち 20 は C と同じ反復)、C だけが 4 秒を超えたもの 13(C は新より遅い)。ランダム入力が暴走ループを作る |
| 実バグ(差分テストが見つけた) | 2 件(別に、混成の境界の実バグ 2 件を無作為分割のテストで見つけた。下の節)。(1) `rep scas/cmps` の範囲の事前検査が早期脱出のループで誤って落ちた → 要素ごとの検査にした。(2) 間接呼びの先が `sete al` で rax の下位 8 ビットだけを書く関数のとき、アダプタが rax を渡さず 0 を返した(`0x4b14d40`、1 反復)→ アダプタに rax を第 6 引数で渡し、callee が rax を書かないときは入力 rax を返す |
| 最初のフル実行(2 本目の C なし)の内訳 | 不一致 148 反復(9 関数)と新だけが落ちる 31 反復(5 関数)。これを出した 12 関数に 2 本目の C と残骸の再実行を足して再実行すると、不一致は 1(`0x4b14d40`、上の(2))、新だけが落ちるは 10 → 残骸で 0、配置依存 238 反復。その後の修正ビルドで全 1,639 関数を再実行したのが上の表 |
| テスト環境側の修正(新バックエンドのバグではない) | 野良書込で C のデータが壊れる(`R[]`・dispatch 表)ため、2 本目の C を別の配置(`--global-base=0x5200000`)で走らせる検査を足した。トラップ後に C の `__stack_pointer` が戻らない問題と import の連番のリセットを直した。『まっさらなメモリでの再実行』を足した |
| check Elim で同じテスト(ビルド c3e) | 483,067 反復。不一致 0、**新だけが落ちる 16,829(744 関数)**。すべて 63:32 検査の `unreachable`(負の基底 + 変位が 4 GiB 未満に戻る場合を、C 経路の `fle_addr` は通し、Elim は落とす。Exact では 0)。C だけ 2,505、配置依存 804、残骸 2、hang 36 |

読み: **不一致 0 は、freetype 1 モジュール・ランダム入力(ポインタは scratch 領域に寄せる)・上の除外つきの値**で、これ自体は FLEngine の正しさを示さない。比較しているのは C 経路(lift した機械語の C)との一致で、実機との一致ではない(**実機との照合はしていない**)。また除外された 728 反復はその分だけ比較できていない。**両方が落ちる反復(241,210)は『落ちた』という一致だけで値は比べていない**ので、値まで比べた反復は 482,464 − 241,210 − 2,505 − 728 − 2 ≈ **238,019**(約 24 万)。

### FreeType 公開 API(実測)

`tools/ftapi.cjs`: 公開 API を順に呼び、各戻り値と import 呼びの列とヒープ全体を比べる。DejaVu Sans(`/snap/gnome-46-2404/…/DejaVuSans.ttf`)、メモリ上の face、グリフごとに `FT_Get_Char_Index`・`FT_Load_Glyph`・`FT_Render_Glyph`。import は HLE の最小実装(`HeapAlloc` は bump 割り当て)。

| 構成 | 結果 |
|---|---|
| 純粋な新 vs C 経路: `FT_Init_FreeType` + `FT_Done_FreeType` | 戻り値一致、import 76 回一致、ヒープ 22,256 B 一致 |
| 純粋な新: `FT_New_Memory_Face` 以降 | **不可**(`_setjmp` が XMM 入口 live で未対応。呼び元ごと lower できない) |
| **混成 vs C 経路: 4 グリフ(A g W @、48 px)** | 戻り値一致。ヒープ(69,888 B)の API 呼びごとのダイジェストは `init` `newface` `setpx` `charidx` `loadglyph A` まで**一致**、`render A` で 4 バイト差(下) |
| **混成 vs C 経路: 65 グリフ(英数字 + &%@、17 px)** | 戻り値一致、import 301 回(引数 3 つ)一致、ヒープ 75,296 B のうち **5 バイトだけ違う** |
| 混成 vs C 経路: 16 グリフ × 60 回の loadglyph + render | 戻り値一致、import 2,116 回一致、ヒープ 287,792 B のうち 5 バイト違い |
| `wasm-merge`で 1 モジュールにした混成(同上 16 グリフ) | 同じ 5 バイト差。戻り値・import 一致 |

5 バイトの差の原因(実測 + 読み): 差は `0x6000720`〜`0x6000738`(rasterizer の `jmp_buf`、`ras + 0x530`。Frame・Rbp・Rsi・Rdi・R12・R13・R14・R15・Rip の順)の R12(+0x20)・R13(+0x28)・R15(+0x38)の保存値に当たる(C 経路: `0x600c790`・`0x23`・`0xc0`、混成: 0)。`_setjmp` の呼び元(`0x4b25a20`)は新が lower し、それを間接呼び(`renderer->raster_render`)で呼ぶテスト用アダプタ(`--adapt-all`)が r12・r13・r15 に 0 を渡すため、jmp_buf に 0 が入る。C 経路は実際のレジスタ値を保存する。**longjmp が起きなければこの値は読まれない**(読み)。longjmp 自体は両経路とも実行していない。境界の往復は new → C が 65 グリフで 3,507 回(`gray_find_cell` `0x4b24af0` と `_setjmp` `0x4b64320`)、C → new は 0 回。

### 混成の境界のテスト: 無作為分割(実測)

freetype で『全部新』の混成は C → 新の往復が 0 回で、ラッパの経路を通らない。そこで、lower できる 1,664 関数の無作為な 30%・50%・80% だけを新にし、残りを C 経路に残して(C 部分は `-O0` で 3 秒でビルド。50% 分割は `-O2` の C 部分でも同じ結果: 戻り値一致・import 301 回一致・ヒープ 2 バイト差・往復の回数も同じ)、同じ公開 API(65 グリフ)を C 経路単独と比べた(`tools/hybrid_try.sh`)。

| 新にした関数 | C → 新の往復 | 新 → C の往復 | 戻り値 | import 呼び(引数 3 つ) | ヒープの差 |
|---|---|---|---|---|---|
| 全部(1,664) | 0 | 3,507 | 一致 | 301 一致 | 5 バイト |
| 80%(1,349) | 9,617 | 17,602 | 一致 | 301 一致 | 5 バイト |
| 50%(843) | 11,373 | 19,284 | 一致 | 301 一致 | 2 バイト |
| 30%(494) | 7,909 | 11,375 | 一致 | 301 一致 | 0 バイト |

差のバイトは上と同じ jmp_buf の保存値(テスト用アダプタの 0)。**このテストが混成の境界の実バグを 2 件見つけた**(最初の 50% 分割は `FT_Init_FreeType` が誤った戻り値を返した): (1) ラッパが `R[4]`(RSP)を元に戻さず、新 → C のスタブが書いた『C 側の戻り先』の値が C の呼び元に残った(C の呼び規約は呼び元が `R[4] += 8` する)→ ラッパが入口の rsp を保存して最後に書き戻す。(2) 新の関数内のスタブ呼びが callee-saved レジスタの一時的な値を `R[]`(= C の呼び元の callee-saved)に書いて残した→ ラッパが入口で `R[]` の callee-saved 8 本を保存して最後に書き戻す(`FT_MulDiv` `0x4b117a0` で再現した)。原因の関数は、分割した集合を `tools/ddmin.py`(delta debugging)で 843 → 1 関数まで 17 回の試行(1 回約 10 秒)で絞った。読み: 無作為分割と ddmin の組み合わせは、境界のテストとして有効で、本番の混成でも各 DLL でこれを回すのが妥当。

### 混成の速度(実測、freetype、CPU 時間、負荷 8〜16)

`--repeat 60`(16 グリフ、`FT_Load_Glyph` + `FT_Render_Glyph`)の合計 CPU:

| 構成 | CPU 時間の最小(3 ラウンド) | C 経路単独に対する比(最小どうし) |
|---|---|---|
| C 経路単独(-O2、`c2/ft-O2-test.wasm`) | 307.2 ms | 1.0 |
| 混成(新 + C、JS の橋) | 96.3 ms | **3.2×** |
| 混成(`wasm-merge` で 1 モジュール) | 71.5 ms | **4.3×** |

同じ run 内の比(参考): 混成(JS)は 3.0〜3.4×、`wasm-merge` は 4.2〜4.5×。負荷平均は約 16(最終ビルドでの測り直し。1 回前の測定は負荷 10〜14 で 3.2× と 4.6×)。測定は `tools/ftapi.cjs --repeat 60`、`logs/hybrid-speed.txt`。この構成(全部新)は C → 新の往復が 0 回で、新 → C が 3,507 回。

読み: **JS を通る橋でも C 経路単独の 3.2×、wasm-merge で境界を wasm の直接呼びにすると 4.3×**(最小どうしの比)。純粋に新だけならさらに速いはず(この run の境界往復 3,507 回のうち大半が `gray_find_cell` で、これを lower できれば往復は消える。`gray_find_cell` は opaque な `longjmp` を呼ぶエラー経路を持つため C 側に残った)。

### 生成時間・メモリ・V8 のコンパイル(実測)

- 生成(freetype): 新 CPU 2.07 s・48 MB(`compile_ms` 1,569)。比較: fe-lift 0.25 s の後に emcc `-Oz` 32.8 s・661 MB、`-O2` 102 s・1.4 GB(負荷約 20 の共有機)。混成モード(C 部分 83 関数だけ emcc): `-O2` で 4.5 s・169 MB(`hy/`)。
- V8 のコンパイル(`node --no-wasm-lazy-compilation`、freetype、初回のみ。V8 は同一バイトの native module をキャッシュする):

| 構成 | バイト | Liftoff CPU | TurboFan CPU |
|---|---|---|---|
| C -Oz + wasm-opt | 2,248,043 | 0.061 s | 1.174 s |
| C -O2 + wasm-opt | 4,905,706 | 0.117 s | 3.181 s |
| 新(Exact)+ wasm-opt -Oz | 1,086,843 | 0.054 s | 1.537 s |
| 新(check なし)+ wasm-opt -Oz | 825,169 | 0.045 s | 1.291 s |

再測定(負荷 16、`tables/v8-compile-time.txt`): TurboFan CPU は C -Oz 1.28 s、C -O2 3.24 s、新 1.58 s、新(check なし)1.33 s で、順序は同じ。

読み: **バイトが半分でも TurboFan のコンパイルは C -Oz より遅い**(新は関数が大きく i64 ローカルが多く、C -Oz は小さな helper 呼びが多い)。-O2 の C と比べれば約半分。起動(P1: 10 秒)は『ダウンロードとコンパイル』と『初期化コードの実行』の合計で、実行が速くなる効果が見込めるが、**起動が 10 秒に入るかは未測定**(エンジン全体を動かしていない)。

## C 経路との比較

| 項目 | C 経路(fe-lift → emcc) | 新(fe-wasmgen) |
|---|---|---|
| 状態 | グローバル `R[16]`・遅延フラグ(メモリ) | wasm ローカル・署名はすべて導出 |
| サイズ(freetype、wasm-opt 後) | -Oz 5.20×、-O2 11.74× | **2.73×**(check なし 2.03×) |
| 速度(K 系、native 比) | -O2: 整数 10〜25×(K3 10×、K5 25×、K6 13×)、packed 150×、スカラー double 12〜17× | 整数 0.98〜1.21×、packed 1.70×、スカラー double 1.5〜2.1×(MXCSR なし) |
| 生成時間 | freetype -Oz 33 s、-O2 102 s(本番全体は約 10 分〜) | 2 s |
| 命令の対応 | すべて(x87・AVX・SEH・setjmp を含む) | 整数の大半 + 最小の SSE |
| 実機との照合 | 既存の G 段階の検証(別担当) | C 経路との差分テストのみ |
| V8 コンパイル(TurboFan CPU) | -Oz 1.17 s | 1.54 s |
| 重複した共有本体 | `--outline-shared` 等で扱う | `--promote-shared` で 6.3% の命令数減(サイズは不変) |

## wwt との比較(PM から受け取った数字。私は wwt を実行していない=未確認)

PM から受け取った wwt(公開の x86 → wasm 変換器)の対 native 比: K1 3.7〜4.9、K2 2.6〜2.8、K3 2.2〜2.7、fib(35) 4.6〜5.2、挿入ソート 14.5〜16.6、CRC32 2.0〜2.2。`-O 0 --mem64 --mem-traps` では K1 2.4、K2 1.9、K3 1.1、fib 4.0、挿入ソート 4.2、CRC32 1.0。freetype のサイズは 5.58 MB(9.2×)、`--no-mem-checks` で 3.85 MB。参考として emcc -O2 -msimd128 の C で書いた K1・K2・K3 の上限は 2.06×・1.53×・1.97×。

読み: 私の K 系(同じ形のカーネルを fe-wasmgen で作ったもの)は K1 1.70×、K2 2.05×、K3 0.98×、fib 1.06〜1.21×、挿入ソート 1.11×。サイズは freetype で 2.73×(wwt は 9.2×)。**条件が違う**(カーネルの作り、wwt のバージョン、ホスト、私は MXCSR なし)ので、優劣は主張しない。方式の差は 2 点: (1) wwt は x86 のレジスタ状態をメモリに置くと読むが、私の生成部はローカルに置き、署名を導出する。(2) 私の生成部は本プロジェクトの fe-decode の関数境界を前提にする(wwt は汎用)。


## メモリ検査の選択肢(PM の指示: 4 方式の表。一次資料つき)

背景: x86-64 のレジスタは 64 ビットで、ゲストの有効アドレスは 63:32 が 0 でなければ実機では落ちる。wasm32 のメモリは下位 32 ビットで切り詰めるので、**切り詰める前に 63:32 を検査しないと、実機なら落ちるアクセスが静かに別の場所に当たる**。wasm32 の範囲外アクセス自体は V8 のガードページが検査なしで処理する。

| 方式 | 内容 | サイズ(freetype、wasm-opt 後) | 速度(K5 挿入ソート、native 比) | 備考 |
|---|---|---|---|---|
| (a) 全アクセス(Exact) | 実効アドレス全体の 63:32 を検査。C 経路の `fle_addr` と同じ意味 | 2.73× | 1.11× | 既定。rsp 基準のアクセスは検査なし |
| (b) 冗長検査の除去(Base / Elim) | Base = 基底レジスタだけ。Elim = 基底を 1 度検査したら書き換えまで再検査しない(支配するブロック内) | Base 2.48×、Elim 2.35× | Base 1.11×、Elim 1.20× | Base は『負の基底 + 変位が 4 GiB 未満に着地する』場合に C 経路と違い得る。**Elim の差分テストを実施した(同じ 1,639 関数 × 300 反復): 値の不一致 0、ただし『新だけが落ちる』が 16,829 反復(744 関数)。すべて 63:32 の検査の `unreachable` で、基底レジスタが負(例 `rdx = -5`)のとき `[rdx+0x24]` が実効アドレス 0x1f に戻るのを、C 経路の `fle_addr`(実効アドレス全体を見る)は通し、Elim は落とす**。実機でもそのアドレスは NULL ページで落ちるので Elim の方が近いとも読めるが、C 経路との差である。ランダム入力は −1〜−16 の値を 10% 混ぜるので頻度は過大(実コードでの頻度は未測定)。Base の差分テストは未実施(Elim と同じ検査の形)。K1〜K4 の出力の一致は全モードで確認した |
| (c) 検査なし | 63:32 を見ない | 2.03× | 0.64× | 実機で落ちるアクセスが静かに通る。差分テストの『両方が落ちる』 50% が壊れる |
| (d) memory64 | アドレスを i64 のまま使い、63:32 の検査が不要。ガードページは使えず、V8 が**すべてのアクセスに明示的な範囲検査**を入れる | **未実施** | **未実施** | 一次資料(下)では 10% から 100% 超の低下。ゲストが 4 GiB 未満（再配置したイメージ）なら得るものがない |

(d) の一次資料(**このセッションで取得**): SpiderMonkey 開発者ブログ『Is Memory64 actually worth using?』(Ben Visness、2025-01-15。`https://spidermonkey.dev/blog/2025/01/15/is-memory64-actually-worth-using.html`)。要点: Memory64 は Firefox 134・Chrome 133 で出荷、**wasm32 は 4 GB を予約して範囲検査を省けるが、wasm64 ではそれができず全アクセスに範囲検査が入る**、速度低下は『10% から 100% 超』、JS API の上限は 16 GB。**前半のセッションの記録(今回再取得していない)**: Chromium issue 388290793(memory64 を Chrome 133.0.6875.0 で既定有効にした記録。`wasm_memory64_trap_handling` は 127.0.6533.4 から既定。範囲検査は明示的)、WebAssembly 3.0 の告知(2025-09-17、64 ビットアドレス空間)、Chrome I/O 2024 のブログ(memory64 は当時フラグつき)。PM から受け取った wwt の `--mem64` の数字(K1 3.7 → 2.4 など)は、wwt では mem64 が検査つき実装より速いことを示すが、条件が違う(読み)。

結論(推奨・仮): サイズは Exact 2.73× → Elim 2.35×(−14%)、速度の差は K5 で 1.11× → 1.20×(Elim が遅いのは測定のばらつきの範囲と読む)。**Elim を既定にする案。ただし C 経路との差(負の基底 + 変位で、C は通し Elim は落とす)を許容できるかの判断が要る**(実コードでの頻度は未測定)。(c) は差分テストの意味を壊すのでテスト専用。(d) は 4 GiB を超えるメモリが必要になるまで採らない。

## モジュール分割・memory64・プラグイン共有・V8 のコードキャッシュ(設計の検討、PM の指示)

- **モジュール分割**。いまの 6 DLL = 6 モジュールを維持し、DLL 内を『熱い関数(新で lower)』と『それ以外(C 経路)』に分ける。実行される関数集合(12,298)を混成にした本体のサイズは約 24.2 MB(計算: lower される 10,808 関数が新 7.0 MB、残りが C -Oz 17.2 MB。C 経路のままなら 28.8 MB)。V8 は関数単位の遅延コンパイル(Liftoff)をするので、起動時に払うのは検証 + 実行された関数の Liftoff だけ(**未測定**)。
- **memory64**。エンジンのイメージは 4 GiB 未満に再配置されている(FLEngine 0x400000、dsp 0x2000000 など)ため、wasm32 で足りる。memory64 は検査が増えて遅くなる(上)。→ 採らない。ゲストのヒープが 4 GiB を超える要求が出た場合に再検討。
- **プラグイン共有**(PM 情報: Delphi プラグインは 93.6% が重複)。新バックエンドは RIP 相対の参照を**絶対アドレスの i32 定数**にしている(イメージが固定の VA に再配置されるため)。同じ機械語の複製を共有するには、(i) 複製ごとの基底アドレスを隠れ引数にして `base + disp` にする、(ii) 間接呼びの表を複製ごとにする(添字 = 目標 − 複製の text 先頭)、が必要になる。(i) は RIP 相対のアクセスごとに加算 1 つ、関数に引数 1 つ。**実装も測定もしていない(推測)**。
- **V8 のコードキャッシュ**。V8 は `WebAssembly.compileStreaming` で取得したモジュールの TurboFan 生成コードを HTTP キャッシュにキャッシュし、同じ URL のモジュールを再取得すると即時に使える(一次資料: `https://v8.dev/docs/wasm-compilation-pipeline`『Code caching』、`https://v8.dev/blog/wasm-code-caching`、2019-06-17。**このセッションで検索結果の抜粋を読んだ。ブログ本文は読んでいない**)。新バックエンドのモジュールは TurboFan のコンパイルが重い(上の表)ので、**2 回目以降の起動ではキャッシュが効く**と読む。初回の起動(P1)には効かない。キャッシュの条件(サイズの上限・しきい値)は**未確認**。

## 本番規模と作業の順序

**規模の見積り(推測)**: 試作の crate は約 6,600 行。本番にするには (1) SSE の残り(Ucomis 旗、XMM 引数・戻り、整数 SIMD、Movd/Movq、Cvtsi2ss)で 3,000〜5,000 行、(2) x87 で 1,500〜2,500 行(FLEngine の census で `Fld`・`Fmul`・`Fstp` が多い)、(3) アトミック(`Xadd`・`Cmpxchg`・`Xchg`・`lock`)で 500〜800 行(メモリを shared にする必要がある)、(4) 例外(`_setjmp`/longjmp と SEH)で 1,500〜3,000 行、(5) AVX/AVX2(dsp)で 4,000〜8,000 行(v128 2 本で ymm を表す)、(6) デバッグ用の対応表(wasm のオフセット → VA)とプロファイラ連携、(7) ランタイムへの組み込み(`fle_dispatch`・HLE の import 規約・TEB・TLS)。合計 1.5〜2.5 万行の追加。

**順序(推奨)**: ① 混成の橋を `wasm-merge` 相当で本番のビルドに入れる(C 経路が既定、新が熱い関数を置き換える)。② 『実行される関数集合』の上位原因(XMM 引数、Ucomis 旗、Cvtsi2ss、アトミック、Cpuid、Bsf/Bsr)。③ 実エンジンの差分テスト(FLEngine の lower 済み関数をランダム入力で C 経路と比較。**freetype 以外では未実施**)。④ promote-shared を既定に。⑤ MXCSR の窓の最適化。⑥ x87・setjmp/longjmp。⑦ AVX。

**最大の不確実性(順に)**: (1) **実エンジンでの速度が未測定**。K 系は小さなカーネルで、実エンジンのホット関数(SSE が支配的。スカラー double が SSE 命令の 96.8%)が MXCSR の窓つきで何倍になるかは K2 の 6〜7× が最悪値の目安。(2) **間接呼びの適合性**: 呼びの先のうち、アダプタの型に収まらない関数(freetype で 1,215 / 1,639)。内部規約の関数が関数ポインタ経由で呼ばれると表が空でトラップする。解析で防げるのは『アドレスが取られていない関数』だけ。(3) **FLEngine の機械語での正しさ**(SEH・x87・スレッド・巨大関数)。(4) setjmp/longjmp と SEH の設計が**未実装・未検証**。(5) 混成の橋の実コスト(JS 経由は 1 往復あたり数百 ns と読む)。(6) 起動(P1)が 10 秒に入るか。コンパイル CPU が減らないため、実行の速さに頼る。

## 推奨

1. **続行して混成として採用する価値がある**(仮)。根拠: 整数コードで C 経路の 10〜25 倍速く native の 0.98〜1.21 倍、サイズは C -Oz の約半分(check の方式による。ただし圧縮後は同等)、実行される関数の 88% を lower でき、freetype の公開 API が(無作為分割の混成でも)通った。
2. ただし**純粋な置き換えは勧めない**。FLEngine の呼び先を含む閉包は 37%(実行集合で 44%)で、尾が長い。
3. 最初の一歩: 混成 + promote-shared を freetype と QFC・ILTools で本番のビルドに入れ、`lift/harness/test.sh full` の既存の検証で C 経路と同じ出力になることを確かめる。
4. P2(120 fps)・P3(音切れなし)への効果は**未測定**。K 系の結果からは整数・スカラー double では十分な余地があるが、窓つきの MXCSR を最適化しない限り SSE の多い DSP では 4× を超える。

## PM の未決の点

1. 混成を前提にするか(純粋な置き換えは長い尾のため非現実的)。
2. メモリ検査の既定: Elim(推奨案。C 経路と違う落ち方を許容するなら)か Exact か。`FLE_CHECKED_MEM` 相当(領域外アドレスの検査)を新にも入れるか。
3. MXCSR: 既定でモデルしない(fplow1 の窓だけ)か、すべての SSE 演算に窓を入れるか。窓の最適化に人手を割くか。
4. setjmp/SEH を wasm の例外処理で実装する(推奨)か、C 経路に永続的に残すか。
5. アトミック命令のために shared メモリ(スレッド)を前提にしてよいか。
6. 本番に入れる前の検証の水準(FLEngine での差分テスト、G 段階の再実行)。
7. 起動(P1)の見積りには、コンパイルの CPU が減らない(TurboFan)ことを織り込むか。

## 限界(確認できなかったこと)と自己申告

**確認できなかったこと**
- FLEngine・dsp・Blend2D・QFC・ILTools の機械語は**一つも動かしていない**(census で lower 率を数えただけ)。
- 実エンジンでの速度、P1・P2・P3 への効果。
- 実機との照合(比べたのは C 経路だけ)。
- longjmp、SEH、スレッド(アトミック)、x87、AVX。
- memory64 の実測(一次資料のみ)。
- brotli、実エンジンの V8 コードキャッシュの効き。
- 前反復の残骸と C 配置依存の除外により、一部の反復は比較できていない(728 + 2 反復)。hang の 38 関数は 300 反復を完了していない。
- 混成の `wasm-merge` 版は memory が 2 本のまま(同じ Memory を渡した)。1 本にする本番の形は**未実施**。
- 混成の無作為分割の C 部分は `-O0` でビルドしたもの(50% 分割だけ `-O2` でも確認。本番の `-Oz` では未実施)。FreeType の jmp_buf の 5 バイトの差は、longjmp が起きなければ読まれないと読んだだけで、longjmp を起こして確かめていない。

**自己申告(規則違反・事故)**: (1) 存在しない `bash` ツールを通算 5 回呼んだ(シェルとして実行された)。(2) `pkill -f` で自分のシェルを落とした 1 回。(3) `sleep` を含むコマンドを実行しようとして拒否された 3 回(`sleep 1` 2 回、`sleep 0` 1 回。いずれも実行されず)、末尾 `&` を含むコマンドが拒否された 1 回(`sleep 0` と同じ呼び)。(4) 過去の単独テストの node プロセスが約 60 分間残って CPU を使っていた(PID 指定で kill)。(5) 関数ベンチを二重に起動し 2 本とも kill した(PID 指定)。(6) 約 2.6 MB のエラー洪水ログを `logs/` に作り、削除した。いずれも他の worker のプロセスは触っていない。

## 成果物

`lift/out-git/wasmgen1-results/`: `wasmgen1.diff`(新 crate + Cargo.toml・Cargo.lock)、`docs/0157-direct-wasm-backend.md`(この文書)、`docs/README-0157-line.txt`、`MERGE-NOTES.md`、`wall-log.md`、`tables/`(census・executed-coverage・サイズ・速度・差分テスト・混成の TSV)、`tools/`(差分テスト `difftest.cjs`、API テスト `ftapi.cjs`、census・サイズ・混成のスクリプト、`kbench/`)、`logs/`(差分テストと API テストの JSON)。
