# X1: Blend2D は描画に JIT を使うか。使わないなら描画の本体はどの関数で、どのバッファにどの形式で画素を書くか

実行日: 2026-10-30。基準 HEAD `b8ed117`（`git archive` を `lift/out-x1/src/` に展開。作業ツリーは読み取りのみ）。**測定のみ。何も決めない**（推奨は書く。決めるのは PM）。ハーネスの差分は `lift/out-git/x1-results/x1.diff`（`lift/harness/` の 5 ファイル。`lift/runtime`、`lift/crates` は変えていない）、ログは `lift/out-git/x1-results/logs/`。

## 0. 凡例

- **実測** = このセッションでコマンドを実行して見た値（ログは `x1-results/logs/` の下。各節に書く）。
- **読み** = 逆アセンブル（capstone。`tools/x1_dis.py` など）とソースを読んだ解釈。実行して確かめていない。
- **未確認** = 調べていない、または調べたが決められない。
- 「ゲストの Blend2D」= `Blend2D.dll`（SHA-256 `66634d24…66066c7`、測定用の基底 0x4700000 に置き、`fe-lift` で C にしたもの）。アドレスは特に断らなければ**基底 0x4700000 での値**（元の ImageBase 0x180000000 の値 − 0x180000000 + 0x4700000）。
- 機械語の抜粋は §4.2 の 1 か所（11 行）だけ。

## 1. 結論（先に）

| # | 問い | 答え | 区分 |
|---|---|---|---|
| 1 | Blend2D は描画に JIT（AsmJit。実行可能メモリに機械語を生成して呼ぶ）を使うか | **使う（既定では）**。`Agg2D` の `attach` が作る描画コンテキストは JIT 有効（`createInfo` が全 0）。最初の描画呼び出し（`clearAll`、`rectangle`、半透明の塗り）で `VirtualAlloc(0, 0x20000, 0x3000, PAGE_EXECUTE_READWRITE=0x40)` が呼ばれ、書かれた機械語（`clear`／`rect` は非 0 が 1,086 バイト、範囲は +0x40〜+0x4fd）に対して**生成コードを呼ぶ `call [reg]` が `fle_dispatch_miss`（"indirect call to 0x…40 is not a known function entry"）で止まった**。そのアドレスは、ゲストが確保した実行可能メモリの中（領域の +0x40）。`line`／`path`／`ellipse` は JIT のアセンブルの前に別の壁（暗黙の TLS。§4.4）で `bl::Pipeline::JIT assembling error: InvalidState` を出し、`OutputDebugStringA`（行なし。`fle_unsupported`）で止まる | 実測（§4） |
| 2 | JIT を使わない描画の経路はあるか | **ある。Blend2D 自身に**。コンテキストを `BL_CONTEXT_CREATE_FLAG_DISABLE_JIT`（`createInfo.flags` の bit 0）で作ると、**ゲストの C++ のパイプライン**（固定パイプライン。lift した C）で、`clearAll`／塗りつぶし／アルファ合成／線／パス／楕円の **6 手順すべてが完走し、`VirtualAlloc`／`VirtualProtect` を 1 回も呼ばず、画素が期待値どおりに書かれた**（2 回の実行でバッファのハッシュまで同一）。**ただし FL Studio の `Agg2D::attach` は、この旗を立てない**（`createInfo` を全 0 で渡す。読み）。旗を立てた別のコンテキストをハーネスが作り直して差し込む**実験**で測った（ゲストのコードは変えていない。ゲストが自分でこの経路を選ぶわけではない） | 実測（§5）、読み（attach が旗を立てない点） |
| 3 | 描画の本体はどの関数か | **ゲストの Blend2D.dll の内部**。Agg2D のラッパー（vtable 0x48dbec0、130 スロット、異なる関数 79 個）→ BLContext（`blContextInitAs` が作る）→ パイプライン。JIT を使わない経路で画素を書く関数（`fle_st` の先頭の書き手）は **`f_482e8a0`（塗りつぶし、clear）、`f_482f170`（アルファ合成）、`f_482e190`（線・パス・楕円のマスク合成）**（元 0x18012e8a0、0x18012f170、0x18012e190）。FLEngine 側の描画の本体ではない | 実測（§5.3） |
| 4 | 画素バッファへの書き込みは確認できたか | **確認できた**（JIT を使わない経路で 6 手順すべて。JIT 経路は、ホストの CPU に生成コードを実行させる実験で 6 手順すべてが同じ画素になった。§6）。**ただし既定の経路（JIT 有効）は lift した C では完走しない** | 実測 |
| 5 | バッファの形式 | **32 ビット、1 画素 4 バイト、リトルエンディアンで B,G,R,A の順、アルファは乗算済み（PRGB32）、ストライド = 幅 × 4（呼び出し側が渡す。測定は 256）、上から下**（`attach(buffer, w, h, stride)`）。FL Studio 側の `DIB`／`BitBlt` に渡す形としては、ストライド正（トップダウン）の 32 ビット BI_RGB に当たる | 実測（画素値）、読み（形式名） |
| 6 | G4 の方式 | JIT 不使用なら「ビットマップ + BitBlt」で進められる、は**条件つきで成立**（測ったのは 64×64 の単純な 6 手順だけ）: 既定の `Agg2D` は JIT を使うので、そのままでは使えない。**(a) ゲストの Blend2D を JIT 無効のコンテキストで使う（attach の旗を立てる。ゲストの 1 点のパッチか、`blContextInitAs` の呼び出しの介入）、(b) Blend2D を JIT 無効でビルドし直した別物に置換、(c) 描画を別実装、(d) JIT の出力を解釈実行**、が選択肢。見積もりは §8 | 読み・推奨（決めない） |

要するに: **「Blend2D は JIT を使う（既定）。ただし Blend2D 自身が JIT 無しの固定パイプラインを持ち、それは lift した C で動き、単純な 6 手順では JIT 経路と同じ画素を書く（実機との比較は無い）」**。lift できない JIT の機械語を避ける道は**実在する**が、FL Studio の呼び出しはその道を選んでいない。

## 2. 手順（実測）

1. **入力の確認**: FLEngine（SHA-256 `b2c64da4…9496d`）、Blend2D、dsp、QFC、ILTools、freetype、フォント 2 ファイルはすべて残っていた（再抽出は不要）。
2. **基準の再現**: `lift/out-x1/src/lift`（`b8ed117` の `git archive`）の `a4-multi.sh` で 6 モジュール構成を一から作り、a4 の 7 行（`init_count=942 of 942`、`runnable_started=895 completed=895 in_progress=0`、`all_units_done=yes`、functions 5,306、entries 1,007,942、`== stop: returned from entry`、`runtime message: (none)`）、終了コード 0 を**再現した**。native の SHA-256 は **`f469d29f…35219`**（HEAD b8ed117 の木。決定 0116 以降の既定ビルドで、依頼文の `9c324428…` とは別。記録するだけ）。FLEngine の生成 C の SHA-256（註釈あり）は、V1 の検証文書の値 `e047beeb…` と**一致しなかった**（実際は `660b4335…`。決定 0117 の記録と同じ値で、決定 0117 の註釈つき生成 C と読む。`EXPECT_FLE_SHA` を `660b4335…` にして測った。最初の試行は `e047beeb…` を渡したためこの不一致で gen 段階で止まった。`logs/baseline/first-attempt-EXPECT_FLE_SHA-mismatch.txt`）。
3. **静的な調査**（§3）: capstone と pefile（読み取り）。
4. **ハーネスの拡張**（§7）: `FLE_BLEND2D_PROBE=1`／`-DFLE_B2D_PROBE`。`b2d_probe.c` 864 行、`b2d_probe.h` 34 行、既存 3 ファイルへの小さな差分。`lift/runtime` と `lift/crates` は HEAD と同一（`diff -rq` で確認）。
5. **動的な測定**（§4〜§6）: DllMain 完走の後に `AggCreateInstance` を `fle_call_guest` で呼ぶ。

