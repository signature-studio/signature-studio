# 0164: 直接 wasm 生成部(fe-wasmgen)で OTT.vst3 の本物の機械語を混成で動かした(7,782 関数・process は命令の 99.997%)。出力は C 経路と一致、process は C 経路 -O2 の 3.0 倍(MXCSR 窓あり)〜6.8 倍(窓なし)

状態: **仮の決定(すべての選択)**。**これは新しい生成部の試作の測定で、進捗の到達ではない。** 本番のビルド(`fe-lift` の C 生成、`lift/runtime/`)は変えていない。依頼は PM(番号 0164 は PM の割り当て、仮)。調査・実装・測定・文書は作業者(wasmgen2)が行った(2026-11-03 07:45〜、予算 3 時間)。差分 `wasmgen2.diff` は HEAD 812e821 の `git archive` した `lift/` に対するもので、`lift/crates/fe-wasmgen/src/` の 6 ファイルの変更と新規 `xabi.rs`、新規の単体テスト `tests/sse_forms.rs`、`lift/harness/vst/hybrid_glue.c`(新規)だけ(9 ファイル)。`patch -p1 --dry-run` は 812e821 の書き出しで、ずれ・拒否なしで通った(下記「再現」)。

区分の凡例: **実測**＝このセッションのコマンドの出力を見た事実。**読み**＝コードや出力を読んで得た解釈で、確かめていない。**未確認**＝確かめていない。**推測**＝根拠が弱い見積り。**出典なしの選択**＝名前つき(`S-wg2-…`)で撤回条件を書く。速度は **CPU 時間**(`process.threadCpuUsage()` で wasm を動かすスレッド)で測り、負荷平均(共有機で 7〜10)を併記する。harness 自身の時計(ゲスト内の wall)で出した数は『harness wall』と書く。

## 冒頭の三点(正直さ)

1. **実行した本物と範囲(実測)**: OTT.vst3(Xfer Records、`lift/out-vst1/bin/OTT.vst3` を 0x2000000 に再配置した PE)の **x86-64 機械語を fe-wasmgen が直接 wasm にした 7,782 関数**(全 8,899 関数のうち。静的に関数の 87.4%、命令の 75.9%)が、vst2 のハーネス(`main_vst.c`)の中で **DllMain → InitDll → createInstance → setActive → process 40 ブロック → ExitDll → 2 本目のインスタンス** まで動いた。実行された命令では **init(DllMain→setActive、1.017e8 命令)の 94.0%、process(1 ブロック 7,822,656 命令)の 99.997%(7,822,407)**。残りは C 経路の生成 C(同じ PE から fe-lift が作った物)で、**C 経路の runtime(import の HLE・間接呼びの dispatch・MXCSR の遅い経路)は混成の中でも C 経路のまま呼ばれる**。process で C に残る関数は 1 つ(`0x2113bc0`、249 命令/ブロック、`lock cmpxchg` を含む)。
2. **代役・再生・推測の範囲(実測)**: 画像・録画・事前生成データの再生は無い。出力は実際に process して作る。ただし次の 2 つは『本物の機械語のうち実行されない経路を落とした』物: (a) `--trap-vex`: 新バックエンドは AVX/FMA(VEX/EVEX/XOP)を持たないので、その命令 **230 箇所**(lower 対象の関数内)を `unreachable` にしている。根拠は C 経路の CPUID が SSE2 しか名乗らず、AVX の経路に入らないこと(vst2 の C 経路の実行で VEX の実行は 0)。**4 シナリオと追加の入力 10 種以上のどれでも一度も落ちていない(実測)**が、CPUID が AVX を名乗る環境では成り立たない(`S-wg2-5`)。(b) MXCSR の『窓なし』の構成は測定専用で、生産の構成ではない(下記)。
3. **HLE: 入っている**。import(`fle_imp_*`)と間接呼びは新コードから C 経路の関数を呼ぶ(`c.fle_imp_<name>`、`c.fle_hyb_dispatch`)。HLE の本体は C 経路の物をそのまま使う。新バックエンドは HLE を持たない。OTT の import は 380 個、うち新が直接呼ぶ形にしたのは 0(すべて C 経路の関数を呼ぶ)。

## 先に結論

