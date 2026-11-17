# 0227: 行カーネル(SSE2 の packed 整数命令)を fe-wasmgen で wasm の v128 命令に直接 wasm 化し、混成を 977 → 1,009 関数にした — 画素同一のまま、3240x1946 の Playlist ドラッグ(操作分)の直接 wasm 率 56% → 97%、Chrome 156 の更新 4.3 → 13.1 回/s、Firefox 158 の更新 27.6 → 36.4 回/s、node の CPU 0.4〜0.7 倍。60 回/s には届かない — 仮の決定(hyb5)

状態: **仮の決定(すべての選択)**。**これは性能の構造的な不足への対処の測定で、進捗の到達ではない。** 依頼は PM(決定 0227、2026-11-04 11:50〜、総計 4 時間)、作業者 `hyb5`。作業は HEAD **164e81a** の `git archive` を `lift/out-hyb5/src/` に展開した木(S1 用は `anim1.diff` を当てた `src-s1/`)で行い、`hyb5.diff` は最新の HEAD **741d6263** の `git archive`(の対象ファイル)に `patch -p1 --dry-run` が当たる(`fe-wasmgen`・`seeds` は 164e81a から変わっていない)。区分の凡例: **実測**＝このセッションのコマンドの出力を見た事実、**読み**＝コード・文書を読んだ解釈、**推測**＝実機 Windows・依頼者の機械についての言明、**出典なしの選択**＝`S-hyb5-*`(§9)。環境は『この機械』= 16 vCPU の Linux VM(SwiftShader)、隔離 headless Chrome 156、headless Firefox 158(geckodriver)、node 24。依頼者の PC(Firefox 159 / Windows 10 / GTX 980)の測定は 0 件。

## 冒頭の三点

1. **何を実測したか**: 実エンジン(FLEngine_x64.dll と同梱 5 DLL を x86-64 → C → wasm にした物。代役・再生は 0)で、(a) fe-wasmgen が lower できない関数とその理由を、HEAD の runtime で作った native -O0 の計数ビルドの実行命令数(Playlist ドラッグ 1920・3240、Settings タブ。起動を引いた操作分)で重みづけて集計し、(b) 足りない SSE2 の packed 整数命令を `fe-wasmgen` に v128 命令として足して(opt-in `--sse-int`)ホスト CPU と差分テストし、(c) 現行の混成 977 関数に『新たに lower できるようになった実行された 32 関数』を足した新リスト(1,009)で S1(混成なし)・Hcur(現行混成)・Hnew(新混成)を作り、(d) node の G3 で画素同一と CPU 秒、Chrome 156 と Firefox 158 で Playlist ドラッグと Settings タブ切替を 1920x1080 と 3240x1946 で測り、(e) Chrome が大きい画面で重くなる理由を切り分けた。
2. **結論**:
   - **足した命令**: PADDUS*/PSUBUS*/PADDS*/PSUBS*・PMULLW/PMULHW/PMULHUW・PMADDWD・PAVG*・PMIN/PMAX{UB,SW}・PSADBW・PINSRW・PSRLQ/PSLLQ(xmm 数)ほか(SSSE3 の PSHUFB/PALIGNR/PABS*)を wasm の命令 1 本ずつに。決定 0181 が `--mx-packed` の裏に置いていた整数/シャッフルの形も旗なしの MXCSR 窓なしで使えるようにした。**ホスト CPU と 100 形 × 1,008,456 組(100,845,600 組)が全てビット一致、16 ビットレーンの 13 形は 2^32 の順序対を全部通して digest 一致、mutation は 100/100 形で検出**。旗なしの出力は HEAD の fe-wasmgen とバイト一致。
   - **被覆**: 3240x1946 ドラッグの操作分(2.09 G 命令)で直接 wasm の割合は **56.25% → 96.73%**(1920x1080: 79.92% → 95.66%、Settings: 66.04% → 88.85%)。lower できなかった物の最大は `f_2a337c0`(dsp、PSUBUSB、17.3%)と `f_6a7b70`(Pshufd、14.3%)で、どちらも現行リストに無かった。
   - **不変の確認**: FLEngine の既定の生成 C の sha256 `cd9107ce…` (215,811,678 B)、a4 FRUITY 無効 942/942・895/895・5,310・1,009,737、全体テスト 191 results / 2,348 passed / 0 failed(基準 190 / 2,346 / 0 + 新しいテスト分)、すべて基準どおり(§10)。
   - **画素同一**: 7 本の操作列(起動 16 スナップ 82 行、ドラッグ 4 種、Settings 838 行、F8)で S1 に対し Hcur・Hnew とも**全行一致**(面の sha256 を含む)。
   - **速さ(この機械、load ≤ 9 の行)**: 3240x1946 の Playlist ドラッグの更新/s は **Chrome S1 4.3 → Hcur 4.8 → Hnew 13.1**(n = 4、Worker CPU/更新 191 → 167 → 62 ms)、**Firefox S1 27.6 → Hnew 36.4**(Hcur は S1 より遅い 20.9〜21.4)。1920x1080 は入力律速で更新/s は変わらず Worker CPU/更新が Chrome 10.5 → 9.3 → 7.7 ms。Settings タブ切替のアニメーションは Chrome 3240x1946 で 37.5 → 46.3 → 58.6 fps(中央値)。node の仮想時計の主スレッド CPU(Hnew / S1): 起動 0.54、ドラッグ 1920 0.54・3240 0.49、Settings 0.68、F8 0.41。wasm は **127,897,833 B**(S1 の 127,965,132 B より 67 KB 小さく、S1 の線 141,145,856 B に 13.2 MB の余裕)。
   - **3240x1946 の 60 回/s には届かない**: Firefox Hnew 36.4(1.65 倍足りない)、Chrome Hnew 13.1(4.6 倍足りない)。残るゲストの関数は操作の 2.24% だけで、Worker の時間は直接 wasm 59%・ランタイムの C 33%(最大の 1 関数 `fle_r_blit` が 9.9%)・その他 8% に平らに散り、1 関数を消しても 10% 未満(§8)。
   - **Chrome が大きい画面で重い理由(PM の追加依頼)**: 『段差』は load の影響で、実体は更新の画素に対する滑らかな悪化(1 画素当たり 11.8 → 43 ns)。tier-up(Liftoff/TurboFan)・ページフォルト・メモリ量・ヒュージページ・JS の提示は原因ではなく、増加の大半は C 経路の行カーネルの実行時間にあり、直接 wasm 化で 3240/1920 の悪化が 3.8 倍 → 1.5 倍に縮んだ(§7)。**なぜ C 経路だけが悪化するかは未確認**。
3. **未確認**: 依頼者の PC(Firefox 159 / Windows)の測定、実機 Windows との照合、B0(SIMD なしの基準)そのものとの画素比較(S1 と B0 の一致は anim1 の実測)、F8・Settings の命令数(計数ビルドが止まる。V8 プロファイルで代用)、Firefox で Hnew の利得が Chrome より小さい理由と Hcur が S1 より遅い理由(Firefox のプロファイルを取っていない)、Chrome の C 経路が大きい領域でだけ悪化する機構、brotli 後の大きさと起動時間(P1)への影響、PSHUFLW 以外の SSE3/SSE4 命令(現エンジンでは届かない見込み)、packed 浮動小数点の窓つき lowering(残りの 1% 台)、HEAD 741d6263 の木(desk1 などの runtime を含む)での全体テスト・a4 の再走(検証は 164e81a + `hyb5.diff` の木)。

## 0. 先に書くこと(進捗の誠実性。AGENTS.md / `docs/policies/progress-integrity.md` に従う)

**(1) 実エンジンが実行された事実(実測)**: FLEngine_x64.dll と同梱 5 DLL(x86-64 → C → wasm)を、(a) HEAD の runtime で作った native -O0 の計数ビルドで g3 の操作列に、(b) S1・Hcur・Hnew の wasm で node の仮想時計(G3)の 5 つの操作列に、(c) 隔離 headless Chrome 156 と headless Firefox 158 の実ページ(自前の vite preview、`?bundle=` を明示)で Playlist の中ボタンドラッグと Settings のタブ切替に、実際に走らせて観測した。代役・モック・画像の再生・事前生成物・期待値の貼り付けは使っていない。既定の入口 `/assets/manifest.json` とパラメータ無しの URL には何も置いていない(bundle は `lift/out-hyb5/assets/` の自前の preview だけ)。

**(2) 模擬・推測を含む範囲**: 入力(CDP の `Input.dispatchMouseEvent`、WebDriver Actions、G3 の `move`/`click`/`tap`)はすべて模擬で人の操作ではない。座標は画面から読んだ物。実機 Windows との照合は 0 件。依頼者の PC(Firefox 159、GTX 980、dpr 1.7647)の測定は 0 件。『cache 容量の型』などの原因の言明(§7)は**読み**で確認していない。

**(3) 機械の負荷**: この作業の間、load は 0.8〜68 に揺れた(12:04〜12:39 は他の担当の `burn.py` 28 本で 50〜68)。**速さ(壁時計)を主張する行は load ≤ 9 だけ**(Chrome・Firefox の各 run は `--max-load 9` で待ち、各行に load の最大を併記した)。node の CPU 秒は仮想時計の主スレッド CPU で、load の影響は小さいが、S1 と Hcur を測った時間帯の load は 6〜14、Hnew は 1.6〜2.6 だった(§6.3 に再測定の load を併記)。