## 3. 静的な調査（読み。実行していない部分を分けて書く）

### 3.1 JIT の痕跡（実測＝文字列・import・PE。意味づけは読み）

- RTTI と文字列に **AsmJit と Blend2D の JIT パイプラインが揃っている**: `asmjit::_abi_1_13::x86::Assembler`／`Compiler`／`JitRuntime`／`X86RAPass`、`bl::Pipeline::JIT::FetchSolidPart`／`FillAnalyticPart`／`FillBoxAPart`／`FillMaskPart`／`CompOpPart`／各種 `Fetch*GradientPart`／`FetchPatternPart`、文字列 `bl::Pipeline::JIT assembling error: %s`（0x1801dd130。使い手は 1 か所 `0x1800a9a46`）、`[Pipeline size: %u bytes]`（`0x1800aad5d`）。**FL Studio 側のラッパー**として `Agg2D`、`Agg2DIntf` の RTTI がある（`.?AVAgg2D@?A0x604ded79@@`）。export は `AggCreateInstance` の 1 つ（RVA 0x59a60）。
- import は KERNEL32 だけ（94 名）。実行可能メモリに関わるのは **`VirtualAlloc`（IAT 0x1801b7038）、`VirtualFree`、`GetLargePageMinimum`、`CreateFileMappingW`、`MapViewOfFile`、`UnmapViewOfFile`**。**`VirtualProtect` と `FlushInstructionCache` は import に無い**（`static/b2d-imports.txt`）。つまり JIT のメモリは `VirtualAlloc` で最初から `PAGE_EXECUTE_READWRITE` で確保する形で、書いた後に保護を変える形ではない。
- `VirtualAlloc` の呼び出し点は 2 か所（`f_470e4d0`＝元 0x18000e4d0 の 0x470e6a7 と、`f_4718250` の 0x4718303）。前者は `flProtect = 0x40`（`lea r9d,[rdx+0x40]`）、`flAllocationType = 0x20003000`（MEM_LARGE_PAGES つき。`GetLargePageMinimum` の結果が要求サイズの約数のときだけ）か 0x3000。後者は引数から 1、2、4、8 のどれかを計算する（既存調査 `blend2d-lift-feasibility.md` の §4.3。この文書の §4.3 とは別）。**測定で実際に通ったのは `f_4718250`（0x4718309。`flAllocationType` 0x3000、`flProtect` 0x40、サイズ 0x20000）**。

### 3.2 コンテキストの作成と JIT の有効・無効の判定（読み）

- `AggCreateInstance`（0x4759a60）は `operator new(0x1170)` して `f_47568a0`（Agg2D のコンストラクタ）へ tail jump する。コンストラクタは vtable 0x48dbec0 を書き、その後 vtable の `+0x1a0`（スロット 52）、`+0x150`（42）、`+0xf0`（30）、`+0xd8`（27）、`+0x160`（44）、`+0x170`（46）、`+0x180`（48）を呼んで既定値（線幅、色など）を作る。
- **`Agg2D::attach`＝ vtable スロット 1（0x4756c90）**: 引数 `(this, buffer, width, height, stride)`（rdx = 画素バッファのポインタ、r8d = 幅、r9d = 高さ、5 番目 = ストライド）。`f_476ea30`（`blImageCreateFromData` 相当。32 ビット深度は `shl r8d,5; add 7; shr 3` で固定、`stride >= width*4`、幅・高さは 1〜65535 を検査）で `BLImage` を作り（Agg2D+8）、続いて **`f_4746ff0`（`blContextInitAs`。0x4756ce9 の 1 か所からだけ呼ばれる）を `createInfo = (rsp+0x60) = 全 0` で呼ぶ**（読み。`xorps; movups [rsp+0x60]; movups [rsp+0x70]` の直後に `lea r8,[rsp+0x60]`）。結果の `BLContext` は Agg2D+0x18。
- `blContextInitAs`（元 0x180146ff0）→ `f_4746040`（元 0x180146040。`ctx.init`）。その中で **`createInfo.flags`（`[r14]`）の bit 0 を `test al,1` で検査し**、立っていなければグローバル 0x48f2ff0（元 0x1801f2ff0。**JIT のパイプライン提供者**。`f_47a9a90`／`f_47a9ad0`／`f_47a9bb0` を持つ。`.xc` の初期化子が `f_47a9eb0` で作る）を、立っていれば 0x48f30e0（元 0x1801f30e0。**固定パイプラインの提供者**。`f_47ac3c0` を持つ。`.xc` の初期化子が元 0x18019aede 以降で作る）を `[ctx+0x608]` に入れる（元 0x18014626f と 0x18014641d の 2 つの `lea r13`）。これは Blend2D の公式ドキュメントの `BL_CONTEXT_CREATE_FLAG_DISABLE_JIT`（`BLContextCreateInfo::flags` の bit 0）と一致する。**ライブラリの版は未確認**（文字列に版番号が無い。RTTI の `_abi_1_13` は AsmJit の ABI 番号）。
- 固定パイプライン側の関数 `f_47ac3c0`（元 0x1800ac3c0）は、`(コンポジット演算, 塗りの種類, 画素形式)` を引数のビットから取り出して、`.rdata` の関数ポインタ表（0x1d6380、0x1d6830、0x1d6da0、0x1d7040、0x1d72e0。**測定した 5 つの表はすべて非 NULL の関数ポインタ**）から選ぶ（読み）。**表が埋まっていること自体は実測**（表の読み出し）、**どの手順がどの表を使うかは読み**。
- **未確認**: Blend2D の JIT が「失敗したら固定パイプラインに自動で落ちる」設計かどうか。測定では、JIT のアセンブルの失敗（§4.4）と `VirtualAlloc` の失敗（§5.4）のどちらでも自動の fallback は**起きなかった**（`line`／`path`／`ellipse` は `OutputDebugStringA` を無視して続けても画素 0 件、`clear` は `VirtualAlloc` が 0 を返すと画素 0 件）。

### 3.3 Agg2D の vtable と描画の入口（実測＝表の読み出し。役割は読み）

- vtable 0x48dbec0（元 0x1801dbec0）: **130 スロット、異なる関数 79 個**（`static/agg-vtable.txt`、`agg-vtable-funcs.txt`）。そのうち 49 スロットは同じ 1 つの関数（`ret`、0x18000c530）、3 スロットは `xor eax,eax; ret`（0x18000b820）。つまり**実体のある入口は約 77 スロット**。
- 役割は逆アセンブルから読んだもの（各ラッパーが `[this+0x18]`＝BLContext の vtable の何番を、どんな定数で呼ぶか）:

| スロット（変位） | 読み | FLEngine の使用箇所数（既存の下限。`fl-vtuse.txt`） |
|---|---|---:|
| 1 (+0x08) | `attach(buf, w, h, stride)`（BLImage と BLContext を作る） | — |
| 8 (+0x40) | `clearAll(r, g, b, a)`（BLContext の `+0x198`＝塗りつぶし色 → `+0x108`） | — |
| 26〜31 (+0xd0〜+0xf8) | `fillColor`／`lineColor`／`noFill`／`noLine`（`Agg2D+0x118`／`+0x11c` に 32 ビットの色を持つ） | 10／5／1／11 |
| 42 (+0x150) | **線幅ではない**: `BLContext` の vtable `+0x118` を呼び `Agg2D+0x59` を 0 にする（役割は未確認。コンストラクタは xmm1 に定数を入れて呼ぶ） | 13 |
| 61, 63, 65, 67〜73 (+0x1e8〜) | `line`／`rectangle`／`roundedRect`／`ellipse`／`arc`… のたぐい（`f_4759490`＝汎用の fill／stroke を呼ぶ。stroke は `+0x238`／`+0x240`、fill は `+0x1a8`／`+0x1b0`） | 2／7／1 |
| 82〜103 (+0x290〜+0x338) | パス: `resetPath`、`moveTo`（0x4773590）、`lineTo`（0x4773650）、`closePolygon`（0x4774510）、…、**`drawPath(flag)`＝スロット 103**（flag 0 = fill、1 = stroke、2 = 両方） | 19／10／19／3／12 |
| 121 (+0x3c8) | `ellipse(cx, cy, rx, ry)`（FLEngine の `P_DigitWheel`＝`f_127d420` がこれを呼ぶ） | 8 |
| 3 (+0x18) | `clipBox` | 3 |

- **FLEngine の呼び方**（既存の読み＋本調査）: `f_6cb190`（呼び出し元 39 か所）が Agg 保持オブジェクトの `[+0x58]` を返し、0 なら `f_6cb0b0` が `AggCreateInstance`（`[0x15e3990]` の指すスロット）を呼ぶ。作成後、`f_6cb020` が `attach` を `(this=Agg, buffer=[owner+0x448] または [obj+0x100], w=[obj+0x80], h=[obj+0x84], stride=[obj+0x78])` で呼ぶ。**バッファは FLEngine 側の `TDIBmp` 系のオブジェクトが持つ DIB の画素**（`CreateDIBSection` の `ppvBits`。読み）。FLEngine の呼び出し点は 130（77 関数）で、描画系の 2,243 関数（goal-ladder §3.3 E4）が Agg2D の上流。

## 4. 動的な測定（1）: 既定の経路（`Agg2D::attach` が作る JIT 有効のコンテキスト）

測定の道具: `FLE_BLEND2D_PROBE=1`（`lift/harness/b2d_probe.c`。§7）。DllMain 完走の後、ゲストの `AggCreateInstance` を呼び、`attach(buffer, 64, 64, stride 256)` し、手順を呼ぶ。ゲストが呼ぶ `VirtualAlloc`／`VirtualProtect`／`CreateFileMappingW`／`MapViewOfFile` を `-Wl,--wrap` で記録する（応答は変えない。runtime は呼ぶだけ）。ログ: `logs/final/agg-*.txt`。

### 4.1 静的な到達の壁を越えるための種の追加（実測。測定の道具の話）

既存の測定（`blend2d-seed58.md`）の種 58 個だけでは、`AggCreateInstance` を呼ぶと**ゲストの Blend2D の `jmp reg` の switch 表の移動先が生成 C に無く**、`fle_dispatch_miss` で止まった（最初の停止 `0x4756400`＝`.pdata` に無い葉関数）。そこで**測定用の種を足した**（`fe-lift` も runtime も変えない。`--seeds` に VA を渡すだけ。ファイル `x1-results/seeds/b2d-seeds-final-4700000.txt`、964 行）:

| 版 | 種 | 生成 C の関数 | 停止（`step clear`） |
|---|---|---:|---|
| 58 | `seeds58`（既存） | 2,066（既存調査の値。DllMain 後の `AggCreateInstance` は 7 入口で `f_4756400` に当たる。実測） | `indirect call to 0x4756400`（`AggCreateInstance` の中。`.pdata` に無い葉関数） |
| 110 | + `lea` で取られる `.pdata` 外の葉関数 52（`seeds/lea_leaf_candidates.py` の出力） | 2,119 | `indirect call to 0x4781377` |
| …… | + `fle_dispatch(R[x])` のまま残った `jmp reg` の switch 表の移動先を、元の DLL の表から読んで足す（`tools/x1_switch_seeds.py`。反復 6 回。**メモリ移動関数 `f_48a2060` の内側には足さない**: 足すと `fe-lift` がその関数の最初の switch の解決を拒否する） | 2,451〜3,093 | `0x48a25b1`、`0x4746e8d`、`0x474be50`、`0x474c5c9`、`0x471d431`（各回 1 か所ずつ別の switch の表） |
| 964 | 最終 | **3,093**（命令 432,647、C 101.4 MB、`unsupported_total` 643、`msvc_tables` 24、未解決の `jmp reg` サイト 81） | **`VirtualAlloc(EXEC)` → 生成コードの呼び出し**（§4.2） |

読み: これは **B3（switch 表。blend2d-lift-feasibility.md）の実測の裏づけ**で、DllMain の `.xc`／`.xi` だけでなく描画経路でも `jmp reg` の switch が大量に実行される。静的な閉包は上界でも下界でもなく、**実行して止まった所の表を足す方式**でここまで来た。種を 964 個に増やした生成 C は、wasm の `local count too large`（B2 の 4 関数。locals 97,904〜102,086。元 0x180159b70、0x180171d60、0x180165f60、0x18017e100）を引き起こす関数を**含む**。JIT 無しのコンテキストで 6 手順を単独で走らせ、各関数の入口の回数を数えた（`logs/gdb/gdb-bigfn-*.txt`。実測）: `f_4865f60`（元 0x180165f60）は `line` で 1 回、`f_487e100`（元 0x18017e100）は `line` で 2 回・`path` で 1 回・`ellipse` で 1 回、残り 2 関数は 0 回。**`clear`／`rect`／`blend` では 4 関数とも 0 回**。したがって B2 は描画の**線・パス・楕円**で実際に使われる（locals の上限内の `f_4776f60`＝元 0x180076f60 も `line` で 1 回）。

### 4.2 最初の描画で JIT が実行可能メモリを確保し、生成コードを呼ぶ（実測）

`FLE_B2D_OPS=clear`（`clearAll(0x11, 0x22, 0x33, 0xff)`）。ログ: `logs/final/agg-all.txt`、`agg-only-clear.txt`。

```
b2d: AggCreateInstance returned 0x…ae0; object vtable 0x48dbec0 (is the Agg2D vtable at RVA 0x1dbec0); 33 lifted entries so far
b2d: attach(…, 64, 64, stride 256) returned; … 60 lifted entries so far; memory events 0 (executable 0)
b2d: BLImage impl: data … (the buffer is …), stride 256, width 64, height 64, format/flags/depth dword 0x200001
b2d: BLContext impl+0x608 (pipe provider the context took) = 0x48f2ff0 (the JIT provider)
b2d: step clear: begin (buffer fnv1a 0x9c1bda7f8c872325)
b2d: [clear] VirtualAlloc(0, 0x20000, 0x3000, 0x40) EXEC caller=0x4718309
b2d: == stop: SIGABRT (runtime abort, see runtime message)
b2d: runtime message: fle: indirect call to 0x7891477a0040 is not a known function entry
b2d: abort kind: fle_dispatch_miss
b2d:   inside the VirtualAlloc region [0x7891477a0000, 0x7891477c0000), alloc_protect=0x40, page protect=0x40 (EXECUTABLE)
b2d:   region #0: 1086 non-zero bytes of 131072, last non-zero byte at offset 0x4fd (a fresh VirtualAlloc region is all zero)
b2d: elapsed 0.01 s; lifted entries since the probe began: 24484; new distinct functions: 404
b2d: pixel buffer at 0x…: 0 of 16384 bytes are non-zero
```