| 項目 | 結果 |
|---|---|
| 一致(実測) | 4 シナリオ(既定・入力×0.5・無音・`--param-first 0.25`)で out_hash が C 経路と一致: 8104f50e938ccf36 / 0c435d2acaa363ed / 96deca0f23b22325 / 07f081139e0d6448。MXCSR 窓あり・なし、さらに窓を常に失敗にして全 float 命令を C 経路の関数に回すテスト(`--mx-force-slow`)でも一致。窓を外れる入力 4 種(`--scale 1e25`・`1e-30`・`1e-39`・`1e30`)でも hash 一致 |
| 忠実度(MXCSR、実測) | 最終のゲスト MXCSR: 窓あり = C 経路と同じ(通常 0x1fa4、denormal 入力 `--scale 1e-39`・`1e-30` で 0x1fb6)。窓なしは denormal 入力で 0x1fa4(DE・UE の sticky を失う)。6 シナリオ全部で窓ありが C と一致 |
| 速度(process、実測、thread CPU、5 反復の中央値、load 3.6、最終の生成器) | **C 経路 wasm -O2: 9.92 ms/ブロック = 1.27 ns/元命令。新・MXCSR なし: 1.46 ms = 0.19 ns/元命令(6.8 倍速)。新・MXCSR 窓あり: 3.35 ms = 0.43 ns/元命令(3.0 倍速)**。窓の費用は窓なし比 2.3 倍(K2 の 5.8〜6.7 倍より小さく、PM の 4 倍の上限の内)。load 8.2 の別の測定(窓の判定を最適化する前を含む)は §3 |
| init(DllMain→setActive、1.017e8 元命令、実測) | C 100.0 ms(0.98 ns/命令)→ 新 75.0 ms(窓なし、0.74)/ 76.8 ms(窓あり、0.76)。元命令の 94% を新にしても 23〜25% しか縮まない(内訳は §3 の profile) |
| 起動全体(node 起動 → setActive、thread CPU 中央値 7 回、実測) | C 経路 144.2 ms → 混成 124.9 ms(−13%)。ただし混成を作ったままでは 151.8 ms で C より遅かった(emscripten の JS グルーが未解決の `f_<va>` 7,782 個のスタブを抱えて `vst.js` が 999 KB になる。除去は `tools/strip_glue_stubs.py`、§9) |
| 被覆(実測、`census` と計数実行) | 静的: 関数 7,782/8,899(87.4%)、命令 388,655/512,101(75.9%)。動的: init の命令 94.0%・関数 1,918/2,041・**呼び出し回数では 26.9%**(C に残る小さな関数が 11.7 万回呼ばれる)。process の命令 99.997%・関数 10/11 |
| 境界通過(実測) | process 1 ブロックあたり 新→C 関数 0、新→C runtime(import/dispatch)1、C→新 5。1 回の費用は 10 ns 以下(上限、下記)。init 全体(blocks=1): 新→C 関数 3,062、新→C runtime 24,938、C→新 31,958 |
| 差分テストで見つけた新バックエンドの実バグ | 5 件(すべて修正済み): 新→C 呼びで XMM 引数が渡らない / i64x2 シフトの命令番号の誤り / **GS:[0x58] を TEB 配列から直接読んだ(C 経路は遅延で埋める)** / `ucomisd x,x`(NaN 判定の慣用句)が遅い経路に落ち続ける性能の欠陥 / `cvttss2si` だけを含むモジュールで補助関数が出ない(新しい単体の差分テストが発見) |
| elision・メモリ検査(実測、窓あり、全関数。load 8 前後・3 反復・窓の判定の入れ子を平らにする前の生成器での相対比較で、絶対値は上の速度の行と比べない) | 既定(Exact+elision): n.wasm 10.90 MB・3.86 ms/ブロック。`--no-elide`: 11.52 MB(+5.7%)・3.96 ms。Elim: 10.36 MB・3.90 ms。Base: 10.62 MB。検査なし: 9.92 MB(−9%)・3.48 ms(−10%)。すべて hash 一致 |
| 全体テスト(実測) | HEAD 812e821 の書き出し + `wasmgen2.diff` で **151 results、2,095 passed、0 failed**(SKIPPED 1 件は既存の `fe-lift/wall_fr9t`)。同じ HEAD の素の書き出しの基準は 150 results・2,094 passed・0 failed なので、差は新規の単体テスト `sse_forms` の 1 results・1 passed |

## 1. 作ったもの

新バックエンドは phase 1(0157)の `fe-wasmgen`(x86-64 機械語 → wasm。レジスタは wasm のローカル、署名は活性解析、フラグは到達定義)を OTT 用に拡張した物。C 経路との混成(扱えない関数は C 経路に残す)を本命にしている(PM の仮の判断)。OTT の混成モードは `--c-imports FILE --mem-import c.memory --hybrid-r ADDR --hybrid-merge --teb ADDR`。C 経路の wasm と `wasm-merge` で 1 つの wasm にし、メモリは C 側の `c.memory` を共有する。

### 1.1 境界(読み+実測)

- 新→C: スタブが `R[4]`(戻り番地を置く rsp)と C の関数が読むレジスタを `R[]` に書き、`c.f_<va>` を呼び、結果を `R[]` から読む。C→新: 新が輸出する wrapper `f_<va>` が `R[]` から読み、callee-saved の `R[]` を保存・復元する。
- import 呼び(`call [IAT]`・`jmp [IAT]`)は C の `fle_imp_<name>`(ハーネスが包む 10 個は `__wrap_fle_imp_<name>`)を呼ぶスタブ。**間接呼び・間接 jmp はすべて C の `fle_hyb_dispatch(va)`**(adapter 表なし。C の dispatch は『C 経路が配る HLE の番地』も受けるので、上位 32 ビットの検査は外した)。MSVC の CFG ガード `jmp rax` のため、間接呼びは rax を『弱い読み』に含める。
- **GS**: `mov r64, gs:[off]` は C の `fle_hyb_gs_ld8(off)` を呼ぶ。他の GS の形は C 経路に残す(`S-wg2-4`)。

### 1.2 XMM の境界(`xabi.rs`、新規)

C 経路は XMM を配列 `X[]` に持つ。新は v128 ローカルに持つので、関数の境界で次の規則で `X[]` とやりとりする:
- 全関数(C に残る物も)の要約 `xin`(関数に live-in の XMM)と `xw`(書く可能性のある XMM、呼び先を含む)を不動点で求める。
- 関数は入口で live-in の XMM を `X[]` から読み、呼びの前に呼び先が読む・書く XMM のうち自分が書いた物を書き出し、呼びの後に呼び先が書いた XMM のうち後で使う物を読み直し、return の前に自分が書いた XMM を書き出す。
- 間接呼びと import は Windows x64 の規約を仮定: xmm0〜3 を読み、xmm0 だけ書き、xmm6〜15 は保存される(`S-wg2-1`。検出はしない)。直接呼びは規約を仮定せず、要約をそのまま使う。
- 純粋(C なし)のモードでは従来の制限(XMM が境界を越えない)のまま。`--hybrid-x ADDR` でこの機構が入る。