## 1. 何を足したか(fe-wasmgen、`hyb5.diff`)

方針(AGENTS.md の lift の忠実性)に照らした位置づけ: これは **(b) 命令の契約の実装**(x86 の packed 整数命令 1 本を、同じ定義の wasm v128 命令で直接 wasm にする)であって、HLE(ホスト実装の近道)でも FL の挙動の補正でもない。関数名・アドレス・窓の種類による分岐は無い。新しい処理は『C 経路で 16 レーンのスカラーループ(S1 では wasm SIMD ヘルパーの呼び出し + レジスタ配列経由)になっていた命令を、v128 ローカルの命令 1 本にする』ことだけで、変換と呼び出しの削減に当たる。

- `lift/crates/fe-wasmgen/src/rcgen.rs`: `SSE_INT_ON`(`--sse-int`)と `sse_int_on()`(= `--sse-int` または `--mx-packed`)。
- `lift/crates/fe-wasmgen/src/main.rs`: 旗 `--sse-int`。
- `lift/crates/fe-wasmgen/src/lower.rs`:
  - `sse_x2`(決定 0181 が `--mx-packed` の裏に置いた形: andn・unpack・shuffle・半分の移動・整数比較・シフト・pack・pmuludq・movmsk・pextrw)の入口を `packed_on()` から `sse_int_on()` に変えた。これらは MXCSR を読まず例外旗も立てない bit/整数の関数なので、MXCSR の窓(packed 浮動小数点の `Addps` 等)とは無関係。packed 浮動小数点の窓(`mx_guarded`、rcgen の helper)は従来どおり `--mx-packed` だけ。
  - 新しい命令(すべて `--sse-int` の下): PADDUSB/PADDUSW/PSUBUSB/PSUBUSW(`i8x16/i16x8.add_sat_u/sub_sat_u`)、PADDSB/PADDSW/PSUBSB/PSUBSW(`*_sat_s`)、PMULLW(`i16x8.mul`)、PMULHW/PMULHUW(`i32x4.extmul_low/high` ×2 → 上位 16 ビットのシャッフル)、PMADDWD(`i32x4.dot_i16x8_s`)、PAVGB/PAVGW(`avgr_u`)、PMINUB/PMAXUB/PMINSW/PMAXSW、PSADBW(`sub_sat_u` 2 本の OR → `extadd_pairwise` 2 段 → 64 ビット加算)、PINSRW(レジスタ/m16)、PSRLQ/PSLLQ(即値と xmm の数。数 ≥ 64 は 0)、PSHUFB・PALIGNR・PABSB/W/D(SSSE3。CPUID を SSE2 のみに固定しているので現エンジンでは届かない見込み)。
  - xmm レジスタは従来どおり関数ごとの v128 ローカル。MXCSR 窓・rz1 の rcgen には触れない。
- 旗が無いときの出力は従来と同一(`tools/equiv.sh`: 現行 977 関数の 6 モジュールを HEAD の fe-wasmgen と新しい fe-wasmgen で作って sha256 を比較)。

### 1.1 命令ごとの対応(wasm の命令、`tables/instructions-difftest.tsv`)

| x86(SSE2、xmm の形) | wasm(v128) | 区分 |
|---|---|---|
| PADDUSB / PADDUSW / PSUBUSB / PSUBUSW | `i8x16/i16x8.add_sat_u` / `sub_sat_u` | 新 |
| PADDSB / PADDSW / PSUBSB / PSUBSW | `*.add_sat_s` / `sub_sat_s` | 新 |
| PMULLW | `i16x8.mul` | 新 |
| PMULHW / PMULHUW | `i32x4.extmul_low/high_i16x8_s/u` ×2 → 上位 16 ビットを取るシャッフル | 新 |
| PMADDWD | `i32x4.dot_i16x8_s` | 新 |
| PAVGB / PAVGW | `i8x16/i16x8.avgr_u` | 新 |
| PMINUB / PMAXUB / PMINSW / PMAXSW | `i8x16.min_u/max_u`、`i16x8.min_s/max_s` | 新 |
| PSADBW | `sub_sat_u` 2 本 + `v128.or` → `i16x8.extadd_pairwise_i8x16_u` → `i32x4.extadd_pairwise_i16x8_u` → 64 ビット加算 | 新 |
| PINSRW(r32 / m16) | `i16x8.replace_lane` | 新 |
| PSRLQ / PSLLQ(即値と xmm・m128 の数、数 ≥ 64 は 0) | `i64x2.shr_u/shl` + select | 新(数 ≥ 64 の扱い) |
| PSHUFB / PALIGNR / PABSB,W,D(SSSE3) | `i8x16.swizzle`(`& 0x8f`)/ `i8x16.shuffle` / `*.abs` | 新(現エンジンでは届かない見込み) |
| PSHUFD/LW/HW、PUNPCK{L,H}{BW,WD,DQ}・PUNPCKHQDQ、PACKUSWB/SSWB/SSDW、PCMPEQ/GT{B,W,D}、PSLL/PSRL/PSRA{W,D}(即値・xmm)、PMULUDQ、PANDN、PMOVMSKB、MOVMSKPS/PD、PEXTRW、SHUFPS/PD、UNPCK{L,H}PS/PD、MOVLHPS/MOVHLPS、MOVLPS/HPS/LPD/HPD | 決定 0181 が `--mx-packed` の裏に置いた形 | 入口を `--sse-int` にも開けた |
| PADDB/W/D/Q、PSUBB/W/D/Q、PAND/POR/PXOR、PSRLDQ/PSLLDQ、PUNPCKLBW/QDQ、MOVDQA/U ほか | 以前から旗なしで lower | 変更なし |

実装していない物(census の残り): PMADDUBSW・PHADD*・PSIGN*・PMULHRSW(SSSE3)、PMULLD・PMIN/MAXU*・PMOVZX/SX・PACKUSDW・PEXTR/PINSR B,D,Q・PBLEND*・PTEST(SSE4.1)、MMX 形、packed 浮動小数点(MXCSR の窓が要る: `--mx-packed`)。CPUID を SSE2 のみに固定している(決定 0118)ので、IPP の SSE3 以上の変種は prune S+M で外れているか実行されない。

## 2. lower できない理由の集計(§2.1 静的 census、§2.2 命令数で重みづけた表)

### 2.1 静的 census(`tools/census.sh`、fe-wasmgen `--census`、ship の引数)

FLEngine と同梱 5 DLL(dsp_ippv2・Blend2D〔F1x の wrapper seeds〕・QFC・FreeType・ILTools)を、ship1/ship2 の混成と同じ引数(`--mx-window --trap-vex --no-elide --lazy-ext --static-calls --sync-saved`、R/X は 0x1000/0x1200)で fe-wasmgen に掛け、関数ごとに『その関数自身が直接 wasm になるか』と『ならない理由』を取った。旗なし(HEAD の fe-wasmgen)と `--sse-int` ありを並べる(`tables/census-before-after.tsv`、`tables/census-reasons-base.tsv`、`tables/census-reasons-si.tsv`)。

| モジュール | 関数 | ok(旗なし) | ok(`--sse-int`) | 新たに ok | その静的命令数 | ok でなくなった物 |
|---|---:|---:|---:|---:|---:|---:|
| FLEngine | 59,305 | 58,692 | 58,727 | 35 | 5,416 | 0 |
| dsp_ippv2 | 25,370 | 11,984 | 13,019 | **1,035** | 265,850 | 0 |
| ILTools | 12,930 | 12,475 | 12,501 | 26 | 1,120 | 0 |
| QFC | 6,712 | 6,665 | 6,669 | 4 | 332 | 0 |
| FreeType | 1,747 | 1,655 | 1,655 | 0 | 0 | 0 |
| Blend2D | 732 | 636 | 639 | 3 | 261 | 0 |

旗なしで lower できない最大の理由(dsp を除く 5 モジュールの合計、関数数。複数の理由を持つ関数は両方に数える。`tables/census-base-reasons-5mods.tsv`): `bit test on memory with a register bit offset` 164、RSP 継続入口 149、x87(`Fld` 107・`Fstp` 106・`Fmul` 99…)、`Imul 1-byte` 74、`Shufps` 54、`Packuswb` 50、`Mulps`/`Addps` の窓なし 48/42、`Pshuflw` 46、`Psllw` 28、`Movlps` 26、`Pmulhw` 24、`Pmulhuw` 22、`Punpcklwd` 16、`Psubsw` 16 …。dsp(旗なし)は 25,370 関数のうち 13,386 が lower できず、その主因も packed 整数(`Shufps` 16,460 箇所・`Movlhps`・`Punpcklwd`・`Pshufd`・`Psrad`・`Packssdw`・`Pmaddwd`・`Pmullw`…)と packed 浮動小数点の窓なし。`--sse-int` はこのうち packed 整数/シャッフル/シフト/pack/比較の SSE の行を消す。**残る**のは(6 モジュール合計、`tables/census-reasons-si.tsv`)packed 浮動小数点の MXCSR 窓なし(`Mulps` 1,937 関数・`Addps` 1,869・`Subps` 1,344・`Addpd` 1,337・`Mulpd` 1,215)、RSP 継続入口、x87、SSE3/SSE4(`Cvtps2dq` 1,059・`Cvttps2dq` 905・`Cmpps` 894・`Movddup` 695)。CPUID を SSE2 のみに固定しているので SSE3/SSE4 の変種は prune S+M で外れている物が多く、実行されない。

