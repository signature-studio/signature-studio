# 0160: Blend2D.dll のコンテキスト(`blContextInitAs` と BLContextVirt の 17 関数)を上流 Blend2D 0.11.1(no-JIT)に置換する。置換は fe-lift の opt-in(`--swap-entries`)+ホスト側アダプタ。オブジェクトモデルは lift のまま(ハイブリッド)。native で走査 142 本・fuzz 300 種・実エンジン 16 スナップ(progress 分離の構成)が lift 版と一致。仮の決定

状態: **仮の決定**。依頼は PM(決定 0160)。調査・実装・測定・文書は作業者(`b2d1`、2026-11-03 05:58〜)。作業は HEAD **291060b** の `git archive` で始め、差分 `b2d1.diff` は PM の指示(HEAD の進行〔hle2/size1/fplow1/wasmgen1 の取り込み〕)で **HEAD 071c53f の `git archive` に載せ直した**(`lift/crates/fe-lift/src/lib.rs`・`main.rs` は 071c53f の版に対して書き直し、`GenOptions` に項目を足す形。`patch -p1 --dry-run` の結果は MERGE-NOTES.md)。

区分の凡例は決定 0152 と同じ。**実測**＝作業者のコマンドの出力を見た事実(下に出所のファイル)。**読み**＝逆アセンブルやコードを読んだ解釈(走らせていない、または走らせても網羅していない)。**出典なしの選択**＝根拠が無く作業者が選んだもの(`S-b2d1-…`、§10 に撤回条件つき)。**未確認**＝確かめていない。**実機 Windows との照合は 0 件**。

## 0. 進捗の誠実性(AGENTS.md の節に従う、3 点)

1. **実エンジンが実行された事実と範囲**: この文書の「実エンジン」の数値は、HEAD の生成 C(FLEngine と同梱 5 DLL、FRUITY 有効の最終構成 = `a4-fruity.sh on-seh` 相当、FLEngine の C の SHA-256 `6d39d04f…`)を clang -O2 で **native** にビルドし、`cmp-short.g3`(16 スナップ)を**実際に走らせて**取った。Blend2D のモジュールだけを別に lift(`--pdata` なし、`--swap-entries`)して差し替えたバイナリと、差し替えないバイナリ(同じオブジェクト群)の両方を走らせて比べた。**wasm でエンジンを走らせてはいない**(§9)。ブラウザでも走らせていない。HEAD 071c53f の木でも native で再実行した範囲は §9 の先頭。アダプタ単体は wasm32(node)でも走らせた(§7.6)。
2. **代役・再生・推測を含む範囲**: 代役・モック・画像の再生・テスト用の期待値は使っていない。比べている相手は『同じビルドの lift 版(置換なし)』の出力で、実機の出力ではない(実機との照合は 0 件)。「同一」はこの 2 つのビルドの間のことである。progress の分離構成(§6.3)は**検証専用**で、既定の入口ではない。推測(wasm の速さ、実機 CPU での libm の差など)は『読み』と書いた。
3. **HLE 入りか無しか**: 置換版は『HLE 入り』(ゲストの関数をホスト実装に置き換えている)。置換なしの lift 版は HLE 無し。この決定自体は**採用の可否を PM が決める**。勝手に既定にしていない(生成 C は、オプションを付けないとき HEAD の fe-lift と byte 同一: §4)。

## 1. 先に結論(表)

| # | 問い | 答え | 区分 |
|---|---|---|---|
| 1 | 計画『Blend2D のコア約 41 入口を上流 wasm に置換』は成り立つか | **そのままでは成り立たない**。ラッパー(Agg2D)は Blend2D オブジェクトの impl を直接読み書きし(refcount/flags、context の virt と state の中身、画像の format)、既定オブジェクトは Blend2D の runtime init が `.data` に書く。上流 wasm32 の impl は配置が違う。→ **オブジェクトモデルは lift のまま、置換はコンテキストだけ(18 入口)**のハイブリッドに変更(PM が承認) | 読み(静的)+実測(gdb) |
| 2 | 置換の機構 | `fe-lift lift … --swap-entries FILE`(`ADDR NAME`)。指定した入口は関数入口として閉包に入るが本体はデコードせず先も辿らず、C は `fle_swap_NAME()` を呼ぶスタブ。**オプション無しの生成 C は HEAD の fe-lift と byte 同一**(FLEngine と 5 DLL)。単体試験 6+4 件 pass | 実測 |
| 3 | 閉包の大きさ(Blend2D) | 現行(`--pdata`+seeds964): 3,093 関数・432,647 命令・C 101,445,366 B。置換版(ラッパー+init+CRT+オブジェクトモデル+必要な分岐先の種 575 個): **733 関数・45,729 命令・C 4,084,565 B**(約 1/25) | 実測 |
| 4 | wasm の大きさ(Blend2D のモジュール全体: lift 部+アダプタ+上流 no-JIT、デッドコード除去を止めて測定) | lift 部 -Oz: **2,344,334 B(brotli 386,046)**、lift 部 -O2: 3,826,426 B(brotli 480,067)。上流部分は常に -O2 でビルド(stock の CMake が -O2 を足す)。現行 lift の Blend2D は 44.0 MB(-Oz)/19.5 MB(outline-shared 後)(**PM の値、再測定していない**)→ 約 1/19、約 1/8 | 実測(置換版)/PM の値(現行) |
| 5 | 置換の同一性(native、lift 版 vs 置換版、同じオブジェクト群) | 走査 142 本: RESULT/CHECK/VALUE と、各呼び出しの後のコンテキスト state(0x120 バイト)のハッシュ 230,331 行が**バイト一致**。fuzz 種 41〜300: 288,648 行一致。swap1 の 20,000 手のハッシュ `e4b385310d0fca08` 一致。実エンジン 16 スナップ(progress 分離構成): **84 行一致**(面 SHA 34 行を含む) | 実測 |
| 6 | 見つかった不一致 | 回転行列 18 個中 2 個が最後の 1 bit 違う。DLL の C ランタイム(MSVC)の `sin(π/4)`、`cos(1.0)` が上流側の glibc と 1 ULP 違う。→ **sin/cos/tan/atan2/acos/pow/fmod を DLL の lift した関数へ橋渡し**して 18 個一致。**libm に依存する上流の計算は、橋渡ししないと lift 版と一致しない**(置換の本質的な制約。cbrt は DLL に無さそうで未橋渡し) | 実測 |
| 7 | 変異検査 | アダプタを 1 件ずつ壊す 16 通り: **16/16 検出**。対照(壊さない)は通過 | 実測 |
| 8 | FL が渡す合成演算(PM の最優先項目) | 16 スナップで **blendMode(Agg2D slot 15)は 0 回、setCompOp(コンテキスト)も 0 回、描画 35,172 回はすべて SRC_OVER**(JIT 専用の演算は未使用)。OVERLAY(グラデーションのストローク)の経路は通っていない | 実測 |
| 9 | 速さ(native -O2、負荷 6.8、CPU 秒、交互 5 回) | 20,000 手: lift 1.18〜1.23(中央 1.19)→ 置換 0.55〜0.61(0.59)。attach+clear+rect+path ×2,000: lift 0.138〜0.144 → 置換 0.044〜0.057。**起動(最初の idle まで)の CPU 秒は差が小さい**: user+sys 中央 lift 3.93 → 置換 3.72(範囲が重なる)。16 スナップ全体 user 6.81 → 5.88 | 実測 |
| 10 | 最大の障害 | (a) ラッパーが impl を直接読む(§2)、(b) 閉包の種: 関数ポインタ・switch 表・CRT 表でしか届かない関数を落とす(§5)、(c) libm の 1 ULP(§6.2)、(d) 仮想時計が progress(間接呼び出しの数)で進むため、置換だけで面 SHA が変わる(§6.3) | 実測 |
| 11 | 推奨 | §11 | 読み |