### 1.3 SSE 命令の追加

`movd/movq`(xmm↔gpr・メモリ)、`movntps/movntdq/movntpd`、`cvtdq2ps/cvtdq2pd/cvtps2pd/cvtpd2ps/cvttpd2dq/cvttss2si`、`sqrtps/sqrtpd/sqrtss`、`maxsd/minsd/maxss/minss/maxpd/minpd/maxps/minps`、`cmpsd`、`paddb/w/d/q`・`psubb/w/d/q`、`psrlq/psllq/psrldq/pslldq`(即値)、`punpcklbw/punpcklqdq/punpckhqdq`、`ucomisd/comisd/ucomiss/comiss`(フラグ形 `FCmp`)。`cvttpd2dq` は x86 の『整数の不定値 0x80000000』を、範囲外・NaN の lane で wasm の飽和(0 / 0x7fffffff)と置き換えて合わせる。

### 1.4 MXCSR の窓(`--mx-window --hybrid-mx MXCSR BUF`)

C 経路の窓の規則(`lift/runtime/fle_mxcsr_vec.h`、決定 0154)を新で再実装した。窓の中なら wasm の演算そのまま(PE はすでに立っていて MXCSR は変わらない)、外なら **C 経路が生成 C で呼ぶのと同じ関数**(`fle_mx_*`)を C の `fle_hyb_mx(kind, imm)` 経由で呼ぶので、NaN・denormal・丸め・MXCSR の sticky は C 経路そのもの。
- 対象: スカラー add/sub/mul/div/sqrt/max/min(ss/sd)、cmpsd、ucomis/comis、cvtss2sd・cvtsd2ss・cvtps2pd・cvtpd2ps・cvtdq2pd・cvtdq2ps・cvttpd2dq・cvttsd2si・cvttss2si・cvtsi2sd(64 ビット)、sqrtpd。**窓の規則を持たない float 形(addps/addpd 等の packed 算術、sqrtps、maxps/minps)を含む関数は新にしない(C 経路に残る)**ので、窓ありの構成は窓の規則の無い命令を黙って通さない。
- 判定は 2 段: 狭い整列窓(f64: 指数 767〜1278 = 2^-256〜2^256、f32: 95〜158 = 2^-32〜2^32)を **SIMD で 2 オペランド分まとめて 1 回**(`((a<<1)-K | (b<<1)-K) >> 62 == 0` を v128 で、汎用レジスタへ移さない)→ 失敗なら C 経路と同じ範囲の精密な窓(0 を含む)→ 失敗なら C の関数。RC=0・PE と PM が立っている、のガードは関数ごとに 1 回(入口と、呼び・ldmxcsr の後に)ローカルへ計算する。
- **`ucomis`(`comis` ではない)は quiet NaN のオペランドを窓に入れる**: SDM で ucomis は signaling NaN の時だけ IE を立てるので、quiet NaN どうしの比較はフラグを変えない(`S-wg2-3`)。これを入れなかった最初の版では OTT の `cmplesd; cmpnltsd; andps; ucomisd x,x; jp`(マスクを NaN として判定する慣用句)が毎回遅い経路に落ち、process あたり 3.6 万回 C を呼んでいた。
- `ldmxcsr`/`stmxcsr` は C の `fle_mxcsr_load/store` を呼ぶ(以前の混成は新モジュール自前のグローバルを使い、C 経路の MXCSR と食い違っていた)。
- テスト用: `--mx-force-slow`(窓の判定を常に失敗にする)。

### 1.5 その他

- `--trap-vex`(上記)。`--hybrid-cnt ADDR`(境界通過の計数、既定は無効)。`hybrid_glue.c`(新規): `fle_hyb_dispatch`、`fle_hyb_gs_ld8`、`fle_hyb_mx`、計数。
- ビルド手順: C 経路の C ユニットから新が lower した関数の定義を除き(`strip_units.py`)、emcc でリンク(`-Wl,--allow-undefined`、link map から `R`・`X`・`fle_teb`・`fle_mxcsr`・`fle_hyb_buf`・`fle_hyb_cnt` の番地を読む)、その番地で新モジュールを生成し `wasm-merge`。C 部分は 7.81 MB(C 経路のみは 27.85 MB)、新は 10.90 MB、結合後 19.55 MB。新のうち wrapper が 2.38 MB: C の dispatch 表が任意の関数に届き得るので、lower した全関数の wrapper が要る(読み)。

## 2. 一致の検証(実測)

`tools/scen.sh` で同じ 4 シナリオを走らせ、C 経路 wasm -O2(`ottw-O2-b1`)の hash と比べた(`tables/scen-*.txt`):

| シナリオ | C 経路 | 新・窓なし(7,782) | 新・窓あり | 窓 + 常に遅い経路 |
|---|---|---|---|---|
| 既定 | 8104f50e938ccf36 | 一致 | 一致 | 一致 |
| 入力×0.5 | 0c435d2acaa363ed | 一致 | 一致 | 一致 |
| 無音 | 96deca0f23b22325 | 一致 | 一致 | 一致 |
| `--param-first 0.25` | 07f081139e0d6448 | 一致 | 一致 | 一致 |