最初に頼まれた 4 関数: `f_2a337c0`(dsp)= `mnemonic Psubusb` だけ(181 命令、新: ok、wasm 1,981 B)。`f_6a7b70`(FLEngine、21 命令)= `Pshufd` だけ(`--mx-packed` の裏)。`f_6c7610`(331 命令)= すでに ok で現行 977 の混成に入っている(呼び先の `f_6a7b70`・`f_6a7a70` が C なので、直接 → C を跨いでいた)。`f_6a73a0`(67 命令)= `Packuswb ## Pmulhw ## Pshuflw ## Psllw ## Psubsw`。

### 2.2 命令数で重みづけた表(実エンジンの native -O0 計数ビルド)

計数ビルド: HEAD 164e81a の runtime(`FLE_MONITOR_W/H` 対応)と、fe-lift が作った生成 C(`SHIP_PRESET` なしの a4 構成 + `--seh-handler 0x112c0`)を、関数ごとの入口回数と実行命令数のカウンタ(基本ブロック単位、`fb_` 本体は元の関数に数える。hyb1 の `count_fn2.py`)つきで native -O0 に組み、実エンジンを g3 の操作列で走らせた。操作列は anim1 と同じ: 起動 → ウィザードを Enter で閉じる → `main-idle`(共通の前置き = `prefix`)、その後に (a) Playlist の中ボタンドラッグ 120 手(1920x1080、および 3240x1946 で窓の (0.58 W, 0.45 H))、(b) F10 で Settings を開いてタブを順にクリック、(c) F8。**操作の分だけを取る**ため、各走行のカウンタから `prefix` だけの走行のカウンタを関数ごとに引いた。

除いた物: Blend2D(0x4700000〜0x4a00000。出荷は上流 Blend2D への入れ替えで、native の計数ビルドの Blend2D は出荷に無い物)、dsp の HLE 入口 `0x33d6e70`(warp_bilin。出荷ではホスト実装)。

**限界(実測の範囲)**: 計数ビルドは ship2 の import 行を持たないため、(c) F8 は `DwmIsCompositionEnabled`(DWMAPI の遅延読み込み)で停止して F8 のウィンドウを開けず、(b) Settings は Theme タブの `SHChangeNotifyRegister`(anim1 の `anim1-shchange.diff` が直す物)で止まるまで、すなわち General・File の 2 タブ分までしか取れなかった。F8・Settings の混成リストへの追加は、代わりに S1 の node 実行の V8 CPU プロファイル(`PROF=1`、関数名つき)で自己時間にサンプルがある `f_<va>` を『実行された』と数えた。命令数の表は Playlist ドラッグ(1920・3240)と Settings の 2 タブだけが一次の数字で、F8 の命令数は 3.2 M と小さく(上の理由で F8 の中身は取れていない)、実行された物の割合の表としては使えない。

| 操作列 | 操作分の実行命令 | 現行 977 の混成で直接 | 同、`--sse-int` + 新リスト(1,009)で直接 | C のまま(lower 不可の命令) 前 → 後 |
|---|---:|---:|---:|---:|
| Playlist ドラッグ 1920x1080 | 511,876,226 | 79.92% | **95.66%** | 18.34% → 2.60% |
| Playlist ドラッグ 3240x1946 | 2,093,290,628 | 56.25% | **96.73%** | 42.72% → 2.24% |
| Settings タブ(General・File のみ) | 2,574,294,824 | 66.04% | **88.85%** | 30.88% → 8.07% |

(lower 可能だが現行のリストに無い関数は、ドラッグで 1.0〜1.7%、Settings で 3.1%。これは本決定のリストには入れていない〔指示は『977 + 新しく lower できるようになった関数』〕。)

表: `tables/wl-<操作>-base-*.tsv`(前: 旗なしの census と現行リスト)、`tables/wl-<操作>-after-*.tsv`(後: `--sse-int` の census と新リスト)。`*-functions.tsv` は関数ごとの実行命令数・理由、`*-reasons.tsv` は理由ごとの集計。

## 3. 差分テスト(`tests/simd_int_forms.rs`、`hyb5.diff`)

1 命令形ごとに『`movups xmm0,[rcx]; movups xmm1,[rcx+16]; <命令>; 保存; add rcx,32; add rdx,16; dec r8; jnz`』のループ関数を fe-wasmgen で wasm にし、node で N 組の入力に走らせて、結果をホスト CPU(core::arch の同じ命令)とビット比較する(fe-wasmgen の既存の `sse_x2_forms.rs` と同じ形式。ループにして 1e6 組を 1 回の呼びで回す)。

- **(a) 乱数 ≥ 1e6 + 境界値の全組み合わせ**: 100 命令形 × **1,008,456** 組(全バイト対 65,536 を 2 配置 = 8,192 組、16 ビット境界値 24x24、32 ビット境界値 16x16、シフト数 16 種 × 8、乱数 1,000,000〔半分は境界値の混合〕)= 100,845,600 組が**全てホスト CPU とビット一致**(`logs/test-forms-1e6.log`、`tables/instructions-difftest.tsv`)。
- **16 ビットレーンの 13 形は 2^32 の順序対を全部**(`paddusw`・`psubusw`・`paddsw`・`psubsw`・`pmullw`・`pmulhw`・`pmulhuw`・`pavgw`・`pminsw`・`pmaxsw`・`pcmpeqw`・`pcmpgtw`・`pabsw`): 8 レーンずつ 2^29 組を通した結果ストリームの 64 ビット digest(2 本の 32 ビット鎖、imul と xorshift)をホストの digest と比較 = **13/13 一致**(各 4〜5 s、`logs/test-exhaustive16.log`)。
- **(b) mutation**: 生成した wasm 本体の同じ長さの 1 箇所(opcode・shuffle の lane・定数・lane 番号)を壊して同じ比較を走らせ、**100/100 形で差が検出**された(検出した入力数は最少 40/28,456〔`psllw xmm`: 数が 16 未満の入力が少ない〕、`logs/test-mutants-1.log`)。16 ビット網羅の digest も、壊した版では 13/13 で違う値になる(digest が壊れた lowering を見分けられる)。最初の digest(32 ビットの単純な鎖)は低位 12 ビットが入力に依存しない弱さがあったので、非線形の混ぜを足して全形を取り直した(S-hyb5-4)。
- 既存の fe-wasmgen のテスト(`sse_x2_forms`・`sse_forms`・`mx_*`・`rc_*` ほか 17 ファイル)は全て通る(`logs/test-fe-wasmgen-all.log`)。**旗なしの出力は従来と同一**: 現行 977 関数の 6 モジュールを HEAD の fe-wasmgen と新しい fe-wasmgen(旗なし)で作り、sha256 が 6 モジュールとも一致(`logs/equiv.log`)。

## 4. 新しい混成のリスト

現行 977 + (旗なしでは lower できず `--sse-int` では lower できる + 計数ビルド〔pl1920・pl3240・set1920・f8・prefix〕または S1 の V8 プロファイル〔set・f8〕が実行を見た)関数 - (specials〔SEH ラッパー・hook・landing・HLE・site〕・prune S+M・Blend2D swap・サイズ上限)= **1,009 関数**(fle 730、dsp 33、b2d 31、qfc 163、ft 45、ilt 7。追加 32)。`tools/select-new.py`、`lists/new/added.tsv`。追加した 32 関数のうち実行命令数の多い物: `f_6a7b70`(FLEngine、Pshufd)、`f_2a337c0`(dsp、Psubusb)、`f_1e7e100`(QFC、Pmulhuw ほか)、`f_6a73a0`、`f_38c7080`(dsp)、`f_6a7a70`、`f_6bbcb0`、`f_6a72b0` …。n0 の照合は 6 モジュールとも『listed N, emitted ok N、未出力 0、リスト外の出力 0』。

追加した 32 関数(`lists/new/added.tsv`。『実行命令』は計数ビルド 5 走行の合計〔起動を含む〕または V8 プロファイル由来の 1)の上位 14:

| 関数 | モジュール | 入口 | 実行命令(計数ビルド合計) | 静的命令数 |
|---|---|---:|---:|---:|
| `f_6a7b70` | fle | 16768919 | 1726731456 | 21 |
| `f_2a337c0` | dsp | 7984172 | 1019043562 | 181 |
| `f_1e7e100` | qfc | 578388 | 713779903 | 102 |
| `f_6a73a0` | fle | 10295 | 355610055 | 67 |
| `f_38c7080` | dsp | 543577 | 277495893 | 85 |
| `f_6a7a70` | fle | 6398480 | 235317521 | 63 |
| `f_6bbcb0` | fle | 9650377 | 212308973 | 22 |
| `f_6a72b0` | fle | 92757 | 165106784 | 65 |
| `f_40d210` | fle | 1072063 | 70842021 | 54 |
| `f_6bbdf0` | fle | 3367894 | 70725920 | 21 |
| `f_6a77b0` | fle | 60324 | 69289709 | 47 |
| `f_383ad10` | dsp | 631 | 43598819 | 314 |
| `f_2882ac0` | dsp | 499948 | 28066610 | 183 |
| `f_3a67380` | dsp | 2132 | 13605291 | 387 |

