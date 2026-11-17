# 0259(mt3x、仮): x87 命令を含む関数を直 wasm に載せる — 同じ fle_x87 のヘルパを直 wasm から呼ぶ

基準 HEAD: 2ca825ff(差分は c0a256bf の `git archive` に `patch -p1 --dry-run` が通る)。成果物: `mt3-x87-lower.diff`(731 行: `src/x87.rs` 新規 + `lower.rs` に 5 行 + `lib.rs` に 約 20 行 + `Cargo.toml`/`Cargo.lock` の dev-dependency + テスト `tests/x87_hybrid.rs`)。この機械(負荷 5〜20)、FL Studio の実物の `FLEngine_x64.dll` を `fe-wasmgen` に通した census とテストの結果。**実エンジンをブラウザや node で走らせた測定は 0 件**(下の 2)。

## 冒頭の 3 点
1. **作った物(事実)**: fe-wasmgen は x87 命令を一つも直 wasm にしていなかった(`Fld` 等は『mnemonic F*』で関数ごと C 経路)。新しい `src/x87.rs`(約 290 行)は、x87 の各命令を、**fe-lift の C が同じ命令で呼ぶ `fle_x87_*` 関数への呼び出し(同じ関数名・同じ定数引数、メモリ演算子は実効アドレス)**にする。演算は wasm でしない(f64 の近似は無い)。x87 の状態(レジスタ 8 本、TOP、制御・状態・タグ語、例外ビット)は C のグローバル 1 か所のままなので、**結果は C 経路と定義上同じ**で、直 wasm と C 経路の関数が交互に呼ばれても状態は 1 つ。FLEngine 全体の census で **63 関数(3,049 命令、wasm +44 KB)が直る**(8 関数は制御語 = 決定 0254、4 関数は Rcpps、2 関数 55 命令は FCOMI/FUCOMI で残る)。
2. **確かめたこと / 確かめていないこと**: 確かめた = D8〜DF の**全 ModRM の符号化**(レジスタ形、`[rdi]`、`[rdi+disp8]`)と WAIT について、直 wasm が呼ぶ関数・定数引数が fe-lift の C のそれと**文字列で一致**(`tests/x87_hybrid.rs`、変異 4 つを検出)。演算そのものの host の x87 命令との bit 一致は fe-lift の既存の x87 テスト(`x87.rs`、`x87_trans.rs`、`x87ctl.rs`)に依る(同じ C 関数を呼ぶので)。**確かめていない = 実エンジンの 84/55/87 行・a4・参照プロジェクト・wasm サイズの実測・CPU の短縮**(全面ビルドが要る。PM の決定で opt1 の E4 で重ねて回す)。
3. **効果の見積りは条件つき**: x87 だけでは小さい(ミキサーの自己時間で f_6a1750 44 ms + f_6a3990 24 ms = 0.5%、直 wasm の平均利得 1.6× を当てても約 25 ms = 0.2%)。ミキサーの本命 **f_67a160(1,537 ms = ミキサー約 11%)は x87 に加えて `rcpps` があり、そこで止まる**。opt1 の WIP(`opt1-f-wip.diff`、Rcpps を C の関数 `fle_hyb_mx` の kind 63 で呼ぶ)を**重ねた census で f_67a160 は ok(直 wasm 14,752 B)**になり、同じ一族の 8 関数(0x679340 ほか)が全部 ok。見積りは 1,550 ms × 0.375 ≈ 580 ms(ミキサーの約 4%、約 1.04 倍)だが、これは**平均の 1.6× を当てただけの未測定の値**で、f_67a160 の内側ループの `rcpps` が毎回 C の関数の呼び出しになる費用と、x87 の呼び出し(softfloat は変わらない)を含まない。**E4 の関数ごとの CPU 測定で確かめる**。