## 2. 計画からの変更: 『約 41 の入口を全部置換』は成り立たず、コンテキストだけを置換する(ハイブリッド)

依頼(決定 0160 の元の計画)は『Agg2D ラッパーだけを lift し、Blend2D のコア約 41 入口を上流 wasm に置き換える』だった。静的解析(`tables/wrapper-reads.tsv`、`tables/wrapper.dis`、`tools/deref_scan.py`)でこれが成り立たないと分かった。PM は第 1 報の後でハイブリッド案を認めた。

| # | 事実 | 根拠 | 区分 |
|---|---|---|---|
| 1 | ラッパーは Blend2D オブジェクトの impl メモリを**直接**読み書きする: refcount(impl-0x10、8 バイトの `lock xadd`)と flags(impl-0x08、`flags&3` と比較して destroy を呼ぶ)は BLContext/BLImage/BLPath/BLGradient のすべてで、コンテキストは impl の virt(+0)と state(+8)、state の中身(compOp@0x20、fillRule@0x21、globalAlpha@0x28、startCap@0x40、join@0x42、width@0x48、userTransform@0xc0〜、finalTransform@0xf0〜)、画像は impl+0x18(format) | `tables/wrapper-reads.tsv`(逆アセンブルの行つき) | 読み(逆アセンブル)。走査 140 と実エンジンが通ったことで、一覧に漏れがあれば画素か鏡の検査で出るはず、という間接の確認のみ |
| 2 | 上流 wasm32 の impl は header が 4+4 バイト、ポインタ 4 バイトで配置が違う(上流 `BLObjectImplHeader {size_t refCount; size_t flags}`)。native の x86-64 では上流とゲストの配置が一致する(`offsetof` の一致を確認) | `b2d_swap.cpp` の `static_assert`、`tmp/off.cpp` の出力 | 実測(native) / 読み(wasm32) |
| 3 | 既定オブジェクト(空の BLPath/BLImage/BLGradient/BLContext)は、Blend2D の runtime init(静的初期化 0x1000 → 0x199d90)が `.data` に書く 16 バイトで、ラッパーのコンストラクタが複製する。init を落とすと 0 のコアになる | `tables/ctor-dtor.dis`、`tools/xref_rip.py` の出力 | 読み |
| 4 | 実コンテキストの仮想関数表は静的な `.rdata` ではなく、init が実行時に `.data`(RVA 0x1f4100)に作る(gdb で実測: virt=0x48f4100)。DLL の `.rdata` にある 6 つの BLContextVirt は実際には呼ばれない(gdb の bp 0 ヒット) | `blend/count3.tsv`、`tables/ctx-syms.txt` | 実測 |

結論: **オブジェクトモデル(BLPath/BLGradient/BLImage の入口約 25 個、impl の確保・解放、既定オブジェクト、refcount)は lift のまま**(ゲスト配置のまま)にし、**置換はコンテキストだけ**: `blContextInitAs`(RVA 0x146ff0)と、ラッパーが呼ぶ仮想関数 16 種+destroy(BLContextVirt の slot 0、3、19、25、32、33、34、35、38、39、45、46、51、53、54、71、72。上流 0.11.1 の `offsetof` と swap1 の表に一致)。置換入口は計 18。

### 2.1 アダプタの構成(`lift/harness/b2d1/b2d_swap.cpp`)