窓を外れる入力 4 種(`tables/scale-hashes.txt`: `--scale 1e25` = 68110585f623c151、`1e-30` = 100704693f80db56、`1e-39` = 604fab929e5dc25a、`1e30` = 2c13499768da4822)でも、全関数の窓なし・窓あり・窓+常に遅い経路の 3 構成すべてが C 経路と一致した。240 ブロックの hash は 11e95c6c38b174b3 で C と一致。elision・メモリ検査の 4 つの変種(§7)も 4 シナリオで C と一致(`tables/scen-v_*.txt`)。

MXCSR の最終値(`tables/mxcsr-compare.txt`、harness が毎ブロック表示する `guest MXCSR`): 既定・×0.5・無音・1e25 は 3 者とも 0x1fa4。`--scale 1e-39`・`1e-30`(float の denormal 入力)は C = 0x1fb6、**窓あり = 0x1fb6、窓なし = 0x1fa4**。したがって窓なしは DE・UE を失う(測定専用である根拠)。

**最終の生成器での取り直し(実測)**: 窓の判定の入れ子を平らにした最後の変更のあとで、窓あり・窓+常に遅い経路の 2 構成を 4 シナリオ(C と一致)、追加の入力 10 種(`--scale` 0.01・2・100・1e25・1e-30・1e-39・1e30、`--param-first` 0・0.5、`--scale 0.3 --param-first 0.9`。`tables/extra-scenarios-final.txt`: 不一致 0)、最終 MXCSR(`tables/mxcsr-compare-final.txt`: 窓あり・窓+常に遅い経路とも C と同じ)で取り直した。それ以前の構成では 4 シナリオ+追加 11 種(`tables/extra-scenarios.txt`)も不一致 0。

**追加した SSE 命令形の単体の差分テスト(実測、`crates/fe-wasmgen/tests/sse_forms.rs`、新規)**: 追加した 49 の命令形(`cvtdq2ps/pd`・`cvtps2pd`・`cvtpd2ps`・`cvttpd2dq`・`cvttss2si`/`cvttsd2si` の 32/64 ビット・`sqrtpd/ps/ss`・`max/min` の ss/sd/ps/pd・`cmpsd` の述語 0〜7・`padd*`/`psub*`・`psrlq/psllq/psrldq/pslldq` の即値・`punpck*`・`movq xmm,xmm`・`movd/movq r,xmm`)を 1 命令ずつの関数にして lower し、node で 600 入力(0・-0・±inf・NaN・denormal・整数範囲外・乱数)を流し、**同じ命令のホスト CPU(`core::arch`)の結果と比べた: 全部一致**(float の結果は NaN どうしを同値とする: wasm は NaN の payload を規定しない)。テストが効くことは、期待値の `psrlq` の即値を 3 から 4 に変えて 599/600 入力で落ちることで確認した。窓(MXCSR)の遅い経路はこのテストの対象外(OTT の hash と最終 MXCSR で見た)。

## 3. 速度(実測)

`tools/bench.sh`: 1 回の実行ごとに `--blocks 40` と `--blocks 240` を走らせ、node プロセスの `threadCpuUsage()` の差を 200 で割る(5 反復の中央値、`tables/bench-4.txt`)。init はハーネスの出力行(`vst: image base`、`vst: LoadLibraryW`、`RESULT init_cpu_ms`)の時点の thread CPU の差。load は 3.6(その時間帯の `/proc/loadavg`)。

| 構成 | process ms/ブロック(最小〜最大) | ns/元命令 | C 比 | init(DllMain→setActive)ms | ns/元命令(init) |
|---|---|---|---|---|---|
| C 経路 wasm -O2 | **9.92**(9.89〜10.21) | 1.27 | 1.0 | 100.0 | 0.98 |
| 新・MXCSR なし(7,782 関数) | **1.46**(1.43〜1.50) | 0.19 | 6.8 倍速 | 75.0 | 0.74 |
| 新・MXCSR 窓あり(7,782 関数) | **3.35**(3.31〜3.40) | 0.43 | 3.0 倍速 | 76.8 | 0.76 |

分母: process は 1 ブロック(4096 フレーム、44.1 kHz = 92.88 ms 分の音)の元命令 7,822,656、init は 1.017e8。表は `tables/bench-4.txt`(load 3.6、最終の生成器。直前の `bench-3.txt`(load 3.6〜4.0)は C 10.13・窓なし 1.47・窓あり 3.42 ms で、同じ傾向)。窓の費用の経過(別の時間帯の測定、`tables/bench-1.txt`・`bench-2.txt`、load 8.2〜8.7): 初版(各オペランドを個別に判定)5.04 ms → 2 段+SIMD 判定 3.99 ms(C 10.35・窓なし 1.61 ms の測定と同じ時間帯)→ 判定の入れ子を平らにして 3.35〜3.42 ms(上の表)。窓の判定をスカラー double の add/sub/mul/div だけ外すと 3.6 → 2.4 ms(harness wall、診断)で、費用の大半はスカラー double の算術(読み)。

**P3 の分母としての読み(推測)**: 1 ブロックの 92.88 ms に対し、OTT 1 インスタンスの process は C 経路 10.7%、新・窓あり 3.6%、新・窓なし 1.6%(1 スレッド)。1 スレッドの 50% を OTT 級の float 処理に使えるなら、同時に動かせる数は C 経路 4.7、新・窓あり 13.9、新・窓なし 32(単純な割り算。実プロジェクトの他のプラグイン・FL のミキサーは含まない)。