## 1. 現状の把握(HEAD 2ca825ff)
- fe-wasmgen の `lower.rs` に x87 算術の腕は無い(`Fnstcw`/`Fldcw` の腕だけ『制御語は C 経路の状態』で拒否)。x87 の命令は『mnemonic <名前>』の理由で、関数ごと C 経路(fe-lift の C。各 x87 命令が `fle_x87_*` の呼び出し)に残る。状態(`fle_x87_r[8]`、`fle_x87` の cw/sw/tw、例外ビット `X`)は最初から C のグローバルなので、**直 wasm から C 経路へ戻る時の x87 状態の受け渡し費用は無い**。C 経路の関数が遅いのは x87 の呼び出しではなく、その関数の他の命令(レジスタが `R[]` 経由、遅延フラグ)。
- FLEngine の census: 変更前に x87 が理由で止まっていたのは 75 関数。→ 63 が ok、8 が制御語、4 が Rcpps(上の 3)。表 `tables/x87-census-transitions.tsv`。
- ミキサーの x87 の自己時間(gpu3 `c-path-candidates-mixer.md`、x87 全体 2,437 ms): f_67a160 1,537(fle、626 B、156 命令、x87 は 37 命令、残りは SSE)、f_6a1750 44、f_6a3990 24、f_679800 13(= x87 の算術 1,618 ms)。制御語だけの関数(0254): dsp f_310fb40 761、f_32e9c00 122、f_306f260 46、f_30c7e20 42、f_30663a0 17、elq f_b1bb1c0 116、f_b1c44c0 65(= 1,169 ms)。UI 側に固有の x87 関数は gpu2/gpu3 の表に出ていない(『UI 2.61 s』は同じ関数の行)。
- f_67a160 の中身(逆アセンブル): 外側のループで FLD/FST/FMUL/FSUB/FSTP/FCOMPP/FLDPI の 37 命令(係数の計算)、内側のループは SSE(`rcpps`、`mulps`、`addps`、`shufps`、`movhlps` ほか)。

## 2. 設計の表
| 案 | 内容 | 行数 | 状態の一貫性(`X`・制御語・タグ・`keep_snan`) | 直る範囲 | リスク |
|---|---|---|---|---|---|
| **A ヘルパ呼び出し(採用)** | 各 x87 命令 = `call c.fle_x87_*`(定数引数 + 実効アドレス)。FNSTSW AX は `fle_x87_sw()` の下位 16 ビットを RAX に合成。FWAIT は何も出さない(fe-lift と同じ) | 約 290 + 25 | C の 1 か所だけ。wasm は触らない。単一 Worker の本番は 1 スレッドなので TLS は不要(実スレッドは本番の経路ではない) | 算術・ロード・ストア・比較・定数・FNSTSW/FNCLEX/FNINIT | 低 |
| B x87 スタックを wasm のローカルに | 80 ビット演算を wasm で再実装 | 1,500 以上 | 二重管理 | — | 高。**不採用** |
| C データ移動(FXCH・FLD ST・FCHS・FABS)だけ wasm のメモリで直接 | 呼び出し 1 回を省く | 約 200 | タグ・TOP・例外の不変条件を二重に持つ | 命令の約 15% | 中。利得が小さい |
| D FCOMI/FUCOMI/FCMOV | `flags.rs` に新しい定義の種類(CF/PF/ZF を helper の戻りから) | 約 120 | 同 A | 2 関数 55 命令 | 中。後回し(PM 了承) |
| 制御語 FNSTCW/FLDCW | 決定 0254(`mt3-wasmgen-small.diff`)。本差分は触らない | — | — | 8 関数(206 命令) | — |