- **プロキシ impl**(ゲスト配置、16 バイト整列の 1 ブロック): header(refCount=1、flags=0x21。DLL 自身のコンテキストが書いていた値を gdb で読んで同じにした)、`virt`(プロキシ自身の 80 スロットの表。置換した 17 関数の VA だけを入れ、他は 0)、`state`(鏡へのポインタ)、contextType、上流の `BLContextCore`、上流の `BLImageCore`(ゲストの画素に被せる)、BLContextState の鏡(0x120 バイト)。ゲストのコアには impl とその info(上流が付けた値)を書く。
- **virt の VA**: 置換した関数は DLL の静的表 0x1d6380(3 表のうち最初のもの。17 アドレスが互いに異なる)の関数で、本体は lift しない(生成 C はスタブ)。プロキシの表の slot k にその VA を入れるので、ゲストの `call [virt+8k]` は `fle_dispatch` を通ってスタブ → `fle_swap_ctx_*` に届く。表の他のスロットは 0 で、呼ばれたら『未知の間接呼び出し』で名前つきで止まる。
- **鏡**: 状態を変える仮想関数の後で、上流の BLContextState をゲスト配置のフィールドへコピー(`fill_state`)。コピーしないフィールド: targetImage(ラッパーは読まない)と dashArray(同)。`B2D1_CHECK=1` では全呼び出しの後に再計算と比較する。
- **変換**(描画時、呼び出しの間だけ): 幾何(BOXI〜TRIANGLE は POD なのでゲストのポインタをそのまま、POLYLINE/POLYGON/ARRAY_VIEW は `{u64 data, u64 size}` を上流の `BLArrayView` に、PATH はゲストの BLPathImpl の command/vertex 配列から上流の BLPath を作る)。スタイル(色は 16 バイトのコピー、グラデーションはゲストの BLGradientImpl の stops/values/transform と info の type/extend から `blGradientInitAs`)。パターンなどは名前つきで停止。呼び出しが終わったら一時オブジェクトを破棄する(上流のコンテキストは描画時に必要なデータをコピーまたは参照カウントで保持する)。
- **寿命**: 上流のコンテキスト/画像はプロキシが持つ。ゲストの inline の解放(`lock xadd -1` が flags&3 に達したら `virt[0]`)で `fle_swap_ctx_destroy` が呼ばれ、`blContextReset`+`blImageReset`+free。画素バッファはゲストのもの(`blImageCreateFromData` が作った画像の pixelData)で、アダプタは書き込むだけ。
- **メモリ確保**: プロキシは `aligned_alloc`(ホストのヒープ。ゲストの `HeapAlloc` ではない)。上流の確保は上流自身のアロケータ(`malloc`)。wasm32 ではゲストと同じ線形メモリ、ポインタは恒等写像(上位 32 ビットが非 0 なら名前つきで停止)。
- **呼び出し規約**: 引数は R[1]、R[2]、R[8]、R[9] と X[1]〜X[3] の下位 double、結果は R[0]。呼び出し元が期待するのは callee-saved レジスタと R[4] の不変だけで、アダプタは R[] の他の要素を触らない(volatile レジスタとフラグは lift した本体が壊した値とは違う値になる。ABI に従う呼び出し元には見えない: 読み)。

### 2.2 鏡の更新(状態変更の仮想関数ごと)と不変条件

| 仮想関数(slot) | 鏡の更新 |
|---|---|
| destroy(0) | プロキシを解放(鏡ごと)。生存数を減らす |
| applyTransformOp(3)・setHint(19)・setStyle(25)・setGlobalAlpha(32)・setCompOp(33)・setFillRule(34)・setStrokeWidth(35)・setStrokeCaps(38)・setStrokeJoin(39)・clipToRectD(45)・restoreClipping(46) | 呼び出しの後に `sync_state`(BLContextState の全欄をコピー) |
| fillAllRgba32(51)・fillGeometry(53)・fillGeometryRgba32(54)・strokeGeometry(71)・strokeGeometryRgba32(72) | 状態を変えない(鏡は更新しない)。`B2D1_CHECK=1` では呼び出しの後に再計算と比較して確かめる |

不変条件: (1) 各呼び出しの後、鏡は上流の BLContextState から新しく作ったコピーと一致する(`B2D1_CHECK=1` が全呼び出しで検査)。(2) 鏡は、lift 版の DLL が持つ BLContextState(同じ呼び出し列の後)と、targetImage と dashArray を除いて一致する(§6.1 の state ハッシュ、全 230,331 行)。変異検査の 2・3・12 が (1)(2) を守る(§6.5)。

### 2.3 変換の費用と寿命

幾何・スタイルの変換は呼び出しごとに作って呼び出しの終わりに破棄する(上流のコンテキストは描画に必要なデータを自分でコピーまたは参照カウントで持つ)ので、ゲストが描画の後に path を変更・解放しても上流は影響を受けない。費用は path の頂点数に比例する(頂点 16 B + コマンド 1 B のコピーと `BLPath` の確保)。測ったのは頂点 3〜6 個のパス(`stress`)までで、**FL の大きなパスでの費用は未測定**。ポリゴン・折れ線は配列ビューの組み立てだけでコピーしない。グラデーションは stop 数に比例(`blGradientInitAs` のコピー)。

## 3. 機構(fe-lift `--swap-entries`)

`fe-lift lift FILE OUT.c … --swap-entries SWAP.txt`。ファイルは 1 行 1 入口 `ADDR NAME`(`ADDR` は VA か `rva:` + 16 進、`#` コメント)。

- `fe-decode`: `analyze_with_swap(pe, extra, specs, opts, &swap)`(`analyze_with_options` は空集合で呼ぶだけ)。置換入口は seed として関数入口になるが、デコードも先の辿りもしない。他の関数からの `call`/`jmp` はその入口への呼び出し/tail call になる(直接 `jmp` が入口に向かうとき、その本体を呼び元のブロックとして辿らない)。
- `fe-lift`: `GenOptions.swap`(`generate_full` の引数は不変)。出力は関数ごとに `void f_<va>(void) { extern void fle_swap_<name>(void); fle_swap_<name>(); }`。dispatch 表には入る(仮想関数表からの間接呼び出しが届く)。`--outline-shared`・`--dead-flags` とは併用不可(明示のエラー)。入口が `.text` に無い/重複は名前つきエラー。
- 呼び出し規約: ホスト実装は入口時点のゲスト状態(引数 R[1] R[2] R[8] R[9] と X[1..3]、戻りアドレスが [R[4]])から読み、結果を R[0] に書く。callee-saved と R[4] は変えない。volatile レジスタ・フラグは lift した本体が残す値とは違う値になる(ABI に従う呼び出し元には見えない: 読み)。
- 試験(`tests/swap_entries.rs` 6 件、`src/swap.rs` 単体 4 件、期待値は命令の意味と上の規則から書いた): 置換入口は stub になり callee は閉包に入らない/入口は dispatch 表に残る/tail call/オプション無しは旧出力と byte 同一/不正入口はエラー/併用不可はエラー/生成 C がコンパイルできる。