**init の内訳(実測、`node --cpu-prof`、200 µs、`--no-process`、DllMain→setActive と 2 本目のインスタンスを含む 213 ms)**: wasm の自己時間のうち、名前のある `f_<va>`(新のコードと wrapper)49 ms、名前のない部分(C 部分と helper)46 ms、node/JS(ファイル読み・instantiate・loader)77 ms、idle/program/GC 41 ms。名前なしのうち最大は `fle_hyb_dispatch` の 7.5 ms(間接呼びのたびに C の dispatch を通る費用。新にした 7,782 関数の間接呼びの箇所は静的に 5,436)。init の 77〜80 ms は新と C の部分がほぼ半々で、新の部分は 0.26 ns/元命令前後(読み)。dispatch を新の中で表引きにする(adapter 表)と減らせる可能性がある(推測、未実装)。

**process の内訳(実測、`node --cpu-prof` 100 µs、300 ブロック、窓あり)**: 自己時間は `f_21b7100` 24.5%(1.13 ms/ブロック、0.51 ns/命令)、`f_21a5ce0` 21.1%(0.97 ms、0.47 ns/命令)、`f_200b640` 18.3%(0.84 ms、0.54 ns/命令、1 回 12.9 ns で float 12 演算)、`f_21b83d0` 14.6%(0.67 ms、0.40 ns/命令)、`f_200b590` 1.8%。熱い 4 関数で 78%(割合は profile 全体の 1,383 ms に対する。残りは node の起動・ローダー・init)。窓の費用は float 演算の個数に比例し、`f_200b640` のような状態変数フィルタ(演算 12 個、すべて前の演算の結果か状態の読み)では各演算が 2 オペランドの判定を要する。

**今後の方向(推測、未実装)**: 関数ごとに『入力(メモリの読みとレジスタ)を 1 回だけ窓に照らし、関数の中の演算の連鎖から指数と最下位ビットの粗さを区間解析して、窓を出ないと証明できた関数は判定なしの版を使う』二版方式。`f_200b640` は積の深さが 1 で、f32 の入力 2^-32〜2^32(または 0)から全中間値が 2^-110 の粒度・2^70 以下に収まる(読み)ので、入口の 1 回の判定だけで済む見込み。窓の費用の大半(3.99 → 1.61 ms の差)を除ける可能性がある。

窓を外れる入力は遅い(実測、harness wall、load 約 8): `--scale 1e25` で C 経路 16.3 ms/ブロックに対し新・窓あり 53.1 ms(3.3 倍遅い)、`1e-39` で 34.0 に対し 80.8(2.4 倍遅い)。通常の音声(窓の内)では遅い経路は init で 7 回だけ。窓の外のデータが続く場面(denormal の尾など)の最悪は C 経路より遅い(`S-wg2-2` の撤回条件)。

## 4. 被覆(実測)

`tools/coverage.py census/s2m.tsv`(`tables/coverage-s2m.txt`)。**静的**: 関数 7,782/8,899(87.4%)、命令 388,655/512,101(75.9%)。**動的(計数実行)**:

| | 実行された関数 | 新にした関数 | 呼び出し回数のうち新 | 命令のうち新 |
|---|---|---|---|---|
| init(DllMain→setActive) | 2,041 | 1,918(94.0%) | 26.9%(184,175 回中) | **94.0%** |
| process(1 ブロック) | 11 | 10 | 100.00% | **99.997%** |

熱い関数(process 1 ブロック): `0x21b7100`(36,867 回、221 万命令)・`0x21a5ce0`(1 回、208 万)・`0x21b83d0`(24,576 回、170 万)・`0x200b640`(65,536 回、157 万)が命令の 97%。残り 7 関数は 20 万以下。

**C 経路に残る関数と原因(静的、命令形別、主な物。関数数は『最初の原因』で数えた)**: `xadd` 297 関数(原子操作は C に残す決定)/ RSP が return で入口値でない(継続入口) 215 / 呼び先が opaque 102 / `cvtsi2ss` 57 / デコード不能 46 / `shufps` 44 / `xchg` 43 / `unpcklps` 35 / `unpckhpd` 30 / `cmpxchg` のフラグ 21 / packed `addpd` の窓の規則なし 21 / `unpcklpd` 10 / `mul` の Opaque フラグ 8 / `shr` 6。

**実行された C 経路の関数との突き合わせ**: init で C に残る 123 関数・610 万命令のうち 594.7 万命令(97%)は『RSP が入口値でない継続入口』19 関数。process で C に残るのは `0x2113bc0`(`cmpxchg` のフラグ、249 命令/ブロック)だけ。

## 5. 差分テストで見つけた新バックエンドの実バグ(5 件、すべて修正済み)

見つけ方: 混成の全関数版や熱い関数の混成を C 経路の hash と比べ、食い違いを二分(ddmin)・スタック・コードで追った。