## 3. 実装(`mt3-x87-lower.diff`)
- `src/x87.rs`(`lower.rs` の子モジュール。`L` の内部を使うため): `plan(insn)` が命令を (ヘルパ名, 引数の並び, 戻り) にする。引数は定数か実効アドレス(`A::C(u32)` / `A::Addr`)。**lifter(`fe-lift/src/x87.rs`、`x87ctl.rs`)と同じ形の検査**(オペランド数・サイズ・ST(0) を含むこと)を移し、取らない形は `Err` で関数が C 経路に残る。対象: FLD/FILD/FST/FSTP/FIST/FISTP/FISTTP、FADD/FSUB/FSUBR/FMUL/FDIV/FDIVR(m32/m64、ST 形、P 形、整数メモリ形)、FCOM/FCOMP/FCOMPP/FUCOM/FUCOMP/FUCOMPP/FICOM/FICOMP、FTST、FXAM、FXCH、FCHS、FABS、FSQRT、FRNDINT、FSCALE、F2XM1、FSINCOS、FLD の定数 7 種、FNSTSW(AX と m16)、FNCLEX、FNINIT、FWAIT。
- `lower.rs`: 1 か所のフック `if self.cx.c_mode && x87::is_x87(m) { return self.x87_insn(&i); }`(`match m` の直前)と `pub mod x87;`(`#[path]` つき)。`c_mode` でないとき(C 部の無い単独の wasm)は従来どおり。
- `lib.rs`: 乾式の `c_imp` に全ヘルパ名を入れ(乾式は呼び出しの添字を使わない)、本番の import は**そのモジュールの関数が実際に使うヘルパだけ**(`helpers_of`)を `c.fle_x87_*` で宣言し、`c_imp` に入れる。x87 の無いモジュールの import は 0。`Ctx` に欄を足さない(`c_imp` の写像を使う)。
- 副作用: `FNCLEX` は C 経路の hybrid(`c_mode`)で従来『何もしない』腕に落ちていた(`M::Fnclex => Ok(())`)が、本差分で `fle_x87_clex` の呼び出しになる(状態語の例外ビットを実際に消す。C 経路の関数が立てたビットが直 wasm で消えない食い違いが、これまで x87 を含む直 wasm が無かったので表に出ていなかった)。

## 4. テスト(`tests/x87_hybrid.rs`、3 件。`dev-dependencies` に `fe-lift`)
1. `every_x87_encoding_makes_the_call_the_lifted_c_makes`: D8〜DF × 全 ModRM(レジスタ形 + `[rdi]` + `[rdi+disp8]`)+ WAIT = 641 の符号化(390 が一致、lifter だけが取る 4 種のヘルパ、151 はどちらも取らない)。直 wasm の呼び出し(`call_text`)と `fe_lift::lift_body` の C の呼び出しが**文字列で一致**(メモリ引数は ADDR)。lifter が取らない形は wasm も取らない。wasm が取らず lifter だけが取る形は **FCOMI/FUCOMI/FCMOV/FNSTCW/FLDCW の 4 種(`fle_x87_comi`・`fcmov`・`store_cw`・`load_cw`)に限る**。表の全ヘルパが少なくとも 1 つの符号化の答えになっている。
2. 生成したコードを node で: 実効アドレス(`[rdi+rsi*8+0x10]` ほか)、引数の順、`FNSTSW AX` が RAX の下位 16 ビットだけを置き換える(helper が上位に垃圾を返しても切る)、FWAIT は何も出さない。
3. import は使うヘルパだけ(`fld1; fstp qword [rdi]` → `fld_const` と `fst_m` の 2 つ)。x87 の無い関数は 0。`FUCOMI` を含む関数は C 経路に残る。
- 変異: dst/src の入れ替え、`pop` フラグの落とし、AX の 16 ビットのマスクの除去、FWAIT で呼び出しを出す、の 4 つはすべて検出。(実効アドレスから `mem_addr` が返す変位を落とす変異は通る: `mem_addr` が変位を基底に畳むので変位は常に 0 の、等価な変異。)
- `cargo test -p fe-wasmgen --no-fail-fast`(HEAD 2ca825ff + 本差分): 21 本すべて ok、27 件通過、失敗 0、無視 1。