## 4. 生成 C の不変(オプション無し)

HEAD 071c53f の fe-lift と、載せ直した fe-lift の出力を、オプション無しで比べた(同じ入力):
FLEngine(本番の seeds・hook・SEH 付き)の C SHA-256 `6d39d04fc58d80b52ec1f39ba1aa15ba378aecd6e99cd757b9ce90dc8fd8904d`(両方同じ)、Blend2D(`--pdata`+seeds964+sites)`9600514f702a22d6f0737714df89d8183b0ced28050bb2cbf3fb1b2ab7da5f4b`(両方同じ)。HEAD 291060b の fe-lift に対しても 6 画像(FLEngine と 5 DLL)で同一(`logs/gen-cmp-sha.txt`)。**既存の全体テスト(`bash lift/harness/test.sh full`、基準 142 results・2,083 passed)と FRUITY 無効 a4 は走らせていない**。生成 C が同一であること(任務が許した代替)に拠る。新しい差分が既存テストの対象に触れるのは `lift/harness/b2d_scan.c`・`b2d_scan_table.h`(試験を足しただけ。環境変数が無ければ既存の試験の出力は同じ)だけ。

## 5. 閉包の種(`--pdata` を外すと必要になるもの)

置換版の Blend2D は `--pdata` を使わない(使うと全関数が種になり、置換入口の先を落とした分しか減らない)。種は `lift/harness/b2d1/blend2d-4700000-seeds-wrapper.txt`(575 行): Agg2D vtable(RVA 0x1dbec0)の 79 関数、CRT/libm 領域(RVA ≥ 0x19d000)の pdata 先頭・`lea` 先・再配置の値 465 個(`tools/crt_seeds.py`)、静的初期化 0x1000 とその周りの CRT 初期化表、ラッパー領域(0x56000〜0x5a000)の pdata 先頭と `lea` で関数ポインタとして渡される 6 関数(0x56400、0x56410、0x56450、0x56460、0x56650、0x566f0。`Agg2D` のコンストラクタが配列初期化に渡す)、CRT の fallback 表(RVA 0x1b8a40〜)の指す先、実行して見つけた 6 個(下)。

**関数ポインタ・switch 表でしか届かない関数は、閉包から落ちる**。落ちた関数への間接呼び出しは `fle: indirect call to 0x… is not a known function entry` で名前つきで止まる(黙って変わらない)。見つけ方: 診断用ビルド(`link_b2d1.sh DIAG_MISS=1`、`miss_log.c`)は未知の入口への間接呼び出しを**記録して飛ばす**ので、1 回の走行で複数が取れる(そのビルドの結果は結果ではない)。走査 140 本+fuzz 300 種+エンジン 16 スナップで、記録 0 になるまで繰り返した(`tools/seed-fix.sh`、`tables/misses-round*.txt`): switch 表の case 本体 2 個(0x4770ed2、0x47748e5)、`lea` で渡される関数 3 個(0x4756410、0x4756460、0x47566f0。ラッパー領域の `lea` 先を静的に全数出して種にしてあったものを、seeds964 との差し引きで一度落としたので、診断で再発見された)、ラッパー領域の関数 1 個(0x47572f0。呼び出し元は同定していない)。

**限界(未確認)**: 種は『走らせた範囲で必要だった物』で、FLEngine が 16 スナップの外(曲再生、他のウィンドウ、プラグイン画面)で使う Blend2D の経路が必要とする関数が足りなければ、そこで名前つきで止まる(止まるだけで、黙って間違えない)。fe-lift の switch 表の解決が不完全(Blend2D で未解決の `jmp reg` 81 サイト: 決定 0120)なので、静的に完全な種は作れていない。seeds964 のうち閉包の関数の内側にある 132 個を全部足すと 4.3 MB 大きくなる(case 本体ごとに末尾までの複製ができる)ので足さなかった(`tables/seeds-wrapper-full964.txt` に全部入りの版)。
## 6. 検証(置換の同一性)

比較の相手は『同じオブジェクト群(`lift/out-b2d1/obj-base` の FLEngine・dsp・QFC・freetype・ILTools・runtime・harness)で Blend2D のモジュールだけが違う』2 つの native バイナリ: lift 版(`--pdata`+seeds964、置換なし)と置換版。コンパイルは clang -O2、`-ffp-contract=off`、builtin 有効(決定 0151 の最終フラグ。HEAD の本番スクリプトは `-fno-builtin` だが、決定 0151 は出力が同一であることを示している)。

### 6.1 走査(Agg2D の 130 スロット、決定 0120 の道具)

- 142 プロセス(名前つき 69〔`stubs` `graddump` ほか〕+ `blendmode:0..28`(29)+ `fuzz:1..40`(40)+ 新規 `stress` `attachloop` `rotprobe` `libmprobe`〔環境変数が無いと何もしない〕)。lift 版も置換版も **142/142 が rc=0**。
- `FLE_B2D1_RET=1` を付けると、各スロットの呼び出しの後に『impl+8 の state(0x120 バイトから targetImage と dashArray を除く)のハッシュ』を `VALUE` 行に出す(lift 版では DLL 自身の BLContextState、置換版では鏡)。RESULT/CHECK/VALUE と合わせて **230,331 行がバイト一致**(`cmp`)。CHECK 100 件は両方 PASS、FAIL 0。置換版は `B2D1_CHECK=1`(各呼び出しの後に鏡を上流 state から再計算して比較)で走らせた(不一致 0)。
- `fuzz:41〜300`(260 本、`FLE_B2D1_RET=1`): 288,648 行一致(`logs/`)。
- `stress`(swap1 の乱数 20,000 手、1280×720、seed 777): lift 版 `0xe4b385310d0fca08`、置換版 同じ。swap1 が実 DLL(Wine)と上流で得た値と同じ。1,000 手ごとの 20 点も一致。

### 6.2 libm の橋渡し(見つかった不一致と修正)