1. **新→C の呼びで XMM 引数が渡らない**(`0x200b010` が `0x200b2e0` へ xmm1..3 を渡す)。ddmin で 6,675 → 1 関数。修正: `xabi.rs`(§1.2)。phase 1 の混成は XMM が境界を越える関数を含めなかったので出なかった。
2. **i64x2 のシフトの命令番号の誤り**(`i64x2.shr_u` を `i16x8.extend_low_i8x16_u` の番号で出した)。`wasm-validate` が検出。修正: 0xcd/0xcb。
3. **GS:[0x58] の読み**: 新コードは `--teb` の番地(C の `fle_teb` 配列)を普通のメモリとして読んだ。C 経路の `fle_gs_ld8` は配列を読まず、**読むたびに `fle_teb[0x58]`(TLS 配列のポインタ)と `[0x30]` を書いて答える**ので、C 経路が GS を一度も読む前は 0 が返る。結果、MSVC の thread-safe static(`_Init_thread_epoch` を TLS から読む `0x201d310`)が『初期化済み』と誤判定し、CS の初期化を飛ばして `EnterCriticalSection ... not initialized` で停止した(全 7,782 関数の版でだけ起きる相互作用型: 一部の関数だけ新にすると先に C が GS を読んでしまう)。ddmin は 1,834 → 12 関数まで縮めたが完了前に打ち切り、**merge -g で名前付きにしたスタック**と `fle_vex.h` を読んで原因を確定、修正で全関数版が hash 一致した(ddmin 単独では特定できていない)。修正: GS の 64 ビット読みを `fle_hyb_gs_ld8` に回す。
4. **`ucomisd x,x` が遅い経路に落ち続ける**(正しさではなく性能の欠陥): §1.4。
5. **`cvttss2si` だけを含むモジュールで、x86 の整数不定値を返す補助関数 `mx_slow_i64` が生成されない**(補助関数の要否を `cvttsd2si` の有無だけで見ていた)。呼びが別の関数の番号になり `wasm-validate` が型の不一致で落とす。新しい単体の差分テスト(§2)が見つけた。OTT では `cvttsd2si` も同じモジュールにあるので出ていなかった(実測: 修正前後で OTT の n.wasm はバイト単位で同一)。修正: `lib.rs` の `need_cvtt` に `Cvttss2si` を加えた。

phase 1 の差分テスト(freetype)が見つけなかった理由(読み): (1)(3)は C 経路と混ぜて初めて出る境界・状態共有の問題、(4)は OTT のコード慣用句、(2)(5)は freetype に該当する命令・組み合わせが無かった(freetype の関数に `cvttss2si` だけを含むモジュールは無い)。

## 6. 境界通過(実測)

`--hybrid-cnt` の計数(`tables/crossings.txt`、`FLE_HYB_STATS=1`): blocks=1 の全体で 新→C 関数 3,062 / 新→C runtime(import・dispatch)24,938 / C→新 31,958。blocks=41 との差 ÷ 40 = **process 1 ブロックあたり 新→C 関数 0・新→C runtime 1・C→新 5**。

1 回の費用: 熱い葉の関数 `0x200b5b0`(2 命令、1 ブロックに 32,768 回呼ばれる)だけを新にし、残りを C にしたテスト(C→新の通過が 32,768 回/ブロック)で、C のみ(10.79 ms)と 10.61 ms(測定のばらつき ±0.3 ms)の差が見えなかった。したがって **1 回は 10 ns 以下(上限)**(0.3 ms ÷ 32,768)。init の通過 約 6 万回は 0.6 ms 以下で、init の 77 ms の主因ではない(読み)。

## 7. elision・メモリ検査(実測+読み)

全関数、窓あり、同じ 7,782 関数、すべて hash 一致(`tables/variants-full.txt`):

| 構成 | n.wasm | xz -9 | 本体(code bytes) | process ms/ブロック | init ms |
|---|---|---|---|---|---|
| Exact + elision(既定) | 10,898,007 | 1,240,932 | 8,255,234 | 3.86 | 83.0 |
| `--no-elide` | 11,518,445(+5.7%) | 1,212,008 | 8,676,098 | 3.96(+2.8%) | 82.9 |
| Elim | 10,363,061(−4.9%) | 1,175,884 | 7,720,367 | 3.90 | 79.0 |
| Base | 10,619,022 | 1,189,024 | 7,976,299 | 4.15 | 79.0 |
| 検査なし | 9,920,396(−9.0%) | 1,116,892 | 7,277,854 | 3.48(−9.7%) | 68.9 |

(時間は 3 反復の中央値で、load 8 前後。±5% はばらつきの内と読む。)

**elision の既定(未決)**: 保存・復元の除去は n.wasm を 5.7% 縮め、時間は測定のばらつき内。除去の前提『callee が呼び元の save スロットを上書きしない』は phase 1 の seed 7 で 1 関数(`0x4b57330`)が破った(付録 A)。OTT の 4 シナリオでは破られなかった(hash 一致)。→ 利得は小さく、前提は反例がある。**既定を ON にするなら『前提が破れる関数を検出して外す』手当てが要る(未実装)**。

**Elim を C 経路と同一にできるか(読み。実装はしていない)**: Elim の差は『基底が負(上位 32 ビットが 1)で変位を足すと 4 GiB 未満に戻る』場合に C は通し、Elim は基底検査で落とす、の一方向。同一にするには基底が負の時だけ全アクセスで実効アドレス全体を検査する分岐が要る。その分岐のアクセスごとの費用(`local.get; if` + 検査の二重化)は Exact の 5 命令の検査と同程度で、Elim の利得(今回: サイズ −4.9%、時間は有意差なし)を消す。もう一つの案(落ちる代わりに関数を C 経路で再実行)は、関数の途中の副作用のため不可。**結論: OTT では Elim の時間の利得は測定できず、サイズ 5% だけ。同一化の費用はその利得より大きい。Exact を既定のままにするのが妥当(推測)。** 全関数・窓ありの Elim・Base・検査なし・`--no-elide` は 4 シナリオとも落ちず C と hash 一致(実測)。これは OTT のこの実行では差が出なかったことを示すのみ。