- **(a) `VirtualAlloc(EXEC)` が出るか: 出る**。`PAGE_EXECUTE_READWRITE`（0x40）、0x20000 バイト、`MEM_COMMIT|MEM_RESERVE`、ゲストの呼び出し元 `0x4718309`（`f_4718250`）。`VirtualProtect` は描画中に 0 回（import に無い）。**`attach` までは 0 回**（`attach` は 60 入口で戻り、メモリ操作 0。`logs/final/agg-attach-only.txt`）。最初の描画の手順の中で初めて出る。`clear`／`rect`／`blend` の 3 手順を単独で走らせても同じ形（`agg-only-*.txt`。領域のアドレスだけが実行ごとに違う）。
- **(d) 生成コードを呼ぶ `call [reg]` の行き先**: 確保した領域の **+0x40**。領域の最初の 0x40 バイトは 0、+0x40 から機械語（`clear`／`rect` は最後の非 0 が +0x4fd＝1,214 バイトの範囲、`blend` は +0x234＝501 バイト）。**行き先は実行可能メモリの中で、`fle_dispatch` の表（lift した関数の入口）には無い**ので `fle_dispatch_miss`。
- **(c) 停止メッセージと C スタック**: メッセージは上のとおり（終了コード 134、SIGABRT）。ゲストの影の呼び出しスタック（実測）: `f_4857930`（元 0x180157930＝コンテキストの塗りつぶしの入口。`call [rsp+0x40]` で JIT 関数を呼ぶ）← `f_4848c80`（元 0x180148c80）← `f_4756dd0`（Agg2D の vtable スロット 8 `clearAll`）。C スタック（`addr2line` で記号化。`logs/final/agg-all.cstack-symbolised.txt`）:

```
report ← on_signal ← (SIGABRT) ← fle_dispatch_miss ← fle_dispatch ← f_4857930 ← f_4848c80
  ← fle_dispatch ← f_4756dd0 ← fle_dispatch ← fle_call_guest ← b2d_probe_run ← main
```

- **生成コードの内容**（読み。gdb で領域をダンプして capstone で逆アセンブル。`logs/gdb/jit-region-*.txt`）。`clear` の関数の先頭（Win64 の引数: `rcx`＝パイプラインの文脈、`rdx`＝箱 x0,y0,x1,y1、`r8`＝色）:

```
040: push rbx ; push rbp ; sub rsp,0x28 ; movaps [rsp],xmm6 ; movaps [rsp+0x10],xmm7 ; pxor xmm6,xmm6
053: mov r11d,0x48d8000                 ; 定数表
059: movaps xmm2,[r11+0xa0] ; movaps xmm7,[r11+0x100]
069: mov rbp,[rcx+8]                    ; ストライド
06d: mov ebx,[rdx+4]                    ; y0
070: mov r10d,[rdx]                     ; x0
073: mov r9,rbp ; imul r9,rbx           ; y0 * stride
07a: movd xmm0,[r8] ; pshufd xmm0,xmm0,0 ; movaps xmm1,xmm0 ; punpcklbw xmm1,xmm6   ; 色を 4 画素分に
08b: lea r9,[r9+r10*4] ; neg r10d ; add r10d,[rdx+8] ; neg ebx                       ; x0*4 を足す、幅・高さの計算
```

  6 手順の生成コード（501〜1,415 バイト）に現れる命令は**合計 53 種**で、すべて通常の整数・SSE2 命令（`pmullw`、`pmulhuw`、`packuswb`、`punpck*`、`pshufd`、`psllw`、`paddw`、`movups`、`cmova`、`bsf`、`imul` ほか。**AVX・FMA・AVX-512 は 0**。実測の逆アセンブルから数えた。JIT が選ぶ命令集合は CPUID の応答に依存し、この環境の CPUID は AVX を名乗らない、と読む）。
- 生成コードは画素バッファのアドレスを内部に持たず、パイプライン文脈（ストライド）と箱と色から計算して、**ゲストのデータ構造（BLImage の画素）を直接読み書きする**（読み）。ホストの CPU が実行すれば画素バッファに直接書く（§6 で実測）。

### 4.3 起動時（DllMain）の実行可能メモリは JIT ではない（実測）

描画の前（DllMain と、その中で読み込まれる 5 モジュールの初期化）の `VirtualAlloc`／`VirtualProtect` を gdb で全部数えた（`logs/gdb/pre-probe-VirtualAlloc-VirtualProtect-by-module.txt`。ゲストの戻りアドレスのモジュールで分類）: `VirtualAlloc` 26 回のうち実行可能なもの 2 回（FLEngine `0x52a5e2`、ILTools `0x61045e2`。どちらも 0x1000 バイト、`PAGE_EXECUTE_READWRITE`。FLEngine のものは `MakeObjectInstance`＝決定 0071 の WndProc のトランポリン）、`VirtualProtect` 18 回はすべて実行可能な保護（FLEngine 6、ILTools 6、QFC 6。5 バイトのフック点＝決定 0105 の仕組み）。**Blend2D 自身は DllMain の間に `VirtualAlloc` も `VirtualProtect` も 0 回**。goal-ladder.md §3.8 M-10 の「a4 の完走で実行可能メモリの確保は `MakeObjectInstance` の 2 回のみ」と**回数は一致**する（ILTools のものも同じ形と読む。呼び出し元の関数の同定はしていない）。**JIT は起動では使われず、最初の描画で初めて走る**（実測）。

### 4.4 線・パス・楕円: 暗黙の TLS の壁と、JIT のアセンブルの失敗（実測）

`FLE_B2D_OPS=line`（`path`、`ellipse` も同じ）、既定の経路（`logs/final/agg-only-line.txt`、`logs/gdb/line-OutputDebugStringA-string.txt`）:

```
b2d: runtime message: fle: unsupported instruction at 0x489b1b5      （OutputDebugStringA の call [IAT]。応答表に行なし）
gdb: R[1] の文字列 = 'bl::Pipeline::JIT assembling error: InvalidState\n'
```

- 停止は `OutputDebugStringA`（元 0x18019b1b5）。引数の文字列を gdb で読むと **`bl::Pipeline::JIT assembling error: InvalidState`**（AsmJit がエラーを返し、Blend2D が `OutputDebugStringA` に出して続行する形）。影のスタックは `f_489b170` ← `f_489b140` ← `f_47a9a40`（JIT 提供者の関数）← …
- **原因の読み**: JIT 提供者の関数は暗黙の TLS（`gs:[0x58]` から `_tls_index` 番目のブロック）を読む。ランタイムは `_tls_index` を書かない（既存調査 B6）。**実測の裏づけ**: `FLE_B2D_TLS=1`（**ハーネスが Windows のローダーのやることを真似る**: TLS テンプレート 32 バイトを新しいブロックにコピーし、`fle_tls_values[1000]` に置き、`_tls_index` に 1000 を書く。ランタイムは変えない）を付けると、`line`／`path`／`ellipse` も `clear` と同じ形に変わる（`VirtualAlloc(EXEC)` が出て、領域の +0x40 の呼び出しで `fle_dispatch_miss`。領域の最後の非 0 は +0x5c6＝1,415 バイトの範囲。`logs/final/agg-tls-only-*.txt`）。`OutputDebugStringA` の止まりは消える。
- つまり**JIT の道は 2 重に塞がっている**: (1) 暗黙の TLS が未モデルなので、線・パス・楕円では JIT のアセンブルが失敗する（TLS を足せば越える）、(2) アセンブルが成功しても、生成コードは lift できず実行できない。(2) が本質。

## 5. 動的な測定（2）: JIT を使わない経路（BL_CONTEXT_CREATE_FLAG_DISABLE_JIT）

### 5.1 実験の作り（読み＋実測。**ゲストのコードは変えていない**）