回転(Agg2D slot 55)後の userTransform を 18 通りの角度で読むと、2 個が 1 ULP 違った(`rot3` π/4 の m01/m10、`rot4` 1.0 の m00/m11: `runs/rotprobe-*`)。lift 版は DLL に静的リンクされた MSVC の `sin`/`cos`(RVA 0x1b4480/0x1b24d0)を実行しており、それは正しく丸められた値と 1 ULP 違う場合がある(`cos(1.0)` が `0x3fe14a280fb5068b`、glibc は `…068c`)。上流の native/wasm は glibc/musl を呼ぶので一致しない。

対処: `b2d_swap.cpp`(`B2D1_LIBM_BRIDGE=1`)が `sin cos tan atan2 acos pow fmod sincos` を定義し、DLL の lift した関数(RVA `0x1b4480 0x1b24d0 0x1b4b60 0x1b1de4 0x1b1b40 0x1b2de0 0x1b2b00`)をゲストの呼び出しとして実行する(同定は『候補を 1 つずつ呼んで結果を Python の libm と突き合わせる』`libmprobe` による。値が一致する物を採った)。ゲストのスタックは 16 バイト境界の呼び出し元として作る(最初の版は整列を誤り `misaligned 16-byte access` で止まった。名前つき停止で見つかった)。結果: 18 個すべて一致。`cbrt` は DLL の C ランタイムに見つからず未橋渡し(上流が呼ぶ経路が走査・エンジンで実行された形跡はないが、**未確認**)。**実機 CPU では DLL の libm が FMA3 版を選ぶので、さらに別の値になりうる**(本プロジェクトの lift は CPUID を基準値に固定する: 決定 0118)。

### 6.3 実エンジン 16 スナップと progress の分離

最初の実エンジンの走行(既定の progress の数え方)では、置換版は最後まで動いた(CPU 7.4 秒、attach 8,016 回)が、84 行のうち面 SHA 16/34・clock・progress が基準と違った。画素の差ではなく**時計**の差: 仮想時計と協調スレッドの切替は `fle_progress_count`(ゲストの間接呼び出しの数)で進む(決定 0147)。置換でゲスト内部の間接呼び出しの数が変わる(idle までの progress: lift 版 10,881,831、置換版 10,959,464)と、アニメーションの位相が変わって面 SHA が変わる。**progress を同じに保たない置換では、どれだけ画素が同じでも『16 スナップの面 SHA が不変』にならない**。

そこで opt-in の検証構成を作った: Blend2D のモジュールの単位だけ `-Dfle_progress_count=fle_progress_count_b2d -Dfle_progress_limit=fle_progress_limit_b2d` でコンパイルする(その単位の間接呼び出しが別の変数を増やす=数えない。割り込み点も Blend2D の単位では起きない)。lift 版も置換版も同じ。**既定のビルド(指定なし)は従来のまま**で、従来の基準(idle `a970cb0a…`、収束 `4fa213b0…`、Enter 後 `dd416725…` ほか 84 行)を再現した(自分のビルド `obj-base` で `cmp` 一致。別担当のバイナリ `out-fplow1/base/two-native-old` の出力とも一致)。

分離構成での結果(`tables/g3lines-*.txt`、`logs/`): **lift 版と置換版の 84 行が全部バイト一致**(snap 16、progress 16、surface 34、layered 2、wintable 16)。置換版は `B2D1_CHECK=1` で 8,016 回の attach、35,172 回の描画を通した(`runs/engine-swap-p.stats`)。分離構成での lift 版の面 SHA 34 行は、既定の構成の基準と**同じ位置で 22 行が一致し 12 行が違う**(最初の idle `a970cb0a…` は一致。数え方の変更だけで、画素を変えたのではない: Blend2D の単位の間接呼び出しが 16 スナップ全体で 518,346 回、従来の progress に入っていた〔既定 18,284,440 − 分離 17,766,094〕)。Blend2D の呼び出し 1 回あたりに直すと、描画 1 回(attach・初期化の分を含めて割った粗い値)あたり約 14.7 回の間接呼び出し(518,346 / 35,172)で、最初の idle までは 202,030 回(10,881,831 − 10,679,801)。置換版は Blend2D 内部(ラスタ化・パイプライン)の間接呼び出しが無く、ラッパー・init・CRT・オブジェクトモデルの分だけ残る(既定の数え方での置換版の progress 18,085,771 − 分離 17,766,094 = 319,677)。分離構成の基準 SHA は `tables/g3lines-split-count.txt`(lift 版)、`tables/g3lines-split-count-swap.txt`(置換版。同一)。

### 6.4 アダプタの単体試験(ポインタ幅に依存する経路)

`test_adapter.cpp`: 『ゲスト』は素のメモリで、ゲスト配置の画像 impl・パス impl(command/vertex 配列)・`{u64 data, u64 size}` の配列ビューを手で作り、R[]/X[] に引数を入れてスタブを呼び、同じ描画を上流の C++ API で直接行った結果と画素ハッシュを比べる(40 ラウンド×60 手: 矩形・楕円・ポリゴン塗り・折れ線ストローク・ストローク幅・結合・塗りルール・平行移動・パス塗り)。**native と wasm32(emscripten、node 24、-Oz と -O2)で全ラウンド一致**。wasm32 の実行は、ゲスト配置と上流の配置が実際に違う条件での実行である(native は 64 ビットで両者が一致する)。

### 6.5 変異検査(アダプタ 16 通り+対照)

`b2d_swap.cpp` を `-DB2D1_MUTANT=n` で 1 件ずつ壊し、走査を決めた順序で走らせ、lift 版と違う行(または停止)が出たら検出(`harness/b2d1/mutate.sh`、`logs/mutation-*.txt`)。