## 5. census(FLEngine 全体、hybrid の実際の旗、`tables/`)
| | functions_ok | insns_ok | code_bytes_ok | module_bytes |
|---|---|---|---|---|
| HEAD 2ca825ff | 58,555 | 2,276,795 | 55,854,908 | 69,138,350 |
| + x87.rs | 58,618(+63) | 2,279,844(+3,049) | 55,898,156(+43,248) | 69,194,604(+56,254) |
| + x87.rs + opt1-f-wip.diff(Rcpps ほか) | 58,637(+82) | 2,281,308 | 56,026,942 | 69,327,341 |
- **f_67a160 は『x87.rs だけ』では `mnemonic Rcpps`、『x87.rs + opt1 の WIP』では ok(14,752 B)**。同じ一族の 0x679340・0x679800・0x679cc0・0x67b9a0・0x67bfd0・0x67c600・0x67cc10 も ok(`tables/rcpps-functions-with-opt1-wip.tsv`)。opt1 の WIP の Rcpps の腕(`mx_c_only`、`src_vec` でレジスタ/メモリのソース、kind 63)は f_67a160 の `rcpps xmm7,xmm4` を通せる。不足は無い。
- wasm サイズ: census は全関数を入れた値(module +56 KB)で、実際は hybrid の一覧に入れた関数だけ。S1 の線 154,885,888 B に対し現在 141,802,968 B(余裕 13.1 MB)。f_67a160 の 8 関数を足すと約 +115 KB(約 14 KB × 8、census の値)= 余裕の 0.9%。

## 6. 効果の見積り(測っていない。読みと平均値の積)
| 項目 | ミキサーの自己時間 | 直る条件 |
|---|---|---|
| f_67a160 | 1,537 ms | x87.rs + Rcpps(opt1 の WIP) |
| f_679800 | 13 ms | 同上 |
| f_6a1750 + f_6a3990 | 68 ms | x87.rs だけ |
| 合計 | 約 1,620 ms(ミキサー 約 14.2 s の 11.4%) | |
- 直 wasm の平均の短縮(非ハイブリッド 2.1 → 1.3 CPU 秒/秒 = 1.6×、短縮 37.5%)を当てると 約 600 ms = ミキサーの約 4.2%(約 1.044 倍)。**1.3 倍(P3 の条件)には遠い**。
- この見積りが当たらない側に動く要因(すべて未測定): (a) f_67a160 の内側ループは `rcpps` を 1 反復に 1 回持つので、opt1 の kind 63 では**毎反復 C の関数(`fle_hyb_mx`)の呼び出し + v128 の退避/復帰**になる(C 経路ではインラインの表引き)。(b) x87 の呼び出しは softfloat の時間を減らさない(外側のループ 1 回に 37 回)。(c) 平均の 1.6× は SSE を多く含む関数の値で、この関数の内側ループが SSE 主体なのでその点は有利。→ **E4 の関数ごとの自己時間(f_67a160 の前後)で確認する**。
- 生成した wasm の静的な事実(x87.rs + opt1 の WIP で f_67a160 と f_679800 だけを一覧に入れて作った module。`wasm-validate --enable-all` が通る。f_67a160 は 14,612 B、f_679800 は 14,955 B): f_67a160 の `fle_x87_*` の呼び出し箇所は 37(命令数と同じ)、`fle_hyb_mx` の呼び出し箇所は 82。後者の大半は MXCSR の窓の遅い経路(窓の外のときだけ通る)で、**無条件に通るのは `rcpps` の 1 箇所**(opt1 の `mx_c_only` → `mx_slow` は v128 を 2 つ退避し、`fle_hyb_mx` を呼び、結果を読み戻す)。それがこの関数の内側ループで毎反復に 1 回。反復数は未計測。
- 『opt1 に返す不足の表』(PM の指示 1): **不足は無い**(f_67a160 の `rcpps xmm7,xmm4` は opt1 の Rcpps の腕を通る)。表 `tables/rcpps-functions-with-opt1-wip.tsv`。