`FLE_B2D_MODE=nojit`: `attach` が作った JIT 有効のコンテキストの後で、ハーネスが **ゲストの `blContextInitAs`（0x4746ff0、`createInfo.flags = 1`）を直接呼んで**別の `BLContext` を作り、Agg2D+0x18／+0x20 に差し込み、`+0x110`（`setGlobalAlpha` 風）と `+0x98` の 2 つの初期化（`attach` が `blContextInitAs` の後に行うことと同じ。0x4756d29、0x4756d40 の読み）を行う。実測: `blContextInitAs` は 0 を返し、新しい `BLContext` の `impl+0x608`（パイプライン提供者）は **0x48f30e0＝固定パイプラインの提供者**（`b2d: nojit: context replaced; BLContext impl+0x608 = 0x48f30e0 (the fixed (non-JIT) provider)`）。**FL Studio の呼び出しがこの旗を立てるわけではない**（`attach` は旗なし。§3.2）。この節は「Blend2D 自身が JIT 無しで描ける」ことの実測で、「FL Studio がそう描く」ことの実測ではない。

### 5.2 6 手順すべてが完走し、`VirtualAlloc` は 0 回、画素は期待値どおり（実測）

`logs/final/nojit-all.txt`（終了コード 0、SUMMARY: `steps 6, memory events 0, executable 0`、lifted entries 215）。バッファは 64×64（16,384 バイト）、手順の前は全 0:

| 手順（Agg2D の呼び出し） | 変わった画素 | 測った画素（メモリ順 B G R A） | 期待値（独立に計算） | 一致 |
|---|---:|---|---|---|
| `clearAll(r=0x11, g=0x22, b=0x33, a=0xff)` | 4096 / 4096 | (0,0) (32,32) (63,63) すべて `33 22 11 ff` | BGRA = 33 22 11 ff | ○ |
| `fillColor(255,0,0,255)`、`noLine`、`rectangle(8,8,40,40)` | 1024 | (20,20) と (8,8) `00 00 ff ff`、(50,50)（外）`33 22 11 ff` のまま | 赤の PRGB32 = 00 00 ff ff | ○ |
| `fillColor(0,0,255,0x80)`、`rectangle(24,24,56,56)`（半透明の青を重ねる） | 1024 | (30,30)（赤の上）`80 00 7f ff`、(50,50)（背景の上）`99 11 08 ff` | 乗算済み α の source-over（丸め込み）: 赤の上 80 00 7f、背景の上 99 11 08 | ○ |
| `lineColor(0,255,0,255)`、`line(2,60,60,2)` | 349 | (31,31) `00 ff 00 ff`、(5,5)（外）`33 22 11 ff` のまま | 緑 = 00 ff 00 ff | ○ |
| `fillColor(r=0,g=255,b=255,a=255)`、`resetPath`、`moveTo(10,50)`、`lineTo(50,50)`、`lineTo(30,10)`、`closePolygon`、`drawPath(0)` | 840 | (30,40)（三角形の内）`ff ff 00 ff`、(5,5)（外）`33 22 11 ff` | シアン（R=0, G=ff, B=ff）のメモリ順 B,G,R,A = ff ff 00 ff | ○ |
| `fillColor(r=255,g=255,b=0,a=255)`、`ellipse(32,32,20,12)` | 800 | (32,32) `00 ff ff ff`、(2,2)（外）`33 22 11 ff` | 黄（R=ff, G=ff, B=0）のメモリ順 B,G,R,A = 00 ff ff ff | ○ |

注: 「期待値」の列は、引数の色（Agg2D の `fillColor`／`lineColor` の最初の 3 引数を r, g, b と読んだ）から**ハーネスの外で手計算した**もの。メモリ順 B,G,R,A は clear（r=0x11,g=0x22,b=0x33 → `33 22 11`）と赤（r=0xff → byte2）で決まる。**独立に計算して画素値まで比べたのは半透明の合成だけ**（`80 00 7f` と `99 11 08` が source-over＋乗算済みアルファの計算と一致）。他の行は「引数の色が正しいチャンネルに入った」ことの確認で、**図形の縁のアンチエイリアスの画素値の正しさは検証していない**（変わった画素数 1024／349／840／800 は実測だが、期待する画素数の独立の計算は長方形 32×32 = 1024 を除いてしていない）。
- **再現性**: nojit の実行を 2 回行い、各手順のバッファのハッシュと画素は同一（`logs/final/nojit-all.txt` と、`nojit-rep` の 2 回）。

### 5.3 描画の本体の関数（実測）

nojit の各手順について、画素 1 つにハードウェア・ウォッチポイントを置き、**最初に書いたゲストの関数**を gdb で読んだ（`logs/final/pixel-writers-nojit.txt`。C の `memcpy`＝`fle_st` の直上のゲスト関数）:

| 手順 | 書いた関数（lift 後。元の VA） | 呼び出しの連鎖（上位 3） |
|---|---|---|
| clear | `f_482e8a0`（0x18012e8a0） | `f_4857930` ← `f_4848c80` ← `f_4756dd0`（Agg2D vtable 8 `clearAll`） |
| rect | `f_482e8a0`（同じ） | `f_4840d30` ← `f_484b4c0` ← `f_4757bd0`（vtable 63 `rectangle`） |
| blend | `f_482f170`（0x18012f170） | `f_4840d30` ← `f_484b4c0` ← `f_4757bd0` |
| line | `f_482e190`（0x18012e190） | `f_4839190` ← `f_4840740` ← `f_4844460` ← `f_484d050` |
| path | `f_482e190`（同じ） | `f_4839190` ← `f_4840740` ← `f_4840d30` ← `f_484b4c0` |
| ellipse | `f_482e190`（同じ） | `f_4839190` ← `f_4840740` ← `f_4840d30` ← `f_484b4c0` |

**描画の本体は 3 つの小さな画素書き込み関数**（ベタ塗りの行、アルファ合成の行、マスクつきの合成）で、その上にコンテキストの文脈（`f_4840d30`／`f_484b4c0`＝塗りつぶしの共通入口）と、アナリティックなラスタライザ（`f_4839190`／`f_4840740`＝パスから被覆マスクを作る。B2 の巨大関数 `f_4865f60`／`f_487e100`／`f_4776f60` は線・パス・楕円で使われる）がある（読み＋実測。ラスタライザの機能の同定は読みで、確認していない）。

### 5.4 JIT が使えないとき Blend2D は自分で固定パイプラインに落ちるか: 落ちない（実測。gdb で応答を差し込んだ実験）

ランタイムは変えず、gdb で応答だけを差し込んだ（`tools/x1_gdb_fallback.py`、`logs/fallback/`）:

| 実験 | 結果 |
|---|---|
| `VirtualAlloc(EXEC)` が 0 を返し last error 1655（`ERROR_DYNAMIC_CODE_BLOCKED`＝動的コード禁止の緩和策を有効にしたプロセスの見え方）と答える。`clear`／`rect` | 手順は**戻る**（`lifted entries 24405`）が、**画素は 0 件変わらない**（バッファのハッシュ不変、全 0）。固定パイプラインへの自動 fallback は**起きない**（描画が黙って捨てられる） |
| JIT のアセンブル失敗（暗黙の TLS が未モデル）で `OutputDebugStringA` を無視して続ける（3 回の呼び出しをスキップ）。`line`／`path`／`ellipse` | 同じく画素 0 件（手順は戻る） |

つまり、**Blend2D の JIT が使えない環境では、FL Studio の `Agg2D` は何も描かない**（読み。実測は上の 2 つの注入の結果）。これは実機の Windows でも JIT が失敗する環境（動的コード禁止）なら同じになるはずだが、**実機での確認はしていない**。FL Studio がそのような環境で動くかは未確認。