| n | 変異 | 検出した試験 |
|---|---|---|
| 1 | setCompOp の値に +1 | clear |
| 2 | 鏡の userTransform を更新しない | rotprobe(state ハッシュ) |
| 3 | 鏡の width に miterLimit を入れる | rotprobe(state ハッシュ) |
| 4 | PATH の頂点数を 1 減らす | pathstroke |
| 5 | グラデーションの stop 数を 1 減らす | gradvert |
| 6 | 配列ビューの size を 1 減らす | geometry |
| 7 | setHint の引数を入れ替え | rotprobe(state の hints) |
| 8 | setStyle のスロット(fill/stroke)を入れ替え | gradient |
| 9 | 画像の幅と高さを入れ替え | stress(小。停止) |
| 10 | fillRule を反転 | rotprobe(state) |
| 11 | proxy header の flags を 0x23(ゲストの解放判定が destroy に届かない) | attachloop(`B2D1_CHECK=1` のリーク検出で停止) |
| 12 | applyTransformOp の後に鏡を更新しない | rotprobe(state) |
| 13 | setStrokeCaps が setStrokeJoin を呼ぶ | linecaps |
| 14 | fillGeometryRgba32 の色の 1 byte を変える | rect |
| 15 | clipToRectD の幅に +1 | clipbox |
| 16 | libm の橋渡しを外す | rotprobe |
| 0(対照) | 変異なし | 全部通過(SURVIVED と表示される=検出されない=正しい) |

注: 変異検査は種の最終版の前(種 549〜710 個の版)の閉包で行った。種はアダプタの変異の検出に関係しない(読み)。最終版では全体の一致(§6.1〜6.3)を再実行した。11 は最初の走行で生き残り(destroy に届かないだけで出力は変わらない)、リーク検出を足して検出するようにした。
## 7. 測定

条件: 16 コアの共有機、負荷 6.7〜6.9(`runs/timing-*-final.txt`。壁時計は負荷 8 以下のときだけ壁時計として書く)。CPU 秒はプロセスの CPU 時間(`CLOCK_PROCESS_CPUTIME_ID` または `/usr/bin/time` の user+sys)。native -O2(clang)、lift 版と置換版を交互に 5 回(起動は 4 回)。実機 Windows・依頼者のブラウザでの測定は 0 件。

### 7.1 大きさ

| 項目 | 値 | 区分 |
|---|---|---|
| 現行の Blend2D の生成 C(`--pdata`+seeds964) | 3,093 関数、432,647 命令、1,207,108 行、101,445,366 B | 実測 |
| 置換版の生成 C(種 575 個、置換 18) | 733 関数、45,729 命令、4,084,565 B(1 ユニット) | 実測 |
| 置換版の wasm(lift 部 + `b2d_swap` + 上流 no-JIT + stub の libc 部分。`-sEXPORTED_FUNCTIONS` で lift した関数を全部残してデッドコード除去を止めた。`tables/wasm-sizes.txt`) | lift 部 -Oz: **2,344,334 B**(brotli 386,046)。lift 部 -O2: **3,826,426 B**(brotli 480,067) | 実測 |
| 上流の部分 | 上流 a7f9476 を emscripten 6.0.12 でビルド(`BLEND2D_NO_JIT=ON`)。**常に -O2**(Blend2D の CMakeLists が Release で -O2 を足し、`-Oz` を渡しても `-O2` が勝つ。MinSizeRel でも同じ結果だった)。ライブラリ(.a)2,796,398 B。上流 wasm の -Oz は作れていない | 実測 |
| 現行 lift の Blend2D の wasm | 44.0 MB(-Oz)、19.5 MB(outline-shared 後)。**PM の値で、再測定していない** | PM の値 |
| 比 | -Oz: 44.0 MB → 2.34 MB(1/19)。outline-shared 後の 19.5 MB → 2.34 MB(1/8)。-O2 の置換版 3.83 MB は 19.5 MB の 1/5(フラグが違うので参考) | 計算 |

### 7.2 速さ(native -O2、`tools/timing.sh`)

| 負荷(5 回の最小/中央/最大、process CPU 秒) | lift 版 | 置換版 |
|---|---|---|
| `stress`(swap1 の乱数 20,000 手、1280×720、ラッパー経由) | 1.180 / 1.193 / 1.227 | 0.550 / 0.589 / 0.610 |
| `attachloop`(attach+clear+矩形+パスのストローク ×2,000、64×64) | 0.138 / 0.139 / 0.144 | 0.044 / 0.052 / 0.057(1 回あたり 26 µs) |
| 起動(`idle-only.g3`、最初の idle まで。user) 4 回 | 3.64 / 3.74 / 3.95 | 3.50 / 3.51 / 3.91 |
| 起動 user+sys | 3.83 / 3.93 / 4.14 | 3.70 / 3.72 / 4.14 |
| 16 スナップ全体(`cmp-short.g3`、既定の progress の数え方)user(各 1 回) | 6.81 | 5.88 |

読み: 置換で native の描画は約 2 倍速い(lift した Blend2D の raster 部が native でも遅い)。**起動は範囲が重なる**(Blend2D は起動の CPU の約 6%: 決定 0151 の標本〔再測定していない〕に一致する大きさの差)。比較用の下限(ラッパーを通さない上流直接): swap1 の同じ乱数 20,000 手の native no-JIT 0.18〜0.20 s、wasm(node)0.47〜0.51 s(swap1 の `stress_ref-*` を本作業が再実行。wasm の上流のビルドフラグは swap1 の指定〔-Oz/-O2〕で、実効は §7.1 の注と同様に -O2 の可能性がある=未確認)。置換版の native 0.59 s との差 0.4 s が、lift したラッパー+アダプタ(1 手あたり約 8 スロット呼び出し)の費用。**置換版の wasm の速さは測っていない**(wasm でエンジン・走査を走らせていない)。

### 7.3 1 回の attach あたりの費用(PM の依頼 2)

- 上流単体(`bench_ctx.cpp`: 64×64 の画像、`BLContext` 作成、塗り 1+矩形 1+ストローク 1、破棄)の 1 サイクル: native 3.3 µs、wasm(node)5.6 µs(bench を -O2)/12.5 µs(bench を -Oz。ライブラリは両方 -O2)。
- 置換版の native の `attachloop`: 26 µs/回(ラッパーの attach+clear+rect+path を含む。lift 版は 70 µs/回)。アダプタ自体(プロキシの確保、上流コンテキストの作成、鏡の初期化)は、`attachloop` で支配的ではない(上流単体が 3.3 µs)。`fle_swap_ctx_init_as` の最初の版は dispatch 表(112,801 行)を 17 回線形に探して 1 回 0.5 ms かかった(attach 2,000 回で 1 秒): 初回だけ探すように直した(読み: 同じ構成の他の探索は無い)。
- 16 スナップでは attach 8,016 回、destroy 8,011 回(生存 5 = Agg2D 5 個)、描画 35,172 回。