## 7. 衝突の回避と取り込み
- lower.rs に触る所は 2 か所(`pub mod x87;` と `match m` の直前の 5 行)。**重ねの確認(この機械、cargo のみ。実エンジンでは未実施)**: ①本差分 + opt1 の `opt1-f-wip.diff`(`lift/target/*` を除く): opt1 の差分は `lower.rs` に offset 7 行で当たり、`cargo test -p fe-wasmgen --no-fail-fast` は 23 本ok・29 通過・失敗 0・無視 1、census は表のとおり。②さらに mt3 の `mt3-wasmgen-small.diff`(0254)を重ねた木: `patch -p1` は通る(`lib.rs` の最初の hunk が **fuzz 2・offset 51 行**、他は offset のみ。`.orig` が出る)、`cargo test -p fe-wasmgen --no-fail-fast` は 24 本ok・38 通過・失敗 0・無視 1。fuzz 2 の hunk は `lib.rs` の乾式の `Ctx` 付近(位置から読んだだけで、原因は確かめていない)。テストは通るが、**fuzz のあるパッチなので E3/E4 では当て方を目で確認する**。0254 の Fnstcw/Fldcw の腕とは別の場所で、本差分はそれを置き換えない(`is_x87` に Fnstcw/Fldcw を含めない)。
- `Cargo.lock` が変わる(fe-wasmgen の dev-dependency に fe-lift)。`--locked` のビルドでは差分に含めたこの変更が要る。

## 8. 限界
- 実エンジン(ブラウザ/node)での走行、84/55/87 行、a4、参照プロジェクト、CPU 測定は 0 件(上の 2、6)。
- FCOMI/FUCOMI/FCMOV を含む関数(2 関数 55 命令)と、制御語(8 関数)、FSIN/FCOS/FPTAN/FPATAN/FYL2X/FYL2XP1/FPREM/FPREM1/FXTRACT と状態の保存/復元(lifter も未対応)を含む関数は C 経路のまま。
- dsp・Elastique など FLEngine 以外のモジュールの census は取っていない(この機械に実物の DLL は FLEngine だけ)。x87 の関数が多い dsp の制御語は 0254 の担当。

## PM の決定と取り込み(2026-11-05 21:35、docs のみ)

- 文書・表・道具・差分(`docs/investigations/mt3x/patches/mt3-x87-lower.diff`)を置いた。**製品の木にはまだ当てていない。** PM は mt3 の census・テストを再現していない。mt3 の測定: HEAD 69c2ea2e の `git archive` に `patch -p1 --dry-run` が通る(offset/fuzz 0)。`cargo test -p fe-wasmgen` 21 本 ok・27 通過・失敗 0(x87 のみ)、opt1 の WIP を重ねた木で 23 本・29 通過、さらに `mt3-wasmgen-small.diff` を重ねた木で 24 本・38 通過。重ねた木では lib.rs の最初の hunk が fuzz 2・offset 51 になるので、E3/E4 で重ねる opt1 は当て方を目で確認すること。
- 実エンジン(ブラウザ/node)の 84/55/87 行・a4・参照プロジェクト・wasm サイズ・CPU の測定は 0 件。取り込み(製品の木への適用)は、opt1 の E4(E3 + 本差分)で次を確認してから: node の 84/55/87 行が変わらない(変わる行は関数ごとに説明)、参照プロジェクトが開き 30 s 再生で止まらない、Export の出力同一、wasm のサイズ(S1 線 154,885,888 B)、`f_67a160`(ミキサーの本命、自己 1,537 ms)の関数ごとの自己時間(前後)と書き込み率。
- 見積りは条件つき: x87 だけではミキサーの 0.5% 未満。`f_67a160` は `rcpps` の C 呼び出し(opt1 の `mx_c_only` → `fle_hyb_mx`)が内側ループの毎反復に 1 回通るので、反復あたりの呼び出し費用が利得(1.6×)を食う/逆転する可能性がある。**E4 で前後の自己時間を取って判定する。**逆転するなら Rcpps の腕は採らない。
- 副作用の記録: C 経路の hybrid で `FNCLEX` が従来『何もしない』腕だった食い違いを `fle_x87_clex` の呼び出しにした(これまで x87 を含む直 wasm が無く表に出ていなかった)。