**memory64(報告のみ、読み。`notes/memory64.txt`)**: Chrome は Memory64 を M133(133.0.6875.0)で既定有効にした(chromestatus 5070065734516736、Chromium issue 388290793)。V8 は x64/arm64 で `wasm_memory64_trap_handling`(M127 から既定)を使い、memory64 でも**添字を `max_memory_size - end_offset` と比べる明示の比較を 1 回入れる**(issue に引用されたコード `set_trap_on_oob_mem64`)。wasm32 は 4 GiB 予約で比較なし。Firefox は 134(SpiderMonkey のブログ 2025-01-15: 明示の bounds check で遅い)。Wasm 3.0(2025-09-17 完了)に含まれる。fe-wasmgen への含意: identity map の wasm32 で上位 32 ビットを自前で検査する今の方式は残る。memory64 にしても『4 GiB 以上の番地で C 経路は落ちる』という意味は、エンジンの検査(メモリ サイズとの比較)では再現できない(読み)。

## 8. 出典なしの選択と撤回条件

- **S-wg2-1**: XMM を境界で `X[]` とやりとりし、間接呼びと import は Windows x64 の規約(xmm0〜3 を読み xmm0 を書く、xmm6〜15 は保存)を仮定する。撤回: 規約を破る間接呼びの先(xmm1〜5 を呼び元が使う)が見つかったら、その関数群を C に残すか、間接呼びの要約を全 XMM にする。
- **S-wg2-2**: MXCSR の窓の幅(整列窓 f64 2^±256、f32 2^±32 → 精密窓は C 経路と同じ)。窓の内で wasm の演算が C の遅い経路と同じ結果になる論拠は C 経路の文書の読み(正しさの証明ではない)で、実測の裏づけは『窓を常に失敗にした版と hash が同じ』『最終 MXCSR が C と同じ』。撤回: 差分テストで窓の内の食い違いが 1 つでも出たら、その形の窓を狭める。性能の撤回: 窓の外のデータが続く実プロジェクトで C 経路より遅くなる場合は、遅い経路を C の関数呼びから wasm 内の整数実装にする。
- **S-wg2-3**: `ucomis` は quiet NaN のオペランドで IE を立てない(SDM)として窓に入れる。撤回: 実機の MXCSR との照合で食い違ったら外す(実機の照合は無い)。
- **S-wg2-4**: GS は 64 ビット読みだけ C の関数経由。撤回: 他の形が lower の対象になった時。
- **S-wg2-5**: `--trap-vex`(AVX/FMA 経路は入らない)。撤回: C 経路の CPUID が AVX を名乗るようになった時は使えない。
- **S-wg2-6**: 除去(elision)の前提『callee が呼び元の save スロットを上書きしない』を既定で ON のまま残す(phase 1 の選択を踏襲)。撤回: 付録 A の反例が OTT 以外でも出る時。`--no-elide` を持っている。

## 9. 未確認・限界

- 実機(Windows)との照合は無い。『C 経路との一致』であって『本物の OTT との一致』ではない。
- OTT の実行は 4 シナリオ+入力のスケール 3 種。VST3 の実ホストでの再生は未実施。process の窓の外の入力(denormal・巨大値)の性能は C 経路より悪い(§3)。
- 窓なしの構成は測定専用。窓ありでも packed 算術を含む関数(静的に 21+)は C に残る。
- init は新にしても 25% しか縮まない。新の部分と C の部分(dispatch・HLE)がほぼ半々(§3)。P1 の 10 秒とは別の問題。
- ddmin は完了していない(§5-3)。全体の一部の関数だけを新にした組み合わせで C 経路の状態共有の問題がほかにも潜む可能性は否定できない(3 件目が示したように、混成の相互作用は全関数版で初めて出るものがある)。
- S-wg2-1(間接呼びの先の XMM 規約)は検出していない。『間接呼びの後で xmm1〜5 が live な箇所』を数える診断を試作したところ、5,436 箇所中 3,429 を報告した。原因は要約が『スカラー書き込みを古い上位 lane の読み』と数える(C 経路と X[] の上位 lane まで一致させるための過大評価)ためで、意味のある検出にならないので外した(差分に含まれない)。規約違反の有無は未確認のまま。この過大評価は余分な入口の読み込み・書き出しも生み得る(性能の損、正しさには影響しない)。スカラー書き込みを『読みなし』に戻すテスト(診断用の環境変数)では n.wasm がバイト単位で同一だったので、少なくとも OTT ではこの規則による損は無い(実測)。3,429 の主因は別にあるが特定していない(未確認)。
- **起動(node 起動からハーネスの最初の行まで+DllMain→setActive、thread CPU、7 回の中央値、`tables/startup.txt`)**: C 経路 45.4 + 98.8 = 144.2 ms。混成を作ったままだと 73.9 + 77.9 = 151.8 ms で C より遅かった。原因は emscripten の JS グルー: C 部分に未解決の `f_<va>`(新が持つ関数)が 7,782 個あり、グルーがそれぞれに『missing function』のスタブを作るので `vst.js` が 75 KB → 999 KB になり、`--cpu-prof` で `wrapSafe`(1.7 → 12.9 ms)・`(anon)`(1.7 → 12.1 ms)・GC(2.0 → 7.3 ms)が増えていた。`wasm-merge` のあとスタブは死んだコードなので、`tools/strip_glue_stubs.py` で除くと `vst.js` は 159 KB、**47.1 + 77.8 = 124.9 ms(C 経路より 13% 短い)**。wasm のモジュールの同期コンパイル(検証)は C 25 ms・混成 20 ms で、増えていない。export を後から削っても(`tools/strip_exports.py`)効かない(実測)。起動全体が数十秒の本物の FL Studio では、これは wasm の実行時間の節約(init 25%)がそのまま効く部分で、モジュールの大きさ・グルーの大きさの影響は別に見る必要がある(未測定)。
- ブラウザ(Chrome の JIT)での測定は無い(node の同じ V8 だが、UI スレッド・音声の AudioWorklet・JSPI は未)。
- 追加した SSE 命令のうち OTT の実行で通らなかった形の差分テストは無い(§2)。