## 8. 合成演算(PM の最優先項目)

16 スナップ(`cmp-short.g3`、FRUITY、native)の範囲で、FLEngine が Blend2D に渡した合成演算(setCompOp と描画の計数は**置換版のアダプタ**で取った。lift 版では実コンテキストの仮想関数表が実行時に `.data` に作られ〔virt=0x48f4100〕、静的な 6 表の関数に gdb の bp を置いても 0 回しか当たらず捕まえ損ねた。呼び出し列はゲストが決め、置換が忠実なら同じはず):

- Agg2D slot 15(blendMode)の呼び出し: **0 回**(gdb の bp 計数、置換なしの lift 版のバイナリ、`blend/count1.tsv`)。
- BLContext の setCompOp(virt+0x108): **0 回**。描画 35,172 回は**すべて SRC_OVER**(既定): fillGeometry 17,468、fillGeometryRgba32 17,427、strokeGeometryRgba32 277、fillAll と strokeGeometry は 0。ウィンドウ別(g3 の snap の clock_ms で切った、その snap までの区間): wiz0 18,539、a0250 6,762、a0500 3,181、a0750 1,714、a1000 1,008、a1250 120、a1500 126、a1750 114、a4000 339、e0300 2,407、e1300 862(他は 0)。表: `tables/blend-comp-op-stats.txt`。
- よって、この範囲では **JIT 専用の合成演算(24 種)は使われない**。swap1 の静的な手がかり(線形グラデーション+OVERLAY+drawPath(4))の経路は、16 スナップでは通らない(曲の再生・他の画面は未確認)。通る場合は swap1 の自前テンプレート(`b2d-fixedpipe-plus-overlay.diff`)を上流のビルドに足す(1 演算あたり半日の桁、swap1 の推測)。

## 9. やっていないこと・確認できなかったこと

- **検証の対象の木**: §6〜§8 の主な走行・測定は HEAD **291060b** の runtime・harness・生成 C に対するもの。HEAD **071c53f**(hle2、size1、fplow1、wasmgen1、vst2 の取り込み後。載せ直した差分を当てた木 `src2`)でも native のオブジェクトを作り直して次を再実行し、結果は同じだった(`logs/newhead-*`): 走査 142 本(RESULT/CHECK/VALUE+state ハッシュ 230,331 行が lift 版と置換版で一致、291060b の lift 版の行とも一致)、エンジン 16 スナップ(既定の数え方の lift 版が従来の基準 84 行を再現、progress 分離構成の lift 版と置換版が 84 行一致で 291060b の分離構成の行とも一致、置換版の既定の数え方の走行が `B2D1_CHECK=1` で完走)。生成 C は 071c53f でも default 構成で 291060b と同一(各モジュールの SHA が同じ)。**071c53f では再実施していない**: fuzz 種 41〜300、変異検査 16 通り、速さ・起動・attach の測定、wasm の大きさ(これらは 291060b の木)。
- **wasm でのエンジン・走査の実行**。理由: 92 単位の wasm ビルド(決定 0129〜0134 の手順で units 4 分〜+ -O1 の別単位 約 30 分、リンク)に、アダプタと上流の wasm ライブラリを加える作業が、残り時間(総計 4 時間の枠)に入らなかった。アダプタ単体(§6.4)は wasm32 で走らせた。置換版の wasm の速さ・ブラウザでの動作は**未確認**。
- 16 スナップの外(曲再生=Song Mode、他のウィンドウ、プラグイン画面、ユーザー操作)での Blend2D の経路。種の不足は名前つき停止になるが、網羅していない。
- 『ラッパーが読むフィールドの一覧』(`tables/wrapper-reads.tsv`)は逆アセンブルの線形走査+手の分類で、**網羅の証明はない**。漏れがあれば画素か state ハッシュの不一致で出るはずだが、通らなかった経路は検査されていない。
- 実機 Windows との照合 0 件。実機 CPU(FMA3 の libm 変種)での DLL の libm の値は未確認。
- `cbrt`、`sincos` 以外の libm の経路(上流が `exp`・`log` を呼ぶ経路は参照が無かった: `nm -u`)。
- 既存の全体テスト 142 results・FRUITY 無効 a4 は走らせていない(生成 C の同一性で代替: §4)。
- パターン(画像のスタイル)、リージョン、非同期(threadCount > 0)のコンテキスト、ゲストが BLContext の他の API(`blContextEnd` など)を呼ぶ経路: Agg2D のラッパーが使わないと読んだ(swap1 の表)が、アダプタは名前つきで停止する(style type、geometry type)か、表のスロットが 0 で停止する。
- 起動と描画の速さの比較は native のみ。lift 版の wasm の速さ(置換前後)は測っていない。
## 10. 出典なしの選択(`S-b2d1-…`)と撤回条件