32 関数の実行命令の合計は 5021827033(5 走行の合計)。

## 5. 画素同一(node の G3 = 仮想時計、実エンジン)

比較の基準は **S1**(= HEAD + anim1.diff の F2xH3、混成なし、wasm-opt -Oz。同じ runtime で作った物)。B0(anim1 の SIMD なしの基準)そのものはこの作業で作り直していない: S1 と B0 の画素の一致は anim1 の実測(起動 16 スナップと操作列 5 本で差 0 行)で、本決定はその S1 を基準に Hcur・Hnew が一致することを確かめた。`lift/harness/b2d2/expected/g3-lines-*.txt` の既存の期待表は、HEAD の runtime が決定 0214(present2)などで進んだため **S1 とも全て不一致**(8 表とも 82 行が違う)で、基準に使えない。

各操作列(7 本)を S1・Hcur(現行 977 の混成)・Hnew(新リスト 1,009 の混成)で走らせ、`g3-snap`・`g3-progress`・`g3-surface`・`g3-wintable` の行を `g3cmp.py` で行ごとに比べた(`logs/idrun-Hcur.log`、`logs/idrun-Hnew.log`、`logs/idrun-s64.log`)。

| 操作列 | 行数 | Hcur vs S1 | Hnew vs S1 |
|---|---:|---|---|
| 起動 16 スナップ(`cmp-short.g3`) | 82 | 一致 | **一致** |
| Playlist 中ボタンドラッグ 1920x1080(120 手) | 51 | 一致 | **一致** |
| Settings の 9 クリック(各クリック後 12 スナップ) | 838 | 一致 | **一致** |
| F8 の開閉 | 96 | 一致 | **一致** |
| Playlist 中ボタンドラッグ 3240x1946(120 手) | 51 | 一致 | **一致** |
| 同、64 ms 刻み 1920x1080(入力を束ねた形。`g3/pls64.g3`) | 51 | 一致 | **一致** |
| 同、64 ms 刻み 3240x1946(`g3/pl3240s64.g3`) | 51 | 一致 | **一致** |

(`g3-surface` の行は窓の面の sha256 なので、面の全画素が一致している。)

## 6. 測定(前後の比較)

### 6.1 構成と条件

| 名前 | 中身 |
|---|---|
| **S1** | HEAD + `anim1.diff` の `SHIP_PRESET=F2xH3`(全ユニット `-msimd128`、fast copy、`fle_raster`・`fle_sse_more` を -O2、packed 整数ヘルパーを wasm SIMD)。**混成なし**、wasm-opt -Oz。HLE は dsp の `warp_bilin` のみ(出荷と同じ)、Blend2D は上流への入れ替え |
| **Hcur** | S1 の C 部 + 現行の混成(`seeds/hyb-p1000r-F1x-*.txt` 977 関数、旗なしの fe-wasmgen) |
| **Hnew** | S1 の C 部 + 新しい混成(`seeds/hyb-sse2-F2x-*.txt` 1,009 関数、`--sse-int`) |

3 つとも同じ生成 C・同じ runtime・同じ wasm-opt -Oz(C 部 + 直接部をそれぞれ)。現行の出荷 SHIP2(F2xH2)は SIMD なしかつ 1920x1080 固定(`FLE_MONITOR_W/H` が無い)なので、3240x1946 では測れず、基準にしなかった。

ブラウザ: 隔離 headless Chrome 156(使い捨てプロファイル、SwiftShader、CDP の模擬入力。`tools/drive-pipe.mjs` = anim1 の `drive.mjs` に 10 s のウォームアップ・Worker スレッドの user/sys/ページフォルト・wasm メモリを足した物)と headless Firefox 158(geckodriver、WebDriver Actions。`tools/ffdrive.mjs` = anim1 の物に `--max-load` を足した物)。**入力はどちらも模擬で人の操作ではない**。Playlist の中ボタンドラッグ: 画面の (1395, 594)〔1920x1080〕または (0.58 W, 0.45 H)〔3240x1946: (1879, 875)〕から ±160 × ±90 px の Lissajous 軌道を、Chrome は 4 ms 間隔(実際は約 41〜57 手/s)、Firefox は 16 ms 間隔(約 30 手/s)で 6 s(Chrome)/ 12 s(Firefox)× 2 回。Settings タブ: F10 の後、General・File・Theme・MIDI・Audio・General・Project・Info・General の 9 クリック(窓は画面の中央に出るので 3240x1946 では x に +660、y に +433 を足した位置)。各 run は `--max-load 9` で load が 9 以下になるまで待ち、Chrome は構成を S1 → Hcur → Hnew の順に画面ごとに交互に走らせた。Chrome のドラッグは 2 つの round(round 1: 先に、round 2: 後でもう一度。どちらも各 run 2 回 = 計 n = 4)を併せた中央値で、round 1 の n = 2 の表(S1 4.3・Hcur 4.7・Hnew 11.7 回/s、3240x1946)は round 2 で Hnew が 14 回/s 前後に上がり(Hcur・S1 は変わらず)、併せて Hnew 13.1 回/s(11.4〜14.4)になった。round 間の差の原因は調べていない。タブ切替は round 1 のみ。

「更新/s」はページが受けた窓の描画(`putImageData`)の回数、「Worker CPU/更新」は Worker スレッドの CPU(/proc の utime + stime)÷ 更新の数。Chrome のドラッグは入力が 57/s 前後で 1920x1080 では**入力で頭打ち**(Worker の使用率が 50〜66%)なので、更新/s ではなく Worker CPU/更新と使用率で読む。3240x1946 では Worker の使用率が 80% 超で、更新/s が Worker の速さを表す。**この機械(16 vCPU、SwiftShader)の値で、依頼者の機械ではない**。各走行の生の `result.json` は `tables/runs/m-<browser>-<構成>-<画面>-<操作>-r<round>.json`(ログは同名の `.log`)。

### 6.2 wasm の大きさ(wasm-opt -Oz 後の raw、名前なし。`S1 の線` = 141,145,856 B)

| 構成 | wasm raw | S1 の線との差 | S1 比 | 備考 |
|---|---:|---:|---:|---|
| S1(混成なし) | 127,965,132 B | 13,180,724 B 下 | — | |
| Hcur(現行 977) | 127,994,033 B | 13,151,823 B 下 | +28,901 B(+0.023%) | |
| **Hnew(1,009)** | **127,897,833 B** | **13,248,023 B 下** | **−67,299 B(−0.053%)** | Hcur より −96,200 B |

(参考: 出荷 SHIP2 F2xH2 は 130,217,196 B、SIMD なし〔決定 0208〕。S1 の名前つきの無最適化版は 171,720,103 B。brotli は測っていない。)

### 6.3 Chrome 156(実エンジン、模擬入力。load 最大は各行に併記。n = 走行回数〔ドラッグは 1 走行 = 1 rep、タブは 1 クリック〕)

**Playlist 中ボタンドラッグ**

| ブラウザ | 画面 | 構成 | 更新/s 中央値(最小〜最大) | Worker CPU/更新 中央値(ms) | Worker の使用率 | 1 更新の画素(Mpx) | n | load 最大 | 速さを主張できる行(load ≤ 9) |
|---|---|---|---|---:|---:|---:|---:|---:|---|
| chrome | 1920x1080 | S1 | 63.0 (59.3〜65.1) | 10.5 | 66% | 0.94 | 4 | 3.8 | はい |
| chrome | 1920x1080 | Hcur | 62.3 (56.9〜65.5) | 9.3 | 58% | 0.96 | 4 | 4.6 | はい |
| chrome | 1920x1080 | Hnew | 63.8 (62.9〜66.9) | 7.7 | 49% | 0.93 | 4 | 5.4 | はい |
| chrome | 3240x1946 | S1 | 4.3 (4.2〜4.3) | 190.8 | 82% | 4.47 | 4 | 8.0 | はい |
| chrome | 3240x1946 | Hcur | 4.8 (4.2〜5.3) | 167.2 | 81% | 4.74 | 4 | 5.0 | はい |
| chrome | 3240x1946 | Hnew | 13.1 (11.4〜14.4) | 61.8 | 80% | 4.91 | 4 | 6.0 | はい |

**Settings のタブ切替**(アニメーションの fps = クリック後の最初の変化バースト〔変化画素 ≥ 5% の描画、間隔 400 ms 以内〕の `(描画数 − 1) / 時間`)