## 6. JIT の機械語をホストの CPU で実行すると、画素は同じになるか（実験。ブラウザでは不可）

`tools/x1_gdb_hostjit.py`: 既定の経路（JIT 有効）で、`fle_dispatch_miss` で止まったとき、gdb から `b2d_native_call(<生成コードの行き先>)`（ハーネスの小さな関数。Microsoft x64 の呼び出し規約で、ゲストの `R[1]`、`R[2]`、`R[8]`、`R[9]` を引数に、ホストの CPU に**そのまま実行させる**）を呼び、その後の画素を読む。`clear`／`rect`／`blend`は暗黙の TLS なしで、`line`／`path`／`ellipse` は `FLE_B2D_TLS=1` で測った。

| 手順 | ホストが生成コードを実行した後の画素バッファのハッシュ | §5 の JIT 無しの経路（lift した C）の同じ手順 1 つだけのハッシュ | 一致 |
|---|---|---|---|
| clear | `0x5ebb39d31700c325` | `0x5ebb39d31700c325` | ○ |
| rect | `0x8fd7827920a68b25` | `0x8fd7827920a68b25` | ○ |
| blend | `0xe792492cee872325` | `0xe792492cee872325` | ○ |
| line | `0x856aca8f5023b3ad` | `0x856aca8f5023b3ad` | ○ |
| path | `0x142226d42263cae5` | `0x142226d42263cae5` | ○ |
| ellipse | `0xec9cfb49536dbf21` | `0xec9cfb49536dbf21` | ○ |

（各行は**1 手順だけ**を単独で実行したときのハッシュで、§5.2 の全手順を通した累積のハッシュとは別。blend 単独は背景が 0 なので画素値が違う: `80 00 00 80`。）**生成コードが書く画素と、JIT 無しの C の画素は、6 手順すべてでバイト単位で同じ**（実測）。さらに、全 6 手順を通して、各 JIT 呼び出しをホストで実行させて続ける一連の実行（`logs/hostjit/loop-all-steps.txt`、ホスト実行 6 回）では、**§5.2 の累積のハッシュと画素がすべて一致**した（`0x5ebb39d31700c325`、`0xd1747fb20144b325`、`0x31decd4681241725`、`0xbc1b3579cbfd7ed9`、`0x8214af8f678e1357`、`0xc6a618135d4ad014`）。

読み: **JIT 経路と固定パイプラインは画素の結果が同じ**（少なくともこの 6 手順の単純な図形で。グラデーション、パターン、他の合成演算、テキストは測っていない）。したがって「JIT 無しの経路に切り替える」ことで、**実機と同じ画素を保てる見込みがある**（実機の Windows の出力との比較はしていない。JIT 経路と固定経路の一致だけが実測）。

**この節はブラウザで可能という意味ではない**。ホストの CPU に機械語を実行させたのは native の gdb の実験で、wasm には実行可能メモリに書いたバイト列を実行する手段が無い。

## 7. ハーネスの拡張（opt-in。未設定なら従来と同一）

- 追加: `lift/harness/b2d_probe.c`（864 行）、`b2d_probe.h`（34 行）。変更: `lift/harness/two/main_two.c`（`#ifdef FLE_B2D_PROBE` の 4 か所）、`two/build_two_native.sh`（`B2D_PROBE=1` の節。`-Wl,--wrap=fle_imp_VirtualAlloc` ほか 6 つ）、`a4-multi.sh`（`FLE_BLEND2D_PROBE=1`。独自の TAG、`---- b2d` の報告ブロック。`blend2d.dll` が無いモジュール表、`FLE_CALL_FRUITY=1` との併用、`FLE_B2D_*` の単独指定は拒否）。**`lift/runtime`、`lift/crates` は変えていない**（`b8ed117` の `git archive` と `diff -rq` で同一）。差分は `b8ed117` と HEAD `9013ad5` の両方に `patch -p1 --dry-run` で当たる。
- 環境変数（`b2d_probe.h` に全文）: `FLE_BLEND2D_PROBE=1`、`FLE_B2D_MODE=agg|nojit`、`FLE_B2D_OPS=clear,rect,blend,line,path,ellipse|none`、`FLE_B2D_TLS=1`（実験）、`FLE_B2D_MAX_CALLS`、`FLE_B2D_SECONDS`、`FLE_B2D_TRAP=1`（SIGTRAP で gdb に渡す）。
- **従来と同一であることの確認（実測）**: (1) プローブのコードが入らないビルド（`B2D_PROBE` なし）で、**`main_two.o` がオリジナルの `main_two.c` から作ったものとバイト一致**（`cmp`）。(2) 変更後のハーネスで作った既定のビルドの native の SHA-256 は **`65c904e3…f96d9`**、オリジナルのハーネスから作り直した既定のビルドも**同じ `65c904e3…f96d9`**（a4 の 7 行、functions 5,306、entries 1,007,942、終了 0 も同一）。基準の 1 回目のビルド（OUT `o1`、native の SHA `f469d29f…35219`）と、`o5` で作った 2 つ（`65c904e3…f96d9`）は、**同じ HEAD の同じ入力なのに SHA が違う**。原因は**未確認**。推測: ビルドしたパス（OUT の名前）が `-g` つきのデバッグ情報や文字列に入るため（`strings` で `o5` のバイナリに `lift/out-x1/o5/…` が 221 か所ある。実測。`o1` のバイナリは削除済みで比べていない）。**ハーネスの変更では SHA は変わらない**: 同じ OUT 名 `o5` で、変更後のハーネスとオリジナルのハーネスから 1 回ずつ作って SHA が一致した。a4 の 7 行と functions／entries／終了コードは `o1`・`o5`（2 つ）の 3 つで同一（実測）。 (3) プローブのビルドでも、`FLE_BLEND2D_PROBE` 未設定／`=0` の実行は終了 0、`returned from entry`、`init_count=942 of 942`、`895/895`、`all_units_done=yes` で、2 つの実行（未設定と `=0`）の標準出力は同一（`logs/optin/`）。**functions／entries は 5,313／1,007,954**（基準は 5,306／1,007,942）で、このビルドは Blend2D の種を 964 個にした生成 C を使うため違う（原因の同定はしていない。§9-4）。(4) `FLE_B2D_OPS` だけの指定、`FLE_BLEND2D_PROBE=2` は終了 2 で、ゲストを走らせる前に拒否（`logs/optin/refused.*`、`bad.err`）。

## 8. 結論の続き: 描画の見込みと G4 の方式（推奨は書く。決めない。見積もりは桁だけで、日数・人月には換算しない）

### 8.1 FLEngine の `Agg2D` ラッパーが 1 フレームの描画に何を呼ぶかの見込み（読み。未実行）

- FLEngine は描画のたびに `f_6cb190` で Agg を得て、**自分の DIB の画素を `attach(buf, w, h, stride)` で Blend2D に渡し**、`fillColor`／`lineColor`／`rectangle`／`ellipse`／パス（`moveTo`／`lineTo`／`drawPath`）／`clipBox`／座標変換を呼ぶ。**FLEngine の呼び出しの下限**（`fl-vtuse.txt`、130 呼び出し点のうち 81 点の vtable 使用）で多いのは: パス系のスロット 82・85（各 19）、103 `drawPath`（12）、31 `noLine`（11）、26 `fillColor`（10）、42（13。役割は未確認）、83（10）、11（8）、121 `ellipse`（8）、63 `rectangle`（7）。文字・画像の貼り付けは Agg2D の入口に**見当たらない**（Blend2D.dll の文字列に `font`／`glyph` は 0 件。画像・テキスト用のスロットは読んでいない）。したがって **Blend2D の描画は「ベクタ図形（矩形・楕円・パス）の塗りと線」が中心で、文字は別経路**（goal-ladder §3.3 の E4〜E7 の読みと整合する）。**ただし FLEngine の 2,243 関数のうち、実際に 1 フレームで何回どの入口が呼ばれるかは未測定**（X3 の窓の領域に届かないと測れない）。
- 画素の出力先は **FLEngine 側の DIB（`CreateDIBSection` の `ppvBits`）で、Blend2D はそこへ直接書く**（読み。`attach` の画素ポインタ＝`[owner+0x448]` または `[obj+0x100]`。**未確認**: これが `CreateDIBSection` のビットと同じポインタか）。窓への最終の転送は、goal-ladder §3.3 E2 の `f_6bb1b0`（`SetDIBitsToDevice` か `BitBlt`）で、**この部分は Blend2D の外**で、現状のモデルに無い（窓の DC への転送。M-4）。