| 名 | 内容 | 撤回条件 |
|---|---|---|
| S-b2d1-hybrid | 置換をコンテキスト(18 入口)に限り、オブジェクトモデルは lift のまま | ラッパーが impl を直接読まない版が出る、または上流 wasm32 の impl をゲスト配置にする方法〔memory64 の lowering 等〕が使えるようになったとき |
| S-b2d1-proxy-flags | プロキシの header は refCount=1、flags=0x21(DLL 自身のコンテキストを gdb で読んだ値) | 別の flags でゲストの解放判定が壊れる、または別の版の DLL |
| S-b2d1-virt-table | プロキシの virt 表は DLL の静的 BLContextVirt(RVA 0x1d6380、3 表の最初。17 アドレスが互いに異なる)の関数 VA を使い、その他のスロットは 0 | 表の他のスロットが呼ばれて止まる(名前つき) |
| S-b2d1-mirror | 鏡は状態を変える仮想関数の後に BLContextState 全体を欄ごとにコピーする(targetImage と dashArray は 0)。ラッパーが読む欄は `tables/wrapper-reads.tsv` | 読まれる欄が増えた(鏡の不一致で検出) |
| S-b2d1-seeds | 種は『走らせた範囲で必要だった物』+静的に全数出せる物(§5) | 16 スナップの外で名前つき停止が出た |
| S-b2d1-progress-split | 検証構成で Blend2D の単位の間接呼び出しを progress に数えない(`-D` で変数を付け替え) | 本番で Blend2D の間接呼び出しを数える必要が決まった(その場合、置換は progress を同じに保てない: §6.3) |
| S-b2d1-libm | libm 7 関数をリンク時に DLL の lift した関数へ向ける(sin cos tan atan2 acos pow fmod)。RVA は結果の突き合わせで同定 | 同定が誤りと分かった/cbrt などが走る/実機 CPU の libm 変種と合わせる必要が決まった |
| S-b2d1-upstream-pin | 上流は a7f947677e85441abe1769586d3bb935b4086a07(0.11.1)、tarball SHA-256 `391a57ad2508ab2291ad04b39b0a82d78473662b6d3029dbaf18de06165ea1f0`(2 回の取得で同一)。swap1 が『DLL は 7e32ebf でなく a7f9476』と確定 | DLL が別の版と分かった |
| S-b2d1-timing | 速さは native の CPU 秒、5 回の交互(起動 4 回)、負荷 6.7〜6.9 | 別の機械・負荷 |

## 11. 推奨(採用の判断は PM)

- **採る価値がある条件**: wasm の大きさ(Blend2D 44.0 MB → 2.34 MB〔lift 部 -Oz、約 1/19。outline-shared 後の 19.5 MB 比で 1/8〕。lift 部 -O2 は 3.83 MB)が目的なら、このハイブリッドは native で lift 版と同じ画素・同じ state を出し(§6)、速さも落ちない(描画は約 2 倍速い、起動は同程度)。採用すると『Blend2D の単位が生成する Blend2D 内部の間接呼び出しが progress に入らなくなる』ので、**既定の構成の基準 SHA(従来の 84 行)は変わる**。変えるなら、新しい基準(§6.3 の分離構成。または置換版の既定の構成で新たに取り直した SHA)に更新する決定が要る。
- **条件**: (1) wasm でのエンジン実行(§9)を先に通す。アダプタ単体は wasm32 で通ったが、エンジンの経路・時間は未確認。(2) libm の橋渡しを維持する(上流の更新・別の libm では不一致が戻る)。(3) 種の不足は名前つき停止なので、本番の経路(曲再生など)の網羅試験が要る。(4) 合成演算: 16 スナップでは SRC_OVER のみ。OVERLAY 等を使う場面があれば swap1 のテンプレートを上流に足す。
- **やめる条件**: wasm でのエンジン実行で時間や正しさに問題が出る、または progress を既定のままで置換の前後を一致させる必要が決まった(本作業の範囲では不可能と分かった: §6.3)。
- **試す価値が残る**: progress を『置換した関数が lift 版なら数えたはずの回数』だけ進める(アダプタが固定値や実測の統計で足す)案は、入力依存で厳密にならない(読み)ので試していない。

## 12. 再現(opt-in。通常のテスト・ビルドはネットワークを要求しない)

`docs/investigations/b2d1/tools/README.md`(手順)と MERGE-NOTES.md。要点:
1. `OUTDIR=… TARGET=native|wasm lift/harness/b2d1/build_upstream.sh`(上流の取得と no-JIT ビルド。コミットと tarball の SHA-256 を検査し、違えば停止。**GitHub が生成する tarball の SHA は将来変わりうる**: その場合 `git clone` して `git archive a7f947677e85441abe1769586d3bb935b4086a07` で同じ内容か確認する)。
2. Blend2D の lift: `fe-lift lift Blend2D.4700000.dll m2.c --seeds lift/harness/b2d1/blend2d-4700000-seeds-wrapper.txt --site-overrides lift/harness/hooks/blend2d-no-jit-sites.txt --swap-entries lift/harness/b2d1/blend2d-context-swap.txt`(`--pdata` を付けない)。`gen_two.sh` の MODULES の lift-args にそのまま書ける(`tools/b2d1-config.sh`)。
3. native: `lift/harness/b2d1/link_b2d1.sh`(`b2d_swap.o` と `libblend2d.a` を足してリンク。`MODE=run|scan`)。
4. 検証: `scan_all.sh`・`results_lines.sh`(走査)、`tools/run-native-b2d1.sh` + `tools/hle1/g3lines.sh`(エンジン)、`mutate.sh`(変異)、`test_adapter.cpp`(単体、native と wasm)。

## 13. 規則違反・つまずきの自己申告

- 存在しない `bash` ツールを数回(3 回と記憶)呼んだ(実害なし。shell として実行された)。
- 起動直後、`native-compile` の xargs と gdb 実行と他で、並列プロセスが 4 本を超えた時間帯がある(約 3 分間の 5〜6 本)。gdb の `FinishBreakpoint` 版の実行が 15 分ハングし、手で `kill` した(その間の測定は無い)。
- 最初の gdb 計数は、バイナリにデバッグ情報が無く `R` が実行時に別の記号に解決され、数回やり直した(gdb の結果は §8 の slot 15 の 0 回だけに使った)。
- 上流の wasm ライブラリを `-Oz` でビルドしたつもりが -O2 だった(CMake の Release が -O3 を、Blend2D の CMakeLists が -O2 を足す)。気づいてビルド手順を直し、数値の注記に反映した(§7.1)。最初の予備報告(第 1・2 報)の wasm の大きさ 1.03 MB/2.08 MB は閉包が小さかった版〔種が足りない版〕の値で、**最終の値は §7.1**。
- 第 2 報の『20,000 手 CPU 秒 lift 1.585、置換 0.57〜1.30』は負荷 16 での雑音の大きい予備値で、最終は §7.2(負荷 6.8、交互 5 回)。
- 検証の途中で、種の絞り込み(seeds964 との差し引き)で必要な種を落とし、走査が止まって気づいた(Agg2D の vtable 関数)。直して全体を再実行した(最終の結果は再実行後)。