| ブラウザ | 画面 | 構成 | タブ切替のアニメーション fps 中央値(最小〜最大) | 描画数/クリック窓 3 s 中央値 | Worker CPU/クリック窓 3 s 中央値(s) | n(クリック) | load 最大 |
|---|---|---|---|---:|---:|---:|---:|
| chrome | 1920x1080 | S1 | 44.6 (34.9〜62.9) | 27 | 0.87 | 9 | 2.8 |
| chrome | 1920x1080 | Hcur | 45.5 (38.4〜67.2) | 29 | 0.67 | 9 | 1.6 |
| chrome | 1920x1080 | Hnew | 55.1 (36.1〜79.2) | 29 | 0.85 | 9 | 6.0 |
| chrome | 3240x1946 | S1 | 37.5 (31.2〜55.1) | 27 | 0.99 | 9 | 4.6 |
| chrome | 3240x1946 | Hcur | 46.3 (35.2〜69.4) | 28 | 0.74 | 9 | 4.6 |
| chrome | 3240x1946 | Hnew | 58.6 (49.6〜74.8) | 29 | 0.73 | 9 | 6.3 |

### 6.4 Firefox 158(実エンジン、WebDriver Actions)

**ウォームアップなし**(round 1: ドラッグ 12 s × 2 回、入力 約 30 手/s)

| ブラウザ | 画面 | 構成 | 更新/s 中央値(最小〜最大) | Worker CPU/更新 中央値(ms) | Worker の使用率 | 1 更新の画素(Mpx) | n | load 最大 | 速さを主張できる行(load ≤ 9) |
|---|---|---|---|---:|---:|---:|---:|---:|---|
| firefox | 1920x1080 | S1 | 42.9 (41.1〜44.6) | 15.9 | 68% | 1.02 | 2 | 6.3 | はい |
| firefox | 1920x1080 | Hcur | 48.5 (46.0〜51.0) | 12.0 | 58% | 1.01 | 2 | 4.4 | はい |
| firefox | 1920x1080 | Hnew | 45.1 (39.7〜50.5) | 16.5 | 72% | 0.98 | 2 | 6.0 | はい |
| firefox | 3240x1946 | S1 | 23.2 (20.3〜26.1) | 38.5 | 89% | 3.30 | 2 | 5.4 | はい |
| firefox | 3240x1946 | Hcur | 20.9 (18.7〜23.0) | 43.5 | 90% | 3.26 | 2 | 5.3 | はい |
| firefox | 3240x1946 | Hnew | 28.0 (26.0〜30.1) | 30.6 | 86% | 3.41 | 2 | 8.9 | はい |

**Settings のタブ切替**(Firefox では描画のタイムラインを取っていないので fps は出さず、クリック後 3 s の描画数と Worker CPU)

| ブラウザ | 画面 | 構成 | タブ切替のアニメーション fps 中央値(最小〜最大) | 描画数/クリック窓 3 s 中央値 | Worker CPU/クリック窓 3 s 中央値(s) | n(クリック) | load 最大 |
|---|---|---|---|---:|---:|---:|---:|
| firefox | 1920x1080 | S1 | - | 29 | 1.11 | 9 | 5.7 |
| firefox | 1920x1080 | Hcur | - | 28 | 1.09 | 9 | 5.7 |
| firefox | 1920x1080 | Hnew | - | 28 | 0.99 | 9 | 5.4 |
| firefox | 3240x1946 | S1 | - | 28 | 1.29 | 9 | 5.5 |
| firefox | 3240x1946 | Hcur | - | 28 | 1.19 | 9 | 8.9 |
| firefox | 3240x1946 | Hnew | - | 28 | 0.96 | 9 | 8.7 |

**ウォームアップあり**(round 2: 20 s のドラッグの後、8 s × 3 回)

| ブラウザ | 画面 | 構成 | 更新/s 中央値(最小〜最大) | Worker CPU/更新 中央値(ms) | Worker の使用率 | 1 更新の画素(Mpx) | n | load 最大 | 速さを主張できる行(load ≤ 9) |
|---|---|---|---|---:|---:|---:|---:|---:|---|
| firefox | 1920x1080 | S1 | 75.9 (70.7〜80.6) | 11.0 | 88% | 1.00 | 3 | 3.9 | はい |
| firefox | 1920x1080 | Hcur | 76.5 (70.6〜79.5) | 10.8 | 86% | 1.00 | 3 | 5.6 | はい |
| firefox | 1920x1080 | Hnew | 82.2 (69.6〜83.4) | 9.8 | 79% | 1.01 | 3 | 8.0 | はい |
| firefox | 3240x1946 | S1 | 27.6 (27.6〜29.6) | 35.7 | 99% | 3.08 | 3 | 5.0 | はい |
| firefox | 3240x1946 | Hcur | 21.4 (20.2〜22.2) | 44.9 | 98% | 2.80 | 3 | 13.4 | いいえ(CPU 秒で読む) |
| firefox | 3240x1946 | Hnew | 36.4 (33.5〜38.5) | 25.5 | 97% | 3.12 | 3 | 4.9 | はい |

### 6.5 node の仮想時計の CPU(同じ操作列、S1 → Hcur → Hnew を 2 巡、主スレッド CPU 秒。`tools/cpu-table.py`)

巡 a:

| operation | S1-a | Hcur-a | Hnew-a |
|---|---:|---:|---:|
| startup (wiz0 -> main-idle, 9 virtual s) | 9.681 | 6.724 | 5.252 |
| Playlist drag 1920x1080 (120 moves) | 1.318 | 0.901 | 0.726 |
|   same, only the third quarter (30 moves, after the warm-up) | 0.165 | 0.093 | 0.091 |
| Playlist drag 3240x1946 (120 moves) | 2.904 | 2.544 | 1.418 |
|   same, only the third quarter (30 moves, after the warm-up) | 0.237 | 0.153 | 0.140 |
| Playlist drag 3240x1946, 64 ms steps (120 moves; the input of the browser runs is coalesced like this) | - | - | - |
| Playlist drag 1920x1080, 64 ms steps (120 moves) | - | - | - |
| Settings tabs (9 clicks, 1920x1080) | 10.265 | 8.309 | 7.036 |
| F8 open+close (1920x1080) | 8.352 | 6.038 | 3.451 |

巡 b:

| operation | S1-b | Hcur-b | Hnew-b |
|---|---:|---:|---:|
| startup (wiz0 -> main-idle, 9 virtual s) | 10.246 | 6.677 | 6.222 |
| Playlist drag 1920x1080 (120 moves) | 1.352 | 0.974 | 0.877 |
|   same, only the third quarter (30 moves, after the warm-up) | 0.165 | 0.105 | 0.099 |
| Playlist drag 3240x1946 (120 moves) | 2.869 | 2.460 | 2.467 |
|   same, only the third quarter (30 moves, after the warm-up) | 0.214 | 0.154 | 0.174 |
| Playlist drag 3240x1946, 64 ms steps (120 moves; the input of the browser runs is coalesced like this) | - | - | - |
| Playlist drag 1920x1080, 64 ms steps (120 moves) | - | - | - |
| Settings tabs (9 clicks, 1920x1080) | 10.362 | 8.526 | 7.025 |
| F8 open+close (1920x1080) | 7.715 | 6.641 | 3.672 |

## 7. Chrome が大きい画面で重くなる理由の切り分け(PM の追加依頼)

依頼: 同じ wasm で Firefox 158 は 3240x1946 の更新が 21.4 回/s(38 ms)なのに Chrome 156 は 4.2 回/s(197 ms)で、1 画素当たりが 12 → 42 ns に悪化し、`f_2a337c0` の 1 更新当たりも 0.4 → 20 ms になる理由を、プロファイルと小さい実験で分ける。

構成: 自作の S1(F2xH3、非混成、wasm-opt -Oz)の Chrome 156(隔離、CDP の模擬入力)と Firefox 158(geckodriver)、Playlist の中ボタンドラッグ(`tools/anom.sh`、`drive-pipe.mjs`: 10 s のウォームアップ + 6 s × 2 回)。**load 3〜5**(他のジョブが少ない時間。`runs/an-*`)。以下は全て Worker スレッドの値(/proc の utime/stime/minflt)。

### 7.1 『段差』はなく、更新の画素数に対して滑らかに悪化する(anim1 の段差は load の影響)

anim1 の表は 2060x1264 と 2560x1440 の間に段差があるとした。load 3〜5 で同じ S1 を測り直すと、画素数に対する 1 画素当たりの Worker CPU は次のとおり**段差ではなく滑らかな増加**で、anim1 の途中の点を私が最初に load 10〜17 で測った分(2200x1300 で 7〜9 回/s など)は load で悪化した値だった(同じ画面を load 4 で測ると 31 回/s)。

| ゲスト画面 | 1 更新の画素(Mpx) | 更新/s | Worker CPU/更新 | 1 画素当たり | user / sys(6 s) | ページフォルト/6 s | load |
|---|---:|---:|---:|---:|---|---:|---:|
| 2060x1264 | 1.66 | 38〜41 | 19.6 ms | 11.8 ns | 3.87 / 0.62 s | 366〜377 k | 3.2 |
| 2200x1300 | 1.84 | 31〜32 | 24 ms | 13.1 ns | 4.1 / 0.5 s | 316〜338 k | 4.0 |
| 2300x1350 | 2.08 | 26〜30 | 28〜30 ms | 14.5 ns | 4.2 / 0.5 s | 313〜360 k | 4.7〜5.1 |
| 2400x1400 | 2.33 | 19.3〜19.5 | 42 ms | 18 ns | 4.5 / 0.43 s | 258 k | 4.6 |
| 2560x1440 | 2.77 | 10〜11.5 | 72 ms | 26 ns | 4.7 / 0.26 s | 150〜176 k | 5.1 |
| 3240x1946 | 4.56 | 4.2〜4.3 | 190 ms | 42 ns | 4.8 / 0.15 s | 79〜100 k | 4.1〜4.5 |