### 8.2 G4（最初の 1 フレーム）の方式: 選択肢と見積もり

| 案 | 内容 | 根拠（実測／読み） | 規模の見積もり（桁。弱い） | 実機との同等性 |
|---|---|---|---|---|
| **A** | ゲストの Blend2D を **JIT 無効のコンテキスト**で使う。`Agg2D::attach` が `blContextInitAs` を呼ぶ 1 か所（元 0x180056ce9。`createInfo` を全 0 で渡す）で `flags = 1` にする。方法は 3 つ: (A1) `Blend2D.dll` のバイト列の 1 点のパッチ（`xorps; movups [rsp+0x60]; movups [rsp+0x70]` の `createInfo` の初期化に `flags = 1` を入れる。lift 前のバイナリを書き換える＝**決定 0073／0055 の「実物をそのまま使う」からは外れる**）、(A2) `fe-lift` の hook point（決定 0105。FLEngine の 5 バイトのフック点の仕組みを Blend2D に使う）で `createInfo` を差し替える、(A3) ランタイム側で `blContextInitAs` の入口に介入（今回のハーネスの nojit の形） | **§5 実測**: JIT 無しの固定パイプラインは lift した C で動き、6 手順の画素が JIT 経路と**バイト単位で同じ**（§6）。VirtualAlloc は 0 回。**未実測**: グラデーション、パターン、画像の貼り付け、他の合成演算（Agg2D のブレンドモード 23 種のうち固定パイプラインが受けるもの）、`clipBox` の複雑な形、テキスト。固定パイプラインの表（5 つ）はすべて非 NULL（§3.2。実測）が、手順ごとに通るかは未測定 | **小〜中**。(A2)(A3) はハーネス／生成器の局所的な追加（今回の測定のハーネス 864 行が先例。ただし本番のランタイムに入れる設計は別）。**大きい仕事は Blend2D の lift の完成**: (i) 暗黙の TLS のモデル（B6。JIT のアセンブルは TLS が無いと失敗する（§4.4）。**JIT 無しの経路は、`FLE_B2D_TLS` を付けない既定のままで 6 手順が完走した**（`logs/final/nojit-all.txt` に `TLS scaffold` の行は 0 件。実測）ので、案 A なら TLS のモデルは**不要な見込み**。ただし DllMain の `.xc` が TLS を読む経路は既存の測定で通っている）、(ii) **switch 表の取りこぼし**（今回は 964 個の種を、実行して止まる度に足して作った。**残りの未解決の `jmp reg` は 81 サイト**。描画の他の経路（グラデーション、画像）で別の switch に当たる可能性が高い。生成器の B3 の解決を広げるか、種の作り方を一般化する仕事）、(iii) wasm の locals 50,000 超（B2 の 4 関数。**JIT 無しの線・パス・楕円で実際に使われる**（§4.1）ので、-O1 の分離（`build_nmod_wasm.sh` の o1 段）が要る。ただし 12 分の wasm の再ビルドは既存の流れ） | JIT 経路と同じ画素（実測の範囲で）。実機の画素との比較は未 |
| **B** | Blend2D を JIT 無効で**ソースからビルドし直した別物**に置換（Blend2D は OSS。AsmJit を使わずビルドできる設定がある（公式ドキュメントの `BL_BUILD_NO_JIT`。**本調査では確認していない**）。FL Studio 向けの `Agg2D` ラッパーは**ソースが無い**ので、`Agg2D` の vtable（130 スロット）の再実装が要る） | `Agg2D` の入口の仕様（上の §3.3 の表）は逆アセンブルから 77 個ほど読める（役割の全部は未同定。**スロット 42 の役割は未確認**） | **大**。ラッパーの再実装（77 入口）と Blend2D 本体のビルド。**「実物の DLL をそのまま lift する」方針（決定 0073）から外れる**。画素は実機と一致する保証が無い | 保証なし（別実装） |
| **C** | 描画を別実装（canvas の 2D API。`fillRect`／`arc`／`Path2D`）に置換 | 描画の入口は Agg2D の 77 スロットに集約（§3.3）。FLEngine の使用は 31 種の変位（goal-ladder §3.3）。画素のバッファの経路が無くなる（canvas に直接描く）ので、**FLEngine の DIB の画素に書かない**ことになり、後段の `BitBlt` の前提（DIB に画素がある）と**整合しない**。DIB の中身を読む FLEngine のコード（合成、ヒットテスト）がある可能性（未確認） | **中〜大**。アンチエイリアスやブレンドの違いで実機と画素が一致しない | 一致しない |
| **D** | JIT の出力（機械語）を**解釈実行**する（x86-64 の命令列を C で解釈。本調査で生成コードは **53 種の整数・SSE2 命令だけ**（AVX 以上は 0。実測。6 手順の合計））。または、JIT が生成した機械語を `fe-lift` に通して C にする（実行時に lift。wasm では動的に wasm を作ってインスタンス化する） | 生成コードは 501〜1,415 バイトの小さな関数（実測）。lift 済みの SSE2 の命令セットは既にある（`fe-lift` の `handled` 表）。**ただし実行時に作った機械語を lift して実行するには、wasm の動的な生成（`WebAssembly.instantiate` を実行中に呼ぶ）が要る**＝M-10 の (b) | **大**（実行時の lift＝新しい機構。解釈実行なら、小さな x86 の解釈器＋JIT のコード領域のメモリモデル。性能は不明） | JIT 経路と同じ（解釈が正しければ） |

**推奨（意見。決めない）**: **案 A（JIT 無効のコンテキストで、ゲストの Blend2D を lift して使う）を第 1 候補にする**。理由: (1) §5・§6 の**実測で、固定パイプラインは lift した C で動き、JIT 経路と同じ画素を書く**。(2) 「実物の DLL をそのまま lift」の方針を、`flags = 1` の 1 点の介入（A2 か A3。A1 のバイナリのパッチは方針から外れる）に留められる。(3) 案 B・C・D は画素の同等性、方針、新しい機構のどれかで大きく外れる。**案 A の主なリスク（未測定）**: (a) グラデーション、パターン、画像、他の合成演算が固定パイプラインで通るか、(b) 残りの switch 表と、wasm の locals 超過（B2）、(c) FL Studio が JIT 無しで描いた画素が**実機（JIT 有り）と同じ**か（JIT 経路との一致は 6 手順で実測。実機との比較は無い）。**次の測定として小さいもの**: 案 A の下で `Agg2D` の残りのスロット（グラデーション、`clipBox`、画像、23 種のブレンド）を 1 つずつ呼んで画素と停止を見る（今回のプローブに手順を足すだけ）。

### 8.3 G4 の他の条件（X1 の範囲外。本調査は測っていない）