## 10. 再現

作業ディレクトリは `lift/out-wasmgen2/`(縮小済み)。手順の要点(スクリプトは `tools/`):
1. `git archive 812e821 lift` を展開し `wasmgen2.diff` を `patch -p1`。`cargo build --release -p fe-wasmgen`。
2. OTT を 0x2000000 に再配置して seeds を作る(vst2 の `seeds-fix2-r2000000.txt`)。`fe-lift` で生成 C を作り(`gen_vst.sh`)、`build_vst_wasm.sh` で C 経路の wasm を作る(約 5 分)。
3. `fe-wasmgen --census ... --report census/s2m.tsv`(lower できる関数の一覧)。`tools/build_hybrid.sh census/s2m.tsv --trap-vex --hybrid-x A --hybrid-mx B C --mx-window --hybrid-cnt D`(C 部分を作る)、`tools/hybrun.sh HYB OUT --trap-vex --mx-window` で新モジュールを作って結合し走らせる。
4. `tools/scen.sh DIR`(4 シナリオ)、`tools/bench.sh DIR 5`(速度)、`tools/coverage.py census/s2m.tsv`(被覆)。
5. 二分: `tools/build_bisect.sh` と `tools/trial.sh LIST`、`tools/ddmin2.py`。

## 全体テストの実行

最終のパッチでの数: `executables: run=3 cached=144`(テスト実行器の結果キャッシュ: 実行ファイルと入力が同じ 144 本は直前の合格を再利用し、実行し直したのは変更のあった `fe-wasmgen` の 3 本(encode・synthetic・sse_forms)とドキュメントテスト)。キャッシュを使わない最初の完全な実行は 450 s(上の表の 2,094 passed+14 は展開の不備)、2 回目は 230 s。

実行: 展開した `lift/` と repo 直下の `tools/`(`fe-lift/authenticode_check` が `../../../tools/authenticode_check.py` を使う)に `wasmgen2.diff` を `patch -p1` し、`FLE_INPUT_ROOT=<repo> bash harness/test-env.sh --link` のあと `bash harness/test.sh full --jobs 2 --threads 2`(wall 230 s、`CARGO_TARGET_DIR` は `lift/out-wasmgen2/exp-target`)。
- 最初の実行は `tools/` を展開に含めなかったため `fe-lift/authenticode_check` が 14 件落ちた(2,080 passed・14 failed)。展開の不備であり、パッチの影響ではない(`tools/` を足して再実行すると 14 passed)。最終のパッチでの結果が上の 151/2,095/0。
- 全体テストは最終の差分で走らせた。その後に差分へ手を入れていない(本文の差分と同一。差分ファイルのヘッダの時刻だけが違う)。


## 付録 A: phase 1(0157)の未決だった項目

phase 1 の差分テスト(freetype、1,639 関数 × 300 反復)の『不一致 0』は **seed 1 の値**。seed 7 では 1 件の不一致と 1 件の『新だけが落ちる』が出た:
- **`0x4b57330`(不一致 1)**: elision の前提が破れた。callee が呼び元の save スロットを野良の書き込みで上書きした(実測)。→ §7・`S-wg2-6`。
- **`0x4b281b0`(新だけが落ちる 1)**: C モジュールのデータ領域を走査してしまうテスト環境の差(読み)。新と C の配置が違うので、走査が C モジュール側の領域に入った時に新だけが落ちる。実バグとは判断していない。

## 付録 B: 規則の自己申告(この作業)

存在しない `bash` ツールの呼び出し 7 回(phase 1 の 5 回 + この作業の 2 回)。`sleep` を 1 回(拒否された)、末尾の `&` 1 回(phase 1)、`pkill -f` 1 回(phase 1)。大量の端末出力 1 回(スクリプトの出力抑止の失敗で約 700 行を端末に出した)。ddmin の python を PID 指定で終了させた 2 回。前回報告の時刻表記が実時刻より 3 分進んでいた 1 回。

## 付録 C: PM の決定(2026-11-03、取り込み時)

PM は『PM が決めること』の 4 項目を次のとおり決めた。いずれも仮の決定で、撤回条件は各項の末尾。

1. **MXCSR 窓あり**(忠実度を保つ構成)を生産の構成とする。窓なし(1.46 ms/ブロック)は測定専用で、生産には使わない。窓を外れる入力の遅さ(最悪 C の 3.3 倍)は未手当てなので、実プロジェクトで窓を外れる入力が多いと分かった時点で、wasm 内の整数実装への置換を依頼する。
2. **メモリ検査は Exact を既定のまま**とする。Elim は時間の利得が測定できず、サイズ −4.9% だけで、C 経路との同一性を守る費用がその利得を消すため。
3. **elision は OFF(`--no-elide`)を既定の運用とする**。利得はサイズ 5.7%・時間は誤差内で、前提が phase 1 の seed 7 で破られた例があり、前提が破れる関数の検出が未実装のため。検出を実装し、全差分テストで 0 件になった時点で ON に切り替えてよい。
4. **`--trap-vex` は C 経路の CPUID が SSE2 のみを名乗る間だけ成り立つ**(決定 0118)。CPUID の応答を AVX 以降へ変える変更は、この前提を破るので、その決定と同時に `--trap-vex` の扱いを見直す。

また、この決定の混成(新バックエンドの関数 + C 経路の runtime)は、OTT のパイロットの範囲の実測であり、FLEngine 本体・dsp への適用は未実施(次の決定)。