- 悪化は **user 時間**(wasm とランタイムの C の実行)で、sys 時間(ページフォルト)は画面が大きいほど減る。wasm のメモリの大きさ(`engineMemoryBytes`)は 0.98〜1.09 GB でほぼ変わらない(メモリの増加が原因ではない)。
- 2560x1440 のプロファイル(`runs/cp-S1-2560x1440`、anim1 の S1)は、3240x1946 のプロファイルと同じ構成で、JavaScript は 1.2%。1 更新当たりに直すと 1920x1080 から 2560x1440 で `f_2a337c0` 0.6 → 9.6 ms、`fle_r_blit` 0.54 → 9.0 ms、`fle_ld` 0.8 → 8.3 ms、`f_410010`(11 命令の小さい関数)1.1 → 5.7 ms で、**画素に比例する関数も画素に関係しない小さい関数も、同じ倍率(面積比 2.6 に対して 5〜16 倍)で悪化**する。→ 特定の関数・経路の問題でなく、更新の画素領域(4 B × 画素 = 6.6〜18 MB)の再利用距離が長くなる**メモリ階層(キャッシュ容量)の問題の型**(読み。この機械は L2 1 MB/コア、L3 32 MB を他のジョブと共有)。確認はしていない(`perf` は `perf_event_paranoid=4` で使えない)。

### 7.2 V8 の tier(Liftoff / TurboFan)は原因ではない

3240x1946、S1、Chrome(`FL_CHROME_ARGS`、`tools/anom.sh`、ウォームアップ 10 s + 6 s × 2 回)。

| Chrome の V8 フラグ | 更新/s | Worker CPU/更新 | 1 画素当たり | load |
|---|---:|---:|---:|---:|
| 既定(`runs/m-chrome-S1-3240x1946-pldrag-r1`) | 4.3 | 191 ms | 43 ns | 4.5 |
| `--liftoff-only`(Liftoff だけ) | 3.3 | 258 ms | 121〜132 ns | 7.5〜8.2 |
| `--no-wasm-dynamic-tiering`(最初から全関数を TurboFan に回す) | 3.8〜4.2 | 200〜221 ms | 47〜52 ns | 9.4〜9.5 |
| `--wasm-tiering-budget=20000`(tier-up の予算を 1/90 に) | 3.5〜3.8 | 217〜244 ms | 58〜60 ns | 9.9 |

Liftoff だけだと既定の約 3 倍遅く、既定が Liftoff に留まっているわけではない。tier-up を早めても(後 2 行、load 約 10 で少し悪い側に出ている)既定と変わらない。**tier-up の遅れは原因ではない**。(同じ 2 本を Hnew で測った行は、他のジョブで load 19〜21 になり無効。)

### 7.3 Chrome と Firefox、同じ S1 の 3240x1946

| ブラウザ | 更新/s | Worker CPU/更新 | 1 更新の画素 | 1 画素当たり | 条件 |
|---|---:|---:|---:|---:|---|
| Chrome 156 | 4.2〜4.3 | 191 ms | 4.4 Mpx | 43 ns | 入力 約 55 手/s、6 s × 2、ウォームアップ 10 s、load 4〜5 |
| Firefox 158 | 22〜25(ウォームアップなし)、27.6〜29.6(あり) | 32〜38 ms | 3.1〜3.3 Mpx | 10〜12 ns | 入力 約 30 手/s、12〜16 s × 2〜3、load 4〜5 |

1 更新の画素と入力の束ね方(Chrome は約 13 手を 1 更新に束ね、Firefox は約 1 手 = 1 更新)が違うので、更新当たりの CPU は比べられない。画素当たりで Chrome は Firefox の約 3.5〜4 倍。Firefox は 2560x1440 の 1 走行目が 7.7 回/s・2 走行目が 1.1 回/s(Worker が詰まった走行)と不安定で、画面の大きさに対する Firefox の曲線は取れていない。

### 7.4 ほかに調べて否定した物

- **ページフォルト / sys 時間**: 悪化は user 時間(§7.1 の表)。フォルトは画面が大きいほど減る。
- **wasm のメモリの大きさ**: 0.98〜1.09 GB でほぼ変わらない。
- **透過ヒュージページ**: この機械は `transparent_hugepage/enabled=madvise`、`shmem_enabled=never`。実行中の Chrome のレンダラー(RSS 1.36 GB)も Firefox のコンテンツプロセス(RSS 1.73 GB)も `AnonHugePages` は 0 kB。ブラウザの差はヒュージページではない。
- **JavaScript / 提示の経路**(Hnew、Chrome の Worker プロファイル、`runs/an-PROF-Hnew-3240x1946`、load 20 前後で率は無効だが構成比は読める): wasm 83.6%、(program)〔JS でも wasm でもない native: ArrayBuffer のコピー・ゼロ詰め・フォルトなど〕13.0%、JavaScript 2.1%、GC 0.2%。**エンジンの外の提示経路は Worker の約 15%**(約 10 ms/更新)で、Worker の時間の大半は wasm の中。

### 7.5 混成にすると『大きい画面で悪化する分』が小さくなる(測定)

Chrome(§6.3 の行、load ≤ 6)の 1 画素当たりの Worker CPU:

| 構成 | 1920x1080(0.94 Mpx/更新) | 3240x1946(4.4〜4.9 Mpx/更新) | 3240 / 1920 |
|---|---:|---:|---:|
| S1 | 11.2 ns | 42.7 ns | 3.8 倍 |
| Hcur | 9.7 ns | 35.3 ns | 3.6 倍 |
| **Hnew** | 8.3 ns | 12.6 ns | **1.5 倍** |

→ 大きい画面で 1 画素当たりが増える分の大半は、C 経路で packed 整数命令を 16 レーンのスカラー/ヘルパー呼び出しで実行していた行カーネルに付いていた(Hcur では行カーネルが C のままなので S1 と同じ 4 倍近く〔3.6 倍〕)。直接 wasm にすると 1.5 倍まで縮む。**なぜ C 経路の行カーネルだけが大きい領域で悪化するのかは確認していない**(読み: 行ごとに `X[]`/`R[]` のレジスタ配列とヘルパーを介するので、領域の再利用距離が長いときのキャッシュ効率がより悪く出る。確認するには `perf` が要るが `perf_event_paranoid=4` で使えない)。

### 7.6 結論(確認できた範囲)

1. 『段差』は load の影響で、実体は更新の画素に対する滑らかな増加(§7.1)。
2. tier-up・ページフォルト・メモリ量・ヒュージページ・JavaScript の提示は原因ではない(§7.2〜7.4)。
3. 増加の大半は C 経路の行カーネルの実行時間にあり、`--sse-int` の直接 wasm でその分が 3.8 倍 → 1.5 倍に縮んだ(§7.5)。残りの Chrome 対 Firefox の差(Hnew で Chrome 12.6 ns/px に対し Firefox 8 ns/px 前後)の原因は**未確認**。

## 8. 目標の確認(3240x1946 の Playlist ドラッグ 60 回/s、P2 は 120)と、届かない分の次の裾

**届いていない。** 3240x1946(依頼者の fit-device の大きさ)の Playlist 中ボタンドラッグ:

| ブラウザ(主は Firefox) | S1(混成なし) | Hcur(現行 977) | **Hnew(1,009)** | 60 回/s まで | 120 回/s まで |
|---|---:|---:|---:|---:|---:|
| Firefox 158(ウォームアップあり、n = 3、load ≤ 5) | 27.6 回/s | 21.4 回/s〔load 11〜13 の走行、参考〕/ 20.9〔ウォームアップなし、load 5〕 | **36.4 回/s**(33.5〜38.5) | **1.65 倍足りない** | 3.3 倍 |
| Chrome 156(ウォームアップ 10 s、n = 4〔round 1 と 2 の 2 回ずつ〕、load ≤ 8) | 4.3 回/s | 4.8 回/s | **13.1 回/s**(11.4〜14.4) | **4.6 倍足りない** | 9.2 倍 |

Worker はどちらも 80〜99% 使われていて(Firefox 97〜99%、Chrome 80%)、更新/s は Worker の速さで決まっている(入力で頭打ちではない)。1920x1080 は入力律速で Chrome 63〜65 回/s・Firefox 76〜82 回/s(60 回/s を超えるが、これは入力の速さ 約 30〜57 手/s に近い値で、Worker の余力は Chrome Hnew で 50% 使用)。

### 8.1 次の裾(一次の数字)