JIT の問題が片付いても、G4 には goal-ladder §3.3・§3.8 の **M-1（窓）、M-4（窓の DC への転送。`f_6bb1b0` の `BitBlt`／`SetDIBitsToDevice` の宛先）、M-3（文字。GDI か FreeType か）**が残る。X1 は「描画の本体が Blend2D の JIT で詰まるか」だけに答え、**詰まらない道（案 A）があることを実測で示した**。FLEngine が最初のフレームで Agg2D を呼ぶかは、G1〜G3（窓とメッセージループ）に届かないと測れない。

## 9. 限界・未確認

1. **測った描画は 64×64 の単純な図形 6 手順だけ**。グラデーション、パターン、画像、テキスト、23 種のブレンドモード、`clipBox`、変換行列（回転・拡大）、半透明の線は測っていない。「JIT 経路と固定経路が同じ画素」は**この 6 手順の範囲**。
2. **FLEngine が実際に呼ぶ Agg2D の入口と回数は未測定**（描画は `CreateFruityInstance` の先で、まだ届かない）。§8.1 は静的な読み。
3. **既定の経路（JIT 有効）は lift した C では最後まで実行できていない**（`clear`／`rect`／`blend` で生成コードの呼び出しで止まる。`line`／`path`／`ellipse` は TLS の壁を越えても同じ）。JIT の画素は、ホストの CPU に機械語を実行させる gdb の実験でだけ測った（native。ブラウザでは不可）。
4. **種の追加（964 個。`x1-results/seeds/b2d-seeds-final-4700000.txt`）は測定の道具**で、決定ではない。実行して止まった所の switch 表を足す方式のため、**この種の集合は「この 6 手順が通る」ことしか保証しない**。種を 964 個にした生成 C では、DllMain の結果（a4 の 7 行）は同一だが functions は 5,306 → 5,313、entries は 1,007,942 → 1,007,954（プローブのビルドの a4。種を足したことで Blend2D の DllMain の経路が少し変わった。原因の同定はしていない）。**基準の a4（種 58 個）の結果は変えていない**（既定のビルドは別。§7(2)）。
5. **switch 表の読み出し（`tools/x1_switch_seeds.py`）は測定の補助で、`fe-lift` の解決の置き換えではない**。表の項目数は `cmp; ja` から、または `movzx` のバイト表の位置から導いた。**誤って足した種は、その VA から始まる閉包を生成 C に足すだけ**（誤っても生成が増えるだけで、止まる場所は変わるが誤った答えにはならない）が、DllMain を壊す例があった（メモリ移動関数 `f_48a2060` の内側に足すと、その関数の最初の switch が `fe-lift` に拒否された。上の §4.1 に書いた）。
6. **`ellipse`／`rect`／`line`／`path` の縁のアンチエイリアスの画素値が正しいかは検証していない**（期待値の独立の計算は `clear`、ベタの塗り、半透明の合成だけ）。JIT 経路と固定経路の一致だけが裏づけ。
7. **Blend2D と AsmJit の版は未確認**。`BL_BUILD_NO_JIT` などのビルド時の設定は、公式のソースを**確認していない**（blend2d.com の文書の取得は 404 だった）。公式の `BL_CONTEXT_CREATE_FLAG_DISABLE_JIT` の bit 0 という値は、**ゲストのコードの `test al,1`（`createInfo.flags` の bit 0 で JIT 提供者と固定提供者を選ぶ）と実測（`flags=1` で固定提供者になった）から**確かめた（公式の文書とは照合していない）。
8. **実機の Windows との比較は 0 件**。JIT が失敗する環境で FL Studio がどう動くかも未確認。
9. **ブラウザ（wasm）では測っていない**。native だけ。生成 C の大きさは 101.4 MB（種 964）で、wasm の locals 超過（B2）と合わせた再ビルドの可否は未測定。
10. **スロット 42 の役割**（FLEngine が 13 か所で呼ぶ）は未同定。「線幅」と書いた初版の読みは、逆アセンブルの読み直しで誤りと分かったので訂正した（ハーネスの定数名は `S_LINEWIDTH` のままで、コメントに「読み。xmm1 を `+0x118` に渡す」と書いてあるが、線幅かは**未確認**。線の手順の画素が正しい位置に出たので、線幅を 3.0 にしたかも未検証）。

## 10. 規則違反の自己申告

- AGENTS.md の「作業は task agent を通じて」: task ツールを持たないサブエージェントなので、調査・実装・測定・文書をすべて自分で行った。
- 作業ツリーへの書き込みは `lift/out-x1/`（`src/` に `git archive` の展開）と `lift/out-git/x1-results/` だけ。git の変更系操作はしていない。commit していない。引数なしの `git checkout` もしていない。
- **`lift/out-x1/` の下の自分の物の削除**（`rm -rf` の対象はすべて `lift/out-x1/` の下の OUT ディレクトリ `o1`〜`o5` と `o3`／`o4` の古い TAG、`diffwork`、`patchtest`、`chk`、`pt`、`cmp`、`cmpwork`、`oX`）。他の作業者の `lift/out-*`（`out-fruity5` を含む）は読み取りのみ。
- Python は `PYTHONDONTWRITEBYTECODE=1`（`tools/x1_*.py` は環境変数つきで実行）。`__pycache__` は 0 件（`find` で確認）。標準ライブラリと衝突する名前のファイルは作っていない。sleep／timeout／nohup／末尾の `&` は使っていない。emcc／wasm は使っていない。`lift/out-cs/` を cwd にしていない。他の作業の `CARGO_TARGET_DIR` は指定していない（`lift/out-x1/target` と、`a4-multi.sh` が OUT 内に作る target）。
- `/tmp/opencode` の下に小さな一時ファイルを 2 回作って、その場で削除した（`x1_before.txt`、`x1_sec42.md`。数 KB。`/tmp` 直下には書いていない）。作業と成果物は `lift/out-x1/` と `lift/out-git/x1-results/` にある。
- **gdb を native に付けた**（`ulimit -c 0`。ブレーク／ウォッチポイント、関数の呼び出し `b2d_native_call` と `fle_set_last_error`、`return`、メモリ・レジスタの書き込み）。**gdb でゲストの状態を書き換えた実験**（§5.4: `VirtualAlloc` の戻りを 0 にする、`OutputDebugStringA` をスキップ。§6: 生成コードをホストで実行して `R[0]` を書く）は、**実験として結果を分けて書いた**（「ふりをして一致扱い」はしていない。失敗を失敗として記録した: §5.4 は画素 0 件）。
- ホストの実ファイル・レジストリをゲストに見せていない（VFS 種付けはフォント 2 ファイルのみ）。資格情報・ブラウザのプロファイルには触れていない。
- 失敗・やり直し: (1) 最初の `a4-multi.sh` が `EXPECT_FLE_SHA` の不一致で gen 段階で止まった（`660b4335…` で通した）。(2) `b2d_probe.c` を変えた後、同じ OUT での再実行は `a4-multi.sh` の stamp 検査に拒否された（新しい OUT を作った）。(3) 種の追加の反復で、巨大な switch を 64 項目で切る誤り、距離のガード（0x4000）で切る誤り、メモリ移動関数の内側への種で DllMain を壊す誤りを順に出して直した（§4.1、§9-5）。(4) `x1_gdb_hostjit.py` の最初の版は未完成のスクリプトを書いて削除した。(5) `x1_gdb_writer.py` の最初の 2 版は gdb の起動の手順の誤りで動かず、直した。(6) 結果ディレクトリへコピーした `gen-m2-with-seeds58-only.txt` は実際には 110 個の種のログだったので、名前を `gen-m2-seeds110-58plus52leaf.txt` に直した。
- 全体テストは走らせていない（依頼のとおり）。