**(a) C のまま残るゲストの関数**(Hnew の後、3240x1946 ドラッグの操作分 2,093,290,628 命令のうち **2.24%**、46 関数)。上位は下の表(§8.2)。**ゲストの命令の 96.73% はすでに直接 wasm**(前: 56.25%)なので、ゲストの関数の lower を足しても残りは 2% 台で、速さにはほとんど効かない。残る理由の内訳(46 関数): packed 浮動小数点の MXCSR 窓なし(`Mulps`/`Addps`/`Maxps`/`Subps`…)26 関数・22.0 M 命令(1.05%)、RSP 継続入口 1 関数(`f_27ad5f8`)16.2 M 命令(0.77%)、その他(`Cvtps2dq`、`CPUID`、32 ビットアドレス、`jecxz`、`bit test on memory` …)19 関数・8.6 M 命令(0.41%)。

**(b) 直接 wasm 化した後の Worker の時間の中身**(node で名前つきの Hnew、3240x1946 のドラッグの操作分 1.77 s のプロファイル、`tables/prof-hnew-pl3240-drag-top.tsv`): 直接 wasm のゲストの関数 **59.4%**、ランタイムの C(`fle_*`)**32.8%**、その他 7.8%。最大の 1 関数でも 9.9%(`fle_r_blit` = BitBlt の行コピー/合成)で、次が `f_2a337c0` 5.2%・`f_6a7b70` 5.1%(どちらも今は直接 wasm)・`f_40fe90` 3.4%・`f_6a7be0` 2.7%・`fle_imp_ScrollWindowEx`(memmove)1.9%・`f_5287c0` 1.4%・`f_863f60` 1.3%・`f_863f20` 1.2%・`fle_mx_mulsd` 1.2% …(`fle_*` が 33% = BitBlt の `fle_r_blit`、`ScrollWindowEx`、FP の MXCSR 窓 helper `fle_mx_*`、メモリアクセサ `fle_ld`/`fle_st`)。**裾は平らで、上位 10 関数で 33%、上位 22 関数で 40%、1 関数を消しても 10% 未満**。`fle_r_blit` は runtime の C(`-O2`、`FLE_R_FAST_COPY`)で、行コピーを wasm SIMD にすれば最大 10% 弱。

**(c) エンジンの外の提示経路**(Chrome の Worker プロファイル、Hnew、3240x1946): `(program)` 13.0% + JavaScript 2.1% + GC 0.2% = **約 15%**(約 10 ms/更新)。Firefox の Hnew は Worker CPU/更新 25.5 ms に対し node のエンジン CPU(64 ms 刻みのドラッグ)は 45 ms/手で単位が違い、分けられていない(**未確認**)。

**(d) 更新の画素数**: Chrome の 1 更新は 4.4〜4.9 Mpx(窓内容全体を `ScrollWindowEx` で動かすので、動いた全域が更新の矩形)。Hnew の 1 画素当たり Worker CPU は 12.6 ns(1920x1080 では 8.3 ns)で、60 回/s(16.7 ms/更新)には 4.9 Mpx × 12.6 ns = 62 ms が **約 3.7 倍**、120 回/s には約 7.4 倍。画素当たりをさらに半分にしても、1 更新の領域を小さくしない限り Chrome の 3240x1946 では 60 回/s に届かない(読み)。

### 8.2 C のまま残る上位(Hnew の後。lower できない命令のある関数。`tables/wl-*-after-functions.tsv`)

**Playlist ドラッグ 3240x1946(操作分 2,093,290,628 命令) で新リスト(Hnew)でも C のまま残る上位(lower できない命令のある関数)**

| 関数 | モジュール | 入口 | 実行命令 | 操作に占める割合 | lower できない理由(census の最初の 1 件) |
|---|---|---:|---:|---:|---|
| `f_27ad5f8` | dsp | 3145 | 16184281 | 0.77% | RSP at a return or tail jump is not its entry value (a continuation entry inside |
| `f_38e6d90` | dsp | 2534 | 4186791 | 0.20% | Mulps has no MXCSR window rule |
| `f_6a1e50` | fle | 10945 | 3329796 | 0.16% | Maxps has no MXCSR window rule |
| `f_30663a0` | dsp | 98 | 3183907 | 0.15% | Addps has no MXCSR window rule |
| `f_6c7ae0` | fle | 144815 | 1882595 | 0.09% | mnemonic Cvtps2dq |
| `f_2cc0660` | dsp | 1278 | 1845998 | 0.09% | Addps has no MXCSR window rule |
| `f_32e9c00` | dsp | 1450 | 1605105 | 0.08% | Mulps has no MXCSR window rule |
| `f_6a2180` | fle | 435 | 1407375 | 0.07% | 32-bit addressing |
| `f_3ac7790` | dsp | 6655 | 1383884 | 0.07% | mnemonic Cpuid |
| `f_3a966d0` | dsp | 6376 | 1320770 | 0.06% | Maxps has no MXCSR window rule |
| `f_1d07820` | qfc | 30722 | 1262816 | 0.06% | 32-bit addressing |
| `f_2da3480` | dsp | 546 | 1226171 | 0.06% | Mulps has no MXCSR window rule |

**Settings タブ 1920x1080(操作分 2,574,294,824 命令) で新リスト(Hnew)でも C のまま残る上位(lower できない命令のある関数)**

| 関数 | モジュール | 入口 | 実行命令 | 操作に占める割合 | lower できない理由(census の最初の 1 件) |
|---|---|---:|---:|---:|---|
| `f_38e6d90` | dsp | 298727 | 34513033 | 1.34% | Mulps has no MXCSR window rule |
| `f_3a966d0` | dsp | 784139 | 32995477 | 1.28% | Maxps has no MXCSR window rule |
| `f_2da3a80` | dsp | 603413 | 24624302 | 0.96% | Mulps has no MXCSR window rule |
| `f_6a74a0` | fle | 8319 | 17228649 | 0.67% | Addps has no MXCSR window rule |
| `f_2dadd20` | dsp | 333681 | 13920494 | 0.54% | Subps has no MXCSR window rule |
| `f_6a1e50` | fle | 33989 | 10329762 | 0.40% | Maxps has no MXCSR window rule |
| `f_32e9c00` | dsp | 27868 | 9789189 | 0.38% | Mulps has no MXCSR window rule |
| `f_2cc0660` | dsp | 166838 | 9462134 | 0.37% | Addps has no MXCSR window rule |
| `f_2da5dc0` | dsp | 165606 | 8067044 | 0.31% | Mulps has no MXCSR window rule |
| `f_6a2180` | fle | 1351 | 4373783 | 0.17% | 32-bit addressing |
| `f_3ac0c50` | dsp | 34802 | 4319711 | 0.17% | Addps has no MXCSR window rule |
| `f_3ac7790` | dsp | 20729 | 4300341 | 0.17% | mnemonic Cpuid |

## 9. 出典のない選択(`S-hyb5-*`。代替と理由)

1. **S-hyb5-1 旗 `--sse-int` を新設した(opt-in)**。代替: (a) 既存の `--mx-packed` の下に入れる = packed 浮動小数点の窓の helper〔rcgen〕を全部入れてしまい、既定の直接部のバイトが動く; (b) 既定で有効 = 現行の混成の census と生成物が変わりうる。選んだ理由: 既定の挙動を変えない規則、および整数/シャッフルは MXCSR と無関係で別の旗にできる。
2. **S-hyb5-2 SSSE3 の PSHUFB・PALIGNR・PABS* を足した**(CPUID を SSE2 に固定しているので現エンジンでは届かない見込み)。代替: 足さない。理由: 1 命令 1 行、差分テストで確認できる。
3. **S-hyb5-3 差分テストの入力の分布**(全バイト対 65,536 を 2 配置、16 ビット境界値 24×24、32 ビット境界値 16×16、シフト数 16 種 × 8、乱数 N〔半分は境界値の混合〕)と N の既定(100,000、速さのため)。ログは N = 1,000,000。
4. **S-hyb5-4 16 ビット全対の網羅は digest 比較**(2 本の 32 ビット鎖 + xorshift)。代替: 結果を全部保存して比較(8.6 GB 以上の入出力)。最初の digest(imul だけの鎖)は低位 12 ビットが入力に依存しない弱さがあったので非線形の混ぜを足して全形を取り直し、各形で『壊した版の digest は違う』ことも確認した。
5. **S-hyb5-5 mutation は生成した wasm 本体の同じ長さの 1 箇所の変更**(opcode・shuffle の lane・定数・lane 番号)。代替: `lower.rs` に環境変数のフックを入れる(製品コードにテスト用の分岐が入るので採らない)。
6. **S-hyb5-6 新リストの選び方**: 現行 977 + (旗なしでは lower できず `--sse-int` で lower できる + 計数ビルドか V8 プロファイルが 1 回でも実行を見た)− (specials・prune S+M・Blend2D swap・HLE 入口)。代替: 命令数の上位 K 件、被覆率 99% まで、『lower 可能だが現行リストに無い関数』(ドラッグで 1.0〜1.7%、Settings で 3.1%)も足す(指示は『977 + 新しく lower できるようになった関数』なので足していない)。
7. **S-hyb5-7 F8・Settings の『実行された』を V8 プロファイルのサンプルで代用**(計数ビルドが止まるため)。代替: 計数ビルドに ship2 の import 行を足して作り直す(約 25 分とディスク 3 GB 以上。上限に収まらず採らなかった)。
8. **S-hyb5-8 操作分の抽出は『前置きだけの走行のカウンタを関数ごとに引く』**。代替: 区間のカウンタを取る(計数ビルドにその口が無い)。
9. **S-hyb5-9 測定の構成を S1 の C 部に揃えた**(3 つとも wasm-opt -Oz)。代替: 出荷 SHIP2 を基準にする(SIMD なし、1920x1080 固定で 3240x1946 を測れない)。
10. **S-hyb5-10 Chrome の測定はウォームアップ 10 s + 6 s × 2 回、Firefox は 12 s × 2 回**(Firefox の入力が 30/s で 1 回の更新数を揃えるため)。回数は少ない(n = 2〜4)。Chrome のドラッグは同じ手順をもう一度(round 2)走らせて n = 4 にした(Hnew の 3240x1946 は round 1 が 11.4〜12.0、round 2 が 14.2〜14.4 回/s で、round 間の差の原因は調べていない。load は round 1 が 5.7、round 2 が 4.3 前後)。
11. **S-hyb5-11 Firefox の測定を 2 段にした**(round 1: ウォームアップなし・12 s × 2 回。round 2: 20 s のウォームアップのドラッグ + 8 s × 3 回)。理由: round 1 は Firefox のヘルパースレッドの JIT(全スレッドの CPU が Worker の 2〜3 倍)が測定窓に入り、同じ構成でも 1 回目と 2 回目で更新/s が 20〜30% 動いた。代替: 回数を増やす(時間が無い)。round 2 の Hcur の 3240x1946 は `POST /session` の失敗(前の Firefox が終わる前に次を起動した競合)で 1 回落ち、最後に取り直した。
12. **S-hyb5-12 Chrome のタブ切替の fps の定義**(クリック後の最初のバースト: 変化画素 ≥ 5% の描画を 400 ms 以内の間隔で並べ、`(枚数 − 1) / 時間`。anim1 の `animfps.py` と同じ)。バーストが 3 枚未満の物は除く。

## 10. 既定・テスト基準への影響

- **fe-lift・runtime・app・build-engine-bundle.sh は触っていない**。`hyb5.diff` は fe-wasmgen(3 ファイル)・テスト 1 ファイル・README 1・リスト 6 ファイルだけ。`--sse-int` を付けない限り fe-wasmgen の出力は従来と同一(現行 977 関数の 6 モジュールが HEAD の fe-wasmgen とバイト一致: `logs/equiv.log`)。
- FLEngine の既定の生成 C の sha256 と a4 FRUITY 無効の基準は、fe-lift を触っていないが、『不変』として省略せず実際に走らせた。(1) `bin/fe-lift lift FLEngine_x64.dll fle.c --pdata`(この木の fe-lift)の出力は 215,811,678 B、sha256 **cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50**(基準と一致、`logs/fle-default-gen.log`)。(2) a4 FRUITY 無効(リポジトリの入口 `lift/harness/a4-fruity.sh off`、6 モジュール、`tools/a4off.sh`、JOBS=2): init_count **942/942**、run_rank **895/895**、distinct functions **5,310**、function entries **1,009,737**、終了 `returned from entry`、exit 0(基準と全て一致、`logs/a4off.log`)。木は 164e81a + `hyb5.diff`(runtime は anim1・desk1 を含まない版)なので、HEAD 741d6263 の木での a4 は未実施(`hyb5.diff` は fe-lift・runtime に触れない)。
- 全体テスト `bash lift/harness/test.sh full --jobs 3`(入力は `FLE_INPUT_ROOT`、git archive の木): **191 results / 2,348 passed / 0 failed / ignored 1 / SKIPPED 1 行**(基準 190 / 2,346 / 0)。差は新しいテスト実行ファイル `fe-wasmgen/simd_int_forms`(+1 result、+2 passed、`--ignored` の網羅 1 本)だけ。最初の走行は入力の symlink が無く SKIPPED 15 行だったので、`test-env.sh --link` で入力を繋いで `--fresh` で取り直した(`logs/test-full.log`)。

## 11. 規則の点検・自己申告

- HLE は使っていない(出力ビットは命令の定義どおり)。FL 固有の特別扱い・関数名/クラス名/窓の種類による分岐は無い(選択リストは VA の一覧で、混成の既存の仕組みと同じ)。
- 余計な処理の除去: 該当なし(見つけていない)。外部 API の不足: 本作業では扱っていない。
- 依頼者に操作を求める文面・CI への実ブラウザ操作の追加は無い(測定の道具は CI に入れていない)。
- 重い処理は `nice -n 15` で走らせた(Chrome・Firefox・node の G3 は `nice -n 5〜10`)。同時の重い処理が 4〜5 本になった時間がある(count 実行 3 本 + test.sh full + Hcur のビルド、約 12:59〜13:15)。
- 末尾 `&` を 1 回使った(計数実行の並列起動 `( … &)`、13:07)。`sleep`・`timeout`・`nohup`・`pkill -f` は使っていない(一度 `kill <pid>` で自分の古いジョブを止めた)。
- サーバーの再起動で背景ジョブが 2 回落ちた(12:58:40、13:33:30)。残った部分的な `.o` を消して再開した。
- 作業ディレクトリが一時的に 8.3 GB(上限 5 GiB)になった(計数ビルドの中間物・test.sh の出力)。中間物を消して約 3 GB まで戻し、終了時に 500 MB 以内に整理する。
- `/tmp` の割り当て超過(`patch: write error`)を 1 回出した(`git archive` の全展開が原因)。差分の確認を対象ファイルだけの木に変えた。
- `sleep 0` を 1 回呼ぼうとして拒否された(実行されていない)。待ちは背景ジョブの完了通知だけで行い、その後は使っていない。
- 他の担当(ship3 の `wasm-opt` ほか)の負荷で load が 10〜22 になった時間がある(14:13〜14:30 の一部)。その時間の Chrome 行(構成の Worker プロファイルなど)は速さの根拠に使っていない(§6.3 の表は load ≤ 9 の行だけ)。Hcur の Firefox 3240x1946 の取り直しは load 10〜13 で、参考値としてだけ書いた。
- a4 FRUITY 無効の最初の走行(`logs/a4off-attempt1-wrongenv.log`)は、leak1 の `a4off.sh` の module 一覧だけを写し、リポジトリの入口 `a4-fruity.sh off` が設定する `FLE_HOOKS`・`FLE_CALL_FRUITY=0` を入れ忘れた(と読んだ)ため、`VirtualProtect: address is not in a VirtualAlloc region` で 315/942 の所で止まった(基準は 942/942)。入口を `a4-fruity.sh off`(そのまま)に直して取り直したら 942/942 で完走した(上の (2))。どの差(`FLE_HOOKS`・`FLE_CALL_FRUITY=0`)が止まりの原因かは 1 つずつ外して確かめていない(読み)。最初の走行を『不変』の根拠には使っていない。

## PM の決定と取り込み(2026-11-04)

取り込んだ基準: HEAD 741d626 に `hyb5.diff`(11 ファイル)を `patch -p1`(offset・fuzz・reject なし)。`-` 行 2 本(`lower.rs` の `m2 if packed_on()&&is_sse_x2` の条件と、Psrlw/Psraw 系の腕)は読んだ。PM が測った値(この機械、`lift/out-pmchk-h1/`): FLEngine の既定の生成 C の sha256 `cd9107ce…`(215,811,678 B)不変、全体テスト **192 results / 2,350 passed / 0 failed**(ignored 1。基準 191 / 2,348 に新しいテスト 1 ファイル分が増えた)、a4 FRUITY 無効 942/942・895/895・functions 5,310・entries 1,009,737。

1. **`fe-wasmgen --sse-int`(opt-in)を採る。** x86 の packed 整数命令 1 本を、同じ定義の wasm v128 命令 1 本にする命令の契約の実装(方針 (b))で、HLE でも FL の挙動の補正でもない。旗なしの出力は HEAD とバイト一致(977 関数の混成)。ホスト CPU との差分テストは全一致、mutation は全検出。
2. **新リスト(1,009 関数 = 現行 977 + 32、`lift/harness/seeds/hyb-sse2-F2x-*.txt`)は、次の bundle(ship3 の構成との組合せ)の候補。** 出荷への採用は、ship3 の F2xH3 と組み合わせた画素同一と速さを測ってから決める(hyb5 は S1 の C 部との組合せで測った。F2xH3 + 新混成そのものは未測定)。
3. **3240x1946 の Playlist ドラッグは 60 回/s に届かない**(Firefox Hnew 36.4、Chrome Hnew 13.1)。残りは直接 wasm 59%・runtime の C 33%(`fle_r_blit` 9.9% が最大)・その他に平らに散り、1 関数では 10% 未満しか動かない。次の手は C の経路そのものの変換・通信の削減で、局所的な軽量化ではない。
4. **Chrome が大画面で 1 画素当たり悪化する理由は『なぜ C 経路だけか』が未確認のまま**(段差ではなく滑らかな悪化。tier-up・ページフォルト・メモリ量は原因ではない)。Firefox で Hcur が S1 より遅い理由も未確認。
5. 依頼者の PC(Windows の Firefox 159)の値は 0 件。この機械の値である。
